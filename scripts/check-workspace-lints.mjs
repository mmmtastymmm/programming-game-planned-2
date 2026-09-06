#!/usr/bin/env node
// Every workspace member opts into the workspace lints.
//
//   node scripts/check-workspace-lints.mjs .
//
// `[workspace.lints]` does nothing for a crate that omits `[lints] workspace =
// true`, and clippy stays green either way — so `arithmetic_side_effects =
// "deny"`, which exists so the coming language crate is covered, can be inert in
// a member and nothing says so. The rule and its TOML reading live in
// scripts/lib/workspace-lints.mjs; this file is the entry point that CI, the
// pre-commit hook and scripts/check-checks.mjs all run, so there is one path
// through the check rather than one path with mutations and another that runs.
//
// It takes the REPO root and reads only Cargo.toml files, so it needs no Rust
// toolchain and belongs in the fast half of scripts/ci.sh.

import { lintOptInProblems } from "./lib/workspace-lints.mjs";

const root = process.argv[2] ?? ".";
const { problems, paths } = lintOptInProblems(root);

if (problems.length) {
  console.error(`✗ ${problems.length} workspace lint problem(s):\n`);
  for (const p of problems) console.error("  " + p);
  process.exit(1);
}

// A count, not a bare tick: "no problems" over zero members is exactly the
// green-on-nothing result this repo keeps finding. The count is the one
// lintOptInProblems actually walked — computing it here from a second parse
// would report a number nothing verified.
console.log(
  `✓ ${paths.length} workspace member(s) opt into the workspace lints ` +
    `(the arithmetic_side_effects deny reaches all of them)`,
);
