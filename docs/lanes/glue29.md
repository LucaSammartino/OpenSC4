# Lane glue29

Per-lane row for the regenerated `HANDOFF.md` 0 table (see `docs/lanes/README.md`).

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue29 (the cSC4NetworkTool replacement glue over cores27's fc4_nettool ABI) | `glue29` / `.claude/worktrees/glue29` (branch `glue29`, engine lane) | in progress (round 4: slot-10 ABI hazard FIXED, conformance 0 mismatches, `qbuild all` running) | (1) `qbuild all` verdict; (2) in-game: `enable = netmgr,nettool` + scripted `network` actions to 2032 (runs from 213); (3) queue file + report |

## Handoff

- **Goal:** wire the network tools replacement: a `cFC4NetworkTool` plugin class over the `fc4_nettool_*` C ABI
  (cores27), host callbacks bound to geometry core (`fc4_netgeom`), occupant manager and network manager; enable
  key `nettool`; in-game checks (REQUIRED before ready).
- **State at this stop (round 4):**
  - SLOT-10 ABI HAZARD FIXED (commit `b53dcc89`): new `cFC4ISC4NetworkToolFull` in FC4Interfaces.h — the
    compiled interface (0x00AA8748, slots 0-50) with the SDK's 3 errors fixed: slot 10 `UpdateOnZoneChange`
    `(FC4Int32Quad, uint32_t)` pop 20 (foundations.md M3 errata; the game's power simulator calls it on the
    type-5 tool — that call now hits our Full vtable correctly); slot 33 returns float in ST0; slot 35 takes
    (index, string). cFC4NetworkTool now inherits the Full (NOT the SDK cISC4NetworkTool).
  - `UpdateOnZoneChange` logs the rect (power.md §5 telemetry). gen_abi_entries.py row added (Full, required);
    kReviewedSlotCounts entry 51/81 (slots 51-80 are class methods, network-geometry.md §1.1 + slot 51 dtor).
  - CONFORMANCE: 4697 slots compared, 0 mismatches required; Full 51/81 reviewed ok; PowerLine row still ok.
  - Includes fixed in FC4Interfaces.h (SC4Point/SC4Vector/cISC4NetworkOccupant); FC4Script RunNetwork now
    `reinterpret_cast<cISC4NetworkTool*>` (class no longer inherits the SDK type).
  - Rebase leftover fixed: duplicate `ToolOf`/`CreateTool` pair removed from cFC4NetworkManager.cpp (kept the
    live one; `new cFC4NetworkTool(type, NetToolRegistry::Enabled())`).
  - `qbuild quick` PASS. `qbuild all` was launched in background at this stop (cargo test workspace + clippy +
    CMake + ctest) — CHECK ITS VERDICT in build-runs/ (qbuild-all/fc4verify-all logs) before proceeding.
- **Next (exact steps):**
  1. Check `qbuild all` result (it was running in background). If clippy/cargo errors: fix, re-run, commit.
  2. In-game (runs from 213, docs/runs format): ini `enable = netmgr,nettool`, Konradshohe, turbo to 2032,
     script `network ax az bx bz 0` actions (road, type 0; FC4HarnessLogic.h ~line 163). Verify: `nettool
     ... inserts=N` lines + `0x99EF1142` data4=3 messages, ref_warnings=0, full-run verdict clean at
     PostSystemServiceShutdown. Also run ONE city with nettool ALONE (original manager + registry path).
  3. docs/lanes/glue29.md full report + build-runs/merge-queue/glue29.json (abi=false).
- **Facts for the successor:**
  - Question NT15 (ClearOccupantHighlights 16 vs ClearCellPreviews 17 order, Windows vs Mac note) NOT yet
    filed — file in docs/specs/questions/network-tools.md before finishing (next free number is NT15; NT6-13
    open already there).
  - The tool logs `nettool type N <context>: inserts=... removed=...` via LogStatus on placements/destroy.
  - Composition: `enable = netmgr,nettool` → manager's own 13 cached tools are LIVE replacement tools
    (CreateTool in cFC4NetworkManager.cpp); zonedev takes types 3/11/0 at PostCityInit; power simulator takes
    type 5. `NetToolRegistry::Take` creates live tools for FC4Script's `network` action. Without the key,
    byte-for-byte glue24 behaviour (stand-in mode, `live_` flag).
  - Game runs: at most 1 request outstanding (6 slots shared); qwait.py for verdicts; runs numbered from 213.
  - Original tool creation detail (§7.1): factory returns ref count 0; our GZCOM class + registry AddRef
    theirs. FC4Subsystems kNetToolClasses CLSID 0x4E543030 ("NT00"), key `nettool`.
