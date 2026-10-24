| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv11: map-view owner questions | `Codex` / `re-map-view-qv11` (RE lane) | ready, partial closure; ac706d691; doccheck passed; QV7/QV8/QV10 answered; ABI unchanged; no game runs | 1. Land docs. 2. Resolve QV4/QV6/QV9 in follow-up. |

## Report

Extended map-view §§4.2.1, 4.3 and 5.0 with behavior and seven conformance vectors (17–23).
Moved QV7, QV8 and QV10 to questions/answered/map-view.md, marked DIFFERS, and regenerated
docs/specs/QUESTIONS.md. No new questions filed. QV4, QV6 and QV9 remain open: partial closure is
intentional under the brief's two-thirds context wind-down instruction. Guard measured 139799/150000
after the first commit. No native game/shadow equivalence is claimed.

Verified native rules:

- Highlights are a signed-pointer-keyed multiset; equal non-null keys retain position/radius without
  comparison. Kinds 6–9 provide zero radius; kind 0 removes previous building records.
- Ramps retain supplied order, extend endpoints, advance through duplicates, and use integer weights
  with upward half rounding. Premultiplication divides by 256 (opaque white becomes FFFEFEFE).
- Coarse resampling clamps centre-aligned fixed-point samples and rounds each separable stage.
- Float indices wrap the low byte after qword truncation; masked invalid conversion selects zero.
- Overlay inclusion uses straight-palette nonzero alpha and native grid coordinates/world sizes.

Evidence: main checkout export queried first with re_query.py; bounded decomp reads then checked
numeric instructions with asm --bytes. Key owners/helpers: 007A1390, 007A1220, 007A1A00, 0078D520,
0079EB40, 0079EFD0, 007A4F45–007A50EC, 007A2E00, 009EEF04, 007A1020 and 007A1120.
Marked-cell geometry is linked to overlay-views §2.2 rather than duplicated approximately.
REA was not invoked: the brief says its session has no binary. No REA error/fallback occurred.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py check` passed
(`build-runs/doccheck.log`); `git diff --check` passed. Cargo/clippy/ctest are n/a for this RE-only change.
No engine changes, ABI proposals, C ABI exports, game requests or game runs.

## Handoff

- Goal: close QV4, QV6, QV7, QV8, QV9, QV10 in map-view with native behavior and vectors.
- Done: QV7, QV8, QV10 behavior and seven conformance vectors added; questions moved to answered.
- Native pointer multiset preserves equal building records without comparing position/radius; kinds 6–9 radius zero.
- Ramp/resampling use integer fixed-point rounding; premultiplication divides by 256, opaque white becomes FFFEFEFE.
- Float conversion takes signed-qword truncation's low byte; masked-invalid results select index zero.
- Overlay includes nonzero straight alpha and passes grid coordinates/sizes to marked-cell view.
- Milestone: ac706d691 commits the three answers and regenerated index; doccheck passed.
- Next: land partial docs, then inspect QV4/QV6/QV9 in a fresh lane context.
- QV4, QV6, QV9 remain open. Evidence gathered but not promoted to spec before context wind-down.
- Relevant uncommitted follow-up evidence: BlockMapByLot 0079FEF0; layer5 007A2080; constructor 007A0D50;
  Update clears dirty at active entry, stores clock at 007A4864; developer-null exit bypasses clock.
- Read only the expressly requested owner README under engine/. No engine, ABI or tool edits; no game requests.
