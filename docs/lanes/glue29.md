# Lane glue29

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue29 (the cSC4NetworkTool replacement glue over cores27's fc4_nettool ABI) | `glue29` / `.claude/worktrees/glue29` (branch `glue29`, engine lane) | in progress | (1) build the plugin glue; (2) register `nettool` in FC4Subsystems.cpp; (3) in-game runs; (4) gate |

## Handoff

- **Goal:** wire the network tools replacement: a `cFC4NetworkTool` plugin class over the `fc4_nettool_*` C ABI
  (cores27), the host callbacks bound to the geometry core (`fc4_netgeom`), the occupant manager and the network
  manager exactly as `engine/core/src/network_tools/host.rs` names them; enable key `nettool`; in-game checks.
- **Done so far:**
  - Read cores27's lane report, `network-tools.md` §1-§9/§11-§13, `network-geometry.md` §1/§3/§5/§10, the
    `fc4_nettool` section of `engine/capi/include/fc4.h` (33 callbacks, 13 entry points), `fc4_netgeom` exports,
    and glue24's `FC4NetMgrHost.cpp` / `cFC4NetworkManager.cpp` / `cFC4NetworkTool.cpp` owner-block pattern.
  - Baseline `qbuild quick` PASS (no Rust touched; CMake build 174/174 edges, 84 s).
- **Next:** write `engine/plugin/FC4NetToolHost.{h,cpp}` (host table, owner block with the context at offset 0),
  rewrite `engine/plugin/cFC4NetworkTool.{h,cpp}` as the real replacement delegating to `fc4_nettool_*`, register
  in `FC4Subsystems.cpp` under `nettool`, then the in-game runs.
- **Facts for the successor:**
  - The worktree is cut from main at `a2e6fdf5` — main does NOT yet carry cores27 (`fc4_nettool_*` lives on
    `integration/10`, commits `81efb249`/`dc5a14f8`). The glue needs the capi exports, so rebase the worktree
    branch onto `integration/10`'s engine files or wait for the train to land before the first `quick` that
    touches capi; as of now the baseline build passes because nothing touched capi yet.
  - Base tool state: `ToolState::new(type_id)` defaults (`stop_on_failure` 1, `data4` 1, `preview_update` 1,
    layer mask `0x1F4F` surface / own bit for 4/5/7).
  - The script action `network ax az bx bz type` drives tool slot 7 through
    `GetNetworkTool(type, true)` — that is the scripted placement path for the in-game check.
