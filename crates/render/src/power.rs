//! Power on the map (`docs/07`, Machines on the map; Q62): what to draw,
//! worked out from the snapshot as plain values, so the rules can be
//! tested without a window. The drawing itself is `view`'s.

use sim::world::{GridFigures, MachineRecord, Model, TileMemory, TileState};
use sim::{TeamId, TilePos};
use std::collections::HashMap;

/// One team's wire on a tile, as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireLook {
    pub team: u32,
    /// Which neighbours it runs a line to: bit 0 north, 1 east, 2 south,
    /// 3 west.
    pub mask: u8,
    /// The player's own wire that is part of no grid, drawn dimmed.
    pub dim: bool,
}

/// The four neighbours in the mask's order: north, east, south, west.
pub const AROUND: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

/// The wires to draw on `p` as the player's team knows it: one per team
/// with a wire there, in team order, each with a line toward every
/// neighbour that carries the same team's wire or one of its buildings —
/// not a site, which conducts nothing (`docs/08`). `live` holds the
/// player's own wire tiles and whether each is part of a grid.
pub fn wires_on(
    p: TilePos,
    mem: &TileMemory,
    memory: &HashMap<TilePos, &TileMemory>,
    player: TeamId,
    live: &HashMap<TilePos, bool>,
) -> Vec<WireLook> {
    mem.wires
        .iter()
        .map(|team| {
            let mut mask = 0u8;
            for (bit, (dx, dy)) in AROUND.into_iter().enumerate() {
                let n = TilePos::new(p.x.wrapping_add(dx), p.y.wrapping_add(dy));
                let joined = memory
                    .get(&n)
                    .filter(|m| m.state != TileState::Unknown)
                    .is_some_and(|m| {
                        m.wires.contains(team)
                            || m.building
                                .as_ref()
                                .is_some_and(|b| b.team == *team && b.model != Model::Site)
                    });
                if joined {
                    mask |= 1 << bit;
                }
            }
            WireLook {
                team: team.0,
                mask,
                dim: *team == player && !live.get(&p).copied().unwrap_or(false),
            }
        })
        .collect()
}

/// The lines pylons draw (`docs/07`, Pylons): from each pylon to every
/// pylon and building of its team within its span, by squared distance,
/// sites excluded — each pair once, in order. `shown` is every building
/// the map draws, with its team, model and tile.
pub fn pylon_pairs(
    shown: &[(TeamId, Model, TilePos)],
    span: i64,
) -> Vec<(TeamId, TilePos, TilePos)> {
    let reach = span.saturating_mul(span);
    let mut out = Vec::new();
    for (team, model, at) in shown {
        if *model != Model::Pylon {
            continue;
        }
        for (other_team, other, to) in shown {
            if other_team != team || *other == Model::Site || *other == Model::Bot || at == to {
                continue;
            }
            if at.dist2(*to) > reach {
                continue;
            }
            // A pair of pylons is found from both ends; keep one.
            let (a, b) = if (at.x, at.y) <= (to.x, to.y) {
                (*at, *to)
            } else {
                (*to, *at)
            };
            out.push((*team, a, b));
        }
    }
    out.sort_by_key(|(team, a, b)| (team.0, a.x, a.y, b.x, b.y));
    out.dedup();
    out
}

/// A rate the sim keeps per tick, shown per second at the speed in force
/// (`docs/07`): the tick figure times the speed, and a dash while paused.
/// The renderer's own number, read back by nothing.
pub fn per_second(per_tick: lang::Num, speed: u64) -> String {
    if speed == 0 {
        return "—".to_string();
    }
    let speed = lang::Num::from_int(i128::from(speed)).unwrap_or(lang::Num::ZERO);
    match per_tick.checked_mul(speed) {
        Ok(n) => n.to_decimal(),
        Err(_) => "?".to_string(),
    }
}

/// The inspector's lines on a machine's power (`docs/07`, the windows):
/// its `charge` against its model's `battery`, where it has one, then a
/// bot's `flat` or a building's `powered` and `grid`.
pub fn machine_lines(r: &MachineRecord, battery: Option<lang::Num>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(b) = battery {
        out.push(format!(
            "charge {} / {}",
            r.charge.to_decimal(),
            b.to_decimal()
        ));
    }
    if r.flat == Some(true) {
        out.push("flat: out of power, charging to wake".to_string());
    }
    if let Some(powered) = r.powered {
        let grid = match r.grid.flatten() {
            Some(g) => format!("grid #{}", g.0),
            None => "no grid".to_string(),
        };
        let state = if powered { "powered" } else { "dark" };
        out.push(format!("{state} · {grid}"));
    }
    if r.flooded {
        out.push("flooded: losing health".to_string());
    }
    out
}

/// The inspector's lines on a tile's ground (`docs/07`; Q65): its height
/// and the water on it, in levels, and a spring's or a sink's rate with
/// its head or its sill. `parts` is how many parts of water stand a level
/// deep.
pub fn ground_lines(mem: &TileMemory, parts: u32) -> Vec<String> {
    let mut out = Vec::new();
    let height = mem.height.level();
    let parts = parts.max(1);
    // An amount of parts as levels, to the hundredth.
    let levels = |amount: u32| format!("{}.{:02}", amount / parts, amount % parts * 100 / parts);
    if mem.water == 0 {
        out.push(format!("height {height}"));
    } else {
        out.push(format!(
            "height {height} · water {} deep",
            levels(mem.water)
        ));
    }
    if let Some(rate) = mem.spring {
        out.push(match mem.head {
            Some(head) => format!(
                "spring: {} a tick, up to height {}",
                levels(rate),
                levels(head)
            ),
            None => format!("spring: {} a tick", levels(rate)),
        });
    }
    if let Some(rate) = mem.sink {
        out.push(match mem.sill {
            Some(sill) => format!(
                "drain: {} a tick, down to height {}",
                levels(rate),
                levels(sill)
            ),
            None => format!("drain: {} a tick", levels(rate)),
        });
    }
    out
}

/// The inspector's line on one of the player's own grids: its generation
/// and drain per second, and its stored charge against its capacity.
pub fn grid_line(g: &GridFigures, speed: u64) -> String {
    format!(
        "  grid #{}: +{}/s −{}/s · {} / {}",
        g.grid.0,
        per_second(g.generation, speed),
        per_second(g.drain, speed),
        g.stored.to_decimal(),
        g.capacity.to_decimal()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::EntityId;
    use sim::map::Terrain;
    use std::collections::BTreeSet;

    fn tile(wires: &[u32], building: Option<(u32, Model)>, at: TilePos) -> TileMemory {
        TileMemory {
            state: TileState::Visible,
            seen_at: 1,
            terrain: Terrain::Ground,
            height: sim::map::Height::default(),
            water: 0,
            spring: None,
            sink: None,
            head: None,
            sill: None,
            deposit: None,
            paint: None,
            overlay: None,
            wires: wires.iter().map(|t| TeamId(*t)).collect::<BTreeSet<_>>(),
            pipes: BTreeSet::new(),
            building: building.map(|(team, model)| MachineRecord {
                id: EntityId(9),
                name: "b".into(),
                kind: "building",
                model,
                team: TeamId(team),
                deployment: None,
                pos: at,
                health: lang::Num::ONE,
                busy: None,
                load: None,
                color: None,
                store: None,
                progress: 0,
                printing: None,
                charge: lang::Num::ZERO,
                flat: None,
                powered: Some(true),
                grid: Some(None),
                flooded: false,
                pace: None,
                water: 0,
                watered: Some(true),
                network: Some(None),
                pouring: None,
            }),
        }
    }

    #[test]
    fn a_wire_runs_to_its_teams_wire_and_buildings_beside_it_and_to_no_site() {
        let at = |x, y| TilePos::new(x, y);
        let tiles = [
            (at(0, 0), tile(&[0, 1], None, at(0, 0))),
            // North: the same team's wire. East: its charger. South: a
            // site of its own. West: a rival's wire only.
            (at(0, 1), tile(&[0], None, at(0, 1))),
            (at(1, 0), tile(&[], Some((0, Model::Charger)), at(1, 0))),
            (at(0, -1), tile(&[], Some((0, Model::Site)), at(0, -1))),
            (at(-1, 0), tile(&[1], None, at(-1, 0))),
        ];
        let memory: HashMap<TilePos, &TileMemory> = tiles.iter().map(|(p, m)| (*p, m)).collect();
        let live: HashMap<TilePos, bool> = [(at(0, 0), true)].into_iter().collect();
        let looks = wires_on(at(0, 0), &tiles[0].1, &memory, TeamId(0), &live);
        assert_eq!(
            looks,
            [
                WireLook {
                    team: 0,
                    mask: 0b0011,
                    dim: false
                },
                // The rival's runs west, to its own; it is never dimmed.
                WireLook {
                    team: 1,
                    mask: 0b1000,
                    dim: false
                },
            ]
        );
        // The player's own wire that the power step found on no grid.
        let off: HashMap<TilePos, bool> = [(at(0, 0), false)].into_iter().collect();
        let looks = wires_on(at(0, 0), &tiles[0].1, &memory, TeamId(0), &off);
        assert!(looks[0].dim && !looks[1].dim);
        // A lone wire still shows, joined to nothing.
        let alone = tile(&[0], None, at(5, 5));
        let memory: HashMap<TilePos, &TileMemory> = [(at(5, 5), &alone)].into_iter().collect();
        assert_eq!(
            wires_on(at(5, 5), &alone, &memory, TeamId(1), &live)[0].mask,
            0
        );
    }

    #[test]
    fn a_pylon_draws_to_its_teams_pylons_and_buildings_within_its_span_once_each() {
        let at = |x, y| TilePos::new(x, y);
        let t0 = TeamId(0);
        let shown = [
            (t0, Model::Pylon, at(0, 0)),
            (t0, Model::Pylon, at(3, 0)),
            (t0, Model::Charger, at(0, 4)),
            // Out of reach, a site, a bot, and a rival's printer in reach.
            (t0, Model::Solar, at(5, 0)),
            (t0, Model::Site, at(1, 0)),
            (t0, Model::Bot, at(0, 1)),
            (TeamId(1), Model::Printer, at(1, 1)),
        ];
        assert_eq!(
            pylon_pairs(&shown, 4),
            [
                // The two pylons, once; the charger, four tiles from the
                // first and five from the second; the panel, two from the
                // second and five from the first.
                (t0, at(0, 0), at(0, 4)),
                (t0, at(0, 0), at(3, 0)),
                (t0, at(3, 0), at(5, 0)),
            ]
        );
        assert!(pylon_pairs(&shown[2..], 4).is_empty(), "no pylon, no line");
    }

    #[test]
    fn the_inspector_says_a_machines_charge_and_whether_it_is_out() {
        let at = TilePos::new(0, 0);
        let num = |s: &str| lang::Num::parse_literal(s).unwrap();
        let mut charger = tile(&[], Some((0, Model::Charger)), at).building.unwrap();
        charger.charge = num("12.5");
        charger.grid = Some(Some(EntityId(4)));
        assert_eq!(
            machine_lines(&charger, Some(num("1000"))),
            ["charge 12.5 / 1000", "powered · grid #4"]
        );
        // A dark building on no grid, of a model with no battery.
        let mut panel = tile(&[], Some((0, Model::Solar)), at).building.unwrap();
        panel.powered = Some(false);
        assert_eq!(machine_lines(&panel, None), ["dark · no grid"]);
        // A bot has no `powered` and no `grid`; flat, it says so.
        let mut bot = charger.clone();
        (bot.powered, bot.grid, bot.flat) = (None, None, Some(true));
        assert_eq!(
            machine_lines(&bot, Some(num("100"))),
            ["charge 12.5 / 100", "flat: out of power, charging to wake"]
        );
        bot.flat = Some(false);
        assert_eq!(machine_lines(&bot, Some(num("100"))).len(), 1);
        // Flooded, a machine of either kind says so.
        panel.flooded = true;
        assert_eq!(
            machine_lines(&panel, None),
            ["dark · no grid", "flooded: losing health"]
        );
    }

    #[test]
    fn the_inspector_says_a_tiles_height_its_water_and_what_feeds_or_drains_it() {
        let at = TilePos::new(0, 0);
        let mut dry = tile(&[], None, at);
        dry.height = sim::map::Height::new(5, 9).unwrap();
        assert_eq!(ground_lines(&dry, 100), ["height 5"]);
        // Water in levels, to the hundredth.
        let mut wet = dry.clone();
        wet.water = 168;
        assert_eq!(ground_lines(&wet, 100), ["height 5 · water 1.68 deep"]);
        wet.water = 5;
        assert_eq!(ground_lines(&wet, 100), ["height 5 · water 0.05 deep"]);
        // A spring with a head, and one without.
        let mut spring = wet.clone();
        (spring.spring, spring.head) = (Some(40), Some(700));
        assert_eq!(
            ground_lines(&spring, 100)[1],
            "spring: 0.40 a tick, up to height 7.00"
        );
        spring.head = None;
        assert_eq!(ground_lines(&spring, 100)[1], "spring: 0.40 a tick");
        // A drain with a sill, and one without.
        let mut drain = dry.clone();
        (drain.sink, drain.sill) = (Some(25), Some(690));
        assert_eq!(
            ground_lines(&drain, 100),
            ["height 5", "drain: 0.25 a tick, down to height 6.90"]
        );
        drain.sill = None;
        assert_eq!(ground_lines(&drain, 100)[1], "drain: 0.25 a tick");
    }

    #[test]
    fn a_grid_line_gives_its_rates_per_second_and_its_store() {
        let num = |s: &str| lang::Num::parse_literal(s).unwrap();
        let g = GridFigures {
            grid: EntityId(7),
            generation: num("6.5"),
            drain: num("2"),
            stored: num("480"),
            capacity: num("1500"),
        };
        assert_eq!(grid_line(&g, 10), "  grid #7: +65/s −20/s · 480 / 1500");
        assert_eq!(grid_line(&g, 0), "  grid #7: +—/s −—/s · 480 / 1500");
    }

    #[test]
    fn a_rate_is_shown_per_second_at_the_speed_in_force() {
        let half = lang::Num::parse_literal("0.5").unwrap();
        assert_eq!(per_second(half, 10), "5");
        assert_eq!(per_second(half, 1), "0.5");
        assert_eq!(per_second(half, 0), "—", "paused");
    }
}
