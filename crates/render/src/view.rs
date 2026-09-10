//! The view: entities built from the completed-tick snapshot, never shared
//! with the sim (Q15). Tiles are spawned once and recoloured per tick;
//! machines are diffed by entity id; positions interpolate between the last
//! two snapshots in floats that nothing reads back.

use crate::app::{DriverResource, ViewState};
use crate::driver::Driver;
use bevy::prelude::*;
use sim::snapshot::{MachineSnapshot, Snapshot};
use sim::world::{Model, TileState};
use sim::{EntityId, TeamId, TilePos};
use std::collections::HashMap;

/// World units per tile.
pub const TILE: f32 = 32.0;

#[derive(Component)]
pub struct TileSprite(pub TilePos);

#[derive(Component)]
pub struct TileMark(pub TilePos);

#[derive(Component)]
pub struct MachineSprite(pub EntityId);

#[derive(Component)]
pub struct MachineLabel(pub EntityId);

#[derive(Component)]
pub struct SoundRing {
    pub age: f32,
}

/// The renderer's own index from snapshot ids to its entities.
#[derive(Resource, Default)]
pub struct Entities {
    pub tiles: HashMap<TilePos, Entity>,
    pub marks: HashMap<TilePos, Entity>,
    pub machines: HashMap<EntityId, Entity>,
    pub labels: HashMap<EntityId, Entity>,
    pub synced_tick: Option<u64>,
    pub sounds_tick: Option<u64>,
}

pub fn world_pos(p: TilePos) -> Vec2 {
    Vec2::new(p.x as f32 * TILE, p.y as f32 * TILE)
}

pub fn tile_of(world: Vec2) -> TilePos {
    TilePos::new(
        (world.x / TILE).round() as i32,
        (world.y / TILE).round() as i32,
    )
}

pub fn team_color(team: TeamId) -> Color {
    match team.0 % 6 {
        0 => Color::srgb(0.95, 0.35, 0.30),
        1 => Color::srgb(0.30, 0.55, 0.95),
        2 => Color::srgb(0.35, 0.85, 0.40),
        3 => Color::srgb(0.95, 0.85, 0.30),
        4 => Color::srgb(0.75, 0.40, 0.95),
        _ => Color::srgb(0.95, 0.60, 0.25),
    }
}

fn paint_color(color: &str) -> Color {
    match color {
        "red" => Color::srgb(0.8, 0.2, 0.2),
        "blue" => Color::srgb(0.2, 0.3, 0.8),
        "green" => Color::srgb(0.2, 0.7, 0.3),
        "yellow" => Color::srgb(0.8, 0.8, 0.2),
        "purple" => Color::srgb(0.6, 0.2, 0.8),
        "orange" => Color::srgb(0.9, 0.5, 0.1),
        "cyan" => Color::srgb(0.2, 0.8, 0.8),
        _ => Color::srgb(0.8, 0.2, 0.7),
    }
}

/// A `num` as a float, for display only.
pub fn num_f32(n: lang::Num) -> f32 {
    (n.raw() as f64 / 1e12) as f32
}

/// Spawn the camera and one sprite per tile from the first snapshot.
pub fn setup(mut commands: Commands, driver: NonSend<DriverResource>, mut ents: ResMut<Entities>) {
    let snap = driver.0.snapshot();
    let player = driver.0.player;
    let team = snap
        .teams
        .iter()
        .find(|t| t.id == player)
        .expect("the player's team");
    let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0.0f32);
    for (p, _) in &team.tiles {
        let e = commands
            .spawn((
                Sprite::from_color(Color::BLACK, Vec2::splat(TILE - 1.0)),
                Transform::from_translation(world_pos(*p).extend(0.0)),
                TileSprite(*p),
            ))
            .id();
        ents.tiles.insert(*p, e);
        let m = commands
            .spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(TILE * 0.45)),
                Transform::from_translation(world_pos(*p).extend(0.5)),
                TileMark(*p),
            ))
            .id();
        ents.marks.insert(*p, m);
        sx += p.x as f32;
        sy += p.y as f32;
        n += 1.0;
    }
    let centre = if n > 0.0 {
        Vec2::new(sx / n, sy / n) * TILE
    } else {
        Vec2::ZERO
    };
    commands.spawn((Camera2d, Transform::from_translation(centre.extend(100.0))));
}

/// Recolour every tile as the player's team sees it (Q21): unknown black,
/// remembered dimmed, visible live; terrain, deposit, paint and overlay.
pub fn sync_tiles(
    driver: NonSend<DriverResource>,
    mut ents: ResMut<Entities>,
    mut tiles: Query<&mut Sprite, (With<TileSprite>, Without<TileMark>)>,
    mut marks: Query<&mut Sprite, (With<TileMark>, Without<TileSprite>)>,
) {
    let snap = driver.0.snapshot();
    if ents.synced_tick == Some(snap.tick) {
        return;
    }
    let player = driver.0.player;
    let Some(team) = snap.teams.iter().find(|t| t.id == player) else {
        return;
    };
    for (p, mem) in &team.tiles {
        let Some(&e) = ents.tiles.get(p) else {
            continue;
        };
        let Ok(mut sprite) = tiles.get_mut(e) else {
            continue;
        };
        let base = match mem.terrain {
            sim::map::Terrain::Ground => Color::srgb(0.30, 0.30, 0.28),
            sim::map::Terrain::Rock => Color::srgb(0.16, 0.15, 0.17),
            sim::map::Terrain::Water => Color::srgb(0.15, 0.30, 0.50),
        };
        let mut c = base;
        if let Some(d) = &mem.deposit {
            let amount = d.values().map(|v| num_f32(*v)).sum::<f32>();
            let t = (amount / 500.0).clamp(0.1, 1.0);
            c = Color::srgb(0.35 + 0.15 * t, 0.55 + 0.35 * t, 0.30);
        }
        if let Some(paint) = &mem.paint {
            c = paint_color(paint).mix(&c, 0.4);
        }
        c = match mem.state {
            TileState::Unknown => Color::BLACK,
            TileState::Remembered => c.darker(0.12),
            TileState::Visible => c,
        };
        sprite.color = c;
        // The mark: an overlay shows as a small square; a plan of the
        // player's as an outline colour.
        if let Some(&me) = ents.marks.get(p)
            && let Ok(mut ms) = marks.get_mut(me)
        {
            ms.color = Color::NONE;
            if mem.state != TileState::Unknown
                && let Some(label) = &mem.overlay
            {
                ms.color = match label.as_str() {
                    "a" => Color::srgb(0.9, 0.9, 0.9),
                    "b" => Color::srgb(0.9, 0.6, 0.9),
                    "c" => Color::srgb(0.6, 0.9, 0.9),
                    _ => Color::srgb(0.9, 0.9, 0.5),
                };
            }
        }
    }
    // Plans: drawn from the sim's per-team plans through the tile record
    // is the program's view; the player sees their own plans as outlines.
    for (p, mem) in &team.tiles {
        let _ = mem;
        if let Some(plan) = driver.0.snapshot_plan(player, *p)
            && let Some(&me) = ents.marks.get(p)
            && let Ok(mut ms) = marks.get_mut(me)
        {
            ms.color = match plan.as_str() {
                "building" => Color::srgb(0.95, 0.95, 0.3),
                "paint" => Color::srgb(0.95, 0.3, 0.95),
                _ => Color::srgb(0.3, 0.95, 0.95),
            };
        }
    }
    ents.synced_tick = Some(snap.tick);
}

fn machine_size(m: &MachineSnapshot) -> Vec2 {
    match m.record.model {
        Model::Bot => Vec2::splat(TILE * 0.5),
        Model::Printer => Vec2::splat(TILE * 0.9),
        Model::Depot => Vec2::new(TILE * 0.8, TILE * 0.6),
        Model::Site => Vec2::splat(TILE * 0.7),
    }
}

/// Diff the machines: spawn new, despawn gone, recolour the rest.
pub fn sync_machines(
    mut commands: Commands,
    driver: NonSend<DriverResource>,
    mut ents: ResMut<Entities>,
    mut sprites: Query<&mut Sprite, With<MachineSprite>>,
    mut labels: Query<&mut Text2d, With<MachineLabel>>,
) {
    let snap = driver.0.snapshot();
    let player = driver.0.player;
    let seen: Vec<EntityId> = snap.machines.iter().map(|m| m.record.id).collect();
    // Only machines the player's team sees or remembers are shown; the
    // snapshot holds every machine for the replay tool, and the fog is the
    // renderer's to honour (Q21).
    let team = snap.teams.iter().find(|t| t.id == player);
    let visible = |m: &MachineSnapshot| -> bool {
        if m.record.team == player {
            return true;
        }
        let Some(team) = team else { return false };
        team.tiles
            .iter()
            .find(|(p, _)| *p == m.record.pos)
            .is_some_and(|(_, mem)| match mem.state {
                TileState::Visible => true,
                TileState::Remembered => {
                    m.record.model != Model::Bot
                        && mem.building.as_ref().is_some_and(|b| b.id == m.record.id)
                }
                TileState::Unknown => false,
            })
    };
    for m in &snap.machines {
        let shown = visible(m);
        let color = {
            let c = team_color(m.record.team);
            let health = num_f32(m.record.health).max(0.0);
            let maxh = match m.record.model {
                Model::Bot => 10.0,
                Model::Printer => 60.0,
                Model::Depot => 40.0,
                Model::Site => 10.0,
            };
            let t = (health / maxh).clamp(0.2, 1.0);
            let c = c.mix(&Color::BLACK, 1.0 - t);
            if m.record.model == Model::Site {
                c.with_alpha(0.55)
            } else {
                c
            }
        };
        match ents.machines.get(&m.record.id) {
            Some(&e) => {
                if let Ok(mut s) = sprites.get_mut(e) {
                    s.color = if shown { color } else { Color::NONE };
                }
                if let Some(&l) = ents.labels.get(&m.record.id)
                    && let Ok(mut t) = labels.get_mut(l)
                {
                    let text = if shown { label_text(m) } else { String::new() };
                    if t.0 != text {
                        t.0 = text;
                    }
                }
            }
            None => {
                let pos = world_pos(m.record.pos);
                let e = commands
                    .spawn((
                        Sprite::from_color(
                            if shown { color } else { Color::NONE },
                            machine_size(m),
                        ),
                        Transform::from_translation(pos.extend(1.0)),
                        MachineSprite(m.record.id),
                    ))
                    .id();
                let l = commands
                    .spawn((
                        Text2d::new(if shown { label_text(m) } else { String::new() }),
                        TextFont {
                            font_size: bevy::text::FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        Transform::from_translation(
                            pos.extend(2.0) + Vec3::new(0.0, TILE * 0.55, 0.0),
                        ),
                        MachineLabel(m.record.id),
                    ))
                    .id();
                ents.machines.insert(m.record.id, e);
                ents.labels.insert(m.record.id, l);
            }
        }
    }
    let gone: Vec<EntityId> = ents
        .machines
        .keys()
        .filter(|id| !seen.contains(id))
        .copied()
        .collect();
    for id in gone {
        if let Some(e) = ents.machines.remove(&id) {
            commands.entity(e).despawn();
        }
        if let Some(l) = ents.labels.remove(&id) {
            commands.entity(l).despawn();
        }
    }
}

fn label_text(m: &MachineSnapshot) -> String {
    let busy = m.record.busy.map(|b| format!(" {b}")).unwrap_or_default();
    format!("{} {}{busy}", m.record.name, num_f32(m.record.health))
}

/// Draw each machine between its previous and current tile by the fraction
/// of the tick elapsed (Q6): floats, never read back.
pub fn interpolate(
    driver: NonSend<DriverResource>,
    ents: Res<Entities>,
    mut sprites: Query<&mut Transform, (With<MachineSprite>, Without<MachineLabel>)>,
    mut labels: Query<&mut Transform, (With<MachineLabel>, Without<MachineSprite>)>,
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
        let cur = world_pos(m.record.pos);
        let from = prev.get(&m.record.id).map(|p| world_pos(*p)).unwrap_or(cur);
        let pos = from.lerp(cur, f);
        if let Some(&e) = ents.machines.get(&m.record.id)
            && let Ok(mut t) = sprites.get_mut(e)
        {
            t.translation = pos.extend(1.0);
        }
        if let Some(&l) = ents.labels.get(&m.record.id)
            && let Ok(mut t) = labels.get_mut(l)
        {
            t.translation = pos.extend(2.0) + Vec3::new(0.0, TILE * 0.55, 0.0);
        }
    }
}

/// A ring for every sound the player's team heard this tick, fading out.
pub fn sounds(
    mut commands: Commands,
    time: Res<Time>,
    driver: NonSend<DriverResource>,
    mut ents: ResMut<Entities>,
    mut rings: Query<(Entity, &mut SoundRing, &mut Sprite, &mut Transform)>,
) {
    let snap = driver.0.snapshot();
    if ents.sounds_tick != Some(snap.tick) {
        ents.sounds_tick = Some(snap.tick);
        for s in &snap.sounds {
            commands.spawn((
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.5), Vec2::splat(TILE * 0.3)),
                Transform::from_translation(world_pos(s.pos).extend(3.0)),
                SoundRing { age: 0.0 },
            ));
        }
    }
    let dt = time.delta_secs();
    for (e, mut ring, mut sprite, mut t) in &mut rings {
        ring.age += dt;
        let life = 0.6;
        if ring.age >= life {
            commands.entity(e).despawn();
            continue;
        }
        let k = ring.age / life;
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.5 * (1.0 - k));
        t.scale = Vec3::splat(1.0 + 3.0 * k);
    }
}

/// Append this tick's diagnostic log lines and faults to the panel.
pub fn collect_log(d: &Driver, state: &mut ViewState) {
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

/// The snapshot's machine records, for the inspector.
pub fn machine(snap: &Snapshot, id: EntityId) -> Option<&MachineSnapshot> {
    snap.machines.iter().find(|m| m.record.id == id)
}
