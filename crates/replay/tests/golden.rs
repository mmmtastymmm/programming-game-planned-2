//! The golden replay (CLAUDE.md; `docs/06`, Testing): `(map, command log)`
//! to a checked-in hash stream. Any behaviour change in `sim` or `lang` —
//! deterministic or not — fails here until the fixture is regenerated on
//! purpose:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test -p replay --test golden
//! ```
//!
//! and the PR explains the hash change. The scenario: the first map, with
//! a spring and a sink in its south-west lake, a player team running the bundles under `tests/golden/player/`, and the
//! Fool as the opposition, with a **mid-match redeploy** of the player's
//! `bot` deployment (T7: the hash-affecting path most likely to differ
//! between peers), a speed change, and marks — among them a printer plan
//! the player's bots build, so the rising price, construction and two
//! printers taking turns (Q42) all feed the hash; and, for power (Q62,
//! Q63), a charger, a pylon and a tile of wire that join the two printers
//! and the charger into one grid, and one bot that paces, so that
//! inside the run's 540 ticks a battery falls to low power, the hook runs
//! and the bot recharges; and, for the ground and water (Q65, Q66), a
//! hollow dug beside the lake, a pylon built in it, and a channel cut from
//! the lake that floods the pylon until it dies of it, while the spring
//! and the sink keep the lake's water moving; and, for moving (Q65), the
//! pacing bot first digs a tile and fills the one beside it, falls from
//! the one onto the other, and mines a rock away, and then, pacing at
//! full pace, bonks a bot that has stopped beside its lane; and, for the
//! water network (Q67), a second program for the bots, deployed once the
//! planned printer's site is full, under which they lay wire south from
//! the base, build a tank, a pump beside the flooded hollow and a spout,
//! join the spout to the tank with a tile of wire and of pipe, and dig a
//! hollow that the spout then fills from the lake. The alive test
//! asserts on what the match must have done, so a run that did nothing
//! cannot score green.

use replay::Replay;
use sim::world::Model;
use sim::{Command, CommandKind, Data, Map, PlanKind, TeamId, TilePos};
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// A raw bundle directory, for the redeploy's second version.
fn bundle(dir: &Path) -> sim::Bundle {
    sim::script::read_bundle(dir).unwrap_or_else(|e| panic!("{e}"))
}

/// The player's tree (Q41), composed: one bundle per robot file.
fn tree(dir: &Path) -> std::collections::BTreeMap<String, sim::Bundle> {
    sim::script::read_tree(dir).unwrap_or_else(|e| panic!("{e}"))
}

/// The fixture's map: the first map, with water that moves. A spring in
/// the south-west lake's far corner brings ground water up to the lake's
/// rim, and a sink beside it takes the lake down to half a level below,
/// so there is a flow across the lake on every tick of the match, and the
/// channel the scenario cuts from it stays fed. The spring brings up
/// twice what the sink takes, so that the sink still has water to take
/// while the scenario's pump lifts a sink's worth from the hollow (Q67).
/// The first map gets water of its own with T47; until then the fixture
/// adds its own.
fn fixture_map() -> String {
    let first = std::fs::read_to_string(data_dir().join("maps/first.toml")).expect("first map");
    let key = "terrain = \"\"\"";
    assert!(first.contains(key), "the first map has no terrain key");
    let mut text = first.replacen(key, &format!("ground_water = 2\n{key}"), 1);
    text.push_str(
        "\n[[spring]]\nat = [-10, -6]\nrate = \"0.1\"\nhead = 4\n\n\
         [[sink]]\nat = [-9, -6]\nrate = \"0.05\"\nsill = \"3.5\"\n",
    );
    text
}

/// The hollow the scenario's digger digs first, two tiles north of the
/// lake, and where the pylon the flood takes is built.
const HOLLOW: TilePos = TilePos::new(-10, -3);
/// The channel: the tile between the hollow and the lake, dug last.
const CHANNEL: TilePos = TilePos::new(-10, -4);
/// The ticks the pylon's plan and the channel's are agreed for: after the
/// hollow is dug, at about tick 350, and after the pylon stands in it,
/// five ticks from its plan. A pylon lasts fifty ticks flooded, so the
/// channel is early enough for the run to see it die.
const PYLON_MARK: u64 = 360;
const CHANNEL_MARK: u64 = 385;

/// The ground the scenario's pacing bot works before it paces, east of
/// its lane (`bot.py`'s `CREW`): a tile it digs a level down, the tile
/// east of that, which it fills a level up with the earth and then falls
/// from onto the first, and a rock it mines away.
const DUG: TilePos = TilePos::new(-3, 7);
const FILLED: TilePos = TilePos::new(-2, 7);
const MINED: TilePos = TilePos::new(0, 6);

/// The tick the waterworks program and its plans are agreed for (Q67):
/// two ticks after the planned printer's site is full, at 384, which is
/// when the haulers run out of anything to haul to.
const WATERWORKS: u64 = 386;
/// The waterworks, south of the base beside the flooded hollow: the wire
/// that carries the base's grid down to the tank, the tank, the pump west
/// of it and north of the hollow, the tile east of the tank that takes a
/// wire and a pipe, the spout east of that, and the hollow east of the
/// spout, which `water/main.py` has a bot dig and the spout fills.
const WATER_WIRE: [TilePos; 3] = [
    TilePos::new(-9, 1),
    TilePos::new(-9, 0),
    TilePos::new(-9, -1),
];
const WATER_TANK: TilePos = TilePos::new(-9, -2);
const WATER_PUMP: TilePos = TilePos::new(-10, -2);
const WATER_LINE: TilePos = TilePos::new(-8, -2);
const WATER_SPOUT: TilePos = TilePos::new(-7, -2);
const WATER_HOLLOW: TilePos = TilePos::new(-6, -2);

/// The canonical golden scenario.
fn golden_replay() -> Replay {
    let map_text = fixture_map();
    let data = Data::load().expect("data");
    let map = Map::parse(&map_text, &data).expect("map");
    let player = TeamId(0);
    let fool = TeamId(1);
    let mut commands = Vec::new();
    // The player's opening set: one deploy per deployment, agreed for tick 0.
    let p = fixture_dir().join("player");
    let composed = tree(&p);
    commands.push(Command::new(
        0,
        player,
        0,
        CommandKind::Deploy {
            deployment: "printer".into(),
            bundle: composed["printer"].clone(),
        },
    ));
    commands.push(Command::new(
        0,
        player,
        1,
        CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: composed["bot"].clone(),
        },
    ));
    // A paint plan and an overlay plan beside the start, for the marks
    // path: agreed for tick 2, since paint and overlay go only on ground
    // the team has seen (Q49), and its first sight lands in tick 1.
    let start = map.teams[0][0];
    commands.push(Command::new(
        2,
        player,
        0,
        CommandKind::Mark {
            at: TilePos::new(start.x, start.y.saturating_add(1)),
            plan: PlanKind::Paint,
            value: Some("red".into()),
        },
    ));
    commands.push(Command::new(
        2,
        player,
        1,
        CommandKind::Mark {
            at: TilePos::new(start.x.saturating_add(1), start.y),
            plan: PlanKind::Overlay,
            value: Some("a".into()),
        },
    ));
    // A printer plan (Q42), which the player's bots haul ore into and
    // build: the rising price, construction and the turn order between two
    // printers all feed the hash.
    commands.push(Command::new(
        0,
        player,
        4,
        CommandKind::Mark {
            at: printer_plan(&map),
            plan: PlanKind::Building,
            value: Some("printer".into()),
        },
    ));
    // Power (Q62): a charger two tiles east of the start, a tile of wire
    // between it and the starting printer, and a pylon whose span reaches
    // the charger and the planned printer, so the three buildings that
    // hold or make charge end as one grid. The wire plan waits for the
    // team's first sight, like paint.
    commands.push(Command::new(
        0,
        player,
        5,
        CommandKind::Mark {
            at: charger_plan(&map),
            plan: PlanKind::Building,
            value: Some("charger".into()),
        },
    ));
    commands.push(Command::new(
        0,
        player,
        6,
        CommandKind::Mark {
            at: pylon_plan(&map),
            plan: PlanKind::Building,
            value: Some("pylon".into()),
        },
    ));
    commands.push(Command::new(
        2,
        player,
        2,
        CommandKind::Mark {
            at: wire_plan(&map),
            plan: PlanKind::Wire,
            value: Some("wire".into()),
        },
    ));
    // The ground and the water (Q65): a hollow a level down, two tiles
    // north of the lake, planned once the team has seen the tile; then a
    // pylon in it; then the tile between it and the lake, which lets the
    // lake in and floods the pylon.
    let down = map
        .tile(HOLLOW)
        .expect("on the map")
        .height
        .level()
        .saturating_sub(1);
    commands.push(Command::new(
        2,
        player,
        3,
        CommandKind::Mark {
            at: HOLLOW,
            plan: PlanKind::Ground,
            value: Some(down.to_string()),
        },
    ));
    commands.push(Command::new(
        PYLON_MARK,
        player,
        0,
        CommandKind::Mark {
            at: HOLLOW,
            plan: PlanKind::Building,
            value: Some("pylon".into()),
        },
    ));
    commands.push(Command::new(
        CHANNEL_MARK,
        player,
        0,
        CommandKind::Mark {
            at: CHANNEL,
            plan: PlanKind::Ground,
            value: Some(down.to_string()),
        },
    ));
    // The water network (Q67): the player's spout has a program from the
    // start, and once the haulers have nothing left to haul to, the
    // `bot` deployment is redeployed with the waterworks program — the
    // same bundle with another `main.py`, so the hooks stay — and the
    // plans it carries out are placed: a line of wire south from the
    // planned printer, a tank at its end with a pump west of it, beside
    // the flooded hollow, and a spout a tile east, joined to the tank by
    // a tile that takes both a wire and a pipe.
    commands.push(Command::new(
        0,
        player,
        7,
        CommandKind::Deploy {
            deployment: "spout".into(),
            bundle: composed["spout"].clone(),
        },
    ));
    let mut waterworks = composed["bot"].clone();
    let main = std::fs::read_to_string(fixture_dir().join("water/main.py")).expect("water/main.py");
    for (name, text) in &mut waterworks.files {
        if name == "main.py" {
            *text = main.clone();
        }
    }
    commands.push(Command::new(
        WATERWORKS,
        player,
        0,
        CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: waterworks,
        },
    ));
    let mut seq = 1u32;
    let mut mark = |at: TilePos, plan: PlanKind, value: &str| {
        commands.push(Command::new(
            WATERWORKS,
            player,
            seq,
            CommandKind::Mark {
                at,
                plan,
                value: Some(value.into()),
            },
        ));
        seq = seq.saturating_add(1);
    };
    for at in WATER_WIRE {
        mark(at, PlanKind::Wire, "wire");
    }
    mark(WATER_LINE, PlanKind::Wire, "wire");
    mark(WATER_LINE, PlanKind::Pipe, "pipe");
    mark(WATER_TANK, PlanKind::Building, "tank");
    mark(WATER_PUMP, PlanKind::Building, "pump");
    mark(WATER_SPOUT, PlanKind::Building, "spout");
    // The Fool's script, tiles relative to its first start.
    let script =
        sim::script::Script::load(&data_dir().join("opposition/fool")).expect("the Fool's script");
    commands.extend(script.resolve(fool, map.teams[1][0]));
    // Mid-match: a speed change, an unmark, and the redeploy of `bot`.
    commands.push(Command::new(40, player, 0, CommandKind::SetSpeed(2)));
    commands.push(Command::new(
        55,
        player,
        0,
        CommandKind::Unmark {
            at: TilePos::new(start.x.saturating_add(1), start.y),
            plan: PlanKind::Overlay,
        },
    ));
    commands.push(Command::new(
        REDEPLOY,
        player,
        0,
        CommandKind::Deploy {
            deployment: "bot".into(),
            bundle: bundle(&fixture_dir().join("red2")),
        },
    ));

    // Built in code, not saved from a match, so it records no version and
    // no final hash (Q60): the stored hash stream is what it is held to.
    Replay {
        map: map_text,
        commands,
        ticks: 540,
        version: None,
        final_hash: None,
        team: None,
    }
}

/// The tick the golden scenario's `bot` redeploy to the guards is agreed
/// for, of its 540: after the pacing bot has run its battery down to the
/// low-power level and recharged, and the last of the planned buildings
/// is on its way up — the waterworks' sites are full by about tick 480,
/// and stand and pump by themselves from 492.
const REDEPLOY: u64 = 500;

fn charger_plan(map: &Map) -> TilePos {
    let start = map.teams[0][0];
    TilePos::new(start.x.saturating_add(2), start.y)
}

fn wire_plan(map: &Map) -> TilePos {
    let start = map.teams[0][0];
    TilePos::new(start.x.saturating_add(1), start.y)
}

fn pylon_plan(map: &Map) -> TilePos {
    let start = map.teams[0][0];
    TilePos::new(start.x.saturating_add(2), start.y.saturating_sub(1))
}

/// Where the golden scenario's printer plan goes: two tiles south of the
/// player's start.
fn printer_plan(map: &Map) -> TilePos {
    let start = map.teams[0][0];
    TilePos::new(start.x, start.y.saturating_sub(2))
}

/// The golden scenario stepped tick by tick, as `Replay::execute` steps
/// it, counting what only shows on the way: bot-ticks spent in
/// `on_low_power`, and recharges — a bot's charge jumping, where its own
/// solar adds a tenth a tick.
fn power_seen(replay: &Replay) -> (u32, u32) {
    let data = Data::load().expect("data");
    let mut sim = sim::Sim::new(Map::parse(&replay.map, &data).expect("map"));
    let log = replay.log();
    let five = lang::Num::from_int(5).expect("five");
    let mut before: std::collections::BTreeMap<sim::EntityId, lang::Num> = Default::default();
    let (mut hooks, mut recharges) = (0u32, 0u32);
    for tick in 1..=replay.ticks {
        let mut commands = if tick == 1 {
            log.sets_for(0)
        } else {
            Vec::new()
        };
        commands.extend(log.sets_for(tick));
        sim.step(&commands, &[]);
        for m in sim.world.machines.values() {
            if m.team != TeamId(0) || m.model != Model::Bot {
                continue;
            }
            if let Some(was) = before.insert(m.id, m.charge)
                && m.charge.checked_sub(was).is_ok_and(|gain| gain > five)
            {
                recharges = recharges.saturating_add(1);
            }
            if m.program
                .as_ref()
                .is_some_and(|p| p.mode_name() == "low_power")
            {
                hooks = hooks.saturating_add(1);
            }
        }
    }
    (hooks, recharges)
}

/// What the water did, seen tick by tick.
struct WaterSeen {
    /// Ticks on which the lake's tiles held other water than the tick
    /// before: the lake finding its level.
    lake_moved: u32,
    /// The least and the most the ground water held after a tick.
    ground: (u64, u64),
    /// Whether the water on the tiles and under them ever summed to
    /// anything but what the map began with.
    total_held: bool,
    /// Ticks the pylon in the hollow stood there, and of them the ticks
    /// it was flooded and dark.
    pylon: u32,
    flooded: u32,
    /// Its health when last seen, and whether it was flooded then.
    last: Option<(lang::Num, bool)>,
    /// Ticks the spout ended turned east, and ticks the water in the
    /// hollow it faces rose; and whether it ever rose on any other.
    pouring: u32,
    poured: u32,
    rose_unpoured: bool,
    /// The most the scenario's tank ever held, in parts.
    stored: u32,
}

/// The golden scenario stepped tick by tick for its water: the lake, the
/// ground water, and the pylon in the hollow.
fn water_seen(replay: &Replay) -> WaterSeen {
    let data = Data::load().expect("data");
    let mut sim = sim::Sim::new(Map::parse(&replay.map, &data).expect("map"));
    let log = replay.log();
    let lake: Vec<usize> = [(-10, -6), (-9, -6), (-10, -5), (-9, -5)]
        .into_iter()
        .map(|(x, y)| sim.world.tile_index(TilePos::new(x, y)).expect("lake"))
        .collect();
    let water =
        |sim: &sim::Sim| -> Vec<u32> { lake.iter().map(|i| sim.world.tiles[*i].water).collect() };
    let total = sim.world.water_total();
    let mut before = water(&sim);
    let mut seen = WaterSeen {
        lake_moved: 0,
        ground: (sim.world.ground_water, sim.world.ground_water),
        total_held: true,
        pylon: 0,
        flooded: 0,
        last: None,
        pouring: 0,
        poured: 0,
        rose_unpoured: false,
        stored: 0,
    };
    let hollow = sim.world.tile_index(WATER_HOLLOW).expect("hollow");
    let mut hollow_held = sim.world.tiles[hollow].water;
    for tick in 1..=replay.ticks {
        let mut commands = if tick == 1 {
            log.sets_for(0)
        } else {
            Vec::new()
        };
        commands.extend(log.sets_for(tick));
        sim.step(&commands, &[]);
        let now = water(&sim);
        if now != before {
            seen.lake_moved = seen.lake_moved.saturating_add(1);
        }
        before = now;
        let ground = sim.world.ground_water;
        seen.ground = (seen.ground.0.min(ground), seen.ground.1.max(ground));
        seen.total_held &= sim.world.water_total() == total;
        let pylon = sim
            .world
            .machines
            .values()
            .find(|m| m.model == Model::Pylon && m.pos == HOLLOW);
        if let Some(m) = pylon {
            seen.pylon = seen.pylon.saturating_add(1);
            if m.flooded && !m.powered {
                seen.flooded = seen.flooded.saturating_add(1);
            }
            seen.last = Some((m.health, m.flooded));
        }
        // The spout turns in its slice and the network step pours in the
        // same tick, so a tick the hollow rose in ends with it turned.
        let turned = sim
            .world
            .machines
            .values()
            .any(|m| m.model == Model::Spout && m.pouring == Some(sim::map::Dir::E));
        let rose = sim.world.tiles[hollow].water > hollow_held;
        hollow_held = sim.world.tiles[hollow].water;
        seen.pouring = seen.pouring.saturating_add(u32::from(turned));
        seen.poured = seen.poured.saturating_add(u32::from(rose));
        seen.rose_unpoured |= rose && !turned;
        let stored = sim
            .world
            .machines
            .values()
            .filter(|m| m.model == Model::Tank)
            .map(|m| m.water)
            .max()
            .unwrap_or(0);
        seen.stored = seen.stored.max(stored);
    }
    seen
}

fn tile_at(w: &sim::World, at: TilePos) -> &sim::world::Tile {
    w.tile(at).expect("on the map")
}

/// What moving did (Q65; `docs/03`, Moving), seen tick by tick.
struct MovingSeen {
    /// The most pace a player bot ever had.
    pace: u64,
    /// Each fall: a bot that a tick left on a tile lower than the one it
    /// stood on by more than a step — the levels, and the health it lost
    /// in that tick.
    falls: Vec<(u8, lang::Num)>,
    /// The ticks a bonk was heard beginning.
    bonks: u32,
    /// Each bonk that put a bot a tile onward: the tick, what that bot
    /// was doing, and whether the bonk ended it with nothing done — no
    /// action left, and its load as it was. A bot that left the tile by
    /// a move of its own is none of these: it was in that move as the
    /// tick began, or was heard beginning it in the tick.
    shoved: Vec<(u64, Option<&'static str>, bool)>,
}

/// The golden scenario stepped tick by tick for its moving.
fn moving_seen(replay: &Replay) -> MovingSeen {
    use sim::world::Effect;
    let data = Data::load().expect("data");
    let mut sim = sim::Sim::new(Map::parse(&replay.map, &data).expect("map"));
    let log = replay.log();
    let step = data.world.height.step;
    let mut seen = MovingSeen {
        pace: 0,
        falls: Vec::new(),
        bonks: 0,
        shoved: Vec::new(),
    };
    for tick in 1..=replay.ticks {
        let mut commands = if tick == 1 {
            log.sets_for(0)
        } else {
            Vec::new()
        };
        commands.extend(log.sets_for(tick));
        // Where every bot stood and what it had as the tick began, and
        // the bonks that will complete in it.
        let before: std::collections::BTreeMap<sim::EntityId, (TilePos, lang::Num)> = sim
            .world
            .machines
            .values()
            .filter(|m| m.model == Model::Bot)
            .map(|m| (m.id, (m.pos, m.health)))
            .collect();
        let due: Vec<_> = sim
            .world
            .machines
            .values()
            .filter_map(|m| match m.action.as_ref() {
                Some(a) if a.remaining <= 1 => match a.effect {
                    Effect::Bonk { to, target } => Some((to, target)),
                    _ => None,
                },
                _ => None,
            })
            .filter_map(|(to, target)| {
                let t = sim.world.machines.get(&target)?;
                (t.pos == to && !t.in_move()).then(|| (to, target, t.busy(), t.load.clone()))
            })
            .collect();
        sim.step(&commands, &[]);
        let w = &sim.world;
        let level = |p: TilePos| w.tile(p).expect("on the map").height.level();
        for m in w.machines.values().filter(|m| m.model == Model::Bot) {
            if m.team == TeamId(0) {
                seen.pace = seen.pace.max(m.pace);
            }
            let Some((pos, health)) = before.get(&m.id).copied() else {
                continue;
            };
            let fell = level(pos).saturating_sub(level(m.pos));
            if m.pos != pos && fell > step {
                let lost = health.checked_sub(m.health).expect("health");
                seen.falls.push((fell, lost));
            }
        }
        seen.bonks = seen.bonks.saturating_add(
            w.sounds_prev
                .iter()
                .filter(|snd| snd.cause == "bonk")
                .count() as u32,
        );
        for (to, target, busy, load) in due {
            let Some(t) = w.machines.get(&target) else {
                continue;
            };
            let left = w
                .sounds_prev
                .iter()
                .any(|snd| snd.cause == "move" && snd.pos == to);
            if t.pos != to && !left {
                seen.shoved
                    .push((tick, busy, t.action.is_none() && t.load == load));
            }
        }
    }
    seen
}

fn hashes_to_text(hashes: &[u64]) -> String {
    hashes.iter().map(|h| format!("{h:016x}\n")).collect()
}

#[test]
fn golden_replay_matches_stored_fixture() {
    let dir = fixture_dir();
    let replay_path = dir.join("showcase.replay.ron");
    let hashes_path = dir.join("showcase.hashes.txt");
    let replay = golden_replay();
    let run = replay.execute();
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&replay_path, replay.to_ron()).expect("write replay fixture");
        std::fs::write(&hashes_path, hashes_to_text(&run.hashes)).expect("write hash fixture");
        eprintln!("golden fixtures regenerated — explain the hash change in the PR (CLAUDE.md)");
        return;
    }
    let stored_replay = Replay::from_ron(
        &std::fs::read_to_string(&replay_path).expect("stored replay fixture exists"),
    )
    .expect("stored replay parses");
    assert_eq!(
        stored_replay, replay,
        "the in-code scenario and the stored artifact diverged — regenerate with UPDATE_GOLDEN=1 and explain why in the PR"
    );
    let stored_hashes = std::fs::read_to_string(&hashes_path).expect("stored hashes exist");
    assert_eq!(
        stored_hashes,
        hashes_to_text(&run.hashes),
        "replay hash drift: sim behaviour changed. If intentional, regenerate with UPDATE_GOLDEN=1 and explain the change in the PR (CLAUDE.md)"
    );
}

/// The fixture is only worth its bytes if the match genuinely happened.
#[test]
fn golden_scenario_is_alive() {
    let replay = golden_replay();
    let run = replay.execute();
    assert!(
        run.dropped.is_empty(),
        "commands were dropped: {:?}",
        run.dropped
    );
    // Written out, not read from the scenario: a scenario cut short must
    // fail here, not agree with itself.
    assert_eq!(
        run.hashes.len(),
        540,
        "the match ended early: {:?}",
        run.sim.ended()
    );
    assert!(
        run.hashes.windows(2).all(|w| w[0] != w[1]),
        "two consecutive ticks hashed the same"
    );
    let w = &run.sim.world;
    let bots = |t: u32| {
        w.machines
            .values()
            .filter(|m| m.team == TeamId(t) && m.model == Model::Bot)
            .count()
    };
    assert!(bots(0) >= 2, "the player printed {} bots", bots(0));
    assert!(bots(1) >= 2, "the Fool printed {} bots", bots(1));
    assert!(
        w.machines
            .values()
            .any(|m| m.team == TeamId(1) && matches!(m.model, Model::Depot | Model::Site)),
        "the Fool placed no site"
    );
    assert!(
        w.machines.values().any(|m| m.team == TeamId(0)
            && m.deployment.as_deref() == Some("bot")
            && m.program
                .as_ref()
                .is_some_and(|p| p.global("guard").is_some())),
        "no player bot took the redeploy"
    );
    // The first program wore its job as its color (Q43); the redeploy's
    // program sets none, so a color still worn at the end survived it.
    assert!(
        w.machines
            .values()
            .any(|m| m.team == TeamId(0) && matches!(m.color.as_deref(), Some("orange" | "cyan"))),
        "no player bot wears a job color"
    );
    let ore_moved = w.tiles.iter().any(|t| {
        t.deposit
            .as_ref()
            .is_some_and(|d| d.amount["ore"] < d.cap["ore"])
    }) || w.machines.values().any(|m| {
        m.load.values().any(|v| *v > lang::Num::ZERO)
            || m.store.values().any(|v| *v > lang::Num::ZERO)
    });
    assert!(ore_moved, "no ore was ever picked");
    assert_eq!(w.speed, 2, "the SetSpeed did not land");
    // The bots built the planned printer (by about tick 435), so the
    // player ends with two, and the plan was consumed.
    let printers: Vec<u32> = w
        .machines
        .values()
        .filter(|m| m.team == TeamId(0) && m.model == Model::Printer)
        .map(|m| m.id.0)
        .collect();
    assert_eq!(printers.len(), 2, "the planned printer was never built");
    assert!(w.plans(TeamId(0), printer_plan(&w.map)).is_none());
    // Power (Q62, Q63): the charger and the pylon were built and the wire
    // laid, and with the second printer they are one grid, labelled by
    // its oldest building, the starting printer.
    let player = TeamId(0);
    let of = |model: Model| -> Vec<&sim::world::Machine> {
        w.machines
            .values()
            .filter(|m| m.team == player && m.model == model)
            .collect()
    };
    assert_eq!(
        of(Model::Charger).len(),
        1,
        "the planned charger was never built"
    );
    // The base's pylon alone: the one in the hollow is gone by now.
    assert_eq!(
        of(Model::Pylon).len(),
        1,
        "the planned pylon was never built, or the flooded one outlived the run"
    );
    assert_eq!(of(Model::Pylon)[0].pos, pylon_plan(&w.map));
    assert!(
        of(Model::Pylon)[0].program.is_none(),
        "a pylon runs no program"
    );
    assert!(
        w.has_wire(player, wire_plan(&w.map)),
        "the planned wire was never laid"
    );
    assert!(w.plans(player, wire_plan(&w.map)).is_none());
    let first = printers.iter().min().copied().map(sim::EntityId);
    for m in w
        .machines
        .values()
        .filter(|m| m.team == player && m.is_building())
    {
        assert_eq!(m.grid, first, "{} is on another grid", m.name);
        assert!(m.powered, "{} is dark", m.name);
    }
    assert_eq!(w.teams[&player].grids.len(), 1);
    assert!(
        of(Model::Charger)[0].charge > lang::Num::ZERO,
        "no power ever reached the charger"
    );
    // And what the final state cannot show: bots fell to low power, ran
    // their hook and recharged, before the redeploy.
    let (hooks, recharges) = power_seen(&replay);
    assert!(hooks > 0, "no bot ever ran its low-power hook");
    assert!(recharges > 0, "no bot ever recharged");
    // The ground and the water (Q65, Q66). The hollow and the channel
    // were dug a level down and their plans are done with; the digger
    // holds the earth of both; and the lake stands in them.
    let wet = w.data.world.water.wet;
    let ground = w.data.world.height.default;
    for (name, at) in [("hollow", HOLLOW), ("channel", CHANNEL)] {
        let t = w.tile(at).expect("on the map");
        assert_eq!(
            t.height.level(),
            ground.saturating_sub(1),
            "the {name} was never dug"
        );
        assert!(w.plans(player, at).is_none(), "the {name} is still planned");
        assert!(t.water >= wet, "the lake never reached the {name}");
    }
    let two = lang::Num::from_int(2).expect("two");
    assert!(
        w.machines
            .values()
            .any(|m| m.team == player && m.load.get("earth") == Some(&two)),
        "no bot holds the earth of the two digs"
    );
    let seen = water_seen(&replay);
    // The spring and the sink ran. The lake's water changed while it
    // found its level, at the start and again when the channel opened,
    // and the ground water with it; it settles to a flow that leaves
    // every tile as it was, so what shows a live spring and a live sink
    // at the end is the slope between their tiles, which still water
    // would not hold; and not a part was made or lost.
    assert!(
        seen.lake_moved >= 20,
        "the lake moved on {} ticks",
        seen.lake_moved
    );
    let held = |x, y| w.tile(TilePos::new(x, y)).expect("lake").water;
    assert!(
        held(-10, -6) > held(-9, -6).saturating_add(w.data.world.water.film),
        "no slope from the spring's tile, {}, to the sink's, {}",
        held(-10, -6),
        held(-9, -6)
    );
    assert!(
        seen.ground.0 < seen.ground.1,
        "the ground water never changed"
    );
    assert!(seen.total_held, "the water's total changed");
    // The pylon stood dry in the hollow, then flooded and dark, and died
    // of the flood: its last tick was a flooded one, at the last of its
    // health, and it is gone.
    assert!(
        seen.pylon > seen.flooded && seen.flooded >= 40,
        "the pylon stood {} ticks, {} of them flooded",
        seen.pylon,
        seen.flooded
    );
    let (health, flooded) = seen.last.expect("no pylon ever stood in the hollow");
    let flood = w.data.world.water.flood_damage;
    assert!(
        flooded && health <= flood,
        "the pylon was last seen at {} health",
        health.to_decimal()
    );
    assert!(
        !w.machines.values().any(|m| m.pos == HOLLOW),
        "the flooded pylon is still standing"
    );
    // The water network (Q67; `docs/09`). Under the waterworks program
    // the bots built a tank, a pump and a spout, one of each and where
    // they were planned; laid the wire that carries the base's grid to
    // them — the loop above has found each on the one grid and powered —
    // and the tile of pipe between the tank and the spout; and every
    // plan of it is done with.
    for (model, at) in [
        (Model::Tank, WATER_TANK),
        (Model::Pump, WATER_PUMP),
        (Model::Spout, WATER_SPOUT),
    ] {
        let built = of(model);
        assert_eq!(
            built.len(),
            1,
            "the planned {} was never built",
            model.name()
        );
        assert_eq!(built[0].pos, at);
        assert!(built[0].watered, "{} is dry", built[0].name);
        assert!(w.plans(player, at).is_none());
    }
    for at in WATER_WIRE {
        assert!(w.has_wire(player, at), "no wire at {at:?}");
        assert!(w.plans(player, at).is_none());
    }
    assert!(w.has_wire(player, WATER_LINE) && w.has_pipe(player, WATER_LINE));
    assert!(w.plans(player, WATER_LINE).is_none(), "the line's plans");
    // The three are one water network, labelled by its oldest building,
    // and the pipe is what makes them one: the spout touches neither of
    // the others. Wire joins no network, so no printer is on it.
    let (tank, pump, spout) = (of(Model::Tank)[0], of(Model::Pump)[0], of(Model::Spout)[0]);
    let label = [tank.id, pump.id, spout.id].into_iter().min();
    for m in [tank, pump, spout] {
        assert_eq!(m.network, label, "{} is on another network", m.name);
    }
    assert!(!spout.pos.is_adjacent(tank.pos) && !spout.pos.is_adjacent(pump.pos));
    assert!(
        of(Model::Printer).iter().all(|m| m.network != label),
        "a printer is on the waterworks' network"
    );
    // The pump has the flooded hollow for a source and still lifts, into
    // the tank; nothing on the network needs any.
    let figures = w.teams[&player]
        .networks
        .iter()
        .find(|n| Some(n.network) == label)
        .expect("the waterworks' figures");
    let rows = (w.data.model("pump"), w.data.model("tank"));
    assert_eq!(
        (figures.supply, figures.drain, figures.capacity),
        (u64::from(rows.0.pump_rate), 0, u64::from(rows.1.tank)),
        "the network's supply, drain and capacity"
    );
    assert_eq!(figures.stored, u64::from(tank.water));
    assert!(tank.water > 0, "the pump never filled the tank");
    // The hollow east of the spout was dug a level down, and the spout
    // filled it, to the four fifths of a level its program stops at, and
    // turned itself off. The ground beside the hollow is dry, so nothing
    // but the spout put water in it: it rose on ticks the spout was
    // turned east and on no other, and the water came from the flooded
    // hollow by the lake, four tiles west, none of it across the ground
    // between.
    let hollow = tile_at(w, WATER_HOLLOW);
    assert_eq!(
        hollow.height.level(),
        ground.saturating_sub(1),
        "the spout's hollow was never dug"
    );
    let full = w
        .data
        .world
        .water
        .parts
        .saturating_mul(4)
        .checked_div(5)
        .unwrap_or(0);
    assert!(
        hollow.water >= full,
        "the spout left the hollow at {} parts of {full}",
        hollow.water
    );
    assert_eq!(spout.pouring, None, "the spout is still pouring");
    for beside in WATER_HOLLOW.neighbours() {
        assert!(
            tile_at(w, beside).water < wet,
            "water stands at {beside:?}, beside the hollow"
        );
    }
    assert!(
        seen.poured >= 10 && seen.pouring >= seen.poured && !seen.rose_unpoured,
        "the hollow rose on {} ticks, the spout was turned east on {}, and it rose on another: {}",
        seen.poured,
        seen.pouring,
        seen.rose_unpoured
    );
    assert!(seen.stored >= tank.water);
    // Moving (Q65; `docs/03`). The pacing bot dug a tile a level down
    // and filled the one beside it a level up with that earth, and mined
    // a rock, which is ground now at the height it had.
    let start = Map::parse(&replay.map, &w.data).expect("map");
    let tile = |at: TilePos| w.tile(at).expect("on the map");
    assert_eq!(
        (tile(DUG).height.level(), tile(FILLED).height.level()),
        (ground.saturating_sub(1), ground.saturating_add(1)),
        "the tile was never dug, or the one beside it never filled"
    );
    assert_eq!(
        start.tile(MINED).map(|t| t.terrain),
        Some(sim::map::Terrain::Rock),
        "the scenario mines what was never rock"
    );
    assert_eq!(
        (tile(MINED).terrain, tile(MINED).height),
        (
            sim::map::Terrain::Ground,
            start.tile(MINED).expect("on the map").height
        ),
        "the rock was never mined"
    );
    // And what the final state cannot show. One bot fell, the two levels
    // from the filled tile to the dug one, and lost a level's damage for
    // it in that tick; a bot reached full pace; and before the redeploy
    // one bonk put a bot onward, one that was picking, and ended its pick
    // with nothing picked. The redeploy's guards all walk west, and bonk
    // one another: every bot a bonk put onward had its action ended, and
    // no bot was put onward by a bonk nobody heard begin.
    let moved = moving_seen(&replay);
    let bot = w.data.model("bot");
    assert_eq!(
        moved.falls,
        [(2, bot.fall_damage)],
        "the falls, as (levels, health lost)"
    );
    assert_eq!(moved.pace, bot.pace_max, "no bot ever reached full pace");
    let first: Vec<_> = moved
        .shoved
        .iter()
        .filter(|(tick, ..)| *tick < REDEPLOY)
        .map(|(_, busy, undone)| (*busy, *undone))
        .collect();
    assert_eq!(
        first,
        [(Some("pick"), true)],
        "the bots a bonk put onward before the redeploy, as (what each was doing, whether it ended undone) {:?}",
        moved.shoved
    );
    assert!(
        moved.shoved.iter().all(|(.., undone)| *undone),
        "a bonk put a bot onward and left its action running: {:?}",
        moved.shoved
    );
    assert!(
        moved.bonks as usize >= moved.shoved.len(),
        "{} bonks were heard and {} bots put onward",
        moved.bonks,
        moved.shoved.len()
    );
    let remembered = w.teams[&TeamId(0)]
        .memory
        .iter()
        .filter(|m| m.state != sim::world::TileState::Unknown)
        .count();
    assert!(
        remembered > 20,
        "the player's memory holds {remembered} tiles"
    );
}

#[test]
fn golden_replay_round_trips_through_ron() {
    let replay = golden_replay();
    let parsed = Replay::from_ron(&replay.to_ron()).expect("fixture RON parses");
    assert_eq!(replay, parsed);
    assert_eq!(replay.identity(), parsed.identity());
}

/// Child half of the cross-process check; the parent spawns it.
#[test]
#[ignore = "spawned by cross_process_replay_matches"]
fn emit_golden_final_hash() {
    let hashes = golden_replay().run();
    println!(
        "GOLDEN_FINAL_HASH={:016x}",
        hashes.last().expect("nonempty run")
    );
}

/// The lockstep guarantee: a SEPARATE PROCESS reaches the same state.
#[test]
fn cross_process_replay_matches() {
    let exe = std::env::current_exe().expect("test binary path");
    let output = std::process::Command::new(exe)
        .args([
            "--ignored",
            "--exact",
            "emit_golden_final_hash",
            "--nocapture",
        ])
        .env_remove("UPDATE_GOLDEN")
        .output()
        .expect("spawn child test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child process failed: {stdout}");
    let child = stdout
        .lines()
        .find_map(|l| l.strip_prefix("GOLDEN_FINAL_HASH="))
        .unwrap_or_else(|| panic!("no hash line in child output: {stdout}"))
        .to_string();
    let local = format!(
        "{:016x}",
        golden_replay().run().last().expect("nonempty run")
    );
    assert_eq!(child, local, "cross-process desync — determinism violation");
}
