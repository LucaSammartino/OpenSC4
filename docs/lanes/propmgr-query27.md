| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| propmgr-query27: closed lot query bounds (engine) | Codex; .claude/worktrees/propmgr-query27; reserved branch propmgr-query27, detached HEAD | WIP r3; main merged 4eb329402; PL24 fix/tests 23e3dbdc4; ABI/seam unchanged; corrected quick core 5192/0; native quick build pending | 1. Finish current quick build. 2. Four full house shadows to 2032, at most three in flight. 3. Compare train260 keys and May 2029. 4. qbuild all, finalize this handoff, enqueue actual HEAD. |

- Contract: read corrected A7.2, T.4 and archived PL24 answer; both upper bounds are closed, unordered X/Z or bounds reject, Y ignored, no kind-0 exception.
- Change: four ordered binary32 comparisons, in lower-X/lower-Z/upper-X/upper-Z order; spatial discovery, caller-list ordering and releases retained.
- Fixtures: observed kind-0/flags-7 bounds; all ten T.4 rows individually and together, caller retention, four NaN bounds, NaN Y, signed zero, infinities and reference counts.
- Pre-fix direct focused test deliberately failed 0/3; corrected x86 quick tests pass all three PL24 tests and 5192 core tests (`build-runs/fc4verify-quick-test.log`).
- Owned-file fmt: 2 files, zero diff lines. Quick native build is still pending; no current game results or full gate are claimed.
- R3 target: `propmgr-query27-r3-closed-a1` through a4, shared queue, parallel=1, at most three in flight, house train260 profile, turbo to 2032.
- Baseline: main `build-runs/regress-train260-shadows.log`; propmgr 220/220, lotcfg 293/293; May 2029 has mismatches=0/calls=160/cells=32/lotCells=20/props=268.
- Causal evidence: the verified prior a2 native-only kind-0 prop at exact Z=768 was rejected by the old strict bound; PL24 now explicitly requires its retention. Exact runs alone do not prove all intermittent failures resolved.
- No new spec question: PL24 specifies all behavior changed here. Existing diagnostics retained; no native vtable patch, replay, output import or tolerance.
- Next: finish quick, collect and compare four runs, run qbuild all once, fill measured acceptance values here and enqueue kind engine with actual detached HEAD.
