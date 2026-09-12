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
        if let Some(consumed) = self.apply_simple_key(key, modifiers) {
            return consumed;
        }
        match key {
            "c" | "x" if crate::shortcut_modifier(modifiers) => {
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
            "v" if crate::shortcut_modifier(modifiers) => {
                if let Some(item) = cx.read_from_clipboard()
                    && let Some(text) = item.text()
                {
                    self.insert(&text);
                }
                true
            }
            _ => false,
        }
    }

    /// Handles keys that don't need clipboard access. Returns `Some` when
    /// the key is fully handled here, `None` for clipboard keys that need
    /// `App`. Split out so unit tests can cover editing without a GPUI `App`.
    fn apply_simple_key(&mut self, key: &str, modifiers: &Modifiers) -> Option<bool> {
        match key {
            "enter" | "escape" => Some(false),
            "backspace" => {
                if !self.remove_selection() && self.caret > 0 {
                    let start = self.prev_boundary(self.caret);
                    self.value.replace_range(start..self.caret, "");
                    self.caret = start;
                }
                Some(true)
            }
            "delete" => {
                if !self.remove_selection() && self.caret < self.value.len() {
                    let end = self.next_boundary(self.caret);
                    self.value.replace_range(self.caret..end, "");
                }
                Some(true)
            }
            "left" => {
                self.move_caret(-1, modifiers.shift);
                Some(true)
            }
            "right" => {
                self.move_caret(1, modifiers.shift);
                Some(true)
            }
            "home" => {
                if !modifiers.shift {
                    self.anchor = None;
                }
                self.caret = 0;
                Some(true)
            }
            "end" => {
                if !modifiers.shift {
                    self.anchor = None;
                }
                self.caret = self.value.len();
                Some(true)
            }
            "a" if crate::shortcut_modifier(modifiers) => {
                self.select_all();
                Some(true)
            }
            "space" => {
                self.insert(" ");
                Some(true)
            }
            typed if typed.chars().count() == 1 => {
                // GPUI already applies Shift to `key` ("a"+Shift arrives as
                // "A", "1"+Shift as "!"). Uppercasing again would corrupt
                // symbols and non-ASCII input, so insert verbatim. Ignore
                // shortcut combos (Cmd/Ctrl/Alt) here; they are handled above
                // or by the caller.
                if modifiers.platform || modifiers.control || modifiers.alt {
                    return Some(false);
                }
                self.insert(typed);
                Some(true)
            }
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::TextField;
    use gpui::Modifiers;

    fn no_mods() -> Modifiers {
        Modifiers {
            platform: false,
            shift: false,
            control: false,
            alt: false,
            function: false,
        }
    }

    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..no_mods()
        }
    }

    #[test]
    fn inserts_shifted_keys_verbatim() {
        let mut field = TextField::new("");
        // GPUI delivers the shifted glyph already ("A", "!"); the field must
        // not uppercase again or "!" would degrade to "1".
        assert_eq!(field.apply_simple_key("A", &shift()), Some(true));
        assert_eq!(field.apply_simple_key("!", &shift()), Some(true));
        assert_eq!(field.value(), "A!");
    }

    #[test]
    fn ignores_shortcut_combos_without_clipboard() {
        let mut field = TextField::new("hi");
        let mods = Modifiers {
            platform: true,
            ..no_mods()
        };
        // Cmd+C with no selection, Cmd+X etc. are clipboard paths; the simple
        // path must not insert text for Ctrl/Alt combos.
        let ctrl = Modifiers {
            control: true,
            ..no_mods()
        };
        assert_eq!(field.apply_simple_key("c", &ctrl), Some(false));
        assert_eq!(field.value(), "hi");
        assert_eq!(field.apply_simple_key("a", &mods), Some(true));
    }

    #[test]
    fn backspace_removes_combining_codepoint() {
        let mut field = TextField::new("สวัสดี");
        assert_eq!(field.apply_simple_key("backspace", &no_mods()), Some(true));
        assert_eq!(field.value(), "สวัสด");
    }

    #[test]
    fn ctrl_a_selects_all_off_macos() {
        // Ctrl is the menu modifier everywhere except macOS.
        let mut field = TextField::new("hi");
        let ctrl = Modifiers {
            control: true,
            ..no_mods()
        };
        field.apply_simple_key("a", &ctrl);
        field.apply_simple_key("backspace", &no_mods());
        if cfg!(target_os = "macos") {
            assert_eq!(field.value(), "h", "Ctrl+A must not select on macOS");
        } else {
            assert_eq!(field.value(), "", "Ctrl+A must select all off macOS");
        }
    }
}
