# Suite census failure triage, round 2

The 66 returned attempts in `suitecov27-census-r2` reparse as 28 corrupt,
13 exited, 9 invalid, 7 stalled, 6 no-load, 2 unknown and 1 clean. This is the
original DLL and source snapshot, before this round's scenario corrections.
No game was rerun before this classification. `python tools/suite_triage.py
suitecov27-census-r2` writes `build-runs/suite/suitecov27-census-r2/triage.tsv`:
one row per attempt, the original verdict line, first exception or stall sample
and address, last completed action when no address was logged, and trace status.
Addresses missing from the log remain `-`; exit code -1073741819 is not a PC.

| Cause | Scenarios | Evidence / disposition |
|---|---|---|
| Unestablished city (6) | empty-small, empty-medium, empty-large, london-sydenham, london-woolwich, timbuktu-terraforming | `AUTOTEST VERDICT: clean`, date=2000-01-01, days_per_second=0.00; suite verdict invalid. Lua now establishes mayor mode before the companion dated script. Terraforming dates were incorrectly in 2022; rebased to 2000, preserving the 92-day window. |
| Window exhausted (3) | gen27-konradshohe-s0-edge, gen27-konradshohe-s2-churn, gen27-tegel-s2-edge | `AUTOTEST VERDICT: clean` at 2027-02-23, 2026-05-31, 2030-03-03 respectively; requested ends 2028-01-28, 2028-01-28, 2031-08-21. Their 750/240/240-second windows were insufficient. Keep original end dates and missing completion; do not relabel these as passing shorter scenarios. |
| Launcher load detection (6) | berlin-schonerlinde, gen27-konradshohe-s0-grow, gen27-konradshohe-s2-default, gen27-konradshohe-s2-grow, gen27-spandau-s2-churn, gen27-tegel-s0-default | Harness `no city load within 180/300 s`, despite `autotest: clock started` and, in several runs, many executed actions. Load detection depended on disabled budget's Init hook. Launcher now also recognizes the autotest clock marker. This correction does not change the historical verdicts. |
| Lua launch rejection (2) | lua-lots-empty, lua-lots-grown | Harness `no verdict line ... (exit code 1)`; `lua_script must name an existing repo-relative .lua file`, before any game log. Main's merged launcher now resolves Lua against the staged DLL's checkout. The existing empty lot companion script is retained and staged. |

The following seven have `AUTOTEST VERDICT: stalled`. These are first watchdog
sample PCs, not proven exception addresses or proof of the blocking caller:

| Scenario | First stall PC | First stall point |
|---|---|---|
| maxisland-aquaton | 0x7724867C | ntdll.dll; no OnTick for 5656 ms |
| maxisland-indereast | 0x65C23CC4 | AcLayers.dll; no OnTick for 7672 ms, before clock start |
| gen27-konradshohe-s1-churn | 0x783A9E02 | FreeCity4.dll; no OnTick for 5797 ms, before clock start |
| gen27-konradshohe-s1-edge | 0x783A9D5D | FreeCity4.dll; no OnTick for 7032 ms, before clock start |
| gen27-konradshohe-s1-grow | 0x7AA0B892 | FreeCity4.dll; no OnTick for 5344 ms |
| gen27-spandau-s2-default | 0x7724915C | ntdll.dll; no OnTick for 5235 ms; game date 2001-09-10 |
| gen27-tegel-s1-grow | 0x7A5EB160 | FreeCity4.dll; no OnTick for 5062 ms, before clock start |

All thirteen exited verdict lines are `exited (code -1073741819) without a
verdict`. None logged an exception address. Last completed progress markers:

| Scenario | Last recorded point |
|---|---|
| maxisland-san-indersty | script line 6, terrain at 2156-06-24 |
| maxisland-simicon-valley | clock started; end target 2102-11-07 |
| maxisland-west-weburg | script line 5, speed 0 at 2197-03-22 |
| timbuktu-rush-hour | clock started; end target 2010-07-31 |
| maxisland-bevopolis | before any clock/action marker |
| maxisland-caramel-island | clock started; end target 2037-11-05 |
| maxisland-darris | script line 25, pricezone at 2097-05-01 |
| maxisland-east-weburg | clock started; end target 2112-08-08 |
| maxisland-letroy | clock started; end target 2138-04-13 |
| timbuktu-making-money | clock started; end target 2009-02-03 |
| gen27-konradshohe-s2-edge | script line 103, pricezone at 2026-06-28 |
| gen27-spandau-s0-edge | script line 23, pricezone at 2001-09-12 |
| gen27-spandau-s1-grow | script line 65, mark at 2002-02-10 |

The remaining 28 corrupt attempts have logged exceptions; the raw TSV retains
their first addresses and exact lines. The known 0x00508317 terminal-disaster
failure is a reach tracer finding, supported by the previous same-DLL reach-off
control. This lane changes no tracer, plugin, executable or subsystem ownership.
Reach-off replacement controls must remain distinct from reach-on census runs.
