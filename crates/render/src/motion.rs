//! A bot's motion on the map (`docs/03`, Moving; Q65; T46): where a body
//! is drawn between two snapshots, so that a move is seen to take the
//! ticks it takes, a step rides its ramp, a drop goes out over the edge
//! and then down, a bonk lunges and comes back, and the bot it shoves is
//! knocked a tile on. Plain Rust on numbers, so it is tested headless;
//! `view.rs`'s `interpolate` is the Bevy half, and nothing here is read
//! back by the sim.
//!
//! A snapshot gives a bot's tile and, while a move or a bonk is in
//! progress, where it is headed and how far through it is
//! ([`sim::snapshot::Motion`]). A move's last tick leaves the bot on the
//! new tile with no action, and a move of one tick shows as nothing but
//! the change of tile. So every case is read from the two snapshots either
//! side of a tick, with no memory of its own: a seek, a pause, a fork's
//! stop in the middle of a tick and a replay all draw the same.

use crate::palette::RAMP_RUN;
use sim::TilePos;
use sim::snapshot::MachineSnapshot;
use std::f32::consts::PI;

/// A move or a bonk in progress, as a snapshot gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Underway {
    /// The tile a move is onto, or the tile of the bot a bonk is on.
    pub to: TilePos,
    /// The ticks the whole takes, and those left of it.
    pub ticks: u64,
    pub left: u64,
    pub bonk: bool,
}

impl Underway {
    /// The ticks the whole takes, as far as these numbers can be right:
    /// at least one, and at least those left.
    fn whole(&self) -> u64 {
        self.ticks.max(self.left).max(1)
    }

    /// How far through it is with `left` ticks to go, from 0 as it begins
    /// to 1 as it completes.
    fn done_with(&self, left: u64) -> f32 {
        let whole = self.whole();
        whole.saturating_sub(left) as f32 / whole as f32
    }

    /// How far through it is as its snapshot's tick ends.
    pub fn done(&self) -> f32 {
        self.done_with(self.left)
    }
}

/// What a snapshot says of where one body is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub pos: TilePos,
    pub act: Option<Underway>,
}

impl Pose {
    pub fn of(m: &MachineSnapshot) -> Pose {
        Pose {
            pos: m.record.pos,
            act: m.moving.as_ref().map(|a| Underway {
                to: a.to,
                ticks: a.ticks,
                left: m.record.progress,
                bonk: a.bonk.is_some(),
            }),
        }
    }
}

/// What is drawn of one body over one tick: the stretch between the
/// snapshot before and the snapshot the map shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Leg {
    /// Standing on its tile.
    Rest(TilePos),
    /// Part of a move from `from` onto `to`: from `s0` of the way through
    /// it to `s1`.
    Move {
        from: TilePos,
        to: TilePos,
        s0: f32,
        s1: f32,
    },
    /// Part of a bonk by a bot on `from` on the bot on `at`, which takes
    /// `ticks`: from `s0` of the way through it to `s1`.
    Lunge {
        from: TilePos,
        at: TilePos,
        ticks: u64,
        s0: f32,
        s1: f32,
    },
    /// Put from `from` onto `to` by a bonk, at once.
    Shoved { from: TilePos, to: TilePos },
}

/// The tile beyond `at`, as seen from `from`: where a bonk puts the bot it
/// is on.
fn onward(from: TilePos, at: TilePos) -> Option<TilePos> {
    let on = |a: i32, b: i32| b.checked_sub(a).and_then(|d| b.checked_add(d));
    Some(TilePos::new(on(from.x, at.x)?, on(from.y, at.y)?))
}

/// Whether a bonk put a bot from `prev` to `cur` in the tick between: a
/// bonk was on it in the snapshot before — by a bot on `bonk.0`, on the
/// tile `bonk.1` — it stood on that tile and not in a move, it stands now
/// on the tile beyond, and it has no pace (`docs/03`: a shoved bot's pace
/// is zero, where a bot that walked there gained some).
pub fn shoved(bonk: Option<(TilePos, TilePos)>, prev: Pose, cur: Pose, pace: Option<u64>) -> bool {
    let Some((by, at)) = bonk else {
        return false;
    };
    prev.pos == at
        && prev.act.is_none_or(|a| a.bonk)
        && cur.act.is_none()
        && cur.pos != prev.pos
        && onward(by, at) == Some(cur.pos)
        && pace == Some(0)
}

/// What to draw of a body over the tick that ended in `cur`, given the
/// snapshot before it — `None` after a jump, when there is nothing to
/// come from — and whether a bonk shoved it ([`shoved`]).
pub fn leg(prev: Option<Pose>, cur: Pose, was_shoved: bool) -> Leg {
    // In a move or a bonk now: as far as this tick took it, from where the
    // tick before left it, or from the start if it began in this one.
    if let Some(act) = cur.act {
        let s1 = act.done();
        let s0 = match prev {
            None => s1,
            Some(p) => match p.act {
                Some(was)
                    if p.pos == cur.pos
                        && was.to == act.to
                        && was.bonk == act.bonk
                        && was.left >= act.left =>
                {
                    act.done_with(was.left)
                }
                _ => 0.0,
            },
        };
        return if act.bonk {
            Leg::Lunge {
                from: cur.pos,
                at: act.to,
                ticks: act.whole(),
                s0,
                s1,
            }
        } else {
            Leg::Move {
                from: cur.pos,
                to: act.to,
                s0,
                s1,
            }
        };
    }
    let Some(prev) = prev else {
        return Leg::Rest(cur.pos);
    };
    if was_shoved {
        return Leg::Shoved {
            from: prev.pos,
            to: cur.pos,
        };
    }
    match prev.act {
        // A bonk that ended in this tick: back to its tile from wherever
        // the lunge had reached.
        Some(was) if was.bonk && prev.pos == cur.pos => Leg::Lunge {
            from: prev.pos,
            at: was.to,
            ticks: was.whole(),
            s0: was.done(),
            s1: 1.0,
        },
        // A move that ended in this tick: the rest of the way if it
        // landed, and back to its tile if something cut it short.
        Some(was) if !was.bonk && (cur.pos == was.to || cur.pos == prev.pos) => Leg::Move {
            from: prev.pos,
            to: was.to,
            s0: was.done(),
            s1: if cur.pos == was.to { 1.0 } else { 0.0 },
        },
        // No action either side and another tile: a move of one tick,
        // begun and landed inside it.
        _ if prev.pos != cur.pos => Leg::Move {
            from: prev.pos,
            to: cur.pos,
            s0: 0.0,
            s1: 1.0,
        },
        _ => Leg::Rest(cur.pos),
    }
}

impl Leg {
    /// The two tiles the body is drawn between ([`Spot::along`]).
    pub fn tiles(&self) -> (TilePos, TilePos) {
        match *self {
            Leg::Rest(p) => (p, p),
            Leg::Move { from, to, .. } | Leg::Shoved { from, to } => (from, to),
            Leg::Lunge { from, at, .. } => (from, at),
        }
    }

    /// The way the body faces on this leg, as the tile it faces from and
    /// the one it faces: the way a move goes, and the bot a bonk is on. A
    /// body at rest keeps the heading it had, and so does one shoved,
    /// which did not choose where it went.
    pub fn heading(&self) -> Option<(TilePos, TilePos)> {
        match *self {
            Leg::Move { from, to, .. } => Some((from, to)),
            Leg::Lunge { from, at, .. } => Some((from, at)),
            Leg::Rest(_) | Leg::Shoved { .. } => None,
        }
    }

    /// Whether the move or the bonk began in this tick, which is when the
    /// body turns to face it.
    pub fn begins(&self) -> bool {
        matches!(*self, Leg::Move { s0, .. } | Leg::Lunge { s0, .. } if s0 == 0.0)
    }

    /// Where the body is `f` of the way through the tick. `from_top` and
    /// `to_top` are the heights of the tops of the two [`Leg::tiles`],
    /// `drop` how much lower the second must be for a move onto it to be a
    /// fall, and `half` half the body's width in tiles.
    pub fn spot(&self, f: f32, from_top: f32, to_top: f32, drop: f32, half: f32) -> Spot {
        let f = f.clamp(0.0, 1.0);
        let falls = from_top - to_top > drop;
        let path = |s: f32| -> (f32, f32) {
            if falls {
                fall(from_top, to_top, half, s)
            } else {
                (s, step_top(from_top, to_top, s))
            }
        };
        match *self {
            Leg::Rest(_) => Spot::on(0.0, from_top),
            Leg::Move { s0, s1, .. } => {
                let (along, top) = path(s0 + (s1 - s0) * f);
                Spot::on(along, top)
            }
            // A lunge stays on its own tile, at its own height.
            Leg::Lunge { ticks, s0, s1, .. } => {
                Spot::on(lunge(ticks, half, s0 + (s1 - s0) * f), from_top)
            }
            Leg::Shoved { .. } => {
                let (along, top) = if falls {
                    path((f / KNOCK_FALL).min(1.0))
                } else {
                    path(knock(f))
                };
                Spot {
                    along,
                    top,
                    hop: if falls { 0.0 } else { knock_hop(f) },
                    tilt: knock_tilt(f),
                }
            }
        }
    }
}

/// Where a body is drawn on its leg.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// How far it is from the first of its leg's tiles toward the second,
    /// in tiles: 0 on the one, 1 on the other.
    pub along: f32,
    /// The height of the ground it rides there, as a tile's top is
    /// measured.
    pub top: f32,
    /// How far it is thrown clear of that ground.
    pub hop: f32,
    /// How far it tips back from the way it is going, in radians: a
    /// shoved bot rocks, and nothing else does.
    pub tilt: f32,
}

impl Spot {
    fn on(along: f32, top: f32) -> Spot {
        Spot {
            along,
            top,
            hop: 0.0,
            tilt: 0.0,
        }
    }
}

/// The height of the ground under a body `u` of the way from the centre of
/// a tile whose top is `from` to the centre of the next, whose top is `to`
/// — a step, not a drop. The ramp at a step lies on the lower tile, its
/// toe [`RAMP_RUN`] out from the edge between the two and its high edge on
/// it (`palette::ramp_mesh`), so the body is level to the toe, rides the
/// ramp to the edge, and is level again on the higher tile.
pub fn step_top(from: f32, to: f32, u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    // Measured from the lower tile's centre toward the higher.
    let (low, high, x) = if to >= from {
        (from, to, u)
    } else {
        (to, from, 1.0 - u)
    };
    let toe = 0.5 - RAMP_RUN;
    low + (high - low) * ((x - toe) / RAMP_RUN).clamp(0.0, 1.0)
}

/// How much of a fall's time takes the body from its tile's centre to the
/// centre of the tile below; the rest it only drops.
const FALL_OUT: f32 = 0.6;

/// A fall (`docs/03`, A fall): where a body is `s` of the way through a
/// move off a tile whose top is `from` onto the next, whose top is `to`,
/// lower by more than a step — how far across, and how high. It goes
/// straight out at the height it left until the whole of it, `half` a
/// width behind its centre, is past the edge between the two tiles, and
/// only then down, faster as it goes: over the edge and then down, never
/// through the cliff's face.
pub fn fall(from: f32, to: f32, half: f32, s: f32) -> (f32, f32) {
    let s = s.clamp(0.0, 1.0);
    let along = (s / FALL_OUT).min(1.0);
    // The moment its trailing side clears the edge.
    let clear = FALL_OUT * (0.5 + half).min(1.0);
    let down = ((s - clear) / (1.0 - clear)).clamp(0.0, 1.0);
    (along, from + (to - from) * down * down)
}

/// A bonk's lunge (`docs/03`, A bonk): how far from its own tile's centre
/// toward the bot it is on a body is, in tiles, `s` of the way through a
/// bonk of `ticks`. It strikes as the bonk's last tick begins — the tick
/// that ends with the other bot a tile onward, which is drawn knocked away
/// over that same tick — and is home as the bonk completes. At the strike
/// two bodies of `half` a width each touch.
pub fn lunge(ticks: u64, half: f32, s: f32) -> f32 {
    let s = s.clamp(0.0, 1.0);
    let reach = (1.0 - 2.0 * half).max(0.1);
    let strike = if ticks >= 2 {
        ticks.saturating_sub(1) as f32 / ticks as f32
    } else {
        0.5
    };
    if s <= strike {
        // Faster into the blow.
        let k = s / strike;
        reach * k * k
    } else {
        // Thrown back off it, and settling.
        let k = (1.0 - s) / (1.0 - strike);
        reach * k * k
    }
}

/// How much of a tick a shoved bot takes to reach the tile it is put on,
/// and how much to go off a cliff and land.
const KNOCK: f32 = 0.4;
const KNOCK_FALL: f32 = 0.7;
/// How high a shoved bot is thrown, in world units, and how far it tips,
/// in radians.
const KNOCK_HOP: f32 = 0.14;
const KNOCK_TILT: f32 = 0.35;

/// A shoved bot (`docs/03`, A bonk): how far it is from the tile it stood
/// on toward the one the bonk put it on, `f` of the way through the tick.
/// The sim puts it there at once, so it is knocked there — fastest as it
/// is hit, there well before the tick is out — and does not glide.
pub fn knock(f: f32) -> f32 {
    let k = (f.clamp(0.0, 1.0) / KNOCK).min(1.0);
    1.0 - (1.0 - k) * (1.0 - k)
}

/// How far clear of the ground a shoved bot is thrown on its way: up and
/// down again by the time it arrives.
pub fn knock_hop(f: f32) -> f32 {
    let k = (f.clamp(0.0, 1.0) / KNOCK).min(1.0);
    KNOCK_HOP * (PI * k).sin()
}

/// How far a shoved bot tips back from the way it was sent: back as it is
/// hit, then rocking, less each time, and upright as the tick ends.
pub fn knock_tilt(f: f32) -> f32 {
    let f = f.clamp(0.0, 1.0);
    KNOCK_TILT * (1.0 - f) * (1.0 - f) * (3.0 * PI * f).sin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::{BOT_HALF, LEVEL_RISE};
    use sim::world::{EntityId, Model};
    use sim::{Command, CommandKind, Data, Map, Sim, TeamId, bundle_of};

    const HALF: f32 = BOT_HALF;
    /// A step is one level, so anything lower by more is a drop.
    const DROP: f32 = 1.5 * LEVEL_RISE;

    fn at(x: i32, y: i32) -> TilePos {
        TilePos::new(x, y)
    }

    fn rest(x: i32, y: i32) -> Pose {
        Pose {
            pos: at(x, y),
            act: None,
        }
    }

    fn moving(from: (i32, i32), to: (i32, i32), ticks: u64, left: u64) -> Pose {
        Pose {
            pos: at(from.0, from.1),
            act: Some(Underway {
                to: at(to.0, to.1),
                ticks,
                left,
                bonk: false,
            }),
        }
    }

    fn bonking(from: (i32, i32), on: (i32, i32), ticks: u64, left: u64) -> Pose {
        Pose {
            pos: at(from.0, from.1),
            act: Some(Underway {
                to: at(on.0, on.1),
                ticks,
                left,
                bonk: true,
            }),
        }
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    /// How far east of x = 0 a body is drawn on level ground, `f` of the
    /// way through the tick that took it from `prev` to `cur`.
    fn east(prev: Option<Pose>, cur: Pose, f: f32) -> f32 {
        let leg = leg(prev, cur, false);
        let (a, b) = leg.tiles();
        let spot = leg.spot(f, 0.0, 0.0, DROP, HALF);
        a.x as f32 + (b.x - a.x) as f32 * spot.along
    }

    // -------------------------------------------------------- a move ---

    #[test]
    fn a_slow_move_is_drawn_over_all_its_ticks() {
        // Three ticks from a standstill: the snapshots show two ticks
        // left, one left, and then the bot on the next tile.
        let poses = [
            rest(0, 0),
            moving((0, 0), (1, 0), 3, 2),
            moving((0, 0), (1, 0), 3, 1),
            rest(1, 0),
        ];
        for (i, pair) in poses.windows(2).enumerate() {
            let (prev, cur) = (Some(pair[0]), pair[1]);
            // A third of a tile each tick, at an even speed, with no jump
            // from one tick to the next.
            let begun = i as f32 / 3.0;
            assert!(near(east(prev, cur, 0.0), begun), "tick {i} begins");
            assert!(near(east(prev, cur, 0.5), begun + 1.0 / 6.0), "tick {i}");
            assert!(
                near(east(prev, cur, 1.0), begun + 1.0 / 3.0),
                "tick {i} ends"
            );
        }
        // It leaves its tile in the tick the move begins, where the tile
        // alone would have had it stand a tick and then cross in one.
        assert!(east(Some(poses[0]), poses[1], 0.5) > 0.0);
    }

    #[test]
    fn a_fast_move_crosses_its_tile_in_its_one_tick() {
        // A move of one tick begins and lands inside it: nothing shows but
        // the tile.
        let (prev, cur) = (Some(rest(0, 0)), rest(1, 0));
        assert!(near(east(prev, cur, 0.0), 0.0));
        assert!(near(east(prev, cur, 0.5), 0.5));
        assert!(near(east(prev, cur, 1.0), 1.0));
        // Three times the speed of the move of three ticks: the pace is
        // seen as speed.
        let slow = (Some(rest(0, 0)), moving((0, 0), (1, 0), 3, 2));
        let fast = east(prev, cur, 1.0) - east(prev, cur, 0.0);
        let slow = east(slow.0, slow.1, 1.0) - east(slow.0, slow.1, 0.0);
        assert!(near(fast, 3.0 * slow));
    }

    #[test]
    fn a_jump_draws_a_move_where_it_stands() {
        // After a seek, or at a fork's stop, there is no snapshot before:
        // the bot is drawn as far through its move as it is, at any
        // moment of the tick.
        let cur = moving((0, 0), (1, 0), 4, 1);
        for f in [0.0, 0.3, 1.0] {
            assert!(near(east(None, cur, f), 0.75));
        }
        // And paused, where the whole of the tick has passed, a move in
        // progress is drawn where its tick left it.
        let prev = moving((0, 0), (1, 0), 4, 2);
        assert!(near(east(Some(prev), cur, 1.0), 0.75));
        // A move the snapshot caught as it began, not yet counted down — a
        // fork stopped inside the tick — has gone nowhere.
        assert!(near(east(None, moving((0, 0), (1, 0), 2, 2), 1.0), 0.0));
        // Ticks that cannot be right still draw: never fewer than left.
        assert!(near(east(None, moving((0, 0), (1, 0), 1, 3), 1.0), 0.0));
    }

    #[test]
    fn a_move_cut_short_goes_back_to_its_tile() {
        let (prev, cur) = (moving((0, 0), (1, 0), 2, 1), rest(0, 0));
        assert!(near(east(Some(prev), cur, 0.0), 0.5));
        assert!(near(east(Some(prev), cur, 1.0), 0.0));
    }

    #[test]
    fn a_body_that_stays_put_rests() {
        assert_eq!(
            leg(Some(rest(2, 3)), rest(2, 3), false),
            Leg::Rest(at(2, 3))
        );
        assert_eq!(leg(None, rest(2, 3), false), Leg::Rest(at(2, 3)));
        let spot = Leg::Rest(at(2, 3)).spot(0.4, 1.2, 1.2, DROP, HALF);
        assert_eq!(
            (spot.along, spot.top, spot.hop, spot.tilt),
            (0.0, 1.2, 0.0, 0.0)
        );
    }

    #[test]
    fn a_body_turns_as_its_move_begins() {
        // The heading is the move's from the tick it begins, not from the
        // tick it lands; it turns in that tick and holds after.
        let begun = leg(Some(rest(0, 0)), moving((0, 0), (0, 1), 3, 2), false);
        assert_eq!(begun.heading(), Some((at(0, 0), at(0, 1))));
        assert!(begun.begins());
        let going = leg(
            Some(moving((0, 0), (0, 1), 3, 2)),
            moving((0, 0), (0, 1), 3, 1),
            false,
        );
        assert_eq!(going.heading(), begun.heading());
        assert!(!going.begins());
        let landed = leg(Some(moving((0, 0), (0, 1), 3, 1)), rest(0, 1), false);
        assert_eq!(landed.heading(), begun.heading());
        assert!(!landed.begins());
        // A move of one tick turns in its one tick.
        assert!(leg(Some(rest(0, 0)), rest(1, 0), false).begins());
        // A jump has no tick to turn in.
        assert!(!leg(None, moving((0, 0), (0, 1), 3, 2), false).begins());
        assert_eq!(leg(Some(rest(0, 0)), rest(0, 0), false).heading(), None);
    }

    // -------------------------------------------------------- a ramp ---

    #[test]
    fn a_step_up_rides_the_ramp() {
        let (low, high) = (2.0 * LEVEL_RISE, 3.0 * LEVEL_RISE);
        let toe = 0.5 - RAMP_RUN;
        // Level on the lower tile to the ramp's toe.
        assert!(near(step_top(low, high, 0.0), low));
        assert!(near(step_top(low, high, toe), low));
        // On the ramp: halfway along it, half a level up.
        assert!(near(
            step_top(low, high, toe + RAMP_RUN / 2.0),
            low + LEVEL_RISE / 2.0
        ));
        // At the top by the edge between the tiles, and level from there.
        assert!(near(step_top(low, high, 0.5), high));
        assert!(near(step_top(low, high, 0.75), high));
        assert!(near(step_top(low, high, 1.0), high));
        // Never under the lower tile nor over the higher, and never
        // downhill on the way up.
        let mut last = low;
        for i in 0..=100 {
            let top = step_top(low, high, i as f32 / 100.0);
            assert!(top >= last && top <= high);
            last = top;
        }
    }

    #[test]
    fn a_step_down_rides_the_same_ramp_the_other_way() {
        let (low, high) = (LEVEL_RISE, 2.0 * LEVEL_RISE);
        for i in 0..=20 {
            let u = i as f32 / 20.0;
            assert!(near(step_top(high, low, u), step_top(low, high, 1.0 - u)));
        }
        // Level ground is level all the way.
        assert!(near(step_top(low, low, 0.3), low));
    }

    #[test]
    fn a_move_up_a_level_is_drawn_on_the_ramp() {
        // Two ticks up a step: by the end of the first the body is at the
        // edge between the tiles, and at the top of the ramp.
        let (low, high) = (0.0, LEVEL_RISE);
        let first = leg(Some(rest(0, 0)), moving((0, 0), (1, 0), 2, 1), false);
        let spot = first.spot(1.0, low, high, DROP, HALF);
        assert!(near(spot.along, 0.5) && near(spot.top, high));
        // A quarter of the way it is on the ramp, between the two.
        let spot = first.spot(0.5, low, high, DROP, HALF);
        assert!(spot.top > low && spot.top < high);
        assert_eq!((spot.hop, spot.tilt), (0.0, 0.0));
    }

    // -------------------------------------------------------- a fall ---

    #[test]
    fn a_fall_goes_out_over_the_edge_and_then_down() {
        let (from, to) = (4.0 * LEVEL_RISE, LEVEL_RISE);
        assert_eq!(fall(from, to, HALF, 0.0), (0.0, from));
        let (along, top) = fall(from, to, HALF, 1.0);
        assert!(near(along, 1.0) && near(top, to));
        let (mut last_along, mut last_top) = (0.0, from);
        for i in 0..=1000 {
            let (along, top) = fall(from, to, HALF, i as f32 / 1000.0);
            // Outside the cliff: while any of the body is still over the
            // tile it left — its trailing side short of the edge between
            // the two, half a tile out — it has not dropped at all.
            if along - HALF < 0.5 {
                assert_eq!(top, from, "inside the cliff at {along}");
            }
            // It only goes out and only goes down, and never below where
            // it lands.
            assert!(along >= last_along && top <= last_top && top >= to);
            (last_along, last_top) = (along, top);
        }
        // It drops faster as it goes: less in the first half of the way
        // down than in the second.
        let clear = FALL_OUT * (0.5 + HALF);
        let mid = fall(from, to, HALF, (clear + 1.0) / 2.0).1;
        assert!(from - mid < mid - to);
    }

    #[test]
    fn a_move_onto_a_tile_lower_by_more_than_a_step_is_drawn_as_a_fall() {
        // Two levels down in one tick.
        let (from, to) = (3.0 * LEVEL_RISE, LEVEL_RISE);
        let drop = leg(Some(rest(0, 0)), rest(1, 0), false);
        for i in 0..=50 {
            let f = i as f32 / 50.0;
            let spot = drop.spot(f, from, to, DROP, HALF);
            assert_eq!((spot.along, spot.top), fall(from, to, HALF, f));
        }
        // One level down is a step, and rides the ramp: at the edge it is
        // still at the top.
        let step = drop.spot(0.5, from, from - LEVEL_RISE, DROP, HALF);
        assert!(near(step.along, 0.5) && near(step.top, from));
        // A fall of more than one tick is the same path, a part a tick.
        let first = leg(Some(rest(0, 0)), moving((0, 0), (1, 0), 2, 1), false);
        let spot = first.spot(1.0, from, to, DROP, HALF);
        assert_eq!((spot.along, spot.top), fall(from, to, HALF, 0.5));
    }

    // -------------------------------------------------------- a bonk ---

    #[test]
    fn a_bonk_lunges_and_comes_back_to_its_tile() {
        // A bonk of two ticks: one snapshot shows it with a tick left, the
        // next shows the bot on its own tile with no action.
        let poses = [rest(0, 0), bonking((0, 0), (1, 0), 2, 1), rest(0, 0)];
        let out = (Some(poses[0]), poses[1]);
        let back = (Some(poses[1]), poses[2]);
        assert!(near(east(out.0, out.1, 0.0), 0.0), "it starts on its tile");
        // It strikes as the first tick ends: far enough that the two
        // bodies touch, and no further.
        let reach = 1.0 - 2.0 * HALF;
        assert!(near(east(out.0, out.1, 1.0), reach));
        assert!(near(east(back.0, back.1, 0.0), reach), "no jump between");
        // And it is home as the bonk completes: it never changed tile.
        assert!(near(east(back.0, back.1, 1.0), 0.0));
        for i in 0..=20 {
            let f = i as f32 / 20.0;
            for (prev, cur) in [out, back] {
                let x = east(prev, cur, f);
                assert!((0.0..=reach + 1e-6).contains(&x), "{x} at {f}");
            }
        }
        // It faces the bot it is on, and stays at its own height whatever
        // the other tile's.
        let leg = leg(out.0, out.1, false);
        assert_eq!(leg.heading(), Some((at(0, 0), at(1, 0))));
        assert!(leg.begins());
        assert!(near(leg.spot(1.0, 0.6, 0.3, DROP, HALF).top, 0.6));
    }

    #[test]
    fn a_longer_bonk_strikes_as_its_last_tick_begins() {
        // The shove is drawn over the bonk's last tick, so that is when
        // the lunge is at its furthest.
        let reach = 1.0 - 2.0 * HALF;
        assert!(near(lunge(4, HALF, 0.75), reach));
        assert!(lunge(4, HALF, 0.5) < reach && lunge(4, HALF, 0.9) < reach);
        for ticks in [1, 2, 3, 7] {
            assert!(near(lunge(ticks, HALF, 0.0), 0.0));
            assert!(near(lunge(ticks, HALF, 1.0), 0.0));
        }
    }

    #[test]
    fn a_shoved_bot_is_knocked_onto_its_tile_and_does_not_glide() {
        let knocked = leg(Some(rest(1, 0)), rest(2, 0), true);
        assert_eq!(
            knocked,
            Leg::Shoved {
                from: at(1, 0),
                to: at(2, 0)
            }
        );
        let spot = |f: f32| knocked.spot(f, 0.0, 0.0, DROP, HALF);
        // It starts where it stood and is on its new tile well before the
        // tick is out — where a walk would be only as far as the tick is.
        assert!(near(spot(0.0).along, 0.0));
        assert!(spot(0.2).along > 0.7, "most of the way in a fifth of it");
        assert!(near(spot(KNOCK).along, 1.0) && near(spot(1.0).along, 1.0));
        // Thrown clear of the ground on the way, tipped back as it is hit,
        // and down and upright by the end — and when the tick is paused.
        assert!(spot(0.2).hop > 0.1 && spot(0.1).tilt > 0.1);
        assert!(near(spot(0.0).hop, 0.0) && near(spot(0.0).tilt, 0.0));
        assert!(near(spot(KNOCK).hop, 0.0) && near(spot(1.0).hop, 0.0));
        assert!(near(spot(1.0).tilt, 0.0));
        let mut last = 0.0;
        for i in 0..=100 {
            let s = spot(i as f32 / 100.0);
            assert!(s.along >= last && s.along <= 1.0 && s.hop >= -1e-6);
            last = s.along;
        }
        // It keeps the way it faced: it did not choose where it went.
        assert_eq!(knocked.heading(), None);
        assert!(!knocked.begins());
    }

    #[test]
    fn a_bot_shoved_off_a_cliff_falls_outside_it() {
        let (from, to) = (4.0 * LEVEL_RISE, 0.0);
        let knocked = leg(Some(rest(1, 0)), rest(2, 0), true);
        for i in 0..=200 {
            let s = knocked.spot(i as f32 / 200.0, from, to, DROP, HALF);
            if s.along - HALF < 0.5 {
                assert_eq!(s.top, from);
            }
            assert_eq!(s.hop, 0.0);
        }
        let landed = knocked.spot(KNOCK_FALL, from, to, DROP, HALF);
        assert!(near(landed.along, 1.0) && near(landed.top, to));
    }

    #[test]
    fn only_a_bonk_shoves() {
        // A bonk from (0, 0) on the bot at (1, 0) puts it on (2, 0).
        let bonk = Some((at(0, 0), at(1, 0)));
        assert!(shoved(bonk, rest(1, 0), rest(2, 0), Some(0)));
        // No bonk was on it: it walked.
        assert!(!shoved(None, rest(1, 0), rest(2, 0), Some(0)));
        // It kept its tile: the way on was closed.
        assert!(!shoved(bonk, rest(1, 0), rest(1, 0), Some(0)));
        // It went another way, or got there with pace: its own move.
        assert!(!shoved(bonk, rest(1, 0), rest(1, 1), Some(0)));
        assert!(!shoved(bonk, rest(1, 0), rest(2, 0), Some(1)));
        // A bot in a move is not bonked, and one that has begun a move
        // since was not put anywhere.
        assert!(!shoved(
            bonk,
            moving((1, 0), (2, 0), 2, 1),
            rest(2, 0),
            Some(0)
        ));
        assert!(!shoved(
            bonk,
            rest(1, 0),
            moving((2, 0), (3, 0), 2, 1),
            Some(0)
        ));
        // A bot that was itself bonking is shoved all the same, and its
        // lunge is dropped for the knock.
        let hitting = bonking((1, 0), (1, 1), 2, 1);
        assert!(shoved(bonk, hitting, rest(2, 0), Some(0)));
        assert!(matches!(
            leg(Some(hitting), rest(2, 0), true),
            Leg::Shoved { .. }
        ));
    }

    // ------------------------------------------------ against the sim ---

    /// 9 by 3, origin at the centre: x in -4..=4, y in -1..=1. The north
    /// row is level. The middle row climbs a level at x = -1 and drops
    /// three at x = 2. The south row holds each team's printer, in its
    /// corner, and a deposit beside it.
    const MAP: &str = r#"
name = "motion"
speed = 10
width = 9
height = 3
ground_water = 20
terrain = """
.........
.........
.o.....o.
"""
heights = """
444444444
444555222
444444444
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-4, -1]
[[team]]
start = [4, -1]
"#;

    const EAST: &str = "while True:\n    move('e')\n";
    const STAND: &str = "wait(1000)\n";

    fn deploy(team: u32, main: &str) -> Command {
        Command::new(
            0,
            TeamId(team),
            0,
            CommandKind::Deploy {
                deployment: "bot".into(),
                bundle: bundle_of(main),
            },
        )
    }

    /// The map with the shipped numbers, a bot of each team placed, each
    /// team's program deployed with tick 1, and run for `ticks`: every
    /// snapshot, the one before tick 1 first.
    fn run(
        bots: &[(u32, (i32, i32))],
        programs: &[(u32, &str)],
        ticks: u64,
    ) -> (Vec<EntityId>, Vec<sim::snapshot::Snapshot>) {
        let data = Data::load().unwrap();
        let map = Map::parse(MAP, &data).unwrap();
        let mut s = Sim::with_data(map, data);
        let ids = bots
            .iter()
            .map(|(team, (x, y))| {
                s.world
                    .place(Model::Bot, TeamId(*team), at(*x, *y), Some("bot".into()))
            })
            .collect();
        let cmds: Vec<Command> = programs.iter().map(|(t, main)| deploy(*t, main)).collect();
        let mut snaps = vec![s.snapshot()];
        for t in 0..ticks {
            s.step(if t == 0 { &cmds } else { &[] }, &[]);
            snaps.push(s.snapshot());
        }
        (ids, snaps)
    }

    /// The top of a tile of [`MAP`], as the view measures one.
    fn top(p: TilePos) -> f32 {
        let level = match (p.x, p.y) {
            (-1..=1, 0) => 5.0,
            (2..=4, 0) => 2.0,
            _ => 4.0,
        };
        level * LEVEL_RISE
    }

    /// Where bot `id` is drawn `f` of the way through the tick that ended
    /// in `snaps[t]`, as `interpolate` works it out: east, north, and the
    /// height of its feet.
    fn drawn(snaps: &[sim::snapshot::Snapshot], id: EntityId, t: usize, f: f32) -> (f32, f32, f32) {
        let find = |t: usize| snaps[t].machines.iter().find(|m| m.record.id == id);
        let cur = find(t).unwrap();
        let prev = t.checked_sub(1).and_then(find);
        let bonk = t.checked_sub(1).and_then(|t| {
            snaps[t].machines.iter().find_map(|m| {
                let a = m.moving.as_ref()?;
                (a.bonk == Some(id)).then_some((m.record.pos, a.to))
            })
        });
        let (was, now) = (prev.map(Pose::of), Pose::of(cur));
        let hit = was.is_some_and(|was| shoved(bonk, was, now, cur.record.pace));
        let leg = leg(was, now, hit);
        let (a, b) = leg.tiles();
        let spot = leg.spot(f, top(a), top(b), DROP, HALF);
        (
            a.x as f32 + (b.x - a.x) as f32 * spot.along,
            a.y as f32 + (b.y - a.y) as f32 * spot.along,
            spot.top + spot.hop,
        )
    }

    /// Every tick's drawing begins where the last one's ended: nothing a
    /// bot does in the sim makes its body jump.
    fn assert_unbroken(snaps: &[sim::snapshot::Snapshot], id: EntityId) {
        for t in 1..snaps.len() - 1 {
            let (end, begin) = (drawn(snaps, id, t, 1.0), drawn(snaps, id, t + 1, 0.0));
            assert!(
                near(end.0, begin.0) && near(end.1, begin.1) && near(end.2, begin.2),
                "tick {t} ends at {end:?} and tick {} begins at {begin:?}",
                t + 1
            );
        }
    }

    #[test]
    fn the_sims_own_walk_is_drawn_at_the_speed_its_pace_gives() {
        // The shipped numbers: a move of two ticks, one fewer for each of
        // two paces. A bot from a standstill on level ground.
        let (ids, snaps) = run(&[(0, (-4, 1))], &[(0, EAST)], 7);
        let bot = ids[0];
        assert_unbroken(&snaps, bot);
        // How far east each tick takes it, as drawn.
        let strides: Vec<f32> = (1..snaps.len())
            .map(|t| drawn(&snaps, bot, t, 1.0).0 - drawn(&snaps, bot, t, 0.0).0)
            .collect();
        // The first move begins in tick 1 and takes two ticks, half a tile
        // each; from there every move takes one.
        assert!(
            near(strides[0], 0.5) && near(strides[1], 0.5),
            "{strides:?}"
        );
        assert!(strides[2..].iter().all(|s| near(*s, 1.0)), "{strides:?}");
        // The snapshot says where the move is headed and how long it is.
        let m = snaps[1].machines.iter().find(|m| m.record.id == bot);
        let m = m.unwrap();
        let moving = m.moving.as_ref().unwrap();
        assert_eq!((moving.to, moving.ticks, moving.bonk), (at(-3, 1), 2, None));
        assert_eq!((m.record.pos, m.record.progress), (at(-4, 1), 1));
        // So in the middle of that tick the bot has left its tile, where
        // its `pos` alone would have it stand a tick more.
        assert!(near(drawn(&snaps, bot, 1, 0.5).0, -3.75));
        // A machine that is not moving has none.
        assert!(snaps[0].machines.iter().all(|m| m.moving.is_none()));
    }

    #[test]
    fn the_sims_own_climb_and_fall_are_drawn_on_the_ramp_and_off_the_cliff() {
        // Along the middle row: up a level onto x = -1, level to x = 1,
        // and off a drop of three onto x = 2.
        let (ids, snaps) = run(&[(0, (-4, 0))], &[(0, EAST)], 12);
        let bot = ids[0];
        assert_unbroken(&snaps, bot);
        let (low, high, floor) = (4.0 * LEVEL_RISE, 5.0 * LEVEL_RISE, 2.0 * LEVEL_RISE);
        let mut climbed = false;
        let mut fell = false;
        for t in 1..snaps.len() {
            for i in 0..=40 {
                let (x, _, y) = drawn(&snaps, bot, t, i as f32 / 40.0);
                if x <= -2.0 {
                    assert!(near(y, low), "at {x} the body is at {y}");
                } else if x < -1.5 {
                    // The ramp lies on the lower tile, up to the edge
                    // between the two.
                    assert!(y >= low - 1e-5 && y <= high + 1e-5);
                    climbed |= y > low + 0.01 && y < high - 0.01;
                } else if x <= 1.5 + HALF {
                    // On the high ground, and past its edge until the
                    // whole body is clear of it: never in the cliff.
                    assert!(near(y, high), "at {x} the body is at {y}");
                } else {
                    assert!(y >= floor - 1e-5 && y <= high + 1e-5);
                    fell |= y < high - 0.01 && y > floor + 0.01;
                }
            }
        }
        assert!(climbed, "it was never seen on the ramp");
        assert!(fell, "it was never seen in the air");
        let end = drawn(&snaps, bot, snaps.len() - 1, 1.0);
        assert!(end.0 >= 2.0 && near(end.2, floor), "it landed: {end:?}");
    }

    #[test]
    fn the_sims_own_bonk_is_drawn_as_a_lunge_and_a_knock() {
        // A runner with pace meets a bot standing in its lane.
        let (ids, snaps) = run(&[(0, (-4, 1)), (1, (0, 1))], &[(0, EAST), (1, STAND)], 9);
        let (runner, other) = (ids[0], ids[1]);
        assert_unbroken(&snaps, runner);
        assert_unbroken(&snaps, other);
        // The tick the other bot is put a tile onward.
        let pos = |t: usize, id: EntityId| {
            let m = snaps[t].machines.iter().find(|m| m.record.id == id);
            m.unwrap().record.pos
        };
        let hit = (1..snaps.len())
            .find(|t| pos(*t, other) != at(0, 1))
            .expect("the bonk put it onward");
        assert_eq!(pos(hit, other), at(1, 1));
        assert_eq!(pos(hit, runner), at(-1, 1), "the runner kept its tile");
        // The runner: out toward it over the tick before, furthest as the
        // tick of the hit begins, and home as it ends.
        let reach = 1.0 - 2.0 * HALF;
        assert!(near(drawn(&snaps, runner, hit - 1, 0.0).0, -1.0));
        assert!(near(drawn(&snaps, runner, hit, 0.0).0, -1.0 + reach));
        assert!(near(drawn(&snaps, runner, hit, 1.0).0, -1.0));
        // The other: still until the hit, then on its new tile before the
        // tick is half out, thrown clear of the ground on the way.
        assert!(near(drawn(&snaps, other, hit - 1, 1.0).0, 0.0));
        assert!(near(drawn(&snaps, other, hit, 0.5).0, 1.0));
        assert!(drawn(&snaps, other, hit, 0.2).2 > top(at(0, 1)) + 0.1);
        assert!(near(drawn(&snaps, other, hit, 1.0).2, top(at(1, 1))));
    }
}
