use std::cell::RefCell;
use std::cmp::min;
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Result;
use gpui::*;
use hunk_editor::{DisplayRow, DisplayRowKind, EditorCommand, EditorState, Viewport};
use hunk_language::{LanguageRegistry, SyntaxSession};
use hunk_text::{BufferId, Selection, TextBuffer, TextPosition};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollDirection {
    Forward,
    Backward,
}

pub(crate) type SharedFilesEditor = Rc<RefCell<FilesEditor>>;

#[derive(Clone)]
pub(crate) struct FilesEditorStatusSnapshot {
    pub(crate) mode: &'static str,
    pub(crate) language: String,
    pub(crate) position: String,
    pub(crate) selection: String,
}

pub(crate) struct FilesEditor {
    editor: EditorState,
    registry: LanguageRegistry,
    syntax: SyntaxSession,
    next_buffer_id: u64,
    active_path: Option<PathBuf>,
    view_state_by_path: BTreeMap<PathBuf, FilesEditorViewState>,
    language_label: String,
    drag_anchor: Option<TextPosition>,
}

#[derive(Clone)]
pub(crate) struct FilesEditorElement {
    state: SharedFilesEditor,
    is_focused: bool,
    style: TextStyle,
    palette: FilesEditorPalette,
}

#[derive(Clone, Copy)]
pub(crate) struct FilesEditorPalette {
    pub(crate) background: Hsla,
    pub(crate) active_line_background: Hsla,
    pub(crate) line_number: Hsla,
    pub(crate) current_line_number: Hsla,
    pub(crate) border: Hsla,
    pub(crate) default_foreground: Hsla,
    pub(crate) muted_foreground: Hsla,
    pub(crate) selection_background: Hsla,
    pub(crate) cursor: Hsla,
}

#[derive(Clone)]
pub struct EditorLayout {
    line_height: Pixels,
    font_size: Pixels,
    cell_width: Pixels,
    gutter_columns: usize,
    hitbox: Hitbox,
    display_snapshot: hunk_editor::DisplaySnapshot,
}

impl EditorLayout {
    fn content_origin_x(&self) -> Pixels {
        self.hitbox.bounds.origin.x
            + px(10.0)
            + (self.cell_width * (self.gutter_columns as f32 + 1.0))
    }
}

#[derive(Clone)]
struct LineNumberPaintParams {
    origin: Point<Pixels>,
    current_line: usize,
    palette: FilesEditorPalette,
    font: Font,
}

#[derive(Clone, Copy)]
struct FilesEditorViewState {
    selection: Selection,
    viewport: Viewport,
}

impl FilesEditor {
    pub(crate) fn new() -> Self {
        Self {
            editor: EditorState::new(TextBuffer::new(BufferId::new(1), "")),
            registry: LanguageRegistry::builtin(),
            syntax: SyntaxSession::new(),
            next_buffer_id: 2,
            active_path: None,
            view_state_by_path: BTreeMap::new(),
            language_label: "text".to_string(),
            drag_anchor: None,
        }
    }

    pub(crate) fn open_document(&mut self, path: &Path, contents: &str) -> Result<()> {
        self.capture_active_view_state();
        let buffer = TextBuffer::new(BufferId::new(self.next_buffer_id), contents);
        self.next_buffer_id = self.next_buffer_id.saturating_add(1);
        self.editor = EditorState::new(buffer);
        self.editor.apply(EditorCommand::SetViewport(Viewport {
            first_visible_row: 0,
            visible_row_count: 1,
            horizontal_offset: 0,
        }));
        let syntax = self.syntax.parse_for_path(&self.registry, path, contents)?;
        self.editor
            .apply(EditorCommand::SetLanguage(syntax.language_id));
        self.editor
            .apply(EditorCommand::SetParseStatus(syntax.parse_status));
        self.active_path = Some(path.to_path_buf());
        self.language_label = self
            .registry
            .language_for_path(path)
            .map(|definition| definition.name.clone())
            .unwrap_or_else(|| "text".to_string());
        self.drag_anchor = None;
        self.restore_view_state(path);
        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        self.active_path = None;
        self.view_state_by_path.clear();
        self.editor = EditorState::new(TextBuffer::new(BufferId::new(self.next_buffer_id), ""));
        self.next_buffer_id = self.next_buffer_id.saturating_add(1);
        self.language_label = "text".to_string();
        self.drag_anchor = None;
    }

    pub(crate) fn shutdown(&mut self) {
        self.clear();
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.editor.is_dirty()
    }

    pub(crate) fn current_text(&self) -> Option<String> {
        self.active_path.as_ref()?;
        Some(self.editor.buffer().text())
    }

    pub(crate) fn status_snapshot(&self) -> Option<FilesEditorStatusSnapshot> {
        self.active_path.as_ref()?;
        let status = self.editor.status_snapshot();
        let selection = self.editor.selection().range();
        Some(FilesEditorStatusSnapshot {
            mode: "EDIT",
            language: self.language_label.clone(),
            position: format!(
                "Ln {}  Col {}  {} lines",
                status.cursor_line, status.cursor_column, status.line_count
            ),
            selection: if selection.is_empty() {
                "1 cursor".to_string()
            } else {
                "Selection".to_string()
            },
        })
    }

    pub(crate) fn mark_saved(&mut self) {
        self.editor.apply(EditorCommand::MarkSaved);
    }

    pub(crate) fn copy_selection_text(&self) -> Option<String> {
        let mut clone = self.editor.clone();
        clone.apply(EditorCommand::CopySelection).copied_text
    }

    pub(crate) fn cut_selection_text(&mut self) -> Option<String> {
        self.active_path.as_ref()?;
        let output = self.editor.apply(EditorCommand::CutSelection);
        output.copied_text
    }

    pub(crate) fn paste_text(&mut self, text: &str) -> bool {
        if text.is_empty() || self.active_path.is_none() {
            return false;
        }

        let output = self.editor.apply(EditorCommand::Paste(text.to_string()));
        output.document_changed || output.selection_changed
    }

    pub(crate) fn sync_theme(&mut self, _is_dark: bool) {}

    pub(crate) fn handle_keystroke(&mut self, keystroke: &Keystroke) -> bool {
        if self.active_path.is_none() {
            return false;
        }

        if self.handle_shortcut(keystroke) {
            return true;
        }

        match keystroke.key.as_str() {
            "left" => self.move_horizontally(false, keystroke.modifiers.shift),
            "right" => self.move_horizontally(true, keystroke.modifiers.shift),
            "up" => self.move_vertically(false, keystroke.modifiers.shift),
            "down" => self.move_vertically(true, keystroke.modifiers.shift),
            "home" => self.move_to_line_boundary(true, keystroke.modifiers.shift),
            "end" => self.move_to_line_boundary(false, keystroke.modifiers.shift),
            "pageup" => {
                self.page_scroll(ScrollDirection::Backward);
                true
            }
            "pagedown" => {
                self.page_scroll(ScrollDirection::Forward);
                true
            }
            "backspace" => {
                self.editor
                    .apply(EditorCommand::DeleteBackward)
                    .document_changed
            }
            "delete" => {
                self.editor
                    .apply(EditorCommand::DeleteForward)
                    .document_changed
            }
            "escape" => self.collapse_selection_to_head(),
            "enter" => self.insert_newline_with_indent(),
            "tab" if !keystroke.modifiers.control && !keystroke.modifiers.platform => {
                self.insert_text("    ")
            }
            _ => self.insert_key_char(keystroke),
        }
    }

    pub(crate) fn scroll_lines(&mut self, line_count: usize, direction: ScrollDirection) {
        let snapshot = self.editor.display_snapshot();
        let max_first_row = snapshot
            .total_display_rows
            .saturating_sub(snapshot.viewport.visible_row_count);
        let next_first_row = match direction {
            ScrollDirection::Backward => snapshot
                .viewport
                .first_visible_row
                .saturating_sub(line_count),
            ScrollDirection::Forward => min(
                snapshot
                    .viewport
                    .first_visible_row
                    .saturating_add(line_count),
                max_first_row,
            ),
        };
        self.editor.apply(EditorCommand::SetViewport(Viewport {
            first_visible_row: next_first_row,
            visible_row_count: snapshot.viewport.visible_row_count,
            horizontal_offset: 0,
        }));
    }

    fn page_scroll(&mut self, direction: ScrollDirection) {
        let snapshot = self.editor.display_snapshot();
        let page = snapshot.viewport.visible_row_count.max(1);
        self.scroll_lines(page, direction);
    }

    fn handle_shortcut(&mut self, keystroke: &Keystroke) -> bool {
        if !uses_primary_shortcut(keystroke) {
            return false;
        }

        match keystroke.key.as_str() {
            "a" if !keystroke.modifiers.shift => self.select_all(),
            "z" if !keystroke.modifiers.shift => {
                self.editor.apply(EditorCommand::Undo).document_changed
            }
            "z" if keystroke.modifiers.shift => {
                self.editor.apply(EditorCommand::Redo).document_changed
            }
            "y" if !cfg!(target_os = "macos") => {
                self.editor.apply(EditorCommand::Redo).document_changed
            }
            _ => false,
        }
    }

    fn move_horizontally(&mut self, forward: bool, extend: bool) -> bool {
        let selection = self.editor.selection();
        if !extend && !selection.is_caret() {
            let target = if forward {
                selection.range().end
            } else {
                selection.range().start
            };
            return self
                .editor
                .apply(EditorCommand::SetSelection(Selection::caret(target)))
                .selection_changed;
        }

        let anchor = selection.anchor;
        let output = if forward {
            self.editor.apply(EditorCommand::MoveRight)
        } else {
            self.editor.apply(EditorCommand::MoveLeft)
        };
        if !extend || !output.selection_changed {
            return output.selection_changed;
        }

        let head = self.editor.selection().head;
        self.editor
            .apply(EditorCommand::SetSelection(Selection::new(anchor, head)))
            .selection_changed
    }

    fn move_vertically(&mut self, forward: bool, extend: bool) -> bool {
        let selection = self.editor.selection();
        if !extend && !selection.is_caret() {
            let target = if forward {
                selection.range().end
            } else {
                selection.range().start
            };
            return self
                .editor
                .apply(EditorCommand::SetSelection(Selection::caret(target)))
                .selection_changed;
        }

        let anchor = selection.anchor;
        let output = if forward {
            self.editor.apply(EditorCommand::MoveDown)
        } else {
            self.editor.apply(EditorCommand::MoveUp)
        };
        if !extend || !output.selection_changed {
            return output.selection_changed;
        }

        let head = self.editor.selection().head;
        self.editor
            .apply(EditorCommand::SetSelection(Selection::new(anchor, head)))
            .selection_changed
    }

    fn move_to_line_boundary(&mut self, start: bool, extend: bool) -> bool {
        let selection = self.editor.selection();
        let snapshot = self.editor.buffer().snapshot();
        let line_text = current_line_text(&snapshot, selection.head.line);
        let column = if start { 0 } else { line_text.chars().count() };
        let target = TextPosition::new(selection.head.line, column);
        let next_selection = if extend {
            Selection::new(selection.anchor, target)
        } else {
            Selection::caret(target)
        };
        self.editor
            .apply(EditorCommand::SetSelection(next_selection))
            .selection_changed
    }

    fn collapse_selection_to_head(&mut self) -> bool {
        let head = self.editor.selection().head;
        self.editor
            .apply(EditorCommand::SetSelection(Selection::caret(head)))
            .selection_changed
    }

    fn select_all(&mut self) -> bool {
        let snapshot = self.editor.buffer().snapshot();
        let Some(end_position) = last_position(&snapshot) else {
            return false;
        };
        self.editor
            .apply(EditorCommand::SetSelection(Selection::new(
                TextPosition::default(),
                end_position,
            )))
            .selection_changed
    }

    fn insert_key_char(&mut self, keystroke: &Keystroke) -> bool {
        if keystroke.modifiers.control || keystroke.modifiers.platform {
            return false;
        }

        let Some(text) = keystroke.key_char.as_deref() else {
            return false;
        };
        if text.is_empty() || matches!(keystroke.key.as_str(), "enter" | "tab") {
            return false;
        }

        self.insert_text(text)
    }

    fn insert_newline_with_indent(&mut self) -> bool {
        let selection = self.editor.selection();
        let snapshot = self.editor.buffer().snapshot();
        let line_text = current_line_text(&snapshot, selection.head.line);
        let indent: String = line_text
            .chars()
            .take_while(|ch| matches!(ch, ' ' | '\t'))
            .collect();
        self.insert_text(format!("\n{indent}").as_str())
    }

    fn insert_text(&mut self, text: &str) -> bool {
        self.editor
            .apply(EditorCommand::InsertText(text.to_string()))
            .document_changed
    }

    fn capture_active_view_state(&mut self) {
        let Some(path) = self.active_path.clone() else {
            return;
        };
        self.view_state_by_path.insert(
            path,
            FilesEditorViewState {
                selection: self.editor.selection(),
                viewport: self.editor.viewport(),
            },
        );
    }

    fn restore_view_state(&mut self, path: &Path) {
        let Some(state) = self.view_state_by_path.get(path).copied() else {
            return;
        };
        self.editor
            .apply(EditorCommand::SetViewport(state.viewport));
        self.editor
            .apply(EditorCommand::SetSelection(state.selection));
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn set_selection_for_test(&mut self, selection: Selection) {
        self.editor.apply(EditorCommand::SetSelection(selection));
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn set_viewport_for_test(&mut self, viewport: Viewport) {
        self.editor.apply(EditorCommand::SetViewport(viewport));
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn selection_for_test(&self) -> Selection {
        self.editor.selection()
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn viewport_for_test(&self) -> Viewport {
        self.editor.viewport()
    }

    fn apply_layout(
        &mut self,
        columns: usize,
        visible_rows: usize,
    ) -> hunk_editor::DisplaySnapshot {
        self.editor
            .apply(EditorCommand::SetWrapWidth(Some(columns.max(1))));
        let viewport = self.editor.viewport();
        self.editor.apply(EditorCommand::SetViewport(Viewport {
            first_visible_row: viewport.first_visible_row,
            visible_row_count: visible_rows.max(1),
            horizontal_offset: 0,
        }));
        self.editor.display_snapshot()
    }

    fn handle_mouse_down(
        &mut self,
        position: Point<Pixels>,
        layout: &EditorLayout,
        shift_held: bool,
    ) -> bool {
        let Some(next_position) = self.position_for_point(position, layout) else {
            return false;
        };
        let anchor = if shift_held {
            self.drag_anchor.unwrap_or(self.editor.selection().anchor)
        } else {
            next_position
        };
        self.drag_anchor = Some(anchor);
        self.editor
            .apply(EditorCommand::SetSelection(Selection::new(
                anchor,
                next_position,
            )));
        true
    }

    fn handle_mouse_drag(&mut self, position: Point<Pixels>, layout: &EditorLayout) -> bool {
        let Some(anchor) = self.drag_anchor else {
            return false;
        };
        let Some(next_position) = self.position_for_point(position, layout) else {
            return false;
        };
        self.editor
            .apply(EditorCommand::SetSelection(Selection::new(
                anchor,
                next_position,
            )));
        true
    }

    fn handle_mouse_up(&mut self) -> bool {
        self.drag_anchor.take().is_some()
    }

    fn position_for_point(
        &self,
        position: Point<Pixels>,
        layout: &EditorLayout,
    ) -> Option<TextPosition> {
        if !layout.hitbox.bounds.contains(&position) {
            return None;
        }
        let row = ((position.y - layout.hitbox.bounds.origin.y) / layout.line_height)
            .floor()
            .max(0.0) as usize;
        let display_row = layout.display_snapshot.visible_rows.get(row)?;
        let display_column = if position.x <= layout.content_origin_x() {
            0
        } else {
            ((position.x - layout.content_origin_x()) / layout.cell_width)
                .floor()
                .max(0.0) as usize
        };
        let raw_column = raw_column_for_display(display_row, display_column);
        Some(TextPosition::new(display_row.source_line, raw_column))
    }
}

impl FilesEditorElement {
    pub(crate) fn new(
        state: SharedFilesEditor,
        is_focused: bool,
        style: TextStyle,
        palette: FilesEditorPalette,
    ) -> Self {
        Self {
            state,
            is_focused,
            style,
            palette,
        }
    }
}

impl IntoElement for FilesEditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FilesEditorElement {
    type RequestLayoutState = ();
    type PrepaintState = EditorLayout;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = gpui::Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let font_id = window.text_system().resolve_font(&self.style.font());
        let font_size = self.style.font_size.to_pixels(window.rem_size());
        let line_height = self.style.line_height_in_pixels(window.rem_size());
        let cell_width = window
            .text_system()
            .advance(font_id, font_size, 'm')
            .map(|size| size.width)
            .unwrap_or_else(|_| px(8.0));
        let columns = (bounds.size.width / cell_width).floor().max(1.0) as usize;
        let rows = (bounds.size.height / line_height).floor().max(1.0) as usize;

        let gutter_columns = self
            .state
            .borrow()
            .editor
            .display_snapshot()
            .line_count
            .max(1)
            .to_string()
            .len()
            + 1;
        let editor_columns = columns.saturating_sub(gutter_columns + 2).max(1);
        let display_snapshot = self.state.borrow_mut().apply_layout(editor_columns, rows);

        EditorLayout {
            line_height,
            font_size,
            cell_width,
            gutter_columns,
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
            display_snapshot,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        layout: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mouse_down_layout = layout.clone();
        let mouse_drag_layout = layout.clone();
        let mouse_state = self.state.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, _cx| {
            if phase == DispatchPhase::Bubble
                && event.button == gpui::MouseButton::Left
                && mouse_down_layout.hitbox.is_hovered(window)
                && mouse_state.borrow_mut().handle_mouse_down(
                    event.position,
                    &mouse_down_layout,
                    event.modifiers.shift,
                )
            {
                window.refresh();
            }
        });
        let mouse_state = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _cx| {
            if phase == DispatchPhase::Bubble
                && event.dragging()
                && mouse_state
                    .borrow_mut()
                    .handle_mouse_drag(event.position, &mouse_drag_layout)
            {
                window.refresh();
            }
        });
        let mouse_state = self.state.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, _cx| {
            if phase == DispatchPhase::Bubble
                && event.button == gpui::MouseButton::Left
                && mouse_state.borrow_mut().handle_mouse_up()
            {
                window.refresh();
            }
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            window.paint_quad(fill(bounds, self.palette.background));

            let content_origin = point(layout.content_origin_x(), bounds.origin.y + px(1.0));
            let gutter_x = bounds.origin.x + (layout.cell_width * layout.gutter_columns as f32);
            window.paint_quad(fill(
                Bounds {
                    origin: point(gutter_x + px(4.0), bounds.origin.y),
                    size: size(px(1.0), bounds.size.height),
                },
                self.palette.border,
            ));

            let selection = self.state.borrow().editor.selection();
            let current_line = selection.head.line;
            let mut row_origin = content_origin;
            for row in &layout.display_snapshot.visible_rows {
                if row.source_line == current_line {
                    window.paint_quad(fill(
                        Bounds {
                            origin: point(bounds.origin.x, row_origin.y),
                            size: size(bounds.size.width, layout.line_height),
                        },
                        self.palette.active_line_background,
                    ));
                }
                if let Some(selection_range) = selection_range_for_row(row, selection) {
                    paint_selection(
                        window,
                        row_origin,
                        layout,
                        selection_range,
                        self.palette.selection_background,
                    );
                }
                for highlight in &row.search_highlights {
                    paint_selection(
                        window,
                        row_origin,
                        layout,
                        highlight.start_column..highlight.end_column,
                        hsla(
                            self.palette.selection_background.h,
                            self.palette.selection_background.s,
                            self.palette.selection_background.l,
                            0.35,
                        ),
                    );
                }

                paint_line_number(
                    window,
                    cx,
                    row,
                    layout,
                    LineNumberPaintParams {
                        origin: row_origin,
                        current_line,
                        palette: self.palette,
                        font: self.style.font(),
                    },
                );

                let row_color = match row.kind {
                    DisplayRowKind::Text => self.palette.default_foreground,
                    DisplayRowKind::FoldPlaceholder { .. } => self.palette.muted_foreground,
                };
                let runs = vec![TextRun {
                    len: row.text.chars().count(),
                    color: row_color,
                    font: self.style.font(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }];
                let line = window.text_system().shape_line(
                    row.text.clone().into(),
                    layout.font_size,
                    &runs,
                    None,
                );
                let _ = line.paint(
                    row_origin,
                    layout.line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
                row_origin.y += layout.line_height;
            }

            if self.is_focused {
                paint_cursor(
                    window,
                    &layout.display_snapshot.visible_rows,
                    selection.head,
                    content_origin,
                    layout,
                    self.palette.cursor,
                );
            }

            if layout.hitbox.is_hovered(window) {
                window.set_cursor_style(CursorStyle::IBeam, &layout.hitbox);
            }
        });
    }
}

pub(crate) fn scroll_direction_and_count(
    event: &ScrollWheelEvent,
    line_height: Pixels,
) -> Option<(ScrollDirection, usize)> {
    let delta = event.delta.pixel_delta(line_height);
    if delta.y.abs() < px(0.5) {
        return None;
    }

    Some((
        if delta.y > Pixels::ZERO {
            ScrollDirection::Backward
        } else {
            ScrollDirection::Forward
        },
        ((delta.y.abs() / line_height).ceil() as usize).max(1),
    ))
}

fn current_line_text(snapshot: &hunk_text::TextSnapshot, line: usize) -> String {
    let start = snapshot.line_to_byte(line).unwrap_or(0);
    let end = if line + 1 < snapshot.line_count() {
        snapshot
            .line_to_byte(line + 1)
            .unwrap_or(snapshot.byte_len())
    } else {
        snapshot.byte_len()
    };
    snapshot
        .slice(start..end)
        .unwrap_or_default()
        .trim_end_matches('\n')
        .to_string()
}

fn last_position(snapshot: &hunk_text::TextSnapshot) -> Option<TextPosition> {
    let line = snapshot.line_count().checked_sub(1)?;
    Some(TextPosition::new(
        line,
        current_line_text(snapshot, line).chars().count(),
    ))
}

fn uses_primary_shortcut(keystroke: &Keystroke) -> bool {
    if cfg!(target_os = "macos") {
        keystroke.modifiers.platform
    } else {
        keystroke.modifiers.control
    }
}

fn paint_line_number(
    window: &mut Window,
    cx: &mut App,
    row: &DisplayRow,
    layout: &EditorLayout,
    params: LineNumberPaintParams,
) {
    let label = if row.start_column == 0 {
        format!("{}", row.source_line + 1)
    } else {
        String::new()
    };
    let color = if row.source_line == params.current_line {
        params.palette.current_line_number
    } else {
        params.palette.line_number
    };
    let runs = vec![TextRun {
        len: label.chars().count(),
        color,
        font: params.font,
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let line = window
        .text_system()
        .shape_line(label.into(), layout.font_size, &runs, None);
    let _ = line.paint(
        point(layout.hitbox.bounds.origin.x + px(2.0), params.origin.y),
        layout.line_height,
        TextAlign::Left,
        None,
        window,
        cx,
    );
}

fn selection_range_for_row(row: &DisplayRow, selection: Selection) -> Option<Range<usize>> {
    let selection = selection.range();
    if selection.is_empty()
        || row.source_line < selection.start.line
        || row.source_line > selection.end.line
    {
        return None;
    }

    let row_start = if row.source_line == selection.start.line {
        selection.start.column.max(row.raw_start_column)
    } else {
        row.raw_start_column
    };
    let row_end = if row.source_line == selection.end.line {
        selection.end.column.min(row.raw_end_column)
    } else {
        row.raw_end_column
    };
    (row_start < row_end)
        .then_some(display_column_for_raw(row, row_start)..display_column_for_raw(row, row_end))
}

fn paint_selection(
    window: &mut Window,
    row_origin: Point<Pixels>,
    layout: &EditorLayout,
    columns: Range<usize>,
    color: Hsla,
) {
    window.paint_quad(fill(
        Bounds {
            origin: point(
                row_origin.x + (layout.cell_width * columns.start as f32),
                row_origin.y,
            ),
            size: size(
                layout.cell_width * columns.end.saturating_sub(columns.start) as f32,
                layout.line_height,
            ),
        },
        color,
    ));
}

fn paint_cursor(
    window: &mut Window,
    rows: &[DisplayRow],
    caret: TextPosition,
    content_origin: Point<Pixels>,
    layout: &EditorLayout,
    color: Hsla,
) {
    if let Some(row) = rows.iter().find(|row| {
        row.source_line == caret.line
            && row.raw_start_column <= caret.column
            && caret.column <= row.raw_end_column
    }) {
        let x = content_origin.x
            + (layout.cell_width * display_column_for_raw(row, caret.column) as f32);
        let y = content_origin.y
            + (layout.line_height * row.row_index.saturating_sub(rows[0].row_index) as f32);
        window.paint_quad(fill(
            Bounds {
                origin: point(x, y),
                size: size(px(1.5), layout.line_height),
            },
            color,
        ));
    }
}

fn display_column_for_raw(row: &DisplayRow, raw_column: usize) -> usize {
    let offset = raw_column.saturating_sub(row.raw_start_column);
    row.raw_column_offsets
        .get(offset)
        .copied()
        .unwrap_or_else(|| row.raw_column_offsets.last().copied().unwrap_or(0))
}

fn raw_column_for_display(row: &DisplayRow, display_column: usize) -> usize {
    let clamped_display = min(display_column, row.text.chars().count());
    let offsets = &row.raw_column_offsets;
    if offsets.is_empty() {
        return row.raw_start_column;
    }

    match offsets.binary_search(&clamped_display) {
        Ok(index) => row.raw_start_column + index,
        Err(0) => row.raw_start_column,
        Err(index) if index >= offsets.len() => row.raw_start_column + offsets.len() - 1,
        Err(index) => {
            let previous_offset = offsets[index - 1];
            let next_offset = offsets[index];
            let snaps_to_next = clamped_display.saturating_sub(previous_offset)
                >= next_offset.saturating_sub(clamped_display);
            row.raw_start_column + if snaps_to_next { index } else { index - 1 }
        }
    }
}
