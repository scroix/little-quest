mod view;

use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBounds, WindowOptions, actions, point, px, size,
};
use view::{LittleQuest, Options, SCALE, TITLEBAR_H};

actions!(little_quest, [Quit]);

fn parse_args() -> Options {
    let mut opts = Options {
        demo: false,
        log: false,
        exit_after_ticks: None,
        seed: 0,
    };
    let mut seed = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--demo" => opts.demo = true,
            "--log" => opts.log = true,
            "--exit-after-ticks" => {
                opts.exit_after_ticks = args.next().and_then(|v| v.parse().ok())
            }
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()),
            other => {
                eprintln!(
                    "usage: little-quest [--demo] [--log] [--exit-after-ticks N] [--seed N]\nunknown argument {other}"
                );
                std::process::exit(2);
            }
        }
    }
    opts.log |= opts.demo;
    opts.seed = seed.unwrap_or_else(|| {
        if opts.demo {
            1
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64)
        }
    });
    opts
}

fn main() {
    let opts = parse_args();
    Application::new().run(move |cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Little Quest".into(),
            items: vec![MenuItem::action("Quit Little Quest", Quit)],
        }]);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let (w, h) = (256.0 * SCALE, TITLEBAR_H + 232.0 * SCALE);
        let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Little Quest".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(10.0), px(10.0))),
                }),
                focus: !opts.demo,
                is_resizable: false,
                ..Default::default()
            },
            |window, cx| cx.new(|cx| LittleQuest::new(opts, window, cx)),
        )
        .expect("open window");
        if !opts.demo {
            cx.activate(true);
        }
    });
}
