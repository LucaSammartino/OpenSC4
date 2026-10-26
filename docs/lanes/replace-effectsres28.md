| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-effectsres28: effects resource (engine) | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/replace-effectsres28; detached HEAD; reserved branch replace-effectsres28 | WIP EF26; code 18212e9df; ABI exports unchanged; focused CTest 9/9; full gate pending; game acceptance unmeasured | 1. Answer EF26. 2. Complete nonempty resource IO. 3. Measure (a)-(c). |

- Merged main first; retained a7c3c61f4's root-version rejection and its core/C ABI tests. EF25 is resolved.
- Added the default-off `effectsres` class registration independently of `effects`; existing manager registration and month-logging source are unchanged.
- Implemented nine primary slots, C=R+4, P=R+11C, canonical unknown=P, shared atomic count, copied name at R+134, and persistence key getters/setters.
- Publication uses the independent native collection service; cleanup restores the constructor's base collection vtable before its destructor. No native effects-resource Read/Write is called.
- Stream support is partial: empty root-3/root-4 directories, root-4 output, camera presence/value, typed strings, ignored primitive status, and interrupted-output prefixes.
- Nonempty descriptions, visual/name maps, bindings and triggers remain unsupported; Read/Write refuse them and log EF26. No full-resource loading is claimed.
- EF26 filed at the end of `docs/specs/questions/effects.md`: typed renderer codec helper parameters/ownership, or missing record framing/reconstruction contracts.
- Focused CTest **9/9 PASS** in `build-runs/effectsres-focused.log`: resource contracts, four resource registration modes, all four existing manager registration modes.
- Corrected-source `qbuild quick` **PASS**, no Rust crate touched; native build 151/151 edges (38.0 s).
- Full `qbuild all`: **pending**, unified session 5037; one invocation waiting 919 s for a shared cargo slot. No current-round cargo/clippy/full-CTest result claimed yet.
- (a) House list plus effectsres to 2032: **unmeasured**; no game label/log; ref_warnings unavailable.
- (b) Cached-original golden to 2026-04-01 and generated gen27 script comparison: **both unmeasured**; no comparison logs.
- (c) Every regress shadows-profile key versus regress-train267-shadows.log: **unmeasured**; no comparison log.
- Launch failures excluded from verdicts: q.py Bash CreateFileMapping failure and ctest missing from PATH; the absolute SDK ctest then passed 9/9.
- **effectsres is not ready for the replace list.** No ready enqueue or cSC4EffectsResource credit requested.
- Next: collect session 5037 (do not launch a duplicate all gate), resolve EF26, finish incremental IO, then measure (a)-(c) through the shared queue (`parallel = 1`, maximum three) and enqueue kind engine.
