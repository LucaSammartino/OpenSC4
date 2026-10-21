| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netmgr-contracts4 | workflow / `re-netmgr-contracts4` | **wip** (specs only, no ABI rows, no gate needed for docs; no game run): NM10, NM11 answered **DIFFERS** in network-manager.md §6.2, §6.3 (new), §8.5, quirk 15, tests 23-24 | implementer applies §6.3 to the netmgr core/shadow, then a verify game run confirms 20,103 / 16,93 match (required before ready) |

## Report

**What changed** (docs only): `docs/specs/network-manager.md`
- §6.2 lot-state row now points at the §6.3 walk instead of "the rect's border".
- §6.3 (new): the `0xE8BDA2E7` handler's iterator is built by `0x0060C930`, which sets the current cell to (0, 0);
  `0x0060C960` sets only the rect (and widens x1/z1 to x0/z0). `GetNext` `0x0060CEE0` therefore walks row 0 from
  x = 0 to x1, then (x0, z) and (x1, z) for z = 1..z1-1, then the full row z1. The zoning handler instead sets its
  start to (x0, z0) at `0x0060FD9B`. Consequence: the top border's inner cells are never queued (NM10), and every
  lot-state message queues columns x0 and x1 from row 1 down to the lot plus row 0 up to x1 (NM11).
- §8.5: table of every path into `pending` / `UpdateRoadsInCell` / `UpdateLot` / lot create/remove, from the call
  graph (no data references to any of them): pending inserts only at `0x0060FE45` (zoning) and `0x0060FFBF` (lot
  state); `UpdateRoadsInCell` only from `ProcessUpdateList` and usable changed; `UpdateLot` also from the avenue
  flood `0x0060F424`. Traffic `DoRoadChanged` inserts into its own set (`+0x7B8`); `RedevelopLot` only rebuilds objects.
- Quirk 15; test 23 rewritten (61 keys for the 2 x 3 lot), test 24 added (lot at row 0).
- NM10, NM11 moved to questions/answered/network-manager.md; QUESTIONS.md regenerated.

**Runs explained** (by rule, not traced): original-only 20,103 (2026-04) and ours-only 16,93 (2026-05) are cells
visited by another lot-state message whose grown rect has x0 or x1 = 20 (resp. 16) and z1 > 103 (resp. 93);
the original re-ran `UpdateRoadsInCell` there, ours (border only) did not. Which exact lot produced each visit is
not proven here: no run trace of the message rects was read.

**Not done**: no game run (docs-only lane, no shadow change). The brief requires a meaningful game trace before
ready, so the lane is wip until an implementer lane applies §6.3 and a verify run shows 20,103 / 16,93 exact.
REA was not needed (export only).

## Coordinator disposition
Supported public NM10/NM11 contracts are ready for docs train. The coordinator prompt incorrectly appended engine gate/game requirements to this RE-only lane. No engine validation is claimed or required for this docs result; the hypothesized runtime mismatch cause remains unverified until a dependent engine lane follows the landed contract and measures it. No netmgr verification credit from this RE result.
