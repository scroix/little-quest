use std::collections::VecDeque;

use crate::actors::ProjKind;
use crate::game::{Game, Input, Mode, PickupKind};
use crate::world::{Dir, Edge, ROOM_H, ROOM_W, RoomId, TILE, Tile, tile_at};

fn perp(vx: f32, vy: f32, cx: f32, cy: f32) -> (f32, f32) {
    let (px, py) = (-vy, vx);
    let side = if px * cx + py * cy > 0.0 { -1.0 } else { 1.0 };
    (px * side, py * side)
}

type Node = (RoomId, i32, i32);

const ENGAGE: f32 = 56.0;
const LINE_UP: f32 = 5.0;

/// Plays the game through `Input`, the way a player would.
#[derive(Default)]
pub struct Bot {
    last_sword: bool,
}

fn tile_of(px: (i32, i32)) -> (i32, i32) {
    ((px.0 + 8).div_euclid(TILE), (px.1 + 12).div_euclid(TILE))
}

fn passable(g: &Game, room: RoomId, tx: i32, ty: i32) -> bool {
    let t = tile_at(room, tx, ty, g.progress());
    !t.solid() || (t == Tile::Locked && g.keys() > 0)
}

fn neighbors(g: &Game, (room, tx, ty): Node) -> Vec<(Node, Dir)> {
    let mut out = vec![];
    let def = room.def();
    for d in Dir::ALL {
        let (dx, dy) = d.delta();
        let (nx, ny) = (tx + dx as i32, ty + dy as i32);
        if (0..ROOM_W).contains(&nx) && (0..ROOM_H).contains(&ny) {
            if passable(g, room, nx, ny) {
                out.push(((room, nx, ny), d));
            }
            continue;
        }
        match def.edge(d) {
            Edge::Scroll(to) => {
                let (wx, wy) = (nx.rem_euclid(ROOM_W), ny.rem_euclid(ROOM_H));
                if passable(g, to, wx, wy) {
                    out.push(((to, wx, wy), d));
                }
            }
            Edge::Wipe(to, arrive) => {
                let (wx, wy) = tile_of(arrive);
                out.push(((to, wx, wy), d));
            }
            Edge::Wall => {}
        }
    }
    if let Some(p) = def.portal_at(tx, ty) {
        let (wx, wy) = tile_of(p.arrive);
        out.push(((p.to, wx, wy), Dir::Up));
    }
    out
}

fn index((room, tx, ty): Node) -> usize {
    (room as usize * ROOM_H as usize + ty as usize) * ROOM_W as usize + tx as usize
}

/// First step of the shortest walk from `from` to `to`.
fn first_step(g: &Game, from: Node, to: Node) -> Option<Dir> {
    if from == to {
        return None;
    }
    let mut first: Vec<Option<Dir>> = vec![None; RoomId::ALL.len() * (ROOM_W * ROOM_H) as usize];
    let mut seen = vec![false; first.len()];
    let mut q = VecDeque::from([from]);
    seen[index(from)] = true;
    while let Some(n) = q.pop_front() {
        for (m, d) in neighbors(g, n) {
            if seen[index(m)] {
                continue;
            }
            seen[index(m)] = true;
            first[index(m)] = if n == from { Some(d) } else { first[index(n)] };
            if m == to {
                return first[index(m)];
            }
            q.push_back(m);
        }
    }
    None
}

impl Bot {
    pub fn new() -> Bot {
        Bot::default()
    }

    pub fn decide(&mut self, g: &Game) -> Input {
        let want = match g.mode() {
            Mode::Title | Mode::GameOver { .. } => Input {
                start: true,
                ..Input::default()
            },
            Mode::Playing => self.play(g),
            _ => Input::default(),
        };
        let sword = want.sword && !self.last_sword;
        self.last_sword = sword;
        Input { sword, ..want }
    }

    fn play(&mut self, g: &Game) -> Input {
        let h = g.hero();
        let hc = (h.x + 8.0, h.y + 8.0);
        if let Some(dir) = self.dodge(g, hc) {
            return Input {
                dir: Some(dir),
                ..Input::default()
            };
        }
        let hunting = self.hunting(g);
        let near = g
            .enemies()
            .iter()
            .map(|e| {
                let c = e.center();
                (((c.0 - hc.0).powi(2) + (c.1 - hc.1).powi(2)).sqrt(), e)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((dist, e)) = near.filter(|(d, _)| *d < ENGAGE || hunting) {
            return self.fight(g, e, dist);
        }
        let here = (g.room(), h.tile().0, h.tile().1);
        let Some((goal, exact)) = self.pickup_goal(g, hc).or_else(|| self.goal(g)) else {
            return Input::default();
        };
        let dir = match exact {
            Some(p) if here == goal => Self::approach(hc, p),
            _ => self.walk_to(g, here, goal),
        };
        Input {
            dir,
            ..Input::default()
        }
    }

    /// Sidestep a fireball that will pass through the hero.
    fn dodge(&self, g: &Game, hc: (f32, f32)) -> Option<Dir> {
        g.shots()
            .iter()
            .filter(|s| s.kind == ProjKind::Fireball)
            .find_map(|s| {
                let (rx, ry) = (s.x - hc.0, s.y - hc.1);
                let v2 = s.vx * s.vx + s.vy * s.vy;
                let t = -(rx * s.vx + ry * s.vy) / v2;
                if !(0.0..30.0).contains(&t) {
                    return None;
                }
                let (cx, cy) = (rx + s.vx * t, ry + s.vy * t);
                (cx.hypot(cy) < 13.0).then(|| {
                    let (px, py) = perp(s.vx, s.vy, cx, cy);
                    Dir::from_vec(px, py)
                })
            })
    }

    fn hunting(&self, g: &Game) -> bool {
        g.progress().sword_taken
            && match g.room() {
                RoomId::D2 => !g.progress().key_taken,
                RoomId::D3 => true,
                _ => false,
            }
    }

    fn approach(from: (f32, f32), to: (f32, f32)) -> Option<Dir> {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        (dx.abs() > 1.0 || dy.abs() > 1.0).then(|| Dir::from_vec(dx, dy))
    }

    fn pickup_goal(&self, g: &Game, hc: (f32, f32)) -> Option<(Node, Option<(f32, f32)>)> {
        let h = g.hero();
        g.pickups()
            .iter()
            .filter(|p| match p.kind {
                PickupKind::Heart => h.hp < h.max_hp,
                PickupKind::Gem => true,
                _ => false,
            })
            .filter(|p| ((p.x + 8.0 - hc.0).powi(2) + (p.y + 8.0 - hc.1).powi(2)).sqrt() < 72.0)
            .map(|p| {
                (
                    (g.room(), (p.x as i32 + 8) / TILE, (p.y as i32 + 8) / TILE),
                    Some((p.x + 8.0, p.y + 8.0)),
                )
            })
            .next()
    }

    fn goal(&self, g: &Game) -> Option<(Node, Option<(f32, f32)>)> {
        let p = g.progress();
        let special = g
            .pickups()
            .iter()
            .find(|k| {
                matches!(
                    k.kind,
                    PickupKind::Sword | PickupKind::Key | PickupKind::Relic
                )
            })
            .map(|k| {
                (
                    (g.room(), (k.x as i32 + 8) / TILE, (k.y as i32 + 8) / TILE),
                    Some((k.x + 8.0, k.y + 8.0)),
                )
            });
        if special.is_some() {
            return special;
        }
        if !p.sword_taken {
            return Some(((RoomId::Cave, 7, 6), None));
        }
        if !p.key_taken {
            return Some(((RoomId::D2, 7, 5), None));
        }
        Some(((RoomId::D3, 8, 9), None))
    }

    fn walk_to(&self, g: &Game, here: Node, goal: Node) -> Option<Dir> {
        if here == goal {
            return None;
        }
        let next = first_step(g, here, goal)?;
        let h = g.hero();
        let (cx, cy) = (h.x + 8.0, h.y + 12.0);
        let (mx, my) = ((here.1 * TILE + 8) as f32, (here.2 * TILE + 8) as f32);
        let (ex, ey) = (cx - mx, cy - my);
        Some(if next.horizontal() && ey.abs() > 2.5 {
            if ey > 0.0 { Dir::Up } else { Dir::Down }
        } else if !next.horizontal() && ex.abs() > 2.5 {
            if ex > 0.0 { Dir::Left } else { Dir::Right }
        } else {
            next
        })
    }

    fn fight(&self, g: &Game, e: &crate::actors::Enemy, dist: f32) -> Input {
        let h = g.hero();
        let hc = (h.x + 8.0, h.y + 8.0);
        let ec = e.center();
        let (dx, dy) = (ec.0 - hc.0, ec.1 - hc.1);
        let size = e.kind.stats().size;
        let reach = 22.0 + (size - 16.0) / 2.0;
        let sword = g.progress().sword_taken;
        let towards = |d: f32, neg: Dir, pos: Dir| if d < 0.0 { neg } else { pos };
        let lined = LINE_UP + (size - 16.0) / 4.0;
        let (along, across, face) = if dx.abs() <= dy.abs() {
            (dy.abs(), dx.abs(), towards(dy, Dir::Up, Dir::Down))
        } else {
            (dx.abs(), dy.abs(), towards(dx, Dir::Left, Dir::Right))
        };
        if dist > 40.0 + (size - 16.0) / 2.0 {
            let here = (g.room(), h.tile().0, h.tile().1);
            let goal = (
                g.room(),
                (ec.0 as i32).div_euclid(TILE),
                (ec.1 as i32).div_euclid(TILE),
            );
            return Input {
                dir: self.walk_to(g, here, goal),
                ..Input::default()
            };
        }
        if across > lined {
            let slide = if face.horizontal() {
                towards(dy, Dir::Up, Dir::Down)
            } else {
                towards(dx, Dir::Left, Dir::Right)
            };
            return Input {
                dir: Some(slide),
                ..Input::default()
            };
        }
        if along <= reach && sword {
            return Input {
                dir: Some(face),
                sword: true,
                ..Input::default()
            };
        }
        Input {
            dir: Some(face),
            ..Input::default()
        }
    }
}
