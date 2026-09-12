//! Meshes, materials and the frame of the 3D view: the tile grid in world
//! units, the atlas boxes the bake's textures wrap, and the team palettes.
//! Ported from the predecessor's `palette.rs`; nothing here is read back
//! by the sim.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim::{TeamId, TilePos};
use std::collections::HashMap;

pub const CLEAR: Color = Color::srgb(0.05, 0.06, 0.09);
/// Half-extent of a bot's cube.
pub const BOT_HALF: f32 = 0.35;
/// Render height of a rock tile's summit (view-only; the sim has no height).
pub const ROCK_HEIGHT: f32 = 0.55;
/// Water sits a little below the plane so it reads as a depression.
pub const WATER_SINK: f32 = -0.05;

// The lens on a bot's front face: a barrel half-sunk into the face and a
// team-accent glass at its tip.
pub const LENS_Y: f32 = 0.044;
pub const LENS_BARREL_RADIUS: f32 = 0.115;
pub const LENS_BARREL_LEN: f32 = 0.24;
pub const LENS_BARREL_Z: f32 = -BOT_HALF;
pub const LENS_GLASS_RADIUS: f32 = 0.08;
pub const LENS_GLASS_LEN: f32 = 0.02;
pub const LENS_GLASS_Z: f32 = LENS_BARREL_Z - LENS_BARREL_LEN / 2.0 - LENS_GLASS_LEN / 4.0;

/// The atlas names `build.rs` bakes: the eight deployment colors of
/// `data/machines.toml` and `white`, which a machine with no color
/// deployment wears before its team's tint; a tenth, `ruined`, is the
/// dead-grey swap.
pub const ATLASES: [&str; 9] = [
    "green", "red", "blue", "yellow", "cyan", "magenta", "orange", "purple", "white",
];

/// `data/interface.toml` (`docs/07`, Costs): the renderer's tuning. Nothing
/// here reaches a hash.
#[derive(Resource, Debug, Clone, serde::Deserialize)]
pub struct Interface {
    pub faults: InterfaceFaults,
    pub teams: InterfaceTeams,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InterfaceFaults {
    pub fault_mark_ticks: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InterfaceTeams {
    /// Ring colors as `#rrggbb`, for the teams that are not the player's,
    /// in team order skipping the player's.
    pub rings: Vec<String>,
}

pub const INTERFACE_TOML: &str = include_str!("../../../data/interface.toml");

fn hex_color(s: &str) -> Option<Color> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color::srgb_u8(
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
    ))
}

impl Interface {
    pub fn load() -> Result<Interface, String> {
        let i: Interface = toml::from_str(INTERFACE_TOML).map_err(|e| e.to_string())?;
        if i.teams.rings.is_empty() {
            return Err("interface.toml: teams.rings is empty".into());
        }
        for r in &i.teams.rings {
            hex_color(r).ok_or_else(|| format!("interface.toml: `{r}` is not #rrggbb"))?;
        }
        Ok(i)
    }

    /// The team's ring color (Q35): the player's white, every other team's
    /// from the list in team order skipping the player's.
    pub fn team_color(&self, player: TeamId, team: TeamId) -> Color {
        if team == player {
            return Color::WHITE;
        }
        let i = if team.0 < player.0 {
            team.0
        } else {
            team.0.saturating_sub(1)
        } as usize;
        hex_color(&self.teams.rings[i % self.teams.rings.len()]).unwrap_or(Color::WHITE)
    }

    pub fn team_color32(&self, player: TeamId, team: TeamId) -> bevy_egui::egui::Color32 {
        let c = self.team_color(player, team).to_srgba();
        bevy_egui::egui::Color32::from_rgb(
            (c.red * 255.0) as u8,
            (c.green * 255.0) as u8,
            (c.blue * 255.0) as u8,
        )
    }
}

/// A paint colour by its name in `data/machines.toml`.
pub fn paint_color(name: &str) -> Color {
    match name {
        "red" => Color::srgb(0.85, 0.22, 0.20),
        "blue" => Color::srgb(0.25, 0.45, 0.90),
        "green" => Color::srgb(0.25, 0.75, 0.35),
        "yellow" => Color::srgb(0.90, 0.80, 0.25),
        "purple" => Color::srgb(0.60, 0.30, 0.85),
        "orange" => Color::srgb(0.95, 0.55, 0.15),
        "cyan" => Color::srgb(0.25, 0.80, 0.80),
        "magenta" => Color::srgb(0.85, 0.30, 0.75),
        "white" => Color::srgb(0.90, 0.90, 0.90),
        "black" => Color::srgb(0.10, 0.10, 0.12),
        _ => Color::srgb(0.70, 0.70, 0.50),
    }
}

/// The frame: tile `(x, y)` sits at world `(x - cx, _, -(y - cy))`, so north
/// (`+y` on the map) is `-Z`, the way the default camera looks.
#[derive(Resource, Clone, Copy)]
pub struct Frame {
    pub min: TilePos,
    pub max: TilePos,
}

impl Frame {
    pub fn centre(&self) -> Vec2 {
        Vec2::new(
            (self.min.x + self.max.x) as f32 / 2.0,
            (self.min.y + self.max.y) as f32 / 2.0,
        )
    }

    pub fn tile_xyz(&self, pos: TilePos, y: f32) -> Vec3 {
        let c = self.centre();
        Vec3::new(pos.x as f32 - c.x, y, -(pos.y as f32 - c.y))
    }

    /// The tile under a world point on the ground plane.
    pub fn tile_at(&self, x: f32, z: f32) -> TilePos {
        let c = self.centre();
        TilePos::new((x + c.x).round() as i32, (c.y - z).round() as i32)
    }

    pub fn contains(&self, p: TilePos) -> bool {
        (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y)
    }
}

/// Axis-aligned box with an explicit UV rectangle `[u0, v0, u1, v1]` per
/// face, ordered [front(-Z), right(+X), back(+Z), left(-X), top(+Y),
/// bottom(-Y)]. Front is -Z so it matches the lens child and the facing
/// math in the view; image-up on the top face is the machine's forward.
pub fn box_with_face_uvs(half: Vec3, face_uvs: [[f32; 4]; 6]) -> Mesh {
    // (outward normal, texture-right, texture-up), chosen so r x u = n.
    const AXES: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    ];
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for ((n, r, u), [u0, v0, u1, v1]) in AXES.into_iter().zip(face_uvs) {
        let center = n * n.abs().dot(half);
        let rv = r * r.abs().dot(half);
        let uv = u * u.abs().dot(half);
        let base = positions.len() as u32;
        for (p, tex) in [
            (center - rv - uv, [u0, v1]),
            (center + rv - uv, [u1, v1]),
            (center + rv + uv, [u1, v0]),
            (center - rv + uv, [u0, v0]),
        ] {
            positions.push(p.to_array());
            normals.push(n.to_array());
            uvs.push(tex);
        }
        indices.extend([base, base + 1, base + 2, base + 2, base + 3, base]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// A body box (bot, printer): each face samples its cell of a 3x2 atlas
/// (front/right/back over left/top/bottom — the layout the bake emits).
pub fn atlas_box_mesh(half: Vec3) -> Mesh {
    let cell = |c: f32, r: f32| [c / 3.0, r / 2.0, (c + 1.0) / 3.0, (r + 1.0) / 2.0];
    box_with_face_uvs(
        half,
        [
            cell(0.0, 0.0),
            cell(1.0, 0.0),
            cell(2.0, 0.0),
            cell(0.0, 1.0),
            cell(1.0, 1.0),
            cell(2.0, 1.0),
        ],
    )
}

/// Textured slab (terrain tile, depot pad): the full texture on top
/// (image-up = north); sides and bottom sample a sliver of the texture's
/// border so they read as dark trim.
pub fn textured_slab_mesh(half: Vec3) -> Mesh {
    const EDGE: [f32; 4] = [0.005, 0.45, 0.02, 0.55];
    box_with_face_uvs(half, [EDGE, EDGE, EDGE, EDGE, [0.0, 0.0, 1.0, 1.0], EDGE])
}

/// A raised rock block mapped into a 2-cell atlas (autotiled summit in the
/// left cell, cliff face in the right). It spans `WATER_SINK..height` when
/// spawned at [`block_y_off`], so the cliff reaches the plane.
pub fn block_mesh(height: f32) -> Mesh {
    const TOP: [f32; 4] = [0.0, 0.0, 0.5, 1.0];
    const SIDE: [f32; 4] = [0.5, 0.0, 1.0, 1.0];
    const EDGE: [f32; 4] = [0.51, 0.45, 0.53, 0.55];
    box_with_face_uvs(
        Vec3::new(0.48, block_y_off(height), 0.48),
        [SIDE, SIDE, SIDE, SIDE, TOP, EDGE],
    )
}

pub fn block_y_off(height: f32) -> f32 {
    (height - WATER_SINK) / 2.0
}

/// Everything the view spawns from, built once at startup.
#[derive(Resource)]
pub struct Palette {
    pub tile_slab: Handle<Mesh>,
    pub rock_block: Handle<Mesh>,
    pub bot_cube: Handle<Mesh>,
    pub printer_box: Handle<Mesh>,
    pub paper_sheet: Handle<Mesh>,
    pub crate_box: Handle<Mesh>,
    pub site_cube: Handle<Mesh>,
    pub cover_quad: Handle<Mesh>,
    pub mark_quad: Handle<Mesh>,
    pub overlay_cube: Handle<Mesh>,
    pub ring: Handle<Mesh>,
    pub team_ring: Handle<Mesh>,
    pub scribble_quad: Handle<Mesh>,
    pub bar: Handle<Mesh>,
    pub lens_barrel: Handle<Mesh>,
    pub lens_glass: Handle<Mesh>,

    pub ground_mat: Handle<StandardMaterial>,
    pub ground_frames: Vec<Handle<Image>>,
    /// Per autotile mask, its material; the frames rotate through them.
    pub water_mats: Vec<Handle<StandardMaterial>>,
    pub water_frames: Vec<Vec<Handle<Image>>>,
    pub ore_mats: Vec<Handle<StandardMaterial>>,
    pub ore_frames: Vec<Vec<Handle<Image>>>,
    pub rock_mats: Vec<Handle<StandardMaterial>>,
    /// Bot atlases by deployment color name (Q35), `white` for none.
    pub bot_mats: HashMap<String, Handle<StandardMaterial>>,
    /// The white printer atlas, tinted per team on demand.
    pub printer_mat: Handle<StandardMaterial>,
    pub printer_ruined_mat: Handle<StandardMaterial>,
    pub lens_barrel_mat: Handle<StandardMaterial>,
    pub scribble_mats: Vec<Handle<StandardMaterial>>,
    pub paper_mat: Handle<StandardMaterial>,
    pub crate_mat: Handle<StandardMaterial>,
    pub site_mat: Handle<StandardMaterial>,
    pub unknown_mat: Handle<StandardMaterial>,
    pub paint_mats: HashMap<String, Handle<StandardMaterial>>,
    pub overlay_mat: Handle<StandardMaterial>,
    pub plan_building_mat: Handle<StandardMaterial>,
    pub plan_paint_mat: Handle<StandardMaterial>,
    pub plan_overlay_mat: Handle<StandardMaterial>,
    pub select_mat: Handle<StandardMaterial>,
    pub hover_mat: Handle<StandardMaterial>,
    pub ring_mats: Vec<Handle<StandardMaterial>>,
    pub bar_bg_mat: Handle<StandardMaterial>,
    pub bar_fill_mat: Handle<StandardMaterial>,
    /// Memory twins: the live material's id to its darkened, unlit copy.
    pub dim: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
    /// Team tints: a base material and a team's ring color to the tinted
    /// copy — buildings, rings and lens glass are these.
    pub tints: HashMap<(AssetId<StandardMaterial>, u32), Handle<StandardMaterial>>,
    pub ring_mat: Handle<StandardMaterial>,
    pub lens_glass_mat: Handle<StandardMaterial>,
}

impl Palette {
    pub fn build(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        assets: &AssetServer,
    ) -> Palette {
        let tex = |materials: &mut Assets<StandardMaterial>, image: Handle<Image>, rough: f32| {
            materials.add(StandardMaterial {
                base_color_texture: Some(image),
                perceptual_roughness: rough,
                ..default()
            })
        };
        let atlas = |materials: &mut Assets<StandardMaterial>, image: Handle<Image>| {
            materials.add(StandardMaterial {
                base_color_texture: Some(image.clone()),
                emissive: LinearRgba::new(0.35, 0.35, 0.35, 1.0),
                emissive_texture: Some(image),
                metallic: 0.1,
                perceptual_roughness: 0.5,
                ..default()
            })
        };
        let plain = |materials: &mut Assets<StandardMaterial>, c: Color, unlit: bool| {
            let mut m = StandardMaterial::from(c);
            m.unlit = unlit;
            if c.alpha() < 1.0 {
                m.alpha_mode = AlphaMode::Blend;
            }
            materials.add(m)
        };
        let frames = |prefix: &str| -> Vec<Vec<Handle<Image>>> {
            (0..3)
                .map(|f| {
                    (0..16)
                        .map(|mask| assets.load(format!("textures/{prefix}_{mask}_f{f}.png")))
                        .collect()
                })
                .collect()
        };
        let ground_frames: Vec<Handle<Image>> = (0..3)
            .map(|f| assets.load(format!("textures/tile_ground_f{f}.png")))
            .collect();
        let water_frames = frames("tile_water");
        let ore_frames = frames("tile_ore");
        let water_mats = water_frames[0]
            .iter()
            .map(|h| tex(materials, h.clone(), 0.25))
            .collect();
        let ore_mats = ore_frames[0]
            .iter()
            .map(|h| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(h.clone()),
                    emissive: LinearRgba::new(0.25, 0.18, 0.05, 1.0),
                    emissive_texture: Some(h.clone()),
                    metallic: 0.2,
                    perceptual_roughness: 0.6,
                    ..default()
                })
            })
            .collect();
        let rock_mats = (0..16)
            .map(|mask| {
                tex(
                    materials,
                    assets.load(format!("textures/rock_atlas_{mask}.png")),
                    0.9,
                )
            })
            .collect();
        let bot_mats = ATLASES
            .iter()
            .map(|t| {
                (
                    t.to_string(),
                    atlas(
                        materials,
                        assets.load(format!("textures/bot_atlas_{t}.png")),
                    ),
                )
            })
            .collect();
        let printer_mat = atlas(materials, assets.load("textures/printer_atlas_white.png"));
        let scribble_mats = (0..3)
            .map(|f| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(
                        assets.load(format!("textures/scribble_error_f{f}.png")),
                    ),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                })
            })
            .collect();
        let paper_tex: Handle<Image> = assets.load("textures/paper.png");
        let paint_mats = [
            "red", "blue", "green", "yellow", "purple", "orange", "cyan", "magenta", "white",
            "black",
        ]
        .into_iter()
        .map(|n| {
            (
                n.to_string(),
                plain(materials, paint_color(n).with_alpha(0.55), false),
            )
        })
        .collect();
        let ring_mats = (0..6)
            .map(|i| {
                plain(
                    materials,
                    Color::srgba(1.0, 0.95, 0.8, 0.6 * (1.0 - i as f32 / 6.0)),
                    true,
                )
            })
            .collect();
        Palette {
            tile_slab: meshes.add(textured_slab_mesh(Vec3::new(0.48, 0.05, 0.48))),
            rock_block: meshes.add(block_mesh(ROCK_HEIGHT)),
            bot_cube: meshes.add(atlas_box_mesh(Vec3::splat(BOT_HALF))),
            printer_box: meshes.add(atlas_box_mesh(Vec3::new(0.45, 0.25, 0.45))),
            paper_sheet: meshes.add(Cuboid::new(0.36, 0.26, 0.02)),
            crate_box: meshes.add(textured_slab_mesh(Vec3::new(0.39, 0.15, 0.39))),
            site_cube: meshes.add(Cuboid::new(0.7, 0.7, 0.7)),
            cover_quad: meshes.add(Cuboid::new(1.0, 0.02, 1.0)),
            mark_quad: meshes.add(Cuboid::new(0.9, 0.02, 0.9)),
            overlay_cube: meshes.add(Cuboid::new(0.22, 0.22, 0.22)),
            ring: meshes.add(Annulus::new(0.36, 0.44)),
            team_ring: meshes.add(Annulus::new(0.40, 0.47)),
            scribble_quad: meshes.add(Rectangle::new(1.05, 0.85)),
            bar: meshes.add(Cuboid::new(0.9, 0.12, 0.01)),
            lens_barrel: meshes.add(Cylinder::new(LENS_BARREL_RADIUS, LENS_BARREL_LEN)),
            lens_glass: meshes.add(Cylinder::new(LENS_GLASS_RADIUS, LENS_GLASS_LEN)),
            ground_mat: tex(materials, ground_frames[0].clone(), 0.95),
            ground_frames,
            water_mats,
            water_frames,
            ore_mats,
            ore_frames,
            rock_mats,
            bot_mats,
            printer_mat,
            printer_ruined_mat: atlas(materials, assets.load("textures/printer_atlas_ruined.png")),
            lens_barrel_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.11, 0.14, 0.18),
                metallic: 0.7,
                perceptual_roughness: 0.35,
                ..default()
            }),
            scribble_mats,
            lens_glass_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.1, 0.12),
                emissive: LinearRgba::new(1.6, 1.6, 1.6, 1.0),
                metallic: 0.3,
                perceptual_roughness: 0.15,
                ..default()
            }),
            ring_mat: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(0.5, 0.5, 0.5, 1.0),
                unlit: false,
                ..default()
            }),
            tints: HashMap::new(),
            paper_mat: materials.add(StandardMaterial {
                base_color_texture: Some(paper_tex.clone()),
                emissive: LinearRgba::new(0.25, 0.25, 0.25, 1.0),
                emissive_texture: Some(paper_tex),
                perceptual_roughness: 0.9,
                ..default()
            }),
            crate_mat: tex(materials, assets.load("textures/crate.png"), 0.9),
            site_mat: plain(materials, Color::srgba(0.85, 0.95, 1.0, 0.45), false),
            unknown_mat: plain(materials, Color::srgb(0.008, 0.008, 0.016), true),
            paint_mats,
            overlay_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 0.9),
                emissive: LinearRgba::new(0.6, 0.6, 0.5, 1.0),
                ..default()
            }),
            plan_building_mat: plain(materials, Color::srgba(0.95, 0.9, 0.3, 0.45), true),
            plan_paint_mat: plain(materials, Color::srgba(0.95, 0.3, 0.9, 0.45), true),
            plan_overlay_mat: plain(materials, Color::srgba(0.3, 0.95, 0.95, 0.45), true),
            select_mat: plain(materials, Color::srgba(0.4, 0.85, 0.95, 0.5), true),
            hover_mat: plain(materials, Color::srgba(1.0, 1.0, 1.0, 0.18), true),
            ring_mats,
            bar_bg_mat: plain(materials, Color::srgb(0.05, 0.05, 0.06), true),
            bar_fill_mat: plain(materials, Color::srgb(0.9, 0.2, 0.15), true),
            dim: HashMap::new(),
        }
    }

    /// A copy of `base` in the team's color: the base color, and the
    /// emissive if the base glows, multiplied by `color`. The player's own
    /// team is white, so its copy is the base.
    pub fn tinted(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        base: &Handle<StandardMaterial>,
        team: TeamId,
        color: Color,
    ) -> Handle<StandardMaterial> {
        if let Some(t) = self.tints.get(&(base.id(), team.0)) {
            return t.clone();
        }
        let Some(src) = materials.get(base) else {
            return base.clone();
        };
        let mut m = src.clone();
        let c = color.to_linear();
        let b = m.base_color.to_linear();
        m.base_color =
            LinearRgba::new(b.red * c.red, b.green * c.green, b.blue * c.blue, b.alpha).into();
        let e = m.emissive;
        m.emissive = LinearRgba::new(e.red * c.red, e.green * c.green, e.blue * c.blue, e.alpha);
        let t = materials.add(m);
        self.tints.insert((base.id(), team.0), t.clone());
        t
    }

    /// The memory twin of a live material: darkened, blue-shifted, unlit.
    pub fn dim(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        live: &Handle<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(d) = self.dim.get(&live.id()) {
            return d.clone();
        }
        let Some(src) = materials.get(live) else {
            return live.clone();
        };
        let mut m = src.clone();
        let c = m.base_color.to_linear();
        m.base_color = LinearRgba::new(c.red * 0.14, c.green * 0.15, c.blue * 0.21, c.alpha).into();
        m.unlit = true;
        m.emissive = LinearRgba::BLACK;
        m.emissive_texture = None;
        let d = materials.add(m);
        self.dim.insert(live.id(), d.clone());
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two bots on adjacent tiles staring at each other must not touch:
    /// everything past the face plane has to fit in half the inter-face
    /// gap with daylight to spare.
    #[test]
    fn facing_lenses_never_clip() {
        let tip = LENS_GLASS_Z - LENS_GLASS_LEN / 2.0;
        let protrusion = -tip - BOT_HALF;
        let gap = 1.0 - 2.0 * BOT_HALF;
        assert!(protrusion * 2.0 < gap * 0.9, "{protrusion} vs {gap}");
    }

    #[test]
    fn frame_round_trips_a_tile() {
        let f = Frame {
            min: TilePos::new(-12, -8),
            max: TilePos::new(11, 7),
        };
        for p in [
            TilePos::new(-12, -8),
            TilePos::new(0, 0),
            TilePos::new(11, 7),
        ] {
            let w = f.tile_xyz(p, 0.0);
            assert_eq!(f.tile_at(w.x, w.z), p);
        }
        // North is -Z.
        assert!(f.tile_xyz(TilePos::new(0, 5), 0.0).z < f.tile_xyz(TilePos::new(0, 0), 0.0).z);
    }
}
