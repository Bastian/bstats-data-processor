use std::collections::{HashMap, HashSet};

use deadpool_redis::cluster::Connection;
use redis::{AsyncCommands, ValueType};
use std::fmt;

/// A single entry in the Redis database.
#[derive(Debug, PartialEq, Hash, Ord, PartialOrd, serde::Serialize)]
pub struct RedisEntry {
    pub value_type: String,
    pub key: String,
    pub value: String,
}

/// The difference between two Redis states.
#[derive(Debug, serde::Serialize)]
pub struct RedisDiff<'a> {
    pub added: Vec<&'a RedisEntry>,
    pub removed: Vec<&'a RedisEntry>,
}

impl Eq for RedisEntry {}

impl fmt::Display for RedisEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (<{}>): {}", self.key, self.value_type, self.value)
    }
}

/// Capture the current state of the Redis database.
pub async fn capture(con: &mut Connection) -> HashSet<RedisEntry> {
    let mut state = HashSet::new();

    let keys: Vec<String> = con.keys("*").await.unwrap_or_default();

    for key in keys {
        let value_type: ValueType = con.key_type(&key).await.unwrap();
        let type_str = match &value_type {
            ValueType::String => "STRING",
            ValueType::List => "LIST",
            ValueType::Set => "SET",
            ValueType::ZSet => "ZSET",
            ValueType::Hash => "HASH",
            ValueType::Stream => "STREAM",
            ValueType::Unknown(name) => &name,
        };

        match value_type {
            ValueType::String => {
                let value: String = con.get(&key).await.unwrap_or_default();
                state.insert(RedisEntry {
                    key,
                    value: format!("'{value}'"),
                    value_type: type_str.to_string(),
                });
            }
            ValueType::List => {
                let items: Vec<String> = con.lrange(&key, 0, -1).await.unwrap_or_default();
                state.insert(RedisEntry {
                    key,
                    value: items
                        .iter()
                        .map(|i| format!("'{i}'"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    value_type: type_str.to_string(),
                });
            }
            ValueType::Set => {
                let mut members: Vec<String> = con.smembers(&key).await.unwrap_or_default();
                members.sort(); // Sort set members for consistent comparison
                state.insert(RedisEntry {
                    key,
                    value: members
                        .iter()
                        .map(|m| format!("'{m}'"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    value_type: type_str.to_string(),
                });
            }
            ValueType::ZSet => {
                let mut zset_items: Vec<(String, f64)> =
                    con.zrange_withscores(&key, 0, -1).await.unwrap_or_default();
                // Sort by member name for consistent comparison
                zset_items.sort_by(|a, b| a.0.cmp(&b.0));
                state.insert(RedisEntry {
                    key,
                    value: zset_items
                        .into_iter()
                        .map(|(member, score)| format!("('{member}', {score})"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    value_type: type_str.to_string(),
                });
            }
            ValueType::Hash => {
                let fields: HashMap<String, String> = con.hgetall(&key).await.unwrap_or_default();
                // Sort hash fields for consistent comparison
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(k, _)| *k);
                state.insert(RedisEntry {
                    key,
                    value: sorted_fields
                        .into_iter()
                        .map(|(field, value)| format!("{{'{field}': '{value}'}}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    value_type: type_str.to_string(),
                });
            }
            _ => {
                state.insert(RedisEntry {
                    key,
                    value: "Unknown".to_string(), // TODO But not important for our tests
                    value_type: type_str.to_string(),
                });
            }
        }
    }

    state
}

/// Compare two Redis states and return a human-readable diff.
///
/// Can be used for simple snapshot testing and for debugging.
///
/// Keys that are updated will show as a removal of the old value and an
/// addition of the new value.
pub fn diff<'a>(before: &'a HashSet<RedisEntry>, after: &'a HashSet<RedisEntry>) -> RedisDiff<'a> {
    let mut added: Vec<&'a RedisEntry> = after.difference(before).collect();
    let mut removed: Vec<&'a RedisEntry> = before.difference(after).collect();

    added.sort();
    removed.sort();

    RedisDiff { added, removed }
}
