//! The interrupt model of `docs/01-language/execution.md`: hooks bound at
//! load, hook budgets, delivery at boundaries, escalation, preemption, and
//! what the fault record holds. The sim raises `dying`, `death`,
//! `low_power` and `redeploy` through `Machine::raise`; here a scripted
//! host does. The last tests are Q62's and Q63's (T39): `low_power`,
//! which sets main flow aside and puts it back, and a program held before
//! a call the host is not ready for, or for a whole slice.

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
    /// Builtins the host is not ready for (`Host::ready`), and every name
    /// it was asked about, in order.
    refuse: Vec<&'static str>,
    asked: Vec<String>,
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
            "move" => Ok(HostCall::Value(Value::None)),
            _ => Ok(HostCall::Unknown),
        }
    }

    fn current_program(&mut self) -> Option<Program> {
        self.next_program.clone()
    }

    fn ready(&mut self, name: &str) -> bool {
        self.asked.push(name.to_string());
        !self.refuse.contains(&name)
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
            Event::LowPower => "low_power".to_string(),
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

/// Q41: a hook `main.py` does not define binds from the interrupt file of
/// its name, whose body runs first, once per main-flow run, in its own
/// globals; a hook in `main.py` wins over the file's.
#[test]
fn a_hook_binds_from_the_interrupt_file_when_main_lacks_it() {
    let (costs, limits) = tables();
    let files = [
        ("main.py", "log('main')\nx = 1 / 0\n"),
        (
            "on_fault.py",
            "log('module ran')\nseen = [0]\ndef on_fault(e):\n    seen[0] = seen[0] + 1\n    log('hook', seen[0], e)\n",
        ),
    ];
    let program = Program::load(&files, &limits).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        program
            .hooks
            .on_fault
            .as_ref()
            .is_some_and(|h| h.module.as_deref() == Some("on_fault"))
    );
    assert!(program.hooks.on_dying.is_none());
    let mut host = ScriptHost::default();
    let mut m = Machine::new(&program, costs, limits);
    for tick in 1..=3 {
        m.tick = tick;
        m.run_slice(&mut host);
        m.take_events();
    }
    // The module body runs before the hook, once per main-flow run — a
    // fault restarts main flow and clears the modules, as an imported
    // module is re-run after a restart — so every hook sees a fresh
    // `seen`, and the body never runs twice within one run.
    let first = host
        .log
        .iter()
        .position(|l| l == "module ran")
        .expect("the interrupt module ran");
    let hooks: Vec<&String> = host.log.iter().filter(|l| l.starts_with("hook ")).collect();
    assert!(hooks.len() >= 2, "{:?}", host.log);
    assert!(host.log[first + 1].starts_with("hook 1 "), "{:?}", host.log);
    assert_eq!(
        host.log.iter().filter(|l| *l == "module ran").count(),
        hooks.len(),
        "{:?}",
        host.log
    );
    assert!(
        hooks.iter().all(|h| h.starts_with("hook 1 ")),
        "{:?}",
        host.log
    );

    // main.py's own def wins; a file with a hook of the other name is
    // ignored for this one; the arity is checked in the file too.
    let files = [
        ("main.py", "def on_fault(e):\n    log('mine')\nx = 1 / 0\n"),
        ("on_fault.py", "def on_fault(e):\n    log('theirs')\n"),
    ];
    let program = Program::load(&files, &limits_of()).unwrap();
    assert!(
        program
            .hooks
            .on_fault
            .as_ref()
            .is_some_and(|h| h.module.is_none())
    );
    let bad = [
        ("main.py", "x = 1\n"),
        ("on_dying.py", "def on_dying(a):\n    pass\n"),
    ];
    let e = Program::load(&bad, &limits_of()).unwrap_err();
    assert_eq!(e.file, "on_dying.py");
    assert!(e.message.contains("exactly 0 parameters"), "{e}");
}

fn limits_of() -> Rc<Limits> {
    tables().1
}

// ------------------------------------------------- Q62: low_power (T39) ---

#[test]
fn low_power_ranks_between_redeploy_and_fault_and_the_old_kinds_keep_their_bytes() {
    assert!(Interrupt::Redeploy < Interrupt::LowPower);
    assert!(Interrupt::LowPower < Interrupt::Fault);
    assert!(Interrupt::Fault < Interrupt::Dying && Interrupt::Dying < Interrupt::Death);
    assert_eq!(Interrupt::LowPower.name(), "low_power");
    // The state hash's byte for a kind is not its rank, so adding a kind
    // in the middle moved no older kind's.
    assert_eq!(
        [
            Interrupt::Redeploy.code(),
            Interrupt::Fault.code(),
            Interrupt::Dying.code(),
            Interrupt::Death.code(),
            Interrupt::LowPower.code(),
        ],
        [0, 1, 2, 3, 4]
    );
}

#[test]
fn on_low_power_binds_at_load_from_main_or_its_file_with_no_parameter() {
    let p = load("def on_low_power():\n    pass\n");
    assert!(p.hooks.on_low_power.is_some());
    assert!(p.hooks.on_fault.is_none() && p.hooks.on_dying.is_none());
    assert!(load_error("def on_low_power(x):\n    pass\n").contains("exactly 0 parameters"));
    assert!(
        load_error("def on_low_power():\n    pass\ndef on_low_power():\n    pass\n")
            .contains("twice")
    );
    let (_, limits) = tables();
    let from_file = Program::load(
        &[
            ("main.py", "x = 1\n"),
            ("on_low_power.py", "def on_low_power():\n    pass\n"),
        ],
        &limits,
    )
    .unwrap();
    assert!(
        from_file
            .hooks
            .on_low_power
            .as_ref()
            .is_some_and(|h| h.module.as_deref() == Some("on_low_power"))
    );
    // `main.py`'s wins over the file's.
    let both = Program::load(
        &[
            ("main.py", "def on_low_power():\n    pass\n"),
            ("on_low_power.py", "def on_low_power():\n    pass\n"),
        ],
        &limits,
    )
    .unwrap();
    assert!(
        both.hooks
            .on_low_power
            .as_ref()
            .is_some_and(|h| h.module.is_none())
    );
}

#[test]
fn low_power_runs_its_hook_and_main_flow_goes_on_where_it_was() {
    let src =
        "log('top')\nn = 5\nt = wait()\nlog('after', n, t)\ndef on_low_power():\n    log('hook')\n";
    let mut host = ScriptHost::default();
    let (m, ticks) = run(src, &[(2, Interrupt::LowPower)], 2, &mut host);
    assert_eq!(ticks[0].slice, Slice::Wait);
    // The wait is over (the run resumed it with the tick), so the hook
    // runs at once and main flow then takes the wait's value, its
    // variables as they were. The restart is main flow's own end.
    assert_eq!(kinds(&ticks[1].events), ["low_power"]);
    assert_eq!(ticks[1].mode, "main");
    assert_eq!(host.log, ["top", "hook", "after 5 1"]);
    assert_eq!(ticks[1].slice, Slice::Restarted);
    assert!(
        m.fault_record.is_none(),
        "nothing failed, so nothing is recorded"
    );
}

#[test]
fn low_power_waits_for_the_action_in_progress_and_a_redeploy_beside_it_does_not() {
    let src =
        "log('top')\nt = wait()\nlog('after', t)\nwait()\ndef on_low_power():\n    log('hook')\n";
    let mut m = machine(src);
    let mut host = ScriptHost::default();
    m.tick = 1;
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    // Raised inside the wait: it stays pending, tick after tick.
    m.raise(Interrupt::LowPower);
    for tick in 2..=3 {
        m.tick = tick;
        assert_eq!(m.run_slice(&mut host), Slice::Wait);
        assert_eq!(kinds(&m.take_events()), Vec::<String>::new());
        assert_eq!(m.mode_name(), "main");
    }
    assert_eq!(host.log, ["top"]);
    // The action completes; the hook runs before main flow takes its value.
    m.resume(Value::int(7));
    m.tick = 4;
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    assert_eq!(kinds(&m.take_events()), ["low_power"]);
    assert_eq!(host.log, ["top", "hook", "after 7"]);

    // A `redeploy` pending beside it is delivered inside the wait, as
    // always; `low_power` then reaches the new program's first boundary.
    let mut m = machine(src);
    let mut host = ScriptHost {
        next_program: Some(load(
            "log('new')\nwait()\ndef on_low_power():\n    log('new hook')\n",
        )),
        ..Default::default()
    };
    m.tick = 1;
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    m.raise(Interrupt::LowPower);
    m.raise(Interrupt::Redeploy);
    m.tick = 2;
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    assert_eq!(kinds(&m.take_events()), ["redeploy:true", "low_power"]);
    assert_eq!(host.log, ["top", "new hook", "new"]);
}

#[test]
fn low_power_with_no_hook_changes_nothing() {
    let src = "log('top')\nt = wait()\nlog('after', t)\n";
    let mut plain = ScriptHost::default();
    let (a, _) = run(src, &[], 3, &mut plain);
    let mut host = ScriptHost::default();
    let (b, ticks) = run(
        src,
        &[(2, Interrupt::LowPower), (2, Interrupt::LowPower)],
        3,
        &mut host,
    );
    // A missing hook is an empty one: the two raises coalesce into one
    // delivery, and main flow runs exactly as it would have.
    assert_eq!(kinds(&events_of(&ticks)), ["low_power"]);
    assert_eq!(host.log, plain.log);
    assert_eq!(a.state_bytes(), b.state_bytes());
    assert_eq!(a.trace().restarts, b.trace().restarts);
}

#[test]
fn the_hook_shares_main_flows_globals_and_a_second_low_power_merges_into_it() {
    let mut host = ScriptHost::default();
    let (_, ticks) = run(
        "seen = []\nt = wait()\nlog('main sees', len(seen), t)\nwait()\ndef on_low_power():\n    seen.append(1)\n    log('hook')\n    wait()\n    log('hook resumes')\n",
        &[(2, Interrupt::LowPower), (3, Interrupt::LowPower)],
        4,
        &mut host,
    );
    assert_eq!(ticks[1].mode, "low_power");
    assert_eq!(ticks[1].slice, Slice::Wait);
    // Raised again while the hook runs: it merges, and the hook runs once.
    assert_eq!(kinds(&events_of(&ticks)), ["low_power"]);
    assert_eq!(
        host.log,
        ["hook", "hook resumes", "main sees 1 1"],
        "main flow went on with its own wait's value and the hook's change"
    );
}

#[test]
fn an_exception_in_on_low_power_escalates_to_fault_and_main_flow_restarts() {
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "log('top')\nwait()\nlog('never')\ndef on_low_power():\n    x = 1 / 0\ndef on_fault(e):\n    log('on_fault', e)\n",
        &[(2, Interrupt::LowPower)],
        3,
        &mut host,
    );
    assert_eq!(
        kinds(&ticks[1].events),
        [
            "low_power",
            "escalated:low_power>fault",
            "fault:ZeroDivisionError"
        ]
    );
    // `fault`'s epilogue restarts main flow: the one set aside is gone.
    assert_eq!(ticks[1].slice, Slice::Restarted);
    assert_eq!(ticks[1].mode, "main");
    let rec = m.fault_record.as_ref().expect("the fault wrote its record");
    assert_eq!(
        rec.exception.as_ref().map(|e| e.class),
        Some(ExcClass::ZeroDivisionError)
    );
    assert_eq!(rec.line, 5);
    assert_eq!(host.log, ["top", "on_fault ZeroDivisionError", "top"]);
    assert!(!m.is_dead());
}

#[test]
fn on_low_power_exhausting_its_budget_is_a_limit_error_fault() {
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "log('top')\nwait()\ndef on_low_power():\n    i = 0\n    while i < 1000000:\n        i += 1\n    log('hook done')\ndef on_fault(e):\n    log('on_fault', e)\n",
        &[(2, Interrupt::LowPower)],
        400,
        &mut host,
    );
    assert_eq!(
        kinds(&events_of(&ticks)),
        ["low_power", "escalated:low_power>fault", "fault:LimitError"]
    );
    assert!(!host.log.contains(&"hook done".to_string()));
    assert!(
        host.log
            .contains(&"on_fault LimitError: on_low_power exhausted its budget".to_string()),
        "{:?}",
        host.log
    );
    // `on_fault(e)` needs an exception, so the record holds one too, and
    // is located at the last operation the hook completed.
    let rec = m.fault_record.as_ref().expect("record");
    assert_eq!(
        rec.exception.as_ref().map(|e| e.class),
        Some(ExcClass::LimitError)
    );
    assert_eq!(rec.exhausted, None);
    assert_eq!(rec.file.as_deref(), Some("main.py"));
    assert!((5..=6).contains(&rec.line), "line {}", rec.line);
    assert!(
        !m.is_dead(),
        "a hook that ran too long costs a fault, not the machine"
    );
}

#[test]
fn low_power_waits_for_on_fault_and_a_redeploy_waits_for_on_low_power() {
    // Raised while `on_fault` waits: it is lower, so it waits for the
    // epilogue and lands at the restarted program's first boundary.
    let mut host = ScriptHost::default();
    let (_, ticks) = run(
        "log('top')\nx = 1 / 0\ndef on_fault(e):\n    log('fault hook')\n    wait()\ndef on_low_power():\n    log('low hook')\n",
        &[(2, Interrupt::LowPower)],
        3,
        &mut host,
    );
    assert_eq!(kinds(&ticks[0].events), ["fault:ZeroDivisionError"]);
    assert_eq!(ticks[0].mode, "fault");
    assert_eq!(kinds(&ticks[1].events), Vec::<String>::new());
    assert_eq!(ticks[1].slice, Slice::Restarted);
    // Then the hook, and main flow from its top, which faults again.
    assert_eq!(
        kinds(&ticks[2].events),
        ["low_power", "fault:ZeroDivisionError"]
    );
    assert_eq!(
        host.log,
        ["top", "fault hook", "low hook", "top", "fault hook"]
    );

    // A `redeploy` raised while `on_low_power` waits does the same: the
    // hook finishes, main flow goes on to its end, and then the swap.
    let mut host = ScriptHost {
        next_program: Some(load("log('new program')\n")),
        ..Default::default()
    };
    let (_, ticks) = run(
        "log('old')\nwait()\nlog('old goes on')\ndef on_low_power():\n    log('low hook')\n    wait()\n    log('low hook resumes')\n",
        &[(2, Interrupt::LowPower), (3, Interrupt::Redeploy)],
        4,
        &mut host,
    );
    assert_eq!(kinds(&ticks[1].events), ["low_power"]);
    assert_eq!(ticks[1].mode, "low_power");
    assert_eq!(kinds(&ticks[2].events), ["redeploy:true"]);
    assert_eq!(
        host.log[..4],
        ["old", "low hook", "low hook resumes", "new program"][..]
    );

    // Both pending at one boundary: `low_power` is the higher. With no
    // hook it is over at once, and the redeploy follows in the same slice.
    let mut host = ScriptHost {
        next_program: Some(load("log('new program')\n")),
        ..Default::default()
    };
    let (_, ticks) = run(
        "log('old')\nwait()\n",
        &[(2, Interrupt::Redeploy), (2, Interrupt::LowPower)],
        2,
        &mut host,
    );
    assert_eq!(kinds(&ticks[1].events), ["low_power", "redeploy:true"]);
    assert_eq!(host.log, ["old", "new program"]);
}

#[test]
fn dying_preempts_on_low_power_and_the_main_flow_set_aside_goes_with_it() {
    let mut host = ScriptHost::default();
    let (m, ticks) = run(
        "log('top')\nwait()\nlog('never')\ndef on_low_power():\n    log('low hook')\n    wait()\n    log('never either')\ndef on_dying():\n    log('last words')\n",
        &[(2, Interrupt::LowPower), (3, Interrupt::Dying)],
        4,
        &mut host,
    );
    assert_eq!(kinds(&events_of(&ticks)), ["low_power", "dying", "death"]);
    assert_eq!(host.log, ["top", "low hook", "last words"]);
    assert!(m.is_dead());
}

#[test]
fn main_flow_set_aside_is_in_the_state_and_a_copy_goes_on_as_its_source_does() {
    let src = "n = [1, 2]\nt = wait()\nlog('main', n, t)\nwait()\ndef on_low_power():\n    log('hook')\n    wait()\n    n.append(3)\n";
    let mut m = machine(src);
    let mut host = ScriptHost::default();
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Wait);
    m.raise(Interrupt::LowPower);
    assert_eq!(tick_once(&mut m, &mut host, 2), Slice::Wait);
    assert_eq!(m.mode_name(), "low_power");
    // In the hook, with main flow set aside: a copy holds both.
    let mut copy = m.deep_copy();
    assert_eq!(copy.state_bytes(), m.state_bytes());
    let mut other = ScriptHost::default();
    for tick in 3..=4 {
        assert_eq!(
            tick_once(&mut m, &mut host, tick),
            tick_once(&mut copy, &mut other, tick)
        );
        assert_eq!(copy.state_bytes(), m.state_bytes());
    }
    assert_eq!(host.log, ["hook", "main [1, 2, 3] 1"]);
    assert_eq!(other.log, ["main [1, 2, 3] 1"]);
}

// ----------------------- Q62, Q63: a program held (T39, docs/08) ---

/// Run one slice at `tick`, resuming a wait with the tick as `run` does.
fn tick_once(m: &mut Machine, host: &mut ScriptHost, tick: u64) -> Slice {
    m.tick = tick;
    let slice = m.run_slice(host);
    if slice == Slice::Wait {
        m.resume(Value::int(i128::from(tick)));
    }
    slice
}

fn machine(src: &str) -> Machine {
    let (costs, limits) = tables();
    Machine::new(&load(src), costs, limits)
}

#[test]
fn a_call_the_host_is_not_ready_for_holds_main_flow_before_it_and_charges_nothing() {
    let mut m = machine("log('a')\nt = wait()\nlog('b', t)\n");
    let mut host = ScriptHost {
        refuse: vec!["wait"],
        ..Default::default()
    };
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Held);
    assert!(m.is_held());
    assert_eq!(host.log, ["a"]);
    let spent_reaching_it = m.spent();
    // Held again: the host is asked once a slice, and nothing is charged.
    let asked = host.asked.len();
    assert_eq!(tick_once(&mut m, &mut host, 2), Slice::Held);
    assert_eq!(host.asked.len(), asked.wrapping_add(1));
    assert_eq!(m.spent(), 0);
    assert!(m.is_held());
    // Ready: the call is made as if first reached now, and waits.
    host.refuse.clear();
    assert_eq!(tick_once(&mut m, &mut host, 3), Slice::Wait);
    assert!(!m.is_held());
    assert!(m.spent() > 0, "the builtin's row is charged when it begins");
    assert!(spent_reaching_it > 0);
    assert_eq!(tick_once(&mut m, &mut host, 4), Slice::Restarted);
    assert_eq!(host.log, ["a", "b 3"]);
}

#[test]
fn a_held_call_is_part_of_the_state_and_a_copy_goes_on_as_its_source_does() {
    let src = "t = wait()\nlog('b', t)\n";
    let mut free = machine(src);
    let mut held = machine(src);
    let mut refusing = ScriptHost {
        refuse: vec!["wait"],
        ..Default::default()
    };
    let before = held.state_bytes();
    assert_eq!(before, free.state_bytes());
    assert_eq!(tick_once(&mut held, &mut refusing, 1), Slice::Held);
    assert_ne!(held.state_bytes(), before, "a held call is in the hash");
    let mut copy = held.deep_copy();
    assert_eq!(copy.state_bytes(), held.state_bytes());
    assert!(copy.is_held());
    let mut a = ScriptHost::default();
    let mut b = ScriptHost::default();
    for tick in 2..=3 {
        assert_eq!(
            tick_once(&mut held, &mut a, tick),
            tick_once(&mut copy, &mut b, tick)
        );
        assert_eq!(copy.state_bytes(), held.state_bytes());
    }
    assert_eq!(a.log, ["b 2"]);
    assert_eq!(b.log, a.log);
    // A machine never held hashes as it always did: the hold adds bytes
    // only while there is one.
    let mut c = ScriptHost::default();
    assert_eq!(tick_once(&mut free, &mut c, 1), Slice::Wait);
    assert!(!free.is_held());
}

#[test]
fn a_redeploy_abandons_a_held_call_and_on_fault_is_never_held() {
    // Held, then redeployed: the swap takes it, and nothing is left held.
    let mut m = machine("log('old')\nwait()\n");
    let mut host = ScriptHost {
        refuse: vec!["wait"],
        next_program: Some(load("log('new')\n")),
        ..Default::default()
    };
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Held);
    m.raise(Interrupt::Redeploy);
    assert_eq!(tick_once(&mut m, &mut host, 2), Slice::Restarted);
    assert!(!m.is_held());
    assert_eq!(host.log, ["old", "new"]);

    // `on_fault`'s call of a refused builtin is made without the host
    // being asked: a hook that must finish is never held.
    let mut m = machine(
        "x = 1 / 0\ndef on_fault(e):\n    log('hook')\n    move('n')\n    log('hook moved')\n",
    );
    let mut host = ScriptHost {
        refuse: vec!["move"],
        ..Default::default()
    };
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Restarted);
    assert_eq!(host.log, ["hook", "hook moved"]);
    assert!(
        !host.asked.contains(&"move".to_string()),
        "{:?}",
        host.asked
    );
}

/// `docs/08`, Out of power: main flow that cannot pay is held; the wake
/// raises `low_power`, whose hook runs first; then main flow makes the
/// call it was held before.
#[test]
fn a_held_main_flow_woken_by_low_power_runs_the_hook_then_makes_its_call() {
    let mut m =
        machine("log('a')\nmove('n')\nlog('b')\nwait()\ndef on_low_power():\n    log('hook')\n");
    let mut host = ScriptHost {
        refuse: vec!["move"],
        ..Default::default()
    };
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Held);
    assert_eq!(tick_once(&mut m, &mut host, 2), Slice::Held);
    host.refuse.clear();
    m.raise(Interrupt::LowPower);
    assert_eq!(tick_once(&mut m, &mut host, 3), Slice::Wait);
    assert_eq!(kinds(&m.take_events()), ["low_power"]);
    assert_eq!(host.log, ["a", "hook", "b"]);
    assert!(!m.is_held());
}

/// `on_low_power` is held like main flow, and a `low_power` that finds
/// it held is the bot waking: the hook starts over, with main flow still
/// set aside, and main flow goes on when it returns.
#[test]
fn on_low_power_held_for_power_starts_over_when_low_power_wakes_it() {
    let mut m = machine(
        "log('a')\nt = wait()\nlog('b', t)\nwait()\ndef on_low_power():\n    log('hook starts')\n    move('s')\n    log('hook moved')\n",
    );
    let mut host = ScriptHost {
        refuse: vec!["move"],
        ..Default::default()
    };
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Wait);
    m.raise(Interrupt::LowPower);
    assert_eq!(tick_once(&mut m, &mut host, 2), Slice::Held);
    assert_eq!(m.mode_name(), "low_power");
    assert!(m.is_held());
    assert!(host.asked.contains(&"move".to_string()));
    let spent_before = m.spent();
    // Held again; the hook's budget is not what a held tick spends.
    assert_eq!(tick_once(&mut m, &mut host, 3), Slice::Held);
    assert_eq!(m.spent(), 0);
    assert!(spent_before > 0);
    // Not held: a second `low_power` would merge. Held: it starts the hook
    // over, from its first line.
    host.refuse.clear();
    m.raise(Interrupt::LowPower);
    assert_eq!(tick_once(&mut m, &mut host, 4), Slice::Wait);
    assert_eq!(m.mode_name(), "main");
    assert_eq!(
        host.log,
        ["a", "hook starts", "hook starts", "hook moved", "b 1"]
    );
    let events: Vec<String> = kinds(&m.take_events());
    assert_eq!(events, ["low_power", "low_power"]);
}

#[test]
fn a_held_call_that_fails_when_it_is_made_raises_then() {
    // The host refuses `move`, then is ready and answers with an error: the
    // exception belongs to the slice that made the call, and is catchable.
    struct Failing {
        ready: bool,
        log: Vec<String>,
    }
    impl Host for Failing {
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
                "move" => Err(lang::Exception::value_error()),
                "wait" => Ok(HostCall::Wait),
                _ => Ok(HostCall::Unknown),
            }
        }
        fn ready(&mut self, name: &str) -> bool {
            name != "move" || self.ready
        }
    }
    let mut m = machine("try:\n    move('n')\nexcept ValueError:\n    log('caught')\nwait(1)\n");
    let mut host = Failing {
        ready: false,
        log: vec![],
    };
    m.tick = 1;
    assert_eq!(m.run_slice(&mut host), Slice::Held);
    assert!(host.log.is_empty());
    host.ready = true;
    m.tick = 2;
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    assert_eq!(host.log, ["caught"]);
    assert!(m.fault_record.is_none());
}

#[test]
fn a_native_call_of_the_host_is_held_and_made_once() {
    // `log` of an instance renders it through `__str__` before the host
    // is called; the hold comes after the rendering and repeats none of it.
    let mut m =
        machine("class A:\n    def __str__(self):\n        return 'an A'\nlog(A())\nwait()\n");
    let mut host = ScriptHost {
        refuse: vec!["log"],
        ..Default::default()
    };
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Held);
    assert!(host.log.is_empty());
    host.refuse.clear();
    assert_eq!(tick_once(&mut m, &mut host, 2), Slice::Wait);
    assert_eq!(host.log, ["an A"]);
}

#[test]
fn a_held_slice_runs_handlers_and_no_main_flow() {
    let mut m = machine("log('main')\nwait()\ndef on_dying():\n    log('last words')\n");
    let mut host = ScriptHost::default();
    // Dark from the start: main flow runs nothing, tick after tick.
    for tick in 1..=3 {
        m.tick = tick;
        assert_eq!(m.run_slice_held(&mut host, None), Slice::Held);
        assert_eq!(m.spent(), 0);
    }
    assert!(host.log.is_empty());
    assert!(!m.is_held(), "a held slice is the host's, not a held call");
    // Lit again: it runs as if nothing had happened.
    m.tick = 4;
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    assert_eq!(host.log, ["main"]);
    // Dark while its action waits: the wait is still a wait.
    m.tick = 5;
    assert_eq!(m.run_slice_held(&mut host, None), Slice::Wait);
    // A handler runs whatever the hold: `dying` ends in `death`.
    m.raise(Interrupt::Dying);
    m.tick = 6;
    assert_eq!(m.run_slice_held(&mut host, None), Slice::Dead);
    assert_eq!(kinds(&m.take_events()), ["dying", "death"]);
    assert_eq!(host.log, ["main", "last words"]);
}

#[test]
fn a_held_slice_runs_on_low_power_and_then_no_main_flow() {
    let mut m =
        machine("log('main')\nwait()\nlog('after')\ndef on_low_power():\n    log('hook')\n");
    let mut host = ScriptHost::default();
    assert_eq!(tick_once(&mut m, &mut host, 1), Slice::Wait);
    // The hook is a handler, so it runs in a held slice; main flow, put
    // back when it returns, does not.
    m.raise(Interrupt::LowPower);
    m.tick = 2;
    assert_eq!(m.run_slice_held(&mut host, None), Slice::Held);
    assert_eq!(host.log, ["main", "hook"]);
    assert_eq!(m.mode_name(), "main");
    assert_eq!(tick_once(&mut m, &mut host, 3), Slice::Restarted);
    assert_eq!(host.log, ["main", "hook", "after"]);
}
