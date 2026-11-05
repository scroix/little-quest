use super::SpriteId;

pub struct Art {
    pub rows: Vec<String>,
    pub pal: Vec<(char, u32)>,
}

fn rows(r: &[&str]) -> Vec<String> {
    r.iter().map(|s| s.to_string()).collect()
}

fn mirror(half: &[&str]) -> Vec<String> {
    half.iter()
        .map(|s| format!("{s}{}", s.chars().rev().collect::<String>()))
        .collect()
}

fn flip_y(mut r: Vec<String>) -> Vec<String> {
    r.reverse();
    r
}

fn transpose(r: &[String]) -> Vec<String> {
    let grid: Vec<Vec<char>> = r.iter().map(|s| s.chars().collect()).collect();
    (0..grid[0].len())
        .map(|x| grid.iter().map(|row| row[x]).collect())
        .collect()
}

fn recolor(r: Vec<String>, from: &str, to: char) -> Vec<String> {
    r.into_iter()
        .map(|s| {
            s.chars()
                .map(|c| if from.contains(c) { to } else { c })
                .collect()
        })
        .collect()
}

fn pal(p: &[(char, u32)]) -> Vec<(char, u32)> {
    p.to_vec()
}

const OUT: u32 = 0x1c1424;

pub fn art(id: SpriteId) -> Art {
    use SpriteId::*;
    match id {
        HeroDown(f) => hero(hero_down(f)),
        HeroUp(f) => hero(hero_up(f)),
        HeroSide(f) => hero(hero_side(f)),
        HeroStabDown => hero(hero_stab_down()),
        HeroStabUp => hero(hero_stab_up()),
        HeroStabSide => hero(hero_stab_side()),
        HeroHold => hero(hero_hold()),
        Sand => tile_sand(),
        Grass => tile_grass(false),
        Flower => tile_grass(true),
        Tree => tile_tree(),
        Boulder => tile_boulder(),
        Water(f) => tile_water(f),
        Bridge => tile_bridge(),
        Bush => tile_bush(),
        CaveMouth => tile_cave_mouth(),
        DungeonDoor => tile_dungeon_door(),
        DungeonFloor => tile_floor(DUNGEON),
        DungeonWall => tile_wall(DUNGEON),
        CaveFloor => tile_floor(CAVE),
        CaveWall => tile_wall(CAVE),
        Door => tile_door(),
        LockedDoor => tile_locked(),
        Black => Art {
            rows: vec!["k".repeat(16); 16],
            pal: pal(&[('k', 0x000000)]),
        },
        SpitterDown(f) => spitter_down(f),
        SpitterUp(f) => spitter_up(f),
        SpitterSide(f) => spitter_side(f),
        Bat(f) => bat(f),
        Skeleton(f) => skeleton(f),
        Gnasher(f) => gnasher(f),
        SwordItem => sword(),
        BladeDown => blade_down(),
        BladeUp => Art {
            rows: flip_y(blade_down().rows),
            ..blade_down()
        },
        BladeSide => Art {
            rows: transpose(&blade_down().rows),
            ..blade_down()
        },
        BeamV => beam_v(),
        BeamH => Art {
            rows: transpose(&beam_v().rows),
            ..beam_v()
        },
        Stone => stone(),
        Fireball(f) => fireball(f),
        HeartFull => heart(0),
        HeartHalf => heart(1),
        HeartEmpty => heart(2),
        Gem => gem(),
        Key => key(),
        Relic => relic(),
        Mentor => mentor(),
        Flame(f) => flame(f),
        Poof(f) => poof(f),
        Glyph(_) => unreachable!("glyphs come from the font"),
    }
}

fn hero(rows: Vec<String>) -> Art {
    Art {
        rows,
        pal: pal(&[
            ('o', OUT),
            ('h', 0x10a898),
            ('s', 0xf8c898),
            ('b', 0xa8602c),
        ]),
    }
}

fn hero_down(f: u8) -> Vec<String> {
    let mut r = rows(&[
        "................",
        ".....oooooo.....",
        "....ohhhhhho....",
        "...ohhhhhhhho...",
        "...ohhhhhhhho...",
        "...ohssssssho...",
        "...osossssoso...",
        "...ohssssssho...",
        "....ohhhhhho....",
        "..obbboooooso...",
        ".obbbbohhhosso..",
        ".obbbbohhhosso..",
        "..obboohhhooo...",
    ]);
    r.extend(rows(if f == 0 {
        &["....obbooobbo...", "....oooo.obbbo..", "..........oooo.."]
    } else {
        &["...obbooobbo....", "..obbbo.oooo....", "..oooo.........."]
    }));
    r
}

fn hero_up(f: u8) -> Vec<String> {
    let mut r = rows(&[
        "................",
        ".....oooooo.....",
        "....ohhhhhho....",
        "...ohhhhhhhho...",
        "...ohhhhhhhho...",
        "...ohhhhhhhho...",
        "...ohhhhhhhho...",
        "...ohbbbbbbho...",
        "....ohhhhhho....",
        "..osoobbbbooso..",
        "..osobbbbbboso..",
        "...ooobbbbooo...",
        "....ohbbbbho....",
    ]);
    r.extend(rows(if f == 0 {
        &["....obbooobbo...", "....oooo.obbbo..", "..........oooo.."]
    } else {
        &["...obbooobbo....", "..obbbo.oooo....", "..oooo.........."]
    }));
    r
}

fn hero_side(f: u8) -> Vec<String> {
    let mut r = rows(&[
        "................",
        ".....oooooo.....",
        "....ohhhhhhoo...",
        "...ohhhhhhhhho..",
        "...ohhhhhhhhso..",
        "...ohhhhhsssso..",
        "...ohhhhhsosso..",
        "...ohhhhhsssso..",
        "....ohhhhhhoo...",
        "....oohhhhoo....",
        "....obbbbhso....",
        "....obbbbhso....",
        "....oobbbhoo....",
    ]);
    r.extend(rows(if f == 0 {
        &["....ohhohhho....", "....obbo.obbo...", "....oooo.oooo..."]
    } else {
        &["....ohhhohho....", ".....obbbbo.....", ".....oooooo....."]
    }));
    r
}

fn hero_stab_down() -> Vec<String> {
    let mut r = hero_down(0);
    r[9] = "..obbboooooso...".into();
    r[10] = ".obbbbohhhosso..".into();
    r[11] = ".obbbbohhosssso.".into();
    r[12] = "..obboohhhosso..".into();
    r
}

fn hero_stab_up() -> Vec<String> {
    let mut r = hero_up(0);
    r[9] = "..osoobbbbooso..".into();
    r[8] = "....ohhhhhhosso.".into();
    r
}

fn hero_stab_side() -> Vec<String> {
    let mut r = hero_side(0);
    r[10] = "....obbbbhsssso.".into();
    r[11] = "....obbbbhoooo..".into();
    r
}

fn hero_hold() -> Vec<String> {
    let mut r = hero_down(0);
    let sides = |row: &str| format!(".os{}so.", &row[3..13]);
    for row in &mut r[3..8] {
        *row = sides(row);
    }
    r[0] = ".os..........so.".into();
    r[1] = ".os..oooooo..so.".into();
    r[2] = ".os.ohhhhhho.so.".into();
    r[8] = ".os.ohhhhhho.so.".into();
    r[9] = "..ooohhhhhhooo..".into();
    r[10] = "...ohhbbbbhho...".into();
    r[11] = "...ohhbbbbhho...".into();
    r[12] = "....oohhhhoo....".into();
    r
}

fn tile_sand() -> Art {
    Art {
        rows: rows(&[
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaa",
            "aaabaaaaaaaaaaaa",
            "aaaaaaaaaacaaaaa",
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaabaaa",
            "aaaaabaaaaaaaaaa",
            "aaaaaaaaaaaaaaaa",
            "aacaaaaaaaaaaaaa",
            "aaaaaaaaaaabaaaa",
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaacaaaaaaa",
            "aabaaaaaaaaaaaaa",
            "aaaaaaaaaaaaabaa",
            "aaaaaaaaaaaaaaaa",
            "aaaaabaaaaaacaaa",
        ]),
        pal: pal(&[('a', 0xe4c478), ('b', 0xc4a058), ('c', 0xf4e0a4)]),
    }
}

fn tile_grass(flowers: bool) -> Art {
    let mut r = rows(&[
        "aaaaaaaaaaaaaaaa",
        "aaabaaaaaaaaaaaa",
        "aababaaaaacaaaaa",
        "aaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaabaaa",
        "aaaaacaaaaaabaaa",
        "aabaaaaaaaaaaaaa",
        "aababaaaaaaaaaaa",
        "aaaaaaaaaacaaaaa",
        "aaaaaaaaaaaaaaaa",
        "aaaaabaaaaaaabaa",
        "aaaaababaaaaabaa",
        "aaaaaaaaaaaaaaaa",
        "aacaaaaaaabaaaaa",
        "aaaaaaaaaababaaa",
        "aaaaaaaaaaaaaaaa",
    ]);
    if flowers {
        r[2] = "aababaaaaapaaaaa".into();
        r[3] = "aaaaaaaaaapwpaaa".into();
        r[4] = "aaaaaaaaaaapaaaa".into();
        r[5] = "aaaaaaaaaaaabaaa".into();
        r[8] = "aapaaaaaaaaaaaaa".into();
        r[9] = "aapwpaaaaaaaaaaa".into();
        r[10] = "aaapaaaaaaaaaaaa".into();
        r[12] = "aaaaaaaaaaaaaaaa".into();
        r[13] = "aaaaaaapaaaaaaaa".into();
        r[14] = "aaaaaapwpaaaaaaa".into();
        r[15] = "aaaaaaapaaaaaaaa".into();
    }
    Art {
        rows: r,
        pal: if flowers {
            pal(&[
                ('a', 0x58b038),
                ('b', 0x388028),
                ('p', 0xf86898),
                ('w', 0xfff8d8),
            ])
        } else {
            pal(&[('a', 0x58b038), ('b', 0x388028), ('c', 0x80d058)])
        },
    }
}

fn tile_tree() -> Art {
    Art {
        rows: rows(&[
            "oooooooaaooooooo",
            "ooooaaaaaaaaoooo",
            "oooaaccaaaaaaooo",
            "ooaaccccaaaaaaoo",
            "ooaacccaaaaaaaoo",
            "oaaaccaaaaaaaaao",
            "oaaaaaaaaaaaaaao",
            "oaaaaaaaaaaaaaao",
            "ooaaaaaaaaaaaaoo",
            "oooaaaaaaaaaaooo",
            "oooooaaaaaaooooo",
            "oooooottttoooooo",
            "ooooootttttooooo",
            "ooooootttttooooo",
            "oooootttttttoooo",
            "oooooooooooooooo",
        ]),
        pal: pal(&[
            ('o', 0x0c3410),
            ('a', 0x1f8a2a),
            ('c', 0x5cc040),
            ('t', 0x8a5424),
        ]),
    }
}

fn tile_boulder() -> Art {
    Art {
        rows: rows(&[
            "aaaaaaaaaaaaaaaa",
            "aaaaooooooooaaaa",
            "aaooclllmmmmooaa",
            "aoclllllmmmmmmoa",
            "aolllllmmmmmmmoa",
            "aolllmmmmmmmmmoa",
            "aoclmmmmmmmmmmoa",
            "aommmmmmmmmmmmoa",
            "aommmmmmmmmmmdoa",
            "aommmmmmmmmmddoa",
            "aoddmmmmmmmdddoa",
            "aaodddddddddddoa",
            "aaaooddddddooaaa",
            "aaaaaoooooooaaaa",
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaa",
        ]),
        pal: pal(&[
            ('a', 0xe4c478),
            ('o', 0x34343c),
            ('l', 0xd0d8e0),
            ('m', 0x9098a8),
            ('d', 0x606878),
            ('c', 0xf0f4f8),
        ]),
    }
    .with_four(&[('c', 'l')])
}

impl Art {
    fn with_four(mut self, merge: &[(char, char)]) -> Art {
        for (from, to) in merge {
            self.rows = recolor(std::mem::take(&mut self.rows), &from.to_string(), *to);
            self.pal.retain(|(k, _)| k != from);
        }
        if self.pal.len() > 4 {
            self.rows = recolor(std::mem::take(&mut self.rows), "d", 'm');
            self.pal.retain(|(k, _)| *k != 'd');
        }
        self
    }
}

fn tile_water(f: u8) -> Art {
    let mut r = vec!["a".repeat(16); 16];
    let marks: &[(usize, usize)] = if f == 0 {
        &[
            (2, 2),
            (2, 3),
            (2, 4),
            (3, 9),
            (3, 10),
            (3, 11),
            (8, 11),
            (8, 12),
            (8, 13),
            (9, 4),
            (9, 5),
            (9, 6),
            (13, 9),
            (13, 10),
            (13, 11),
        ]
    } else {
        &[
            (2, 5),
            (2, 6),
            (2, 7),
            (3, 12),
            (3, 13),
            (3, 14),
            (8, 1),
            (8, 2),
            (8, 3),
            (9, 8),
            (9, 9),
            (9, 10),
            (13, 3),
            (13, 4),
            (13, 5),
        ]
    };
    for &(y, x) in marks {
        r[y].replace_range(x..x + 1, "c");
        if y + 1 < 16 {
            r[y + 1].replace_range(x..x + 1, "b");
        }
    }
    Art {
        rows: r,
        pal: pal(&[('a', 0x2848d8), ('b', 0x1c34b0), ('c', 0x78a8f8)]),
    }
}

fn tile_bridge() -> Art {
    Art {
        rows: rows(&[
            "wwwwwwwwwwwwwwww",
            "wwwwwwwwwwwwwwww",
            "oooooooooooooooo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "opppdpppdpppdppo",
            "oooooooooooooooo",
            "wwwwwwwwwwwwwwww",
            "wwwwwwwwwwwwwwww",
        ]),
        pal: pal(&[
            ('w', 0x2848d8),
            ('o', 0x3c2008),
            ('p', 0xd09048),
            ('d', 0x8c5a24),
        ]),
    }
}

fn tile_bush() -> Art {
    Art {
        rows: rows(&[
            "................",
            "................",
            "......oooo......",
            ".....oaaccao....",
            "....oaaccaaao...",
            "...oaaccaaaaao..",
            "..oaaaaaaaaaaao.",
            "..oaaaaaaaaaaao.",
            "..oaaaaaaaaaaao.",
            "..oaaaaaaaaaaao.",
            "...oaaaaaaaaao..",
            "....ooooooooo...",
            "................",
            "................",
            "................",
            "................",
        ]),
        pal: pal(&[('o', 0x0c3410), ('a', 0x1f8a2a), ('c', 0x5cc040)]),
    }
}

fn tile_cave_mouth() -> Art {
    Art {
        rows: rows(&[
            "rrrrrrlrrrrrrrrr",
            "rrlrrrrrrrrlrrrr",
            "rrrrrroooooorrrr",
            "rrrrooookkkoorrr",
            "rrrookkkkkkkoorr",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
            "rlookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkolr",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
            "rrookkkkkkkkkoor",
        ]),
        pal: pal(&[
            ('r', 0x9a8466),
            ('l', 0xc8b490),
            ('o', 0x40301c),
            ('k', 0x08040c),
        ]),
    }
}

fn tile_dungeon_door() -> Art {
    let mut r = rows(&[
        "oooooooooooooooo",
        "osssssssssssssso",
        "osddddddddddddso",
        "osddkkkkkkkkddso",
    ]);
    for _ in 4..16 {
        r.push("osdkkkkkkkkkkdso".into());
    }
    Art {
        rows: r,
        pal: pal(&[
            ('o', 0x101040),
            ('s', 0x86a0f0),
            ('d', 0x4a62c8),
            ('k', 0x000000),
        ]),
    }
}

struct Style {
    out: u32,
    mid: u32,
    light: u32,
    floor: u32,
    floor_hi: u32,
}

const DUNGEON: Style = Style {
    out: 0x101040,
    mid: 0x4a62c8,
    light: 0x86a0f0,
    floor: 0x1a2268,
    floor_hi: 0x2c3a94,
};
const CAVE: Style = Style {
    out: 0x2c1808,
    mid: 0x7c5430,
    light: 0xa88050,
    floor: 0x4c3018,
    floor_hi: 0x64422a,
};

fn tile_wall(s: Style) -> Art {
    let a = "lmmmmmmolmmmmmmo";
    let b = "mmmolmmmmmmmolmm";
    let m = "oooooooooooooooo";
    Art {
        rows: rows(&[a, a, a, m, b, b, b, m, a, a, a, m, b, b, b, m]),
        pal: pal(&[('o', s.out), ('m', s.mid), ('l', s.light)]),
    }
}

fn tile_floor(s: Style) -> Art {
    let mut r = vec!["dfffffffffffffff".to_string(); 16];
    r[0] = "d".repeat(16);
    r[4].replace_range(4..6, "ll");
    r[11].replace_range(11..13, "ll");
    r[12].replace_range(3..4, "l");
    Art {
        rows: r,
        pal: pal(&[('d', s.out), ('f', s.floor), ('l', s.floor_hi)]),
    }
}

fn tile_door() -> Art {
    let mut r = rows(&["lmmmmmmolmmmmmmo", "oooooooooooooooo"]);
    for _ in 2..16 {
        r.push("mmokkkkkkkkkkomm".into());
    }
    Art {
        rows: r,
        pal: pal(&[
            ('o', DUNGEON.out),
            ('m', DUNGEON.mid),
            ('l', DUNGEON.light),
            ('k', 0x0a0e30),
        ]),
    }
}

fn tile_locked() -> Art {
    let mut r = rows(&["mmmmmmmommmmmmmo", "oooooooooooooooo", "mmoyyyyyyyyyyomm"]);
    for y in 3..15 {
        r.push(
            match y {
                8 | 9 => "mmoydddoodddyomm",
                10 => "mmoydddoodddyomm",
                _ => "mmoyddddddddyomm",
            }
            .into(),
        );
    }
    r.push("mmoyyyyyyyyyyomm".into());
    r[10] = "mmoyddddoodddyomm"
        .replace("oodddy", "odddy")
        .chars()
        .take(16)
        .collect();
    Art {
        rows: r,
        pal: pal(&[
            ('o', DUNGEON.out),
            ('m', DUNGEON.mid),
            ('y', 0xd09830),
            ('d', 0x7c4c14),
        ]),
    }
}

fn spitter_down(f: u8) -> Art {
    let mut h = rows(&[
        "........", "........", "....oooo", "..oorrrr", ".orrrrrr", ".orrrwor", ".orrrrrr",
        ".orrrllo", ".orrllll", "..orrlll", "..oorrrr", "...ooooo",
    ]);
    h.extend(rows(if f == 0 {
        &["..orro..", ".oro.oo.", "........", "........"]
    } else {
        &["...orro.", "..oo.oro", "........", "........"]
    }));
    Art {
        rows: mirror(&h.iter().map(String::as_str).collect::<Vec<_>>()),
        pal: spitter_pal(),
    }
}

fn spitter_pal() -> Vec<(char, u32)> {
    pal(&[
        ('o', OUT),
        ('r', 0xd05838),
        ('l', 0xf8b888),
        ('w', 0xfffff0),
    ])
}

fn spitter_up(f: u8) -> Art {
    let mut a = spitter_down(f);
    let top = [
        "........", "........", "....oooo", "..oorrrr", ".orrrrrr", ".orrrrrr", ".orrrrrr",
        ".orrrrrr", ".orrrrrr", "..orrrrr", "..oorrrr", "...ooooo",
    ];
    let mut h: Vec<String> = top.iter().map(|s| s.to_string()).collect();
    h[3] = "..oorrrr".into();
    h[6] = ".orrllrr".into();
    h[7] = ".orrllrr".into();
    let tail: Vec<String> = a.rows[12..].iter().map(|s| s[..8].to_string()).collect();
    h.extend(tail);
    a.rows = mirror(&h.iter().map(String::as_str).collect::<Vec<_>>());
    a
}

fn spitter_side(f: u8) -> Art {
    let mut r = rows(&[
        "................",
        "................",
        ".....oooooo.....",
        "...ooorrrrroo...",
        "..orrrrrrrrrro..",
        "..orrrrrrrwoorl.",
        "..orrrrrrrrrooo.",
        "..orrllrrrrrooo.",
        "..orllllrrrrro..",
        "...orllllrrrro..",
        "....oorrrrrroo..",
        ".....oooooooo...",
    ]);
    r.extend(rows(if f == 0 {
        &[
            "....orro.orro...",
            "...oo.oo..oo....",
            "................",
            "................",
        ]
    } else {
        &[
            "....oorr.orro...",
            "....oo.orro.oo..",
            "................",
            "................",
        ]
    }));
    Art {
        rows: r,
        pal: spitter_pal(),
    }
}

fn bat(f: u8) -> Art {
    let h: &[&str] = if f == 0 {
        &[
            "........",
            "........",
            "o.......",
            "oo....oo",
            "opo..opp",
            "oppooppp",
            "opppopop",
            "oppppppp",
            ".opppppl",
            "..oopppp",
            "....opppl",
            "......oo",
            "........",
            "........",
            "........",
            "........",
        ]
    } else {
        &[
            "........",
            "........",
            "........",
            "........",
            "........",
            "...ooooo",
            "..opppop",
            "oooppppp",
            "oppppppl",
            "opppppr.",
            "oppoopppl",
            ".oo.oppp",
            "......oo",
            "........",
            "........",
            "........",
        ]
    };
    let h: Vec<&str> = h.iter().map(|s| &s[..s.len().min(8)]).collect();
    let h: Vec<String> = h.iter().map(|s| format!("{s:.<8}")).collect();
    Art {
        rows: mirror(&h.iter().map(String::as_str).collect::<Vec<_>>()),
        pal: pal(&[
            ('o', OUT),
            ('p', 0x7a44b0),
            ('l', 0xb08ae8),
            ('r', 0xf84848),
        ]),
    }
}

fn skeleton(f: u8) -> Art {
    let mut h = rows(&[
        "........", "....oooo", "...owwww", "..owwwww", "..owoowo", "..owwwww", "...owwoo",
        "....owww", "..oowogg", ".owwwwog", "..oowogg", "....wowg", "....owww",
    ]);
    h.extend(rows(if f == 0 {
        &["...owgow", "..owg.ow", "..oo..oo"]
    } else {
        &["...owwow", "...oww.o", "...oo..o"]
    }));
    Art {
        rows: mirror(&h.iter().map(String::as_str).collect::<Vec<_>>()),
        pal: pal(&[('o', OUT), ('w', 0xf0ecd8), ('g', 0x9a92a8)]),
    }
}

fn gnasher(f: u8) -> Art {
    let top = [
        "................",
        "................",
        "..o.............",
        ".ogo..........oo",
        ".ogo.....ooooogg",
        ".ogoo..ooggggggg",
        "..ogoooggggggggg",
        "...oggggggggggoo",
        "...ogggggggggowo",
        "..oggggggggggowo",
        "..ogggggggggggoo",
        ".ogggggggggggggg",
        ".ogggggggggggggg",
        ".ogggggoooooooo.",
        ".oggggoowwowwow.",
        ".oggggowwowwowwo",
    ];
    let bot_closed = [
        ".oggggoooooooooo",
        ".oggggggggggggg.",
        ".ogggggggggggggg",
        "..oggggggggggggg",
        "..oogggggggggggg",
        "...oooggggggggg.",
        ".....ooggggggggg",
        "......ooooggoggg",
        ".......ggoooogg.",
        "........ogg..og.",
        "........oogg.oo.",
        ".......oooooooo.",
        "................",
        "................",
        "................",
        "................",
    ];
    let bot_open = [
        ".oggggowrrrrrrrr",
        ".oggggowrrrrrrrr",
        ".ogggogrrrrrrrrr",
        "..oggooorrrrrrrr",
        "..oogowwowwowwow",
        "...ooogggggggggg",
        ".....oogggggggg.",
        "......ooggggggog",
        ".......ggoooogg.",
        "........ogg..og.",
        "........oogg.oo.",
        ".......oooooooo.",
        "................",
        "................",
        "................",
        "................",
    ];
    let mut r: Vec<&str> = top.to_vec();
    r.extend(if f == 0 {
        &bot_closed[..]
    } else {
        &bot_open[..]
    });
    Art {
        rows: mirror(&r),
        pal: pal(&[
            ('o', 0x14201c),
            ('g', 0x48b048),
            ('w', 0xfafafa),
            ('r', 0xd03030),
        ]),
    }
}

fn blade_down() -> Art {
    Art {
        rows: mirror(&[
            "......og", "......og", "...ooooo", "...ogggg", "...ooooo", "......ow", "......ow",
            "......ow", "......ow", "......ow", "......ow", "......ow", "......ow", ".....oow",
            "......oo", "........",
        ]),
        pal: pal(&[('o', OUT), ('g', 0xe0a830), ('w', 0xf4f8ff)]),
    }
}

fn sword() -> Art {
    let mut a = blade_down();
    a.rows = flip_y(a.rows);
    a
}

fn beam_v() -> Art {
    Art {
        rows: mirror(&[
            "........", ".......w", "......cw", "......cw", "......cw", "......cw", "......cw",
            "......cw", "......cw", "......cw", "......cw", "......cw", "......cw", "......cw",
            ".......w", "........",
        ]),
        pal: pal(&[('w', 0xfafaff), ('c', 0x60d8f8)]),
    }
}

fn stone() -> Art {
    Art {
        rows: mirror(&[
            "........", "........", "........", "........", "........", "....oooo", "...ogggl",
            "..ogglll", "..ogggll", "...ogggg", "....oooo", "........", "........", "........",
            "........", "........",
        ]),
        pal: pal(&[('o', OUT), ('g', 0x8890a0), ('l', 0xd0d8e0)]),
    }
}

fn fireball(f: u8) -> Art {
    let (a, b) = if f == 0 { ('y', 'r') } else { ('r', 'y') };
    let half = [
        "........".to_string(),
        "........".to_string(),
        "........".to_string(),
        "......oo".to_string(),
        format!("....oo{b}{b}"),
        format!("...o{b}{b}{b}{b}"),
        format!("..o{b}{b}{a}{a}{a}"),
        format!("..o{b}{a}{a}w{a}"),
        format!("..o{b}{a}{a}{a}{a}"),
        format!("..o{b}{b}{a}{a}{a}"),
        format!("...o{b}{b}{b}{b}"),
        format!("....oo{b}{b}"),
        "......oo".to_string(),
        "........".to_string(),
        "........".to_string(),
        "........".to_string(),
    ];
    Art {
        rows: mirror(&half.iter().map(String::as_str).collect::<Vec<_>>()),
        pal: pal(&[
            ('o', 0x501008),
            ('r', 0xe83820),
            ('y', 0xf8c830),
            ('w', 0xfffbd0),
        ]),
    }
}

fn heart(kind: u8) -> Art {
    let full = mirror(&[
        "........", "........", "..oooo..", ".orrllro", ".orlrrrr", ".orrrrrr", ".orrrrrr",
        "..orrrrr", "...orrrr", "....orrr", ".....orr", "......or", ".......o", "........",
        "........", "........",
    ]);
    let rows = match kind {
        0 => full,
        1 => full
            .into_iter()
            .map(|s| {
                let (l, r) = s.split_at(8);
                format!("{l}{}", r.replace(['r', 'l'], "e"))
            })
            .collect(),
        _ => recolor(full, "rl", 'e'),
    };
    Art {
        rows,
        pal: pal(&[
            ('o', OUT),
            ('r', 0xe83030),
            ('l', 0xff9a9a),
            ('e', 0x3c2c40),
        ]),
    }
}

fn gem() -> Art {
    Art {
        rows: mirror(&[
            "........", "........", ".....ooo", "....obll", "...oblbb", "..oblbbb", "..oooooo",
            "..odbbbb", "...odbbb", "....odbb", ".....odb", "......oo", "........", "........",
            "........", "........",
        ]),
        pal: pal(&[
            ('o', OUT),
            ('b', 0x2c70f0),
            ('l', 0x9ad0ff),
            ('d', 0x1840a0),
        ]),
    }
}

fn key() -> Art {
    Art {
        rows: rows(&[
            "................",
            "......oooo......",
            ".....oyyyyo.....",
            "....oyyooyyo....",
            "....oyo..oyo....",
            "....oyyooyyo....",
            ".....oyyyyo.....",
            "......oyyo......",
            "......oyyo......",
            "......oyyoooo...",
            "......oyyyyyo...",
            "......oyyoyyo...",
            "......oyyo.oo...",
            "......oyyyyo....",
            ".......oooo.....",
            "................",
        ]),
        pal: pal(&[('o', OUT), ('y', 0xf8c820)]),
    }
}

fn relic() -> Art {
    Art {
        rows: mirror(&[
            "........", "..oooooo", ".oyllyyy", ".oyllyyy", ".oyyyygg", ".oyyyygg", "..oyyyyy",
            "...oyyyy", "....ooyy", ".....oyy", ".....oyy", "....oyyy", "...oyyyy", "..oooooo",
            "........", "........",
        ]),
        pal: pal(&[
            ('o', OUT),
            ('y', 0xe8b020),
            ('l', 0xfff088),
            ('g', 0x30c8f0),
        ]),
    }
}

fn mentor() -> Art {
    Art {
        rows: mirror(&[
            "........", "....oooo", "...owwww", "..owwwww", "..owwsss", "..owsoss", "..owssss",
            "..owwwww", "...owwww", "..orrwww", ".orrrrww", ".orrrrrr", ".orrrrrr", ".orrrrrr",
            ".ooooooo", "........",
        ]),
        pal: pal(&[
            ('o', OUT),
            ('w', 0xeeeeee),
            ('s', 0xf8c898),
            ('r', 0xc03838),
        ]),
    }
}

fn flame(f: u8) -> Art {
    let h: &[&str] = if f == 0 {
        &[
            ".......r", ".......r", "......rr", "......ro", ".....roo", "....rooy", "....rooy",
            "...rooyy", "...rooyw", "...rooyw", "...rooyy", "....rooy", ".....roo", "......rr",
            "........", "........",
        ]
    } else {
        &[
            "........", ".......r", ".......r", "......rr", "......ro", ".....roo", "....rooy",
            "...rooyy", "...rooyy", "..rrooyw", "..rooyyw", "..rooyyw", "...rooyy", "....rooy",
            ".....rrr", "........",
        ]
    };
    Art {
        rows: mirror(h),
        pal: pal(&[
            ('r', 0xd82818),
            ('o', 0xf88018),
            ('y', 0xf8d838),
            ('w', 0xfff8c8),
        ]),
    }
}

fn poof(f: u8) -> Art {
    let h: &[&str] = if f == 0 {
        &[
            "........", "........", "........", "........", "........", "....oooo", "...owwww",
            "..owwwgg", "..owwwgg", "...owwww", "....oooo", "........", "........", "........",
            "........", "........",
        ]
    } else {
        &[
            "........", "........", "....o...", "..o...w.", "...ow...", ".o....g.", "........",
            "o..g....", "....g..o", ".o......", "...w..o.", "..o...w.", "....o...", "........",
            "........", "........",
        ]
    };
    Art {
        rows: mirror(h),
        pal: pal(&[('o', 0x686878), ('w', 0xfafaff), ('g', 0xb8b8c8)]),
    }
}
