mod app_icon;
mod editor;
mod native_menu;
mod theme;

use editor::Editor;
use gpui_kit::*;

#[cfg(target_os = "macos")]
const TITLEBAR_HEIGHT: f32 = 44.;

fn window_chrome_options() -> WindowOptions {
    #[cfg(target_os = "macos")]
    {
        // Let Kit own dragging/double-clicks; AppKit still draws the traffic lights.
        let mut options = component::TitleBar::window_options();
        options.titlebar = Some(TitlebarOptions {
            title: Some("Popcorn - Untitled.pop".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(12.), px((TITLEBAR_HEIGHT - 14.) / 2.))),
        });
        options
    }
    #[cfg(not(target_os = "macos"))]
    {
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Popcorn - Untitled.pop".into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            ..Default::default()
        }
    }
}

fn main() {
    application().with_assets(assets::AllAssets).run(|cx| {
        app_icon::init();
        init(cx);
        theme::init(cx);
        native_menu::init(cx);

        let window_size = size(px(1440.), px(900.));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(window_size, cx)),
            window_min_size: Some(size(px(1120.), px(680.))),
            ..window_chrome_options()
        };

        open_window(options, cx, |_, cx| cx.new(|_| Editor::default()))
            .expect("failed to open Popcorn window");
    });
}
