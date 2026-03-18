use std::path::Path;

use hunk_language::{HighlightStyleMap, LanguageId, LanguageRegistry};

#[test]
fn registry_resolves_builtin_languages_by_name_and_path() {
    let registry = LanguageRegistry::builtin();

    let rust = registry.language_by_name("rust").expect("rust language");
    assert_eq!(rust.id, LanguageId::new(1));

    let tsx = registry
        .language_for_path(Path::new("/tmp/component.tsx"))
        .expect("tsx language");
    assert_eq!(tsx.scope_name, "tsx");
}

#[test]
fn style_map_prefers_most_specific_capture_name() {
    let map = HighlightStyleMap::default();

    assert_eq!(
        map.resolve("function.method.builtin"),
        Some("function.builtin")
    );
    assert_eq!(
        map.resolve("variable.parameter"),
        Some("variable.parameter")
    );
}
