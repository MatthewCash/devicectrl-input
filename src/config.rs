use anyhow::Result;
use devicectrl_common::UpdateRequest;
use evdev::KeyCode;
use serde::{Deserialize, de};
use serde_derive::Deserialize;
use std::path::Path;
use tokio::fs;

use crate::transport::ServerConnectionConfig;

#[derive(Clone, Debug, Deserialize, Hash, PartialEq, Eq)]
pub struct InputTrigger {
    pub device_names: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_code")]
    pub key: KeyCode,
    pub value: Option<i32>,
}

fn deserialize_code<'de, D>(deserializer: D) -> Result<KeyCode, D::Error>
where
    D: de::Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(de::Error::custom)
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub server_connection: ServerConnectionConfig,
    pub actions: Vec<(InputTrigger, Vec<UpdateRequest>)>,
}

pub async fn load_config(path: &Path) -> Result<Config> {
    Ok(serde_json::from_slice(&fs::read(path).await?)?)
}
