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
    pub fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("light") => Self::light(),
            Some("midnight") => Self::midnight(),
            Some("ocean") => Self::ocean(),
            Some("forest") => Self::forest(),
            Some("rose") => Self::rose(),
            Some("paper") => Self::paper(),
            Some("lavender") => Self::lavender(),
            Some("sand") => Self::sand(),
            Some("high_contrast") => Self::high_contrast(),
            Some("one_light") => Self::one_light(),
            Some("vscode_light_plus") => Self::vscode_light_plus(),
            Some("vscode_quiet_light") => Self::vscode_quiet_light(),
            Some("solarized_light") => Self::solarized_light(),
            Some("ubuntu") => Self::ubuntu(),
            Some("vscode_dark_plus") => Self::vscode_dark_plus(),
            Some("vscode_abyss") => Self::vscode_abyss(),
            Some("dracula") => Self::dracula(),
            Some("nord") => Self::nord(),
            Some("gruvbox_dark") => Self::gruvbox_dark(),
            Some("one_dark") => Self::one_dark(),
            Some("solarized_dark") => Self::solarized_dark(),
            Some("catppuccin_mocha") => Self::catppuccin_mocha(),
            Some("tokyo_night") => Self::tokyo_night(),
            Some("monokai") => Self::monokai(),
            Some("ayu_dark") => Self::ayu_dark(),
            Some("github_dark") => Self::github_dark(),
            _ => Self::zed_dark(),
        }
    }

    pub fn builtins() -> [(&'static str, fn() -> Self); 27] {
        let mut themes: [(&'static str, fn() -> Self); 27] = [
            ("light", Self::light),
            ("midnight", Self::midnight),
            ("ocean", Self::ocean),
            ("forest", Self::forest),
            ("rose", Self::rose),
            ("paper", Self::paper),
            ("lavender", Self::lavender),
            ("sand", Self::sand),
            ("high_contrast", Self::high_contrast),
            ("one_light", Self::one_light),
            ("vscode_light_plus", Self::vscode_light_plus),
            ("vscode_quiet_light", Self::vscode_quiet_light),
            ("solarized_light", Self::solarized_light),
            ("ubuntu", Self::ubuntu),
            ("zed_dark", Self::zed_dark),
            ("vscode_dark_plus", Self::vscode_dark_plus),
            ("vscode_abyss", Self::vscode_abyss),
            ("dracula", Self::dracula),
            ("nord", Self::nord),
            ("gruvbox_dark", Self::gruvbox_dark),
            ("one_dark", Self::one_dark),
            ("solarized_dark", Self::solarized_dark),
            ("catppuccin_mocha", Self::catppuccin_mocha),
            ("tokyo_night", Self::tokyo_night),
            ("monokai", Self::monokai),
            ("ayu_dark", Self::ayu_dark),
            ("github_dark", Self::github_dark),
        ];
        themes.sort_by_key(|(name, _)| Self::display_name(name));
        themes
    }

    pub fn display_name(name: &str) -> String {
        if let Some(rest) = name.strip_prefix("vscode_") {
            return format!("VS Code {}", Self::display_name(rest));
        }
        name.split('_')
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn is_dark(name: &str) -> bool {
        !matches!(
            name,
            "light"
                | "paper"
                | "lavender"
                | "sand"
                | "one_light"
                | "vscode_light_plus"
                | "vscode_quiet_light"
                | "solarized_light"
        )
    }

    #[allow(dead_code)]
    pub fn light() -> Self {
        Self {
            bg_main: rgb(0xf7f8fa),
            bg_sidebar: rgb(0xedf0f3),
            bg_tab_active: rgb(0xffffff),
            bg_tab_inactive: rgb(0xe3e7eb),
            border: rgb(0xd1d7de),
            text_primary: rgb(0x1f2328),
            text_muted: rgb(0x656d76),
            accent: rgb(0x0969da),
            ansi: [
                rgb(0x24292f),
                rgb(0xcf222e),
                rgb(0x1a7f37),
                rgb(0x9a6700),
                rgb(0x0969da),
                rgb(0x8250df),
                rgb(0x1b7c83),
                rgb(0x6e7781),
                rgb(0x57606a),
                rgb(0xa40e26),
                rgb(0x116329),
                rgb(0x7d4e00),
                rgb(0x218bff),
                rgb(0x8250df),
                rgb(0x3192aa),
                rgb(0x8c959f),
            ],
        }
    }

    fn from_palette(
        bg_main: u32,
        bg_sidebar: u32,
        bg_tab_active: u32,
        bg_tab_inactive: u32,
        border: u32,
        text_primary: u32,
        text_muted: u32,
        accent: u32,
        ansi: [u32; 16],
    ) -> Self {
        Self {
            bg_main: rgb(bg_main),
            bg_sidebar: rgb(bg_sidebar),
            bg_tab_active: rgb(bg_tab_active),
            bg_tab_inactive: rgb(bg_tab_inactive),
            border: rgb(border),
            text_primary: rgb(text_primary),
            text_muted: rgb(text_muted),
            accent: rgb(accent),
            ansi: ansi.map(|color| rgb(color)),
        }
    }

    pub fn midnight() -> Self {
        Self::from_palette(
            0x141827,
            0x0f1220,
            0x141827,
            0x20263a,
            0x303852,
            0xe4e7f2,
            0x9098b8,
            0x8ea7ff,
            [
                0x1a1e2e, 0xff6b81, 0x7ed6a5, 0xf2c97d, 0x8ea7ff, 0xc6a0f6, 0x7dd8d3, 0xd5d9e5,
                0x59617a, 0xff8fa0, 0x9ae6bc, 0xffdda0, 0xb1c2ff, 0xdabfff, 0x9af2ec, 0xffffff,
            ],
        )
    }

    pub fn ocean() -> Self {
        Self::from_palette(
            0x10202a,
            0x0d1a22,
            0x10202a,
            0x18323e,
            0x28505e,
            0xd6f0f0,
            0x7ea5ad,
            0x4fd1c5,
            [
                0x102b36, 0xff6b6b, 0x8bd49c, 0xe9c46a, 0x5aa9e6, 0xc084d6, 0x4fd1c5, 0xc8e6e6,
                0x42606b, 0xff9696, 0xa9e8b7, 0xf4d58a, 0x86c7ff, 0xe0a5ef, 0x83ebe3, 0xf2ffff,
            ],
        )
    }

    pub fn forest() -> Self {
        Self::from_palette(
            0x17201a,
            0x101711,
            0x17201a,
            0x233126,
            0x354735,
            0xdce8dc,
            0x8fa28e,
            0x8fcf72,
            [
                0x202b22, 0xeb6f6f, 0x8fcf72, 0xe3c477, 0x7ab8d8, 0xc39bd3, 0x72c9bc, 0xd3ded2,
                0x536356, 0xff9494, 0xa9e58e, 0xf1d99a, 0x9bcdf0, 0xddb8e7, 0x91e8da, 0xf2fff0,
            ],
        )
    }

    pub fn rose() -> Self {
        Self::from_palette(
            0x21171d,
            0x180f15,
            0x21171d,
            0x30212a,
            0x49313d,
            0xf2e1e8,
            0xb49aa8,
            0xf08cae,
            [
                0x281c24, 0xf06c8b, 0x9dcc82, 0xe7c57b, 0x8fb8e8, 0xd2a0dc, 0x7fc8c5, 0xe9dce2,
                0x66505e, 0xff93ac, 0xb7e49b, 0xf4d998, 0xb0d0ff, 0xe9b8ec, 0x9be7e0, 0xffffff,
            ],
        )
    }

    pub fn paper() -> Self {
        Self::from_palette(
            0xfbf8f2,
            0xf1ece2,
            0xfffdf8,
            0xe8e0d3,
            0xd7cbbb,
            0x3a332b,
            0x85776a,
            0xb45f28,
            [
                0x3a332b, 0xb33a3a, 0x4f7d3a, 0x9a6a16, 0x356aa0, 0x8654a8, 0x287f7a, 0x8e857a,
                0x6b6259, 0xd05353, 0x72a95a, 0xc18c29, 0x5f91c9, 0xa978c5, 0x48aaa2, 0xfffdf8,
            ],
        )
    }

    pub fn lavender() -> Self {
        Self::from_palette(
            0xf7f4ff,
            0xeee9fb,
            0xffffff,
            0xe7def7,
            0xd4c5eb,
            0x352e42,
            0x847a98,
            0x7957c8,
            [
                0x352e42, 0xc4475a, 0x4d8a62, 0xa26f19, 0x526ebd, 0x7957c8, 0x3d8c8c, 0x82798f,
                0x675d72, 0xe15e73, 0x72b887, 0xc18d32, 0x718edc, 0x9b7ce8, 0x5bb8b5, 0xffffff,
            ],
        )
    }

    pub fn sand() -> Self {
        Self::from_palette(
            0xf8f3e8,
            0xeee5d5,
            0xfcf8ef,
            0xe4d7c2,
            0xd2c1a6,
            0x40372d,
            0x8c7b68,
            0xb86b3e,
            [
                0x40372d, 0xb64945, 0x5c873f, 0xa57422, 0x4f79a8, 0x8b5ca8, 0x3d8580, 0x8e8373,
                0x6b5d4e, 0xd8665f, 0x82aa5d, 0xc99a3b, 0x7099c8, 0xb27bc6, 0x5aada5, 0xfff9ec,
            ],
        )
    }

    pub fn high_contrast() -> Self {
        Self::from_palette(
            0x000000,
            0x080808,
            0x000000,
            0x1b1b1b,
            0x777777,
            0xffffff,
            0xc8c8c8,
            0x00e5ff,
            [
                0x000000, 0xff3333, 0x33ff66, 0xffff33, 0x3399ff, 0xff66ff, 0x33ffff, 0xffffff,
                0x888888, 0xff7777, 0x77ff99, 0xffff77, 0x77bbff, 0xff99ff, 0x77ffff, 0xffffff,
            ],
        )
    }

    pub fn one_light() -> Self {
        Self::from_palette(
            0xfafafa,
            0xf3f3f3,
            0xfafafa,
            0xe5e5e6,
            0xd0d0d0,
            0x383a42,
            0xa0a1a7,
            0x4078f2,
            [
                0x383a42, 0xe45649, 0x50a14f, 0xc18401, 0x4078f2, 0xa626a4, 0x0184bb, 0xfafafa,
                0x696c77, 0xf44747, 0x2d9d35, 0xd19a66, 0x709dff, 0xd16bca, 0x00a7aa, 0xffffff,
            ],
        )
    }

    pub fn vscode_light_plus() -> Self {
        Self::from_palette(
            0xffffff,
            0xf3f3f3,
            0xffffff,
            0xe7e7e7,
            0xd4d4d4,
            0x333333,
            0x6a737d,
            0x0066bf,
            [
                0x000000, 0xcd3131, 0x008000, 0x795e26, 0x0451a5, 0xaf00db, 0x267f99, 0x666666,
                0x808080, 0xcd3131, 0x14ce14, 0xb89500, 0x0451a5, 0xbc05bc, 0x2aa1ae, 0xffffff,
            ],
        )
    }

    pub fn vscode_quiet_light() -> Self {
        Self::from_palette(
            0xf5f5f5,
            0xeeeeee,
            0xf5f5f5,
            0xe3e3e3,
            0xd5d5d5,
            0x333333,
            0x7a7a7a,
            0x2f6f9f,
            [
                0x333333, 0xaa3731, 0x448c27, 0xcb9000, 0x2f6f9f, 0x7a3e9d, 0x3d8b8b, 0x777777,
                0x666666, 0xf05050, 0x61c554, 0xfacb43, 0x5d9bcf, 0xad7fa8, 0x6fc2c2, 0xffffff,
            ],
        )
    }

    pub fn solarized_light() -> Self {
        Self::from_palette(
            0xfdf6e3,
            0xf5efdc,
            0xfdf6e3,
            0xeee8d5,
            0xe1d8c1,
            0x657b83,
            0x93a1a1,
            0x268bd2,
            [
                0x073642, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5,
                0x002b36, 0xcb4b16, 0x586e75, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3,
            ],
        )
    }

    pub fn vscode_dark_plus() -> Self {
        Self::from_palette(
            0x1e1e1e,
            0x252526,
            0x1e1e1e,
            0x333333,
            0x3c3c3c,
            0xd4d4d4,
            0x969696,
            0x007acc,
            [
                0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5,
                0x666666, 0xcd3131, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xffffff,
            ],
        )
    }

    pub fn vscode_abyss() -> Self {
        Self::from_palette(
            0x000c18,
            0x001521,
            0x000c18,
            0x032b3d,
            0x06445a,
            0x6688a3,
            0x456579,
            0x75beff,
            [
                0x000000, 0xff6c8b, 0x9fe463, 0xffd75f, 0x75beff, 0x9f7fff, 0x75e6da, 0xd5e5f5,
                0x405570, 0xff8fa3, 0xb5ef80, 0xffe58a, 0x9cc9ff, 0xc0a4ff, 0x9af2e9, 0xffffff,
            ],
        )
    }

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
                rgb(0x000000),
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

#[cfg(test)]
mod tests {
    use super::Theme;

    #[test]
    fn catalog_contains_requested_theme_families() {
        let themes = Theme::builtins();
        assert!(themes.iter().any(|(name, _)| *name == "light"));
        assert!(themes.iter().any(|(name, _)| *name == "midnight"));
        assert!(themes.iter().any(|(name, _)| *name == "paper"));
        assert!(themes.iter().any(|(name, _)| *name == "vscode_abyss"));
        assert_eq!(Theme::display_name("vscode_dark_plus"), "VS Code Dark Plus");
        assert!(!Theme::is_dark("vscode_light_plus"));
        assert!(Theme::is_dark("zed_dark"));

        let names: Vec<_> = themes
            .iter()
            .map(|(name, _)| Theme::display_name(name))
            .collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }
}
