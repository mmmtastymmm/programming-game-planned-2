#!/usr/bin/env bash
# Refuse a file larger than MAX_KB. Two modes, one limit:
#
#   scripts/check-file-size.sh            # the STAGED blobs — the pre-commit hook
#   scripts/check-file-size.sh repo       # the index AND this branch's history — CI
#
# A large file is permanent once pushed. A nested workspace's target/ directory
# put 2057 files and 528 MB into this repo, and the only reason it was ever
# removed is that the branch had not been pushed yet.
#
# WHY TWO MODES. The staged mode is the one that can still save you — it fires
# before the blob is written to a commit. But it runs only in the pre-commit
# hook, which is opt-in per clone (scripts/install-hooks.sh), which
# `--no-verify` skips, and which stands down when Node is absent. For a while
# this was the ONLY size gate: `scripts/ci.sh` never called it and neither CI
# job ran it, so a contributor who had not installed the hook could push 600 MB
# and watch both jobs go green — for the one failure class this corpus describes
# as permanent. The repo mode closes that. It reports rather than prevents,
# which is the most a checkout can do.
#
# WHY THE REPO MODE READS HISTORY, not just the index. An earlier version listed
# `git ls-files` and stopped there — so a branch that added a 700 MB blob in one
# commit and deleted it in the next passed CI while the blob sat in the history
# the merge would carry, which is precisely the incident above. The gate's own
# failure text says "the blob is already in the history"; it now looks there.
#
# Reachable from HEAD, not `--all`: with `fetch-depth: 0` the CI checkout has
# every branch, and `--all` would fail this PR for a blob somebody else pushed
# to an unrelated branch.
#
# Its own history is the reason this lives in a file of its own rather than
# inline in the hook. As an inline loop it aborted EVERY commit: under
# `set -euo pipefail`, `oversized=$(... | while ...; done)` takes the loop's exit
# status, the loop takes its last command's status, and that last command was
# `[ size -gt max ] && printf` — which is 1 for every file under the limit. The
# hook died silently before running a single check. Split out, it is testable,
# and scripts/check-checks.mjs now tests it — in both modes, including the
# boundary and the deleted-but-committed blob.
set -euo pipefail

MODE="${1:-staged}"
MAX_KB="${MAX_KB:-512}"
# Compared in BYTES. `kb=$((size / 1024))` truncates, so a gate advertised as
# 512 KB accepted anything up to 525,311 bytes — and the only sizes ever tested
# were 0 and 700 KB, which no rounding error can tell apart.
MAX_BYTES=$((MAX_KB * 1024))
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
hist=$(mktemp)
trap 'rm -f "$list" "$hist"' EXIT

case "$MODE" in
  staged) git diff --cached --name-only -z --diff-filter=ACMR > "$list" ;;
  repo)   git ls-files -z > "$list" ;;
  *) echo "usage: scripts/check-file-size.sh [staged|repo]" >&2; exit 2 ;;
esac

oversized=""
oversized_paths=$'\n'
unreadable=""
seen=0

# The blob size via `git cat-file -s :path` rather than the working tree:
# `git add big.bin && rm big.bin` still commits the blob, and a working-tree stat
# would have waved it through. -z also survives paths git would otherwise quote.
while IFS= read -r -d '' f; do
  # A path git cannot size — an unmerged path mid-merge, a corrupt object — is
  # REPORTED, not skipped. It used to `continue` in silence and not count toward
  # `seen`, so the success line named a total that quietly excluded it.
  if ! size=$(git cat-file -s ":$f" 2>/dev/null); then
    unreadable="${unreadable}  ${f}"$'\n'
    continue
  fi
  seen=$((seen + 1))
  if [ "$size" -gt "$MAX_BYTES" ]; then
    oversized="${oversized}  $(((size + 1023) / 1024)) KB  ${f}"$'\n'
    oversized_paths="${oversized_paths}${f}"$'\n'
  fi
done < "$list"

# Every blob this branch's history carries. `rev-list --objects` emits each
# object once, so there is nothing to de-duplicate here; a path already reported
# from the index is skipped so one file is not named twice.
#
# Line-based, unlike the loop above: `--objects` has no NUL form. A path
# containing a newline would be misread — such a path breaks a great deal of git
# tooling, and the alternative is not scanning history at all.
if [ "$MODE" = "repo" ] && git rev-parse --verify -q HEAD >/dev/null; then
  git rev-list --objects HEAD \
    | git cat-file --batch-check='%(objectname) %(objecttype) %(objectsize) %(rest)' > "$hist"
  while read -r sha type size rest; do
    [ "$type" = "blob" ] || continue
    seen=$((seen + 1))
    [ "$size" -gt "$MAX_BYTES" ] || continue
    case "$oversized_paths" in *$'\n'"$rest"$'\n'*) continue ;; esac
    oversized="${oversized}  $(((size + 1023) / 1024)) KB  ${rest:-$sha}  (in this branch's history)"$'\n'
  done < "$hist"
fi

# Zero files inspected is a pass in staged mode — a commit that only deletes
# stages nothing under ACMR — and is meaningless in repo mode, where it means
# the gate ran against a repository with no files in it and said everything was
# fine. Scoring green on zero inputs is this repo's recurring failure.
if [ "$MODE" = "repo" ] && [ "$seen" -eq 0 ]; then
  echo "check-file-size: no files in the index or the history — the gate is checking nothing" >&2
  exit 2
fi

if [ -n "$unreadable" ]; then
  echo "file(s) git could not size — the gate cannot vouch for these:" >&2
  printf '%s' "$unreadable" >&2
fi

if [ -n "$oversized" ]; then
  echo "file(s) over the ${MAX_KB} KB cap:" >&2
  printf '%s' "$oversized" >&2
  echo >&2
  if [ "$MODE" = "staged" ]; then
    echo "a large file is permanent once pushed. If it belongs, commit with" >&2
    echo "'git commit --no-verify' and say why in the message." >&2
  else
    echo "a large file is permanent once pushed, so a follow-up commit does not" >&2
    echo "fix this — deleting the file leaves the blob in the history. Rewrite" >&2
    echo "the branch, or raise MAX_KB here and say why." >&2
  fi
  exit 1
fi

[ -n "$unreadable" ] && exit 1
[ "$MODE" = "repo" ] && echo "✓ ${seen} blob(s) in the index and history, none over the ${MAX_KB} KB cap"
exit 0
