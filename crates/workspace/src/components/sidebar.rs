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
            .w_48()
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

    div()
        .w_48()
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
                            let bg = if is_active {
                                theme.bg_tab_inactive
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
                                .flex()
                                .justify_between()
                                .items_center()
                                .text_color(theme.text_primary)
                                .hover(|s| s.bg(theme.bg_tab_inactive))
                                .cursor_pointer()
                                .on_drag(DragDir(i), move |_drag_dir, _pos, _window, cx| {
                                    let drag_name = drag_name.clone();
                                    let drag_theme = drag_theme.clone();
                                    cx.new(move |_cx| DragDirPreview {
                                        name: drag_name,
                                        theme: drag_theme,
                                    })
                                })
                                .on_drop(cx.listener(
                                    move |this, drag_dir: &DragDir, _window, cx| {
                                        this.move_dir(drag_dir.0, i, cx);
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
                                .child(ws.name.clone())
                                .child(
                                    div()
                                        .id(("del-dir", i))
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
        .child(
            // Add new dir button
            div()
                .id("add-workspace")
                .w_full()
                .p_2()
                .mt_2()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .hover(|s| s.bg(theme.bg_tab_inactive))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _event, window, cx| {
                    this.add_dir(window, cx);
                }))
                .flex()
                .justify_center()
                .text_color(theme.text_muted)
                .child("+ Add Workspace"),
        )
}
