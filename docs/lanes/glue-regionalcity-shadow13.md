| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-shadow13 (regional city shadow) | Codex / `glue-regionalcity-shadow13` | in progress; correction 386435447; focused precision test and owned fmt PASS; stable all PASS: cargo 7409/0/0, clippy clean, native 165/165, CTest 81/81; runs 632/633 clean, 61/73 and 63/73 exact; RY8 filed | Final corrected-source 2032 measurement, then READY report. |

## Handoff
Goal: default-off regionalcity_shadow comparing loaded region records at load/monthly through 2032.
Observer/bridge: 93cfd16e3; SDK string fix f95ee4ee6; reviewed returns 7842bb59f; no public C ABI change.
Final all PASS on stable correction 386435447: cargo 7409/0/0 (51 summaries), clippy 0 diagnostics, native build 165/165 edges (188.5 s), CTest 81/81 (258.6 s).
Prior runs now recorded: 632 regionalcity13-r2-2032-20261024a 61/73; 633 regionalcity13-r2-normal-20261024b 63/73.
Both clean/ref_warnings=0 through 2032; 85 records, 15555 fields/observation, 1135515 fields total.
Only differences: garbage commodity balance arithmetic (12 turbo / 10 normal lines); roundtrips and other getters exact.
RY8 filed: regional-city §5 double wording conflicts with foundations §18.2–18.3 operation rounding.
Current correction: r24(r24(produced+imported)-r24(demanded+exported)), preserving unrounded loads.
Regression vector tests operation rounding, decoded signed operands, and observed fractional result.
Load timestamp now uses borrowed native simulator GetSimDateNumber instead of Julian 0.
Focused operation TERMINATED PASS: cargo test fc4-core commodity_balance_rounds_operations_without_rounding_loaded_doubles; absolute-path qbuild fmt PASS (2 files, zero diff lines).
Log build-runs/regionalcity-r3-focused.log; session 8225 collected to terminal result. Host-target compilation took about 41 minutes under contention.
Initial q.py wrapper failed Git Bash CreateFileMapping access denied; relative fmt paths failed; preserved evidence.
Stable all TERMINATED PASS: exec session 3983 collected; native queue qbuild-20261025-000629-build-ctest PASS (449.3 s).
Logs build-runs/regionalcity-r3-all-transcript.log (appears at first qbuild output), qbuild-test.log, qbuild-clippy.log; native queue follows workspace tests/clippy.
NEXT: unique game request regionalcity13-r3-exact-2032-20261025a through 2032, with rebuilt DLL.
Use build-runs/wait-regionalcity.py to wait once without qwait; helper regionalcity-result.py prints exact clean/total.
No game request outstanding. Next free run number 634 (max checked 631 before adding 632/633).
Keep key default-off; never edit forbidden tools files. Coordinator appends regress shadows key after merge and exact verification.
Final report/merge queue still needed; detached HEAD, commit normally, no ref updates.
