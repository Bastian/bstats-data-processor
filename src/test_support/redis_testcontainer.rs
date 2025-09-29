use crate::util::redis::{get_redis_cluster_pool, RedisClusterPool};
use std::sync::{Arc, OnceLock};
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tokio::sync::{Mutex, OnceCell};

static SHARED_CONTAINER: OnceCell<Arc<Mutex<Option<SharedRedisContainer>>>> = OnceCell::const_new();

// Ensures we only register the atexit hook once
static REGISTERED_TEARDOWN: OnceLock<()> = OnceLock::new();

struct SharedRedisContainer {
    _container: ContainerAsync<GenericImage>,
    redis_urls: String,
    container_id: String,
}

pub struct RedisTestcontainer {
    pool: RedisClusterPool,
}

impl RedisTestcontainer {
    pub async fn new() -> Self {
        let shared = SHARED_CONTAINER
            .get_or_init(|| async { Arc::new(Mutex::new(None)) })
            .await;

        let mut guard = shared.lock().await;

        if guard.is_none() {
            let container = Self::start_cluster().await;
            let redis_urls = container.redis_urls.clone();

            // Store container so it's kept alive for the whole process
            *guard = Some(container);

            register_global_teardown();
            std::env::set_var("REDIS_CLUSTER__URLS", &redis_urls);
        }

        let redis_urls = guard.as_ref().unwrap().redis_urls.clone();
        drop(guard);

        std::env::set_var("REDIS_CLUSTER__URLS", &redis_urls);
        let pool = get_redis_cluster_pool().await;

        Self { pool }
    }

    async fn start_cluster() -> SharedRedisContainer {
        // Clean up any orphaned containers from previous test runs
        let _ = std::process::Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                "label=bstats.test=redis-cluster",
                "-q",
            ])
            .output()
            .and_then(|output| {
                let ids = String::from_utf8_lossy(&output.stdout);
                for id in ids.lines().filter(|s| !s.is_empty()) {
                    let _ = std::process::Command::new("docker")
                        .args(["rm", "-f", id])
                        .status();
                }
                Ok(())
            });

        let container = GenericImage::new("grokzen/redis-cluster", "7.0.7")
            .with_wait_for(WaitFor::message_on_stdout(
                "Running mode=cluster, port=7000",
            ))
            .with_wait_for(WaitFor::message_on_stdout(
                "Running mode=cluster, port=7001",
            ))
            .with_wait_for(WaitFor::message_on_stdout(
                "Running mode=cluster, port=7002",
            ))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connection"))
            .with_exposed_port(7000.tcp())
            .with_exposed_port(7001.tcp())
            .with_exposed_port(7002.tcp())
            .with_env_var("MASTERS", "3")
            .with_env_var("SLAVES_PER_MASTER", "0")
            .with_env_var("INITIAL_PORT", "7000")
            .with_env_var("IP", "0.0.0.0")
            .with_label("bstats.test", "redis-cluster")
            .start()
            .await
            .expect("Failed to start Redis container");

        // Capture the container ID so we can force-remove it at process exit
        let container_id = container.id().to_string();

        let redis_addr = format!(
            "redis://{}:{}/, redis://{}:{}/, redis://{}:{}/",
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(7000).await.unwrap(),
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(7001).await.unwrap(),
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(7002).await.unwrap()
        );

        // Wait for cluster to be ready
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > std::time::Duration::from_secs(30) {
                panic!("Redis cluster failed to initialize within 30 seconds");
            }

            let output = container
                .exec(ExecCommand::new([
                    "redis-cli",
                    "-c",
                    "-p",
                    "7000",
                    "cluster",
                    "info",
                ]))
                .await;

            if let Ok(mut output) = output {
                let output_vec = output.stdout_to_vec().await.unwrap_or_default();
                if let Ok(output_str) = std::str::from_utf8(&output_vec) {
                    if output_str.contains("cluster_state:ok") {
                        break;
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Configure cluster announce ports
        for port in 7000..=7002 {
            if let Ok(host_port) = container.get_host_port_ipv4(port).await {
                let _ = container
                    .exec(ExecCommand::new([
                        "redis-cli",
                        "-p",
                        &port.to_string(),
                        "CONFIG",
                        "SET",
                        "cluster-announce-port",
                        &host_port.to_string(),
                    ]))
                    .await;
            }
        }

        // Give it a moment to settle
        // Without this, the tests sometimes hang indefinitely
        // TODO: Find a better solution than sleeping
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        println!("Redis cluster ready at {}", &redis_addr);

        SharedRedisContainer {
            _container: container,
            redis_urls: redis_addr,
            container_id,
        }
    }

    pub fn pool(&self) -> &RedisClusterPool {
        &self.pool
    }

    pub async fn cleanup(&self) {
        if let Ok(mut con) = self.pool.get().await {
            let _: Result<(), _> = redis::cmd("FLUSHALL").query_async(&mut con).await;
        }
    }
}

// Common cleanup logic used by both atexit and signal handlers
fn perform_cleanup() {
    if let Some(shared) = SHARED_CONTAINER.get() {
        if let Ok(mut guard) = shared.try_lock() {
            if let Some(shared_container) = guard.take() {
                let id = shared_container.container_id.clone();

                // Prevent async Drop from running (no Tokio runtime now)
                std::mem::forget(shared_container);

                // Force remove the container; ignore errors
                let _ = std::process::Command::new("docker")
                    .args(["rm", "-f", &id])
                    .status();
            }
        }
    }
}

// Register a one-time atexit hook to tear down the shared container
fn register_global_teardown() {
    REGISTERED_TEARDOWN.get_or_init(|| {
        extern "C" fn cleanup() {
            perform_cleanup();
        }

        unsafe { libc::atexit(cleanup) };
    });
}
