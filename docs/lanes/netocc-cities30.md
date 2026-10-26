| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| netocc-cities30: census network shadows (engine) | Codex; .claude/worktrees/netocc-cities30; detached HEAD, reserved branch netocc-cities30 | ready; source 5c65bfecc; ABI unchanged; Rust 8211, clippy 0, CTest 143/143; 109/109 cities, Lua 9/9; netocc 345/345, netmgr 452/452 | 1. Land the full committed HEAD. 2. Coordinator updates coverage credit. |

- Acceptance: 109/109 census region/city pairs; 112 qualified clean runs; ref_warnings=0, watchdog_stalls=0, foreign_exceptions=0; netocc 345/345 and netmgr 452/452 exact. No request remains outstanding.
- Source: netocc-cities29 was already included; merged main e841a16f6 as 5c65bfecc. Inherited NO46/NO47/NM18/NM19 fixes and tests remain; no engine source edits after this round's gate.
- Frozen-source qbuild all ONCE: PASS; Rust 8211/0, clippy 0 diagnostics, native build 265/265, CTest 143/143. Logs in this worktree: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
- Every qualified run uses DLL SHA256 09bb1541d6b82a2f9622f91eaaa35d32927f64a2f0b9a65dd56685bccf450a97; all four staged copies match the gated DLL.
- Suite evidence root: C:/Users/tlori/FreeCity4/build-runs/suite/. Tags netocc-cities30-r4-{lua,maxi,paradis,refresh}; each has summary.tsv, per_key.tsv and per-scenario .log/.verdict/.json.
- Qualified suite rows: lua 6/9, maxi 14/15, paradis 46/47, refresh 41/41; five direct clean runs below supply complete Lua sweeps, established Magnolia and the house network workload. This is city-pair/network qualification, not all 125 full-suite workloads passing.
- Direct evidence root: C:/Users/tlori/FreeCity4/build-runs/game-results/; labels below have prefix netocc-cities30-r4- and .log/.verdict; Lua details are q-main-<label>.lua.log.
- Lua 9/9: six setup drivers completed in the lua suite; lua-empty-normal, lua-grown-complete and lua-qualified-complete each logged 175 lot results, three demolition modes, both saves and scenario-complete. All clean/end/ref0/watchdog0.
- Complete sweep samples (netocc; netmgr): lua-empty-normal 3/3;3/3, lua-grown-complete 7/7;7/7, lua-qualified-complete 7/7;7/7. Normal pacing; grown end dates extended to 2197-09-20 to let scripts finish.
- Lua placements retain every result: empty 161 accepted/14 refused; grown 61/114; qualified sites 133/42. Total 355/525 accepted, 170 refused; refusals are not successful placements.
- magnolia-established: documented SetCityEstablished setup plus original Magnolia dated script; all 29 actions attempted, clean/end/ref0/watchdog0, netocc 3/3 and netmgr 4/4. Original no-setup paradis-magnolia remains stalled/unqualified.
- house-census: FC4 Test/Konradshohe with the Berlin house census script; 29 actions attempted, 11 native refusals retained; clean/end/ref0/watchdog0, netocc 4/4 and netmgr 5/5. Original ui29-house had 48 pending actions and receives no workload credit.
- Empty accounting: 12 equal/unexercised occupant comparisons receive no ownership credit; the two declared Berlin load-only cities ran 12 loaded seconds. Other qualified runs reached their requested end dates.
- Former blockers: refresh/maxisland-{bevopolis,letroy,north-weburg}.log each clean/end/ref0/watchdog0, netocc 3/3 and netmgr 4/4. Paradis Port also clean/exact in paradis/paradis-port-of-shiganshina.log.
- c047 reviewed: netocc-cities30-r3-c047.log clean/end/ref0/watchdog0, netocc 3/3 and netmgr 4/4; Sable was then refreshed on the final DLL in refresh/maxisland-sable.log.
- Unqualified controls preserved: original turbo Lua sweeps stopped at 96/80/80 results; short normal grown variants stopped at 144/136; lua-empty-long ended before the demolition pass; lua-grown-normal was rejected for a nonexistent companion script. None count toward acceptance.
- Suite commands exit nonzero for retained native refusals and incomplete original controls; qualification above requires clean verdicts, exact comparisons, completed dated actions and Lua completion markers. No comparison, unsupported record or transient input was exempted or resynced.
- Operations: shared main game queue only, at most three own runs in flight; no direct game launch, coverage/regress edits, new ABI exports, game files or run histories committed.
- Questions: no new questions. NC30-HARNESS resolved by documented lua_script/lua_extensions launch and observed completion; NM18-A/NM19-A remain writer-owned runtime observations, with no cause inferred from passing cases.
- Next: enqueue actual full committed HEAD with all 112 qualified labels; coordinator may land and update coverage credit. All acceptance evidence is complete.
