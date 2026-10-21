| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrids4: layout-compatible cSC4SimGrid<T> family (enable=simgrids) | glue-simgrids4 / .claude/worktrees/glue-simgrids4 | in progress: run4 validation (joint 36+simgrids run, original-factory reload) | see Handoff |

## Report

What changed (no capi / C ABI change, no FC4_ABI_VERSION bump):
- `engine/plugin/FC4SimGridLogic.h`: `cFC4SimGridT<T, Alloc, Host>` for the ten element types (simgrid.md
  §2-§6): data kept as the STLport `vector<vector<T>>` (12-byte {begin,end,cap}, rows by x), every buffer from
  the allocator policy (in the game: allocator B, FC4GameList.h `StlNodeAllocator`, the one the original's
  default-allocator vectors use); count 0 on creation, Release at count <= 1 destroys; slots 0-27 per §4
  (§4.1 averages incl. float truncation quirk and _ftol conversion, §4.2 resize keeping values), virtual
  destructor as slot 28; the v1 record (§5; Read takes every field as stored, false on stream errors or counts
  > 4096 [choice]); a trace (read/write lines, raw-change detection by checksum).
- `engine/plugin/cFC4SimGrid.{h,cpp}`: factories for 0x49B9E601..0x49B9E60B (bool -> uint8 grid), city via
  cISC4App::GetCity (FC4_ACQUIRED/FC4_RELEASE on the app), refused without allocator B.
- `FC4Subsystems.cpp`: `kSimGridClasses`, subsystem "simgrids", off by default. CMake: sources + `simgrid_test`.
- `engine/README.md`: switch entry. `docs/specs/questions/simgrid.md`: SG1 (structural use of the raw vectors
  by original code; out-of-range indices: ours return 0 / ignore writes [choice]).

Measured:
- Gate: cargo test 3652 passed, clippy 0, ctest 63/63 (baseline 62/62 + simgrid_test).
- Game simgrids4-a1 (turbo to 2026-07-01, save 2026-04-01): clean, ref_warnings=0; 136 grids created by the
  class store, read 136 (0 failures), written 136, released 136; grid_data_calls=3142408, iface_writes=314134,
  raw_changed=50 (grids whose values the original changed only through GetGridData).
- Game simgrids4-a2 (turbo to 2032-01-01, save 2031-12-01): clean, ref_warnings=0, date 2032-01-13; 136/136/136,
  grid_data_calls=42228312, iface_writes=4070298, raw_changed=55, alloc_failures=0.
- No shadow (none exists for grids); not run at normal speed (no turbo-only difference to chase).

Next: coordinator union run (enable=simgrids with the grid-owning replacements: their GetClassObject grids then
come from these factories); a save written with simgrids reloaded by the original (format is byte-identical by
test, not yet by a game round-trip); answer SG1.

## Handoff
- goal (run4): (1) joint run, evidence-run4/run4-simgrids-joint.ini (36 replacements + simgrids), turbo to 2032, clean/ref0, 136 grid traces + save; (2) original grid factory (simgrids off, other 36 on) reloads a city saved by ours, with per-grid record digests.
- done: merged main (17cc9565).
- next: add record digest (FNV-1a 64 over the v1 record bytes) to our read/write trace; add `simgrids_probe = 1` (tooling): grids read through IStreamAdapter::ReadRef are held and at PostCityInit each one's own Write is captured (WriteCapture) and its digest logged; then gate, joint run sg4-j1, capture slot city after save, reload run sg4-r1 via FC4 Test alias City - FC4SGReload-run4-<label>.sc4.
- facts: slot city mirrored before each run, not restored after; copy it during own run after script save. Never overwrite Konradshohe.
