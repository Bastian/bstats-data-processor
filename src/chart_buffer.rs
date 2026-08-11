use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::future::join_all;
use redis::cluster::{ClusterClient, ClusterClientBuilder};
use redis::cluster_async::ClusterConnection;
use redis::cluster_routing::{RoutingInfo, SingleNodeRoutingInfo};
use redis::{Cmd, ErrorKind, RedisError, RedisResult, Value, from_redis_value};

use crate::util::redis::redis_cluster_urls;

/// How long chart updates are held back before they are flushed to Redis.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

/// Emergency brake against unbounded memory while Redis is unreachable.
const MAX_BUFFERED_ENTRIES: usize = 500_000;

/// The additive chart updates of a single request. Applied to the shared
/// [`ChartBuffer`] all at once, so rejected requests write nothing.
#[derive(Default)]
pub struct ChartOps {
    ops: Vec<ChartOp>,
}

enum ChartOp {
    ZIncr {
        key: String,
        member: String,
        delta: i64,
    },
    HIncr {
        key: String,
        field: String,
        delta: i64,
    },
    Expire {
        key: String,
        seconds: i64,
    },
}

impl ChartOps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zincr(&mut self, key: String, member: String, delta: i64) {
        self.ops.push(ChartOp::ZIncr { key, member, delta });
    }

    pub fn hincr(&mut self, key: String, field: String, delta: i64) {
        self.ops.push(ChartOp::HIncr { key, field, delta });
    }

    pub fn expire(&mut self, key: String, seconds: i64) {
        self.ops.push(ChartOp::Expire { key, seconds });
    }
}

/// Process-wide buffer that sums chart deltas between flushes.
#[derive(Default)]
pub struct ChartBuffer {
    state: Mutex<BufferState>,
}

#[derive(Default)]
struct BufferState {
    /// (key, member) -> summed delta
    zincrs: HashMap<(String, String), i64>,
    /// (key, field) -> summed delta
    hincrs: HashMap<(String, String), i64>,
    /// key -> TTL in seconds
    expires: HashMap<String, i64>,
}

impl BufferState {
    fn len(&self) -> usize {
        self.zincrs.len() + self.hincrs.len() + self.expires.len()
    }
}

impl ChartBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&self, ops: ChartOps) {
        self.merge_with_limit(ops, MAX_BUFFERED_ENTRIES);
    }

    fn merge_with_limit(&self, ops: ChartOps, limit: usize) {
        let mut dropped = 0;
        {
            let mut state = self.state.lock().unwrap();
            for op in ops.ops {
                match op {
                    ChartOp::ZIncr { key, member, delta } => {
                        *state.zincrs.entry((key, member)).or_insert(0) += delta;
                    }
                    ChartOp::HIncr { key, field, delta } => {
                        *state.hincrs.entry((key, field)).or_insert(0) += delta;
                    }
                    ChartOp::Expire { key, seconds } => {
                        state.expires.insert(key, seconds);
                    }
                }
            }
            if state.len() > limit {
                // The buffer only grows this far when flushes fail, and
                // without Redis there is nowhere to flush to: drop everything.
                dropped = state.len();
                *state = BufferState::default();
            }
        }
        // Log outside the lock: stderr can block, and a panicking write would
        // poison the mutex for every later request.
        if dropped > 0 {
            eprintln!("chart buffer exceeded {limit} entries, dropped {dropped} buffered updates");
        }
    }

    fn is_empty(&self) -> bool {
        self.state.lock().unwrap().len() == 0
    }

    /// Swaps the buffer out under the lock, so requests never wait on a flush.
    fn take(&self) -> BufferState {
        std::mem::take(&mut *self.state.lock().unwrap())
    }
}

#[derive(Default)]
struct SlotMap {
    nodes: Vec<(String, u16)>,
    /// (first slot, last slot, index into `nodes`)
    ranges: Vec<(u16, u16, usize)>,
}

impl SlotMap {
    fn node_for_key(&self, key: &str) -> Option<usize> {
        let slot = key_hash_slot(key.as_bytes());
        self.ranges
            .iter()
            .find(|(start, end, _)| (*start..=*end).contains(&slot))
            .map(|(_, _, node)| *node)
    }
}

/// Sends the buffered updates as one non-atomic pipeline per master node.
///
/// Grouping by node keeps a flush at a handful of round trips, where grouping
/// by slot would send thousands of tiny pipelines. The cluster client refuses
/// cross-slot pipelines, so the flusher keeps its own slot map and routes each
/// pipeline to its node explicitly.
pub struct ChartFlusher {
    client: ClusterClient,
    /// Established on the first flush, so the processor starts (and buffers)
    /// even while Redis is still unreachable.
    connection: Option<ClusterConnection>,
    slot_map: SlotMap,
}

impl ChartFlusher {
    pub fn new() -> RedisResult<Self> {
        // No internal retries: a retry would resend a partially executed
        // pipeline and double count deltas. Failed batches are dropped.
        let client = ClusterClientBuilder::new(redis_cluster_urls())
            .retries(0)
            .build()?;
        Ok(Self {
            client,
            connection: None,
            slot_map: SlotMap::default(),
        })
    }

    /// Redis executes every command of a pipeline it receives and reports
    /// errors per command, so a failed node batch is dropped, never resent.
    ///
    /// The buffer is only taken once a connection and a slot map exist. While
    /// Redis is unreachable it keeps accumulating (up to its size limit)
    /// instead of losing a window per attempt.
    pub async fn flush(&mut self, buffer: &ChartBuffer) -> RedisResult<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let connection = self.connection().await?;
        if self.slot_map.ranges.is_empty() {
            self.refresh_slot_map().await?;
        }

        let state = buffer.take();
        let (pipelines, unroutable) = build_node_pipelines(state, &self.slot_map);

        let results = join_all(pipelines.into_iter().map(|(node, pipeline)| {
            let (host, port) = self.slot_map.nodes[node].clone();
            let mut connection = connection.clone();
            async move {
                let count = pipeline.len();
                let route = SingleNodeRoutingInfo::ByAddress { host, port };
                connection.route_pipeline(&pipeline, 0, count, route).await
            }
        }))
        .await;

        let mut result = Ok(());
        for outcome in results {
            if let Err(e) = outcome {
                // Most likely a stale topology (failover or resharding)
                self.slot_map = SlotMap::default();
                result = Err(e);
            }
        }
        if unroutable > 0 {
            // A hole in the slot map, e.g. a master that died without a
            // replica. Refetch next flush instead of dropping keys forever.
            self.slot_map = SlotMap::default();
            eprintln!("chart buffer flush dropped {unroutable} updates without a slot owner");
        }
        result
    }

    async fn connection(&mut self) -> RedisResult<ClusterConnection> {
        if let Some(connection) = &self.connection {
            return Ok(connection.clone());
        }
        let connection = self.client.get_async_connection().await?;
        self.connection = Some(connection.clone());
        Ok(connection)
    }

    async fn refresh_slot_map(&mut self) -> RedisResult<()> {
        let mut connection = self.connection().await?;
        let mut cmd = redis::cmd("CLUSTER");
        cmd.arg("SLOTS");
        let reply = connection
            .route_command(&cmd, RoutingInfo::SingleNode(SingleNodeRoutingInfo::Random))
            .await?;
        self.slot_map = parse_cluster_slots(reply)?;
        Ok(())
    }
}

/// Runs until `shutdown` fires, then flushes one final time so a regular
/// shutdown loses nothing.
pub async fn flush_loop(
    buffer: Arc<ChartBuffer>,
    mut flusher: ChartFlusher,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let mut interval = tokio::time::interval(FLUSH_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = flusher.flush(&buffer).await {
                    eprintln!("chart buffer flush failed: {e}");
                }
            }
            _ = &mut shutdown => break,
        }
    }
    if let Err(e) = flusher.flush(&buffer).await {
        eprintln!("chart buffer shutdown flush failed: {e}");
    }
}

/// Returns how many commands had no node in the slot map.
fn build_node_pipelines(
    state: BufferState,
    slot_map: &SlotMap,
) -> (HashMap<usize, redis::Pipeline>, usize) {
    let mut pipelines: HashMap<usize, redis::Pipeline> = HashMap::new();
    let mut unroutable = 0;
    let mut push = |key: &str, cmd: Cmd| match slot_map.node_for_key(key) {
        Some(node) => {
            pipelines
                .entry(node)
                .or_insert_with(redis::pipe)
                .add_command(cmd);
        }
        None => unroutable += 1,
    };

    for ((key, member), delta) in &state.zincrs {
        let mut cmd = Cmd::new();
        cmd.arg("ZINCRBY").arg(key).arg(delta).arg(member);
        push(key, cmd);
    }
    for ((key, field), delta) in &state.hincrs {
        let mut cmd = Cmd::new();
        cmd.arg("HINCRBY").arg(key).arg(field).arg(delta);
        push(key, cmd);
    }
    // Last, so they run after the increments that create their keys
    for (key, seconds) in &state.expires {
        let mut cmd = Cmd::new();
        cmd.arg("EXPIRE").arg(key).arg(seconds);
        push(key, cmd);
    }
    (pipelines, unroutable)
}

fn parse_cluster_slots(reply: Value) -> RedisResult<SlotMap> {
    fn type_error() -> RedisError {
        RedisError::from((ErrorKind::TypeError, "unexpected CLUSTER SLOTS reply"))
    }

    let mut slot_map = SlotMap::default();
    let ranges: Vec<Vec<Value>> = from_redis_value(&reply)?;
    for range in ranges {
        let (Some(start), Some(end), Some(master)) = (range.first(), range.get(1), range.get(2))
        else {
            return Err(type_error());
        };
        let start: u16 = from_redis_value(start)?;
        let end: u16 = from_redis_value(end)?;
        let master: Vec<Value> = from_redis_value(master)?;
        let (Some(host), Some(port)) = (master.first(), master.get(1)) else {
            return Err(type_error());
        };
        let host: String = from_redis_value(host)?;
        let port: u16 = from_redis_value(port)?;

        let node = match slot_map
            .nodes
            .iter()
            .position(|n| n.0 == host && n.1 == port)
        {
            Some(index) => index,
            None => {
                slot_map.nodes.push((host, port));
                slot_map.nodes.len() - 1
            }
        };
        slot_map.ranges.push((start, end, node));
    }
    if slot_map.ranges.is_empty() {
        return Err(type_error());
    }
    Ok(slot_map)
}

/// The slot of a key: CRC16-XMODEM over the hashtag (or the whole key) mod
/// 16384, as defined by the Redis cluster specification.
fn key_hash_slot(key: &[u8]) -> u16 {
    let key = hashtag(key).unwrap_or(key);
    crc16(key) % 16384
}

/// The content between the first `{` and the next `}`, unless it is empty.
fn hashtag(key: &[u8]) -> Option<&[u8]> {
    let open = key.iter().position(|&b| b == b'{')?;
    let close = key[open + 1..].iter().position(|&b| b == b'}')?;
    if close == 0 {
        return None;
    }
    Some(&key[open + 1..open + 1 + close])
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_matches_the_spec_vector() {
        // Test vector from the Redis cluster specification appendix
        assert_eq!(crc16(b"123456789"), 0x31C3);
    }

    #[test]
    fn hash_slot_uses_the_hashtag() {
        assert_eq!(key_hash_slot(b"data:{1}.2.3"), key_hash_slot(b"{1}"));
        assert_eq!(
            key_hash_slot(b"data:{1}.2.3"),
            key_hash_slot(b"data:{1}.9.9")
        );
        // An empty hashtag does not count, the whole key is hashed
        assert_eq!(key_hash_slot(b"a{}b"), crc16(b"a{}b") % 16384);
        assert_eq!(key_hash_slot(b"no-tag"), crc16(b"no-tag") % 16384);
    }

    #[test]
    fn merge_sums_deltas_and_dedupes_expires() {
        let buffer = ChartBuffer::new();

        let mut ops = ChartOps::new();
        ops.zincr("k".into(), "m".into(), 2);
        ops.hincr("h".into(), "f".into(), 1);
        ops.expire("k".into(), 3660);
        buffer.merge(ops);

        let mut ops = ChartOps::new();
        ops.zincr("k".into(), "m".into(), 3);
        ops.hincr("h".into(), "f".into(), 4);
        ops.expire("k".into(), 3660);
        buffer.merge(ops);

        let state = buffer.take();
        assert_eq!(state.zincrs[&("k".to_string(), "m".to_string())], 5);
        assert_eq!(state.hincrs[&("h".to_string(), "f".to_string())], 5);
        assert_eq!(state.expires["k"], 3660);
        assert_eq!(state.len(), 3);
    }

    #[test]
    fn taking_the_buffer_leaves_it_empty() {
        let buffer = ChartBuffer::new();
        let mut ops = ChartOps::new();
        ops.zincr("k".into(), "m".into(), 1);
        buffer.merge(ops);

        assert_eq!(buffer.take().len(), 1);
        assert_eq!(buffer.take().len(), 0);
    }

    #[test]
    fn overflowing_the_limit_drops_the_buffer() {
        let buffer = ChartBuffer::new();
        let mut ops = ChartOps::new();
        for i in 0..4 {
            ops.zincr(format!("k{i}"), "m".into(), 1);
        }
        buffer.merge_with_limit(ops, 3);
        assert_eq!(buffer.take().len(), 0);
    }

    #[test]
    fn expires_come_after_increments() {
        let slot_map = SlotMap {
            nodes: vec![("localhost".into(), 6379)],
            ranges: vec![(0, 16383, 0)],
        };
        let mut state = BufferState::default();
        state.expires.insert("k".into(), 3660);
        state.zincrs.insert(("k".into(), "m".into()), 1);
        state.hincrs.insert(("k2".into(), "f".into()), 1);

        let (pipelines, unroutable) = build_node_pipelines(state, &slot_map);
        assert_eq!(pipelines.len(), 1);
        assert_eq!(unroutable, 0);

        let commands: Vec<_> = pipelines[&0]
            .cmd_iter()
            .map(|cmd| match cmd.args_iter().next().unwrap() {
                redis::Arg::Simple(name) => String::from_utf8(name.to_vec()).unwrap(),
                redis::Arg::Cursor => unreachable!(),
            })
            .collect();
        assert_eq!(commands, ["ZINCRBY", "HINCRBY", "EXPIRE"]);
    }

    #[test]
    fn keys_without_a_slot_owner_are_counted() {
        // A slot map with a hole, as after losing a master without a replica
        let slot = key_hash_slot(b"covered");
        let slot_map = SlotMap {
            nodes: vec![("localhost".into(), 6379)],
            ranges: vec![(slot, slot, 0)],
        };
        let mut state = BufferState::default();
        state.zincrs.insert(("covered".into(), "m".into()), 1);
        state.zincrs.insert(("uncovered".into(), "m".into()), 1);
        state.expires.insert("uncovered".into(), 3660);

        let (pipelines, unroutable) = build_node_pipelines(state, &slot_map);
        assert_eq!(pipelines[&0].len(), 1);
        assert_eq!(unroutable, 2);
    }
}

#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use crate::test_support::test_environment::TestEnvironment;
    use redis::AsyncCommands;

    #[tokio::test]
    async fn flushes_summed_deltas_across_nodes() {
        let test_environment = TestEnvironment::empty().await;
        let mut con = test_environment.redis_connection().await;
        let mut flusher = ChartFlusher::new().unwrap();

        let buffer = ChartBuffer::new();
        let mut ops = ChartOps::new();
        ops.zincr("data:{9301}.1.100".into(), "value".into(), 2);
        ops.zincr("data:{9301}.1.100".into(), "value".into(), 3);
        ops.hincr("data:9302.1".into(), "1000".into(), 4);
        ops.expire("data:{9301}.1.100".into(), 3660);
        buffer.merge(ops);
        flusher.flush(&buffer).await.unwrap();

        let pie: i64 = con.zscore("data:{9301}.1.100", "value").await.unwrap();
        let line: i64 = con.hget("data:9302.1", "1000").await.unwrap();
        let ttl: i64 = con.ttl("data:{9301}.1.100").await.unwrap();
        assert_eq!(pie, 5);
        assert_eq!(line, 4);
        assert!(ttl > 0, "expire must be applied");

        // A second flush adds on top, as concurrent instances would
        let mut ops = ChartOps::new();
        ops.zincr("data:{9301}.1.100".into(), "value".into(), 10);
        buffer.merge(ops);
        flusher.flush(&buffer).await.unwrap();

        let pie: i64 = con.zscore("data:{9301}.1.100", "value").await.unwrap();
        assert_eq!(pie, 15);
    }

    #[tokio::test]
    async fn flushing_an_empty_buffer_is_a_no_op() {
        let _test_environment = TestEnvironment::empty().await;
        let mut flusher = ChartFlusher::new().unwrap();
        flusher.flush(&ChartBuffer::new()).await.unwrap();
    }

    #[tokio::test]
    async fn hash_slots_match_cluster_keyslot() {
        let test_environment = TestEnvironment::empty().await;
        let mut con = test_environment.redis_connection().await;

        for key in [
            "data:{27400}.1.1217905",
            "data:335501.1",
            "ratelimit:1#abc.bukkit.1337",
            "a{}b",
            "a{b}c",
            "{foo}{bar}",
        ] {
            let mut cmd = redis::cmd("CLUSTER");
            cmd.arg("KEYSLOT").arg(key);
            let reply = con
                .route_command(&cmd, RoutingInfo::SingleNode(SingleNodeRoutingInfo::Random))
                .await
                .unwrap();
            let expected: u16 = from_redis_value(&reply).unwrap();
            assert_eq!(key_hash_slot(key.as_bytes()), expected, "key {key}");
        }
    }

    #[tokio::test]
    async fn a_failing_command_does_not_affect_the_others() {
        let test_environment = TestEnvironment::empty().await;
        let mut con = test_environment.redis_connection().await;
        let mut flusher = ChartFlusher::new().unwrap();
        flusher.refresh_slot_map().await.unwrap();

        // Two keys on the same node and one on another node
        let mut by_node: HashMap<usize, Vec<String>> = HashMap::new();
        for i in 0..100 {
            let key = format!("data:{{{i}}}.1.1");
            if let Some(node) = flusher.slot_map.node_for_key(&key) {
                by_node.entry(node).or_default().push(key);
            }
        }
        let mut nodes = by_node.into_values().filter(|keys| keys.len() >= 2);
        let first_node = nodes.next().expect("no node with two keys");
        let other_node = nodes.next().expect("test needs a second node");
        let (poisoned, same_node) = (&first_node[0], &first_node[1]);
        let other = &other_node[0];

        let _: () = con.set(poisoned, "not-a-sorted-set").await.unwrap();

        let buffer = ChartBuffer::new();
        let mut ops = ChartOps::new();
        for key in [poisoned, same_node, other] {
            ops.zincr(key.clone(), "value".into(), 1);
        }
        buffer.merge(ops);

        let error = flusher
            .flush(&buffer)
            .await
            .expect_err("wrong type must surface");
        assert!(
            error.to_string().contains("WRONGTYPE"),
            "unexpected error: {error}"
        );

        // Redis executed everything else in both node pipelines
        let same_node_value: i64 = con.zscore(same_node, "value").await.unwrap();
        let other_value: i64 = con.zscore(other, "value").await.unwrap();
        assert_eq!(same_node_value, 1);
        assert_eq!(other_value, 1);
    }

    #[tokio::test]
    async fn shutdown_flush_writes_pending_updates() {
        let test_environment = TestEnvironment::empty().await;

        let buffer = Arc::new(ChartBuffer::new());
        let mut ops = ChartOps::new();
        ops.zincr("data:{9401}.1.100".into(), "value".into(), 7);
        buffer.merge(ops);

        let flusher = ChartFlusher::new().unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let flush_task = tokio::spawn(flush_loop(buffer.clone(), flusher, shutdown_rx));
        shutdown_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), flush_task)
            .await
            .expect("flush loop must stop on shutdown")
            .unwrap();

        let mut con = test_environment.redis_connection().await;
        let value: i64 = con.zscore("data:{9401}.1.100", "value").await.unwrap();
        assert_eq!(value, 7);
    }
}
