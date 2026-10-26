| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| netocc-cities29: city-wide network shadows (engine) | Codex; .claude/worktrees/netocc-cities29; detached HEAD, reserved branch netocc-cities29 | blocked; code cf4ce5612, 22414662d; main merged 11931e530; ABI unchanged; cargo 7893, clippy 0, CTest 124/124; final house clean/exact; NO46/47, NM18/19 | 1. Resolve questions. 2. Re-run blocked scenarios and qualify the broad sample. 3. Enqueue engine; no coverage flip yet. |

- Changed: reproduce avenue work/accepted hash-set order, inclusive bounds and tile requery; compare all terrain outcomes at native handler completion; retain every unsupported record and mismatch.
- Frozen-source gate PASS: 7893 Rust tests, clippy 0 diagnostics, CTest 124/124; owned Rust fmt clean. Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-ctest.log. An earlier passing gate flagged a mid-run log-text edit and was repeated after freezing source.
- Broad sample: 12 attempted, verdicts 9 clean/1 corrupt/2 stalled, all ref_warnings=0; samples netocc 90/102, netmgr 104/107. Three clean-labelled runs also have watchdog counters; no all-suite exactness claimed.
- Below: census3 before -> broad6 after, (netocc exact/total; netmgr exact/total); logs C:/Users/tlori/FreeCity4/build-runs/game-results/netocc-cities29-broad6-NN.log, DLL 33ee32c9 (before final timing fix). C=clean, X=corrupt, S=stalled, W=watchdog counter; failed runs are partial evidence.
- 01 berlin-konradshohe: (4/4;5/5) -> (4/4;5/5), C.
- 02 berlin-spandau: (4/4;5/5) -> (4/4;5/5), C.
- 03 berlin-tegel: (4/4;5/5) -> (1/1;1/1), S at first terrain action.
- 04 gen27-konradshohe-s0-grow: (25/25;25/25) -> (26/26;26/26), X at shutdown; no acceptance credit.
- 05 gen27-spandau-s1-grow: (25/25;24/25) -> (25/25;24/25), C, 49 terrain errors (NM18).
- 06 gen27-tegel-s1-grow: (25/25;24/25) -> (25/25;24/25), C, 17 terrain errors before boundary fix.
- 07 maxisland-bevopolis: (0/3;4/4) -> (0/3;4/4), C/W1; prebuilt envelope (NO46).
- 08 maxisland-letroy: (3/3;1/4) -> (3/3;4/4), C/W1; lot choices now exact.
- 09 maxisland-north-weburg: (2/3;3/4) -> (2/3;3/4), C/W2; tunnel envelope/110 terrain errors (NO46/NM18).
- 10 lua-lots-empty: (2/3;3/3) -> (0/1;1/1), S: calendar frozen 601 s; empty load sample (NO47), later samples not reached.
- 11 lua-lots-grown: (0/3;1/3) -> (0/3;3/3), C; all netmgr choices exact, prebuilt parse errors retained.
- 12 lua-lots-grown-qualified: (0/3;1/3) -> (0/4;4/4), C; all netmgr choices exact, prebuilt parse errors retained.
- Final DLL 94af2e73, netocc-cities29-bound8-grow.log: C/ref0, netocc 25/25, netmgr 24/25, terrain errors 17 -> 12; every remaining listed removal placed before/after handler=1/0.
- Final netocc-cities29-bound8-default.log: C/ref0, netocc 2/2, netmgr 2/3; terrain errors 16 -> 10, remaining listed removals 1/0. These are true handler-boundary differences, still NM18.
- Final house netocc-cities29-bound8-house.log: C/ref0/watchdog0, netocc 4/4 and netmgr 5/5 exact, end date reached. Final logs share the main game-results root above.
- NO46 (prebuilt/bridge/tunnel envelopes) and NO47 (zero-record census accounting) filed in docs/specs/questions/network-occupants.md; no codec, empty sample or class has been exempted.
- NM18 (handler removals) and NM19 (Port edge lot/random input) filed in docs/specs/questions/network-manager.md. Port census netmgr 1/2 -> latest 1/1 load only; profile5-paradis stalled at 2040-12-12 terrain action before the differing month; probe4-paradis was corrupt during load.
- Ready/flip criteria unmet: prebuilt codecs lack a stream contract, real terrain removals differ, Port cannot reach its census mismatch, and the 12-scenario sample is neither all clean nor exact. WIP queue only; ready enqueue withheld.
- Next: resolve NO46/NO47/NM18/NM19, re-run failed scenarios with the final DLL, then repeat the broad qualification and enqueue kind engine. No coverage/regress lists changed.
