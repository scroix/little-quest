use std::path::Path;

use image::{Rgba, RgbaImage};
use little_quest::game::{Draw, Game, Input, PickupKind};
use little_quest::sprites::{self, SpriteId};
use little_quest::world::{PLAY_H, PLAY_W, RoomId};

fn blit(img: &mut RgbaImage, id: SpriteId, flip_x: bool, flash: bool, x: i32, y: i32, scale: i32) {
    let s = sprites::pixels(id, flip_x, flash);
    for sy in 0..s.h as i32 {
        for sx in 0..s.w as i32 {
            let o = ((sy as u32 * s.w + sx as u32) * 4) as usize;
            if s.rgba[o + 3] == 0 {
                continue;
            }
            let px = Rgba([s.rgba[o], s.rgba[o + 1], s.rgba[o + 2], 255]);
            for dy in 0..scale {
                for dx in 0..scale {
                    let (ix, iy) = ((x + sx) * scale + dx, (y + sy) * scale + dy);
                    if ix >= 0 && iy >= 0 && (ix as u32) < img.width() && (iy as u32) < img.height()
                    {
                        img.put_pixel(ix as u32, iy as u32, px);
                    }
                }
            }
        }
    }
}

fn frame(draws: &[Draw], scale: i32) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(
        (PLAY_W * scale) as u32,
        (PLAY_H * scale) as u32,
        Rgba([0, 0, 0, 255]),
    );
    for d in draws {
        blit(&mut img, d.sprite, d.flip_x, d.flash, d.x, d.y, scale);
    }
    img
}

fn sheet(out: &Path) {
    let all = SpriteId::all();
    let cols = 12usize;
    let cell = 36i32;
    let rows = all.len().div_ceil(cols) + 3;
    let mut img = RgbaImage::from_pixel(
        (cols as i32 * cell * 4 + 8) as u32,
        (rows as i32 * cell * 4 + 8) as u32,
        Rgba([72, 72, 88, 255]),
    );
    for (i, id) in all.iter().enumerate() {
        let (cx, cy) = ((i % cols) as i32 * cell + 2, (i / cols) as i32 * cell + 2);
        blit(&mut img, *id, false, false, cx, cy, 4);
    }
    let base_row = all.len().div_ceil(cols) as i32 * cell;
    let glyphs: Vec<SpriteId> =
        sprites::text_glyphs("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,!?'-:/>");
    for (i, g) in glyphs.iter().enumerate() {
        blit(
            &mut img,
            *g,
            false,
            false,
            2 + (i as i32 % 30) * 9,
            base_row + 2 + (i as i32 / 30) * 10,
            4,
        );
    }
    img.save(out.join("sheet.png")).unwrap();
}

fn overview<const N: usize>(out: &Path, name: &str, grid: &[[RoomId; N]]) {
    let scale = 2;
    let (w, h) = ((PLAY_W * scale) as u32, (PLAY_H * scale) as u32);
    let mut img = RgbaImage::from_pixel(w * N as u32, h * grid.len() as u32, Rgba([0, 0, 0, 255]));
    for (r, row) in grid.iter().enumerate() {
        for (c, room) in row.iter().enumerate() {
            let tile = frame(&Game::new_at(1, *room).scene(), scale);
            image::imageops::replace(&mut img, &tile, c as i64 * w as i64, r as i64 * h as i64);
        }
    }
    img.save(out.join(format!("{name}.png"))).unwrap();
}

fn zoom(out: &Path) {
    use SpriteId::*;
    let ids = [
        HeroDown(0),
        HeroDown(1),
        HeroUp(0),
        HeroUp(1),
        HeroSide(0),
        HeroSide(1),
        HeroStabDown,
        HeroStabUp,
        HeroStabSide,
        HeroHold,
        Mentor,
        SpitterDown(0),
        SpitterUp(0),
        SpitterSide(0),
        Bat(0),
        Bat(1),
        Skeleton(0),
        Skeleton(1),
    ];
    let mut img = RgbaImage::from_pixel(
        ids.len() as u32 * 17 * 6,
        17 * 6 * 2,
        Rgba([72, 72, 88, 255]),
    );
    for (i, id) in ids.iter().enumerate() {
        blit(&mut img, *id, false, false, i as i32 * 17, 0, 6);
        blit(&mut img, *id, true, true, i as i32 * 17, 17, 6);
    }
    img.save(out.join("zoom.png")).unwrap();
}

fn main() {
    let out = Path::new("target/snapshots");
    std::fs::create_dir_all(out).unwrap();
    let bad = sprites::malformed();
    assert!(bad.is_empty(), "malformed sprites: {bad:?}");
    sheet(out);
    for room in RoomId::ALL {
        let mut g = Game::new_at(1, room);
        if room == RoomId::Cave {
            g.teleport(room, 120.0, 140.0);
        }
        frame(&g.scene(), 3)
            .save(out.join(format!("room_{}.png", room.def().name)))
            .unwrap();
    }
    zoom(out);
    overview(
        out,
        "overworld",
        &[
            [RoomId::Lake, RoomId::Pass, RoomId::Approach],
            [RoomId::Maze, RoomId::Meadow, RoomId::Shore],
        ],
    );
    overview(
        out,
        "dungeon",
        &[
            [RoomId::D3, RoomId::Cave, RoomId::Cave],
            [RoomId::D1, RoomId::D2, RoomId::Cave],
        ],
    );
    let mut g = Game::new(1);
    frame(&g.scene(), 3)
        .save(out.join("screen_title.png"))
        .unwrap();
    g.tick(Input {
        start: true,
        ..Input::default()
    });
    g.add_pickup(PickupKind::Gem, 40.0, 40.0);
    g.add_pickup(PickupKind::Heart, 60.0, 40.0);
    g.add_pickup(PickupKind::Key, 80.0, 40.0);
    g.add_pickup(PickupKind::Relic, 100.0, 40.0);
    frame(&g.scene(), 3)
        .save(out.join("room_meadow_items.png"))
        .unwrap();
    println!("wrote {}", out.display());
}
