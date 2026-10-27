| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-effects31: effects method evidence (engine) | Codex / method-effects31 / detached HEAD, reserved branch method-effects31; ABI exports unchanged, no seam reservation | wip; gate PASS, recorded-profile credit +104 B, UI 52/52; remaining EF22 observations unavailable; questions none | Supply a conforming EF22 observation host and compare the remaining manager/parser outputs before ready enqueue. |

Changed: native comparison now calls EffectsManager::has_visual_effect, with no native answers copied into core state; embedded-NUL inputs refuse before any query; house script holds turbo off around all UI actions.
Commits: main merged as 8f787a098; tests 65d17423f, manager comparison 2d1edc050, UI script 6530f6eac. Prior independent native calls/complete counters retained.
Gate: qbuild all PASS, Rust 8386/0, clippy 0 diagnostics, native 355/355, CTest 145/145; [final gate](../../build-runs/method-effects31-clean-gate.out) names all full logs.
Tests: focused comparison tests 2/2 passed; both owned Rust files fmt PASS. Truncated C strings cause no native calls or completion credit and leave source bytes unchanged.
Final DLL SHA256: 22e2c546bee20b5838148cf558d25a476fff585dc3f23dc3b5ad6088353a8ee6; unchanged after both final game runs.
House: [recorded r4 profile](../../build-runs/regress-method-effects31-r4.out): turbo to 2026-04-01, PASS, clean/ref_warnings=0; main build-runs/game-results/regress-method-effects31-r4-shadows.verdict.
Exact method (address, bytes, comparison calls, complete, mismatches): 0x005916C0 cSC4EffectsManager::HasVisualEffect, 104, 13860, 13860, 0; resyncs=0; 3 monthly lines, final [log:4994](../../build-runs/regress-method-effects31-r4-shadows.log#L4994).
Scope: 1154 directory names in original/upper/lower spellings plus empty/end/missing; 3465 probes/sample, four samples including load. Boolean is the sole specified output (effects 4.2); probes are separate from natural reach.
Credit: [coverage:205](../../build-runs/method-effects31-final-coverage.md#L205) credits +104 B; method-verified 20520 B/73 functions versus initial 20416 B/72. Profile provenance line 387; no whole-manager/parser credit.
UI: main build-runs/game-results/method-effects31-ui-r3.{log,verdict}: clean/ref=0 at 2026-04-01; all 52 actions ran (log:4584), 52 completion lines, 0 FAILED/SKIPPED; membership 13860/13860 exact, resyncs=0 (log:5538).
Full per-method table: [324 entries](../../build-runs/method-effects31-final-methods.tsv), sorted by bytes, with address, bytes, current capped reach, comparison calls, complete, mismatches/unobserved and missing outputs.
Reach: [UI house census](../../build-runs/game-results/method-effects31-ui-r3.reach.tsv), 92 entries/29607 B; native HasVisualEffect >=106. Startup/shutdown and shadow probes excluded; zero entries are not proof of never being called.
Left reached: 91 entries/29503 B have comparison calls=complete=0, mismatches unobserved. Largest: 005BEC20 3732 B/>=128, Update 2176 B/>=99, UpdateVisualEffect 2012 B/>=98, DeactivateUnusedVisualEffects 1881 B/>=99.
Missing M: natural arguments/results/out-parameters, pre/post manager/live-effect fields and ordered collaborator calls. EF22 pre-Init attachment, exception-transparent forwarding and all-writer exclusion remain unavailable (UI log:143).
Missing P: all 114 command entries have comparison calls=complete=0; no startup entry measurement. Need tokens/consumption, numeric/service replies, exceptional prefixes, description/parser post-state and ordered collaborator calls.
Ownership export: manager 210 entries/68542 B; commands 114/43287 B (brief 42078 B); mapping untouched. 232 entries were not reached in this census, not proven uncalled across the session.
Detailed missing outputs: [eight high-byte methods](../../build-runs/method-effects31-observation-gaps.tsv), including the largest parser commands; remaining per-method reasons are in the full table. Refusal is required by effects 18.8.6-7.
Distinct earlier runs: r3 normal/DLL dae23f20 qualified 58905/58905; baseline house-r2 used DLL 8174b5ea and passed but is not final-source evidence; prior round results remain separate.
Gate failures preserved: build-runs/method-effects31-r2-first-gate-test.log (cache startup), method-effects31-second-native-failure.log (incremental 1/9 rejection); direct CMake clean interrupted, Ninja clean removed 357 files, final clean gate PASS.
No spec questions: no observed membership differences; missing manager/parser observations retain complete=0. Queue stays wip because the requested broader verification remains incomplete.
Next: coordinator publish detached tip; provide EF22 host capabilities, capture/replay remaining reached manager calls and startup parser commands, compare every specified output without resync, then enqueue ready (engine).
