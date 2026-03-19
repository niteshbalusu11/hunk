use std::path::Path;

use hunk_language::{HighlightStyleMap, LanguageId, LanguageRegistry};

#[test]
fn registry_resolves_builtin_languages_by_name_and_path() {
    let registry = LanguageRegistry::builtin();

    let rust = registry.language_by_name("rust").expect("rust language");
    assert_eq!(rust.id, LanguageId::new(1));
    assert!(registry.language_by_name("python").is_some());
    assert!(registry.language_by_name("powershell").is_some());
    assert!(registry.language_by_name("java").is_some());
    assert!(registry.language_by_name("csharp").is_some());
    assert!(registry.language_by_name("terraform").is_some());
    assert!(registry.language_by_name("swift").is_some());
    assert!(registry.language_by_name("kotlin").is_some());
    assert!(registry.language_by_name("nix").is_some());
    assert!(registry.language_by_name("sql").is_some());
    assert!(registry.language_by_name("dockerfile").is_some());
    assert!(registry.language_by_name("markdown").is_some());
    assert!(registry.language_by_name("markdown-inline").is_some());

    let tsx = registry
        .language_for_path(Path::new("/tmp/component.tsx"))
        .expect("tsx language");
    assert_eq!(tsx.scope_name, "tsx");
    assert!(
        registry
            .language_for_path(Path::new("/tmp/build.sh"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/build.ps1"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/tool.py"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/App.java"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/main.c"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/main.cpp"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/Program.cs"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/main.tf"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/main.swift"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/Main.kt"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/flake.nix"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/schema.sql"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/Dockerfile"))
            .is_some()
    );
    assert!(
        registry
            .language_for_path(Path::new("/tmp/README.md"))
            .is_some()
    );
}

#[test]
fn style_map_prefers_most_specific_capture_name() {
    let map = HighlightStyleMap::default();

    assert_eq!(
        map.resolve("function.builtin.static"),
        Some("function.builtin")
    );
    assert_eq!(
        map.resolve("variable.parameter"),
        Some("variable.parameter")
    );
    assert_eq!(map.resolve("function.method.builtin"), Some("function"));
}
