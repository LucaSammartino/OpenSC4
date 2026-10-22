| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-emitter-final4: EF14 emitter corrections | Codex / `core-effects-emitter-final4` (engine lane, detached HEAD) | in progress | Implement EF14, deterministic tests, full gate; pure core only. |

## Handoff
- Goal: implement effects.md §7.1.1 EF14(a-e) literally in particles.rs and focused tests.
- Done: c3833c3a implements EF14(a-e); literal plain top-up, sampled life/stored age, scale reset/precedence, period floor and safe curve edges. Eleven deterministic tests now added; fmt passes on all three touched Rust files.
- Baseline all: cargo 4213 passed, clippy 0 diagnostics, CMake 206/206, ctest 67/67 PASS; wrapper reported tree changed during run (implementation edits began after baseline cargo/clippy).
- Next: quick running (session 4672); check results, run final all on committed code, report/queue.
- Conformance: adapter defaults D+0x10 to life; retain original 7-update delayed-stop vector (the first quick caught an incorrect expected 6); add a separate 2.3-target test with exactly 3 creations and death on update 3.
- First quick: conformance fixture assertion failed (7 observed vs incorrect 6 expected); corrected as above. No implementation failure established by that run.
- Constraints: no plugin/CAPI/ABI changes; no native owner outputs as host inputs; final family round, park any remainder.
- Guard: C:/Users/tlori/FreeCity4/build-runs/run4-core-effects-emitter-final4-r1-manifest.json; 150000 tokens / 80 calls; no subagents.
- Game requests: none; pure core coverage only, no full effects-family claim.
