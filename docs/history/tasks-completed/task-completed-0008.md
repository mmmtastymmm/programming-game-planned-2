*Closed record — see [../README.md](../README.md). Not spec.*

# T8 — Write `docs/01`

Written as a doorway, [01-language.md](../../01-language.md), with four parts:
syntax, execution, numbers and costs. The rulings it waited on — Q8, Q11 and
Q14, with Q17 and Q18 amending Q8 and Q19 amending Q14 — are the Decided
entries of the parts that elaborate them, and the six language rulings left
the overview's Decided section for there.

The task carried a list of things `docs/01` had to pin that no open question
owned. Each landed:

- **Every interpreter limit is spec** — the limits table in execution, one row
  per limit with what exhausting it does, and every value in
  `data/language/limits.toml`.
- **`isinstance` is the one permitted type query** — stated in syntax's type
  section.
- **The interrupt mechanism** — execution's interrupt section: the two states,
  the four kinds, hooks bound at load, delivery, preemption, coalescing,
  escalation, the swap, and the shape of `e`.
- **The number model** — numbers, around the one type `num`, with the scale
  stated once as spec.
- **What Q13 named but did not pin** — the closed dunder set, the module
  resolution order and the `match` pattern forms, all in syntax.

Two review rounds on the uncommitted doc found twenty-seven defects between
them — contradictions between parts, an arithmetic error in the range table,
a non-injective bundle hash (P1), and stale fixtures in the check on the
checks — all fixed before the first commit.

Completed in `89a1d37`.
