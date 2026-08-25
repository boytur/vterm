use crate::workspace::Workspace;
use gpui::prelude::*;
use gpui::*;
use terminal::{palette_rgb, CellColor, TermCell};

// Vertical nudge applied to each grid line so glyphs sit centered inside their
// cell. Overlays (selection, IME, cursor) must add the same offset or they
// drift above the text. Keep in sync with the line `.pt`. Horizontal padding
// was removed from text runs (it accumulated per run); glyphs start at the
// column origin, so CELL_PAD_X is 0.
const CELL_PAD_X: f32 = 0.0;
const CELL_PAD_Y: f32 = 3.0;

// Colors resolve through the `terminal` crate's shared TerminalColors handle
// so OSC 10/11/4 color-query replies are guaranteed to match what this
// renderer paints — including live theme switches.

fn channels_to_u32([r, g, b]: [u8; 3]) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

struct Palette {
    fg: u32,
    bg: u32,
    ansi: [u32; 16],
}

impl Palette {
    fn from_channels((fg, bg, ansi): ([u8; 3], [u8; 3], [[u8; 3]; 16])) -> Self {
        Self {
            fg: channels_to_u32(fg),
            bg: channels_to_u32(bg),
            ansi: ansi.map(channels_to_u32),
        }
    }
}

fn cell_color_to_gpui(color: &CellColor, default_color: Hsla, pal: &Palette) -> Hsla {
    match color {
        CellColor::Foreground => gpui::rgb(pal.fg).into(),
        CellColor::Background => default_color,
        CellColor::Rgb([r, g, b]) => {
            gpui::rgb((*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)).into()
        }
        CellColor::Palette(idx) if (*idx as usize) < 16 => gpui::rgb(pal.ansi[*idx as usize]).into(),
        // Single source of truth with the OSC responder for cube/grayscale.
        CellColor::Palette(idx) => match palette_rgb(*idx as usize) {
            Some([r, g, b]) => gpui::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32).into(),
            None => default_color,
        },
    }
}

fn cell_contents(cell: Option<&TermCell>) -> String {
    // Blank cells become a non-breaking space so the monospace grid keeps
    // its alignment through text runs.
    match cell {
        Some(cell) if !cell.wide_spacer && cell.ch != ' ' => cell.ch.to_string(),
        _ => "\u{00A0}".to_string(),
    }
}

pub fn render_terminal_view(
    workspace: &Workspace,
    _window: &gpui::Window,
    cx: &mut Context<Workspace>,
    viewport: gpui::Size<gpui::Pixels>,
) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let font_size = workspace.state.font_size;

    let mut lines_elements = Vec::new();
    let mut selection_overlay = div();
    let mut scrollbar_element = div();
    let mut ime_composition_overlay = div();
    // Fallback for frames with no active terminal: the built-in dark palette.
    let mut pal = Palette::from_channels(terminal::TerminalColors::dark().get());

    if let Some(ws) = workspace
        .state
        .workspaces
        .get(workspace.state.active_workspace)
        && let Some(term_model) = workspace
            .terminals
            .get(workspace.state.active_workspace)
            .and_then(|t| t.get(ws.active_term))
    {
        let term = term_model.read(cx);
        let snap = term.snapshot();
        pal = Palette::from_channels(term.colors.get());

        let rows_count = snap.rows;
        let cols_count = snap.cols;

        let cell_w = font_size * (8.4 / 14.0);
        let cell_h = font_size * (20.0 / 14.0);
        let (expected_rows, expected_cols) = Workspace::terminal_size(viewport, font_size);

        if expected_cols != cols_count || expected_rows != rows_count {
            let term_model = term_model.clone();
            cx.defer(move |app| {
                term_model.update(app, |term, cx| {
                    term.resize(expected_rows, expected_cols);
                    cx.notify();
                });
            });
        }

        let (current_offset, max_offset) = (snap.offset, snap.history);
        // The live cursor only exists on the bottom view; while scrolled into
        // history it would highlight an arbitrary historical cell. The
        // snapshot already drops it there.
        if max_offset > 0 {
            let total_lines = max_offset as f32 + rows_count as f32;
            let visible_ratio = rows_count as f32 / total_lines;
            let thumb_h = visible_ratio.max(0.05);
            let max_scroll = 1.0 - thumb_h;
            let scroll_fraction = (max_offset - current_offset) as f32 / max_offset as f32;
            let thumb_top = scroll_fraction * max_scroll;

            scrollbar_element = div()
                .absolute()
                .top(px(16.0))
                .bottom(px(16.0))
                .right(px(2.0))
                .w(px(8.0))
                .rounded_full()
                .bg(theme.border)
                .child(
                    div()
                        .absolute()
                        .w_full()
                        .h(relative(thumb_h))
                        .top(relative(thumb_top))
                        .rounded_full()
                        .bg(theme.text_muted)
                        .hover(|s| s.bg(theme.text_primary)),
                );
        }

        if let Some((cursor_row, cursor_col)) = snap.cursor
            && !workspace.ime_composition.is_empty()
        {
            ime_composition_overlay = div()
                .absolute()
                .left(px(16.0 + CELL_PAD_X + cursor_col as f32 * cell_w))
                .top(px(16.0 + CELL_PAD_Y + cursor_row as f32 * cell_h))
                .min_w(px(cell_w))
                .h(px(cell_h))
                .px(px(1.0))
                .bg(theme.bg_main)
                .border_b_1()
                .border_color(gpui::rgb(0x66ccff))
                .text_color(gpui::rgb(0xffffff))
                .whitespace_nowrap()
                .child(workspace.ime_composition.clone());
        }

        if let Some(sel) = workspace.selection {
            let ((c1, r1), (c2, r2)) = sel;
            let min_c = c1.min(c2);
            let max_c = c1.max(c2);
            let min_r = r1.min(r2);
            let max_r = r1.max(r2);
            let mut rects: Vec<gpui::AnyElement> = Vec::new();
            for r in min_r..=max_r {
                let x = 16.0 + CELL_PAD_X + min_c as f32 * cell_w;
                let y = 16.0 + CELL_PAD_Y + r as f32 * cell_h;
                let w = (max_c - min_c + 1) as f32 * cell_w;
                rects.push(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(w))
                        .h(px(cell_h))
                        .bg(rgba(0x0a84ff55))
                        .into_any_element(),
                );
            }
            selection_overlay = div().absolute().inset_0().children(rects);
        }

        for r in 0..rows_count {
            let mut line_children = Vec::new();

            let mut current_text = String::new();
            let mut current_fg = CellColor::Foreground;
            let mut current_bg = CellColor::Background;
            let mut current_bold = false;
            let mut current_inverse = false;

            let mut flush = |text: &mut String,
                             fg: CellColor,
                             bg: CellColor,
                             bold: bool,
                             inv: bool| {
                if !text.is_empty() {
                    let fg_base = cell_color_to_gpui(&fg, gpui::rgb(pal.fg).into(), &pal);
                    let bg_base = cell_color_to_gpui(&bg, gpui::transparent_black(), &pal);

                    let (final_fg, final_bg) = if inv {
                        let inv_fg = if bg == CellColor::Background {
                            gpui::rgb(pal.bg).into()
                        } else {
                            bg_base
                        };
                        let inv_bg = if fg == CellColor::Foreground {
                            gpui::rgb(pal.fg).into()
                        } else {
                            fg_base
                        };
                        (inv_fg, inv_bg)
                    } else {
                        (fg_base, bg_base)
                    };

                    let mut remaining = text.as_str();
                    while !remaining.is_empty() {
                        let http_idx = remaining.find("http://");
                        let https_idx = remaining.find("https://");

                        let idx = match (http_idx, https_idx) {
                            (Some(i), Some(j)) => Some(i.min(j)),
                            (Some(i), None) => Some(i),
                            (None, Some(j)) => Some(j),
                            (None, None) => None,
                        };

                        let create_el = || {
                            let mut base =
                                div().whitespace_nowrap().text_color(final_fg);
                            if bg != CellColor::Background || inv {
                                base = base.bg(final_bg);
                            }
                            if bold {
                                base = base.font_weight(gpui::FontWeight::BOLD);
                            }
                            base
                        };

                        if let Some(i) = idx {
                            if i > 0 {
                                line_children.push(create_el().child(remaining[..i].to_string()));
                            }
                            let url_start = &remaining[i..];
                            let end_idx = url_start
                                .find(|c: char| {
                                    c.is_whitespace()
                                        || c == '"'
                                        || c == '\''
                                        || c == ')'
                                        || c == ']'
                                        || c == '>'
                                })
                                .unwrap_or(url_start.len());
                            let url = &url_start[..end_idx];
                            let url_string = url.to_string();

                            line_children.push(
                                create_el()
                                    .text_color(gpui::rgba(0x4488FFFF))
                                    .hover(|s| s.bg(gpui::rgba(0x4488FF33)))
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, move |_e, _w, cx| {
                                        cx.open_url(&url_string);
                                    })
                                    .child(url.to_string()),
                            );
                            remaining = &url_start[end_idx..];
                        } else {
                            line_children.push(create_el().child(remaining.to_string()));
                            break;
                        }
                    }
                    text.clear();
                }
            };

            for c in 0..cols_count {
                let is_cursor = snap.cursor == Some((r, c));

                let cell = snap.cell(r, c);
                let (fg, bg, bold, mut inv, contents) = match cell {
                    Some(cell) if !cell.wide_spacer => (
                        cell.fg,
                        cell.bg,
                        cell.bold,
                        cell.inverse,
                        cell_contents(Some(cell)),
                    ),
                    _ => (
                        CellColor::Foreground,
                        CellColor::Background,
                        false,
                        false,
                        "\u{00A0}".to_string(),
                    ),
                };

                if is_cursor {
                    inv = !inv;
                }

                if fg != current_fg
                    || bg != current_bg
                    || bold != current_bold
                    || inv != current_inverse
                {
                    flush(
                        &mut current_text,
                        current_fg,
                        current_bg,
                        current_bold,
                        current_inverse,
                    );
                    current_fg = fg;
                    current_bg = bg;
                    current_bold = bold;
                    current_inverse = inv;
                }

                current_text.push_str(&contents);
            }

            flush(
                &mut current_text,
                current_fg,
                current_bg,
                current_bold,
                current_inverse,
            );

            lines_elements.push(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                .pt(px(CELL_PAD_Y))
                .h(px(cell_h))
                    .children(line_children),
            );
        }
    }

    let workspace_entity = cx.entity();
    let focus_handle = workspace.focus_handle.clone();

    div()
        .relative()
        .w_full()
        .h_full()
        .bg(gpui::rgb(pal.bg))
        .px_4()
        .pt_4()
        .pb_4()
        .font_family("Menlo")
        .text_size(px(font_size))
        .line_height(relative(20.0 / 14.0))
        .overflow_hidden()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(Workspace::on_terminal_mouse_down),
        )
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(Workspace::on_terminal_mouse_down),
        )
        .on_mouse_move(cx.listener(Workspace::on_terminal_mouse_move))
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(Workspace::on_terminal_mouse_up),
        )
        .on_mouse_up(
            gpui::MouseButton::Right,
            cx.listener(Workspace::on_terminal_mouse_up),
        )
        .on_scroll_wheel(cx.listener(Workspace::on_terminal_scroll_wheel))
        .child(selection_overlay)
        .children(lines_elements)
        .child(scrollbar_element)
        .child(ime_composition_overlay)
        .when(!workspace.settings_open, move |this| {
            this.child(
                canvas(
                    |_bounds, _window, _cx| {},
                    move |bounds, _prepaint, window, cx| {
                        window.handle_input(
                            &focus_handle,
                            ElementInputHandler::new(bounds, workspace_entity.clone()),
                            cx,
                        );
                    },
                )
                .absolute()
                .inset_0(),
            )
        })
}
