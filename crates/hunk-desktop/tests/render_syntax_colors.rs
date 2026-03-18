#[path = "../src/app/highlight.rs"]
mod highlight;
mod app {
    pub(crate) mod highlight {
        pub(crate) use crate::highlight::SyntaxTokenKind;
    }

    pub(crate) mod theme {
        use gpui::Hsla;
        use gpui_component::Theme;

        #[derive(Clone, Copy)]
        pub(crate) struct HunkEditorSyntaxColors {
            pub keyword: Hsla,
            pub string: Hsla,
            pub number: Hsla,
            pub comment: Hsla,
            pub function: Hsla,
            pub type_name: Hsla,
            pub constant: Hsla,
            pub variable: Hsla,
            pub operator: Hsla,
        }

        pub(crate) fn hunk_editor_syntax_colors(
            _theme: &Theme,
            is_dark: bool,
        ) -> HunkEditorSyntaxColors {
            if is_dark {
                HunkEditorSyntaxColors {
                    keyword: gpui::rgb(0x569cd6).into(),
                    string: gpui::rgb(0xce9178).into(),
                    number: gpui::rgb(0xb5cea8).into(),
                    comment: gpui::rgb(0x6a9955).into(),
                    function: gpui::rgb(0xdcdcaa).into(),
                    type_name: gpui::rgb(0x4ec9b0).into(),
                    constant: gpui::rgb(0x4fc1ff).into(),
                    variable: gpui::rgb(0x9cdcfe).into(),
                    operator: gpui::rgb(0xd4d4d4).into(),
                }
            } else {
                HunkEditorSyntaxColors {
                    keyword: gpui::rgb(0x0000ff).into(),
                    string: gpui::rgb(0xa31515).into(),
                    number: gpui::rgb(0x098658).into(),
                    comment: gpui::rgb(0x008000).into(),
                    function: gpui::rgb(0x795e26).into(),
                    type_name: gpui::rgb(0x267f99).into(),
                    constant: gpui::rgb(0x0070c1).into(),
                    variable: gpui::rgb(0x001080).into(),
                    operator: gpui::rgb(0x000000).into(),
                }
            }
        }
    }
}
#[path = "../src/app/render/syntax_colors.rs"]
mod syntax_colors;

use gpui::Hsla;
use gpui_component::Theme;
use highlight::SyntaxTokenKind;
use hunk_domain::markdown_preview::MarkdownCodeTokenKind;
use syntax_colors::{diff_syntax_color, markdown_syntax_color};

fn default_color() -> Hsla {
    gpui::rgb(0x112233).into()
}

#[test]
fn shared_syntax_palette_matches_diff_and_markdown_tokens() {
    let default = default_color();
    let theme = Theme::default();
    assert_eq!(
        diff_syntax_color(&theme, default, SyntaxTokenKind::Keyword),
        markdown_syntax_color(&theme, default, MarkdownCodeTokenKind::Keyword),
    );
    assert_eq!(
        diff_syntax_color(&theme, default, SyntaxTokenKind::String),
        markdown_syntax_color(&theme, default, MarkdownCodeTokenKind::String),
    );
    assert_eq!(
        diff_syntax_color(&theme, default, SyntaxTokenKind::Plain),
        default,
    );
}
