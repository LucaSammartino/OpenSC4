| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-occprops4 (lot-growth occupant records: building, base texture, retaining wall, foundation, bare occupant) | `core-occprops4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: lot-records §5-§8, §12 save records in Rust core (engine/core/src/lot_occupants/), reusing automata::OccupantBase. No capi/plugin. game=false.
- Done: lane file.
- Next: write module + tests, qbuild quick, then all.
- Facts: not covered by core before this lane: lot 0xC9BD5D4A, business unit, building/base texture/retaining wall/foundation/construction/signpost/text decal records.
