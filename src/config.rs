use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};

const CONFIG_DIR_NAME: &str = ".hunkdiff";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffViewMode {
    Fit,
    #[default]
    Pan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: ThemePreference,
    pub diff_view: DiffViewMode,
    pub show_whitespace: bool,
    pub show_eol_markers: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow!("failed to resolve home directory"))?;
        let path = home_dir.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME);
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_create_default(&self) -> Result<AppConfig> {
        if !self.path.exists() {
            let config = AppConfig::default();
            self.save(&config)?;
            return Ok(config);
        }

        let raw = fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read config file at {}", self.path.display()))?;
        toml::from_str::<AppConfig>(&raw).with_context(|| {
            format!(
                "failed to parse TOML config file at {}",
                self.path.display()
            )
        })
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| anyhow!("config path has no parent: {}", self.path.display()))?;

        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;

        let contents =
            toml::to_string_pretty(config).context("failed to serialize app config to TOML")?;
        fs::write(&self.path, contents)
            .with_context(|| format!("failed to write config file at {}", self.path.display()))?;
        Ok(())
    }
}
