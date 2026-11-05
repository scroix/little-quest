mod scene;
mod setup;

use crate::actors::{BEAM_SPEED, Ctx, Enemy, EnemyKind, ProjKind, Projectile, Rng};
use crate::world::{
    Dir, Edge, PLAY_H, PLAY_W, Progress, ROOM_H, ROOM_W, ROOMS, Rect, RoomId, TILE, blocked,
    tile_at,
};

pub use scene::Draw;

pub const TICK_HZ: u32 = 60;
pub const HERO_SPEED: f32 = 1.5;
pub const SCROLL_SPEED: i32 = 4;
pub const STAB_TICKS: u32 = 14;
pub const KNOCKBACK_TICKS: u32 = 12;
pub const KNOCKBACK_SPEED: f32 = 3.0;
pub const INVULN_TICKS: u32 = 48;
pub const START_HEARTS: u32 = 3;
pub const DROP_LIFE: u32 = 480;
pub const DROP_FLICKER: u32 = 120;
pub const WIPE_HALF: u32 = 16;
pub const MENTOR_TEXT: &str = "IT'S A LONG ROAD, TRAVELER. TAKE THIS.";
const SWORD_REACH: f32 = 12.0;
const MAX_GEMS: u32 = 99;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub dir: Option<Dir>,
    pub sword: bool,
    pub start: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    Gem,
    Heart,
    Sword,
    Key,
    Relic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Title,
    Playing,
    Scrolling {
        from: RoomId,
        to: RoomId,
        dir: Dir,
        t: u32,
    },
    Wipe {
        to: RoomId,
        arrive: (i32, i32),
        t: u32,
    },
    Dialogue {
        chars: u32,
        t: u32,
    },
    ItemGet {
        item: PickupKind,
        t: u32,
    },
    GameOver {
        t: u32,
    },
    Victory {
        t: u32,
    },
}

impl Mode {
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Title => "title",
            Mode::Playing => "playing",
            Mode::Scrolling { .. } => "scrolling",
            Mode::Wipe { .. } => "wipe",
            Mode::Dialogue { .. } => "dialogue",
            Mode::ItemGet { .. } => "itemget",
            Mode::GameOver { .. } => "gameover",
            Mode::Victory { .. } => "victory",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeroAction {
    Free,
    Stabbing { t: u32 },
    Knockback { dir: Dir, t: u32 },
}

#[derive(Clone, Debug)]
pub struct Hero {
    pub x: f32,
    pub y: f32,
    pub facing: Dir,
    pub action: HeroAction,
    pub hp: u32,
    pub max_hp: u32,
    pub invuln: u32,
    pub steps: u32,
    pub walking: bool,
}

impl Hero {
    pub fn body(&self) -> Rect {
        Rect::new(self.x + 2.0, self.y + 8.0, 12.0, 8.0)
    }

    pub fn hurtbox(&self) -> Rect {
        Rect::new(self.x + 3.0, self.y + 4.0, 10.0, 11.0)
    }

    pub fn tile(&self) -> (i32, i32) {
        (
            ((self.x + 8.0) / TILE as f32).floor() as i32,
            ((self.y + 12.0) / TILE as f32).floor() as i32,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pickup {
    pub kind: PickupKind,
    pub x: f32,
    pub y: f32,
    pub age: u32,
}

impl Pickup {
    fn new(kind: PickupKind, x: f32, y: f32) -> Pickup {
        Pickup { kind, x, y, age: 0 }
    }

    fn rect(&self) -> Rect {
        Rect::new(self.x + 3.0, self.y + 3.0, 10.0, 10.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Effect {
    pub x: f32,
    pub y: f32,
    pub t: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartState {
    Full,
    Half,
    Empty,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hud {
    pub hearts: Vec<HeartState>,
    pub gems: u32,
    pub keys: u32,
    pub sword: bool,
    pub map_size: (u8, u8),
    pub map_cell: (u8, u8),
    /// The cells of the grid that hold a room. The rest are gaps.
    pub map_rooms: Vec<(u8, u8)>,
}

pub struct Game {
    rng: Rng,
    tick: u64,
    run_ticks: u64,
    mode: Mode,
    room: RoomId,
    progress: Progress,
    hero: Hero,
    enemies: Vec<Enemy>,
    shots: Vec<Projectile>,
    pickups: Vec<Pickup>,
    effects: Vec<Effect>,
    gems: u32,
    keys: u32,
}

const START_POS: (f32, f32) = (124.0, 108.0);

impl Game {
    pub fn new(seed: u64) -> Game {
        let mut g = Game {
            rng: Rng::new(seed),
            tick: 0,
            run_ticks: 0,
            mode: Mode::Title,
            room: RoomId::Meadow,
            progress: Progress::default(),
            hero: Hero {
                x: START_POS.0,
                y: START_POS.1,
                facing: Dir::Down,
                action: HeroAction::Free,
                hp: START_HEARTS * 2,
                max_hp: START_HEARTS * 2,
                invuln: 0,
                steps: 0,
                walking: false,
            },
            enemies: vec![],
            shots: vec![],
            pickups: vec![],
            effects: vec![],
            gems: 0,
            keys: 0,
        };
        g.load_room(RoomId::Meadow);
        g
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn room(&self) -> RoomId {
        self.room
    }
    pub fn hero(&self) -> &Hero {
        &self.hero
    }
    pub fn enemies(&self) -> &[Enemy] {
        &self.enemies
    }
    pub fn shots(&self) -> &[Projectile] {
        &self.shots
    }
    pub fn pickups(&self) -> &[Pickup] {
        &self.pickups
    }
    pub fn gems(&self) -> u32 {
        self.gems
    }
    pub fn keys(&self) -> u32 {
        self.keys
    }
    pub fn progress(&self) -> &Progress {
        &self.progress
    }
    pub fn ticks(&self) -> u64 {
        self.tick
    }

    fn load_room(&mut self, room: RoomId) {
        self.room = room;
        self.shots.clear();
        self.pickups.clear();
        self.effects.clear();
        self.enemies = room
            .def()
            .spawns
            .iter()
            .filter(|(k, _, _)| !(*k == EnemyKind::Gnasher && self.progress.boss_dead))
            .map(|&(k, tx, ty)| Enemy::new(k, tx, ty))
            .collect();
        if room == RoomId::Cave && !self.progress.sword_taken {
            self.pickups
                .push(Pickup::new(PickupKind::Sword, 112.0, 96.0));
        }
        if room == RoomId::D3 && self.progress.boss_dead {
            self.pickups
                .push(Pickup::new(PickupKind::Relic, 120.0, 72.0));
        }
    }

    pub fn tick(&mut self, input: Input) {
        self.tick += 1;
        if !matches!(
            self.mode,
            Mode::Title | Mode::GameOver { .. } | Mode::Victory { .. }
        ) {
            self.run_ticks += 1;
        }
        match self.mode {
            Mode::Title => {
                if input.start {
                    self.mode = Mode::Playing;
                }
            }
            Mode::Playing => self.tick_playing(input),
            Mode::Scrolling { from, to, dir, t } => self.tick_scroll(from, to, dir, t),
            Mode::Wipe { to, arrive, t } => self.tick_wipe(to, arrive, t),
            Mode::Dialogue { chars, t } => self.tick_dialogue(chars, t, input),
            Mode::ItemGet { item, t } => {
                let dur = if item == PickupKind::Sword { 100 } else { 70 };
                if t + 1 >= dur {
                    self.mode = if item == PickupKind::Relic {
                        Mode::Victory { t: 0 }
                    } else {
                        Mode::Playing
                    };
                } else {
                    self.mode = Mode::ItemGet { item, t: t + 1 };
                }
            }
            Mode::GameOver { t } | Mode::Victory { t } => {
                if input.start && t >= 20 {
                    *self = Game::new(self.rng.next_u64());
                } else if matches!(self.mode, Mode::GameOver { .. }) {
                    self.mode = Mode::GameOver { t: t + 1 };
                } else {
                    self.mode = Mode::Victory { t: t + 1 };
                }
            }
        }
    }

    fn tick_playing(&mut self, input: Input) {
        self.hero.invuln = self.hero.invuln.saturating_sub(1);
        self.update_hero(input);
        if self.mode != Mode::Playing {
            return;
        }
        self.update_enemies();
        self.update_shots();
        self.hit_enemies();
        self.hero_contacts();
        self.update_pickups();
        self.effects.retain_mut(|e| {
            e.t += 1;
            e.t < 16
        });
        self.room_events();
        if self.hero.hp == 0 && self.mode == Mode::Playing {
            self.mode = Mode::GameOver { t: 0 };
        }
    }

    fn update_hero(&mut self, input: Input) {
        self.hero.walking = false;
        match self.hero.action {
            HeroAction::Free => {
                if input.sword && self.progress.sword_taken {
                    if let Some(d) = input.dir {
                        self.hero.facing = d;
                    }
                    self.hero.action = HeroAction::Stabbing { t: 0 };
                    self.fire_beam();
                    return;
                }
                if let Some(d) = input.dir {
                    self.walk(d);
                    self.try_open_door();
                }
                self.check_exits();
            }
            HeroAction::Stabbing { t } => {
                self.hero.action = if t + 1 >= STAB_TICKS {
                    HeroAction::Free
                } else {
                    HeroAction::Stabbing { t: t + 1 }
                };
            }
            HeroAction::Knockback { dir, t } => {
                for _ in 0..6 {
                    if !self.try_step(dir, KNOCKBACK_SPEED / 6.0, false) {
                        break;
                    }
                }
                self.hero.action = if t + 1 >= KNOCKBACK_TICKS {
                    HeroAction::Free
                } else {
                    HeroAction::Knockback { dir, t: t + 1 }
                };
            }
        }
    }

    fn fire_beam(&mut self) {
        let full = self.hero.hp == self.hero.max_hp;
        if !full || self.shots.iter().any(|s| s.kind == ProjKind::Beam) {
            return;
        }
        let (dx, dy) = self.hero.facing.delta();
        self.shots.push(Projectile {
            kind: ProjKind::Beam,
            x: self.hero.x + 8.0 + dx * 14.0,
            y: self.hero.y + 8.0 + dy * 14.0,
            vx: dx * BEAM_SPEED,
            vy: dy * BEAM_SPEED,
        });
    }

    fn body_blocked(&self, x: f32, y: f32, leave: bool) -> bool {
        blocked(
            self.room,
            Rect::new(x + 2.0, y + 8.0, 12.0, 8.0),
            &self.progress,
            leave,
        )
    }

    fn try_step(&mut self, d: Dir, amt: f32, leave: bool) -> bool {
        let (dx, dy) = d.delta();
        let (nx, ny) = (self.hero.x + dx * amt, self.hero.y + dy * amt);
        if self.body_blocked(nx, ny, leave) {
            return false;
        }
        self.hero.x = nx;
        self.hero.y = ny;
        true
    }

    fn walk(&mut self, d: Dir) {
        self.hero.facing = d;
        let mut moved_all = true;
        let mut moved_any = false;
        for _ in 0..3 {
            if self.try_step(d, HERO_SPEED / 3.0, true) {
                moved_any = true;
            } else {
                moved_all = false;
                break;
            }
        }
        if !moved_all {
            self.corner_assist(d);
        }
        if moved_any {
            self.hero.steps += 1;
            self.hero.walking = true;
        }
    }

    fn solid_at(&self, tx: i32, ty: i32) -> bool {
        let (px, py) = ((tx * TILE + 7) as f32, (ty * TILE + 7) as f32);
        blocked(self.room, Rect::new(px, py, 2.0, 2.0), &self.progress, true)
    }

    /// Slide the hero toward the free half of a two-tile front when only one tile blocks the way.
    fn corner_assist(&mut self, d: Dir) {
        const MIN_OVERLAP: f32 = 3.0;
        let (dx, dy) = d.delta();
        let b = {
            let b = self.hero.body();
            Rect::new(b.x + dx * 0.5, b.y + dy * 0.5, b.w, b.h)
        };
        let t = TILE as f32;
        let tile = |v: f32| (v / t).floor() as i32;
        let (lo_a, hi_a, lead, perp_lo, perp_hi) = if d.horizontal() {
            let lead = tile(if d == Dir::Right {
                b.x + b.w - 0.001
            } else {
                b.x
            });
            (tile(b.y), tile(b.y + b.h - 0.001), lead, b.y, b.y + b.h)
        } else {
            let lead = tile(if d == Dir::Down {
                b.y + b.h - 0.001
            } else {
                b.y
            });
            (tile(b.x), tile(b.x + b.w - 0.001), lead, b.x, b.x + b.w)
        };
        if lo_a == hi_a {
            return;
        }
        let solid = |a: i32| {
            if d.horizontal() {
                self.solid_at(lead, a)
            } else {
                self.solid_at(a, lead)
            }
        };
        let (s_lo, s_hi) = (solid(lo_a), solid(hi_a));
        if s_lo == s_hi {
            return;
        }
        let (overlap, dist, push) = if s_lo {
            (perp_hi - hi_a as f32 * t, hi_a as f32 * t - perp_lo, 1.0)
        } else {
            (
                (lo_a + 1) as f32 * t - perp_lo,
                perp_hi - (lo_a + 1) as f32 * t,
                -1.0,
            )
        };
        if overlap < MIN_OVERLAP {
            return;
        }
        let amt = dist.min(1.0);
        let side = match (d.horizontal(), push > 0.0) {
            (true, true) => Dir::Down,
            (true, false) => Dir::Up,
            (false, true) => Dir::Right,
            (false, false) => Dir::Left,
        };
        self.try_step(side, amt, true);
    }

    fn try_open_door(&mut self) {
        if self.keys == 0 {
            return;
        }
        let b = self.hero.body();
        let (cx, cy) = b.center();
        let (dx, dy) = self.hero.facing.delta();
        let (px, py) = (cx + dx * 8.0, cy + dy * 8.0);
        let (tx, ty) = (
            (px / TILE as f32).floor() as i32,
            (py / TILE as f32).floor() as i32,
        );
        if !(0..ROOM_W).contains(&tx) || !(0..ROOM_H).contains(&ty) {
            return;
        }
        if self.room.def().raw(tx, ty) == 'L' && !self.progress.is_open(self.room, tx, ty) {
            self.keys -= 1;
            self.progress.opened.push((self.room, tx, ty));
        }
    }

    fn check_exits(&mut self) {
        let (cx, cy) = (self.hero.x + 8.0, self.hero.y + 8.0);
        let dir = if cx < 0.0 {
            Some(Dir::Left)
        } else if cx >= PLAY_W as f32 {
            Some(Dir::Right)
        } else if cy < 0.0 {
            Some(Dir::Up)
        } else if cy >= PLAY_H as f32 {
            Some(Dir::Down)
        } else {
            None
        };
        if let Some(d) = dir {
            match self.room.def().edge(d) {
                Edge::Scroll(to) => self.start_scroll(d, to),
                Edge::Wipe(to, arrive) => self.mode = Mode::Wipe { to, arrive, t: 0 },
                Edge::Wall => {}
            }
            return;
        }
        let (tx, ty) = self.hero.tile();
        if let Some(p) = self.room.def().portal_at(tx, ty) {
            self.mode = Mode::Wipe {
                to: p.to,
                arrive: p.arrive,
                t: 0,
            };
        }
    }

    fn start_scroll(&mut self, dir: Dir, to: RoomId) {
        let from = self.room;
        match dir {
            Dir::Up => self.hero.y += PLAY_H as f32,
            Dir::Down => self.hero.y -= PLAY_H as f32,
            Dir::Left => self.hero.x += PLAY_W as f32,
            Dir::Right => self.hero.x -= PLAY_W as f32,
        }
        let (x, y) = (self.hero.x, self.hero.y);
        self.load_room(to);
        self.hero.x = x;
        self.hero.y = y;
        self.mode = Mode::Scrolling {
            from,
            to,
            dir,
            t: 0,
        };
    }

    fn scroll_len(dir: Dir) -> u32 {
        let px = if dir.horizontal() { PLAY_W } else { PLAY_H };
        (px / SCROLL_SPEED) as u32
    }

    fn settled(&self) -> (f32, f32) {
        (
            self.hero.x.clamp(0.0, (PLAY_W - TILE) as f32),
            self.hero.y.clamp(0.0, (PLAY_H - TILE) as f32),
        )
    }

    fn tick_scroll(&mut self, from: RoomId, to: RoomId, dir: Dir, t: u32) {
        self.hero.walking = true;
        self.hero.steps += 1;
        if t + 1 >= Self::scroll_len(dir) {
            let (x, y) = self.settled();
            self.hero.x = x;
            self.hero.y = y;
            self.hero.walking = false;
            self.mode = Mode::Playing;
        } else {
            self.mode = Mode::Scrolling {
                from,
                to,
                dir,
                t: t + 1,
            };
        }
    }

    fn tick_wipe(&mut self, to: RoomId, arrive: (i32, i32), t: u32) {
        let t = t + 1;
        if t == WIPE_HALF {
            self.load_room(to);
            self.hero.x = arrive.0 as f32;
            self.hero.y = arrive.1 as f32;
            self.hero.action = HeroAction::Free;
        }
        self.mode = if t >= 2 * WIPE_HALF {
            if to == RoomId::Cave && !self.progress.sword_taken {
                Mode::Dialogue { chars: 0, t: 0 }
            } else {
                Mode::Playing
            }
        } else {
            Mode::Wipe { to, arrive, t }
        };
    }

    fn tick_dialogue(&mut self, chars: u32, t: u32, input: Input) {
        let total = MENTOR_TEXT.len() as u32;
        let t = t + 1;
        let mut chars = chars;
        if chars < total && t.is_multiple_of(3) {
            chars += 1;
        }
        let pressed = input.sword || input.start;
        if pressed && chars < total {
            chars = total;
        } else if (pressed && t > 12) || (chars == total && t > total * 3 + 90) {
            self.mode = Mode::Playing;
            return;
        }
        self.mode = Mode::Dialogue { chars, t };
    }

    fn update_enemies(&mut self) {
        let hero = (self.hero.x + 8.0, self.hero.y + 8.0);
        let room = self.room;
        let progress = &self.progress;
        let field = |r: Rect| blocked(room, r, progress, false);
        for e in &mut self.enemies {
            let mut ctx = Ctx {
                rng: &mut self.rng,
                hero,
                blocked: &field,
                shots: &mut self.shots,
            };
            e.step(&mut ctx);
        }
    }

    fn update_shots(&mut self) {
        let room = self.room;
        let mut i = 0;
        while i < self.shots.len() {
            let s = &mut self.shots[i];
            s.x += s.vx;
            s.y += s.vy;
            let (tx, ty) = (
                (s.x / TILE as f32).floor() as i32,
                (s.y / TILE as f32).floor() as i32,
            );
            let out = !(0..ROOM_W).contains(&tx) || !(0..ROOM_H).contains(&ty);
            let wall = !out && tile_at(room, tx, ty, &self.progress).stops_shots();
            if out || wall {
                self.shots.swap_remove(i);
                continue;
            }
            i += 1;
        }
        let hurt = self.hero.hurtbox();
        let mut incoming = vec![];
        self.shots.retain(|s| {
            if s.kind == ProjKind::Beam || !s.rect().overlaps(&hurt) {
                return true;
            }
            incoming.push(*s);
            false
        });
        for s in incoming {
            let shielded = s.kind == ProjKind::Rock
                && self.hero.facing == s.dir().opposite()
                && !matches!(self.hero.action, HeroAction::Stabbing { .. });
            if !shielded {
                self.hurt_hero(s.damage(), s.dir());
            }
        }
    }

    fn sword_rect(&self) -> Option<Rect> {
        let HeroAction::Stabbing { t } = self.hero.action else {
            return None;
        };
        if !(1..STAB_TICKS - 2).contains(&t) {
            return None;
        }
        let (x, y) = (self.hero.x, self.hero.y);
        Some(match self.hero.facing {
            Dir::Up => Rect::new(x + 5.0, y + 4.0 - SWORD_REACH, 6.0, SWORD_REACH + 6.0),
            Dir::Down => Rect::new(x + 5.0, y + 10.0, 6.0, SWORD_REACH + 6.0),
            Dir::Left => Rect::new(x + 4.0 - SWORD_REACH, y + 6.0, SWORD_REACH + 6.0, 7.0),
            Dir::Right => Rect::new(x + 10.0, y + 6.0, SWORD_REACH + 6.0, 7.0),
        })
    }

    fn hit_enemies(&mut self) {
        let sword = self.sword_rect();
        let facing = self.hero.facing;
        let mut dead = vec![];
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if sword.is_some_and(|r| r.overlaps(&e.rect())) && e.hit(1, facing) == Some(true) {
                dead.push(i);
            }
        }
        let mut beams_spent = vec![];
        for (bi, s) in self.shots.iter().enumerate() {
            if s.kind != ProjKind::Beam {
                continue;
            }
            for (i, e) in self.enemies.iter_mut().enumerate() {
                if dead.contains(&i) || !s.rect().overlaps(&e.rect()) {
                    continue;
                }
                if let Some(killed) = e.hit(1, s.dir()) {
                    beams_spent.push(bi);
                    if killed {
                        dead.push(i);
                    }
                    break;
                }
            }
        }
        beams_spent.sort_unstable_by(|a, b| b.cmp(a));
        for bi in beams_spent {
            self.shots.remove(bi);
        }
        dead.sort_unstable_by(|a, b| b.cmp(a));
        dead.dedup();
        for i in dead {
            let e = self.enemies.remove(i);
            self.on_kill(e);
        }
    }

    /// The pickup spot nearest `(cx, cy)` that a hero can stand on, so drops never land inside walls or water.
    fn drop_spot(&self, cx: f32, cy: f32) -> (f32, f32) {
        let here = (
            (cx / TILE as f32).floor() as i32,
            (cy / TILE as f32).floor() as i32,
        );
        let free = |tx: i32, ty: i32| {
            (0..ROOM_W).contains(&tx)
                && (0..ROOM_H).contains(&ty)
                && !tile_at(self.room, tx, ty, &self.progress).solid()
        };
        if free(here.0, here.1) {
            return (cx - 8.0, cy - 8.0);
        }
        let best = (0..ROOM_H)
            .flat_map(|ty| (0..ROOM_W).map(move |tx| (tx, ty)))
            .filter(|&(tx, ty)| free(tx, ty))
            .min_by_key(|&(tx, ty)| (tx - here.0).pow(2) + (ty - here.1).pow(2))
            .unwrap_or(here);
        ((best.0 * TILE) as f32, (best.1 * TILE) as f32)
    }

    fn on_kill(&mut self, e: Enemy) {
        let (cx, cy) = e.center();
        self.effects.push(Effect {
            x: cx - 8.0,
            y: cy - 8.0,
            t: 0,
        });
        let kind = if e.kind == EnemyKind::Gnasher {
            self.progress.boss_dead = true;
            PickupKind::Relic
        } else if self.rng.below(3) == 0 {
            if self.rng.chance(60) {
                PickupKind::Gem
            } else {
                PickupKind::Heart
            }
        } else {
            return;
        };
        let (x, y) = self.drop_spot(cx, cy);
        self.pickups.push(Pickup::new(kind, x, y));
    }

    fn hurt_hero(&mut self, damage: u8, push: Dir) {
        if self.hero.invuln > 0 {
            return;
        }
        self.hero.hp = self.hero.hp.saturating_sub(damage as u32);
        self.hero.invuln = INVULN_TICKS;
        self.hero.action = HeroAction::Knockback { dir: push, t: 0 };
    }

    fn hero_contacts(&mut self) {
        let hurt = self.hero.hurtbox();
        let (hx, hy) = hurt.center();
        let hit = self
            .enemies
            .iter()
            .find(|e| e.rect().overlaps(&hurt))
            .map(|e| {
                let (ex, ey) = e.center();
                (e.kind.stats().contact, Dir::from_vec(hx - ex, hy - ey))
            });
        if let Some((dmg, push)) = hit {
            self.hurt_hero(dmg, push);
        }
    }

    fn update_pickups(&mut self) {
        let hurt = self.hero.hurtbox();
        let mut taken = None;
        self.pickups.retain_mut(|p| {
            p.age += 1;
            let temporary = matches!(p.kind, PickupKind::Gem | PickupKind::Heart);
            if temporary && p.age >= DROP_LIFE {
                return false;
            }
            if taken.is_none() && p.rect().overlaps(&hurt) {
                taken = Some(p.kind);
                return false;
            }
            true
        });
        let Some(kind) = taken else { return };
        match kind {
            PickupKind::Gem => self.gems = (self.gems + 1).min(MAX_GEMS),
            PickupKind::Heart => self.hero.hp = (self.hero.hp + 2).min(self.hero.max_hp),
            PickupKind::Sword => self.begin_item(kind, |p| p.sword_taken = true),
            PickupKind::Key => {
                self.keys += 1;
                self.begin_item(kind, |p| p.key_taken = true);
            }
            PickupKind::Relic => self.begin_item(kind, |_| {}),
        }
    }

    fn begin_item(&mut self, kind: PickupKind, grant: impl FnOnce(&mut Progress)) {
        grant(&mut self.progress);
        self.hero.action = HeroAction::Free;
        self.mode = Mode::ItemGet { item: kind, t: 0 };
    }

    fn room_events(&mut self) {
        let key_out = self.pickups.iter().any(|p| p.kind == PickupKind::Key);
        if self.room == RoomId::D2
            && self.enemies.is_empty()
            && !self.progress.key_taken
            && !key_out
        {
            self.pickups.push(Pickup::new(PickupKind::Key, 120.0, 80.0));
        }
    }

    pub fn hud(&self) -> Hud {
        let hearts = (0..self.hero.max_hp / 2)
            .map(|i| match self.hero.hp.saturating_sub(i * 2) {
                0 => HeartState::Empty,
                1 => HeartState::Half,
                _ => HeartState::Full,
            })
            .collect();
        let def = self.room.def();
        Hud {
            hearts,
            gems: self.gems,
            keys: self.keys,
            sword: self.progress.sword_taken,
            map_size: def.area.grid(),
            map_cell: def.cell,
            map_rooms: ROOMS
                .iter()
                .filter(|r| r.area == def.area)
                .map(|r| r.cell)
                .collect(),
        }
    }

    /// Seconds from the start press until victory or game over, transitions and dialogue included.
    pub fn elapsed_secs(&self) -> u64 {
        self.run_ticks / TICK_HZ as u64
    }
}
