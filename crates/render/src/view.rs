//! The view: entities built from the completed-tick snapshot, never shared
//! with the sim (Q15). Tiles are textured slabs rebuilt when their look
//! changes; machines are atlas boxes diffed by entity id; positions
//! interpolate between the last two snapshots in floats nothing reads back.
//! The look is the predecessor's: fog as an opaque cover over the unknown
//! and a cold, unlit twin of every material for the remembered.

use crate::app::{DriverResource, ViewState};
use crate::camera::{OrbitCam, orbit_transform};
use crate::driver::Driver;
use crate::palette::{
    BOT_HALF, Frame, LENS_BARREL_Z, LENS_GLASS_Z, LENS_Y, Palette, ROCK_HEIGHT, WATER_SINK,
    block_y_off, team_index,
};
use bevy::prelude::*;
use sim::map::Terrain;
use sim::snapshot::{MachineSnapshot, Snapshot};
use sim::world::{Model, TileMemory, TileState};
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
    pub plan: Option<String>,
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
    pub yaw: f32,
    pub yaw_prev: f32,
    pub pos_seen: TilePos,
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
) {
    let palette = Palette::build(&mut meshes, &mut materials, &assets);
    let snap = driver.0.snapshot();
    let player = driver.0.player;
    if let Some(team) = snap.teams.iter().find(|t| t.id == player) {
        for (p, _) in &team.tiles {
            let e = commands
                .spawn((
                    FogCover,
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
    commands.spawn((Camera3d::default(), orbit_transform(&cam), cam));
    commands.insert_resource(palette);
}

fn look_of(
    p: TilePos,
    mem: &TileMemory,
    memory: &HashMap<TilePos, &TileMemory>,
    plan: Option<String>,
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
        plan,
    }
}

fn top_of(look: &TileLook) -> f32 {
    match look.terrain {
        Terrain::Rock => ROCK_HEIGHT,
        Terrain::Water => WATER_SINK,
        Terrain::Ground => 0.0,
    }
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
    for (p, mem) in &team.tiles {
        let look = look_of(*p, mem, &memory, driver.0.snapshot_plan(player, *p));
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
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(frame.tile_xyz(p, y)),
            ))
            .id(),
    );
    let top = top_of(look);
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
                    Mesh3d(palette.mark_quad.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(frame.tile_xyz(p, top + 0.03)),
                ))
                .id(),
        );
    }
    if look.overlay.is_some() {
        let material = mat(palette, palette.overlay_mat.clone());
        out.push(
            commands
                .spawn((
                    Mesh3d(palette.overlay_cube.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(frame.tile_xyz(p, top + 0.16))
                        .with_rotation(Quat::from_rotation_y(0.6)),
                ))
                .id(),
        );
    }
    if let Some(plan) = &look.plan {
        let m = match plan.as_str() {
            "building" => palette.plan_building_mat.clone(),
            "paint" => palette.plan_paint_mat.clone(),
            _ => palette.plan_overlay_mat.clone(),
        };
        out.push(
            commands
                .spawn((
                    Mesh3d(palette.mark_quad.clone()),
                    MeshMaterial3d(m),
                    Transform::from_translation(frame.tile_xyz(p, top + 0.05))
                        .with_scale(Vec3::new(0.8, 1.0, 0.8)),
                ))
                .id(),
        );
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

fn spawn_machine(
    commands: &mut Commands,
    frame: &Frame,
    palette: &mut Palette,
    materials: &mut Assets<StandardMaterial>,
    ents: &Entities,
    m: &MachineSnapshot,
    shown: bool,
    dimmed: bool,
) -> MachineView {
    let model = m.record.model;
    let team = team_index(m.record.team);
    let (mesh, live_mat) = match model {
        Model::Bot => (palette.bot_cube.clone(), palette.bot_mats[team].clone()),
        Model::Printer => (
            palette.printer_box.clone(),
            palette.printer_mats[team].clone(),
        ),
        Model::Depot => (palette.crate_box.clone(), palette.crate_mat.clone()),
        Model::Site => (palette.site_cube.clone(), palette.site_mat.clone()),
    };
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
    let body = commands
        .spawn((
            MachineBody(m.record.id),
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
                        MeshMaterial3d(palette.lens_glass_mats[team].clone()),
                        Transform::from_xyz(0.0, LENS_Y, LENS_GLASS_Z).with_rotation(tip),
                    ));
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
        yaw,
        yaw_prev: yaw,
        pos_seen: m.record.pos,
    }
}

/// Draw each machine between its previous and current tile by the fraction
/// of the tick elapsed (Q6), turning the shortest way round.
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
        let from = prev
            .get(&m.record.id)
            .map(|p| frame.tile_xyz(*p, ents.top(*p) + lift))
            .unwrap_or(cur);
        t.translation = from.lerp(cur, f);
        t.rotation = Quat::from_rotation_y(view.yaw_prev).slerp(Quat::from_rotation_y(view.yaw), f);
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

/// The hover and selection markers ride their tiles.
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
                t.translation = frame.tile_xyz(p, ents.top(p) + 0.04);
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
                t.translation = frame.tile_xyz(p, ents.top(p) + 0.045);
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
