| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| luacmd27 - Maxis Lua scenario tooling and catalog sweeps (tools) | Codex / .claude/worktrees/luacmd27 / detached HEAD, reserved luacmd27; round 4 main merge in progress; ABI unchanged | wip final round: round-three evidence below retained; fresh gate and reach pending; questions none | 1. Qualify grown/coastal footprints. 2. Compare Lua and no-Lua reach by address. 3. Gate once and enqueue tools at actual HEAD. |

- Round 4 in flight: main merged before further work; no engine implementation changes planned. Keep rejected lot calls and unqualified commands visible.

- AC launch/control: round-two Lua print, rename/read-back and save proof remains accepted; extension control accepted by coordinator. No more extension control or GetSimulationDate work.
- AC suite: .lua rows emit lua_script plus opt-in lua_extensions; optional same-name dated .script handles terrain. Harvested Lua commands/completion are required; missing, unfinished or rejected commands cannot pass the suite.
- AC catalog: python tools/lotconfigs.py --output build-runs/lotconfigs-r3.tsv printed 1909 lots, 22 packages, 0 warnings. python tools/lot_sweep.py [--size 128] printed deterministic 175-lot Lua tables; real-catalog tests verify bounds/disjoint footprints at 128 and 256 cells.
- AC empty: luacmd27-empty-r3e, Berlin / New City (30), slot 11, DLL 11f57a7c; plugin terrain companion and three read-backs verified 280 m before placing; 175 calls, 161 status-0 / 14 status-2; 2000-01-01 to 2000-03-01.
- AC grown: luacmd27-grown-r3e, Maxisland / West Weburg, slot 12, same DLL; measured size 128x128; all 175 calls completed, 61 status-0 / 114 status-2; 2197-03-20 to 2197-05-20. New power/fire/health placements still all rejected.
- Category accepted/failed, empty then grown: power 8/0,0/8; water 4/0,3/1; garbage 3/0,1/2; education 13/0,8/5; health 3/0,0/3; police 6/0,2/4; fire 3/0,0/3.
- Category accepted/failed, empty then grown: parks 24/1,15/10; landmarks 39/1,7/33; rewards 51/2,23/30; transit 17/10,7/20; utilities union 13/0,4/9. Categories overlap; statuses are not per-lot persistence claims.
- AC demolition: None, Sim, Explode returned 0; empty newly placed school/jail/prison removals returned cost payloads 3220/230/19500. Both scripts printed scenario-complete, no pending calls.
- Game verdicts: both plugin clean, but watchdog samples were 1 empty (recovered after 8.125 s) and 3 grown; foreign exceptions/RTC/overruns/ref warnings all 0. Preserve these diagnostic limits.
- Shadows: final empty 370/383 exact, grown 373/444; combined 743/827. Suite failed-action counts 15/115 include rejected lot calls plus a rejected landfill zone in each. No compare/replace/reach credit.
- Baselines without Lua: empty-base clean 77/77, stayed god mode at 2000-01-01; grown-base clean 639/677, ended 2197-07-10. These differ in dates/mayor state; deltas are whole-run observations, not category attribution or regression proof.
- Changed-key examples: empty power 0->3, plumbing/civic/parks 0->1, lotcfg 5->21, terrain 9->27; grown commercial 0->14, pollution 11->15. All keys and first mismatch lines: build-runs/suite/luacmd27-r3-final/{per_key,shadow-delta}.tsv.
- First mismatch examples: empty aura line 3398, dbpf 3488, traffic 3531, occmgr 3912; grown nettool_plan 2665, pollution 3110, landvalue 3531, terrain 3913. docs/ops/lua-commands.md lists every nonexact key/first line. Findings only; no engine fixes.
- Calibration: PlaceLot numeric IID, cell x/z, facing 0, false,false; x=128/256/512 rejected on a 128-cell city. SetCellAltitude returned 0 without measured effect; use the verified dated terrain action. Reward cheat probe returned 2; final direct reward lots need no successful cheat.
- Timing correction: auto-yield only around waits; bounded construction batches; grown city resumes for frame waits after paused r3d stalled at 24 returned lot calls. r3a/r3b/r3c/r3d failed/incomplete/corrected-source evidence remains distinct.
- Python gate: python -m pytest tools/tests -q --tb=short printed 384 passed in 92.70s (earlier full gates 382 and 383 passed). Final focused suite/plan test printed 12 passed.
- Once-only qbuild.py --wt <WT> all: Cargo 7844 passed, clippy 0 diagnostics, native 47/47 edges, CTest 117/117, qbuild PASS. Only engine test data/helpers changed; no engine implementation, exports or ABI version changed.
- Real suite dry run: tools/suite.py run --set full --scenario lua-lots-empty --scenario lua-lots-grown --profile shadows --dll engine/build-x86/FreeCity4.dll --tag luacmd27-final-dry-r3 --dry-run printed prepared 2 requests (not submitted); actual games used WT launcher directly on reserved 11/12.
- Source saves unchanged: final input manifests verify matching before/after SHA-256 for both Documents originals. Grown saved lot stream 1114023->1070766 bytes; empty saved alias unresolved, no independent save credit; no .sc4 committed.
- Evidence: WT build-runs/luacmd27-{empty,grown}-r3e{,.lua}.log, *-inputs.json and *-console.txt; build-runs/suite/luacmd27-r3-final/{summary,per_key,categories,shadow-delta}.tsv and JSON. Lua logs also harvested in main build-runs/game-results/.
- Missing: successful grown power/fire/health, coastal pieces and several airport stages; landfill PlaceZone(14) returned 2. Older broad UI/network/region candidates remain unregistered/unqualified. Questions none.
- Next: enqueue these tools; qualify more suitable grown/coastal footprints in a later scenario round, keeping rejected calls visible.
