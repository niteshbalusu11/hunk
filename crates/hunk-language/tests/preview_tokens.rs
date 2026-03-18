use hunk_language::PreviewSyntaxToken;

#[test]
fn preview_syntax_token_maps_representative_captures() {
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("keyword"),
        PreviewSyntaxToken::Keyword
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("string.escape"),
        PreviewSyntaxToken::String
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("number"),
        PreviewSyntaxToken::Number
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("function.builtin"),
        PreviewSyntaxToken::Function
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("type"),
        PreviewSyntaxToken::TypeName
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("constant.builtin"),
        PreviewSyntaxToken::Constant
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("variable.parameter"),
        PreviewSyntaxToken::Variable
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("punctuation.bracket"),
        PreviewSyntaxToken::Operator
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("comment.documentation"),
        PreviewSyntaxToken::Comment
    );
}

#[test]
fn preview_syntax_token_defaults_to_plain_for_unknown_captures() {
    assert_eq!(
        PreviewSyntaxToken::from_capture_name("markup.heading"),
        PreviewSyntaxToken::Plain
    );
    assert_eq!(
        PreviewSyntaxToken::from_capture_name(""),
        PreviewSyntaxToken::Plain
    );
}
