# Lane glue21 (the prop manager / lot configuration manager replacements and the 0x89C48F47 save sweep)

A HANDOFF §0-style row for the coordinator's table, plus the knowledge the next lane needs.

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| glue21 (`enable = propmgr` / `enable = lotcfg`, the save sweep) | `C:\Users\tlori\FreeCity4\.claude\worktrees\glue21`, branch `glue21`, tip `<git rev-parse HEAD>` | Gate at this commit: cargo 2860/0, clippy clean, ctest 58/58. **Save sweep done** (see below). **Game run `glue21-base-1` `clean`, `ref_warnings=0`, turbo to 2032-01-08, all three shadows exact** (propmgr 220/220, lotcfg 293/293, tutorial 146/146). The two replacement classes are **wired, off by default, and not yet in-game clean**: `enable = propmgr` and `enable = lotcfg` each end `corrupt` with one foreign first-chance exception (GLUE21-2 and GLUE21-1). Landed: `FC4PropLotInterfaces.h` (the no-overload prop manager vtable), `cFC4PropManager.{h,cpp}` (a complete host table), `cFC4LotConfigManager.{h,cpp}` (same), `enable = propmgr` / `lotcfg` in the GZCOM registry, `fc4::PropManagerAppShutdown` / `fc4::LotConfigManagerAppShutdown` at `PreAppShutdown`, the shadows kept on over a replacement, the `0x89C48F47` and `0xEAD4BA36` save-sweep arms, and the `cFC4ISC4LotConfigurationManager` corrections in `FC4TractInterfaces.h`. | (1) GLUE21-2: find the null the game reads at `+0x7C` on the prop path (the city now loads and ticks to SimNewDay 2026-01-28 with `enable = propmgr`); (2) GLUE21-1 blocks `enable = lotcfg` outright - `cSC4LotConfiguration` is not GZCOM-creatable, so the catalogue cannot be built; (3) re-run `glue21-gate-1` once (1) is fixed and record `clean` with the shadows exact over the replacements; (4) file the numbered questions below. Cores15's PL1 (text decal slots), PL3 (monopoly-piece slot 19 sign), PL8-PL14, TU1-TU2 and TU4-TU5 are still with an RE lane and are not this lane's |

## The save sweep (done)

Record `0x89C48F47` is the **prop developer** (`cSC4PropDeveloper`, city component 10), not a lot-configuration
record; the "city detail class" is `cSC4CityDetailManager` (`0xEAD4BA36`, component 51). Both already had
`fc4-capi` `Read` / `Write` and unit tests; neither was in the class-record sweep. Both are now
(`engine/capi/src/save_sweep_tests.rs`, `save_sweep_class_records`), with `prop_developer_tests::sweep_round_trip`
and `city_detail_tests::sweep_round_trip`.

Run over the 421-distinct-save Documents Regions corpus
(`FC4_SAVES_DIR=C:\Users\tlori\Documents\SimCity 4\Regions`, 502 `.sc4` files, 421 distinct):

```
save sweep commercial: records=421 identical=421 ... failures=0; sizes: 19 B x421
save sweep industrial: records=421 identical=421 ... failures=0; sizes: 2 B x421
save sweep civic:      records=421 identical=421 ... second-trip-identical=421 failures=0
save sweep parks:      records=421 identical=421 ... failures=0
save sweep prop-developer: records=421 identical=12 converted=409 ... second-trip-identical=421 missing=0 failures=0;
  sizes: 122 B x409, 127 B x2, 4655 B x1, 6823 B x1, 7199 B x1, 7335 B x2, 16879 B x1, 24935 B x1, 44575 B x2, 47055 B x1
save sweep city-detail: records=16 identical=16 ... missing=405 failures=0; sizes: 8 B x16
```

That is exactly the corpus census of `prop-developer.md` §7 (409 version-4 records at 122 bytes, 12 version-5
records of 127 to 47,055 bytes) and `city-detail.md` §4 (the record present in 16 of the 421 saves, all
`00 00 00 00 03 00 00 00`, the shipped templates having none). Two expectations are encoded:

- the prop developer's `Write` always emits version 5, so a version-4 record is written back **converted** (the
  version word set to 5 and the five bytes of the v5 tail appended - `signsVisible` 1 and an empty sign list) and
  a version-5 record byte-identically; either way a **second** round trip must be stable;
- a save with no city detail record is counted `missing`, not failed (`save_sweep_tests.rs` fails on a missing
  record for the other five classes; `city-detail.md` says only a city that ran and saved has one).

## Implementing the two replacement classes (what is in place)

- **`cFC4ISC4PropManager`** is `engine/plugin/FC4PropLotInterfaces.h`: no overloads, the game's verified slot
  order of `prop-lotconfig.md` §A9 (slots 0-39; slot 40 is the class's scalar deleting destructor, which the
  concrete class's `virtual ~` supplies - the `cFC4Development.h` / `cFC4TrafficSimulator.h` convention).
- **`cFC4ISC4LotConfigurationManager` is NOT in that header**: `FC4TractInterfaces.h` already declared it once for
  the whole plugin. Its slot 5 / 6 names were the reverse of §B7's game order (the header's SDK error) and its
  set queries returned `bool` where EAX carries the count; corrected in place against §B7 and the `ret` column of
  `docs/abi/interfaces.tsv`. No call site used the wrong shapes.
- **Both host tables are complete.** `cFC4PropManager.cpp` fills all 45 `fc4_prop_manager_host` callbacks
  (the 25 city-mutating ones `FC4PropLotShadow.cpp` leaves NULL: the shutdown-callback note, the command pair,
  `subscribe` / `unsubscribe`, `view_filter`, the occupant mutators and `insert_occupant` / `remove_occupant`, the
  four building-prop property callbacks, `terrain_altitude`, and the text decal ones); `cFC4LotConfigManager.cpp`
  fills all 40 `fc4_lot_config_host` ones. The read-only half reuses the shadow's own call shapes
  (`ExemplarKeyFilter`, `LoadProps`, `PropUsageFilter`, `HandOver`).
- **Process lifetime** (glue15's two traps, kept): each catalogue is created once per process and torn down only
  at `PreAppShutdown`; `fc4_lot_config_unload` runs **once** there. Never per city or per run.
- **`Read` before `Init`** (HANDOFF §5.1): the cores are created in the **constructors**, not in `Init`. This was
  the first real bug: with the core created in `Init`, `Read` could never see one and the city died during load.
  It is the single biggest of the three fixes (`glue21-prop-3` -> `glue21-prop-4`).

## The three shadows over a replacement

`FC4PropLotShadow.cpp`'s `PropLotShadowStartup` used to turn a shadow off with
`requested but INACTIVE: it compares the ORIGINAL, and enable = <key> replaces it`. That made "shadows exact"
vacuous whenever a replacement was on, so since this lane a replacement does **not** turn its shadow off: the
shadow reads whatever the city holds - FreeCity4's class when `enable = propmgr` / `lotcfg` / `tutorial` is on,
the game's otherwise - writes that class's own record and runs a private core over it on a read-only host. Over a
replacement the comparison is a class-layer self-check of the vtable, the QI table and the record round trip; over
the original it is glue15's differential. The shadow and the replacement keep separate process catalogues and
neither tears the other's down.

The one run that exercised this cleanly is `glue21-base-1` (the shadows over the originals). The replacement runs
crash before the first shadow line, so the "over a replacement" path is **not yet observed in game** - see
GLUE21-2.

## Three bugs found on the way (all in the two classes)

1. **`Read` before `Init`.** The core was created in `Init`; `Read` runs first and found none. Fixed by creating
   the core in the constructor. (`glue21-prop-3` -> `glue21-prop-4`: the city went from dying during load to
   loading and ticking a game day.)
2. **`GetPropName` handed back a fake object.** §A5 slot 12 returns a reference to a string that is always empty;
   the first cut returned `*reinterpret_cast<cIGZString*>(&a zeroed uint32_t[4])` - a null vtable, so the first
   method the game called on it faulted. Now a process `fc4::GameString` (the game's own `cRZString` vtable and a
   B character buffer) initialised empty.
3. **Two tokens handed to the core without a reference.** `acquire_service` and `building_at_cell` returned
   objects the core then `release`d - a refaudit violation and, once the core stores and reuses them, a
   use-after-free. Both now `FC4_ADDREF` / `FC4_ACQUIRED` first, as `FC4PropLotShadow.cpp` and
   `cFC4Development.cpp` do.

## Numbered questions

1. **GLUE21-1 (`cSC4LotConfiguration` is not GZCOM-creatable).** `lot-configuration.md` §1 says the class ID is
   `0xA99062B4` and it is "**not registered with GZCOM for creation: the manager allocates it directly**" (0xC0
   bytes, constructor `0x004797A0`, deleting destructor slot 33). FreeCity4 does not replace the configuration
   class and has no route to the game's allocate-and-construct pair, so `enable = lotcfg` cannot build a
   configuration catalogue: `create_configuration` probes GZCOM and refuses when it does not answer rather than
   inventing a layout, and the game then gets `GetLotConfiguration -> null` where it expects a configuration
   (`glue21-lot-1` / `glue21-lot-2` fault at `SimCity 4.exe+0x5BF49` reading `0x8C`). Question: is there a route
   to the game's `cSC4LotConfiguration` allocator-and-construct pair that the spec has not named (an application
   or class-store slot), or is `cSC4LotConfiguration` itself meant to be reimplemented (it is never saved: "a lot
   saves only the configuration ID")?
2. **GLUE21-2 (the remaining prop-manager fault).** With `enable = propmgr` the city now loads and ticks to
   `SimNewDay 2026-01-28` and then takes a first-chance `ACCESS_VIOLATION` outside `FreeCity4.dll`
   (`SimCity 4.exe+0x2F44EF`, reading `0x7C` from `esi = 0`) in the prop developer / construction-occupant path
   that calls prop manager slot 15 (`CreateProp`). The core's `create_prop_instance` returns 0 when GZCOM does
   not answer `GetClassObject(cls, 0xE9793A65, &p)`, and the class then hands the game a null where it expects a
   prop occupant. Question: should `CreateProp` / `Add*Prop` fail the call outright (return `false` and leave the
   out-parameter null) rather than returning a null the caller dereferences, or is a prop occupant creatable by
   some route the spec has not named?
3. **GLUE21-3 (what a shadow compares when a replacement is on).** This lane made the three shadows stay on when
   `enable = propmgr` / `lotcfg` / `tutorial` is set, so that "shadows exact" is not vacuous. Over a replacement
   the comparison is then a class-layer self-check (ours against our own private core), not an independent
   differential. Question: is that the intended reading, or should a replacement run compare FreeCity4's class
   against the **original**, which would need the original class kept reachable after the registry swap?
4. **GLUE21-4 (`cFC4ISC4LotConfigurationManager` naming).** `FC4TractInterfaces.h` declared slot 5 as
   `AddLotConfiguration(cISC4LotConfiguration* config)` and slot 6 as `AddLotConfigurationByKey(key const&,
   void*)` - the reverse of §B7's game order, where slot 5 takes **only the key** (pop 4) and slot 6 takes the
   configuration and the flag (pop 8). This lane renamed them to match §B7 and corrected the set queries' return
   channels (count in EAX, not `bool`). No call site used the wrong shapes. Question: confirm the renames are the
   right fix for the whole plugin rather than a per-lane override.
