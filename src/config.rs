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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardShortcuts {
    pub select_next_line: Vec<String>,
    pub select_previous_line: Vec<String>,
    pub extend_selection_next_line: Vec<String>,
    pub extend_selection_previous_line: Vec<String>,
    pub copy_selection: Vec<String>,
    pub select_all_diff_rows: Vec<String>,
    pub next_hunk: Vec<String>,
    pub previous_hunk: Vec<String>,
    pub next_file: Vec<String>,
    pub previous_file: Vec<String>,
    pub open_project: Vec<String>,
    pub save_current_file: Vec<String>,
    pub open_settings: Vec<String>,
    pub quit_app: Vec<String>,
}

impl Default for KeyboardShortcuts {
    fn default() -> Self {
        Self {
            select_next_line: vec!["down".into()],
            select_previous_line: vec!["up".into()],
            extend_selection_next_line: vec!["shift-down".into()],
            extend_selection_previous_line: vec!["shift-up".into()],
            copy_selection: vec!["cmd-c".into(), "ctrl-c".into()],
            select_all_diff_rows: vec!["cmd-a".into(), "ctrl-a".into()],
            next_hunk: vec!["f7".into()],
            previous_hunk: vec!["shift-f7".into()],
            next_file: vec!["alt-down".into()],
            previous_file: vec!["alt-up".into()],
            open_project: vec!["cmd-shift-o".into(), "ctrl-shift-o".into()],
            save_current_file: vec!["cmd-s".into(), "ctrl-s".into()],
            open_settings: vec!["cmd-,".into(), "ctrl-,".into()],
            quit_app: vec!["cmd-q".into()],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: ThemePreference,
    pub show_whitespace: bool,
    pub show_eol_markers: bool,
    pub keyboard_shortcuts: KeyboardShortcuts,
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
