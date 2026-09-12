// The check on the Rust gates (docs/06, Testing; T7): each determinism gate
// seeded with a defect it must catch, the way check-checks.mjs seeds the doc
// checks. Run as `node scripts/check-checks.mjs . --rust` from the Rust half
// of scripts/ci.sh, because every case here needs a cargo build.
//
// The gates under test are the ones docs/06's table names: the golden replay,
// the language fixtures, the language's cross-process trace, the source scan,
// the determinism battery, and the battery's cross-architecture compare. A
// mutation is a defect that once shipped or one a gate claims to catch: a
// drifted hash, a transcript gone missing, a match that ran one tick, a float
// in the wrong crate, an emitter that prints no hash line, two architectures
// that disagree. The suite asserts the unmutated tree passes every gate, and
// that every mutation is caught by the named gate with a message naming the
// real problem.
//
// ONE FIXTURE, RESTORED BETWEEN CASES. The doc half copies the tree per
// mutation because a copy is cheap; a Rust fixture is a cargo workspace, and
// cargo fingerprints by path, so the copy lives at a stable path under target/
// (rebuilt incrementally across runs, and cached by CI with the rest of
// target/) and shares the repo's target directory. Every mutation names the
// files it touches, which are backed up before and restored after — a
// mutation that touched an unlisted file would leak into the next case, so the
// runner also refuses one whose named file it cannot find.

import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { SKIP } from "./md-files.mjs";

/** Copy the working tree, minus what the doc walker skips (target/ among them). */
function copyTree(src, dst) {
  mkdirSync(dst, { recursive: true });
  let n = 0;
  for (const entry of readdirSync(src, { withFileTypes: true })) {
    if (SKIP.has(entry.name)) continue;
    const from = join(src, entry.name);
    const to = join(dst, entry.name);
    if (entry.isDirectory()) n += copyTree(from, to);
    else if (entry.isFile()) {
      cpSync(from, to);
      n++;
    }
  }
  return n;
}

/** Rewrite a file, failing loudly if the edit matched nothing. */
function edit(path, from, to) {
  const before = readFileSync(path, "utf8");
  const after = before.replace(from, to);
  if (after === before) {
    throw new Error(`mutation matched nothing in ${path} — the fixture drifted from the mutation`);
  }
  writeFileSync(path, after);
}

/** Flip the first hex digit of the first line of a hash fixture. */
function flipHash(path) {
  const text = readFileSync(path, "utf8");
  const c = text[0];
  const flipped = c === "0" ? "1" : "0";
  writeFileSync(path, flipped + text.slice(1));
}

export function runRustGates(repo) {
  const root = resolve(repo);
  const fixture = join(root, "target", "check-checks", "tree");
  const env = {
    ...process.env,
    CARGO_TARGET_DIR: join(root, "target"),
    // A leaked UPDATE_GOLDEN would turn every drift into a regeneration.
    UPDATE_GOLDEN: undefined,
  };
  delete env.UPDATE_GOLDEN;

  const run = (cmd, args) => {
    const r = spawnSync(cmd, args, { cwd: fixture, env, encoding: "utf8", maxBuffer: 64 << 20 });
    return { code: r.status ?? 1, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
  };
  const cargo = (...args) => () => run("cargo", ["test", "-q", ...args]);
  const battery = (...args) => () => run(join(fixture, "scripts/determinism-battery.sh"), args);
  const hashesOut = join(fixture, "target-hashes.txt");
  const cmpA = join(fixture, "compare-a.txt");
  const cmpB = join(fixture, "compare-b.txt");

  // docs/06, Testing: one entry per gate the table names.
  const GATES = {
    "replay-golden": cargo("-p", "replay", "--test", "golden"),
    "lang-golden": cargo("-p", "lang", "--test", "golden"),
    "lang-determinism": cargo("-p", "lang", "--test", "determinism"),
    "source-scan": cargo("-p", "sim", "--test", "no_floats"),
    battery: battery(hashesOut),
    compare: battery("compare", cmpA, cmpB),
  };

  const replayGolden = "crates/replay/tests/golden";
  const langGolden = "crates/lang/tests/golden";
  const MUTATIONS = [
    // ── the golden replay ───────────────────────────────────────────────────
    { name: "golden replay: a stored hash changed", check: "replay-golden",
      expect: "replay hash drift", files: [`${replayGolden}/showcase.hashes.txt`],
      mutate: (d) => flipHash(join(d, replayGolden, "showcase.hashes.txt")) },
    { name: "golden replay: the stored artifact and the scenario diverged", check: "replay-golden",
      expect: "the in-code scenario and the stored artifact diverged", files: [`${replayGolden}/showcase.replay.ron`],
      mutate: (d) => edit(join(d, replayGolden, "showcase.replay.ron"), /ticks: 220/, "ticks: 221") },
    // A match that ran one tick hashes identically everywhere, which is what
    // the alive assertions exist to refuse (CLAUDE.md).
    { name: "golden replay: a match that ran one tick", check: "replay-golden",
      expect: "the match ended early", files: ["crates/replay/tests/golden.rs"],
      mutate: (d) => edit(join(d, "crates/replay/tests/golden.rs"), /ticks: 220,/, "ticks: 1,") },

    // ── the language fixtures ───────────────────────────────────────────────
    { name: "language fixture: a stored hash changed", check: "lang-golden",
      expect: "hash drift in `showcase` with an identical transcript", files: [`${langGolden}/showcase.hashes.txt`],
      mutate: (d) => flipHash(join(d, langGolden, "showcase.hashes.txt")) },
    { name: "language fixture: the stored transcript changed", check: "lang-golden",
      expect: "transcript drift in `showcase`", files: [`${langGolden}/showcase.transcript.txt`],
      mutate: (d) => writeFileSync(join(d, langGolden, "showcase.transcript.txt"),
        readFileSync(join(d, langGolden, "showcase.transcript.txt"), "utf8") + "seeded line\n") },
    { name: "language fixture: a transcript gone missing", check: "lang-golden",
      expect: "no stored transcript for `interrupts`", files: [`${langGolden}/interrupts.transcript.txt`],
      mutate: (d) => unlinkSync(join(d, langGolden, "interrupts.transcript.txt")) },

    // ── the source scan (CLAUDE.md rules 2, 3 and 4) ────────────────────────
    { name: "source scan: a float in net", check: "source-scan",
      expect: "`f64` — floats are not bit-reproducible", files: ["crates/net/src/lib.rs"],
      mutate: (d) => writeFileSync(join(d, "crates/net/src/lib.rs"),
        readFileSync(join(d, "crates/net/src/lib.rs"), "utf8") + "\npub fn seeded_defect() -> f64 {\n    0.5\n}\n") },
    { name: "source scan: a HashMap in sim", check: "source-scan",
      expect: "`HashMap` — hash iteration order is nondeterministic", files: ["crates/sim/src/lib.rs"],
      mutate: (d) => writeFileSync(join(d, "crates/sim/src/lib.rs"),
        readFileSync(join(d, "crates/sim/src/lib.rs"), "utf8") + "\npub use std::collections::HashMap as SeededDefect;\n") },
    { name: "source scan: a wall clock in lang", check: "source-scan",
      expect: "`Instant` — no wall clock in the sim", files: ["crates/lang/src/lib.rs"],
      mutate: (d) => writeFileSync(join(d, "crates/lang/src/lib.rs"),
        readFileSync(join(d, "crates/lang/src/lib.rs"), "utf8") + "\npub fn seeded_defect() -> std::time::Instant {\n    std::time::Instant::now()\n}\n") },
    // The scan strips comments, so a banned name in one is legal — and a
    // scan that flagged it would be ignored the first time it fired.
    { name: "source scan: a banned name inside a comment is legal", check: "source-scan",
      skipIfClean: true, files: ["crates/net/src/lib.rs"],
      mutate: (d) => writeFileSync(join(d, "crates/net/src/lib.rs"),
        readFileSync(join(d, "crates/net/src/lib.rs"), "utf8") + "\n// no f64 or HashMap here, and no Instant either\n") },

    // ── the battery and the cross-architecture compare ──────────────────────
    { name: "battery: a drifted fixture fails before any hash is emitted", check: "battery",
      expect: "replay hash drift", files: [`${replayGolden}/showcase.hashes.txt`],
      mutate: (d) => flipHash(join(d, replayGolden, "showcase.hashes.txt")) },
    // An emitter that prints nothing would leave a file the compare step reads
    // as agreement on zero streams.
    { name: "battery: an emitter that prints no hash line", check: "battery",
      expect: "no hash line", files: ["crates/replay/tests/golden.rs"],
      mutate: (d) => edit(join(d, "crates/replay/tests/golden.rs"), /"GOLDEN_FINAL_HASH=\{:016x\}"/, '"golden final hash {:016x}"') },
    { name: "compare: the two architectures disagree on a stream", check: "compare",
      expect: "disagree on GOLDEN_FINAL_HASH", files: ["compare-b.txt"],
      mutate: () => edit(cmpB, /^GOLDEN_FINAL_HASH=(.)/m, (_, c) => `GOLDEN_FINAL_HASH=${c === "0" ? "1" : "0"}`) },
    { name: "compare: a hash file with too few streams", check: "compare",
      expect: "fewer than 3 streams", files: ["compare-b.txt"],
      mutate: () => writeFileSync(cmpB, readFileSync(cmpB, "utf8").split("\n").slice(0, 2).join("\n") + "\n") },
    { name: "compare: identical files are accepted", check: "compare",
      skipIfClean: true, expect: "agree on 3 streams", files: [],
      mutate: () => {} },
  ];

  const failures = [];

  // ── The fixture ─────────────────────────────────────────────────────────────
  rmSync(fixture, { recursive: true, force: true });
  if (copyTree(root, fixture) === 0) {
    return { failures: ["check-checks --rust: copied an empty tree — the suite would test nothing"], defects: 0, inverse: 0, gates: 0 };
  }

  // ── The tree itself must pass every gate ────────────────────────────────────
  // The battery runs before the compare so the compare's baseline is the
  // battery's own output against itself.
  for (const gate of Object.keys(GATES)) {
    if (gate === "compare") {
      if (!existsSync(hashesOut)) {
        failures.push("baseline: the battery left no hash file for the compare to read");
        continue;
      }
      cpSync(hashesOut, cmpA);
      cpSync(hashesOut, cmpB);
    }
    const { code, out } = GATES[gate]();
    if (code !== 0) {
      failures.push(
        `baseline: ${gate} rejects your working tree, before any defect was seeded.\n` +
          `  The fixture is a copy of the working tree, so ${gate} fails here for the same\n` +
          `  reason it would fail on its own.\n${out.trim().split("\n").slice(-12).join("\n")}`,
      );
    }
  }
  if (failures.length) return { failures, defects: 0, inverse: 0, gates: Object.keys(GATES).length };

  // ── Every mutation must be caught, by the right gate, with a real message ──
  for (const m of MUTATIONS) {
    if (!GATES[m.check]) {
      failures.push(`${m.name}: names gate "${m.check}", which is not in the GATES table`);
      continue;
    }
    const backups = new Map();
    let ok = true;
    for (const rel of m.files) {
      const path = join(fixture, rel);
      if (!existsSync(path)) {
        failures.push(`${m.name}: names ${rel}, which the fixture does not hold`);
        ok = false;
        break;
      }
      backups.set(path, readFileSync(path));
    }
    if (!ok) continue;
    try {
      m.mutate(fixture);
      const { code, out } = GATES[m.check]();
      const tail = () => out.trim().split("\n").filter((l) => l.trim()).slice(-6).join(" / ");
      if (m.skipIfClean) {
        if (code !== 0) failures.push(`FALSE POSITIVE from ${m.check}: ${m.name}\n  ${tail()}`);
        else if (m.expect && !out.includes(m.expect)) {
          failures.push(`WRONG REPORT from ${m.check}: ${m.name}\n  accepted, but the run does not say ${JSON.stringify(m.expect)}\n  got: ${tail()}`);
        }
      } else if (code === 0) {
        failures.push(`NOT CAUGHT by ${m.check}: ${m.name}\n  the gate passed a tree containing this defect`);
      } else if (m.expect && !out.includes(m.expect)) {
        failures.push(`WRONG MESSAGE from ${m.check}: ${m.name}\n  expected to see ${JSON.stringify(m.expect)}\n  got: ${tail()}`);
      }
    } catch (e) {
      failures.push(`${m.name}: mutation itself failed — ${e.message}`);
    } finally {
      for (const [path, bytes] of backups) writeFileSync(path, bytes);
    }
  }

  const inverse = MUTATIONS.filter((m) => m.skipIfClean).length;
  return { failures, defects: MUTATIONS.length - inverse, inverse, gates: Object.keys(GATES).length };
}
