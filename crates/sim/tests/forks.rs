//! Forks (`docs/06`, Forks), one rule each on a small map: the steps and
//! their stops, back and the saved states a fork may use, edits, what-if,
//! the three flags, and what the match's watches report. The hash stream a
//! fork must reproduce is `crates/replay/tests/forks.rs`'s.

use lang::{EditTarget, Num};
use sim::world::{EntityId, Model};
use sim::{
    Cause, Command, CommandKind, Data, Fork, Hit, Map, PlanKind, Position, Saved, Sim, Step,
    TeamId, TilePos, Timeline, Watch, bundle_of,
};
use std::collections::BTreeMap;

/// `machines.rs`'s map: team 0's printer at (-3, 0), team 1's at (3, 0).
const MAP: &str = r#"
name = "test"
speed = 10
width = 9
height = 7
terrain = """
.........
...##....
.o.......
.........
.....o...
.........
.........
"""
[deposit_default]
cap = 50
regrowth = 1
amount = 50
[[team]]
start = [-3, 0]
[[team]]
start = [3, 0]
"#;

/// The ticks a step runs with no other stop, in these tests.
const LIMIT: u64 = 60;

fn map_of(text: &str) -> Map {
    Map::parse(text, &Data::load().unwrap()).unwrap()
}

/// The match's log, as a test grows it.
#[derive(Clone)]
struct Log {
    commands: Vec<Command>,
    final_tick: u64,
}

impl Timeline for Log {
    fn commands(&self, tick: u64) -> Vec<Command> {
        let from = if tick == 1 { 0 } else { tick };
        let held = |c: &&Command| c.tick >= from && c.tick <= tick;
        self.commands.iter().filter(held).cloned().collect()
    }

    fn final_tick(&self) -> u64 {
        self.final_tick
    }
}

/// A log of `commands` whose every tick is final: a replay's.
fn log_of(commands: Vec<Command>) -> Log {
    Log {
        commands,
        final_tick: 1000,
    }
}

fn command(tick: u64, team: u32, kind: CommandKind) -> Command {
    Command::new(tick, TeamId(team), 0, kind)
}

fn deploy(tick: u64, team: u32, deployment: &str, main: &str) -> Command {
    let kind = CommandKind::Deploy {
        deployment: deployment.into(),
        bundle: bundle_of(main),
    };
    command(tick, team, kind)
}

/// Team 0's fork of a match on [`MAP`] whose printer runs `main` from
/// tick 1, and the match's log.
fn printer_fork(main: &str) -> (Fork, Log, EntityId) {
    let fork = Fork::from_map(map_of(MAP), TeamId(0));
    let id = machine(&fork, 0, Model::Printer);
    (fork, log_of(vec![deploy(0, 0, "printer", main)]), id)
}

fn machine(fork: &Fork, team: u32, model: Model) -> EntityId {
    let mut all = fork.world().machines.values();
    all.find(|m| m.team == TeamId(team) && m.model == model)
        .unwrap_or_else(|| panic!("team {team} has no {model:?}"))
        .id
}

/// A watch of team 0's on a line of a `main.py` holding `main`.
fn watch(main: &str, line: u32) -> Watch {
    Watch {
        team: TeamId(0),
        file: "main.py".into(),
        content: lang::content_hash(main.as_bytes()),
        line,
        deployment: None,
        machine: None,
    }
}

/// The line the machine's top frame stands on.
fn line(fork: &Fork, id: EntityId) -> u32 {
    let shown = fork.inspect(id, 80).expect("the team's machine");
    shown
        .program
        .expect("a program")
        .frames
        .last()
        .expect("a frame")
        .line
}

/// A global of the machine's `main`, as text.
fn global(fork: &Fork, id: EntityId, name: &str) -> Option<String> {
    let program = fork.inspect(id, 80)?.program?;
    let main = program.modules.into_iter().find(|m| m.name == "main")?;
    main.globals.into_iter().find(|g| g.0 == name).map(|g| g.1)
}

/// What the machine logged in the tick the fork stands in.
fn logged(fork: &Fork, id: EntityId) -> Vec<String> {
    let m = fork.world().machines.get(&id).expect("the machine");
    m.log.iter().map(|l| l.text.clone()).collect()
}

fn slice(tick: u64, machine: EntityId, n: u64) -> Position {
    Position::Slice { tick, machine, n }
}

fn end(tick: u64) -> Position {
    Position::End { tick }
}

/// Five statements, a two-tick wait between the third and the fourth.
const LINES: &str = "x = 1\ny = x + 1\nwait(2)\nlog(y)\nwait(1000)\n";

/// A call on line 4, then [`LINES`]'s wait.
const CALL: &str = "def f():\n    a = 1\n    return a\nx = f()\nwait(2)\nlog(x)\nwait(1000)\n";

/// A loop that never yields of its own accord.
const SPIN: &str = "x = 0\nwhile True:\n    x = x + 1\n";

/// Take `step` and give the stop's cause, tick and the selected machine's
/// line.
fn took(fork: &mut Fork, step: Step, id: EntityId, log: &Log) -> (Cause, u64, u32) {
    let stop = fork.step(step, Some(id), &[], LIMIT, log).unwrap();
    (stop.cause, stop.position.tick(), line(fork, id))
}

#[test]
fn a_fork_opens_stopped_as_opened_and_back_is_not_offered_at_a_first_stop() {
    let (mut fork, log, _) = printer_fork(LINES);
    assert_eq!(fork.position(), end(0));
    let [stop] = fork.stops() else {
        panic!("one stop: {:?}", fork.stops());
    };
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::Opened, end(0), None)
    );
    let refused = fork.back(&BTreeMap::new(), &log).unwrap_err();
    assert_eq!(refused, "there is no earlier stop");
    assert_eq!(fork.position(), end(0), "a refused back moves nothing");
    assert!(fork.flags(&log).faithful());
}

#[test]
fn line_stops_at_each_statement_the_selected_machine_begins() {
    let (mut fork, log, id) = printer_fork(LINES);
    // Tick 1's deploy restarts the printer's program: a stop of its own,
    // standing where line 1 is about to begin.
    let stop = fork.step(Step::Line, Some(id), &[], LIMIT, &log).unwrap();
    assert_eq!((stop.cause, stop.machine), (Cause::Restart, Some(id)));
    assert_eq!(stop.position, slice(1, id, 1));
    assert_eq!(line(&fork, id), 1);
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 2)
    );
    assert_eq!(global(&fork, id, "x").as_deref(), Some("1"));
    assert_eq!(global(&fork, id, "y"), None, "line 2 has not run");
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 3)
    );
    // The wait spans two ticks: every other machine runs meanwhile.
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 4)
    );
    assert_eq!(fork.position(), slice(3, id, 1));
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 5)
    );
    assert_eq!(logged(&fork, id), ["2"]);
    let causes: Vec<Cause> = fork.stops().iter().map(|s| s.cause).collect();
    assert_eq!(causes.first(), Some(&Cause::Opened));
    assert_eq!(causes.len(), 6, "every stop is kept: {causes:?}");
}

#[test]
fn a_fork_stopped_mid_tick_has_no_hash_and_saves_no_state() {
    let (mut fork, log, id) = printer_fork(LINES);
    fork.step(Step::Line, Some(id), &[], LIMIT, &log).unwrap();
    assert!(matches!(fork.position(), Position::Slice { .. }));
    assert_eq!(fork.state_hash(), None);
    assert!(fork.save().is_none());
    assert_eq!(
        fork.keep().unwrap_err(),
        "a fork saves its state at a tick's end"
    );
    let stop = fork.step(Step::Tick, Some(id), &[], LIMIT, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::TickEnd, end(1)));
    // The tick completed: the fork hashes as the match does there.
    let mut sim = Sim::new(map_of(MAP));
    sim.step(&log.commands(1), &[]);
    assert_eq!(fork.state_hash(), Some(sim.state_hash()));
    assert_eq!(fork.keep(), Ok(1));
    assert_eq!(fork.kept(), [1]);
    fork.forget(1);
    assert!(fork.kept().is_empty());
}

#[test]
fn line_and_over_need_a_selected_machine_of_the_team_that_runs_a_program() {
    let (mut fork, log, _) = printer_fork(LINES);
    let rival = machine(&fork, 1, Model::Printer);
    let message = "select one of your machines that runs a program to step it";
    for step in [Step::Line, Step::Over] {
        for selected in [None, Some(rival), Some(EntityId(99))] {
            let refused = fork.step(step, selected, &[], LIMIT, &log).unwrap_err();
            assert_eq!(refused, message);
        }
    }
    assert_eq!(fork.position(), end(0), "a refused step runs nothing");
    // Tick and continue need none.
    let stop = fork.step(Step::Tick, None, &[], LIMIT, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::TickEnd, end(1)));
    let stop = fork
        .step(Step::Continue, Some(rival), &[], 2, &log)
        .unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::StepLimit, end(3)));
}

#[test]
fn over_takes_a_call_and_a_wait_as_one_step() {
    let (mut fork, log, id) = printer_fork(CALL);
    assert_eq!(
        took(&mut fork, Step::Over, id, &log),
        (Cause::Restart, 1, 1)
    );
    assert_eq!(
        took(&mut fork, Step::Over, id, &log),
        (Cause::Statement, 1, 4)
    );
    // Over the call: the next statement of this frame, the call run.
    assert_eq!(
        took(&mut fork, Step::Over, id, &log),
        (Cause::Statement, 1, 5)
    );
    assert_eq!(global(&fork, id, "x").as_deref(), Some("1"));
    // Over the wait: one step, two ticks on.
    assert_eq!(
        took(&mut fork, Step::Over, id, &log),
        (Cause::Statement, 3, 6)
    );

    // Line goes into the call, and over inside it comes back out to the
    // nearest frame still on the stack once that frame returns.
    let (mut fork, log, id) = printer_fork(CALL);
    took(&mut fork, Step::Line, id, &log);
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 4)
    );
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 2)
    );
    let frames = fork.inspect(id, 80).unwrap().program.unwrap().frames;
    assert_eq!(frames.len(), 2);
    assert_eq!((frames[0].line, frames[1].function.as_str()), (4, "f"));
    assert_eq!(
        took(&mut fork, Step::Over, id, &log),
        (Cause::Statement, 1, 3)
    );
    assert_eq!(
        took(&mut fork, Step::Over, id, &log),
        (Cause::Statement, 1, 5)
    );
}

/// What `sim.step` reports of [`SPIN`] on team 0's printer over `ticks`
/// ticks under `watches`, and the hash it ends on.
fn watched(watches: &[Watch], ticks: u64) -> (Vec<Hit>, u64) {
    let log = log_of(vec![deploy(0, 0, "printer", SPIN)]);
    let mut sim = Sim::new(map_of(MAP));
    let mut hits = Vec::new();
    for tick in 1..=ticks {
        hits.extend(sim.step(&log.commands(tick), watches).hits);
    }
    (hits, sim.state_hash())
}

#[test]
fn a_watch_reports_a_machines_first_beginning_each_tick_and_a_count() {
    let id = printer_fork(SPIN).2;
    let (hits, hash) = watched(&[watch(SPIN, 3)], 2);
    let [first, second] = hits.as_slice() else {
        panic!("one hit a tick: {hits:?}");
    };
    assert_eq!(
        (first.machine, first.file.as_str(), first.line),
        (id, "main.py", 3)
    );
    assert_eq!(first.position, slice(1, id, 7));
    assert!(first.more > 0, "the loop came round again: {first:?}");
    assert_eq!(second.position.tick(), 2);
    // A line that begins once is one hit, with nothing more.
    let (hits, _) = watched(&[watch(SPIN, 1)], 2);
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].position.tick(), hits[0].more), (1, 0));
    // Watching charges nothing and changes nothing.
    assert_eq!(watched(&[], 2), (Vec::new(), hash));
}

#[test]
fn a_watch_hits_only_its_teams_text_deployment_and_machine() {
    let id = printer_fork(SPIN).2;
    let hit = |change: &dyn Fn(&mut Watch)| {
        let mut w = watch(SPIN, 3);
        change(&mut w);
        !watched(&[w], 1).0.is_empty()
    };
    assert!(hit(&|_| ()));
    assert!(hit(&|w| w.deployment = Some("printer".into())));
    assert!(hit(&|w| w.machine = Some(id)));
    assert!(!hit(&|w| w.team = TeamId(1)), "another team's watch");
    assert!(!hit(&|w| w.deployment = Some("bot".into())));
    assert!(!hit(&|w| w.machine = Some(EntityId(99))));
    assert!(!hit(&|w| w.file = "lib.py".into()));
    // A watch set on other text is not this file's, whatever its line.
    assert!(!hit(&|w| w.content = lang::content_hash(b"x = 0\n")));
}

/// [`SPIN`] with a `pass` before the assignment. A boundary is a yield
/// only where the operation before it was charged for, and `pass`'s is:
/// seven units a pass against a budget of 200, the budget runs out
/// standing exactly before line 4 once in every seven ticks.
const PASS: &str = "x = 0\nwhile True:\n    pass\n    x = x + 1\n";

/// `docs/06`, Statements: reaching a statement's boundary and yielding
/// there begins nothing.
#[test]
fn a_yield_before_a_watched_statement_is_no_hit() {
    let (mut fork, log, id) = printer_fork(PASS);
    let watches = [watch(PASS, 4)];
    let x = |fork: &Fork| -> u64 { global(fork, id, "x").map_or(0, |x| x.parse().unwrap()) };
    let mut begun = 0u64;
    let mut yielded_before = 0u32;
    for _ in 0..16 {
        let (x_before, begun_before) = (x(&fork), begun);
        let report = fork.advance(&watches, &log);
        let [hit] = report.hits.as_slice() else {
            panic!("one machine, one hit: {:?}", report.hits);
        };
        begun = begun.saturating_add(hit.more).saturating_add(1);
        // A hit at the slice's opening boundary: the tick before ended
        // with the machine standing before line 4, out of budget, every
        // beginning of it until then run.
        if matches!(hit.position, Position::Slice { n: 0, .. }) {
            yielded_before = yielded_before.saturating_add(1);
            assert_eq!(
                begun_before, x_before,
                "a yield before line 4 was counted as a beginning of it"
            );
        }
    }
    assert!(yielded_before > 0, "the scenario never came up");
    // Every beginning ran its statement but, at most, the last.
    let x = x(&fork);
    assert!(
        begun == x || begun == x.saturating_add(1),
        "{begun} beginnings, x = {x}"
    );
}

#[test]
fn tick_reports_watched_beginnings_as_sim_step_does_and_stops_at_none() {
    let (mut fork, log, id) = printer_fork(SPIN);
    let watches = [watch(SPIN, 3)];
    let (hits, hash) = watched(&watches, 1);
    let stop = fork
        .step(Step::Tick, Some(id), &watches, LIMIT, &log)
        .unwrap();
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::TickEnd, end(1), None)
    );
    assert_eq!(stop.hits, hits);
    assert_eq!(fork.state_hash(), Some(hash));
}

#[test]
fn continue_stops_at_every_watched_beginning_and_selects_another_machine() {
    let bot = "log('bot')\nwait(1000)\n";
    let (mut fork, mut log, id) = printer_fork("wait(1000)\n");
    log.commands.push(deploy(0, 0, "bot", bot));
    log.commands[1].seq = 1;
    let watches = [watch(bot, 1), watch(SPIN, 3)];
    let stop = fork
        .step(Step::Continue, Some(id), &watches, LIMIT, &log)
        .unwrap();
    assert_eq!(stop.cause, Cause::Restart, "the selected printer's deploy");
    // The printer waits; the first bot it prints begins line 1.
    let stop = fork
        .step(Step::Continue, Some(id), &watches, LIMIT, &log)
        .unwrap();
    let printed = machine(&fork, 0, Model::Bot);
    assert_eq!((stop.cause, stop.machine), (Cause::Watch, Some(printed)));
    assert_ne!(printed, id);
    let [hit] = stop.hits.as_slice() else {
        panic!("a watch stop carries its one hit: {:?}", stop.hits);
    };
    assert_eq!((hit.machine, hit.line, hit.more), (printed, 1, 0));
    assert_eq!(
        (hit.position, fork.position()),
        (stop.position, stop.position)
    );
    assert_eq!(line(&fork, printed), 1);
    assert!(
        logged(&fork, printed).is_empty(),
        "stopped before the line ran"
    );

    // Every beginning, not only a tick's first: three stops in one tick.
    let (mut fork, log, id) = printer_fork(SPIN);
    let mut at = Vec::new();
    for _ in 0..3 {
        let stop = fork
            .step(Step::Continue, None, &watches, LIMIT, &log)
            .unwrap();
        assert_eq!((stop.cause, stop.machine), (Cause::Watch, Some(id)));
        at.push(stop.position);
    }
    assert!(
        at.iter().all(|p| p.tick() == 1) && at[0] < at[1] && at[1] < at[2],
        "{at:?}"
    );
    assert_eq!(global(&fork, id, "x").as_deref(), Some("2"));
}

#[test]
fn run_goes_a_tick_at_a_time_until_it_is_paused_or_continue_would_stop() {
    let (mut fork, log, id) = printer_fork(SPIN);
    for tick in 1..=80 {
        assert_eq!(fork.run(None, &[], &log), None, "run has no step limit");
        assert_eq!(fork.position(), end(tick));
    }
    let stop = fork.pause(&log);
    assert_eq!((stop.cause, stop.position), (Cause::Paused, end(80)));
    // A watch stops it mid-tick, and a pause there finishes the tick.
    let stop = fork.run(None, &[watch(SPIN, 3)], &log).expect("a stop");
    assert_eq!((stop.cause, stop.machine), (Cause::Watch, Some(id)));
    assert_eq!(stop.position.tick(), 81);
    assert_eq!(fork.state_hash(), None);
    let stop = fork.pause(&log);
    assert_eq!((stop.cause, stop.position), (Cause::Paused, end(81)));
    assert!(fork.state_hash().is_some());
}

#[test]
fn a_step_with_no_other_stop_stops_at_its_limit_of_ticks() {
    let (mut fork, log, _) = printer_fork(SPIN);
    let stop = fork.step(Step::Continue, None, &[], 3, &log).unwrap();
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::StepLimit, end(3), None)
    );
    // No limit at all would be a step that never returns: zero is one.
    let stop = fork.step(Step::Continue, None, &[], 0, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::StepLimit, end(4)));
}

#[test]
fn every_step_stops_at_the_matchs_end_and_none_goes_past_it() {
    let (mut fork, mut log, id) = printer_fork(SPIN);
    log.commands.push(command(3, 1, CommandKind::Resign));
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!(stop.cause, Cause::Restart);
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::MatchEnded, end(3), None)
    );
    assert_eq!(fork.ended(), Some(Some(TeamId(0))));
    let stops = fork.stops().len();
    for step in [Step::Line, Step::Over, Step::Tick, Step::Continue] {
        let stop = fork.step(step, Some(id), &[], LIMIT, &log).unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::MatchEnded, end(3)));
    }
    let stop = fork.run(Some(id), &[], &log).expect("a stop");
    assert_eq!((stop.cause, stop.position), (Cause::MatchEnded, end(3)));
    assert_eq!(fork.pause(&log).cause, Cause::MatchEnded);
    assert_eq!(
        fork.stops().len(),
        stops,
        "a fork that goes nowhere stops nowhere new"
    );

    // A tick step, and a run, that reach the end say so there.
    let mut fork = Fork::from_map(map_of(MAP), TeamId(0));
    tick(&mut fork, &log);
    tick(&mut fork, &log);
    let stop = fork.step(Step::Tick, None, &[], LIMIT, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::MatchEnded, end(3)));
    let mut fork = Fork::from_map(map_of(MAP), TeamId(0));
    assert_eq!(fork.run(None, &[], &log), None);
    assert_eq!(fork.run(None, &[], &log), None);
    let stop = fork.run(None, &[], &log).expect("the match's end");
    assert_eq!((stop.cause, stop.position), (Cause::MatchEnded, end(3)));
}

/// A hook that outlives its tick, so `dying` and `death` are two stops.
const HOOK: &str = "x = 1\ndef on_dying():\n    log('going', x)\n    wait(2)\nwait(1000)\n";

/// A match one tick in with a team-0 bot running `bot` beside team 1's
/// printer, by fiat, at `health`: the printer's defence takes one a tick.
fn besieged(bot: &str, health: i128) -> (Saved, Log, EntityId) {
    let log = log_of(vec![deploy(0, 0, "bot", bot)]);
    let mut sim = Sim::new(map_of(MAP));
    sim.step(&log.commands(1), &[]);
    let at = TilePos::new(2, 0);
    let id = sim
        .world
        .place(Model::Bot, TeamId(0), at, Some("bot".into()));
    sim.world.machines.get_mut(&id).unwrap().health = Num::from_int(health).unwrap();
    (sim.save(), log, id)
}

fn x_of_main() -> EditTarget {
    EditTarget::Global {
        module: "main".into(),
        name: "x".into(),
    }
}

#[test]
fn a_step_stops_at_the_selected_machines_dying_and_at_its_death_in_its_slice() {
    let (saved, log, id) = besieged(HOOK, 1);
    let mut fork = Fork::open(&saved, TeamId(0), false);
    assert_eq!(fork.stops()[0].position, end(1));
    // Tick 2's damage raises `dying`; tick 3's slice is about to deliver it.
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!((stop.cause, stop.machine), (Cause::Dying, Some(id)));
    assert_eq!(
        stop.position,
        slice(3, id, 0),
        "the slice's opening boundary"
    );
    // A machine at `dying` may be edited, and its hook reads the edit.
    fork.edit(id, x_of_main(), "5").unwrap();
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 3)
    );
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 4)
    );
    assert_eq!(logged(&fork, id), ["going 5"]);
    // The hook ends: `death`, inside the machine's slice.
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!((stop.cause, stop.machine), (Cause::Death, Some(id)));
    assert!(matches!(stop.position, Position::Slice { machine, .. } if machine == id));
    let refused = fork.edit(id, x_of_main(), "6").unwrap_err();
    assert_eq!(refused, "the machine is at its death");
    assert_eq!(fork.can_edit(id).unwrap_err(), refused);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("5"));
    // The step after has no machine to follow past the tick.
    let tick = stop.position.tick();
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::TickEnd, end(tick), None)
    );
    assert!(!fork.world().machines.contains_key(&id));
}

#[test]
fn a_step_stops_at_a_death_in_step_4_and_nothing_is_changed_there() {
    // One point of damage takes a bot at -4 to its death threshold.
    let (saved, log, id) = besieged(HOOK, -4);
    let mut fork = Fork::open(&saved, TeamId(0), false);
    let stop = fork.step(Step::Line, Some(id), &[], LIMIT, &log).unwrap();
    assert_eq!((stop.cause, stop.position.tick()), (Cause::Statement, 2));
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    let damage = Position::Damage {
        tick: 2,
        machine: id,
    };
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::Death, damage, Some(id))
    );
    assert_eq!(fork.position(), damage);
    // After the damage, before the removal.
    let shown = fork.inspect(id, 80).expect("the machine still stands");
    assert_eq!(shown.record.health, Num::from_int(-5).unwrap());
    assert_eq!(fork.state_hash(), None);
    let message = "nothing is changed at a stop in step 4";
    assert_eq!(fork.edit(id, x_of_main(), "6").unwrap_err(), message);
    assert_eq!(fork.can_edit(id).unwrap_err(), message);
    let tried = bundle_of("wait(1)\n");
    assert_eq!(fork.what_if("bot", &tried).unwrap_err(), message);
    assert_eq!(fork.can_what_if().unwrap_err(), message);
    assert_eq!(fork.check_what_if("bot", &tried).unwrap_err(), message);
    assert!(!fork.flags(&log).diverged);
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::TickEnd, end(2)));
    assert!(!fork.world().machines.contains_key(&id));
    // A slice position sorts before a damage position, and that before
    // the tick's end.
    assert!(slice(2, EntityId(99), 50) < damage && damage < end(2) && end(2) < slice(3, id, 0));
}

#[test]
fn a_step_stops_at_the_ticks_end_when_step_5_removes_the_machine_with_its_team() {
    // A third team, so the match outlives team 0.
    let three = format!("{MAP}[[team]]\nstart = [0, -2]\n");
    let mut fork = Fork::from_map(map_of(&three), TeamId(0));
    let id = machine(&fork, 0, Model::Printer);
    let mut log = log_of(vec![deploy(0, 0, "printer", SPIN)]);
    log.commands.push(command(3, 0, CommandKind::Resign));
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    assert_eq!(stop.cause, Cause::Restart);
    let stop = fork
        .step(Step::Continue, Some(id), &[], LIMIT, &log)
        .unwrap();
    // The machine is named though gone.
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::TeamOut, end(3), Some(id))
    );
    assert!(!fork.world().machines.contains_key(&id));
    assert_eq!(fork.ended(), None, "two teams play on");
    assert!(fork.step(Step::Line, Some(id), &[], LIMIT, &log).is_err());
    let stop = fork.step(Step::Continue, Some(id), &[], 2, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::StepLimit, end(5)));
}

/// A wait between binding `x` and reading it.
const EDIT: &str = "x = 1\nwait(2)\nlog(x)\nwait(1000)\n";

#[test]
fn a_re_run_across_an_edit_applies_it_to_its_machine() {
    let (mut fork, log, id) = printer_fork(EDIT);
    took(&mut fork, Step::Line, id, &log);
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 2)
    );
    let made_at = fork.position();
    assert!(!fork.flags(&log).diverged);
    fork.edit(id, x_of_main(), "7").unwrap();
    assert!(fork.flags(&log).diverged);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("7"));
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 3)
    );
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 4)
    );
    assert_eq!(logged(&fork, id), ["7"]);

    // Back, with no saved state: from the map, across the edit.
    let stop = fork.back(&BTreeMap::new(), &log).unwrap();
    assert_eq!(
        (stop.cause, stop.position),
        (Cause::Statement, slice(3, id, 1))
    );
    assert_eq!(fork.position(), stop.position);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("7"));
    assert!(logged(&fork, id).is_empty(), "line 3 has not run again yet");
    // Back to the edit's own position: the world there holds it.
    assert_eq!(fork.back(&BTreeMap::new(), &log).unwrap().position, made_at);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("7"));
    // And before it, it does not.
    assert_eq!(
        fork.back(&BTreeMap::new(), &log).unwrap().cause,
        Cause::Restart
    );
    assert_eq!(global(&fork, id, "x"), None);
    // Forward again, the edit is applied where it was made.
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 2)
    );
    assert_eq!(global(&fork, id, "x").as_deref(), Some("7"));
    took(&mut fork, Step::Line, id, &log);
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 3, 4)
    );
    assert_eq!(logged(&fork, id), ["7"]);
}

#[test]
fn an_edit_sets_a_local_by_its_frames_depth_and_a_refusal_changes_nothing() {
    let (mut fork, log, id) = printer_fork(CALL);
    for _ in 0..4 {
        took(&mut fork, Step::Line, id, &log);
    }
    assert_eq!(line(&fork, id), 3, "about to return `a`");
    let a = |depth| EditTarget::Local {
        depth,
        name: "a".into(),
    };
    let rival = machine(&fork, 1, Model::Printer);
    assert_eq!(
        fork.edit(rival, a(1), "9").unwrap_err(),
        "no such machine of this team"
    );
    assert_eq!(
        fork.edit(EntityId(99), a(1), "9").unwrap_err(),
        "no such machine of this team"
    );
    assert!(
        fork.edit(id, a(0), "9").is_err(),
        "main flow's frame has no `a`"
    );
    assert!(fork.edit(id, a(1), "f()").is_err(), "a call is no literal");
    assert!(!fork.flags(&log).diverged, "a refused edit enters no log");
    fork.edit(id, a(1), "[9, 'nine']").unwrap();
    assert_eq!(
        took(&mut fork, Step::Line, id, &log),
        (Cause::Statement, 1, 5)
    );
    assert_eq!(global(&fork, id, "x").as_deref(), Some("[9, 'nine']"));
}

/// Logs `x` once a tick.
const EACH_TICK: &str = "x = 1\nwhile True:\n    log(x)\n    wait(1)\n";

fn tick(fork: &mut Fork, log: &Log) -> Position {
    fork.step(Step::Tick, None, &[], LIMIT, log)
        .unwrap()
        .position
}

#[test]
fn a_new_branch_discards_the_later_entries_and_the_saved_states_after_it() {
    let (mut fork, log, id) = printer_fork(EACH_TICK);
    tick(&mut fork, &log);
    assert_eq!(tick(&mut fork, &log), end(2));
    assert_eq!(fork.keep(), Ok(2));
    fork.edit(id, x_of_main(), "5").unwrap();
    tick(&mut fork, &log);
    assert_eq!(tick(&mut fork, &log), end(4));
    assert_eq!(fork.keep(), Ok(4));
    fork.edit(id, x_of_main(), "9").unwrap();
    tick(&mut fork, &log);
    assert_eq!(tick(&mut fork, &log), end(6));
    assert_eq!(logged(&fork, id), ["9"]);
    assert_eq!(fork.kept(), [2, 4]);

    // Back to tick 3's end, between the two edits, from the state kept at
    // tick 2 — which held neither.
    for _ in 0..3 {
        fork.back(&BTreeMap::new(), &log).unwrap();
    }
    assert_eq!(fork.position(), end(3));
    assert_eq!(global(&fork, id, "x").as_deref(), Some("5"));
    assert_eq!(fork.kept(), [2, 4], "back discards nothing");
    // An edit here is a new branch: the edit at tick 4 and the state kept
    // there are gone, and the one kept before stays.
    fork.edit(id, x_of_main(), "7").unwrap();
    assert_eq!(fork.kept(), [2]);
    for _ in 0..3 {
        tick(&mut fork, &log);
    }
    assert_eq!(fork.position(), end(6));
    assert_eq!(logged(&fork, id), ["7"], "the discarded edit never applies");
    // Seeking back and forth across both: still one timeline.
    fork.seek(2, &log);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("5"));
    fork.seek(4, &log);
    assert_eq!(logged(&fork, id), ["7"], "the new edit, where it was made");
    fork.seek(6, &log);
    assert_eq!(logged(&fork, id), ["7"]);
}

#[test]
fn a_fork_told_to_keep_every_few_ticks_saves_them_on_any_step_and_keeps_the_last() {
    // A fork never told saves only when asked.
    let (mut plain, log, _) = printer_fork(EACH_TICK);
    plain.step(Step::Continue, None, &[], 9, &log).unwrap();
    assert_eq!(plain.position(), end(9));
    assert!(plain.states().is_empty());

    let (mut fork, log, id) = printer_fork(EACH_TICK);
    fork.keep_every(2, 3);
    tick(&mut fork, &log);
    assert_eq!(fork.keep(), Ok(1));
    // One step through eight tick ends: it saves at each multiple of two
    // on its way, not only where it stops, and the oldest go, the one
    // kept by hand first.
    let stop = fork.step(Step::Continue, None, &[], 8, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::StepLimit, end(9)));
    assert_eq!(fork.kept(), [4, 6, 8]);
    let ticks: Vec<u64> = fork.states().values().map(Saved::tick).collect();
    assert_eq!(ticks, [4, 6, 8]);
    // A state saved on the way is the fork's world there: restored, it
    // hashes as a fork that ran there does.
    let hash = fork.state_hash();
    fork.seek(8, &log);
    plain.seek(8, &log);
    assert_eq!(fork.state_hash(), plain.state_hash());
    fork.seek(9, &log);
    assert_eq!(fork.state_hash(), hash);

    // A state saved after an edit holds the edit, and counts it: a run on
    // from it applies the edit no second time and loses it no more.
    fork.edit(id, x_of_main(), "5").unwrap();
    assert_eq!(fork.kept(), [4, 6, 8], "none is after the edit");
    fork.step(Step::Continue, None, &[], 4, &log).unwrap();
    assert_eq!(fork.kept(), [8, 10, 12]);
    fork.seek(12, &log);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("5"));
    fork.seek(13, &log);
    assert_eq!(logged(&fork, id), ["5"]);
}

/// The state of tick `tick` of `log`, with a site of team 1's put in it by
/// fiat: a state the match never reached, so a fork restored from it
/// shows the site and one that re-ran from the map does not.
fn marked_state(log: &Log, tick: u64) -> BTreeMap<u64, Saved> {
    let mut sim = Sim::new(map_of(MAP));
    for t in 1..=tick {
        sim.step(&log.commands(t), &[]);
    }
    sim.world
        .place(Model::Site, TeamId(1), TilePos::new(3, -2), None);
    BTreeMap::from([(tick, sim.save())])
}

fn marked(fork: &Fork) -> bool {
    let mut all = fork.world().machines.values();
    all.any(|m| m.model == Model::Site)
}

/// A plan on a tile: what a `Mark` leaves in the world, and in its hash.
fn mark(tick: u64) -> Command {
    let kind = CommandKind::Mark {
        at: TilePos::new(0, 0),
        plan: PlanKind::Building,
        value: Some("depot".into()),
    };
    command(tick, 0, kind)
}

#[test]
fn back_uses_a_state_the_match_saved_only_before_the_forks_first_entry() {
    // Faithful, with no log: the match's state at tick 2 will do.
    let (mut fork, log, id) = printer_fork(EACH_TICK);
    let states = marked_state(&log, 2);
    for _ in 0..4 {
        tick(&mut fork, &log);
    }
    assert_eq!(fork.back(&states, &log).unwrap().position, end(3));
    assert!(
        marked(&fork),
        "restored from the match's state, then run on"
    );

    // An entry at tick 3's end is after the state: still usable, and the
    // run from it applies the entry.
    let mut fork = printer_fork(EACH_TICK).0;
    for _ in 0..3 {
        tick(&mut fork, &log);
    }
    fork.edit(id, x_of_main(), "5").unwrap();
    tick(&mut fork, &log);
    assert_eq!(fork.back(&states, &log).unwrap().position, end(3));
    assert!(marked(&fork));
    assert_eq!(global(&fork, id, "x").as_deref(), Some("5"));

    // An entry at tick 1's end is before it: the state holds a world the
    // fork never had, and the fork re-runs from the map.
    let mut fork = printer_fork(EACH_TICK).0;
    tick(&mut fork, &log);
    fork.edit(id, x_of_main(), "5").unwrap();
    for _ in 0..3 {
        tick(&mut fork, &log);
    }
    assert_eq!(fork.back(&states, &log).unwrap().position, end(3));
    assert!(!marked(&fork));
    assert_eq!(global(&fork, id, "x").as_deref(), Some("5"));
}

#[test]
fn back_uses_no_state_the_match_saved_at_or_after_the_tick_a_fork_went_stale() {
    let (mut fork, mut log, _) = printer_fork(EACH_TICK);
    log.final_tick = 1;
    for _ in 0..4 {
        tick(&mut fork, &log);
    }
    log.commands.push(mark(2));
    assert_eq!(fork.flags(&log).stale, Some(2));
    // The match's tick 2 took a command this fork did not.
    assert_eq!(
        fork.back(&marked_state(&log, 2), &log).unwrap().position,
        end(3)
    );
    assert!(!marked(&fork));
    // Its tick 1 is the fork's own.
    assert_eq!(
        fork.back(&marked_state(&log, 1), &log).unwrap().position,
        end(2)
    );
    assert!(marked(&fork));
}

#[test]
fn a_fork_run_ahead_goes_stale_when_the_matchs_log_gains_a_mark_but_not_a_set_speed() {
    let (mut fork, mut log, _) = printer_fork(SPIN);
    log.final_tick = 2;
    assert!(fork.flags(&log).faithful());
    let stop = fork.step(Step::Continue, None, &[], 6, &log).unwrap();
    assert_eq!(stop.position, end(6));
    let flags = fork.flags(&log);
    assert_eq!(
        (flags.ahead, flags.stale, flags.diverged),
        (true, None, false)
    );
    assert!(!flags.faithful());
    // `SetSpeed` changes no world.
    log.commands.push(command(4, 0, CommandKind::SetSpeed(20)));
    assert_eq!(fork.flags(&log).stale, None);
    log.commands.push(mark(5));
    assert_eq!(fork.flags(&log).stale, Some(5));
    // Stale from the first such tick, and for good: the match catching up
    // ends *ahead* and nothing else.
    log.commands.push(mark(4));
    assert_eq!(fork.flags(&log).stale, Some(4));
    log.final_tick = 9;
    let flags = fork.flags(&log);
    assert_eq!((flags.ahead, flags.stale), (false, Some(4)));

    // A fork the match catches up with, nothing new agreed, is faithful.
    let (mut fork, mut log, _) = printer_fork(SPIN);
    log.final_tick = 2;
    fork.step(Step::Continue, None, &[], 6, &log).unwrap();
    log.commands.push(command(4, 0, CommandKind::SetSpeed(20)));
    log.final_tick = 6;
    assert!(fork.flags(&log).faithful());
    // And a tick that is final cannot stale one: its set was complete.
    let (mut fork, mut log, _) = printer_fork(SPIN);
    log.commands.push(mark(4));
    fork.step(Step::Continue, None, &[], 6, &log).unwrap();
    assert!(fork.flags(&log).faithful());
    assert!(fork.world().plans(TeamId(0), TilePos::new(0, 0)).is_some());
}

#[test]
fn a_stale_forks_re_run_takes_none_of_the_commands_that_staled_it() {
    let (mut fork, mut log, _) = printer_fork(SPIN);
    log.final_tick = 2;
    fork.step(Step::Continue, None, &[], 6, &log).unwrap();
    let ran = fork.state_hash();
    log.commands.push(mark(4));
    assert_eq!(fork.flags(&log).stale, Some(4));
    // The command changes the world: a fork opened now takes it.
    let mut fresh = Fork::from_map(map_of(MAP), TeamId(0));
    fresh.seek(6, &log);
    assert!(fresh.world().plans(TeamId(0), TilePos::new(0, 0)).is_some());
    assert_ne!(fresh.state_hash(), ran);
    // Back to the opening and forward again: from the map, the match's
    // log before the fork's first tick and its own record after.
    assert_eq!(fork.back(&BTreeMap::new(), &log).unwrap().position, end(0));
    let stop = fork.step(Step::Continue, None, &[], 6, &log).unwrap();
    assert_eq!(stop.position, end(6));
    assert!(fork.world().plans(TeamId(0), TilePos::new(0, 0)).is_none());
    assert_eq!(fork.state_hash(), ran);
    fork.seek(3, &log);
    fork.seek(6, &log);
    assert_eq!(fork.state_hash(), ran);
    // Past what it recorded it reads the log again, as any fork does.
    log.commands.push(mark(7));
    assert_eq!(tick(&mut fork, &log), end(7));
    assert!(fork.world().plans(TeamId(0), TilePos::new(0, 0)).is_some());
}

/// A match three ticks in whose latest completed tick is 5, saved.
fn live() -> (Saved, Log, EntityId) {
    let (fork, mut log, id) = printer_fork(EACH_TICK);
    drop(fork);
    log.final_tick = 5;
    let mut sim = Sim::new(map_of(MAP));
    for t in 1..=3 {
        sim.step(&log.commands(t), &[]);
    }
    (sim.save(), log, id)
}

#[test]
fn a_fork_of_a_live_pvp_match_stops_at_the_live_tick_and_takes_no_entry() {
    let (saved, mut log, id) = live();
    let mut fork = Fork::open(&saved, TeamId(0), true);
    assert_eq!(fork.position(), end(3));
    let stop = fork.step(Step::Continue, None, &[], LIMIT, &log).unwrap();
    assert_eq!(
        (stop.cause, stop.position, stop.machine),
        (Cause::LiveTick, end(5), None)
    );
    let message = "a fork of a live PvP match takes no edit and no what-if";
    assert_eq!(fork.edit(id, x_of_main(), "5").unwrap_err(), message);
    assert_eq!(fork.can_edit(id).unwrap_err(), message);
    let tried = bundle_of("wait(1)\n");
    assert_eq!(fork.what_if("printer", &tried).unwrap_err(), message);
    assert_eq!(fork.can_what_if().unwrap_err(), message);
    assert_eq!(fork.check_what_if("printer", &tried).unwrap_err(), message);
    assert_eq!(global(&fork, id, "x").as_deref(), Some("1"));
    assert!(fork.flags(&log).faithful(), "never ahead, never diverged");
    // It goes no further, by any step, and stops nowhere new.
    let stops = fork.stops().len();
    for step in [Step::Line, Step::Over, Step::Tick, Step::Continue] {
        let stop = fork.step(step, Some(id), &[], LIMIT, &log).unwrap();
        assert_eq!((stop.cause, stop.position), (Cause::LiveTick, end(5)));
    }
    assert_eq!(
        fork.run(None, &[], &log).map(|s| s.cause),
        Some(Cause::LiveTick)
    );
    assert_eq!(fork.stops().len(), stops);
    // The match completes a tick: the fork may follow it there.
    log.final_tick = 6;
    assert_eq!(tick(&mut fork, &log), end(6));
    let stop = fork.step(Step::Tick, None, &[], LIMIT, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::LiveTick, end(6)));

    // Any other fork of the same state runs past the match and diverges.
    let mut fork = Fork::open(&saved, TeamId(0), false);
    let stop = fork.step(Step::Continue, None, &[], 4, &log).unwrap();
    assert_eq!((stop.cause, stop.position), (Cause::StepLimit, end(7)));
    fork.edit(id, x_of_main(), "5").unwrap();
    let flags = fork.flags(&log);
    assert_eq!(
        (flags.ahead, flags.stale, flags.diverged),
        (true, None, true)
    );
}

#[test]
fn a_what_if_deploy_is_checked_as_a_submission_is_and_applies_next_tick() {
    let (mut fork, mut log, id) = printer_fork("wait(1000)\n");
    log.commands
        .push(deploy(5, 0, "printer", "log('theirs')\nwait(1000)\n"));
    tick(&mut fork, &log);
    tick(&mut fork, &log);
    let tried = bundle_of("log('tried')\nwait(1000)\n");
    assert_eq!(fork.can_what_if(), Ok(()));
    let refused = fork.what_if("nope", &tried).unwrap_err();
    assert!(refused.contains("no deployment `nope`"), "{refused}");
    assert_eq!(fork.check_what_if("nope", &tried).unwrap_err(), refused);
    // A bundle that fails to load is refused naming the error.
    let broken = bundle_of("x = (\n");
    let refused = fork.what_if("printer", &broken).unwrap_err();
    assert!(refused.contains("main.py"), "{refused}");
    assert_eq!(fork.check_what_if("printer", &broken).unwrap_err(), refused);
    assert!(
        !fork.flags(&log).diverged,
        "a refused what-if enters no log"
    );
    // A check sends nothing.
    let before = fork.deployed("printer");
    assert!(before.is_some() && fork.deployed("depot").is_none());
    assert_eq!(fork.deployed("nope"), None);
    assert_eq!(fork.check_what_if("printer", &tried), Ok(()));
    assert!(!fork.flags(&log).diverged, "a check enters no log");
    fork.what_if("printer", &tried).unwrap();
    assert!(fork.flags(&log).diverged);
    assert!(logged(&fork, id).is_empty(), "agreed for the next tick");
    assert_eq!(fork.deployed("printer"), before, "not before its tick");
    // The tick's step 1 puts it in the slot: a step into the tick finds
    // it there, while the snapshot drawn there is still the last tick's.
    let stop = fork.step(Step::Line, Some(id), &[], LIMIT, &log).unwrap();
    assert!(matches!(stop.position, Position::Slice { tick: 3, .. }));
    let version = fork.deployed("printer");
    assert!(version.is_some() && version != before);
    let own = |s: &sim::Snapshot| s.teams[0].deployments["printer"];
    assert_eq!(own(&fork.snapshot()), before);
    assert_eq!(tick(&mut fork, &log), end(3));
    assert_eq!(own(&fork.snapshot()), version);
    assert_eq!(logged(&fork, id), ["tried"]);
    // The match's later deploy to the deployment replaces the tried one.
    tick(&mut fork, &log);
    assert_eq!(tick(&mut fork, &log), end(5));
    assert_eq!(logged(&fork, id), ["theirs"]);
    // A re-run applies it on the same tick.
    fork.seek(2, &log);
    assert!(logged(&fork, id).is_empty());
    fork.seek(3, &log);
    assert_eq!(logged(&fork, id), ["tried"]);

    // In the match's own tick it goes after the match's commands.
    let (mut fork, mut log, id) = printer_fork("wait(1000)\n");
    log.commands
        .push(deploy(3, 0, "printer", "log('theirs')\nwait(1000)\n"));
    tick(&mut fork, &log);
    tick(&mut fork, &log);
    fork.what_if("printer", &tried).unwrap();
    tick(&mut fork, &log);
    assert_eq!(logged(&fork, id), ["tried"]);
}

#[test]
fn a_fork_opened_at_a_hit_runs_from_its_saved_state_to_the_hits_position() {
    let log = log_of(vec![deploy(0, 0, "printer", LINES)]);
    let watches = [watch(LINES, 4)];
    let mut sim = Sim::new(map_of(MAP));
    let mut states = BTreeMap::new();
    let mut hits = Vec::new();
    for t in 1..=3 {
        states.insert(sim.world.tick, sim.save());
        hits.extend(sim.step(&log.commands(t), &watches).hits);
    }
    let [hit] = hits.as_slice() else {
        panic!("line 4 begins once, in tick 3: {hits:?}");
    };
    let id = hit.machine;
    assert_eq!(hit.position, slice(3, id, 1));
    // From the state before the hit's tick, and from one further back.
    for from in [2, 0] {
        let Ok(fork) = Fork::open_at(&states[&from], hit, TeamId(0), false, &log) else {
            panic!("the state at {from} leads to the hit");
        };
        assert_eq!(fork.position(), hit.position);
        let [stop] = fork.stops() else {
            panic!("its first stop: {:?}", fork.stops());
        };
        assert_eq!((stop.cause, stop.machine), (Cause::Watch, Some(id)));
        assert_eq!(
            (stop.position, stop.hits.as_slice()),
            (hit.position, [hit.clone()].as_slice())
        );
        assert_eq!(line(&fork, id), 4);
        assert!(logged(&fork, id).is_empty(), "stopped before line 4 ran");
    }
    // A hit naming a boundary the slice never passes is no position.
    let nowhere = Hit {
        position: slice(3, id, 500),
        ..hit.clone()
    };
    let refused = Fork::open_at(&states[&2], &nowhere, TeamId(0), false, &log).err();
    assert_eq!(
        refused.as_deref(),
        Some("the saved state does not lead to the hit")
    );
}

#[test]
fn a_fork_reports_its_teams_machines_and_no_other() {
    let (mut fork, log, id) = printer_fork(LINES);
    for _ in 0..3 {
        took(&mut fork, Step::Line, id, &log);
    }
    let shown = fork.inspect(id, 80).expect("the team's own machine");
    assert_eq!(shown.record.id, id);
    assert_eq!((shown.record.team, shown.record.busy), (TeamId(0), None));
    let program = shown.program.expect("a printer runs a program");
    assert_eq!(program.frames.len(), 1);
    assert_eq!(
        (program.frames[0].file.as_str(), program.frames[0].line),
        ("main.py", 3)
    );
    assert!(program.spent > 0 && program.spent < program.budget);
    assert!(!program.waiting && program.handler.is_none() && !program.dead);
    // At the tick's end it is inside its wait, the action in its record.
    tick(&mut fork, &log);
    let shown = fork.inspect(id, 80).unwrap();
    assert_eq!(shown.record.busy, Some("wait"));
    assert!(shown.record.progress > 0);
    assert!(shown.program.unwrap().waiting);
    // A value is cut at the caller's length.
    let short = fork.inspect(id, 0).unwrap().program.unwrap();
    assert!(
        short.modules[0]
            .globals
            .iter()
            .all(|g| g.1.chars().count() <= 1),
        "{short:?}"
    );
    let rival = machine(&fork, 1, Model::Printer);
    assert!(
        fork.inspect(rival, 80).is_none(),
        "a rival's program is not shown"
    );
    assert!(fork.inspect(EntityId(99), 80).is_none());
    // An edit is offered of the team's own machine with a program, and
    // of no other.
    assert_eq!(fork.can_edit(id), Ok(()));
    assert_eq!(
        fork.can_edit(rival).unwrap_err(),
        "no such machine of this team"
    );
    assert_eq!(
        fork.can_edit(EntityId(99)).unwrap_err(),
        "no such machine of this team"
    );
}

/// `docs/06`, Drawing a fork: mid-tick, the team's own machines as they
/// stand and every other as at the end of the last completed tick.
#[test]
fn a_mid_tick_snapshot_draws_rivals_as_at_the_last_completed_tick() {
    // Team 1's fork: team 0's printer, the lower id, has run its slice of
    // the tick by the time team 1's stops.
    let mut fork = Fork::from_map(map_of(MAP), TeamId(1));
    let rival = machine(&fork, 0, Model::Printer);
    let mine = machine(&fork, 1, Model::Printer);
    let log = log_of(vec![
        deploy(0, 0, "printer", "wait(1000)\n"),
        deploy(0, 1, "printer", "log('here')\nwait(1000)\n"),
    ]);
    took(&mut fork, Step::Line, mine, &log);
    assert_eq!(
        took(&mut fork, Step::Line, mine, &log),
        (Cause::Statement, 1, 2)
    );
    assert_eq!(
        fork.world().machines[&rival].busy(),
        Some("wait"),
        "the fork's world ran it"
    );
    let drawn = fork.snapshot();
    let of = |id| drawn.machines.iter().find(|m| m.record.id == id).unwrap();
    assert_eq!(of(rival).record.busy, None, "drawn as tick 0 left it");
    assert_eq!(of(mine).log.len(), 1, "the team's own, as it stands");
    assert_eq!(drawn.machines.len(), 2);
    let ids: Vec<EntityId> = drawn.machines.iter().map(|m| m.record.id).collect();
    assert_eq!(ids, [rival, mine], "in id order");
    // At the tick's end the last completed tick is this one.
    tick(&mut fork, &log);
    let drawn = fork.snapshot();
    let of = |id| drawn.machines.iter().find(|m| m.record.id == id).unwrap();
    assert_eq!(of(rival).record.busy, Some("wait"));
    assert_eq!(drawn.tick, 1);
    // Each machine with the version of the bundle it runs, mid-tick and
    // at the tick's end alike.
    let version = |main: &str| {
        let limits = lang::Limits::parse(lang::data::LIMITS_TOML).unwrap();
        Some(sim::load_bundle(&bundle_of(main), &limits).unwrap().version)
    };
    assert_eq!(of(rival).runs, version("wait(1000)\n"));
    assert_eq!(of(mine).runs, version("log('here')\nwait(1000)\n"));
    assert_ne!(of(rival).runs, of(mine).runs);
}
