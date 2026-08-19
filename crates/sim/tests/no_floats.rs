//! The mechanical half of CLAUDE.md's determinism rules 2, 3 and 4: scan the
//! workspace's own source for the banned constructs.
//!
//! A test that greps its own source is unusual, but these rules are *syntactic*
//! — a violation is visible without running anything — and they are exactly the
//! rules broken by an innocent-looking convenience commit six months from now.
//! Catching them at review time requires someone to remember; catching them here
//! does not.
//!
//! SCOPE: every `crates/*/src` in the workspace, not just this crate's. The
//! language crate (Q5) will be a tree-walking interpreter sitting directly on
//! the hash-critical path, and a scan rooted at `sim` alone would not see one
//! line of it.
//!
//! LIMITS, so a green run is not read as more than it is:
//!   * Comments and string/char literals are both stripped, so a banned token
//!     inside a string is neither a false positive nor — the direction that
//!     actually matters — a way to blank out the rest of the file. Raw strings
//!     with more than one `#` are not handled.
//!   * This finds names and float literals. It cannot find a float arriving
//!     through a dependency's API, or a `BTreeMap` iterated in an order that
//!     depends on something nondeterministic upstream. Rules first, gate second.

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

fn crate_sources() -> Vec<PathBuf> {
    // crates/sim/tests -> crates/sim -> crates
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ is the parent of this crate")
        .to_path_buf();
    let mut out = Vec::new();
    for entry in fs::read_dir(&crates).expect("readable crates dir") {
        let src = entry.expect("dir entry").path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut out);
        }
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

/// `1.5` — a bare float literal, which is how a float normally enters Rust and
/// which no type name accompanies. Excludes ranges (`0..20`) and tuple field
/// access (`self.0`), neither of which is `digit . digit`.
fn has_float_literal(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    for i in 1..c.len().saturating_sub(1) {
        if c[i] == '.'
            && c[i - 1].is_ascii_digit()
            && c[i + 1].is_ascii_digit()
            && c.get(i.wrapping_sub(2)) != Some(&'.')
            && c.get(i + 2) != Some(&'.')
        {
            return true;
        }
    }
    // `1f64` / `1.0f32`: the suffix never appears as its own token.
    tokens(line).any(|t| {
        (t.ends_with("f32") || t.ends_with("f64")) && t.starts_with(|ch: char| ch.is_ascii_digit())
    })
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
