| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| zones2 (zone developer / lot manager) | `a04b0cfb664b373d3` / `agent-a04b0cfb664b373d3` | Reserved ABI 54, seam op 53. **Stage A merged `9a9b0df`** (ABI 51; regress zones2 run 112 PASS); **Stage B merged `3cb9b43`** (ABI 54, taken as 55; seam op 53; gate zones2b: cargo 2026/0, clippy clean, ctest 55/55; regress zones2b run 117 PASS): game lots are created from the game's lot pool and constructor (zones.md §8.5.1; direct calls `0x006BF430`, `0x006C1E50`), slot 21 CanCreatePloppedLot (§8.4.1-§8.4.3), UpdateZoneType (§7.2.2), the §5.2.1-§5.2.2 street joins (network tool slot 7) and the dezone (the zone manager's PlaceZone); `enable = zonedev` clean for a turbo year (`zonedev-enable-final`: 68 lots built, 0 refused, ref_warnings=0); `zonedev_shadow` exact (70/70 in `zonedev-shadow-final4`); ZG34 does not reproduce | Next: (1) a scripted run that zones a rect, dezones and plops a lot (the dezone, the joins and slot 21 have not run in game; mind apply10b's note that `zone 128 10 130 12 14` crashes PlaceZone on Konradshohe); (2) apply ZG22 (slot 19 always writes `*ok`) and ZG24 (the zone tool owns the highlight region; null with a zone tool present) in a small fix lane; (3) add `zonedev` to the regress replace list (clean in the lane's enable runs); (4) optional: a slot-21 compare in `FC4ZoneDevShadow.cpp`. Questions ZG35-ZG40 filed; ZG26, ZG30-ZG33 still need coordinator decisions |

## Retired roster row

Kept verbatim from the HANDOFF §0 `| Lane | Agent ID | Worktree | Reserved | State |` table (2026-10-14 wind-down); the generator does not read this section.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| zones2: zone developer / lot manager replacement (`enable = zonedev`) and shadow (`zonedev_shadow = 1`); lot creation waits for re-specs11 | `a04b0cfb664b373d3` | worktree | ABI 54, seam op 53 | Stage A merged `9a9b0df` (ABI 51); Stage B merged `3cb9b43` (ABI 55); ZG22-ZG40 |
