use std::path::Path;

use hunk_language::{FileMatcher, LanguageDefinition, LanguageId, LanguageRegistry};

fn rust_language() -> LanguageDefinition {
    LanguageDefinition {
        id: LanguageId::new(1),
        name: "Rust".to_string(),
        file_matcher: FileMatcher {
            extensions: vec!["rs".to_string()],
            file_names: vec!["Cargo.toml".to_string()],
        },
        grammar_name: "tree-sitter-rust".to_string(),
        highlight_query: "(identifier) @variable".to_string(),
        injection_query: None,
        locals_query: None,
    }
}

#[test]
fn registry_resolves_language_by_name_case_insensitively() {
    let mut registry = LanguageRegistry::new();
    registry.register(rust_language());

    let language = registry.language_by_name("rust").expect("language");
    assert_eq!(language.id, LanguageId::new(1));
}

#[test]
fn registry_resolves_language_by_path() {
    let mut registry = LanguageRegistry::new();
    registry.register(rust_language());

    let language = registry
        .language_for_path(Path::new("/tmp/example.rs"))
        .expect("language");
    assert_eq!(language.name, "Rust");
}
