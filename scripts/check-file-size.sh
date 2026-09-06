#!/usr/bin/env bash
# Refuse a file larger than MAX_KB. Two modes, one limit:
#
#   scripts/check-file-size.sh            # the STAGED blobs — the pre-commit hook
#   scripts/check-file-size.sh repo       # the index AND this branch's history — CI
#
#   SIZE_BASE_REF=<ref>  bounds the history walk to commits not already in <ref>
#
# A large file is permanent once pushed. A nested workspace's target/ directory
# put 2057 files and 528 MB into this repo, and the only reason it was ever
# removed is that the branch had not been pushed yet.
#
# WHY TWO MODES. The staged mode is the one that can still save you — it fires
# before the blob is written to a commit. But it runs only in the pre-commit
# hook, which is opt-in per clone (scripts/install-hooks.sh) and which
# `--no-verify` skips. (It runs before the hook's Node test, so a missing Node
# disables everything after it and not this.) For a while
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
# Guarded: with git absent or outside a repository, the substitution is empty and
# `cd ""` is a silent no-op in bash, so the gate went on to run `git ls-files` in
# whatever directory it inherited and died with git's own error and exit 128. A
# check that names a specific culprit had better name the right one.
if ! root=$(git rev-parse --show-toplevel 2>/dev/null) || [ -z "$root" ]; then
  echo "check-file-size: not a git repository (or git is not installed) — this gate" >&2
  echo "reads the index and the history, so it has nothing to check here." >&2
  exit 2
fi
cd "$root"

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
raw=$(mktemp)
hist=$(mktemp)
trap 'rm -f "$list" "$raw" "$hist"' EXIT

TAB=$'\t'

# One record per blob to inspect: "<key><TAB><label>", NUL-terminated. The key is
# what `git cat-file -s` is given AND what de-duplicates the run, so the index
# and the history cannot report the same blob twice or count it twice.
#
# A path containing a literal tab would split wrongly here. -z keeps newlines and
# quoting safe; a tab in a path breaks enough git tooling that it is out of scope.
case "$MODE" in
  staged)
    git diff --cached --name-only -z --diff-filter=ACMR > "$raw"
    while IFS= read -r -d '' f; do printf ':%s%s%s\0' "$f" "$TAB" "$f"; done < "$raw" > "$list"
    ;;
  repo)
    # -s for the blob SHA. Keyed by path, a 700 KB blob committed at a path and
    # an 800 KB one staged at the same path collapsed into one report, hiding
    # what a clone would still carry.
    git ls-files -s -z > "$raw"
    while IFS= read -r -d '' rec; do
      meta="${rec%%"$TAB"*}"
      path="${rec#*"$TAB"}"
      # mode SHA stage — unquoted on purpose, to split the three fields.
      # shellcheck disable=SC2086
      set -- $meta
      printf '%s%s%s\0' "$2" "$TAB" "$path"
    done < "$raw" > "$list"
    ;;
  *) echo "usage: scripts/check-file-size.sh [staged|repo]" >&2; exit 2 ;;
esac

oversized=""
unreadable=""
blobs="|"
seen=0

record() {
  local key="$1" size="$2" label="$3"
  case "$blobs" in *"|$key|"*) return 0 ;; esac
  blobs="${blobs}${key}|"
  seen=$((seen + 1))
  if [ "$size" -gt "$MAX_BYTES" ]; then
    oversized="${oversized}  $(((size + 1023) / 1024)) KB  ${label}"$'\n'
  fi
}

# The blob size from git, not from the working tree: `git add big.bin && rm
# big.bin` still commits the blob, and a working-tree stat would have waved it
# through.
while IFS= read -r -d '' rec; do
  key="${rec%%"$TAB"*}"
  label="${rec#*"$TAB"}"
  # A key git cannot size — an unmerged path mid-merge, a corrupt object — is
  # REPORTED, not skipped. It used to `continue` in silence and not count toward
  # `seen`, so the success line named a total that quietly excluded it.
  if ! size=$(git cat-file -s "$key" 2>/dev/null); then
    unreadable="${unreadable}  ${label}"$'\n'
    continue
  fi
  record "$key" "$size" "$label"
done < "$list"

# Every blob this branch's history carries. `rev-list --objects` emits each
# object once, and `record` drops any blob the index already accounted for.
#
# SIZE_BASE_REF bounds the walk to the commits under review — CI passes the PR's
# base — because an unbounded walk judges the whole base branch: one oversized
# blob anywhere in main's past would fail every PR forever, with a message
# telling authors to rewrite a branch that did not introduce it. Unset (a local
# run, or a push to the default branch) means the whole of HEAD, which is the
# right scope for the history you own.
#
# Line-based, unlike the loop above: `--objects` has no NUL form. A path
# containing a newline would be misread — the alternative is not scanning history
# at all.
if [ "$MODE" = "repo" ] && git rev-parse --verify -q HEAD >/dev/null; then
  range=(HEAD)
  base="${SIZE_BASE_REF:-}"
  if [ -n "$base" ] && git rev-parse --verify -q "${base}^{commit}" >/dev/null; then
    range=(HEAD --not "$base")
  fi
  git rev-list --objects "${range[@]}" \
    | git cat-file --batch-check='%(objectname) %(objecttype) %(objectsize) %(rest)' > "$hist"
  while read -r sha type size rest; do
    [ "$type" = "blob" ] || continue
    record "$sha" "$size" "${rest:-$sha}  (in this branch's history)"
  done < "$hist"
fi

# Reported BEFORE the zero-input guard below: a repository whose objects git
# cannot read has "no files" by that guard's arithmetic, and diagnosing a corrupt
# or partial object store as an empty repository is the confident-wrong-answer
# failure this script's header is about.
if [ -n "$unreadable" ]; then
  echo "file(s) git could not size — the gate cannot vouch for these:" >&2
  printf '%s' "$unreadable" >&2
  echo >&2
fi

# Zero blobs inspected is a pass in staged mode — a commit that only deletes
# stages nothing under ACMR — and is meaningless in repo mode, where it means the
# gate ran against a repository with nothing in it and said everything was fine.
# Scoring green on zero inputs is this repo's recurring failure.
if [ "$MODE" = "repo" ] && [ "$seen" -eq 0 ] && [ -z "$unreadable" ]; then
  echo "check-file-size: no files in the index or the history — the gate is checking nothing" >&2
  exit 2
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
[ "$MODE" = "repo" ] && echo "✓ ${seen} distinct blob(s) in the index and history, none over the ${MAX_KB} KB cap"
exit 0
