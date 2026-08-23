use crate::theme::Theme;
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
            .bg(rgb(0x0a84ff))
            .hover(|s| s.bg(rgb(0x007aff)))
            .text_color(rgb(0xffffff));
    } else {
        btn = btn
            .bg(rgb(0x3a3a3c))
            .hover(|s| s.bg(rgb(0x4a4a4c)))
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
        btn = btn.hover(|s| s.bg(rgb(0x4a4d4e)).text_color(rgb(0xff5555)));
    } else {
        btn = btn.hover(|s| s.bg(rgb(0x4a4d4e)).text_color(theme.text_primary));
    }

    btn.child(icon)
}
