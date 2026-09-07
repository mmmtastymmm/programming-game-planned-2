*Closed record — see [../README.md](../README.md). Not spec.*

# I1 — Names a Python programmer will reach for and miss

Writing `docs/01` against Q13's tables left out `super()`, `tuple()`,
`reversed`, `divmod`, `chr`/`ord`, `repr`, `map`/`filter`, `print` and the
`del` statement. `print` is covered — diagnostic output is a game builtin for
`docs/02`. The rest are absent because the ruling's list did not include them,
and the doc says so (its divergence 15) rather than widening silently. If
playtesting shows `super()` or `reversed` missed often, a widening is a new
question number.

## Outcome

- **Docs:** [01-language/syntax.md](../../01-language/syntax.md) — the
  divergence list names each absence, and `tuple` being a present name that
  cannot be called has its own item. Nothing widened; a widening stays a new
  question number, to be opened if playtesting asks for it.
