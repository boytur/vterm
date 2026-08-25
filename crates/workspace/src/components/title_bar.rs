use crate::workspace::Workspace;
use gpui::prelude::*;
use gpui::*;

pub fn render_title_bar(workspace: &Workspace, _cx: &mut Context<Workspace>) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let modal_open = workspace.branch_menu_open || workspace.theme_menu_open;

    // Get active workspace name
    let ws_name = workspace
        .state
        .workspaces
        .get(workspace.state.active_workspace)
        .map(|ws| ws.name.clone())
        .unwrap_or_else(|| "vterm".into());

    let branch_element = if workspace.git_branch.is_empty() {
        div().id("empty-branch")
    } else {
        div()
            .id("branch-btn")
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_color(if workspace.branch_menu_open {
                theme.text_primary
            } else {
                theme.text_muted
            })
            .bg(if workspace.branch_menu_open {
                theme.bg_tab_inactive
            } else {
                gpui::rgba(0x00000000)
            })
            .when(!modal_open, |el| {
                el.hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.text_primary))
            })
            .on_mouse_down(
                MouseButton::Left,
                _cx.listener(|this, _e, _w, cx| {
                    cx.stop_propagation();
                    this.toggle_branch_menu(cx);
                }),
            )
            .child(
                div()
                    .w(px(14.0))
                    .h(px(14.0))
                    // A simple git branch SVG icon string
                    .child(
                        gpui::svg()
                            .path("icons/git_branch.svg")
                            .text_color(if workspace.branch_menu_open {
                                theme.text_primary
                            } else {
                                theme.text_muted
                            })
                            .size(px(14.0)),
                    ),
            )
            .child(
                div()
                    .max_w(px(240.0))
                    .truncate()
                    .child(workspace.git_branch.clone()),
            )
    };

    div()
        .id("title-bar")
        .relative()
        .w_full()
        .h(px(32.0))
        .bg(theme.bg_sidebar)
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pl(px(80.0)) // Clear the macOS traffic lights
        .pr(px(16.0))
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .on_mouse_down(MouseButton::Left, |event, window, _cx| {
                    if event.click_count == 2 {
                        window.zoom_window();
                    } else {
                        window.start_window_move();
                    }
                }),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .text_color(theme.text_primary)
                .text_size(px(13.0))
                .child(ws_name)
                .child(branch_element),
        )
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id("theme-btn")
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_color(if workspace.theme_menu_open {
                            theme.text_primary
                        } else {
                            theme.text_muted
                        })
                        .bg(if workspace.theme_menu_open {
                            theme.bg_tab_inactive
                        } else {
                            gpui::rgba(0x00000000)
                        })
                        .text_size(px(12.0))
                        .cursor_pointer()
                        .when(!modal_open, |el| {
                            el.hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.text_primary))
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            _cx.listener(|this, _e, _w, cx| {
                                cx.stop_propagation();
                                this.toggle_theme_menu(cx);
                            }),
                        )
                        .child("Theme"),
                )
                .child(
                    div()
                        .id("settings-btn")
                        .relative()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_color(if workspace.settings_open {
                            theme.text_primary
                        } else {
                            theme.text_muted
                        })
                        .text_size(px(12.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.text_primary))
                        .on_mouse_down(
                            MouseButton::Left,
                            _cx.listener(|this, _e, _w, cx| {
                                cx.stop_propagation();
                                if this.update_info.is_some() {
                                    this.settings_section =
                                        crate::components::settings::SettingsSection::About;
                                }
                                this.toggle_settings(cx);
                            }),
                        )
                        .child("Settings")
                        .when(workspace.update_info.is_some(), |this| {
                            this.child(
                                div()
                                    .absolute()
                                    .top(px(1.0))
                                    .right(px(2.0))
                                    .w(px(6.0))
                                    .h(px(6.0))
                                    .rounded_full()
                                    .bg(theme.accent),
                            )
                        }),
                ),
        )
}
