//! Moving (`docs/03`, Moving; Q65): pace, a move's ticks over a slope, a
//! drop and its fall, and a bonk. T43's last part, one rule a test.

use lang::Num;
use sim::world::{Effect, EntityId, Model};
use sim::{Command, CommandKind, Data, Map, Sim, TeamId, TilePos, bundle_of};

/// 13 by 7, origin at the centre: x in -6..=6, y in -3..=3, the first row
/// the north edge. Team 0's printer is in the south-west corner and team
/// 1's in the south-east, each with a deposit beside it; a rock stands at
/// (4, -2).
///
/// The two northern rows are level, for pace. Row 1 rises two levels and
/// comes down again. Row 0 has a ledge two levels above the tile east of
/// it, at (-6, 0), and a pillar four levels above its sides, at (0, 0).
/// Row -1 has a pit four deep at (2, -1), and a ledge at (-6, -1) two
/// levels above the tile beside team 0's printer. Row -2 is level, with
/// the rock.
const MAP: &str = r#"
name = "moving"
speed = 10
width = 13
height = 7
ground_water = 20
terrain = """
.............
.............
.............
.............
.............
..........#..
.o.........o.
"""
heights = """
4444444444444
4444444444444
4456544444444
6444448444444
6444444404444
4444444444444
4444444444444
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-6, -3]
[[team]]
start = [6, -3]
"#;

const PIT: (i32, i32) = (2, -1);

fn at((x, y): (i32, i32)) -> TilePos {
    TilePos::new(x, y)
}

fn num(text: &str) -> Num {
    Num::parse_literal(text).unwrap()
}

/// The tables, with the lab's own numbers for moving, so that these tests
/// hold the rules to account and do not move when the shipped numbers are
/// tuned: a move of three ticks, a pace of two at the most, two ticks more
/// uphill and one fewer down, a bonk of two ticks, a fall that costs 1.5 a
/// level, half a level of water to break one, and a dig long enough to be
/// interrupted.
fn lab_data() -> Data {
    let mut data = Data::load().unwrap();
    let bot = data.machines.models.get_mut("bot").unwrap();
    bot.move_ticks = 3;
    bot.pace_max = 2;
    bot.up_ticks = 2;
    bot.down_ticks = 1;
    bot.bonk_ticks = 2;
    bot.fall_damage = num("1.5");
    bot.dig_ticks = 20;
    data.world.water.cushion = data.world.water.parts.checked_div(2).unwrap();
    data
}

fn sim() -> Sim {
    let data = lab_data();
    let map = Map::parse(MAP, &data).unwrap();
    Sim::with_data(map, data)
}

fn bot_at(s: &mut Sim, team: u32, tile: (i32, i32)) -> EntityId {
    s.world
        .place(Model::Bot, TeamId(team), at(tile), Some("bot".into()))
}

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

/// What a tick left of one bot: where it stood, its pace, its health, and
/// what it logged.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    pos: (i32, i32),
    pace: u64,
    health: Num,
    logs: Vec<String>,
}

/// The run, tick by tick: `ticks[t - 1]` is what tick `t` left of each bot
/// watched, in the order given.
struct Run {
    sim: Sim,
    bots: Vec<EntityId>,
    ticks: Vec<Vec<Seen>>,
}

impl Run {
    /// The bot `i` as tick `t` left it.
    fn seen(&self, t: usize, i: usize) -> &Seen {
        &self.ticks[t.checked_sub(1).unwrap()][i]
    }

    /// Each tick that left bot `i` on another tile than the one before:
    /// the tick, the tile, and its pace then.
    fn landings(&self, i: usize, from: (i32, i32)) -> Vec<(usize, (i32, i32), u64)> {
        let mut out = Vec::new();
        let mut last = from;
        for (t, tick) in self.ticks.iter().enumerate() {
            let seen = &tick[i];
            if seen.pos != last {
                last = seen.pos;
                out.push((t.checked_add(1).unwrap(), seen.pos, seen.pace));
            }
        }
        out
    }

    /// Whether bot `i` is in an action as the run ends.
    fn busy(&self, i: usize) -> bool {
        self.sim.world.machines[&self.bots[i]].action.is_some()
    }

    /// Everything bot `i` logged, in order.
    fn logs(&self, i: usize) -> Vec<String> {
        self.ticks
            .iter()
            .flat_map(|tick| tick[i].logs.clone())
            .collect()
    }
}

/// Run `s` for `ticks`, the programs deployed with tick 1, watching `bots`.
fn run_on(mut s: Sim, programs: &[(u32, &str)], bots: &[EntityId], ticks: u64) -> Run {
    let cmds: Vec<Command> = programs.iter().map(|(t, main)| deploy(*t, main)).collect();
    let mut out = Vec::new();
    for t in 0..ticks {
        s.step(if t == 0 { &cmds } else { &[] }, &[]);
        out.push(
            bots.iter()
                .map(|id| {
                    let m = &s.world.machines[id];
                    Seen {
                        pos: (m.pos.x, m.pos.y),
                        pace: m.pace,
                        health: m.health,
                        logs: m.log.iter().map(|l| l.text.clone()).collect(),
                    }
                })
                .collect(),
        );
    }
    Run {
        sim: s,
        bots: bots.to_vec(),
        ticks: out,
    }
}

/// One bot of team 0 placed at `from`, run on `main`.
fn walk(from: (i32, i32), main: &str, ticks: u64) -> Run {
    let mut s = sim();
    let bot = bot_at(&mut s, 0, from);
    run_on(s, &[(0, main)], &[bot], ticks)
}

const EAST: &str = "while True:\n    move('e')\n";

/// A program that waits where it stands.
const STAND: &str = "wait(100)\nlog('stood up at', me().pos[0])\nwait(1000)\n";

// ------------------------------------------------------------- pace ---

#[test]
fn a_bot_that_keeps_moving_gathers_pace() {
    let run = walk((-6, 3), EAST, 8);
    // Three ticks from a standstill, two with a pace of one, and one from
    // then on: the pace stops at `pace_max`, and a move is never nothing.
    assert_eq!(
        run.landings(0, (-6, 3)),
        [
            (3, (-5, 3), 1),
            (5, (-4, 3), 2),
            (6, (-3, 3), 2),
            (7, (-2, 3), 2),
            (8, (-1, 3), 2),
        ]
    );
    // Its pace is gained as a move completes, not as it begins.
    assert_eq!((run.seen(1, 0).pace, run.seen(2, 0).pace), (0, 0));
    assert_eq!(run.seen(4, 0).pace, 1);
}

#[test]
fn a_bot_that_pauses_starts_again_from_a_standstill() {
    let main = "move('e')\nmove('e')\nwait(1)\nwhile True:\n    move('e')\n";
    let run = walk((-6, 3), main, 11);
    // Two moves, of three ticks and two; the wait of tick 6 is no move, so
    // as that tick's slices end the pace is gone; and the next move takes
    // three ticks again, to tick 9, and the one after it two.
    assert_eq!(
        run.landings(0, (-6, 3)),
        [
            (3, (-5, 3), 1),
            (5, (-4, 3), 2),
            (9, (-3, 3), 1),
            (11, (-2, 3), 2),
        ]
    );
    assert_eq!((run.seen(5, 0).pace, run.seen(6, 0).pace), (2, 0));
}

#[test]
fn a_failed_move_that_ends_the_slice_loses_the_pace() {
    // North of the top row is off the map: a `ValueError`, and a fault,
    // which ends the slice with the bot in no move.
    let main = "move('e')\nmove('e')\nmove('n')\n";
    let run = walk((-6, 3), main, 7);
    assert_eq!((run.seen(5, 0).pace, run.seen(6, 0).pace), (2, 0));
    assert_eq!(run.seen(6, 0).pos, (-4, 3));
}

#[test]
fn a_failed_move_followed_by_a_move_in_the_same_slice_keeps_the_pace() {
    // The rule is the one test as the slices end (`docs/06`, step 2): a
    // bot that caught its `ValueError` and began another move before its
    // slice was over is in a move then, and keeps its pace.
    let main = "move('e')\nmove('e')\ntry:\n    move('n')\nexcept ValueError:\n    pass\nmove('e')\nwait(1000)\n";
    let run = walk((-6, 3), main, 7);
    assert_eq!(
        run.landings(0, (-6, 3)),
        [(3, (-5, 3), 1), (5, (-4, 3), 2), (6, (-3, 3), 2)]
    );
}

#[test]
fn a_bot_keeps_its_pace_through_a_turn() {
    let main = "move('e')\nmove('e')\nmove('s')\nmove('w')\nmove('w')\nwait(1000)\n";
    let run = walk((-6, 3), main, 9);
    // East, east, south, west, west: the turns cost nothing, so every
    // move after the second takes a tick.
    assert_eq!(
        run.landings(0, (-6, 3)),
        [
            (3, (-5, 3), 1),
            (5, (-4, 3), 2),
            (6, (-4, 2), 2),
            (7, (-5, 2), 2),
            (8, (-6, 2), 2),
        ]
    );
}

#[test]
fn move_to_takes_its_ticks_from_the_pace_too() {
    let main = "while True:\n    move_to(6, 3)\n";
    let run = walk((-6, 3), main, 7);
    assert_eq!(
        run.landings(0, (-6, 3)),
        [
            (3, (-5, 3), 1),
            (5, (-4, 3), 2),
            (6, (-3, 3), 2),
            (7, (-2, 3), 2),
        ]
    );
}

#[test]
fn a_climb_is_slower_and_gains_nothing_and_a_descent_is_faster_and_gains_two() {
    // Row 1: level to (-5, 1), up to 5 and to 6, down to 5 and to 4.
    let run = walk((-6, 1), EAST, 14);
    assert_eq!(
        run.landings(0, (-6, 1)),
        [
            // Level, from a standstill: three ticks.
            (3, (-5, 1), 1),
            // Up with a pace of one: three, less one, plus two. No gain.
            (7, (-4, 1), 1),
            (11, (-3, 1), 1),
            // Down with a pace of one: three, less one, less one. It
            // gains two, to the most a bot has.
            (12, (-2, 1), 2),
            // Down at full pace would be nothing: never fewer than one.
            (13, (-1, 1), 2),
            (14, (0, 1), 2),
        ]
    );
    // From a standstill a climb is five ticks and a descent two.
    let up = walk((-5, 1), "move('e')\nwait(1000)\n", 6);
    assert_eq!(up.landings(0, (-5, 1)), [(5, (-4, 1), 0)]);
    // The descent gains its two, which the wait that follows then loses.
    let down = walk((-3, 1), "move('e')\nwait(1000)\n", 6);
    assert_eq!(down.landings(0, (-3, 1)), [(2, (-2, 1), 2)]);
    assert_eq!(down.seen(3, 0).pace, 0);
}

#[test]
fn a_program_reads_its_pace_and_a_building_has_none() {
    let main = "log(me().pace)\nmove('e')\nlog(me().pace)\nmove('e')\nlog(me().pace)\nwait(1000)\n";
    let run = walk((-6, 3), main, 7);
    assert_eq!(run.logs(0), ["0", "1", "2"]);
    let s = sim();
    let printer = s.world.machines.values().next().unwrap();
    assert_eq!(printer.model, Model::Printer);
    assert_eq!(sim::world::MachineRecord::of(printer).pace, None);
}

#[test]
fn a_bots_pace_is_in_the_hash() {
    let hash_with = |pace: u64| {
        let mut s = sim();
        let bot = bot_at(&mut s, 0, (0, 3));
        s.world.machines.get_mut(&bot).unwrap().pace = pace;
        s.state_hash()
    };
    assert_ne!(hash_with(0), hash_with(1));
    assert_eq!(hash_with(1), hash_with(1));
}

// ------------------------------------------------------------ falls ---

const FALL: &str = "move('e')\nlog('landed', me().pace)\nwait(1000)\n";

#[test]
fn a_fall_of_two_levels_costs_one_level_of_damage() {
    // The ledge at (-6, 0) is two above the tile east of it.
    let run = walk((-6, 0), FALL, 4);
    // A drop takes the ticks of a move down: two, from a standstill.
    assert_eq!(run.landings(0, (-6, 0)), [(2, (-5, 0), 0)]);
    // One level beyond the step, in the damage step of the tick it lands.
    assert_eq!(run.seen(1, 0).health, num("10"));
    assert_eq!(run.seen(2, 0).health, num("8.5"));
    assert_eq!(run.seen(4, 0).health, num("8.5"), "once, not every tick");
    // It went down, and has no pace for it: it fell.
    assert_eq!(run.logs(0), ["landed 0"]);
}

#[test]
fn a_fall_of_four_levels_costs_three() {
    // The pillar at (0, 0) is four above the tiles beside it.
    let run = walk((0, 0), FALL, 4);
    assert_eq!(run.landings(0, (0, 0)), [(2, (1, 0), 0)]);
    assert_eq!(run.seen(2, 0).health, num("5.5"));
}

#[test]
fn a_fall_ends_a_run_with_no_pace() {
    // Row -1, east at full pace to the pit's edge and over it.
    let run = walk((-1, -1), EAST, 7);
    assert_eq!(
        run.landings(0, (-1, -1)),
        [
            (3, (0, -1), 1),
            (5, (1, -1), 2),
            // Down, at full pace: a tick. Four levels, and no pace.
            (6, PIT, 0),
        ]
    );
    assert_eq!(run.seen(6, 0).health, num("5.5"));
    // The pit's sides are cliffs, so there it stays.
    assert_eq!(run.seen(7, 0).pos, PIT);
}

#[test]
fn only_move_drops_and_nothing_climbs_a_cliff() {
    // On the pillar every tile beside the bot is a drop, so `move_to`,
    // which only steps, has nowhere to go.
    let main = "try:\n    move_to(3, 0)\nexcept ValueError:\n    log('no step')\nwait(1000)\n";
    let run = walk((0, 0), main, 4);
    assert_eq!(run.logs(0), ["no step"]);
    assert_eq!(run.seen(4, 0).pos, (0, 0));
    // And from below the pillar is a cliff, to `move` as well.
    let main = "try:\n    move('w')\nexcept ValueError:\n    log('a cliff')\nwait(1000)\n";
    let run = walk((1, 0), main, 4);
    assert_eq!(run.logs(0), ["a cliff"]);
    assert_eq!(run.seen(4, 0).pos, (1, 0));
}

#[test]
fn water_as_deep_as_the_cushion_breaks_a_fall() {
    let fall_into = |parts: u32| {
        let mut s = sim();
        let bot = bot_at(&mut s, 0, (1, -1));
        pour(&mut s, PIT, parts);
        let run = run_on(s, &[(0, FALL)], &[bot], 3);
        assert_eq!(run.landings(0, (1, -1)), [(2, PIT, 0)]);
        run.seen(3, 0).health
    };
    let cushion = lab_data().world.water.cushion;
    // At the cushion, all of it; a part short, none of it.
    assert_eq!(fall_into(cushion), num("10"));
    assert_eq!(fall_into(cushion.saturating_sub(1)), num("5.5"));
    assert_eq!(fall_into(0), num("5.5"));
}

#[test]
fn a_fall_is_summed_with_the_ticks_other_damage() {
    // Team 1's bot drops off the ledge at (-6, -1) onto the tile beside
    // team 0's printer, which defends itself in the same step.
    let mut s = sim();
    let bot = bot_at(&mut s, 1, (-6, -1));
    let defence = s.world.data.model("printer").defence_damage;
    assert_eq!(defence, num("1"));
    let main = "move('s')\nwait(1000)\n";
    let run = run_on(s, &[(1, main)], &[bot], 3);
    assert_eq!(run.landings(0, (-6, -1)), [(2, (-6, -2), 0)]);
    // The fall's 1.5 and the printer's 1, then the printer's alone.
    assert_eq!(run.seen(2, 0).health, num("7.5"));
    assert_eq!(run.seen(3, 0).health, num("6.5"));
}

// ------------------------------------------------------------ bonks ---

/// Team 0's runner: a move east for its pace, then east again.
const RUNNER: &str =
    "move('e')\nmove('e')\nlog('done at', me().pos[0], 'pace', me().pace)\nwait(1000)\n";

/// A runner of team 0 at `runner` and a bot of team 1 at `other`, two
/// tiles east of it, run on [`RUNNER`] and `theirs`. The runner is bot 0
/// of the run and the other bot 1.
fn bonk(runner: (i32, i32), other: (i32, i32), theirs: &str, ticks: u64) -> Run {
    let mut s = sim();
    let a = bot_at(&mut s, 0, runner);
    let b = bot_at(&mut s, 1, other);
    run_on(s, &[(0, RUNNER), (1, theirs)], &[a, b], ticks)
}

#[test]
fn a_bonk_puts_the_bot_it_hits_a_tile_onward() {
    let mut s = sim();
    let a = bot_at(&mut s, 0, (-4, -2));
    let b = bot_at(&mut s, 1, (-2, -2));
    let mut s = run_on(s, &[(0, RUNNER), (1, STAND)], &[a, b], 3).sim;
    // The runner has landed beside the other bot, with a pace of one.
    assert_eq!(s.world.machines[&a].pos, at((-3, -2)));
    assert_eq!(s.world.machines[&a].pace, 1);
    // Tick 4: its move onto the bot's tile begins as a bonk. It is on its
    // own tile alone, and what is heard is a bonk and no move.
    s.step(&[], &[]);
    let runner = &s.world.machines[&a];
    assert_eq!(runner.pos, at((-3, -2)));
    assert!(!runner.occupies(at((-2, -2))) && !runner.in_move());
    assert_eq!(runner.busy(), Some("move"));
    assert_eq!(runner.progress(), 1, "a bonk of two ticks, one gone");
    assert_eq!(
        runner.action.as_ref().map(|act| act.effect.clone()),
        Some(Effect::Bonk {
            to: at((-2, -2)),
            target: b
        })
    );
    let heard: Vec<(&str, TilePos, i64)> = s
        .world
        .sounds_prev
        .iter()
        .map(|snd| (snd.cause.as_str(), snd.pos, snd.loudness))
        .collect();
    let loudness = s.world.data.machines.loudness["bonk"];
    assert_eq!(heard, [("bonk", at((-3, -2)), loudness)]);
    assert_eq!(s.world.machines[&b].pos, at((-2, -2)), "not yet");
    // Tick 5: it completes. The other bot is a tile onward, the runner
    // where it was with no pace, and neither is hurt.
    s.step(&[], &[]);
    let (runner, other) = (&s.world.machines[&a], &s.world.machines[&b]);
    assert_eq!((runner.pos, runner.pace), (at((-3, -2)), 0));
    assert_eq!((other.pos, other.pace), (at((-1, -2)), 0));
    assert_eq!((runner.health, other.health), (num("10"), num("10")));
    assert!(runner.action.is_none());
}

#[test]
fn a_bonked_bots_action_ends_with_nothing_done() {
    // The other bot is digging the tile north of it, twenty ticks' work.
    let digging = "dig(-2, -1)\nlog('dig returned at', me().pos[0], 'with', me().load['earth'])\nwait(1000)\n";
    let run = bonk((-4, -2), (-2, -2), digging, 30);
    let s = &run.sim;
    // Bonked on tick 5, its dig returns in tick 6's slice, long before it
    // would have, with no earth; and the tile is never dug.
    assert_eq!(run.seen(5, 1).pos, (-1, -2));
    assert_eq!(run.seen(5, 1).logs, Vec::<String>::new());
    assert_eq!(run.seen(6, 1).logs, ["dig returned at -1 with 0"]);
    assert_eq!(s.world.tile(at((-2, -1))).unwrap().height.level(), 4);
    assert_eq!(run.logs(0), ["done at -3 pace 0"]);
    // A bot waiting is a bot in an action too: its wait of a hundred
    // ticks is over.
    let run = bonk((-4, -2), (-2, -2), STAND, 7);
    assert_eq!(run.seen(6, 1).logs, ["stood up at -1"]);
}

#[test]
fn a_bonk_off_a_cliff_is_a_fall() {
    // The pit's edge: the bot at (1, -1) is put into the pit, four deep.
    let run = bonk((-1, -1), (1, -1), STAND, 6);
    assert_eq!(run.seen(4, 1).pos, (1, -1));
    assert_eq!(run.seen(5, 1).pos, PIT);
    // The bonk does no damage; the fall does, in the same tick's damage
    // step, and only to the bot that fell.
    assert_eq!(run.seen(4, 1).health, num("10"));
    assert_eq!(run.seen(5, 1).health, num("5.5"));
    assert_eq!(run.seen(6, 1).health, num("5.5"));
    assert_eq!(run.seen(6, 0).health, num("10"));
    assert_eq!((run.seen(5, 0).pos, run.seen(5, 0).pace), ((0, -1), 0));
    // Water in the pit as deep as the cushion would have broken it.
    let mut s = sim();
    let a = bot_at(&mut s, 0, (-1, -1));
    let b = bot_at(&mut s, 1, (1, -1));
    let cushion = s.world.data.world.water.cushion;
    pour(&mut s, PIT, cushion);
    let run = run_on(s, &[(0, RUNNER), (1, STAND)], &[a, b], 6);
    assert_eq!(
        (run.seen(6, 1).pos, run.seen(6, 1).health),
        (PIT, num("10"))
    );
}

#[test]
fn a_bonk_against_a_wall_moves_nothing() {
    // The rock at (4, -2) is beyond the bot at (3, -2).
    let run = bonk((1, -2), (3, -2), STAND, 8);
    assert_eq!(run.seen(8, 1).pos, (3, -2));
    assert_eq!(run.logs(1), Vec::<String>::new(), "its wait goes on");
    assert!(run.busy(1));
    // The runner bonked all the same, and stopped dead.
    assert_eq!(run.logs(0), ["done at 2 pace 0"]);
    assert_eq!(run.seen(8, 0).pos, (2, -2));

    // A machine on the tile beyond is a wall too, and so is the pillar's
    // side, a cliff no bot is put up.
    let mut s = sim();
    let a = bot_at(&mut s, 0, (-4, -2));
    let b = bot_at(&mut s, 1, (-2, -2));
    let c = bot_at(&mut s, 1, (-1, -2));
    let run = run_on(s, &[(0, RUNNER), (1, STAND)], &[a, b, c], 8);
    assert_eq!(
        (run.seen(8, 1).pos, run.seen(8, 2).pos),
        ((-2, -2), (-1, -2))
    );
    assert_eq!(run.logs(0), ["done at -3 pace 0"]);
    let run = bonk((-3, 0), (-1, 0), STAND, 8);
    assert_eq!(run.seen(8, 1).pos, (-1, 0));
    assert_eq!(run.logs(0), ["done at -2 pace 0"]);
}

#[test]
fn a_bonk_from_a_standstill_is_refused() {
    // Beside the other bot from the start, with no pace: the move is the
    // `ValueError` it always was, and nothing is heard of it.
    let main = "try:\n    move('e')\n    log('bonked')\nexcept ValueError:\n    log('refused')\nwait(1000)\n";
    let mut s = sim();
    let a = bot_at(&mut s, 0, (-3, -2));
    let b = bot_at(&mut s, 1, (-2, -2));
    let run = run_on(s, &[(0, main), (1, STAND)], &[a, b], 1);
    assert_eq!(run.logs(0), ["refused"]);
    let mut s = run.sim;
    // The printers began their prints this tick, and that is all.
    assert!(s.world.sounds_prev.iter().all(|snd| snd.cause == "print"));
    for _ in 0..5 {
        s.step(&[], &[]);
    }
    assert_eq!(s.world.machines[&a].pos, at((-3, -2)));
    assert_eq!(s.world.machines[&b].pos, at((-2, -2)));
    assert!(s.world.machines[&b].action.is_some(), "its wait goes on");
}

#[test]
fn a_building_is_never_bonked() {
    // West for a pace of one, then south onto team 0's printer: refused,
    // pace or no pace.
    let main = "move('w')\ntry:\n    move('s')\n    log('bonked')\nexcept ValueError:\n    log('refused with pace', me().pace)\nwait(1000)\n";
    let run = walk((-5, -2), main, 5);
    assert_eq!(run.logs(0), ["refused with pace 1"]);
    assert_eq!(run.seen(5, 0).pos, (-6, -2));
}

#[test]
fn a_bot_in_a_move_is_not_bonked() {
    // The other bot begins a move east in tick 3's slice; the runner,
    // which has the lower id, reaches for its tile in tick 4's. A bot in
    // a move is on two tiles and is no bot to bonk: the runner's move
    // fails, as onto any occupied tile.
    let main = "move('e')\ntry:\n    move('e')\n    log('bonked')\nexcept ValueError:\n    log('refused with pace', me().pace)\nwait(1000)\n";
    let leaving = "wait(2)\nmove('e')\nwait(1000)\n";
    let mut s = sim();
    let a = bot_at(&mut s, 0, (-4, -2));
    let b = bot_at(&mut s, 1, (-2, -2));
    let run = run_on(s, &[(0, main), (1, leaving)], &[a, b], 7);
    assert_eq!(run.logs(0), ["refused with pace 1"]);
    assert_eq!(run.seen(4, 1).pos, (-2, -2), "still on its way");
    assert_eq!(run.seen(7, 1).pos, (-1, -2));
    assert_eq!(run.seen(7, 0).pos, (-3, -2));
}
