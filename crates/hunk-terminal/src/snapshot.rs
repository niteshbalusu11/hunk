#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalNamedColorSnapshot {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Foreground,
    Background,
    Cursor,
    DimBlack,
    DimRed,
    DimGreen,
    DimYellow,
    DimBlue,
    DimMagenta,
    DimCyan,
    DimWhite,
    BrightForeground,
    DimForeground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColorSnapshot {
    Named(TerminalNamedColorSnapshot),
    Indexed(u8),
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCursorShapeSnapshot {
    Hidden,
    Block,
    Underline,
    Beam,
    HollowBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalScroll {
    Delta(i32),
    PageUp,
    PageDown,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCursorSnapshot {
    pub line: i32,
    pub column: usize,
    pub shape: TerminalCursorShapeSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalModeSnapshot {
    pub alt_screen: bool,
    pub app_cursor: bool,
    pub app_keypad: bool,
    pub show_cursor: bool,
    pub line_wrap: bool,
    pub bracketed_paste: bool,
    pub focus_in_out: bool,
    pub mouse_mode: bool,
    pub mouse_motion: bool,
    pub mouse_drag: bool,
    pub sgr_mouse: bool,
    pub utf8_mouse: bool,
    pub alternate_scroll: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalDamageLineSnapshot {
    pub line: usize,
    pub left: usize,
    pub right: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalDamageSnapshot {
    Full,
    Partial(Vec<TerminalDamageLineSnapshot>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCellSnapshot {
    pub line: i32,
    pub column: usize,
    pub character: char,
    pub fg: TerminalColorSnapshot,
    pub bg: TerminalColorSnapshot,
    pub flags: u16,
    pub zerowidth: Vec<char>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalScreenSnapshot {
    pub rows: u16,
    pub cols: u16,
    pub display_offset: usize,
    pub cursor: TerminalCursorSnapshot,
    pub mode: TerminalModeSnapshot,
    pub damage: TerminalDamageSnapshot,
    pub cells: Vec<TerminalCellSnapshot>,
}
