| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au40 (audio owner contracts round 2) | Codex / `re-audio-au40`, detached HEAD, RE | ready: partial closure; AU41 answered, AU42 capacity/wrapper specified, 18 new vectors; doccheck passed; no ABI change | (1) Finish AU42 resource/instance/timing contracts; (2) answer AU43 preferences |

## Report

Round 1 answered AU40 (`1a339eeb2`) and AU44 (`6ee211ad9`), with 18 vectors and passing
doccheck. Round 2 adds AU41 (`2e1ac72c8`) in audio.md §19.2, correcting supplement §19 and moving the
original question to questions/answered/audio.md. Regenerated docs/specs/QUESTIONS.md.

AU41 supplies SEM/model construction, binding, registration and teardown, all six model
slot 5/6 operations, radius/projection/pan arithmetic, return conventions and twelve
conformance vectors. Registration has no rollback. Allocation failure reaches an unchecked
QI; teardown requires Hit. Primary model slot 4 calls the host's slot 14 with the embedded
name interface. Final volume uses a logical shift after wrapped multiplication, and effects
pan uses absolute projected X. Startup/teardown ordering is verified against director callers.

Evidence: main-checkout re_query, decomp and instruction bytes, with constants read from the
working executable. No REA call attempted: this session has no binary open. No assembly or
pseudocode copied into specs. Read core-audio-close13 Owner verdicts before answering.

Gate: foreground `python build-runs/check-docs.py` passed `doccheck check: ok`; output in
`build-runs/doccheck-re-audio-au40.log`. Cargo, clippy, CTest and game runs: n/a for prose.
No ABI changes or proposals; no engine reads/edits, plugin, capi or coverage-map changes.
AU42 also gains a bounded partial answer (`35885e408`) in audio §20.4: slot 11's signed capacity,
untouched outputs on failure and raw position-copy behavior, plus slot 13's dynamic slot 12
wrapper, with six vectors. AU42 and AU43 remain open; there is no full owner-closure claim
for scape/preferences. No new questions were filed. AU40/AU44 remain answered without
duplicating their open entries. This round contributes 18 vectors (36 across both rounds).

The two real-work commits succeeded and HEAD advanced despite Git reporting an attempted
`packed-refs.lock` write denied by the shared-ref sandbox. No ref/config/maintenance command
was issued to work around that message; the supervisor publishes this detached HEAD.

## Handoff
- Goal remaining: AU42 full scape contract, then AU43 preferences; neither claimed closed.
- Done: AU41 `2e1ac72c8`; AU42 capacity/wrapper `35885e408`; both doccheck passed.
- Guard last measured 94,421 / 150,000 before report finalization; stopped new RE near two thirds.
- Exact next step: re_query func/asm 0x004F64F0 and resource helpers for AU42 group loading.
- Source: main decomp/_unnamed/region_004f0000.c; 0x004F64F0 starts line 1944.
- Other known offsets: slot 7 helper 0x004F5F20 line 1597; slot 8 0x004F6620 line 2018;
  slot 12 0x004F7300 line 2733. Query export first and verify all numeric claims with bytes.
- AU42 slots 11/13 now in audio §20.4; do not reopen or overwrite settled capacity behavior.
- AU43 not investigated this round: supplement §21 and open question give starting addresses.
- AU41 source: region_004e0000.c and SEM class; constants helper build-runs/read-floats.py.
- Detached HEAD intentional; supervisor publishes commits. No branches/refs changed.
