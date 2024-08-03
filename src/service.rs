use std::collections::{HashMap, HashSet};
extern crate redis;
use redis::{aio::ConnectionLike, AsyncCommands};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: i32,
    pub name: String,
    pub owner: String,
    pub software_id: i16,
    pub global: bool,
    pub charts: Vec<i64>,
}

pub async fn find_all<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
) -> Result<Vec<Service>, redis::RedisError> {
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

pub async fn find_by_software_url_and_name<C: ConnectionLike + AsyncCommands>(
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

pub async fn find_by_id<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    id: i32,
) -> Result<Option<Service>, redis::RedisError> {
    let service: HashMap<String, String> = con.hgetall(format!("plugins:{}", id)).await?;
    if service.is_empty() {
        return Ok(None);
    }

    Ok(Some(Service {
        id,
        name: service.get("name").unwrap().to_string(),
        owner: service.get("owner").unwrap().to_string(),
        software_id: service.get("software").unwrap().parse().unwrap(),
        global: service.get("global").unwrap_or(&String::from("0")) != "0",
        charts: serde_json::from_str(service.get("charts").unwrap()).unwrap(),
    }))
}

async fn find_all_service_ids<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
) -> Result<HashSet<i32>, redis::RedisError> {
    con.smembers("plugins.ids").await
}

async fn _find_service_id_by_software_url_and_name<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    software_url: &str,
    name: &str,
) -> Result<Option<i32>, redis::RedisError> {
    con.get(format!(
        "plugins.index.id.url+name:{}.{}",
        software_url,
        name.to_ascii_lowercase()
    ))
    .await
}
