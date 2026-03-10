#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeSyntaxColorToken {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Function,
    TypeName,
    Constant,
    Variable,
    Operator,
}

impl From<crate::app::highlight::SyntaxTokenKind> for CodeSyntaxColorToken {
    fn from(value: crate::app::highlight::SyntaxTokenKind) -> Self {
        match value {
            crate::app::highlight::SyntaxTokenKind::Plain => Self::Plain,
            crate::app::highlight::SyntaxTokenKind::Keyword => Self::Keyword,
            crate::app::highlight::SyntaxTokenKind::String => Self::String,
            crate::app::highlight::SyntaxTokenKind::Number => Self::Number,
            crate::app::highlight::SyntaxTokenKind::Comment => Self::Comment,
            crate::app::highlight::SyntaxTokenKind::Function => Self::Function,
            crate::app::highlight::SyntaxTokenKind::TypeName => Self::TypeName,
            crate::app::highlight::SyntaxTokenKind::Constant => Self::Constant,
            crate::app::highlight::SyntaxTokenKind::Variable => Self::Variable,
            crate::app::highlight::SyntaxTokenKind::Operator => Self::Operator,
        }
    }
}

impl From<hunk_domain::markdown_preview::MarkdownCodeTokenKind> for CodeSyntaxColorToken {
    fn from(value: hunk_domain::markdown_preview::MarkdownCodeTokenKind) -> Self {
        match value {
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Plain => Self::Plain,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Keyword => Self::Keyword,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::String => Self::String,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Number => Self::Number,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Comment => Self::Comment,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Function => Self::Function,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::TypeName => Self::TypeName,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Constant => Self::Constant,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Variable => Self::Variable,
            hunk_domain::markdown_preview::MarkdownCodeTokenKind::Operator => Self::Operator,
        }
    }
}

fn code_syntax_color(
    default_color: gpui::Hsla,
    token: impl Into<CodeSyntaxColorToken>,
    is_dark: bool,
) -> gpui::Hsla {
    let github = |dark: u32, light: u32| -> gpui::Hsla {
        gpui::rgb(crate::app::theme::hunk_pick(is_dark, dark, light)).into()
    };
    match token.into() {
        CodeSyntaxColorToken::Plain => default_color,
        CodeSyntaxColorToken::Keyword => github(0xff7b72, 0xcf222e),
        CodeSyntaxColorToken::String => github(0xa5d6ff, 0x0a3069),
        CodeSyntaxColorToken::Number => github(0x79c0ff, 0x0550ae),
        CodeSyntaxColorToken::Comment => github(0x8b949e, 0x57606a),
        CodeSyntaxColorToken::Function => github(0xd2a8ff, 0x8250df),
        CodeSyntaxColorToken::TypeName => github(0xffa657, 0x953800),
        CodeSyntaxColorToken::Constant => github(0x79c0ff, 0x0550ae),
        CodeSyntaxColorToken::Variable => github(0xffa657, 0x953800),
        CodeSyntaxColorToken::Operator => github(0xff7b72, 0xcf222e),
    }
}

pub(crate) fn diff_syntax_color(
    default_color: gpui::Hsla,
    token: crate::app::highlight::SyntaxTokenKind,
    is_dark: bool,
) -> gpui::Hsla {
    code_syntax_color(default_color, token, is_dark)
}

pub(crate) fn markdown_syntax_color(
    default_color: gpui::Hsla,
    token: hunk_domain::markdown_preview::MarkdownCodeTokenKind,
    is_dark: bool,
) -> gpui::Hsla {
    code_syntax_color(default_color, token, is_dark)
}
