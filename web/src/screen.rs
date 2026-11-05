use std::collections::HashMap;

use little_quest::game::{Draw, HeartState, Hud};
use little_quest::sprites::{self, SpriteId, SpriteImage};
use little_quest::world::{PLAY_H, PLAY_W};

pub const WIDTH: i32 = PLAY_W;
pub const HUD_H: i32 = 56;
pub const HEIGHT: i32 = HUD_H + PLAY_H;

const MAP_CELL: i32 = 11;
const MAP_GAP: i32 = 2;
const MAP_ROWS: i32 = 2;
const MAP_ROOM: u32 = 0x4a5066;
const MAP_HERE: u32 = 0xf8d838;
const ITEM_BORDER: u32 = 0x4a5066;
const PAUSE_DIM: u32 = 64;

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

const FIELD: Rect = Rect {
    x: 0,
    y: HUD_H,
    w: WIDTH,
    h: PLAY_H,
};
const ALL: Rect = Rect {
    x: 0,
    y: 0,
    w: WIDTH,
    h: HEIGHT,
};

type SpriteKey = (SpriteId, bool, bool);

pub struct Screen {
    rgba: Vec<u8>,
    sprites: HashMap<SpriteKey, SpriteImage>,
}

impl Screen {
    pub fn new() -> Screen {
        Screen {
            rgba: vec![0; (WIDTH * HEIGHT * 4) as usize],
            sprites: HashMap::new(),
        }
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn draw(&mut self, hud: Option<(&Hud, &str)>, field: &[Draw]) {
        self.fill(ALL, 0x000000);
        if let Some((hud, room_name)) = hud {
            self.hud(hud, room_name);
        }
        for d in field {
            self.blit(d.sprite, d.flip_x, d.flash, d.x, d.y + HUD_H, FIELD);
        }
    }

    pub fn dim_field_with(&mut self, title: &str, hint: &str) {
        for row in FIELD.y..FIELD.y + FIELD.h {
            for x in 0..WIDTH {
                let o = ((row * WIDTH + x) * 4) as usize;
                for c in &mut self.rgba[o..o + 3] {
                    *c = (*c as u32 * PAUSE_DIM / 256) as u8;
                }
            }
        }
        let middle = FIELD.y + (PLAY_H - 8) / 2;
        for (line, y) in [(title, middle - 8), (hint, middle + 8)] {
            self.text(line, (PLAY_W - line.len() as i32 * 8) / 2, y, FIELD);
        }
    }

    fn fill(&mut self, r: Rect, color: u32) {
        let rgb = [(color >> 16) as u8, (color >> 8) as u8, color as u8, 255];
        for y in r.y.max(0)..(r.y + r.h).min(HEIGHT) {
            for x in r.x.max(0)..(r.x + r.w).min(WIDTH) {
                let o = ((y * WIDTH + x) * 4) as usize;
                self.rgba[o..o + 4].copy_from_slice(&rgb);
            }
        }
    }

    fn blit(&mut self, id: SpriteId, flip_x: bool, flash: bool, x: i32, y: i32, clip: Rect) {
        let s = self
            .sprites
            .entry((id, flip_x, flash))
            .or_insert_with(|| sprites::pixels(id, flip_x, flash));
        let (x0, x1) = (x.max(clip.x), (x + s.w as i32).min(clip.x + clip.w));
        let (y0, y1) = (y.max(clip.y), (y + s.h as i32).min(clip.y + clip.h));
        for dy in y0..y1 {
            for dx in x0..x1 {
                let from = (((dy - y) * s.w as i32 + (dx - x)) * 4) as usize;
                if s.rgba[from + 3] != 0 {
                    let to = ((dy * WIDTH + dx) * 4) as usize;
                    self.rgba[to..to + 4].copy_from_slice(&s.rgba[from..from + 4]);
                }
            }
        }
    }

    fn text(&mut self, s: &str, x: i32, y: i32, clip: Rect) {
        for (i, g) in sprites::text_glyphs(s).into_iter().enumerate() {
            if g != SpriteId::Glyph(' ') {
                self.blit(g, false, false, x + i as i32 * 8, y, clip);
            }
        }
    }

    fn hud(&mut self, hud: &Hud, room_name: &str) {
        let (cols, rows) = hud.map_size;
        let (map_x, map_y) = (12, 10);
        for cy in 0..rows {
            for cx in 0..cols {
                let color = if (cx, cy) == hud.map_cell {
                    MAP_HERE
                } else if hud.map_rooms.contains(&(cx, cy)) {
                    MAP_ROOM
                } else {
                    continue;
                };
                self.fill(
                    Rect {
                        x: map_x + cx as i32 * (MAP_CELL + MAP_GAP),
                        y: map_y + cy as i32 * (MAP_CELL + MAP_GAP),
                        w: MAP_CELL,
                        h: MAP_CELL,
                    },
                    color,
                );
            }
        }
        let name_y = map_y + MAP_ROWS * MAP_CELL + (MAP_ROWS - 1) * MAP_GAP + 8;
        self.text(room_name, map_x, name_y, ALL);

        let item = Rect {
            x: 70,
            y: 10,
            w: 22,
            h: 22,
        };
        self.fill(item, ITEM_BORDER);
        self.fill(
            Rect {
                x: item.x + 1,
                y: item.y + 1,
                w: item.w - 2,
                h: item.h - 2,
            },
            0x000000,
        );
        if hud.sword {
            self.blit(SpriteId::BladeUp, false, false, item.x + 3, item.y + 3, ALL);
        }

        for (row, (icon, n)) in [(SpriteId::Gem, hud.gems), (SpriteId::Key, hud.keys)]
            .into_iter()
            .enumerate()
        {
            let y = 6 + row as i32 * 22;
            self.blit(icon, false, false, 104, y, ALL);
            self.text(&format!("X{n}"), 122, y + 4, ALL);
        }

        self.text("-LIFE-", 176, 10, ALL);
        for (i, h) in hud.hearts.iter().enumerate() {
            let heart = match h {
                HeartState::Full => SpriteId::HeartFull,
                HeartState::Half => SpriteId::HeartHalf,
                HeartState::Empty => SpriteId::HeartEmpty,
            };
            self.blit(heart, false, false, 176 + i as i32 * 16, 24, ALL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use little_quest::game::{Game, Prompts};
    use little_quest::world::RoomId;

    fn px(s: &Screen, x: i32, y: i32) -> [u8; 3] {
        let o = ((y * WIDTH + x) * 4) as usize;
        [s.rgba[o], s.rgba[o + 1], s.rgba[o + 2]]
    }

    fn rgb(c: u32) -> [u8; 3] {
        [(c >> 16) as u8, (c >> 8) as u8, c as u8]
    }

    fn drawn(g: &Game) -> Screen {
        let mut s = Screen::new();
        let hud = g.hud();
        s.draw(
            Some((&hud, g.room().def().name)),
            &g.scene(&Prompts::KEYBOARD),
        );
        s
    }

    #[test]
    fn a_sprite_sliding_in_from_above_never_paints_the_hud() {
        let mut s = Screen::new();
        let slide = Draw {
            sprite: SpriteId::Tree,
            x: 0,
            y: -8,
            flip_x: false,
            flash: false,
        };
        s.draw(None, &[slide]);
        for y in 0..HUD_H {
            for x in 0..WIDTH {
                assert_eq!(px(&s, x, y), [0, 0, 0], "({x}, {y})");
            }
        }
        assert_ne!(px(&s, 0, HUD_H), [0, 0, 0], "the visible half is drawn");
    }

    #[test]
    fn the_hud_sits_where_the_native_layout_puts_it() {
        let g = Game::new_at(1, RoomId::Approach);
        let s = drawn(&g);
        let hud = g.hud();
        let (here_x, here_y) = hud.map_cell;
        assert_eq!(
            px(&s, 12 + here_x as i32 * 13 + 5, 10 + here_y as i32 * 13 + 5),
            rgb(MAP_HERE)
        );
        assert_eq!(px(&s, 70, 20), rgb(ITEM_BORDER), "item box edge");
        assert_ne!(px(&s, 176 + 8, 24 + 8), [0, 0, 0], "first heart");
        assert_eq!(px(&s, 255, 55), [0, 0, 0], "HUD corner stays black");
    }

    #[test]
    fn the_title_screen_has_no_hud() {
        let g = Game::new(1);
        let mut s = Screen::new();
        s.draw(None, &g.scene(&Prompts::TOUCH));
        for y in 0..HUD_H {
            for x in 0..WIDTH {
                assert_eq!(px(&s, x, y), [0, 0, 0]);
            }
        }
    }

    #[test]
    fn pausing_dims_the_playfield_and_leaves_the_hud() {
        let g = Game::new_at(1, RoomId::Approach);
        let before = drawn(&g);
        let mut after = drawn(&g);
        after.dim_field_with("PAUSED", "TAP TO RESUME");
        for x in 0..WIDTH {
            assert_eq!(px(&before, x, 20), px(&after, x, 20), "HUD row {x}");
        }
        let lum = |p: [u8; 3]| p.iter().map(|c| *c as u32).sum::<u32>();
        assert!(lum(px(&after, 1, HUD_H + 1)) < lum(px(&before, 1, HUD_H + 1)));
        assert!(
            (0..WIDTH).any(|x| px(&after, x, HUD_H + 76 + 3) == rgb(0xf8f8f8)),
            "the PAUSED text is drawn"
        );
        assert!(
            (0..WIDTH).any(|x| px(&after, x, HUD_H + 92 + 3) == rgb(0xf8f8f8)),
            "the resume hint is drawn"
        );
    }
}
