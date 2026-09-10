//! `import` over a closed module set (T13): `docs/01-language/syntax.md`'s
//! `import` section, end to end. A bundle is several files here.

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

fn load(files: &[(&str, &str)]) -> Program {
    let (_, limits) = tables();
    Program::load(files, &limits).unwrap_or_else(|e| panic!("{e}"))
}

/// Run until main flow has restarted `runs` times; return the log.
fn run_bundle(files: &[(&str, &str)], runs: u32) -> (Vec<String>, Slice) {
    let (costs, limits) = tables();
    let program = load(files);
    let mut m = Machine::new(&program, costs, limits);
    let mut host = TestHost::default();
    let mut ticks = 0u32;
    let mut restarts = 0u32;
    loop {
        ticks = ticks.wrapping_add(1);
        assert!(ticks < 10_000, "program did not finish in 10000 ticks");
        m.tick = u64::from(ticks);
        match m.run_slice(&mut host) {
            Slice::Yield => {}
            Slice::Restarted => {
                restarts = restarts.wrapping_add(1);
                if restarts >= runs {
                    return (host.log, Slice::Restarted);
                }
            }
            other => return (host.log, other),
        }
    }
}

fn log_of(files: &[(&str, &str)]) -> Vec<String> {
    let (log, end) = run_bundle(files, 1);
    assert_eq!(
        end,
        Slice::Restarted,
        "program faulted: {end:?}\nlog: {log:?}"
    );
    log
}

fn fault(files: &[(&str, &str)]) -> lang::Exception {
    let (log, end) = run_bundle(files, 1);
    match end {
        Slice::Fault(e) => e,
        other => panic!("expected a fault, got {other:?}; log {log:?}"),
    }
}

fn load_error(files: &[(&str, &str)]) -> lang::errors::LoadError {
    let (_, limits) = tables();
    match Program::load(files, &limits) {
        Ok(_) => panic!("loaded, but should have been refused: {files:?}"),
        Err(e) => e,
    }
}

const UTIL: &str = r#"log("util runs")
count = 0
PI = 3.14

def bump():
    count_local = count + 1
    return count_local

def describe():
    return f"util sees PI={PI} and count={count}"

class Box:
    def __init__(self, v):
        self.v = v
    def __str__(self):
        return f"Box({self.v})"
"#;

#[test]
fn import_binds_the_module_and_from_import_its_names() {
    let log = log_of(&[
        (
            "main.py",
            "import util\nfrom util import bump, PI as pi, Box\nimport util as u\nlog(util.PI, pi, bump(), util.describe(), Box(2), u is None, str(util))\nlog(util.count, u.count)\n",
        ),
        ("util.py", UTIL),
    ]);
    assert_eq!(
        log,
        [
            "util runs",
            "3.14 3.14 1 util sees PI=3.14 and count=0 Box(2) False <module util>",
            "0 0",
        ]
    );
}

#[test]
fn a_module_runs_once_per_run_and_a_second_import_is_the_same_module() {
    let log = log_of(&[
        (
            "main.py",
            "import util\nimport util\nfrom util import PI\nimport other\nlog(util.PI, other.via_util, other.same)\n",
        ),
        ("util.py", UTIL),
        (
            "other.py",
            "import util\nvia_util = util.PI * 2\nimport util as again\nsame = again.describe()\n",
        ),
    ]);
    assert_eq!(
        log,
        ["util runs", "3.14 6.28 util sees PI=3.14 and count=0"]
    );
}

#[test]
fn a_restart_reruns_every_module_it_imports() {
    let (log, _) = run_bundle(
        &[("main.py", "import util\nlog('main')\n"), ("util.py", UTIL)],
        3,
    );
    assert_eq!(
        log,
        [
            "util runs",
            "main",
            "util runs",
            "main",
            "util runs",
            "main"
        ]
    );
}

#[test]
fn module_globals_are_the_modules_own() {
    // A function defined in a module reads and writes its module's
    // globals, whatever main's are; main's globals do not leak in.
    let log = log_of(&[
        (
            "main.py",
            "PI = 'main pi'\nimport util\nlog(util.describe(), PI)\nutil_count = util.count\nfrom util import bump\nlog(bump(), util_count)\nimport counter\ncounter.inc()\ncounter.inc()\nlog(counter.n, counter.read())\n",
        ),
        ("util.py", UTIL),
        (
            "counter.py",
            "n = 0\nstate = []\ndef inc():\n    state.append(1)\ndef read():\n    return len(state)\n",
        ),
    ]);
    assert_eq!(
        log,
        [
            "util runs",
            "util sees PI=3.14 and count=0 main pi",
            "1 0",
            "0 2"
        ]
    );
    // A module's function that names a main.py global: NameError.
    let e = fault(&[
        ("main.py", "secret = 1\nimport peek\npeek.get()\n"),
        ("peek.py", "def get():\n    return secret\n"),
    ]);
    assert_eq!(
        (e.class, e.file.as_deref()),
        (ExcClass::NameError, Some("peek.py"))
    );
}

#[test]
fn module_attributes_are_read_only_and_missing_ones_raise() {
    assert_eq!(
        fault(&[("main.py", "import util\nutil.PI = 3\n"), ("util.py", UTIL)]).class,
        ExcClass::TypeError
    );
    let e = fault(&[
        ("main.py", "import util\nx = util.nothing\n"),
        ("util.py", UTIL),
    ]);
    assert_eq!(
        (e.class, e.args),
        (ExcClass::AttributeError, vec![Value::str("nothing")])
    );
    let e = fault(&[("main.py", "from util import nothing\n"), ("util.py", UTIL)]);
    assert_eq!((e.class, e.line), (ExcClass::AttributeError, 1));
    // Set after the import ran: visible, like any global.
    assert_eq!(
        log_of(&[
            ("main.py", "import late\nlog(late.x)\n"),
            ("late.py", "x = 1\nx = 2\n")
        ]),
        ["2"]
    );
}

#[test]
fn an_exception_in_a_module_body_unwinds_into_the_importer() {
    let log = log_of(&[
        (
            "main.py",
            "try:\n    import broken\nexcept ZeroDivisionError as e:\n    log('caught', e.file, e.line)\nimport broken\n",
        ),
        ("broken.py", "x = 1\ny = 1 / 0\n"),
    ]);
    // The second import finds the module already begun, and returns it as
    // it stands: its body does not run twice.
    assert_eq!(log, ["caught broken.py 2"]);
    let e = fault(&[
        ("main.py", "import broken\n"),
        ("broken.py", "raise KeyError('k')\n"),
    ]);
    assert_eq!(
        (e.class, e.file.as_deref(), e.line),
        (ExcClass::KeyError, Some("broken.py"), 1)
    );
}

#[test]
fn imports_are_reached_dynamically_in_order() {
    let log = log_of(&[
        (
            "main.py",
            "def f():\n    import b\n    return b.v\nlog('before')\nimport a\nlog(f(), f())\nif False:\n    import never\n",
        ),
        ("a.py", "log('a')\n"),
        ("b.py", "log('b')\nv = 7\n"),
        ("never.py", "log('never')\n"),
    ]);
    assert_eq!(log, ["before", "a", "b", "7 7"]);
}

#[test]
fn the_module_set_is_closed_at_load() {
    let e = load_error(&[("main.py", "import missing\n")]);
    assert!(
        e.message.contains("`missing`") && e.file == "main.py",
        "{e}"
    );
    let e = load_error(&[
        ("main.py", "x = 1\n"),
        ("helper.py", "def f():\n    import gone\n"),
    ]);
    assert!(e.message.contains("`gone`") && e.file == "helper.py", "{e}");
    // Never imported, but still a file of the bundle: it must parse.
    let e = load_error(&[("main.py", "x = 1\n"), ("bad.py", "def (:\n")]);
    assert_eq!(e.file, "bad.py");
}

#[test]
fn circular_imports_are_a_load_error() {
    let e = load_error(&[
        ("main.py", "import a\n"),
        ("a.py", "import b\n"),
        ("b.py", "import a\n"),
    ]);
    assert!(
        e.message.contains("circular") && e.message.contains("a -> b -> a"),
        "{e}"
    );
    let e = load_error(&[("main.py", "import main\n")]);
    assert!(e.message.contains("circular"), "{e}");
    let e = load_error(&[
        ("main.py", "import a\n"),
        ("a.py", "def f():\n    from a import f\n"),
    ]);
    assert!(e.message.contains("circular"), "{e}");
    // A diamond is not a cycle.
    let log = log_of(&[
        (
            "main.py",
            "import left\nimport right\nlog(left.v, right.v)\n",
        ),
        ("left.py", "import base\nv = base.v + 1\n"),
        ("right.py", "import base\nv = base.v + 2\n"),
        ("base.py", "log('base once')\nv = 10\n"),
    ]);
    assert_eq!(log, ["base once", "11 12"]);
}

#[test]
fn the_forms_the_language_excludes_are_refused() {
    for (src, word) in [
        ("from util import *\n", "*"),
        ("from . import util\n", "relative"),
        ("import util.sub\n", "dotted"),
        ("from util.sub import x\n", "dotted"),
        ("import util, other\n", "one module"),
        ("import\n", "name"),
        ("from util\n", "import"),
    ] {
        let e = load_error(&[("main.py", src), ("util.py", UTIL)]);
        assert!(
            e.message.contains(word),
            "{src:?}: message {:?} does not contain {word:?}",
            e.message
        );
    }
    let e = load_error(&[("main.py", "x = 1\n"), ("main.py", "y = 2\n")]);
    assert!(e.message.contains("unique"), "{e}");
}

#[test]
fn the_version_covers_every_file() {
    let (_, limits) = tables();
    let a = Program::load(&[("main.py", "import u\n"), ("u.py", "x = 1\n")], &limits).unwrap();
    let b = Program::load(&[("main.py", "import u\n"), ("u.py", "x = 2\n")], &limits).unwrap();
    let c = Program::load(&[("u.py", "x = 1\n"), ("main.py", "import u\n")], &limits).unwrap();
    assert_ne!(
        a.version, b.version,
        "a change in a module did not change the version"
    );
    assert_eq!(
        a.version, c.version,
        "the version depends on the order files were given"
    );
}

#[test]
fn a_swapped_program_starts_with_its_own_modules() {
    let (costs, limits) = tables();
    let p1 = load(&[("main.py", "import u\nlog(u.x)\n"), ("u.py", "x = 'one'\n")]);
    let p2 = load(&[("main.py", "import u\nlog(u.x)\n"), ("u.py", "x = 'two'\n")]);
    let mut m = Machine::new(&p1, costs, limits);
    let mut host = TestHost::default();
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
    m.swap_program(&p2);
    assert_eq!(m.run_slice(&mut host), Slice::Restarted);
    assert_eq!(host.log, ["one", "two"]);
}
