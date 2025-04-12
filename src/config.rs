use std::{fs::File, io::Read, path::Path};

use anyhow::Result;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[serde_inline_default]
#[derive(Debug, Deserialize)]
pub struct MQTTConfig {
    pub hostname: String,
    #[serde_inline_default(1883)]
    pub port: u16,
    pub username: String,
    pub password: String,
    pub client_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MonitorsConfig {
    pub flood_topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PushoverConfig {
    pub user: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub mqtt: MQTTConfig,
    pub monitors: MonitorsConfig,
    pub pushover: PushoverConfig,
}

impl AppConfig {
    pub fn from_file(config_file_path: &Path) -> Result<Self> {
        let mut f = File::open(&config_file_path).map_err(|err| {
            anyhow::anyhow!(
                "Opening {path} failed: {err}",
                path = config_file_path.display()
            )
        })?;
        let mut data: String = String::new();
        f.read_to_string(&mut data)?;
        Ok(toml::from_str::<Self>(&data)?)
    }
}

impl MonitorsConfig {
    pub fn is_flood_topic(&self, topic: &str) -> bool {
        self.flood_topics.iter().any(|f| rumqttc::matches(topic, f))
    }
}
