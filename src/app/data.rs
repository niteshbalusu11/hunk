use std::collections::{BTreeMap, BTreeSet};
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

pub(super) struct DiffStream {
    pub(super) rows: Vec<SideBySideRow>,
    pub(super) file_ranges: Vec<FileRowRange>,
    pub(super) file_line_stats: BTreeMap<String, LineStats>,
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
    let mut file_ranges = Vec::with_capacity(files.len());
    let mut file_line_stats = BTreeMap::new();

    for file in files {
        let start_row = rows.len();
        rows.push(message_row(
            DiffRowKind::Meta,
            format!("── {} [{}] ──", file.path, file.status.tag()),
        ));

        let (parsed_rows, stats) = match load_patch(repo_root, &file.path, file.status) {
            Ok(patch) => {
                let parsed_rows = parse_patch_side_by_side(&patch);
                let stats = line_stats_from_rows(&parsed_rows);
                (parsed_rows, stats)
            }
            Err(err) => (
                vec![message_row(
                    DiffRowKind::Meta,
                    format!("Failed to load patch for {}: {err:#}", file.path),
                )],
                LineStats::default(),
            ),
        };

        file_line_stats.insert(file.path.clone(), stats);

        if collapsed_files.contains(file.path.as_str()) {
            rows.push(message_row(
                DiffRowKind::Empty,
                format!("File collapsed ({} changed lines hidden).", stats.changed()),
            ));
        } else {
            rows.extend(parsed_rows);
        }

        rows.push(message_row(
            DiffRowKind::Meta,
            format!("── End of {} ──", file.path),
        ));

        let end_row = rows.len();
        file_ranges.push(FileRowRange {
            path: file.path.clone(),
            status: file.status,
            start_row,
            end_row,
        });
    }

    if rows.is_empty() {
        rows.push(message_row(DiffRowKind::Empty, "No changed files."));
    } else {
        rows.push(message_row(DiffRowKind::Meta, "── End of change set ──"));
        rows.push(message_row(
            DiffRowKind::Empty,
            "You are at the bottom of the diff stream.",
        ));
        for _ in 0..DIFF_FOOTER_SPACER_ROWS {
            rows.push(message_row(DiffRowKind::Empty, ""));
        }
    }

    Ok(DiffStream {
        rows,
        file_ranges,
        file_line_stats,
    })
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
