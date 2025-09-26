use std::collections::{HashMap, HashSet};
extern crate redis;
use once_cell::sync::Lazy;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::cache::Cache;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: u32,
    pub name: String,
    pub owner: String,
    pub software_id: u16,
    pub global: bool,
    pub charts: Vec<u64>,
}

static SERVICE_CACHE: Lazy<Cache<u32, Service>> = Lazy::new(|| Cache::with_ttl_minutes(30));
static SERVICE_INDEX_CACHE: Lazy<Cache<String, u32>> = Lazy::new(|| Cache::with_ttl_minutes(30));

pub async fn find_all<C: AsyncCommands>(con: &mut C) -> Result<Vec<Service>, redis::RedisError> {
    let service_ids = find_all_service_ids(con).await?;
    let mut services: Vec<_> = Vec::new();
    for id in service_ids {
        if let Some(s) = find_by_id(con, id).await? {
            services.push(s);
        }
    }

    services.sort_by_key(|s| s.id);

    Ok(services)
}

pub async fn find_by_software_url_and_name<C: AsyncCommands>(
    con: &mut C,
    software_url: &str,
    name: &str,
) -> Result<Option<Service>, redis::RedisError> {
    let id = _find_service_id_by_software_url_and_name(con, software_url, name).await?;
    if id.is_none() {
        return Ok(None);
    }

    find_by_id(con, id.unwrap()).await
}

pub async fn find_by_id<C: AsyncCommands>(
    con: &mut C,
    id: u32,
) -> Result<Option<Service>, redis::RedisError> {
    if let Some(cached) = SERVICE_CACHE.get(&id).await {
        return Ok(Some(cached));
    }

    let service: HashMap<String, String> = con.hgetall(format!("plugins:{}", id)).await?;
    if service.is_empty() {
        return Ok(None);
    }

    let service_obj = Service {
        id,
        name: service.get("name").unwrap().to_string(),
        owner: service.get("owner").unwrap().to_string(),
        software_id: service.get("software").unwrap().parse().unwrap(),
        global: service.get("global").unwrap_or(&String::from("0")) != "0",
        charts: serde_json::from_str(service.get("charts").unwrap()).unwrap(),
    };

    SERVICE_CACHE.insert(id, service_obj.clone()).await;
    Ok(Some(service_obj))
}

async fn find_all_service_ids<C: AsyncCommands>(
    con: &mut C,
) -> Result<HashSet<u32>, redis::RedisError> {
    con.smembers("plugins.ids").await
}

async fn _find_service_id_by_software_url_and_name<C: AsyncCommands>(
    con: &mut C,
    software_url: &str,
    name: &str,
) -> Result<Option<u32>, redis::RedisError> {
    let key = format!(
        "plugins.index.id.url+name:{}.{}",
        software_url,
        name.to_ascii_lowercase()
    );

    if let Some(cached_id) = SERVICE_INDEX_CACHE.get(&key).await {
        return Ok(Some(cached_id));
    }

    let id: Option<u32> = con.get(&key).await?;

    if let Some(id) = id {
        SERVICE_INDEX_CACHE.insert(key, id).await;
        Ok(Some(id))
    } else {
        Ok(None)
    }
}
