| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au40 (audio owner contracts round 2) | Codex / `re-audio-au40`, detached HEAD, RE | in progress: AU41 answered, 12 new spec vectors, doccheck passed; AU40/AU44 previously answered | Check measured guard; finish report and queue, or continue AU42/AU43 within budget |

## Report

Round 1 answered AU40 (`1a339eeb2`) and AU44 (`6ee211ad9`), with 18 vectors and passing
doccheck. Round 2 adds AU41 in audio.md §19.2, correcting supplement §19 and moving the
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
AU42 and AU43 remain open; there is no full owner-closure claim for scape/preferences.

## Handoff
- Goal: close audio owner questions with prose and vectors; round 2 AU41 completed.
- Done: AU40/AU44 prior round; AU41 ready for milestone commit; doccheck passed.
- Next: run measured guard after commit; at two thirds stop new RE and finalize report/queue.
- If budget permits future work: AU42 scape resource/group/instance slots, then AU43 preferences.
- Exact source: main decomp/_unnamed/region_004e0000.c, SEM class file and export instructions.
- Numeric verification covered all six model entries; constants helper build-runs/read-floats.py.
- Detached HEAD intentional; supervisor publishes commits. No branches/refs changed.
