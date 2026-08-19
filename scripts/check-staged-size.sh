#!/usr/bin/env bash
# Refuse to stage a file larger than MAX_KB.
#
#   scripts/check-staged-size.sh          # checks the current index
#
# A large file is permanent once pushed. A nested workspace's target/ directory
# put 2057 files and 528 MB into this repo, and the only reason it was ever
# removed is that the branch had not been pushed yet.
#
# Its own history is the reason this lives in a file of its own rather than
# inline in the hook. As an inline loop it aborted EVERY commit: under
# `set -euo pipefail`, `oversized=$(... | while ...; done)` takes the loop's exit
# status, the loop takes its last command's status, and that last command was
# `[ size -gt max ] && printf` — which is 1 for every file under the limit. The
# hook died silently before running a single check. Split out, it is testable,
# and scripts/check-checks.mjs now tests it.
set -euo pipefail

MAX_KB="${MAX_KB:-512}"
cd "$(git rev-parse --show-toplevel)"

# Written to a file FIRST, not piped in from a process substitution: a failure
# inside `< <(...)` is invisible to `set -e`, so a git error would leave the loop
# reading nothing and the gate would report success having inspected zero files.
# That is the same silent-success shape this script's own header is about.
#
# A file rather than `$(...)` because command substitution CANNOT carry NUL
# bytes — it silently drops them, which would concatenate every path in the -z
# output into one string. The meta-check caught that within a minute of it being
# written, which is the entire argument for the meta-check.
list=$(mktemp)
trap 'rm -f "$list"' EXIT
git diff --cached --name-only -z --diff-filter=ACMR > "$list"

oversized=""
# The STAGED blob size via `git cat-file -s :path` rather than the working tree:
# `git add big.bin && rm big.bin` still commits the blob, and a working-tree stat
# would have waved it through. -z also survives paths git would otherwise quote.
while IFS= read -r -d '' f; do
  size=$(git cat-file -s ":$f" 2>/dev/null) || continue
  kb=$((size / 1024))
  if [ "$kb" -gt "$MAX_KB" ]; then
    oversized="${oversized}  ${kb} KB  ${f}"$'\n'
  fi
done < "$list"

if [ -n "$oversized" ]; then
  echo "refusing to stage file(s) over ${MAX_KB} KB:" >&2
  printf '%s' "$oversized" >&2
  echo >&2
  echo "a large file is permanent once pushed. If it belongs, commit with" >&2
  echo "'git commit --no-verify' and say why in the message." >&2
  exit 1
fi
