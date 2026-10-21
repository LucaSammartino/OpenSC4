| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-simgrids4: layout-compatible cSC4SimGrid<T> family (enable=simgrids) | glue-simgrids4 / .claude/worktrees/glue-simgrids4 | in progress: round3 merged main f45e6b91 (automata fix aeaca0e2 in); gate + joint sg4-j2 (37 current + simgrids) pending | read sg4-j2 verdict; reload proof sg4-r1 (116/116) stands, grid code unchanged since |

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

## Run4 validation (2026-10-21)

Changes (tooling only, no behaviour change, no C ABI change): merged main (17cc9565); read/write trace lines carry
`digest=` (FNV-1a 64 of the v1 record bytes, `RecordDigest`, unit-tested against the written bytes) and `nonzero=`;
`simgrids_probe = 1` (`engine/plugin/FC4SimGridProbe.*`, hooks in IStreamAdapter::ReadRef and the observer's
PostCityInit, known key in FC4Config.cpp) logs the digest of each save-read grid's own Write (7edf7923, f4d3f703).
Gate: cargo test 3676 passed, clippy 0, ctest 63/63.

Game runs (evidence copied to build-runs/evidence-run4/):
- sg4-j1 joint (evidence-run4/run4-simgrids-joint.ini = 36 replacements + simgrids, turbo to 2032-01-01, script save
  2031-06-01, probe): verdict clean, date 2032-01-01, BUT ref_warnings=238. Grids: 136 read (0 failures), 136 written
  at the save (+117 probe captures), created=248 destroyed=248, alloc_failures 0. Saved slot city captured during
  the run after SaveCity returned: sg4-j1-saved.sc4, 1314158 B, sha256 df7a9ca2f03a5d18...
- sg4-r1 reload by the ORIGINAL grid class (same 36, simgrids off, probe; city = alias City - FC4SGReload-run4-r1.sc4
  in FC4 Test, removed once the run started; Konradshohe untouched; LoadCity on the slot1 alias -> true; DIGESTs from
  2031-07-01): clean, ref_warnings=0. Probe: 116 grids from the save, all captured, grid class "the game's";
  116/116 record digests equal the digests our class wrote at the save (0 diff), 98 non-blank (77566 nonzero cells):
  persisted grids, not regenerated blanks. The other 20 saved grids do not pass through our ReadRef (not probed).
- sg4-c1 control = sg4-j1 without simgrids: the same ref_warnings=238, the same 100 lines `released last reference to
  game object [class 0x4A1628C5 x92, 0x6A1628BA x8] at HostRelease:53` (cFC4Automata.cpp, city shutdown).
  So the joint run's warnings belong to the automata replacement in the 36 profile, not to simgrids.

Not ready: the joint ref0 requirement fails for a reason outside this lane. Next: automata lane fixes the shutdown
release in cFC4Automata.cpp HostRelease; rerun sg4-j1's request (game-queue/done/sg4-j1.req); SG1 still open.
