| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-replace16 (default-off layer, managers and instances) | `Codex` / `glue-disasters-replace16` (engine lane) | Ready measured scope: cargo 7503/0, clippy 0 diagnostics, CTest 83/83; d16-e471-only-r2-a clean at 2032-01-08; d16-e471-shadow-r2-a 6/6 exact at 2032-01-11; both ref_warnings=0; C ABI unchanged | (1) Validate the current house replace profile plus disasters before adding it to that profile. (2) Complete specialized manager/instance interfaces and active-disaster coverage. |

The default-off `disasters` key now registers the layer, seven manager factories and seven instance factories. The layer creates Rust-backed managers rather than retaining native managers (DS44 requires replacing their instances together).
Managers share one city runtime over `core/src/disasters`. Backend-created instances have stable COM identity, generic state/location queries, animation ticks and a typed fire interface. Period and location changes flow through the public layer; its pause/save and assessment state remain authoritative.
Added manager/default-off registration checks, fire identity/cause checks, and direct thiscall tests for manager vtable slots 6 and 7. MSVC reverses overloaded virtual declarations; the declaration follows the SDK so slot 6 takes coordinates and slot 7 takes none.
PruneGone already released Keep-owned references with the final-holder audit classification; this round retains that fix.

Gate: final qbuild all PASS on the slot-corrected source; cargo 7503 passed, 0 failed; clippy 0 diagnostics; native build 48/48 edges; CTest 83/83. Native verdict: `build-runs/build-results/qbuild-20261025-013505-build-ctest.verdict`.
Game: `d16-e471-only-r2-a` clean at 2032-01-08, 7 Rust managers, Shutdown refused=0; `d16-e471-shadow-r2-a` clean at 2032-01-11. Both foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0; DLL hash prefix 537bdf25.
Exact coverage line: `d16-e471-shadow-r2-a 6/6` (fire 4/4, riot 0/0, quiet 2/2; differences by kind: none). Original steps 0, ours 0: sustained fire/riot behavior and the PruneGone path were not exercised.
Evidence: `build-runs/game-results/d16-e471-only-r2-a.{log,verdict}` and `d16-e471-shadow-r2-a.{log,verdict}`. The earlier `d16-6656-only-r2-a` clean verdict belongs to the pre-slot-correction DLL and is not final-source evidence.

Open: the combined house profile was not run: its current list is only identified in prohibited `tools/regress.ps1`; a permitted copy was requested but not supplied. Keep disasters out of the house list until that run passes; no completed credit flip is claimed.
Open: generic factory instances expose IUnknown identity but lack specified numeric typed IIDs and non-fire initialization entry points. Specialized volcano/effect/toxic manager interfaces are not implemented. The key remains default-off. Existing host camera, special-building names, and several terrain/pipe/effect writes remain incomplete.
Questions: none filed under revised lane rules; the missing generic-instance IID contract has a `// SPEC-GAP:` comment. No C ABI exports or coverage/tooling files changed. Queue state ready covers this measured implementation only.
