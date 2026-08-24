use gpui::*;
use crate::workspace::Workspace;

fn vt100_color_to_gpui(color: &vt100::Color, default_color: Hsla, _is_fg: bool, theme: &crate::theme::Theme) -> Hsla {
    match color {
        vt100::Color::Default => default_color,
        vt100::Color::Rgb(r, g, b) => gpui::rgb((*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)).into(),
        vt100::Color::Idx(idx) => {
            match idx {
                0..=15 => theme.ansi[*idx as usize].into(),
                16..=231 => {
                    // 216 colors
                    let mut i = *idx - 16;
                    let b = (i % 6) * 51; i /= 6;
                    let g = (i % 6) * 51; i /= 6;
                    let r = (i % 6) * 51;
                    gpui::rgb((r as u32) << 16 | (g as u32) << 8 | (b as u32)).into()
                }
                232..=255 => {
                    // Grayscale
                    let v = (*idx - 232) * 11 + 8;
                    gpui::rgb((v as u32) << 16 | (v as u32) << 8 | (v as u32)).into()
                }
            }
        }
    }
}

pub fn render_terminal_view(workspace: &Workspace, _window: &gpui::Window, cx: &mut Context<Workspace>, viewport: gpui::Size<gpui::Pixels>) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let font_size = workspace.state.font_size;

    let mut lines_elements = Vec::new();
    let mut selection_overlay = div();
    let mut scrollbar_element = div();

    if let Some(ws) = workspace.state.workspaces.get(workspace.state.active_workspace)
        && let Some(term_model) = workspace.terminals.get(workspace.state.active_workspace).and_then(|t| t.get(ws.active_term)) {
            let (rows_count, cols_count) = {
                let term = term_model.read(cx);
            let parser = term.parser.lock().unwrap();
            parser.screen().size()
        };
        
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
        
        let term = term_model.read(cx);
        let parser = term.parser.lock().unwrap();
        let screen = parser.screen();
        let cursor = screen.cursor_position();
        drop(parser); // release lock so we can call scroll_info on term_model

        let (current_offset, max_offset) = term_model.read(cx).scroll_info();
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
                        .hover(|s| s.bg(theme.text_primary))
                );
        }

        let parser = term.parser.lock().unwrap();
        let screen = parser.screen();

        if let Some(sel) = workspace.selection {
            let (c1, r1) = sel.0;
            let (c2, r2) = sel.1;
            let min_c = c1.min(c2);
            let max_c = c1.max(c2);
            let min_r = r1.min(r2);
            let max_r = r1.max(r2);
            let mut rects: Vec<gpui::AnyElement> = Vec::new();
            for r in min_r..=max_r {
                let x = 16.0 + min_c as f32 * cell_w;
                let y = 16.0 + r as f32 * cell_h;
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
            let mut current_fg = vt100::Color::Default;
            let mut current_bg = vt100::Color::Default;
            let mut current_bold = false;
            let mut current_inverse = false;
            
            let mut flush = |text: &mut String, fg: vt100::Color, bg: vt100::Color, bold: bool, inv: bool| {
                if !text.is_empty() {
                    let fg_base = vt100_color_to_gpui(&fg, theme.text_primary.into(), true, theme);
                    let bg_base = vt100_color_to_gpui(&bg, gpui::transparent_black(), false, theme);

                    let (final_fg, final_bg) = if inv {
                        let inv_fg = if bg == vt100::Color::Default { theme.bg_main.into() } else { bg_base };
                        let inv_bg = if fg == vt100::Color::Default { theme.text_primary.into() } else { fg_base };
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
                            let mut base = div().whitespace_nowrap().text_color(final_fg).px(px(0.5));
                            if bg != vt100::Color::Default || inv {
                                base = base.bg(final_bg);
                            }
                            if bold && !inv {
                                base = base.text_color(vt100_color_to_gpui(&fg, gpui::white(), true, theme));
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
                            let end_idx = url_start.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ')' || c == ']' || c == '>')
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
                                    .child(url.to_string())
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
                let is_cursor = cursor.0 == r && cursor.1 == c && !screen.hide_cursor();
                
                let (fg, bg, bold, mut inv, contents) = match screen.cell(r, c) {
                    Some(cell) => {
                        let text = if cell.has_contents() { 
                            let c_text = cell.contents();
                            if c_text.is_empty() {
                                "\u{00A0}".to_string()
                            } else {
                                c_text.replace(" ", "\u{00A0}")
                            }
                        } else { 
                            "\u{00A0}".to_string() 
                        };
                        (cell.fgcolor(), cell.bgcolor(), cell.bold(), cell.inverse(), text)
                    }
                    None => (vt100::Color::Default, vt100::Color::Default, false, false, "\u{00A0}".to_string()),
                };
                
                if is_cursor {
                    inv = !inv;
                }
                
                if fg != current_fg || bg != current_bg || bold != current_bold || inv != current_inverse {
                    flush(&mut current_text, current_fg, current_bg, current_bold, current_inverse);
                    current_fg = fg;
                    current_bg = bg;
                    current_bold = bold;
                    current_inverse = inv;
                }
                
                current_text.push_str(&contents);
            }
            
            flush(&mut current_text, current_fg, current_bg, current_bold, current_inverse);
            
            lines_elements.push(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .pt(px(3.0))
                    .h(px(cell_h))
                    .children(line_children)
            );
        }
    }

    div()
        .relative()
        .w_full()
        .h_full()
        .bg(theme.bg_main)
        .px_4()
        .pt_4()
        .pb_4()
        .font_family("Menlo")
        .text_size(px(font_size))
        .line_height(relative(20.0 / 14.0))
        .overflow_hidden()
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(Workspace::on_terminal_mouse_down))
        .on_mouse_down(gpui::MouseButton::Right, cx.listener(Workspace::on_terminal_mouse_down))
        .on_mouse_move(cx.listener(Workspace::on_terminal_mouse_move))
        .on_mouse_up(gpui::MouseButton::Left, cx.listener(Workspace::on_terminal_mouse_up))
        .on_mouse_up(gpui::MouseButton::Right, cx.listener(Workspace::on_terminal_mouse_up))
        .on_scroll_wheel(cx.listener(Workspace::on_terminal_scroll_wheel))
        .child(selection_overlay)
        .children(lines_elements)
        .child(scrollbar_element)
}
