#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreviewSyntaxToken {
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

impl PreviewSyntaxToken {
    pub fn from_capture_name(capture_name: &str) -> Self {
        if capture_name.is_empty() {
            return Self::Plain;
        }

        let name = capture_name.to_ascii_lowercase();
        let name = name.as_str();

        if matches_capture(name, &["comment"]) {
            return Self::Comment;
        }
        if matches_capture(name, &["text.literal", "link_uri"]) {
            return Self::String;
        }
        if matches_capture(name, &["string", "escape"]) {
            return Self::String;
        }
        if matches_capture(name, &["number", "boolean"]) {
            return Self::Number;
        }
        if matches_capture(name, &["function", "constructor"]) {
            return Self::Function;
        }
        if matches_capture(name, &["type", "module", "enum", "variant", "tag"]) {
            return Self::TypeName;
        }
        if matches_capture(name, &["constant"]) {
            return Self::Constant;
        }
        if matches_capture(name, &["title", "emphasis"]) {
            return Self::Keyword;
        }
        if matches_capture(name, &["keyword", "preproc", "attribute"]) {
            return Self::Keyword;
        }
        if matches_capture(name, &["link_text"]) {
            return Self::Variable;
        }
        if matches_capture(name, &["variable", "property", "label"]) {
            return Self::Variable;
        }
        if matches_capture(name, &["operator", "punctuation"]) {
            return Self::Operator;
        }

        Self::Plain
    }
}

fn matches_capture(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        name == *prefix
            || name
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with('.'))
    })
}
