#![recursion_limit = "256"]
use gpui::*;

mod state;
mod theme;
mod components;
mod workspace;
mod pty;
mod ui;
mod update;

use crate::workspace::Workspace;

struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match path {
            "icons/git_branch.svg" => Ok(Some(std::borrow::Cow::Borrowed(include_bytes!("../assets/icons/git_branch.svg")))),
            _ => Ok(None),
        }
    }
    
    fn list(&self, _path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(vec![])
    }
}

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("vterm".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(12.0), px(9.0))),
            }),
            ..Default::default()
        };
        cx.open_window(options, |_window, cx| cx.new(|cx| Workspace::new(cx)))
            .expect("failed to open window");

        spawn_update_checker();
    });
}

fn spawn_update_checker() {
    std::thread::spawn(|| {
        if let Some(info) = update::check_for_update() {
            eprintln!(
                "vterm: update available -> v{} ({})",
                info.version, info.download_url
            );
            update::notify(&info);
        }
    });
}
