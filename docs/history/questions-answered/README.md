*Closed record — see [../README.md](../README.md). Not spec.*

# Answered questions

**One file per answered question**, named for its number:
`question-answered-0013.md` holds Q13 and nothing else. The filename is the
index — there is no table to maintain and none to go stale.

Each file carries three parts in order: the **ruling**, its **outcome**, and the
**worksheet**. The ruling is what gets cited. The worksheet is the argument
behind it, read rarely and only to recover *why* a call was made.

The **Outcome** section is the one that keeps this directory from becoming a
graveyard. A ruling must land somewhere — it declares at least one of:

- `- **Docs:**` — edits made in the same commit. It must link them.
- `- **Question:**` — a `Q<n>` opened, because the ruling raised something new.
- `- **Problem:**` — a `P<n>` opened, because existing text or code is now wrong.
- `- **Task:**` — a `T<n>` opened, because the consequence is not a doc edit.

There is deliberately no `- **Dropped:**` here. An inbox entry may turn out to be
a misreading; a *ruling* that changed nothing anywhere was not a ruling, and the
checker rejects the bullet in this directory for that reason. See
[../inbox-triaged/](../inbox-triaged/README.md), which is the one register that
has it.

Every cited number must be a real register entry, and every bullet must carry
something to resolve — a bare `- **Docs:**` with no links asserts a propagation
that may never have happened, which is the state this section exists to catch.

Two is normal: a ruling that lands in a Decided section often also
creates work. Zero is not allowed, and that is the point. A ruling recorded here
but never propagated to the doc that owns it leaves the stalest text in the
corpus sitting in the most authoritative-looking place, where reading passes skim
it because it looks settled — the predecessor's characteristic failure.

Answering a question moves it out of [../../QUESTIONS.md](../../QUESTIONS.md)
entirely. `scripts/check-registers.mjs` rejects a number that is open and
answered at once, a file whose heading disagrees with its name, and gaps in the
numbering — with one honest limit: the range it checks is derived from the entry
count, so deleting the *highest*-numbered entry outright shrinks the range and
goes unnoticed. Gaps below the top are caught.

An amended ruling gets a **new number and a new file**, never an edit to an old
one — citations to the pre-amendment meaning outlive the amendment, and the only
way to catch that is for both texts to exist.
