#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRowKind {
    Code,
    HunkHeader,
    Meta,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffCellKind {
    None,
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct DiffCell {
    pub line: Option<u32>,
    pub text: String,
    pub kind: DiffCellKind,
}

impl DiffCell {
    fn empty() -> Self {
        Self {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        }
    }

    fn new(line: Option<u32>, text: impl Into<String>, kind: DiffCellKind) -> Self {
        Self {
            line,
            text: text.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SideBySideRow {
    pub kind: DiffRowKind,
    pub left: DiffCell,
    pub right: DiffCell,
    pub text: String,
}

impl SideBySideRow {
    fn meta(kind: DiffRowKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            left: DiffCell::empty(),
            right: DiffCell::empty(),
            text: text.into(),
        }
    }

    fn code(left: DiffCell, right: DiffCell) -> Self {
        Self {
            kind: DiffRowKind::Code,
            left,
            right,
            text: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub text: String,
}

impl DiffLine {
    fn new(
        kind: DiffLineKind,
        old_line: Option<u32>,
        new_line: Option<u32>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            old_line,
            new_line,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub header: String,
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<DiffLine>,
    pub trailing_meta: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffDocument {
    pub prelude: Vec<String>,
    pub hunks: Vec<DiffHunk>,
    pub epilogue: Vec<String>,
}

pub fn parse_patch_document(patch: &str) -> DiffDocument {
    let mut document = DiffDocument::default();
    if patch.trim().is_empty() {
        return document;
    }

    let lines = patch.lines().collect::<Vec<_>>();
    let mut ix = 0_usize;

    while ix < lines.len() {
        let line = lines[ix];

        if line.starts_with("@@") {
            let (old_start, new_start) = parse_hunk_header(line).unwrap_or((0, 0));
            ix += 1;

            let mut old_line = old_start;
            let mut new_line = new_start;
            let mut hunk_lines = Vec::new();
            let mut trailing_meta = Vec::new();

            while ix < lines.len() {
                let hunk_line = lines[ix];

                if hunk_line.starts_with("@@") || hunk_line.starts_with("diff --git") {
                    break;
                }
                if is_meta_line(hunk_line) && !hunk_line.starts_with("\\ No newline at end of file")
                {
                    break;
                }

                match hunk_line.chars().next() {
                    Some(' ') => {
                        hunk_lines.push(DiffLine::new(
                            DiffLineKind::Context,
                            Some(old_line),
                            Some(new_line),
                            hunk_line.trim_start_matches(' '),
                        ));
                        old_line = old_line.saturating_add(1);
                        new_line = new_line.saturating_add(1);
                    }
                    Some('-') => {
                        hunk_lines.push(DiffLine::new(
                            DiffLineKind::Removed,
                            Some(old_line),
                            None,
                            hunk_line.trim_start_matches('-'),
                        ));
                        old_line = old_line.saturating_add(1);
                    }
                    Some('+') => {
                        hunk_lines.push(DiffLine::new(
                            DiffLineKind::Added,
                            None,
                            Some(new_line),
                            hunk_line.trim_start_matches('+'),
                        ));
                        new_line = new_line.saturating_add(1);
                    }
                    _ => trailing_meta.push(hunk_line.to_string()),
                }

                ix += 1;
            }

            document.hunks.push(DiffHunk {
                header: line.to_string(),
                old_start,
                new_start,
                lines: hunk_lines,
                trailing_meta,
            });
            continue;
        }

        if document.hunks.is_empty() {
            document.prelude.push(line.to_string());
        } else {
            document.epilogue.push(line.to_string());
        }
        ix += 1;
    }

    document
}

pub fn parse_patch_side_by_side(patch: &str) -> Vec<SideBySideRow> {
    if patch.trim().is_empty() {
        return vec![SideBySideRow::meta(
            DiffRowKind::Empty,
            "No diff for this file.",
        )];
    }

    let mut rows = Vec::new();
    let document = parse_patch_document(patch);

    for hunk in document.hunks {
        append_hunk_rows(&hunk, &mut rows);
    }

    if rows.is_empty() {
        rows.push(SideBySideRow::meta(
            DiffRowKind::Empty,
            "No diff for this file.",
        ));
    }

    rows
}

fn append_hunk_rows(hunk: &DiffHunk, rows: &mut Vec<SideBySideRow>) {
    let mut ix = 0_usize;
    while ix < hunk.lines.len() {
        let line = &hunk.lines[ix];
        match line.kind {
            DiffLineKind::Removed => {
                let removed_start = ix;
                while ix < hunk.lines.len() && hunk.lines[ix].kind == DiffLineKind::Removed {
                    ix += 1;
                }

                let added_start = ix;
                while ix < hunk.lines.len() && hunk.lines[ix].kind == DiffLineKind::Added {
                    ix += 1;
                }

                let removed = &hunk.lines[removed_start..added_start];
                let added = &hunk.lines[added_start..ix];
                let max_len = removed.len().max(added.len());

                for entry_ix in 0..max_len {
                    let left = removed.get(entry_ix).map_or_else(DiffCell::empty, |line| {
                        DiffCell::new(line.old_line, line.text.clone(), DiffCellKind::Removed)
                    });
                    let right = added.get(entry_ix).map_or_else(DiffCell::empty, |line| {
                        DiffCell::new(line.new_line, line.text.clone(), DiffCellKind::Added)
                    });
                    rows.push(SideBySideRow::code(left, right));
                }
            }
            DiffLineKind::Added => {
                rows.push(SideBySideRow::code(
                    DiffCell::empty(),
                    DiffCell::new(line.new_line, line.text.clone(), DiffCellKind::Added),
                ));
                ix += 1;
            }
            DiffLineKind::Context => {
                rows.push(SideBySideRow::code(
                    DiffCell::new(line.old_line, line.text.clone(), DiffCellKind::Context),
                    DiffCell::new(line.new_line, line.text.clone(), DiffCellKind::Context),
                ));
                ix += 1;
            }
        }
    }
}

fn is_meta_line(line: &str) -> bool {
    line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
        || line.starts_with("new file mode")
        || line.starts_with("deleted file mode")
        || line.starts_with("rename from")
        || line.starts_with("rename to")
        || line.starts_with("Binary files")
        || line.starts_with("\\ No newline at end of file")
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    let left_marker = line.find('-')?;
    let right_marker = line.find('+')?;

    let left_part = line[left_marker + 1..].split_whitespace().next()?;
    let right_part = line[right_marker + 1..].split_whitespace().next()?;

    let left_start = parse_range_start(left_part)?;
    let right_start = parse_range_start(right_part)?;

    Some((left_start, right_start))
}

fn parse_range_start(range: &str) -> Option<u32> {
    range.split(',').next()?.parse::<u32>().ok()
}
