use std::collections::{HashSet, VecDeque};

use little_quest::actors::{EnemyKind, ProjKind};
use little_quest::autoplay::Bot;
use little_quest::game::{Game, HeroAction, Input, Mode, PickupKind};
use little_quest::input::{Clock, Key, Keys, MAX_TICKS_PER_FRAME};
use little_quest::sprites::{self, SpriteId};
use little_quest::world::{Dir, Edge, Progress, ROOM_H, ROOM_W, RoomId, Tile, tile_at};

const IDLE: Input = Input {
    dir: None,
    sword: false,
    start: false,
};

fn go(d: Dir) -> Input {
    Input {
        dir: Some(d),
        ..IDLE
    }
}

fn stab() -> Input {
    Input {
        sword: true,
        ..IDLE
    }
}

fn run(g: &mut Game, input: Input, n: u32) {
    for _ in 0..n {
        g.tick(input);
    }
}

fn arena(room: RoomId, tile: (i32, i32)) -> Game {
    let mut g = Game::new_at(1, room);
    g.clear_enemies();
    g.teleport(room, (tile.0 * 16) as f32, (tile.1 * 16 - 4) as f32);
    g.clear_enemies();
    g
}

fn open(room: RoomId, tx: i32, ty: i32) -> bool {
    let t = tile_at(room, tx, ty, &Progress::default());
    !t.solid() || t == Tile::Locked
}

#[test]
fn hero_is_stopped_by_trees() {
    let mut g = arena(RoomId::Meadow, (1, 3));
    run(&mut g, go(Dir::Left), 60);
    assert!(g.hero().x >= 14.0 && g.hero().x <= 16.0, "x={}", g.hero().x);
    assert_eq!(g.mode(), Mode::Playing);
}

#[test]
fn hero_is_stopped_by_water() {
    let mut g = arena(RoomId::Lake, (2, 5));
    run(&mut g, go(Dir::Right), 90);
    let right_edge = g.hero().x + 14.0;
    assert!(
        (60.0..=64.0).contains(&right_edge),
        "right edge {right_edge}"
    );
}

#[test]
fn hero_crosses_the_bridge() {
    let mut g = arena(RoomId::Lake, (4, 6));
    run(&mut g, go(Dir::Right), 200);
    assert!(
        g.hero().x > 11.0 * 16.0 || g.room() != RoomId::Lake,
        "x={}",
        g.hero().x
    );
}

#[test]
fn corner_assist_slides_hero_into_a_one_tile_gap() {
    let mut g = arena(RoomId::Pass, (7, 9));
    g.place_hero(117.0, 140.0);
    run(&mut g, go(Dir::Up), 60);
    assert!(
        g.hero().tile().1 <= 7,
        "hero stuck at tile {:?}",
        g.hero().tile()
    );
    assert_eq!(g.hero().tile().0, 7);
}

#[test]
fn corner_assist_works_from_the_other_side_and_horizontally() {
    let mut g = arena(RoomId::Pass, (9, 9));
    g.place_hero(139.0, 140.0);
    run(&mut g, go(Dir::Up), 60);
    assert!(
        g.hero().tile().1 <= 7,
        "hero stuck at tile {:?}",
        g.hero().tile()
    );
    assert_eq!(g.hero().tile().0, 9);

    let mut g = arena(RoomId::Shore, (1, 5));
    g.place_hero(16.0, 68.0);
    run(&mut g, go(Dir::Right), 60);
    assert!(g.hero().x > 80.0, "stuck at x={}", g.hero().x);
}

#[test]
fn walking_off_an_edge_reaches_the_neighbour_at_the_opposite_edge() {
    let mut g = arena(RoomId::Meadow, (7, 0));
    g.teleport(RoomId::Meadow, 120.0, 8.0);
    g.clear_enemies();
    run(&mut g, go(Dir::Up), 12);
    assert!(matches!(
        g.mode(),
        Mode::Scrolling {
            from: RoomId::Meadow,
            to: RoomId::Pass,
            dir: Dir::Up,
            ..
        }
    ));
    run(&mut g, IDLE, 60);
    assert_eq!(g.mode(), Mode::Playing);
    assert_eq!(g.room(), RoomId::Pass);
    assert!(g.hero().y >= 150.0, "arrived at y={}", g.hero().y);
    assert_eq!(g.hero().x, 120.0);
}

#[test]
fn every_neighbour_pair_has_aligned_openings() {
    for room in RoomId::ALL {
        for d in Dir::ALL {
            let Edge::Scroll(to) = room.def().edge(d) else {
                continue;
            };
            assert_eq!(
                to.def().edge(d.opposite()),
                Edge::Scroll(room),
                "{room} {d:?} not mutual"
            );
            let span = if d.horizontal() { ROOM_H } else { ROOM_W };
            for i in 0..span {
                let (a, b) = match d {
                    Dir::Up => ((i, 0), (i, ROOM_H - 1)),
                    Dir::Down => ((i, ROOM_H - 1), (i, 0)),
                    Dir::Left => ((0, i), (ROOM_W - 1, i)),
                    Dir::Right => ((ROOM_W - 1, i), (0, i)),
                };
                assert_eq!(
                    open(room, a.0, a.1),
                    open(to, b.0, b.1),
                    "{room} {d:?} index {i}"
                );
            }
        }
    }
}

#[test]
fn every_room_is_reachable_on_foot() {
    let start = (RoomId::Meadow, 8, 7);
    let mut seen = HashSet::from([start]);
    let mut q = VecDeque::from([start]);
    while let Some((room, tx, ty)) = q.pop_front() {
        let def = room.def();
        let mut next = vec![];
        if let Some(p) = def.portal_at(tx, ty) {
            next.push((
                p.to,
                (p.arrive.0 + 8).div_euclid(16),
                (p.arrive.1 + 12).div_euclid(16),
            ));
        }
        for d in Dir::ALL {
            let (dx, dy) = d.delta();
            let (nx, ny) = (tx + dx as i32, ty + dy as i32);
            if (0..ROOM_W).contains(&nx) && (0..ROOM_H).contains(&ny) {
                if open(room, nx, ny) {
                    next.push((room, nx, ny));
                }
                continue;
            }
            match def.edge(d) {
                Edge::Scroll(to) => next.push((to, nx.rem_euclid(ROOM_W), ny.rem_euclid(ROOM_H))),
                Edge::Wipe(to, a) => {
                    next.push((to, (a.0 + 8).div_euclid(16), (a.1 + 12).div_euclid(16)))
                }
                Edge::Wall => {}
            }
        }
        for n in next {
            if open(n.0, n.1, n.2) && seen.insert(n) {
                q.push_back(n);
            }
        }
    }
    for room in RoomId::ALL {
        assert!(seen.iter().any(|n| n.0 == room), "{room} unreachable");
    }
}

#[test]
fn no_sword_means_no_stab_and_the_sword_kills_a_spitter() {
    let mut g = arena(RoomId::Meadow, (2, 5));
    g.add_enemy(EnemyKind::Spitter, 3, 5);
    g.face(Dir::Right);
    g.tick(stab());
    assert_eq!(g.hero().action, HeroAction::Free);
    assert_eq!(g.enemies().len(), 1);

    g.set_hp(6);
    g.clear_enemies();
    g.place_hero(32.0, 76.0);
    g.grant_sword();
    g.add_enemy(EnemyKind::Spitter, 3, 5);
    g.face(Dir::Right);
    g.tick(stab());
    assert!(matches!(g.hero().action, HeroAction::Stabbing { .. }));
    run(&mut g, IDLE, 14);
    assert!(g.enemies().is_empty(), "spitter survived the stab");
    assert_eq!(g.hero().action, HeroAction::Free);
}

#[test]
fn a_stab_cannot_be_held_down() {
    let mut g = arena(RoomId::Meadow, (2, 5));
    g.grant_sword();
    g.tick(stab());
    run(&mut g, IDLE, 14);
    assert_eq!(g.hero().action, HeroAction::Free);
    g.tick(stab());
    assert!(matches!(g.hero().action, HeroAction::Stabbing { t: 0 }));
}

fn beams(g: &Game) -> usize {
    g.shots()
        .iter()
        .filter(|s| s.kind == ProjKind::Beam)
        .count()
}

#[test]
fn the_beam_needs_full_hearts_and_only_one_flies_at_a_time() {
    let mut g = arena(RoomId::Meadow, (1, 5));
    g.grant_sword();
    g.face(Dir::Right);
    g.tick(stab());
    assert_eq!(beams(&g), 1);
    run(&mut g, IDLE, 14);
    g.tick(stab());
    assert_eq!(beams(&g), 1, "second beam while the first still flies");
    run(&mut g, IDLE, 120);
    assert_eq!(beams(&g), 0, "beam should vanish at the playfield edge");
    g.set_hp(5);
    g.tick(stab());
    assert_eq!(beams(&g), 0, "no beam below full hearts");
}

#[test]
fn the_beam_hurts_enemies() {
    let mut g = arena(RoomId::Meadow, (1, 5));
    g.grant_sword();
    g.face(Dir::Right);
    g.add_enemy(EnemyKind::Spitter, 9, 5);
    g.tick(stab());
    run(&mut g, IDLE, 60);
    assert!(g.enemies().is_empty());
}

#[test]
fn contact_costs_hearts_knocks_back_and_invulnerability_blocks_a_second_hit() {
    let mut g = arena(RoomId::Meadow, (5, 5));
    let start = (g.hero().x, g.hero().y);
    g.add_enemy(EnemyKind::Spitter, 5, 5);
    g.tick(IDLE);
    assert_eq!(g.hero().hp, 5);
    assert!(matches!(g.hero().action, HeroAction::Knockback { .. }));
    run(&mut g, IDLE, 14);
    assert_eq!(g.hero().action, HeroAction::Free);
    let moved = (g.hero().x - start.0).abs() + (g.hero().y - start.1).abs();
    assert!(moved >= 20.0, "knocked back only {moved}px");
    assert!(g.hero().invuln > 0);
    let (hx, hy) = g.hero().tile();
    g.add_enemy(EnemyKind::Skeleton, hx, hy);
    g.tick(IDLE);
    assert_eq!(
        g.hero().hp,
        5,
        "second hit during invulnerability must be free"
    );
    run(&mut g, IDLE, 60);
    g.clear_enemies();
    let (hx, hy) = g.hero().tile();
    g.add_enemy(EnemyKind::Skeleton, hx, hy);
    g.tick(IDLE);
    assert_eq!(g.hero().hp, 4, "hits count again once invulnerability ends");
}

fn rock_hits(facing: Dir, action_stab: bool) -> u32 {
    let mut g = arena(RoomId::Meadow, (5, 5));
    g.grant_sword();
    g.face(facing);
    if action_stab {
        g.tick(stab());
    }
    let (hx, hy) = (g.hero().x, g.hero().y);
    let reach = if action_stab { 16.0 } else { 40.0 };
    g.add_shot(ProjKind::Rock, hx + 8.0 + reach, hy + 10.0, -2.0, 0.0);
    run(&mut g, IDLE, 30);
    g.hero().hp
}

#[test]
fn the_shield_blocks_a_rock_only_when_facing_it_and_not_stabbing() {
    assert_eq!(rock_hits(Dir::Right, false), 6, "facing the rock");
    assert_eq!(rock_hits(Dir::Left, false), 5, "facing away");
    assert_eq!(rock_hits(Dir::Up, false), 5, "facing sideways");
    assert_eq!(rock_hits(Dir::Right, true), 5, "while stabbing");
}

#[test]
fn fireballs_are_never_blocked() {
    let mut g = arena(RoomId::Meadow, (5, 5));
    g.face(Dir::Right);
    let (hx, hy) = (g.hero().x, g.hero().y);
    g.add_shot(ProjKind::Fireball, hx + 8.0 + 30.0, hy + 10.0, -1.5, 0.0);
    run(&mut g, IDLE, 30);
    assert_eq!(g.hero().hp, 5);
}

#[test]
fn hearts_cap_and_gems_and_keys_count_up() {
    let mut g = arena(RoomId::Meadow, (5, 5));
    g.set_hp(5);
    let (x, y) = (g.hero().x, g.hero().y);
    g.add_pickup(PickupKind::Heart, x, y);
    g.tick(IDLE);
    assert_eq!(g.hero().hp, 6);
    g.add_pickup(PickupKind::Heart, x, y);
    g.tick(IDLE);
    assert_eq!(g.hero().hp, 6, "hearts cap at max");
    for _ in 0..3 {
        g.add_pickup(PickupKind::Gem, x, y);
        g.tick(IDLE);
    }
    assert_eq!(g.gems(), 3);
    g.add_pickup(PickupKind::Key, x, y);
    run(&mut g, IDLE, 80);
    assert_eq!(g.keys(), 1);
    assert_eq!(g.mode(), Mode::Playing);
}

#[test]
fn drops_vanish_after_their_time() {
    let mut g = arena(RoomId::Meadow, (5, 5));
    g.add_pickup(PickupKind::Gem, 200.0, 20.0);
    run(&mut g, IDLE, 479);
    assert_eq!(g.pickups().len(), 1);
    run(&mut g, IDLE, 2);
    assert!(g.pickups().is_empty());
}

#[test]
fn a_locked_door_needs_a_key_spends_it_and_stays_open() {
    let mut g = arena(RoomId::D1, (8, 1));
    run(&mut g, go(Dir::Up), 80);
    assert_eq!(g.room(), RoomId::D1, "door opened without a key");
    assert!(g.hero().y >= 7.0);

    g.grant_keys(1);
    run(&mut g, go(Dir::Up), 30);
    assert!(matches!(g.mode(), Mode::Scrolling { to: RoomId::D3, .. }));
    assert_eq!(g.keys(), 0, "the key is spent");
    g.clear_enemies();
    run(&mut g, IDLE, 60);
    assert_eq!(g.room(), RoomId::D3);

    run(&mut g, go(Dir::Down), 120);
    assert_eq!(g.room(), RoomId::D1);
    run(&mut g, go(Dir::Up), 130);
    assert_eq!(g.room(), RoomId::D3, "door must stay open");
    assert_eq!(g.keys(), 0);
}

#[test]
fn the_cave_grants_the_sword_once() {
    let mut g = arena(RoomId::Meadow, (4, 2));
    g.teleport(RoomId::Meadow, 60.0, 28.0);
    g.clear_enemies();
    run(&mut g, go(Dir::Up), 20);
    assert!(matches!(
        g.mode(),
        Mode::Wipe {
            to: RoomId::Cave,
            ..
        }
    ));
    let mut guard = 0;
    while !matches!(g.mode(), Mode::Dialogue { .. }) {
        g.tick(IDLE);
        guard += 1;
        assert!(guard < 100);
    }
    let mut seen = 0;
    while let Mode::Dialogue { chars, .. } = g.mode() {
        seen = seen.max(chars);
        g.tick(IDLE);
    }
    assert_eq!(
        seen as usize,
        "IT'S A LONG ROAD, TRAVELER. TAKE THIS.".len()
    );
    assert!(!g.progress().sword_taken);
    run(&mut g, go(Dir::Up), 60);
    assert!(matches!(
        g.mode(),
        Mode::ItemGet {
            item: PickupKind::Sword,
            ..
        }
    ));
    run(&mut g, IDLE, 120);
    assert_eq!(g.mode(), Mode::Playing);
    assert!(g.progress().sword_taken);

    run(&mut g, go(Dir::Down), 120);
    assert_eq!(g.room(), RoomId::Meadow);
    g.clear_enemies();
    g.teleport(RoomId::Meadow, 60.0, 28.0);
    run(&mut g, go(Dir::Up), 20);
    run(&mut g, IDLE, 40);
    assert_eq!(g.room(), RoomId::Cave);
    assert_eq!(g.mode(), Mode::Playing, "no second speech");
    assert!(g.pickups().is_empty(), "no second sword");
}

#[test]
fn clearing_d2_grants_the_key() {
    let mut g = arena(RoomId::D2, (7, 6));
    g.add_enemy(EnemyKind::Skeleton, 3, 3);
    run(&mut g, IDLE, 10);
    assert!(g.pickups().iter().all(|p| p.kind != PickupKind::Key));
    g.clear_enemies();
    g.tick(IDLE);
    let key = g
        .pickups()
        .iter()
        .find(|p| p.kind == PickupKind::Key)
        .copied()
        .expect("key appears");
    g.place_hero(key.x, key.y);
    run(&mut g, IDLE, 120);
    assert_eq!(g.keys(), 1);
    assert!(g.progress().key_taken);
}

#[test]
fn the_boss_drops_the_relic_and_the_relic_wins_the_game() {
    let mut g = arena(RoomId::D3, (1, 9));
    assert_eq!(g.enemies().len(), 0);
    g.add_enemy(EnemyKind::Gnasher, 9, 4);
    for _ in 0..6 {
        let c = g.enemies()[0].center();
        g.add_shot(ProjKind::Beam, c.0, c.1, 0.0, -0.5);
        run(&mut g, IDLE, 25);
        g.set_hp(6);
        if g.enemies().is_empty() {
            break;
        }
    }
    assert!(g.enemies().is_empty(), "boss still alive");
    assert!(g.progress().boss_dead);
    let relic = g
        .pickups()
        .iter()
        .find(|p| p.kind == PickupKind::Relic)
        .copied()
        .expect("relic");
    g.place_hero(relic.x, relic.y);
    run(&mut g, IDLE, 3);
    assert!(matches!(
        g.mode(),
        Mode::ItemGet {
            item: PickupKind::Relic,
            ..
        }
    ));
    run(&mut g, IDLE, 100);
    assert!(matches!(g.mode(), Mode::Victory { .. }));
}

#[test]
fn zero_hearts_is_game_over_and_start_gives_a_fresh_game() {
    let mut g = arena(RoomId::Meadow, (5, 5));
    g.grant_sword();
    g.set_hp(1);
    g.add_enemy(EnemyKind::Spitter, 5, 5);
    g.tick(IDLE);
    assert!(matches!(g.mode(), Mode::GameOver { .. }));
    g.tick(Input {
        start: true,
        ..IDLE
    });
    assert!(
        matches!(g.mode(), Mode::GameOver { .. }),
        "a stray press right away must not skip the screen"
    );
    run(&mut g, IDLE, 25);
    g.tick(Input {
        start: true,
        ..IDLE
    });
    assert_eq!(g.mode(), Mode::Title);
    assert_eq!(g.hero().hp, 6);
    assert!(!g.progress().sword_taken);
    assert_eq!(g.room(), RoomId::Meadow);
    g.tick(Input {
        start: true,
        ..IDLE
    });
    assert_eq!(g.mode(), Mode::Playing);
}

fn play_to_victory(seed: u64, limit: u64) -> Option<u64> {
    let mut g = Game::new(seed);
    let mut bot = Bot::new();
    while g.ticks() < limit {
        let input = bot.decide(&g);
        g.tick(input);
        assert!(
            !matches!(g.mode(), Mode::GameOver { .. }),
            "bot died with seed {seed} at tick {}",
            g.ticks()
        );
        if matches!(g.mode(), Mode::Victory { .. }) {
            return Some(g.ticks());
        }
    }
    None
}

#[test]
fn the_bot_wins_from_title_to_victory_and_is_deterministic() {
    let a = play_to_victory(1, 60_000).expect("bot should reach victory");
    let b = play_to_victory(1, 60_000).expect("bot should reach victory again");
    assert_eq!(a, b, "same seed, same inputs, same finish tick");
    eprintln!("bot victory at tick {a}");
}

#[test]
fn the_bot_wins_with_other_seeds_too() {
    for seed in [6, 8, 10] {
        assert!(play_to_victory(seed, 60_000).is_some(), "seed {seed}");
    }
}

#[test]
fn the_clock_caps_catch_up_at_five_ticks_and_keeps_the_remainder() {
    let mut c = Clock::default();
    let step = Clock::STEP;
    assert_eq!(c.advance(0.0), 0);
    assert_eq!(c.advance(step * 1.01), 1);
    assert_eq!(c.advance(step * 2.5), 2);
    let rem = c.remainder();
    assert!(rem > 0.4 * step && rem < 0.6 * step, "remainder {rem}");
    assert_eq!(c.advance(step * 0.5), 1, "half-steps accumulate");
    assert_eq!(c.advance(step * 100.0), MAX_TICKS_PER_FRAME);
    assert!(
        c.remainder() < step,
        "backlog beyond the cap is dropped, not replayed"
    );
    assert_eq!(c.advance(step), 1);
}

#[test]
fn key_presses_become_one_tick_edges_and_the_latest_direction_wins() {
    let mut k = Keys::default();
    k.press(Key::Sword);
    k.press(Key::Sword);
    assert!(k.take().sword);
    assert!(!k.take().sword, "held sword key must not repeat");
    k.release(Key::Sword);
    k.press(Key::Sword);
    assert!(k.take().sword);

    k.press(Key::Dir(Dir::Left));
    k.press(Key::Dir(Dir::Up));
    assert_eq!(k.take().dir, Some(Dir::Up));
    k.release(Key::Dir(Dir::Up));
    assert_eq!(k.take().dir, Some(Dir::Left));
    k.clear();
    assert_eq!(k.take().dir, None);
}

#[test]
fn every_sprite_is_well_formed_and_every_drawn_glyph_exists() {
    assert_eq!(sprites::malformed(), vec![]);
    let mut g = Game::new(1);
    let mut scenes = vec![g.scene()];
    g.tick(Input {
        start: true,
        ..IDLE
    });
    g.teleport(RoomId::Cave, 120.0, 140.0);
    scenes.push(g.scene());
    for scene in scenes {
        for d in scene {
            if let SpriteId::Glyph(_) = d.sprite {
                let img = sprites::pixels(d.sprite, false, false);
                assert!(
                    img.rgba.chunks(4).any(|p| p[3] > 0),
                    "missing glyph {:?}",
                    d.sprite
                );
            }
        }
    }
}

#[test]
fn the_minimap_lists_only_rooms_that_exist() {
    let cells = |room| Game::new_at(1, room).hud().map_rooms;
    assert_eq!(
        cells(RoomId::D1).len(),
        3,
        "the dungeon has three rooms in a 2 by 2 grid"
    );
    assert_eq!(cells(RoomId::Meadow).len(), 6);
    assert_eq!(cells(RoomId::Cave).len(), 1);
}

#[test]
fn victory_time_counts_every_tick_from_the_start_press() {
    let mut g = Game::new(1);
    let mut bot = Bot::new();
    let mut started = None;
    while !matches!(g.mode(), Mode::Victory { .. }) {
        assert!(g.ticks() < 60_000, "bot should reach victory");
        g.tick(bot.decide(&g));
        if started.is_none() && g.mode() != Mode::Title {
            started = Some(g.ticks());
        }
    }
    let run = g.ticks() - started.expect("the bot presses start");
    assert_eq!(g.elapsed_secs(), run / 60);
    for _ in 0..600 {
        g.tick(Input::default());
    }
    assert_eq!(g.elapsed_secs(), run / 60, "the clock stops at victory");
}

#[test]
fn every_documented_control_maps_to_its_action() {
    use little_quest::input::key_from_name;
    let expected = [
        ("up", Key::Dir(Dir::Up)),
        ("w", Key::Dir(Dir::Up)),
        ("down", Key::Dir(Dir::Down)),
        ("s", Key::Dir(Dir::Down)),
        ("left", Key::Dir(Dir::Left)),
        ("a", Key::Dir(Dir::Left)),
        ("right", Key::Dir(Dir::Right)),
        ("d", Key::Dir(Dir::Right)),
        ("space", Key::Sword),
        ("z", Key::Sword),
        ("x", Key::Sword),
        ("enter", Key::Start),
    ];
    for (name, key) in expected {
        assert_eq!(key_from_name(name), Some(key), "{name}");
    }
    assert_eq!(key_from_name("q"), None);
}
