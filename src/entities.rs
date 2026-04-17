use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ConfigCreateRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub r_type: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteConfig {
    #[serde(rename = "type")]
    pub r#type: String,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
