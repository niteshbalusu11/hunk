use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk_jj::jj::{checkout_or_create_bookmark, commit_staged, resolve_workspace_switch_target};

#[test]
fn resolve_workspace_switch_target_returns_secondary_workspace_details() {
    let fixture = TempRepo::new("workspace-switch-clean");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_bookmark(fixture.path(), "main").expect("main bookmark should be created");

    let secondary_workspace_path = fixture.path().with_file_name(format!(
        "{}-ws2",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));
    let secondary_workspace_path_string = secondary_workspace_path.to_string_lossy().to_string();
    run_jj(
        fixture.path(),
        [
            "workspace",
            "add",
            secondary_workspace_path_string.as_str(),
            "--name",
            "ws2",
            "-r",
            "@",
        ],
    );

    let target = resolve_workspace_switch_target(fixture.path(), "ws2")
        .expect("workspace switch target should resolve");
    let expected_root = fs::canonicalize(&secondary_workspace_path)
        .expect("secondary workspace path should canonicalize");
    let resolved_root =
        fs::canonicalize(&target.root).expect("resolved workspace root should canonicalize");

    assert_eq!(target.name, "ws2");
    assert_eq!(resolved_root, expected_root);
    assert!(
        !target.is_current,
        "secondary workspace should not be marked current from default workspace"
    );
    assert!(
        !target.commit_id.is_empty(),
        "workspace switch target should include commit id"
    );

    let _ = fs::remove_dir_all(&secondary_workspace_path);
}

#[test]
fn resolve_workspace_switch_target_rejects_missing_workspace() {
    let fixture = TempRepo::new("workspace-switch-missing");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let err = resolve_workspace_switch_target(fixture.path(), "missing-ws")
        .expect_err("missing workspace should fail");
    let message = err.to_string();
    assert!(
        message.contains("does not exist in this repository view"),
        "unexpected error message: {message}"
    );
}

#[test]
fn resolve_workspace_switch_target_rejects_stale_workspace_path() {
    let fixture = TempRepo::new("workspace-switch-stale");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");
    checkout_or_create_bookmark(fixture.path(), "main").expect("main bookmark should be created");

    let secondary_workspace_path = fixture.path().with_file_name(format!(
        "{}-ws2",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));
    let secondary_workspace_path_string = secondary_workspace_path.to_string_lossy().to_string();
    run_jj(
        fixture.path(),
        [
            "workspace",
            "add",
            secondary_workspace_path_string.as_str(),
            "--name",
            "ws2",
            "-r",
            "@",
        ],
    );

    fs::remove_dir_all(&secondary_workspace_path)
        .expect("secondary workspace path should be removable");

    let err = resolve_workspace_switch_target(fixture.path(), "ws2")
        .expect_err("stale workspace root should fail");
    let message = err.to_string();
    assert!(
        message.contains("no accessible root path"),
        "unexpected stale workspace error: {message}"
    );
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hunk-{prefix}-{unique}"));
        fs::create_dir_all(&path).expect("temp repo directory should be created");

        run_jj(&path, ["git", "init", "--colocate"]);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_jj<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("jj")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run jj command");
    if !output.status.success() {
        panic!(
            "jj command failed (status {}):\nstdout:\n{}\nstderr:\n{}",
            output
                .status
                .code()
                .map_or_else(|| "signal".to_string(), |code| code.to_string()),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn write_file(path: PathBuf, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory should exist");
    }
    fs::write(path, contents).expect("file should be written");
}
