use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tracing::{debug, warn};

const HELIX_GIT_REV_PREFIX: &str = "78b999f";

pub(super) fn initialize_helix_runtime_environment() {
    if env::var_os("HELIX_RUNTIME").is_some() {
        return;
    }

    if let Some(runtime_dir) = discover_helix_runtime_dir() {
        debug!("setting HELIX_RUNTIME to {}", runtime_dir.to_string_lossy());
        // This is only called during app bootstrap, before GPUI starts worker threads.
        unsafe { env::set_var("HELIX_RUNTIME", runtime_dir) };
    } else {
        warn!("failed to discover Helix runtime directory");
    }
}

pub(super) fn ensure_helix_loader_initialized() {
    static HELIX_LOADER_INIT: OnceLock<()> = OnceLock::new();
    HELIX_LOADER_INIT.get_or_init(|| {
        helix_loader::initialize_config_file(None);
        helix_loader::initialize_log_file(None);
    });
}

pub(super) fn with_tokio_runtime<T>(f: impl FnOnce() -> T) -> T {
    static HELIX_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    let runtime = HELIX_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("helix helper runtime must build")
    });
    let guard = runtime.enter();
    let result = f();
    drop(guard);
    result
}

fn discover_helix_runtime_dir() -> Option<OsString> {
    if let Ok(current_exe) = env::current_exe()
        && let Some(runtime) = discover_bundled_helix_runtime_dir(current_exe.as_path())
    {
        return Some(runtime.into_os_string());
    }

    let workspace_runtime = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|dir| dir.join("runtime"));
    if let Some(runtime) = workspace_runtime.filter(|path| path.is_dir()) {
        return Some(runtime.into_os_string());
    }

    let cargo_home = default_cargo_home()?;
    let checkouts_dir = cargo_home.join("git").join("checkouts");
    let entries = std::fs::read_dir(checkouts_dir).ok()?;
    for entry in entries.flatten() {
        let repo_dir = entry.path();
        if !repo_dir.is_dir() {
            continue;
        }
        if !repo_dir
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("helix-"))
        {
            continue;
        }

        let preferred_runtime = repo_dir.join(HELIX_GIT_REV_PREFIX).join("runtime");
        if preferred_runtime.is_dir() {
            return Some(preferred_runtime.into_os_string());
        }

        let revisions = std::fs::read_dir(&repo_dir).ok()?;
        for revision in revisions.flatten() {
            let runtime = revision.path().join("runtime");
            if runtime.is_dir() {
                return Some(runtime.into_os_string());
            }
        }
    }
    None
}

fn discover_bundled_helix_runtime_dir(current_exe: &Path) -> Option<PathBuf> {
    let candidates = helix_runtime_dir_candidates(current_exe);
    debug!(
        current_exe = %current_exe.display(),
        candidates = ?candidates,
        "probing bundled Helix runtime directories"
    );
    candidates.into_iter().find(|candidate| candidate.is_dir())
}

fn helix_runtime_dir_candidates(current_exe: &Path) -> Vec<PathBuf> {
    let Some(exe_dir) = current_exe.parent() else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    if cfg!(target_os = "macos")
        && let Some(contents_dir) = exe_dir.parent()
        && contents_dir
            .file_name()
            .is_some_and(|name| name == "Contents")
    {
        candidates.push(contents_dir.join("Resources").join("runtime"));
    }

    candidates.push(exe_dir.join("Resources").join("runtime"));
    candidates.push(exe_dir.join("runtime"));
    candidates
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn discover_bundled_helix_runtime_dir_for_tests(current_exe: &Path) -> Option<PathBuf> {
    discover_bundled_helix_runtime_dir(current_exe)
}

fn default_cargo_home() -> Option<PathBuf> {
    env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".cargo")))
}
