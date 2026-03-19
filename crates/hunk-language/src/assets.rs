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
        bash_language(),
        java_language(),
        c_language(),
        cpp_language(),
        c_sharp_language(),
        json_language(),
        yaml_language(),
        go_language(),
        html_language(),
        css_language(),
        sql_language(),
        dockerfile_language(),
        markdown_language(),
        toml_language(),
        python_language(),
        powershell_language(),
        hcl_language(),
        swift_language(),
        kotlin_language(),
        nix_language(),
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
        format!(
            "{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_typescript::HIGHLIGHTS_QUERY
        ),
        tree_sitter_javascript::INJECTIONS_QUERY,
        format!(
            "{}\n{}",
            tree_sitter_javascript::LOCALS_QUERY,
            tree_sitter_typescript::LOCALS_QUERY
        ),
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
        format!(
            "{}\n{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
            tree_sitter_typescript::HIGHLIGHTS_QUERY
        ),
        tree_sitter_javascript::INJECTIONS_QUERY,
        format!(
            "{}\n{}",
            tree_sitter_javascript::LOCALS_QUERY,
            tree_sitter_typescript::LOCALS_QUERY
        ),
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

fn bash_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(11),
        "Bash",
        "bash",
        FileMatcher {
            extensions: vec!["sh".to_string(), "bash".to_string(), "zsh".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_bash::LANGUAGE.into(),
        tree_sitter_bash::HIGHLIGHT_QUERY,
        "",
        "",
        &[],
        &["bash", "shell", "sh", "zsh"],
    )
}

fn java_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(14),
        "Java",
        "java",
        FileMatcher {
            extensions: vec!["java".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_java::LANGUAGE.into(),
        tree_sitter_java::HIGHLIGHTS_QUERY,
        "",
        "",
        &["class_body", "block", "argument_list", "array_initializer"],
        &["java"],
    )
}

fn c_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(15),
        "C",
        "c",
        FileMatcher {
            extensions: vec!["c".to_string(), "h".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_c::LANGUAGE.into(),
        include_str!("queries/c_highlights.scm"),
        "",
        "",
        &[
            "compound_statement",
            "enumerator_list",
            "field_declaration_list",
            "initializer_list",
        ],
        &["c"],
    )
}

fn cpp_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(16),
        "C++",
        "cpp",
        FileMatcher {
            extensions: vec![
                "cc".to_string(),
                "cpp".to_string(),
                "cxx".to_string(),
                "hpp".to_string(),
                "hh".to_string(),
                "hxx".to_string(),
            ],
            file_names: Vec::new(),
        },
        || tree_sitter_cpp::LANGUAGE.into(),
        format!(
            "{}\n{}",
            include_str!("queries/c_highlights.scm"),
            include_str!("queries/cpp_highlights.scm")
        ),
        include_str!("queries/cpp_injections.scm"),
        "",
        &[
            "compound_statement",
            "field_declaration_list",
            "enumerator_list",
            "initializer_list",
            "namespace_definition",
        ],
        &["cpp", "c++", "cplusplus"],
    )
}

fn c_sharp_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(17),
        "C#",
        "csharp",
        FileMatcher {
            extensions: vec!["cs".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_c_sharp::LANGUAGE.into(),
        include_str!("queries/c_sharp_highlights.scm"),
        "",
        "",
        &[
            "block",
            "declaration_list",
            "switch_body",
            "initializer_expression",
        ],
        &["csharp", "c#", "cs"],
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

fn sql_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(22),
        "SQL",
        "sql",
        FileMatcher {
            extensions: vec!["sql".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_sequel::LANGUAGE.into(),
        tree_sitter_sequel::HIGHLIGHTS_QUERY,
        "",
        "",
        &[
            "select_expression",
            "from_expression",
            "where_expression",
            "join_expression",
            "group_expression",
        ],
        &["sql"],
    )
}

fn dockerfile_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(23),
        "Dockerfile",
        "dockerfile",
        FileMatcher {
            extensions: vec!["dockerfile".to_string()],
            file_names: vec![
                "Dockerfile".to_string(),
                "dockerfile".to_string(),
                "Containerfile".to_string(),
            ],
        },
        tree_sitter_dockerfile_updated::language,
        include_str!("queries/dockerfile_highlights.scm"),
        "",
        "",
        &["json_array", "json_object"],
        &["dockerfile", "containerfile"],
    )
}

fn markdown_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(24),
        "Markdown",
        "markdown",
        FileMatcher {
            extensions: vec![
                "md".to_string(),
                "markdown".to_string(),
                "mdown".to_string(),
            ],
            file_names: Vec::new(),
        },
        || tree_sitter_md::LANGUAGE.into(),
        include_str!("queries/markdown_highlights.scm"),
        tree_sitter_md::INJECTION_QUERY_BLOCK,
        "",
        &["section", "list", "block_quote", "fenced_code_block"],
        &["markdown", "md"],
    )
}

fn toml_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(10),
        "TOML",
        "toml",
        FileMatcher {
            extensions: vec!["toml".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_toml_ng::LANGUAGE.into(),
        tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
        "",
        "",
        &["table", "inline_table", "array"],
        &["toml"],
    )
}

fn python_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(12),
        "Python",
        "python",
        FileMatcher {
            extensions: vec!["py".to_string(), "pyi".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_python::LANGUAGE.into(),
        tree_sitter_python::HIGHLIGHTS_QUERY,
        "",
        "",
        &["block", "dictionary", "list", "tuple", "set"],
        &["python", "py"],
    )
}

fn powershell_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(13),
        "PowerShell",
        "powershell",
        FileMatcher {
            extensions: vec!["ps1".to_string(), "psm1".to_string(), "psd1".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_powershell::LANGUAGE.into(),
        include_str!("queries/powershell_highlights.scm"),
        "",
        "",
        &[],
        &["powershell", "pwsh", "ps1"],
    )
}

fn hcl_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(18),
        "Terraform",
        "terraform",
        FileMatcher {
            extensions: vec!["tf".to_string(), "tfvars".to_string(), "hcl".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_hcl::LANGUAGE.into(),
        include_str!("queries/hcl_highlights.scm"),
        "",
        "",
        &["body", "block", "object", "tuple"],
        &["terraform", "hcl", "tf"],
    )
}

fn swift_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(19),
        "Swift",
        "swift",
        FileMatcher {
            extensions: vec!["swift".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_swift::LANGUAGE.into(),
        tree_sitter_swift::HIGHLIGHTS_QUERY,
        tree_sitter_swift::INJECTIONS_QUERY,
        tree_sitter_swift::LOCALS_QUERY,
        &[
            "statements",
            "class_body",
            "struct_body",
            "protocol_body",
            "enum_class_body",
        ],
        &["swift"],
    )
}

fn kotlin_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(20),
        "Kotlin",
        "kotlin",
        FileMatcher {
            extensions: vec!["kt".to_string(), "kts".to_string()],
            file_names: Vec::new(),
        },
        || tree_sitter_kotlin_sg::LANGUAGE.into(),
        tree_sitter_kotlin_sg::HIGHLIGHTS_QUERY,
        "",
        "",
        &[
            "class_body",
            "function_body",
            "control_structure_body",
            "lambda_literal",
            "when_entry",
        ],
        &["kotlin", "kt", "kts"],
    )
}

fn nix_language() -> LanguageDefinition {
    LanguageDefinition::new(
        LanguageId::new(21),
        "Nix",
        "nix",
        FileMatcher {
            extensions: vec!["nix".to_string()],
            file_names: vec![
                "default.nix".to_string(),
                "flake.nix".to_string(),
                "shell.nix".to_string(),
            ],
        },
        || tree_sitter_nix::LANGUAGE.into(),
        tree_sitter_nix::HIGHLIGHTS_QUERY,
        tree_sitter_nix::INJECTIONS_QUERY,
        "",
        &["binding_set", "list_expression", "attrset_expression"],
        &["nix"],
    )
}
