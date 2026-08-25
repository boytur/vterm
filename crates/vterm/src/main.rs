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
            "icons/vterm_logo.svg" => Ok(Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../../../assets/icons/vterm_logo.svg"
            )))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(vec![])
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
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some(format!("{app_name} v{app_version}").into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(12.0), px(9.0))),
        }),
        ..Default::default()
    };
    cx.open_window(options, |window, cx| cx.new(|cx| Workspace::new(window, cx)))
        .expect("failed to open window");
}
