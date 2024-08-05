use serde_json::Value;

use crate::submit_data_schema::SubmitDataSchema;

use super::Parser;

pub struct NameInRequestParser {
    pub name_in_request: String,
    pub position: String,
}

impl Parser for NameInRequestParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        if self.position == "global" {
            return schema.extra.get(&self.name_in_request).cloned();
        }

        if self.position == "plugin" {
            return schema.service.extra.get(&self.name_in_request).cloned();
        }

        return None;
    }
}
