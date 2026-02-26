use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write as _;
use std::path::Path;

use anyhow::{Result, anyhow};

use super::highlight::{
    StyledSegment, SyntaxTokenKind, build_line_segments, render_with_whitespace_markers,
};
use super::*;
use hunk::diff::parse_patch_side_by_side;
use hunk::jj::{
    JjRepo, RepoTreeEntry, RepoTreeEntryKind, load_patch_from_open_repo, open_repo_for_patch,
};

#[derive(Default)]
struct DiffTreeFolder {
    folders: BTreeMap<String, DiffTreeFolder>,
    files: BTreeMap<String, FileStatus>,
}

#[derive(Default)]
struct RepoTreeFolder {
    ignored: bool,
    folders: BTreeMap<String, RepoTreeFolder>,
    files: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SidebarTreeMode {
    Diff,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RightPaneMode {
    Diff,
    FileEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepoTreeNodeKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepoTreeNode {
    pub(super) path: String,
    pub(super) name: String,
    pub(super) kind: RepoTreeNodeKind,
    pub(super) ignored: bool,
    pub(super) children: Vec<RepoTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepoTreeRow {
    pub(super) path: String,
    pub(super) name: String,
    pub(super) kind: RepoTreeNodeKind,
    pub(super) ignored: bool,
    pub(super) depth: usize,
    pub(super) expanded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileEditorDocument {
    pub(super) text: String,
    pub(super) byte_len: usize,
    pub(super) language: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CachedStyledSegment {
    pub(super) plain_text: String,
    pub(super) whitespace_text: String,
    pub(super) syntax: SyntaxTokenKind,
    pub(super) changed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct DiffRowSegmentCache {
    pub(super) left: Vec<CachedStyledSegment>,
    pub(super) right: Vec<CachedStyledSegment>,
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
    FileHeader,
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
    pub(super) row_segments: BTreeMap<u64, DiffRowSegmentCache>,
    pub(super) file_ranges: Vec<FileRowRange>,
    pub(super) file_line_stats: BTreeMap<String, LineStats>,
}

struct LoadedFileDiffRows {
    core_rows: Vec<SideBySideRow>,
    stats: LineStats,
    load_error: Option<String>,
}

pub(super) fn build_tree_items(files: &[ChangedFile]) -> Vec<TreeItem> {
    let mut root = DiffTreeFolder::default();

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

fn build_folder_items(folder: &DiffTreeFolder, prefix: &str) -> Vec<TreeItem> {
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

pub(super) fn build_repo_tree(entries: &[RepoTreeEntry]) -> Vec<RepoTreeNode> {
    let mut root = RepoTreeFolder::default();

    for entry in entries {
        let mut parts = entry.path.split('/').peekable();
        let mut cursor = &mut root;
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                cursor = cursor.folders.entry(part.to_string()).or_default();
                continue;
            }

            match entry.kind {
                RepoTreeEntryKind::Directory => {
                    let folder = cursor.folders.entry(part.to_string()).or_default();
                    folder.ignored = entry.ignored;
                }
                RepoTreeEntryKind::File => {
                    cursor.files.insert(part.to_string(), entry.ignored);
                }
            }
        }
    }

    build_repo_tree_nodes(&root, "")
}

pub(super) fn flatten_repo_tree_rows(
    nodes: &[RepoTreeNode],
    expanded_dirs: &BTreeSet<String>,
) -> Vec<RepoTreeRow> {
    let mut rows = Vec::new();
    append_repo_tree_rows(nodes, expanded_dirs, 0, &mut rows);
    rows
}

pub(super) fn load_file_editor_document(
    repo_root: &Path,
    file_path: &str,
    max_bytes: usize,
) -> Result<FileEditorDocument> {
    let absolute_path = repo_root.join(file_path);
    let bytes = fs::read(&absolute_path)
        .map_err(|err| anyhow!("failed to read {}: {err}", absolute_path.display()))?;
    if bytes.len() > max_bytes {
        return Err(anyhow!(
            "file is too large to edit ({} bytes, max {})",
            bytes.len(),
            max_bytes
        ));
    }
    if is_probably_binary_bytes(&bytes) {
        return Err(anyhow!("binary file editing is not supported"));
    }

    let text = String::from_utf8(bytes)
        .map_err(|_| anyhow!("file is not UTF-8 text and cannot be edited"))?;

    Ok(FileEditorDocument {
        byte_len: text.len(),
        language: editor_language_hint(file_path),
        text,
    })
}

pub(super) fn save_file_editor_document(
    repo_root: &Path,
    file_path: &str,
    text: &str,
) -> Result<()> {
    let absolute_path = repo_root.join(file_path);
    let parent = absolute_path.parent().ok_or_else(|| {
        anyhow!(
            "cannot save {}: resolved path has no parent",
            absolute_path.display()
        )
    })?;

    let mut temp_name = absolute_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("hunk-save")
        .to_string();
    temp_name.push_str(".hunk-tmp.");
    temp_name.push_str(&std::process::id().to_string());
    temp_name.push('.');
    temp_name.push_str(
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string(),
    );

    let temp_path = parent.join(temp_name);
    let mut temp_file =
        fs::File::create(&temp_path).map_err(|err| anyhow!("failed to create temp file: {err}"))?;
    temp_file
        .write_all(text.as_bytes())
        .map_err(|err| anyhow!("failed to write temp file {}: {err}", temp_path.display()))?;
    temp_file
        .sync_all()
        .map_err(|err| anyhow!("failed to fsync temp file {}: {err}", temp_path.display()))?;
    drop(temp_file);

    if let Err(err) = fs::rename(&temp_path, &absolute_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(anyhow!(
            "failed to move {} into place: {err}",
            absolute_path.display()
        ));
    }

    if let Ok(dir_handle) = fs::File::open(parent) {
        let _ = dir_handle.sync_all();
    }

    Ok(())
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

fn build_repo_tree_nodes(folder: &RepoTreeFolder, prefix: &str) -> Vec<RepoTreeNode> {
    let mut nodes = Vec::new();

    for (name, child_folder) in &folder.folders {
        let path = join_path(prefix, name);
        nodes.push(RepoTreeNode {
            path: path.clone(),
            name: name.clone(),
            kind: RepoTreeNodeKind::Directory,
            ignored: child_folder.ignored,
            children: build_repo_tree_nodes(child_folder, &path),
        });
    }

    for (name, ignored) in &folder.files {
        let path = join_path(prefix, name);
        nodes.push(RepoTreeNode {
            path,
            name: name.clone(),
            kind: RepoTreeNodeKind::File,
            ignored: *ignored,
            children: Vec::new(),
        });
    }

    nodes
}

fn append_repo_tree_rows(
    nodes: &[RepoTreeNode],
    expanded_dirs: &BTreeSet<String>,
    depth: usize,
    rows: &mut Vec<RepoTreeRow>,
) {
    for node in nodes {
        let expanded =
            node.kind == RepoTreeNodeKind::Directory && expanded_dirs.contains(node.path.as_str());
        rows.push(RepoTreeRow {
            path: node.path.clone(),
            name: node.name.clone(),
            kind: node.kind,
            ignored: node.ignored,
            depth,
            expanded,
        });

        if expanded && node.kind == RepoTreeNodeKind::Directory {
            append_repo_tree_rows(&node.children, expanded_dirs, depth + 1, rows);
        }
    }
}

fn is_probably_binary_bytes(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

fn editor_language_hint(file_path: &str) -> String {
    let path = Path::new(file_path);

    if let Some(name) = path.file_name().and_then(|file| file.to_str()) {
        match name {
            "Dockerfile" => return "text".to_string(),
            "Makefile" => return "make".to_string(),
            "CMakeLists.txt" => return "cmake".to_string(),
            ".zshrc" | ".bashrc" | ".bash_profile" => return "bash".to_string(),
            "Cargo.toml" | "Cargo.lock" => return "toml".to_string(),
            _ => {}
        }
    }

    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        let extension = extension.to_ascii_lowercase();
        let language = match extension.as_str() {
            "rs" => "rust",
            "toml" => "toml",
            "js" | "mjs" | "cjs" | "jsx" => "javascript",
            "tsx" => "tsx",
            "ts" => "typescript",
            "json" | "jsonc" => "json",
            "yaml" | "yml" => "yaml",
            "md" | "markdown" | "mdx" => "markdown",
            "py" => "python",
            "rb" => "ruby",
            "go" => "go",
            "java" => "java",
            "swift" => "swift",
            "c" | "h" => "c",
            "cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" => "cpp",
            "cs" => "csharp",
            "cmake" => "cmake",
            "graphql" | "gql" => "graphql",
            "bash" | "sh" | "zsh" => "bash",
            "html" | "htm" => "html",
            "css" | "scss" | "sass" => "css",
            "ejs" => "ejs",
            "erb" => "erb",
            "ex" | "exs" => "elixir",
            "sql" => "sql",
            "proto" => "proto",
            "scala" => "scala",
            "zig" => "zig",
            "diff" | "patch" => "diff",
            "lock" => "toml",
            _ => "text",
        };
        return language.to_string();
    }

    "text".to_string()
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
    previous_file_line_stats: &BTreeMap<String, LineStats>,
) -> Result<DiffStream> {
    let mut rows = Vec::new();
    let mut row_metadata = Vec::new();
    let mut row_segments = BTreeMap::new();
    let mut file_ranges = Vec::with_capacity(files.len());
    let mut file_line_stats = BTreeMap::new();
    let repo = open_repo_for_patch(repo_root)?;

    for file in files {
        let start_row = rows.len();
        let mut file_row_ordinal = 0_usize;
        push_stream_row(
            &mut rows,
            &mut row_metadata,
            message_row(DiffRowKind::Meta, file.path.clone()),
            DiffStreamRowKind::FileHeader,
            Some(file.path.as_str()),
            Some(file.status),
            file_row_ordinal,
        );
        file_row_ordinal = file_row_ordinal.saturating_add(1);

        if collapsed_files.contains(file.path.as_str()) {
            let collapsed_stats = previous_file_line_stats
                .get(file.path.as_str())
                .copied()
                .unwrap_or_default();
            file_line_stats.insert(file.path.clone(), collapsed_stats);
            let collapsed_message = if collapsed_stats.changed() > 0 {
                format!(
                    "File collapsed ({} changed lines hidden).",
                    collapsed_stats.changed()
                )
            } else {
                "File collapsed. Expand to load its diff.".to_string()
            };
            push_stream_row(
                &mut rows,
                &mut row_metadata,
                message_row(DiffRowKind::Empty, collapsed_message),
                DiffStreamRowKind::FileCollapsed,
                Some(file.path.as_str()),
                Some(file.status),
                file_row_ordinal,
            );
        } else {
            let loaded_file = load_file_diff_rows(&repo, file);
            file_line_stats.insert(file.path.clone(), loaded_file.stats);
            if let Some(load_error) = loaded_file.load_error {
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
                for row in loaded_file.core_rows.into_iter().filter(|row| {
                    matches!(
                        row.kind,
                        DiffRowKind::Code | DiffRowKind::HunkHeader | DiffRowKind::Empty
                    )
                }) {
                    let row_kind = stream_kind_for_core_row(&row);
                    let stable_id = push_stream_row(
                        &mut rows,
                        &mut row_metadata,
                        row.clone(),
                        row_kind,
                        Some(file.path.as_str()),
                        Some(file.status),
                        file_row_ordinal,
                    );
                    if row.kind == DiffRowKind::Code {
                        row_segments.insert(
                            stable_id,
                            build_diff_row_segment_cache(Some(file.path.as_str()), &row),
                        );
                    }
                    file_row_ordinal = file_row_ordinal.saturating_add(1);
                }
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
        row_segments,
        file_ranges,
        file_line_stats,
    })
}

fn load_file_diff_rows(repo: &JjRepo, file: &ChangedFile) -> LoadedFileDiffRows {
    if is_probably_binary_extension(file.path.as_str()) {
        return LoadedFileDiffRows {
            core_rows: Vec::new(),
            stats: LineStats::default(),
            load_error: Some(format!(
                "Preview unavailable for {}: binary file type.",
                file.path
            )),
        };
    }

    match load_patch_from_open_repo(repo, &file.path, file.status) {
        Ok(patch) => {
            if is_binary_patch(patch.as_str()) {
                return LoadedFileDiffRows {
                    core_rows: Vec::new(),
                    stats: LineStats::default(),
                    load_error: Some(format!(
                        "Preview unavailable for {}: binary diff.",
                        file.path
                    )),
                };
            }

            let core_rows = parse_patch_side_by_side(&patch);
            if patch_has_unrenderable_text_diff(patch.as_str(), &core_rows) {
                return LoadedFileDiffRows {
                    core_rows: Vec::new(),
                    stats: LineStats::default(),
                    load_error: Some(format!(
                        "Preview unavailable for {}: unsupported diff format.",
                        file.path
                    )),
                };
            }

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

pub(super) fn cached_segments_from_styled(
    segments: Vec<StyledSegment>,
) -> Vec<CachedStyledSegment> {
    segments
        .into_iter()
        .map(|segment| {
            let plain_text = segment.text;
            let whitespace_text = render_with_whitespace_markers(plain_text.as_str());
            CachedStyledSegment {
                plain_text,
                whitespace_text,
                syntax: segment.syntax,
                changed: segment.changed,
            }
        })
        .collect::<Vec<_>>()
}

fn build_diff_row_segment_cache(
    file_path: Option<&str>,
    row: &SideBySideRow,
) -> DiffRowSegmentCache {
    let left = cached_segments_from_styled(build_line_segments(
        file_path,
        &row.left.text,
        row.left.kind,
        &row.right.text,
        row.right.kind,
    ));
    let right = cached_segments_from_styled(build_line_segments(
        file_path,
        &row.right.text,
        row.right.kind,
        &row.left.text,
        row.left.kind,
    ));

    DiffRowSegmentCache { left, right }
}

fn is_probably_binary_extension(path: &str) -> bool {
    let Some(extension) = Path::new(path).extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    let extension = extension.to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "7z" | "a"
            | "apk"
            | "bin"
            | "bmp"
            | "class"
            | "dll"
            | "dmg"
            | "doc"
            | "docx"
            | "ear"
            | "eot"
            | "exe"
            | "gif"
            | "gz"
            | "ico"
            | "jar"
            | "jpeg"
            | "jpg"
            | "lib"
            | "lockb"
            | "mov"
            | "mp3"
            | "mp4"
            | "o"
            | "obj"
            | "otf"
            | "pdf"
            | "png"
            | "pyc"
            | "so"
            | "tar"
            | "tif"
            | "tiff"
            | "ttf"
            | "war"
            | "wasm"
            | "webm"
            | "webp"
            | "woff"
            | "woff2"
            | "xls"
            | "xlsx"
            | "zip"
    )
}

fn is_binary_patch(patch: &str) -> bool {
    patch.contains('\0')
        || patch.contains("\nGIT binary patch\n")
        || patch
            .lines()
            .any(|line| line.starts_with("Binary files ") && line.ends_with(" differ"))
}

fn patch_has_unrenderable_text_diff(patch: &str, rows: &[SideBySideRow]) -> bool {
    !patch.trim().is_empty()
        && !rows.is_empty()
        && rows.iter().all(|row| {
            row.kind == DiffRowKind::Empty
                && row.left.kind == DiffCellKind::None
                && row.right.kind == DiffCellKind::None
        })
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
) -> u64 {
    let stable_id = compute_stable_row_id(file_path, kind, ordinal, &row);
    rows.push(row);
    row_metadata.push(DiffStreamRowMeta {
        stable_id,
        file_path: file_path.map(ToString::to_string),
        file_status,
        kind,
    });
    stable_id
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
        DiffStreamRowKind::FileHeader => "file-header",
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
        let first = compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreMeta, 0, &row);
        let second =
            compute_stable_row_id(Some("src/lib.rs"), DiffStreamRowKind::CoreMeta, 1, &row);

        assert_ne!(first, second);
    }

    #[test]
    fn editor_language_hint_maps_rust_and_ts() {
        assert_eq!(editor_language_hint("src/main.rs"), "rust");
        assert_eq!(editor_language_hint("web/app.ts"), "typescript");
        assert_eq!(editor_language_hint("web/app.tsx"), "tsx");
        assert_eq!(editor_language_hint("web/app.jsx"), "javascript");
    }

    #[test]
    fn editor_language_hint_uses_filename_for_special_cases() {
        assert_eq!(editor_language_hint("Dockerfile"), "text");
        assert_eq!(editor_language_hint("Cargo.lock"), "toml");
        assert_eq!(editor_language_hint("CMakeLists.txt"), "cmake");
    }

    #[test]
    fn editor_language_hint_falls_back_to_text_for_unknown_extensions() {
        assert_eq!(editor_language_hint("docs/schema.xml"), "text");
    }
}
