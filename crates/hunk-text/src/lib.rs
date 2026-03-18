use ropey::Rope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(u64);

impl BufferId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

impl TextPosition {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

impl TextRange {
    pub fn new(start: TextPosition, end: TextPosition) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: TextPosition,
    pub head: TextPosition,
}

impl Selection {
    pub const fn caret(position: TextPosition) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    pub const fn new(anchor: TextPosition, head: TextPosition) -> Self {
        Self { anchor, head }
    }

    pub fn range(self) -> TextRange {
        TextRange::new(self.anchor, self.head)
    }

    pub fn is_caret(self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnapshot {
    pub buffer_id: BufferId,
    pub version: u64,
    pub text: String,
    pub line_count: usize,
    pub byte_len: usize,
}

#[derive(Debug, Clone)]
pub struct TextBuffer {
    id: BufferId,
    rope: Rope,
    version: u64,
}

impl TextBuffer {
    pub fn new(id: BufferId, text: &str) -> Self {
        Self {
            id,
            rope: Rope::from_str(text),
            version: 0,
        }
    }

    pub const fn id(&self) -> BufferId {
        self.id
    }

    pub const fn version(&self) -> u64 {
        self.version
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn byte_len(&self) -> usize {
        self.rope.len_bytes()
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.version = self.version.saturating_add(1);
    }

    pub fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            buffer_id: self.id,
            version: self.version,
            text: self.text(),
            line_count: self.line_count(),
            byte_len: self.byte_len(),
        }
    }
}
