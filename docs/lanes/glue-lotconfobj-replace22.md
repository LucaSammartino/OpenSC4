| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lotconfobj-replace22: Rust-backed lot configurations | Codex / `glue-lotconfobj-replace22` (engine lane, detached HEAD) | in progress; native build 279/279 and CTest 94/94 PASS; formatting PASS; workspace gate and game pending | Collect existing operations below; finish corrected house run and gate before enqueueing. |

## Handoff
- Code milestone: `c4f269d06b46580b7e9287d41445583ab6792126`; this final amend adds only this report.
- Rust `LotConfiguration` owns loaded fields/geometry; opaque mutex-backed C ABI owns, loads, snapshots and frees it.
- Manager allocation uses one shared FreeCity4 factory, never GZCOM for configuration objects (spec 1.1).
- C++ Load projects Rust geometry into game-shaped mutable arrays; removed its duplicate geometry parser.
- Added retained-exemplar/reload Rust test and plugin registration/reference ownership test, seam 75.
- Four C ABI exports added; ABI version unchanged. Shadows remain default-off.
- Native verify terminal PASS: `qbuild-20261025-043648-build-ctest`, build 279/279, CTest 94/94 (ownership test 94 passed).
- Logs: `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`; build 221.0 s, CTest 488.5 s.
- Full gate already running: unified exec session **9553**, `qbuild.py --wt <WT> all`; collect THIS operation, do not duplicate it.
- Focused Cargo was stopped to reduce contention, terminal exit 15; preserved `build-runs/focused-lotconf.log`. Full gate includes that test.
- Game request outstanding: **lc22-r1-house-20261025a**, turbo to 2032, lotconf_shadow=1; event-driven wait session **82573**.
- Collect that SAME wait. Completed verdict remains valid; helper `build-runs/wait-lotconf-verdict.ps1` blocks on filesystem events without qwait.
- IMPORTANT: request a omitted flora, lotdev, commercial, industrial. Preserve its result, then submit unique label b adding those four to its enable list.
- The request otherwise includes all class replacement keys plus span565/span4444/span1555; canonical tools profile was not read (clean-room prohibition).
- Need corrected full replacement run clean through 2032, ref_warnings=0, exact lotconf line count; normal-speed rerun only if turbo differs.
- No game verdict or exact count collected yet. No merge-queue entry written. No run records, tools edits, or spec edits.
- Known gap: legacy version-1 tags use deterministic process serial without GZCOM clock seed (`SPEC-GAP` comment); shipped version-2 geometry measured next.
- Next: collect sessions 9553 and 82573, run corrected request, report actual counts, amend this <=25-line file and enqueue full 40-character tip when ready.
