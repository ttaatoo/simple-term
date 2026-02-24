//! Zed Terminal - A standalone terminal application

mod terminal_view;

use gpui::{point, px, size, AppContext, Application, Bounds, WindowBounds, WindowOptions};
use simple_term::TerminalSettings;
use terminal_view::TerminalView;

fn main() {
    env_logger::init();
    Application::new().run(|cx| {
        let settings = TerminalSettings::load(&TerminalSettings::config_path());
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.), px(0.)),
                size: size(
                    px(settings.default_width as f32),
                    px(settings.default_height as f32),
                ),
            })),
            ..Default::default()
        };

        cx.open_window(options, move |window, cx| {
            let settings = settings.clone();
            cx.new(move |cx| TerminalView::new(window, cx, settings))
        })
        .expect("Failed to open window");
    });
}
