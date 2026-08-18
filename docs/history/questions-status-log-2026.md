*Closed record — see [README.md](README.md). Not spec.*

# Question status log — 2026

Dated board-state blocks displaced from [QUESTIONS.md](../QUESTIONS.md) as newer
ones superseded them. **Newest first.** Sharded by year rather than by number,
because blocks are dated rather than numbered.

Each block records what was open on its date and what that day's rulings
changed. Blocks are kept exactly as they were received: a closed day's content
is never corrected, only superseded by a later one. Moving a block here changes
exactly two things — it loses its `(latest)` marker, and its relative links gain
a `../` because the archive sits a directory deeper.

**Status 2026-08-17:** the framing pass is done. Q1–Q4 are answered and
live in [questions-answered-001.md](questions-answered-001.md):
this is a fresh take on the predecessor's core idea, the player programs a fleet
of identical units, **programs are rewritten mid-match as lockstep-synchronized
updates**, and PvE ships before PvP. Those rulings are summarised in
[00-overview.md](../00-overview.md)'s Decided section.

Eight questions are open below. Q5 is the next one that blocks the rest. Q11 and
Q12 exist because Q3 admits mid-match updates, and neither can be deferred past
Q5 — both change what the language and the tick loop have to be.
