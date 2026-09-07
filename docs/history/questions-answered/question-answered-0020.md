*Closed record — see [../README.md](../README.md). Not spec.*

# Q20 — Amend Q18: the fault record is written on every failure and survives a redeploy

## Ruling

*Ruling (2026-09-07):* one amendment to rule 5 of
[Q18](question-answered-0018.md), made under a new number because Q18's file
is a closed record. Everything else in Q18 stands.

**The fault record is written by every failure, not only by the `fault`
prologue, and nothing but the next failure overwrites it.** Q18 had the
`fault` prologue write it and a `redeploy` clear it. That left three failures
unrecorded — an exception escaping `on_fault` or `on_dying`, a hook's budget
running out, and an exception whose unwinding an interrupt cut off — and made
the last of them *silent* whenever the interrupt was a `redeploy`, which
cleared the record its own delivery had just caused to be written.

### The rules

1. **Every failure writes the record**: an exception escaping main flow (the
   `fault` prologue); an exception escaping a hook, or a hook's budget running
   out (the escalation itself, before the next kind is delivered); and an
   exception whose unwinding an interrupt abandons (the abandonment, before
   that interrupt's prologue).
2. **The record holds**: the exception's class and arguments, or the kind of
   hook whose budget ran out; the `file` and `line` of the raising operation,
   or of the last operation the hook completed; the tick of the raise; and
   the version of the bundle it belongs to.
3. **The record survives everything.** A restart, a `redeploy`, a `dying`
   handler — none clears it. The next write replaces it. It ends only with
   the unit.
4. **It is world state**, so it is in the state hash, any renderer can show
   it, and a replay reproduces it — as Q18 said.

### Consequences

- **Nothing is silent**, including the case that motivated the abandonment
  rule: a `redeploy` landing mid-unwind leaves the exception on record, tagged
  with the bundle it happened in, so a player who redeployed past a bug can
  still see the bug.
- **A fault-restart loop leaves one record**, the latest, which is what a
  renderer would show anyway.

## Outcome

- **Docs:** [01-language/execution.md](../../01-language/execution.md) — the
  Decided section, the lifecycle's fault-record paragraph, and the escalation
  bullets.

## Worksheet

The argument, not the conclusion — options weighed, paths not taken.
Cite the ruling above; open this half only to recover *why*.

| Option | What it costs |
|---|---|
| Keep Q18: written by the `fault` prologue, cleared by a `redeploy` | Three failures unrecorded, one of them silent. The one the abandonment rule was written to keep visible is the one a `redeploy` erases. |
| Written on every failure; still cleared by a `redeploy` | Closes the escalation gap and reopens the silence: the abandonment write and the `redeploy` clear happen in the same delivery, in an order nothing had fixed. |
| **Written on every failure; survives everything** *(chosen)* | One more field, the bundle version, so a record is never mistaken for the current program's. |

### How the answer took its shape

A review of the merged doc asked, for each failure path, who wrote the record
and who cleared it, and found the abandonment rule writing into a record the
next step erased. Making the record survive was the only reading in which the
abandonment rule does what it says; tagging it with the bundle version is what
makes surviving harmless.
