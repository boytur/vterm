use gpui::*;
use std::time::Duration;
use theme::Theme;

/// Minimal single-line editable text field with caret, selection and
/// clipboard support. gpui ships no input element, so modals (rename, ...)
/// build on this.
#[derive(Debug, Clone)]
pub struct TextField {
    value: String,
    /// Byte offset of the caret; always on a char boundary.
    caret: usize,
    /// Byte offset where the active selection started, if any.
    anchor: Option<usize>,
    /// Extra left padding (px) so an icon can sit inside the box.
    left_pad: f32,
}

impl TextField {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let caret = value.len();
        Self {
            value,
            caret,
            anchor: None,
            left_pad: 0.0,
        }
    }

    /// Reserve `pad` px of left padding inside the box (for an inline icon).
    pub fn with_left_pad(mut self, pad: f32) -> Self {
        self.left_pad = pad;
        self
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        Some((anchor.min(self.caret), anchor.max(self.caret)))
    }

    fn select_all(&mut self) {
        self.anchor = Some(0);
        self.caret = self.value.len();
    }

    fn remove_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection() {
            self.value.replace_range(start..end, "");
            self.caret = start;
            self.anchor = None;
            true
        } else {
            false
        }
    }

    fn insert(&mut self, text: &str) {
        self.remove_selection();
        self.value.insert_str(self.caret, text);
        self.caret += text.len();
    }

    fn prev_boundary(&self, index: usize) -> usize {
        self.value[..index]
            .char_indices()
            .next_back()
            .map_or(0, |(idx, _)| idx)
    }

    fn next_boundary(&self, index: usize) -> usize {
        self.value[index..]
            .chars()
            .next()
            .map_or(index, |c| index + c.len_utf8())
    }

    fn move_caret(&mut self, delta: isize, extend: bool) {
        let target = if delta < 0 {
            self.prev_boundary(self.caret)
        } else if delta > 0 {
            self.next_boundary(self.caret)
        } else {
            self.caret
        };
        if extend {
            if self.anchor.is_none() {
                // keep the old caret as the fixed end of the growing selection
                self.anchor = Some(self.caret);
            }
        } else {
            self.anchor = None;
        }
        self.caret = target;
    }

    /// Handles one keystroke; returns true when the field consumed it.
    /// `enter`/`escape` are left to the caller for confirm/cancel semantics.
    pub fn key(&mut self, key: &str, modifiers: &Modifiers, cx: &App) -> bool {
        match key {
            "enter" | "escape" => false,
            "backspace" => {
                if !self.remove_selection() && self.caret > 0 {
                    let start = self.prev_boundary(self.caret);
                    self.value.replace_range(start..self.caret, "");
                    self.caret = start;
                }
                true
            }
            "delete" => {
                if !self.remove_selection() && self.caret < self.value.len() {
                    let end = self.next_boundary(self.caret);
                    self.value.replace_range(self.caret..end, "");
                }
                true
            }
            "left" => {
                self.move_caret(-1, modifiers.shift);
                true
            }
            "right" => {
                self.move_caret(1, modifiers.shift);
                true
            }
            "home" => {
                if !modifiers.shift {
                    self.anchor = None;
                }
                self.caret = 0;
                true
            }
            "end" => {
                if !modifiers.shift {
                    self.anchor = None;
                }
                self.caret = self.value.len();
                true
            }
            "a" if modifiers.platform => {
                self.select_all();
                true
            }
            "c" | "x" if modifiers.platform => {
                let (start, end) = match self.selection() {
                    Some((s, e)) => (s, e),
                    None => return true,
                };
                cx.write_to_clipboard(ClipboardItem::new_string(
                    self.value[start..end].to_string(),
                ));
                if key == "x" {
                    self.remove_selection();
                }
                true
            }
            "v" if modifiers.platform => {
                if let Some(item) = cx.read_from_clipboard()
                    && let Some(text) = item.text()
                {
                    self.insert(&text);
                }
                true
            }
            "space" => {
                self.insert(" ");
                true
            }
            typed if typed.chars().count() == 1 => {
                let c = typed.chars().next().unwrap();
                let text = if modifiers.shift {
                    c.to_ascii_uppercase().to_string()
                } else {
                    c.to_string()
                };
                self.insert(&text);
                true
            }
            _ => false,
        }
    }

    /// Renders the bordered edit box with selection highlight and caret.
    pub fn render(&self, theme: &Theme) -> Div {
        let (start, end) = match self.selection() {
            Some((s, e)) => (s, e),
            None => (self.caret, self.caret),
        };
        let before = &self.value[..start];
        let selected = &self.value[start..end];
        let after = &self.value[end..];

        let mut row = div().flex().flex_row().items_start();

        if !before.is_empty() {
            row = row.child(text_span(before));
        }

        if selected.is_empty() {
            row = row.child(
                div()
                    .w(px(1.5))
                    .h(px(14.0))
                    .mt(px(2.0))
                    .flex_shrink_0()
                    .bg(theme.text_primary)
                    .with_animation(
                        "text-field-caret",
                        Animation::new(Duration::from_millis(1000))
                            .repeat()
                            .with_easing(|t| 0.5 + 0.5 * (t * 2.0 * std::f32::consts::PI).cos()),
                        |el, alpha| el.opacity(alpha),
                    ),
            );
        } else {
            row = row.child(text_span(selected).bg(gpui::rgba(0x0a84ff55)));
        }

        if !after.is_empty() {
            row = row.child(text_span(after));
        }

        div()
            .w_full()
            .h(px(34.0))
            .p_2()
            .pt(px(6.0))
            .pl(px(self.left_pad))
            .bg(theme.bg_tab_inactive)
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .child(row)
    }
}

fn text_span(text: &str) -> Div {
    div().whitespace_nowrap().child(text.to_string())
}
