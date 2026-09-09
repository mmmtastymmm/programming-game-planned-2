//! Whole programs through the interpreter: the behaviour
//! `docs/01-language` specifies, checked at the boundary a machine sees —
//! slices, globals, faults, and what the host was asked for.
//!
//! Every assertion here is on a *value*: a golden-replay style hash would
//! pass on any deterministic bug. `determinism.rs` is the one that pins the
//! trace.

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, ExcClass, Host, HostCall, Limits, Machine, Program, Slice, Value};
use std::rc::Rc;

/// A host with `log(...)` and a waiting `wait()`, recording every call.
/// Names come from the cost table's `[game]` rows; an unknown one is a
/// `NameError` before the host is asked.
#[derive(Default)]
struct TestHost {
    log: Vec<String>,
    calls: Vec<String>,
}

impl Host for TestHost {
    fn call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        _kwargs: Vec<(String, Value)>,
    ) -> lang::value::R<HostCall> {
        self.calls.push(name.to_string());
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

fn load(src: &str) -> Program {
    let (_, limits) = tables();
    Program::load(&[("main.py", src)], &limits).unwrap_or_else(|e| panic!("{e}"))
}

/// Run until main flow restarts, or fault. Ticks are capped so a runaway
/// program fails the test instead of hanging it.
fn run_to_end(src: &str) -> (Machine, TestHost, Slice, u32) {
    let (costs, limits) = tables();
    let program = load(src);
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    let mut ticks = 0u32;
    loop {
        ticks = ticks.wrapping_add(1);
        assert!(ticks < 10_000, "program did not finish in 10000 ticks");
        m.tick = u64::from(ticks);
        match m.run_slice(&mut host) {
            Slice::Yield => {}
            other => return (m, host, other, ticks),
        }
    }
}

/// Run a program that must complete normally; return its final globals via
/// the machine, plus the host's log.
fn run(src: &str) -> (Machine, Vec<String>) {
    let (m, host, end, _) = run_to_end(src);
    assert_eq!(
        end,
        Slice::Restarted,
        "program faulted: {end:?}\nlog: {:?}",
        host.log
    );
    (m, host.log)
}

/// Run a program and expect it to fault with `class`.
fn fault(src: &str) -> lang::Exception {
    let (_, host, end, _) = run_to_end(src);
    match end {
        Slice::Fault(e) => e,
        other => panic!("expected a fault, got {other:?}; log {:?}", host.log),
    }
}

fn log_of(src: &str) -> Vec<String> {
    run(src).1
}

fn load_error(src: &str) -> String {
    let (_, limits) = tables();
    match Program::load(&[("main.py", src)], &limits) {
        Ok(_) => panic!("loaded, but should have been refused:\n{src}"),
        Err(e) => e.message,
    }
}

// --------------------------------------------------------------- numbers ---

#[test]
fn numbers_floor_everywhere() {
    let log = log_of(
        "log(1 / 3)\nlog(-7 // 2)\nlog(-7 % 3)\nlog(2 ** 10)\nlog(0.1 + 0.2 == 0.3)\nlog(7 / 2)\nlog(-1 / 3)\nlog(int(-1.5))\nlog(round(2.5), round(3.5), round(-2.5))\nlog(3 ** -2)\nlog(10 ** 20)\n",
    );
    assert_eq!(
        log,
        [
            "0.333333333333",
            "-4",
            "2",
            "1024",
            "True",
            "3.5",
            "-0.333333333334",
            "-1",
            "2 4 -2",
            "0.11111111111",
            "100000000000000000000",
        ]
    );
}

#[test]
fn overflow_and_zero_division_are_exceptions() {
    assert_eq!(
        fault("x = 10 ** 20 * 10 ** 20\n").class,
        ExcClass::OverflowError
    );
    assert_eq!(fault("x = 1 / 0\n").class, ExcClass::ZeroDivisionError);
    assert_eq!(fault("x = 1 % 0\n").class, ExcClass::ZeroDivisionError);
    assert_eq!(fault("x = 0 ** -1\n").class, ExcClass::ZeroDivisionError);
    assert_eq!(fault("x = 2 ** 0.5\n").class, ExcClass::TypeError);
}

#[test]
fn a_literal_past_twelve_places_is_a_load_error() {
    assert!(load_error("x = 0.0000000000001\n").contains("literal"));
    assert!(load_error("x = 1e40\n").contains("literal"));
    assert_eq!(log_of("log(1_000.5e-3)\n"), ["1.0005"]);
}

#[test]
fn bool_is_a_num_and_comparisons_chain() {
    let log = log_of(
        "log(True + True)\nlog(0 <= 1 < 2)\nlog(0 <= 2 < 2)\nlog(1 == True)\nlog('a' < 'b')\nlog((1, 2) < (1, 3))\nlog(isinstance(True, num))\n",
    );
    assert_eq!(log, ["2", "True", "False", "True", "True", "True", "True"]);
    assert_eq!(fault("x = 1 < 'a'\n").class, ExcClass::TypeError);
}

// --------------------------------------------------------------- strings ---

#[test]
fn f_strings_and_the_numeric_specs() {
    let log = log_of(
        "x = -2 / 3\nlog(f'{x:.2f}|{-0.4:.0f}|{1234567.5:,}|{7:03d}|{\"ab\":>4}|{\"ab\":<4}|{\"ab\":^5}|{x!s}|{\"abc\":^6}|{\"abcdefg\":^3}')\nlog(f'{{literal}} {1 + 1}')\nlog(str(1.50), str(-0.5), str(2))\n",
    );
    assert_eq!(
        log,
        [
            "-0.67|-1|1,234,567.5|007|  ab|ab  | ab  |-0.666666666667| abc  |abcdefg",
            "{literal} 2",
            "1.5 -0.5 2"
        ]
    );
    assert_eq!(fault("x = f'{1.5:d}'\n").class, ExcClass::TypeError);
    assert_eq!(fault("x = f'{1:.13f}'\n").class, ExcClass::ValueError);
}

#[test]
fn string_methods_and_escapes() {
    let log = log_of(
        r#"s = "  a,b,,c  "
log(s.strip().split(","))
log("-".join(["x", "y"]))
log("hello".upper(), "HeLLo".lower(), "abc".find("c"), "abc".find("z"))
log("a\tb\n".strip(), len("héllo"), "héllo"[1], "abc"[-1], "abcdef"[1:5:2], "abc"[::-1])
log("42".isdigit(), "".isdigit(), "ab".isalpha(), "-5".zfill(4), "ab".replace("b", "c"))
log("x" in "xyz", "x" * 3, "ab" + "cd", "a" < "b", "abc".startswith("ab"))
log('\x41\u00e9\U00000061')
"#,
    );
    assert_eq!(
        log,
        [
            "['a', 'b', '', 'c']",
            "x-y",
            "HELLO hello 2 -1",
            "a\tb 5 é c bd cba",
            "True False True -005 ac",
            "True xxx abcd True True",
            "Aéa",
        ]
    );
    assert_eq!(fault("x = 'a' + 1\n").class, ExcClass::TypeError);
    assert_eq!(fault("x = 'abc'[3]\n").class, ExcClass::IndexError);
}

// ----------------------------------------------------------- collections ---

#[test]
fn lists_dicts_sets_and_tuples() {
    let log = log_of(
        r#"xs = [3, 1, 2]
xs.append(0)
xs.extend((9, 8))
xs.insert(0, 7)
log(xs, xs.pop(), xs.pop(0), xs, xs.index(2), xs.count(1))
xs.sort()
log(xs, sorted(xs, reverse=True), xs[1:3], xs[-1], len(xs))
xs.reverse()
log(xs, [x * 2 for x in xs if x > 1], sum(xs), min(xs), max(xs), any([0, 1]), all([]))
d = {"b": 2, "a": 1}
d["c"] = 3
log(d, list(d.keys()), d.get("z", 0), d.pop("a"), d.setdefault("q", 5), "b" in d, len(d))
log({k: v for k, v in d.items()}, dict(a=1), dict([("x", 1)]), dict(d, b=20))
s = {3, 1, 2, 1}
s.add(4)
s.discard(9)
log(sorted(s), sorted(s | {5}), sorted(s & {1, 2, 99}), sorted(s - {1}), sorted(s ^ {1, 7}), 3 in s, {x % 2 for x in s})
t = (1, 2, 3)
a, *rest = t
log(t[0], t[-1], t + (4,), t * 2, rest, t.index(3), t.count(1), (1,), ())
log([1, 2] == [1, 2], [1, 2] != [1, 2], (1, [2]) == (1, [2]), [1, 2] < [1, 3], [] + [1])
for i, (k, v) in enumerate(sorted(d.items()), 1):
    log(i, k, v)
log(list(zip([1, 2, 3], "ab")), list(range(5, 0, -2)), list(range(3)))
"#,
    );
    assert_eq!(
        log,
        [
            "[3, 1, 2, 0, 9] 8 7 [3, 1, 2, 0, 9] 2 1",
            "[0, 1, 2, 3, 9] [9, 3, 2, 1, 0] [1, 2] 9 5",
            "[9, 3, 2, 1, 0] [18, 6, 4] 15 0 9 True True",
            "{'b': 2, 'c': 3, 'q': 5} ['b', 'a', 'c'] 0 1 5 True 3",
            "{'b': 2, 'c': 3, 'q': 5} {'a': 1} {'x': 1} {'b': 20, 'c': 3, 'q': 5}",
            "[1, 2, 3, 4] [1, 2, 3, 4, 5] [1, 2] [2, 3, 4] [2, 3, 4, 7] True {1, 0}",
            "1 3 (1, 2, 3, 4) (1, 2, 3, 1, 2, 3) [2, 3] 2 1 (1,) ()",
            "True False True True [1]",
            "1 b 2",
            "2 c 3",
            "3 q 5",
            "[(1, 'a'), (2, 'b')] [5, 3, 1] [0, 1, 2]",
        ]
    );
}

#[test]
fn collection_errors() {
    assert_eq!(fault("x = [1][5]\n").class, ExcClass::IndexError);
    assert_eq!(fault("x = {}['k']\n").class, ExcClass::KeyError);
    assert_eq!(fault("x = {[1]: 2}\n").class, ExcClass::TypeError);
    assert_eq!(
        fault("x = {1: 2}.remove(1)\n").class,
        ExcClass::AttributeError
    );
    assert_eq!(fault("x = [1].pop(4)\n").class, ExcClass::IndexError);
    assert_eq!(fault("x = [3, 'a']\nx.sort()\n").class, ExcClass::TypeError);
    assert_eq!(fault("x = (1, 2)\nx[0] = 5\n").class, ExcClass::TypeError);
    assert_eq!(fault("a, b = [1, 2, 3]\n").class, ExcClass::ValueError);
    assert_eq!(fault("x = set().pop()\n").class, ExcClass::KeyError);
    assert_eq!(
        fault("x = list(range(10001))\n").class,
        ExcClass::LimitError
    );
    assert_eq!(fault("x = 'a' * 20000\n").class, ExcClass::LimitError);
}

#[test]
fn sorting_is_stable_and_takes_a_key() {
    let log = log_of(
        "xs = [(1, 'b'), (0, 'a'), (1, 'a'), (0, 'b')]\nlog(sorted(xs, key=len))\nlog(sorted(['bb', 'a', 'ccc', 'dd'], key=len))\nlog(sorted(['bb', 'a', 'ccc', 'dd'], key=len, reverse=True))\nys = [3, 1, 2]\nys.sort(reverse=True)\nlog(ys)\n",
    );
    assert_eq!(
        log,
        [
            "[(1, 'b'), (0, 'a'), (1, 'a'), (0, 'b')]",
            "['a', 'bb', 'dd', 'ccc']",
            "['ccc', 'bb', 'dd', 'a']",
            "[3, 2, 1]",
        ]
    );
}

// ------------------------------------------------------------- functions ---

#[test]
fn functions_defaults_star_args_and_recursion() {
    let log = log_of(
        r#"def f(a, b=2, *rest, **kw):
    return (a, b, rest, sorted(kw.items()))

log(f(1))
log(f(1, 3, 4, 5, key=6, z=7))
log(f(*[1, 2, 3], **{"key": 9}))

def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

log(fib(15))

total = [0]
def add(x):
    total[0] += x
add(5)
add(6)
log(total[0])
sq = lambda x: x * x
log(sq(7), [sq(i) for i in (1, 2)])
"#,
    );
    assert_eq!(
        log,
        [
            "(1, 2, (), [])",
            "(1, 3, (4, 5), [('key', 6), ('z', 7)])",
            "(1, 2, (3,), [('key', 9)])",
            "610",
            "11",
            "49 [1, 4]",
        ]
    );
}

#[test]
fn function_call_errors() {
    assert_eq!(
        fault("def f(a):\n    pass\nf()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("def f(a):\n    pass\nf(1, 2)\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("def f(a):\n    pass\nf(1, a=2)\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("def f():\n    return f()\nf()\n").class,
        ExcClass::RecursionError
    );
    assert_eq!(fault("x = undefined_name\n").class, ExcClass::NameError);
    assert_eq!(
        fault("def f():\n    return x\n    x = 1\nf()\n").class,
        ExcClass::UnboundLocalError
    );
    assert_eq!(fault("x = (1).foo\n").class, ExcClass::AttributeError);
    assert_eq!(fault("x = 'a'.encode\n").class, ExcClass::AttributeError);
    assert_eq!(fault("x = 5()\n").class, ExcClass::TypeError);
}

#[test]
fn a_lambda_cannot_name_an_enclosing_local() {
    // syntax.md: a lambda sees globals and its own parameters, nothing in
    // between. Rejected at load, not at call.
    assert!(load_error("def f(y):\n    return lambda x: x + y\n").contains("lambda"));
}

// ---------------------------------------------------------- control flow ---

#[test]
fn loops_else_break_continue_and_unpacking() {
    let log = log_of(
        r#"out = []
for i in range(6):
    if i == 1:
        continue
    if i == 4:
        break
    out.append(i)
else:
    out.append("no")
log(out)
for i in range(2):
    pass
else:
    log("else")
i = 0
while i < 3:
    i += 1
    if i == 2:
        continue
    log("w", i)
else:
    log("wend")
while True:
    break
else:
    log("never")
for a, (b, c) in [(1, (2, 3)), (4, (5, 6))]:
    log(a + b + c)
first, *mid, last = range(5)
log(first, mid, last)
xs = [1, 2, 3]
xs[0], xs[2] = xs[2], xs[0]
log(xs, [y for x in [[1, 2], [3]] for y in x], {i: i * i for i in range(3)})
log(1 if True else 2, 0 or "x", 1 and 2, not 0, None is None, None is not None)
"#,
    );
    assert_eq!(
        log,
        [
            "[0, 2, 3]",
            "else",
            "w 1",
            "w 3",
            "wend",
            "6",
            "15",
            "0 [1, 2, 3] 4",
            "[3, 2, 1] [1, 2, 3] {0: 0, 1: 1, 2: 4}",
            "1 x 2 True True False",
        ]
    );
}

#[test]
fn mutating_a_list_while_iterating_sees_the_snapshot() {
    // execution.md: `for` iterates a snapshot; appending inside the loop
    // cannot extend it.
    let log = log_of("xs = [1, 2]\nfor x in xs:\n    xs.append(x)\nlog(xs)\n");
    assert_eq!(log, ["[1, 2, 1, 2]"]);
}

#[test]
fn try_except_finally_and_the_class_tree() {
    let log = log_of(
        r#"def risky(k):
    try:
        if k == 0:
            return 1 / 0
        if k == 1:
            return [][1]
        if k == 2:
            raise ValueError("bad", 7)
        return "ok"
    except ZeroDivisionError as e:
        return ("zd", isinstance(e, Exception), isinstance(e, IndexError))
    except (IndexError, KeyError) as e:
        return ("lookup", e.args)
    except Exception as e:
        return ("exc", e.args, e.line, e.file)
    finally:
        log("finally", k)

for k in range(4):
    log(risky(k))

try:
    try:
        raise KeyError("inner")
    finally:
        log("cleanup")
except KeyError as e:
    log("outer", e.args)

try:
    raise TypeError()
except TypeError:
    try:
        raise
    except TypeError as again:
        log("re-raised", again.args)

try:
    raise LimitError()
except Exception as e:
    log("caught", e, str(ValueError("bad")), str(KeyError(1, 2)))
try:
    def g():
        x = 1
        def_local = x
    raise UnboundLocalError("v")
except NameError as e:
    log("UnboundLocalError is a NameError", e.args)

def f():
    try:
        return "try"
    finally:
        log("finally runs on return")
log(f())
"#,
    );
    assert_eq!(
        log,
        [
            "finally 0",
            "('zd', True, False)",
            "finally 1",
            "('lookup', (1,))",
            "finally 2",
            "('exc', ('bad', 7), 8, 'main.py')",
            "finally 3",
            "ok",
            "cleanup",
            "outer ('inner',)",
            "re-raised ()",
            "caught LimitError ValueError: bad KeyError: 1, 2",
            "UnboundLocalError is a NameError ('v',)",
            "finally runs on return",
            "try",
        ]
    );
}

#[test]
fn raising_a_non_exception_is_a_type_error() {
    assert_eq!(fault("raise 5\n").class, ExcClass::TypeError);
    // Bare `raise` outside an `except`: nothing to re-raise, so the root.
    assert_eq!(fault("raise\n").class, ExcClass::Exception);
    // The clause is only looked at once something is raised.
    assert_eq!(
        fault("try:\n    raise ValueError()\nexcept 5:\n    pass\n").class,
        ExcClass::TypeError
    );
}

// --------------------------------------------------------- the boundary ---

#[test]
fn what_is_not_in_the_language_is_refused_at_load() {
    for (src, word) in [
        ("class A:\n    pass\n", "class"),
        ("import x\n", "import"),
        ("match x:\n    case 1:\n        pass\n", "match"),
        ("x = 1\ndel x\n", "del"),
        ("def f():\n    def g():\n        pass\n", "nest"),
        ("x = (i for i in range(3))\n", "generator"),
        ("def f():\n    yield 1\n", "yield"),
        ("x = 1 if 1 is 1 else 2\n", "is"),
        ("x = 1 << 2\n", "<<"),
        ("x = (y := 1)\n", ":="),
        ("async def f():\n    pass\n", "async"),
        ("with x:\n    pass\n", "with"),
        ("x = 0x1f\n", "hex"),
        ("x = 1\n  y = 2\n", "indent"),
        ("x = [1,\n", "closed"),
        ("def f(a, a):\n    pass\n", "duplicate"),
        ("x = 1\ndef f():\n    global x\n    x = 2\n", "global"),
        ("def f():\n    nonlocal x\n", "nonlocal"),
        ("x = 1e400\n", "literal"),
        ("x = \"\\q\"\n", "escape"),
    ] {
        let msg = load_error(src);
        assert!(
            msg.to_lowercase().contains(word),
            "{src:?}: message {msg:?} does not name {word:?}"
        );
    }
}

#[test]
fn identifiers_and_strings_are_ascii_classed() {
    // syntax.md: identifiers are ASCII; strings may hold any scalar.
    assert!(load_error("é = 1\n").contains("not part of the language"));
    assert_eq!(log_of("log(len('é'))\n"), ["1"]);
}

// ----------------------------------------------------------- the machine ---

#[test]
fn a_program_yields_when_the_budget_is_spent_and_carries_a_deficit() {
    let (costs, limits) = tables();
    let program = load("n = 0\nwhile True:\n    n += 1\n");
    let mut m = Machine::new(&program, costs, limits.clone());
    let mut host = TestHost::default();
    assert_eq!(m.run_slice(&mut host), Slice::Yield);
    let after_one = m.global("n").unwrap().expect_integral().unwrap();
    assert!(after_one > 0, "no progress in a tick");
    assert_eq!(m.run_slice(&mut host), Slice::Yield);
    let after_two = m.global("n").unwrap().expect_integral().unwrap();
    let per_tick = after_two.wrapping_sub(after_one);
    assert!(per_tick > 0, "{after_one} then {after_two}");
    // Steady state: every tick makes the same progress, so the budget is
    // a function of the table and nothing else.
    for _ in 0..5 {
        let before = m.global("n").unwrap().expect_integral().unwrap();
        assert_eq!(m.run_slice(&mut host), Slice::Yield);
        let after = m.global("n").unwrap().expect_integral().unwrap();
        assert!(
            (after - before - per_tick).abs() <= 1,
            "{before} -> {after}, expected ~{per_tick}"
        );
    }
}

#[test]
fn an_empty_program_restarts_once_per_tick() {
    let (costs, limits) = tables();
    let program = load("");
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
}

#[test]
fn a_restart_clears_every_global() {
    let (costs, limits) = tables();
    let program = load("x = 1\n");
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
    // Main flow ended and restarted: the *next* slice starts from the top,
    // with `x` not yet bound.
    assert!(m.global("x").is_none(), "globals survived a restart");
}

#[test]
fn a_fault_writes_the_record_restarts_and_survives_a_redeploy() {
    let (costs, limits) = tables();
    let program = load("x = 1\ny = 2\nz = 1 / 0\n");
    let mut m = Machine::new(&program, costs, limits.clone());
    let mut host = TestHost::default();
    m.tick = 42;
    let Slice::Fault(e) = m.run_slice(&mut host) else {
        panic!("no fault")
    };
    assert_eq!(e.class, ExcClass::ZeroDivisionError);
    assert_eq!(
        (e.file.as_deref(), e.line, e.tick),
        (Some("main.py"), 3, 42)
    );
    assert_eq!(m.fault_record.as_ref().map(|r| r.line), Some(3));
    assert!(m.global("x").is_none(), "globals survived a fault");
    let fresh = load("w = 5\n");
    m.swap_program(&fresh);
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
    assert_eq!(
        m.fault_record.as_ref().map(|r| r.line),
        Some(3),
        "the fault record did not survive redeploy"
    );
}

#[test]
fn a_waiting_host_call_suspends_and_resumes_with_its_result() {
    let (costs, limits) = tables();
    let program = load("r = wait()\nlog(r)\n");
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    assert_eq!(m.run_slice(&mut host), Slice::Wait);
    assert_eq!(m.run_slice(&mut host), Slice::Wait, "a waiting machine ran");
    m.resume(Value::str("done"));
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
    assert_eq!(host.log, ["done"]);
    assert_eq!(host.calls, ["wait", "log"]);
}

#[test]
fn a_host_exception_is_the_program_s_exception() {
    struct Raising;
    impl Host for Raising {
        fn call(
            &mut self,
            _: &str,
            _: Vec<Value>,
            _: Vec<(String, Value)>,
        ) -> lang::value::R<HostCall> {
            Err(lang::Exception::new(
                ExcClass::ValueError,
                vec![Value::str("blocked")],
            ))
        }
    }
    let (costs, limits) = tables();
    let program =
        load("try:\n    move(1, 2)\nexcept ValueError as e:\n    caught = e.args\n    move(3)\n");
    let mut m = Machine::new(&program, costs, limits);
    let Slice::Fault(e) = m.run_slice(&mut Raising) else {
        panic!("the second raise did not fault")
    };
    assert_eq!((e.class, e.line), (ExcClass::ValueError, 5));
}

#[test]
fn an_unknown_name_is_a_name_error_even_for_a_host_call() {
    assert_eq!(fault("no_such_thing()\n").class, ExcClass::NameError);
}

#[test]
fn the_bundle_hash_is_over_bytes_in_sorted_name_order() {
    let (_, limits) = tables();
    let a = Program::load(&[("main.py", "x = 1\n")], &limits).unwrap();
    let b = Program::load(&[("main.py", "x = 1\n")], &limits).unwrap();
    let c = Program::load(&[("main.py", "x = 1 \n")], &limits).unwrap();
    assert_eq!(a.version, b.version);
    assert_ne!(a.version, c.version, "whitespace was normalised");
    assert!(
        Program::load(&[("other.py", "x = 1\n")], &limits).is_err(),
        "a bundle without main.py loaded"
    );
    assert!(
        Program::load(&[("Main.py", "x = 1\n")], &limits).is_err(),
        "a bad file name loaded"
    );
}
