#![recursion_limit = "256"]
use gpui::*;

use workspace::Workspace;

struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match path {
            "icons/git_branch.svg" => Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../../../assets/icons/git_branch.svg"
            )))),
            "icons/search.svg" => Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../../../assets/icons/search.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        // GPUI queries this for directory listings (e.g. "icons").
        // Return the bundled icons so svg().path() resolves in all contexts.
        const ICONS: &[&str] = &["icons/git_branch.svg", "icons/search.svg"];
        let prefix = path.trim_matches('/').trim_end_matches('/');
        let listed: Vec<gpui::SharedString> = if prefix.is_empty() {
            vec!["icons".into()]
        } else if prefix == "icons" {
            ICONS.iter().map(|s| (*s).into()).collect()
        } else {
            vec![]
        };
        Ok(listed)
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.on_reopen(|cx: &mut App| {
        if cx.windows().is_empty() {
            open_window(cx);
        }
    });

    app.run(|cx: &mut App| {
        open_window(cx);
    });
}

fn open_window(cx: &mut App) {
    let app_name = std::env::var("VTERM_APP_NAME").unwrap_or_else(|_| "vterm".to_string());
    let app_version = std::env::var("VTERM_DEV_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    // On macOS every TitlebarOptions field above is specified, so the struct
    // update is needless there — but dropping it breaks other platforms
    // where `traffic_light_position` is compiled out.
    #[allow(clippy::needless_update)]
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(format!("{app_name} v{app_version}").into()),
            appears_transparent: cfg!(target_os = "macos"),
            #[cfg(target_os = "macos")]
            traffic_light_position: Some(point(px(12.0), px(9.0))),
            ..Default::default()
        }),
        ..Default::default()
    };
    cx.open_window(options, |window, cx| {
        cx.new(|cx| Workspace::new(window, cx))
    })
    .expect("failed to open window");
}
