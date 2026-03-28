#[allow(
    dead_code,
    reason = "Alacritty stays in-tree as temporary migration scaffolding until the Ghostty cutover is fully validated"
)]
mod alacritty;
mod ghostty;

pub(crate) use ghostty::GhosttyTerminalVt as TerminalVt;
