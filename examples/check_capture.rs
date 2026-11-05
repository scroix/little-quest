use std::collections::HashSet;
use std::process::ExitCode;

use image::{Rgba, RgbaImage};
use little_quest::sprites::{self, SpriteId};

const LOGICAL_W: u32 = 768;
const HUD_TOP: u32 = 28;
const PLAY_TOP: u32 = HUD_TOP + 168;
const PLAY_H: u32 = 528;
const SLACK: usize = 24;

/// Every colour a sprite can produce, flashed or not.
fn palette() -> HashSet<[u8; 3]> {
    let mut set = HashSet::new();
    let mut ids = SpriteId::all();
    ids.push(SpriteId::Glyph('A'));
    for id in ids {
        for flash in [false, true] {
            for p in sprites::pixels(id, false, flash)
                .rgba
                .chunks(4)
                .filter(|p| p[3] > 0)
            {
                set.insert([p[0], p[1], p[2]]);
            }
        }
    }
    set
}

/// Crop the playfield. The window can be a pixel or two taller than the 724 logical points the
/// layout fills, so try the few plausible vertical offsets and keep the crispest crop.
fn playfield(img: &RgbaImage) -> Result<RgbaImage, String> {
    let scale = (img.width() as f32 / LOGICAL_W as f32).round() as u32;
    let base = (HUD_TOP + 696) * scale;
    if scale == 0
        || img.width() != LOGICAL_W * scale
        || img.height() < base
        || img.height() > base + 2 * scale
    {
        return Err(format!(
            "capture is {}x{}, expected 768x724 times a whole scale",
            img.width(),
            img.height()
        ));
    }
    let slack = img.height() - base;
    (0..=slack)
        .map(|off| {
            image::imageops::crop_imm(
                img,
                0,
                PLAY_TOP * scale + off,
                LOGICAL_W * scale,
                PLAY_H * scale,
            )
            .to_image()
        })
        .min_by_key(distinct_colors)
        .ok_or_else(|| "no crop".to_string())
}

fn distinct_colors(img: &RgbaImage) -> usize {
    img.pixels()
        .map(|p| [p[0], p[1], p[2]])
        .collect::<HashSet<_>>()
        .len()
}

/// Box blur, the way a linear-filtered upscale would smear sprite edges.
fn blur(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for y in 1..img.height() - 1 {
        for x in 1..img.width() - 1 {
            let mut sum = [0u32; 3];
            for dy in 0..3 {
                for dx in 0..3 {
                    let p = img.get_pixel(x + dx - 1, y + dy - 1);
                    for c in 0..3 {
                        sum[c] += p[c] as u32;
                    }
                }
            }
            out.put_pixel(
                x,
                y,
                Rgba([
                    (sum[0] / 9) as u8,
                    (sum[1] / 9) as u8,
                    (sum[2] / 9) as u8,
                    255,
                ]),
            );
        }
    }
    out
}

/// Fraction of horizontal colour runs whose length is not a whole number of 3-pixel sprite cells.
fn ragged_runs(img: &RgbaImage, cell: u32) -> f64 {
    let (mut runs, mut ragged) = (0u64, 0u64);
    for y in 0..img.height() {
        let mut start = 0;
        for x in 1..=img.width() {
            if x == img.width() || img.get_pixel(x, y) != img.get_pixel(start, y) {
                runs += 1;
                if (x - start) % cell != 0 {
                    ragged += 1;
                }
                start = x;
            }
        }
    }
    ragged as f64 / runs as f64
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: check_capture <capture.png>");
        return ExitCode::from(2);
    };
    let img = match image::open(&path) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let field = match playfield(&img) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let scale = field.width() / LOGICAL_W;
    let threshold = palette().len() + SLACK;
    let colors = distinct_colors(&field);
    let ragged = ragged_runs(&field, 3 * scale);
    println!("capture {path}");
    println!(
        "playfield {}x{} (scale {scale}), distinct colors {colors}, threshold {threshold}, ragged runs {:.3}%",
        field.width(),
        field.height(),
        ragged * 100.0
    );

    let blurred = distinct_colors(&blur(&field));
    println!("self-test: blurred copy has {blurred} distinct colors");
    if blurred <= threshold {
        eprintln!("self-test failed: the blurred copy should exceed the threshold");
        return ExitCode::from(3);
    }
    if colors > threshold {
        eprintln!("FAIL: capture is not crisp");
        return ExitCode::FAILURE;
    }
    println!("PASS: capture is crisp");
    ExitCode::SUCCESS
}
