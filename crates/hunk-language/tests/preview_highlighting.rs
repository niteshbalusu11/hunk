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

#[test]
fn preview_highlighting_supports_phase_one_language_paths() {
    let java = preview_highlight_spans_for_path(
        Some("src/Main.java"),
        "class Main { static void main(String[] args) {} }\n",
    );
    let c = preview_highlight_spans_for_path(Some("src/main.c"), "int main(void) { return 0; }\n");
    let cpp = preview_highlight_spans_for_path(
        Some("src/main.cpp"),
        "class Widget { public: int value() const { return 1; } };\n",
    );
    let csharp = preview_highlight_spans_for_path(
        Some("src/Program.cs"),
        "class Program { static void Main() { Console.WriteLine(\"hi\"); } }\n",
    );
    let terraform = preview_highlight_spans_for_path(
        Some("infra/main.tf"),
        "resource \"aws_s3_bucket\" \"logs\" { bucket = \"demo\" }\n",
    );
    let swift = preview_highlight_spans_for_path(
        Some("Sources/App/main.swift"),
        "class App { func run() { print(\"hi\") } }\n",
    );

    assert!(
        java.iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        c.iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        cpp.iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        csharp
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        terraform
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Variable)
    );
    assert!(
        swift
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
}

#[test]
fn preview_highlighting_supports_phase_two_language_paths() {
    let kotlin = preview_highlight_spans_for_path(
        Some("app/src/main/kotlin/Main.kt"),
        "class App { fun run() { println(\"hi\") } }\n",
    );
    let nix = preview_highlight_spans_for_path(
        Some("flake.nix"),
        "{ pkgs, ... }: let name = \"hunk\"; in pkgs.mkShell { buildInputs = [ pkgs.git ]; }\n",
    );

    assert!(
        kotlin
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        kotlin
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Function)
    );
    assert!(
        nix.iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        nix.iter()
            .any(|span| span.token == PreviewSyntaxToken::Variable)
    );
}

#[test]
fn preview_highlighting_supports_phase_three_and_four_language_paths() {
    let sql = preview_highlight_spans_for_path(
        Some("db/schema.sql"),
        "SELECT users.id FROM users WHERE users.active = true;\n",
    );
    let dockerfile = preview_highlight_spans_for_path(
        Some("Dockerfile"),
        "FROM rust:1.88\nRUN cargo build --release\n",
    );
    let markdown = preview_highlight_spans_for_path(
        Some("README.md"),
        "# Hunk\n\n- fast diff viewer\n\n```rust\nfn main() {}\n```\n",
    );

    assert!(
        sql.iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        sql.iter()
            .any(|span| span.token == PreviewSyntaxToken::TypeName)
    );
    assert!(
        dockerfile
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Keyword)
    );
    assert!(
        markdown
            .iter()
            .any(|span| span.token == PreviewSyntaxToken::Operator)
    );
}
