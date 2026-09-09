//! `class` (T11): single inheritance and the closed dunder set of
//! `docs/01-language/syntax.md`'s Classes section, checked end to end.
//! The interesting property is that every dispatch is a real call — paused
//! by the budget, faulting like any other — so several tests run programs
//! whose dunders are slow or raise.

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, ExcClass, Host, HostCall, Limits, Machine, Program, Slice, Value};
use std::rc::Rc;

#[derive(Default)]
struct TestHost {
    log: Vec<String>,
    waits: u32,
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
            "wait" => {
                self.waits = self.waits.wrapping_add(1);
                Ok(HostCall::Wait)
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

fn load(src: &str) -> Program {
    let (_, limits) = tables();
    Program::load(&[("main.py", src)], &limits).unwrap_or_else(|e| panic!("{e}"))
}

/// Run to the first restart or fault; waits are resumed with the tick.
fn run_to_end(src: &str) -> (TestHost, Slice, u32) {
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
            Slice::Wait => m.resume(Value::int(i128::from(ticks))),
            other => return (host, other, ticks),
        }
    }
}

fn log_of(src: &str) -> Vec<String> {
    let (host, end, _) = run_to_end(src);
    assert_eq!(
        end,
        Slice::Restarted,
        "program faulted: {end:?}\nlog: {:?}",
        host.log
    );
    host.log
}

fn fault(src: &str) -> lang::Exception {
    let (host, end, _) = run_to_end(src);
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

#[test]
fn attributes_methods_and_single_inheritance() {
    let log = log_of(
        r#"class Machine:
    speed = 1
    kind = "machine"

    def __init__(self, target):
        self.target = target
        self.trail = []

    def step(self):
        self.trail.append(self.target)
        return self.speed

    def describe(self):
        return f"{self.kind} at {self.target}"

class Scout(Machine):
    speed = 2

    def __init__(self, target, extra):
        Machine.__init__(self, target)
        self.extra = extra

    def describe(self):
        return "scout:" + Machine.describe(self)

m = Machine((1, 2))
s = Scout((3, 4), "x")
log(m.step(), s.step(), s.step(), m.trail, s.trail)
log(m.describe(), s.describe(), s.extra, s.kind, Scout.speed, Machine.speed)
Machine.kind = "bot"
log(m.kind, s.kind, isinstance(s, Machine), isinstance(m, Scout), isinstance(s, Scout), isinstance(3, Machine))
s.kind = "own"
log(s.kind, m.kind, str(m), str(Machine), str(m.step))
f = s.step
log(f(), s.trail)
"#,
    );
    assert_eq!(
        log,
        [
            "1 2 2 [(1, 2)] [(3, 4), (3, 4)]",
            "machine at (1, 2) scout:machine at (3, 4) x machine 2 1",
            "bot bot True False True False",
            "own bot <Machine> <class Machine> <function step>",
            "2 [(3, 4), (3, 4), (3, 4)]",
        ]
    );
}

#[test]
fn attribute_errors_and_construction_errors() {
    assert_eq!(
        fault("class A:\n    pass\nA().x\n").class,
        ExcClass::AttributeError
    );
    assert_eq!(
        fault("class A:\n    pass\nA.x\n").class,
        ExcClass::AttributeError
    );
    assert_eq!(
        fault("class A:\n    pass\nA(1)\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    def __init__(self):\n        return 1\nA()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    def __init__(self, x):\n        pass\nA()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    pass\nA().m()\n").class,
        ExcClass::AttributeError
    );
    assert_eq!(fault("class A(3):\n    pass\n").class, ExcClass::TypeError);
    assert_eq!(
        fault("class A:\n    def __init__(self):\n        A()\nA()\n").class,
        ExcClass::RecursionError
    );
    assert_eq!(fault("x = 1\nx.y = 2\n").class, ExcClass::TypeError);
}

#[test]
fn class_body_is_its_own_scope_and_methods_see_globals() {
    let log = log_of(
        r#"base = 10
class A:
    n = base + 1
    m = n * 2
    def get(self):
        return base
log(A.n, A.m, A().get())
"#,
    );
    assert_eq!(log, ["11 22 10"]);
    // Methods see globals, not class-body names.
    assert_eq!(
        fault("class A:\n    n = 1\n    def get(self):\n        return n\nA().get()\n").class,
        ExcClass::NameError
    );
}

#[test]
fn str_eq_and_lt_dispatch_with_the_instance_on_the_left() {
    let log = log_of(
        r#"class V:
    def __init__(self, x):
        self.x = x
    def __str__(self):
        return f"V{self.x}"
    def __eq__(self, other):
        return isinstance(other, V) and self.x == other.x
    def __lt__(self, other):
        return self.x < other.x

a, b, c = V(1), V(2), V(1)
log(a, [a, b], (a,), {"k": b}, f"{a}|{b:>4}|{a:<3}|")
log(a == c, a == b, a != c, 1 == a, a == 1, a is None, [a, b] == [c, b], (a, 1) == (c, 1), [a] == [b])
log(a < b, b < a, a > b, b > a, a <= c, a >= c, b >= a, a <= b, (1, a) < (1, b), [a] < [b], [b] < [a])
log(a in [b, c], b in [a, c], a in (c,), 1 in [a], a in {"z": 1}, [a, b].index(c), [a, c, b].count(a))
xs = [b, a, c]
xs.remove(c)
log(xs, sorted([b, a, c]), sorted([b, a, c], reverse=True), min([b, a, c]), max([a, c, b]), min(b, c))
ys = [V(3), V(1), V(2)]
ys.sort()
log(ys, sorted(ys, key=lambda v: -v.x), sorted(["bb", "a"], key=len), any([V(0)]), all([]))
"#,
    );
    assert_eq!(
        log,
        [
            "V1 [V1, V2] (V1,) {'k': V2} V1|  V2|V1 |",
            "True False False False False False True True False",
            "True False False True True True True True True True False",
            "True False True False False 0 2",
            "[V2, V1] [V1, V1, V2] [V2, V1, V1] V1 V2 V1",
            "[V1, V2, V3] [V3, V2, V1] ['a', 'bb'] True True",
        ]
    );
}

#[test]
fn without_eq_or_lt_instances_are_identity_and_unordered() {
    let log = log_of(
        r#"class P:
    pass
a, b = P(), P()
d = {a: 1, b: 2}
s = {a, a, b}
log(a == a, a == b, a != b, len(d), d[a], d[b], len(s), a in d, a in [b, a], [a].index(a))
"#,
    );
    assert_eq!(log, ["True False True 2 1 2 2 True True 0"]);
    assert_eq!(
        fault("class P:\n    pass\nx = P() < P()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class P:\n    pass\nx = sorted([P(), P()])\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class P:\n    pass\nx = 1 < P()\n").class,
        ExcClass::TypeError
    );
}

#[test]
fn identity_keys_survive_a_custom_eq() {
    // Divergence 22: instances are dict keys by identity even with __eq__.
    let log = log_of(
        r#"class V:
    def __init__(self, x):
        self.x = x
    def __eq__(self, other):
        return True
a, b = V(1), V(1)
d = {a: "a"}
d[b] = "b"
log(len(d), d[a], d[b], a == b, {a, b} == {a, b}, len({a, b}))
"#,
    );
    assert_eq!(log, ["2 a b True True 2"]);
}

#[test]
fn len_contains_getitem_setitem_and_iteration() {
    let log = log_of(
        r#"class Grid:
    def __init__(self, cells):
        self.cells = cells
        self.reads = 0
    def __len__(self):
        return len(self.cells)
    def __contains__(self, item):
        return item in self.cells
    def __getitem__(self, i):
        self.reads += 1
        return self.cells[i]
    def __setitem__(self, i, v):
        self.cells[i] = v

g = Grid([5, 6, 7])
log(len(g), bool(g), bool(Grid([])), 6 in g, 9 in g, g[0], g[-1])
g[1] = 60
seen = []
for x in g:
    seen.append(x)
    g.cells.append(0)
log(seen, g.reads, len(g), list(Grid([1, 2])), sorted(Grid([3, 1, 2])), sum(Grid([1, 2, 3])), max(Grid([4, 9, 2])))
a, b = Grid([1, 2])
log(a, b, [*Grid([7, 8])], "-".join(Grid(["x", "y"])), any(Grid([0, 0])), all(Grid([1, 1])))
if Grid([]):
    log("nonempty")
else:
    log("empty")
log(not Grid([1]), Grid([]) or "fallback", Grid([1]) and "second")
def f(*args):
    return args
log(f(*Grid([1, 2])), list(enumerate(Grid(["a"]))), list(zip(Grid([1, 2]), "ab")), dict(Grid([("k", 1)])))
"#,
    );
    assert_eq!(
        log,
        [
            "3 True False True False 5 7",
            "[5, 60, 7] 5 6 [1, 2] [1, 2, 3] 6 9",
            "1 2 [7, 8] x-y False True",
            "empty",
            "False fallback second",
            "(1, 2) [(0, 'a')] [(1, 'a'), (2, 'b')] {'k': 1}",
        ]
    );
    assert_eq!(
        fault("class A:\n    pass\nfor x in A():\n    pass\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    pass\nx = 1 in A()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    def __len__(self):\n        return -1\nlen(A())\n").class,
        ExcClass::ValueError
    );
    assert_eq!(
        fault("class A:\n    def __len__(self):\n        return 'no'\nlen(A())\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    pass\nlen(A())\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    pass\nA()[0]\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    def __str__(self):\n        return 5\nstr(A())\n").class,
        ExcClass::TypeError
    );
}

#[test]
fn arithmetic_dunders_have_no_reflected_form() {
    let log = log_of(
        r#"class V:
    def __init__(self, x):
        self.x = x
    def __add__(self, other):
        return V(self.x + other.x)
    def __sub__(self, other):
        return V(self.x - other.x)
    def __mul__(self, k):
        return V(self.x * k)
    def __neg__(self):
        return V(-self.x)
    def __str__(self):
        return f"V{self.x}"

a, b = V(1), V(2)
log(a + b, a - b, a * 3, -a, (a + b) * 2)
a += b
log(a, b)
"#,
    );
    assert_eq!(log, ["V3 V-1 V3 V-1 V6", "V3 V2"]);
    assert_eq!(
        fault("class V:\n    def __mul__(self, k):\n        return 1\nx = 2 * V()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class V:\n    pass\nx = V() + V()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class V:\n    pass\nx = +V()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class V:\n    def __add__(self, o):\n        return 1\nx = V() / V()\n").class,
        ExcClass::TypeError
    );
}

#[test]
fn unlisted_dunders_are_ordinary_methods() {
    let log = log_of(
        r#"class A:
    def __repr__(self):
        return "repr"
    def __hash__(self):
        return 7
    def __bool__(self):
        return False
a = A()
log(str(a), bool(a), a.__repr__(), a.__hash__())
"#,
    );
    assert_eq!(log, ["<A> True repr 7"]);
}

#[test]
fn user_exception_classes() {
    let log = log_of(
        r#"class GameError(Exception):
    pass

class Blocked(GameError):
    def __init__(self, where, why):
        self.where = where
        self.why = why

class Loud(ValueError):
    def __str__(self):
        return "loud!"

def attempt(k):
    try:
        if k == 0:
            raise Blocked((1, 2), "wall")
        if k == 1:
            raise GameError("plain", 3)
        if k == 2:
            raise Loud("x")
        if k == 3:
            raise GameError
        return "fine"
    except Blocked as e:
        return ("blocked", e.args, e.where, e.why, e.line, e.file, isinstance(e, GameError), isinstance(e, Exception), isinstance(e, ValueError), str(e))
    except ValueError as e:
        return ("value", str(e), e.args, isinstance(e, Loud))
    except Exception as e:
        return ("exc", str(e), e.args)

for k in range(4):
    log(attempt(k))
"#,
    );
    assert_eq!(
        log,
        [
            "('blocked', ((1, 2), 'wall'), (1, 2), 'wall', 16, 'main.py', True, True, False, 'Blocked: (1, 2), wall')",
            "('exc', 'GameError: plain, 3', ('plain', 3))",
            "('value', 'loud!', ('x',), True)",
            "('exc', 'GameError', ())",
        ]
    );
    // A class deriving from no exception cannot be raised.
    assert_eq!(
        fault("class A:\n    pass\nraise A()\n").class,
        ExcClass::TypeError
    );
    assert_eq!(
        fault("class A:\n    pass\nraise A\n").class,
        ExcClass::TypeError
    );
    // Uncaught, the fault record names the user class.
    let e = fault("class E(KeyError):\n    pass\nraise E(1)\n");
    assert_eq!(
        (e.class, e.class_name().as_str(), e.args.len(), e.line),
        (ExcClass::KeyError, "E", 1, 3)
    );
    assert_eq!(e.display(), "E: 1");
    // `except E` with the base's `except` first: order decides, as in Python.
    assert_eq!(
        log_of(
            "class E(KeyError):\n    pass\ntry:\n    raise E()\nexcept KeyError:\n    log('base')\nexcept E:\n    log('own')\n"
        ),
        ["base"]
    );
    // A bare `raise` re-raises the same instance.
    assert_eq!(
        log_of(
            "class E(KeyError):\n    pass\ntry:\n    try:\n        raise E('a')\n    except E as e:\n        e.seen = True\n        raise\nexcept KeyError as e:\n    log(e.seen, e.args)\n"
        ),
        ["True ('a',)"]
    );
}

#[test]
fn a_dunder_is_a_real_call_paused_by_the_budget_and_faulting_like_one() {
    // A `__lt__` slow enough that a sort of eight elements spans several
    // ticks; the result must still be the stable merge sort's.
    let (costs, limits) = tables();
    let program = load(
        r#"class Slow:
    def __init__(self, k, tag):
        self.k = k
        self.tag = tag
    def __lt__(self, other):
        n = 0
        while n < 30:
            n += 1
        return self.k < other.k
    def __str__(self):
        return f"{self.k}{self.tag}"

xs = [Slow(3, "a"), Slow(1, "a"), Slow(2, "a"), Slow(1, "b"), Slow(3, "b"), Slow(2, "b"), Slow(1, "c"), Slow(0, "a")]
xs.sort()
log(xs, min(xs), max(xs))
"#,
    );
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    let mut ticks = 0;
    loop {
        ticks += 1;
        assert!(ticks < 10_000);
        match m.run_slice(&mut host) {
            Slice::Yield => {}
            Slice::Restarted => break,
            other => panic!("{other:?}"),
        }
    }
    assert!(ticks > 5, "the sort did not span ticks: {ticks}");
    // `max` keeps the first of equals (`syntax.md`): `3a`, not `3b`.
    assert_eq!(host.log, ["[0a, 1a, 1b, 1c, 2a, 2b, 3a, 3b] 0a 3a"]);

    // A `__lt__` that raises: the sort propagates it from that comparison.
    let e = fault(
        "class B:\n    def __lt__(self, o):\n        raise KeyError('cmp')\nsorted([B(), B()])\n",
    );
    assert_eq!((e.class, e.line), (ExcClass::KeyError, 3));
    // Caught by the caller of the sort, like any exception from a call.
    assert_eq!(
        log_of(
            "class B:\n    def __lt__(self, o):\n        raise KeyError('cmp')\ntry:\n    sorted([B(), B()])\nexcept KeyError as e:\n    log('caught', e.args)\n"
        ),
        ["caught ('cmp',)"]
    );
    // A waiting host call inside `__str__`: the machine waits, then resumes.
    let (host, end, _) = run_to_end(
        "class W:\n    def __str__(self):\n        t = wait()\n        return f'w{t}'\nlog(W(), [W()])\n",
    );
    assert_eq!(end, Slice::Restarted);
    assert_eq!(host.waits, 2);
    assert_eq!(host.log.len(), 1);
    assert!(
        host.log[0].starts_with("w") && host.log[0].contains("[w"),
        "{:?}",
        host.log
    );
}

#[test]
fn a_fault_inside_a_dunder_unwinds_through_the_operation() {
    // The exception leaves the `__eq__` frame and the `in` walk, and lands
    // in the enclosing `try` with the stack intact for what follows.
    let log = log_of(
        r#"class E:
    def __eq__(self, o):
        raise ValueError("eq")
try:
    x = [1, 2] + ([E()] if 1 in [E()] else [3])
except ValueError as e:
    x = e.args
log(x, [1, 2] + [3])
"#,
    );
    assert_eq!(log, ["('eq',) [1, 2, 3]"]);
}

#[test]
fn the_load_boundary_for_classes() {
    for (src, word) in [
        ("class A(B, C):\n    pass\n", "inheritance"),
        ("if x:\n    class A:\n        pass\n", "top level"),
        ("def f():\n    class A:\n        pass\n", "top level"),
        ("class A:\n    class B:\n        pass\n", "top level"),
        ("class A:\n    if x:\n        pass\n", "body"),
        (
            "class A:\n    def f(self):\n        def g():\n            pass\n",
            "nest",
        ),
    ] {
        let msg = load_error(src);
        assert!(
            msg.to_lowercase().contains(word),
            "{src:?}: message {msg:?} does not name {word:?}"
        );
    }
}

#[test]
fn reflection_names_do_not_exist() {
    // `super`, `type` and `__dict__` are not names (`syntax.md`): a
    // `NameError` or `AttributeError` at the use, not a parse error.
    assert_eq!(
        fault("class A:\n    def f(self):\n        return super().f()\nA().f()\n").class,
        ExcClass::NameError
    );
    assert_eq!(
        fault("class A:\n    pass\nx = A.__dict__\n").class,
        ExcClass::AttributeError
    );
    assert_eq!(
        fault("class A:\n    pass\nx = type(A())\n").class,
        ExcClass::NameError
    );
    assert_eq!(
        fault("class A:\n    pass\nx = A().__class__\n").class,
        ExcClass::AttributeError
    );
}

#[test]
fn instances_render_inside_containers_by_str() {
    let log = log_of(
        r#"class N:
    def __init__(self, v):
        self.v = v
    def __str__(self):
        return f"n{self.v}"
class P:
    pass
p = P()
log([N(1), {N(2): [N(3)]}, (N(4),), p], f"{[N(5)]}", str({p: 1}) == "{" + str(p) + ": 1}")
"#,
    );
    assert_eq!(log, ["[n1, {n2: [n3]}, (n4,), <P>] [n5] True"]);
}
