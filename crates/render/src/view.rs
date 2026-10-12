//! The view: entities built from the completed-tick snapshot, never shared
//! with the sim (Q15). Tiles are textured slabs rebuilt when their look
//! changes; machines are atlas boxes diffed by entity id; positions
//! interpolate between the last two snapshots in floats nothing reads back.
//! The look is the predecessor's: fog as an opaque cover over the unknown
//! and a cold, unlit twin of every material for the remembered.

use crate::app::{DriverResource, Screen, Tool, ViewState};
use crate::camera::{OrbitCam, orbit_transform};
use crate::driver::Driver;
use crate::outlines::{TeamColors, TeamMats};
use crate::palette::{
    BOT_HALF, Frame, Interface, LENS_BARREL_Z, LENS_GLASS_Z, LENS_Y, MARK_HALF, Palette,
    ROCK_HEIGHT, SLAB_HALF, WATER_SINK, block_y_off, body_half, tint,
};
use bevy::prelude::*;
use sim::map::Terrain;
use sim::snapshot::{MachineSnapshot, Snapshot};
use sim::world::{Model, Plan, Plans, TileMemory, TileState};
use sim::{EntityId, TilePos};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

/// What a tile looked like when its entities were last built.
#[derive(Clone, PartialEq, Eq)]
pub struct TileLook {
    pub state: TileState,
    pub terrain: Terrain,
    pub ore: bool,
    pub mask: u8,
    pub paint: Option<String>,
    pub overlay: Option<String>,
    /// The player's team's plans here, drawn as their effect (Q37).
    pub plans: Option<Plans>,
}

pub struct TileView {
    pub look: TileLook,
    pub entities: Vec<Entity>,
}

#[derive(Component)]
pub struct FogCover;

#[derive(Component)]
pub struct MachineBody(pub EntityId);

/// A bar that always faces the camera.
#[derive(Component)]
pub struct BillboardBar;

#[derive(Component)]
pub struct SoundRing {
    pub age: f32,
}

#[derive(Component)]
pub struct SelectMarker;

/// The scribble over a faulting machine (`docs/07`, Q34).
#[derive(Component)]
pub struct FaultMark;

#[derive(Component)]
pub struct HoverMarker;

pub struct MachineView {
    pub body: Entity,
    pub model: Model,
    pub live_mat: Handle<StandardMaterial>,
    pub dimmed: bool,
    pub shown: bool,
    pub health_seen: lang::Num,
    pub health_age: f32,
    pub bar_root: Entity,
    pub bar_fill: Entity,
    pub mark: Entity,
    pub yaw: f32,
    pub yaw_prev: f32,
    pub pos_seen: TilePos,
    /// The bot's `color` as last drawn (Q43); `None` on a building.
    pub color_seen: Option<Option<String>>,
}

/// The renderer's own index from snapshot ids to its entities.
#[derive(Resource, Default)]
pub struct Entities {
    pub tiles: HashMap<TilePos, TileView>,
    pub covers: HashMap<TilePos, Entity>,
    pub machines: HashMap<EntityId, MachineView>,
    /// The render height of each tile's top as the player knows it.
    pub tops: HashMap<TilePos, f32>,
    pub synced_tick: Option<u64>,
    pub sounds_tick: Option<u64>,
    /// The armed tool's ghost: what it shows, and on which tiles — the one
    /// under the cursor, or a `Shift`+drag's rectangle (Q44).
    pub cursor: (Tool, Vec<TilePos>),
    pub cursor_entities: Vec<Entity>,
}

impl Entities {
    pub fn top(&self, p: TilePos) -> f32 {
        self.tops.get(&p).copied().unwrap_or(0.0)
    }
}

/// Each model's full health, for the bars; from `data/machines.toml`.
#[derive(Resource)]
pub struct Tuning {
    pub max_health: HashMap<&'static str, f32>,
}

impl Tuning {
    pub fn load() -> Tuning {
        let mut max_health = HashMap::new();
        if let Ok(d) = sim::Data::load() {
            for m in [Model::Bot, Model::Printer, Model::Depot, Model::Site] {
                if let Some(md) = d.machines.models.get(m.name()) {
                    max_health.insert(m.name(), num_f32(md.health).max(1.0));
                }
            }
        }
        Tuning { max_health }
    }

    pub fn fraction(&self, m: &MachineSnapshot) -> f32 {
        let max = self
            .max_health
            .get(m.record.model.name())
            .copied()
            .unwrap_or(10.0);
        (num_f32(m.record.health) / max).clamp(0.0, 1.0)
    }
}

/// A `num` as a float, for display only.
pub fn num_f32(n: lang::Num) -> f32 {
    (n.raw() as f64 / 1e12) as f32
}

/// How high a model's body centre sits above its tile's top.
fn body_lift(model: Model) -> f32 {
    match model {
        Model::Bot => BOT_HALF,
        Model::Printer => 0.25,
        Model::Depot => 0.15,
        Model::Site => 0.35,
    }
}

fn bar_height(model: Model) -> f32 {
    match model {
        Model::Bot => 0.85,
        Model::Printer => 0.75,
        Model::Depot => 0.6,
        Model::Site => 0.85,
    }
}

/// Spawn the palette, the lights, the camera, the fog covers and the
/// markers; the tiles and machines follow from the first snapshot.
pub fn setup(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    frame: Res<Frame>,
    mut ents: ResMut<Entities>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    let palette = Palette::build(&mut meshes, &mut materials, &assets);
    let snap = driver.0.snapshot();
    let player = driver.0.player;
    if let Some(team) = snap.teams.iter().find(|t| t.id == player) {
        for (p, _) in &team.tiles {
            let e = commands
                .spawn((
                    FogCover,
                    DespawnOnEnter(Screen::Start),
                    Mesh3d(palette.cover_quad.clone()),
                    MeshMaterial3d(palette.unknown_mat.clone()),
                    Transform::from_translation(frame.tile_xyz(*p, 0.04)),
                ))
                .id();
            ents.covers.insert(*p, e);
        }
    }
    for (marker, mat) in [
        (true, palette.select_mat.clone()),
        (false, palette.hover_mat.clone()),
    ] {
        let mut e = commands.spawn((
            DespawnOnEnter(Screen::Start),
            Mesh3d(palette.mark_quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Hidden,
        ));
        if marker {
            e.insert(SelectMarker);
        } else {
            e.insert(HoverMarker);
        }
    }
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.75, 0.78, 0.92),
        brightness: 250.0,
        ..default()
    });
    commands.spawn((
        DespawnOnEnter(Screen::Start),
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(6.0, 14.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Start over the player's printer.
    let home = snap
        .machines
        .iter()
        .find(|m| m.record.team == player && m.record.model == Model::Printer)
        .map(|m| frame.tile_xyz(m.record.pos, 0.0))
        .unwrap_or(Vec3::ZERO);
    let cam = OrbitCam {
        focus: home,
        distance: 22.0,
        yaw: 0.0,
        pitch: 0.85,
    };
    if let Ok(e) = cameras.single() {
        commands.entity(e).insert((orbit_transform(&cam), cam));
    }
    commands.insert_resource(palette);
}

fn look_of(
    p: TilePos,
    mem: &TileMemory,
    memory: &HashMap<TilePos, &TileMemory>,
    plans: Option<Plans>,
) -> TileLook {
    let ore = mem.deposit.is_some();
    // A NESW same-neighbour mask (bit 0 = N … bit 3 = W); off the map or
    // unknown counts as same, so the art runs off clean and gives nothing
    // away.
    let same = |n: TilePos| -> bool {
        match memory.get(&n) {
            None => true,
            Some(m) if m.state == TileState::Unknown => true,
            Some(m) => match mem.terrain {
                Terrain::Ground => !ore || m.deposit.is_some(),
                t => m.terrain == t,
            },
        }
    };
    let mut mask = 0u8;
    for (bit, (dx, dy)) in [(0, 1), (1, 0), (0, -1), (-1, 0)].into_iter().enumerate() {
        if same(TilePos::new(p.x.wrapping_add(dx), p.y.wrapping_add(dy))) {
            mask |= 1 << bit;
        }
    }
    TileLook {
        state: mem.state,
        terrain: mem.terrain,
        ore,
        mask,
        paint: mem.paint.clone(),
        overlay: mem.overlay.clone(),
        plans,
    }
}

fn top_of(look: &TileLook) -> f32 {
    match look.terrain {
        Terrain::Rock => ROCK_HEIGHT,
        Terrain::Water => WATER_SINK,
        Terrain::Ground => 0.0,
    }
}

/// The height of a tile's upper face, which a flat mark must clear to be
/// seen: a rock block's summit, or a slab's top, half its thickness above
/// where the slab is spawned.
fn surface_of(look: &TileLook) -> f32 {
    match look.terrain {
        Terrain::Rock => ROCK_HEIGHT,
        Terrain::Water => WATER_SINK + SLAB_HALF,
        Terrain::Ground => SLAB_HALF,
    }
}

/// The face a flat marker on `p` must clear, as drawn now: the tile's own
/// surface, or the fog cover's top, which is a slab's, on an unknown tile.
fn surface_at(ents: &Entities, p: TilePos) -> f32 {
    ents.tiles
        .get(&p)
        .filter(|t| t.look.state != TileState::Unknown)
        .map_or(SLAB_HALF, |t| surface_of(&t.look))
}

/// Rebuild every tile whose look changed since the last snapshot (Q21):
/// unknown under the cover, remembered in the cold twin, visible live.
pub fn sync_tiles(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    frame: Res<Frame>,
    mut palette: ResMut<Palette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ents: ResMut<Entities>,
    mut covers: Query<&mut Visibility, With<FogCover>>,
) {
    let snap = driver.0.snapshot();
    if ents.synced_tick == Some(snap.tick) {
        return;
    }
    ents.synced_tick = Some(snap.tick);
    let player = driver.0.player;
    let Some(team) = snap.teams.iter().find(|t| t.id == player) else {
        return;
    };
    let memory: HashMap<TilePos, &TileMemory> = team.tiles.iter().map(|(p, m)| (*p, m)).collect();
    let plans: HashMap<TilePos, &Plans> = team.plans.iter().map(|(p, x)| (*p, x)).collect();
    for (p, mem) in &team.tiles {
        let look = look_of(*p, mem, &memory, plans.get(p).map(|x| (*x).clone()));
        if let Some(&c) = ents.covers.get(p)
            && let Ok(mut v) = covers.get_mut(c)
        {
            *v = if look.state == TileState::Unknown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        ents.tops.insert(
            *p,
            if look.state == TileState::Unknown {
                0.0
            } else {
                top_of(&look).max(0.0)
            },
        );
        if ents.tiles.get(p).is_some_and(|t| t.look == look) {
            continue;
        }
        if let Some(old) = ents.tiles.remove(p) {
            for e in old.entities {
                commands.entity(e).despawn();
            }
        }
        let entities = spawn_tile(
            &mut commands,
            &frame,
            &mut palette,
            &mut materials,
            *p,
            &look,
        );
        ents.tiles.insert(*p, TileView { look, entities });
    }
}

/// A tile's plans as a translucent version of their effect — the paint's
/// color, the overlay's marker, the building — and a plan to clear a slot
/// as that slot hatched (Q37). The overlay's label is painted over the map
/// by the UI. `surface` is the tile's upper face ([`surface_of`]); the
/// paint plan sits just above any applied paint there.
/// How much smaller than the real thing a plan's preview is (Q50).
const PLAN_SCALE: f32 = 0.6;

fn spawn_plans(
    commands: &mut Commands,
    frame: &Frame,
    palette: &mut Palette,
    materials: &mut Assets<StandardMaterial>,
    p: TilePos,
    top: f32,
    surface: f32,
    plans: &Plans,
) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut spawn = |mesh: Handle<Mesh>, mat: Handle<StandardMaterial>, t: Transform| {
        out.push(
            commands
                .spawn((
                    DespawnOnEnter(Screen::Start),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    t,
                ))
                .id(),
        );
    };
    if let Some(plan) = &plans.paint {
        let at = Transform::from_translation(frame.tile_xyz(p, surface + MARK_HALF + 0.01));
        let (mat, t) = match plan {
            // A smaller version of the paint it plans (Q50): the new color
            // in a square inset from the tile's edge, so any paint already
            // there shows round it rather than mixing with it.
            Plan::Set(c) => {
                let live = palette
                    .paint_mats
                    .get(c)
                    .cloned()
                    .unwrap_or_else(|| palette.paint_mats["white"].clone());
                (
                    palette.solid(materials, &live),
                    at.with_scale(Vec3::new(PLAN_SCALE, 1.0, PLAN_SCALE)),
                )
            }
            Plan::Clear => (palette.hatch_mat.clone(), at),
        };
        spawn(palette.mark_quad.clone(), mat, t);
    }
    if let Some(plan) = &plans.overlay {
        let mat = match plan {
            Plan::Set(_) => {
                let live = palette.overlay_mat.clone();
                palette.ghost(materials, &live)
            }
            Plan::Clear => palette.hatch_mat.clone(),
        };
        spawn(
            palette.overlay_cube.clone(),
            mat,
            Transform::from_translation(frame.tile_xyz(p, top + 0.16))
                .with_rotation(Quat::from_rotation_y(0.6)),
        );
    }
    if let Some(plan) = &plans.building {
        let model = match plan {
            Plan::Set(m) => Model::from_name(m).unwrap_or(Model::Depot),
            Plan::Clear => Model::Depot,
        };
        // A smaller, translucent version of the building it plans (Q50),
        // standing on the tile.
        let (mesh, live) = match model {
            Model::Site => (palette.site_cube.clone(), palette.site_mat.clone()),
            Model::Printer => (palette.printer_box.clone(), palette.printer_mat.clone()),
            _ => (palette.crate_box.clone(), palette.crate_mat.clone()),
        };
        let (mat, scale) = match plan {
            Plan::Set(_) => (palette.ghost(materials, &live), PLAN_SCALE),
            Plan::Clear => (palette.hatch_mat.clone(), 1.0),
        };
        spawn(
            mesh,
            mat,
            Transform::from_translation(frame.tile_xyz(p, top + body_lift(model) * scale))
                .with_scale(Vec3::splat(scale)),
        );
    }
    out
}

/// The armed tool's ghost under the cursor, or over a `Shift`+drag's
/// rectangle: what marking there would plan (Q37, Q44), rebuilt when the
/// tool or the tiles change.
/// The tiles the armed tool's ghost covers ([`ViewState::preview`]) that
/// it can plan: a paint or overlay tool none that is rock, water or still
/// unknown to the team (Q49).
pub fn plannable_preview(state: &ViewState, driver: &Driver) -> Vec<TilePos> {
    let mut tiles = state.preview();
    if let Tool::Mark(plan, _) = &state.tool {
        tiles.retain(|p| driver.may_plan(*p, *plan).is_ok());
    }
    tiles
}

pub fn cursor_ghost(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    state: Res<ViewState>,
    frame: Res<Frame>,
    mut palette: ResMut<Palette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ents: ResMut<Entities>,
) {
    let want = (state.tool.clone(), plannable_preview(&state, &driver.0));
    if ents.cursor == want {
        return;
    }
    for e in std::mem::take(&mut ents.cursor_entities) {
        commands.entity(e).despawn();
    }
    if let Some(plans) = crate::marks::ghost_of(&want.0) {
        for p in &want.1 {
            let top = ents.top(*p);
            let surface = surface_at(&ents, *p);
            let spawned = spawn_plans(
                &mut commands,
                &frame,
                &mut palette,
                &mut materials,
                *p,
                top,
                surface,
                &plans,
            );
            ents.cursor_entities.extend(spawned);
        }
    }
    ents.cursor = want;
}

fn spawn_tile(
    commands: &mut Commands,
    frame: &Frame,
    palette: &mut Palette,
    materials: &mut Assets<StandardMaterial>,
    p: TilePos,
    look: &TileLook,
) -> Vec<Entity> {
    let mut out = Vec::new();
    if look.state == TileState::Unknown {
        return out;
    }
    let remembered = look.state == TileState::Remembered;
    // The live handle is cloned before the palette is borrowed for its twin.
    let mut mat = |palette: &mut Palette, live: Handle<StandardMaterial>| {
        if remembered {
            palette.dim(materials, &live)
        } else {
            live
        }
    };
    let mask = look.mask as usize;
    let (mesh, live, y) = match look.terrain {
        Terrain::Rock => (
            palette.rock_block.clone(),
            palette.rock_mats[mask].clone(),
            block_y_off(ROCK_HEIGHT) + WATER_SINK,
        ),
        Terrain::Water => (
            palette.tile_slab.clone(),
            palette.water_mats[mask].clone(),
            WATER_SINK,
        ),
        Terrain::Ground if look.ore => (
            palette.tile_slab.clone(),
            palette.ore_mats[mask].clone(),
            0.0,
        ),
        Terrain::Ground => (palette.tile_slab.clone(), palette.ground_mat.clone(), 0.0),
    };
    let material = mat(palette, live);
    out.push(
        commands
            .spawn((
                DespawnOnEnter(Screen::Start),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(frame.tile_xyz(p, y)),
            ))
            .id(),
    );
    let top = top_of(look);
    let surface = surface_of(look);
    if let Some(paint) = &look.paint {
        let live = palette
            .paint_mats
            .get(paint)
            .cloned()
            .unwrap_or_else(|| palette.paint_mats["white"].clone());
        let material = mat(palette, live);
        out.push(
            commands
                .spawn((
                    DespawnOnEnter(Screen::Start),
                    Mesh3d(palette.mark_quad.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(frame.tile_xyz(p, surface + MARK_HALF + 0.005)),
                ))
                .id(),
        );
    }
    if look.overlay.is_some() {
        let material = mat(palette, palette.overlay_mat.clone());
        out.push(
            commands
                .spawn((
                    DespawnOnEnter(Screen::Start),
                    Mesh3d(palette.overlay_cube.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(frame.tile_xyz(p, top + 0.16))
                        .with_rotation(Quat::from_rotation_y(0.6)),
                ))
                .id(),
        );
    }
    if let Some(plans) = &look.plans {
        out.extend(spawn_plans(
            commands, frame, palette, materials, p, top, surface, plans,
        ));
    }
    out
}

/// Whether the player's team sees the machine: its own always; another
/// team's while its tile is visible, or remembered with that building on
/// it. The snapshot holds every machine for the replay tool, so the fog is
/// the renderer's to honour (Q21).
fn visibility(snap: &Snapshot, player: sim::TeamId, m: &MachineSnapshot) -> Option<TileState> {
    if m.record.team == player {
        return Some(TileState::Visible);
    }
    let team = snap.teams.iter().find(|t| t.id == player)?;
    let (_, mem) = team.tiles.iter().find(|(p, _)| *p == m.record.pos)?;
    match mem.state {
        TileState::Visible => Some(TileState::Visible),
        TileState::Remembered
            if m.record.model != Model::Bot
                && mem.building.as_ref().is_some_and(|b| b.id == m.record.id) =>
        {
            Some(TileState::Remembered)
        }
        _ => None,
    }
}

/// Diff the machines: spawn new, despawn gone, fog and face the rest.
pub fn sync_machines(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    frame: Res<Frame>,
    interface: Res<Interface>,
    mut colors: ResMut<TeamColors>,
    mut palette: ResMut<Palette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ents: ResMut<Entities>,
    mut bodies: Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<MachineBody>>,
) {
    let snap = driver.0.snapshot();
    let player = driver.0.player;
    let seen: Vec<EntityId> = snap.machines.iter().map(|m| m.record.id).collect();
    for m in &snap.machines {
        let state = visibility(snap, player, m);
        let shown = state.is_some();
        let dimmed = state == Some(TileState::Remembered);
        let id = m.record.id;
        if let Some(view) = ents.machines.get_mut(&id) {
            if let Ok((mut vis, mut mat)) = bodies.get_mut(view.body) {
                // A bot repainted by its program (Q43) wears its new color.
                if m.record.color != view.color_seen {
                    view.live_mat =
                        body_mat(&palette, &interface, &mut colors, &mut materials, player, m);
                    view.color_seen = m.record.color.clone();
                    mat.0 = if dimmed {
                        palette.dim(&mut materials, &view.live_mat)
                    } else {
                        view.live_mat.clone()
                    };
                    view.dimmed = dimmed;
                }
                if view.shown != shown {
                    *vis = if shown {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                    view.shown = shown;
                }
                if view.dimmed != dimmed {
                    mat.0 = if dimmed {
                        palette.dim(&mut materials, &view.live_mat)
                    } else {
                        view.live_mat.clone()
                    };
                    view.dimmed = dimmed;
                }
            }
            if m.record.pos != view.pos_seen {
                let from = frame.tile_xyz(view.pos_seen, 0.0);
                let to = frame.tile_xyz(m.record.pos, 0.0);
                let d = to - from;
                view.yaw_prev = view.yaw;
                // The face is on -Z; lead with it.
                view.yaw = (-d.x).atan2(-d.z);
                view.pos_seen = m.record.pos;
            }
            if m.record.health != view.health_seen {
                view.health_seen = m.record.health;
                view.health_age = 0.0;
            }
            continue;
        }
        let view = spawn_machine(
            &mut commands,
            &frame,
            &interface,
            &mut colors,
            player,
            &mut palette,
            &mut materials,
            &ents,
            m,
            shown,
            dimmed,
        );
        ents.machines.insert(id, view);
    }
    let gone: Vec<EntityId> = ents
        .machines
        .keys()
        .filter(|id| !seen.contains(id))
        .copied()
        .collect();
    for id in gone {
        if let Some(v) = ents.machines.remove(&id) {
            commands.entity(v.body).despawn();
        }
    }
}

/// A machine's body material, its lights glowing in its team's color: a
/// bot's atlas in its `color`, or the neutral one while it has none (Q43,
/// Q45); a printer's and a depot's own neutral art (Q47). A site, which
/// has no lights, is its own translucent material, outlined instead.
fn body_mat(
    palette: &Palette,
    interface: &Interface,
    colors: &mut TeamColors,
    materials: &mut Assets<StandardMaterial>,
    player: sim::TeamId,
    m: &MachineSnapshot,
) -> Handle<StandardMaterial> {
    let (key, base, lights, rough) = match m.record.model {
        Model::Bot => {
            let color = m.record.color.as_ref().and_then(|c| c.as_deref());
            let atlas = color
                .filter(|c| palette.bot_textures.contains_key(*c))
                .unwrap_or(&interface.bodies.neutral)
                .to_string();
            let base = palette.bot_textures[&atlas].clone();
            (
                format!("bot_{atlas}"),
                base,
                palette.bot_lights.clone(),
                0.5,
            )
        }
        Model::Printer => (
            "printer".to_string(),
            palette.printer_texture.clone(),
            palette.printer_lights.clone(),
            0.5,
        ),
        Model::Depot => (
            "depot".to_string(),
            palette.crate_texture.clone(),
            palette.crate_lights.clone(),
            0.9,
        ),
        Model::Site => return palette.site_mat.clone(),
    };
    let team = m.record.team;
    if let Some(h) = colors.bodies.get(&(team.0, key.clone())) {
        return h.clone();
    }
    let glow = colors.color(interface, player, team);
    let h = materials.add(StandardMaterial {
        base_color_texture: Some(base),
        emissive: glow_of(glow),
        emissive_texture: Some(lights),
        metallic: 0.1,
        perceptual_roughness: rough,
        ..default()
    });
    colors.bodies.insert((team.0, key), h.clone());
    h
}

/// How brightly a team's lights glow: its color, past white so it reads as
/// lit under any light.
fn glow_of(color: Color) -> LinearRgba {
    color.to_linear() * GLOW
}

const GLOW: f32 = 2.0;

/// The team's outline, lens-glass and lit-segment materials, shared by
/// all its machines and recolored in place when the player changes its
/// color.
fn team_mats(
    colors: &mut TeamColors,
    interface: &Interface,
    player: sim::TeamId,
    team: sim::TeamId,
    palette: &Palette,
    materials: &mut Assets<StandardMaterial>,
) -> TeamMats {
    if let Some(m) = colors.mats.get(&team.0) {
        return m.clone();
    }
    let color = colors.color(interface, player, team);
    let glass = match materials.get(&palette.lens_glass_mat) {
        Some(base) => materials.add(tint(base, color)),
        None => palette.lens_glass_mat.clone(),
    };
    let m = TeamMats {
        outline: materials.add(outline_material(color)),
        glass,
        segment: materials.add(segment_material(color)),
    };
    colors.mats.insert(team.0, m.clone());
    m
}

/// A lit segment: the team's color, glowing like the bot's lights.
fn segment_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        emissive: glow_of(color),
        ..default()
    }
}

/// Where the number's display sits on a bot's top face, in the art's
/// 64-unit square (`assets/art/bot_face_top.svg`, image up = the bot's
/// front): the bezel's center, and its cell, so the lattice's five cells
/// fill the bezel's forty units (Q46).
const DISPLAY_AT: (f32, f32) = (32.0, 41.0);
const CELL: f32 = 8.0;
/// A segment's thickness, and how much shorter than its cell it is so
/// neighbours do not touch, in art units.
const SEGMENT: (f32, f32) = (2.0, 2.6);

/// A bot's number: the fixed segments on its lid, and which turn of the
/// number they last showed (Q46).
#[derive(Component)]
pub struct BotDigits {
    pub id: u32,
    pub lit: Handle<StandardMaterial>,
    /// One per `digits::lattice()` edge, in its order.
    pub segments: Vec<Entity>,
    pub quarter: Option<u8>,
}

fn spawn_digits(
    parent: &mut bevy::ecs::relationship::RelatedSpawnerCommands<ChildOf>,
    palette: &Palette,
    lit: &Handle<StandardMaterial>,
    id: u32,
) {
    let unit = 2.0 * BOT_HALF / 64.0;
    let (t, short) = SEGMENT;
    let at = Vec3::new(
        (DISPLAY_AT.0 - 32.0) * unit,
        BOT_HALF + 0.003,
        (DISPLAY_AT.1 - 32.0) * unit,
    );
    let mut segments = Vec::new();
    parent
        .spawn((Transform::from_translation(at), Visibility::Inherited))
        .with_children(|display| {
            for ((x0, y0), (x1, y1)) in crate::digits::lattice() {
                // Cells to art units about the display's center; the
                // lattice's y is the bot's front, which is -Z.
                let mid = |a: i32, b: i32, c: f32| ((a + b) as f32 / 2.0 - c) * CELL;
                let x = mid(x0, x1, crate::digits::CENTER.0);
                let y = mid(y0, y1, crate::digits::CENTER.1);
                let (sx, sz) = if y0 == y1 {
                    (CELL - short, t)
                } else {
                    (t, CELL - short)
                };
                segments.push(
                    display
                        .spawn((
                            Mesh3d(palette.segment.clone()),
                            MeshMaterial3d(palette.segment_off_mat.clone()),
                            Transform::from_xyz(x * unit, 0.0, -y * unit).with_scale(Vec3::new(
                                sx * unit,
                                0.004,
                                sz * unit,
                            )),
                        ))
                        .id(),
                );
            }
        })
        .insert(BotDigits {
            id,
            lit: lit.clone(),
            segments,
            quarter: None,
        });
}

/// Light each bot's number for the turn that reads upright from the
/// camera: the segments stay where they are on the lid and only switch on
/// and off, as the bot turns or the camera orbits past halfway (Q46).
pub fn light_digits(
    cams: Query<&OrbitCam>,
    palette: Res<Palette>,
    bodies: Query<&Transform, With<MachineBody>>,
    mut displays: Query<(&mut BotDigits, &ChildOf)>,
    mut segments: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let Ok(cam) = cams.single() else { return };
    let lattice = crate::digits::lattice();
    for (mut d, child_of) in &mut displays {
        let Ok(body) = bodies.get(child_of.parent()) else {
            continue;
        };
        let (bot_yaw, _, _) = body.rotation.to_euler(EulerRot::YXZ);
        let quarter = crate::digits::quarter_for(cam.yaw - bot_yaw);
        if d.quarter == Some(quarter) {
            continue;
        }
        d.quarter = Some(quarter);
        let on = crate::digits::lit(&lattice, d.id, quarter);
        for (e, lit_here) in d.segments.iter().zip(on) {
            if let Ok(mut m) = segments.get_mut(*e) {
                m.0 = if lit_here {
                    d.lit.clone()
                } else {
                    palette.segment_off_mat.clone()
                };
            }
        }
    }
}

/// An inverted hull: drawn only from behind and a little larger than the
/// body, so it shows as a rim round the body's silhouette.
fn outline_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        cull_mode: Some(bevy::render::render_resource::Face::Front),
        ..default()
    }
}

/// The outline's thickness, in tiles, round every side of a body.
const OUTLINE: f32 = 0.05;

/// Recolor every team's lights, outline and lens glass after the player
/// changes a color (Q43, Q45).
pub fn recolor_teams(
    driver: NonSend<DriverResource>,
    interface: Res<Interface>,
    palette: Res<Palette>,
    mut colors: ResMut<TeamColors>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !colors.dirty {
        return;
    }
    colors.dirty = false;
    let player = driver.0.player;
    let base = materials.get(&palette.lens_glass_mat).cloned();
    for ((team, _), body) in &colors.bodies {
        let color = colors.color(&interface, player, sim::TeamId(*team));
        if let Some(mut m) = materials.get_mut(body) {
            m.emissive = glow_of(color);
        }
    }
    for (team, m) in &colors.mats {
        let color = colors.color(&interface, player, sim::TeamId(*team));
        if let Some(mut o) = materials.get_mut(&m.outline) {
            o.base_color = color;
        }
        if let (Some(base), Some(mut g)) = (&base, materials.get_mut(&m.glass)) {
            *g = tint(base, color);
        }
        if let Some(mut seg) = materials.get_mut(&m.segment) {
            *seg = segment_material(color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_machine(
    commands: &mut Commands,
    frame: &Frame,
    interface: &Interface,
    colors: &mut TeamColors,
    player: sim::TeamId,
    palette: &mut Palette,
    materials: &mut Assets<StandardMaterial>,
    ents: &Entities,
    m: &MachineSnapshot,
    shown: bool,
    dimmed: bool,
) -> MachineView {
    let model = m.record.model;
    let team = m.record.team;
    let mesh = match model {
        Model::Bot => palette.bot_cube.clone(),
        Model::Printer => palette.printer_box.clone(),
        Model::Depot => palette.crate_box.clone(),
        Model::Site => palette.site_cube.clone(),
    };
    let live_mat = body_mat(palette, interface, colors, materials, player, m);
    let TeamMats {
        outline: outline_mat,
        glass: glass_mat,
        segment: segment_mat,
    } = team_mats(colors, interface, player, team, palette, materials);
    let half = body_half(model);
    let outline_scale = (half + Vec3::splat(OUTLINE)) / half;
    let mat = if dimmed {
        palette.dim(materials, &live_mat)
    } else {
        live_mat.clone()
    };
    let start = frame.tile_xyz(m.record.pos, ents.top(m.record.pos) + body_lift(model));
    // Printers face the default camera (the atlas front is -Z).
    let yaw = if model == Model::Printer { PI } else { 0.0 };
    let mut bar_root = Entity::PLACEHOLDER;
    let mut bar_fill = Entity::PLACEHOLDER;
    let mut mark = Entity::PLACEHOLDER;
    let mesh_for_outline = mesh.clone();
    let body = commands
        .spawn((
            MachineBody(m.record.id),
            DespawnOnEnter(Screen::Start),
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(start).with_rotation(Quat::from_rotation_y(yaw)),
            if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ))
        .with_children(|parent| {
            match model {
                Model::Bot => {
                    // The lens on the -Z face; cylinders are Y-up, so tip
                    // them along Z.
                    let tip = Quat::from_rotation_x(FRAC_PI_2);
                    parent.spawn((
                        Mesh3d(palette.lens_barrel.clone()),
                        MeshMaterial3d(palette.lens_barrel_mat.clone()),
                        Transform::from_xyz(0.0, LENS_Y, LENS_BARREL_Z).with_rotation(tip),
                    ));
                    parent.spawn((
                        Mesh3d(palette.lens_glass.clone()),
                        MeshMaterial3d(glass_mat),
                        Transform::from_xyz(0.0, LENS_Y, LENS_GLASS_Z).with_rotation(tip),
                    ));
                    spawn_digits(parent, palette, &segment_mat, m.record.id.0);
                }
                Model::Printer => {
                    parent.spawn((
                        Mesh3d(palette.paper_sheet.clone()),
                        MeshMaterial3d(palette.paper_mat.clone()),
                        Transform::from_xyz(0.0, 0.33, 0.0)
                            .with_rotation(Quat::from_rotation_z(0.06)),
                    ));
                }
                Model::Depot | Model::Site => {}
            }
            // A site's team is its outline (Q43): the body's own mesh, a
            // little larger. Every other machine's is its lights (Q45, Q47).
            if model == Model::Site {
                parent.spawn((
                    Mesh3d(mesh_for_outline),
                    MeshMaterial3d(outline_mat),
                    Transform::from_scale(outline_scale),
                ));
            }
            mark = parent
                .spawn((
                    FaultMark,
                    BillboardBar,
                    Mesh3d(palette.scribble_quad.clone()),
                    MeshMaterial3d(palette.scribble_mats[0].clone()),
                    Transform::from_xyz(0.0, bar_height(model) + 0.55, 0.0),
                    Visibility::Hidden,
                ))
                .id();
            bar_root = parent
                .spawn((
                    BillboardBar,
                    Transform::from_xyz(0.0, bar_height(model), 0.0),
                    Visibility::Hidden,
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Mesh3d(palette.bar.clone()),
                        MeshMaterial3d(palette.bar_bg_mat.clone()),
                        Transform::default().with_scale(Vec3::new(0.8, 0.7, 1.0)),
                    ));
                    bar_fill = bar
                        .spawn((
                            Mesh3d(palette.bar.clone()),
                            MeshMaterial3d(palette.bar_fill_mat.clone()),
                            Transform::from_xyz(0.0, 0.0, 0.0105)
                                .with_scale(Vec3::new(0.8, 0.5, 1.0)),
                        ))
                        .id();
                })
                .id();
        })
        .id();
    MachineView {
        body,
        model,
        live_mat,
        dimmed,
        shown,
        health_seen: m.record.health,
        health_age: f32::MAX,
        bar_root,
        bar_fill,
        mark,
        yaw,
        yaw_prev: yaw,
        pos_seen: m.record.pos,
        color_seen: m.record.color.clone(),
    }
}

/// The fault mark shows for `fault_mark_ticks` after the record's tick
/// (`docs/07`, Q34), cycling its frames.
pub fn fault_marks(
    time: Res<Time>,
    driver: NonSend<DriverResource>,
    interface: Res<Interface>,
    palette: Res<Palette>,
    ents: Res<Entities>,
    mut marks: Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<FaultMark>>,
) {
    let snap = driver.0.snapshot();
    let frame = ((time.elapsed_secs() * 4.0) as usize) % palette.scribble_mats.len();
    for m in &snap.machines {
        let Some(view) = ents.machines.get(&m.record.id) else {
            continue;
        };
        let Ok((mut vis, mut mat)) = marks.get_mut(view.mark) else {
            continue;
        };
        let fresh = m
            .fault
            .as_ref()
            .is_some_and(|f| snap.tick.saturating_sub(f.tick) < interface.faults.fault_mark_ticks);
        *vis = if fresh && view.shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if fresh && mat.0 != palette.scribble_mats[frame] {
            mat.0 = palette.scribble_mats[frame].clone();
        }
    }
}

/// Draw each machine between its previous and current tile by the fraction
/// of the tick elapsed (Q6), turning the shortest way round on the tick it
/// moves and holding its heading otherwise.
pub fn interpolate(
    driver: NonSend<DriverResource>,
    frame: Res<Frame>,
    ents: Res<Entities>,
    mut bodies: Query<&mut Transform, With<MachineBody>>,
) {
    let d = &driver.0;
    let f = d.snapshots.fraction(d.speed);
    let prev: HashMap<EntityId, TilePos> = d
        .snapshots
        .prev
        .as_ref()
        .map(|s| {
            s.machines
                .iter()
                .map(|m| (m.record.id, m.record.pos))
                .collect()
        })
        .unwrap_or_default();
    for m in &d.snapshots.cur.machines {
        let Some(view) = ents.machines.get(&m.record.id) else {
            continue;
        };
        let Ok(mut t) = bodies.get_mut(view.body) else {
            continue;
        };
        let lift = body_lift(view.model);
        let cur = frame.tile_xyz(m.record.pos, ents.top(m.record.pos) + lift);
        let was = prev.get(&m.record.id).copied();
        let from = was
            .map(|p| frame.tile_xyz(p, ents.top(p) + lift))
            .unwrap_or(cur);
        t.translation = from.lerp(cur, f);
        // Turn only on the tick the machine moved: the headings change only
        // on a move, so a machine standing still after a turn would
        // otherwise replay that turn every tick.
        let yaw = Quat::from_rotation_y(view.yaw);
        t.rotation = if was.is_some_and(|p| p != m.record.pos) {
            Quat::from_rotation_y(view.yaw_prev).slerp(yaw, f)
        } else {
            yaw
        };
    }
}

/// Health bars show for a few seconds after any change, the fill the
/// fraction of the model's full health.
pub fn health_bars(
    time: Res<Time>,
    driver: NonSend<DriverResource>,
    tuning: Res<Tuning>,
    mut ents: ResMut<Entities>,
    mut roots: Query<&mut Visibility, With<BillboardBar>>,
    mut fills: Query<&mut Transform, Without<BillboardBar>>,
) {
    let dt = time.delta_secs();
    let snap = driver.0.snapshot();
    for m in &snap.machines {
        let Some(view) = ents.machines.get_mut(&m.record.id) else {
            continue;
        };
        view.health_age = (view.health_age + dt).min(f32::MAX);
        let show = view.shown && view.health_age < 4.0;
        if let Ok(mut v) = roots.get_mut(view.bar_root) {
            *v = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if show && let Ok(mut t) = fills.get_mut(view.bar_fill) {
            let frac = tuning.fraction(m).max(0.02);
            t.scale = Vec3::new(frac * 0.8, 0.5, 1.0);
            t.translation.x = -(0.9 * 0.8 * (1.0 - frac)) / 2.0;
        }
    }
}

/// Bars face the camera: world rotation = parent * local, so local is the
/// parent's inverse times the camera's.
pub fn billboard_bars(
    cams: Query<&Transform, (With<Camera3d>, Without<BillboardBar>)>,
    parents: Query<&Transform, (Without<BillboardBar>, Without<Camera3d>)>,
    mut bars: Query<(&mut Transform, Option<&ChildOf>), With<BillboardBar>>,
) {
    let Ok(cam) = cams.single() else { return };
    for (mut bar, child_of) in &mut bars {
        let parent_rotation = child_of
            .and_then(|c| parents.get(c.parent()).ok())
            .map(|t| t.rotation)
            .unwrap_or(Quat::IDENTITY);
        bar.rotation = parent_rotation.inverse() * cam.rotation;
    }
}

/// The hover and selection markers ride their tiles, on the tile's face
/// above any paint (`+0.005`) or plan (`+0.01`), so neither hides them and
/// they do not fight the slab's top.
pub fn markers(
    state: Res<ViewState>,
    driver: NonSend<DriverResource>,
    frame: Res<Frame>,
    ents: Res<Entities>,
    mut hover: Query<(&mut Transform, &mut Visibility), (With<HoverMarker>, Without<SelectMarker>)>,
    mut select: Query<
        (&mut Transform, &mut Visibility),
        (With<SelectMarker>, Without<HoverMarker>),
    >,
) {
    if let Ok((mut t, mut v)) = hover.single_mut() {
        match state.hover {
            Some(p) => {
                t.translation = frame.tile_xyz(p, surface_at(&ents, p) + MARK_HALF + 0.015);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
    if let Ok((mut t, mut v)) = select.single_mut() {
        let at = state
            .selected
            .and_then(|id| machine(driver.0.snapshot(), id))
            .map(|m| m.record.pos);
        match at {
            Some(p) => {
                t.translation = frame.tile_xyz(p, surface_at(&ents, p) + MARK_HALF + 0.02);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
}

/// A ring for every sound the player's team heard this tick, growing and
/// fading out.
pub fn sounds(
    mut commands: Commands,
    time: Res<Time>,
    driver: NonSend<DriverResource>,
    frame: Res<Frame>,
    palette: Res<Palette>,
    mut ents: ResMut<Entities>,
    mut rings: Query<(
        Entity,
        &mut SoundRing,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Transform,
    )>,
) {
    let snap = driver.0.snapshot();
    if ents.sounds_tick != Some(snap.tick) {
        ents.sounds_tick = Some(snap.tick);
        for s in &snap.sounds {
            commands.spawn((
                SoundRing { age: 0.0 },
                DespawnOnEnter(Screen::Start),
                Mesh3d(palette.ring.clone()),
                MeshMaterial3d(palette.ring_mats[0].clone()),
                Transform::from_translation(frame.tile_xyz(s.pos, ents.top(s.pos) + 0.06))
                    .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
            ));
        }
    }
    let dt = time.delta_secs();
    for (e, mut ring, mut mat, mut t) in &mut rings {
        ring.age += dt;
        let life = 0.7;
        if ring.age >= life {
            commands.entity(e).despawn();
            continue;
        }
        let k = ring.age / life;
        let step = ((k * palette.ring_mats.len() as f32) as usize).min(palette.ring_mats.len() - 1);
        mat.0 = palette.ring_mats[step].clone();
        t.scale = Vec3::splat(1.0 + 2.5 * k);
    }
}

/// Grass sways, water flows and ore glints: the frames rotate through the
/// materials.
pub fn animate(
    time: Res<Time>,
    palette: Res<Palette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clock: Local<f32>,
    mut frame: Local<usize>,
) {
    *clock += time.delta_secs();
    if *clock < 0.45 {
        return;
    }
    *clock = 0.0;
    *frame = (*frame + 1) % 3;
    if let Some(mut m) = materials.get_mut(&palette.ground_mat) {
        m.base_color_texture = Some(palette.ground_frames[*frame].clone());
    }
    for (mats, frames) in [
        (&palette.water_mats, &palette.water_frames),
        (&palette.ore_mats, &palette.ore_frames),
    ] {
        for (mask, mat) in mats.iter().enumerate() {
            if let Some(mut m) = materials.get_mut(mat) {
                let image = frames[*frame][mask].clone();
                if m.emissive_texture.is_some() {
                    m.emissive_texture = Some(image.clone());
                }
                m.base_color_texture = Some(image);
            }
        }
    }
}

/// Append this tick's diagnostic log lines, faults and exchange events to
/// the panel.
pub fn collect_log(d: &mut Driver, state: &mut ViewState) {
    for e in d.events.drain(..) {
        state.log_lines.push(format!("t{} net: {e}", d.tick));
    }
    let snap = d.snapshot();
    for m in &snap.machines {
        for l in &m.log {
            state.log_lines.push(format!(
                "t{} {} [{}] {}",
                l.tick, m.record.name, l.level, l.text
            ));
        }
        // One line per new fault record (Q34).
        if let Some(f) = &m.fault
            && state.faults_seen.get(&m.record.id) != Some(&f.tick)
        {
            state.faults_seen.insert(m.record.id, f.tick);
            state.log_lines.push(format!(
                "t{} {} faulted: {} at {}:{}",
                f.tick,
                m.record.name,
                fault_what(f),
                f.file.as_deref().unwrap_or("?"),
                f.line
            ));
        }
    }
    for ev in &d.last_report.dropped {
        state
            .log_lines
            .push(format!("t{} dropped: {:?}", snap.tick, ev.kind));
    }
    if let Some(ended) = d.ended {
        state.log_lines.push(match ended {
            Some(t) => format!("t{} match over: team {} wins", snap.tick, t.0),
            None => format!("t{} match over: a draw", snap.tick),
        });
    }
    let keep = 400;
    if state.log_lines.len() > keep {
        let drop = state.log_lines.len() - keep;
        state.log_lines.drain(..drop);
    }
}

/// What a fault record says went wrong.
pub fn fault_what(f: &lang::FaultRecord) -> String {
    f.exception
        .as_ref()
        .map(|e| e.display())
        .unwrap_or_else(|| format!("hook budget exhausted ({:?})", f.exhausted))
}

/// The snapshot's machine record, for the inspector.
pub fn machine(snap: &Snapshot, id: EntityId) -> Option<&MachineSnapshot> {
    snap.machines.iter().find(|m| m.record.id == id)
}

/// The player's memory of a tile, for the inspector.
pub fn tile(snap: &Snapshot, player: sim::TeamId, p: TilePos) -> Option<&TileMemory> {
    snap.teams
        .iter()
        .find(|t| t.id == player)?
        .tiles
        .iter()
        .find(|(q, _)| *q == p)
        .map(|(_, m)| m)
}
