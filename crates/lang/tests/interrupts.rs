//! The interrupt model of `docs/01-language/execution.md`: hooks bound at
//! load, hook budgets, delivery at boundaries, escalation, preemption, and
//! what the fault record holds. The sim raises `dying`, `death` and
//! `redeploy` through `Machine::raise`; here a scripted host does.

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{
    Costs, Event, ExcClass, Host, HostCall, Interrupt, Limits, Machine, Program, Slice, Value,
};
use std::rc::Rc;

/// A host that logs, waits on `wait()`, and can hand over a bundle for a
/// `redeploy`.
#[derive(Default)]
struct ScriptHost {
    log: Vec<String>,
    next_program: Option<Program>,
}

impl Host for ScriptHost {
    fn call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        _: Vec<(String, Value)>,
    ) -> lang::value::R<HostCall> {
        match name {
            "log" => {
                self.log.push(
                    args.iter()
                        .map(Value::to_str_value)
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                Ok(HostCall::Value(Value::None))
            }
            "wait" => Ok(HostCall::Wait),
            _ => Ok(HostCall::Unknown),
        }
    }

    fn current_program(&mut self) -> Option<Program> {
        self.next_program.clone()
    }
}

fn tables() -> (Rc<Costs>, Rc<Limits>) {
    (
        Rc::new(Costs::parse(COSTS_TOML).expect("costs")),
        Rc::new(Limits::parse(LIMITS_TOML).expect("limits")),
    )
}

fn load(src: &str) -> Program {
    let (_, limits) = tables();
    Program::load(&[("main.py", src)], &limits).unwrap_or_else(|e| panic!("{e}"))
}

fn load_error(src: &str) -> String {
    let (_, limits) = tables();
    match Program::load(&[("main.py", src)], &limits) {
        Ok(_) => panic!("loaded, but should have been refused:\n{src}"),
        Err(e) => e.message,
    }
}

/// One step of a scripted run: what to raise before the slice, and how
/// the slice and its events came out.
#[derive(Debug, PartialEq)]
struct Tick {
    tick: u64,
    slice: Slice,
    events: Vec<Event>,
    mode: &'static str,
}

/// Run `ticks` slices, raising `schedule`'s interrupts before the slice of
/// their tick; waits resume with the tick number.
fn run(
    src: &str,
    schedule: &[(u64, Interrupt)],
    ticks: u64,
    host: &mut ScriptHost,
) -> (Machine, Vec<Tick>) {
    let (costs, limits) = tables();
    let program = load(src);
    let mut m = Machine::new(&program, costs, limits);
    let mut out = Vec::new();
    for tick in 1..=ticks {
        m.tick = tick;
        for (t, kind) in schedule {
            if *t == tick {
                m.raise(*kind);
            }
        }
        let slice = m.run_slice(host);
        let events = m.take_events();
        out.push(Tick {
            tick,
            slice,
            events,
            mode: m.mode_name(),
        });
        if slice == Slice::Wait {
            m.resume(Value::int(i128::from(tick)));
        }
        if slice == Slice::Dead {
            break;
        }
    }
    (m, out)
}

fn events_of(ticks: &[Tick]) -> Vec<Event> {
    ticks.iter().flat_map(|t| t.events.clone()).collect()
}

fn kinds(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            Event::Fault(x) => format!("fault:{}", x.class.name()),
            Event::Abandoned(x) => format!("abandoned:{}", x.class.name()),
            Event::Escalated { from, to } => format!("escalated:{}>{}", from.name(), to.name()),
            Event::Dying => "dying".to_string(),
            Event::Death => "death".to_string(),
            Event::Redeploy { swapped } => format!("redeploy:{swapped}"),
        })
        .collect()
}

#[test]
fn hooks_are_bound_at_load_with_their_arity_checked() {
    let p = load("def on_fault(e):\n    pass\n");
    assert!(p.hooks.on_fault.is_some() && p.hooks.on_dying.is_none());
    let p = load("x = 1\ndef on_dying():\n    pass\ndef helper():\n    pass\n");
    assert!(p.hooks.on_dying.is_some() && p.hooks.on_fault.is_none());
    assert!(load_error("def on_fault():\n    pass\n").contains("exactly 1 parameter"));
    assert!(load_error("def on_fault(a, b):\n    pass\n").contains("exactly 1 parameter"));
    assert!(load_error("def on_fault(*a):\n    pass\n").contains("exactly 1 parameter"));
    assert!(load_error("def on_dying(x):\n    pass\n").contains("exactly 0 parameters"));
    assert!(
        load_error("def on_fault(e):\n    pass\ndef on_fault(e):\n    pass\n").contains("twice")
    );
    // Any other `on_` name is an ordinary function; a hook in a module is not
    // a hook; a class method named like a hook is not a hook.
    let p = load("def on_boot():\n    pass\nclass A:\n    def on_fault(self, e):\n        pass\n");
    assert!(p.hooks.on_fault.is_none());
    let (_, limits) = tables();
    let p = Program::load(
        &[
            ("main.py", "import m\n"),
            ("m.py", "def on_fault(e):\n    pass\n"),
        ],
        &limits,
    )
    .unwrap();
    assert!(p.hooks.on_fault.is_none());
}

#[test]
fn a_fault_runs_on_fault_before_the_restart_and_the_hook_survives_the_clear() {
    // The hook is defined on the last line, the fault happens on the first:
    // binding at load means it still runs. It sees the globals as they were.
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "state = 'set'\nlog('main start')\nx = 1 / 0\nlog('never')\n\ndef on_fault(e):\n    log('hook', str(e), e.file, e.line, e.tick, state)\n",
        &[],
        2,
        &mut host,
    );
    assert_eq!(
        host.log,
        [
            "main start",
            "hook ZeroDivisionError main.py 3 1 set",
            "main start",
            "hook ZeroDivisionError main.py 3 2 set"
        ]
    );
    assert_eq!(ticks[0].slice, Slice::Restarted);
    assert_eq!(kinds(&ticks[0].events), ["fault:ZeroDivisionError"]);
    assert_eq!(ticks[0].mode, "main");
    let rec = m.fault_record.expect("record");
    assert_eq!((rec.line, rec.tick, rec.exhausted), (3, 2, None));
    assert_eq!(
        rec.exception.map(|e| e.class),
        Some(ExcClass::ZeroDivisionError)
    );
}

#[test]
fn an_exception_in_on_fault_escalates_to_dying_then_on_dying_to_death() {
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "x = 1 / 0\ndef on_fault(e):\n    log('on_fault')\n    raise KeyError('in hook')\ndef on_dying():\n    log('on_dying')\n",
        &[],
        3,
        &mut host,
    );
    assert_eq!(host.log, ["on_fault", "on_dying"]);
    assert_eq!(
        kinds(&events_of(&ticks)),
        [
            "fault:ZeroDivisionError",
            "escalated:fault>dying",
            "dying",
            "death"
        ]
    );
    assert_eq!(ticks[0].slice, Slice::Dead);
    assert!(m.is_dead());
    // The record was rewritten for the escalation.
    let rec = m.fault_record.expect("record");
    assert_eq!(rec.exception.map(|e| e.class), Some(ExcClass::KeyError));
    assert_eq!(rec.line, 4);
    // A dead machine runs nothing further.
    assert_eq!(ticks.len(), 1);
}

#[test]
fn a_fault_with_no_hooks_restarts_and_dying_with_no_hook_is_death_at_once() {
    let mut host = ScriptHost::default();
    let (_, ticks) = run(
        "log('run')\nx = [][0]\n",
        &[(3, Interrupt::Dying)],
        5,
        &mut host,
    );
    assert_eq!(host.log, ["run", "run"]);
    assert_eq!(kinds(&ticks[0].events), ["fault:IndexError"]);
    assert_eq!(ticks[0].slice, Slice::Restarted);
    assert_eq!(kinds(&ticks[2].events), ["dying", "death"]);
    assert_eq!(ticks[2].slice, Slice::Dead);
}

/// The hook budget is exhausted when the total debited *exceeds* it,
/// checked at every boundary including the one after the last operation.
/// The loop count at which escalation first happens is found by search,
/// and the count one below it must return: that is "on its last
/// operation", whatever the cost table says.
#[test]
fn a_hook_exhausting_its_budget_on_its_last_operation_escalates() {
    fn outcome(n: u32) -> (Vec<String>, Option<Interrupt>, bool) {
        let src = format!(
            "x = 1 / 0\ndef on_fault(e):\n    i = 0\n    while i < {n}:\n        i += 1\n    log('hook done')\ndef on_dying():\n    log('dying')\n"
        );
        let mut host = ScriptHost::default();
        let (m, ticks) = run(&src, &[], 200, &mut host);
        let exhausted = m.fault_record.as_ref().and_then(|r| r.exhausted);
        (
            kinds(&events_of(&ticks)),
            exhausted,
            host.log.contains(&"hook done".to_string()),
        )
    }
    // Search for the first n that escalates.
    let mut lo = 1u32;
    let mut hi = 4000u32;
    assert!(
        outcome(hi).1.is_some(),
        "the budget was never exhausted at n={hi}"
    );
    assert!(
        outcome(lo).1.is_none(),
        "the budget was exhausted at n={lo}"
    );
    while hi.wrapping_sub(lo) > 1 {
        let mid = lo.wrapping_add(hi.wrapping_sub(lo).wrapping_div(2));
        if outcome(mid).1.is_some() {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let (ok_kinds, _, ok_done) = outcome(lo);
    let (bad_kinds, exhausted, _) = outcome(hi);
    // Below the threshold the hook returns every time and main flow
    // restarts into the same fault, over and over for 200 ticks.
    assert!(ok_done, "n={lo} should have returned");
    assert!(
        ok_kinds.len() > 1 && ok_kinds.iter().all(|k| k == "fault:ZeroDivisionError"),
        "{ok_kinds:?}"
    );
    // One iteration more and the total exceeds the budget at the hook's
    // last boundary: no epilogue, the machine dies instead.
    assert_eq!(exhausted, Some(Interrupt::Fault));
    assert_eq!(
        bad_kinds,
        [
            "fault:ZeroDivisionError",
            "escalated:fault>dying",
            "dying",
            "death"
        ]
    );
    assert!(hi > 10, "budget exhausted after only {hi} iterations");
}

#[test]
fn a_waiting_hook_pays_per_tick_and_can_be_preempted() {
    // `on_dying` waits; `death` raised during the wait preempts the hook.
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "log('main')\nwait()\ndef on_dying():\n    log('dying starts')\n    wait()\n    log('dying never')\n",
        &[(2, Interrupt::Dying), (3, Interrupt::Death)],
        6,
        &mut host,
    );
    assert_eq!(host.log, ["main", "dying starts"]);
    assert_eq!(ticks[1].slice, Slice::Wait);
    assert_eq!(kinds(&ticks[1].events), ["dying"]);
    assert_eq!(ticks[1].mode, "dying");
    // The wait was answered at the end of tick 2; `death` arrives at the
    // boundary before the hook can resume and preempts it.
    assert_eq!(kinds(&ticks[2].events), ["death"]);
    assert_eq!(ticks[2].slice, Slice::Dead);
    assert!(m.is_dead());
}

#[test]
fn an_interrupt_mid_unwind_records_the_exception_and_runs_no_hook() {
    // A `finally` slow enough to span ticks, entered by an exception that
    // would escape; `dying` lands inside it.
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "try:\n    raise KeyError('unwinding')\nfinally:\n    i = 0\n    while i < 500:\n        i += 1\n    log('finally done')\ndef on_fault(e):\n    log('on_fault')\ndef on_dying():\n    log('on_dying')\n",
        &[(2, Interrupt::Dying)],
        6,
        &mut host,
    );
    assert_eq!(host.log, ["on_dying"]);
    assert_eq!(
        kinds(&events_of(&ticks)),
        ["abandoned:KeyError", "dying", "death"]
    );
    let rec = m.fault_record.expect("record");
    assert_eq!(
        rec.exception.map(|e| e.args),
        Some(vec![Value::str("unwinding")])
    );
}

#[test]
fn a_finally_that_raises_replaces_the_exception_being_unwound() {
    let mut host = ScriptHost::default();
    let (m, _) = run(
        "try:\n    raise KeyError('first')\nfinally:\n    raise ValueError('second')\ndef on_fault(e):\n    log(str(e))\n",
        &[],
        1,
        &mut host,
    );
    assert_eq!(host.log, ["ValueError: second"]);
    assert_eq!(
        m.fault_record.and_then(|r| r.exception).map(|e| e.class),
        Some(ExcClass::ValueError)
    );
}

#[test]
fn redeploy_waits_for_on_fault_and_swaps_at_the_next_boundary() {
    let mut host = ScriptHost {
        next_program: Some(load("log('new program')\n")),
        ..Default::default()
    };
    let (_, ticks) = run(
        "log('old')\nx = 1 / 0\ndef on_fault(e):\n    log('hook')\n",
        &[(1, Interrupt::Redeploy)],
        3,
        &mut host,
    );
    // Tick 1: redeploy is pending before the first operation, so the swap
    // happens at once and the old program never runs.
    assert_eq!(kinds(&ticks[0].events), ["redeploy:true"]);
    assert_eq!(host.log, ["new program", "new program", "new program"]);

    // Raised while `on_fault` runs: it waits for the epilogue, then lands
    // at the boundary before the restarted program's first operation.
    let mut host = ScriptHost {
        next_program: Some(load("log('new program')\n")),
        ..Default::default()
    };
    let (_, ticks) = run(
        "log('old')\nx = 1 / 0\ndef on_fault(e):\n    log('hook')\n    wait()\n    log('hook resumes')\n",
        &[(2, Interrupt::Redeploy)],
        4,
        &mut host,
    );
    assert_eq!(kinds(&ticks[0].events), ["fault:ZeroDivisionError"]);
    assert_eq!(ticks[0].slice, Slice::Wait);
    // Tick 2: the hook resumes and returns; the epilogue's restart ends the
    // slice with the redeploy still pending — nothing of the old program
    // ran. Tick 3: the boundary before the first operation delivers it.
    assert_eq!(ticks[1].slice, Slice::Restarted);
    assert_eq!(ticks[1].mode, "main");
    assert_eq!(kinds(&ticks[1].events), Vec::<String>::new());
    assert_eq!(kinds(&ticks[2].events), ["redeploy:true"]);
    assert_eq!(
        host.log,
        ["old", "hook", "hook resumes", "new program", "new program"]
    );
}

#[test]
fn a_redeploy_with_no_bundle_still_restarts_and_the_pending_set_coalesces() {
    let mut host = ScriptHost::default();
    let (_, ticks) = run(
        "log('run')\nwait()\n",
        &[(2, Interrupt::Redeploy), (2, Interrupt::Redeploy)],
        3,
        &mut host,
    );
    assert_eq!(kinds(&ticks[1].events), ["redeploy:false"]);
    // Tick 3 resumes the wait, reaches the end and restarts: that ends the
    // slice, so the third run begins on tick 4.
    assert_eq!(host.log, ["run", "run"]);
}

#[test]
fn dying_outranks_redeploy_and_death_clears_the_pending_set() {
    let mut host = ScriptHost {
        next_program: Some(load("log('new')\n")),
        ..Default::default()
    };
    let (m, ticks) = run(
        "log('main')\nwait()\ndef on_dying():\n    log('last words')\n",
        &[(2, Interrupt::Redeploy), (2, Interrupt::Dying)],
        4,
        &mut host,
    );
    assert_eq!(kinds(&events_of(&ticks)), ["dying", "death"]);
    assert_eq!(host.log, ["main", "last words"]);
    assert!(m.is_dead());
    assert_eq!(ticks.len(), 2);
}

#[test]
fn the_record_survives_restart_and_redeploy_and_carries_the_version() {
    let mut host = ScriptHost::default();
    let (m, _) = run("x = {}['k']\n", &[(2, Interrupt::Redeploy)], 3, &mut host);
    let rec = m.fault_record.expect("record");
    assert_eq!(
        rec.exception.as_ref().map(|e| e.class),
        Some(ExcClass::KeyError)
    );
    assert_eq!(rec.version, load("x = {}['k']\n").version);
    assert_eq!(rec.tick, 3, "the latest fault's tick");
}
