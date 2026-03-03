use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hunk_jj::jj::{
    GraphSnapshotOptions, commit_staged, create_workspace_at_revision, forget_workspace,
    load_graph_snapshot, resolve_workspace_switch_target,
};

#[test]
fn create_workspace_at_revision_registers_workspace_and_target_path() {
    let fixture = TempRepo::new("workspace-create");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let target_revision_id = latest_graph_revision_id(fixture.path());
    let workspace_path = fixture.path().with_file_name(format!(
        "{}-ws2",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));

    let created = create_workspace_at_revision(
        fixture.path(),
        "ws2",
        target_revision_id.as_str(),
        workspace_path.as_path(),
    )
    .expect("workspace should be created");

    let switch_target =
        resolve_workspace_switch_target(fixture.path(), "ws2").expect("workspace should resolve");
    let snapshot = load_graph_snapshot(fixture.path(), GraphSnapshotOptions::default())
        .expect("snapshot loads");

    assert_eq!(created.name, "ws2");
    assert_eq!(switch_target.name, "ws2");
    assert_eq!(
        fs::canonicalize(created.root).expect("created root canonicalizes"),
        fs::canonicalize(&workspace_path).expect("workspace path canonicalizes")
    );
    assert!(
        snapshot
            .workspaces
            .iter()
            .any(|workspace| workspace.name == "ws2"),
        "workspace should appear in graph workspace summary"
    );

    let _ = fs::remove_dir_all(&workspace_path);
}

#[test]
fn create_workspace_at_revision_rejects_duplicate_workspace_name() {
    let fixture = TempRepo::new("workspace-create-duplicate");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let target_revision_id = latest_graph_revision_id(fixture.path());
    let workspace_path_a = fixture.path().with_file_name(format!(
        "{}-ws-a",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));
    let workspace_path_b = fixture.path().with_file_name(format!(
        "{}-ws-b",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));

    create_workspace_at_revision(
        fixture.path(),
        "ws2",
        target_revision_id.as_str(),
        workspace_path_a.as_path(),
    )
    .expect("first workspace creation should succeed");

    let err = create_workspace_at_revision(
        fixture.path(),
        "ws2",
        target_revision_id.as_str(),
        workspace_path_b.as_path(),
    )
    .expect_err("duplicate workspace name should fail");
    let message = err.to_string();
    assert!(
        message.contains("already exists"),
        "unexpected duplicate error: {message}"
    );

    let _ = fs::remove_dir_all(&workspace_path_a);
    let _ = fs::remove_dir_all(&workspace_path_b);
}

#[test]
fn create_workspace_at_revision_rejects_non_empty_destination_without_registering_workspace() {
    let fixture = TempRepo::new("workspace-create-non-empty");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let target_revision_id = latest_graph_revision_id(fixture.path());
    let workspace_path = fixture.path().with_file_name(format!(
        "{}-ws2",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));
    fs::create_dir_all(&workspace_path).expect("workspace destination dir should be created");
    write_file(workspace_path.join("keep.txt"), "keep\n");

    let err = create_workspace_at_revision(
        fixture.path(),
        "ws2",
        target_revision_id.as_str(),
        workspace_path.as_path(),
    )
    .expect_err("non-empty destination should fail");
    let message = err.to_string();
    assert!(
        message.contains("not empty"),
        "unexpected non-empty destination error: {message}"
    );

    let missing_err = resolve_workspace_switch_target(fixture.path(), "ws2")
        .expect_err("workspace should not be registered on failed create");
    assert!(
        missing_err
            .to_string()
            .contains("does not exist in this repository view"),
        "workspace should remain unregistered after create failure"
    );

    let _ = fs::remove_dir_all(&workspace_path);
}

#[test]
fn forget_workspace_removes_non_current_workspace_but_keeps_directory() {
    let fixture = TempRepo::new("workspace-forget");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let target_revision_id = latest_graph_revision_id(fixture.path());
    let workspace_path = fixture.path().with_file_name(format!(
        "{}-ws2",
        fixture
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
    ));

    create_workspace_at_revision(
        fixture.path(),
        "ws2",
        target_revision_id.as_str(),
        workspace_path.as_path(),
    )
    .expect("workspace should be created");

    forget_workspace(fixture.path(), "ws2").expect("forget workspace should succeed");

    let err = resolve_workspace_switch_target(fixture.path(), "ws2")
        .expect_err("forgotten workspace should not resolve");
    assert!(
        err.to_string()
            .contains("does not exist in this repository view"),
        "unexpected forget error: {err:#}"
    );
    assert!(
        workspace_path.is_dir(),
        "forget should not delete workspace directory from disk"
    );

    let snapshot = load_graph_snapshot(fixture.path(), GraphSnapshotOptions::default())
        .expect("snapshot loads");
    assert!(
        snapshot
            .workspaces
            .iter()
            .all(|workspace| workspace.name != "ws2"),
        "forgotten workspace should be removed from graph summary"
    );

    let _ = fs::remove_dir_all(&workspace_path);
}

#[test]
fn forget_workspace_rejects_current_workspace() {
    let fixture = TempRepo::new("workspace-forget-current");

    write_file(fixture.path().join("tracked.txt"), "line one\n");
    commit_staged(fixture.path(), "initial commit").expect("initial commit should succeed");

    let snapshot = load_graph_snapshot(fixture.path(), GraphSnapshotOptions::default())
        .expect("snapshot loads");
    let err = forget_workspace(fixture.path(), snapshot.current_workspace_name.as_str())
        .expect_err("forget current workspace should fail");
    let message = err.to_string();
    assert!(
        message.contains("cannot forget current workspace"),
        "unexpected current-workspace forget error: {message}"
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

fn latest_graph_revision_id(repo_root: &Path) -> String {
    load_graph_snapshot(repo_root, GraphSnapshotOptions::default())
        .expect("snapshot loads")
        .nodes
        .first()
        .expect("graph should have at least one node")
        .id
        .clone()
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
