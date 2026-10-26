# `cSC4ModelMakerUtility`: the occupant model helpers (behaviour spec)

Scope: the static helpers of `cSC4ModelMakerUtility` that graphics-formats.md §4.2 does not cover, so that with it the
owner is complete. graphics-formats.md §4.2 specifies `GetModelInstanceID(holder, …)` (`0x00496830`, the RKT rules)
and `GetAllModelInstanceIDs` (`0x00496C90` and its occupant form `0x00497120`); the model maker that calls these is
model-maker.md.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function below was read in the disassembly. Written 2026-10-25 (re-mmutil8).

All helpers are cdecl. "The holder" is the occupant's property holder (occupant slot 6); "has p" is the holder's
`HasProperty(p)` (slot 3).

## 1. Model ids *[compat]*

- **GetModelProperty1Instance(base, zoom, rotation)** (`0x00496590`): `base + 0x100·zoom + 0x10·rotation`, computed as
  `((zoom << 4) + rotation) << 4` plus base (32-bit wrap).
- **GetModelInstanceID(occupant, a2 … a7)** (`0x00497180`): calls `GetAllModelInstanceIDs` with a local list, no offset
  list, the holder, the occupant's flag `0x80` (slot 32 `IsFlagSet`) and then, in this order, its own arguments a2,
  a3, a4, a5, the value 0, a6 and a7; returns the list's first id, or 0 for an empty list (network-occupants.md uses it
  for a piece's main model).
- The helper `0x005F8AB0(i, p)` returns `p + 4·i` (the address of dword i).

## 2. Building-style models *[compat]*

- **OccupantUsesBuildingStyleModels(occupant)** (`0x004965F0`): false when the holder has `0xE9822D62`; otherwise true
  when it has any of `0x27812821`, `0x27812820`, `0x27812822`, `0x27812823`, `0x27812824` (RKT1, RKT0, RKT2, RKT3,
  RKT4, tested in that order), else false.
- **FoundationOccupantUsesBuildingStyleModels(occupant)** (`0x004965B0`): false when the holder has `0xE9822D62`;
  otherwise true exactly when it does **not** have `0x68FCFF37` (the foundation's P1 array, building-development.md).

## 3. Orientation and state (`GetOccupantOrientationAndState(occupant, &state, &orientation)`, `0x00496670`) *[compat]*

Both outputs := 0, then the first interface the occupant answers decides:

| Interface | orientation | state |
|---|---|---|
| building `0x87DDA3A9` | slot 14 | stays 0 |
| prop `0xE9793A65` | slot 9 (`GetOrientation`) | slot 12 (`GetState`) |
| flora `0xC83DD057` | slot 12 | slot 10 |
| landfill `0x2A15F262` | slot 7 | stays 0 |
| foundation `0xC8FCE101` | slot 11 | stays 0 |

Each queried interface is released; an occupant answering none keeps both zeros.

## 4. Model classes *[compat]*

v is the occupant's property `0xEA123CEF` read as a list of 32-bit values (`cSCPropertyHelp::GetPropertyValue`, the
vector overload); a failed read gives 0 / false at once. A **plain** occupant is one whose type (slot 7) is not
`0xA823821E` (props), or a prop whose holder has `0x4A70D491`.

**GetOccupantModelClass(occupant)** (`0x00496950`) → class:

- plain: `v[0]`, or 0 for an empty list;
- otherwise the prop interface `0x2977AA47` (none: 0) gives the state s (slot 12). With one value: `v[0]` when `s = 0`,
  else 0. With an even count, v is read as `(state, class)` pairs and the class of the first pair whose state is s
  is returned, else 0. Any other odd count gives 0.

**HasOccupantModelClass(occupant, classes, count)** (`0x00496AC0`) → bool:

- plain: true when `v[0]` is one of `classes[0 … count−1]` (an empty v gives false);
- otherwise (the prop interface must exist, else false): with one value, true when `v[0]` is in the list; with an even
  count, true when **any** pair's class is in the list, whatever its state; other odd counts false.

So for a prop with state pairs, HasOccupantModelClass ignores the state that GetOccupantModelClass applies
*[quirk]*.

## 5. Conformance tests

1. `GetModelProperty1Instance(0x12340000, 2, 3)` = `0x12340230`.
2. A prop without `0x4A70D491`, state 1 and `0xEA123CEF` = (0, A, 1, B): the class is B; HasOccupantModelClass with
   the list {A} is true.
3. A building whose holder has `0x27812822` and not `0xE9822D62` uses building-style models; one that also has
   `0xE9822D62` does not.
4. A flora occupant gives orientation = its slot 12 and state = its slot 10.
