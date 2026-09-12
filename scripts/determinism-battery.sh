#!/usr/bin/env bash
# The determinism battery (docs/06, Testing; T15): run every golden gate
# against its checked-in fixture, then emit the final hashes of each stream
# so two architectures can be compared line for line.
#
#   scripts/determinism-battery.sh              # hashes to stdout
#   scripts/determinism-battery.sh OUT.txt      # hashes to OUT.txt
#   scripts/determinism-battery.sh compare A B  # two hash files agree, line for line
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

# The cross-architecture job's compare step lives here rather than inline in
# ci.yml so the check-on-the-checks can seed a disagreement against it: a
# check nobody has seen fail is a check nobody has seen work.
if [ "${1:-}" = "compare" ]; then
  A="${2:?compare A B}"; B="${3:?compare A B}"
  for f in "$A" "$B"; do
    # The same shape the emit filter below accepts — a stricter count here
    # rejected the lang fixtures' line, whose value is several hashes.
    n=$(grep -cE '^[A-Z_]+=' "$f" || true)
    if [ "$n" -lt 3 ]; then
      echo "$f holds fewer than 3 streams ($n): the battery did not emit every gate" >&2
      exit 1
    fi
  done
  if ! diff "$A" "$B" >/dev/null; then
    keys=$( (diff "$A" "$B" || true) | sed -nE 's/^[<>] ([A-Z_]+)=.*/\1/p' | sort -u | tr '\n' ' ')
    echo "the two architectures disagree on ${keys% }:" >&2
    diff "$A" "$B" >&2 || true
    exit 1
  fi
  echo "the two architectures agree on $(wc -l < "$A" | tr -d ' ') streams:"
  cat "$A"
  exit 0
fi

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
