#!/usr/bin/env bash
# The determinism battery (docs/06, Testing; T15): run every golden gate
# against its checked-in fixture, then emit the final hashes of each stream
# so two architectures can be compared line for line.
#
#   scripts/determinism-battery.sh              # hashes to stdout
#   scripts/determinism-battery.sh OUT.txt      # hashes to OUT.txt
#
# CI runs this on an x86-64 and an arm64 runner and diffs the two files: a
# difference is a desync that would have hit two peers on different
# machines, caught before a match ever runs. The golden tests below already
# compare every stream against the checked-in fixture, so each side also
# agrees with the machine that generated it.
#
# Only `KEY=hash` lines go to the output; the host triple is printed to
# stderr so the file compares clean across machines.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-/dev/stdout}"

rustc -vV | sed -n 's/^host: /host: /p' >&2

# 1. Every golden gate against its fixture (fails loudly on drift).
cargo test -q -p replay --test golden >&2
cargo test -q -p lang --test golden >&2
cargo test -q -p lang --test determinism >&2

# 2. The final hashes, one line each. The emitters are `#[ignore]`d tests
#    that print a `KEY=…` line; anything else they print is dropped.
emit() {
  cargo test -q -p "$1" --test "$2" -- --ignored --exact "$3" --nocapture 2>/dev/null \
    | grep -E "^[A-Z_]+=" || { echo "no hash line from $1/$2::$3" >&2; exit 1; }
}
{
  emit replay golden emit_golden_final_hash
  emit lang golden emit_fixture_hashes
  emit lang determinism emit_trace_hash
} > "$OUT"

if [ "$OUT" != "/dev/stdout" ]; then
  echo "wrote $(wc -l < "$OUT" | tr -d ' ') hash lines to $OUT" >&2
fi
