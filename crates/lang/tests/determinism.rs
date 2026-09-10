//! The lockstep guarantee for the language: the same bundle, run from the
//! same tables with the same host answers, produces the same trace on every
//! peer. Same-process pairs miss the whole class of per-process instability
//! — a hasher seeded at startup, an address-ordered map — so the real check
//! runs the trace in a SEPARATE PROCESS, as `crates/sim/tests/golden.rs` does.
//!
//! The trace is every `log` line, every host call, the slice kinds and the
//! fault records, in order: what the sim would feed into its state hash.

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::{Costs, Event, Host, HostCall, Limits, Machine, Program, Slice, Value};
use std::rc::Rc;

/// A program that touches most of what T10 built: wide arithmetic, sorting,
/// dicts and sets in insertion order, f-strings, exceptions, a fault, and a
/// waiting host call resumed with a value that depends on the trace so far.
/// A helper module, so the trace crosses an `import` and a function whose
/// globals are another module's.
const UTIL: &str = r#"
scale = 7
def cube_over(i):
    return (i ** 3) / scale
"#;

const PROGRAM: &str = r#"
from util import cube_over
import util
acc = {}
seen = set()
for i in range(40):
    k = f"{(i * 7919) % 23:02d}"
    acc[k] = acc.get(k, 0) + cube_over(i)
    seen.add((i % 5, k))
order = sorted(acc.items(), key=len)
log(order[:3], len(seen), sorted(seen)[:2])
xs = [(-1) ** i * (10 ** 15 + i) / 3 for i in range(9)]
xs.sort()
log(xs, sum(xs), min(xs), max(xs))
def tri(n):
    return n if n < 2 else n + tri(n - 1)
try:
    log(tri(60), 2 ** 80, 3 ** -7, -7 // 2, -7 % 3, round(2.5), round(-0.5))
    log([str(x) for x in (1/3, -2/3, 1e-12, 123456789.987654321)])
except OverflowError as e:
    log("overflow", e.line)
r = wait()
log("resumed", r, {1: "a", True: "b"}, sorted({3, 1, 2} | {0}))
1 / 0
"#;

struct Trace {
    lines: Vec<String>,
}

impl Host for Trace {
    fn call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        _: Vec<(String, Value)>,
    ) -> lang::value::R<HostCall> {
        let rendered: Vec<String> = args.iter().map(Value::to_str_value).collect();
        self.lines
            .push(format!("call {name}({})", rendered.join(", ")));
        match name {
            "wait" => Ok(HostCall::Wait),
            _ => Ok(HostCall::Value(Value::None)),
        }
    }
}

fn trace() -> Vec<String> {
    let costs = Rc::new(Costs::parse(COSTS_TOML).expect("costs"));
    let limits = Rc::new(Limits::parse(LIMITS_TOML).expect("limits"));
    let program = Program::load(&[("main.py", PROGRAM), ("util.py", UTIL)], &limits)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut m = Machine::new(&program, costs, limits);
    let mut host = Trace {
        lines: vec![format!("version {:016x}", program.version)],
    };
    let mut faults = 0u32;
    for tick in 1..=400u64 {
        m.tick = tick;
        let slice = m.run_slice(&mut host);
        host.lines.push(format!("tick {tick}: {slice:?}"));
        for ev in m.take_events() {
            host.lines.push(format!("event {ev:?}"));
            if matches!(ev, Event::Fault(_)) {
                faults = faults.wrapping_add(1);
            }
        }
        if faults >= 2 {
            break;
        }
        if slice == Slice::Wait {
            m.resume(Value::int(i128::from(host.lines.len() as u64)));
        }
    }
    assert_eq!(
        faults,
        2,
        "the program did not reach its fault twice; trace:\n{}",
        host.lines.join("\n")
    );
    host.lines
}

fn fnv(lines: &[String]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for l in lines {
        for b in l.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0xff;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[test]
fn the_same_bundle_traces_identically_twice() {
    assert_eq!(trace(), trace());
}

#[test]
fn the_trace_covers_what_it_claims() {
    let t = trace();
    let text = t.join("\n");
    for needle in [
        "call log(",
        "tick 1:",
        "Wait",
        "resumed",
        "Fault(Exception { class: ZeroDivisionError",
    ] {
        assert!(text.contains(needle), "trace lacks {needle:?}:\n{text}");
    }
    assert!(
        !text.contains("overflow"),
        "the arithmetic overflowed, so the wide routine is not exercised:\n{text}"
    );
}

/// Child half: prints the trace hash. Ignored in normal runs; the parent
/// test spawns it in a fresh process.
#[test]
#[ignore = "spawned by cross_process_trace_matches"]
fn emit_trace_hash() {
    println!("LANG_TRACE_HASH={:016x}", fnv(&trace()));
}

#[test]
fn cross_process_trace_matches() {
    let exe = std::env::current_exe().expect("test binary path");
    let output = std::process::Command::new(exe)
        .args(["--ignored", "--exact", "emit_trace_hash", "--nocapture"])
        .output()
        .expect("spawn child test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child process failed: {stdout}");
    let child = stdout
        .lines()
        .find_map(|l| l.strip_prefix("LANG_TRACE_HASH="))
        .unwrap_or_else(|| panic!("no hash line in child output: {stdout}"))
        .to_string();
    let local = format!("{:016x}", fnv(&trace()));
    assert_eq!(
        child, local,
        "cross-process trace mismatch — determinism violation"
    );
}
