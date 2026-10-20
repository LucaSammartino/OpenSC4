# glue18 — terrain replacement (`enable = terrain`)

Branch `glue18`. Wires `cISTETerrain` and `cISTESurfaceWater` to `fc4_terrain_*`
as a replacement GZCOM class, off by default, `enable = terrain`
(`docs/specs/terrain.md` §4-§10, §15). Merges `main` and `apply15terrain` (the
terrain DIFFERS answers + `tr18_15`-`tr18_23`, which was **not** on main).

## Gate

| | |
|---|---|
| cargo test --workspace --target i686-pc-windows-msvc | **2620 / 0** (14 `test result:` lines) |
| cargo clippy --workspace --all-targets -- -D warnings | clean |
| rustfmt --check --edition 2021 | n/a (no `.rs` touched) |
| CMake build + ctest | build exit 0, **ctest 57/57** (one rerun needed: the two known-flaky flora lock-timing tests fail under load) |

Game runs: [glue18-shadow-1](../runs/glue18-shadow-1.md),
[glue18-latency-1](../runs/glue18-latency-1.md),
[glue18-repl-9](../runs/glue18-repl-9.md).

## What is wired

- **`FC4TerrainInterfaces.h`** — `cISTETerrain` in the **game's** slot order,
  all 98 slots (§15.1 / `docs/abi/interfaces.tsv`). Every method has its own
  name so MSVC cannot reorder an overload group away from the game: the three
  `SetAltitude` shapes are 49 rect / 50 `(index, h)` / 51 `(x, z, h)`, and the
  SDK's declaration order reverses 50 and 51. Slot 91 is the deleting
  destructor declared as one ordinary virtual with that ABI (a C++ virtual
  destructor takes two vtable slots and pushes the tail down). `TerrainFields`
  reproduces the §2.3 field offsets and is the first data member
  (`static_assert` at +0x0C).
- **`cFC4Terrain.cpp`** — `cFC4STETerrain` (the 98 slots over `fc4_terrain_*`)
  and `cFC4STESurfaceWater` (§10), both registered (`0xE98F9525`, `0x0B8DA7DB`)
  so the city never builds the original of either. The water is a stateless
  view over the one handle, so it does not matter which of the two the city
  constructs.
- **Slots that carry spec behaviour** are exact: §3 bounds, §4 height queries
  (including the cached min/max of §4.6), §5 water queries, §6 setters and the
  depth bookkeeping, §7 flags, §8 `RedisplayTerrain` and the long operations,
  §9 brushes, §10 surface water. Render-only slots (64-67, 78-80) are safe
  stubs; §15.1 slots with no rule in §4-§10 are refused and listed as
  questions.
- **Save/load (§13)** — the class records are read and written with the
  `u16 version` **plus the object reference** of foundations §2.7. The
  `cIGZPersistDBSegment` record path (slots 96/97) is not wired; `Init` builds
  the terrain from the city grid instead and the water is rebuilt from the sea
  level either way (§10.1, quirk 15).
- **The terrain 3D view** (class `0xC9B84E10`) is created at `DoPostCityInit`
  through the game's own class factory, as the original does. See the open
  question below: its initialisation contract is not in `terrain.md`.

## TR18 (§15.4) — re-entrancy

Enforced at the glue boundary as well as in the core. Every `fc4_terrain_host`
callback (`random_uniform`, `lot_at_cell`, `bridge_at`, `has_power_pole`, the
lava's flammability/fire calls, `next_burn_cell`, `post`) goes through one
gate: while one of our entry points is inside the handle (`inside > 0`) a
callback is **refused, counted and logged**, and the core's own refuse-and-count
runs as well. Both counters are reported at `Shutdown`.

Observed: **refused=0 in every shadow line of both clean runs** (the
navigability host is the only host that runs there, and it reads the game's
traffic network and occupant manager only). That matches §15.4: on the
simulation's paths the lava and brush kinds 0/2/5/7 never re-enter, so the
count stays 0. God-mode brush kinds 1 and 8-10 do call the flora and the
surface water by design; the glue never drives them, and if a caller does, that
work must run outside the borrow.

## TR9 — the default brush level

`0xA6B79602` is subscribed and the default brush level follows it
(`fc4_terrain_set_default_brush_level`), as §2.3 says. The initial copy the
original makes in `DoPostCityInit` from the 3D camera (global `0x00B43DD8`,
`+0xC`) is **not** reproduced — the plugin has no camera interface — so the
level stays 0 until the first view message, which is also what a harness with no
view sees (conformance test 20). Which word of that message carries the zoom is
open (question TR23); the run logs both `data1` and `*void2` so it can be
settled from a log rather than a guess.

## Terrain shadow

`terrain_shadow = 1` alone is **unchanged**: glue18-shadow-1 is exact, 98/98
lines, `unf` 0 / `late` 18 / `pend` 0.

`enable = terrain` with `terrain_shadow = 1` does not fight: the shadow
compares against the ORIGINAL terrain, which then does not exist, so it goes
**inactive** with one log line (the same rule the history shadow has). It is
never in the way of the replacement and never compares ours with itself.

## Message latency (task item 4)

The queued `0x278E6003` posts are late under turbo, as glue16 measured. In
glue18-latency-1 a setter ran on **2026-03-01** and its change message was
followed on **2026-04-10** — about **40 game days**. The replacement therefore
does **not** assume prompt delivery: `RedisplayTerrain` hands the posts to the
game's message queue (foundations §5, `void1 = IGZUnknown = the sender`) and
nothing in the glue treats the message as arriving with the edit. Consumers
that do (the network manager's terrain handler, the landfill, the BDS) see the
change weeks late; the shadow's "ahead / followed late" bookkeeping is the
reference for how to think about that window.

## Not done / open

`enable = terrain` does **not** reach a `clean` verdict. It gets the city
through `PostCityInit` and then faults in the terrain 3D view. See
[glue18-repl-9](../runs/glue18-repl-9.md) for the crash history; four
integration bugs were found and fixed on the way and are in the commit log.

## Numbered questions

- **TR21.** The terrain 3D view (class `0xC9B84E10`, §2.3 `+0xE8`, slot 66):
  what does `DoPostCityInit` pass it, and what is its initialisation contract?
  `terrain.md` puts the view out of scope, but the game hands the pointer around
  (zones.md §5.2 takes it at `PostCityInit` from slot 66 and calls the view's
  slots 9 / `+0x10` / `+0x38`), so a replacement cannot survive without it. The
  class factory creates the object; what is missing is how to bring it up.
- **TR22.** Does the game read the terrain object's fields directly, beyond the
  slots? `terrain.md` §2.3 marks them internal, but §1's `this` / `this + 4` /
  `this + 8` and the in-game faults show the layout is observable. Which fields
  does which caller read? (This lane reproduced the whole §2.3 table.)
- **TR23.** (TR9, the note the HANDOFF leaves.) Which word of `0xA6B79602`
  carries the view's zoom index: `data1` (the literal reading of §2.3, "whose
  first data word differs from it stores that word") or `*GetVoid2()` (what
  zones.md and `cFC4ZoneDeveloper` read for the same message)? And what is the
  initial value at `DoPostCityInit` — is there any way to read the 3D camera
  (`0x00B43DD8`, `+0xC`) from the plugin?
- **TR24.** Rules for the §15.1 slots that §4-§10 do not define: 25
  `GetTerrainGridIndex`, 39 `AreSeaVertices`, 58 `GetBrushExemplarID`, 61
  `BrushRadius`, 62 `GetCostPerBrushApplication` (the mayor-mode per-application
  charge of §9.6).
- **TR25.** Rules for 43 `CreateFlatAreas`, 44 `CreateShoreCliffs` (both post
  data4 0), 46 `ChangeSeaLevel`, 48 `ChangeTerrainLevel` (data4 1), 56
  `StartFlood` (§15.1 says +500 m on vertices 20..39 x 20..39).
- **TR26.** The five argument dwords of surface water slot 7
  `IsNavigableAtPosition` (§10.2 says it converts the position to a cell with
  city slot 95 and calls slot 8).
- **TR27.** Does the surface water need its own message target (§10.1
  subscriptions, §10.3's triggers) and its animation-tick registration at
  priority `0xC0`? §12.2's `AnimateWater` also posts `RedisplayTerrain(0, 1,
  rect, 3)`; the core does not model it, so this lane's water does not animate.

## Next steps

1. TR21, then the view's up: that is the only thing between the current state
   and a `clean` run to 2032.
2. Re-run `glue18-repl-9`'s request once TR21 is answered, and add `terrain`
   to the regress replace list and `terrain_shadow` to the shadows list only
   after a clean enable run.
3. `cIGZPersistDBSegment` records (slots 96/97) for a save/load round trip.
