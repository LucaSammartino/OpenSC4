| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-replace16 (default-off disaster layer) | `Codex` / `glue-disasters-replace16` (engine lane) | Ready measured scope: cargo 7503/0, clippy 0 diagnostics, CTest 83/83; d16-ea8-only-a clean; d16-ea8-shadow-a 10/10 exact; both reached 2032-01-19 with ref_warnings=0; C ABI unchanged | (1) Validate the current house replace profile plus disasters. (2) Complete camera movement and special-building names before claiming full replacement coverage. |

The new default-off `disasters` factory replaces cSC4DisasterLayer through the existing Rust layer API. It retains the seven native managers as required by disasters.md §2.8 / DS44; native manager and instance replacement is outside that boundary.
Implemented the 19-slot layer interface, manager lifecycle, borrowed manager lookup, enable counter, period notifications, locations, damage statistics and dispatch timers. Native shutdown can re-enter the layer without holding Rust state. Added factory/default-off CTests and the ABI return-contract citation.
PruneGone releases only references acquired by Keep, using the final-holder audit classification after an occupant leaves the city.

Gate: final qbuild all PASS; cargo 7503 passed, 0 failed; clippy 0 diagnostics; native build 47/47 edges; CTest 83/83. Return audit: 0 unreviewed, 80 reviewed.
Game: `d16-ea8-only-a` clean at 2032-01-19, 7 native managers, Shutdown refused=0; `d16-ea8-shadow-a` clean at 2032-01-19. Both have foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0.
Exact coverage line: `d16-ea8-shadow-a 10/10` (fire 9/9, riot 0/0, quiet 1/1; differences by kind: none). This run did not exercise riot steps or sustained fire steps.
Evidence: `build-runs/game-results/d16-ea8-only-a.{log,verdict}` and `d16-ea8-shadow-a.{log,verdict}`; game DLL hash prefix 3cdb2817. The subsequent source change is only the ABI-review comment; final native gate passed after that change.

Remaining: the existing host's camera movement callback is unwired; the native-format special-building name vector stays empty. The current combined house profile was not run: its list is in the prohibited tools/regress.ps1. Keep disasters out of the house list until the coordinator validates it; no completed credit flip is claimed.
Questions: none filed (revised lane rules); no C ABI exports changed. Failed build and return-audit evidence remains under build-runs/fc4verify-*-failed.log.
