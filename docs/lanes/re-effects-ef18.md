| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-ef18 (EF18/EF19 contracts) | `re-effects-ef18` / `re-effects-ef18` (RE, detached worktree) | ready: EF18/EF19 answered; 18 proposed vectors; doccheck and questions checks pass; no ABI or game runs | Implement §§3.1, 6.2 and 18.6 in core-effects-close5; execute the proposed vectors. |

## Report

Answered both blocking questions in `docs/specs/effects.md`, moved the original
entries to `docs/specs/questions/answered/effects.md` with dated DIFFERS verdicts,
and regenerated `docs/specs/QUESTIONS.md`. EF17 remains reserved and untouched.

- `88f1af3c5`: §3.1 specifies the 56-byte legacy placement mapping, file versus
  memory layout, low-byte revision retention, transform flags, row convention,
  x87 operation order and the different brush/game binary32 spill boundaries.
- `ea548753c`: §§6.2 and 18.6 specify retained file inputs, synchronous parse
  reporting, packed/unpacked bootstrap ordering, collection reload retention,
  description value copies and reference fixups, ID collisions and triggers.
- `b3eeefdb6`: final review specifies optional-file exception suppression,
  links runtime transform rules to §3.1 and records camera-copy behavior.

The important differences from the engine lane's provisional choices are:
queue/bootstrap are supported manager operations, not additional independent COM
objects; zero offsets do not select the transform shortcut; dot sums use z,x,y
order; legacy numeric identity still has nonzero flags; serialized revision upper
bits do not survive native load/write. Required versus optional file errors also
have different batch-success behavior, and the not-ready path oddly returns true.

There are **18 proposed, unexecuted prose conformance vectors**: EF19-1 through
EF19-6, and EF18-1 through EF18-12. These are spec-side fixtures and expected
results, not engine test files or claims of native execution. Numeric examples
include a nontrivial row/scale/translation case, cancellation distinguishing sum
orders, and signed zero distinguishing flag identity from zero offset.

Evidence came from the main checkout's read-only `re_query.py` export, scoped
decomp reads and the local original binary's scalar/table bytes. Numeric order,
widths and store boundaries were checked with `asm --bytes`; address ranges are
in the spec. No REA call was made because the brief states no binary is open.
No external implementation source was opened or copied.

Validation on the final spec changes:

- `python tools/questions.py index`: generated 16,311-byte index.
- `python tools/questions.py check`: **PASS**, `questions: ok`.
- `python tools/doccheck.py check`: **PASS**, `doccheck check: ok`.
- `git diff --check`: **PASS**.
- Cargo/Clippy/CTest and game/shadow: **n/a**, specification-only work.

No engine, plugin, capi, ABI, coverage-map or coordinator handoff edits. No ABI
rows proposed; no C ABI export change. No new question or outstanding game run.
Commits succeeded on detached HEAD; Git also printed a packed-refs.lock permission
diagnostic during each commit. No shared ref/config change was attempted.

Next: core-effects-close5 can implement the newly specified boundaries and run
the 18 proposed cases. Full shipped-resource census fixtures remain that lane's
separate limitation; this work does not claim those data-corpus checks ran.

## Handoff
- Goal complete: EF18/EF19 archived; spec and questions index committed.
- Done: source/spec milestones `88f1af3c5`, `ea548753c`, `b3eeefdb6`.
- Done: final questions check, doccheck and whitespace checks passed.
- Next: supervisor publishes detached HEAD; coordinator lands the ready RE lane.
- Queue: build-runs/merge-queue/re-effects-ef18.json, kind re, no game runs/ABI changes.
- Implementers should read effects.md §§3.1, 6.2, 18.6 and their 18 proposed vectors.
