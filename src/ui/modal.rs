use crate::theme::Theme;
use gpui::*;

pub fn modal_overlay(
    theme: &Theme,
    title: impl IntoElement,
    subtitle: impl IntoElement,
    content: impl IntoElement,
    actions: impl IntoElement,
) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(rgba(0x00000077))
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .w(px(320.0))
                .p_6()
                .bg(theme.bg_sidebar)
                .border_1()
                .border_color(theme.border)
                .rounded_xl()
                .shadow_lg()
                .flex()
                .flex_col()
                .items_center()
                .gap_4()
                .text_color(theme.text_primary)
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::BOLD)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.text_muted)
                        .child(subtitle),
                )
                .child(content)
                .child(
                    div()
                        .flex()
                        .w_full()
                        .justify_center()
                        .gap_3()
                        .mt_2()
                        .child(actions),
                ),
        )
}
