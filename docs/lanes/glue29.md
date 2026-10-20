# Lane glue29

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue29 (the cSC4NetworkTool replacement glue over cores27's fc4_nettool ABI) | `glue29` / `.claude/worktrees/glue29` (branch `glue29`, engine lane) | in progress (round 3: rebased onto glue24's main; composed) | (1) fix the slot-10 ABI hazard (declare the as-compiled tool interface, cFC4ISC4NetworkToolFull, replacing the SDK base); (2) `qbuild all`; (3) in-game: `enable = netmgr,nettool` + scripted `network` actions to 2032; (4) queue file + report |

## Handoff

- **Goal:** wire the network tools replacement: a `cFC4NetworkTool` plugin class over the `fc4_nettool_*` C ABI
  (cores27), the host callbacks bound to the geometry core (`fc4_netgeom`), the occupant manager and the network
  manager; enable key `nettool`; in-game checks.
- **State at this stop (round 3, context budget OVER):**
  - Round 2's skeleton landed on the branch: `FC4NetToolHost.{h,cpp}` (the 36-callback fc4_nettool_host over
    the game's interfaces; NetToolContext at offset 0), `cFC4NetworkTool.{h,cpp}` (the tool over the SDK
    vtable + `NetToolRegistry`), `FC4Subsystems.cpp` key `nettool` (CLSID 0x4E543030), `FC4Script.cpp`
    RunNetwork registry path, `FC4Observer.cpp` city-shutdown release.
  - REBASED onto main at `66a309b8` (train12 landed glue24: the replacement network manager `enable = netmgr`,
    the lot manager `netlot`, and a do-nothing `cFC4NetworkTool` stand-in under the SAME filename).
    Conflicts resolved: this lane's real tool kept; glue24's stand-in semantics preserved inside it as a
    `live_` flag. Commit `4883aa89`.
  - COMPOSITION (4883aa89): `cFC4NetworkTool(uint32_t type, bool live)` - live = the fc4_nettool core,
    !live = glue24's verified no-ops (Init/Shutdown true, placements fail, getters idle). glue24's
    `cFC4NetworkManager.cpp CreateTool` now creates `new cFC4NetworkTool(type, NetToolRegistry::Enabled())`:
    with `enable = netmgr,nettool` the manager's own 13 cached tools are the REAL replacement and slot 5
    hands them to the game's callers (zonedev takes 3/11/0 at PostCityInit; the power simulator the type-5
    shared tool). With nettool off the behaviour is byte-for-byte glue24's. NetToolRegistry::Take still
    creates live tools (key-gated); FC4Script RunNetwork prefers the registry when enabled.
  - `qbuild quick` had passed pre-rebase; NOT yet re-run after the rebase.
- **Next (exact steps):**
  1. FIX the slot-10 ABI hazard first: the SDK `cISC4NetworkTool` declares `UpdateOnZoneChange()` pop 0 but
     the game pops 20 (SC4Rect by value + u32; foundations.md errata ~line 2344; interfaces.tsv slot 10).
     The game's own power simulator calls slot 10 on the type-5 tool (power.md line 250) - with netmgr on
     that is OUR tool. Declare a local "as compiled" interface (the FC4LotDevInterfaces.h pattern,
     `cFC4ISC4NetworkToolFull` in cFC4NetworkTool.h, 48 slots mirroring the SDK order with slot 10 fixed to
     `(FC4Int32Quad, uint32_t)`), inherit THAT in cFC4NetworkTool, add the stand-in method bodies, then a
     question or waiver note for abi_conformance (gen_abi_entries.py row for the SDK header is
     report-only, line ~321; the required row cFC4ISC4NetworkTool slots 3-10 stays as is).
  2. `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/glue29 quick`, then `all`.
  3. In-game (runs numbered from 213; docs/runs format): `enable = netmgr,nettool`, turbo to 2032 on
     Konradshohe, script with `network ax az bx bz 0` actions (type 0 road; FC4HarnessLogic.h line 163);
     verify `nettool ... inserts=N` log lines + `0x99EF1142` data4=3 messages; ref_warnings=0; a full-run
     verdict at PostSystemServiceShutdown. Also one run with nettool ALONE (original manager) via the
     registry path if slots allow.
  4. docs/lanes/glue29.md report + build-runs/merge-queue/glue29.json (abi=false).
- **Facts for the successor:**
  - main has glue24's replacement netmgr/netlot (train12); the tick-5500 int3 there was DISARMED (RTC
    handler in every build; runs 210-212 clean). glue24's stand-in no longer exists as a separate file -
    it is the !live_ mode of this lane's class.
  - The core's netmgr Init caches all 13 tools via host.create_tool (manager.rs ~line 143) and slot 5
    `unique=true` creates fresh ones; `destroy_network_occupants` calls tool_init/tool_destroy_occupants/
    tool_shutdown on the cached tools (manager.rs 346-352); the underground refit calls
    tool_update_underground_heights (manager.rs 544) - the manager's ToolUpdateUndergroundHeights is a
    no-op stub; wiring it to the live tool's fc4_nettool_update_underground_heights is OPTIONAL follow-up
    (the tool needs the underground pieces' slot-94 reads, which the stub does not collect).
  - The per-cell piece choice (network-rules.md section 11) is NOT wired: every solved cell inserts one
    ordinary network occupant via GZCOM class 0xC9C05C6E with piece id 0 - documented [choice] in the host.
  - Underground exemplar depth fields passed as zeros (Init reads city size only) - fine for road/power;
    the pipe refit needs a follow-up.
  - `OccupantExists` (slot 9): the core answers from its world cache itself (host callback NULL).
  - Paths: worktree C:/Users/tlori/FreeCity4/.claude/worktrees/glue29; branch glue29; tip 4883aa89;
    questions file docs/specs/questions/network-tools.md holds NT6-NT13 (open, from cores27).
