use super::{Game, HeroAction, Mode, Pickup, PickupKind};
use crate::actors::{Enemy, EnemyKind, ProjKind, Projectile};
use crate::world::{Dir, ROOM_H, ROOM_W, RoomId, TILE, tile_at};

/// Direct state setters. Normal play never calls them. They exist for tests and the snapshot tool.
impl Game {
    /// A game already in `Playing`, standing in the middle of `room`.
    pub fn new_at(seed: u64, room: RoomId) -> Game {
        let mut g = Game::new(seed);
        g.mode = Mode::Playing;
        let free = (0..ROOM_H)
            .flat_map(|ty| (0..ROOM_W).map(move |tx| (tx, ty)))
            .filter(|&(tx, ty)| {
                !tile_at(room, tx, ty, &g.progress).solid()
                    && room.def().portal_at(tx, ty).is_none()
            })
            .min_by_key(|&(tx, ty)| (tx - 8).pow(2) + (ty - 5).pow(2))
            .unwrap_or((8, 5));
        g.teleport(room, (free.0 * TILE) as f32, (free.1 * TILE - 4) as f32);
        g
    }

    pub fn teleport(&mut self, room: RoomId, x: f32, y: f32) {
        self.load_room(room);
        self.hero.x = x;
        self.hero.y = y;
        self.hero.action = HeroAction::Free;
    }

    /// Move the hero within the current room without reloading it.
    pub fn place_hero(&mut self, x: f32, y: f32) {
        self.hero.x = x;
        self.hero.y = y;
    }

    pub fn grant_sword(&mut self) {
        self.progress.sword_taken = true;
    }
    pub fn grant_keys(&mut self, n: u32) {
        self.keys += n;
    }
    pub fn set_hp(&mut self, half_hearts: u32) {
        self.hero.hp = half_hearts.min(self.hero.max_hp);
    }
    pub fn face(&mut self, d: Dir) {
        self.hero.facing = d;
    }
    pub fn clear_enemies(&mut self) {
        self.enemies.clear();
    }
    pub fn add_enemy(&mut self, kind: EnemyKind, tx: i32, ty: i32) {
        self.enemies.push(Enemy::new(kind, tx, ty));
    }
    pub fn add_pickup(&mut self, kind: PickupKind, x: f32, y: f32) {
        self.pickups.push(Pickup::new(kind, x, y));
    }
    pub fn add_shot(&mut self, kind: ProjKind, x: f32, y: f32, vx: f32, vy: f32) {
        self.shots.push(Projectile { kind, x, y, vx, vy });
    }
}
