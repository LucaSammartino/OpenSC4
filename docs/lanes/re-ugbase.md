| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ugbase: the network occupant and the underground bases, folded into network-occupants.md §2 and §3 (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `f01d1ce4`, after re-occfilters) | **ready** (docs and RE tables only). network-occupants.md §2.1–§2.7 and §3.1, §3.2, §3.4, §3.6 rewritten from the instruction bytes and the SDK slot order, §2.9–§2.11 added (geometry, the edge store, drawing); nine NO and three UG questions answered, NO6 narrowed, NO32–NO35 filed; 178 names (126 new, 52 renamed). Specified coverage +1,874 B / +18 functions | NO32 (`HasProperty(0x88696402)` and the `+0x20` interface), NO33 (where a piece's own path info is saved), NO34 (the underground model builders' geometry), NO35 (the unexplained saved words) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), read
with the local disassembler and the Ghidra export, and the gzcom-dll SDK's `cISC4NetworkOccupant` slot order. This
lane was written on 2026-10-20 as a separate `network-occupant.md` plus underground-occupants.md §7–§8 and held back
when `main` gained network-occupants.md; this is that work folded into network-occupants.md, checked against the
bytes wherever the two disagreed. Every disagreement checked came out on the lane's side.

### What changed (docs/specs/network-occupants.md)

- **§2 rewritten**: the sub-object map and identity (class ID, QI, factory and pool, destructors, the model instance
  interface's slots); the fields with object offsets and the property-holder properties; the record with every field
  named, the path-info key rule and the read conversions; the full interface slot table 3–80; the lifecycle
  (construction, `StaticInit`, both `Init` forms, `Shutdown`); path connections; flags, lighting and messages.
  Upstream's transform rules, record byte count, V5 flag table, path origin offset and prebuilt facts are kept.
- **New §2.9–§2.11**: `GetAltitude`, `GetNormal`, `GetOccupiedCell`, `IsPartOfTheSameStructure`, `GetPosition`,
  `ComputeBox`, `Pick`; the edge store (`cSC4EdgeConnectionStore`, slots 37–45); the textured quad, `SetQuadDraw`,
  texture coordinates, parts, the sort value, textures and their cache, translucency, `Draw` step by step and the
  one-way arrows.
- **§3.1, §3.2, §3.4, §3.6 rewritten**: the underground occupant's fields, every extended slot by what it does, both
  `Init` forms, the static set-up, the quad's triangles, the ceiling, the ratio tables, `GetPosition`, the neighbour
  rules (`IsImaginaryConnection`, `CountRun`, `GetZipperPieceType`), `Refresh`, model and drawing; the record with
  field meanings; the connection occupant's layout, fields, record, `Init` forms and message handling.
- **Corrections**, each read in the bytes: `+0xFC` piece id, `+0x100` under-texture, `+0x104` rotation and flip,
  `+0x105` variation (slots 23–29 by the SDK: 27 is `SetVariation`, 29 `SetUnderTexture`); the type at `+0x28`
  (`0x0061BF85`); `+0x4C` the properties `Init` sets; `Init` clears `0x2000` (`0x0061CB7A`), gates the retrofit list on
  `CheckForRetrofit`, and re-sets `0x10000000` / `0x800000` on a second `Init`; `StaticInit` fills the 13-entry
  avenue set (`0x00AA7D40`) and looks up the texture manager (no subscription); the path-connection bytes are
  `[type][entry]` with a bit per exit (`0x00619790`); `GetPosition` is the corners' mean and highest y
  (`0x006193D0`); `GetOccupiedCell` rounds to nearest (`0x0061B010`); the underground constructor is `0x0065C330` and
  `0x0065C0C0` its destructor body; the slot-101 descriptor lands at `+0x26C`; the connection's `Read` is
  `0x00657590` and its `Init(classID, descriptor)` begins at `0x00654710`.
- Quirks 9 and 10 corrected, 14–20 added; §12's inline list updated; tests 6 and 10 corrected, 26–34 added.
- **underground-occupants.md**: its pointers to the base classes and to UG1, UG2, UG4 now go to network-occupants.md.

### Names and coverage

- re/symbols/network-occupant.tsv: 178 names (126 new; 52 replace auto or folded names, among them `0x00654710`
  `cSC4UndergroundConnectionOccupant::Init`, was a folded destructor name, and the 2026-10-20 Mac-pairing fixes
  `0x00604CA0`, `0x00659D70`, `0x00654160`, `0x00654620`). The `names_windows.tsv` delta (−52 +178) is identical
  to `merge_names.py`'s. `0x0065AB50` takes the name network-occupants.md already used, `ComputeModelYPosition`.
- tools/coverage-map.tsv: `cSC4EdgeConnectionStore`, `fn:GZCOM_CreatecSC4NetworkOccupant`,
  `fn:SC4WriteGZPersistRef`, `fn:SC4ReadGZPersistRef` and `(anonymous_namespace)::ConvertV5FlagsToCurrentFlags` →
  network-occupants.md (the classes are already owned by its rows).

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: after re-occfilters 3,233,636 bytes in 14,620
functions; with this lane 3,235,510 bytes in 14,638 functions (+1,874 B, +18 functions; denominator 5,584,579 bytes
unchanged). `main` already owned these classes; the gain is the newly named functions. `docs/coverage-report.md` was
not regenerated (the tool accepts only the GOG exe).

### Questions

- Answered and moved to questions/answered/network-occupants.md: NO1 (DIFFERS: flag `0x40` is the avenue type),
  NO3 (DIFFERS: underground pieces carry `0xC772BF98`), NO5 (the SDK names of the property slots), NO7 (the piece
  id), NO8 (model descriptors, not strings), NO9 (MATCHES: false), NO12 (DIFFERS: the table shape and the counts),
  NO14 (slot 24 is `GetRotation`; test 6 fixed), NO20 (DIFFERS: no bounds check).
- Answered and moved to questions/answered/underground-occupants.md: UG1, UG2, UG4.
- Narrowed: NO6 (only the pipe's `+0x270` stays). Filed: NO32–NO35. NO19 (the v < 7 rounding steps) was not
  re-read.

### Not covered

- The model builders' vertex geometry (NO34) and the connection's draw states are summaries.
- The code Ghidra lists as a function at `0x0065471D` lies inside `0x00654710`; its auto name was left in the table.
