//! Flooding (`docs/09`, Wading and flooding; Q65): `flooded` set by the
//! water step, a flooded bot that cannot move, a flooded building dark
//! through the power step, and the health a flood takes. T45's second
//! part.

use lang::Num;
use sim::world::{EntityId, Model};
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos, bundle_of};

/// 9 by 5, origin at the centre: x in -4..=4, y in -2..=2, on ground at 4.
/// A pit two deep at (-3, 1) with a ledge one deep east of it, at (-2, 1),
/// and a hollow one deep at (1, 0), beside team 0's printer at (0, 0).
/// Team 1 starts at (4, -2).
fn map_text(heights: &str) -> String {
    format!(
        r#"
name = "flood"
speed = 10
width = 9
height = 5
ground_water = 20
terrain = """
.........
.........
.........
.o.......
.......o.
"""
{heights}
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [0, 0]
[[team]]
start = [4, -2]
"#
    )
}

const HEIGHTS: &str = r#"heights = """
444444444
423444444
444443444
444444444
444444444
""""#;

/// The same ground with team 0's printer in a hollow of its own.
const SUNK_PRINTER: &str = r#"heights = """
444444444
423444444
444434444
444444444
444444444
""""#;

const PIT: (i32, i32) = (-3, 1);
const LEDGE: (i32, i32) = (-2, 1);
const HOLLOW: (i32, i32) = (1, 0);

fn at((x, y): (i32, i32)) -> TilePos {
    TilePos::new(x, y)
}

/// A match on `heights`, with `flood_damage` the tables' or the one given.
fn sim(heights: &str, flood_damage: Option<Num>) -> Sim {
    let mut data = Data::load().unwrap();
    if let Some(d) = flood_damage {
        data.world.water.flood_damage = d;
    }
    let map = Map::parse(&map_text(heights), &data).unwrap();
    Sim::with_data(map, data)
}

/// Put `parts` on a tile by fiat, out of the ground water so the total
/// stands, and say the tile is disturbed.
fn pour(s: &mut Sim, tile: (i32, i32), parts: u32) {
    let w = &mut s.world;
    let idx = w.tile_index(at(tile)).unwrap();
    w.ground_water = w
        .ground_water
        .saturating_add(u64::from(w.tiles[idx].water))
        .saturating_sub(u64::from(parts));
    w.tiles[idx].water = parts;
    w.stir(idx);
}

/// Make a tile wet: the least water that counts.
fn soak(s: &mut Sim, tile: (i32, i32)) {
    let wet = s.world.data.world.water.wet;
    pour(s, tile, wet);
}

fn step(s: &mut Sim, ticks: u64) {
    for _ in 0..ticks {
        s.step(&[], &[]);
    }
}

/// Deploy `main` to team 0's bots on the next tick and run `ticks`: what
/// the bot `id` logged.
fn run(s: &mut Sim, id: EntityId, main: &str, ticks: u64) -> Vec<String> {
    let mut logs = Vec::new();
    for t in 0..ticks {
        let mut cmds = Vec::new();
        if t == 0 {
            let mut c = Command::new(
                0,
                TeamId(0),
                0,
                CommandKind::Deploy {
                    deployment: "bot".into(),
                    bundle: bundle_of(main),
                },
            );
            c.tick = s.world.tick.saturating_add(1);
            cmds.push(c);
        }
        s.step(&cmds, &[]);
        if let Some(m) = s.world.machines.get(&id) {
            logs.extend(m.log.iter().map(|l| l.text.clone()));
        }
    }
    logs
}

fn bot_at(s: &mut Sim, team: u32, tile: (i32, i32)) -> EntityId {
    s.world
        .place(Model::Bot, TeamId(team), at(tile), Some("bot".into()))
}

fn printer_of(s: &Sim, team: u32) -> EntityId {
    s.world
        .machines
        .values()
        .find(|m| m.team == TeamId(team) && m.model == Model::Printer)
        .map(|m| m.id)
        .unwrap()
}

/// The water that floods the pit and leaves its ledge wadeable: a level
/// and a quarter in the pit, a quarter on the ledge, one surface.
fn flood_the_pit(s: &mut Sim) {
    let parts = s.world.data.world.water.parts;
    let quarter = parts.checked_div(4).unwrap();
    pour(s, PIT, parts.saturating_add(quarter));
    pour(s, LEDGE, quarter);
}

const IDLE: &str = "while True:\n    wait(1)\n";

#[test]
fn the_water_step_sets_flooded_on_a_bot_in_more_than_it_wades() {
    let mut s = sim(HEIGHTS, None);
    let wade = s.world.data.world.water.wade;
    let deep = bot_at(&mut s, 0, PIT);
    let wading = bot_at(&mut s, 0, LEDGE);
    let dry = bot_at(&mut s, 0, (3, 2));
    step(&mut s, 1);
    let flooded = |s: &Sim, id| s.world.machines[&id].flooded;
    assert!(!flooded(&s, deep) && !flooded(&s, wading) && !flooded(&s, dry));

    // Exactly what a bot wades is not a flood; a part more is.
    pour(&mut s, PIT, wade);
    assert!(!flooded(&s, deep), "set by the water step and nothing else");
    step(&mut s, 1);
    assert!(!flooded(&s, deep), "a bot wades a level");
    flood_the_pit(&mut s);
    step(&mut s, 1);
    assert!(flooded(&s, deep));
    // By its own tile's water alone: the bot wading beside it is not.
    assert!(!flooded(&s, wading) && !flooded(&s, dry));

    // And it stands only until the next water step finds otherwise.
    pour(&mut s, PIT, 0);
    pour(&mut s, LEDGE, 0);
    assert!(flooded(&s, deep));
    step(&mut s, 1);
    assert!(!flooded(&s, deep));
}

#[test]
fn a_flooded_bot_cannot_move_and_runs_everything_else() {
    let mut s = sim(HEIGHTS, None);
    let bot = bot_at(&mut s, 0, PIT);
    flood_the_pit(&mut s);
    step(&mut s, 1);
    // The ledge is a step from the pit and its water is waded: only the
    // flood stops the move.
    assert!(s.world.steps(at(PIT), at(LEDGE)));
    let main = "\
try:
    move('e')
    log('moved')
except ValueError:
    log('move refused')
try:
    move_to(4, 1)
    log('moved')
except ValueError:
    log('move_to refused')
log(me().flooded, tile(-3, 1).water > 1)
dig(-3, 2)
log('dug')
while True:
    wait(1)
";
    let logs = run(&mut s, bot, main, 8);
    assert_eq!(
        logs,
        ["move refused", "move_to refused", "True True", "dug"]
    );
    assert_eq!(s.world.machines[&bot].pos, at(PIT));

    // The water gone, the next water step frees it.
    pour(&mut s, PIT, 0);
    pour(&mut s, LEDGE, 0);
    step(&mut s, 1);
    let logs = run(
        &mut s,
        bot,
        "move('e')\nlog(me().flooded)\nwhile True:\n    wait(1)\n",
        5,
    );
    assert_eq!(logs, ["False"]);
    assert_eq!(s.world.machines[&bot].pos, at(LEDGE));
}

#[test]
fn a_move_begun_before_the_flood_completes() {
    let mut s = sim(HEIGHTS, None);
    let bot = bot_at(&mut s, 0, PIT);
    step(&mut s, 1);
    // The water arrives in the tick the move begins: the slice runs
    // first, the water step after it.
    flood_the_pit(&mut s);
    let main = "move('e')\nlog('there')\nwhile True:\n    wait(1)\n";
    let mut logs = run(&mut s, bot, main, 1);
    let m = &s.world.machines[&bot];
    assert!(m.flooded && m.action.is_some() && m.pos == at(PIT));
    logs.extend(run(&mut s, bot, IDLE, 0));
    for _ in 0..4 {
        s.step(&[], &[]);
        logs.extend(s.world.machines[&bot].log.iter().map(|l| l.text.clone()));
    }
    assert_eq!(logs, ["there"]);
    let m = &s.world.machines[&bot];
    assert!(m.pos == at(LEDGE) && !m.flooded);
}

#[test]
fn a_flooded_building_is_dark_from_the_water_step_and_through_the_power_step() {
    let mut s = sim(HEIGHTS, None);
    let wet = s.world.data.world.water.wet;
    let printer = printer_of(&s, 0);
    // Two chargers on the printer's grid, one of them in the hollow.
    let low = s.world.place(Model::Charger, TeamId(0), at(HOLLOW), None);
    let high = s.world.place(Model::Charger, TeamId(0), at((-1, 0)), None);
    step(&mut s, 2);
    let state = |s: &Sim, id| {
        let m = &s.world.machines[&id];
        (m.flooded, m.powered, m.grid, m.charge)
    };
    let (_, _, _, before) = state(&s, low);
    assert_eq!(state(&s, low), (false, true, Some(printer), before));
    assert!(before > Num::ZERO, "the grid's leftover fills it");

    // Damp is not wet, and not a flood.
    pour(&mut s, HOLLOW, wet.saturating_sub(1));
    step(&mut s, 1);
    assert!(!s.world.machines[&low].flooded);

    // Wet: the power step of the tick filled it once more, and then the
    // water step flooded it, and it is dark at once.
    pour(&mut s, HOLLOW, wet);
    step(&mut s, 1);
    let (flooded, powered, grid, held) = state(&s, low);
    assert!(flooded && !powered && held > before);
    assert_eq!(grid, Some(printer), "it stays on its grid");

    // Through the power steps that follow it is dark, and nothing enters
    // its battery, its own generation included; the one beside it fills.
    let (_, _, _, beside) = state(&s, high);
    step(&mut s, 5);
    assert_eq!(state(&s, low), (true, false, Some(printer), held));
    assert!(state(&s, high).3 > beside);
    assert!(state(&s, high).1);

    // The water gone: the water step clears `flooded`, and the next
    // power step, a tick on, powers it.
    pour(&mut s, HOLLOW, 0);
    step(&mut s, 1);
    assert_eq!(state(&s, low), (false, false, Some(printer), held));
    step(&mut s, 1);
    let (flooded, powered, _, after) = state(&s, low);
    assert!(!flooded && powered && after > held);
}

#[test]
fn a_flooded_battery_gives_nothing_to_its_grid() {
    // A depot's grid with nothing generating on it: a charger holding
    // charge beside a printer whose own generation is taken away.
    let run = |flood: bool| {
        let mut data = Data::load().unwrap();
        let printer = data.machines.models.get_mut("printer").unwrap();
        (printer.generation, printer.drain) = (Num::ZERO, Num::ONE);
        let charger = data.machines.models.get_mut("charger").unwrap();
        charger.generation = Num::ZERO;
        let map = Map::parse(&map_text(HEIGHTS), &data).unwrap();
        let mut s = Sim::with_data(map, data);
        let id = printer_of(&s, 0);
        s.world.machines.get_mut(&id).unwrap().charge = Num::ZERO;
        let charger = s.world.place(Model::Charger, TeamId(0), at(HOLLOW), None);
        s.world.machines.get_mut(&charger).unwrap().charge = Num::from_int(50).unwrap();
        if flood {
            soak(&mut s, HOLLOW);
        }
        // The first tick's power step runs before any flood, and its
        // water step floods the charger; the three after it find it so.
        step(&mut s, 1);
        let held = s.world.machines[&charger].charge;
        step(&mut s, 3);
        let (p, c) = (&s.world.machines[&id], &s.world.machines[&charger]);
        (p.powered, held, c.charge)
    };
    let (powered, held, left) = run(false);
    assert!(powered && left < held, "dry, the charger pays the printer");
    let (powered, held, left) = run(true);
    assert!(!powered && left == held, "flooded, it gives nothing");
}

#[test]
fn a_flooded_machine_loses_health_every_tick() {
    let mut s = sim(HEIGHTS, Some(Num::ONE));
    let bot = bot_at(&mut s, 0, PIT);
    let wading = bot_at(&mut s, 0, LEDGE);
    let solar = s.world.place(Model::Solar, TeamId(0), at(HOLLOW), None);
    run(&mut s, bot, IDLE, 1);
    let health = |s: &Sim, id| s.world.machines[&id].health;
    let (bot_full, solar_full) = (health(&s, bot), health(&s, solar));
    flood_the_pit(&mut s);
    soak(&mut s, HOLLOW);
    // The tick that floods them takes nothing: its damage step ran
    // before its water step.
    step(&mut s, 1);
    assert_eq!((health(&s, bot), health(&s, solar)), (bot_full, solar_full));
    let less = |from: Num, n: i128| from.checked_sub(Num::from_int(n).unwrap()).unwrap();
    for n in 1..=4 {
        step(&mut s, 1);
        assert_eq!(health(&s, bot), less(bot_full, n));
        assert_eq!(health(&s, solar), less(solar_full, n));
    }
    assert_eq!(health(&s, wading), bot_full, "wading costs nothing");

    // Out of the water, the tick after the water step that says so is
    // the first to take nothing.
    pour(&mut s, HOLLOW, 0);
    step(&mut s, 1);
    assert_eq!(health(&s, solar), less(solar_full, 5));
    step(&mut s, 3);
    assert_eq!(health(&s, solar), less(solar_full, 5));

    // Left in it, the bot dies of it.
    step(&mut s, 30);
    assert!(!s.world.machines.contains_key(&bot));
    assert!(s.world.machines.contains_key(&wading));
}

#[test]
fn a_flooded_printer_prints_nothing_does_not_defend_and_dies() {
    // Dry, the printer has its first bot out by tick 31.
    let mut dry = sim(SUNK_PRINTER, Some(Num::from_int(5).unwrap()));
    step(&mut dry, 31);
    let bots = |s: &Sim| {
        s.world
            .machines
            .values()
            .filter(|m| m.team == TeamId(0) && m.model == Model::Bot)
            .count()
    };
    assert_eq!(bots(&dry), 1);

    let mut s = sim(SUNK_PRINTER, Some(Num::from_int(5).unwrap()));
    let printer = printer_of(&s, 0);
    soak(&mut s, (0, 0));
    // The first tick's power step began a print before its water step
    // flooded the printer.
    step(&mut s, 1);
    let p = &s.world.machines[&printer];
    let printing = p.printing;
    assert!(p.flooded && !p.powered && printing > 0);
    // An enemy bot beside it, on dry ground.
    let enemy = bot_at(&mut s, 1, (0, -1));
    let full = s.world.machines[&enemy].health;
    step(&mut s, 5);
    let p = &s.world.machines[&printer];
    assert_eq!(p.printing, printing, "a print in progress holds");
    assert_eq!(s.world.machines[&enemy].health, full, "it does not defend");

    // It dies of the flood without a bot printed, and its team is out.
    for _ in 0..60 {
        step(&mut s, 1);
        assert_eq!(bots(&s), 0);
    }
    assert!(!s.world.machines.contains_key(&printer));
    assert!(s.world.teams[&TeamId(0)].out);
}

#[test]
fn flooded_is_in_the_hash() {
    // Two matches alike in everything but one machine's `flooded`.
    let hash = |flooded: bool| {
        let mut s = sim(HEIGHTS, None);
        let bot = bot_at(&mut s, 0, PIT);
        step(&mut s, 1);
        s.world.machines.get_mut(&bot).unwrap().flooded = flooded;
        s.state_hash()
    };
    assert_ne!(hash(false), hash(true));
    assert_eq!(hash(true), hash(true));
}
