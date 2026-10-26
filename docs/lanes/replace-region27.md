| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-region27 / RegionalCity replacement (engine) | Codex / replace-region27 / detached HEAD | Blocked/WIP: merge 45f373821; bridge c42c7a0a9; compile fix 108812220; ABI unchanged; cargo 7859/0, clippy 0, native 50/50, CTest 119/119; game acceptance unmeasured; RY9 filed | 1. Resolve RY9/RR3 and collect queued baseline. 2. Implement remaining native cache/connection hosts and regionalcity COM factory. 3. Measure acceptance (a)-(c), rerun gate on implementation and enqueue ready. |

Round 2 resumed from c42c7a0a9; the earlier merge is already present.
Native compile fix 108812220: supply FC4_ACQUIRED provenance; quick native build completed 47/47 edges (build-runs/fc4verify-build.log).
(a) House replace list plus regionalcity through 2032: NOT RUN; ref_warnings unmeasured; no replacement factory exists.
(b) regionalcity shadow with replacement on: NOT RUN; 73/73 unmeasured; no supporting log.
(c) Berlin neighbour crossing/deal and save in replace mode: NOT RUN; native border host blocked on RY9.
Full qbuild all PASS: cargo 7859/0, clippy 0 diagnostics, native 50/50, CTest 119/119 (qbuild-20261026-004545-build-ctest).
Gate logs: build-runs/qbuild-{test,clippy}.log and build-runs/fc4verify-{build,ctest}.log; tree-change notice was comments/handoff only.
Formatting: eight owned Rust bridge files passed; an initial relative-path invocation found no files and is superseded.
Baseline region27-r2-records2032a rejected (script outside [ini]); see MAIN/build-runs/game-results/region27-r2-records2032a.verdict.
Corrected region27-r2-records2032b remains queued in MAIN/build-runs/game-queue at 2026-10-26 01:10:39; no verdict or plugin log.
Its timeout+15-minute cutoff was 01:09:33; two qwait calls timed out, final wait interrupted; request preserved, no duplicate pending run.
Baseline enables simulator,budget only; it cannot satisfy replacement acceptance. DLL SHA256 F8F1E4EE4FDD49F36D2018B590B454088C1627FDD077DD961D36A1541337AC31.
RY9 filed at docs/specs/questions/regional-city.md: RR3 leaves border-demolition privilege/flags/filter policy unspecified; demolition §4.4 makes these observable.
No regionalcity factory/key is registered; RegionalCity is NOT ready for the replace list. The bridge is prerequisite work only.
Next: resolve RY9 and collect the outstanding baseline; stop the border-host/COM-registration item until the demolition policy is specified.
