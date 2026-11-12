//! What a debugger asks of the language (`docs/06`, Forks; T26): where a
//! statement begins, what a stopped machine shows, and what an edit may
//! set. That a debugged slice runs as an undebugged one is `golden.rs`'s,
//! against the stored fixtures.

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{
    Costs, EditTarget, Host, HostCall, Limits, Machine, Program, Slice, Tracer, Value, content_hash,
};
use std::collections::BTreeSet;
use std::rc::Rc;

/// A host that logs and waits on `wait()`.
#[derive(Default)]
struct LogHost {
    log: Vec<String>,
}

impl Host for LogHost {
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
}

fn tables() -> (Rc<Costs>, Rc<Limits>) {
    (
        Rc::new(Costs::parse(COSTS_TOML).expect("costs")),
        Rc::new(Limits::parse(LIMITS_TOML).expect("limits")),
    )
}

fn load(files: &[(&str, &str)]) -> Program {
    let (_, limits) = tables();
    Program::load(files, &limits).unwrap_or_else(|e| panic!("{e}"))
}

/// Records every beginning, and stops at the first on `stop_at`.
#[derive(Default)]
struct Lines {
    begun: Vec<(String, u32)>,
    stop_at: Option<u32>,
}

impl Tracer for Lines {
    fn boundary(&mut self, vm: &Machine) -> bool {
        let Some((file, line)) = vm.beginning() else {
            return false;
        };
        self.begun.push((file.to_string(), line));
        self.stop_at == Some(line)
    }
}

/// The lines `src` begins a statement on, in order, until its first
/// `wait()` returns the slice — with a tick budget of `budget`.
fn begun(files: &[(&str, &str)], budget: u64) -> Vec<(String, u32)> {
    let (costs, limits) = tables();
    let limits = Rc::new(Limits {
        budget_tick: budget,
        ..(*limits).clone()
    });
    let mut m = Machine::new(&load(files), costs, limits);
    let mut host = LogHost::default();
    let mut lines = Lines::default();
    for tick in 1..=100_000u64 {
        m.tick = tick;
        if m.run_slice_traced(&mut host, Some(&mut lines)) == Slice::Wait {
            return lines.begun;
        }
    }
    panic!("the program never reached its wait");
}

const CLAUSES: &str = r#""""A docstring begins nothing."""
x: num
def f(a):
    """Nor does this one."""
    return a + 1
i = 0
while i < 2:
    i += 1
for k in [1, 2, 3]:
    pass
if i == 5:
    pass
elif i == 2:
    y = f(
        i)
else:
    pass
try:
    raise KeyError(1)
except ValueError:
    pass
except KeyError:
    pass
finally:
    z = 1
match i:
    case 1:
        pass
    case 2:
        pass
wait()
"#;

/// `docs/06`, Statements, clause by clause.
#[test]
fn statements_and_clauses_begin_where_the_doc_says() {
    let lines: Vec<u32> = begun(&[("main.py", CLAUSES)], 100_000)
        .into_iter()
        .map(|(file, line)| {
            assert_eq!(file, "main.py");
            line
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            3, // `def`, when it is executed
            6, 7, 8, 7, 8, 7, // a `while` on each pass, the last one failing
            9, 10, 9, 10, 9, 10, // a `for` again for each element after the first
            11, 13, // an `elif` when its condition is evaluated
            14, 5, // a call's statement, then the callee's; no continuation line
            18, 19, 20, 22, 23, // `try`, and each `except` tested
            25, // nothing on `finally:`
            26, 27, 29, 30, // `match`, and each `case` tested
            31,
        ]
    );
}

/// Yielding at a boundary, and sitting at one in deficit, begin nothing:
/// however the budget cuts the run into slices, the same statements begin
/// the same number of times.
#[test]
fn a_yield_before_a_statement_begins_nothing() {
    let files = [("main.py", CLAUSES)];
    let whole = begun(&files, 100_000);
    for budget in [1, 2, 3, 5, 7, 11, 13, 50] {
        assert_eq!(
            begun(&files, budget),
            whole,
            "a tick budget of {budget} moved a beginning"
        );
    }
}

#[test]
fn a_breakpoint_may_sit_only_where_a_statement_begins() {
    let program = load(&[("main.py", CLAUSES), ("lib.py", "def g():\n    return 1\n")]);
    let expect: BTreeSet<u32> = [
        3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 29, 30,
        31,
    ]
    .into_iter()
    .collect();
    assert_eq!(program.begin_lines("main.py"), expect);
    assert_eq!(
        program.begin_lines("lib.py"),
        [1, 2].into_iter().collect::<BTreeSet<u32>>()
    );
    assert!(program.begin_lines("absent.py").is_empty());
    // The content hash is of the file's bytes alone: FNV-1a 64.
    assert_eq!(content_hash(b""), 0xcbf29ce484222325);
    assert_eq!(content_hash(b"a"), 0xaf63dc4c8601ec8c);
    assert_eq!(
        program.content.get("main.py").copied(),
        Some(content_hash(CLAUSES.as_bytes()))
    );
    assert_eq!(
        program.content.keys().collect::<Vec<_>>(),
        vec!["lib.py", "main.py"]
    );
}

const NESTED: &str = "import lib
total = 10
def outer(a):
    b = a * 2
    return lib.inner(b)
log(outer(3), total)
wait()
";

const LIB: &str = "scale = 4
class Box:
    def __init__(self, v):
        self.v = v
def inner(n):
    box = Box(n)
    return box.v * scale
";

/// A machine stopped at `lib.py`'s line 7, inside two calls.
fn stopped_in_inner() -> (Machine, LogHost, Lines) {
    let (costs, limits) = tables();
    let program = load(&[("main.py", NESTED), ("lib.py", LIB)]);
    let mut m = Machine::new(&program, costs, limits);
    let mut host = LogHost::default();
    let mut lines = Lines {
        stop_at: Some(7),
        ..Lines::default()
    };
    m.tick = 1;
    assert_eq!(
        m.run_slice_traced(&mut host, Some(&mut lines)),
        Slice::Stopped
    );
    assert_eq!(m.beginning(), Some(("lib.py", 7)));
    (m, host, lines)
}

/// Run a stopped machine on to its `wait()`.
fn finish(m: &mut Machine, host: &mut LogHost, lines: &mut Lines) {
    lines.stop_at = None;
    let mut slice = m.resume_slice(host, Some(lines));
    for tick in 2..=1000u64 {
        if slice == Slice::Wait {
            return;
        }
        m.tick = tick;
        slice = m.run_slice_traced(host, Some(lines));
    }
    panic!("the program never reached its wait");
}

#[test]
fn a_stopped_machine_shows_its_frames_and_globals() {
    let (m, _, _) = stopped_in_inner();
    let seen = m.inspect(200);
    let frames: Vec<(&str, u32, &str)> = seen
        .frames
        .iter()
        .map(|f| (f.file.as_str(), f.line, f.function.as_str()))
        .collect();
    assert_eq!(
        frames,
        vec![
            ("main.py", 6, "<module>"),
            ("main.py", 5, "outer"),
            ("lib.py", 7, "inner"),
        ]
    );
    assert_eq!(
        seen.frames[1].locals,
        vec![
            ("a".to_string(), Some("3".to_string())),
            ("b".to_string(), Some("6".to_string())),
        ]
    );
    assert_eq!(
        seen.frames[2].locals,
        vec![
            ("n".to_string(), Some("6".to_string())),
            ("box".to_string(), Some("Box(v=6)".to_string())),
        ]
    );
    let names: Vec<&str> = seen.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["main", "lib"]);
    assert!(
        seen.modules[0]
            .globals
            .contains(&("total".to_string(), "10".to_string()))
    );
    assert!(
        seen.modules[1]
            .globals
            .contains(&("scale".to_string(), "4".to_string()))
    );
    assert_eq!(seen.budget, 200);
    assert!(seen.spent > 0 && seen.spent < 200, "spent {}", seen.spent);
    assert_eq!(seen.deficit, 0);
    assert!(seen.pending.is_empty());
    assert_eq!(seen.handler, None);
    assert!(!seen.waiting && !seen.dead);
    // Reading it changed nothing.
    let before = m.state_bytes();
    let _ = m.inspect(3);
    assert!(m.state_bytes() == before);
}

#[test]
fn a_shown_value_is_cut_off_and_survives_a_cycle() {
    let (costs, limits) = tables();
    let program = load(&[(
        "main.py",
        "a = [1, 'two', (3,), {'k': {4}}]\na.append(a)\nwait()\n",
    )]);
    let mut m = Machine::new(&program, costs, limits);
    m.tick = 1;
    assert_eq!(m.run_slice(&mut LogHost::default()), Slice::Wait);
    let shown = |chars| {
        m.inspect(chars).modules[0]
            .globals
            .iter()
            .find(|(k, _)| k == "a")
            .map(|(_, v)| v.clone())
            .expect("a")
    };
    let full = shown(100_000);
    assert!(
        full.starts_with("[1, 'two', (3,), {'k': {4}}, [1, 'two', (3,), {'k': {4}}, ["),
        "{full}"
    );
    assert!(full.contains("..."), "the cycle was not cut off: {full}");
    assert_eq!(shown(9), "[1, 'two'…");
}

#[test]
fn an_edit_sets_a_local_or_a_global_and_the_program_runs_on_with_it() {
    let (mut m, mut host, mut lines) = stopped_in_inner();
    let local = |depth: usize, name: &str| EditTarget::Local {
        depth,
        name: name.to_string(),
    };
    let global = |module: &str, name: &str| EditTarget::Global {
        module: module.to_string(),
        name: name.to_string(),
    };
    m.edit(&global("lib", "scale"), "-5").expect("scale");
    m.edit(
        &global("main", "total"),
        "[1, {'k': (2, -3.5)}, {4, 4}, None, True]",
    )
    .expect("total");
    m.edit(&local(1, "b"), "'text'").expect("b");
    let seen = m.inspect(200);
    assert_eq!(seen.frames[1].locals[1].1.as_deref(), Some("'text'"));
    assert!(seen.modules[0].globals.contains(&(
        "total".to_string(),
        "[1, {'k': (2, -3.5)}, {4}, None, True]".to_string()
    )));
    finish(&mut m, &mut host, &mut lines);
    // `box.v * scale` with the edited scale, beside the edited global.
    assert_eq!(host.log, vec!["-30 [1, {'k': (2, -3.5)}, {4}, None, True]"]);
}

#[test]
fn an_edit_that_is_no_literal_or_names_nothing_is_refused() {
    let (mut m, _, _) = stopped_in_inner();
    let before = m.state_bytes();
    let scale = EditTarget::Global {
        module: "lib".to_string(),
        name: "scale".to_string(),
    };
    for text in [
        "scale",
        "1 + 2",
        "inner(1)",
        "[1, total]",
        "+1",
        "-'a'",
        "1\nscale = 2",
        "[x for x in [1]]",
        "{[1]: 2}",
        "{[1]}",
        "",
        "(",
    ] {
        assert!(
            m.edit(&scale, text).is_err(),
            "{text:?} was taken as a literal"
        );
    }
    for target in [
        EditTarget::Global {
            module: "lib".to_string(),
            name: "absent".to_string(),
        },
        EditTarget::Global {
            module: "never_run".to_string(),
            name: "scale".to_string(),
        },
        EditTarget::Local {
            depth: 9,
            name: "n".to_string(),
        },
        EditTarget::Local {
            depth: 2,
            name: "absent".to_string(),
        },
    ] {
        assert!(m.edit(&target, "1").is_err(), "{target:?} was set");
    }
    assert!(
        m.state_bytes() == before,
        "a refused edit changed the machine"
    );
}

/// An edit is held to the limits the program's own assignment is, and a
/// refused one leaves the machine as it was — what it goes on to do too.
#[test]
fn an_edit_over_a_limit_is_refused_naming_it() {
    let (costs, limits) = tables();
    let program = load(&[("main.py", NESTED), ("lib.py", LIB)]);
    let scale = EditTarget::Global {
        module: "lib".to_string(),
        name: "scale".to_string(),
    };
    let run = |limits: Rc<Limits>, text: Option<&str>| {
        let mut m = Machine::new(&program, costs.clone(), limits);
        let mut host = LogHost::default();
        let mut lines = Lines {
            stop_at: Some(7),
            ..Lines::default()
        };
        m.tick = 1;
        assert_eq!(
            m.run_slice_traced(&mut host, Some(&mut lines)),
            Slice::Stopped
        );
        let refusal = text.map(|t| m.edit(&scale, t));
        finish(&mut m, &mut host, &mut lines);
        (refusal, host.log, m.state_bytes())
    };
    let untouched = run(limits.clone(), None);

    // The value's own size.
    let too_long = format!("[{}]", vec!["0"; 10_001].join(", "));
    let (refusal, log, state) = run(limits.clone(), Some(&too_long));
    let why = refusal
        .expect("edited")
        .expect_err("a list over the size limit");
    assert!(
        why.contains("LimitError") && why.contains("collection"),
        "{why}"
    );
    assert!(log == untouched.1 && state == untouched.2);
    let fits = format!("[{}]", vec!["0"; 10_000].join(", "));
    assert_eq!(run(limits.clone(), Some(&fits)).0, Some(Ok(())));

    // The machine's live values, counted with the new value in place.
    let tight = Rc::new(Limits {
        size_live_values: 40,
        ..(*limits).clone()
    });
    let untouched = run(tight.clone(), None);
    let (refusal, log, state) = run(
        tight.clone(),
        Some(
            "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40]",
        ),
    );
    let why = refusal
        .expect("edited")
        .expect_err("over the live-values limit");
    assert!(
        why.contains("LimitError") && why.contains("live values"),
        "{why}"
    );
    assert!(log == untouched.1 && state == untouched.2);
    assert_eq!(run(tight, Some("[1, 2, 3]")).0, Some(Ok(())));
}

/// What a step compares between boundaries: the stack's depth, and the
/// counters of restarts, abandoned stacks and `dying`s.
#[test]
fn the_trace_counts_restarts_and_abandoned_stacks() {
    let (costs, limits) = tables();
    let program = load(&[("main.py", "def on_fault(e):\n    pass\nx = [][1]\n")]);
    let mut m = Machine::new(&program, costs, limits);
    let start = m.trace();
    assert_eq!(m.depth(), 1);
    m.tick = 1;
    assert_eq!(m.run_slice(&mut LogHost::default()), Slice::Restarted);
    let after = m.trace();
    // The fault abandoned main flow's stack, and its epilogue restarted it.
    assert_eq!(after.restarts, start.restarts.wrapping_add(1));
    assert_eq!(after.abandons, start.abandons.wrapping_add(2));
    assert_eq!(after.dyings, 0);
    m.raise(lang::Interrupt::Dying);
    m.tick = 2;
    m.run_slice(&mut LogHost::default());
    assert_eq!(m.trace().dyings, 1);
}
