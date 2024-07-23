use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct DrilldownPie {
    pub values: HashMap<String, HashMap<String, i32>>,
}
