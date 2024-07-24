use std::collections::{HashMap, HashSet};
extern crate redis;
use redis::{aio::ConnectionLike, AsyncCommands};

use crate::charts::chart::DefaultChartTemplate;

#[derive(Debug, Clone)]
pub struct Software {
    pub id: i16,
    pub name: String,
    pub url: String,
    pub global_plugin: Option<i64>,
    pub metrics_class: Option<String>,
    pub example_plugin: Option<String>,
    pub max_requests_per_ip: i32,
    pub default_charts: Vec<DefaultChartTemplate>,
    pub hide_in_plugin_list: bool,
}

pub async fn find_all<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
) -> Result<Vec<Software>, redis::RedisError> {
    // TODO: Cache result since it hardly ever changes
    let software_ids = find_all_software_ids(con).await?;
    let mut software = Vec::new();
    for id in software_ids {
        if let Some(s) = find_by_id(con, id).await? {
            software.push(s);
        }
    }

    Ok(software)
}

pub async fn find_by_id<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    id: i16,
) -> Result<Option<Software>, redis::RedisError> {
    // TODO: Cache result since it hardly ever changes
    let software: HashMap<String, String> = con.hgetall(format!("software:{}", id)).await?;
    if software.is_empty() {
        return Ok(None);
    }

    Ok(Some(Software {
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
    }))
}

async fn find_all_software_ids<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
) -> Result<HashSet<i16>, redis::RedisError> {
    con.smembers("software.ids").await
}

async fn _find_software_id_by_url<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    url: &str,
) -> Result<Option<i16>, redis::RedisError> {
    con.get(format!("software.index.id.url:{}", url)).await
}
