//! The two limits T10 left open: source nesting depth, a parse error at
//! load, and live values, a `LimitError` at the operation that crosses it
//! (`docs/01-language/execution.md`, Limits).

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, Event, ExcClass, Host, HostCall, Limits, Machine, Program, Slice, Value};
use std::rc::Rc;

struct Quiet;

impl Host for Quiet {
    fn call(
        &mut self,
        _: &str,
        _: Vec<Value>,
        _: Vec<(String, Value)>,
    ) -> lang::value::R<HostCall> {
        Ok(HostCall::Value(Value::None))
    }
}

fn tables() -> (Rc<Costs>, Rc<Limits>) {
    (
        Rc::new(Costs::parse(COSTS_TOML).expect("costs")),
        Rc::new(Limits::parse(LIMITS_TOML).expect("limits")),
    )
}

/// Run until main flow completes a run or faults; `Err` is the fault.
fn run(src: &str) -> Result<(), lang::Exception> {
    let (costs, limits) = tables();
    let program = Program::load(&[("main.py", src)], &limits).unwrap_or_else(|e| panic!("{e}"));
    let mut m = Machine::new(&program, costs, limits);
    for tick in 1..=20_000u64 {
        m.tick = tick;
        let slice = m.run_slice(&mut Quiet);
        for ev in m.take_events() {
            if let Event::Fault(e) = ev {
                return Err(e);
            }
        }
        if slice == Slice::Restarted {
            return Ok(());
        }
    }
    panic!("the program did not finish in 20000 ticks");
}

fn load_error(src: &str) -> String {
    let (_, limits) = tables();
    match Program::load(&[("main.py", src)], &limits) {
        Ok(_) => panic!("loaded, but should have been refused"),
        Err(e) => e.message,
    }
}

fn depth_limit() -> usize {
    let (_, limits) = tables();
    limits.depth_nesting as usize
}

#[test]
fn brackets_nested_past_the_limit_are_a_load_error() {
    let d = depth_limit();
    let ok = format!("x = {}1{}\n", "[".repeat(d), "]".repeat(d));
    assert!(run(&ok).is_ok(), "{d} nested brackets must load");
    let bad = format!(
        "x = {}1{}\n",
        "(".repeat(d.wrapping_add(1)),
        ")".repeat(d.wrapping_add(1))
    );
    let msg = load_error(&bad);
    assert!(
        msg.contains("nest") && msg.contains(&d.to_string()),
        "{msg}"
    );
    // Mixed brackets count together, and a pattern's brackets too.
    let mixed = format!("x = {}1{}\n", "([{".repeat(11), "}])".repeat(11));
    assert!(load_error(&mixed).contains("nest"));
    let pattern = format!(
        "match x:\n    case {}1{}:\n        pass\n",
        "[".repeat(d.wrapping_add(1)),
        "]".repeat(d.wrapping_add(1))
    );
    assert!(load_error(&pattern).contains("nest"));
}

#[test]
fn blocks_nested_past_the_limit_are_a_load_error() {
    let d = depth_limit();
    let nest = |n: usize| -> String {
        let mut src = String::new();
        for i in 0..n {
            src.push_str(&"    ".repeat(i));
            src.push_str("if True:\n");
        }
        src.push_str(&"    ".repeat(n));
        src.push_str("x = 1\n");
        src
    };
    assert!(run(&nest(d)).is_ok(), "{d} nested blocks must load");
    let msg = load_error(&nest(d.wrapping_add(1)));
    assert!(msg.contains("nest") && msg.contains("blocks"), "{msg}");
    // A `def` body and a `class` body are blocks too.
    let mut deep = String::from("def f():\n");
    for i in 1..=d {
        deep.push_str(&"    ".repeat(i));
        deep.push_str("while True:\n");
    }
    deep.push_str(&"    ".repeat(d.wrapping_add(1)));
    deep.push_str("break\n");
    assert!(load_error(&deep).contains("nest"));
}

#[test]
fn live_values_are_capped_at_the_crossing_operation() {
    // Ten lists of ten thousand cross the limit of one hundred thousand on
    // the tenth append: the fault is there, and the earlier nine are kept.
    let e = run("xs = []\nfor i in range(20):\n    xs.append(list(range(10000)))\n")
        .expect_err("no fault");
    assert_eq!((e.class, e.line), (ExcClass::LimitError, 3));
    // The same total built with a display rather than a method.
    let e = run("xs = [list(range(10000)) for i in range(20)]\n").expect_err("no fault");
    assert_eq!(e.class, ExcClass::LimitError);
}

#[test]
fn garbage_does_not_count_and_shared_objects_count_once() {
    // Thirty lists of nine thousand, each replacing the last: never more
    // than about nine thousand live at once.
    assert!(run("for i in range(30):\n    tmp = list(range(9000))\n").is_ok());
    // One object referenced from nine thousand slots counts once, plus the
    // slots; a tuple is a value, copied into every slot that holds it.
    assert!(
        run("l = list(range(10))\nxs = [l] * 9000\nys = [l] * 9000\nzs = [xs, ys] * 5000\n")
            .is_ok()
    );
    let e = run("t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10)\nxs = [t] * 9000\nys = [t] * 200\n")
        .expect_err("no fault");
    assert_eq!((e.class, e.line), (ExcClass::LimitError, 3));
    // Values reachable only from a frame that has returned are gone.
    assert!(run("def make():\n    return len([list(range(9000)) for i in range(10)])\nfor i in range(3):\n    n = make()\n").is_ok());
}

#[test]
fn instances_modules_and_frames_are_roots() {
    let e = run(
        "class Bag:\n    def __init__(self):\n        self.items = list(range(10000))\nbags = []\nfor i in range(12):\n    bags.append(Bag())\n",
    )
    .expect_err("no fault");
    // The crossing is the store inside `__init__`, on line 3: the list is
    // reachable from the new instance, which the frame's `self` holds.
    assert_eq!((e.class, e.line), (ExcClass::LimitError, 3));
    let (_, limits) = tables();
    let program = Program::load(
        &[
            (
                "main.py",
                "import heap\nmore = [list(range(10000)) for i in range(2)]\n",
            ),
            ("heap.py", "held = [list(range(10000)) for i in range(9)]\n"),
        ],
        &limits,
    )
    .unwrap();
    let (costs, _) = tables();
    let mut m = Machine::new(&program, costs, limits);
    let mut fault = None;
    for tick in 1..=20_000u64 {
        m.tick = tick;
        let slice = m.run_slice(&mut Quiet);
        for ev in m.take_events() {
            if let Event::Fault(e) = ev {
                fault = Some(e);
            }
        }
        if fault.is_some() || slice == Slice::Restarted {
            break;
        }
    }
    let e = fault.expect("a module's globals are live");
    assert_eq!(
        (e.class, e.file.as_deref()),
        (ExcClass::LimitError, Some("main.py"))
    );
}
