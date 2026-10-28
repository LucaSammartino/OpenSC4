| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| growth-exact34 - engine: commercial stopwatch candidate | Codex / growth-exact34 / detached HEAD (reserved branch growth-exact34); candidate 6c13fe47d, earlier questions d6acbc7ed; ABI exports unchanged, no seam reservations | wip; no newly exact owner; cargo 8410 passed, clippy 0 diagnostics, CTest 146/146; commercial and none blocked by schedule fingerprint; development baseline exact 1/2; diagnostic catalogue and suitability 25/25 with no mismatch, no replacement credit; G34-CM2 and earlier G34-LD1/G34-LD2 filed | 1. Confirm G34-CM2 and provide compatible reference fingerprints. 2. Run commercial and matching none qchecks. 3. Continue the eight unresolved owners below. |

- R2 candidate preserves commercial §4.3's unused stopwatch start: one counter read on every daily/idle delivery, including idle state 3. ScheduleCounterRead uses the virtual counter when fixed, otherwise QueryPerformanceCounter.
- SPEC-GAP: the stopwatch unit is unspecified; units 1–6 share the performance counter (timers §1.2). G34-CM2 asks for the unit and exact read count. This is an unproven candidate for G33-CM1's vehicle floats.
- Final R2 full gate: qbuild all PASS; cargo 8410/0/0, clippy -D warnings 0 diagnostics, CMake 3/3 edges, CTest 146/146. No owned Rust files; fmt not applicable.
- Candidate qcheck-8 commercial and qcheck-9 none on timbuktu-rush-hour both exit 2 before game execution: schedule fingerprint 7bc01a9a908a6189 differs from references a33857dd12297904. No fingerprint bypass/re-pin; no newly exact owner.
- lotdev: NOT EXACT, qcheck-1 exit 1; gen27-spandau-s2-default differs in 103 records (89 only relabelled), first shifted ID 4 class A98F9D28, first content occmgr 098F964D/299B2D1B/0; DIGESTs and sampled RNG agree.
- R1 Spandau: golden-27c1ede8a3f940bb8586e7e7cc6007c5, compare-20261028-031440-e64205ad/result; clean/ref_warnings=0. Reference has four construction occupants, replacement three; G34-LD1 asks for the missing-building lifecycle. Safeguard retained.
- lotdev: NOT EXACT on ui29-volcano; only C9BD5D4A/299B2D1B/0 differs, expected 238079 bytes, actual 238699. Several lots add a zero-valued C999C45E property; the first lot also differs at payload byte 122 (0x26/0x20).
- R1 Volcano: golden-7bf94949f4f34a6ebc39c89e8e5cb747, compare-20261028-031440-6adb2921/result; clean/ref_warnings=0. G34-LD2 asks about nonvirtual zot-helper property creation and remaining daily/counter behavior.
- --replace none: qcheck-2 exit 0, gen27-spandau-s2-default and ui29-volcano exact 2/2; these lotdev differences are absent in the controls.
- Both worktree shadow requests (R1 g34-dev-shadow-1 and R2 g34-r2-dev-shadow-1) stayed unclaimed and were cancelled. No request remains outstanding.
- R2 development diagnostic: forced shadow build, qcheck-7 exit 2 (shadow guard), golden-4e44a05183964d3594a2f27dd48ad79e, compare-20261028-041226-f4caf694/result; game clean/ref_warnings=0. Catalogue 744 profiles and 512 density entries, plus 24 monthly suitability samples, report no mismatches. No exactness credit; diagnostic override reverted.
- The other unchanged owners below retain the coordinator's supplied first differences; they were not rerun in R2:
- residential: NOT EXACT; gen28-konradshohe-s2-grow first delivery 2026-02-07 #21, ui29-bevopolis 2257-04-20 #3381; first content occmgr 098F964D/299B2D1B/0.
- commercial: NOT EXACT; R2 baseline qcheck-3 exit 1, timbuktu-rush-hour one differing record 896E714A/299B2D1B/0; golden-b12426bfc60f4c0d9ab47b8aedf1be09, compare-20261028-034540-f24fe92c/result, clean/ref_warnings=0. Candidate remains blocked as above.
- development: NOT EXACT; R2 qcheck-4 exit 1, Spandau first delivery 2001-09-19 #21, DIGEST 2001-10-09, first shifted ID 4 class 2A5F877D, first content 096E75F5/299B2D1B/0; reach29-growth exact. Runs compare-20261028-035329-44c990d5/result and -86ce97ed/result; both clean/ref_warnings=0.
- R2 none qcheck-5 and qcheck-6 exit 0, gen27-spandau-s2-default exact; DLL bf1eb177 (fallback diagnostic setting, shadow did not activate). Qcheck-6 staged that earlier DLL while the forced build was pending; it is not forced-shadow or final-candidate evidence.
- demand: NOT EXACT; gen28-konradshohe-s2-default first delivery 2026-05-02 #21, first content 096E75F5/299B2D1B/0.
- zonedev: NOT EXACT; gen28-tegel-s2-default first delivery 2030-04-30 #21, first content occmgr 098F964D/299B2D1B/0.
- lotcfg: NOT EXACT; gen28-tegel-s1-default first delivery 2029-08-30 #21, first content occmgr 098F964D/299B2D1B/0.
- propmgr: NOT EXACT; maxisland-north-forks first shifted ID 21 class 6A26D049, first content occmgr 098F964D/299B2D1B/0.
- exemplars: NOT EXACT; ui29-weburg first delivery 2197-04-12 #21, first content occmgr 098F964D/299B2D1B/0.
- Run directories above are under C:/Users/tlori/FreeCity4/build-runs/golden-runs; qcheck-1 through qcheck-9 and gate logs are in this worktree's build-runs. G34-CM2 is in docs/specs/questions/commercial-industrial.md; earlier G34-LD1/G34-LD2 remain open.
- No unlanded branches merged, no game files committed, no C ABI exports changed. Next: resolve timer contract/reference fingerprint, then test commercial alone and none on timbuktu-rush-hour before considering ready.
