use std::collections::{HashMap, HashSet};
extern crate redis;
use once_cell::sync::Lazy;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::charts::chart::DefaultChartTemplate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Software {
    pub id: u16,
    pub name: String,
    pub url: String,
    pub global_plugin: Option<u32>,
    pub metrics_class: Option<String>,
    pub example_plugin: Option<String>,
    pub max_requests_per_ip: u16,
    pub default_charts: Vec<DefaultChartTemplate>,
    pub hide_in_plugin_list: bool,
}

static SOFTWARE_CACHE: Lazy<Cache<u16, Software>> = Lazy::new(|| Cache::with_ttl_minutes(300));
static SOFTWARE_IDS_CACHE: Lazy<Cache<&'static str, HashSet<u16>>> =
    Lazy::new(|| Cache::with_ttl_minutes(300));
static URL_TO_ID_CACHE: Lazy<Cache<String, u16>> = Lazy::new(|| Cache::with_ttl_minutes(300));

pub async fn find_all<C: AsyncCommands>(con: &mut C) -> Result<Vec<Software>, redis::RedisError> {
    let software_ids = find_all_software_ids(con).await?;
    let mut software = Vec::new();
    for id in software_ids {
        if let Some(s) = find_by_id(con, id).await? {
            software.push(s);
        }
    }

    software.sort_by_key(|s| s.id);

    Ok(software)
}

pub async fn find_by_url<C: AsyncCommands>(
    con: &mut C,
    url: &str,
) -> Result<Option<Software>, redis::RedisError> {
    let id = _find_software_id_by_url(con, url).await?;
    if id.is_none() {
        return Ok(None);
    }

    find_by_id(con, id.unwrap()).await
}

pub async fn find_by_id<C: AsyncCommands>(
    con: &mut C,
    id: u16,
) -> Result<Option<Software>, redis::RedisError> {
    if let Some(cached) = SOFTWARE_CACHE.get(&id).await {
        return Ok(Some(cached));
    }

    let software: HashMap<String, String> = con.hgetall(format!("software:{}", id)).await?;
    if software.is_empty() {
        return Ok(None);
    }

    let software_obj = Software {
        id,
        name: software.get("name").unwrap().to_string(),
        url: software.get("url").unwrap().to_string(),
        global_plugin: software.get("globalPlugin").map(|s| s.parse().unwrap()),
        metrics_class: software.get("metricsClass").map(|s| s.to_string()),
        example_plugin: software.get("examplePlugin").map(|s| s.to_string()),
        max_requests_per_ip: software.get("maxRequestsPerIp").unwrap().parse().unwrap(),
        hide_in_plugin_list: software
            .get("hideInPluginList")
            .unwrap_or(&String::from("0"))
            != "0",
        default_charts: serde_json::from_str(software.get("defaultCharts").unwrap()).unwrap(),
    };

    SOFTWARE_CACHE.insert(id, software_obj.clone()).await;
    Ok(Some(software_obj))
}

async fn find_all_software_ids<C: AsyncCommands>(
    con: &mut C,
) -> Result<HashSet<u16>, redis::RedisError> {
    if let Some(cached) = SOFTWARE_IDS_CACHE.get(&"all").await {
        return Ok(cached);
    }

    let ids: HashSet<u16> = con.smembers("software.ids").await?;
    SOFTWARE_IDS_CACHE.insert("all", ids.clone()).await;
    Ok(ids)
}

async fn _find_software_id_by_url<C: AsyncCommands>(
    con: &mut C,
    url: &str,
) -> Result<Option<u16>, redis::RedisError> {
    if let Some(cached_id) = URL_TO_ID_CACHE.get(&url.to_string()).await {
        return Ok(Some(cached_id));
    }

    let id: Option<u16> = con.get(format!("software.index.id.url:{}", url)).await?;

    if let Some(id) = id {
        URL_TO_ID_CACHE.insert(url.to_string(), id).await;
        Ok(Some(id))
    } else {
        Ok(None)
    }
}
