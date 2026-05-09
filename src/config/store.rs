use std::fs;

use anyhow::Result;

use crate::app::paths::Paths;
use crate::model::config::Config;
use crate::model::state::State;

#[derive(Debug, Clone)]
pub struct Store {
    paths: Paths,
}

impl Store {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        fs::create_dir_all(&self.paths.config_dir)?;
        let serialized = toml::to_string_pretty(config)?;
        fs::write(&self.paths.config_file, serialized)?;
        Ok(())
    }

    pub fn load_config(&self) -> Result<Config> {
        let raw = fs::read_to_string(&self.paths.config_file)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save_state(&self, state: &State) -> Result<()> {
        fs::create_dir_all(&self.paths.state_dir)?;
        let serialized = serde_json::to_string_pretty(state)?;
        fs::write(&self.paths.state_file, serialized)?;
        Ok(())
    }
}
