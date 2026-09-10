//! The orbit camera and the cursor ray: pan with the middle button,
//! Shift+right or a left drag; orbit with the right button; zoom with the
//! wheel; WASD pans the focus. A left press that travels less than the
//! dead zone before release is a click, which `input::click` acts on.
//! Ported from the predecessor's `camera.rs`.

use crate::palette::Frame;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use sim::TilePos;

#[derive(Component)]
pub struct OrbitCam {
    pub focus: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

/// The left button's gesture: how far it has travelled since the press,
/// whether that made it a pan, and whether it just ended as a click.
#[derive(Resource, Default)]
pub struct LmbGesture {
    pub travel: Option<f32>,
    pub panning: bool,
    pub clicked: bool,
}

pub const LMB_DRAG_THRESHOLD: f32 = 6.0;

pub fn orbit_transform(cam: &OrbitCam) -> Transform {
    let rot = Quat::from_euler(EulerRot::YXZ, cam.yaw, -cam.pitch, 0.0);
    Transform::from_translation(cam.focus + rot * Vec3::new(0.0, 0.0, cam.distance))
        .looking_at(cam.focus, Vec3::Y)
}

pub fn orbit_camera(
    mut contexts: EguiContexts,
    time: Res<Time>,
    mut gesture: ResMut<LmbGesture>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut last_cursor: Local<Option<Vec2>>,
    mut wheel: MessageReader<MouseWheel>,
    mut cams: Query<(&mut OrbitCam, &mut Transform)>,
) {
    let over_ui = contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.egui_wants_pointer_input());
    let Ok((mut cam, mut transform)) = cams.single_mut() else {
        return;
    };
    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    let delta = match (cursor, *last_cursor) {
        (Some(now), Some(before)) => now - before,
        _ => Vec2::ZERO,
    };
    *last_cursor = cursor;
    let scroll: f32 = wheel.read().map(|w| w.y).sum();

    gesture.clicked = false;
    if buttons.just_released(MouseButton::Left) {
        gesture.clicked = gesture.travel.is_some() && !gesture.panning;
        gesture.travel = None;
        gesture.panning = false;
    }

    // Keys pan the focus in the camera's own frame.
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let mut d = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        d.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        d.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        d.x -= 1.0;
    }
    if (keys.pressed(KeyCode::KeyD) && !shift) || keys.pressed(KeyCode::ArrowRight) {
        d.x += 1.0;
    }
    if d != Vec2::ZERO {
        let forward = Vec3::new(-cam.yaw.sin(), 0.0, -cam.yaw.cos());
        let right = Vec3::new(cam.yaw.cos(), 0.0, -cam.yaw.sin());
        let step = 0.6 * cam.distance * time.delta_secs();
        cam.focus += (forward * d.y + right * d.x).normalize() * step;
    }

    if over_ui {
        *transform = orbit_transform(&cam);
        return;
    }
    if buttons.just_pressed(MouseButton::Left) {
        gesture.travel = Some(0.0);
    }
    if buttons.pressed(MouseButton::Left)
        && let Some(travel) = &mut gesture.travel
    {
        *travel += delta.length();
        if *travel > LMB_DRAG_THRESHOLD {
            gesture.panning = true;
        }
    }
    let panning = buttons.pressed(MouseButton::Middle)
        || (buttons.pressed(MouseButton::Right) && shift)
        || (buttons.pressed(MouseButton::Left) && gesture.panning);
    if panning && delta != Vec2::ZERO {
        let right = transform.right();
        let up = transform.up();
        let pan_scale = 0.0015 * cam.distance;
        cam.focus += (-right * delta.x + up * delta.y) * pan_scale;
    } else if buttons.pressed(MouseButton::Right) && delta != Vec2::ZERO {
        cam.yaw -= delta.x * 0.005;
        cam.pitch = (cam.pitch + delta.y * 0.005).clamp(0.1, 1.5);
    }
    if scroll != 0.0 {
        cam.distance = (cam.distance * (1.0 - scroll * 0.1)).clamp(3.0, 80.0);
    }
    *transform = orbit_transform(&cam);
}

/// The tile under the cursor: the ray walks the tiles it crosses and stops
/// at the first whose rendered top it dips below, so a rock's cliff face is
/// picked before the ground behind it. `top` is the render height of a tile.
pub fn cursor_tile(
    window: &Window,
    camera: &Camera,
    cam_tf: &GlobalTransform,
    frame: &Frame,
    top: impl Fn(TilePos) -> f32,
) -> Option<TilePos> {
    let cursor = window.cursor_position()?;
    let ray = camera.viewport_to_world(cam_tf, cursor).ok()?;
    raycast_tile(frame, ray.origin, *ray.direction, top)
}

/// The heightfield walk (a DDA over the tile grid), in tile space where
/// tile `(i, j)` covers `[i-0.5, i+0.5] x [j-0.5, j+0.5]`.
pub fn raycast_tile(
    frame: &Frame,
    origin: Vec3,
    dir: Vec3,
    top: impl Fn(TilePos) -> f32,
) -> Option<TilePos> {
    if dir.y >= -1e-5 {
        return None;
    }
    let c = frame.centre();
    // Tile space: north is +j, which is world -Z.
    let p0 = Vec2::new(origin.x + c.x, c.y - origin.z);
    let d = Vec2::new(dir.x, -dir.z);
    let (mut i, mut j) = (p0.x.round() as i32, p0.y.round() as i32);
    let step_i = if d.x > 0.0 { 1 } else { -1 };
    let step_j = if d.y > 0.0 { 1 } else { -1 };
    let t_delta_i = if d.x != 0.0 {
        (1.0 / d.x).abs()
    } else {
        f32::INFINITY
    };
    let t_delta_j = if d.y != 0.0 {
        (1.0 / d.y).abs()
    } else {
        f32::INFINITY
    };
    let bound_i = i as f32 + 0.5 * step_i as f32;
    let bound_j = j as f32 + 0.5 * step_j as f32;
    let mut t_max_i = if d.x != 0.0 {
        (bound_i - p0.x) / d.x
    } else {
        f32::INFINITY
    };
    let mut t_max_j = if d.y != 0.0 {
        (bound_j - p0.y) / d.y
    } else {
        f32::INFINITY
    };
    // Where the ray meets the plane: past it there is nothing to hit.
    let t_ground = -origin.y / dir.y;
    let span = (frame.max.x - frame.min.x + frame.max.y - frame.min.y) as i64;
    let mut entered = false;
    for _ in 0..(4 * span + 4096) {
        let t_exit = t_max_i.min(t_max_j);
        let pos = TilePos::new(i, j);
        if frame.contains(pos) {
            entered = true;
            let y_lo = origin.y + t_exit.min(t_ground) * dir.y;
            if y_lo <= top(pos) {
                return Some(pos);
            }
        } else if entered {
            break;
        }
        if t_exit > t_ground {
            break;
        }
        if t_max_i <= t_max_j {
            i += step_i;
            t_max_i += t_delta_i;
        } else {
            j += step_j;
            t_max_j += t_delta_j;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::ROCK_HEIGHT;

    fn frame() -> Frame {
        Frame {
            min: TilePos::new(-2, -2),
            max: TilePos::new(2, 2),
        }
    }

    #[test]
    fn vertical_ray_hits_the_tile_below() {
        let f = frame();
        let origin = f.tile_xyz(TilePos::new(1, 1), 5.0);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        assert_eq!(
            raycast_tile(&f, origin, dir, |_| 0.0),
            Some(TilePos::new(1, 1))
        );
    }

    #[test]
    fn shallow_ray_hits_the_cliff_face_not_the_ground_behind_it() {
        let f = frame();
        let wall = |p: TilePos| if p.x == 0 { ROCK_HEIGHT } else { 0.0 };
        // From the east, along row y = 1, dipping slowly.
        let origin = f.tile_xyz(TilePos::new(6, 1), 1.0);
        let dir = Vec3::new(-1.0, -0.1, 0.0).normalize();
        assert_eq!(
            raycast_tile(&f, origin, dir, wall),
            Some(TilePos::new(0, 1))
        );
    }

    #[test]
    fn far_origin_still_reaches_the_grid() {
        let f = frame();
        let wall = |p: TilePos| if p.x == 0 { ROCK_HEIGHT } else { 0.0 };
        let origin = f.tile_xyz(TilePos::new(60, 1), 1.0);
        let dir = Vec3::new(-1.0, -0.015, 0.0).normalize();
        assert_eq!(
            raycast_tile(&f, origin, dir, wall),
            Some(TilePos::new(0, 1))
        );
    }

    #[test]
    fn a_ray_that_crosses_flat_ground_without_meeting_it_misses() {
        let f = frame();
        let origin = f.tile_xyz(TilePos::new(6, 1), 1.0);
        let dir = Vec3::new(-1.0, -0.1, 0.0).normalize();
        assert_eq!(raycast_tile(&f, origin, dir, |_| 0.0), None);
    }

    #[test]
    fn a_ray_from_the_north_lands_on_the_north_row() {
        let f = frame();
        let origin = f.tile_xyz(TilePos::new(0, 8), 2.0);
        let dir = (f.tile_xyz(TilePos::new(0, 2), 0.0) - origin).normalize();
        assert_eq!(
            raycast_tile(&f, origin, dir, |_| 0.0),
            Some(TilePos::new(0, 2))
        );
    }
}
