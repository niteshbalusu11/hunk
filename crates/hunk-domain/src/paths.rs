use std::fs;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

pub const HUNK_HOME_DIR_ENV_VAR: &str = "HUNK_HOME_DIR";
pub const HUNK_HOME_DIR_NAME: &str = ".hunkdiff";

pub fn hunk_home_dir() -> Result<PathBuf> {
    if let Some(override_dir) = std::env::var_os(HUNK_HOME_DIR_ENV_VAR) {
        return Ok(canonicalize_if_exists(PathBuf::from(override_dir)));
    }

    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("failed to resolve home directory"))?;
    Ok(canonicalize_if_exists(home_dir).join(HUNK_HOME_DIR_NAME))
}

fn canonicalize_if_exists(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    fs::canonicalize(path.as_path()).unwrap_or(path)
}
