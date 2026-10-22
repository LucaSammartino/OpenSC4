| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lighting4: lighting owners | Codex / `core-lighting4` (engine lane) | in progress; baseline gate running | Implement scoped public contracts, test and full gate. |

## Handoff
- Goal: pure Rust lighting module for publicly specified cSC4LightingManager and cImageARGB8 owners; no ABI/plugin edits.
- Reserved detached worktree reused; clean at start, no commits beyond main.
- Baseline all: workspace tests/clippy passed; native build passed; ctest 67/67 passed (build-runs logs).
- 7ad0ac97: image, persistent properties, lifecycle/time, color paths, shadow sweep/messages and 18 tests.
- Initial quick: 2324 passed/1 failed (sweep accumulated f32 error 0.000022 vs tolerance 0.000020); tolerance corrected to 0.000050.
- Added all nine spec numerical scenarios plus ownership/image/quirk/failure tests; rerun after formatting.
- Quick PASS: core 2334/2334 (18 lighting), dependents compiled, native build 4/4 (141.5s).
- Added drift in both directions, all rotated grids, deferred dirty messages, render settings, terrain/vertex color rules, decal overrides and bounds/failure cases.
- Literal review correction: terrain vertex normals bend by grid depth even at zero; point color tests retain positive-depth checks. Terrain color lowers directly to stored land height.
- 4f7dc86f: 27 lighting tests. Full workspace tests PASS (core 2343); first final clippy found Rust-1.75 MSRV and test precedence errors, both corrected. Native portion pending.
- New-module fmt PASS (8 files); lib.rs formatter diffs are inherited module ordering/line wrapping, plus additive registration.
- Final all after clippy fixes PASS: Rust 4200/4200, core 2343/2343 (27 lighting), clippy 0 diagnostics, native build 4/4, ctest 67/67 (153.0s native).
- Lifecycle review: deleting destructor now consumes Box and retains destroyed storage iff bit 0 is clear; added both-bit ownership tests (28 lighting total).
- Next: rerun all/fmt for this final lifecycle change, then owner-by-owner final report and WIP queue (contract gaps remain).
- Formatting: new files formatted; qbuild fmt with absolute paths detects inherited lib.rs ordering/line-wrap diffs (no unrelated reformat). q.py Bash is denied by environment; native PowerShell rustfmt works.
- Clean room: only allowed docs and engine source; no tools scripts or analysis sources.
- Filed LT5-LT11 in docs/specs/questions/lighting.md. Projection and terrain rectangle are explicit ContractGap; upper terrain clamp and rectangular sweep also unresolved.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-lighting4-r1-manifest.json.
- No game requests; core-only task.
