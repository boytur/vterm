pub mod button;
pub mod modal;
pub mod text_input;

use gpui::Modifiers;

/// Menu-shortcut modifier: ⌘ on macOS, Ctrl elsewhere. Alt is excluded so
/// AltGr combos on international Windows layouts keep typing characters
/// instead of firing shortcuts.
pub fn shortcut_modifier(m: &Modifiers) -> bool {
    m.platform || (!cfg!(target_os = "macos") && m.control && !m.alt)
}

#[cfg(test)]
mod tests {
    use super::shortcut_modifier;
    use gpui::Modifiers;

    fn mods(platform: bool, control: bool, alt: bool) -> Modifiers {
        Modifiers {
            platform,
            control,
            alt,
            shift: false,
            function: false,
        }
    }

    #[test]
    fn shortcut_modifier_matches_platform_conventions() {
        // ⌘-style always works.
        assert!(shortcut_modifier(&mods(true, false, false)));
        // AltGr (Ctrl+Alt) never fires shortcuts — it types characters.
        assert!(!shortcut_modifier(&mods(false, true, true)));
        if cfg!(target_os = "macos") {
            assert!(!shortcut_modifier(&mods(false, true, false)));
        } else {
            assert!(shortcut_modifier(&mods(false, true, false)));
        }
    }
}
