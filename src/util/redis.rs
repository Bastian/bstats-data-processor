use deadpool::managed::Pool;
use deadpool_redis::cluster::{Config, Connection, Manager, Runtime};
use std::env;

pub type RedisClusterPool = Pool<Manager, Connection>;

pub fn redis_cluster_urls() -> Vec<String> {
    env::var("REDIS_CLUSTER__URLS")
        .expect("REDIS_CLUSTER__URLS is not set")
        .split(',')
        .map(String::from)
        .collect()
}

pub async fn get_redis_cluster_pool() -> RedisClusterPool {
    let cfg = Config::from_urls(redis_cluster_urls());
    cfg.create_pool(Some(Runtime::Tokio1)).unwrap()
}
