mod screen;
mod timing;

use little_quest::autoplay::Bot;
use little_quest::game::{Game, Mode, Prompts};
use little_quest::input::{Clock, Key, Keys};
use little_quest::world::Dir;
use wasm_bindgen::prelude::*;

use screen::Screen;
use timing::Pacer;

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Keyboard,
    Touch,
}

impl InputKind {
    fn prompts(self) -> &'static Prompts {
        match self {
            InputKind::Keyboard => &Prompts::KEYBOARD,
            InputKind::Touch => &Prompts::TOUCH,
        }
    }

    fn resume_hint(self) -> &'static str {
        match self {
            InputKind::Keyboard => "PRESS ANY KEY",
            InputKind::Touch => "TAP TO RESUME",
        }
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum WebKey {
    Up,
    Down,
    Left,
    Right,
    Sword,
    Start,
}

impl From<WebKey> for Key {
    fn from(key: WebKey) -> Key {
        match key {
            WebKey::Up => Key::Dir(Dir::Up),
            WebKey::Down => Key::Dir(Dir::Down),
            WebKey::Left => Key::Dir(Dir::Left),
            WebKey::Right => Key::Dir(Dir::Right),
            WebKey::Sword => Key::Sword,
            WebKey::Start => Key::Start,
        }
    }
}

#[wasm_bindgen]
pub struct WebGame {
    game: Game,
    keys: Keys,
    pacer: Pacer,
    clock: Clock,
    bot: Option<Bot>,
    input: InputKind,
    screen: Screen,
    ticks_run: u32,
    mode_start_tick: u32,
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, demo: bool, touch: bool) -> WebGame {
        let mut web = WebGame {
            game: Game::new(seed.into()),
            keys: Keys::default(),
            pacer: Pacer::default(),
            clock: Clock::default(),
            bot: demo.then(Bot::new),
            input: if touch {
                InputKind::Touch
            } else {
                InputKind::Keyboard
            },
            screen: Screen::new(),
            ticks_run: 0,
            mode_start_tick: 0,
        };
        web.render();
        web
    }

    pub fn press(&mut self, key: WebKey) {
        if self.bot.is_none() {
            self.keys.press(key.into());
        }
    }

    pub fn release(&mut self, key: WebKey) {
        if self.bot.is_none() {
            self.keys.release(key.into());
        }
    }

    pub fn clear_keys(&mut self) {
        self.keys.clear();
    }

    pub fn set_touch(&mut self, touch: bool) {
        let input = if touch {
            InputKind::Touch
        } else {
            InputKind::Keyboard
        };
        if input != self.input {
            self.input = input;
            self.render();
        }
    }

    /// Advance by one browser frame of `dt_ms`. Returns true when a tick ran and the pixels changed.
    pub fn frame(&mut self, dt_ms: f64) -> bool {
        let ticks = self.clock.advance(self.pacer.pace(dt_ms / 1000.0));
        for _ in 0..ticks {
            self.step();
        }
        if ticks > 0 {
            self.render();
        }
        ticks > 0
    }

    pub fn redraw(&mut self) {
        self.render();
    }

    pub fn draw_paused(&mut self) {
        self.render();
        self.screen
            .dim_field_with("PAUSED", self.input.resume_hint());
    }

    /// False on the title, game over, and victory screens, where leaving loses nothing.
    pub fn in_play(&self) -> bool {
        !matches!(
            self.game.mode(),
            Mode::Title | Mode::GameOver { .. } | Mode::Victory { .. }
        )
    }

    pub fn pixels_ptr(&self) -> *const u8 {
        self.screen.rgba().as_ptr()
    }

    pub fn pixels_len(&self) -> usize {
        self.screen.rgba().len()
    }

    pub fn width(&self) -> u32 {
        screen::WIDTH as u32
    }

    pub fn height(&self) -> u32 {
        screen::HEIGHT as u32
    }

    pub fn ticks(&self) -> u32 {
        self.ticks_run
    }

    pub fn mode_start_tick(&self) -> u32 {
        self.mode_start_tick
    }

    pub fn mode(&self) -> String {
        self.game.mode().name().to_string()
    }

    pub fn room(&self) -> String {
        self.game.room().def().name.to_string()
    }

    pub fn hero_x(&self) -> f32 {
        self.game.hero().x
    }

    pub fn hero_y(&self) -> f32 {
        self.game.hero().y
    }

    pub fn half_hearts(&self) -> u32 {
        self.game.hero().hp
    }

    pub fn gems(&self) -> u32 {
        self.game.gems()
    }

    pub fn keys_held(&self) -> u32 {
        self.game.keys()
    }

    pub fn has_sword(&self) -> bool {
        self.game.hud().sword
    }
}

impl WebGame {
    fn step(&mut self) {
        if let Some(bot) = &mut self.bot {
            let want = bot.decide(&self.game);
            self.keys.follow(want);
        }
        let mode = self.game.mode().name();
        self.game.tick(self.keys.take());
        self.ticks_run += 1;
        if self.game.mode().name() != mode {
            self.mode_start_tick = self.ticks_run;
        }
    }

    fn render(&mut self) {
        let hud = self.game.hud();
        let title = matches!(self.game.mode(), Mode::Title);
        let hud = (!title).then_some((&hud, self.game.room().def().name));
        self.screen
            .draw(hud, &self.game.scene(self.input.prompts()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_one_bot_wins_on_tick_2574_through_the_wasm_api() {
        let mut web = WebGame::new(1, true, false);
        let mut guard = 0;
        while web.mode() != "victory" {
            web.frame(1000.0 / 60.0);
            guard += 1;
            assert!(guard < 10_000, "no victory by tick {}", web.ticks());
        }
        assert_eq!(web.mode_start_tick(), 2574);
    }

    #[test]
    fn a_frame_between_ticks_does_not_redraw() {
        let mut web = WebGame::new(1, false, false);
        assert!(!web.frame(1000.0 / 120.0), "half a tick");
        assert!(web.frame(1000.0 / 120.0), "the second half completes it");
        assert_eq!(web.ticks(), 1);
    }

    #[test]
    fn a_demo_ignores_the_keyboard() {
        let mut web = WebGame::new(1, true, false);
        while web.mode() != "victory" {
            web.press(WebKey::Start);
            web.press(WebKey::Sword);
            web.frame(1000.0 / 60.0);
            assert!(web.ticks() < 10_000);
        }
        assert_eq!(
            web.ticks(),
            2574,
            "stray keys leave the bot's run unchanged"
        );
    }

    #[test]
    fn only_a_run_in_progress_counts_as_in_play() {
        let mut web = WebGame::new(1, false, false);
        assert!(!web.in_play(), "title");
        web.press(WebKey::Start);
        web.frame(1000.0 / 60.0);
        assert!(web.in_play(), "playing");
    }

    #[test]
    fn enter_starts_the_game_on_the_next_tick() {
        let mut web = WebGame::new(1, false, false);
        web.press(WebKey::Start);
        web.release(WebKey::Start);
        web.frame(1000.0 / 60.0);
        assert_ne!(web.mode(), "title");
    }

    #[test]
    fn switching_input_changes_the_words_on_the_title_screen() {
        let mut web = WebGame::new(1, false, false);
        let keyboard =
            unsafe { std::slice::from_raw_parts(web.pixels_ptr(), web.pixels_len()) }.to_vec();
        web.set_touch(true);
        let touch = unsafe { std::slice::from_raw_parts(web.pixels_ptr(), web.pixels_len()) };
        assert_ne!(keyboard, touch);
    }
}
