use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    Bounds, ContentMask, Context, Corners, Div, FocusHandle, Img, IntoElement, KeyDownEvent,
    KeyUpEvent, ObjectFit, ParentElement, Pixels, Render, RenderImage, Styled, StyledImage,
    Subscription, Window, canvas, div, img, point, prelude::*, px, rgb, rgba, size,
};
use image::{Frame, ImageBuffer, Rgba};
use little_quest::autoplay::Bot;
use little_quest::game::{Game, HeartState, Hud, Mode};
use little_quest::input::{Clock, Keys, key_from_name};
use little_quest::sprites::{self, SpriteId};
use little_quest::world::{PLAY_H, PLAY_W, RoomId};

pub const SCALE: f32 = 3.0;
pub const HUD_H: i32 = 56;
pub const TITLEBAR_H: f32 = 28.0;

#[derive(Clone, Copy)]
pub struct Options {
    pub demo: bool,
    pub log: bool,
    pub exit_after_ticks: Option<u64>,
    pub seed: u64,
}

type SpriteKey = (SpriteId, bool, bool);

/// Sprites pre-scaled by nearest neighbour to whole device pixels. GPUI samples images with a
/// linear filter, so any other size or a fractional position would blur them.
#[derive(Default)]
struct SpriteCache {
    pixels_per_pixel: u32,
    images: HashMap<SpriteKey, Arc<RenderImage>>,
}

impl SpriteCache {
    fn retarget(&mut self, window: &mut Window) {
        let k = (SCALE * window.scale_factor()).round().max(1.0) as u32;
        if k != self.pixels_per_pixel {
            for (_, image) in self.images.drain() {
                window.drop_image(image).expect("sprite is in the atlas");
            }
            self.pixels_per_pixel = k;
        }
    }

    fn get(&mut self, id: SpriteId, flip_x: bool, flash: bool) -> Arc<RenderImage> {
        let k = self.pixels_per_pixel;
        self.images
            .entry((id, flip_x, flash))
            .or_insert_with(|| {
                let src = sprites::pixels(id, flip_x, flash);
                let (w, h) = (src.w * k, src.h * k);
                let mut bgra = vec![0u8; (w * h * 4) as usize];
                for y in 0..h {
                    for x in 0..w {
                        let s = (((y / k) * src.w + x / k) * 4) as usize;
                        let o = ((y * w + x) * 4) as usize;
                        bgra[o..o + 4].copy_from_slice(&[
                            src.rgba[s + 2],
                            src.rgba[s + 1],
                            src.rgba[s],
                            src.rgba[s + 3],
                        ]);
                    }
                }
                let buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::from_raw(w, h, bgra).expect("buffer size");
                Arc::new(RenderImage::new(vec![Frame::new(buf)]))
            })
            .clone()
    }

    /// A sprite as a flex child, sized to its logical pixels.
    fn sprite(&mut self, id: SpriteId) -> Img {
        let (w, h) = id.size();
        img(self.get(id, false, false))
            .object_fit(ObjectFit::Fill)
            .flex_none()
            .w(logical(w as i32))
            .h(logical(h as i32))
    }

    /// A row of glyph images. A space is an empty box of one glyph width.
    fn text(&mut self, s: &str) -> Div {
        sprites::text_glyphs(s)
            .into_iter()
            .fold(div().flex().flex_row().flex_none(), |row, g| {
                if g == SpriteId::Glyph(' ') {
                    row.child(div().flex_none().w(logical(8)))
                } else {
                    row.child(self.sprite(g))
                }
            })
    }
}

#[derive(Default)]
struct FrameStats {
    frames: u32,
    sum: f64,
    max: f64,
}

impl FrameStats {
    fn record(&mut self, dt: f64) {
        self.frames += 1;
        self.sum += dt;
        self.max = self.max.max(dt);
    }

    fn report(&mut self) {
        if self.frames == 0 {
            return;
        }
        eprintln!(
            "FRAMES avg_ms={:.2} max_ms={:.2} fps={:.1}",
            self.sum * 1000.0 / self.frames as f64,
            self.max * 1000.0,
            self.frames as f64 / self.sum
        );
        *self = FrameStats::default();
    }
}

pub struct LittleQuest {
    game: Game,
    clock: Clock,
    keys: Keys,
    bot: Option<Bot>,
    focus: FocusHandle,
    sprites: SpriteCache,
    opts: Options,
    last_frame: Instant,
    ticks_run: u64,
    last_mark: Option<(&'static str, RoomId)>,
    stats: FrameStats,
    _activation: Subscription,
}

impl LittleQuest {
    pub fn new(opts: Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        let activation = cx.observe_window_activation(window, |this, _window, cx| {
            this.keys.clear();
            cx.notify();
        });
        cx.on_next_frame(window, Self::on_frame);
        LittleQuest {
            game: Game::new(opts.seed),
            clock: Clock::default(),
            keys: Keys::default(),
            bot: opts.demo.then(Bot::new),
            focus,
            sprites: SpriteCache::default(),
            opts,
            last_frame: Instant::now(),
            ticks_run: 0,
            last_mark: None,
            stats: FrameStats::default(),
            _activation: activation,
        }
    }

    /// Runs once per display frame, reschedules itself, and asks for a redraw only when the game moved.
    fn on_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;
        if self.opts.demo || window.is_window_active() {
            self.stats.record(dt);
            let ticks = self.clock.advance(dt);
            for _ in 0..ticks {
                self.step();
                if self
                    .opts
                    .exit_after_ticks
                    .is_some_and(|n| self.ticks_run >= n)
                {
                    cx.quit();
                    break;
                }
            }
            if ticks > 0 {
                cx.notify();
            }
        }
        cx.on_next_frame(window, Self::on_frame);
    }

    fn step(&mut self) {
        if let Some(bot) = &mut self.bot {
            let want = bot.decide(&self.game);
            self.keys.follow(want);
        }
        let input = self.keys.take();
        self.game.tick(input);
        self.ticks_run += 1;
        if self.opts.log {
            self.log_marks();
            if self.ticks_run.is_multiple_of(120) {
                self.stats.report();
            }
        }
    }

    fn log_marks(&mut self) {
        let now = (self.game.mode().name(), self.game.room());
        if self.last_mark == Some(now) {
            return;
        }
        self.last_mark = Some(now);
        let hud = self.game.hud();
        let half = self.game.hero().hp;
        let hearts = if half.is_multiple_of(2) {
            format!("{}", half / 2)
        } else {
            format!("{}.5", half / 2)
        };
        eprintln!(
            "MARK tick={} mode={} room={} hearts={} gems={} keys={}",
            self.ticks_run, now.0, now.1, hearts, hud.gems, hud.keys
        );
    }

    fn key_down(&mut self, e: &KeyDownEvent, _: &mut Window, _: &mut Context<Self>) {
        let m = e.keystroke.modifiers;
        if e.is_held || m.platform || m.control || self.opts.demo {
            return;
        }
        if let Some(k) = key_from_name(&e.keystroke.key) {
            self.keys.press(k);
        }
    }

    fn key_up(&mut self, e: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if self.opts.demo {
            return;
        }
        if let Some(k) = key_from_name(&e.keystroke.key) {
            self.keys.release(k);
        }
    }
}

fn logical(v: i32) -> Pixels {
    px(v as f32 * SCALE)
}

const MAP_CELL: i32 = 11;
const MAP_GAP: i32 = 2;
const MAP_ROWS_MAX: i32 = 2;

fn minimap(hud: &Hud) -> Div {
    let (cols, rows) = hud.map_size;
    let mut grid = div()
        .flex()
        .flex_col()
        .flex_none()
        .gap(logical(MAP_GAP))
        .h(logical(
            MAP_ROWS_MAX * MAP_CELL + (MAP_ROWS_MAX - 1) * MAP_GAP,
        ));
    for cy in 0..rows {
        let mut row = div().flex().flex_row().gap(logical(MAP_GAP));
        for cx in 0..cols {
            let cell = div().w(logical(MAP_CELL)).h(logical(MAP_CELL));
            row = row.child(if !hud.map_rooms.contains(&(cx, cy)) {
                cell
            } else if (cx, cy) == hud.map_cell {
                cell.bg(rgb(0xf8d838))
            } else {
                cell.bg(rgb(0x4a5066))
            });
        }
        grid = grid.child(row);
    }
    grid
}

fn hud(cache: &mut SpriteCache, hud: &Hud, room_name: &str) -> Div {
    let item_box = div()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .w(logical(22))
        .h(logical(22))
        .mt(logical(4))
        .border_2()
        .border_color(rgb(0x4a5066))
        .children(hud.sword.then(|| cache.sprite(SpriteId::BladeUp)));
    let counter = |cache: &mut SpriteCache, icon: SpriteId, n: u32| {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(logical(2))
            .h(logical(16))
            .child(cache.sprite(icon))
            .child(cache.text(&format!("X{n}")))
    };
    let counters = div()
        .flex()
        .flex_col()
        .flex_none()
        .gap(logical(6))
        .w(logical(84))
        .pl(logical(12))
        .child(counter(cache, SpriteId::Gem, hud.gems))
        .child(counter(cache, SpriteId::Key, hud.keys));
    let hearts = hud.hearts.iter().fold(div().flex().flex_row(), |row, h| {
        row.child(cache.sprite(match h {
            HeartState::Full => SpriteId::HeartFull,
            HeartState::Half => SpriteId::HeartHalf,
            HeartState::Empty => SpriteId::HeartEmpty,
        }))
    });
    let life = div()
        .flex()
        .flex_col()
        .flex_none()
        .gap(logical(6))
        .mt(logical(4))
        .child(cache.text("-LIFE-"))
        .child(hearts);
    div()
        .flex()
        .flex_row()
        .items_start()
        .flex_none()
        .w(logical(PLAY_W))
        .h(logical(HUD_H))
        .pl(logical(12))
        .pt(logical(6))
        .bg(rgb(0x000000))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .gap(logical(8))
                .mt(logical(4))
                .w(logical(58))
                .child(minimap(hud))
                .child(cache.text(&room_name.to_uppercase())),
        )
        .child(item_box)
        .child(counters)
        .child(life)
}

impl Render for LittleQuest {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sprites.retarget(window);
        let paused = !self.opts.demo && !window.is_window_active();

        let draws: Vec<_> = self
            .game
            .scene()
            .into_iter()
            .map(|d| {
                let (w, h) = d.sprite.size();
                (
                    self.sprites.get(d.sprite, d.flip_x, d.flash),
                    d.x,
                    d.y,
                    w as i32,
                    h as i32,
                )
            })
            .collect();
        let hud_view = if matches!(self.game.mode(), Mode::Title) {
            div()
                .flex_none()
                .w(logical(PLAY_W))
                .h(logical(HUD_H))
                .bg(rgb(0x000000))
        } else {
            hud(
                &mut self.sprites,
                &self.game.hud(),
                self.game.room().def().name,
            )
        };
        let field = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    for (image, x, y, w, h) in &draws {
                        let at = Bounds::new(
                            point(bounds.origin.x + logical(*x), bounds.origin.y + logical(*y)),
                            size(logical(*w), logical(*h)),
                        );
                        window
                            .paint_image(at, Corners::default(), image.clone(), 0, false)
                            .expect("sprite fits in the atlas");
                    }
                });
            },
        )
        .flex_none()
        .w(logical(PLAY_W))
        .h(logical(PLAY_H));

        div()
            .id("little-quest")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .relative()
            .size_full()
            .bg(rgb(0x000000))
            .flex()
            .flex_col()
            .child(div().w_full().h(px(TITLEBAR_H)).flex_none())
            .child(hud_view)
            .child(field)
            .children(paused.then(|| {
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(TITLEBAR_H + HUD_H as f32 * SCALE))
                    .w(logical(PLAY_W))
                    .h(logical(PLAY_H))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x000000c0))
                    .child(self.sprites.text("PAUSED"))
            }))
    }
}
