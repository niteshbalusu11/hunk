#[allow(dead_code)]
#[path = "../src/app/files_editor/runtime_env.rs"]
mod runtime_env;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn packaged_macos_bundle_resolves_runtime_from_contents_resources() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("hunk-helix-runtime-{unique}"));
    let exe_path = root
        .join("Hunk.app")
        .join("Contents")
        .join("MacOS")
        .join("hunk_desktop");
    let bundled_runtime = root
        .join("Hunk.app")
        .join("Contents")
        .join("Resources")
        .join("runtime");

    fs::create_dir_all(
        exe_path
            .parent()
            .expect("macOS bundle executable should have a parent"),
    )
    .expect("bundle executable parent should exist");
    fs::create_dir_all(&bundled_runtime).expect("bundled runtime should exist");

    let resolved = runtime_env::discover_bundled_helix_runtime_dir_for_tests(exe_path.as_path());

    assert_eq!(resolved, Some(PathBuf::from(&bundled_runtime)));

    fs::remove_dir_all(root).expect("temporary bundle fixture should be cleaned up");
}

#[test]
fn packaged_binary_resolves_runtime_from_sibling_resources_dir() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("hunk-helix-runtime-resources-{unique}"));
    let exe_path = root.join("bin").join("hunk_desktop");
    let bundled_runtime = root.join("bin").join("Resources").join("runtime");

    fs::create_dir_all(exe_path.parent().expect("binary should have a parent"))
        .expect("binary parent should exist");
    fs::create_dir_all(&bundled_runtime).expect("bundled runtime should exist");

    let resolved = runtime_env::discover_bundled_helix_runtime_dir_for_tests(exe_path.as_path());

    assert_eq!(resolved, Some(PathBuf::from(&bundled_runtime)));

    fs::remove_dir_all(root).expect("temporary resource fixture should be cleaned up");
}

#[test]
fn packaged_binary_resolves_runtime_from_sibling_runtime_dir() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("hunk-helix-runtime-sibling-{unique}"));
    let exe_path = root.join("bin").join("hunk_desktop");
    let bundled_runtime = root.join("bin").join("runtime");

    fs::create_dir_all(exe_path.parent().expect("binary should have a parent"))
        .expect("binary parent should exist");
    fs::create_dir_all(&bundled_runtime).expect("bundled runtime should exist");

    let resolved = runtime_env::discover_bundled_helix_runtime_dir_for_tests(exe_path.as_path());

    assert_eq!(resolved, Some(PathBuf::from(&bundled_runtime)));

    fs::remove_dir_all(root).expect("temporary runtime fixture should be cleaned up");
}
