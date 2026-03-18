use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageId(u16);

impl LanguageId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMatcher {
    pub extensions: Vec<String>,
    pub file_names: Vec<String>,
}

impl FileMatcher {
    pub fn matches_path(&self, path: &Path) -> bool {
        if let Some(file_name) = path.file_name().and_then(|value| value.to_str())
            && self
                .file_names
                .iter()
                .any(|candidate| candidate == file_name)
        {
            return true;
        }

        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                self.extensions
                    .iter()
                    .any(|candidate| candidate == extension)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDefinition {
    pub id: LanguageId,
    pub name: String,
    pub file_matcher: FileMatcher,
    pub grammar_name: String,
    pub highlight_query: String,
    pub injection_query: Option<String>,
    pub locals_query: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseStatus {
    Idle,
    Parsing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightCapture {
    pub name: String,
    pub byte_range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldKind {
    Block,
    Comment,
    Region,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldCandidate {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: FoldKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxSnapshot {
    pub language_id: Option<LanguageId>,
    pub parse_status: ParseStatus,
    pub tree_revision: u64,
    pub highlight_revision: u64,
}

#[derive(Debug, Default, Clone)]
pub struct LanguageRegistry {
    definitions: BTreeMap<LanguageId, LanguageDefinition>,
    ids_by_lower_name: BTreeMap<String, LanguageId>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn register(&mut self, definition: LanguageDefinition) -> Option<LanguageDefinition> {
        let key = definition.name.to_ascii_lowercase();
        self.ids_by_lower_name.insert(key, definition.id);
        self.definitions.insert(definition.id, definition)
    }

    pub fn language_by_name(&self, name: &str) -> Option<&LanguageDefinition> {
        let language_id = self.ids_by_lower_name.get(&name.to_ascii_lowercase())?;
        self.definitions.get(language_id)
    }

    pub fn language_for_path(&self, path: &Path) -> Option<&LanguageDefinition> {
        self.definitions
            .values()
            .find(|definition| definition.file_matcher.matches_path(path))
    }
}
