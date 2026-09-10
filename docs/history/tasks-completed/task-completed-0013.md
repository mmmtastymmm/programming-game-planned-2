*Closed record — see [../README.md](../README.md). Not spec.*

# T13 — `import` over a closed module set

A bundle became several files in `crates/lang`. The loader compiles each,
checks that every import names a bundle file or a game module (none yet),
refuses a bundle file named like a game module, and rejects a circular
import with the path that closes it — all at load, as determinism rule 7
requires. The version had covered every file's name and bytes in sorted
name order since T10; a test now pins that.

At run time a module runs the first time it is reached in a main-flow run,
a second import is the same module, a restart clears every module's globals
and re-runs what it imports, and a `def` captures the globals of the module
that defined it. With this the boundary `docs/01-language/syntax.md` draws
is implemented in full; what remains of the language is the determinism
suite (T14) and the two gaps recorded under T10.

Completed in `4b5b503`.
