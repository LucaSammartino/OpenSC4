# Lane glue29

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue29 (the cSC4NetworkTool replacement glue over cores27's fc4_nettool ABI) | `glue29` / `.claude/worktrees/glue29` (branch `glue29`, engine lane) | in progress | (1) full gate (`qbuild all`); (2) in-game runs: `enable = nettool` + scripted `network` actions, full run to 2032; (3) queue file + report |

## Handoff

- **Goal:** wire the network tools replacement: a `cFC4NetworkTool` plugin class over the `fc4_nettool_*` C ABI
  (cores27), the host callbacks bound to the geometry core (`fc4_netgeom`), the occupant manager and the network
  manager; enable key `nettool`; in-game checks.
- **Done so far:**
  - Rebased the worktree onto main (cores27/train10 landed; `fc4_nettool_*` is now on main at ABI 75).
  - `engine/plugin/FC4NetToolHost.{h,cpp}` (commit `ed77c929`): the 36-callback `fc4_nettool_host` over the game's
    interfaces — owner block `NetToolContext` at offset 0 (city, geom, msgServer, self, held refs, counters);
    draw/solve replay through `fc4_netgeom`; §3.3 probes/§6 inserts/§10 destroy through the occupant manager and
    the city's network manager; §11 refit through the extended underground interface (`cFC4UndergroundOccupant`,
    vtable `0x00AAAD40`, slots 50/84/86/94/95).
  - `engine/plugin/cFC4NetworkTool.{h,cpp}`: the replacement tool over the SDK vtable (44 virtuals, order
    matches interfaces.tsv 0x00AA8748), delegating to `fc4_nettool_*`; plus `NetToolRegistry` — with
    `enable = nettool` the plugin's own callers (scripted `network` action) take replacement tools instead of
    the manager's factory (the game's tool classes have no GZCOM IDs — question NM11, so no vtable hooking).
  - Registered additively in `FC4Subsystems.cpp` (key `nettool`, synthetic CLSID `0x4E543030`), wired into
    `FC4Script.cpp` (RunNetwork: registry tool if enabled, else original path) and `FC4Observer.cpp`
    (registry summary + CityShutdown at city shutdown); listed in `CMakeLists.txt`.
  - Gate: `qbuild quick` PASS (build 6/6 edges).
- **Next:** `qbuild all` (full gate); then in-game: `enable = nettool`, script with `network ax az bx bz 0`
  actions (type 0 road), verify placement validates/commits through our core (`nettool ... inserts=N` log lines,
  `0x99EF1142` data4=3 messages), full run to 2032, ref_warnings=0; runs numbered from 198.
- **Facts for the successor:**
  - main does NOT carry glue24's `cFC4NetworkManager` (still wip on branch glue24) — the tool replacement
    therefore rides the ORIGINAL manager: our registry is reached only from our own plugin callers, not from
    the game's UI drag path. The scripted `network` action and zonedev street tools are the real test paths.
  - `OccupantExists` (slot 9): the core answers it from its world cache itself (host callback NULL).
  - The per-cell piece choice (network-rules.md §11) is NOT wired: every solved cell inserts one ordinary
    network occupant via GZCOM class `0xC9C05C6E` with piece id 0 — documented [choice] in the host source.
  - The underground exemplar depth fields (0xA96E5641 etc.) are passed as zeros for now — Init reads city size
    only; fine for surface tools (road/power), the pipe refit needs a follow-up.
  - `IntersectionStructure` flood and `NeighboursByNetworkType` use slot-32 queries + flags; capped arrays,
    tokens held in ctx until NetToolReleaseHeld.
