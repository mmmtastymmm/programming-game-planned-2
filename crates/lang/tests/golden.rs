//! Stored golden fixtures for the language (T14), mirroring
//! `crates/sim/tests/golden.rs`: a program's transcript is hashed tick by
//! tick and compared against CHECKED-IN hashes, so any behaviour change —
//! deterministic or not — fails CI until the fixture is regenerated on
//! purpose:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test -p lang --test golden
//! ```
//!
//! A PR that regenerates a fixture must explain why (CLAUDE.md). The
//! transcript itself is stored too, so a drift shows *what* changed and not
//! only that something did. Each tick's line carries the budget the slice
//! spent and the deficit it left, so a cost change that moves no call or
//! event to another tick still moves the transcript (I5, T34).
//!
//! Three fixtures:
//! - `showcase` — one program through most of the language, two files.
//! - `interrupts` — the paths a match replay is least likely to cover: a
//!   fault escaping mid-unwind, a hook exhausting its budget, escalation,
//!   preemption, redeploy. A scripted host raises the interrupts.
//! - `wide` — the 256-bit routine behind `num` at its boundaries: the
//!   products and quotients nearest the range, the floor at each sign, and
//!   the `**` procedure's count of steps. Computed in Rust, since a program
//!   cannot observe a step count.
//!
//! The two program fixtures are also run as a debugger runs a program
//! (`docs/06`, Forks; T26) — watched at every boundary, stopped and resumed
//! at every boundary, and replaced by a deep copy at every stop — and each
//! way must give the stored fixture, transcript, budget and state alike.
//!
//! Every fixture has an "alive" test asserting on what the transcript must
//! contain, so a test that fails to run — an error transcript hashes the
//! same in every process — cannot score green (CLAUDE.md).

use lang::data::{COSTS_TOML, LIMITS_TOML};
use lang::num::Num;
use lang::{Costs, Host, HostCall, Interrupt, Limits, Machine, Program, Slice, Tracer, Value};
use std::path::PathBuf;
use std::rc::Rc;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn tables() -> (Rc<Costs>, Rc<Limits>) {
    (
        Rc::new(Costs::parse(COSTS_TOML).expect("costs")),
        Rc::new(Limits::parse(LIMITS_TOML).expect("limits")),
    )
}

/// The bundle of a fixture: `<name>.py` is `main.py`, and `<name>.<mod>.py`
/// is the module `mod`.
fn bundle(name: &str) -> Program {
    let dir = fixture_dir();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("fixture dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
        .collect();
    entries.sort();
    for entry in entries {
        let Some(stem) = entry.strip_suffix(".py") else {
            continue;
        };
        if stem == name {
            files.push((
                "main.py".to_string(),
                std::fs::read_to_string(dir.join(&entry)).expect("read"),
            ));
        } else if let Some(module) = stem.strip_prefix(&format!("{name}.")) {
            files.push((
                format!("{module}.py"),
                std::fs::read_to_string(dir.join(&entry)).expect("read"),
            ));
        }
    }
    assert!(!files.is_empty(), "no fixture named {name}");
    let (_, limits) = tables();
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    Program::load(&refs, &limits).unwrap_or_else(|e| panic!("fixture {name} does not load: {e}"))
}

/// A host that records every call into the transcript, waits on `wait()`
/// (resumed with the tick), and can hand over a bundle for a `redeploy`.
struct TranscriptHost {
    lines: Vec<String>,
    next_program: Option<Program>,
}

impl Host for TranscriptHost {
    fn call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        kwargs: Vec<(String, Value)>,
    ) -> lang::value::R<HostCall> {
        let rendered: Vec<String> = args.iter().map(Value::to_str_value).collect();
        let kw: Vec<String> = kwargs
            .iter()
            .map(|(k, v)| format!("{k}={}", v.to_str_value()))
            .collect();
        self.lines.push(format!(
            "  call {name}({}{}{})",
            rendered.join(", "),
            if kw.is_empty() { "" } else { "; " },
            kw.join(", ")
        ));
        match name {
            "wait" => Ok(HostCall::Wait),
            "log" => Ok(HostCall::Value(Value::None)),
            _ => Ok(HostCall::Unknown),
        }
    }

    fn current_program(&mut self) -> Option<Program> {
        self.next_program.clone()
    }
}

/// How a run drives its slices: as the match does, or as a debugger does
/// (`docs/06`, Forks).
#[derive(Clone, Copy, PartialEq, Debug)]
enum How {
    /// `run_slice`, as the match's sim calls it.
    Plain,
    /// Shown to a tracer at every boundary, which stops at none.
    Watched,
    /// Stopped at every boundary and resumed from it.
    Stopped,
    /// Stopped at every boundary, and the machine replaced there, and at
    /// every tick's end, by its deep copy.
    Copied,
}

/// What a run gave: the transcript, one hash per tick, the machine's
/// state bytes at each tick's end, and what its tracer saw.
#[derive(Debug, PartialEq)]
struct Ran {
    lines: Vec<String>,
    hashes: Vec<u64>,
    states: Vec<Vec<u8>>,
}

/// What a debugger's runs saw, which a plain run has none of.
#[derive(Debug, Default)]
struct Seen {
    boundaries: u64,
    beginnings: u64,
    stops: u64,
}

/// A tracer that counts, and stops at every boundary or at none.
struct EveryBoundary<'a> {
    stop: bool,
    seen: &'a mut Seen,
}

impl Tracer for EveryBoundary<'_> {
    fn boundary(&mut self, vm: &Machine) -> bool {
        self.seen.boundaries = self.seen.boundaries.wrapping_add(1);
        if vm.beginning().is_some() {
            self.seen.beginnings = self.seen.beginnings.wrapping_add(1);
        }
        self.stop
    }
}

/// One tick's slice of `m`, driven as `how` says.
fn slice(m: &mut Machine, host: &mut TranscriptHost, how: How, seen: &mut Seen) -> Slice {
    if how == How::Plain {
        return m.run_slice(host);
    }
    let mut tracer = EveryBoundary {
        stop: how != How::Watched,
        seen,
    };
    let mut result = m.run_slice_traced(host, Some(&mut tracer));
    let mut n = 0u64;
    while result == Slice::Stopped {
        // The position's `n`: the boundaries this slice passed before.
        assert_eq!(m.boundary(), n, "a stop names the wrong boundary");
        n = n.wrapping_add(1);
        tracer.seen.stops = tracer.seen.stops.wrapping_add(1);
        if how == How::Copied {
            *m = m.deep_copy();
        }
        result = m.resume_slice(host, Some(&mut tracer));
    }
    result
}

/// Run a bundle for `ticks` ticks under a schedule of raised interrupts;
/// the transcript and one hash per tick.
fn run(
    program: &Program,
    ticks: u64,
    schedule: &[(u64, Interrupt)],
    next_program: Option<Program>,
) -> (Vec<String>, Vec<u64>) {
    let ran = run_as(program, ticks, schedule, next_program, How::Plain).0;
    (ran.lines, ran.hashes)
}

fn run_as(
    program: &Program,
    ticks: u64,
    schedule: &[(u64, Interrupt)],
    next_program: Option<Program>,
    how: How,
) -> (Ran, Seen) {
    let (costs, limits) = tables();
    let mut m = Machine::new(program, costs, limits);
    let mut seen = Seen::default();
    let mut states = Vec::new();
    let mut host = TranscriptHost {
        lines: vec![format!("version {:016x}", program.version)],
        next_program,
    };
    let mut hashes = Vec::new();
    let mut h = fnv_start();
    for tick in 1..=ticks {
        m.tick = tick;
        for (t, kind) in schedule {
            if *t == tick {
                m.raise(*kind);
                host.lines
                    .push(format!("tick {tick}: raise {}", kind.name()));
            }
        }
        let slice = slice(&mut m, &mut host, how, &mut seen);
        for ev in m.take_events() {
            host.lines.push(format!("  event {ev:?}"));
        }
        if how == How::Copied {
            m = m.deep_copy();
        }
        states.push(m.state_bytes());
        let record = match &m.fault_record {
            Some(r) => format!(
                " record={}@{}:{} tick {} v{:016x}",
                r.exception
                    .as_ref()
                    .map(|e| e.display())
                    .unwrap_or_else(|| format!(
                        "exhausted {}",
                        r.exhausted.map(Interrupt::name).unwrap_or("?")
                    )),
                r.file.as_deref().unwrap_or("?"),
                r.line,
                r.tick,
                r.version
            ),
            None => String::new(),
        };
        // The budget the slice spent and the deficit it left (T34): a cost
        // change that moves nothing to another tick still moves these.
        host.lines.push(format!(
            "tick {tick}: {slice:?} mode={} spent={} deficit={}{record}",
            m.mode_name(),
            m.spent(),
            m.deficit()
        ));
        h = fnv_lines(h, &host.lines);
        hashes.push(h);
        if slice == Slice::Wait {
            m.resume(Value::int(i128::from(tick)));
        }
        if slice == Slice::Dead {
            break;
        }
    }
    (
        Ran {
            lines: host.lines,
            hashes,
            states,
        },
        seen,
    )
}

/// A scenario: its name, its program, the interrupts raised on it, and
/// the program a `deploy` among them installs.
type Scenario = (
    &'static str,
    Program,
    &'static [(u64, Interrupt)],
    Option<Program>,
);

/// Every scenario the program fixtures run, by name.
fn scenarios() -> Vec<Scenario> {
    vec![
        ("showcase", bundle("showcase"), &[], None),
        (
            "interrupts A",
            bundle("interrupts"),
            SCHEDULE_A,
            Some(bundle("interrupts")),
        ),
        ("interrupts B", bundle("interrupts"), SCHEDULE_B, None),
    ]
}

/// The forks-and-watches property at the language's level (`docs/06`,
/// Testing): a slice a debugger watches, stops and resumes, or copies,
/// runs as the match's does — the same calls on the same ticks, the same
/// budget spent and deficit left, the same state at every tick's end.
#[test]
fn a_debugged_run_gives_the_fixture() {
    for (name, program, schedule, next) in scenarios() {
        let (plain, _) = run_as(&program, 60, schedule, next.clone(), How::Plain);
        // A run that carries no deficit could not show one charged twice,
        // and the showcase is the scenario that carries them.
        if name == "showcase" {
            let carried = plain
                .lines
                .iter()
                .filter(|l| {
                    l.rsplit_once("deficit=")
                        .is_some_and(|(_, d)| !d.starts_with('0'))
                })
                .count();
            assert!(carried >= 5, "only {carried} ticks left a deficit");
        }
        for how in [How::Watched, How::Stopped, How::Copied] {
            let (ran, seen) = run_as(&program, 60, schedule, next.clone(), how);
            assert_eq!(
                ran.lines.join("\n"),
                plain.lines.join("\n"),
                "{name}, {how:?}: the transcript moved"
            );
            assert_eq!(ran.hashes, plain.hashes, "{name}, {how:?}");
            assert!(ran.states == plain.states, "{name}, {how:?}: a state moved");
            // Alive: the tracer was shown the run, and stopped it.
            let ticks = ran.hashes.len() as u64;
            assert!(
                seen.boundaries > ticks.saturating_mul(4),
                "{name}, {how:?}: only {} boundaries in {ticks} ticks",
                seen.boundaries
            );
            assert!(seen.beginnings > 10, "{name}, {how:?}: {seen:?}");
            if how == How::Watched {
                assert_eq!(seen.stops, 0, "{name}: a watch stopped a slice");
            } else {
                assert_eq!(seen.stops, seen.boundaries, "{name}, {how:?}");
            }
        }
    }
}

/// A copy shares nothing with its source: one taken at a tick's end and
/// left behind while the source runs to the end still runs, from that
/// tick, as the source did.
#[test]
fn a_copy_left_behind_runs_as_its_source_did() {
    for (name, program, schedule, next) in scenarios() {
        let (whole, _) = run_as(&program, 60, schedule, next.clone(), How::Plain);
        let (costs, limits) = tables();
        let mut m = Machine::new(&program, costs, limits);
        let mut host = TranscriptHost {
            lines: vec![],
            next_program: next.clone(),
        };
        let mut copies: Vec<(u64, Machine)> = Vec::new();
        let mut last = 0;
        for tick in 1..=60u64 {
            last = tick;
            m.tick = tick;
            for (t, kind) in schedule {
                if *t == tick {
                    m.raise(*kind);
                }
            }
            let slice = m.run_slice(&mut host);
            m.take_events();
            if slice == Slice::Wait {
                m.resume(Value::int(i128::from(tick)));
            }
            if slice == Slice::Dead {
                break;
            }
            copies.push((tick, m.deep_copy()));
        }
        assert!(copies.len() > 5, "{name}: only {} copies", copies.len());
        // The source is at its end; each copy now runs the rest.
        for (from, mut copy) in copies {
            let mut host = TranscriptHost {
                lines: vec![],
                next_program: next.clone(),
            };
            for tick in from.wrapping_add(1)..=last {
                copy.tick = tick;
                for (t, kind) in schedule {
                    if *t == tick {
                        copy.raise(*kind);
                    }
                }
                let slice = copy.run_slice(&mut host);
                copy.take_events();
                let at = usize::try_from(tick.wrapping_sub(1)).expect("tick");
                assert!(
                    copy.state_bytes() == whole.states[at],
                    "{name}: the copy taken at tick {from} differs at tick {tick}"
                );
                if slice == Slice::Wait {
                    copy.resume(Value::int(i128::from(tick)));
                }
                if slice == Slice::Dead {
                    break;
                }
            }
        }
    }
}

fn fnv_start() -> u64 {
    0xcbf29ce484222325
}

/// Fold every line so far into the running hash: the hash of tick `n` is
/// the hash of the whole transcript up to it, as the sim's state hash is
/// of the whole state.
fn fnv_lines(mut h: u64, lines: &[String]) -> u64 {
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

fn hashes_to_text(hashes: &[u64]) -> String {
    hashes.iter().map(|h| format!("{h:016x}\n")).collect()
}

/// Compare a fixture's transcript and hashes with the stored ones, or
/// regenerate them under `UPDATE_GOLDEN`.
fn check_fixture(name: &str, transcript: &str, hashes: &[u64]) {
    let dir = fixture_dir();
    let transcript_path = dir.join(format!("{name}.transcript.txt"));
    let hashes_path = dir.join(format!("{name}.hashes.txt"));
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&transcript_path, transcript).expect("write transcript");
        std::fs::write(&hashes_path, hashes_to_text(hashes)).expect("write hashes");
        eprintln!("golden fixture `{name}` regenerated — explain the change in the PR (CLAUDE.md)");
        return;
    }
    let stored = std::fs::read_to_string(&transcript_path).unwrap_or_else(|_| {
        panic!("no stored transcript for `{name}`: generate it with UPDATE_GOLDEN=1")
    });
    assert_eq!(
        stored, transcript,
        "transcript drift in `{name}`: the language's behaviour changed. If intentional, regenerate with UPDATE_GOLDEN=1 and explain the change in the PR (CLAUDE.md)"
    );
    let stored_hashes = std::fs::read_to_string(&hashes_path).expect("stored hashes exist");
    assert_eq!(
        stored_hashes,
        hashes_to_text(hashes),
        "hash drift in `{name}` with an identical transcript: the hashing changed"
    );
}

// ------------------------------------------------------------ showcase ---

fn showcase() -> (Vec<String>, Vec<u64>) {
    run(&bundle("showcase"), 60, &[], None)
}

#[test]
fn showcase_matches_stored_fixture() {
    let (lines, hashes) = showcase();
    check_fixture("showcase", &lines.join("\n"), &hashes);
}

#[test]
fn showcase_is_alive() {
    let (lines, hashes) = showcase();
    let text = lines.join("\n");
    let logs = lines
        .iter()
        .filter(|l| l.starts_with("  call log("))
        .count();
    assert!(logs >= 20, "the showcase logged only {logs} lines:\n{text}");
    for needle in [
        "call wait(",
        "Wait",
        "Restarted",
        "event Fault(",
        "AttributeError",
        "finally, 2",
        "overflow",
        "bot b1",
        "map x",
        "small 1",
        "waited",
    ] {
        assert!(
            text.contains(needle),
            "showcase transcript lacks {needle:?}:\n{text}"
        );
    }
    assert!(
        hashes.len() >= 3,
        "the showcase ran only {} ticks",
        hashes.len()
    );
    assert!(
        hashes.windows(2).all(|w| w[0] != w[1]),
        "two consecutive ticks hashed the same"
    );
}

// ---------------------------------------------------------- interrupts ---

/// Scenario A: a redeploy, then a fault whose `on_fault` exhausts its
/// budget, escalating through `dying` (whose hook waits) to `death`.
const SCHEDULE_A: &[(u64, Interrupt)] = &[(5, Interrupt::Redeploy)];

/// Scenario B: `dying` lands while an exception unwinds through a slow
/// `finally`, so the exception is recorded and no hook runs for it; then
/// `death` preempts the waiting `on_dying`.
const SCHEDULE_B: &[(u64, Interrupt)] = &[(8, Interrupt::Dying), (9, Interrupt::Death)];

fn interrupts() -> (Vec<String>, Vec<u64>) {
    let program = bundle("interrupts");
    let (mut lines, mut hashes) = run(&program, 60, SCHEDULE_A, Some(bundle("interrupts")));
    lines.push("---- scenario B ----".to_string());
    let (lines_b, hashes_b) = run(&program, 60, SCHEDULE_B, None);
    lines.extend(lines_b);
    hashes.extend(hashes_b);
    (lines, hashes)
}

#[test]
fn interrupts_match_stored_fixture() {
    let (lines, hashes) = interrupts();
    check_fixture("interrupts", &lines.join("\n"), &hashes);
}

#[test]
fn interrupts_are_alive() {
    let (lines, _) = interrupts();
    let text = lines.join("\n");
    for needle in [
        "event Fault(",
        "event Escalated { from: Fault, to: Dying }",
        "event Redeploy { swapped: true }",
        "event Dying",
        "event Death",
        "exhausted fault",
        "on_dying starts",
        "Dead mode=main",
    ] {
        assert!(
            text.contains(needle),
            "interrupts transcript lacks {needle:?}:\n{text}"
        );
    }
    assert!(
        text.contains("event Abandoned(Exception { class: KeyError"),
        "no exception was cut off mid-unwind:\n{text}"
    );
}

// ---------------------------------------------------------------- wide ---

/// The `num` boundary table: each line an operation and its outcome.
fn wide_table() -> Vec<String> {
    let max = Num::MAX;
    let min = Num::MIN;
    let one = Num::ONE;
    let n = |s: &str| match s.strip_prefix('-') {
        Some(rest) => Num::parse_literal(rest)
            .expect("literal")
            .checked_neg()
            .expect("neg"),
        None => Num::parse_literal(s).expect("literal"),
    };
    let neg = |x: Num| x.checked_neg().expect("neg");
    let show = |r: lang::value::R<Num>| match r {
        Ok(v) => v.to_decimal(),
        Err(e) => format!("!{}", e.class.name()),
    };
    let mut out = Vec::new();
    let mut line = |label: &str, r: lang::value::R<Num>| out.push(format!("{label} = {}", show(r)));
    // Products nearest the range.
    line("MAX * 1", max.checked_mul(one));
    line("MAX * 1.000000000001", max.checked_mul(n("1.000000000001")));
    line("MIN * 1", min.checked_mul(one));
    line("MIN * -1", min.checked_mul(neg(one)));
    line("MAX * -1", max.checked_mul(neg(one)));
    line("MAX * 0.5", max.checked_mul(n("0.5")));
    line("MIN * 0.5", min.checked_mul(n("0.5")));
    line(
        "(MAX * 0.5) * 2",
        max.checked_mul(n("0.5"))
            .and_then(|v| v.checked_mul(n("2"))),
    );
    line("1e25 * 1e1", n("1e25").checked_mul(n("1e1")));
    line("1e25 * 1e2", n("1e25").checked_mul(n("1e2")));
    line("-1e25 * 1e2", neg(n("1e25")).checked_mul(n("1e2")));
    line("1e13 * 1e13", n("1e13").checked_mul(n("1e13")));
    line("1e-6 * 1e-6", n("1e-6").checked_mul(n("1e-6")));
    line("1e-6 * 1e-7", n("1e-6").checked_mul(n("1e-7")));
    line("-1e-6 * 1e-7", neg(n("1e-6")).checked_mul(n("1e-7")));
    line(
        "0.999999999999 * 0.999999999999",
        n("0.999999999999").checked_mul(n("0.999999999999")),
    );
    line(
        "-0.999999999999 * 0.999999999999",
        neg(n("0.999999999999")).checked_mul(n("0.999999999999")),
    );
    // Quotients nearest the range and the floor at each sign.
    line("MAX / 1", max.checked_div(one));
    line("MAX / 0.5", max.checked_div(n("0.5")));
    line("MIN / 1", min.checked_div(one));
    line("MIN / -1", min.checked_div(neg(one)));
    line("MAX / -1", max.checked_div(neg(one)));
    line("MAX / MAX", max.checked_div(max));
    line("MIN / MIN", min.checked_div(min));
    line("MIN / MAX", min.checked_div(max));
    line("1 / 3", one.checked_div(n("3")));
    line("-1 / 3", neg(one).checked_div(n("3")));
    line("1 / -3", one.checked_div(neg(n("3"))));
    line("-1 / -3", neg(one).checked_div(neg(n("3"))));
    line("2 / 3", n("2").checked_div(n("3")));
    line("-2 / 3", neg(n("2")).checked_div(n("3")));
    line("1e-12 / 2", n("1e-12").checked_div(n("2")));
    line("-1e-12 / 2", neg(n("1e-12")).checked_div(n("2")));
    line("1e-12 / 1e12", n("1e-12").checked_div(n("1e12")));
    line("1 / 1e-12", one.checked_div(n("1e-12")));
    line("MAX / 1e-12", max.checked_div(n("1e-12")));
    line("1e25 / 1e-12", n("1e25").checked_div(n("1e-12")));
    line("1 / 0", one.checked_div(Num::ZERO));
    // Floor division and modulo at each sign.
    line("-7 // 2", neg(n("7")).checked_floordiv(n("2")));
    line("7 // -2", n("7").checked_floordiv(neg(n("2"))));
    line("-7 % 2", neg(n("7")).checked_rem(n("2")));
    line("7 % -2", n("7").checked_rem(neg(n("2"))));
    line("MIN // -1", min.checked_floordiv(neg(one)));
    line("MIN % -1", min.checked_rem(neg(one)));
    line("MAX % 0.7", max.checked_rem(n("0.7")));
    line("MIN % 0.7", min.checked_rem(n("0.7")));
    line("-0.5 // 1", neg(n("0.5")).checked_floordiv(one));
    line("-0.5 % 1", neg(n("0.5")).checked_rem(one));
    // Sums at the edge, negation and abs of the minimum.
    line("MAX + 1e-12", max.checked_add(n("1e-12")));
    line("MIN - 1e-12", min.checked_sub(n("1e-12")));
    line("-MIN", min.checked_neg());
    line("abs(MIN)", min.checked_abs());
    line("abs(MAX)", max.checked_abs());
    // The `**` procedure: result and count of steps.
    let pow = |base: &str, exp: &str| -> String {
        let b = base.trim_start_matches('(').trim_end_matches(')');
        match n(b).checked_pow(n(exp)) {
            Ok((v, steps)) => format!("{} in {steps} steps", v.to_decimal()),
            Err(e) => format!("!{}", e.class.name()),
        }
    };
    for (b, e) in [
        ("2", "0"),
        ("2", "1"),
        ("2", "2"),
        ("2", "3"),
        ("2", "10"),
        ("2", "86"),
        ("2", "87"),
        ("(-2)", "87"),
        ("2", "88"),
        ("1.5", "2"),
        ("1.5", "60"),
        ("3", "-2"),
        ("0.00001", "-3"),
        ("0.1", "-25"),
        ("0.1", "-26"),
        ("(-1)", "1000"),
        ("(-1)", "1001"),
        ("0", "0"),
        ("0", "5"),
        ("0", "-1"),
        ("10", "26"),
        ("10", "27"),
        ("(-10)", "26"),
        ("0.5", "40"),
        ("0.5", "41"),
    ] {
        out.push(format!("{b} ** {e} = {}", pow(b, e)));
    }
    // Rounding at the edges.
    out.push(format!(
        "round(MAX) = {}",
        max.round_half_even(0)
            .map(|v| v.to_decimal())
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "round(MIN) = {}",
        min.round_half_even(0)
            .map(|v| v.to_decimal())
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "round(2.5) = {}",
        n("2.5")
            .round_half_even(0)
            .map(|v| v.to_decimal())
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "round(-2.5) = {}",
        neg(n("2.5"))
            .round_half_even(0)
            .map(|v| v.to_decimal())
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "round(0.000000000005, 11) = {}",
        n("0.000000000005")
            .round_half_even(11)
            .map(|v| v.to_decimal())
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "round(0.000000000015, 11) = {}",
        n("0.000000000015")
            .round_half_even(11)
            .map(|v| v.to_decimal())
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "MAX:.12f = {}",
        max.to_fixed(12)
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "MIN:.0f = {}",
        min.to_fixed(0)
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out.push(format!(
        "-0.4:.0f = {}",
        neg(n("0.4"))
            .to_fixed(0)
            .unwrap_or_else(|e| format!("!{}", e.class.name()))
    ));
    out
}

#[test]
fn wide_arithmetic_matches_stored_fixture() {
    let table = wide_table();
    let text = table.join("\n");
    let hashes = vec![fnv_lines(fnv_start(), &table)];
    check_fixture("wide", &text, &hashes);
}

#[test]
fn wide_arithmetic_is_alive() {
    let table = wide_table();
    let text = table.join("\n");
    let overflows = table
        .iter()
        .filter(|l| l.ends_with("!OverflowError"))
        .count();
    let zero_div = table
        .iter()
        .filter(|l| l.ends_with("!ZeroDivisionError"))
        .count();
    assert!(overflows >= 8, "only {overflows} overflows in:\n{text}");
    assert_eq!(zero_div, 2, "in:\n{text}");
    assert!(text.contains("2 ** 10 = 1024 in 5 steps"), "{text}");
    assert!(
        text.contains("-7 // 2 = -4") && text.contains("7 // -2 = -4"),
        "{text}"
    );
    assert!(text.contains("-1 / 3 = -0.333333333334"), "{text}");
    assert!(
        text.contains("0.00001 ** -3 = 1000000000000000 in"),
        "{text}"
    );
    assert!(
        text.contains("MAX / MAX = 1") && text.contains("MIN / MIN = 1"),
        "{text}"
    );
}

// ------------------------------------------------------- cross-process ---

fn all_final_hashes() -> String {
    let (_, a) = showcase();
    let (_, b) = interrupts();
    let c = fnv_lines(fnv_start(), &wide_table());
    format!(
        "{:016x} {:016x} {c:016x}",
        a.last().copied().unwrap_or(0),
        b.last().copied().unwrap_or(0)
    )
}

/// Child half of the cross-process check; the parent spawns it.
#[test]
#[ignore = "spawned by cross_process_fixtures_match"]
fn emit_fixture_hashes() {
    println!("LANG_GOLDEN_HASHES={}", all_final_hashes());
}

/// The lockstep guarantee: a SEPARATE PROCESS reaches the same transcript
/// hashes. Same-process pairs miss per-process instability.
#[test]
fn cross_process_fixtures_match() {
    let exe = std::env::current_exe().expect("test binary path");
    let output = std::process::Command::new(exe)
        .args(["--ignored", "--exact", "emit_fixture_hashes", "--nocapture"])
        .env_remove("UPDATE_GOLDEN")
        .output()
        .expect("spawn child test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child process failed: {stdout}");
    let child = stdout
        .lines()
        .find_map(|l| l.strip_prefix("LANG_GOLDEN_HASHES="))
        .unwrap_or_else(|| panic!("no hash line in child output: {stdout}"))
        .to_string();
    assert_eq!(
        child,
        all_final_hashes(),
        "cross-process fixture mismatch — determinism violation"
    );
}
