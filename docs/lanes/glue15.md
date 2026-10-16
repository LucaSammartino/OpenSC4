# Lane glue15 (prop manager / lot configuration manager / tutorial glue)

A HANDOFF §0-style row for the coordinator's table, plus the knowledge the next lane needs.

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue15 (prop manager, lot configuration manager, tutorial cores) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue15`, branch `glue15`, tip `72b27bc5` | Gate at `72b27bc5`: cargo 2596/0, clippy clean, ctest 57/57; game runs `glue15-sh-1` / `glue15-sh-3` / `glue15-sh-7` (turbo to 2028-01-01) and `glue15-sh-4` (normal speed to 2032-07-21) all `clean`, `ref_warnings=0`, and **all three shadows exact** (propmgr 76/76, lotcfg 101/101, tutorial 50/50; normal speed 238/238, 317/317, 158/158). Landed over main: **PL2 applied** (`8367c7e6`: `kSC4MessagePropExemplarDataUpdated`'s data2 is the prop type map's count, not 0, with a core and capi vector); **`cFC4TutorialSystem`** (`3a403e52`, `enable = tutorial`, off by default) over `fc4_tutorial` — the QI table of tutorial.md §1, the 11-byte v2 record with the GUID mapping, IsTutorial and the getters, Init / Shutdown, the last Release, and `fc4::TutorialIsTutorial` which the fire host (`FC4DisasterHost.cpp`) and the flammability glue (`cFC4Flammability.cpp`) now read instead of tutorial slot 8; **the three shadows** (`7a9a757c` + fixes: `FC4PropLotShadow.{h,cpp}`, `propmgr_shadow` / `propmgr_shadow_cells` / `lotcfg_shadow` / `tutorial_shadow`, off by default) with the process catalogues torn down at PreAppShutdown (`fc4_prop_catalogue_app_shutdown`, `fc4_lot_config_unload`); **the handle rename** (`6e427284`) `fc4_lot_config` → `fc4_lot_config_manager` (cores15 item 2; the C ABI entry points keep their `fc4_lot_config_*` names, `abi_entries.inc` regenerated with no diff, ABI stays 68). `plugin_ini_enable_tutorial_test` (57th ctest). **Not done:** the `enable = propmgr` and `enable = lotcfg` replacement classes (cores15 item 1's remaining half), and the save sweep for record `0x89C48F47` / the city detail class | (1) **`cFC4PropManager`** and **`cFC4LotConfigurationManager`** as GZCOM classes over the `fc4_prop_manager` / `fc4_lot_config_manager` cores — see "Implementing the two replacement classes" below; (2) the save sweep for record `0x89C48F47` and the city detail class; (3) add `tutorial` to the regress replace list and the three shadows to the shadows list (exact in every lane run here); (4) file the two numbered questions below. Cores15's PL1 (text decal slots), PL3 (monopoly-piece slot 19 sign), PL8-PL14, TU1-TU2, TU4-TU5 are still with an RE lane and are not this lane's |

## Implementing the two replacement classes

What is already in place and reusable:

- **The cores are done and validated.** `engine/core/src/prop_manager/`, `lot_config_manager/`, `tutorial/` with
  their capi modules. The shadows in `FC4PropLotShadow.cpp` prove they answer exactly what the originals answer on
  a real city (see `docs/runs/glue15.md`), so a replacement is a host/ABI problem, not an algorithm problem.
- **The shadow's host tables are the working reference.** `PropHost()` fills 21 of the 45 `fc4_prop_manager_host`
  callbacks (`FC4PropLotShadow.cpp`) and `LcmHost()` fills 21 of the `fc4_lot_config_host` ones. The ones it
  deliberately leaves NULL are exactly the ones a replacement needs to add: `register_shutdown_callback`,
  `register_command` / `unregister_command`, `subscribe` / `unsubscribe`, `view_filter`, the occupant mutators
  (`occupant_flag`, `occupant_init`, `occupant_shutdown`, `set_prop_usage`, `set_position`, `set_orientation`,
  `hide`), `insert_occupant` / `remove_occupant`, the four building-prop property callbacks
  (`building_props`, `set_building_props`, `add_building_props_property`, `remove_building_props_property`),
  the text decal ones and `terrain_altitude`. Those are the "changes the city" callbacks.
- **The class template is `cFC4Tutorial.{h,cpp}`.** QI table, refcount with the last `Release` calling `Shutdown`,
  the process catalogue created once per process, `CoreLock` for the getters a second thread calls, `FC4_ACQUIRED` /
  `FC4_RELEASE` / `FC4_RELEASE_CREATED` for refaudit.
- **Shared helpers already exist:** `fc4::TokenOf` / `fc4::FromToken` (`FC4Streams.h`), `fc4::ToCoreAll` /
  `CoreProps` (`FC4CoreProps.h`), `CoreLock` (`FC4HistClock.h`), `LoadProps` (in `FC4PropLotShadow.cpp`'s
  anonymous namespace — lift it if you need it).

**The one thing that will bite you: the vtable has no overloads.** `docs/specs/prop-lotconfig.md` §A9 says the
game's vtable lists each SDK overload pair in **reverse** declaration order, so a C++ subclass of the SDK's
`cISC4PropManager` would emit the SDK's order and the game would call the wrong slots. The rule in
`engine/plugin/FC4Interfaces.h` is the fix: declare a `cFC4ISC4PropManager` / `cFC4ISC4LotConfigurationManager`
there with **no overloads** — every method uniquely named (`GetCityPropsWorld` / `GetCityPropsCells`,
`AddCityPropOcc` / `AddCityPropNew`, …) — in the game's verified slot order (§A9 for the prop manager, 0-40;
Part B's table for the lot configuration manager, 0-28), counting `QueryInterface` / `AddRef` / `Release` as
slots 0-2. Then implement that class. `tests/vtable_layout_test.cpp` and `tests/abi_conformance_test.cpp` check
it. The SDK's `cISC4PropManager.h` overloads `GetCityProps`, `RemoveCityProps`, `GetLotProps`, `RemoveLotProps`,
`GetStreetProps`, `GetBuildingProps`, `RemoveBuildingProps`, `AddCityProp`, `AddLotProp` — every one of those
pairs is reversed in the game.

Other facts worth having:

- CLSID `0xA990BD10` (prop manager) / `0x299062C0` (lot configuration manager, fc4.h's own note).
- QI (prop manager, §A1): `0x28326F3E` → +0, `1` and `cIGZMessageTarget2` `0x452294AA` → +4,
  `cIGZSerializable` `0xE4FDA3D4` → +8, otherwise false. Reference count at +0x10. The last `Release` calls
  `Shutdown` then the deleting destructor (slot 40).
- Prop **usage** values (prop occupant slots 22/23): 1 city, 2 lot, 3 building, 4 street.
- The lot configuration manager's `Load` goes through `fc4_lot_config_object_array` (the cores15 item); pass a
  **process-wide** `next_serial` (lot-configuration.md §3.5, `0x00B2166C`), seeded when 0 — not NULL, or the
  generated tag counter restarts per call.
- `fc4_lot_config_unload` is the **process** unload callback (host `register_shutdown_callback`, application
  slot 69): call it at app shutdown. Do **not** call it per city/run on a manager that shares a process
  catalogue — it empties the map and the next rebuild puts the configurations in a different bucket order
  (found in game, `glue15-sh-5`).
- Whatever you use to name a configuration, keep it alive as long as the process catalogue does: the catalogue
  caches configurations by those tokens (found in game, `glue15-sh-6`).

## Numbered questions

1. **GLUE15-1 (generated lot tags).** For a lot configuration exemplar **below version 2**, lot-configuration.md
   §3.5 makes the object's lot tag a process-wide serial (`0x00B2166C`, "seeded from the GZCOM clock when 0, then
   incremented"). Two independent loads therefore generate different tag values, so a shadow cannot compare them
   byte for byte. The shadow seeds its own counter from the same wall clock the plugin uses for the session seed
   and compares the generated tags as generated; every shipped configuration is version 2 and carries its own
   tags, which compare exactly (23302 objects, 0 differing). Question: should the shadow compare only that both
   sides generated a tag for a version-<2 object, or is there a way to read the original's counter so the two
   can be compared exactly?
2. **GLUE15-2 (`glue15-sh-2`'s stall).** One turbo run ended `stalled` (watchdog: no OnTick for 5515 ms,
   `[sample 1/10]`) with the shadows themselves only taking ~10 ms per month; the same DLL then ran the same
   request clean and full (`glue15-sh-3`), and the log ends mid-turbo with normal day posts and no exception.
   This looks like turbo under machine load (three slots were busy), not the DLL. Question: should the watchdog
   treat a long single OnTick during turbo as a stall, or does the harness need a turbo-aware threshold?
