pub mod advanced_pie;
pub mod chart;
pub mod drilldown_pie;
pub mod simple_map;
pub mod simple_pie;
pub mod single_line_chart;

use std::collections::HashMap;

use chart::ChartType;
use redis::{aio::ConnectionLike, AsyncCommands};
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

static CHART_FIELDS: (&str, &str, &str, &str, &str, &str, &str) = (
    "id", "type", "position", "title", "default", "data", "pluginId",
);

/// Find all charts with the given IDs.
///
/// Using this function is more efficient than calling `find_by_id` multiple times.
pub async fn find_by_ids<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    ids: Vec<u64>,
) -> Result<HashMap<u64, Option<Chart>>, redis::RedisError> {
    let mut pipeline = redis::pipe();
    for id in &ids {
        pipeline.hget(format!("charts:{}", id), CHART_FIELDS);
    }
    let charts: Vec<[Option<String>; 7]> = pipeline.query_async(con).await.unwrap();

    let mut result: HashMap<u64, Option<Chart>> = HashMap::new();
    for (i, values) in charts.iter().enumerate() {
        let id = ids[i];

        fn map_strings(id: u64, values: &[Option<String>; 7]) -> Option<Chart> {
            Some(Chart {
                id,
                id_custom: values[0].as_ref()?.to_string(),
                r#type: match serde_json::from_str(&format!("\"{}\"", values[1].as_ref()?)) {
                    Ok(t) => t,
                    // TODO Log warning
                    Err(_) => return None,
                },
                position: match values[2].as_ref()?.parse() {
                    Ok(p) => p,
                    Err(_) => 0,
                },
                title: values[3].as_ref()?.to_string(),
                default: values[4].as_ref().unwrap_or(&String::from("0")) == "1",
                data: match serde_json::from_str(&values[5].as_ref()?) {
                    Ok(d) => d,
                    Err(_) => Value::Null,
                },
                service_id: values[6]
                    .as_ref()?
                    .parse()
                    .expect("Chart with non-numeric 'pluginId'"),
            })
        }

        let chart = map_strings(id, values);
        result.insert(id, chart);
    }

    Ok(result)
}

pub async fn find_by_id<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    id: u64,
) -> Result<Option<Chart>, redis::RedisError> {
    find_by_ids(con, vec![id])
        .await
        .map(|mut m| m.remove(&id).unwrap())
}
