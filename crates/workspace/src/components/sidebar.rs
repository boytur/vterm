use crate::workspace::Workspace;
use gpui::prelude::*;
use gpui::*;
use theme::Theme;

#[derive(Clone, PartialEq, Debug)]
pub struct DragDir(pub usize);

pub struct DragDirPreview {
    pub name: String,
    pub theme: Theme,
}

impl Render for DragDirPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_64()
            .bg(self.theme.bg_tab_active)
            .border_1()
            .border_color(self.theme.border)
            .rounded_md()
            .p_2()
            .text_color(self.theme.text_primary)
            .child(self.name.clone())
    }
}

pub fn render_sidebar(workspace: &Workspace, cx: &mut Context<Workspace>) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let dir_drop_target = workspace.dir_drop_target;
    let dir_count = workspace.state.workspaces.len();

    div()
        .w_64()
        .h_full()
        .bg(theme.bg_sidebar)
        .border_r_1()
        .border_color(theme.border)
        .flex_shrink_0()
        .flex()
        .flex_col()
        .p_2()
        .gap_1()
        .child(
            div()
                .id("sidebar-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1()
                .children(
                    workspace
                        .state
                        .workspaces
                        .iter()
                        .enumerate()
                        .map(|(i, ws)| {
                            let is_active = i == workspace.state.active_workspace;
                            let show_indicator = dir_drop_target == Some(i);
                            let accent = theme.accent;
                            let bg = if is_active {
                                theme.bg_tab_inactive
                            } else if show_indicator {
                                Rgba { a: 0.25, ..accent }
                            } else {
                                theme.bg_sidebar
                            };

                            let drag_name = ws.name.clone();
                            let drag_theme = theme.clone();

                            div()
                                .id(("dir", i))
                                .w_full()
                                .p_2()
                                .rounded_md()
                                .bg(bg)
                                .border_1()
                                .border_color(if show_indicator {
                                    accent
                                } else {
                                    Rgba { a: 0.0, ..accent }
                                })
                                .flex()
                                .justify_between()
                                .items_center()
                                .text_color(theme.text_primary)
                                .when(!show_indicator, |el| {
                                    el.hover(|s| s.bg(theme.bg_tab_inactive))
                                })
                                .cursor_pointer()
                                .drag_over::<DragDir>(move |style, _, _, _| {
                                    style.bg(Rgba { a: 0.3, ..accent }).border_color(accent)
                                })
                                .on_drag(DragDir(i), move |_drag_dir, _pos, _window, cx| {
                                    let drag_name = drag_name.clone();
                                    let drag_theme = drag_theme.clone();
                                    cx.new(move |_cx| DragDirPreview {
                                        name: drag_name,
                                        theme: drag_theme,
                                    })
                                })
                                .on_drag_move(cx.listener(
                                    move |this, event: &DragMoveEvent<DragDir>, _window, cx| {
                                        if !event.bounds.contains(&event.event.position) {
                                            return;
                                        }
                                        // Vertical list: drop above when in the top
                                        // half, below when in the bottom half.
                                        let insert = if event.event.position.y
                                            < event.bounds.center().y
                                        {
                                            i
                                        } else {
                                            i + 1
                                        };
                                        if this.dir_drop_target != Some(insert) {
                                            this.dir_drop_target = Some(insert);
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_drop(cx.listener(
                                    move |this, drag_dir: &DragDir, _window, cx| {
                                        let to = this.dir_drop_target.unwrap_or(i);
                                        this.dir_drop_target = None;
                                        this.move_dir(drag_dir.0, to, cx);
                                    },
                                ))
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener(
                                        move |this, event: &gpui::MouseDownEvent, _window, cx| {
                                            cx.stop_propagation();
                                            this.open_dir_menu(i, event.position, cx);
                                        },
                                    ),
                                )
                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                    this.select_dir(i, cx);
                                }))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .child(ws.name.clone()),
                                )
                                .child(
                                    div()
                                        .id(("del-dir", i))
                                        .flex_shrink_0()
                                        .p_0()
                                        .w(px(16.0))
                                        .h(px(16.0))
                                        .flex()
                                        .justify_center()
                                        .items_center()
                                        .rounded_full()
                                        .text_color(theme.text_muted)
                                        .hover(|s| {
                                            s.bg(theme.bg_tab_inactive).text_color(theme.ansi[1])
                                        })
                                        .on_click(cx.listener(
                                            move |this, _event: &gpui::ClickEvent, _window, cx| {
                                                this.delete_dir(i, cx);
                                            },
                                        ))
                                        .child(
                                            div()
                                                .id("x-icon")
                                                .flex()
                                                .justify_center()
                                                .items_center()
                                                .text_size(px(10.0))
                                                .child("x"),
                                        ),
                                )
                        }),
                ),
        )
        .child({
            // Add new dir button
            let accent = theme.accent;
            div()
                .id("add-workspace")
                .w_full()
                .p_2()
                .mt_2()
                .rounded_md()
                .border_1()
                .border_color(if dir_drop_target == Some(dir_count) {
                    accent
                } else {
                    Rgba { a: 0.0, ..accent }
                })
                .bg(if dir_drop_target == Some(dir_count) {
                    Rgba { a: 0.3, ..accent }
                } else {
                    theme.bg_sidebar
                })
                .when(dir_drop_target != Some(dir_count), |el| {
                    el.hover(|s| s.bg(theme.bg_tab_inactive))
                })
                .cursor_pointer()
                .drag_over::<DragDir>(move |style, _, _, _| {
                    style.bg(Rgba { a: 0.3, ..accent }).border_color(accent)
                })
                .on_drag_move(cx.listener(
                    move |this, event: &DragMoveEvent<DragDir>, _window, cx| {
                        if !event.bounds.contains(&event.event.position) {
                            return;
                        }
                        if this.dir_drop_target != Some(dir_count) {
                            this.dir_drop_target = Some(dir_count);
                            cx.notify();
                        }
                    },
                ))
                .on_click(cx.listener(move |this, _event, window, cx| {
                    this.add_dir(window, cx);
                }))
                .flex()
                .justify_center()
                .text_color(theme.text_muted)
                .child("+ Add Workspace")
        })
}
