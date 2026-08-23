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

    let mut lines_elements = Vec::new();

    if let Some(ws) = workspace.state.workspaces.get(workspace.state.active_workspace) {
        if let Some(term_model) = workspace.terminals.get(workspace.state.active_workspace).and_then(|t| t.get(ws.active_term)) {
            let (rows_count, cols_count) = {
                let term = term_model.read(cx);
            let parser = term.parser.lock().unwrap();
            parser.screen().size()
        };
        
        let expected_cols = ((f32::from(viewport.width) - 192.0 - 32.0) / 8.4).max(10.0) as u16;
        let expected_rows = ((f32::from(viewport.height) - 32.0 - 32.0) / 20.0).max(10.0) as u16;
        
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
                    
                    let mut el = div().whitespace_nowrap().text_color(final_fg).px(px(0.5));
                    if bg != vt100::Color::Default || inv {
                        el = el.bg(final_bg);
                    }
                    if bold && !inv {
                        el = el.text_color(vt100_color_to_gpui(&fg, gpui::white(), true, theme));
                    }
                    if bold {
                        el = el.font_weight(gpui::FontWeight::BOLD);
                    }
                    
                    line_children.push(el.child(text.clone()));
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
                    .h(px(20.0))
                    .children(line_children)
            );
        }
    }
    }

    div()
        .w_full()
        .h_full()
        .bg(theme.bg_main)
        .p_4()
        .font_family("Menlo")
        .text_size(px(14.0))
        .line_height(relative(1.0))
        .overflow_hidden()
        .children(lines_elements)
}
