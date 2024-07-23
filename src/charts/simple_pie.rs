use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct SimplePie {
    #[validate(length(min = 1))]
    pub value: String,
}
