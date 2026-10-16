| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| lotglue (lot developer) | `ab9c3d71e1a39f129` / `agent-ab9c3d71e1a39f129` | Reserved ABI 53, seam op 52. **merged `5212df2`** (ABI 54, taken as 53; seam op 52; gate lotglue: cargo 2012/0, clippy clean, ctest 55/55; regress lotglue run 116 PASS): `cFC4LotDeveloper` (`enable = lotdev`, off by default), `FC4LotDevHost` with nine switches (219 switches in all); replacement clean to 2032 alone (`lotglue-final-ld-4`, ref_warnings=0, 203 started, 402 abandoned) and with `development`; `lotdev_shadow = 1` clean, every line `roundtrip=same`, the only differences slots 25 / 26 flatness on bit-identical slot 27 ranges 1.0355-1.68 (LD29); one stall under machine load (`-ld-3`, not lotdev; rerun clean) | Next: (1) an RE lane answers LD29 (what slot 25 compares), then change only `LotDeveloper::is_area_flat` (core `lot_developer/terrain.rs`) with a §16 vector and rerun `lotdev_shadow = 1` to 2032 for mismatches=0; (2) LD27 (slot 22's raw AL) if callers use the byte; (3) LD12-LD26, LD30 need spec answers (retaining walls, burnt / rubble props, network nodes, UpdateLotObjectAlitudes, business unit creation); (4) a run that changes the visual time (or with `clock24`) to exercise the night batches and the shadow's night compare (LD28); (5) add `lotdev` to the regress replace list (clean in the lane's runs) and `lotdev_shadow` to the shadows list once LD29 makes it exact. Questions LD2-LD30 filed |

## Retired roster row

Kept verbatim from the HANDOFF §0 `| Lane | Agent ID | Worktree | Reserved | State |` table (2026-10-14 wind-down); the generator does not read this section.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| lotglue: lot developer replacement (`enable = lotdev`) and shadow (`lotdev_shadow = 1`); Q4 slots wait for re-specs11 | `ab9c3d71e1a39f129` | worktree | ABI 53, seam op 52 | merged `5212df2` (ABI 54); LD2-LD30 |
