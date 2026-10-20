# port-developers: answered questions (archive)
<!-- Archive for [port-developers.md](../../port-developers.md). Lanes do not read this file. Open questions are in ../port-developers.md. -->

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PD1:** `GetLeastEfficientPort` loops forever with two or more ports. Does anything call it? [no caller in the exe
  and no Lua binding found; FreeCity4 returns the least efficient port (*[choice]*). Searching the shipped Lua
  (`SimCity_1.dat` advisor scripts) for a binding, then a turbo run with two airports and every advisor open, would
  settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed; port-developers.md §4.7). No direct caller, and no Lua binding: the only port names the executable exposes to scripts are the `g_*airport*` and `g_seaport_*` count and volume globals, and no string contains `LeastEfficient`.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PD2:** the port's monthly `UpdateOccupantEffects(building, old ratio, new ratio)` adds the usage-ratio change on
  top of the demand effects the building already has as an occupied building. Do both apply? [yes: the port call
  moves the effect by the ratio change only. Logging the demand caps (demand.md §8) before and after an airport's
  first month with known office jobs in a turbo run would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (port-developers.md §4.3). `UpdateOccupantEffects` removes the effects at the old ratio and adds them at the new one (demand.md §7.1), starting from 0, on top of the effects the occupied building already adds, so a port gives (1 + usage) × its effects *[inferred: port lots take the common occupancy path]*.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PD3:** what does lot condition 2 mean on a port lot (set when the port is stressed for more than 90 days)?
  [what it means on any lot (lot-developer.md §5.5): the presentation of a distressed lot; the port developer resets
  it to 0 each month it is not stressed. A turbo run with an airport above 96 % usage for four months, logging
  condition and state changes of its lot, would show whether anything else reacts]~~ **ANSWERED 2026-10-16 (re-answers19):** MATCHES (port-developers.md §4.3).
  Nothing reads condition 2 as "port stressed"; the readers are the generic ones: the base-texture decay variant 1
  (re-picked on every condition change, lot-developer.md §6.5), the crime lot bit `0x2` (crime.md §6.7) and the MySim
  `home_condition`. The lot developer's civic branch touches only 0 ↔ 1 and 5, 6 → 0, so condition 2 stays until
  the port month resets it. Consequence *[compat]*: a stressed port that loses power is not set to 1, so the port
  month does not skip it and it keeps updating.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PD4:** the seaport type declares an 11 × 5 terrain grid but ships 72 values (12 × 6). Data error, or are the
  dimensions counted in cells while the values are vertices? [data error as far as the code goes: the grid is
  rejected, so seaports never re-level terrain. Plopping a seaport on an uneven coast and comparing vertex heights
  before and after would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (data error; port-developers.md §4.8). The code counts cells in both places: load needs rows × columns = value count (`0x006EA676`–`0x006EA690`), and use needs rows × columns = the lot rectangle's cell area, one vertex per cell (`0x007013D7`–`0x0070141A`). The declared 11 × 5 is wrong for 72 values; 12 × 6 would load.

## From the city detail, port developer and prop developer cores (filed by the coordinator from the `cores12b` lane's report, 2026-10-16; merged `3eb6d22`)

cores12b implemented city-detail.md, port-developers.md and prop-developer.md as Rust cores
(`engine/core/src/{city_detail,port_developer,prop_developer}/`), C ABIs (`fc4_city_detail`, `fc4_port_developer`,
`fc4_prop_developer`, ABI 65) and conformance vectors (`cd8_*`, `pd12_*`, `pv12_*`). There is no plugin glue yet, so
none of these readings has been checked in game. The current choice is in brackets.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD5:** when the next stage's lot configuration is missing, growth fails before anything is demolished and no funds
  are withdrawn. Is that the original order, or is the lot demolished first?~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** (port-developers.md §4.5). The original never tests the next-stage configuration: the swap (`0x006E92A0`) uses `GetLotConfiguration`'s result as is (`0x006E9303`), demolishes the old lot, creates the new one, sets a null configuration and calls `ConstructDefaultLot(new, null, 0, 1)`, which reaches `StartConstruction` with type 0: state 2, condition 1, no construction occupant, return true. So the swap succeeds, the stage advances, the upgrade cost is withdrawn and the budget updated; the port is left on an empty lot stuck in state 2 (skipped every month, never grows again). Unreachable with shipped data; reproduce it, or keep failing early as a documented *[choice]*.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD6:** §4.8 does not say how the terrain grid's orientation follows the lot's facing. [the grid is rotated by
  facing; facing 0 = rows along z as stored. Moot with shipped data (PD4: the grid never loads), but a correct 12 × 6
  declaration would use it]~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** (port-developers.md §4.8, with a correction to PD4). At facing 0 the grid's rows run along **x** (row = x − x1, column = z − z1), not along z; facing 2 is the 180° turn of 0; facings 1 and 3 are the **transpose** (row = z2 − 1 − z, column = x2 − 1 − x for 1; row = z − z1, column = x − x1 for 3), so they mirror rather than rotate (jump table `0x007016F8`). The use test compares only the product R × C with (x2 − x1)(z2 − z1), i.e. 55 for a 12 × 6 lot: the shipped 11 × 5 declaration is what the use test wants, and a 12 × 6 declaration would load but would not apply to a 12 × 6 lot. The write passes `trunc(CellCenterToPosition)` metres to `SetAltitude(x, z, h)`, which takes vertex indices, so it would write the wrong vertices (`0x007014AE`). Never runs with shipped data.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD7:** after a failed swap the port's lot lookup can return null. [the month skips such a port (counted in
  `month_skipped`) and does not stop] Confirm.~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** from the original, *[choice]* confirmed (port-developers.md §4.3). After a failed swap `p.lot` is null (`0x006E9385`) and the original faults at the next month (`GetState` on the null lot, `0x006E9A2D`–`0x006E9A31`), and likewise in the removal lookup (`0x006E947A`) and the seaport tick. Skipping such a port and counting it is a safe *[choice]*; apply it to every walk over the ports.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD8:** in the month, does the usage value use cap = trunc(capacity × fr) (the §4.3 comment) or the plain capacity
  (§4.7 PortUsageValues)? [§4.7]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES the plain capacity (port-developers.md §4.3, §4.7). The month calls the internal `0x006E9770` (`0x006E9A7C`): usage = min(trips, stage capacity), cap = stage capacity, no funding scaling. Only the public slot 12 query (`0x00700FE0`) then scales the cap by funding, as trunc(float(capacity) / full × float(current)) (`0x00701027`–`0x00701063`), and clamps usage again. If the core applies the funding scaling in the month, that DIFFERS.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD9:** is a seaport port with no sea access counted for the "if any" advisor-event condition in the seaport tick?
  [excluded]~~ **ANSWERED 2026-10-16 (re-answers22):** Answered (port-developers.md §4.4). "If any" is over ports: the event flag is set for every listed port whose lot has no sea access (`0x00701FDE`), before the prop switch and whatever props the port has; a port with access never raises it. So a port without access **is counted**; if [excluded] means such a port does not raise the event, it **DIFFERS**.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD10:** Init removes any existing agent before adding it, and Shutdown does not clear the port list (Read or
  adoption rebuilds it). Confirm against the original.~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** on both points (port-developers.md §3). `Init` is guarded by `initialized` (`0x0068B013`, `0x006EA956`) and adds the agent without removing anything first (a duplicate cannot arise; removing first is harmless). `Shutdown` (`0x006E9F60`) **does** clear the port list: it deletes every port lot (resolving its lot first) and empties the list (`0x006E9FB2`–`0x006EA03D`), and deletes the port types and empties their vector (`0x006EA040`–`0x006EA07D`). It does not reset `tuningLoaded`, `typeCount` or the instance list *[quirk]*; the city never re-uses a developer.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PD11:** Read loads the tuning exemplar from the record's instance even when a later field rejects the record. Does
  the original leave the tuning loaded? [the rest of Read is atomic; the tuning stays loaded]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (port-developers.md §7). In the original nothing after the version checks can reject the record: the port lots are appended without any check and `Read` returns true. The tuning is loaded (`0x006EAAC8`) before the first port lot, so a FreeCity4 rejection for an out-of-range index leaving the tuning loaded is what the original's order gives.
