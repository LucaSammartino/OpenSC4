| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propdev-pv18 (prop developer RE) | Codex / `re-propdev-pv18` | ready; PV18 answered; doccheck passed | Implement independent creation/dated effect observation; retain ineligibility until measured |

## Report

Merged the explicitly requested `glue-propdev-close7` into the reserved detached checkout at `bbdfde67e`.
The owner report was read first; it contains no Owner closure table or `open:` rows. Its Limits names
successful creation registration/hold/holder/tag ordering, dated timer/state/date outputs, separate occupant
RNG, queued notification lifetime/order, and model calls/releases as the outstanding PV18 effects.

Answered PV18 in `docs/specs/prop-developer.md`:

- §§3.3.2–3.3.3 specify ordinary range field offsets/widths and pointer bases, safe pre/post snapshot
  boundaries, save-layout corroboration, the global occupant RNG snapshot/per-draw boundary, notification
  retention through final message release, ordered model enumeration/QI/SetFrame/release contracts and
  virtual-unknown adjustment. GetOrCreate may install an empty container; model failure does not roll back
  occupant state or prevent date advancement.
- §4.4 identifies the service-return `cISC4Occupant*` separately from the primary prop interface and
  specifies borrowed concrete holder access, u32 tag assignment and registration observation boundaries.
- §4.6 corrects the calendar property: two or more u8 values supply month/day; fewer supply zero/zero.
  Interval-zero props still append to the dated list and retain a hold despite a null range pointer.
- §12 adds seven synthetic prose conformance vectors, PV18-01–07: raw/absent range, unsigned wrapping dates,
  separate RNG/equality, mixed model interfaces/frame failure, empty container, queued lifetime, creation/calendar.

Moved the complete PV18 question into `docs/specs/questions/answered/prop-developer.md`, marked
**ANSWERED 2026-10-24 (re-propdev-pv18): DIFFERS**, and regenerated `docs/specs/QUESTIONS.md`.
The open prop-developer question file now states no open questions.

Source work used the main checkout's lock-free export queries, `decomp/cSC4PropOccupant.c` and
`decomp/cSC4PropDeveloper.c`, and read-only PE vtable words. Numeric fields, widths, unsigned comparisons,
date arithmetic and signatures were checked against instructions with bytes; citations are in the spec.
No REA query was needed; no missing-binary/error fallback occurred. The PE helper is ignored under this
worktree's `build-runs/`; no original code/listing or game asset is committed. No engine source was read
or manually edited. The initial merge imports the owner's already-authored engine changes as requested.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py check` passed;
`git diff --check` passed. Cargo/clippy/CTest are n/a for this specification-only authored change.
No game run requested or performed. The owner's inherited 42537/42537 eligible projection matches and
2928 incomplete creation/dated operations remain its measurements, not a new full-operation exact verdict.
No C ABI export changed; no ABI rows proposed. Git emitted a shared packed-refs-lock permission diagnostic
while merging/committing, but each operation completed successfully with its new detached HEAD commit.

## Handoff

- Goal: answer PV18 with independently observable dated/creation effect contracts and prose conformance cases.
- Done: merge `bbdfde67e`; date/model/lifetime contracts `96a028f89`; creation/vectors/archive/index `3892f9c4f`.
- Owner: 42537/42537 eligible projections; 2928 creation/dated deliveries incomplete. No full-operation exact claim.
- Next: engine owner implements independent scoped effect capture under §§3.3.2–3.3.3/4.4 and exercises PV18-01–07.
- Retain creation/dated ineligibility until all actual effects are captured/compared; no double tick or opaque-effect replay.
- Snapshot R = primary P +0x54; I/L = R +0/+4, S/E day numbers = R +0x0C/+0x1C; occupant generator G = 0x00B231B0.
- Calendar pair correction and interval-zero list hold must be carried into implementation.
- No outstanding game requests, new questions, ABI rows or source-code work.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-propdev-pv18-r1-manifest.json`.
