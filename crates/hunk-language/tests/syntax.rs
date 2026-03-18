use std::path::Path;

use hunk_language::{LanguageRegistry, ParseStatus, SyntaxSession};

#[test]
fn rust_source_parses_and_highlights_keywords() {
    let registry = LanguageRegistry::builtin();
    let mut session = SyntaxSession::new();
    let source = "fn main() {\n    let answer = 42;\n}\n";

    let snapshot = session
        .parse_for_path(&registry, Path::new("main.rs"), source)
        .expect("parse");
    assert_eq!(snapshot.parse_status, ParseStatus::Ready);

    let captures = session
        .highlight_visible_range(&registry, source, 0..source.len())
        .expect("highlights");
    assert!(
        captures
            .iter()
            .any(|capture| capture.style_key == "keyword")
    );
    assert!(
        captures
            .iter()
            .any(|capture| { capture.style_key == "function" || capture.style_key == "variable" })
    );
}

#[test]
fn html_injection_highlights_embedded_javascript_and_css() {
    let registry = LanguageRegistry::builtin();
    let mut session = SyntaxSession::new();
    let source = "<html><body><script>const answer = 42;</script><style>.card { color: red; }</style></body></html>";

    session
        .parse_for_path(&registry, Path::new("index.html"), source)
        .expect("parse html");
    let captures = session
        .highlight_visible_range(&registry, source, 0..source.len())
        .expect("html highlights");

    let const_offset = source.find("const").expect("const");
    let color_offset = source.find("color").expect("color");
    assert!(captures.iter().any(|capture| {
        capture.style_key == "keyword"
            && capture.byte_range.start <= const_offset
            && capture.byte_range.end >= const_offset + "const".len()
    }));
    assert!(captures.iter().any(|capture| {
        capture.style_key == "property"
            && capture.byte_range.start <= color_offset
            && capture.byte_range.end >= color_offset + "color".len()
    }));
}

#[test]
fn fold_candidates_cover_multiline_rust_blocks() {
    let registry = LanguageRegistry::builtin();
    let mut session = SyntaxSession::new();
    let source = "fn main() {\n    if true {\n        println!(\"hi\");\n    }\n}\n";

    session
        .parse_for_path(&registry, Path::new("main.rs"), source)
        .expect("parse");
    let folds = session.fold_candidates(&registry, source);

    assert!(
        folds
            .iter()
            .any(|fold| fold.start_line == 0 && fold.end_line >= 4)
    );
    assert!(
        folds
            .iter()
            .any(|fold| fold.start_line == 1 && fold.end_line >= 3)
    );
}
