use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct SingleLineChart {
    pub value: i16,
}

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct SingleLineChartFilter {
    pub enabled: bool,
    #[serde(rename = "maxValue")]
    pub max_value: Option<i64>,
    #[serde(rename = "minValue")]
    pub min_value: Option<i64>,
}

impl SingleLineChartFilter {
    pub fn should_block(&self, data: &SingleLineChart) -> bool {
        if self.enabled {
            if let Some(max_value) = self.max_value {
                if i64::from(data.value) > max_value {
                    return true;
                }
            }
            if let Some(min_value) = self.min_value {
                if i64::from(data.value) < min_value {
                    return true;
                }
            }
        }
        return false;
    }
}
