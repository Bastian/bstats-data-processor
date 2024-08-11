use data_processor::util::redis::{get_redis_cluster_pool, RedisClusterPool};
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

pub struct RedisTestcontainer {
    pool: RedisClusterPool,
    // Bind the container to the struct to keep it alive
    _container: ContainerAsync<GenericImage>,
}

impl RedisTestcontainer {
    pub async fn new() -> Self {
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
            .start()
            .await
            .expect("Failed to start Redis container");

        let redis_addr = format!(
            "redis://{}:{}/, redis://{}:{}/, redis://{}:{}/",
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(7000).await.unwrap(),
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(7001).await.unwrap(),
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(7002).await.unwrap()
        );

        loop {
            let mut output = container
                .exec(ExecCommand::new([
                    "redis-cli",
                    "-c",
                    "-p",
                    "7000",
                    "cluster",
                    "info",
                ]))
                .await
                .expect("Failed to get cluster info");

            let output_vec = output.stdout_to_vec().await.expect("Failed to get stdout");

            let output = match std::str::from_utf8(&output_vec) {
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };

            if output.contains("cluster_state:ok") {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        for port in 7000..=7002 {
            container
                .exec(ExecCommand::new([
                    "redis-cli",
                    "-p",
                    port.to_string().as_str(),
                    "CONFIG",
                    "SET",
                    "cluster-announce-port",
                    container
                        .get_host_port_ipv4(port)
                        .await
                        .unwrap()
                        .to_string()
                        .as_str(),
                ]))
                .await
                .expect("Failed to set cluster-announce-port");
        }

        std::env::set_var("REDIS_CLUSTER__URLS", &redis_addr);

        println!("Redis container started at {}", &redis_addr);

        let pool = get_redis_cluster_pool().await;

        Self {
            pool,
            _container: container,
        }
    }

    pub fn pool(&self) -> &RedisClusterPool {
        &self.pool
    }
}
