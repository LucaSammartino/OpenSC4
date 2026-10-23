| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-textdecals: the label's save record, state and decal building, folded into lot-records.md §10.4 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4abc0e4`, after re-app; one commit with re-signposts) | **ready** (docs and RE tables only). lot-records.md §10.4: the record and the state rewritten in object offsets, `SetPosition` and `UpdateDecal` rewritten (the width rule, the box, the angle table), the `Shutdown` and `SetAllFlags` offsets explained; OP1 and OP2 answered, OP19 filed; 35 names (10 new, 25 renamed). No coverage change (`cSC4TextDecalOccupant` was already claimed) | OP19 (the text texture service, `AddDecal`'s numbers) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-20 as a separate `text-decals.md` and held back when `main`
gained lot-records.md §10.4; this is that work folded in.

### What changed (docs/specs/lot-records.md §10.4)

- The conflicts the earlier text recorded as OP1 came from sub-object views: `SetPosition` (`0x005F7AA0`), `Shutdown`
  and `SetAllFlags` run on the `+0x0C` occupant view (`SetPosition` writes view `+0x88`/`+0x8C` = object
  `+0x94`/`+0x98`; `Shutdown` reads view `+0xA8`/`+0x78` = object `+0xB4`/`+0x84`), and `Write` (`0x005F7DB0`) on the
  `+0x10` view. In object offsets every field has one role: `+0x84`/`+0x88` the land and water decal handles,
  `+0x94`/`+0x98` the position, `+0xB4`/`+0xB8` the managers.
- The record re-read from `Write`: version 3, the prop record, the text (slot 15), position, height, width, the
  position again, colour, alpha, orientation (u8).
- `UpdateDecal` (`0x005F7810`): `w = 16 · r · h` with `r` the texture's aspect (the earlier text read the texture as a
  manager and multiplied by `+0x98`), the box with `a = w / 2`, `b = 8 · h`, and the angle table at `0x00B0A978`
  indexed by rotation and orientation (the earlier text called it a view-level scale).

### Names and coverage

- re/symbols/text-decals.tsv: 35 names (10 new: the QueryInterface and Write/Read/destructor thunks and the destructor
  body; 25 replace auto names). The `names_windows.tsv` delta (−25 +35) is identical to `merge_names.py`'s; no
  non-auto upstream name is replaced.
- No coverage row change; specified bytes unchanged (3,478,182).

### Questions

- Answered: OP1 (the field roles), OP2 (slot `+0x3C` is a string; with re-signposts).
- Filed: OP19 (the text texture service `0x00B43CE4` and `AddDecal`'s width and angle, the held lane's TD1).

### Not covered

OP19.
