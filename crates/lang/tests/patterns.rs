//! `match` / `case` (T12): every pattern form in `docs/01-language/syntax.md`'s
//! `match` table, the binding rules, and the boundary the parser draws.

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, ExcClass, Host, HostCall, Limits, Machine, Program, Slice, Value};
use std::rc::Rc;

#[derive(Default)]
struct TestHost {
    log: Vec<String>,
}

impl Host for TestHost {
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

fn run_to_end(src: &str) -> (TestHost, Slice) {
    let (costs, limits) = tables();
    let program = Program::load(&[("main.py", src)], &limits).unwrap_or_else(|e| panic!("{e}"));
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    let mut ticks = 0u32;
    loop {
        ticks = ticks.wrapping_add(1);
        assert!(ticks < 10_000, "program did not finish in 10000 ticks");
        m.tick = u64::from(ticks);
        match m.run_slice(&mut host) {
            Slice::Yield => {}
            other => return (host, other),
        }
    }
}

fn log_of(src: &str) -> Vec<String> {
    let (host, end) = run_to_end(src);
    assert_eq!(
        end,
        Slice::Restarted,
        "program faulted: {end:?}\nlog: {:?}",
        host.log
    );
    host.log
}

fn fault(src: &str) -> lang::Exception {
    let (host, end) = run_to_end(src);
    match end {
        Slice::Fault(e) => e,
        other => panic!("expected a fault, got {other:?}; log {:?}", host.log),
    }
}

fn load_error(src: &str) -> String {
    let (_, limits) = tables();
    match Program::load(&[("main.py", src)], &limits) {
        Ok(_) => panic!("loaded, but should have been refused:\n{src}"),
        Err(e) => e.message,
    }
}

/// `describe(x)` for each of `values`, through the arms of `arms`.
fn classify(arms: &str, values: &str) -> Vec<String> {
    let src = format!(
        "def describe(x):\n    match x:\n{arms}\n    return \"none\"\nfor v in {values}:\n    log(describe(v))\n"
    );
    log_of(&src)
}

#[test]
fn literal_patterns_and_the_singletons() {
    let log = classify(
        r#"        case 0:
            return "zero"
        case 1.5:
            return "one and a half"
        case -2:
            return "minus two"
        case "s":
            return "ess"
        case True:
            return "true"
        case False:
            return "false"
        case None:
            return "none-literal""#,
        r#"[0, 1.5, -2, "s", True, False, None, 1, "S", 0.0]"#,
    );
    // `1` does not match `case True:`. `True` is `1` by `==`, so it passes
    // `case 0:` and reaches `case True:`; `False` is `0` and stops there.
    assert_eq!(
        log,
        [
            "zero",
            "one and a half",
            "minus two",
            "ess",
            "true",
            "zero",
            "none-literal",
            "none",
            "none",
            "zero"
        ]
    );
    // `True` matches a literal `1` by `==`; `case True` needs a bool.
    assert_eq!(
        classify("        case 1:\n            return \"one\"", "[True]"),
        ["one"]
    );
    assert_eq!(
        classify("        case True:\n            return \"t\"", "[1]"),
        ["none"]
    );
}

#[test]
fn capture_wildcard_or_as_and_guards() {
    let log = classify(
        r#"        case 0 | 1 | 2 as small:
            return f"small {small}"
        case n if isinstance(n, num) and n < 0:
            return f"negative {n}"
        case ([n] | n) if n == 10:
            return f"ten {n}"
        case "a" | "b":
            return "ab"
        case _ if x == 99:
            return "guarded wildcard"
        case _:
            return "anything""#,
        r#"[1, -5, 10, [10], "b", 99, "zzz"]"#,
    );
    assert_eq!(
        log,
        [
            "small 1",
            "negative -5",
            "ten 10",
            "ten 10",
            "ab",
            "guarded wildcard",
            "anything"
        ]
    );
}

#[test]
fn sequence_patterns() {
    let log = classify(
        r#"        case []:
            return "empty"
        case [x]:
            return f"one {x}"
        case (a, b):
            return f"pair {a} {b}"
        case [first, *rest]:
            return f"first {first} rest {rest}"
        case _:
            return "not a sequence""#,
        r#"[[], (), [7], (1, 2), [1, 2], [1, 2, 3], (1, 2, 3, 4), "ab", {"a": 1}, 5]"#,
    );
    assert_eq!(
        log,
        [
            "empty",
            "empty",
            "one 7",
            "pair 1 2",
            "pair 1 2",
            "first 1 rest [2, 3]",
            "first 1 rest [2, 3, 4]",
            "not a sequence",
            "not a sequence",
            "not a sequence",
        ]
    );
    let log = classify(
        r#"        case [*_, last]:
            return f"last {last}"
        case _:
            return "none""#,
        "[[1, 2, 3], []]",
    );
    assert_eq!(log, ["last 3", "none"]);
    let log = classify(
        r#"        case [a, *mid, b, c]:
            return f"{a} {mid} {b} {c}"
        case _:
            return "none""#,
        "[[1, 2, 3, 4, 5], [1, 2, 3], [1, 2]]",
    );
    assert_eq!(log, ["1 [2, 3] 4 5", "1 [] 2 3", "none"]);
    // Nested sequences, and the star binding a list even from a tuple.
    let log = classify(
        r#"        case [(a, b), [c, *d]]:
            return f"{a} {b} {c} {d}"
        case _:
            return "none""#,
        "[[(1, 2), (3, 4, 5)], [[1, 2], [3]]]",
    );
    assert_eq!(log, ["1 2 3 [4, 5]", "1 2 3 []"]);
}

#[test]
fn mapping_patterns() {
    let log = classify(
        r#"        case {"kind": "bot", "hp": hp}:
            return f"bot {hp}"
        case {"kind": k, **rest}:
            return f"{k} {rest}"
        case {}:
            return "any dict"
        case _:
            return "not a dict""#,
        r#"[{"kind": "bot", "hp": 3, "x": 1}, {"kind": "depot", "hp": 9}, {"other": 1}, {}, [1], 1]"#,
    );
    assert_eq!(
        log,
        [
            "bot 3",
            "depot {'hp': 9}",
            "any dict",
            "any dict",
            "not a dict",
            "not a dict"
        ]
    );
    // Keys by key equality: `1` and `True` are one key; a nested pattern
    // on a value.
    let log = classify(
        r#"        case {1: [a, b]}:
            return f"{a} {b}"
        case _:
            return "none""#,
        "[{True: (5, 6)}, {1: [5]}, {2: [5, 6]}]",
    );
    assert_eq!(log, ["5 6", "none", "none"]);
}

#[test]
fn class_patterns_read_attributes_and_fail_without_raising() {
    let log = log_of(
        r#"class Machine:
    def __init__(self, kind, hp):
        self.kind = kind
        self.hp = hp

class Scout(Machine):
    pass

class Other:
    pass

def describe(x):
    match x:
        case Scout(hp=0):
            return "dead scout"
        case Scout(kind=k, hp=hp) if hp > 5:
            return f"healthy scout {k} {hp}"
        case Machine(kind="printer"):
            return "printer"
        case Machine(kind=k, missing=m):
            return "never"
        case Machine(hp=hp):
            return f"machine {hp}"
        case Other():
            return "other"
        case num(x=1):
            return "never"
        case num():
            return "a number"
        case str() | list():
            return "text or list"
        case ValueError(args=(a,)):
            return f"value error {a}"
        case Exception():
            return "some exception"
        case _:
            return "unknown"

for v in [Scout("s", 0), Scout("s", 9), Scout("s", 2), Machine("printer", 1), Machine("bot", 4), Other(), 3, "t", [], ValueError("bad"), ValueError(1, 2), KeyError("k"), None]:
    log(describe(v))
"#,
    );
    assert_eq!(
        log,
        [
            "dead scout",
            "healthy scout s 9",
            "machine 2",
            "printer",
            "machine 4",
            "other",
            "a number",
            "text or list",
            "text or list",
            "value error bad",
            "some exception",
            "some exception",
            "unknown",
        ]
    );
    // A class attribute counts as an attribute of the instance.
    assert_eq!(
        log_of("class A:\n    speed = 2\nmatch A():\n    case A(speed=s):\n        log(s)\n"),
        ["2"]
    );
    // The class expression must name a class: anything else is a TypeError.
    assert_eq!(
        fault("x = 1\nmatch 1:\n    case x():\n        pass\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("match 1:\n    case len():\n        pass\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("match 1:\n    case Undefined():\n        pass\n").class,
        ExcClass::NameError
    );
}

#[test]
fn bindings_of_a_failed_arm_persist_and_no_arm_is_fine() {
    let log = log_of(
        r#"def f(v):
    a = "unset"
    match v:
        case [a, 0]:
            pass
        case _:
            pass
    return a
log(f([1, 2]), f([3, 0]), f(5))
match 5:
    case 6:
        log("no")
log("fell through")
"#,
    );
    // `[1, 2]` binds `a = 1` before `0` fails to match `2`.
    assert_eq!(log, ["1 3 unset", "fell through"]);
}

#[test]
fn a_subject_is_evaluated_once_and_arms_stop_at_the_first_hit() {
    let log = log_of(
        r#"calls = []
def subject():
    calls.append(1)
    return 2
match subject():
    case 1:
        log("one")
    case 2:
        log("two")
    case 2:
        log("two again")
log(len(calls))
"#,
    );
    assert_eq!(log, ["two", "1"]);
}

#[test]
fn a_literal_pattern_dispatches_eq_on_an_instance_subject() {
    let log = log_of(
        r#"class Any:
    def __eq__(self, other):
        return other == 7
match Any():
    case 3:
        log("three")
    case 7:
        log("seven")
match Any():
    case True:
        log("true")
    case None:
        log("none")
    case _:
        log("neither singleton")
"#,
    );
    assert_eq!(log, ["seven", "neither singleton"]);
}

#[test]
fn match_inside_functions_loops_and_try() {
    let log = log_of(
        r#"def walk(xs):
    out = []
    for x in xs:
        match x:
            case ("stop",):
                break
            case ("skip", _):
                continue
            case ("add", n):
                out.append(n)
            case _:
                try:
                    match x:
                        case {"boom": True}:
                            raise ValueError("boom")
                except ValueError as e:
                    out.append(str(e))
    return out
log(walk([("add", 1), ("skip", 2), ("add", 3), {"boom": True}, {"boom": False}, ("stop",), ("add", 9)]))
"#,
    );
    assert_eq!(log, ["[1, 3, 'ValueError: boom']"]);
}

#[test]
fn a_pattern_that_binds_a_name_twice_or_unequal_alternatives_is_refused() {
    for (src, word) in [
        ("match x:\n    case [a, a]:\n        pass\n", "twice"),
        ("match x:\n    case a | b:\n        pass\n", "same names"),
        ("match x:\n    case [a, *a]:\n        pass\n", "twice"),
        ("match x:\n    case {1: a, **a}:\n        pass\n", "twice"),
        ("match x:\n    case (a as a):\n        pass\n", "twice"),
        ("match x:\n    case [*a, *b]:\n        pass\n", "one `*`"),
        ("match x:\n    case A(1):\n        pass\n", "keyword"),
        ("match x:\n    case A(k=1, k=2):\n        pass\n", "twice"),
        ("match x:\n    case Color.RED:\n        pass\n", "dotted"),
        ("match x:\n    case {a: 1}:\n        pass\n", "literals"),
        ("match x:\n    case *a:\n        pass\n", "sequence"),
        (
            "match x:\n    case 1 as _:\n        pass\n",
            "binds nothing",
        ),
        ("match x:\n    case if:\n        pass\n", "pattern"),
        ("match x:\n    pass\n", "case"),
        (
            "match x:\n    case 1:\n        pass\n    y = 2\n",
            "`case` arms only",
        ),
        ("match x: pass\n", "block"),
        (
            "match x:\n    case 1:\n        def f():\n            pass\n",
            "compound",
        ),
    ] {
        let msg = load_error(src);
        assert!(
            msg.contains(word),
            "{src:?}: message {msg:?} does not contain {word:?}"
        );
    }
}

#[test]
fn match_and_case_are_hard_keywords() {
    assert!(load_error("match = 1\n").contains("reserved"));
    assert!(load_error("case = 1\n").contains("`case`"));
    assert!(load_error("x = match\n").contains("`match`"));
}
