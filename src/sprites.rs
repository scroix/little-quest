mod art;
mod font;

use art::Art;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpriteId {
    HeroDown(u8),
    HeroUp(u8),
    HeroSide(u8),
    HeroStabDown,
    HeroStabUp,
    HeroStabSide,
    HeroHold,
    Sand,
    Grass,
    Flower,
    Tree,
    Boulder,
    Water(u8),
    Bridge,
    Bush,
    CaveMouth,
    DungeonDoor,
    DungeonFloor,
    DungeonWall,
    CaveFloor,
    CaveWall,
    Door,
    LockedDoor,
    Black,
    SpitterDown(u8),
    SpitterUp(u8),
    SpitterSide(u8),
    Bat(u8),
    Skeleton(u8),
    Gnasher(u8),
    SwordItem,
    BladeUp,
    BladeDown,
    BladeSide,
    BeamV,
    BeamH,
    Stone,
    Fireball(u8),
    HeartFull,
    HeartHalf,
    HeartEmpty,
    Gem,
    Key,
    Relic,
    Mentor,
    Flame(u8),
    Poof(u8),
    Glyph(char),
}

pub struct SpriteImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl SpriteId {
    pub fn size(self) -> (u32, u32) {
        match self {
            SpriteId::Gnasher(_) => (32, 32),
            SpriteId::Glyph(_) => (8, 8),
            _ => (16, 16),
        }
    }

    /// Every distinct sprite, for the contact sheet and for tests.
    pub fn all() -> Vec<SpriteId> {
        use SpriteId::*;
        let mut v = vec![];
        for f in 0..2 {
            v.extend([
                HeroDown(f),
                HeroUp(f),
                HeroSide(f),
                Water(f),
                SpitterDown(f),
                SpitterUp(f),
                SpitterSide(f),
                Bat(f),
                Skeleton(f),
                Flame(f),
                Poof(f),
                Fireball(f),
            ]);
        }
        v.extend([
            HeroStabDown,
            HeroStabUp,
            HeroStabSide,
            HeroHold,
            Sand,
            Grass,
            Flower,
            Tree,
            Boulder,
            Bridge,
            Bush,
            CaveMouth,
            DungeonDoor,
            DungeonFloor,
            DungeonWall,
            CaveFloor,
            CaveWall,
            Door,
            LockedDoor,
            Black,
            SwordItem,
            BladeUp,
            BladeDown,
            BladeSide,
            BeamV,
            BeamH,
            Stone,
            HeartFull,
            HeartHalf,
            HeartEmpty,
            Gem,
            Key,
            Relic,
            Mentor,
            Gnasher(0),
            Gnasher(1),
        ]);
        v
    }
}

const OUTLINE_FLASH: u32 = 0xf8f8f8;

pub fn pixels(id: SpriteId, flip_x: bool, flash: bool) -> SpriteImage {
    let (w, h) = id.size();
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, c: u32| {
        let x = if flip_x { w - 1 - x } else { x };
        let o = ((y * w + x) * 4) as usize;
        let c = if flash { lighten(c) } else { c };
        rgba[o..o + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
    };
    if let SpriteId::Glyph(ch) = id {
        for (y, x) in font::lit(ch) {
            put(x, y, OUTLINE_FLASH);
        }
    } else {
        let Art { rows, pal } = art::art(id);
        for (y, row) in rows.iter().enumerate() {
            debug_assert_eq!(row.len() as u32, w, "{id:?} row {y}");
            for (x, ch) in row.bytes().enumerate() {
                if let Some(&(_, c)) = pal.iter().find(|(k, _)| *k == ch as char) {
                    put(x as u32, y as u32, c);
                }
            }
        }
    }
    SpriteImage { w, h, rgba }
}

fn lighten(c: u32) -> u32 {
    let mix = |v: u32| v + (255 - v) * 3 / 5;
    mix((c >> 16) & 255) << 16 | mix((c >> 8) & 255) << 8 | mix(c & 255)
}

/// Rows that do not match the sprite's declared size, as `(id, row, len)`.
pub fn malformed() -> Vec<(SpriteId, usize, usize)> {
    let mut bad = vec![];
    for id in SpriteId::all() {
        let (w, h) = id.size();
        let Art { rows, pal } = art::art(id);
        if rows.len() as u32 != h {
            bad.push((id, usize::MAX, rows.len()));
        }
        for (y, r) in rows.iter().enumerate() {
            if r.len() as u32 != w {
                bad.push((id, y, r.len()));
            }
            if let Some(c) = r
                .chars()
                .find(|c| *c != '.' && !pal.iter().any(|(k, _)| k == c))
            {
                bad.push((id, y, c as usize));
            }
        }
        if pal.len() > 4 {
            bad.push((id, usize::MAX - 1, pal.len()));
        }
    }
    bad
}

/// A glyph for each char the font knows, in text order. Unknown chars become spaces.
pub fn text_glyphs(s: &str) -> Vec<SpriteId> {
    s.chars()
        .map(|c| SpriteId::Glyph(c.to_ascii_uppercase()))
        .collect()
}
