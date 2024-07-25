use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};

pub struct RedisTestcontainer {
    client: redis::Client,
    // Bind the container to the struct to keep it alive
    _container: ContainerAsync<GenericImage>,
}

impl RedisTestcontainer {
    pub async fn new() -> Self {
        let container = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(6379.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .unwrap();

        let redis_addr = format!(
            "redis://{}:{}/",
            container.get_host().await.unwrap(),
            container.get_host_port_ipv4(6379).await.unwrap()
        );

        let client = redis::Client::open(redis_addr).unwrap();

        Self {
            client,
            _container: container,
        }
    }

    pub fn client(&self) -> &redis::Client {
        &self.client
    }
}
