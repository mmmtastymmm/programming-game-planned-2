*Closed record — see [../README.md](../README.md). Not spec.*

# T1 — CI harness

`scripts/ci.sh` as the single source of truth, the GitHub Actions workflow that
shells out to it, and the pre-commit doc gate installed by
`scripts/install-hooks.sh`.

Ported from the predecessor project and adapted: fmt and clippy gate from the
first commit, and the hook extracts the whole staged tree rather than only `*.md`
(the ported version reported a doc→source link as missing under the hook while
CI passed it).

Completed in `feaa794`.
