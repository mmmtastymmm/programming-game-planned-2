//! The mark tools (`docs/07`, Marks; Q37, Q44): the strip's three rows
//! from data, picking an entry from the keys, the tiles a drag crosses or
//! a `Shift`+drag's rectangle fills, and the command marking a tile
//! becomes. Plain Rust, so it is tested headless;
//! `input.rs`, `ui.rs` and `view.rs` are the Bevy half.

use crate::app::Tool;
use bevy::prelude::Resource;
use sim::world::{Plan, Plans};
use sim::{CommandKind, PlanKind, Sim, TilePos};

/// The strip: a row per plan kind, in the order the screen shows them.
pub const ROWS: [PlanKind; 5] = [
    PlanKind::Building,
    PlanKind::Paint,
    PlanKind::Overlay,
    PlanKind::Wire,
    PlanKind::Ground,
];

/// The key that picks a row: `B`, `P`, `O`, `C` or `G`.
pub fn row_key(plan: PlanKind) -> char {
    match plan {
        PlanKind::Building => 'B',
        PlanKind::Paint => 'P',
        PlanKind::Overlay => 'O',
        // For cable, since `W` pans the camera (Q62).
        PlanKind::Wire => 'C',
        PlanKind::Ground => 'G',
        // The pipe row's key (Q67); T49 puts the row on the strip.
        PlanKind::Pipe => 'T',
    }
}

/// The ground row's tools (Q65; `docs/07`, Marks). Each becomes a `Mark`
/// of a height that depends on the tile: one below, or one above, the
/// height its plan holds or the team knows it at, or the height of the
/// tile the stroke began on.
pub const LOWER: &str = "lower";
pub const RAISE: &str = "raise";
pub const LEVEL: &str = "level";

/// What each row lists: the values a plan may name, from the data `sim`
/// validates against ([`Sim::markable`]) — but for the ground row, whose
/// entries are its tools.
#[derive(Resource, Debug, Clone)]
pub struct MarkRows {
    pub building: Vec<String>,
    pub paint: Vec<String>,
    pub overlay: Vec<String>,
    pub wire: Vec<String>,
    /// The pipe row (Q67), which T49 puts on the strip.
    pub pipe: Vec<String>,
    /// The ground row's tools: [`LOWER`], [`RAISE`] and [`LEVEL`].
    pub ground: Vec<String>,
    /// The highest a tile may stand, which *raise* stops at.
    pub height_max: u8,
}

impl MarkRows {
    pub fn load() -> Result<MarkRows, String> {
        let data = sim::Data::load().map_err(|e| e.to_string())?;
        Ok(MarkRows {
            building: Sim::markable(&data, PlanKind::Building),
            paint: Sim::markable(&data, PlanKind::Paint),
            overlay: Sim::markable(&data, PlanKind::Overlay),
            wire: Sim::markable(&data, PlanKind::Wire),
            pipe: Sim::markable(&data, PlanKind::Pipe),
            ground: [LOWER, RAISE, LEVEL].map(str::to_string).to_vec(),
            height_max: data.world.height.max,
        })
    }

    /// A row's entries in strip order: every value, then *clear* (`None`),
    /// the plan to empty the slot.
    pub fn entries(&self, plan: PlanKind) -> Vec<Option<String>> {
        let values = match plan {
            PlanKind::Building => &self.building,
            PlanKind::Paint => &self.paint,
            PlanKind::Overlay => &self.overlay,
            PlanKind::Wire => &self.wire,
            PlanKind::Ground => &self.ground,
            PlanKind::Pipe => &self.pipe,
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

/// The team's plan of `kind` in `plans`.
pub fn slot(plans: &Plans, kind: PlanKind) -> &Option<Plan> {
    plans.of(kind)
}

/// What the team knows of a tile's ground: its height, and whether it is
/// ground or still rock to mine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Known {
    pub height: u8,
    pub ground: bool,
}

/// What a ground tool has to go on beside the tile itself: the known
/// height of the tile its stroke began on, for *level*, and the highest a
/// tile may stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stroke {
    pub anchor: Option<u8>,
    pub max: u8,
}

/// The height the team's ground plan in `held` asks for.
pub fn planned_height(held: Option<&Plans>) -> Option<u8> {
    match held.and_then(|p| p.ground.as_ref()) {
        Some(Plan::Set(v)) => v.parse().ok(),
        _ => None,
    }
}

/// The height a ground tool asks of a tile (`docs/07`, Marks): *lower*
/// one below, and *raise* one above, the height its plan already holds or
/// else the height the team knows it at; *level* the stroke's own. `None`
/// where there is no such height: below the lowest, above the highest, or
/// a stroke begun on a tile the team has not seen.
pub fn ground_target(tool: &str, known: Known, planned: Option<u8>, stroke: Stroke) -> Option<u8> {
    let base = planned.unwrap_or(known.height);
    match tool {
        LOWER => base.checked_sub(1),
        RAISE => base.checked_add(1).filter(|h| *h <= stroke.max),
        LEVEL => stroke.anchor,
        _ => None,
    }
}

/// What a ground tool submits for a tile: a `Mark` of the height it asks
/// for. Nothing for a tile the team has not seen, nor where the tile
/// already carries that plan. Where it asks for the height the tile is
/// already known to be ground at, there is nothing to plan, and a plan
/// the tile holds for some other height is withdrawn. *Clear* (`None`)
/// withdraws a plan and does nothing where there is none: a ground plan
/// has no plan to clear (`docs/03`).
pub fn ground_command(
    tool: Option<&str>,
    at: TilePos,
    known: Option<Known>,
    held: Option<&Plans>,
    stroke: Stroke,
) -> Option<CommandKind> {
    let unmark = CommandKind::Unmark {
        at,
        plan: PlanKind::Ground,
    };
    let holds = held.is_some_and(|p| p.ground.is_some());
    let Some(tool) = tool else {
        return holds.then_some(unmark);
    };
    let known = known?;
    let planned = planned_height(held);
    let target = ground_target(tool, known, planned, stroke)?;
    if planned == Some(target) {
        return None;
    }
    if known.ground && target == known.height {
        return holds.then_some(unmark);
    }
    Some(CommandKind::Mark {
        at,
        plan: PlanKind::Ground,
        value: Some(target.to_string()),
    })
}

/// What marking `at` with `tool` armed submits, given the team's plans
/// there: a `Mark` of its kind and value — except *clear* on a tile where
/// the team already holds a plan of that kind, which withdraws it as an
/// `Unmark` (Q44). Nothing unarmed, and nothing for a ground tool, whose
/// command is [`ground_command`]'s.
pub fn command(tool: &Tool, at: TilePos, held: Option<&Plans>) -> Option<CommandKind> {
    let Tool::Mark(plan, value) = tool else {
        return None;
    };
    if *plan == PlanKind::Ground {
        return None;
    }
    let holds = held.is_some_and(|p| slot(p, *plan).is_some());
    Some(if value.is_none() && holds {
        CommandKind::Unmark { at, plan: *plan }
    } else {
        CommandKind::Mark {
            at,
            plan: *plan,
            value: value.clone(),
        }
    })
}

/// Every tile of the rectangle with corners `a` and `b`, both included,
/// row by row from the lowest — what a `Shift`+drag fills (Q44).
pub fn rect(a: TilePos, b: TilePos) -> Vec<TilePos> {
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| TilePos::new(x, y)))
        .collect()
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
    ROWS.into_iter()
        .filter_map(|kind| {
            slot(plans, kind).as_ref().map(|p| match p {
                // A wire has no kinds: its plan reads `wire`, not
                // `wire wire`.
                Plan::Set(_) if kind == PlanKind::Wire => kind.name().to_string(),
                Plan::Set(v) => format!("{} {v}", kind.name()),
                Plan::Clear => format!("{} clear", kind.name()),
            })
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Whether a wire mark would change anything on a tile (`docs/03`, Q62):
/// a plan to lay one only where the team's wire does not lie, and a plan
/// to clear one only where it does — or where the team holds a wire plan,
/// which a clear withdraws as any clear does (Q44). The sim drops the rest
/// on their tick; the strip does not offer them.
pub fn wire_changes(lay: bool, wired: bool, holds_plan: bool) -> bool {
    if lay { !wired } else { wired || holds_plan }
}

/// The plans the armed tool would leave on a tile, for its ghost under the
/// cursor: `None` unarmed, and for a ground tool, whose plan depends on
/// the tile ([`ground_ghost`]).
pub fn ghost_of(tool: &Tool) -> Option<Plans> {
    let Tool::Mark(plan, value) = tool else {
        return None;
    };
    if *plan == PlanKind::Ground {
        return None;
    }
    let p = Some(match value {
        Some(v) => Plan::Set(v.clone()),
        None => Plan::Clear,
    });
    let mut plans = Plans::default();
    *plans.of_mut(*plan) = p;
    Some(plans)
}

/// The plan a ground tool's command would leave on a tile, for its ghost:
/// the height a `Mark` asks for, and nothing for an `Unmark`.
pub fn ground_ghost(command: &CommandKind) -> Option<Plans> {
    match command {
        CommandKind::Mark {
            plan: PlanKind::Ground,
            value: Some(v),
            ..
        } => Some(Plans {
            ground: Some(Plan::Set(v.clone())),
            ..Plans::default()
        }),
        _ => None,
    }
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
            vec![
                Some("printer".into()),
                Some("depot".into()),
                Some("solar".into()),
                Some("charger".into()),
                Some("pylon".into()),
                // Q67's four, as the data lists them; T49 arms the
                // entries past the ninth.
                Some("pump".into()),
                Some("tank".into()),
                Some("spout".into()),
                Some("aqueduct".into()),
                None
            ]
        );
        let paint = r.entries(PlanKind::Paint);
        assert_eq!(paint.len(), 9, "eight colors and clear");
        assert_eq!(paint[0], Some("red".into()));
        assert_eq!(paint[8], None);
        assert_eq!(r.entries(PlanKind::Overlay).last(), Some(&None));
        // The wire row (Q62): the wire, and its clear.
        assert_eq!(r.entries(PlanKind::Wire), vec![Some("wire".into()), None]);
        // The ground row (Q65): its three tools, and its clear.
        assert_eq!(
            r.entries(PlanKind::Ground),
            vec![
                Some(LOWER.into()),
                Some(RAISE.into()),
                Some(LEVEL.into()),
                None
            ]
        );
        assert_eq!(r.height_max, 9);
        assert_eq!(ROWS.map(row_key), ['B', 'P', 'O', 'C', 'G']);
    }

    #[test]
    fn the_ground_tools_each_ask_a_height_of_a_tile() {
        let at = TilePos::new(0, 0);
        let stroke = |anchor| Stroke { anchor, max: 9 };
        let ground = |height| {
            Some(Known {
                height,
                ground: true,
            })
        };
        let plan = |h: u8| Plans {
            ground: Some(Plan::Set(h.to_string())),
            ..Plans::default()
        };
        let mark = |h: u8| {
            Some(CommandKind::Mark {
                at,
                plan: PlanKind::Ground,
                value: Some(h.to_string()),
            })
        };
        let unmark = Some(CommandKind::Unmark {
            at,
            plan: PlanKind::Ground,
        });
        let send = |tool, known, held: Option<&Plans>, anchor| {
            ground_command(tool, at, known, held, stroke(anchor))
        };
        // Lower and raise: one from the height the team knows.
        assert_eq!(send(Some(LOWER), ground(4), None, None), mark(3));
        assert_eq!(send(Some(RAISE), ground(4), None, None), mark(5));
        // Or one from the height the tile's plan already holds, so a
        // second stroke goes a level further.
        assert_eq!(send(Some(LOWER), ground(4), Some(&plan(3)), None), mark(2));
        assert_eq!(send(Some(RAISE), ground(4), Some(&plan(6)), None), mark(7));
        // Never below the lowest, nor above the highest.
        assert_eq!(send(Some(LOWER), ground(0), None, None), None);
        assert_eq!(send(Some(RAISE), ground(9), None, None), None);
        assert_eq!(send(Some(RAISE), ground(4), Some(&plan(9)), None), None);
        // A stroke back to the height the ground has withdraws the plan.
        assert_eq!(send(Some(RAISE), ground(4), Some(&plan(3)), None), unmark);
        assert_eq!(send(Some(LOWER), ground(4), Some(&plan(5)), None), unmark);
        // Level: the height of the tile the stroke began on, for every
        // tile of it; nothing where the tile is ground at that height and
        // holds no plan, nor where it carries that plan already.
        assert_eq!(send(Some(LEVEL), ground(6), None, Some(4)), mark(4));
        assert_eq!(send(Some(LEVEL), ground(2), None, Some(4)), mark(4));
        assert_eq!(send(Some(LEVEL), ground(4), None, Some(4)), None);
        assert_eq!(send(Some(LEVEL), ground(6), Some(&plan(4)), Some(4)), None);
        assert_eq!(
            send(Some(LEVEL), ground(4), Some(&plan(2)), Some(4)),
            unmark
        );
        assert_eq!(send(Some(LEVEL), ground(6), None, None), None, "no anchor");
        // Rock at the height asked for is not ground at it: the plan
        // stands, and a bot mines it.
        let rock = Some(Known {
            height: 4,
            ground: false,
        });
        assert_eq!(send(Some(LEVEL), rock, None, Some(4)), mark(4));
        assert_eq!(send(Some(LOWER), rock, None, None), mark(3));
        // Nothing for a tile the team has not seen.
        for tool in [LOWER, RAISE, LEVEL] {
            assert_eq!(send(Some(tool), None, None, Some(4)), None);
        }
        // Clear withdraws a plan, and does nothing where there is none.
        assert_eq!(send(None, ground(4), Some(&plan(3)), None), unmark);
        assert_eq!(send(None, None, Some(&plan(3)), None), unmark);
        assert_eq!(send(None, ground(4), None, None), None);
        // The general command leaves the ground row to this one.
        let tool = Tool::Mark(PlanKind::Ground, Some(LOWER.into()));
        assert_eq!(command(&tool, at, None), None);
        assert_eq!(ghost_of(&tool), None);
        // The ghost is the plan a mark would leave, and nothing for an
        // unmark.
        assert_eq!(ground_ghost(&mark(3).unwrap()), Some(plan(3)));
        let withdrawn = CommandKind::Unmark {
            at,
            plan: PlanKind::Ground,
        };
        assert_eq!(ground_ghost(&withdrawn), None);
        assert_eq!(describe(&plan(3)), "ground 3");
    }

    #[test]
    fn a_wire_mark_is_offered_only_where_it_would_change_the_tile() {
        // Lay: only where the team's wire does not lie.
        assert!(wire_changes(true, false, false));
        assert!(!wire_changes(true, true, false));
        assert!(
            wire_changes(true, false, true),
            "over its own plan to clear"
        );
        // Clear: where a wire lies, or where a plan stands to withdraw.
        assert!(wire_changes(false, true, false));
        assert!(wire_changes(false, false, true));
        assert!(!wire_changes(false, false, false));
        let lay = Plans {
            wire: Some(Plan::Set("wire".into())),
            ..Plans::default()
        };
        assert_eq!(describe(&lay), "wire");
        let clear = Plans {
            wire: Some(Plan::Clear),
            paint: Some(Plan::Set("red".into())),
            ..Plans::default()
        };
        assert_eq!(describe(&clear), "paint red · wire clear");
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
        // The building row holds Q67's four now, so its clear is its
        // tenth entry, past what a digit reaches; T49 arms it by a click.
        assert_eq!(
            r.pick(PlanKind::Building, 10, &paint_red),
            Some(Tool::Mark(PlanKind::Building, None)),
            "the last entry is clear"
        );
        assert_eq!(r.pick(PlanKind::Building, 11, &Tool::Select), None);
        assert_eq!(r.pick(PlanKind::Paint, 0, &Tool::Select), None);
    }

    #[test]
    fn a_click_marks_clear_withdraws_a_held_plan_and_unarmed_does_neither() {
        let at = TilePos::new(2, -1);
        let tool = Tool::Mark(PlanKind::Overlay, Some("a".into()));
        let clear = Tool::Mark(PlanKind::Overlay, None);
        let mark = |value: Option<&str>| {
            Some(CommandKind::Mark {
                at,
                plan: PlanKind::Overlay,
                value: value.map(str::to_string),
            })
        };
        let unmark = Some(CommandKind::Unmark {
            at,
            plan: PlanKind::Overlay,
        });
        let held = Plans {
            overlay: Some(Plan::Set("b".into())),
            ..Plans::default()
        };
        let other_kind = Plans {
            paint: Some(Plan::Set("red".into())),
            ..Plans::default()
        };
        let held_clear = Plans {
            overlay: Some(Plan::Clear),
            ..Plans::default()
        };
        assert_eq!(command(&tool, at, None), mark(Some("a")));
        assert_eq!(
            command(&tool, at, Some(&held)),
            mark(Some("a")),
            "a value overwrites"
        );
        assert_eq!(
            command(&clear, at, None),
            mark(None),
            "nothing held: plan to clear"
        );
        assert_eq!(command(&clear, at, Some(&other_kind)), mark(None));
        assert_eq!(command(&clear, at, Some(&held)), unmark, "held: withdraw");
        assert_eq!(command(&clear, at, Some(&held_clear)), unmark);
        assert_eq!(command(&Tool::Select, at, None), None);
    }

    #[test]
    fn a_rectangle_fills_every_tile_between_its_corners() {
        let t = TilePos::new;
        assert_eq!(rect(t(1, 1), t(1, 1)), vec![t(1, 1)]);
        let r = rect(t(2, 0), t(0, -1));
        assert_eq!(
            r,
            vec![t(0, -1), t(1, -1), t(2, -1), t(0, 0), t(1, 0), t(2, 0)]
        );
        assert_eq!(rect(t(0, -1), t(2, 0)), r, "either corner first");
        assert_eq!(rect(t(-3, 4), t(2, -2)).len(), 6 * 7);
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
            wire: None,
            ground: None,
            pipe: None,
        };
        assert_eq!(describe(&plans), "paint red · overlay clear");
        assert_eq!(ghost_of(&Tool::Select), None);
        let g = ghost_of(&Tool::Mark(PlanKind::Building, None)).unwrap();
        assert_eq!(g.building, Some(Plan::Clear));
        assert!(g.paint.is_none() && g.overlay.is_none());
    }
}
