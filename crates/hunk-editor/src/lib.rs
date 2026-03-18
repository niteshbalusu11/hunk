use hunk_language::{LanguageId, ParseStatus};
use hunk_text::{Selection, TextBuffer, TextPosition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub first_visible_line: usize,
    pub visible_line_count: usize,
    pub horizontal_offset: usize,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            first_visible_line: 0,
            visible_line_count: 1,
            horizontal_offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySnapshot {
    pub viewport: Viewport,
    pub line_count: usize,
    pub dirty: bool,
    pub language_id: Option<LanguageId>,
    pub parse_status: ParseStatus,
    pub selection_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorStatusSnapshot {
    pub line_count: usize,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub selection_count: usize,
    pub dirty: bool,
    pub language_id: Option<LanguageId>,
    pub parse_status: ParseStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    SetViewport(Viewport),
    SetSelection(Selection),
    ReplaceAll(String),
    SetLanguage(Option<LanguageId>),
    SetParseStatus(ParseStatus),
    MarkSaved,
}

#[derive(Debug, Clone)]
pub struct EditorState {
    buffer: TextBuffer,
    primary_selection: Selection,
    secondary_selections: Vec<Selection>,
    viewport: Viewport,
    dirty: bool,
    language_id: Option<LanguageId>,
    parse_status: ParseStatus,
}

impl EditorState {
    pub fn new(buffer: TextBuffer) -> Self {
        Self {
            buffer,
            primary_selection: Selection::caret(TextPosition::default()),
            secondary_selections: Vec::new(),
            viewport: Viewport::default(),
            dirty: false,
            language_id: None,
            parse_status: ParseStatus::Idle,
        }
    }

    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn display_snapshot(&self) -> DisplaySnapshot {
        DisplaySnapshot {
            viewport: self.viewport,
            line_count: self.buffer.line_count(),
            dirty: self.dirty,
            language_id: self.language_id,
            parse_status: self.parse_status,
            selection_count: 1 + self.secondary_selections.len(),
        }
    }

    pub fn status_snapshot(&self) -> EditorStatusSnapshot {
        EditorStatusSnapshot {
            line_count: self.buffer.line_count(),
            cursor_line: self.primary_selection.head.line + 1,
            cursor_column: self.primary_selection.head.column + 1,
            selection_count: 1 + self.secondary_selections.len(),
            dirty: self.dirty,
            language_id: self.language_id,
            parse_status: self.parse_status,
        }
    }

    pub fn apply(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::SetViewport(viewport) => {
                self.viewport = viewport;
            }
            EditorCommand::SetSelection(selection) => {
                self.primary_selection = selection;
            }
            EditorCommand::ReplaceAll(text) => {
                self.buffer.set_text(&text);
                self.dirty = true;
            }
            EditorCommand::SetLanguage(language_id) => {
                self.language_id = language_id;
            }
            EditorCommand::SetParseStatus(parse_status) => {
                self.parse_status = parse_status;
            }
            EditorCommand::MarkSaved => {
                self.dirty = false;
            }
        }
    }
}
