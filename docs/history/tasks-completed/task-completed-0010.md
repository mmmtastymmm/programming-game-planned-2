*Closed record — see [../README.md](../README.md). Not spec.*

# T10 — Lexer with significant indentation, then the procedural core

The first code of the language: the lexer, the parser that draws Q13's
boundary, a compiler to a resumable bytecode, the metered evaluator with
the tick budget and the deficit carry, `num` as an i128 scaled by 10¹² with
a 256-bit intermediate, the language builtins and methods, and the cost and
limit tables read from `data/language/` with the row↔key check in both
directions. It landed in PR #34 and stayed open while T11 through T14 built
on it, each closing a gap it had recorded: `key=` re-entering a `def` (T11),
the interrupt kinds and hook budgets (T14), and finally the source
nesting-depth parse error and the live-values limit, an exact reachability
count walked only when a running bound on growth says the limit could have
been crossed.

With T10 closed, milestone M2 — the language implementation — is finished:
every task under it has left the file.

Completed in `f5152f2`.
