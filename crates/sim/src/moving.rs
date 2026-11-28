//! Moving (`docs/03`, Moving; Q65): how long a move takes, what a bot
//! gains by keeping going, and what happens when it leaves a cliff or
//! meets another bot. A bot's position changes here and nowhere else:
//! by its own move, and by a bonk.

use crate::map::{Slope, TilePos};
use crate::world::{EntityId, World};
use lang::{Num, Value};

impl World {
    /// Whether `move(dir)` may take a bot on `from` onto the adjacent
    /// `to` (`docs/03`, A step and A drop): a step, or a drop of any
    /// depth. Only a cliff up, and a tile no bot may stand on, are
    /// closed to it.
    pub fn steps_or_drops(&self, from: TilePos, to: TilePos) -> bool {
        self.stands(to)
            && self.slope(from, to).is_some_and(|s| match s {
                Slope::Level | Slope::Up | Slope::Down | Slope::Drop(_) => true,
                Slope::Cliff => false,
            })
    }

    /// The ticks of a move by bot `id` onto the adjacent `to`, fixed as
    /// it begins (`docs/03`, Pace): the model's `move_ticks`, less the
    /// bot's pace, plus `up_ticks` onto a higher tile, less `down_ticks`
    /// onto a lower one — a drop is a move down — and never fewer than
    /// one.
    pub fn move_ticks(&self, id: EntityId, to: TilePos) -> u64 {
        let Some(m) = self.machines.get(&id) else {
            return 1;
        };
        let md = self.data.model(m.model.name());
        let (up, down) = match self.slope(m.pos, to) {
            Some(Slope::Up | Slope::Cliff) => (md.up_ticks, 0),
            Some(Slope::Down | Slope::Drop(_)) => (0, md.down_ticks),
            Some(Slope::Level) | None => (0, 0),
        };
        md.move_ticks
            .saturating_add(up)
            .saturating_sub(m.pace.saturating_add(down))
            .max(1)
    }

    /// The bot standing on `p` that a bonk may shove (`docs/03`, A
    /// bonk): any team's, and not itself in a move. Never a building or
    /// a site.
    pub fn bonkable(&self, p: TilePos) -> Option<EntityId> {
        self.machines
            .values()
            .find(|m| m.pos == p)
            .filter(|m| m.is_bot() && !m.in_move())
            .map(|m| m.id)
    }

    /// As a tick's slices end (`docs/06`, step 2), a bot that is not in
    /// a move has no pace.
    pub(crate) fn slices_end(&mut self) {
        for m in self.machines.values_mut() {
            if !m.in_move() {
                m.pace = 0;
            }
        }
    }

    /// A move by `id` onto `to` completes (`docs/03`, Pace and A fall):
    /// the bot stands on `to`, and its pace rises by one, by two if the
    /// move went down a level and not at all if it went up, to at most
    /// `pace_max` — or it has fallen, if the move was a drop.
    pub(crate) fn land(&mut self, id: EntityId, to: TilePos) {
        let Some(m) = self.machines.get(&id) else {
            return;
        };
        let slope = self.slope(m.pos, to);
        let max = self.data.model(m.model.name()).pace_max;
        let gained = match slope {
            Some(Slope::Level) => 1,
            Some(Slope::Down) => 2,
            Some(Slope::Up | Slope::Cliff | Slope::Drop(_)) | None => 0,
        };
        let pace = m.pace.saturating_add(gained).min(max);
        self.move_machine(id, to);
        if let Some(m) = self.machines.get_mut(&id) {
            m.pace = pace;
        }
        if let Some(Slope::Drop(levels)) = slope {
            self.fall(id, levels);
        }
    }

    /// Bot `id` has fallen `levels` onto the tile it now stands on
    /// (`docs/03`, A fall): its pace is zero, and in this tick's damage
    /// step it loses `fall_damage` for each level beyond a step — unless
    /// the tile holds water as deep as `water.cushion`, which takes all
    /// of it. The damage joins the tick's other hits, to be summed.
    fn fall(&mut self, id: EntityId, levels: u8) {
        let Some(m) = self.machines.get_mut(&id) else {
            return;
        };
        m.pace = 0;
        let (pos, model) = (m.pos, m.model);
        let beyond = levels.saturating_sub(self.data.world.height.step);
        let cushioned = self
            .tile(pos)
            .is_some_and(|t| t.water >= self.data.world.water.cushion);
        if beyond == 0 || cushioned {
            return;
        }
        let damage = Num::from_int(i128::from(beyond))
            .and_then(|n| self.data.model(model.name()).fall_damage.checked_mul(n))
            .unwrap_or(Num::MAX);
        if damage > Num::ZERO {
            let d = self.pending_damage.entry(id).or_insert(Num::ZERO);
            *d = d.checked_add(damage).unwrap_or(Num::MAX);
        }
    }

    /// A bonk by `id` on the bot `target`, which stood on `to` as it
    /// began, completes (`docs/03`, A bonk). The bot that bonked has no
    /// pace. If the other is still on that tile and not in a move, and
    /// the tile beyond it in the same direction is one it could step or
    /// drop onto with no machine on it or moving onto it, it is put
    /// there at once: its action in progress ends with nothing done, its
    /// pace is zero, and if the tile was a drop it has fallen. Otherwise
    /// nothing happens to it.
    pub(crate) fn bonk(&mut self, id: EntityId, to: TilePos, target: EntityId) {
        let Some(m) = self.machines.get_mut(&id) else {
            return;
        };
        m.pace = 0;
        let from = m.pos;
        let still = self
            .machines
            .get(&target)
            .is_some_and(|t| t.pos == to && t.is_bot() && !t.in_move());
        if !still {
            return;
        }
        // The tile beyond: as far from `to` as `to` is from the bot that
        // bonked, the same way.
        let onward = |a: i32, b: i32| b.checked_sub(a).and_then(|d| b.checked_add(d));
        let (Some(x), Some(y)) = (onward(from.x, to.x), onward(from.y, to.y)) else {
            return;
        };
        let beyond = TilePos::new(x, y);
        if !self.steps_or_drops(to, beyond) || self.occupied(beyond) {
            return;
        }
        let slope = self.slope(to, beyond);
        self.move_machine(target, beyond);
        if let Some(t) = self.machines.get_mut(&target) {
            t.pace = 0;
            // Its action ends with nothing done: the builtin it waited
            // in returns, and its program goes on from there.
            if t.action.take().is_some()
                && let Some(p) = t.program.as_mut()
            {
                p.resume(Value::None);
            }
        }
        if let Some(Slope::Drop(levels)) = slope {
            self.fall(target, levels);
        }
    }
}
