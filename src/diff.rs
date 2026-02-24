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

pub fn parse_patch_side_by_side(patch: &str) -> Vec<SideBySideRow> {
    if patch.trim().is_empty() {
        return vec![SideBySideRow::meta(
            DiffRowKind::Empty,
            "No diff for this file.",
        )];
    }

    let lines = patch.lines().collect::<Vec<_>>();
    let mut rows = Vec::new();

    let mut left_line = 0_u32;
    let mut right_line = 0_u32;

    let mut ix = 0_usize;
    while ix < lines.len() {
        let line = lines[ix];

        if line.starts_with("@@") {
            if let Some((left_start, right_start)) = parse_hunk_header(line) {
                left_line = left_start;
                right_line = right_start;
            }
            rows.push(SideBySideRow::meta(DiffRowKind::HunkHeader, line));
            ix += 1;
            continue;
        }

        if is_meta_line(line) {
            rows.push(SideBySideRow::meta(DiffRowKind::Meta, line));
            ix += 1;
            continue;
        }

        if line.starts_with('-') && ix + 1 < lines.len() && lines[ix + 1].starts_with('+') {
            let removed = line.trim_start_matches('-');
            let added = lines[ix + 1].trim_start_matches('+');

            rows.push(SideBySideRow::code(
                DiffCell::new(Some(left_line), removed, DiffCellKind::Removed),
                DiffCell::new(Some(right_line), added, DiffCellKind::Added),
            ));

            left_line = left_line.saturating_add(1);
            right_line = right_line.saturating_add(1);
            ix += 2;
            continue;
        }

        if line.starts_with('-') {
            let removed = line.trim_start_matches('-');
            rows.push(SideBySideRow::code(
                DiffCell::new(Some(left_line), removed, DiffCellKind::Removed),
                DiffCell::empty(),
            ));

            left_line = left_line.saturating_add(1);
            ix += 1;
            continue;
        }

        if line.starts_with('+') {
            let added = line.trim_start_matches('+');
            rows.push(SideBySideRow::code(
                DiffCell::empty(),
                DiffCell::new(Some(right_line), added, DiffCellKind::Added),
            ));

            right_line = right_line.saturating_add(1);
            ix += 1;
            continue;
        }

        if line.starts_with(' ') {
            let context = line.trim_start_matches(' ');
            rows.push(SideBySideRow::code(
                DiffCell::new(Some(left_line), context, DiffCellKind::Context),
                DiffCell::new(Some(right_line), context, DiffCellKind::Context),
            ));

            left_line = left_line.saturating_add(1);
            right_line = right_line.saturating_add(1);
            ix += 1;
            continue;
        }

        rows.push(SideBySideRow::meta(DiffRowKind::Meta, line));
        ix += 1;
    }

    rows
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
