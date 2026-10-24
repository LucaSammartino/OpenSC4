| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-shadow13 (regional city shadow) | Codex / `glue-regionalcity-shadow13` | in progress; correction 386435447; focused precision test PASS and owned fmt PASS; previous gate cargo 7408/0, clippy clean, CTest 81/81; runs 632/633 clean, 61/73 and 63/73 exact; RY8 filed | Run stable all and final 2032 measurement. |

## Handoff
Goal: default-off regionalcity_shadow comparing loaded region records at load/monthly through 2032.
Observer/bridge: 93cfd16e3; SDK string fix f95ee4ee6; reviewed returns 7842bb59f; no public C ABI change.
Prior all PASS on 7842bb59f: cargo 7408/0/0, clippy clean, CTest 81/81; evidence under build-runs.
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
NEXT: run qbuild all stable source (includes queued native verification if supported), then unique game request to 2032.
Use build-runs/wait-regionalcity.py to wait once without qwait; helper regionalcity-result.py prints exact clean/total.
No game request outstanding. Next free run number 634 (max checked 631 before adding 632/633).
Keep key default-off; never edit forbidden tools files. Coordinator appends regress shadows key after merge and exact verification.
Final report/merge queue still needed; detached HEAD, commit normally, no ref updates.
