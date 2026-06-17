# Aconfig Diff

Projects and tests in this folder are intended to support functional diff'ing of
aconfig-flagged code. The idea is to have tooling and infrastructure that can
check whether a particular flagged-change yields build artifacts that are
functionally equivalent when the flag is read-only and disabled.

See also b/425731005.

## Tests

A collection of various aconfig-flagged scenarios relative to an unflagged
baseline, with various levels of optimzied outputs. This both validate blessed
patterns of flagged developement, and guards against regressions in either
codegen output or related optimization infrastructure.


