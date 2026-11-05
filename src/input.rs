use crate::game::{Input, TICK_HZ};
use crate::world::Dir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Dir(Dir),
    Sword,
    Start,
}

/// Held-key state shared by real key events and the demo bot.
#[derive(Clone, Debug, Default)]
pub struct Keys {
    dirs: Vec<Dir>,
    sword_down: bool,
    sword_edge: bool,
    start_edge: bool,
}

impl Keys {
    pub fn press(&mut self, key: Key) {
        match key {
            Key::Dir(d) => {
                self.dirs.retain(|x| *x != d);
                self.dirs.push(d);
            }
            Key::Sword => {
                if !self.sword_down {
                    self.sword_edge = true;
                }
                self.sword_down = true;
            }
            Key::Start => self.start_edge = true,
        }
    }

    pub fn release(&mut self, key: Key) {
        match key {
            Key::Dir(d) => self.dirs.retain(|x| *x != d),
            Key::Sword => self.sword_down = false,
            Key::Start => {}
        }
    }

    pub fn clear(&mut self) {
        *self = Keys::default();
    }

    pub fn holds(&self, key: Key) -> bool {
        match key {
            Key::Dir(d) => self.dirs.contains(&d),
            Key::Sword => self.sword_down,
            Key::Start => false,
        }
    }

    /// Press and release keys so the held state matches what the bot wants this tick.
    pub fn follow(&mut self, want: Input) {
        for d in Dir::ALL {
            let k = Key::Dir(d);
            match (want.dir == Some(d), self.holds(k)) {
                (true, false) => self.press(k),
                (false, true) => self.release(k),
                _ => {}
            }
        }
        if want.sword {
            self.press(Key::Sword);
        } else {
            self.release(Key::Sword);
        }
        if want.start {
            self.press(Key::Start);
        }
    }

    /// The input for one tick. Sword and start edges fire once.
    pub fn take(&mut self) -> Input {
        Input {
            dir: self.dirs.last().copied(),
            sword: std::mem::take(&mut self.sword_edge),
            start: std::mem::take(&mut self.start_edge),
        }
    }
}

/// Map a GPUI key name to a game key.
pub fn key_from_name(name: &str) -> Option<Key> {
    Some(match name {
        "up" | "w" => Key::Dir(Dir::Up),
        "down" | "s" => Key::Dir(Dir::Down),
        "left" | "a" => Key::Dir(Dir::Left),
        "right" | "d" => Key::Dir(Dir::Right),
        "space" | "z" | "x" => Key::Sword,
        "enter" => Key::Start,
        _ => return None,
    })
}

pub const MAX_TICKS_PER_FRAME: u32 = 5;

/// Fixed-step accumulator. A stall never makes more than `MAX_TICKS_PER_FRAME` ticks catch up.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    acc: f64,
}

impl Clock {
    pub const STEP: f64 = 1.0 / TICK_HZ as f64;

    pub fn advance(&mut self, dt: f64) -> u32 {
        self.acc += dt.max(0.0);
        let due = (self.acc / Self::STEP).floor() as u32;
        let ticks = due.min(MAX_TICKS_PER_FRAME);
        self.acc -= ticks as f64 * Self::STEP;
        if due > MAX_TICKS_PER_FRAME {
            self.acc %= Self::STEP;
        }
        ticks
    }

    pub fn remainder(&self) -> f64 {
        self.acc
    }
}
