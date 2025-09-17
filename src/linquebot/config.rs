use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub tg: ConfigTg,
    pub ai: ConfigAi,
}

#[derive(Deserialize)]
pub struct ConfigTg {
    pub bot: ConfigTgBot,
}

#[derive(Deserialize)]
pub struct ConfigTgBot {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ConfigAi {
    pub api: ConfigAiApi,
}

#[derive(Deserialize)]
pub struct ConfigAiApi {
    pub model: String,
    pub url: String,
    pub token: String,
}

impl Config {
    pub async fn new() -> anyhow::Result<Self> {
        let config_str = fs::read_to_string("./config.toml")?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}
