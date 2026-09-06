// Is the workspace lint opt-in actually in force?
//
// `[workspace.lints]` does nothing for a crate that omits `[lints] workspace =
// true`, and clippy stays green either way. The deny exists so the coming
// language crate is covered; a silent opt-out would defeat it exactly when it
// starts to matter — so every branch below REPORTS, and none of them `continue`
// quietly.
//
// WHY THIS IS A MODULE. It used to live inside check-checks.mjs, where it had
// mutations and no CI step of its own: it was absent from scripts/ci.sh, absent
// from the workflow's step label, and reached the real manifests only as a side
// effect of the meta-check's baseline loop — which scripts/ci.sh skips whenever
// it is handed a directory, i.e. on every pre-commit run. A real invariant was
// being enforced by an accident of another check's control flow, and refactoring
// that loop would have retired it silently while the meta-check went on printing
// that the opt-in was in force. `check-workspace-lints.mjs` is now the entry
// point CI and the hook run; check-checks.mjs spawns that same script, so the
// thing with mutations and the thing CI runs are one thing.
//
// This is not a TOML parser and does not try to be. It handles the spellings
// Cargo accepts that a person would plausibly write; anything it cannot read is
// a reported failure rather than a silent pass.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

/** Strip a `#` comment from a line, ignoring `#` inside quotes. */
function uncomment(line) {
  let q = null;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (q) {
      if (c === q) q = null;
    } else if (c === '"' || c === "'") {
      q = c;
    } else if (c === "#") {
      return line.slice(0, i);
    }
  }
  return line;
}

/**
 * The body of a `[section]`, up to the next table header.
 *
 * Every line is read with its comment stripped FIRST. `[lints]  # inherit the
 * workspace deny` is valid TOML that Cargo accepts, and the header pattern
 * demanded end-of-line after the bracket — so the section was not found, the
 * body came back empty, and a fully opted-in crate was reported as opting out.
 * The comment at the call site claimed trailing comments were handled, and it
 * was true of the body and false of the header.
 */
function tomlSection(text, name) {
  const lines = text.split("\n").map(uncomment);
  const at = lines.findIndex((l) => l.trim() === `[${name}]`);
  if (at === -1) return null;
  const rest = lines.slice(at + 1);
  const next = rest.findIndex((l) => /^\s*\[/.test(l));
  return (next === -1 ? rest : rest.slice(0, next)).join("\n");
}

/** Expand one `members` entry, which Cargo allows to be a glob, per segment. */
function expandMember(dir, glob) {
  if (!glob.includes("*")) return [glob];
  let candidates = [""];
  for (const seg of glob.split("/")) {
    const next = [];
    for (const base of candidates) {
      const abs = join(dir, base);
      if (!existsSync(abs)) continue;
      if (!seg.includes("*")) {
        if (existsSync(join(abs, seg))) next.push(base ? `${base}/${seg}` : seg);
        continue;
      }
      const re = new RegExp(
        `^${seg.split("*").map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join(".*")}$`,
      );
      for (const e of readdirSync(abs, { withFileTypes: true })) {
        if (e.isDirectory() && re.test(e.name)) next.push(base ? `${base}/${e.name}` : e.name);
      }
    }
    candidates = next;
  }
  // Slicing at the first `*` mishandled both shapes that matter: `crates/sim-*`
  // produced the base `crates/sim-`, which does not exist and was skipped in
  // silence, and `crates/*/core` expanded to `crates/sim` — a crate that is not
  // a member — while skipping every crate that is.
  return candidates.filter((c) => existsSync(join(dir, c, "Cargo.toml")));
}

/**
 * The workspace's member paths, read once and shared.
 *
 * `members\s*=` unanchored also matches the tail of `default-members =`, which
 * would silently validate a subset — hence the `^\s*`. Comments are stripped
 * before the strings are pulled out, because Cargo allows them and people write
 * them.
 */
export function workspaceMembers(dir) {
  const rootPath = join(dir, "Cargo.toml");
  const empty = { listed: [], paths: [], problems: { unreadable: true, emptyGlobs: [] } };
  if (!existsSync(rootPath)) return empty;
  const block = /^\s*members\s*=\s*\[([\s\S]*?)\]/m.exec(readFileSync(rootPath, "utf8"));
  if (!block) return empty;

  const listed =
    block[1]
      .split("\n")
      .map(uncomment)
      .join("\n")
      .match(/"[^"]+"|'[^']+'/g)
      ?.map((t) => t.slice(1, -1)) ?? [];

  const paths = [];
  const emptyGlobs = [];
  for (const g of listed) {
    const expanded = expandMember(dir, g);
    if (expanded.length === 0) {
      emptyGlobs.push(`Cargo.toml: member "${g}" matched no crate — every member must be checked`);
    }
    paths.push(...expanded);
  }
  return { listed, paths, problems: { unreadable: false, emptyGlobs } };
}

/**
 * `{ problems, paths }` — what is wrong, and the member paths that were checked.
 *
 * The paths come back rather than being recomputed by the caller: printing "N
 * members opt in" from a second `workspaceMembers()` call means the N reported
 * is not the N inspected, and the two can disagree if anything touches the tree
 * between them.
 */
export function lintOptInProblems(dir) {
  const found = [];
  const rootPath = join(dir, "Cargo.toml");
  if (!existsSync(rootPath)) {
    return { problems: ["Cargo.toml: workspace root manifest is missing"], paths: [] };
  }
  const rootManifest = readFileSync(rootPath, "utf8");

  const declaresDeny = rootManifest
    .split("\n")
    .map(uncomment)
    .some((l) => l.trim() === "[workspace.lints.clippy]");
  if (!declaresDeny) {
    found.push("Cargo.toml: no [workspace.lints.clippy] section — the deny is declared nowhere");
  }

  const { listed, paths, problems: memberProblems } = workspaceMembers(dir);
  if (memberProblems.unreadable) {
    return { problems: [...found, "Cargo.toml: could not read workspace members"], paths: [] };
  }
  if (listed.length === 0) {
    found.push("Cargo.toml: workspace members list is empty or unreadable");
  }
  found.push(...memberProblems.emptyGlobs);

  for (const rel of paths) {
    const manifest = join(dir, rel, "Cargo.toml");
    if (!existsSync(manifest)) {
      found.push(`${rel}/Cargo.toml: listed as a workspace member but absent`);
      continue;
    }
    const text = readFileSync(manifest, "utf8");
    // Both spellings Cargo accepts, and a trailing comment is valid TOML.
    const section = tomlSection(text, "lints") ?? "";
    const inSection = section
      .split("\n")
      .map(uncomment)
      .some((l) => /^\s*workspace\s*=\s*true\s*$/.test(l));
    const dotted = text
      .split("\n")
      .map(uncomment)
      .some((l) => /^\s*lints\.workspace\s*=\s*true\s*$/.test(l));
    if (!inSection && !dotted) {
      found.push(
        `${rel}/Cargo.toml: does not opt into the workspace lints ` +
          `([lints] workspace = true) — the arithmetic_side_effects deny is inert here`,
      );
    }
  }
  return { problems: found, paths };
}
