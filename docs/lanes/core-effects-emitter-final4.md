| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-emitter-final4: EF14 emitter corrections | Codex / `core-effects-emitter-final4` (engine lane, detached HEAD) | in progress | Implement EF14, deterministic tests, full gate; pure core only. |

## Handoff
- Goal: implement effects.md §7.1.1 EF14(a-e) literally in particles.rs and focused tests.
- Done: EF14(a-e) read; literal plain top-up, sampled life/stored age, scale reset/precedence, period floor and safe curve edges implemented; nine deterministic tests added.
- Baseline all: cargo 4213 passed, clippy 0 diagnostics, CMake 206/206, ctest 67/67 PASS; wrapper reported tree changed during run (implementation edits began after baseline cargo/clippy).
- Next: quick running (session 50024); fix rustfmt check and failures, run final all on committed code, report/queue.
- Conformance: adapter defaults D+0x10 to life; plain delayed-stop vector corrected from update 7 to 6 (no newborn pre-age, synchronized top-ups).
- Constraints: no plugin/CAPI/ABI changes; no native owner outputs as host inputs; final family round, park any remainder.
- Guard: C:/Users/tlori/FreeCity4/build-runs/run4-core-effects-emitter-final4-r1-manifest.json; 150000 tokens / 80 calls; no subagents.
- Game requests: none; pure core coverage only, no full effects-family claim.
