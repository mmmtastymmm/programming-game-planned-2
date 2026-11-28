//! The water network (`docs/09`, The water network; Q67), one rule a
//! test, on a small map: networks and what joins them, the network step
//! — sources, needs, spouts, tanks — a dry building, pipe, what a program
//! reads, and the water of a match never changing in sum. The golden
//! replay pins these as a whole; here a change names the rule it broke.
//!
//! The arithmetic lint is relaxed here: this file counts parts of water in
//! plain sums, and a sum that overflowed would be a failed test, not a
//! desync, since nothing here reaches the sim's state but through its API.
#![allow(clippy::arithmetic_side_effects)]

use lang::Num;
use sim::map::{Dir, Height};
use sim::world::{EntityId, Model, NetworkFigures, Plan, TileState};
use sim::{Command, CommandKind, Data, Map, PlanKind, Sim, TeamId, TilePos, bundle_of};

/// 15 by 9, origin at the centre: x in -7..=7, y in -4..=4, the first row
/// the north edge, all ground at height 4 but a rock at (0, 3). The teams
/// start in the south corners with a deposit beside each, and the map
/// begins with a hundred levels of ground water and none standing: a test
/// digs its own pits and fills them from the ground water.
const MAP: &str = r#"
name = "network"
speed = 10
width = 15
height = 9
ground_water = 100
terrain = """
...............
.......#.......
...............
...............
...............
...............
...............
...............
.o...........o.
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-7, -4]
[[team]]
start = [7, -4]
"#;

const T0: TeamId = TeamId(0);
const T1: TeamId = TeamId(1);

fn sim() -> Sim {
    sim_with(|_| {})
}

/// A match on tables edited from the shipped ones. An amount of water on
/// a model is in parts, a thousand to a level as shipped.
fn sim_with(edit: impl FnOnce(&mut Data)) -> Sim {
    let mut data = Data::load().unwrap();
    edit(&mut data);
    let map = Map::parse(MAP, &data).unwrap();
    Sim::with_data(map, data)
}

/// The row of `model` in the tables, to edit.
fn row<'a>(data: &'a mut Data, model: &str) -> &'a mut sim::data::ModelData {
    data.machines.models.get_mut(model).unwrap()
}

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y)
}

fn num(text: &str) -> Num {
    Num::parse_literal(text).unwrap()
}

fn place(s: &mut Sim, model: Model, team: TeamId, p: TilePos) -> EntityId {
    s.world
        .place(model, team, p, model.deployment().map(str::to_string))
}

fn machine(s: &Sim, id: EntityId) -> &sim::world::Machine {
    &s.world.machines[&id]
}

fn printer(s: &Sim, team: TeamId) -> EntityId {
    s.world
        .machines
        .values()
        .find(|m| m.team == team && m.model == Model::Printer)
        .unwrap()
        .id
}

/// Stand the ground of a tile at `level`, by fiat.
fn level(s: &mut Sim, p: TilePos, level: u8) {
    let idx = s.world.tile_index(p).unwrap();
    let max = s.world.data.world.height.max;
    s.world.set_height(idx, Height::new(level, max).unwrap());
}

/// Put `parts` on a tile by fiat, out of the ground water so the total
/// stands, and say the tile is disturbed.
fn put(s: &mut Sim, p: TilePos, parts: u32) {
    let w = &mut s.world;
    let idx = w.tile_index(p).unwrap();
    w.ground_water = w.ground_water + u64::from(w.tiles[idx].water) - u64::from(parts);
    w.tiles[idx].water = parts;
    w.stir(idx);
}

/// A pit a level deep holding `parts`: water that stays where it is put.
fn pit(s: &mut Sim, p: TilePos, parts: u32) {
    level(s, p, 3);
    put(s, p, parts);
}

/// Put `parts` in a building's tank by fiat, out of the ground water.
fn fill(s: &mut Sim, id: EntityId, parts: u32) {
    let w = &mut s.world;
    let m = w.machines.get_mut(&id).unwrap();
    w.ground_water = w.ground_water + u64::from(m.water) - u64::from(parts);
    m.water = parts;
}

fn water(s: &Sim, p: TilePos) -> u32 {
    s.world.tile(p).unwrap().water
}

fn held(s: &Sim, id: EntityId) -> u32 {
    machine(s, id).water
}

/// The tiles, the ground water and the tanks, each summed by hand.
fn parts_of(s: &Sim) -> (u64, u64, u64) {
    let tiles = s.world.tiles.iter().map(|t| u64::from(t.water)).sum();
    let tanks = s.world.machines.values().map(|m| u64::from(m.water)).sum();
    (tiles, s.world.ground_water, tanks)
}

fn total(s: &Sim) -> u64 {
    let (tiles, ground, tanks) = parts_of(s);
    tiles + ground + tanks
}

fn network_of(s: &Sim, id: EntityId) -> Option<EntityId> {
    machine(s, id).network
}

fn grid_of(s: &Sim, id: EntityId) -> Option<EntityId> {
    machine(s, id).grid
}

/// The team's figures for the network labelled `label`.
fn figures(s: &Sim, team: TeamId, label: EntityId) -> NetworkFigures {
    s.world.teams[&team]
        .networks
        .iter()
        .find(|n| n.network == label)
        .cloned()
        .unwrap_or_else(|| panic!("no network {label:?}"))
}

fn deploy(team: u32, deployment: &str, main: &str) -> Command {
    Command::new(
        0,
        TeamId(team),
        0,
        CommandKind::Deploy {
            deployment: deployment.into(),
            bundle: bundle_of(main),
        },
    )
}

/// Step `n` ticks, the first with `opening`, each dated for that tick;
/// every machine's log lines of those ticks, in order, as `(id, text)`.
fn run(s: &mut Sim, opening: Vec<Command>, n: u64) -> Vec<(EntityId, String)> {
    let mut cmds = opening;
    for (i, c) in cmds.iter_mut().enumerate() {
        c.seq = i as u32;
        c.tick = s.world.tick + 1;
    }
    let mut lines = Vec::new();
    for tick in 0..n {
        let report = s.step(if tick == 0 { &cmds } else { &[] }, &[]);
        assert!(report.dropped.is_empty(), "dropped {:?}", report.dropped);
        for m in s.world.machines.values() {
            lines.extend(m.log.iter().map(|l| (m.id, l.text.clone())));
        }
    }
    lines
}

fn lines_of(lines: &[(EntityId, String)], id: EntityId) -> Vec<&str> {
    lines
        .iter()
        .filter(|(who, _)| *who == id)
        .map(|(_, text)| text.as_str())
        .collect()
}

/// A bot of `team` on `p` with a full battery and `ore` in its load.
fn bot(s: &mut Sim, team: TeamId, p: TilePos, ore: &str) -> EntityId {
    let id = place(s, Model::Bot, team, p);
    s.world
        .machines
        .get_mut(&id)
        .unwrap()
        .load
        .insert("ore".into(), num(ore));
    id
}

fn ore(s: &Sim, id: EntityId) -> Num {
    machine(s, id).load["ore"]
}

/// A pump on `p` with a panel on `panel`, beside it, to power it: a pump
/// makes no power and drains some (`docs/09`, Pumps, tanks and spouts).
fn pump(s: &mut Sim, p: TilePos, panel: TilePos) -> EntityId {
    assert!(p.is_adjacent(panel));
    let id = place(s, Model::Pump, T0, p);
    place(s, Model::Solar, T0, panel);
    id
}

// --------------------------------------------------------------- networks ---

#[test]
fn a_match_starts_with_its_printer_watered_empty_and_a_network_of_one() {
    let s = sim();
    let p = printer(&s, T0);
    let m = machine(&s, p);
    assert!(m.watered);
    assert_eq!(m.network, Some(p));
    assert_eq!(m.water, 0, "every tank starts empty, a map's printer's too");
    assert_eq!(
        s.world.teams[&T0].networks,
        [NetworkFigures {
            network: p,
            supply: 0,
            drain: 0,
            stored: 0,
            capacity: 2000,
        }]
    );
}

#[test]
fn buildings_join_a_network_by_touching_by_pipe_and_by_an_aqueducts_span() {
    let mut s = sim();
    let p = printer(&s, T0);
    // Two that touch, and a site beside them, which is on no network and
    // bridges nothing.
    let a = place(&mut s, Model::Tank, T0, at(-2, 2));
    let b = place(&mut s, Model::Tank, T0, at(-1, 2));
    let site = place(&mut s, Model::Site, T0, at(0, 2));
    // A tile off, past the site: its own network until pipe reaches it.
    let far = place(&mut s, Model::Tank, T0, at(1, 2));
    // Further still, with an aqueduct between it and `far`, within its
    // span of both and of nothing else.
    let aqueduct = place(&mut s, Model::Aqueduct, T0, at(3, 0));
    let beyond = place(&mut s, Model::Tank, T0, at(6, -1));
    run(&mut s, vec![], 1);
    assert_eq!(network_of(&s, a), Some(a));
    assert_eq!(network_of(&s, b), Some(a));
    assert_eq!(network_of(&s, far), Some(far));
    assert_eq!(
        network_of(&s, aqueduct),
        Some(far),
        "(3, 0) to (1, 2) is within 4"
    );
    assert_eq!(
        network_of(&s, beyond),
        Some(far),
        "(3, 0) to (6, -1) is within 4"
    );
    assert_eq!(network_of(&s, site), None);
    assert!(!machine(&s, site).watered, "a site is not watered");
    assert_eq!(network_of(&s, p), Some(p));
    assert_eq!(s.world.teams[&T0].networks.len(), 3);
    // Pipe under the site's tile, which it shares: one network, labelled
    // by its oldest building.
    assert!(s.world.lay_pipe(T0, at(0, 2)));
    run(&mut s, vec![], 1);
    for id in [a, b, far, aqueduct, beyond] {
        assert_eq!(network_of(&s, id), Some(a));
    }
    let snap = s.snapshot();
    let team = snap.teams.iter().find(|t| t.id == T0).unwrap();
    assert_eq!(team.networks.len(), 2);
    assert_eq!(team.pipes, [(at(0, 2), true)]);
    // A rival's pipe on the same tile joins nothing of this team's, and
    // pipe that reaches no building is part of no network.
    assert!(s.world.remove_pipe(T0, at(0, 2)));
    assert!(s.world.lay_pipe(T1, at(0, 2)));
    assert!(s.world.lay_pipe(T0, at(-4, -2)));
    run(&mut s, vec![], 1);
    assert_eq!(network_of(&s, far), Some(far));
    let snap = s.snapshot();
    let team = snap.teams.iter().find(|t| t.id == T0).unwrap();
    assert_eq!(team.pipes, [(at(-4, -2), false)]);
}

/// `docs/09`, Pipes and aqueducts: any team's bot may deconstruct an
/// aqueduct, and that is how a rival cuts a network.
#[test]
fn a_network_is_cut_by_a_deconstructed_aqueduct() {
    let mut s = sim();
    // Two tanks eight tiles apart and an aqueduct midway, within its
    // span of both: one network, labelled by the older tank.
    let west = place(&mut s, Model::Tank, T0, at(-4, 2));
    let east = place(&mut s, Model::Tank, T0, at(4, 2));
    let aqueduct = place(&mut s, Model::Aqueduct, T0, at(0, 2));
    run(&mut s, vec![], 1);
    assert_eq!(network_of(&s, east), Some(west));
    assert_eq!(network_of(&s, aqueduct), Some(west));
    assert!(machine(&s, aqueduct).program.is_none());
    // A rival's bot beside the aqueduct takes it down.
    let rival = bot(&mut s, T1, at(0, 1), "0");
    let cut = "try:\n    deconstruct(0, 2)\nexcept ValueError:\n    pass\nwait(1000)\n";
    run(&mut s, vec![deploy(1, "bot", cut)], 5);
    assert!(
        !s.world.machines.contains_key(&aqueduct),
        "{rival:?} cut the line"
    );
    assert_eq!(network_of(&s, west), Some(west));
    assert_eq!(network_of(&s, east), Some(east));
}

/// `docs/09`, Networks: this is `08`'s grid in every respect but what
/// joins. Wire and a pylon's span join a grid and no network; pipe and an
/// aqueduct's span a network and no grid.
#[test]
fn wire_and_a_pylon_join_no_network_and_pipe_and_an_aqueduct_join_no_grid() {
    let mut s = sim();
    // The north row: two tanks with a tile of wire between them, and two
    // with a pylon between them, in its span.
    let wired_a = place(&mut s, Model::Tank, T0, at(-4, 4));
    let wired_b = place(&mut s, Model::Tank, T0, at(-2, 4));
    assert!(s.world.lay_wire(T0, at(-3, 4)));
    let pylon_a = place(&mut s, Model::Tank, T0, at(1, 4));
    let pylon = place(&mut s, Model::Pylon, T0, at(3, 4));
    let pylon_b = place(&mut s, Model::Tank, T0, at(5, 4));
    // Five rows south, out of the pylon's span: the same with pipe, and
    // with an aqueduct.
    let piped_a = place(&mut s, Model::Tank, T0, at(-4, -1));
    let piped_b = place(&mut s, Model::Tank, T0, at(-2, -1));
    assert!(s.world.lay_pipe(T0, at(-3, -1)));
    let aqueduct_a = place(&mut s, Model::Tank, T0, at(1, -1));
    let aqueduct = place(&mut s, Model::Aqueduct, T0, at(3, -1));
    let aqueduct_b = place(&mut s, Model::Tank, T0, at(5, -1));
    run(&mut s, vec![], 1);
    // Wire: one grid, two networks.
    assert_eq!(grid_of(&s, wired_b), Some(wired_a));
    assert_eq!(network_of(&s, wired_a), Some(wired_a));
    assert_eq!(network_of(&s, wired_b), Some(wired_b));
    // A pylon: one grid, and three networks, the pylon's own tile one.
    assert_eq!(grid_of(&s, pylon), Some(pylon_a));
    assert_eq!(grid_of(&s, pylon_b), Some(pylon_a));
    assert_eq!(network_of(&s, pylon_a), Some(pylon_a));
    assert_eq!(network_of(&s, pylon), Some(pylon));
    assert_eq!(network_of(&s, pylon_b), Some(pylon_b));
    // Pipe: one network, two grids.
    assert_eq!(network_of(&s, piped_b), Some(piped_a));
    assert_eq!(grid_of(&s, piped_a), Some(piped_a));
    assert_eq!(grid_of(&s, piped_b), Some(piped_b));
    // An aqueduct: one network, three grids.
    assert_eq!(network_of(&s, aqueduct), Some(aqueduct_a));
    assert_eq!(network_of(&s, aqueduct_b), Some(aqueduct_a));
    assert_eq!(grid_of(&s, aqueduct_a), Some(aqueduct_a));
    assert_eq!(grid_of(&s, aqueduct), Some(aqueduct));
    assert_eq!(grid_of(&s, aqueduct_b), Some(aqueduct_b));
    // A wire and a pipe share a tile, and each still joins only its own.
    assert!(s.world.lay_pipe(T0, at(-3, 4)));
    assert!(s.world.lay_wire(T0, at(-3, -1)));
    run(&mut s, vec![], 1);
    assert_eq!(network_of(&s, wired_b), Some(wired_a));
    assert_eq!(grid_of(&s, piped_b), Some(piped_a));
}

// -------------------------------------------------------------- the needs ---

/// A charger that needs `parts` of water a tick, where the shipped one
/// needs none: the rules read the numbers, never the model's name.
fn thirsty(parts: u32) -> impl FnOnce(&mut Data) {
    move |data| row(data, "charger").water_drain = parts
}

#[test]
fn a_need_is_met_by_a_pump_or_by_a_tank_and_by_neither_the_building_is_dry() {
    let mut s = sim_with(thirsty(30));
    // By a pump: a pit of water, a pump beside it, and a charger.
    pit(&mut s, at(0, 0), 1000);
    let lifting = pump(&mut s, at(1, 0), at(2, 0));
    let by_pump = place(&mut s, Model::Charger, T0, at(1, 1));
    // By a tank that holds enough.
    let tank = place(&mut s, Model::Tank, T0, at(-3, 0));
    fill(&mut s, tank, 100);
    let by_tank = place(&mut s, Model::Charger, T0, at(-2, 0));
    // By neither: nothing at all, and a tank that holds too little.
    let alone = place(&mut s, Model::Charger, T0, at(-3, -2));
    let low = place(&mut s, Model::Tank, T0, at(4, 3));
    fill(&mut s, low, 20);
    let by_low = place(&mut s, Model::Charger, T0, at(5, 3));
    let (sum, ground) = (total(&s), s.world.ground_water);
    run(&mut s, vec![], 1);
    // The pump lifted the need and no more: nothing else took any.
    assert!(machine(&s, by_pump).watered);
    assert_eq!(water(&s, at(0, 0)), 970);
    assert_eq!(
        figures(&s, T0, lifting),
        NetworkFigures {
            network: lifting,
            supply: 50,
            drain: 30,
            stored: 0,
            capacity: 0,
        }
    );
    assert!(machine(&s, by_tank).watered);
    assert_eq!(held(&s, tank), 70);
    // A need not covered takes nothing.
    assert!(!machine(&s, alone).watered);
    assert!(!machine(&s, by_low).watered);
    assert_eq!(held(&s, low), 20, "it takes nothing");
    assert_eq!(figures(&s, T0, low).drain, 30, "every need, watered or dry");
    // What the two used went to the ground water.
    assert_eq!(s.world.ground_water, ground + 60);
    assert_eq!(total(&s), sum);
    // Being dry and being dark are apart: the dry chargers are powered.
    assert!(machine(&s, alone).powered && machine(&s, by_low).powered);
}

/// `docs/09`, The network step: water is drawn from the pumps, each as
/// far as it may lift, and what they cannot give from the tanks; and the
/// tanks are then filled by what the pumps may still lift.
#[test]
fn a_draw_takes_from_the_pumps_first_and_then_from_the_tanks() {
    let mut s = sim_with(|data| {
        thirsty(30)(data);
        row(data, "pump").pump_rate = 10;
    });
    pit(&mut s, at(0, 0), 1000);
    pump(&mut s, at(1, 0), at(2, 0));
    let tank = place(&mut s, Model::Tank, T0, at(1, 1));
    fill(&mut s, tank, 25);
    let charger = place(&mut s, Model::Charger, T0, at(1, -1));
    // The pump lifts 10 a tick. With 25 stored the need of 30 is covered,
    // by the pump's 10 and 20 of the tank's, and the pump has nothing
    // left to fill the tank with. With 5, and then 15, it is not: the
    // charger is dry and takes nothing, and the pump's 10 goes to the
    // tank. At 25 again it is covered again.
    let mut seen = Vec::new();
    for _ in 0..7 {
        run(&mut s, vec![], 1);
        seen.push((machine(&s, charger).watered, held(&s, tank)));
    }
    assert_eq!(
        seen,
        [
            (true, 5),
            (false, 15),
            (false, 25),
            (true, 5),
            (false, 15),
            (false, 25),
            (true, 5)
        ]
    );
    assert_eq!(water(&s, at(0, 0)), 930, "ten a tick, whoever took it");
}

/// `docs/09`, The network step: the needs are served oldest first, each
/// in full or not at all, and the next is tried when one does not fit.
#[test]
fn a_short_network_serves_the_oldest_first_and_tries_the_next_when_one_does_not_fit() {
    let mut s = sim_with(|data| {
        thirsty(30)(data);
        row(data, "depot").water_drain = 10;
    });
    let older = place(&mut s, Model::Charger, T0, at(0, 0));
    let newer = place(&mut s, Model::Charger, T0, at(1, 0));
    let depot = place(&mut s, Model::Depot, T0, at(2, 0));
    let tank = place(&mut s, Model::Tank, T0, at(3, 0));
    fill(&mut s, tank, 45);
    run(&mut s, vec![], 1);
    // 45 covers the older's 30 and leaves 15: not the newer's 30, but
    // the depot's 10.
    assert!(machine(&s, older).watered, "the oldest is served first");
    assert!(!machine(&s, newer).watered, "the next did not fit");
    assert!(
        machine(&s, depot).watered,
        "the one after was tried, and did"
    );
    assert_eq!(held(&s, tank), 5);
    assert_eq!(figures(&s, T0, older).drain, 70);
    // With 5 left nothing fits, and nothing is taken.
    run(&mut s, vec![], 1);
    for id in [older, newer, depot] {
        assert!(!machine(&s, id).watered);
    }
    assert_eq!(held(&s, tank), 5);
}

// -------------------------------------------------------------- the pumps ---

#[test]
fn a_pump_lifts_into_a_tank_from_the_wettest_tile_beside_it() {
    let mut s = sim();
    // Wet to the north and to the south, the south the deeper; and equal
    // water east and west of a second pump, which takes the east, the
    // first of `n`, `e`, `s`, `w`.
    pit(&mut s, at(-3, 1), 300);
    pit(&mut s, at(-3, -1), 400);
    let first = pump(&mut s, at(-3, 0), at(-4, 0));
    let first_tank = place(&mut s, Model::Tank, T0, at(-2, 0));
    pit(&mut s, at(4, 0), 300);
    pit(&mut s, at(2, 0), 300);
    let second = pump(&mut s, at(3, 0), at(3, 1));
    let second_tank = place(&mut s, Model::Tank, T0, at(3, -1));
    let sum = total(&s);
    run(&mut s, vec![], 1);
    assert_eq!((water(&s, at(-3, 1)), water(&s, at(-3, -1))), (300, 350));
    assert_eq!(held(&s, first_tank), 50, "its rate, a twentieth of a level");
    assert_eq!((water(&s, at(4, 0)), water(&s, at(2, 0))), (250, 300));
    assert_eq!(held(&s, second_tank), 50);
    for (pump, tank) in [(first, first_tank), (second, second_tank)] {
        let f = figures(&s, T0, pump);
        assert_eq!((f.supply, f.stored, f.capacity), (50, 50, 20000));
        assert_eq!(network_of(&s, tank), Some(pump));
    }
    assert_eq!(total(&s), sum);
    // A full tank takes no more, and a pump lifts nothing that nothing
    // takes.
    fill(&mut s, first_tank, 20000);
    run(&mut s, vec![], 3);
    assert_eq!(held(&s, first_tank), 20000);
    assert_eq!(water(&s, at(-3, -1)), 350);
}

#[test]
fn a_pump_with_no_wet_tile_one_unpowered_and_one_flooded_lift_nothing() {
    let mut s = sim();
    // Damp is not wet: a pit holding a part less than counts.
    pit(&mut s, at(-5, 2), 99);
    let thirsting = pump(&mut s, at(-4, 2), at(-4, 3));
    let no_water = place(&mut s, Model::Tank, T0, at(-3, 2));
    // A wet pit, and a pump with nothing to power it: it makes none and
    // drains some, so alone on its grid it is dark.
    pit(&mut s, at(0, 0), 1000);
    let unpowered = place(&mut s, Model::Pump, T0, at(1, 0));
    let no_power = place(&mut s, Model::Tank, T0, at(2, 0));
    // A powered pump beside a wet pit, standing in water itself.
    pit(&mut s, at(5, -2), 1000);
    pit(&mut s, at(4, -2), 300);
    let flooded = pump(&mut s, at(4, -2), at(4, -1));
    let behind = place(&mut s, Model::Tank, T0, at(3, -2));
    run(&mut s, vec![], 1);
    assert_eq!(held(&s, no_water), 0);
    assert_eq!(water(&s, at(-5, 2)), 99);
    assert_eq!(figures(&s, T0, thirsting).supply, 0, "it had no source");
    assert!(!machine(&s, unpowered).powered);
    assert_eq!(held(&s, no_power), 0);
    assert_eq!(water(&s, at(0, 0)), 1000);
    assert_eq!(figures(&s, T0, unpowered).supply, 0);
    // The third lifted in the match's first tick, before any water step
    // had found it flooded; that step did, and it lifts no more.
    assert!(machine(&s, flooded).flooded);
    let lifted = held(&s, behind);
    assert_eq!(lifted, 50);
    run(&mut s, vec![], 5);
    assert_eq!(held(&s, behind), lifted);
    assert_eq!(figures(&s, T0, flooded).supply, 0);
    // A flooded building takes no part: it is dry without being tried,
    // and still on its network.
    assert!(!machine(&s, flooded).watered);
    assert_eq!(network_of(&s, behind), Some(flooded));
    assert!(machine(&s, behind).watered);
}

/// `docs/09`, The network step: a pump never lifts more than its source
/// holds at the moment it lifts, so two pumps beside one tile share what
/// is on it, the older first.
#[test]
fn two_pumps_on_one_tile_share_what_it_holds() {
    let build = |need: Option<u32>| {
        let mut s = sim_with(|data| {
            row(data, "pump").pump_rate = 80;
            row(data, "charger").water_drain = need.unwrap_or(0);
        });
        pit(&mut s, at(0, 0), 120);
        let older = pump(&mut s, at(-1, 0), at(-2, 0));
        place(&mut s, Model::Pump, T0, at(0, 1));
        // Beside both pumps, and not beside the pit.
        let taker = match need {
            None => place(&mut s, Model::Tank, T0, at(-1, 1)),
            Some(_) => place(&mut s, Model::Charger, T0, at(-1, 1)),
        };
        (s, older, taker)
    };
    // A tank takes all there is: 80 by the older pump and the 40 that
    // leaves by the newer, not 80 again.
    let (mut s, older, tank) = build(None);
    let sum = total(&s);
    run(&mut s, vec![], 1);
    assert_eq!(held(&s, tank), 120);
    assert_eq!(water(&s, at(0, 0)), 0);
    assert_eq!(figures(&s, T0, older).supply, 160, "both had a source");
    assert_eq!(total(&s), sum);
    // What the two may lift is what the tile holds, 120, not their rates
    // together, 160: a need of 150 is not covered, and takes nothing.
    let (mut s, _, charger) = build(Some(150));
    run(&mut s, vec![], 1);
    assert!(!machine(&s, charger).watered);
    assert_eq!(water(&s, at(0, 0)), 120);
    // A need of 110 is: 80 and 30.
    let (mut s, _, charger) = build(Some(110));
    let ground = s.world.ground_water;
    run(&mut s, vec![], 1);
    assert!(machine(&s, charger).watered);
    assert_eq!(water(&s, at(0, 0)), 10);
    assert_eq!(s.world.ground_water, ground + 110);
}

// ------------------------------------------------------------- the spouts ---

/// A spout on (0, 0) with a pit on each side of it, and a tank holding
/// half a level joined to it by pipe under the east pit.
fn spout_among_pits(edit: impl FnOnce(&mut Data)) -> (Sim, EntityId, EntityId) {
    let mut s = sim_with(edit);
    let spout = place(&mut s, Model::Spout, T0, at(0, 0));
    for p in at(0, 0).neighbours() {
        level(&mut s, p, 3);
    }
    assert!(s.world.lay_pipe(T0, at(1, 0)));
    let tank = place(&mut s, Model::Tank, T0, at(2, 0));
    fill(&mut s, tank, 500);
    (s, spout, tank)
}

fn pouring(s: &mut Sim, spout: EntityId, dir: Option<Dir>) {
    s.world.machines.get_mut(&spout).unwrap().pouring = dir;
}

#[test]
fn a_spout_pours_onto_the_tile_it_faces_each_way() {
    for (i, dir) in Dir::ALL.into_iter().enumerate() {
        let (mut s, spout, tank) = spout_among_pits(|_| {});
        // As built it pours nowhere.
        assert_eq!(machine(&s, spout).pouring, None);
        run(&mut s, vec![], 2);
        assert_eq!(held(&s, tank), 500);
        pouring(&mut s, spout, Some(dir));
        let sum = total(&s);
        run(&mut s, vec![], 1);
        let around: Vec<u32> = at(0, 0)
            .neighbours()
            .into_iter()
            .map(|p| water(&s, p))
            .collect();
        let mut wanted = [0; 4];
        wanted[i] = 50;
        assert_eq!(around, wanted, "pouring {dir:?}: n, e, s, w");
        assert_eq!(held(&s, tank), 450, "its rate, drawn from the tank");
        assert_eq!(total(&s), sum);
        // As much as there is, when that is less than its rate.
        fill(&mut s, tank, 20);
        run(&mut s, vec![], 2);
        assert_eq!(held(&s, tank), 0);
        assert_eq!(water(&s, dir.of(at(0, 0))), 70);
    }
}

#[test]
fn a_spout_facing_rock_or_the_edge_of_the_map_pours_nowhere() {
    let mut s = sim();
    // On the north edge, with the map's rock south of it.
    let spout = place(&mut s, Model::Spout, T0, at(0, 4));
    let tank = place(&mut s, Model::Tank, T0, at(1, 4));
    fill(&mut s, tank, 500);
    let sum = total(&s);
    for dir in [Dir::N, Dir::S] {
        pouring(&mut s, spout, Some(dir));
        run(&mut s, vec![], 3);
        assert_eq!(held(&s, tank), 500, "pouring {dir:?} draws nothing");
    }
    assert_eq!(water(&s, at(0, 3)), 0, "rock holds no water");
    assert_eq!(total(&s), sum);
    // West is ground, and the water lands.
    pouring(&mut s, spout, Some(Dir::W));
    run(&mut s, vec![], 1);
    assert_eq!(held(&s, tank), 450);
    assert_eq!(total(&s), sum);
}

#[test]
fn a_dark_spout_and_a_dry_spout_pour_nothing() {
    // Dark: a spout that drains power it cannot have, alone on its grid.
    let (mut s, spout, tank) = spout_among_pits(|data| row(data, "spout").drain = num("5"));
    pouring(&mut s, spout, Some(Dir::N));
    run(&mut s, vec![], 3);
    assert!(!machine(&s, spout).powered && machine(&s, spout).watered);
    assert_eq!((held(&s, tank), water(&s, at(0, 1))), (500, 0));
    // Dry: one that needs more water than its network holds.
    let (mut s, spout, tank) = spout_among_pits(|data| row(data, "spout").water_drain = 600);
    pouring(&mut s, spout, Some(Dir::N));
    run(&mut s, vec![], 3);
    assert!(machine(&s, spout).powered && !machine(&s, spout).watered);
    assert_eq!((held(&s, tank), water(&s, at(0, 1))), (500, 0));
    // One whose need is covered pours what is left after it.
    let (mut s, spout, tank) = spout_among_pits(|data| row(data, "spout").water_drain = 470);
    pouring(&mut s, spout, Some(Dir::N));
    run(&mut s, vec![], 1);
    assert!(machine(&s, spout).watered);
    assert_eq!((held(&s, tank), water(&s, at(0, 1))), (0, 30));
}

/// `docs/09`, What a program reads: `pour(dir)` is a spout's alone and
/// takes no ticks.
#[test]
fn a_spouts_program_sets_the_way_it_pours_and_no_other_machine_may() {
    let (mut s, spout, tank) = spout_among_pits(|_| {});
    let b = bot(&mut s, T0, at(-4, 3), "0");
    let program = "log(me().pouring)\npour('e')\nlog(me().pouring)\nwait(2)\npour(None)\nlog(me().pouring)\nfor bad in ['x', 3, 'north']:\n    try:\n        pour(bad)\n    except ValueError:\n        log('refused', bad)\nwait(1000)\n";
    let other = "try:\n    pour('n')\nexcept ValueError:\n    log('not a spout')\nwait(1000)\n";
    let lines = run(
        &mut s,
        vec![
            deploy(0, "spout", program),
            deploy(0, "bot", other),
            deploy(0, "tank", other),
        ],
        6,
    );
    assert_eq!(
        lines_of(&lines, spout),
        [
            "None",
            "e",
            "None",
            "refused x",
            "refused 3",
            "refused north"
        ]
    );
    assert_eq!(lines_of(&lines, b), ["not a spout"]);
    assert_eq!(lines_of(&lines, tank), ["not a spout"]);
    // It poured in the two ticks its `wait` took, and `pour` none: the
    // turn to the east was in force by the first tick's network step.
    assert_eq!(water(&s, at(1, 0)), 100);
    assert_eq!(held(&s, tank), 400);
    assert_eq!(machine(&s, spout).pouring, None);
}

// ------------------------------------------------- moving a pond uphill ---

/// A pond of three tiles two levels deep, a pump on its east bank, pipe,
/// and a spout pouring into a hollow whose floor stands four levels above
/// the pond's. Returns the pump and the spout.
fn pond_and_hollow(s: &mut Sim) -> (EntityId, EntityId) {
    for x in -4..=-2 {
        level(s, at(x, 0), 2);
        put(s, at(x, 0), 500);
    }
    let lifting = pump(s, at(-1, 0), at(-1, 1));
    for x in 0..=2 {
        assert!(s.world.lay_pipe(T0, at(x, 0)));
    }
    let spout = place(s, Model::Spout, T0, at(3, 0));
    // The hollow at (4, 0), its floor at 6 and everything beside it at 9,
    // the spout's own tile included.
    level(s, at(4, 0), 6);
    for p in at(4, 0).neighbours() {
        level(s, p, 9);
    }
    pouring(s, spout, Some(Dir::E));
    (lifting, spout)
}

const POND: [(i32, i32); 3] = [(-4, 0), (-3, 0), (-2, 0)];
const HOLLOW: (i32, i32) = (4, 0);

#[test]
fn a_pond_is_pumped_dry_and_a_hollow_filled_from_it_uphill() {
    let mut s = sim();
    let (lifting, spout) = pond_and_hollow(&mut s);
    let sum = total(&s);
    let ground = s.world.ground_water;
    run(&mut s, vec![], 1);
    assert_eq!(network_of(&s, spout), Some(lifting), "joined by the pipe");
    assert_eq!(water(&s, at(HOLLOW.0, HOLLOW.1)), 50);
    run(&mut s, vec![], 199);
    // A pump lifts only from a wet tile, so a pond pumped dry is one
    // whose tile beside the pump is no longer wet: what remains is a
    // tenth of a level a tile, give or take the slope still water holds,
    // where each began with half a level.
    let left: u32 = POND.iter().map(|(x, y)| water(&s, at(*x, *y))).sum();
    let source = s.world.tile_index(at(-2, 0)).unwrap();
    assert!(!s.world.is_wet(source), "{}", water(&s, at(-2, 0)));
    for (x, y) in POND {
        assert!(water(&s, at(x, y)) < 110, "({x}, {y})");
    }
    assert!(left < 330, "{left} parts are left of 1500");
    // Every part lifted is in the hollow, whose floor is above the pond's
    // old surface: pipes take no account of height.
    assert_eq!(water(&s, at(HOLLOW.0, HOLLOW.1)), 1500 - left);
    assert_eq!(s.world.ground_water, ground, "none was used or dried");
    assert_eq!(total(&s), sum);
    assert_eq!(figures(&s, T0, lifting).supply, 0, "no source any more");
}

/// `docs/09`, The water network: the parts on every tile, the ground
/// water and what every tank holds together never change — through
/// pumps, spouts, tanks, a printer that uses water, a spring, digging
/// and filling, a building deconstructed full and a team going out.
#[test]
fn the_tiles_the_ground_water_and_the_tanks_never_change_in_sum() {
    // A printer with no well: it uses water, which goes to the ground
    // water. And a spring on the pond that brings up what the pump
    // lifts, so the pond lasts the run.
    let mut data = Data::load().unwrap();
    row(&mut data, "printer").well = 0;
    let text = format!("{MAP}[[spring]]\nat = [-4, 0]\nrate = \"0.05\"\n");
    let map = Map::parse(&text, &data).unwrap();
    let mut s = Sim::with_data(map, data);
    let (lifting, spout) = pond_and_hollow(&mut s);
    // A tank on the pump's network, pipe from it to team 0's printer, and
    // a tank of the rival's with water in it.
    let tank = place(&mut s, Model::Tank, T0, at(-1, -1));
    for x in -7..=-1 {
        assert!(s.world.lay_pipe(T0, at(x, -2)));
    }
    assert!(s.world.lay_pipe(T0, at(-7, -3)));
    let rivals = place(&mut s, Model::Tank, T1, at(6, 2));
    fill(&mut s, rivals, 700);
    // A bot beside the pond's north bank that digs it down two levels and
    // fills it back, over and over, and one that takes the tank down
    // late on.
    let digger = bot(&mut s, T0, at(-3, 2), "0");
    let wrecker = bot(&mut s, T0, at(0, -1), "0");
    let bots = "if me().pos == (-3, 2):\n    n = 0\n    while True:\n        try:\n            if n % 4 < 2:\n                dig(-3, 1)\n            else:\n                fill(-3, 1)\n        except ValueError:\n            wait(1)\n        n += 1\nelif me().pos == (0, -1):\n    wait(400)\n    deconstruct(-1, -1)\nwait(1000)\n";
    // The spout pours in bursts, so the tanks fill between them.
    let spouts = "while True:\n    pour('e')\n    wait(6)\n    pour(None)\n    wait(6)\n";
    let (sum, ground) = (total(&s), s.world.ground_water);
    assert_eq!(s.world.water_total(), sum);
    let mut cmds = vec![deploy(0, "bot", bots), deploy(0, "spout", spouts)];
    cmds[1].seq = 1;
    let (mut most_stored, mut dug, mut used) = (0, false, false);
    let p0 = printer(&s, T0);
    for tick in 1..=500 {
        if tick == 500 {
            // The rival goes out with 700 parts in its tank.
            cmds.push(Command::new(tick, T1, 0, CommandKind::Resign));
        }
        let report = s.step(&cmds, &[]);
        assert!(report.dropped.is_empty(), "dropped {:?}", report.dropped);
        cmds.clear();
        assert_eq!(total(&s), sum, "tick {tick}: {:?}", parts_of(&s));
        assert_eq!(s.world.water_total(), sum, "tick {tick}");
        if tick == 500 {
            assert!(s.world.teams[&T1].out);
        }
        most_stored = most_stored.max(s.world.machines.get(&tank).map_or(0, |m| m.water));
        dug |= s.world.tile(at(-3, 1)).unwrap().height.level() < 4;
        used |= machine(&s, p0).printing > 0 && machine(&s, p0).watered;
    }
    // The run did what it claims to have survived.
    assert!(water(&s, at(HOLLOW.0, HOLLOW.1)) > 500, "the spout poured");
    assert!(most_stored > 1000, "the tank filled between bursts");
    assert!(s.world.ground_water != ground, "the ground water moved");
    assert!(dug, "{digger:?} dug the bank");
    assert!(used, "the printer printed on piped water");
    assert!(
        !s.world.machines.contains_key(&tank),
        "{wrecker:?} took the tank down"
    );
    assert!(!s.world.machines.contains_key(&rivals));
    assert_eq!(network_of(&s, spout), Some(lifting));
}

/// `docs/09`, Pumps, tanks and spouts: a building that leaves the world —
/// deconstructed, dead, or its team out — gives what its tank holds to
/// the ground water.
#[test]
fn a_building_that_leaves_the_world_gives_its_tanks_water_to_the_ground_water() {
    let mut s = sim();
    let taken_down = place(&mut s, Model::Tank, T0, at(0, 0));
    let killed = place(&mut s, Model::Tank, T0, at(0, -2));
    let rivals = place(&mut s, Model::Tank, T1, at(4, 2));
    for (id, parts) in [(taken_down, 700), (killed, 400), (rivals, 300)] {
        fill(&mut s, id, parts);
    }
    let sum = total(&s);
    // Deconstructed, by a bot.
    bot(&mut s, T0, at(1, 0), "0");
    let ground = s.world.ground_water;
    let take_down = "try:\n    deconstruct(0, 0)\nexcept ValueError:\n    pass\nwait(1000)\n";
    run(&mut s, vec![deploy(0, "bot", take_down)], 12);
    assert!(!s.world.machines.contains_key(&taken_down));
    assert_eq!(s.world.ground_water, ground + 700);
    // Dead: `dying` ends in `death`.
    s.world.set_health(killed, Num::ZERO);
    s.world
        .machines
        .get_mut(&killed)
        .unwrap()
        .program
        .as_mut()
        .unwrap()
        .raise(lang::Interrupt::Dying);
    run(&mut s, vec![], 2);
    assert!(!s.world.machines.contains_key(&killed));
    assert_eq!(s.world.ground_water, ground + 1100);
    // Its team out.
    s.step(
        &[Command::new(s.world.tick + 1, T1, 0, CommandKind::Resign)],
        &[],
    );
    assert!(!s.world.machines.contains_key(&rivals));
    assert_eq!(s.world.ground_water, ground + 1400);
    assert_eq!(total(&s), sum);
}

// ---------------------------------------------------------- a dry building ---

/// `docs/09`, A dry building: a dry printer begins no print, and its
/// print in progress holds; it still counts for its team not being out.
#[test]
fn a_dry_printer_prints_nothing() {
    let no_well = |data: &mut Data| row(data, "printer").well = 0;
    let bots = |s: &Sim| s.world.bots_of(T0);
    // The match is built with every building watered, so the printer
    // begins a print in tick 1; that tick's network step then finds its
    // need uncovered, and the print holds where it stood.
    let mut s = sim_with(no_well);
    let p = printer(&s, T0);
    run(&mut s, vec![], 1);
    assert!(!machine(&s, p).watered);
    let ticks = s.world.data.model("printer").print_ticks;
    assert_eq!(machine(&s, p).printing, ticks);
    run(&mut s, vec![], 200);
    assert_eq!(machine(&s, p).printing, ticks, "the print holds");
    assert_eq!(bots(&s), 0, "a dry printer prints nothing");
    assert!(!s.world.teams[&T0].out, "it still counts for its team");
    // Dry is not dark: the power step serves it as it does any other.
    assert!(machine(&s, p).powered);
    assert_eq!(machine(&s, p).charge, num("500"));
    // One dry before it began: it begins none.
    let mut s = sim_with(no_well);
    let p = printer(&s, T0);
    s.world.machines.get_mut(&p).unwrap().watered = false;
    run(&mut s, vec![], 100);
    assert_eq!(machine(&s, p).printing, 0, "a dry printer begins no print");
    assert_eq!(bots(&s), 0);
    // A dry printer does not defend: a rival's bot beside it is unharmed.
    let rival = bot(&mut s, T1, at(-7, -3), "0");
    run(&mut s, vec![], 5);
    assert_eq!(machine(&s, rival).health, num("10"));
    // Water in its own tank covers its need: watered at the next network
    // step, it begins a print the tick after and the bot appears.
    s.world.remove(rival);
    fill(&mut s, p, 2000);
    run(&mut s, vec![], 1);
    assert!(machine(&s, p).watered);
    assert_eq!(held(&s, p), 1998, "an idle printer's need, two parts");
    run(&mut s, vec![], 1);
    assert_eq!(machine(&s, p).printing, ticks);
    assert_eq!(held(&s, p), 1988, "and ten while it prints");
    run(&mut s, vec![], ticks);
    assert_eq!(bots(&s), 1);
}

/// `docs/09`, A dry building: its program and its senses stop and no bot
/// recharges from it, as a dark one's do, whatever its power; and it goes
/// on when its network covers it again.
#[test]
fn a_dry_building_runs_no_main_flow_sees_nothing_and_gives_no_charge() {
    // A tank that sees its own tile and no further, so that the tiles
    // round the charger are seen by the charger alone.
    let mut s = sim_with(|data| {
        thirsty(30)(data);
        row(data, "tank").vision = 0;
    });
    let c = place(&mut s, Model::Charger, T0, at(6, 3));
    let tank = place(&mut s, Model::Tank, T0, at(6, 4));
    fill(&mut s, tank, 30);
    s.world.machines.get_mut(&c).unwrap().charge = num("60");
    let beside = bot(&mut s, T1, at(5, 3), "0");
    s.world.machines.get_mut(&beside).unwrap().charge = num("10");
    let program = "while True:\n    log('charger ticks')\n    wait(1)\n";
    let bot_program = "wait(3)\ntry:\n    recharge()\n    log('recharged')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    let own = at(6, 2);
    let idx = s.world.tile_index(own).unwrap();
    let mut lines = run(
        &mut s,
        vec![deploy(0, "charger", program), deploy(1, "bot", bot_program)],
        1,
    );
    // Placed watered, and watered for a tick by the tank's 30.
    assert!(machine(&s, c).watered);
    assert_eq!(s.world.teams[&T0].memory[idx].state, TileState::Visible);
    lines.extend(run(&mut s, vec![], 5));
    // The tank could not cover a second tick: dry from tick 2's network
    // step, so its program ran in ticks 1 and 2 and not after.
    assert!(!machine(&s, c).watered);
    assert_eq!(lines_of(&lines, c), ["charger ticks", "charger ticks"]);
    assert_eq!(s.world.teams[&T0].memory[idx].state, TileState::Remembered);
    assert_eq!(lines_of(&lines, beside), ["refused"]);
    // Everything else stands: it is powered, its battery still fills —
    // its own half a tick for six ticks — and it is on its network.
    assert!(machine(&s, c).powered);
    assert_eq!(machine(&s, c).charge, num("63"));
    assert_eq!(network_of(&s, c), Some(c));
    // Covered again, it goes on from where it stopped.
    fill(&mut s, tank, 90);
    let lines = run(&mut s, vec![], 3);
    assert!(machine(&s, c).watered);
    assert_eq!(lines_of(&lines, c), ["charger ticks", "charger ticks"]);
    assert_eq!(s.world.teams[&T0].memory[idx].state, TileState::Visible);
}

// ------------------------------------------------------------------- pipe ---

#[test]
fn a_bot_builds_an_aqueduct_which_runs_no_program_and_a_pump_which_does() {
    let mut s = sim();
    let b = bot(&mut s, T0, at(0, 0), "10");
    let program = "build('aqueduct', 1, 0)\ndrop('ore')\nwait(1000)\n";
    run(&mut s, vec![deploy(0, "bot", program)], 12);
    let aqueduct = s
        .world
        .machines
        .values()
        .find(|m| m.model == Model::Aqueduct)
        .expect("the aqueduct was built");
    assert_eq!(aqueduct.pos, at(1, 0));
    assert_eq!(aqueduct.deployment, None);
    assert!(aqueduct.program.is_none(), "an aqueduct runs no program");
    assert!(aqueduct.watered && aqueduct.network == Some(aqueduct.id));
    assert_eq!(ore(&s, b), num("5"), "an aqueduct costs 5");
    assert!(s.validate(&deploy(0, "aqueduct", "wait(1)\n")).is_err());
    for model in [Model::Pump, Model::Tank, Model::Spout] {
        let id = place(&mut s, model, T0, at(-3, 3));
        assert_eq!(machine(&s, id).deployment.as_deref(), Some(model.name()));
        assert!(machine(&s, id).program.is_some());
        s.world.remove(id);
    }
}

/// Q67: pipe is laid for ore by the team's own bots, consumes its plan,
/// and a second bot laying the same tile in the same tick pays nothing.
#[test]
fn two_bots_piping_one_tile_pay_once_and_the_plan_is_consumed() {
    let mut s = sim();
    let a = bot(&mut s, T0, at(-1, 0), "4");
    let b = bot(&mut s, T0, at(1, 0), "4");
    // One tick first: a pipe plan goes only on ground the team has seen.
    run(&mut s, vec![], 1);
    let mark = Command::new(
        2,
        T0,
        0,
        CommandKind::Mark {
            at: at(0, 0),
            plan: PlanKind::Pipe,
            value: Some("pipe".into()),
        },
    );
    let program = "log(plans('pipe'), tile(0, 0).plans['pipe'])\npipe(0, 0)\nlog('piped', tile(0, 0).pipes, pipes(), plans('pipe'))\nwait(1000)\n";
    let lines = run(&mut s, vec![mark, deploy(0, "bot", program)], 4);
    assert!(s.world.has_pipe(T0, at(0, 0)));
    assert!(
        s.world.plans(T0, at(0, 0)).is_none(),
        "the plan is consumed"
    );
    assert_eq!(
        lines_of(&lines, a),
        ["[(0, 0, True)] True", "piped [0] [(0, 0)] []"]
    );
    assert_eq!(
        (ore(&s, a), ore(&s, b)),
        (num("1"), num("4")),
        "the lower id paid 3"
    );
    assert!(!s.world.has_wire(T0, at(0, 0)), "a pipe is no wire");
    // Laying where the team's pipe lies, and piping without the ore, are
    // refused; a rival may lay its own on the same tile.
    let c = bot(&mut s, T1, at(0, 1), "3");
    let again = "try:\n    pipe(0, 0)\n    log('laid')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    let lines = run(
        &mut s,
        vec![deploy(0, "bot", again), deploy(1, "bot", again)],
        4,
    );
    assert_eq!(lines_of(&lines, a), ["refused"], "its team's pipe is there");
    assert_eq!(lines_of(&lines, b), ["refused"]);
    assert_eq!(lines_of(&lines, c), ["laid"]);
    assert_eq!(
        s.world.tile(at(0, 0)).unwrap().pipes,
        [T0, T1].into_iter().collect()
    );
    // Only its own team removes a pipe, with nothing refunded.
    let take_up = "try:\n    unpipe(0, 0)\n    log('gone')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    s.world.remove_pipe(T1, at(0, 0));
    let lines = run(
        &mut s,
        vec![deploy(0, "bot", take_up), deploy(1, "bot", take_up)],
        4,
    );
    assert_eq!(lines_of(&lines, c), ["refused"], "a rival cannot cut it");
    assert_eq!(lines_of(&lines, a), ["gone"]);
    assert!(!s.world.has_pipe(T0, at(0, 0)));
    assert_eq!(ore(&s, a), num("1"));
}

/// `docs/03`: a pipe plan is a wire plan in every rule — only on ground
/// the team has seen, and one asking for what the tile already has is
/// dropped on its tick.
#[test]
fn a_pipe_plan_that_cannot_be_carried_out_is_dropped() {
    let mut s = sim();
    s.step(&[], &[]);
    let pipe = |at: TilePos, lay: bool, tick: u64, seq: u32| {
        Command::new(
            tick,
            T0,
            seq,
            CommandKind::Mark {
                at,
                plan: PlanKind::Pipe,
                value: lay.then(|| "pipe".to_string()),
            },
        )
    };
    let seen = at(-6, -2);
    let unseen = at(6, 4);
    assert!(s.world.lay_pipe(T0, at(-6, -3)));
    let report = s.step(
        &[
            pipe(seen, true, 2, 0),
            // Still unknown to the team; and a clear where no pipe lies;
            // and a lay where one does.
            pipe(unseen, true, 2, 1),
            pipe(at(-5, -2), false, 2, 2),
            pipe(at(-6, -3), true, 2, 3),
            pipe(at(-6, -3), false, 2, 4),
        ],
        &[],
    );
    let dropped: Vec<u32> = report.dropped.iter().map(|c| c.seq).collect();
    assert_eq!(dropped, [1, 2, 3]);
    assert_eq!(
        s.world.plans(T0, seen).unwrap().pipe,
        Some(Plan::Set("pipe".into()))
    );
    assert_eq!(
        s.world.plans(T0, at(-6, -3)).unwrap().pipe,
        Some(Plan::Clear)
    );
    // A wire plan on the same tile is its own plan, and an unmark
    // withdraws the one it names.
    let wire = Command::new(
        3,
        T0,
        0,
        CommandKind::Mark {
            at: seen,
            plan: PlanKind::Wire,
            value: Some("wire".into()),
        },
    );
    let unmark = Command::new(
        3,
        T0,
        1,
        CommandKind::Unmark {
            at: seen,
            plan: PlanKind::Pipe,
        },
    );
    assert!(s.step(&[wire, unmark], &[]).dropped.is_empty());
    let plans = s.world.plans(T0, seen).unwrap();
    assert_eq!((plans.pipe.clone(), plans.wire.is_some()), (None, true));
    // A pipe plan's value is `pipe`, checked at submission.
    let bad = Command::new(
        4,
        T0,
        0,
        CommandKind::Mark {
            at: seen,
            plan: PlanKind::Pipe,
            value: Some("wire".into()),
        },
    );
    assert!(s.validate(&bad).is_err());
}

#[test]
fn a_team_that_is_out_loses_its_pipes_and_its_networks() {
    let mut s = sim();
    assert!(s.world.lay_pipe(T1, at(4, 0)));
    assert!(s.world.lay_pipe(T0, at(4, 0)));
    s.step(&[Command::new(1, T1, 0, CommandKind::Resign)], &[]);
    assert!(!s.world.has_pipe(T1, at(4, 0)));
    assert!(s.world.has_pipe(T0, at(4, 0)), "another team's stays");
    assert!(s.world.teams[&T1].networks.is_empty());
    assert!(s.world.teams[&T1].pipe_tiles.is_empty());
}

// ------------------------------------------------ what a program reads ---

#[test]
fn a_program_reads_its_water_its_network_and_the_teams_water() {
    let mut s = sim();
    let p = printer(&s, T0);
    // Half a level in the printer's tank, a pump north of the printer
    // with a pit east of it, and a spout: one network, and one grid, the
    // printer the oldest on both.
    fill(&mut s, p, 500);
    pit(&mut s, at(-6, -3), 1000);
    let lifting = place(&mut s, Model::Pump, T0, at(-7, -3));
    let spout = place(&mut s, Model::Spout, T0, at(-7, -2));
    let b = bot(&mut s, T0, at(0, 2), "0");
    let bot_program = "m = me()\nlog(m.water, m.tank)\nwait(1000)\n";
    let printer_program = "wait(1)\nm = me()\nlog(m.water, m.tank, m.watered, m.network == m.id)\nfor n in water():\n    log(n.network == m.id, n.supply, n.drain, n.stored, n.capacity)\nlog([(b.model, b.watered, b.network == m.id) for b in see() if b.model in ('pump', 'spout')])\nwait(1000)\n";
    let spout_program = "log(me().pouring, me().tank)\nwait(1000)\n";
    let lines = run(
        &mut s,
        vec![
            deploy(0, "bot", bot_program),
            deploy(0, "printer", printer_program),
            deploy(0, "spout", spout_program),
        ],
        2,
    );
    assert_eq!(lines_of(&lines, b), ["0 0"], "a bot carries no water");
    assert_eq!(lines_of(&lines, spout), ["None 0"]);
    // The pump, on the printer's grid, lifted its 0.05 into the printer's
    // tank in tick 1: 0.55 of its 2 levels.
    assert_eq!(network_of(&s, lifting), Some(p));
    assert_eq!(
        lines_of(&lines, p),
        [
            "0.55 2 True True",
            "True 0.05 0 0.55 2",
            "[('pump', True, True), ('spout', True, True)]"
        ]
    );
}

/// `docs/06`, The state hash: every attribute the network adds is in it.
/// Each is changed by fiat after a tick, alone, so that nothing a step
/// derives from it — a figure, a team's memory of the tile — stands in
/// for it.
#[test]
fn the_state_hash_sees_everything_the_network_adds() {
    let after = |change: fn(&mut Sim, EntityId, EntityId)| {
        let mut s = sim();
        let tank = place(&mut s, Model::Tank, T0, at(0, 0));
        let spout = place(&mut s, Model::Spout, T0, at(0, 2));
        s.step(&[], &[]);
        change(&mut s, tank, spout);
        s.state_hash()
    };
    let hashes = [
        after(|_, _, _| {}),
        // A pipe on a tile, and a plan for one.
        after(|s, _, _| assert!(s.world.lay_pipe(T0, at(3, 0)))),
        after(|s, _, _| {
            s.world.plans_mut(T0, at(3, 0)).unwrap().pipe = Some(Plan::Set("pipe".into()));
        }),
        // A pipe in a team's memory of a tile.
        after(|s, _, _| {
            let idx = s.world.tile_index(at(-7, -3)).unwrap();
            let team = s.world.teams.get_mut(&T0).unwrap();
            team.memory[idx].pipes.insert(T1);
        }),
        // What a tank holds; a building dry, and on another network; the
        // way a spout pours.
        after(|s, tank, _| s.world.machines.get_mut(&tank).unwrap().water = 1),
        after(|s, tank, _| s.world.machines.get_mut(&tank).unwrap().watered = false),
        after(|s, tank, _| s.world.machines.get_mut(&tank).unwrap().network = None),
        after(|s, _, spout| s.world.machines.get_mut(&spout).unwrap().pouring = Some(Dir::N)),
        after(|s, _, spout| s.world.machines.get_mut(&spout).unwrap().pouring = Some(Dir::E)),
        // A network's figures, each of the four.
        after(|s, _, _| s.world.teams.get_mut(&T0).unwrap().networks[0].supply = 7),
        after(|s, _, _| s.world.teams.get_mut(&T0).unwrap().networks[0].drain = 7),
        after(|s, _, _| s.world.teams.get_mut(&T0).unwrap().networks[0].stored = 7),
        after(|s, _, _| s.world.teams.get_mut(&T0).unwrap().networks[0].capacity = 7),
    ];
    for (i, a) in hashes.iter().enumerate() {
        for b in &hashes[i + 1..] {
            assert_ne!(a, b, "{hashes:x?}");
        }
    }
}

// ---------------------------------------------------- nothing piped yet ---

/// `docs/09`, A need and a well: data starts the printer's well at its
/// `water_drain` plus its `print_water`, so a team that lays no pipe
/// plays exactly as it did. The well is what makes that so and nothing
/// else: a match on the shipped tables is the same, hash for hash, as one
/// on tables whose printer needs no water at all.
#[test]
fn a_team_that_lays_no_pipe_plays_as_before() {
    let shipped = Data::load().unwrap();
    let printer_row = shipped.model("printer");
    assert!(printer_row.water_drain > 0 && printer_row.print_water > 0);
    assert_eq!(
        printer_row.well,
        printer_row.water_drain + printer_row.print_water,
        "the well covers the printer's need, printing or not"
    );
    // Bots that wander south-east, picking ore as they pass a deposit.
    let program = "n = me().id\nwhile True:\n    n += 1\n    try:\n        if n % 5 == 0:\n            pick('ore')\n        else:\n            move(['n', 'e', 'e', 's', 'w'][n % 5])\n    except ValueError:\n        wait(1)\n";
    let play = |edit: fn(&mut Data)| {
        let mut s = sim_with(edit);
        let sum = total(&s);
        let mut hashes = Vec::new();
        for tick in 1..=400 {
            let cmds = if tick == 1 {
                vec![deploy(0, "bot", program), {
                    let mut c = deploy(1, "bot", program);
                    c.seq = 1;
                    c
                }]
            } else {
                Vec::new()
            };
            s.step(&cmds, &[]);
            hashes.push(s.state_hash());
            // Nothing is asked of the map's water, tick after tick.
            assert_eq!(parts_of(&s), (0, sum, 0), "tick {tick}");
            assert!(
                s.world
                    .machines
                    .values()
                    .filter(|m| m.is_building())
                    .all(|m| m.watered && m.powered),
                "tick {tick}"
            );
        }
        (hashes, s.world.bots_of(T0), s.world.bots_of(T1))
    };
    let (with_well, bots0, bots1) = play(|_| {});
    let (no_need, ..) = play(|data| {
        let p = row(data, "printer");
        (p.water_drain, p.print_water, p.well) = (0, 0, 0);
    });
    assert!(
        bots0 >= 5 && bots1 >= 5,
        "the printers printed: {bots0}, {bots1}"
    );
    assert!(
        with_well == no_need,
        "a printer whose well covers its need is one with no need"
    );
}
