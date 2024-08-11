use deadpool::managed::Pool;
use deadpool_redis::cluster::{Config, Connection, Manager, Runtime};
use std::env;

pub type RedisClusterPool = Pool<Manager, Connection>;

pub async fn get_redis_cluster_pool() -> RedisClusterPool {
    let redis_urls = env::var("REDIS_CLUSTER__URLS")
        .expect("REDIS_CLUSTER__URLS is not set")
        .split(',')
        .map(String::from)
        .collect::<Vec<_>>();
    let cfg = Config::from_urls(redis_urls);
    let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

    return pool;
}
