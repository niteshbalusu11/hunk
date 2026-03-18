use hunk_language::{
    PreviewSyntaxToken, preview_highlight_spans_for_language_hint, preview_highlight_spans_for_path,
};

#[test]
fn preview_highlighting_resolves_rust_from_language_hint() {
    let spans = preview_highlight_spans_for_language_hint(
        Some("rust"),
        "fn main() {\n    let answer = 42;\n}\n",
    );

    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
}

#[test]
fn preview_highlighting_resolves_typescript_from_extension_hint() {
    let spans = preview_highlight_spans_for_language_hint(
        Some("ts"),
        "const answer = parseBIP321(\"bitcoin:addr\");",
    );

    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Function)
    );
}

#[test]
fn preview_highlighting_supports_toml_paths() {
    let spans =
        preview_highlight_spans_for_path(Some("Cargo.toml"), "name = \"hunk\" # application name");

    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::String)
    );
    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Comment)
    );
}

#[test]
fn preview_highlighting_supports_python_and_bash_hints() {
    let python =
        preview_highlight_spans_for_language_hint(Some("python"), "def main():\n    return 42\n");
    let bash = preview_highlight_spans_for_language_hint(
        Some("bash"),
        "if [ -n \"$HOME\" ]; then\necho ok\nfi\n",
    );

    assert!(
        python
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        bash.iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
}

#[test]
fn preview_highlighting_supports_powershell_paths() {
    let spans = preview_highlight_spans_for_path(
        Some("scripts/build.ps1"),
        "function Invoke-Build { Write-Host \"hi\" }\n",
    );

    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        spans
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Function)
    );
}
