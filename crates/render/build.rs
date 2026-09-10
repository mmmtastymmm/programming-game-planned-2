//! Asset bake: rasterize the SVG sources in `assets/art` into the PNG
//! textures the view loads from `assets/textures`, which is not tracked —
//! the art is the source and the bake reruns when it changes. Ported from
//! the predecessor's game crate.
//!
//! Bot and printer faces are authored once in the green master palette;
//! team variants are string-level palette swaps of the accent hexes, baked
//! into one 3x2 atlas per team (front/right/back over left/top/bottom — the
//! layout `palette::atlas_box_mesh` maps UVs to). Water and ore autotile:
//! a NESW same-neighbour bitmask picks one of 16 variants with edge art on
//! the "different" sides. Mountains bake as a pair, summit beside cliff.

// Pixel and mask arithmetic on build-time constants: nothing here runs in
// the sim, so the workspace's overflow lint has nothing to guard.
#![allow(clippy::arithmetic_side_effects)]

use resvg::{tiny_skia, usvg};
use std::fs;
use std::path::Path;

/// Pixels per face / per tile texture.
const SIZE: u32 = 256;

/// Plain single textures.
const PLAIN: &[&str] = &["crate", "paper"];

/// The ground: three sway frames, no edges — it is what every other terrain
/// draws its edge against. Baked as `tile_ground_f{frame}.png`.
const GROUND: [&str; 3] = ["tile_grass", "tile_grass_sway_1", "tile_grass_sway_2"];

/// Animated autotiles: (frames, edge master, output prefix), baked as
/// `{prefix}_{mask}_f{frame}.png`.
const ANIMATED: &[([&str; 3], &str, &str)] = &[
    (
        ["tile_water", "tile_water_flow_1", "tile_water_flow_2"],
        "tile_water_bank",
        "tile_water",
    ),
    (
        ["tile_ore", "tile_ore_glint_1", "tile_ore_glint_2"],
        "tile_ore_edge",
        "tile_ore",
    ),
];

/// Atlased 6-face bodies: (svg prefix, output prefix).
const ATLASES: &[(&str, &str)] = &[("bot_face", "bot_atlas"), ("printer_face", "printer_atlas")];

/// (face, atlas column, atlas row)
const FACES: &[(&str, u32, u32)] = &[
    ("front", 0, 0),
    ("right", 1, 0),
    ("back", 2, 0),
    ("left", 0, 1),
    ("top", 1, 1),
    ("bottom", 2, 1),
];

/// Master accent palette as authored (green).
const MASTER: [&str; 3] = ["#39d98a", "#2aa86b", "#c8ffe6"];

/// (atlas name, [accent, accent-dark, highlight]) — `palette::TEAM_PALETTES`
/// names these in the same order; "ruined" is the dead-grey swap.
const TEAMS: &[(&str, [&str; 3])] = &[
    ("green", ["#39d98a", "#2aa86b", "#c8ffe6"]),
    ("red", ["#f24c40", "#c03227", "#ffd8d3"]),
    ("blue", ["#4fa3f2", "#2f7fd1", "#d9ecff"]),
    ("yellow", ["#f2c94c", "#c9a12e", "#fff3c9"]),
    ("cyan", ["#45d4d4", "#2cadad", "#d2ffff"]),
    ("magenta", ["#e254c7", "#b13a9c", "#ffd6f6"]),
    ("orange", ["#f2913f", "#cc7122", "#ffe0c2"]),
    ("purple", ["#9b59f2", "#7a3fd1", "#e8d9ff"]),
    ("white", ["#e8e8f0", "#b9bcc9", "#ffffff"]),
    ("ruined", ["#5a5f6a", "#4a4e57", "#8a8e99"]),
];

fn render(svg: &str, px: u32) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("valid SVG");
    let mut pixmap = tiny_skia::Pixmap::new(px, px).expect("pixmap alloc");
    let scale = px as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
}

fn main() {
    println!("cargo:rerun-if-changed=assets/art");

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let art = root.join("assets/art");
    let out = root.join("assets/textures");
    fs::create_dir_all(&out).expect("create textures dir");
    let read = |name: &str| -> String {
        fs::read_to_string(art.join(format!("{name}.svg")))
            .unwrap_or_else(|e| panic!("assets/art/{name}.svg: {e}"))
    };

    for name in PLAIN {
        render(&read(name), SIZE)
            .save_png(out.join(format!("{name}.png")))
            .expect("save png");
    }
    for (f, name) in GROUND.iter().enumerate() {
        render(&read(name), SIZE)
            .save_png(out.join(format!("tile_ground_f{f}.png")))
            .expect("save ground png");
    }

    // Bit order matches the view's mask computation: 0 = N, 1 = E, 2 = S,
    // 3 = W, image-up = north. An unset bit gets the edge overlay, rotated
    // from its north-edge master.
    let autotile = |base: &str, edge: &str| -> Vec<tiny_skia::Pixmap> {
        let base_svg = read(base);
        let edge_px = render(&read(edge), SIZE);
        (0..16u32)
            .map(|mask| {
                let mut px = render(&base_svg, SIZE);
                for bit in 0..4 {
                    if mask & (1 << bit) == 0 {
                        let half = SIZE as f32 / 2.0;
                        px.draw_pixmap(
                            0,
                            0,
                            edge_px.as_ref(),
                            &tiny_skia::PixmapPaint::default(),
                            tiny_skia::Transform::from_rotate_at(90.0 * bit as f32, half, half),
                            None,
                        );
                    }
                }
                px
            })
            .collect()
    };
    for (frames, edge, prefix) in ANIMATED {
        for (f, base) in frames.iter().enumerate() {
            for (mask, px) in autotile(base, edge).iter().enumerate() {
                px.save_png(out.join(format!("{prefix}_{mask}_f{f}.png")))
                    .expect("save autotile png");
            }
        }
    }

    // Rock: the summit autotiles against its rim; each variant ships as an
    // atlas pair with the cliff face (the layout `palette::block_mesh` maps).
    let rock = render(&read("rock_face"), SIZE);
    for (mask, top) in autotile("tile_mountain", "tile_mountain_rim")
        .iter()
        .enumerate()
    {
        let mut pair = tiny_skia::Pixmap::new(SIZE * 2, SIZE).expect("pair alloc");
        for (i, px) in [top, &rock].into_iter().enumerate() {
            pair.draw_pixmap(
                (i as u32 * SIZE) as i32,
                0,
                px.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }
        pair.save_png(out.join(format!("rock_atlas_{mask}.png")))
            .expect("save rock atlas");
    }

    for (svg_prefix, out_prefix) in ATLASES {
        for (team, colors) in TEAMS {
            let mut atlas = tiny_skia::Pixmap::new(SIZE * 3, SIZE * 2).expect("atlas alloc");
            for (face, col, row) in FACES {
                let mut svg = read(&format!("{svg_prefix}_{face}"));
                for (master, team_color) in MASTER.iter().zip(colors) {
                    svg = svg.replace(master, team_color);
                }
                let face_px = render(&svg, SIZE);
                atlas.draw_pixmap(
                    (col * SIZE) as i32,
                    (row * SIZE) as i32,
                    face_px.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
            atlas
                .save_png(out.join(format!("{out_prefix}_{team}.png")))
                .expect("save atlas png");
        }
    }
}
