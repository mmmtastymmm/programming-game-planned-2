//! The mechanical half of CLAUDE.md's determinism rules 2, 3 and 4: scan the
//! workspace's own source for the banned constructs.
//!
//! A test that greps its own source is unusual, but these rules are *syntactic*
//! — a violation is visible without running anything — and they are exactly the
//! rules broken by an innocent-looking convenience commit six months from now.
//! Catching them at review time requires someone to remember; catching them here
//! does not.
//!
//! SCOPE: every workspace MEMBER's `src/`, read from the root `Cargo.toml`, not
//! just this crate's. The language crate (Q5) will be a tree-walking interpreter
//! sitting directly on the hash-critical path, and a scan rooted at `sim` alone
//! would not see one line of it.
//!
//! The member list is the authority on where crates live, and the directory
//! layout is not: this used to walk `crates/*/src`, so a member added at
//! `lang/` or `tools/lang/` would have been skipped in silence while the test
//! went on passing — `files` is non-empty either way. Where the language crate
//! lands is still open, which is exactly why the scope cannot be a guess.
//!
//! LIMITS, so a green run is not read as more than it is:
//!   * Comments and string/char literals are both stripped, so a banned token
//!     inside a string is neither a false positive nor — the direction that
//!     actually matters — a way to blank out the rest of the file. Raw strings
//!     with more than one `#` are not handled.
//!   * This finds names and float literals. It cannot find a float arriving
//!     through a dependency's API, or a `BTreeMap` iterated in an order that
//!     depends on something nondeterministic upstream. Rules first, gate second.

// Test-harness arithmetic is not sim state: it never runs on a peer, never
// enters a state hash, and a panic here is a failing test rather than a desync.
// The workspace deny exists for code on the hash-critical path.
#![allow(clippy::arithmetic_side_effects)]

use std::fs;
use std::path::{Path, PathBuf};

/// Token → why it is banned. The reasons live here because the failure message
/// is the only place most people will read them.
const BANNED: &[(&str, &str)] = &[
    (
        "f32",
        "floats are not bit-reproducible across machines (rule 2)",
    ),
    (
        "f64",
        "floats are not bit-reproducible across machines (rule 2)",
    ),
    (
        "HashMap",
        "hash iteration order is nondeterministic — use BTreeMap (rule 3)",
    ),
    (
        "HashSet",
        "hash iteration order is nondeterministic — use BTreeSet (rule 3)",
    ),
    ("SystemTime", "no wall clock in the sim (rule 4)"),
    ("Instant", "no wall clock in the sim (rule 4)"),
    // hash.rs exists *because* of DefaultHasher: its doc comment says FNV-1a is
    // used since, unlike DefaultHasher, it is not seeded with per-process
    // randomness. That hazard had no mechanical backstop until this line.
    (
        "DefaultHasher",
        "seeded with per-process randomness (rule 4)",
    ),
    ("RandomState", "seeded with per-process randomness (rule 4)"),
    (
        "SipHasher",
        "not guaranteed stable across Rust releases (rule 4)",
    ),
    (
        "thread_rng",
        "OS randomness — use a named seeded Stream (rule 4)",
    ),
    (
        "getrandom",
        "OS randomness — use a named seeded Stream (rule 4)",
    ),
    (
        "random",
        "OS randomness — use a named seeded Stream (rule 4)",
    ),
];

/// CARGO_MANIFEST_DIR is the PACKAGE directory, not this test's: crates/sim ->
/// crates -> the workspace root.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the workspace root is two levels above this crate")
        .to_path_buf()
}

/// Strip a `#` comment, ignoring a `#` inside quotes.
fn uncomment(line: &str) -> String {
    let mut quote: Option<char> = None;
    let mut out = String::new();
    for c in line.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == '#' {
            break;
        }
        out.push(c);
    }
    out
}

/// Every quoted string in a fragment of TOML.
fn quoted(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    let mut buf = String::new();
    for c in s.chars() {
        if let Some(q) = quote {
            if c == q {
                out.push(std::mem::take(&mut buf));
                quote = None;
            } else {
                buf.push(c);
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        }
    }
    out
}

/// The `members = [...]` list, as written.
///
/// Anchored on a line whose first token is `members`, because `default-members`
/// ends with the same word and reading it instead would validate a subset —
/// the same trap `scripts/lib/workspace-lints.mjs` documents on its own copy of
/// this parse. Comments are stripped first: `# "crates/lang",  # not yet` is an
/// ordinary thing to write.
fn parse_members(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for raw in manifest.lines() {
        let line = uncomment(raw);
        let scan: &str = if inside {
            &line
        } else {
            let t = line.trim_start();
            if !t.starts_with("members") {
                continue;
            }
            match t.find('=') {
                Some(eq) => {
                    inside = true;
                    &t[eq..]
                }
                None => continue,
            }
        };
        out.extend(quoted(scan));
        if scan.contains(']') {
            break;
        }
    }
    out
}

/// `crates/*` style globs, which Cargo allows in `members`.
fn expand(root: &Path, member: &str) -> Vec<String> {
    if !member.contains('*') {
        return vec![member.to_string()];
    }
    let (parent, pattern) = member.rsplit_once('/').unwrap_or((".", member));
    assert!(
        !parent.contains('*'),
        "workspace member `{member}` globs above its last segment — teach this test that \
         shape rather than letting the determinism scan cover less than the workspace"
    );
    let mut out = Vec::new();
    let entries = fs::read_dir(root.join(parent)).expect("readable member directory");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let matches = match pattern.split_once('*') {
            Some((pre, post)) => {
                name.starts_with(pre)
                    && name.ends_with(post)
                    && name.len() >= pre.len() + post.len()
            }
            None => pattern == name,
        };
        if matches && path.join("Cargo.toml").is_file() {
            out.push(format!("{parent}/{name}"));
        }
    }
    out.sort();
    // A glob that matches nothing would narrow the scan without saying so: the
    // member is dropped here, so `every_workspace_member_is_scanned` never sees
    // it and both tests would pass over less than the workspace. Cargo refuses
    // an unmatched glob today, so this is a second line rather than the only
    // one — and it keeps this parser saying what its JS twin says
    // (`member "…" matched no crate`), which is the disagreement two parsers of
    // one format are for.
    assert!(
        !out.is_empty(),
        "workspace member `{member}` matched no crate — every member must be scanned"
    );
    out
}

/// Every workspace member path, expanded.
fn workspace_members(root: &Path) -> Vec<String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("readable root manifest");
    parse_members(&manifest)
        .iter()
        .flat_map(|m| expand(root, m))
        .collect()
}

fn crate_sources() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    for member in workspace_members(&root) {
        let src = root.join(&member).join("src");
        assert!(
            src.is_dir(),
            "workspace member `{member}` has no src/ — the determinism scan cannot cover it"
        );
        rust_files(&src, &mut out);
    }
    out.sort();
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Blank out comments **and string/char literals**, preserving line structure so
/// reported line numbers stay right.
///
/// The literal handling is not decoration. Without it, `"http://example.com"`
/// puts the stripper into line-comment mode at the `//`, hiding everything after
/// it on that line — and a `/*` inside a literal swallows the rest of the file
/// until an unrelated `*/`. Both are false *negatives*, which is the direction
/// that costs something.
fn strip(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let (mut i, mut block) = (0usize, 0usize);
    while i < c.len() {
        let two = (c[i], c.get(i + 1).copied().unwrap_or('\0'));
        if block > 0 {
            match two {
                ('*', '/') => {
                    block -= 1;
                    i += 2;
                }
                ('/', '*') => {
                    block += 1;
                    i += 2;
                }
                _ => {
                    if c[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
        } else if two == ('/', '*') {
            block += 1;
            i += 2;
        } else if two == ('/', '/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if c[i] == 'r' && (two.1 == '"' || (two.1 == '#' && c.get(i + 2) == Some(&'"'))) {
            // r"..." and r#"..."#
            let hashes = if two.1 == '#' { 1 } else { 0 };
            i += 1 + hashes + 1;
            loop {
                if i >= c.len() {
                    break;
                }
                if c[i] == '"' && (hashes == 0 || c.get(i + 1) == Some(&'#')) {
                    i += 1 + hashes;
                    break;
                }
                if c[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
        } else if c[i] == '"' {
            i += 1;
            while i < c.len() && c[i] != '"' {
                if c[i] == '\\' {
                    i += 1;
                }
                if i < c.len() && c[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 1;
        } else if c[i] == '\'' {
            // A char literal is `'x'` or `'\n'`; anything else starting with `'`
            // is a lifetime and must be left alone.
            let esc = c.get(i + 1) == Some(&'\\');
            let close = if esc { i + 3 } else { i + 2 };
            if c.get(close) == Some(&'\'') {
                i = close + 1;
            } else {
                out.push(c[i]);
                i += 1;
            }
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// Split on anything that cannot appear in a Rust identifier, so `f32` matches
/// but `buf32` does not.
fn tokens(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
}

/// Does this line contain a float literal?
///
/// Rust has four spellings and the first version of this recognised one and a
/// half of them. `digit . digit` plus an f32/f64 suffix left `1e-3` and `2E5`
/// invisible — both are `f64`, both are rule-2 violations, and the scan stayed
/// green on a file where `let _ = 1.5;` immediately turned it red, which is the
/// worst possible shape for a backstop. `1.` was invisible for the same reason.
/// In the other direction the suffix test was a bare `ends_with("f64")` over a
/// token starting with a digit, so the hex constant `0x1f64` was reported as a
/// float — and this crate already carries `0xcbf29ce484222325` and
/// `0x9E3779B97F4A7C15`.
///
/// So this walks numeric literals properly rather than pattern-matching around
/// them. A literal is entered only where a digit is not preceded by an
/// identifier character or a `.`, which is what keeps tuple access (`self.0`,
/// `x.0.1`) and ranges (`0..20`) out.
fn has_float_literal(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    let ident = |ch: char| ch.is_alphanumeric() || ch == '_';
    let mut i = 0;
    while i < c.len() {
        if !c[i].is_ascii_digit() || (i > 0 && (ident(c[i - 1]) || c[i - 1] == '.')) {
            i += 1;
            continue;
        }
        // A radix prefix is always an integer: consume it whole so its digits
        // can never be read as a mantissa, an exponent or a suffix.
        if c[i] == '0' && i + 1 < c.len() && matches!(c[i + 1], 'x' | 'X' | 'o' | 'O' | 'b' | 'B') {
            i += 2;
            while i < c.len() && ident(c[i]) {
                i += 1;
            }
            continue;
        }
        let mut j = i;
        while j < c.len() && (c[j].is_ascii_digit() || c[j] == '_') {
            j += 1;
        }
        // `1.5`, and the trailing-dot form `1.` — but not `0..20` (a range) and
        // not `1.foo()` (a method call, which needs a typed receiver anyway).
        if j < c.len() && c[j] == '.' {
            let after = c.get(j + 1).copied();
            if after.is_some_and(|ch| ch.is_ascii_digit()) {
                return true;
            }
            if !after.is_some_and(|ch| ch == '.' || ident(ch)) {
                return true;
            }
        }
        // `1e-3`, `2E5`, `1.5e10`. Scientific notation is always a float.
        let mut k = j;
        if k < c.len() && c[k] == '.' && c.get(k + 1).is_some_and(char::is_ascii_digit) {
            k += 1;
            while k < c.len() && (c[k].is_ascii_digit() || c[k] == '_') {
                k += 1;
            }
        }
        if k < c.len() && matches!(c[k], 'e' | 'E') {
            let mut e = k + 1;
            if e < c.len() && matches!(c[e], '+' | '-') {
                e += 1;
            }
            if c.get(e).is_some_and(char::is_ascii_digit) {
                return true;
            }
        }
        // An explicit suffix: `1f64`, `1.0f32`, `3_f64`.
        let suffix: String = c[k..].iter().take_while(|ch| ident(**ch)).collect();
        let suffix = suffix.trim_start_matches('_');
        if suffix == "f32" || suffix == "f64" {
            return true;
        }
        i = k.max(i + 1);
    }
    false
}

#[test]
fn workspace_source_is_free_of_nondeterministic_constructs() {
    let files = crate_sources();
    assert!(
        !files.is_empty(),
        "found no source to scan — the test is checking nothing"
    );

    let mut violations = Vec::new();
    for file in &files {
        let text = strip(&fs::read_to_string(file).expect("readable source"));
        for (n, line) in text.lines().enumerate() {
            let at = format!("{}:{}", file.display(), n + 1);
            for token in tokens(line) {
                if let Some((_, why)) = BANNED.iter().find(|(t, _)| *t == token) {
                    violations.push(format!("{at}  `{token}` — {why}\n      {}", line.trim()));
                }
            }
            if has_float_literal(line) {
                violations.push(format!(
                    "{at}  float literal — floats are not bit-reproducible (rule 2)\n      {}",
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} determinism violation(s) in workspace source:\n\n{}\n\nSee CLAUDE.md. \
         If one of these is genuinely safe, it needs a documented exemption here \
         — not a deleted check.",
        violations.len(),
        violations.join("\n")
    );
}

/// The SCOPE, checked — every workspace member contributes source to the scan.
///
/// `!files.is_empty()` was the only guard, and it is satisfied by `sim` alone:
/// a member outside `crates/` was skipped in silence while the run stayed green,
/// which is the shape of coverage loss this whole file exists to prevent one
/// level down.
#[test]
fn every_workspace_member_is_scanned() {
    let root = repo_root();
    let members = workspace_members(&root);
    assert!(
        !members.is_empty(),
        "read no workspace members from Cargo.toml — the scan's scope is a guess"
    );

    let files = crate_sources();
    for member in &members {
        let src = root.join(member).join("src");
        assert!(
            files.iter().any(|f| f.starts_with(&src)),
            "workspace member `{member}` contributed no file to the determinism scan"
        );
    }
}

/// The member parser, on the shapes Cargo accepts and a person writes.
#[test]
fn member_parser_reads_the_shapes_cargo_accepts() {
    let manifest = "[workspace]\n\
         default-members = [\"crates/sim\"]\n\
         members = [\n\
         \x20   \"crates/sim\",\n\
         \x20   # \"crates/lang\",  # not written yet\n\
         \x20   \"tools/x\",\n\
         ]\n";
    assert_eq!(
        parse_members(manifest),
        vec!["crates/sim".to_string(), "tools/x".to_string()],
        "default-members must not be read as the member list, and a commented-out \
         member must not be read as one"
    );

    let inline = "members = [\"a\", \"b\"]\nsomething-else = [\"c\"]\n";
    assert_eq!(
        parse_members(inline),
        vec!["a".to_string(), "b".to_string()],
        "the list ends at its closing bracket"
    );
}

/// The scanner, scanned.
///
/// Every case below is a line the main test would have to judge, and the point
/// is the FALSE column as much as the TRUE one: before this existed, nothing
/// planted a violation and watched the scan fire, so `1e-3` sat unrecognised
/// while the suite reported the workspace clean. A backstop nobody has seen
/// fail is not a backstop. Runs in microseconds and needs no fixture.
#[test]
fn float_literal_scanner_sees_every_spelling() {
    const FLOATS: &[&str] = &[
        "let x = 1.5;",
        "let x = 0.5;",
        "let x = 1.0.floor();",
        "let x = 1.;",
        "let dt = 1e-3;",
        "let scale = 2E5;",
        "let x = 1.5e10;",
        "let x = 6.02e+23;",
        "let x = 1f64;",
        "let x = 3_f32;",
        "let x = 1.0f32;",
    ];
    const INTEGERS: &[&str] = &[
        "let x = 0..20;",
        "for i in 0..=9 {}",
        "let a = self.0;",
        "let b = x.0.1;",
        "const M: u64 = 0x1f64;",
        "const P: u64 = 0xcbf29ce484222325;",
        "const G: u64 = 0x9E3779B97F4A7C15;",
        "let m = 0b1010;",
        "let o = 0o755;",
        "let n = 1_000_000i64;",
        "let n = 42;",
        "let v = buf32;",
        "let t = tick2e;",
    ];

    let mut wrong = Vec::new();
    for line in FLOATS {
        if !has_float_literal(line) {
            wrong.push(format!("MISSED a float literal: {line}"));
        }
    }
    for line in INTEGERS {
        if has_float_literal(line) {
            wrong.push(format!("FALSE POSITIVE on integer source: {line}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} scanner error(s) — the rule-2 backstop does not see what it claims to:\n\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
