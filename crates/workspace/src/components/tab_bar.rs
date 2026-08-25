use crate::workspace::Workspace;
use gpui::prelude::*;
use gpui::*;
use theme::Theme;

#[derive(Clone, PartialEq, Debug)]
pub struct DragTab(pub usize);

pub struct DragPreview {
    pub name: String,
    pub theme: Theme,
}

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(self.theme.bg_tab_active)
            .border_1()
            .border_color(self.theme.border)
            .rounded_md()
            .p_2()
            .text_color(self.theme.text_primary)
            .child(self.name.clone())
    }
}

pub fn render_tab_bar(workspace: &Workspace, cx: &mut Context<Workspace>) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let drop_target = workspace.tab_drop_target;
    let tab_count = workspace.state.workspaces[workspace.state.active_workspace]
        .terminals
        .len();

    div()
        .id("tab-bar")
        .w_full()
        .h(px(32.0))
        .bg(theme.bg_tab_inactive)
        .flex()
        .flex_row()
        .items_center()
        .overflow_x_scroll()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .children(
            workspace.state.workspaces[workspace.state.active_workspace]
                .terminals
                .iter()
                .enumerate()
                .map(|(i, term)| {
                    let is_active = i
                        == workspace.state.workspaces[workspace.state.active_workspace].active_term;
                    let text_col = if is_active {
                        theme.text_primary
                    } else {
                        theme.text_muted
                    };

                    let drag_term = term.name.clone();
                    let drag_theme = theme.clone();
                    let show_indicator = drop_target == Some(i);
                    let accent = theme.accent;

                    let bg = if is_active {
                        theme.bg_tab_active
                    } else if show_indicator {
                        Rgba { a: 0.25, ..accent }
                    } else {
                        theme.bg_tab_inactive
                    };
                    let border = if is_active || show_indicator {
                        accent
                    } else {
                        Rgba { a: 0.0, ..accent }
                    };

                    div()
                        .id(("tab", i))
                        .relative()
                        .h_full()
                        .px_4()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .bg(bg)
                        .border_t_2()
                        .border_color(border)
                        .text_color(text_col)
                        .when(!show_indicator, |el| {
                            el.hover(|s| s.bg(theme.bg_tab_inactive))
                        })
                        .cursor_pointer()
                        .drag_over::<DragTab>(move |style, _, _, _| {
                            style.bg(Rgba { a: 0.3, ..accent }).border_color(accent)
                        })
                        .on_drag(DragTab(i), move |_drag_tab, _pos, _window, cx| {
                            let drag_term = drag_term.clone();
                            let drag_theme = drag_theme.clone();
                            cx.new(move |_cx| DragPreview {
                                name: drag_term,
                                theme: drag_theme,
                            })
                        })
                        .on_drag_move(cx.listener(
                            move |this, event: &DragMoveEvent<DragTab>, _window, cx| {
                                if !event.bounds.contains(&event.event.position) {
                                    return;
                                }
                                let insert = if event.event.position.x < event.bounds.center().x {
                                    i
                                } else {
                                    i + 1
                                };
                                if this.tab_drop_target != Some(insert) {
                                    this.tab_drop_target = Some(insert);
                                    cx.notify();
                                }
                            },
                        ))
                        .on_drop(cx.listener(move |this, drag_tab: &DragTab, _window, cx| {
                            let to = this.tab_drop_target.unwrap_or(i);
                            this.tab_drop_target = None;
                            this.move_tab(drag_tab.0, to, cx);
                        }))
                        .when(show_indicator, |el| el.child(drop_indicator(theme)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_, _, _, cx| cx.stop_propagation()),
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
                                cx.stop_propagation();
                                this.open_tab_menu(i, event.position, cx);
                            }),
                        )
                        .on_click(cx.listener(move |this, _event, _window, cx| {
                            this.select_term(i, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .items_center()
                                .child(term.name.clone())
                                .child(
                                    div()
                                        .id(("del-tab", i))
                                        .ml_2()
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
                                                this.delete_term(i, cx);
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
                                ),
                        )
                }),
        )
        .child({
            // Add new tab button
            let accent = theme.accent;
            div()
                .id("add-tab")
                .relative()
                .h_full()
                .px_4()
                .flex_shrink_0()
                .flex()
                .items_center()
                .text_color(theme.text_muted)
                .when(drop_target != Some(tab_count), |el| {
                    el.hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.text_primary))
                })
                .cursor_pointer()
                .bg(if drop_target == Some(tab_count) {
                    Rgba { a: 0.3, ..accent }
                } else {
                    theme.bg_tab_inactive
                })
                .drag_over::<DragTab>(move |style, _, _, _| {
                    style.bg(Rgba { a: 0.3, ..accent }).border_color(accent)
                })
                .on_drag_move(cx.listener(
                    move |this, event: &DragMoveEvent<DragTab>, _window, cx| {
                        if !event.bounds.contains(&event.event.position) {
                            return;
                        }
                        if this.tab_drop_target != Some(tab_count) {
                            this.tab_drop_target = Some(tab_count);
                            cx.notify();
                        }
                    },
                ))
                .on_drop(cx.listener(move |this, drag_tab: &DragTab, _window, cx| {
                    let to = this.tab_drop_target.unwrap_or(tab_count);
                    this.tab_drop_target = None;
                    this.move_tab(drag_tab.0, to, cx);
                }))
                .when(drop_target == Some(tab_count), |el| {
                    el.child(drop_indicator(theme))
                })
                .on_click(cx.listener(move |this, _event, window, cx| {
                    this.add_term(window, cx);
                }))
                .child("+")
        })
    }

fn drop_indicator(theme: &Theme) -> impl IntoElement {
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .w(px(2.0))
        .bg(theme.accent)
}
