mod backend;
mod input;
mod runtime;
mod snapshot;

pub use input::{
    TerminalGridPoint, TerminalInputModifiers, TerminalKeyInput, TerminalKeystroke,
    TerminalMouseButton, TerminalPointerInput, TerminalWheelInput, terminal_focus_input_bytes,
    terminal_key_input, terminal_keystroke_input_bytes, terminal_mouse_button_input,
    terminal_mouse_move_input, terminal_paste_input_bytes, terminal_wheel_input,
};
pub use runtime::{
    TerminalEvent, TerminalSessionHandle, TerminalSpawnRequest, spawn_terminal_session,
};
pub use snapshot::{
    TerminalCellSnapshot, TerminalColorSnapshot, TerminalCursorShapeSnapshot,
    TerminalCursorSnapshot, TerminalDamageLineSnapshot, TerminalDamageSnapshot,
    TerminalModeSnapshot, TerminalNamedColorSnapshot, TerminalScreenSnapshot, TerminalScroll,
};
