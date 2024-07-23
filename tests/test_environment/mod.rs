use crate::bound_redis_client::BoundRedisClient;
use data_processor::software::Software;
use redis::AsyncCommands;

pub struct TestEnvironment {
    redis_client: BoundRedisClient,
    software: Vec<Software>,
}

impl TestEnvironment {
    pub async fn empty() -> Self {
        let redis_client: BoundRedisClient = BoundRedisClient::new().await;
        Self {
            redis_client,
            software: Vec::new(),
        }
    }

    pub async fn with_data() -> Self {
        let mut environment: TestEnvironment = Self::empty().await;

        environment.add_software(Software {
            id: 1,
            name: String::from("Bukkit / Spigot"),
            url: String::from("bukkit"),
            global_plugin: Some(1),
            metrics_class: Some(String::from("https://raw.githubusercontent.com/Bastian/bstats-metrics/single-file/bukkit/Metrics.java")),
            example_plugin: Some(String::from("https://github.com/Bastian/bstats-metrics/blob/1.x.x/bstats-bukkit/src/examples/java/ExamplePlugin.java")),
            max_requests_per_ip: 10,
            hide_in_plugin_list: false,
        }).await;

        environment.add_software(Software {
            id: 2,
            name: String::from("Bungeecord"),
            url: String::from("bungeecord"),
            global_plugin: Some(2),
            metrics_class: Some(String::from("https://raw.githubusercontent.com/Bastian/bstats-metrics/single-file/bungeecord/Metrics.java")),
            example_plugin: Some(String::from("https://github.com/Bastian/bstats-metrics/blob/1.x.x/bstats-bungeecord/src/examples/java/ExamplePlugin.java")),
            max_requests_per_ip: 10,
            hide_in_plugin_list: false,
        }).await;

        environment
    }

    pub async fn add_software(&mut self, software: Software) {
        let mut con: redis::aio::MultiplexedConnection = self.redis_multiplexed_connection().await;

        let _: () = con.sadd("software.ids", software.id).await.unwrap();
        let _: () = con
            .set(
                format!("software.index.id.url:{}", software.url),
                software.id,
            )
            .await
            .unwrap();

        let cloned_software = software.clone();

        let _: () = con
            .hset_multiple(
                format!("software:{}", software.id),
                &vec![
                    ("name", software.name),
                    ("url", software.url),
                    ("globalPlugin", software.global_plugin.unwrap().to_string()),
                    ("metricsClass", software.metrics_class.unwrap().to_string()),
                    (
                        "examplePlugin",
                        software.example_plugin.unwrap().to_string(),
                    ),
                    ("maxRequestsPerIp", software.max_requests_per_ip.to_string()),
                    ("hideInPluginList", software.hide_in_plugin_list.to_string()),
                ],
            )
            .await
            .unwrap();
        self.software.push(cloned_software);
    }

    pub fn redis_client(&self) -> &redis::Client {
        &self.redis_client.client()
    }

    pub fn software(&self) -> &Vec<Software> {
        &self.software
    }

    pub async fn redis_multiplexed_connection(&self) -> redis::aio::MultiplexedConnection {
        self.redis_client()
            .get_multiplexed_tokio_connection()
            .await
            .unwrap()
    }
}
