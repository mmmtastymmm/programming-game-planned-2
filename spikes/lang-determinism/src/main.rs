//! Q5 spike: can a stock embeddable runtime be made bit-deterministic?
//!
//! The rules it has to survive (CLAUDE.md): no floats in state-affecting paths,
//! no hash-order iteration, no wall clock, no OS randomness — and the whole
//! thing must agree BIT FOR BIT ACROSS PROCESSES, which is the property a
//! same-process test cannot see.
//!
//!   cargo run --manifest-path spikes/lang-determinism/Cargo.toml
//!
//! Each engine runs a battery that would EXPOSE nondeterminism if it existed:
//! map/table keys inserted in scrambled order and then iterated, a sort with
//! ties, integer arithmetic, string building. The transcript is hashed, and the
//! parent re-runs each battery in fresh child processes and compares.

use std::process::Command;

// ── FNV-1a, same as crates/sim/src/hash.rs (copied: this is a spike) ─────────
fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ── Rhai ────────────────────────────────────────────────────────────────────
// Built from `new_raw()` and a hand-picked package list rather than
// `Engine::new()`: the standard set includes BasicTimePackage, i.e. a wall
// clock, which rule 4 forbids outright.
fn rhai_engine() -> rhai::Engine {
    use rhai::packages::Package;
    let mut engine = rhai::Engine::new_raw();
    engine.register_global_module(rhai::packages::CorePackage::new().as_shared_module());
    engine.register_global_module(rhai::packages::BasicArrayPackage::new().as_shared_module());
    engine.register_global_module(rhai::packages::BasicMapPackage::new().as_shared_module());
    engine.register_global_module(rhai::packages::MoreStringPackage::new().as_shared_module());

    // FINDING: Rhai's recursion and size limits DEFAULT DIFFERENTLY between
    // debug and release builds. fib(18) stack-overflowed under the debug
    // default and would not have under release. Two peers on different build
    // profiles would fault at different depths — a desync produced by a build
    // flag rather than by code. Every limit has to be pinned explicitly and
    // become part of the spec, not left to a default.
    engine.set_max_call_levels(256);
    engine.set_max_operations(10_000_000);
    engine.set_max_expr_depths(64, 64);
    engine.set_max_string_size(1_000_000);
    engine.set_max_array_size(100_000);
    engine.set_max_map_size(100_000);
    engine
}

// Battery v2. v1 passed, which proves nothing unless it could have failed:
// eight map keys and a sort is not enough surface to expose a hash-ordered
// container or an address-dependent dispatch. This version pushes on the places
// a scripting runtime usually hides one — many keys, many user functions,
// closures, recursion, and integer edge cases.
const RHAI_BATTERY: &str = r#"
let out = "";

// 60 keys inserted in an order that is neither sorted nor insertion-stable.
// Any HashMap anywhere in the map implementation shows up here.
let m = #{};
let i = 0;
while i < 60 {
    let k = "k" + ((i * 37) % 60).to_string();
    m[k] = i;
    i += 1;
}
for k in m.keys() { out += k; out += ","; }
out += "|";

// Many user-defined functions. Rhai hashes function signatures for dispatch;
// if that hasher were randomly seeded, collision resolution could reorder.
fn f0(x) { x + 0 } fn f1(x) { x + 1 } fn f2(x) { x + 2 } fn f3(x) { x + 3 }
fn f4(x) { x + 4 } fn f5(x) { x + 5 } fn f6(x) { x + 6 } fn f7(x) { x + 7 }
out += (f0(1) + f1(2) + f2(3) + f3(4) + f4(5) + f5(6) + f6(7) + f7(8)).to_string();
out += "|";

// Closures capturing environment.
let base = 10;
let add = |x| x + base;
out += add.call(5).to_string();
out += "|";

// Recursion.
fn fib(n) { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
out += fib(18).to_string();
out += "|";

// Integer edge cases: division truncation and modulo sign are the classic
// place two implementations quietly disagree.
out += (7 / 2).to_string() + "," + (-7 / 2).to_string() + ",";
out += (7 % 3).to_string() + "," + (-7 % 3).to_string();
out += "|";

// Sort with many ties — where an unstable sort's tie order can differ.
let a = [];
let j = 0;
while j < 40 { a.push((j * 7) % 5); j += 1; }
a.sort();
for x in a { out += x.to_string(); }
out += "|";

// String building at size, and map values read back in key order.
let s = "";
for k in m.keys() { s += m[k].to_string(); }
out += s.len.to_string() + ":" + s;
out
"#;

fn rhai_transcript() -> String {
    let engine = rhai_engine();
    let mut out = match engine.eval::<String>(RHAI_BATTERY) {
        Ok(s) => s,
        Err(e) => format!("BATTERY ERROR: {e}"),
    };
    // Ambient nondeterminism must be absent, not merely unused.
    for probe in ["timestamp()", "rand()", "print(1)", "1.5"] {
        let reachable = engine.eval::<rhai::Dynamic>(probe).is_ok();
        out += &format!(
            "|{probe}={}",
            if reachable { "REACHABLE" } else { "absent" }
        );
    }
    out
}

// ── Lua ─────────────────────────────────────────────────────────────────────
// Only the libraries that cannot observe the outside world are opened: no os,
// no io, no math.random seeding.
fn lua_transcript() -> String {
    let lua = match mlua::Lua::new_with(
        mlua::StdLib::TABLE | mlua::StdLib::STRING,
        mlua::LuaOptions::default(),
    ) {
        Ok(l) => l,
        Err(e) => return format!("LUA INIT ERROR: {e}"),
    };
    const BATTERY: &str = r#"
        local out = {}
        local m = {}
        m.zeta = 1; m.alpha = 2; m.mike = 3; m.quebec = 4
        m.bravo = 5; m.yankee = 6; m.delta = 7; m.oscar = 8
        for k, _ in pairs(m) do out[#out+1] = k .. "," end
        out[#out+1] = "|"
        local a = {5, 3, 9, 1, 3, 7, 1}
        table.sort(a)
        for _, x in ipairs(a) do out[#out+1] = tostring(x) .. "," end
        out[#out+1] = "|"
        local acc = 1
        for i = 1, 19 do acc = (acc * 31 + i) % 1000003 end
        out[#out+1] = tostring(acc)
        out[#out+1] = "|"
        -- Lua 5.4 has integers, but '/' is ALWAYS float division.
        out[#out+1] = "div:" .. tostring(7 / 2) .. ",idiv:" .. tostring(7 // 2)
        out[#out+1] = "|os=" .. tostring(os ~= nil) .. ",math=" .. tostring(math ~= nil)
        return table.concat(out)
    "#;
    match lua.load(BATTERY).eval::<String>() {
        Ok(s) => s,
        Err(e) => format!("BATTERY ERROR: {e}"),
    }
}

// ── Throughput ──────────────────────────────────────────────────────────────
// Determinism is necessary, not sufficient: a deterministic interpreter that
// cannot run 50 units per tick is still the wrong answer. This measures the
// budget Q6 has to spend. Wall clock is fine here — a spike is not sim code.
fn rhai_throughput() {
    // A plausible per-unit tick: look around, decide, act. Deliberately not a
    // microbenchmark of arithmetic.
    const UNIT_PROGRAM: &str = r#"
        let best = ();
        let best_d = 999999;
        for e in enemies {
            let d = (e[0] - me_x).abs() + (e[1] - me_y).abs();
            if d < best_d { best_d = d; best = e; }
        }
        if best_d < 3 { "retreat" } else if best == () { "scan" } else { "advance" }
    "#;
    let engine = rhai_engine();
    let ast = match engine.compile(UNIT_PROGRAM) {
        Ok(a) => a,
        Err(e) => {
            println!("throughput: compile failed: {e}");
            return;
        }
    };

    let mut enemies = rhai::Array::new();
    for i in 0..8i64 {
        let mut pair = rhai::Array::new();
        pair.push((i * 3 % 17).into());
        pair.push((i * 5 % 13).into());
        enemies.push(pair.into());
    }

    let runs = 20_000;
    let start = std::time::Instant::now();
    for i in 0..runs {
        let mut scope = rhai::Scope::new();
        scope.push("enemies", enemies.clone());
        scope.push("me_x", (i % 20) as i64);
        scope.push("me_y", (i % 13) as i64);
        if let Err(e) = engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast) {
            // Same rule as the battery: a measurement of a program that does not
            // run is worse than no measurement, because it gets quoted. This
            // number appears in Q5's ruling and in TASKS M2 as the budget.
            println!("\n=== rhai throughput ===");
            println!("ABORTED: the program errored on iteration {i}: {e}");
            println!("the timing would have been the cost of failing, not of working");
            return;
        }
    }
    let elapsed = start.elapsed();
    let per_run = elapsed.as_secs_f64() / runs as f64;
    println!("\n=== rhai throughput ===");
    println!("per unit-tick   : {:.1} us", per_run * 1e6);
    println!("unit-ticks/sec  : {:.0}", 1.0 / per_run);
    for (units, tps) in [(50, 10), (50, 30), (200, 10), (200, 30)] {
        let budget = per_run * units as f64 * tps as f64;
        println!(
            "  {units:>3} units @ {tps:>2} ticks/s : {:.1}% of one core{}",
            budget * 100.0,
            if budget < 0.5 { "" } else { "   <-- tight" }
        );
    }
    println!(
        "({} build{})",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        if cfg!(debug_assertions) {
            " — release is several times faster; re-run with --release"
        } else {
            ""
        }
    );
}

fn transcript(engine: &str) -> String {
    match engine {
        "rhai" => rhai_transcript(),
        "lua" => lua_transcript(),
        other => panic!("unknown engine {other}"),
    }
}

const ENGINES: [&str; 2] = ["rhai", "lua"];
const CHILD_RUNS: usize = 4;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 && args[1] == "--child" {
        println!("HASH={:016x}", fnv(&transcript(&args[2])));
        return;
    }

    let exe = std::env::current_exe().expect("own path");
    let mut verdicts = Vec::new();

    for engine in ENGINES {
        let local = transcript(engine);
        println!("\n=== {engine} ===");
        println!("transcript : {local}");
        let local_hash = format!("{:016x}", fnv(&local));
        println!("local hash : {local_hash}");

        let mut hashes = Vec::new();
        let mut failures = Vec::new();
        for _ in 0..CHILD_RUNS {
            let out = Command::new(&exe)
                .args(["--child", engine])
                .output()
                .expect("spawn child");
            let s = String::from_utf8_lossy(&out.stdout);
            match s.lines().find_map(|l| l.strip_prefix("HASH=")) {
                Some(h) => hashes.push(h.to_string()),
                // A child that produced NO hash must never be folded into the
                // agreement test as a string. Four identically-failing children
                // are four equal strings, and "all equal" would read as
                // DETERMINISTIC — the exact hole this harness claims to close.
                None => failures.push(String::from_utf8_lossy(&out.stderr).trim().to_string()),
            }
        }
        println!("child hashes ({CHILD_RUNS} fresh processes):");
        for h in &hashes {
            println!("  {h}");
        }
        for f in &failures {
            println!("  CHILD FAILED: {f}");
        }

        // A battery that fails to run produces an identical error transcript in
        // every process, which is indistinguishable from a pass by hash alone.
        // v2 of this spike did exactly that for one run. Score it as invalid —
        // and check BOTH halves: the parent's transcript, and whether every
        // child actually produced a hash.
        let ran = !local.contains("BATTERY ERROR") && !local.contains("INIT ERROR");
        let verdict = if !ran {
            "INVALID — the battery did not run in this process"
        } else if !failures.is_empty() {
            "INVALID — a child produced no hash at all"
        } else if hashes.iter().all(|h| *h == local_hash) {
            // Compared against the PARENT's hash, not merely against each
            // other: four children can agree with one another and all disagree
            // with the parent, and that is a desync too.
            "DETERMINISTIC across processes"
        } else {
            "NONDETERMINISTIC across processes"
        };
        println!("verdict    : {verdict}");
        verdicts.push((engine, verdict.starts_with("DETERMINISTIC")));
    }

    rhai_throughput();

    println!("\n=== summary ===");
    for (engine, agree) in verdicts {
        println!("  {engine:<6} {}", if agree { "pass" } else { "FAIL" });
    }
}
