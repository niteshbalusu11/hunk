use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::Result;

use super::*;
use hunk::diff::parse_patch_side_by_side;
use hunk::git::load_patch;

#[derive(Default)]
struct TreeFolder {
    folders: BTreeMap<String, TreeFolder>,
    files: BTreeMap<String, FileStatus>,
}

#[derive(Debug, Clone)]
pub(super) struct FileRowRange {
    pub(super) path: String,
    pub(super) status: FileStatus,
    pub(super) start_row: usize,
    pub(super) end_row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffStreamRowKind {
    CoreCode,
    CoreHunkHeader,
    CoreMeta,
    FileCollapsed,
    FileError,
    EmptyState,
}

#[derive(Debug, Clone)]
pub(super) struct DiffStreamRowMeta {
    pub(super) stable_id: u64,
    pub(super) file_path: Option<String>,
    pub(super) file_status: Option<FileStatus>,
    pub(super) kind: DiffStreamRowKind,
}

pub(super) struct DiffStream {
    pub(super) rows: Vec<SideBySideRow>,
    pub(super) row_metadata: Vec<DiffStreamRowMeta>,
    pub(super) file_ranges: Vec<FileRowRange>,
    pub(super) file_line_stats: BTreeMap<String, LineStats>,
}

struct LoadedFileDiffRows {
    core_rows: Vec<SideBySideRow>,
    stats: LineStats,
    load_error: Option<String>,
}

pub(super) fn build_tree_items(files: &[ChangedFile]) -> Vec<TreeItem> {
    let mut root = TreeFolder::default();

    for file in files {
        let mut cursor = &mut root;
        let mut parts = file.path.split('/').peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                cursor = cursor.folders.entry(part.to_string()).or_default();
            } else {
                cursor.files.insert(part.to_string(), file.status);
            }
        }
    }

    build_folder_items(&root, "")
}

fn build_folder_items(folder: &TreeFolder, prefix: &str) -> Vec<TreeItem> {
    let mut items = Vec::new();

    for (name, child_folder) in &folder.folders {
        let id = join_path(prefix, name);
        let children = build_folder_items(child_folder, &id);
        items.push(
            TreeItem::new(
                SharedString::from(id.clone()),
                SharedString::from(name.clone()),
            )
            .expanded(true)
            .children(children),
        );
    }

    for name in folder.files.keys() {
        let id = join_path(prefix, name);
        items.push(TreeItem::new(
            SharedString::from(id),
            SharedString::from(name.clone()),
        ));
    }

    items
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

pub(super) fn message_row(kind: DiffRowKind, text: impl Into<String>) -> SideBySideRow {
    SideBySideRow {
        kind,
        left: DiffCell {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        },
        right: DiffCell {
            line: None,
            text: String::new(),
            kind: DiffCellKind::None,
        },
        text: text.into(),
    }
}

pub(super) fn load_diff_stream(
    repo_root: &Path,
    files: &[ChangedFile],
    collapsed_files: &BTreeSet<String>,
) -> Result<DiffStream> {
    let mut rows = Vec::new();
    let mut row_metadata = Vec::new();
    let mut file_ranges = Vec::with_capacity(files.len());
    let mut file_line_stats = BTreeMap::new();

    for file in files {
        let start_row = rows.len();
        let mut file_row_ordinal = 0_usize;
        let loaded_file = load_file_diff_rows(repo_root, file);
        file_line_stats.insert(file.path.clone(), loaded_file.stats);

        if collapsed_files.contains(file.path.as_str()) {
            push_stream_row(
                &mut rows,
                &mut row_metadata,
                message_row(
                    DiffRowKind::Empty,
                    format!(
                        "File collapsed ({} changed lines hidden).",
                        loaded_file.stats.changed()
                    ),
                ),
                DiffStreamRowKind::FileCollapsed,
                Some(file.path.as_str()),
                Some(file.status),
                file_row_ordinal,
            );
        } else if let Some(load_error) = loaded_file.load_error {
            push_stream_row(
                &mut rows,
                &mut row_metadata,
                message_row(DiffRowKind::Meta, load_error),
                DiffStreamRowKind::FileError,
                Some(file.path.as_str()),
                Some(file.status),
                file_row_ordinal,
            );
        } else {
            for row in loaded_file
                .core_rows
                .into_iter()
                .filter(|row| matches!(row.kind, DiffRowKind::Code | DiffRowKind::Empty))
            {
                let row_kind = stream_kind_for_core_row(&row);
                push_stream_row(
                    &mut rows,
                    &mut row_metadata,
                    row,
                    row_kind,
                    Some(file.path.as_str()),
                    Some(file.status),
                    file_row_ordinal,
                );
                file_row_ordinal = file_row_ordinal.saturating_add(1);
            }
        }

        let end_row = rows.len();
        file_ranges.push(FileRowRange {
            path: file.path.clone(),
            status: file.status,
            start_row,
            end_row,
        });
    }

    if rows.is_empty() {
        push_stream_row(
            &mut rows,
            &mut row_metadata,
            message_row(DiffRowKind::Empty, "No changed files."),
            DiffStreamRowKind::EmptyState,
            None,
            None,
            0,
        );
    }

    Ok(DiffStream {
        rows,
        row_metadata,
        file_ranges,
        file_line_stats,
    })
}

fn load_file_diff_rows(repo_root: &Path, file: &ChangedFile) -> LoadedFileDiffRows {
    match load_patch(repo_root, &file.path, file.status) {
        Ok(patch) => {
            let core_rows = parse_patch_side_by_side(&patch);
            let stats = line_stats_from_rows(&core_rows);
            LoadedFileDiffRows {
                core_rows,
                stats,
                load_error: None,
            }
        }
        Err(err) => LoadedFileDiffRows {
            core_rows: Vec::new(),
            stats: LineStats::default(),
            load_error: Some(format!("Failed to load patch for {}: {err:#}", file.path)),
        },
    }
}

fn stream_kind_for_core_row(row: &SideBySideRow) -> DiffStreamRowKind {
    match row.kind {
        DiffRowKind::Code => DiffStreamRowKind::CoreCode,
        DiffRowKind::HunkHeader => DiffStreamRowKind::CoreHunkHeader,
        DiffRowKind::Meta => DiffStreamRowKind::CoreMeta,
        DiffRowKind::Empty => DiffStreamRowKind::EmptyState,
    }
}

fn push_stream_row(
    rows: &mut Vec<SideBySideRow>,
    row_metadata: &mut Vec<DiffStreamRowMeta>,
    row: SideBySideRow,
    kind: DiffStreamRowKind,
    file_path: Option<&str>,
    file_status: Option<FileStatus>,
    ordinal: usize,
) {
    let stable_id = compute_stable_row_id(file_path, kind, ordinal, &row);
    rows.push(row);
    row_metadata.push(DiffStreamRowMeta {
        stable_id,
        file_path: file_path.map(ToString::to_string),
        file_status,
        kind,
    });
}

fn compute_stable_row_id(
    file_path: Option<&str>,
    kind: DiffStreamRowKind,
    ordinal: usize,
    row: &SideBySideRow,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    file_path.unwrap_or("__stream__").hash(&mut hasher);
    stable_kind_tag(kind).hash(&mut hasher);
    ordinal.hash(&mut hasher);
    hash_row(row, &mut hasher);
    hasher.finish()
}

fn hash_row(row: &SideBySideRow, hasher: &mut impl Hasher) {
    diff_row_kind_tag(row.kind).hash(hasher);
    row.text.hash(hasher);
    hash_cell(&row.left, hasher);
    hash_cell(&row.right, hasher);
}

fn hash_cell(cell: &DiffCell, hasher: &mut impl Hasher) {
    cell.line.hash(hasher);
    diff_cell_kind_tag(cell.kind).hash(hasher);
    cell.text.hash(hasher);
}

fn stable_kind_tag(kind: DiffStreamRowKind) -> &'static str {
    match kind {
        DiffStreamRowKind::CoreCode => "core-code",
        DiffStreamRowKind::CoreHunkHeader => "core-hunk-header",
        DiffStreamRowKind::CoreMeta => "core-meta",
        DiffStreamRowKind::FileCollapsed => "file-collapsed",
        DiffStreamRowKind::FileError => "file-error",
        DiffStreamRowKind::EmptyState => "empty-state",
    }
}

fn diff_row_kind_tag(kind: DiffRowKind) -> &'static str {
    match kind {
        DiffRowKind::Code => "code",
        DiffRowKind::HunkHeader => "hunk-header",
        DiffRowKind::Meta => "meta",
        DiffRowKind::Empty => "empty",
    }
}

fn diff_cell_kind_tag(kind: DiffCellKind) -> &'static str {
    match kind {
        DiffCellKind::None => "none",
        DiffCellKind::Context => "context",
        DiffCellKind::Added => "added",
        DiffCellKind::Removed => "removed",
    }
}

fn line_stats_from_rows(rows: &[SideBySideRow]) -> LineStats {
    let mut stats = LineStats::default();

    for row in rows {
        if row.kind != DiffRowKind::Code {
            continue;
        }

        if row.left.kind == DiffCellKind::Removed {
            stats.removed = stats.removed.saturating_add(1);
        }
        if row.right.kind == DiffCellKind::Added {
            stats.added = stats.added.saturating_add(1);
        }
    }

    stats
}

pub(super) fn display_width(text: &str) -> usize {
    text.chars().fold(0, |acc, ch| {
        acc + match ch {
            '\t' => 4,
            ch if ch.is_control() => 0,
            _ => 1,
        }
    })
}

pub(super) fn decimal_digits(value: u32) -> u32 {
    if value == 0 { 1 } else { value.ilog10() + 1 }
}

pub(super) fn line_number_column_width(digits: u32) -> f32 {
    digits as f32 * DIFF_MONO_CHAR_WIDTH + DIFF_LINE_NUMBER_EXTRA_PADDING
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_row_id_is_deterministic_for_same_row() {
        let row = SideBySideRow {
            kind: DiffRowKind::Code,
            left: DiffCell {
                line: Some(10),
                text: "old".to_string(),
                kind: DiffCellKind::Removed,
            },
            right: DiffCell {
                line: Some(10),
                text: "new".to_string(),
                kind: DiffCellKind::Added,
            },
            text: String::new(),
        };

        let first = compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreCode, 2, &row);
        let second =
            compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreCode, 2, &row);

        assert_eq!(first, second);
    }

    #[test]
    fn stable_row_id_changes_when_ordinal_changes() {
        let row = message_row(DiffRowKind::Meta, "header");
        let first =
            compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreMeta, 0, &row);
        let second =
            compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreMeta, 1, &row);

        assert_ne!(first, second);
    }
}
