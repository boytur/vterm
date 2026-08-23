use gpui::*;

#[derive(Clone)]
pub struct Theme {
    pub bg_main: Rgba,
    pub bg_sidebar: Rgba,
    pub bg_tab_active: Rgba,
    pub bg_tab_inactive: Rgba,
    pub border: Rgba,
    pub text_primary: Rgba,
    pub text_muted: Rgba,
    pub accent: Rgba,
    pub ansi: [Rgba; 16],
}

impl Default for Theme {
    fn default() -> Self {
        Self::zed_dark()
    }
}

impl Theme {
    #[allow(dead_code)]
    pub fn ubuntu() -> Self {
        Self {
            bg_main: rgb(0x300a24),
            bg_sidebar: rgb(0x2a081f),
            bg_tab_active: rgb(0x300a24),
            bg_tab_inactive: rgb(0x401030),
            border: rgb(0x501c3d),
            text_primary: rgb(0xffffff),
            text_muted: rgb(0xcccccc),
            accent: rgb(0xe95420),
            ansi: [
                rgb(0x2e3436),
                rgb(0xcc0000),
                rgb(0x4e9a06),
                rgb(0xc4a000),
                rgb(0x3465a4),
                rgb(0x75507b),
                rgb(0x06989a),
                rgb(0xd3d7cf),
                rgb(0x555753),
                rgb(0xef2929),
                rgb(0x8ae234),
                rgb(0xfce94f),
                rgb(0x729fcf),
                rgb(0xad7fa8),
                rgb(0x34e2e2),
                rgb(0xeeeeec),
            ],
        }
    }

    #[allow(dead_code)]
    pub fn zed_dark() -> Self {
        Self {
            bg_main: rgb(0x1e1e1e),
            bg_sidebar: rgb(0x252526),
            bg_tab_active: rgb(0x1e1e1e),
            bg_tab_inactive: rgb(0x2d2d2d),
            border: rgb(0x3c3c3c),
            text_primary: rgb(0xd4d4d4),
            text_muted: rgb(0x969696),
            accent: rgb(0x007fd4),
            ansi: [
                rgb(0x1e1e1e),
                rgb(0xf2495a),
                rgb(0x2fb943),
                rgb(0xe2b93d),
                rgb(0x117df5),
                rgb(0xb267e6),
                rgb(0x35b3b3),
                rgb(0xe0e0e0),
                rgb(0x808080),
                rgb(0xf97380),
                rgb(0x4bd05e),
                rgb(0xeadd5c),
                rgb(0x3d9bf9),
                rgb(0xcb8df1),
                rgb(0x56d4d4),
                rgb(0xffffff),
            ],
        }
    }

    #[allow(dead_code)]
    pub fn dracula() -> Self {
        Self {
            bg_main: rgb(0x282a36),
            bg_sidebar: rgb(0x21222c),
            bg_tab_active: rgb(0x282a36),
            bg_tab_inactive: rgb(0x21222c),
            border: rgb(0x44475a),
            text_primary: rgb(0xf8f8f2),
            text_muted: rgb(0x6272a4),
            accent: rgb(0xbd93f9), // Purple
            ansi: [
                rgb(0x21222c), // Black
                rgb(0xff5555), // Red
                rgb(0x50fa7b), // Green
                rgb(0xf1fa8c), // Yellow
                rgb(0xbd93f9), // Blue
                rgb(0xff79c6), // Magenta
                rgb(0x8be9fd), // Cyan
                rgb(0xf8f8f2), // White
                rgb(0x6272a4), // Bright Black
                rgb(0xff6e6e), // Bright Red
                rgb(0x69ff94), // Bright Green
                rgb(0xffffa5), // Bright Yellow
                rgb(0xd6acff), // Bright Blue
                rgb(0xff92df), // Bright Magenta
                rgb(0xa4ffff), // Bright Cyan
                rgb(0xffffff), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn nord() -> Self {
        Self {
            bg_main: rgb(0x2e3440),
            bg_sidebar: rgb(0x242933),
            bg_tab_active: rgb(0x2e3440),
            bg_tab_inactive: rgb(0x242933),
            border: rgb(0x3b4252),
            text_primary: rgb(0xd8dee9),
            text_muted: rgb(0x4c566a),
            accent: rgb(0x88c0d0), // Frost Blue
            ansi: [
                rgb(0x3b4252), // Black
                rgb(0xbf616a), // Red
                rgb(0xa3be8c), // Green
                rgb(0xebcb8b), // Yellow
                rgb(0x81a1c1), // Blue
                rgb(0xb48ead), // Magenta
                rgb(0x88c0d0), // Cyan
                rgb(0xe5e9f0), // White
                rgb(0x4c566a), // Bright Black
                rgb(0xbf616a), // Bright Red
                rgb(0xa3be8c), // Bright Green
                rgb(0xebcb8b), // Bright Yellow
                rgb(0x81a1c1), // Bright Blue
                rgb(0xb48ead), // Bright Magenta
                rgb(0x8fbcbb), // Bright Cyan
                rgb(0xeceff4), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn gruvbox_dark() -> Self {
        Self {
            bg_main: rgb(0x282828),
            bg_sidebar: rgb(0x1d2021),
            bg_tab_active: rgb(0x282828),
            bg_tab_inactive: rgb(0x1d2021),
            border: rgb(0x3c3836),
            text_primary: rgb(0xebdbb2),
            text_muted: rgb(0x928374),
            accent: rgb(0xfe8019), // Orange
            ansi: [
                rgb(0x282828), // Black
                rgb(0xcc241d), // Red
                rgb(0x98971a), // Green
                rgb(0xd79921), // Yellow
                rgb(0x458588), // Blue
                rgb(0xb16286), // Magenta
                rgb(0x689d6a), // Cyan
                rgb(0xa89984), // White
                rgb(0x928374), // Bright Black
                rgb(0xfb4934), // Bright Red
                rgb(0xb8bb26), // Bright Green
                rgb(0xfabd2f), // Bright Yellow
                rgb(0x83a598), // Bright Blue
                rgb(0xd3869b), // Bright Magenta
                rgb(0x8ec07c), // Bright Cyan
                rgb(0xebdbb2), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn one_dark() -> Self {
        Self {
            bg_main: rgb(0x282c34),
            bg_sidebar: rgb(0x21252b),
            bg_tab_active: rgb(0x282c34),
            bg_tab_inactive: rgb(0x21252b),
            border: rgb(0x181a1f),
            text_primary: rgb(0xabb2bf),
            text_muted: rgb(0x5c6370),
            accent: rgb(0x61afef), // Blue
            ansi: [
                rgb(0x282c34), // Black
                rgb(0xe06c75), // Red
                rgb(0x98c379), // Green
                rgb(0xe5c07b), // Yellow
                rgb(0x61afef), // Blue
                rgb(0xc678dd), // Magenta
                rgb(0x56b6c2), // Cyan
                rgb(0xabb2bf), // White
                rgb(0x5c6370), // Bright Black
                rgb(0xe06c75), // Bright Red
                rgb(0x98c379), // Bright Green
                rgb(0xe5c07b), // Bright Yellow
                rgb(0x61afef), // Bright Blue
                rgb(0xc678dd), // Bright Magenta
                rgb(0x56b6c2), // Bright Cyan
                rgb(0xffffff), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn solarized_dark() -> Self {
        Self {
            bg_main: rgb(0x002b36),
            bg_sidebar: rgb(0x00212b),
            bg_tab_active: rgb(0x002b36),
            bg_tab_inactive: rgb(0x00212b),
            border: rgb(0x073642),
            text_primary: rgb(0x839496),
            text_muted: rgb(0x586e75),
            accent: rgb(0x268bd2), // Blue
            ansi: [
                rgb(0x073642), // Black
                rgb(0xdc322f), // Red
                rgb(0x859900), // Green
                rgb(0xb58900), // Yellow
                rgb(0x268bd2), // Blue
                rgb(0xd33682), // Magenta
                rgb(0x2aa198), // Cyan
                rgb(0xeee8d5), // White
                rgb(0x002b36), // Bright Black
                rgb(0xcb4b16), // Bright Red
                rgb(0x586e75), // Bright Green
                rgb(0x657b83), // Bright Yellow
                rgb(0x839496), // Bright Blue
                rgb(0x6c71c4), // Bright Magenta
                rgb(0x93a1a1), // Bright Cyan
                rgb(0xfdf6e3), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn catppuccin_mocha() -> Self {
        Self {
            bg_main: rgb(0x1e1e2e),
            bg_sidebar: rgb(0x11111b),
            bg_tab_active: rgb(0x1e1e2e),
            bg_tab_inactive: rgb(0x181825),
            border: rgb(0x313244),
            text_primary: rgb(0xcdd6f4),
            text_muted: rgb(0x7f849c),
            accent: rgb(0xcba6f7), // Mauve
            ansi: [
                rgb(0x45475a), // Surface 1
                rgb(0xf38ba8), // Red
                rgb(0xa6e3a1), // Green
                rgb(0xf9e2af), // Yellow
                rgb(0x89b4fa), // Blue
                rgb(0xf5c2e7), // Pink
                rgb(0x94e2d5), // Teal
                rgb(0xbac2de), // Subtext 1
                rgb(0x585b70), // Surface 2
                rgb(0xf38ba8), // Bright Red
                rgb(0xa6e3a1), // Bright Green
                rgb(0xf9e2af), // Bright Yellow
                rgb(0x89b4fa), // Bright Blue
                rgb(0xf5c2e7), // Bright Pink
                rgb(0x94e2d5), // Bright Teal
                rgb(0xa6adc8), // Subtext 0
            ],
        }
    }

    #[allow(dead_code)]
    pub fn tokyo_night() -> Self {
        Self {
            bg_main: rgb(0x1a1b26),
            bg_sidebar: rgb(0x16161e),
            bg_tab_active: rgb(0x1a1b26),
            bg_tab_inactive: rgb(0x16161e),
            border: rgb(0x292e42),
            text_primary: rgb(0xc0caf5),
            text_muted: rgb(0x565f89),
            accent: rgb(0x7aa2f7), // Blue
            ansi: [
                rgb(0x15161e), // Black
                rgb(0xf7768e), // Red
                rgb(0x9ece6a), // Green
                rgb(0xe0af68), // Yellow
                rgb(0x7aa2f7), // Blue
                rgb(0xbb9af7), // Magenta
                rgb(0x7dcfff), // Cyan
                rgb(0xa9b1d6), // White
                rgb(0x414868), // Bright Black
                rgb(0xf7768e), // Bright Red
                rgb(0x9ece6a), // Bright Green
                rgb(0xe0af68), // Bright Yellow
                rgb(0x7aa2f7), // Bright Blue
                rgb(0xbb9af7), // Bright Magenta
                rgb(0x7dcfff), // Bright Cyan
                rgb(0xc0caf5), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn monokai() -> Self {
        Self {
            bg_main: rgb(0x272822),
            bg_sidebar: rgb(0x1e1f1c),
            bg_tab_active: rgb(0x272822),
            bg_tab_inactive: rgb(0x1e1f1c),
            border: rgb(0x3e3d32),
            text_primary: rgb(0xf8f8f2),
            text_muted: rgb(0x75715e),
            accent: rgb(0xf92672), // Pink/Red
            ansi: [
                rgb(0x272822), // Black
                rgb(0xf92672), // Red
                rgb(0xa6e22e), // Green
                rgb(0xf4bf75), // Yellow
                rgb(0x66d9ef), // Blue
                rgb(0xae81ff), // Magenta
                rgb(0xa1efe4), // Cyan
                rgb(0xf8f8f2), // White
                rgb(0x75715e), // Bright Black
                rgb(0xf92672), // Bright Red
                rgb(0xa6e22e), // Bright Green
                rgb(0xf4bf75), // Bright Yellow
                rgb(0x66d9ef), // Bright Blue
                rgb(0xae81ff), // Bright Magenta
                rgb(0xa1efe4), // Bright Cyan
                rgb(0xf9f8f5), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn ayu_dark() -> Self {
        Self {
            bg_main: rgb(0x0f1419),
            bg_sidebar: rgb(0x0b0e14),
            bg_tab_active: rgb(0x0f1419),
            bg_tab_inactive: rgb(0x0b0e14),
            border: rgb(0x242d35),
            text_primary: rgb(0xe6e1cf),
            text_muted: rgb(0x5c6773),
            accent: rgb(0xffb454), // Orange
            ansi: [
                rgb(0x000000), // Black
                rgb(0xff3333), // Red
                rgb(0xb8cc52), // Green
                rgb(0xe7c547), // Yellow
                rgb(0x36a3d9), // Blue
                rgb(0xf07178), // Magenta
                rgb(0x95e6cb), // Cyan
                rgb(0xffffff), // White
                rgb(0x323232), // Bright Black
                rgb(0xff6565), // Bright Red
                rgb(0xeaee0a), // Bright Green
                rgb(0xe6e1cf), // Bright Yellow
                rgb(0x68a1f0), // Bright Blue
                rgb(0xf07178), // Bright Magenta
                rgb(0xc3e88d), // Bright Cyan
                rgb(0xffffff), // Bright White
            ],
        }
    }

    #[allow(dead_code)]
    pub fn github_dark() -> Self {
        Self {
            bg_main: rgb(0x0d1117),
            bg_sidebar: rgb(0x010409),
            bg_tab_active: rgb(0x0d1117),
            bg_tab_inactive: rgb(0x010409),
            border: rgb(0x30363d),
            text_primary: rgb(0xc9d1d9),
            text_muted: rgb(0x8b949e),
            accent: rgb(0x58a6ff), // Blue
            ansi: [
                rgb(0x484f58), // Black
                rgb(0xff7b72), // Red
                rgb(0x3fb950), // Green
                rgb(0xd29922), // Yellow
                rgb(0x58a6ff), // Blue
                rgb(0xbc8cff), // Magenta
                rgb(0x39c5cf), // Cyan
                rgb(0xb1bac4), // White
                rgb(0x6e7681), // Bright Black
                rgb(0xffa198), // Bright Red
                rgb(0x56d364), // Bright Green
                rgb(0xe3b341), // Bright Yellow
                rgb(0x79c0ff), // Bright Blue
                rgb(0xd2a8ff), // Bright Magenta
                rgb(0x56d4dd), // Bright Cyan
                rgb(0xf0f6fc), // Bright White
            ],
        }
    }
}

