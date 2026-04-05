use std::error::Error;
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use hunk_text::BufferId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceDocumentId(u64);

impl WorkspaceDocumentId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceExcerptId(u64);

impl WorkspaceExcerptId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDocument {
    pub id: WorkspaceDocumentId,
    pub path: PathBuf,
    pub buffer_id: BufferId,
    pub line_count: usize,
}

impl WorkspaceDocument {
    pub fn new(
        id: WorkspaceDocumentId,
        path: impl Into<PathBuf>,
        buffer_id: BufferId,
        line_count: usize,
    ) -> Self {
        Self {
            id,
            path: path.into(),
            buffer_id,
            line_count,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceExcerptKind {
    FullFile,
    DiffHunk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceExcerptSpec {
    pub id: WorkspaceExcerptId,
    pub document_id: WorkspaceDocumentId,
    pub kind: WorkspaceExcerptKind,
    pub line_range: Range<usize>,
    pub leading_rows: usize,
    pub trailing_rows: usize,
}

impl WorkspaceExcerptSpec {
    pub fn new(
        id: WorkspaceExcerptId,
        document_id: WorkspaceDocumentId,
        kind: WorkspaceExcerptKind,
        line_range: Range<usize>,
    ) -> Self {
        Self {
            id,
            document_id,
            kind,
            line_range,
            leading_rows: 0,
            trailing_rows: 0,
        }
    }

    pub fn with_chrome_rows(mut self, leading_rows: usize, trailing_rows: usize) -> Self {
        self.leading_rows = leading_rows;
        self.trailing_rows = trailing_rows;
        self
    }

    pub fn content_row_count(&self) -> usize {
        self.line_range.end.saturating_sub(self.line_range.start)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceExcerptLayout {
    pub spec: WorkspaceExcerptSpec,
    pub global_row_range: Range<usize>,
}

impl WorkspaceExcerptLayout {
    pub fn leading_row_range(&self) -> Range<usize> {
        self.global_row_range.start..self.global_row_range.start + self.spec.leading_rows
    }

    pub fn content_row_range(&self) -> Range<usize> {
        let start = self.global_row_range.start + self.spec.leading_rows;
        start..start + self.spec.content_row_count()
    }

    pub fn trailing_row_range(&self) -> Range<usize> {
        self.content_row_range().end..self.global_row_range.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRowKind {
    LeadingChrome,
    Content,
    TrailingChrome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRowLocation {
    pub excerpt_id: WorkspaceExcerptId,
    pub document_id: WorkspaceDocumentId,
    pub row_kind: WorkspaceRowKind,
    pub document_line: Option<usize>,
    pub row_in_excerpt: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    documents: Vec<WorkspaceDocument>,
    excerpts: Vec<WorkspaceExcerptLayout>,
    gap_rows: usize,
    total_rows: usize,
}

impl WorkspaceLayout {
    pub fn new(
        documents: Vec<WorkspaceDocument>,
        excerpts: Vec<WorkspaceExcerptSpec>,
        gap_rows: usize,
    ) -> Result<Self, WorkspaceLayoutError> {
        let excerpt_count = excerpts.len();
        let mut total_rows = 0usize;
        let mut laid_out_excerpts = Vec::with_capacity(excerpt_count);

        for (index, spec) in excerpts.into_iter().enumerate() {
            let document = documents
                .iter()
                .find(|document| document.id == spec.document_id)
                .ok_or(WorkspaceLayoutError::MissingDocument {
                    document_id: spec.document_id,
                })?;

            if spec.line_range.start > spec.line_range.end
                || spec.line_range.end > document.line_count
            {
                return Err(WorkspaceLayoutError::LineRangeOutOfBounds {
                    excerpt_id: spec.id,
                    document_id: spec.document_id,
                    line_range: spec.line_range,
                    line_count: document.line_count,
                });
            }

            let excerpt_rows = spec.leading_rows + spec.content_row_count() + spec.trailing_rows;
            let start = total_rows;
            let end = start + excerpt_rows;
            laid_out_excerpts.push(WorkspaceExcerptLayout {
                spec,
                global_row_range: start..end,
            });
            total_rows = end;

            if gap_rows > 0 && index + 1 < excerpt_count {
                total_rows = total_rows.saturating_add(gap_rows);
            }
        }

        Ok(Self {
            documents,
            excerpts: laid_out_excerpts,
            gap_rows,
            total_rows,
        })
    }

    pub fn documents(&self) -> &[WorkspaceDocument] {
        &self.documents
    }

    pub fn excerpts(&self) -> &[WorkspaceExcerptLayout] {
        &self.excerpts
    }

    pub fn gap_rows(&self) -> usize {
        self.gap_rows
    }

    pub fn total_rows(&self) -> usize {
        self.total_rows
    }

    pub fn document(&self, id: WorkspaceDocumentId) -> Option<&WorkspaceDocument> {
        self.documents.iter().find(|document| document.id == id)
    }

    pub fn excerpt(&self, id: WorkspaceExcerptId) -> Option<&WorkspaceExcerptLayout> {
        self.excerpts.iter().find(|excerpt| excerpt.spec.id == id)
    }

    pub fn excerpt_at_row(&self, row: usize) -> Option<&WorkspaceExcerptLayout> {
        self.excerpts
            .iter()
            .find(|excerpt| excerpt.global_row_range.contains(&row))
    }

    pub fn locate_row(&self, row: usize) -> Option<WorkspaceRowLocation> {
        let excerpt = self.excerpt_at_row(row)?;
        let row_in_excerpt = row.saturating_sub(excerpt.global_row_range.start);

        if row < excerpt.leading_row_range().end {
            return Some(WorkspaceRowLocation {
                excerpt_id: excerpt.spec.id,
                document_id: excerpt.spec.document_id,
                row_kind: WorkspaceRowKind::LeadingChrome,
                document_line: None,
                row_in_excerpt,
            });
        }

        let content = excerpt.content_row_range();
        if content.contains(&row) {
            return Some(WorkspaceRowLocation {
                excerpt_id: excerpt.spec.id,
                document_id: excerpt.spec.document_id,
                row_kind: WorkspaceRowKind::Content,
                document_line: Some(excerpt.spec.line_range.start + row - content.start),
                row_in_excerpt,
            });
        }

        Some(WorkspaceRowLocation {
            excerpt_id: excerpt.spec.id,
            document_id: excerpt.spec.document_id,
            row_kind: WorkspaceRowKind::TrailingChrome,
            document_line: None,
            row_in_excerpt,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceLayoutError {
    MissingDocument {
        document_id: WorkspaceDocumentId,
    },
    LineRangeOutOfBounds {
        excerpt_id: WorkspaceExcerptId,
        document_id: WorkspaceDocumentId,
        line_range: Range<usize>,
        line_count: usize,
    },
}

impl fmt::Display for WorkspaceLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDocument { document_id } => {
                write!(
                    f,
                    "workspace excerpt references missing document {}",
                    document_id.get()
                )
            }
            Self::LineRangeOutOfBounds {
                excerpt_id,
                document_id,
                line_range,
                line_count,
            } => write!(
                f,
                "workspace excerpt {} references invalid line range {:?} for document {} with {} lines",
                excerpt_id.get(),
                line_range,
                document_id.get(),
                line_count
            ),
        }
    }
}

impl Error for WorkspaceLayoutError {}
