| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv11: map-view owner questions | `Codex` / `re-map-view-qv11` (RE lane) | in progress; QV7, QV8, QV10 specified | Run doccheck; commit partial closure |

## Handoff

- Goal: close QV4, QV6, QV7, QV8, QV9, QV10 in map-view with native behavior and vectors.
- Done: QV7, QV8, QV10 behavior and seven conformance vectors added; questions moved to answered.
- Native pointer multiset preserves equal building records without comparing position/radius; kinds 6–9 radius zero.
- Ramp/resampling use integer fixed-point rounding; premultiplication divides by 256, opaque white becomes FFFEFEFE.
- Float conversion takes signed-qword truncation's low byte; masked-invalid results select index zero.
- Overlay includes nonzero straight alpha and passes grid coordinates/sizes to marked-cell view.
- Next: regenerate question index, doccheck, commit and enqueue partial ready work.
- QV4, QV6, QV9 remain open. Evidence gathered but not promoted to spec before context wind-down.
- Relevant uncommitted follow-up evidence: BlockMapByLot 0079FEF0; layer5 007A2080; constructor 007A0D50;
  Update clears dirty at active entry, stores clock at 007A4864; developer-null exit bypasses clock.
- Read only the expressly requested owner README under engine/. No engine, ABI or tool edits; no game requests.
