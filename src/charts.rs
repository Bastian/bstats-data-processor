pub mod advanced_pie;
pub mod chart;
pub mod drilldown_pie;
pub mod simple_map;
pub mod simple_pie;
pub mod single_line_chart;

use std::collections::HashMap;

use crate::cache::Cache;
use chart::ChartType;
use once_cell::sync::Lazy;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub id: u64,
    pub id_custom: String,
    pub r#type: ChartType,
    pub position: u16,
    pub title: String,
    pub default: bool,
    pub data: Value,
    pub service_id: u32,
}

static CACHE: Lazy<Cache<u64, Chart>> = Lazy::new(|| Cache::with_ttl_minutes(60));

/// Find all charts with the given IDs.
pub async fn find_by_ids<C: AsyncCommands>(
    con: &mut C,
    ids: Vec<u64>,
) -> Result<HashMap<u64, Option<Chart>>, redis::RedisError> {
    // TODO: Move all charts from a single service in a single hash and use pipelining
    let mut response = HashMap::new();
    for id in ids {
        response.insert(id, find_by_id(con, id).await?);
    }
    Ok(response)
}

pub async fn find_by_id<C: AsyncCommands>(
    con: &mut C,
    id: u64,
) -> Result<Option<Chart>, redis::RedisError> {
    if let Some(cached) = CACHE.get(&id).await {
        return Ok(Some(cached));
    }

    let map: HashMap<String, String> = con.hgetall(format!("charts:{}", id)).await?;

    if map.is_empty() {
        return Ok(None);
    }

    let chart = Chart {
        id,
        id_custom: map
            .get("id")
            .expect("Chart without 'id_custom'")
            .to_string(),
        r#type: match serde_json::from_str(&format!(
            "\"{}\"",
            map.get("type").expect("Chart without 'type'")
        )) {
            Ok(t) => t,
            // TODO Log warning
            Err(_) => return Ok(None),
        },
        position: map
            .get("position")
            .expect("Chart without 'position'")
            .parse()
            .expect("Chart with non-numeric or to small/large 'position"),
        title: map.get("title").expect("Chart without 'title'").to_string(),
        default: map.get("default").unwrap_or(&String::from("0")) == "1",
        data: match serde_json::from_str(map.get("data").expect("Chart without 'data'")) {
            Ok(d) => d,
            Err(_) => Value::Null,
        },
        service_id: map
            .get("pluginId")
            .expect("Chart without 'pluginId'")
            .parse()
            .expect("Chart with non-numeric 'pluginId"),
    };

    CACHE.insert(id, chart.clone()).await;
    Ok(Some(chart))
}
