| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-signposts: the signpost's save record, text textures and highlight colours, folded into lot-records.md §10.3 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4abc0e4`, after re-app; one commit with re-textdecals) | **ready** (docs and RE tables only). lot-records.md §10.3: record fields 3 and 4 corrected (the text string and the model's position), `LoadTextures` rewritten (the text textures, font `0x2C1FD612`; the ten piece textures are `Init`'s), the state rows of the piece textures, the text-texture vector and the render state corrected, the corner colours given; OP2 (with re-textdecals) and OP3 answered, OP18 filed; 31 names (12 new, 19 renamed). No coverage change (`cSC4SignpostOccupant` was already claimed) | OP18 (the model's vertex layout), OP19 |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export and the SDK headers. Written on 2026-10-20 as a separate `signposts.md` and held back when `main` gained
lot-records.md §10.3; this is that work folded in.

### What changed (docs/specs/lot-records.md §10.3)

- The save record: `Write` (`0x005F5080`) and `Read` run on the `+0x10` serialisable view, so the field the earlier
  text called "u32 at `+0x88`" is the sign text (object `+0x98`, stream slot 15, a string) and the position is object
  `+0xDC…+0xE4`.
- `LoadTextures` (`0x005F5EF0`) renders the text into the vector at object `+0x574` with font `0x2C1FD612` through the
  service `0x00B43CE4`; the ten piece textures (`0x8B4A6560`… at `0x00AA5214`) are loaded by `Init` (`0x005F745A`),
  which runs on the `+0x0C` view, so their bindings are object `+0x54C…+0x570` and the packed render state object
  `+0x580`. The "visual effects" vector of the earlier text is the same text-texture vector (its type name comes from
  a folded template, `0x00527180`).
- `SetDynamic`'s corner colours: static (144, 172, 190, 255), dynamic (255, 255, 255, 204) (`0x00AA5204`).

### Names and coverage

- re/symbols/signposts.tsv: 31 names (12 new: the caret timer, the box helper, the constructor, the creator, the
  destructor body and the `+4`/`+0xC`/`+0x10`/`+0x14` thunks; 19 replace auto names). The `names_windows.tsv` delta
  (−19 +31) is identical to `merge_names.py`'s; no non-auto upstream name is replaced.
- No coverage row change; specified bytes unchanged (3,478,182).

### Questions

- Answered: OP2 (slot `+0x3C` is a string; with re-textdecals), OP3 (the corner colours).
- Filed: OP18 (the vertex layout of `RebuildModel`, the held lane's SP1).

### Not covered

`RebuildModel`'s geometry (OP18).
