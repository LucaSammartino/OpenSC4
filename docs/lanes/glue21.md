# Lane glue21 (the prop manager / lot configuration manager replacements, the save sweep and the refcount fixes)

A HANDOFF §0-style row for the coordinator's table, plus the knowledge the next lane needs.

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue21 (`enable = propmgr` / `enable = lotcfg`, the save sweep, the refcount fixes) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21`, branch `glue21`, tip `9f3bb016` | Gate at this tip: cargo 2860/0, clippy clean, ctest 58/58. **Game run `glue21-base-1` `clean`, `ref_warnings=0`, turbo to 2032-01-08, all three shadows exact over the originals** (propmgr 220/220, lotcfg 293/293, tutorial 146/146). **`glue21-gate-11` is still `corrupt`, but the two faults this lane fixed are gone**: the city Init now passes the lot configuration manager and the building development simulator's density walk, PostCityInit is reached and all three shadows go ON over the replacements; the remaining fault is in the game's own `CreateLotObjects` (network-lot redevelopment, city-detail §3.4) and is filed as **PL17**. Landed since `baf55a41`: (1) the creation reference (`CreateConfiguration` AddRefs the new `cFC4LotConfiguration`, the original helper's AddRef — without it the catalogue's release after the add deleted the object); (2) the game-shaped slot 20/21 tables (`GetLotConfigurationMap` / `GetLotConfigurationFamilyIDSet` rebuild STLport hash containers from the core and hand the game real, walkable addresses — the BDS's density walk faults on a null table, building-development §4.7.4); (3) `QueryInterface` AddRefs in both replacement managers (the COM contract; the class store's loader takes the factory result as the caller's reference and releases it after `Read`, so without the AddRef the release dropped the table's reference and destroyed the manager — the city's component-9 pointer dangled into city Init, the 0x45BF49 fault); (4) `Position` reads the occupant token as `cISC4Occupant*` (the occupant manager hands the core occupant sub-object tokens; calling `AsOccupant` through a prop-occupant cast invoked occupant slot 3 = `Init`, returned 1, and dereferenced 1). Questions PL15/PL16/PL17 filed in `docs/specs/questions/prop-lotconfig.md` (the file is new on this branch). | (1) PL17: the `CreateLotObjects` fault (`SimCity 4.exe+0x60D4B5` reading `0x06E932F8`, `ecx=0`, `eax=edx=0x01BA4CBE`, after civic Init, before PostCityInit) — candidates are our `GetObjectIDArray` (slot 27) and `AddStreetProp` (slot 32); (2) once clean, re-run `glue21-gate-1` and record `clean` with the shadows exact over the replacements; (3) the merge-queue entry is `wip` and must be flipped to `ready`. Cores15's PL1 (text decal slots), PL3 (monopoly-piece slot 19 sign), PL8-PL14, TU1-TU2 and TU4-TU5 are still with an RE lane and are not this lane's |

## The refcount fixes (what was actually wrong)

1. **The manager was destroyed during the class-store walk.** With the original
   `QueryInterface` (no AddRef), the segment table's reference and the loader's
   transient reference were the same count; the loader's release after `Read`
   reached 0 and `delete this` ran while the city's component-9 field still held
   the pointer. City Init then called slot 3 through a freed vtable
   (`[0x097A0710] = 0x80`, the heap's fill) — the 0x45BF49 fault of gates 1-4.
   The fix is the COM contract: every successful `QueryInterface` AddRefs
   (cFC4Simulator::QueryInterfaceImpl is the model).
2. **The configurations were deleted right after the add.** `add_by_key` creates
   the object, `add_configuration` takes the catalogue's reference and
   `add_by_key` releases the creation one. The original helper AddRefs right
   after the construct, so the creation reference exists to be released.
   `CreateConfiguration` now does the same; without it the count hit 0 and the
   object died, leaving every `GetLotConfiguration` a dangling token (the
   garbage-config fault of gates 7/8).
3. **Slot 20 handed the game null.** `GetLotConfigurationMap` /
   `GetLotConfigurationFamilyIDSet` returned nullptr; the BDS's density walk
   (building-development §4.7.4) dereferences the map with no null check
   (gate-5: the AV at `SimCity 4.exe+0x4B0390` reading 8). Both now rebuild
   game-shaped STLport hash containers (0x14-byte table, 12-byte map nodes, 8-byte
   set nodes, prime buckets) from the core's catalogue on every call and hand the
   game real addresses. The map holds one reference per configuration.
4. **`Position` used the wrong interface on the token.** The occupant manager's
   queries hand the core `cISC4Occupant*` tokens; `Position` cast them to
   `cISC4PropOccupant*` and called `AsOccupant` — prop-occupant slot 3 on the
   OCCUPANT vtable is `Init` (returns 1), and the follow-up `GetPosition`
   dereferenced 1 (gates 9/10: the AV at `+0xA843DB` reading 1). Fixed to call
   occupant slot 8 directly, as the shadow's `PropPosition` already does.

## The run ladder (all `glue21-gate-N`, enable = propmgr, lotcfg, three shadows)

| Run | Verdict | Fault |
|---|---|---|
| gate-1..4 | corrupt | `SimCity 4.exe+0x5BF49` reading `0x8C` — the manager freed under the city (fix 1) |
| gate-5 | corrupt | `SimCity 4.exe+0x4B0390` reading 8 — the BDS walked our null slot-20 map (fix 2) |
| gate-6 | corrupt | eip 0 — the map's rebuild crashed on the dangling config tokens (fix 3) |
| gate-7/8 | corrupt | the lot's `GetLotConfiguration` returned freed objects (fix 3) |
| gate-9/10 | exited | our `Position` dereferenced 1 after the wrong-vtable `AsOccupant` (fix 4) |
| gate-11 | corrupt | `SimCity 4.exe+0x60D4B5` in the game's `CreateLotObjects` — **open, PL17** |

gate-11 reaches: PostCityInit, `lotcfg: process configuration map rebuilt, 1909
entries`, the shadows ON over the replacements, and the demand/growth/... agent
registrations — the whole city Init except the tail.

## The save sweep (done, unchanged)

Record `0x89C48F47` is the **prop developer** (`cSC4PropDeveloper`, city component 10), not a lot-configuration
record; the "city detail class" is `cSC4CityDetailManager` (`0xEAD4BA36`, component 51). Both have `fc4-capi`
`Read` / `Write` and unit tests and are in the class-record sweep (`engine/capi/src/save_sweep_tests.rs`).
Run over the 421-distinct-save corpus: prop-developer 421 records (409 converted v4 → v5, second trip stable),
city-detail 16 records, 0 failures.

## The three shadows over a replacement

`FC4PropLotShadow.cpp`'s shadows stay on when `enable = propmgr` / `lotcfg` / `tutorial` is set: the shadow reads
whatever the city holds, writes that class's own record and runs a private core over it on a read-only host.
Over a replacement the comparison is a class-layer self-check; over the original it is glue15's differential.
gate-11 reaches PostCityInit with all three ON and no INCOMPLETE lines before the fault.

## Numbered questions

1. **PL15** (the shadow-over-a-replacement comparison) and **PL16** (the B7 slot 5/6 renames) — filed in
   `docs/specs/questions/prop-lotconfig.md` with the answers bracketed.
2. **PL17** (the network-lot `CreateLotObjects` fault) — filed open; see the table row's next steps.
## Handoff
- State: in progress (run4 round 1). Goal: lotcfg/propmgr replacements game-clean (enable=propmgr,lotcfg,
  turbo to 2032, ref_warnings=0, propmgr/lotcfg/tutorial shadows exact), then the full gate.
- Diagnosis (from glue21fix2-pin-1.log, `build-runs/wave_walk.py <log> 11922`): PL19's "drain" is NOT a
  reference deficit; the growth pass's walk over our slot-9 set CYCLES (8 distinct releases, then 6 nodes
  round-robin forever). Our insert placed nodes at v mod 193; the game's increment recomputes the bucket as
  (v >> 2) mod n (26/26 chain-end jumps fit; identity 0/26). Filed PL20 (questions/prop-lotconfig.md).
- Fix: d5f89876 (FC4GameList.h Hash policy, PointerHashU32 = v>>2; slots 9-12 LotConfigSet; PL18 second add
  dropped). 6d362fea removes the per-op trace and slot-9 markers.
- Game: glue21-r4-1 (enable=propmgr,lotcfg, turbo) clean to 2032-01-07, ref_warnings=0; ledger core slot-9
  adds 949608 = releases at 00709798 949608, zero deletes. glue21-r4-2 (no trace) clean to 2032-01-04.
  The propmgr/lotcfg shadows are no-ops over FreeCity4's own class (FC4PropLotShadow.cpp IsOurObject).
  glue21-r4-3 (shadows over the originals, no enable) clean: propmgr 219/220 (1: GetLotProps lot(10,46)
  2028-10-01 3/2), lotcfg 293/293, tutorial 146/146; base-1 had propmgr 220/220.
- Next: r4-4 = r4-3 again to see if the propmgr mismatch repeats; then qbuild all, report.
