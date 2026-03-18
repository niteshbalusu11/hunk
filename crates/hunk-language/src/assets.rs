use crate::{FileMatcher, LanguageDefinition, LanguageId};

pub const CANONICAL_HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "boolean",
    "carriage-return",
    "comment",
    "comment.documentation",
    "constant",
    "constant.builtin",
    "constructor",
    "constructor.builtin",
    "embedded",
    "error",
    "escape",
    "function",
    "function.builtin",
    "keyword",
    "markup",
    "markup.bold",
    "markup.heading",
    "markup.italic",
    "markup.link",
    "markup.link.url",
    "markup.list",
    "markup.list.checked",
    "markup.list.numbered",
    "markup.list.unchecked",
    "markup.list.unnumbered",
    "markup.quote",
    "markup.raw",
    "markup.raw.block",
    "markup.raw.inline",
    "markup.strikethrough",
    "module",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.escape",
    "string.regexp",
    "string.special",
    "string.special.symbol",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.member",
    "variable.parameter",
];

pub fn builtin_language_definitions() -> Vec<LanguageDefinition> {
    vec![
        rust_language(),
        javascript_language(),
        typescript_language(),
        tsx_language(),
        json_language(),
        yaml_language(),
        go_language(),
        html_language(),
        css_language(),
    ]
}

fn rust_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(1),
        "Rust",
        "rust",
        FileMatcher {
            extensions: vec!["rs".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_rust::LANGUAGE.into(),
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        "",
        "",
        &[
            "block",
            "declaration_list",
            "match_block",
            "field_declaration_list",
        ],
        &["rust"],
    )
}

fn javascript_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(2),
        "JavaScript",
        "javascript",
        FileMatcher {
            extensions: vec![
                "js".to_string(),
                "mjs".to_string(),
                "cjs".to_string(),
                "jsx".to_string(),
            ],
            file_names: Vec::new(),
        },
        || tree_sitter_javascript::LANGUAGE.into(),
        format!(
            "{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
        ),
        tree_sitter_javascript::INJECTIONS_QUERY,
        tree_sitter_javascript::LOCALS_QUERY,
        &[
            "statement_block",
            "class_body",
            "object",
            "array",
            "jsx_element",
            "jsx_fragment",
        ],
        &["javascript", "ecma", "js"],
    )
}

fn typescript_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(3),
        "TypeScript",
        "typescript",
        FileMatcher {
            extensions: vec!["ts".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
        &[
            "statement_block",
            "class_body",
            "object",
            "array",
            "interface_body",
            "enum_body",
        ],
        &["typescript", "ts"],
    )
}

fn tsx_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(4),
        "TSX",
        "tsx",
        FileMatcher {
            extensions: vec!["tsx".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_typescript::LANGUAGE_TSX.into(),
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
        &[
            "statement_block",
            "class_body",
            "object",
            "array",
            "jsx_element",
            "jsx_fragment",
        ],
        &["tsx", "typescriptreact"],
    )
}

fn json_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(5),
        "JSON",
        "json",
        FileMatcher {
            extensions: vec!["json".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_json::LANGUAGE.into(),
        tree_sitter_json::HIGHLIGHTS_QUERY,
        "",
        "",
        &["object", "array"],
        &["json"],
    )
}

fn yaml_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(6),
        "YAML",
        "yaml",
        FileMatcher {
            extensions: vec!["yaml".to_string(), "yml".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_yaml::LANGUAGE.into(),
        tree_sitter_yaml::HIGHLIGHTS_QUERY,
        "",
        "",
        &[
            "block_mapping",
            "block_sequence",
            "flow_mapping",
            "flow_sequence",
        ],
        &["yaml"],
    )
}

fn go_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(7),
        "Go",
        "go",
        FileMatcher {
            extensions: vec!["go".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_go::LANGUAGE.into(),
        tree_sitter_go::HIGHLIGHTS_QUERY,
        "",
        "",
        &[
            "block",
            "parameter_list",
            "literal_value",
            "field_declaration_list",
        ],
        &["go"],
    )
}

fn html_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(8),
        "HTML",
        "html",
        FileMatcher {
            extensions: vec!["html".to_string(), "htm".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_html::LANGUAGE.into(),
        tree_sitter_html::HIGHLIGHTS_QUERY,
        tree_sitter_html::INJECTIONS_QUERY,
        "",
        &["element"],
        &["html"],
    )
}

fn css_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(9),
        "CSS",
        "css",
        FileMatcher {
            extensions: vec!["css".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_css::LANGUAGE.into(),
        tree_sitter_css::HIGHLIGHTS_QUERY,
        "",
        "",
        &["block", "rule_set", "media_statement", "supports_statement"],
        &["css"],
    )
}
