use super::{
    DROP_FLICKER, DROP_LIFE, Game, HeroAction, MENTOR_TEXT, Mode, PickupKind, SCROLL_SPEED,
    WIPE_HALF,
};
use crate::actors::{Enemy, EnemyKind, ProjKind};
use crate::sprites::{SpriteId, text_glyphs};
use crate::world::{Dir, PLAY_H, PLAY_W, Progress, ROOM_H, ROOM_W, RoomId, TILE, Tile, tile_at};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draw {
    pub sprite: SpriteId,
    pub x: i32,
    pub y: i32,
    pub flip_x: bool,
    pub flash: bool,
}

fn draw(sprite: SpriteId, x: i32, y: i32) -> Draw {
    Draw {
        sprite,
        x,
        y,
        flip_x: false,
        flash: false,
    }
}

pub fn text(out: &mut Vec<Draw>, s: &str, x: i32, y: i32) {
    for (i, g) in text_glyphs(s).into_iter().enumerate() {
        if g != SpriteId::Glyph(' ') {
            out.push(draw(g, x + i as i32 * 8, y));
        }
    }
}

fn centered(out: &mut Vec<Draw>, s: &str, y: i32) {
    text(out, s, (PLAY_W - s.len() as i32 * 8) / 2, y);
}

fn black_field(out: &mut Vec<Draw>) {
    for ty in 0..ROOM_H {
        for tx in 0..ROOM_W {
            out.push(draw(SpriteId::Black, tx * TILE, ty * TILE));
        }
    }
}

pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    for word in text.split(' ') {
        match lines.last_mut() {
            Some(l) if l.len() + 1 + word.len() <= cols => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

pub fn tile_draws(room: RoomId, progress: &Progress, frame: u64, ox: i32, oy: i32) -> Vec<Draw> {
    let water = SpriteId::Water(((frame / 30) % 2) as u8);
    let mut out = Vec::with_capacity(200);
    for ty in 0..ROOM_H {
        for tx in 0..ROOM_W {
            let (x, y) = (ox + tx * TILE, oy + ty * TILE);
            let tile = tile_at(room, tx, ty, progress);
            let flame = SpriteId::Flame(((frame / 8) % 2) as u8);
            let (base, over) = match tile {
                Tile::Sand => (SpriteId::Sand, None),
                Tile::Grass => (SpriteId::Grass, None),
                Tile::Flower => (SpriteId::Flower, None),
                Tile::Tree => (SpriteId::Tree, None),
                Tile::Boulder => (SpriteId::Boulder, None),
                Tile::Water => (water, None),
                Tile::Bridge => (SpriteId::Bridge, None),
                Tile::Bush => (SpriteId::Grass, Some(SpriteId::Bush)),
                Tile::SandBush => (SpriteId::Sand, Some(SpriteId::Bush)),
                Tile::CaveMouth => (SpriteId::CaveMouth, None),
                Tile::DungeonDoor => (SpriteId::DungeonDoor, None),
                Tile::DungeonWall => (SpriteId::DungeonWall, None),
                Tile::DungeonFloor => (SpriteId::DungeonFloor, None),
                Tile::CaveWall => (SpriteId::CaveWall, None),
                Tile::CaveFloor => (SpriteId::CaveFloor, None),
                Tile::Door => (SpriteId::Door, None),
                Tile::Locked => (SpriteId::LockedDoor, None),
                Tile::Flame => (SpriteId::CaveFloor, Some(flame)),
                Tile::Mentor => (SpriteId::CaveFloor, Some(SpriteId::Mentor)),
            };
            out.push(draw(base, x, y));
            if let Some(o) = over {
                out.push(draw(o, x, y));
            }
        }
    }
    out
}

impl Game {
    fn hero_sprite(&self) -> (SpriteId, bool) {
        let frame = if self.hero.walking {
            ((self.hero.steps / 6) % 2) as u8
        } else {
            0
        };
        let f = self.hero.facing;
        let flip = f == Dir::Left;
        if matches!(self.mode, Mode::ItemGet { .. }) {
            return (SpriteId::HeroHold, false);
        }
        if matches!(self.hero.action, HeroAction::Stabbing { .. }) {
            return match f {
                Dir::Up => (SpriteId::HeroStabUp, false),
                Dir::Down => (SpriteId::HeroStabDown, false),
                _ => (SpriteId::HeroStabSide, flip),
            };
        }
        match f {
            Dir::Up => (SpriteId::HeroUp(frame), false),
            Dir::Down => (SpriteId::HeroDown(frame), false),
            _ => (SpriteId::HeroSide(frame), flip),
        }
    }

    fn enemy_sprite(e: &Enemy) -> (SpriteId, bool) {
        let side = |f: Dir| f == Dir::Left;
        match e.kind {
            EnemyKind::Spitter => {
                let fr = ((e.age / 12) % 2) as u8;
                match e.face {
                    Dir::Up => (SpriteId::SpitterUp(fr), false),
                    Dir::Down => (SpriteId::SpitterDown(fr), false),
                    f => (SpriteId::SpitterSide(fr), side(f)),
                }
            }
            EnemyKind::Bat => (SpriteId::Bat(((e.age / 6) % 2) as u8), false),
            EnemyKind::Skeleton => (SpriteId::Skeleton(((e.age / 10) % 2) as u8), false),
            EnemyKind::Gnasher => (SpriteId::Gnasher(((e.age / 20) % 2) as u8), false),
        }
    }

    /// Everything inside the playfield for one room, entities included, at offset `(ox, oy)`.
    fn entity_draws(&self, ox: i32, oy: i32, out: &mut Vec<Draw>) {
        let at = |v: f32, o: i32| v.round() as i32 + o;
        for p in &self.pickups {
            if matches!(p.kind, PickupKind::Gem | PickupKind::Heart)
                && p.age >= DROP_LIFE - DROP_FLICKER
                && (p.age / 4) % 2 == 0
            {
                continue;
            }
            let sprite = match p.kind {
                PickupKind::Gem => SpriteId::Gem,
                PickupKind::Heart => SpriteId::HeartFull,
                PickupKind::Sword => SpriteId::SwordItem,
                PickupKind::Key => SpriteId::Key,
                PickupKind::Relic => SpriteId::Relic,
            };
            let bob = if matches!(
                p.kind,
                PickupKind::Sword | PickupKind::Key | PickupKind::Relic
            ) {
                ((p.age / 12) % 2) as i32
            } else {
                0
            };
            out.push(draw(sprite, at(p.x, ox), at(p.y, oy) - bob));
        }
        for e in &self.enemies {
            let (sprite, flip_x) = Self::enemy_sprite(e);
            let flash = e.iframes > 0 && (e.iframes / 3) % 2 == 0;
            out.push(Draw {
                sprite,
                x: at(e.x, ox),
                y: at(e.y, oy),
                flip_x,
                flash,
            });
        }
        for s in &self.shots {
            let sprite = match s.kind {
                ProjKind::Rock => SpriteId::Stone,
                ProjKind::Fireball => SpriteId::Fireball(((self.tick / 6) % 2) as u8),
                ProjKind::Beam if s.dir().horizontal() => SpriteId::BeamH,
                ProjKind::Beam => SpriteId::BeamV,
            };
            out.push(draw(sprite, at(s.x - 8.0, ox), at(s.y - 8.0, oy)));
        }
        if self.mode != Mode::Title {
            let (hx, hy) = self.hero_pos_for_draw();
            let (sprite, flip_x) = self.hero_sprite();
            let flash = self.hero.invuln > 0 && (self.hero.invuln / 3).is_multiple_of(2);
            let (x, y) = (at(hx, ox), at(hy, oy));
            if let HeroAction::Stabbing { .. } = self.hero.action {
                let (blade, bx, by) = match self.hero.facing {
                    Dir::Up => (SpriteId::BladeUp, x, y - 12),
                    Dir::Down => (SpriteId::BladeDown, x, y + 12),
                    Dir::Left => (SpriteId::BladeSide, x - 12, y),
                    Dir::Right => (SpriteId::BladeSide, x + 12, y),
                };
                out.push(Draw {
                    sprite: blade,
                    x: bx,
                    y: by,
                    flip_x: self.hero.facing == Dir::Left,
                    flash: false,
                });
            }
            out.push(Draw {
                sprite,
                x,
                y,
                flip_x,
                flash,
            });
            if let Mode::ItemGet { item, .. } = self.mode {
                let sprite = match item {
                    PickupKind::Sword => SpriteId::SwordItem,
                    PickupKind::Key => SpriteId::Key,
                    _ => SpriteId::Relic,
                };
                out.push(draw(sprite, x, y - 14));
            }
        }
        for e in &self.effects {
            out.push(draw(
                SpriteId::Poof(u8::from(e.t >= 8)),
                at(e.x, ox),
                at(e.y, oy),
            ));
        }
    }

    fn hero_pos_for_draw(&self) -> (f32, f32) {
        match self.mode {
            Mode::Scrolling { dir, t, .. } => {
                let (fx, fy) = self.settled();
                let k = (t as f32 / Self::scroll_len(dir) as f32).min(1.0);
                (
                    self.hero.x + (fx - self.hero.x) * k,
                    self.hero.y + (fy - self.hero.y) * k,
                )
            }
            _ => (self.hero.x, self.hero.y),
        }
    }

    fn curtain(&self, t: u32, out: &mut Vec<Draw>) {
        let (lo, hi) = if t <= WIPE_HALF {
            (0, t as i32)
        } else {
            (t as i32 - WIPE_HALF as i32, ROOM_W)
        };
        for tx in lo..hi.min(ROOM_W) {
            for ty in 0..ROOM_H {
                out.push(draw(SpriteId::Black, tx * TILE, ty * TILE));
            }
        }
    }

    /// Everything in the playfield, in draw order, in logical pixels.
    pub fn scene(&self) -> Vec<Draw> {
        let mut out = vec![];
        match self.mode {
            Mode::Title => {
                black_field(&mut out);
                centered(&mut out, "LITTLE QUEST", 24);
                let f = ((self.tick / 12) % 2) as u8;
                out.push(draw(SpriteId::Flame(f), 88, 52));
                out.push(draw(SpriteId::HeroDown(f), 120, 52));
                out.push(draw(SpriteId::Flame(1 - f), 152, 52));
                centered(&mut out, "ARROWS OR WASD: MOVE", 88);
                centered(&mut out, "SPACE, Z OR X: SWORD", 100);
                centered(&mut out, "CMD-Q: QUIT", 112);
                if (self.tick / 30).is_multiple_of(2) {
                    centered(&mut out, "PRESS ENTER", 140);
                }
            }
            Mode::GameOver { .. } => {
                black_field(&mut out);
                centered(&mut out, "GAME OVER", 64);
                out.push(draw(SpriteId::Poof(1), 120, 88));
                centered(&mut out, "PRESS ENTER", 130);
            }
            Mode::Victory { .. } => {
                black_field(&mut out);
                centered(&mut out, "YOU WIN!", 24);
                out.push(draw(SpriteId::Relic, 120, 48));
                out.push(draw(SpriteId::HeroHold, 120, 72));
                centered(&mut out, &format!("GEMS: {}", self.gems), 104);
                let s = self.elapsed_secs();
                centered(&mut out, &format!("TIME: {}:{:02}", s / 60, s % 60), 116);
                centered(&mut out, "PRESS ENTER", 146);
            }
            Mode::Scrolling { from, to, dir, t } => {
                let s =
                    (t as i32 * SCROLL_SPEED).min(if dir.horizontal() { PLAY_W } else { PLAY_H });
                let (sx, sy) = match dir {
                    Dir::Up => (0, s),
                    Dir::Down => (0, -s),
                    Dir::Left => (s, 0),
                    Dir::Right => (-s, 0),
                };
                let (bx, by) = match dir {
                    Dir::Up => (0, -PLAY_H),
                    Dir::Down => (0, PLAY_H),
                    Dir::Left => (-PLAY_W, 0),
                    Dir::Right => (PLAY_W, 0),
                };
                out.extend(tile_draws(from, &self.progress, self.tick, sx, sy));
                out.extend(tile_draws(to, &self.progress, self.tick, sx + bx, sy + by));
                self.entity_draws(sx + bx, sy + by, &mut out);
            }
            _ => {
                out.extend(tile_draws(self.room, &self.progress, self.tick, 0, 0));
                self.entity_draws(0, 0, &mut out);
                match self.mode {
                    Mode::Wipe { t, .. } => self.curtain(t, &mut out),
                    Mode::Dialogue { chars, .. } => {
                        for ty in 1..3 {
                            for tx in 0..ROOM_W {
                                out.push(draw(SpriteId::Black, tx * TILE, ty * TILE));
                            }
                        }
                        let mut left = chars as usize;
                        for (i, line) in wrap(MENTOR_TEXT, 28).iter().enumerate() {
                            let shown: String = line.chars().take(left).collect();
                            left = left.saturating_sub(line.len() + 1);
                            text(&mut out, &shown, 16, 22 + i as i32 * 12);
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }
}
