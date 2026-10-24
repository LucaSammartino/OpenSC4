| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lots-lr15apply12 (LR15 scoped owner inventories) | `Codex` / `glue-lots-lr15apply12` (engine lane) | in progress | Implement LR15 inventories, gate and measure house city to 2032. |

## Handoff
Goal: apply LR15 business-unit, base-texture and retaining-wall scoped inventories and extend default-off lotrec_shadow.
Done: 85f3cc6c3 initialized lane; baseline a150cd2db already contains LR15 and glue-lots-shadow12.
Done: implemented three default-off scoped shadows, isolated owner codecs and inventory/list validation tests; focused lotrec_lr15 4/4 PASS; fmt 3 files PASS.
Next: run qbuild all on stable source and collect terminal result; native gate uses qbuild verify coordinator queue; then house-city run to 2032.
Facts: no game requests outstanding; ABI exports unchanged (private static-link transport only).
Facts: Bash q wrapper failed CreateFileMapping before Cargo; direct PowerShell cargo test succeeded (build-runs/lr15-focused.log). Initial relative fmt paths failed; absolute paths passed.
Facts: no new questions; LR15 specifies master-rooted units and full-grid registered occupants, not all allocations. Read effects stay mismatches.
Facts: main is C:/Users/tlori/FreeCity4; manifest build-runs/run4-glue-lots-lr15apply12-r1-manifest.json in main.
