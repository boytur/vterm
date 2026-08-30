use crate::workspace::{Workspace, TERMINAL_FONT};
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
const BLOCK_BLEED: f32 = 0.35;
const LINE_BLEED: f32 = 0.35;

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

fn u32_to_channels(color: u32) -> [u8; 3] {
    [
        (color >> 16) as u8,
        (color >> 8) as u8,
        color as u8,
    ]
}

fn cell_color_rgb(color: &CellColor, fallback: [u8; 3], pal: &Palette) -> [u8; 3] {
    match color {
        CellColor::Foreground => u32_to_channels(pal.fg),
        CellColor::Background => fallback,
        CellColor::Rgb(rgb) => *rgb,
        CellColor::Palette(idx) => match palette_rgb(*idx as usize) {
            Some(rgb) => rgb,
            None => fallback,
        },
    }
}

fn relative_luminance([r, g, b]: [u8; 3]) -> f32 {
    fn linear(channel: u8) -> f32 {
        let channel = channel as f32 / 255.0;
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f32 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn mix_rgb(first: [u8; 3], second: [u8; 3], amount: f32) -> [u8; 3] {
    [
        (first[0] as f32 + (second[0] as f32 - first[0] as f32) * amount).round() as u8,
        (first[1] as f32 + (second[1] as f32 - first[1] as f32) * amount).round() as u8,
        (first[2] as f32 + (second[2] as f32 - first[2] as f32) * amount).round() as u8,
    ]
}

fn readable_color(color: [u8; 3], background: [u8; 3], pal: &Palette) -> [u8; 3] {
    const MIN_CONTRAST: f32 = 3.0;

    if contrast_ratio(color, background) >= MIN_CONTRAST {
        return color;
    }

    let targets = [
        u32_to_channels(pal.fg),
        u32_to_channels(pal.ansi[0]),
        u32_to_channels(pal.ansi[7]),
        u32_to_channels(pal.ansi[15]),
    ];
    let target = targets
        .into_iter()
        .max_by(|a, b| {
            contrast_ratio(*a, background)
                .partial_cmp(&contrast_ratio(*b, background))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(u32_to_channels(pal.fg));

    for step in 1..=8 {
        let candidate = mix_rgb(color, target, step as f32 / 8.0);
        if contrast_ratio(candidate, background) >= MIN_CONTRAST {
            return candidate;
        }
    }
    target
}

fn color_to_gpui(color: [u8; 3]) -> Hsla {
    gpui::rgb(channels_to_u32(color)).into()
}

fn cell_contents(cell: Option<&TermCell>) -> String {
    // Blank cells become a non-breaking space so the monospace grid keeps
    // its alignment through text runs.
    match cell {
        Some(cell) if !cell.wide_spacer => {
            let mut contents = if cell.ch == ' ' {
                "\u{00A0}".to_string()
            } else {
                cell.ch.to_string()
            };
            contents.extend(cell.zero_width.iter().copied());
            contents
        }
        _ => "\u{00A0}".to_string(),
    }
}

fn cell_index_at_byte_offset(cells: &[String], offset: usize) -> usize {
    let mut bytes = 0;
    for (index, cell) in cells.iter().enumerate() {
        if bytes >= offset {
            return index;
        }
        bytes += cell.len();
        if bytes >= offset {
            return index + 1;
        }
    }
    cells.len()
}

#[derive(Clone, Copy)]
struct BoxDrawing {
    mask: u8,
    stroke: f32,
}

fn box_drawing(ch: char) -> Option<BoxDrawing> {
    let mask = match ch {
        '─' | '━' | '═' => 0b1010,
        '│' | '┃' | '║' => 0b0101,
        '┌' | '┏' | '╔' | '╭' => 0b0110,
        '┐' | '┓' | '╗' | '╮' => 0b1100,
        '└' | '┗' | '╚' | '╰' => 0b0011,
        '┘' | '┛' | '╝' | '╯' => 0b1001,
        '├' | '┣' | '╠' => 0b0111,
        '┤' | '┫' | '╣' => 0b1101,
        '┬' | '┳' | '╦' => 0b1110,
        '┴' | '┻' | '╩' => 0b1011,
        '┼' | '╋' | '╬' => 0b1111,
        '╴' => 0b0010,
        '╶' => 0b1000,
        '╷' => 0b0100,
        '╵' => 0b0001,
        _ => return None,
    };

    let stroke = if matches!(
        ch,
        '━' | '┃' | '┏' | '┓' | '┗' | '┛' | '┣' | '┫' | '┳' | '┻' | '╋'
    ) {
        1.5
    } else if matches!(
        ch,
        '═' | '║' | '╔' | '╗' | '╚' | '╝' | '╠' | '╣' | '╦' | '╩' | '╬'
    ) {
        1.25
    } else {
        1.0
    };

    Some(BoxDrawing { mask, stroke })
}

fn snap_to_device_pixel(value: f32, scale_factor: f32) -> f32 {
    (value * scale_factor).round() / scale_factor
}

fn push_box_drawing(
    elements: &mut Vec<gpui::AnyElement>,
    drawing: BoxDrawing,
    x: f32,
    y: f32,
    cell_w: f32,
    cell_h: f32,
    color: Hsla,
    scale_factor: f32,
) {
    let half_w = cell_w / 2.0;
    let half_h = cell_h / 2.0;
    let center_x = snap_to_device_pixel(x + half_w, scale_factor);
    let center_y = snap_to_device_pixel(y + half_h, scale_factor);
    let stroke = (drawing.stroke * scale_factor).round().max(1.0) / scale_factor;
    let has_left = drawing.mask & 0b1000 != 0;
    let has_right = drawing.mask & 0b0010 != 0;
    let has_up = drawing.mask & 0b0001 != 0;
    let has_down = drawing.mask & 0b0100 != 0;

    if has_left || has_right {
        let left = if has_left {
            snap_to_device_pixel(x - LINE_BLEED, scale_factor)
        } else {
            center_x
        };
        let right = if has_right {
            snap_to_device_pixel(x + cell_w + LINE_BLEED, scale_factor)
        } else {
            center_x
        };
        let top = snap_to_device_pixel(center_y - stroke / 2.0, scale_factor);
        elements.push(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(right - left))
                .h(px(stroke))
                .bg(color)
                .into_any_element(),
        );
    }

    if has_up || has_down {
        let top = if has_up {
            snap_to_device_pixel(y - LINE_BLEED, scale_factor)
        } else {
            center_y
        };
        let bottom = if has_down {
            snap_to_device_pixel(y + cell_h + LINE_BLEED, scale_factor)
        } else {
            center_y
        };
        let left = snap_to_device_pixel(center_x - stroke / 2.0, scale_factor);
        elements.push(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(stroke))
                .h(px(bottom - top))
                .bg(color)
                .into_any_element(),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BlockRect {
    left: u8,
    top: u8,
    right: u8,
    bottom: u8,
}

#[derive(Clone, Copy)]
struct BlockShape {
    rects: [BlockRect; 3],
    count: usize,
}

fn block_shape(ch: char) -> Option<BlockShape> {
    let rect = |left, top, right, bottom| BlockRect {
        left,
        top,
        right,
        bottom,
    };
    let empty = rect(0, 0, 0, 0);
    let one = |shape| BlockShape {
        rects: [shape, empty, empty],
        count: 1,
    };
    let two = |first, second| BlockShape {
        rects: [first, second, empty],
        count: 2,
    };
    let three = |first, second, third| BlockShape {
        rects: [first, second, third],
        count: 3,
    };

    let full = rect(0, 0, 8, 8);
    let upper_left = rect(0, 0, 4, 4);
    let upper_right = rect(4, 0, 8, 4);
    let lower_left = rect(0, 4, 4, 8);
    let lower_right = rect(4, 4, 8, 8);

    Some(match ch {
        '█' => one(full),
        '▀' => one(rect(0, 0, 8, 4)),
        '▄' => one(rect(0, 4, 8, 8)),
        '▌' => one(rect(0, 0, 4, 8)),
        '▐' => one(rect(4, 0, 8, 8)),
        '▏' => one(rect(0, 0, 1, 8)),
        '▎' => one(rect(0, 0, 2, 8)),
        '▍' => one(rect(0, 0, 3, 8)),
        '▋' => one(rect(0, 0, 5, 8)),
        '▊' => one(rect(0, 0, 6, 8)),
        '▉' => one(rect(0, 0, 7, 8)),
        '▕' => one(rect(7, 0, 8, 8)),
        '▁' => one(rect(0, 7, 8, 8)),
        '▂' => one(rect(0, 6, 8, 8)),
        '▃' => one(rect(0, 5, 8, 8)),
        '▅' => one(rect(0, 3, 8, 8)),
        '▆' => one(rect(0, 2, 8, 8)),
        '▇' => one(rect(0, 1, 8, 8)),
        '▔' => one(rect(0, 0, 8, 1)),
        '▖' => one(lower_left),
        '▗' => one(lower_right),
        '▘' => one(upper_left),
        '▝' => one(upper_right),
        '▚' => two(upper_left, lower_right),
        '▞' => two(upper_right, lower_left),
        '▙' => three(upper_left, lower_left, lower_right),
        '▛' => three(upper_left, upper_right, lower_left),
        '▜' => three(upper_right, lower_left, lower_right),
        '▟' => three(upper_left, upper_right, lower_right),
        _ => return None,
    })
}

fn push_block_shape(
    elements: &mut Vec<gpui::AnyElement>,
    shape: BlockShape,
    x: f32,
    y: f32,
    cell_w: f32,
    cell_h: f32,
    color: Hsla,
) {
    for block in shape.rects.into_iter().take(shape.count) {
        let left = x + cell_w * f32::from(block.left) / 8.0 - BLOCK_BLEED;
        let top = y + cell_h * f32::from(block.top) / 8.0 - BLOCK_BLEED;
        let right = x + cell_w * f32::from(block.right) / 8.0 + BLOCK_BLEED;
        let bottom = y + cell_h * f32::from(block.bottom) / 8.0 + BLOCK_BLEED;
        elements.push(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(right - left))
                .h(px(bottom - top))
                .bg(color)
                .into_any_element(),
        );
    }
}

pub fn render_terminal_view(
    workspace: &Workspace,
    window: &gpui::Window,
    cx: &mut Context<Workspace>,
    viewport: gpui::Size<gpui::Pixels>,
) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let font_size = workspace.state.font_size;
    let scale_factor = window.scale_factor();

    let mut lines_elements = Vec::new();
    let mut box_drawing_elements = Vec::new();
    let mut block_elements = Vec::new();
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

        let cell_w = Workspace::terminal_cell_width(window, font_size);
        let cell_h = font_size * (20.0 / 14.0);
        let (expected_rows, expected_cols) =
            Workspace::terminal_size_for_window(viewport, font_size, window);

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
            let mut current_cells = Vec::new();
            let mut current_start_col = 0;
            let mut current_fg = CellColor::Foreground;
            let mut current_bg = CellColor::Background;
            let mut current_bold = false;
            let mut current_inverse = false;

            let mut flush = |text: &mut String,
                             cells: &mut Vec<String>,
                             start_col: usize,
                             fg: CellColor,
                             bg: CellColor,
                             bold: bool,
                             inv: bool| {
                if !text.is_empty() {
                    let run_text = std::mem::take(text);
                    let run_cells = std::mem::take(cells);
                    let fg_rgb = cell_color_rgb(&fg, u32_to_channels(pal.fg), &pal);
                    let bg_rgb = cell_color_rgb(&bg, u32_to_channels(pal.bg), &pal);

                    let (final_fg_rgb, final_bg_rgb) = if inv {
                        (readable_color(bg_rgb, fg_rgb, &pal), fg_rgb)
                    } else {
                        (readable_color(fg_rgb, bg_rgb, &pal), bg_rgb)
                    };
                    let final_fg = color_to_gpui(final_fg_rgb);
                    let final_bg = color_to_gpui(final_bg_rgb);

                    let mut byte_offset = 0;
                    let mut cell_offset = 0;
                    while byte_offset < run_text.len() {
                        let remaining = &run_text[byte_offset..];
                        let http_idx = remaining.find("http://");
                        let https_idx = remaining.find("https://");

                        let idx = match (http_idx, https_idx) {
                            (Some(i), Some(j)) => Some(i.min(j)),
                            (Some(i), None) => Some(i),
                            (None, Some(j)) => Some(j),
                            (None, None) => None,
                        };

                        let create_el = |start_cell: usize, width_cells: usize| {
                            let mut base =
                                div()
                                    .absolute()
                                    .left(px(start_cell as f32 * cell_w))
                                    .whitespace_nowrap()
                                    .flex_shrink_0()
                                    .w(px(width_cells as f32 * cell_w))
                                    .h(px(cell_h))
                                    .pt(px(CELL_PAD_Y))
                                    .text_color(final_fg);
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
                                let end = byte_offset + i;
                                let end_cell = cell_index_at_byte_offset(&run_cells, end);
                                line_children.push(
                                    create_el(
                                        start_col + cell_offset,
                                        end_cell - cell_offset,
                                    )
                                        .child(run_text[byte_offset..end].to_string()),
                                );
                                byte_offset = end;
                                cell_offset = end_cell;
                            }
                            let url_start = &run_text[byte_offset..];
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
                            let end = byte_offset + end_idx;
                            let end_cell = cell_index_at_byte_offset(&run_cells, end);
                            let url = &run_text[byte_offset..end];
                            let url_string = url.to_string();

                            line_children.push(
                                create_el(start_col + cell_offset, end_cell - cell_offset)
                                    .text_color(color_to_gpui(readable_color(
                                        [0x44, 0x88, 0xff],
                                        final_bg_rgb,
                                        &pal,
                                    )))
                                    .hover(|s| s.bg(gpui::rgba(0x4488FF33)))
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, move |_e, _w, cx| {
                                        cx.open_url(&url_string);
                                    })
                                    .child(url.to_string()),
                            );
                            byte_offset = end;
                            cell_offset = end_cell;
                        } else {
                            line_children.push(
                                create_el(
                                    start_col + cell_offset,
                                    run_cells.len() - cell_offset,
                                )
                                    .child(remaining.to_string()),
                            );
                            break;
                        }
                    }
                }
            };

            for c in 0..cols_count {
                let is_cursor = snap.cursor == Some((r, c));

                let cell = snap.cell(r, c);
                let (fg, bg, bold, mut inv, contents, drawing, block) = match cell {
                    Some(cell) if !cell.wide_spacer => {
                        let drawing = box_drawing(cell.ch);
                        let block = block_shape(cell.ch);
                        (
                            cell.fg,
                            cell.bg,
                            cell.bold,
                            cell.inverse,
                            if drawing.is_some() || block.is_some() {
                                "\u{00A0}".to_string()
                            } else {
                                cell_contents(Some(cell))
                            },
                            drawing,
                            block,
                        )
                    }
                    _ => (
                        CellColor::Foreground,
                        CellColor::Background,
                        false,
                        false,
                        "\u{00A0}".to_string(),
                        None,
                        None,
                    ),
                };

                if is_cursor {
                    inv = !inv;
                }

                if let Some(drawing) = drawing {
                    let fg_rgb = cell_color_rgb(&fg, u32_to_channels(pal.fg), &pal);
                    let bg_rgb = cell_color_rgb(&bg, u32_to_channels(pal.bg), &pal);
                    let fill = if inv { bg_rgb } else { fg_rgb };
                    push_box_drawing(
                        &mut box_drawing_elements,
                        drawing,
                        16.0 + CELL_PAD_X + c as f32 * cell_w,
                        16.0 + CELL_PAD_Y + r as f32 * cell_h,
                        cell_w,
                        cell_h,
                        color_to_gpui(readable_color(
                            fill,
                            u32_to_channels(pal.bg),
                            &pal,
                        )),
                        scale_factor,
                    );
                }

                if let Some(block) = block {
                    let fg_rgb = cell_color_rgb(&fg, u32_to_channels(pal.fg), &pal);
                    let bg_rgb = cell_color_rgb(&bg, u32_to_channels(pal.bg), &pal);
                    let fill = if inv { bg_rgb } else { fg_rgb };
                    push_block_shape(
                        &mut block_elements,
                        block,
                        16.0 + CELL_PAD_X + c as f32 * cell_w,
                        16.0 + CELL_PAD_Y + r as f32 * cell_h,
                        cell_w,
                        cell_h,
                        color_to_gpui(readable_color(
                            fill,
                            u32_to_channels(pal.bg),
                            &pal,
                        )),
                    );
                }

                if fg != current_fg
                    || bg != current_bg
                    || bold != current_bold
                    || inv != current_inverse
                {
                    flush(
                        &mut current_text,
                        &mut current_cells,
                        current_start_col,
                        current_fg,
                        current_bg,
                        current_bold,
                        current_inverse,
                    );
                    current_fg = fg;
                    current_bg = bg;
                    current_bold = bold;
                    current_inverse = inv;
                    current_start_col = c as usize;
                }

                current_text.push_str(&contents);
                current_cells.push(contents);
            }

            flush(
                &mut current_text,
                &mut current_cells,
                current_start_col,
                current_fg,
                current_bg,
                current_bold,
                current_inverse,
            );

            lines_elements.push(
                div()
                    .relative()
                    .h(px(cell_h))
                    .w(px(cols_count as f32 * cell_w))
                    .overflow_hidden()
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
        .font_family(TERMINAL_FONT)
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
        .children(block_elements)
        .children(box_drawing_elements)
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

#[cfg(test)]
mod tests {
    use super::{
        BlockRect, Palette, block_shape, box_drawing, contrast_ratio, readable_color,
        snap_to_device_pixel,
    };

    #[test]
    fn block_elements_share_cell_edges() {
        assert_eq!(
            block_shape('█').unwrap().rects[0],
            BlockRect {
                left: 0,
                top: 0,
                right: 8,
                bottom: 8,
            }
        );
        assert!(block_shape('A').is_none());

        let block_elements = "█▀▄▌▐▏▎▍▋▊▉▕▁▂▃▅▆▇▔▖▗▘▝▚▞▙▛▜▟";
        assert!(block_elements.chars().all(|ch| block_shape(ch).is_some()));
        assert!(block_shape(' ').is_none());
    }

    #[test]
    fn box_drawing_segments_share_cell_edges() {
        assert_eq!(box_drawing('─').unwrap().mask, 0b1010);
        assert_eq!(box_drawing('│').unwrap().mask, 0b0101);
        assert_eq!(box_drawing('╋').unwrap().mask, 0b1111);
        assert_eq!(box_drawing('╭').unwrap().mask, 0b0110);
        assert!(box_drawing('A').is_none());
        assert_eq!(snap_to_device_pixel(10.24, 2.0), 10.0);
        assert_eq!(snap_to_device_pixel(10.26, 2.0), 10.5);
    }

    #[test]
    fn cli_colors_keep_contrast_and_only_tint_when_needed() {
        let mut ansi = [[0, 0, 0]; 16];
        ansi[15] = [255, 255, 255];
        let palette = Palette::from_channels(([255, 255, 255], [0, 0, 0], ansi));

        let visible = [255, 0, 0];
        assert_eq!(readable_color(visible, [0, 0, 0], &palette), visible);

        let hidden = [20, 20, 20];
        let adjusted = readable_color(hidden, [0, 0, 0], &palette);
        assert_ne!(adjusted, hidden);
        assert!(contrast_ratio(adjusted, [0, 0, 0]) >= 3.0);
    }
}
