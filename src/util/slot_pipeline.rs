use std::collections::HashMap;

use futures_util::future::join_all;
use redis::cluster_async::ClusterConnection;
use redis::cluster_routing::{Route, RoutingInfo, SingleNodeRoutingInfo};
use redis::{Cmd, RedisResult, ToRedisArgs};

/// A pipeline for commands whose keys may live in different cluster slots.
///
/// Needed because the cluster client routes a pipeline to the slot of its first
/// command and rejects the rest with `CrossSlot`. Commands are grouped by route
/// here and flushed as one pipeline per route.
#[derive(Default)]
pub struct SlotPipeline {
    pipelines: HashMap<Option<Route>, redis::Pipeline>,
}

impl SlotPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Only for commands that route to a single key. Commands the client would
    /// normally fan out to every master (`DBSIZE`, `FLUSHALL`, `INFO`, ...)
    /// would silently hit a single node here and return a partial result.
    pub fn add_command(&mut self, cmd: Cmd) {
        let routing = RoutingInfo::for_routable(&cmd);
        let route = match routing {
            Some(RoutingInfo::SingleNode(SingleNodeRoutingInfo::SpecificNode(route))) => {
                Some(route)
            }
            ref other => {
                debug_assert!(false, "command without a single route: {other:?}");
                None
            }
        };
        self.pipelines
            .entry(route)
            .or_insert_with(redis::pipe)
            .add_command(cmd);
    }

    pub fn zincr<K: ToRedisArgs, M: ToRedisArgs, D: ToRedisArgs>(
        &mut self,
        key: K,
        member: M,
        delta: D,
    ) {
        let mut cmd = Cmd::new();
        cmd.arg("ZINCRBY").arg(key).arg(delta).arg(member);
        self.add_command(cmd);
    }

    pub fn hincr<K: ToRedisArgs, F: ToRedisArgs, D: ToRedisArgs>(
        &mut self,
        key: K,
        field: F,
        delta: D,
    ) {
        let mut cmd = Cmd::new();
        cmd.arg("HINCRBY").arg(key).arg(field).arg(delta);
        self.add_command(cmd);
    }

    pub fn expire<K: ToRedisArgs>(&mut self, key: K, seconds: i64) {
        let mut cmd = Cmd::new();
        cmd.arg("EXPIRE").arg(key).arg(seconds);
        self.add_command(cmd);
    }

    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }

    /// Number of pipelines a flush would send. Useful to assert in tests that
    /// commands are grouped as expected.
    pub fn slot_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Sends one pipeline per slot concurrently.
    ///
    /// This is not atomic across slots. Every pipeline is awaited even if
    /// another one fails, so a failing slot cannot cancel the writes of another,
    /// but it also means the successful slots stay committed when an error is
    /// returned. If several slots fail, which of their errors is returned is
    /// unspecified.
    pub async fn query_async(self, con: &ClusterConnection) -> RedisResult<()> {
        let results = join_all(self.pipelines.into_values().map(|pipeline| {
            let mut con = con.clone();
            async move { pipeline.query_async::<()>(&mut con).await }
        }))
        .await;

        results.into_iter().collect::<RedisResult<Vec<()>>>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_commands_by_slot() {
        let mut pipeline = SlotPipeline::new();
        assert!(pipeline.is_empty());

        // Same hash tag, so both keys share a slot
        pipeline.zincr("data:{1}.2.3", "value", 1);
        pipeline.expire("data:{1}.2.3", 60 * 61);
        assert_eq!(pipeline.slot_count(), 1);

        // No hash tag, so this key is routed independently
        pipeline.hincr("data:3.1", 1000, 1);
        assert_eq!(pipeline.slot_count(), 2);

        // A second key without a hash tag lands in yet another slot
        pipeline.hincr("data:4.1", 1000, 1);
        assert_eq!(pipeline.slot_count(), 3);
    }
}

#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use crate::test_support::test_environment::TestEnvironment;
    use redis::AsyncCommands;

    #[tokio::test]
    async fn writes_keys_from_different_slots() {
        let test_environment = TestEnvironment::empty().await;
        let mut con = test_environment.redis_connection().await;

        let mut pipeline = SlotPipeline::new();
        pipeline.hincr("data:9101.1", 1000, 5);
        pipeline.hincr("data:9102.1", 1000, 7);
        pipeline.zincr("data:{9103}.2.3", "value", 3);
        pipeline.expire("data:{9103}.2.3", 60 * 61);
        assert!(pipeline.slot_count() > 1, "test needs more than one slot");

        pipeline.query_async(&con).await.unwrap();

        let line_a: i64 = con.hget("data:9101.1", 1000).await.unwrap();
        let line_b: i64 = con.hget("data:9102.1", 1000).await.unwrap();
        let pie: i64 = con.zscore("data:{9103}.2.3", "value").await.unwrap();
        let ttl: i64 = con.ttl("data:{9103}.2.3").await.unwrap();
        assert_eq!(line_a, 5);
        assert_eq!(line_b, 7);
        assert_eq!(pie, 3);
        assert!(ttl > 0, "expire must be applied within its slot group");
    }

    #[tokio::test]
    async fn empty_pipeline_is_a_no_op() {
        let test_environment = TestEnvironment::empty().await;
        let con = test_environment.redis_connection().await;

        let pipeline = SlotPipeline::new();
        assert!(pipeline.is_empty());
        pipeline.query_async(&con).await.unwrap();
    }

    #[tokio::test]
    async fn reports_errors_instead_of_swallowing_them() {
        let test_environment = TestEnvironment::empty().await;
        let mut con = test_environment.redis_connection().await;

        // A string cannot be incremented as a hash
        let _: () = con.set("data:9201.1", "not-a-hash").await.unwrap();

        let mut pipeline = SlotPipeline::new();
        pipeline.hincr("data:9201.1", 1000, 1);
        pipeline.hincr("data:9202.1", 1000, 1);

        let error = pipeline
            .query_async(&con)
            .await
            .expect_err("wrong type must surface");
        assert!(
            error.to_string().contains("WRONGTYPE"),
            "unexpected error: {error}"
        );

        // The other slot was still written
        let other: i64 = con.hget("data:9202.1", 1000).await.unwrap();
        assert_eq!(other, 1);
    }
}
