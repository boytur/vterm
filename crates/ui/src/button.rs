use theme::Theme;
use gpui::*;

pub fn button(label: impl IntoElement, theme: &Theme, is_primary: bool) -> Div {
    let mut btn = div()
        .flex()
        .justify_center()
        .p_2()
        .px_4()
        .rounded_full()
        .cursor_pointer();

    if is_primary {
        btn = btn
            .bg(theme.accent)
            .hover(|s| s.bg(theme.accent))
            .text_color(rgb(0xffffff));
    } else {
        btn = btn
            .bg(theme.bg_tab_inactive)
            .hover(|s| s.bg(theme.border))
            .text_color(theme.text_primary);
    }

    btn.child(label)
}

#[allow(dead_code)]
pub fn icon_button(icon: impl IntoElement, theme: &Theme, is_danger: bool) -> Div {
    let mut btn = div()
        .p_0()
        .w(px(20.0))
        .h(px(20.0))
        .flex()
        .justify_center()
        .items_center()
        .rounded_full()
        .text_color(theme.text_muted);

    if is_danger {
        btn = btn.hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.ansi[1]));
    } else {
        btn = btn.hover(|s| s.bg(theme.bg_tab_inactive).text_color(theme.text_primary));
    }

    btn.child(icon)
}
