#!/usr/bin/env bash
# The whole CI suite, runnable locally. .github/workflows/ci.yml calls THIS
# script rather than duplicating the commands, so local and CI cannot drift.
#
#   scripts/ci.sh              # everything
#   scripts/ci.sh rust         # cargo checks only
#   scripts/ci.sh docs         # doc checks only (fast — no Rust build)
set -euo pipefail

cd "$(dirname "$0")/.."
WHICH="${1:-all}"
FAILED=()

step() { printf '\n\033[1m=== %s\033[0m\n' "$1"; }

run_rust() {
  # fmt and clippy are gated from day one deliberately. The predecessor project
  # could not turn the fmt gate on without a whole-tree reformat commit first,
  # because the drift accumulated before anyone checked. It is free now.
  step "cargo fmt --check"
  cargo fmt --all --check || FAILED+=("fmt")

  step "cargo clippy (deny warnings)"
  cargo clippy --workspace --all-targets -- -D warnings || FAILED+=("clippy")

  # spikes/ are detached workspaces so that `cargo test` at the root never
  # builds a scripting runtime. That also put their Rust outside every gate
  # above. Formatting is free to check (fmt does not build), so it is checked;
  # clippy is not, because linting a spike would pull in the dependency the
  # detachment exists to avoid. A spike is throwaway code — but unformatted
  # throwaway code still gets read.
  for spike in spikes/*/Cargo.toml; do
    [ -f "$spike" ] || continue
    step "cargo fmt --check ($spike)"
    cargo fmt --manifest-path "$spike" --check || FAILED+=("fmt:$spike")
  done

  step "cargo test (incl. golden replays and the determinism scan)"
  cargo test --workspace || FAILED+=("rust")
}

run_docs() {
  # $1 is a directory to check *instead of* the working tree: the pre-commit
  # hook extracts the staged index and passes that prefix, so the hook and a
  # bare local run check the same paths against different content. Defaults to
  # the repo root. Every path below is built from $ROOT — hardcoding one is what
  # made the hook's extraction dead machinery in the predecessor project.
  local ROOT="${1:-.}"

  step "relative links and line citations resolve"
  if ! command -v node >/dev/null 2>&1; then
    echo "node not found — install Node 20+ to run the doc checks" >&2
    FAILED+=("docs (node missing)")
    return
  fi
  # No deps, so it runs before the npm install below.
  # $ROOT, not $ROOT/docs: CLAUDE.md and .claude/design-invariants.md carry
  # cross-references too, and the hook extracts every tracked .md.
  node scripts/check-links.mjs "$ROOT" || FAILED+=("links")

  step "registers are consistent and bounded"
  # Counts are derived from the entries themselves — open ones in the live
  # register, closed ones in docs/history/ — never read from prose. Also caps
  # every doc's size, which is what makes splitting mechanical rather than
  # something nobody does until a file is already 166 KB.
  node scripts/check-registers.mjs "$ROOT" || FAILED+=("registers")

  step "split-doc part files open with their breadcrumb"
  node scripts/check-doc-layout.mjs "$ROOT/docs" || FAILED+=("doc-layout")

  step "headings unique per file, tables well formed"
  # A file that says the same thing twice is a splice, and the two copies
  # disagree. The Q13 ruling carried both its narrow and its broad version for
  # two commits before a human noticed.
  node scripts/check-structure.mjs "$ROOT" || FAILED+=("structure")

  step "docs describing the register scheme agree with the code enforcing it"
  # Five doc/tooling drifts were found in a single review round; each was a fact
  # about the tooling restated by hand elsewhere.
  node scripts/check-vocabulary.mjs "$ROOT" || FAILED+=("vocabulary")

  step "mermaid diagrams parse"
  # Install on first run, or whenever the lockfile is newer than the tree.
  if [ ! -d scripts/node_modules ] \
     || [ scripts/package-lock.json -nt scripts/node_modules ]; then
    echo "installing doc-check deps…"
    if [ -f scripts/package-lock.json ]; then
      npm ci --prefix scripts --silent --no-fund --no-audit
    else
      npm install --prefix scripts --silent --no-fund --no-audit
    fi
  fi
  node scripts/check-mermaid.mjs "$ROOT" || FAILED+=("mermaid")

  # The meta-check copies the working tree and shells out to git, so it needs
  # the repository — not the extracted index the pre-commit hook passes as
  # $ROOT. Running it there would check the working tree while the hook is
  # deliberately checking the staged content, and unstaged breakage would block
  # a clean commit, which the hook's header explicitly promises it will not.
  if [ "$ROOT" = "." ]; then
    step "the checks themselves catch what they claim to"
    node scripts/check-checks.mjs . || FAILED+=("check-checks")
  else
    step "the checks themselves (skipped: needs the repo, not an extracted index)"
  fi
}

# Optional $2: check this directory instead of the working tree (see run_docs).
case "$WHICH" in
  all)  run_docs "${2:-.}"; run_rust ;;   # docs first: seconds, vs. minutes for Rust
  rust) run_rust ;;
  docs) run_docs "${2:-.}" ;;
  *)    echo "usage: scripts/ci.sh [all|rust|docs] [root]" >&2; exit 2 ;;
esac

if [ ${#FAILED[@]} -gt 0 ]; then
  printf '\n\033[31mFAILED: %s\033[0m\n' "${FAILED[*]}"
  exit 1
fi

printf '\n\033[32mAll checks passed.\033[0m\n'
