use crate::theme::Theme;
use crate::ui::button::button;
use crate::update::CURRENT_VERSION;
use crate::workspace::Workspace;
use gpui::prelude::*;
use gpui::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Appearance,
    Terminal,
    About,
}

fn nav_item(
    workspace: &Workspace,
    section: SettingsSection,
    label: &'static str,
    cx: &mut Context<Workspace>,
) -> Stateful<Div> {
    let theme = &workspace.state.theme;
    let active = workspace.settings_section == section;

    div()
        .id(label)
        .w_full()
        .p_2()
        .rounded_md()
        .cursor_pointer()
        .bg(if active {
            theme.bg_tab_inactive
        } else {
            theme.bg_sidebar
        })
        .text_color(if active {
            theme.text_primary
        } else {
            theme.text_muted
        })
        .hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.text_primary))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.select_settings_section(section, cx);
        }))
        .child(label)
}

fn theme_card(
    workspace: &Workspace,
    name: &'static str,
    factory: fn() -> Theme,
    cx: &mut Context<Workspace>,
) -> Stateful<Div> {
    let theme = &workspace.state.theme;
    let preview = factory();
    let active = workspace.state.theme_name.as_deref() == Some(name);
    let theme_name = name.to_string();

    div()
        .id(SharedString::from(format!("settings-theme-{name}")))
        .w_full()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(if active { theme.accent } else { theme.border })
        .bg(if active {
            theme.bg_tab_inactive
        } else {
            theme.bg_sidebar
        })
        .flex()
        .items_center()
        .gap_3()
        .cursor_pointer()
        .hover(|s| s.bg(theme.bg_tab_inactive))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.set_theme(factory, theme_name.clone(), cx);
        }))
        .child(
            div()
                .w(px(54.0))
                .h(px(34.0))
                .rounded_sm()
                .border_1()
                .border_color(preview.border)
                .bg(preview.bg_main)
                .flex()
                .items_center()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .w(px(7.0))
                        .h(px(7.0))
                        .rounded_full()
                        .bg(preview.ansi[1]),
                )
                .child(
                    div()
                        .w(px(7.0))
                        .h(px(7.0))
                        .rounded_full()
                        .bg(preview.ansi[2]),
                )
                .child(
                    div()
                        .w(px(7.0))
                        .h(px(7.0))
                        .rounded_full()
                        .bg(preview.ansi[4]),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(Theme::display_name(name))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(if active {
                            theme.accent
                        } else {
                            theme.text_muted
                        })
                        .child(if active { "Active" } else { "Click to apply" }),
                ),
        )
}

fn render_appearance(workspace: &Workspace, cx: &mut Context<Workspace>) -> Div {
    let theme = &workspace.state.theme;
    let mut themes = div().flex().flex_col().gap_2();
    for (name, factory) in Theme::builtins() {
        themes = themes.child(theme_card(workspace, name, factory, cx));
    }

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::BOLD)
                        .child("Appearance"),
                )
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child("Choose how vterm looks and feels."),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.text_muted)
                        .child("COLOR THEME"),
                )
                .child(themes),
        )
}

fn render_terminal(workspace: &Workspace, cx: &mut Context<Workspace>) -> Div {
    let theme = &workspace.state.theme;
    let font_size = workspace.state.font_size;

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::BOLD)
                        .child("Terminal"),
                )
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child("Tune the terminal reading experience."),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.bg_sidebar)
                .child(
                    div().flex().flex_col().gap_1().child("Font size").child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child("⌘ + / ⌘ -"),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(button("−", theme, false).w(px(36.0)).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, _window, cx| {
                                this.adjust_font_size(-1.0, cx);
                            }),
                        ))
                        .child(
                            div()
                                .w(px(54.0))
                                .flex()
                                .justify_center()
                                .text_color(theme.text_primary)
                                .child(format!("{font_size:.0} px")),
                        )
                        .child(button("+", theme, false).w(px(36.0)).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, _window, cx| {
                                this.adjust_font_size(1.0, cx);
                            }),
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.text_muted)
                        .child("KEYBOARD SHORTCUTS"),
                )
                .child(shortcut_row("New terminal", "⌘ T", theme))
                .child(shortcut_row("New workspace", "⌘ N", theme))
                .child(shortcut_row("Close terminal", "⌘ W", theme))
                .child(shortcut_row("Reset font size", "⌘ 0", theme)),
        )
}

fn shortcut_row(label: &'static str, shortcut: &'static str, theme: &Theme) -> Div {
    div()
        .flex()
        .justify_between()
        .items_center()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(label)
        .child(div().text_color(theme.text_muted).child(shortcut))
}

fn render_about(workspace: &Workspace, cx: &mut Context<Workspace>) -> Div {
    let theme = &workspace.state.theme;
    let update_status = if workspace.update_checking {
        "Checking for updates…".to_string()
    } else if workspace.update_downloading {
        "Downloading update in the background…".to_string()
    } else if workspace.update_staged.is_some() {
        format!(
            "v{} is ready — relaunch when convenient",
            workspace
                .update_info
                .as_ref()
                .map(|info| info.version.as_str())
                .unwrap_or_default()
        )
    } else if workspace.update_error.is_some() {
        "Could not prepare the update".to_string()
    } else if let Some(info) = &workspace.update_info {
        if info.can_auto_install {
            format!("v{} is available", info.version)
        } else {
            format!("v{} is available from the release page", info.version)
        }
    } else {
        "You're up to date".to_string()
    };

    let update_status_color = if workspace.update_staged.is_some() {
        theme.accent
    } else if workspace.update_error.is_some() {
        theme.ansi[1]
    } else {
        theme.text_muted
    };

    let mut update_card = div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(if workspace.update_staged.is_some() {
            theme.accent
        } else {
            theme.border
        })
        .bg(theme.bg_sidebar)
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div().flex().flex_col().gap_1().child("Updates").child(
                        div()
                            .text_size(px(11.0))
                            .text_color(update_status_color)
                            .child(update_status),
                    ),
                )
                .child(
                    button(
                        if workspace.update_checking {
                            "Checking…"
                        } else {
                            "Check for updates"
                        },
                        theme,
                        false,
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.check_for_updates(cx);
                        }),
                    ),
                ),
        );

    if let Some(info) = &workspace.update_info {
        let action_label = if workspace.update_installing {
            "Relaunching…"
        } else if workspace.update_staged.is_some() {
            "Relaunch to update"
        } else if workspace.update_downloading {
            "Downloading…"
        } else {
            "Download update"
        };

        update_card = update_card.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pt_2()
                .border_t_1()
                .border_color(theme.border)
                .child(format!("New version: v{}", info.version))
                .when(info.can_auto_install, |this| {
                    this.child(
                        button(action_label, theme, workspace.update_staged.is_some())
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event, _window, cx| {
                                    if this.update_staged.is_some() {
                                        this.install_update(cx);
                                    } else if let Some(info) = this.update_info.clone() {
                                        this.download_update(info, cx);
                                    }
                                }),
                            ),
                    )
                }),
        );
    }

    if workspace.update_downloading {
        let progress = workspace
            .update_download_progress
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        update_card = update_card.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .border_t_1()
                .border_color(theme.border)
                .pt_3()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_size(px(11.0))
                        .text_color(theme.text_muted)
                        .child("Downloading update")
                        .child(format!("{}%", (progress * 100.0).round() as u32)),
                )
                .child(
                    div()
                        .h(px(6.0))
                        .w_full()
                        .rounded_full()
                        .overflow_hidden()
                        .bg(theme.border)
                        .child(div().h_full().w(relative(progress)).bg(theme.accent)),
                ),
        );
    }

    if let Some(error) = &workspace.update_error {
        update_card = update_card.child(
            div()
                .text_size(px(11.0))
                .text_color(theme.ansi[1])
                .child(error.clone()),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::BOLD)
                        .child("About"),
                )
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child("A fast, native terminal for your daily work."),
                ),
        )
        .child(
            div()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.bg_sidebar)
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::BOLD)
                        .child("vterm"),
                )
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child(format!("Version {CURRENT_VERSION}")),
                )
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .child("Built with Rust and GPUI."),
                ),
        )
        .child(update_card)
}

pub fn render_settings(workspace: &Workspace, cx: &mut Context<Workspace>) -> impl IntoElement {
    let theme = &workspace.state.theme;
    let content = match workspace.settings_section {
        SettingsSection::Appearance => render_appearance(workspace, cx),
        SettingsSection::Terminal => render_terminal(workspace, cx),
        SettingsSection::About => render_about(workspace, cx),
    };

    div()
        .absolute()
        .inset_0()
        .bg(rgba(0x00000066))
        .flex()
        .justify_center()
        .items_center()
        .p_6()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _event, _window, cx| {
                this.toggle_settings(cx);
            }),
        )
        .child(
            div()
                .id("settings-dialog")
                .w(px(760.0))
                .h(px(560.0))
                .bg(theme.bg_main)
                .border_1()
                .border_color(theme.border)
                .rounded_xl()
                .shadow_lg()
                .flex()
                .flex_col()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .p_5()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_size(px(18.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("Settings"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(theme.text_muted)
                                        .child("Changes are applied instantly"),
                                ),
                        )
                        .child(
                            div()
                                .id("close-settings")
                                .w(px(28.0))
                                .h(px(28.0))
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(theme.text_muted)
                                .hover(|s| {
                                    s.bg(theme.bg_tab_inactive).text_color(theme.text_primary)
                                })
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _event, _window, cx| {
                                    this.toggle_settings(cx);
                                }))
                                .child("×"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(
                            div()
                                .w(px(160.0))
                                .flex_shrink_0()
                                .p_3()
                                .border_r_1()
                                .border_color(theme.border)
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(nav_item(
                                    workspace,
                                    SettingsSection::Appearance,
                                    "Appearance",
                                    cx,
                                ))
                                .child(nav_item(
                                    workspace,
                                    SettingsSection::Terminal,
                                    "Terminal",
                                    cx,
                                ))
                                .child(nav_item(workspace, SettingsSection::About, "About", cx)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .id("settings-content")
                                .overflow_y_scroll()
                                .p_6()
                                .child(content),
                        ),
                ),
        )
}
