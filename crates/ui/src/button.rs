use gpui::*;
use theme::Theme;

const MIN_BUTTON_CONTRAST: f32 = 3.0;

fn relative_luminance(color: Rgba) -> f32 {
    fn linear(channel: f32) -> f32 {
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast_ratio(first: Rgba, second: Rgba) -> f32 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn readable_text(background: Rgba, preferred: Rgba) -> Rgba {
    if contrast_ratio(background, preferred) >= MIN_BUTTON_CONTRAST {
        return preferred;
    }

    [rgb(0x000000), rgb(0xffffff)]
        .into_iter()
        .max_by(|first, second| {
            contrast_ratio(background, *first)
                .partial_cmp(&contrast_ratio(background, *second))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(rgb(0xffffff))
}

pub fn button(label: impl IntoElement, theme: &Theme, is_primary: bool) -> Div {
    let mut btn = div()
        .flex()
        .justify_center()
        .p_2()
        .px_4()
        .rounded_full()
        .cursor_pointer();

    if is_primary {
        let text = readable_text(theme.accent, theme.text_primary);
        btn = btn
            .bg(theme.accent)
            .hover(|s| s.bg(theme.accent))
            .text_color(text);
    } else {
        let text = readable_text(theme.bg_tab_inactive, theme.text_primary);
        let hover_text = readable_text(theme.border, theme.text_primary);
        btn = btn
            .bg(theme.bg_tab_inactive)
            .hover(move |s| s.bg(theme.border).text_color(hover_text))
            .text_color(text);
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
        let text = readable_text(theme.bg_tab_inactive, theme.ansi[1]);
        btn = btn.hover(move |s| s.bg(theme.bg_tab_inactive).text_color(text));
    } else {
        let text = readable_text(theme.bg_tab_inactive, theme.text_primary);
        btn = btn.hover(move |s| s.bg(theme.bg_tab_inactive).text_color(text));
    }

    btn.child(icon)
}

#[cfg(test)]
mod tests {
    use super::{MIN_BUTTON_CONTRAST, contrast_ratio, readable_text};
    use gpui::rgb;
    use theme::Theme;

    #[test]
    fn button_states_have_readable_label_colors() {
        for (name, factory) in Theme::builtins() {
            let theme = factory();
            let primary = readable_text(theme.accent, theme.text_primary);
            let secondary = readable_text(theme.bg_tab_inactive, theme.text_primary);
            let hover = readable_text(theme.border, theme.text_primary);

            assert!(
                contrast_ratio(theme.accent, primary) >= MIN_BUTTON_CONTRAST,
                "primary button contrast failed for {name}"
            );
            assert!(
                contrast_ratio(theme.bg_tab_inactive, secondary) >= MIN_BUTTON_CONTRAST,
                "secondary button contrast failed for {name}"
            );
            assert!(
                contrast_ratio(theme.border, hover) >= MIN_BUTTON_CONTRAST,
                "hover button contrast failed for {name}"
            );
        }

        assert_eq!(readable_text(rgb(0xffffff), rgb(0xffffff)), rgb(0x000000));
    }
}
