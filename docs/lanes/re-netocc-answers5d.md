| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-answers5d: network and underground owner answers | Codex / `re-netocc-answers5d` (RE, detached HEAD) | in progress: NO36 and prior NO41 answered; 4 new pylon vectors; doccheck passed; ABI unchanged | Assess remaining NO33 NO40 NO43 UG5 within research allowance; enqueue verified answers READY |

## Report

Answered NO41 in network §3.6.3/test 51 and moved its original question to the answered archive.
The base +268 word is opaque and must round-trip; the descriptor store is adjusted primary+26C.
Connection +68/+170/+178/+240 are active wall count, upper model anchor height, shade index and
creation class ID. Regenerated docs/specs/QUESTIONS.md. Commit: `2869269f9`.

Network §6.2.3/tests 52–53 specify pylon Draw and all message decisions, including pass-zero-only
dirty rebuilding, dispatch-state label suppression, unsigned tick conversion and visibility transitions.
Commit: `cb51c38e8`. Network §4.7.2/tests 54–55 specify pole occupant bounds, unsigned destination
comparisons and the signed-conversion/wrapping quirk (`9162e3b21`). These are partial NO33/NO36/NO43 answers.

Validation: `python tools/doccheck.py check` and `git diff --check` passed on the final spec edits.
No engine, plugin, capi, tools/coverage-map.tsv or ABI edits; no ABI proposals; no game runs or
implementation tests. Evidence is the main checkout export and bounded decomp excerpts; numeric
decisions above were checked in instruction bytes. REA was not used because the session has no binary.
The owner closure table is inside forbidden engine paths and was not read; core-netocc-close5's
lane report and its question entries supply the requirements instead.

NO33, NO36, NO40, NO43 and UG5 remain open. Underground spec unchanged: its remaining requirements
depend on the still-unfinished geometry and connection owner research. The two-thirds research
allowance has been spent; this round is partial and must not be advertised as ready or owner-complete.
Git commits succeeded despite an incidental packed-refs.lock permission message; detached HEAD retained.

## Handoff
- Goal: answer NO33, NO36, NO40, NO41, NO43 and UG5 without engine reads or edits.
- Round 2 milestone: NO36 completed in §6.2.4/tests 56–59, archived and index regenerated; doccheck and diff check pass.
- Round 2 evidence: pylon 005F12D0/005F1D00/005F20A0 instruction bytes; PE constants via ignored build-runs/evidence_constants.py.
- Correction: pylon +150 is the call-number texture ID; MySim uploads main +148.
- Done: NO41 answered (2869269f9); pylon Draw/messages (cb51c38e8); pole bounds (9162e3b21), tests 54–55.
- Round 2 resumed 2026-10-24 at detached c686f8eae; next: finish NO36 model/MySim/number-label composition, then NO40/UG5 geometry; finish READY for verified answers even if other questions remain open.
- Still open: NO33/NO43 pole model/resource/transform/Pick/wire/Draw and diagonal-floor numeric order; NO40/UG5 underground geometry.
- Pylon source: main decomp/cSC4PylonOccupant.c lines 762–978 (MySim/replacement), 1217–1363 (label), 1385–1575 (model).
- Underground source: decomp/cSC4UndergroundOccupant.c 1076–1837; connection source 1132–2168.
- Pole source: decomp/cSC4PowerPoleOccupant.c 495–832 (Pick/model), 1003–1166 (wire draw), 1632–1784 (Draw).
- Caution: pole GetBoundingBox receives primary+8; pylon Draw virtual model view is primary+1C8.
- Decompiler stack variables in label/MySim/model-load paths are unreliable; verify from asm before publishing.
- Constraint: owner closure table is inside engine and forbidden by task; use core lane report and question text as closure requirements.
- Sources: main checkout export/decomp only; no REA binary open. No ABI or game runs expected.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-netocc-answers5d-r2-manifest.json.
