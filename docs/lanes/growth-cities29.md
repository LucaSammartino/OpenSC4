| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| growth-cities29; engine; city-wide shadows | Codex / growth-cities29 / detached HEAD; reserved branch growth-cities29 | CONTINUE; commits f8ba9dde7, c9f966411, b0828b1a6; ABI unchanged; Rust 7893/7893, clippy 0 diagnostics; native gate pending; two questions | 1. Collect submitted games and native gate. 2. Run targeted profile and resolve remaining cases. 3. Complete 12-scenario acceptance and enqueue. |

- Changes: complete flora instance variant capture; preserve neighbours' captured trade statistics through shadow Init; restore specified 24-bit precision at simulation entries and terrain load queries (foundations §18.2).
- MySims establishment snapshot did not fix the measured difference; reverted in b0828b1a6. Final source needs native validation after that revert.
- Focused neighbours tests: 8/8 passed, including unchanged statistics on busy/not-ready months.
- Quick: 672 Rust tests and 316/316 native edges PASS; tree changed during that run, so this is not the final gate.
- Full gate invoked once: 7893 tests passed, clippy 0 diagnostics; logs `build-runs/qbuild-{test,clippy}.log`.
- Full gate still running (exec session 7105); native request `build-runs/build-queue/qbuild-20261026-121806-build-ctest.req`; no final CTest verdict yet. Do not launch a duplicate while it is outstanding.
- Game evidence root: `C:/Users/tlori/FreeCity4/build-runs/suite/growth-cities29-2/`; each completed scenario has `.json`, `.log`, `.verdict`; JSON `per_key` holds exact before/after counts against census3.
- Before/after terrain: london-sydenham 33/35→35/35; london-woolwich 33/35→35/35; timbuktu-terraforming 33/35→35/35; empty-small 33/35→35/35; all four verdicts clean, end reached, actions passed.
- Terrain evidence: Sydenham log :2791 restores CW 0x027F→0x007F; :2792 and :2849 compare every query exactly. No tolerance changed.
- House berlin-konradshohe: nine keys remain exact: flora-record 3/3, mysims 4/4, development 28/28, terrain 38/38, demand 96/96, aura 3/3, tract 7/7, pollution 9/9, neighbours 3/3; clean/end reached, but 11 script actions failed.
- MySims remains 1/4 exact in each of the four completed new-city scenarios. Empty-small demand remains one differing sample: 96/97 before, 95/96 after. Other targeted keys in those completed scenarios are exact.
- Open: MS-GC29-1 in `docs/specs/questions/mysims.md` (unsaved scheduler lastDay/load dispatch); DM-GC29-1 in `docs/specs/questions/demand.md` (empty-history maximum).
- Flora variants: original North Weburg and Spandau logs stop at 32 bytes when the old capture rejects WriteVariant; fixed capture uses the established variant-capable stream. Failing-city reruns still required.
- Neighbours: Init previously removed live L-list trade contributions before a not-ready/busy month. Snapshot restored after Init; regression test passes. Lua reruns still required.
- Broad sample incomplete: 16 staged, six results collected (five clean, Kensington exited); suite waiter interrupted with Ctrl-C after the unrelated demolition shadow crashed in FC4DemolitionHost.cpp:204–205 (RTC ESP mismatch, then access violation). No acceptance credit from that failed run.
- At stop, main shared game-results still showed `suite-growth-cities29-2-empty-large-shadows.running` and `suite-growth-cities29-2-gen27-konradshohe-s1-churn-shadows.running`; empty-medium finished after the waiter stopped. Collect their final verdicts before submitting more games.
- `--set all --keys ...` failed selection before submission (no measured calibration); tag growth-cities29-1 ran no games. Use explicit scenarios with profile plain plus the nine shadow ini switches to avoid unrelated shadows, or provide a valid calibration source.
- Remaining: development stage/population cases, aura park/landmark timing, Lua pollution/tract/demand precision, flora and neighbours failing-city validation; file narrow questions if corrected-precision runs still differ. Census inputs remain under main `build-runs/run7/census3/`.
- Next: finish the existing native gate, rebuild after any fix, complete at least 12 clean region/gen27/Lua scenarios and house regression on final source, measure ref_warnings=0, then write a ready queue entry and enqueue kind engine. No merge-queue entry was written this round.
