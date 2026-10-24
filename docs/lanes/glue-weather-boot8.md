| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-weather-boot8: weather/SCRes observer coexistence | Codex, `glue-weather-boot8` | ready; full gate PASS; weather 73/73 with SCRes on and off | Coordinator review and credit weather coverage; land reserved branch |

## Change and measured cause

The regression was an observer conflict. `scres_file_shadow` arms all four hardware execution breakpoints (`DR7=0x55`); weather previously refused an occupied DR0. Its first weekly CRT observation therefore failed, and the independent iterator used different random inputs. Diagnostic delivery/native dates matched. A diagnostic run also diverged with February `steps=5`, while both final exact runs had `steps=4`.

SCRes now permits weather to borrow its DR0 only on the installing thread outside active parser/file frames or pending forwarding. Weather accepts only that known execution breakpoint, saves DR0/DR6/DR7, observes the genuine weekly CRT calls, then restores the complete saved debug state. Weather section 5.2's synchronous weekly operation enters no resource parser and only queues notifications. Foreign occupied DR0 remains refused and counted as a mismatch.

Bootstrap remains a single native record capture; subsequent day/week/month calls and CRT draws are replayed independently. No native weather state is copied afterward, no mismatch is discarded, and no extra catch-up steps are invented. The evidence did not support the proposed missing-day diagnosis. Bounded diagnostics retain native/delivery dates, observed CRT sequences, observer-arm failures, and the first byte difference even when an earlier observer error has already incremented the mismatch count.

Default-off switches and the C ABI are unchanged. Core algorithms are unchanged. No tools files, generated ABI entries, shared regress profile, or coverage maps were edited. The supplied train183 profile already contains `weather_shadow=1`.

## Validation

- Final stable-source `qbuild.py all`: Rust **6960 passed, 0 failed, 0 ignored**, clippy **0 diagnostics**, native build **48/48 edges**, full CTest **78/78**. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; native queue verdict `build-runs/build-results/qbuild-20261024-070503-build-ctest.verdict`. All asynchronous operations collected to terminal.
- `weather-boot8-fixed-on-05 73/73` — SCRes file replay on, house train183 switches, turbo to **2032-01-01**, **clean**, `ref_warnings=0`, 655.437 seconds. [Run 442](../runs/weather-boot8-fixed-on-05-442.md).
- `weather-boot8-fixed-off-06 73/73` — SCRes file replay off, otherwise the same profile, turbo to **2032-01-01**, **clean**, `ref_warnings=0`, 700.750 seconds. [Run 443](../runs/weather-boot8-fixed-off-06-443.md).
- SCRes remained exact in the on run: summary **146/146**; final raw file replay and native command token lines both `mismatches=0`, `refused=0`; raw replay `resyncs=0`.
- Evidence for each run: `build-runs/game-results/<label>.log` and `.verdict`. Counts use every dated weather shadow line, including load and 72 month lines, with zero cumulative mismatches throughout.

## Reproduction evidence

- [Run 439](../runs/weather-boot8-before-on-02-439.md): `weather-boot8-before-on-02 2/27`; March `mismatches=54102`, first byte 108. Clean/ref_warnings=0; the initial 240-second limit ended at 2028-03-24, so this is not credited as a 2032 run.
- [Run 440](../runs/weather-boot8-diag-on-03-440.md): `weather-boot8-diag-on-03 2/4`, clean through 2026-04-02, February steps=5; first record difference step=7, byte=107.
- [Run 441](../runs/weather-boot8-diag-on-04-441.md): `weather-boot8-diag-on-04 2/4`, clean through 2026-04-02; native and delivered dates agree, first week observer refuses `DR7=0x55`, followed by first record difference at step 7.
- `weather-boot8-before-on-01` was rejected before DLL construction; retained verdict, no game run credit.

Questions: none. No missing native contract was needed for this plugin observer coordination. No game requests remain outstanding. Coordinator next: review the scoped breakpoint loan, credit weather's exact 73/73 run evidence, and land the ready entry. No merge or ref update was performed from this detached worktree.
