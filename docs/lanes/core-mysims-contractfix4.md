# core-mysims-contractfix4 — correct MySim dispatch/pool cores to answered MS11/MS12

| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysims-contractfix4 (correct mysims dispatch §4.7.1 / pool §4.12 cores to MS11/MS12 answers) | `core-mysims-contractfix4` / `.claude/worktrees/core-mysims-contractfix4` | ready (gate PASS, no game run: core + conformance only) | answer MS13/MS14; remainder §4.6/§7.2 trip + CheckCommute/FailCommute |

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

### Remaining owner methods (not credited, declared remainder)
- §4.6 `UpdateTripVariables` (sim.rs) not re-audited against the trip-state contract.
- §7.2 `cISC4MySim` `CheckCommute` (dispatch.rs `check_commute` "not reconciled with §7.2") and `FailCommute`
  (sim.rs still `dispatch_command("reset_commute")`, MySim side).
- No plugin/CAPI wiring of the pool or dispatch (out of scope for this lane).
