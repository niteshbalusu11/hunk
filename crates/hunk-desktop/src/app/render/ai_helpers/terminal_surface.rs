use std::ops::Range;

use gpui::{App, Hsla, SharedString, px, rgb};
use hunk_terminal::{
    TerminalColorSnapshot, TerminalCursorShapeSnapshot, TerminalNamedColorSnapshot,
    TerminalScreenSnapshot,
};

#[derive(Debug, Clone)]
struct AiTerminalRenderableLine {
    text: SharedString,
    highlights: Vec<(Range<usize>, gpui::HighlightStyle)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AiTerminalCellStyle {
    color: Hsla,
    background: Hsla,
    underline: Option<gpui::UnderlineStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AiTerminalRenderCell {
    character: char,
    fg: TerminalColorSnapshot,
    bg: TerminalColorSnapshot,
    zerowidth: String,
    cursor: bool,
}

impl DiffViewer {
    fn render_ai_terminal_surface(
        &self,
        state: &AiTerminalPanelState,
        is_dark: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some(screen) = state.screen.as_ref() {
            return self.render_ai_terminal_vt_surface(screen, is_dark, cx);
        }

        if state.has_transcript {
            return v_flex()
                .w_full()
                .gap_0p5()
                .children(
                    state
                        .transcript
                        .lines()
                        .map(|line| {
                            div()
                                .w_full()
                                .text_xs()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(cx.theme().foreground)
                                .whitespace_nowrap()
                                .child(line.to_string())
                                .into_any_element()
                        })
                        .collect::<Vec<_>>(),
                )
                .into_any_element();
        }

        div()
            .w_full()
            .text_xs()
            .font_family(cx.theme().mono_font_family.clone())
            .text_color(cx.theme().muted_foreground)
            .child("Run a command to start a terminal session.")
            .into_any_element()
    }

    fn render_ai_terminal_vt_surface(
        &self,
        screen: &TerminalScreenSnapshot,
        is_dark: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chrome = hunk_editor_chrome_colors(cx.theme(), is_dark);
        let lines = ai_terminal_renderable_lines(screen, is_dark, cx);

        v_flex()
            .w_full()
            .gap_0()
            .bg(chrome.background)
            .children(lines.into_iter().map(|line| {
                let styled_text = if line.highlights.is_empty() {
                    gpui::StyledText::new(line.text.clone())
                } else {
                    gpui::StyledText::new(line.text.clone()).with_highlights(line.highlights)
                };

                div()
                    .w_full()
                    .text_xs()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(chrome.foreground)
                    .whitespace_nowrap()
                    .child(styled_text)
                    .into_any_element()
            }))
            .into_any_element()
    }
}

fn ai_terminal_renderable_lines(
    screen: &TerminalScreenSnapshot,
    is_dark: bool,
    cx: &App,
) -> Vec<AiTerminalRenderableLine> {
    ai_terminal_screen_grid(screen)
        .into_iter()
        .map(|row| ai_terminal_renderable_line(&row, screen, is_dark, cx))
        .collect()
}

fn ai_terminal_screen_grid(screen: &TerminalScreenSnapshot) -> Vec<Vec<AiTerminalRenderCell>> {
    let rows = usize::from(screen.rows.max(1));
    let cols = usize::from(screen.cols.max(1));
    let first_visible_line = screen
        .cells
        .iter()
        .map(|cell| cell.line)
        .min()
        .unwrap_or(screen.cursor.line.max(0));

    let mut grid = vec![
        vec![
            AiTerminalRenderCell {
                character: ' ',
                fg: TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Foreground),
                bg: TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Background),
                zerowidth: String::new(),
                cursor: false,
            };
            cols
        ];
        rows
    ];

    for cell in &screen.cells {
        let relative_line = cell.line - first_visible_line;
        if relative_line < 0 {
            continue;
        }
        let Ok(row_index) = usize::try_from(relative_line) else {
            continue;
        };
        if row_index >= rows || cell.column >= cols {
            continue;
        }

        grid[row_index][cell.column] = AiTerminalRenderCell {
            character: ai_terminal_render_character(cell.character),
            fg: cell.fg,
            bg: cell.bg,
            zerowidth: cell.zerowidth.iter().collect(),
            cursor: false,
        };
    }

    if screen.mode.show_cursor {
        let relative_line = screen.cursor.line - first_visible_line;
        if relative_line >= 0
            && let Ok(row_index) = usize::try_from(relative_line)
            && row_index < rows
            && screen.cursor.column < cols
        {
            grid[row_index][screen.cursor.column].cursor = true;
        }
    }

    grid
}

fn ai_terminal_renderable_line(
    row: &[AiTerminalRenderCell],
    screen: &TerminalScreenSnapshot,
    is_dark: bool,
    cx: &App,
) -> AiTerminalRenderableLine {
    let default_foreground = ai_terminal_snapshot_color(
        TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Foreground),
        is_dark,
        cx,
    );
    let default_background = ai_terminal_snapshot_color(
        TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Background),
        is_dark,
        cx,
    );

    let mut text = String::with_capacity(row.len());
    let mut highlights = Vec::new();
    let mut active_range_start = 0;
    let mut active_style: Option<AiTerminalCellStyle> = None;

    for cell in row {
        let start = text.len();
        text.push(cell.character);
        text.push_str(cell.zerowidth.as_str());

        let style = ai_terminal_cell_style(
            cell,
            screen.cursor.shape,
            default_foreground,
            default_background,
            is_dark,
            cx,
        );

        if Some(style) != active_style {
            if let Some(previous_style) = active_style.take() {
                highlights.push((
                    active_range_start..start,
                    gpui::HighlightStyle {
                        color: Some(previous_style.color),
                        background_color: Some(previous_style.background),
                        underline: previous_style.underline,
                        ..gpui::HighlightStyle::default()
                    },
                ));
            }
            active_range_start = start;
            active_style = Some(style);
        }
    }

    if let Some(style) = active_style {
        highlights.push((
            active_range_start..text.len(),
            gpui::HighlightStyle {
                color: Some(style.color),
                background_color: Some(style.background),
                underline: style.underline,
                ..gpui::HighlightStyle::default()
            },
        ));
    }

    AiTerminalRenderableLine {
        text: text.into(),
        highlights,
    }
}

fn ai_terminal_cell_style(
    cell: &AiTerminalRenderCell,
    cursor_shape: TerminalCursorShapeSnapshot,
    default_foreground: Hsla,
    default_background: Hsla,
    is_dark: bool,
    cx: &App,
) -> AiTerminalCellStyle {
    let mut style = AiTerminalCellStyle {
        color: ai_terminal_snapshot_color(cell.fg, is_dark, cx),
        background: ai_terminal_snapshot_color(cell.bg, is_dark, cx),
        underline: None,
    };

    if cell.cursor {
        let cursor_color = ai_terminal_snapshot_color(
            TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Cursor),
            is_dark,
            cx,
        );

        match cursor_shape {
            TerminalCursorShapeSnapshot::Hidden => {}
            TerminalCursorShapeSnapshot::Underline => {
                style.underline = Some(gpui::UnderlineStyle {
                    thickness: px(1.5),
                    color: Some(cursor_color),
                    wavy: false,
                });
            }
            TerminalCursorShapeSnapshot::Beam => {
                style.background = hunk_opacity(cursor_color, is_dark, 0.32, 0.18);
            }
            TerminalCursorShapeSnapshot::Block | TerminalCursorShapeSnapshot::HollowBlock => {
                style.color = default_background;
                style.background = cursor_color;
            }
        }
    }

    if style.color == default_foreground && style.background == default_background && style.underline.is_none() {
        return AiTerminalCellStyle {
            color: default_foreground,
            background: default_background,
            underline: None,
        };
    }

    style
}

fn ai_terminal_render_character(character: char) -> char {
    if character == '\0' || character.is_control() {
        ' '
    } else {
        character
    }
}

fn ai_terminal_snapshot_color(color: TerminalColorSnapshot, is_dark: bool, cx: &App) -> Hsla {
    match color {
        TerminalColorSnapshot::Named(named) => ai_terminal_named_color(named, is_dark, cx),
        TerminalColorSnapshot::Indexed(index) => ai_terminal_indexed_color(index, is_dark, cx),
        TerminalColorSnapshot::Rgb { r, g, b } => Hsla::from(rgb(
            (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b),
        )),
    }
}

fn ai_terminal_named_color(
    color: TerminalNamedColorSnapshot,
    is_dark: bool,
    cx: &App,
) -> Hsla {
    let theme = cx.theme();
    let chrome = hunk_editor_chrome_colors(theme, is_dark);
    let magenta = hunk_blend(theme.accent, theme.danger, is_dark, 0.42, 0.30);
    let cyan = hunk_blend(theme.accent, theme.success, is_dark, 0.30, 0.26);
    let black = hunk_blend(chrome.background, chrome.foreground, is_dark, 0.14, 0.26);

    match color {
        TerminalNamedColorSnapshot::Black => black,
        TerminalNamedColorSnapshot::Red => theme.danger,
        TerminalNamedColorSnapshot::Green => theme.success,
        TerminalNamedColorSnapshot::Yellow => theme.warning,
        TerminalNamedColorSnapshot::Blue => theme.accent,
        TerminalNamedColorSnapshot::Magenta => magenta,
        TerminalNamedColorSnapshot::Cyan => cyan,
        TerminalNamedColorSnapshot::White => chrome.foreground,
        TerminalNamedColorSnapshot::BrightBlack => hunk_opacity(chrome.foreground, is_dark, 0.62, 0.58),
        TerminalNamedColorSnapshot::BrightRed => hunk_blend(theme.danger, chrome.foreground, is_dark, 0.16, 0.08),
        TerminalNamedColorSnapshot::BrightGreen => hunk_blend(theme.success, chrome.foreground, is_dark, 0.16, 0.08),
        TerminalNamedColorSnapshot::BrightYellow => hunk_blend(theme.warning, chrome.foreground, is_dark, 0.14, 0.08),
        TerminalNamedColorSnapshot::BrightBlue => hunk_blend(theme.accent, chrome.foreground, is_dark, 0.14, 0.08),
        TerminalNamedColorSnapshot::BrightMagenta => hunk_blend(magenta, chrome.foreground, is_dark, 0.16, 0.08),
        TerminalNamedColorSnapshot::BrightCyan => hunk_blend(cyan, chrome.foreground, is_dark, 0.16, 0.08),
        TerminalNamedColorSnapshot::BrightWhite => hunk_blend(chrome.foreground, theme.background, is_dark, 0.02, 0.02),
        TerminalNamedColorSnapshot::Foreground | TerminalNamedColorSnapshot::BrightForeground => {
            chrome.foreground
        }
        TerminalNamedColorSnapshot::Background => chrome.background,
        TerminalNamedColorSnapshot::Cursor => theme.primary,
        TerminalNamedColorSnapshot::DimBlack => hunk_opacity(black, is_dark, 0.58, 0.68),
        TerminalNamedColorSnapshot::DimRed => hunk_opacity(theme.danger, is_dark, 0.72, 0.82),
        TerminalNamedColorSnapshot::DimGreen => hunk_opacity(theme.success, is_dark, 0.72, 0.82),
        TerminalNamedColorSnapshot::DimYellow => hunk_opacity(theme.warning, is_dark, 0.72, 0.82),
        TerminalNamedColorSnapshot::DimBlue => hunk_opacity(theme.accent, is_dark, 0.72, 0.82),
        TerminalNamedColorSnapshot::DimMagenta => hunk_opacity(magenta, is_dark, 0.72, 0.82),
        TerminalNamedColorSnapshot::DimCyan => hunk_opacity(cyan, is_dark, 0.72, 0.82),
        TerminalNamedColorSnapshot::DimWhite | TerminalNamedColorSnapshot::DimForeground => {
            hunk_opacity(chrome.foreground, is_dark, 0.72, 0.82)
        }
    }
}

fn ai_terminal_indexed_color(index: u8, is_dark: bool, cx: &App) -> Hsla {
    match index {
        0 => ai_terminal_named_color(TerminalNamedColorSnapshot::Black, is_dark, cx),
        1 => ai_terminal_named_color(TerminalNamedColorSnapshot::Red, is_dark, cx),
        2 => ai_terminal_named_color(TerminalNamedColorSnapshot::Green, is_dark, cx),
        3 => ai_terminal_named_color(TerminalNamedColorSnapshot::Yellow, is_dark, cx),
        4 => ai_terminal_named_color(TerminalNamedColorSnapshot::Blue, is_dark, cx),
        5 => ai_terminal_named_color(TerminalNamedColorSnapshot::Magenta, is_dark, cx),
        6 => ai_terminal_named_color(TerminalNamedColorSnapshot::Cyan, is_dark, cx),
        7 => ai_terminal_named_color(TerminalNamedColorSnapshot::White, is_dark, cx),
        8 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightBlack, is_dark, cx),
        9 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightRed, is_dark, cx),
        10 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightGreen, is_dark, cx),
        11 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightYellow, is_dark, cx),
        12 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightBlue, is_dark, cx),
        13 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightMagenta, is_dark, cx),
        14 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightCyan, is_dark, cx),
        15 => ai_terminal_named_color(TerminalNamedColorSnapshot::BrightWhite, is_dark, cx),
        16..=231 => {
            let palette_index = index - 16;
            let red = palette_index / 36;
            let green = (palette_index % 36) / 6;
            let blue = palette_index % 6;
            Hsla::from(rgb(
                (u32::from(ai_terminal_cube_component(red)) << 16)
                    | (u32::from(ai_terminal_cube_component(green)) << 8)
                    | u32::from(ai_terminal_cube_component(blue)),
            ))
        }
        232..=255 => {
            let component = 8 + ((index - 232) * 10);
            Hsla::from(rgb(
                (u32::from(component) << 16) | (u32::from(component) << 8) | u32::from(component),
            ))
        }
    }
}

fn ai_terminal_cube_component(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}
