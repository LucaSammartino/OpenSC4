| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin-vw20 (VW19 trigger loading) | `re-view3dwin-vw20` / RE lane, detached worktree | ready; spec commit `2b867c233`; VW19 answered MATCHES; VW-T40–VW-T44; doccheck passed; no ABI change or game runs | - |

## Report

Extended view3d-window §6k/§6l with literal trigger reconstruction: assignments in stream order into the
existing map, no clearing or pre-sizing, normal growth and head insertion for new keys, last duplicate
value wins without growth or movement. Counts remain unsigned 32-bit words, including present zero.
Loading sends no trigger messages. Saving traverses the resulting distinct-key table, so collision order
can reverse after a load. Added five conformance vectors, including capacity-boundary duplicate handling
and an intermediate rehash whose result differs from pre-sizing.

This closes exactly the VW19 gaps in the three requested owner-table rows: §6k save/triggers, §8 question
state, and view3d §5.3 reference-adapter trigger order. Other implementation integration items in the
owner table were outside this task. No engine, ABI, plugin, capi or coverage-map edits were made.

Evidence: export queries first, then DoLoad decomp and instruction-byte verification at `0x007EBFA0`,
map lookup `0x004F8D30`, new-key insertion `0x00750C30`, and rehash `0x004F0940`. No REA call was needed.
The decompiler misrepresents the pair-read locals; instruction addresses in the spec establish the
separate key/count reads and direct 32-bit assignment. Results are static RE findings, not game measurements.

VW19 moved to docs/specs/questions/answered/view3d-window.md as **ANSWERED 2026-10-24
(re-view3dwin-vw20): MATCHES**. No new questions. The generated questions index has no open VW19.
`python tools/doccheck.py check` passed. Cargo, clippy, ctest and game runs are not applicable to this
spec-only change; no ABI rows proposed. Gate logs: build-runs/questions-index-vw20.log and
build-runs/doccheck-vw20.log. The first Bash-wrapper attempts failed at process startup (Win32 error 5),
before running either check; build-runs/check-vw20.py ran both checks in the foreground successfully.

## Handoff

- Goal: close VW19 for the three owner rows covering trigger save/load, question state and reference adapter.
- Done: spec/question/index milestone committed as `2b867c233`; doccheck passed.
- Next: coordinator may land the queued tip; no further RE work remains for VW19.
- Facts: export/decomp only; detached HEAD is intentional. No game request is outstanding.
