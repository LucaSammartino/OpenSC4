# core-mysims-contractfix4 — correct MySim dispatch/pool cores to answered MS11/MS12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysims-contractfix4 (correct mysims dispatch §4.7.1 / pool §4.12 cores to MS11/MS12 answers) | `core-mysims-contractfix4` / `.claude/worktrees/core-mysims-contractfix4` | ready (gate PASS at b5c6d533, no game run: core + conformance only) | answer MS13/MS14/MS15; CheckCommute pathfinder config stays a declared remainder (MS15 a) |

## Report

Scope: Rust core + conformance only (no plugin, CAPI or tools). No game run: nothing here is reachable from the
plugin yet, so there is no shadow to compare.

### What changed
- 06cee36b dispatch location contract (MS11 a-e): neighbour -1, IsValid (non-ferry, city bounds, no occupant
  check), SetByOccupant clears byCommute, trip-state field, job-search notifications. Tests t30-t41; vectors
  ms12_21..25 migrated, ms12_28 added. Question MS13 filed.
- 90dcb31a pool per MS12 (`engine/core/src/mysims/pool.rs`):
  - `encode_record`: each string's length byte = byte length mod 256, followed by that many bytes (e).
  - `userdata_dir`: `path + "\UserData" [+ i]`, the `\` always inserted (f).
  - Export listing on `<ud>\Export\`; `F.ini` = Export dir + `Left(len-4)` + `.ini` (d).
  - Picture folder `<ud>\Web Pages\<value>\FamilyGFX\`, source `...family<k>_face.jpg` (c).
  - `IniFile::get` case-insensitive (`rz_files::ini::ini_lower`), `IniFile::read_number` = `scan_long`
    (`%ld`, miss -> None) (b); nummembers result untested; guid from ReadNumber.
  - Member loop unsigned `k = 1..=n` (`n as u32`); only `k` with a `sims_familymember<k>_name` entry are visited
    (every other `k` fails `_name` and skips without effect), so a negative `n` is handled without 2^32 iterations.
  - `MySimTemplate.indeterminate` bitmask (`INDETERMINATE_BITMAP/MALE/SIGN`) flags absent exemplar properties
    that the original leaves uninitialised (a); values stay 0/false/0. Conformance shows it only when non-zero.
  - Tests p8-p10 (all §4.12 MS12 examples); vector ms12_27 paths updated.
- c3761dc8 clippy fixes.

### Measured
- `qbuild all` at c3761dc8: cargo test 3986 passed / 0 failed (baseline 3980), clippy 0, fc4verify build PASS,
  ctest 65/65.

### Questions
- MS13 (§4.7.1): floor cast for NaN/out of range, SetCellPosition world fields, home start rect fallback.
- MS14 (§4.12): (a) `n` after a nummembers miss [0]; (b) key/`family%d_face` text for k >= 2^31 [`%d`];
  (c) F.ini for an entry name < 4 bytes [empty stem]; (d) missing `_gender`/`_zodiac` [empty value].

### Round 3 (final): §4.6 / §4.7 / §7.2
- 06efe8a1 `FailCommute` (sim.rs): `QuitJob`, `MYSIM_COMMUTE_FAILED`, then `MySimHost::find_new_job` =
  `SetCurrentCommuteDestination(default, byCommute 1, notify)` (§4.7.1 callers table: a new job search). The
  placeholder `dispatch_command("reset_commute")` is gone. Tests t06b/t06c check order, the search call, state 2.
- `CheckCommute`: §4.7 says its pathfinder slot use is in §7.2; §7.2 lists none. The dispatch-side
  `check_commute`/`run_pathfinder` (an invented config borrowed from pathfinder.md §8's dispatch trip row) and
  `DispatchState.trip_length` are removed; the MySim keeps the typed host callback `commute_time` and the
  specified `lastCommuteTime := uint(trunc(max(0, t)))` / failure → `FailCommute`. t37 now covers only the
  state-2 default destination.
- `UpdateTripVariables` audited line by line against §4.6: gate, arrival states, destination_subject, flag block
  (bit 0x100 mask, bit-15 event, travel types, 255), local square with the force (0,0) quirk, simulators block,
  lot statistics float32, landmark, disasters, the 0x4BC42C48 post. One change: `trip_traffic_congestion` is read
  with `grid_at(position)` (literal "All grid reads ... PositionToTract"), was a cell read.
- b5c6d533 rustfmt of the lane's own dispatch test lines.
- Gate at b5c6d533: cargo test 3986 passed / 0 failed, clippy 0, fc4verify build + ctest PASS.
- Question MS15 filed (`docs/specs/questions/mysims.md`): (a) CheckCommute pathfinder slots/values/rect source/
  success result/totalTime getter/missing home or job; (b) congestion by position or cell; (c) force (0,0) for
  GetTravelTimeRatio; (d) the gate's "active" check [state != 0]; (e) destination_subject "travelling".

### Remaining (not credited, declared remainder)
- `CheckCommute`'s own pathfinder configuration (MS15 a): only the host callback exists, no core body.
- MS13/MS14 inputs stay explicit indeterminate where the spec leaves them undefined.
- No plugin/CAPI wiring of the pool or dispatch (out of scope); a CAPI host still implementing `reset_commute`
  now gets a `find_new_job` call from FailCommute instead.
