| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach27 native entry census; tools | Codex / .claude/worktrees/reach27 / reserved reach27 (detached HEAD) | BLOCKED on queue output routing; code c25ae211d; gate Rust 7844, clippy 0, CTest 118/118; r4-house-off-1, r4-house-on-1, r4-house-on-2 clean; shadows have documented differences; ABI unchanged; questions none | 1. Harness owner forwards reach_output for bare reach=1. 2. Verify labelled TSV delivery and enqueue kind tools. |

Merged main (19a2aa1f9); saturation/fences a8754355c; standalone test fix 8bbeffebc; initial-date fix c25ae211d. Everything remains default off; no owned Rust files or C ABI changes.
At shutdown, emits `reach window N date=YYYY-MM-DD new=N total=N`; first/last window TSV fields retained. Native tests cover empty windows and cumulative counts.
Arms after compare/shadow hooks. Pending overlay, queued DBPF, and observe load probes now pause tracing process-wide; idle ticks do not suspend/re-arm. Initial date comes from the borrowed simulator interface.
Canonical census preserved: docs/abi/reach.tsv, reach27-r2-house-on-1, Konradshohe shadow profile, 2026-Jan..2032-Jan, 73 windows, build 4d26a816e, DLL 5245b5d4; pilot remains reach-pilot.tsv.
(1)/(4) Canonical off/on and all three R4 house runs clean: foreign exceptions, stalls, rtc, overruns, ref_warnings, panics and INCOMPLETE=0; no crash/hang. Canonical verdict dates off/on=2032-01-08/06.
Canonical monthly shadows off/on=5659/5731 exact each: 72 occupant queued-message ordering findings; records, queries and returns exact. Whole-city equality was waived by coordinator.
Known limitation: prior reach-on load overlay flags original=0x91/core=0xd1 (self-lighting bit 6), four fields plus aggregate; not investigated further. Both R4 reach-on load/month overlays are exact after fencing.
(2) Native DLL A=29/29, B=44/44, float/flags/ret/fault=2/2; WoW64 translated breakpoint/step=7/7, body=10/10, re-arm/cap passed. Latest concurrent same-entry count=879/10000: documented lower bounds, accepted by coordinator.
R2 game calibration: PlaceZone=260/260 independently counted (256 monthly + 4 scripted, pre-arm load excluded); CanCreateLot observed=17712; traps=35944=2*(260+17712), aborted/exhaustions=0. Canonical PlaceZone=0; CanCreateLot=2304 across 72 windows, capped.
(3) Canonical loaded seconds off/on=1128.250/1658.969 (1.4704x); harness wall=1145/1676s (1.4638x); traps=18551190, 11182.4/s.
R4 off-1/on-2 loaded=99.297/116.531s (1.1736x), wall=116/135s (1.1638x), verdict dates Apr-27/Apr-12; same requested Apr-01 end, turbo/shadows. Traps=1011250, 8677.9/s; pauses=92; ignored/aborted/exhaustions=0.
R4 on-2 TSV/log check passed: Jan-27/Feb-01/Mar-01/Apr-01 first-seen=5776/1142/47/90, cumulative=5776/6918/6965/7055; every count agrees with first_window rows, first/last bounds checked.
Roster=35648; armed=35274; skipped=374: exception/loader 43, jump/hook 258, unsafe prefix 69, changed entry 3, unreadable/non-executable 1. Every address/reason logged and exported; no late conflicts.
| Canonical metric | Functions | Known bytes |
|---|---:|---:|
| Reached | 7883 | 1580524 |
| At least 20 capped entries | 6410 | 1288351 |
| At least 3 windows hit | 6426 | 1270541 |
| Both thresholds | 6148 | 1221078 |
R4 monthly exact off-1/on-2=220/226 vs 215/221. Both have only baseline lotwall record differences (42/41/41) and occupant queued ordering; occupant records/queries/returns exact. On-1 additionally had one live prop-lot-list difference at (45,116), absent off-1/on-2; cause unresolved.
Bare [ini] reach=1 DOES trace (on-1, 7112 functions/4 windows), but queue leaves output in slot13/Plugins/FreeCity4.log.reach.tsv; no labelled results TSV. On-1 also exposed initial date zero; fixed and verified on-2. On-2 supplies reach_output explicitly; these cases remain distinct.
BLOCKER / Next: tools/ is forbidden to this clean-room lane. Queue owner must default reach_output=<WT>/build-runs/game-results/<label>.reach.tsv when ini reach is true, preserving explicit output. Then replay a bare-key request before marking ready; no questions filed or tools accessed.
Evidence: build-runs/game-results/reach27-r{2,4}-*.{log,verdict,reach.tsv}; reach-r4-report.py; native LastTest.log; reach-r4-failed-gate/ and reach-r4-pre-date-fix-gate/ distinguish failed, corrected, and final gates. Final qbuild all passed; canonical provenance unchanged.
