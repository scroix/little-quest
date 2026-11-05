use crate::world::{Dir, PLAY_H, PLAY_W, Rect, TILE};

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u64() >> 32) % n as u64) as u32
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Spitter,
    Bat,
    Skeleton,
    Gnasher,
}

#[derive(Clone, Copy, Debug)]
pub enum Ai {
    Grid { shoots: bool, chase: u32 },
    Flutter,
    Patrol,
}

pub struct EnemyStats {
    pub hp: u8,
    pub speed: f32,
    pub size: f32,
    pub inset: f32,
    pub contact: u8,
    pub knock: u32,
    pub ai: Ai,
}

pub const STATS: [EnemyStats; 4] = [
    EnemyStats {
        hp: 1,
        speed: 0.75,
        size: 16.0,
        inset: 2.0,
        contact: 1,
        knock: 10,
        ai: Ai::Grid {
            shoots: true,
            chase: 20,
        },
    },
    EnemyStats {
        hp: 1,
        speed: 1.5,
        size: 16.0,
        inset: 3.0,
        contact: 1,
        knock: 10,
        ai: Ai::Flutter,
    },
    EnemyStats {
        hp: 2,
        speed: 1.0,
        size: 16.0,
        inset: 2.0,
        contact: 1,
        knock: 10,
        ai: Ai::Grid {
            shoots: false,
            chase: 55,
        },
    },
    EnemyStats {
        hp: 6,
        speed: 0.75,
        size: 32.0,
        inset: 4.0,
        contact: 2,
        knock: 4,
        ai: Ai::Patrol,
    },
];

pub const ROCK_SPEED: f32 = 2.0;
pub const FIREBALL_SPEED: f32 = 1.5;
pub const BEAM_SPEED: f32 = 3.0;
pub const GNASHER_BOUNDS: Rect = Rect {
    x: 112.0,
    y: 24.0,
    w: 96.0,
    h: 96.0,
};
const GNASHER_FIRE_EVERY: u32 = 90;

impl EnemyKind {
    pub fn stats(self) -> &'static EnemyStats {
        &STATS[self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyState {
    Walk { dir: Dir, left: f32 },
    Pause { left: u32 },
    Aim { dir: Dir, left: u32 },
    Drift { dx: f32, dy: f32, t: u32, len: u32 },
    Patrol { dx: f32, dy: f32, left: u32 },
    Hurt { dir: Dir, left: u32 },
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub x: f32,
    pub y: f32,
    pub hp: u8,
    pub face: Dir,
    pub state: EnemyState,
    pub iframes: u32,
    pub age: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjKind {
    Rock,
    Fireball,
    Beam,
}

/// Position is the centre of the projectile.
#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    pub kind: ProjKind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

impl Projectile {
    pub fn rect(&self) -> Rect {
        let s = match self.kind {
            ProjKind::Rock => 6.0,
            ProjKind::Fireball => 8.0,
            ProjKind::Beam => 10.0,
        };
        Rect::new(self.x - s / 2.0, self.y - s / 2.0, s, s)
    }

    pub fn damage(&self) -> u8 {
        match self.kind {
            ProjKind::Rock => 1,
            ProjKind::Fireball => 1,
            ProjKind::Beam => 1,
        }
    }

    pub fn dir(&self) -> Dir {
        Dir::from_vec(self.vx, self.vy)
    }
}

pub struct Ctx<'a> {
    pub rng: &'a mut Rng,
    pub hero: (f32, f32),
    pub blocked: &'a dyn Fn(Rect) -> bool,
    pub shots: &'a mut Vec<Projectile>,
}

impl Enemy {
    pub fn new(kind: EnemyKind, tx: i32, ty: i32) -> Enemy {
        Enemy {
            kind,
            x: (tx * TILE) as f32,
            y: (ty * TILE) as f32,
            hp: kind.stats().hp,
            face: Dir::Down,
            state: EnemyState::Pause {
                left: 30 + (tx * 7 + ty * 3) as u32 % 30,
            },
            iframes: 0,
            age: 0,
        }
    }

    pub fn rect(&self) -> Rect {
        let s = self.kind.stats();
        Rect::new(
            self.x + s.inset,
            self.y + s.inset,
            s.size - 2.0 * s.inset,
            s.size - 2.0 * s.inset,
        )
    }

    pub fn center(&self) -> (f32, f32) {
        let h = self.kind.stats().size / 2.0;
        (self.x + h, self.y + h)
    }

    /// Returns `None` while the enemy cannot be hit, otherwise whether the hit killed it.
    pub fn hit(&mut self, damage: u8, push: Dir) -> Option<bool> {
        if self.iframes > 0 {
            return None;
        }
        self.hp = self.hp.saturating_sub(damage);
        if self.hp == 0 {
            return Some(true);
        }
        self.state = EnemyState::Hurt {
            dir: push,
            left: self.kind.stats().knock,
        };
        self.iframes = 20;
        Some(false)
    }

    pub fn step(&mut self, c: &mut Ctx) {
        self.age += 1;
        self.iframes = self.iframes.saturating_sub(1);
        let st = self.kind.stats();
        if let EnemyState::Hurt { dir, left } = self.state {
            let (dx, dy) = dir.delta();
            let moved = Rect {
                x: self.rect().x + dx * 2.0,
                y: self.rect().y + dy * 2.0,
                ..self.rect()
            };
            let free = match st.ai {
                Ai::Flutter => true,
                _ => !(c.blocked)(moved),
            };
            if free {
                self.x += dx * 2.0;
                self.y += dy * 2.0;
            }
            self.state = if left <= 1 {
                EnemyState::Pause { left: 12 }
            } else {
                EnemyState::Hurt {
                    dir,
                    left: left - 1,
                }
            };
            self.keep_in_field();
            return;
        }
        match st.ai {
            Ai::Grid { shoots, chase } => self.step_grid(c, st.speed, shoots, chase),
            Ai::Flutter => self.step_flutter(c, st.speed),
            Ai::Patrol => self.step_patrol(c, st.speed),
        }
    }

    fn keep_in_field(&mut self) {
        let size = self.kind.stats().size;
        self.x = self.x.clamp(0.0, PLAY_W as f32 - size);
        self.y = self.y.clamp(0.0, PLAY_H as f32 - size);
    }

    fn step_grid(&mut self, c: &mut Ctx, speed: f32, shoots: bool, chase: u32) {
        match self.state {
            EnemyState::Walk { dir, left } => {
                let step = speed.min(left);
                let (dx, dy) = dir.delta();
                self.x += dx * step;
                self.y += dy * step;
                if left - step <= 0.001 {
                    self.x = (self.x / TILE as f32).round() * TILE as f32;
                    self.y = (self.y / TILE as f32).round() * TILE as f32;
                    self.decide_grid(c, shoots, chase);
                } else {
                    self.state = EnemyState::Walk {
                        dir,
                        left: left - step,
                    };
                }
            }
            EnemyState::Pause { left } => {
                if left == 0 {
                    self.decide_grid(c, shoots, chase);
                } else {
                    self.state = EnemyState::Pause { left: left - 1 };
                }
            }
            EnemyState::Aim { dir, left } => {
                if left == 0 {
                    let (cx, cy) = self.center();
                    let (dx, dy) = dir.delta();
                    c.shots.push(Projectile {
                        kind: ProjKind::Rock,
                        x: cx + dx * 8.0,
                        y: cy + dy * 8.0,
                        vx: dx * ROCK_SPEED,
                        vy: dy * ROCK_SPEED,
                    });
                    self.state = EnemyState::Pause { left: 30 };
                } else {
                    self.state = EnemyState::Aim {
                        dir,
                        left: left - 1,
                    };
                }
            }
            _ => self.state = EnemyState::Pause { left: 10 },
        }
    }

    fn decide_grid(&mut self, c: &mut Ctx, shoots: bool, chase: u32) {
        let (hx, hy) = c.hero;
        let (cx, cy) = self.center();
        let toward = Dir::from_vec(hx - cx, hy - cy);
        if shoots && c.rng.chance(22) {
            let dir = if c.rng.chance(60) { toward } else { self.face };
            self.face = dir;
            self.state = EnemyState::Aim { dir, left: 24 };
            return;
        }
        if c.rng.chance(12) {
            self.state = EnemyState::Pause {
                left: 15 + c.rng.below(40),
            };
            return;
        }
        let first = if c.rng.chance(chase) {
            toward
        } else if c.rng.chance(50) {
            self.face
        } else {
            Dir::ALL[c.rng.below(4) as usize]
        };
        let start = c.rng.below(4) as usize;
        let order = std::iter::once(first).chain((0..4).map(|i| Dir::ALL[(start + i) % 4]));
        for dir in order {
            let (dx, dy) = dir.delta();
            let r = self.rect();
            let dest = Rect {
                x: r.x + dx * TILE as f32,
                y: r.y + dy * TILE as f32,
                ..r
            };
            if !(c.blocked)(dest) {
                self.face = dir;
                self.state = EnemyState::Walk {
                    dir,
                    left: TILE as f32,
                };
                return;
            }
        }
        self.state = EnemyState::Pause { left: 20 };
    }

    fn step_flutter(&mut self, c: &mut Ctx, speed: f32) {
        match self.state {
            EnemyState::Drift { dx, dy, t, len } => {
                let ramp = (std::f32::consts::PI * t as f32 / len as f32).sin();
                self.x += dx * speed * ramp;
                self.y += dy * speed * ramp;
                self.keep_in_field();
                self.face = if dx < 0.0 { Dir::Left } else { Dir::Right };
                self.state = if t + 1 >= len {
                    EnemyState::Pause {
                        left: 10 + c.rng.below(30),
                    }
                } else {
                    EnemyState::Drift {
                        dx,
                        dy,
                        t: t + 1,
                        len,
                    }
                };
            }
            EnemyState::Pause { left } if left > 0 => {
                self.state = EnemyState::Pause { left: left - 1 }
            }
            _ => {
                let (hx, hy) = c.hero;
                let (cx, cy) = self.center();
                let dirs: [(f32, f32); 8] = [
                    (0., -1.),
                    (0., 1.),
                    (-1., 0.),
                    (1., 0.),
                    (0.7, 0.7),
                    (-0.7, 0.7),
                    (0.7, -0.7),
                    (-0.7, -0.7),
                ];
                let (dx, dy) = if c.rng.chance(35) {
                    let v = ((hx - cx).signum(), (hy - cy).signum());
                    (v.0 * 0.7, v.1 * 0.7)
                } else {
                    dirs[c.rng.below(8) as usize]
                };
                self.state = EnemyState::Drift {
                    dx,
                    dy,
                    t: 0,
                    len: 40 + c.rng.below(50),
                };
            }
        }
    }

    fn step_patrol(&mut self, c: &mut Ctx, speed: f32) {
        let b = GNASHER_BOUNDS;
        if (self.age + GNASHER_FIRE_EVERY / 2).is_multiple_of(GNASHER_FIRE_EVERY) {
            let (cx, cy) = self.center();
            let base = (c.hero.1 - cy).atan2(c.hero.0 - cx);
            for spread in [-0.4f32, 0.0, 0.4] {
                let a = base + spread;
                c.shots.push(Projectile {
                    kind: ProjKind::Fireball,
                    x: cx,
                    y: cy,
                    vx: a.cos() * FIREBALL_SPEED,
                    vy: a.sin() * FIREBALL_SPEED,
                });
            }
        }
        let (dx, dy, left) = match self.state {
            EnemyState::Patrol { dx, dy, left } if left > 0 => (dx, dy, left - 1),
            _ => {
                let pick = |r: &mut Rng| r.below(3) as f32 - 1.0;
                let (mut dx, dy) = (pick(c.rng), pick(c.rng));
                if dx == 0.0 && dy == 0.0 {
                    dx = if self.x > b.x + b.w / 2.0 { -1.0 } else { 1.0 };
                }
                (dx, dy, 50 + c.rng.below(60))
            }
        };
        let (mut dx, mut dy) = (dx, dy);
        let (nx, ny) = (self.x + dx * speed, self.y + dy * speed);
        if nx < b.x || nx > b.x + b.w {
            dx = -dx;
        }
        if ny < b.y || ny > b.y + b.h {
            dy = -dy;
        }
        self.x = (self.x + dx * speed).clamp(b.x, b.x + b.w);
        self.y = (self.y + dy * speed).clamp(b.y, b.y + b.h);
        self.face = if dx < 0.0 { Dir::Left } else { Dir::Right };
        self.state = EnemyState::Patrol { dx, dy, left };
    }
}
