use std::sync::Arc;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{
    Config, RenderableContent, RenderableCursor, Term, TermDamage, TermMode,
};
use alacritty_terminal::vte::ansi::{
    Color as AlacrittyColor, CursorShape as AlacrittyCursorShape,
    NamedColor as AlacrittyNamedColor, Processor, Rgb, StdSyncHandler,
};

use crate::snapshot::{
    TerminalCellSnapshot, TerminalColorSnapshot, TerminalCursorShapeSnapshot,
    TerminalCursorSnapshot, TerminalDamageLineSnapshot, TerminalDamageSnapshot,
    TerminalModeSnapshot, TerminalNamedColorSnapshot, TerminalScreenSnapshot, TerminalScroll,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalDimensions {
    rows: usize,
    cols: usize,
}

impl TerminalDimensions {
    fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows: rows.max(1) as usize,
            cols: cols.max(1) as usize,
        }
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        *self = Self::new(rows, cols);
    }

    fn rows_u16(&self) -> u16 {
        self.rows as u16
    }

    fn cols_u16(&self) -> u16 {
        self.cols as u16
    }
}

impl Dimensions for TerminalDimensions {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub(crate) struct TerminalVt {
    dimensions: TerminalDimensions,
    parser: Processor<StdSyncHandler>,
    term: Term<VoidListener>,
}

impl TerminalVt {
    pub(crate) fn new(rows: u16, cols: u16) -> Self {
        let dimensions = TerminalDimensions::new(rows, cols);
        let term = Term::new(Config::default(), &dimensions, VoidListener);
        Self {
            dimensions,
            parser: Processor::new(),
            term,
        }
    }

    pub(crate) fn snapshot(&mut self) -> Arc<TerminalScreenSnapshot> {
        Arc::new(build_snapshot(&mut self.term, self.dimensions))
    }

    pub(crate) fn advance(&mut self, bytes: &[u8]) -> Arc<TerminalScreenSnapshot> {
        self.parser.advance(&mut self.term, bytes);
        self.snapshot()
    }

    pub(crate) fn resize(&mut self, rows: u16, cols: u16) -> Arc<TerminalScreenSnapshot> {
        self.dimensions.resize(rows, cols);
        self.term.resize(self.dimensions);
        self.snapshot()
    }

    pub(crate) fn scroll_display(&mut self, scroll: TerminalScroll) -> Arc<TerminalScreenSnapshot> {
        self.term.scroll_display(match scroll {
            TerminalScroll::Delta(count) => Scroll::Delta(count),
            TerminalScroll::PageUp => Scroll::PageUp,
            TerminalScroll::PageDown => Scroll::PageDown,
            TerminalScroll::Top => Scroll::Top,
            TerminalScroll::Bottom => Scroll::Bottom,
        });
        self.snapshot()
    }
}

fn build_snapshot(
    term: &mut Term<VoidListener>,
    dimensions: TerminalDimensions,
) -> TerminalScreenSnapshot {
    let damage = snapshot_damage(term.damage());
    let content = term.renderable_content();
    let cursor = snapshot_cursor(content.cursor);
    let mode = snapshot_mode(content.mode);
    let display_offset = content.display_offset;
    let cells = snapshot_cells(content);
    term.reset_damage();

    TerminalScreenSnapshot {
        rows: dimensions.rows_u16(),
        cols: dimensions.cols_u16(),
        display_offset,
        cursor,
        mode,
        damage,
        cells,
    }
}

fn snapshot_damage(damage: TermDamage<'_>) -> TerminalDamageSnapshot {
    match damage {
        TermDamage::Full => TerminalDamageSnapshot::Full,
        TermDamage::Partial(lines) => TerminalDamageSnapshot::Partial(
            lines
                .map(|line| TerminalDamageLineSnapshot {
                    line: line.line,
                    left: line.left,
                    right: line.right,
                })
                .collect(),
        ),
    }
}

fn snapshot_cells(content: RenderableContent<'_>) -> Vec<TerminalCellSnapshot> {
    content
        .display_iter
        .map(|indexed_cell| {
            snapshot_cell(
                indexed_cell.point.line.0,
                indexed_cell.point.column.0,
                indexed_cell.cell,
            )
        })
        .collect()
}

fn snapshot_cell(line: i32, column: usize, cell: &Cell) -> TerminalCellSnapshot {
    TerminalCellSnapshot {
        line,
        column,
        character: cell.c,
        fg: snapshot_color(cell.fg),
        bg: snapshot_color(cell.bg),
        flags: snapshot_flags(cell.flags),
        zerowidth: cell.zerowidth().map(ToOwned::to_owned).unwrap_or_default(),
    }
}

fn snapshot_color(color: AlacrittyColor) -> TerminalColorSnapshot {
    match color {
        AlacrittyColor::Named(named) => TerminalColorSnapshot::Named(snapshot_named_color(named)),
        AlacrittyColor::Indexed(index) => TerminalColorSnapshot::Indexed(index),
        AlacrittyColor::Spec(Rgb { r, g, b }) => TerminalColorSnapshot::Rgb { r, g, b },
    }
}

fn snapshot_named_color(color: AlacrittyNamedColor) -> TerminalNamedColorSnapshot {
    match color {
        AlacrittyNamedColor::Black => TerminalNamedColorSnapshot::Black,
        AlacrittyNamedColor::Red => TerminalNamedColorSnapshot::Red,
        AlacrittyNamedColor::Green => TerminalNamedColorSnapshot::Green,
        AlacrittyNamedColor::Yellow => TerminalNamedColorSnapshot::Yellow,
        AlacrittyNamedColor::Blue => TerminalNamedColorSnapshot::Blue,
        AlacrittyNamedColor::Magenta => TerminalNamedColorSnapshot::Magenta,
        AlacrittyNamedColor::Cyan => TerminalNamedColorSnapshot::Cyan,
        AlacrittyNamedColor::White => TerminalNamedColorSnapshot::White,
        AlacrittyNamedColor::BrightBlack => TerminalNamedColorSnapshot::BrightBlack,
        AlacrittyNamedColor::BrightRed => TerminalNamedColorSnapshot::BrightRed,
        AlacrittyNamedColor::BrightGreen => TerminalNamedColorSnapshot::BrightGreen,
        AlacrittyNamedColor::BrightYellow => TerminalNamedColorSnapshot::BrightYellow,
        AlacrittyNamedColor::BrightBlue => TerminalNamedColorSnapshot::BrightBlue,
        AlacrittyNamedColor::BrightMagenta => TerminalNamedColorSnapshot::BrightMagenta,
        AlacrittyNamedColor::BrightCyan => TerminalNamedColorSnapshot::BrightCyan,
        AlacrittyNamedColor::BrightWhite => TerminalNamedColorSnapshot::BrightWhite,
        AlacrittyNamedColor::Foreground => TerminalNamedColorSnapshot::Foreground,
        AlacrittyNamedColor::Background => TerminalNamedColorSnapshot::Background,
        AlacrittyNamedColor::Cursor => TerminalNamedColorSnapshot::Cursor,
        AlacrittyNamedColor::DimBlack => TerminalNamedColorSnapshot::DimBlack,
        AlacrittyNamedColor::DimRed => TerminalNamedColorSnapshot::DimRed,
        AlacrittyNamedColor::DimGreen => TerminalNamedColorSnapshot::DimGreen,
        AlacrittyNamedColor::DimYellow => TerminalNamedColorSnapshot::DimYellow,
        AlacrittyNamedColor::DimBlue => TerminalNamedColorSnapshot::DimBlue,
        AlacrittyNamedColor::DimMagenta => TerminalNamedColorSnapshot::DimMagenta,
        AlacrittyNamedColor::DimCyan => TerminalNamedColorSnapshot::DimCyan,
        AlacrittyNamedColor::DimWhite => TerminalNamedColorSnapshot::DimWhite,
        AlacrittyNamedColor::BrightForeground => TerminalNamedColorSnapshot::BrightForeground,
        AlacrittyNamedColor::DimForeground => TerminalNamedColorSnapshot::DimForeground,
    }
}

fn snapshot_flags(flags: Flags) -> u16 {
    flags.bits()
}

fn snapshot_cursor(cursor: RenderableCursor) -> TerminalCursorSnapshot {
    TerminalCursorSnapshot {
        line: cursor.point.line.0,
        column: cursor.point.column.0,
        shape: snapshot_cursor_shape(cursor.shape),
    }
}

fn snapshot_cursor_shape(shape: AlacrittyCursorShape) -> TerminalCursorShapeSnapshot {
    match shape {
        AlacrittyCursorShape::Hidden => TerminalCursorShapeSnapshot::Hidden,
        AlacrittyCursorShape::Block => TerminalCursorShapeSnapshot::Block,
        AlacrittyCursorShape::Underline => TerminalCursorShapeSnapshot::Underline,
        AlacrittyCursorShape::Beam => TerminalCursorShapeSnapshot::Beam,
        AlacrittyCursorShape::HollowBlock => TerminalCursorShapeSnapshot::HollowBlock,
    }
}

fn snapshot_mode(mode: TermMode) -> TerminalModeSnapshot {
    TerminalModeSnapshot {
        alt_screen: mode.contains(TermMode::ALT_SCREEN),
        app_cursor: mode.contains(TermMode::APP_CURSOR),
        app_keypad: mode.contains(TermMode::APP_KEYPAD),
        show_cursor: mode.contains(TermMode::SHOW_CURSOR),
        line_wrap: mode.contains(TermMode::LINE_WRAP),
        bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
        focus_in_out: mode.contains(TermMode::FOCUS_IN_OUT),
        mouse_mode: mode.intersects(TermMode::MOUSE_MODE),
        mouse_motion: mode.contains(TermMode::MOUSE_MOTION),
        mouse_drag: mode.contains(TermMode::MOUSE_DRAG),
        sgr_mouse: mode.contains(TermMode::SGR_MOUSE),
        utf8_mouse: mode.contains(TermMode::UTF8_MOUSE),
        alternate_scroll: mode.contains(TermMode::ALTERNATE_SCROLL),
    }
}
