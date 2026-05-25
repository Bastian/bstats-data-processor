use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Submitted data for `simple_bar` and `advanced_bar` charts: a map of category
/// to bar values (simple bars send a single-element list).
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct Bar {
    pub values: HashMap<String, Vec<i64>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_advanced_bar() {
        let bar: Bar =
            serde_json::from_value(json!({"values": {"Feature A": [0, 1], "Feature B": [1, 0]}}))
                .unwrap();
        assert_eq!(bar.values.get("Feature A"), Some(&vec![0, 1]));
        assert_eq!(bar.values.get("Feature B"), Some(&vec![1, 0]));
    }

    #[test]
    fn parses_simple_bar() {
        // SimpleBarChart wraps its single value in a one-element array.
        let bar: Bar = serde_json::from_value(json!({"values": {"Feature A": [1]}})).unwrap();
        assert_eq!(bar.values.get("Feature A"), Some(&vec![1]));
    }
}
