use crate::actors::EnemyKind;

pub const TILE: i32 = 16;
pub const ROOM_W: i32 = 16;
pub const ROOM_H: i32 = 11;
pub const PLAY_W: i32 = 256;
pub const PLAY_H: i32 = 176;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    pub fn delta(self) -> (f32, f32) {
        match self {
            Dir::Up => (0.0, -1.0),
            Dir::Down => (0.0, 1.0),
            Dir::Left => (-1.0, 0.0),
            Dir::Right => (1.0, 0.0),
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }

    pub fn horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The axis-aligned direction that best matches a vector.
    pub fn from_vec(dx: f32, dy: f32) -> Dir {
        if dx.abs() > dy.abs() {
            if dx > 0.0 { Dir::Right } else { Dir::Left }
        } else if dy > 0.0 {
            Dir::Down
        } else {
            Dir::Up
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoomId {
    Meadow,
    Maze,
    Lake,
    Pass,
    Approach,
    Shore,
    Cave,
    D1,
    D2,
    D3,
}

impl RoomId {
    pub const ALL: [RoomId; 10] = [
        RoomId::Meadow,
        RoomId::Maze,
        RoomId::Lake,
        RoomId::Pass,
        RoomId::Approach,
        RoomId::Shore,
        RoomId::Cave,
        RoomId::D1,
        RoomId::D2,
        RoomId::D3,
    ];

    pub fn def(self) -> &'static RoomDef {
        &ROOMS[self as usize]
    }
}

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.def().name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Sand,
    Grass,
    Flower,
    Tree,
    Boulder,
    Water,
    Bridge,
    Bush,
    SandBush,
    CaveMouth,
    DungeonDoor,
    DungeonWall,
    DungeonFloor,
    CaveWall,
    CaveFloor,
    Door,
    Locked,
    Flame,
    Mentor,
}

impl Tile {
    pub fn from_char(c: char) -> Tile {
        match c {
            '.' => Tile::Sand,
            'g' => Tile::Grass,
            'f' => Tile::Flower,
            'T' => Tile::Tree,
            'R' => Tile::Boulder,
            'W' => Tile::Water,
            '=' => Tile::Bridge,
            'b' => Tile::Bush,
            'B' => Tile::SandBush,
            'C' => Tile::CaveMouth,
            'D' => Tile::DungeonDoor,
            '#' => Tile::DungeonWall,
            ',' => Tile::DungeonFloor,
            'L' => Tile::Locked,
            'O' => Tile::Door,
            'X' => Tile::CaveWall,
            'x' => Tile::CaveFloor,
            'F' => Tile::Flame,
            'M' => Tile::Mentor,
            other => panic!("unknown tile char {other:?}"),
        }
    }

    pub fn solid(self) -> bool {
        matches!(
            self,
            Tile::Tree
                | Tile::Boulder
                | Tile::Water
                | Tile::Bush
                | Tile::SandBush
                | Tile::DungeonWall
                | Tile::CaveWall
                | Tile::Locked
                | Tile::Flame
                | Tile::Mentor
        )
    }

    pub fn stops_shots(self) -> bool {
        self.solid() && self != Tile::Water
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Wall,
    Scroll(RoomId),
    Wipe(RoomId, (i32, i32)),
}

#[derive(Clone, Copy, Debug)]
pub struct Portal {
    pub tile: (i32, i32),
    pub to: RoomId,
    pub arrive: (i32, i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Overworld,
    Dungeon,
    Cave,
}

impl Area {
    pub fn grid(self) -> (u8, u8) {
        match self {
            Area::Overworld => (3, 2),
            Area::Dungeon => (2, 2),
            Area::Cave => (1, 1),
        }
    }
}

pub struct RoomDef {
    pub id: RoomId,
    pub name: &'static str,
    pub area: Area,
    pub cell: (u8, u8),
    pub map: [&'static str; 11],
    /// Indexed by `Dir::index`: up, down, left, right.
    pub edges: [Edge; 4],
    pub portals: &'static [Portal],
    pub spawns: &'static [(EnemyKind, i32, i32)],
}

impl RoomDef {
    pub fn raw(&self, tx: i32, ty: i32) -> char {
        self.map[ty as usize].as_bytes()[tx as usize] as char
    }

    pub fn edge(&self, d: Dir) -> Edge {
        self.edges[d.index()]
    }

    pub fn portal_at(&self, tx: i32, ty: i32) -> Option<&Portal> {
        self.portals.iter().find(|p| p.tile == (tx, ty))
    }
}

/// Facts that outlive a room visit.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub opened: Vec<(RoomId, i32, i32)>,
    pub sword_taken: bool,
    pub key_taken: bool,
    pub boss_dead: bool,
}

impl Progress {
    pub fn is_open(&self, room: RoomId, tx: i32, ty: i32) -> bool {
        self.opened.contains(&(room, tx, ty))
    }
}

/// The tile as the player sees it, with opened locked doors turned into doorways.
pub fn tile_at(room: RoomId, tx: i32, ty: i32, progress: &Progress) -> Tile {
    let t = Tile::from_char(room.def().raw(tx, ty));
    if t == Tile::Locked && progress.is_open(room, tx, ty) {
        Tile::Door
    } else {
        t
    }
}

/// Whether a box is blocked. With `leave` set, tiles beyond an open room edge count as free.
pub fn blocked(room: RoomId, r: Rect, progress: &Progress, leave: bool) -> bool {
    let def = room.def();
    let x0 = (r.x / TILE as f32).floor() as i32;
    let x1 = ((r.x + r.w - 0.001) / TILE as f32).floor() as i32;
    let y0 = (r.y / TILE as f32).floor() as i32;
    let y1 = ((r.y + r.h - 0.001) / TILE as f32).floor() as i32;
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let inside_x = (0..ROOM_W).contains(&tx);
            let inside_y = (0..ROOM_H).contains(&ty);
            if inside_x && inside_y {
                if tile_at(room, tx, ty, progress).solid() {
                    return true;
                }
                continue;
            }
            let dir = match (inside_x, inside_y) {
                (true, false) => Some(if ty < 0 { Dir::Up } else { Dir::Down }),
                (false, true) => Some(if tx < 0 { Dir::Left } else { Dir::Right }),
                _ => None,
            };
            let open = leave && dir.is_some_and(|d| def.edge(d) != Edge::Wall);
            if !open {
                return true;
            }
        }
    }
    false
}

use EnemyKind::{Bat, Gnasher, Skeleton, Spitter};

use Edge::{Scroll, Wall, Wipe};
use RoomId::*;

pub static ROOMS: [RoomDef; 10] = [
    RoomDef {
        id: Meadow,
        name: "meadow",
        area: Area::Overworld,
        cell: (1, 1),
        map: [
            "TTRRRRRggRRTTTTT",
            "TfgRCRgggggfgggT",
            "TggggggfgggggbgT",
            "TgbgggggggggggfT",
            "Tgggfgggbggggggg",
            "gggggggggggfgggg",
            "gggbgggggggggggT",
            "TgfggggggggggbgT",
            "TgggggfggggggggT",
            "TbgggggggggggbgT",
            "TTTTbTTTTTbTTTTT",
        ],
        edges: [Scroll(Pass), Wall, Scroll(Maze), Scroll(Shore)],
        portals: &[Portal {
            tile: (4, 1),
            to: Cave,
            arrive: (120, 140),
        }],
        spawns: &[(Spitter, 11, 3), (Spitter, 3, 8)],
    },
    RoomDef {
        id: Maze,
        name: "maze",
        area: Area::Overworld,
        cell: (0, 1),
        map: [
            "TTTggTTTTTTTTTTT",
            "TggggggTggggggbT",
            "TgTTTTgTgTTTTgTT",
            "TgTggggggTggTggT",
            "TgTgTTTTTTgTTgTT",
            "TggggggggggTgggg",
            "TgTTTTTTTTgTgggg",
            "TgggggggggggTggT",
            "TgTTTTTTTTgTTgTT",
            "TgggggggbgggggfT",
            "TTTTTTTTTTTTTTTT",
        ],
        edges: [Scroll(Lake), Wall, Wall, Scroll(Meadow)],
        portals: &[],
        spawns: &[(Spitter, 3, 5), (Spitter, 10, 7), (Bat, 6, 9)],
    },
    RoomDef {
        id: Lake,
        name: "lake",
        area: Area::Overworld,
        cell: (0, 0),
        map: [
            "TTTTTTTTTTTTTTTT",
            "TgfgWWWWWWWWgfgT",
            "TggWWWWWWWWWWggT",
            "TgbWWWWWWWWWWbgT",
            "TggWWWWWWWWWWggT",
            "TgggWWWWWWWWgggT",
            "Tgggg======ggggg",
            "TgggWWWWWWWWgggg",
            "TggggWWWWWWggggT",
            "TgfggWWWWWWgfggT",
            "TTTggTTTTTTTTTTT",
        ],
        edges: [Wall, Scroll(Maze), Wall, Scroll(Pass)],
        portals: &[],
        spawns: &[(Spitter, 2, 3), (Spitter, 13, 3), (Bat, 8, 2)],
    },
    RoomDef {
        id: Pass,
        name: "pass",
        area: Area::Overworld,
        cell: (1, 0),
        map: [
            "RRRRRRRRRRRRRRRR",
            "R..............R",
            "R.RRR....RRRR..R",
            "R.R.....R.......",
            "R.R.RRR.R.RR....",
            "R...R.R...R.R.RR",
            "....R.R.RRR.R..R",
            "....R...R...R..R",
            "RRR.RRR.R.RRR..R",
            "R.....R...R....R",
            "RRRRRRR..RRRRRRR",
        ],
        edges: [Wall, Scroll(Meadow), Scroll(Lake), Scroll(Approach)],
        portals: &[],
        spawns: &[(Spitter, 13, 2), (Spitter, 4, 9), (Bat, 6, 5), (Bat, 11, 6)],
    },
    RoomDef {
        id: Approach,
        name: "approach",
        area: Area::Overworld,
        cell: (2, 0),
        map: [
            "TRRRRR#####RRRRT",
            "TRRRRR##D##RRRRT",
            "T.....B.....B..T",
            "........R....R.T",
            "......R......R.T",
            "T.R.......R....T",
            "T......R.......T",
            "T.BB.......R..BT",
            "T..............T",
            "T.R...........RT",
            "TTTTTTTTTTT..TTT",
        ],
        edges: [Wall, Scroll(Shore), Scroll(Pass), Wall],
        portals: &[Portal {
            tile: (8, 1),
            to: D1,
            arrive: (120, 140),
        }],
        spawns: &[(Spitter, 4, 7), (Spitter, 11, 6), (Bat, 8, 5)],
    },
    RoomDef {
        id: Shore,
        name: "shore",
        area: Area::Overworld,
        cell: (2, 1),
        map: [
            "TTTTTTTTTTT..TTT",
            "T.........B..WWW",
            "T..R.........WWW",
            "T.........B..WWW",
            "...........R.WWW",
            "..R...........WW",
            "T............WWW",
            "T..B.......WWWWW",
            "T.........WWWWWW",
            "T...R...WWWWWWWW",
            "TWWWWWWWWWWWWWWW",
        ],
        edges: [Scroll(Approach), Wall, Scroll(Meadow), Wall],
        portals: &[],
        spawns: &[(Spitter, 5, 3), (Spitter, 8, 7), (Bat, 10, 5), (Bat, 4, 8)],
    },
    RoomDef {
        id: Cave,
        name: "cave",
        area: Area::Cave,
        cell: (0, 0),
        map: [
            "XXXXXXXXXXXXXXXX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxFxMxFxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XxxxxxxxxxxxxxxX",
            "XXXXXXXxxXXXXXXX",
        ],
        edges: [Wall, Wipe(Meadow, (60, 28)), Wall, Wall],
        portals: &[],
        spawns: &[],
    },
    RoomDef {
        id: D1,
        name: "d1",
        area: Area::Dungeon,
        cell: (0, 1),
        map: [
            "########L#######",
            "#,,,,,,,,,,,,,,#",
            "#,,#,,,,,,,,#,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,O",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,#,,,,,,,,#,,#",
            "#,,,,,,,,,,,,,,#",
            "#######,,#######",
        ],
        edges: [Scroll(D3), Wipe(Approach, (124, 28)), Wall, Scroll(D2)],
        portals: &[],
        spawns: &[(Skeleton, 4, 4), (Skeleton, 11, 7)],
    },
    RoomDef {
        id: D2,
        name: "d2",
        area: Area::Dungeon,
        cell: (1, 1),
        map: [
            "################",
            "#,,,,,,,,,,,,,,#",
            "#,,##,,,,,,##,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "O,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,##,,,,,,##,,#",
            "#,,,,,,,,,,,,,,#",
            "################",
        ],
        edges: [Wall, Wall, Scroll(D1), Wall],
        portals: &[],
        spawns: &[(Skeleton, 3, 3), (Skeleton, 12, 3), (Skeleton, 7, 7)],
    },
    RoomDef {
        id: D3,
        name: "d3",
        area: Area::Dungeon,
        cell: (0, 0),
        map: [
            "################",
            "#,,,,,,,,,,,,,,#",
            "#,,#,,,,,,,,#,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,,,,,,,,,,,,,#",
            "#,,#,,,,,,,,#,,#",
            "#,,,,,,,,,,,,,,#",
            "########,#######",
        ],
        edges: [Wall, Scroll(D1), Wall, Wall],
        portals: &[],
        spawns: &[(Gnasher, 9, 4)],
    },
];
