#[path = "../src/app/highlight.rs"]
mod highlight;
mod app {
    pub(crate) mod highlight {
        pub(crate) use crate::highlight::SyntaxTokenKind;
    }

    pub(crate) mod theme {
        pub(crate) fn hunk_pick<T: Copy>(is_dark: bool, dark: T, light: T) -> T {
            if is_dark { dark } else { light }
        }
    }
}
#[path = "../src/app/render/syntax_colors.rs"]
mod syntax_colors;

use gpui::Hsla;
use highlight::SyntaxTokenKind;
use hunk_domain::markdown_preview::MarkdownCodeTokenKind;
use syntax_colors::{diff_syntax_color, markdown_syntax_color};

fn default_color() -> Hsla {
    gpui::rgb(0x112233).into()
}

#[test]
fn shared_syntax_palette_matches_diff_and_markdown_tokens() {
    let default = default_color();
    assert_eq!(
        diff_syntax_color(default, SyntaxTokenKind::Keyword, false),
        markdown_syntax_color(default, MarkdownCodeTokenKind::Keyword, false),
    );
    assert_eq!(
        diff_syntax_color(default, SyntaxTokenKind::String, true),
        markdown_syntax_color(default, MarkdownCodeTokenKind::String, true),
    );
    assert_eq!(
        diff_syntax_color(default, SyntaxTokenKind::Plain, false),
        default,
    );
}
