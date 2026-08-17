//! The mechanical half of CLAUDE.md's determinism rules 2, 3 and 4: scan this
//! crate's own source for the banned constructs.
//!
//! A test that greps its own source is unusual, but these three rules are
//! *syntactic* — a violation is visible without running anything — and they
//! are exactly the rules that get broken by an innocent-looking convenience
//! commit six months from now. Catching them at review time requires someone
//! to remember; catching them here does not.
//!
//! LIMITS, so a green run is not read as more than it is:
//!   * Comments are stripped, string literals are not — a banned token inside
//!     a string is a false positive. None exist today; if one appears, extend
//!     the stripper rather than deleting the check.
//!   * This finds the *names*. It cannot find a float that arrives through a
//!     dependency's API, or a `BTreeMap` iterated in an order that depends on
//!     something nondeterministic upstream. Rules first, gate second.

use std::fs;
use std::path::{Path, PathBuf};

/// Token → why it is banned. Keep the reasons here: the failure message is the
/// only place most people will ever read them.
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
];

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Blank out `//` line comments and `/* */` block comments, preserving line
/// structure so reported line numbers stay right.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut block = 0usize;
    while i < bytes.len() {
        let two = (bytes[i], bytes.get(i + 1).copied().unwrap_or('\0'));
        if block > 0 {
            if two == ('*', '/') {
                block -= 1;
                i += 2;
                continue;
            }
            if two == ('/', '*') {
                block += 1;
                i += 2;
                continue;
            }
            if bytes[i] == '\n' {
                out.push('\n');
            }
            i += 1;
        } else if two == ('/', '*') {
            block += 1;
            i += 2;
        } else if two == ('/', '/') {
            while i < bytes.len() && bytes[i] != '\n' {
                i += 1;
            }
        } else {
            out.push(bytes[i]);
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

#[test]
fn sim_source_is_free_of_nondeterministic_constructs() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files(&src);
    assert!(
        !files.is_empty(),
        "found no source to scan — the test is checking nothing"
    );

    let mut violations = Vec::new();
    for file in &files {
        let text = strip_comments(&fs::read_to_string(file).expect("readable source"));
        for (n, line) in text.lines().enumerate() {
            for token in tokens(line) {
                if let Some((_, why)) = BANNED.iter().find(|(t, _)| *t == token) {
                    violations.push(format!(
                        "{}:{}  `{token}` — {why}\n      {}",
                        file.display(),
                        n + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} determinism violation(s) in sim source:\n\n{}\n\nSee CLAUDE.md. \
         If one of these is genuinely safe, it needs a documented exemption \
         here — not a deleted check.",
        violations.len(),
        violations.join("\n")
    );
}
