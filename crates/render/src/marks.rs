//! The mark tools (`docs/07`, Marks; Q37): the strip's three rows from
//! data, picking an entry from the keys, the tiles a drag crosses, and the
//! command a click on a tile becomes. Plain Rust, so it is tested headless;
//! `input.rs`, `ui.rs` and `view.rs` are the Bevy half.

use crate::app::Tool;
use bevy::prelude::Resource;
use sim::world::{Plan, Plans};
use sim::{CommandKind, PlanKind, Sim, TilePos};

/// The strip: a row per plan kind, in the order the screen shows them.
pub const ROWS: [PlanKind; 3] = [PlanKind::Building, PlanKind::Paint, PlanKind::Overlay];

/// The key that picks a row: `B`, `P` or `O`.
pub fn row_key(plan: PlanKind) -> char {
    match plan {
        PlanKind::Building => 'B',
        PlanKind::Paint => 'P',
        PlanKind::Overlay => 'O',
    }
}

/// What each row lists: the values a plan may name, from the data `sim`
/// validates against ([`Sim::markable`]).
#[derive(Resource, Debug, Clone)]
pub struct MarkRows {
    pub building: Vec<String>,
    pub paint: Vec<String>,
    pub overlay: Vec<String>,
}

impl MarkRows {
    pub fn load() -> Result<MarkRows, String> {
        let data = sim::Data::load().map_err(|e| e.to_string())?;
        Ok(MarkRows {
            building: Sim::markable(&data, PlanKind::Building),
            paint: Sim::markable(&data, PlanKind::Paint),
            overlay: Sim::markable(&data, PlanKind::Overlay),
        })
    }

    /// A row's entries in strip order: every value, then *clear* (`None`),
    /// the plan to empty the slot.
    pub fn entries(&self, plan: PlanKind) -> Vec<Option<String>> {
        let values = match plan {
            PlanKind::Building => &self.building,
            PlanKind::Paint => &self.paint,
            PlanKind::Overlay => &self.overlay,
        };
        values.iter().cloned().map(Some).chain([None]).collect()
    }

    /// The tool `1`–`9` picks in `row`: entry `n`, counted from one, or the
    /// tool disarmed if that entry is already armed. Out of range picks
    /// nothing.
    pub fn pick(&self, row: PlanKind, n: usize, current: &Tool) -> Option<Tool> {
        let value = self.entries(row).get(n.checked_sub(1)?)?.clone();
        Some(toggle(current, Tool::Mark(row, value)))
    }
}

/// Arm `tool`, or disarm it if it is the one already armed (Q37).
pub fn toggle(current: &Tool, tool: Tool) -> Tool {
    if *current == tool { Tool::Select } else { tool }
}

/// What a click on `at` with `tool` armed submits: a `Mark` of its kind
/// and value, or with `Shift` an `Unmark` of its kind. Nothing unarmed.
pub fn command(tool: &Tool, at: TilePos, shift: bool) -> Option<CommandKind> {
    let Tool::Mark(plan, value) = tool else {
        return None;
    };
    Some(if shift {
        CommandKind::Unmark { at, plan: *plan }
    } else {
        CommandKind::Mark {
            at,
            plan: *plan,
            value: value.clone(),
        }
    })
}

/// Every tile on the way from `a` to `b`, both included, each sharing an
/// edge with the last — so a drag faster than a tile per frame still
/// marks every tile the cursor crossed.
pub fn walk(a: TilePos, b: TilePos) -> Vec<TilePos> {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut out = vec![a];
    let (mut x, mut y) = (a.x, a.y);
    let (mut ix, mut iy) = (0, 0);
    // Step along whichever axis the line is further behind on.
    while ix < dx || iy < dy {
        if (1 + 2 * ix) * dy < (1 + 2 * iy) * dx {
            x += sx;
            ix += 1;
        } else {
            y += sy;
            iy += 1;
        }
        out.push(TilePos::new(x, y));
    }
    out
}

/// The plans as one line for the inspector: `paint red · overlay clear`.
pub fn describe(plans: &Plans) -> String {
    [
        (PlanKind::Building, &plans.building),
        (PlanKind::Paint, &plans.paint),
        (PlanKind::Overlay, &plans.overlay),
    ]
    .into_iter()
    .filter_map(|(kind, plan)| {
        plan.as_ref().map(|p| match p {
            Plan::Set(v) => format!("{} {v}", kind.name()),
            Plan::Clear => format!("{} clear", kind.name()),
        })
    })
    .collect::<Vec<_>>()
    .join(" · ")
}

/// The plans the armed tool would leave on a tile, for its ghost under the
/// cursor: `None` unarmed.
pub fn ghost_of(tool: &Tool) -> Option<Plans> {
    let Tool::Mark(plan, value) = tool else {
        return None;
    };
    let p = Some(match value {
        Some(v) => Plan::Set(v.clone()),
        None => Plan::Clear,
    });
    let mut plans = Plans::default();
    match plan {
        PlanKind::Building => plans.building = p,
        PlanKind::Paint => plans.paint = p,
        PlanKind::Overlay => plans.overlay = p,
    }
    Some(plans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> MarkRows {
        MarkRows::load().unwrap()
    }

    #[test]
    fn each_row_lists_the_data_then_clear() {
        let r = rows();
        assert_eq!(
            r.entries(PlanKind::Building),
            vec![Some("printer".into()), Some("depot".into()), None]
        );
        let paint = r.entries(PlanKind::Paint);
        assert_eq!(paint.len(), 9, "eight colors and clear");
        assert_eq!(paint[0], Some("red".into()));
        assert_eq!(paint[8], None);
        assert_eq!(r.entries(PlanKind::Overlay).last(), Some(&None));
    }

    #[test]
    fn a_digit_arms_an_entry_and_a_second_press_disarms_it() {
        let r = rows();
        let paint_red = Tool::Mark(PlanKind::Paint, Some("red".into()));
        assert_eq!(
            r.pick(PlanKind::Paint, 1, &Tool::Select),
            Some(paint_red.clone())
        );
        assert_eq!(r.pick(PlanKind::Paint, 1, &paint_red), Some(Tool::Select));
        assert_eq!(
            r.pick(PlanKind::Building, 3, &paint_red),
            Some(Tool::Mark(PlanKind::Building, None)),
            "the last entry is clear"
        );
        assert_eq!(r.pick(PlanKind::Building, 4, &Tool::Select), None);
        assert_eq!(r.pick(PlanKind::Paint, 0, &Tool::Select), None);
    }

    #[test]
    fn a_click_marks_shift_withdraws_and_unarmed_does_neither() {
        let at = TilePos::new(2, -1);
        let tool = Tool::Mark(PlanKind::Overlay, Some("a".into()));
        assert_eq!(
            command(&tool, at, false),
            Some(CommandKind::Mark {
                at,
                plan: PlanKind::Overlay,
                value: Some("a".into())
            })
        );
        assert_eq!(
            command(&tool, at, true),
            Some(CommandKind::Unmark {
                at,
                plan: PlanKind::Overlay
            })
        );
        assert_eq!(command(&Tool::Select, at, false), None);
    }

    #[test]
    fn a_walk_crosses_every_tile_edge_to_edge() {
        let t = TilePos::new;
        assert_eq!(walk(t(0, 0), t(0, 0)), vec![t(0, 0)]);
        assert_eq!(
            walk(t(0, 0), t(3, 0)),
            vec![t(0, 0), t(1, 0), t(2, 0), t(3, 0)]
        );
        for (a, b) in [
            (t(0, 0), t(5, 2)),
            (t(3, -4), t(-2, 1)),
            (t(1, 1), t(-1, 6)),
        ] {
            let w = walk(a, b);
            assert_eq!(w.first(), Some(&a));
            assert_eq!(w.last(), Some(&b));
            let steps = (b.x - a.x).abs() + (b.y - a.y).abs();
            assert_eq!(w.len() as i32, steps + 1, "{a:?} → {b:?}: {w:?}");
            for pair in w.windows(2) {
                let d = (pair[1].x - pair[0].x).abs() + (pair[1].y - pair[0].y).abs();
                assert_eq!(d, 1, "{w:?} jumps a tile");
            }
        }
    }

    #[test]
    fn the_inspector_line_and_the_cursor_ghost() {
        let plans = Plans {
            paint: Some(Plan::Set("red".into())),
            overlay: Some(Plan::Clear),
            building: None,
        };
        assert_eq!(describe(&plans), "paint red · overlay clear");
        assert_eq!(ghost_of(&Tool::Select), None);
        let g = ghost_of(&Tool::Mark(PlanKind::Building, None)).unwrap();
        assert_eq!(g.building, Some(Plan::Clear));
        assert!(g.paint.is_none() && g.overlay.is_none());
    }
}
