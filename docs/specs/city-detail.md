# Spec: the city detail manager (`cSC4CityDetailManager`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from observing the original program and its data files. Implementers must work from this spec, the
public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled
code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (§3.6 components, the `Init` order, §6 random numbers),
[lot-developer.md](lot-developer.md) (§6.5 lot props and flora, slot 39 object positions),
[prop-developer.md](prop-developer.md) (optional props), [flora.md](flora.md) (slots 17 and 19),
[lot-records.md](lot-records.md) (lot tags).

## 0. Summary

The city detail manager remembers the **city detail** preference (low, medium, high) the city was last run with.
When a city is opened with a different setting it brings the lots' decorations in line with the configurations
built for the new setting: at a lower setting it removes lot props and lot flora that no longer belong to their
lot's configuration; at a higher one it adds the missing required props and flora. Either way it then asks the
network lot manager to redevelop every network lot. The rest of the game never calls it.

It is mostly a presentation feature; the simulation side is this one-time adjustment in `Init` and the 8-byte
record.

Implementation size: about 120 lines; one clock-seeded temporary generator when detail increases.

## 1. Identity

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`, `0x0045D300`) | `0xEAD4BA36` |
| Component | city component **51**, IID `0x6AD4B9F1`, created by the city if the record did not provide one; the city reads its reference only for city versions ≥ 9 (foundations §3.6) |
| Factory | `0x0045D420` (registered by the COM director, `0x00460521`): global heap, **0x10** bytes |
| Interface at +0 | vtable `0x00A87888` (7 slots, §6) |
| Serializable at +4 | vtable `0x00A87870`, `Write` `0x0045D380`, `Read` `0x0045D3B0` |

`QueryInterface` (`0x0045D280`): `1` and `0x6AD4B9F1` → +0; `0xE4FDA3D4` → +4; otherwise false. The reference count
is at +0xC; the last `Release` calls the deleting destructor (slot 6).

## 2. State

| Offset | Field | Initial | Saved |
|---|---|---|---|
| `+0x08` | detail level the city was last initialised with (u32) | `0xFFFFFFFF` (none) | **yes** |
| `+0x0C` | reference count | 0 | no |

The current setting is the preferences byte at `+0xF16` of the preferences object (global `0x00B43CD8`): 1 low,
2 medium, 3 high; default 3 (`cSC4Preferences` constructor). The video options page writes it. The lot
configuration manager builds lot configurations for the current setting and rebuilds them when it changes
(`StaticInit`, `0x0048A540`), so a configuration's object array depends on it *[inferred: which objects are
dropped at which level is a lot configuration matter]*.

No agents, no messages, no tuning.

## 3. Behaviour

### 3.1 `Init` (slot 3, `0x0045DD10`) → true *[compat]*

`cur :=` the current setting; `saved :=` the stored level.

- `saved = 0xFFFFFFFF` (a new city, or a city saved before this record existed): nothing.
- `cur < saved` (unsigned): **reduce** (§3.2).
- `cur > saved`: **increase** (§3.3).
- Then `stored := cur`.

The city detail manager is the last component the city initialises (foundations §3.6), so every lot, prop, flora
occupant and network lot exists when it runs.

`Shutdown` (slot 4) returns true and does nothing. Slot 5 returns the stored level *[inferred: name
GetCityDetail]*; nothing calls it.

### 3.2 Reduce (`0x0045D690`)

For every lot (lot manager slot 15, the safe lot iterator, in the lot list order):

1. `cfg := lot.GetLotConfiguration()`; none → next lot.
2. `keep :=` the set of object IDs (record +0x24) of the configuration's objects of type 1 (prop) or 4 (flora),
   whatever their flags.
3. For every occupant of the lot (`lot.GetLotOccupants`, lot slot 49) that has a lot tag:
   - flora (`0x74758926`) whose tag is not in `keep` → `flora.DemolishFloraOccupant(occ, 0)` (flora slot 19);
   - prop (`0xA823821E`) whose tag is not in `keep` → `propManager.RemovePropA(occ)` (slot 29).
   Other occupants, and occupants without a tag, are untouched.

Then §3.4.

### 3.3 Increase (`0x0045D9D0`)

One temporary `cRZRandom` is created for the whole pass, seeded with `0xFFFFFFFF` (the wall clock, foundations
§6.2). For every lot, in the same order:

1. `cfg := lot.GetLotConfiguration()`; none → next lot.
2. `need :=` the set of object IDs of the objects of type 1 or 4 whose flag bit 0 is **clear** (required objects;
   optional props are the prop developer's business).
3. Visit every occupant in the lot's cell rectangle (`lot.GetBoundingRect`, lot slot 14; occupant manager slot 46
   `IterateOccupantsByStandardCityCell` with no filter). For each flora or prop occupant with a lot tag, remove the
   tag from `need`.
4. For each configuration object in array order whose ID is still in `need`:
   - `(pos, orientation) := lotDeveloper.GetPositionForLotConfigurationObject(lot, cfg, object)` (lot developer
     slot 39);
   - `ids := lotConfigurationManager.GetObjectIDArray(object)` (slot 27; families expanded, lot-developer.md §6.5);
     empty → next object;
   - `id := ids[RandomUint32Uniform(n)]` (the temporary generator); `id = 0` → next object;
   - type 1: `occ := propManager.AddLotProp(id, pos, orientation)` (prop manager slot 26); type 4:
     `occ := flora.AddNewFloraOccupant(id, pos, false)` (flora slot 17);
   - if `occ` is not null, `occ.SetLotTag(object ID)` (occupant slot 28).

An ID stays in `need` after it is used, so two required objects with the same ID are both created *[quirk]*.

Then §3.4.

### 3.4 Network lots

After either pass, `networkLotManager.IterateAllNetworkLots(callback)` (slot 11) calls
`networkLotManager.RedevelopLot(x, z, networkLot)` (slot 12) for every network lot, in the manager's own iteration
order. Redevelopment is the network lot manager's behaviour (no spec yet); it draws from that manager's file
generator `0x00B44870` (foundations §6.3) *[inferred]*.

### 3.5 Random numbers

Only §3.3 draws: one `RandomUint32Uniform(n)` per created object with a non-empty ID list, from a temporary
`cRZRandom` seeded from the clock. *[choice]* FreeCity4 seeds it with `derive(0xEAD4BA36 XOR n)`, `n` the number of
earlier increase passes in the session (foundations §6.3, temporaries seeded from the clock). Network-lot
redevelopment draws as its owner specifies.

## 4. Persistence *[compat]*

Record class `0xEAD4BA36`, 8 bytes:

| Field | Encoding |
|---|---|
| version | u32 = **0** |
| detail level | u32 |

`Read`: a version other than 0 → false. `Write` always returns true.

Corpus: the record is present in 16 of the 421 saves (the cities that were run and saved by the game; the shipped
templates have none), all `00 00 00 00 03 00 00 00` (high detail).

## 5. Quirks

1. The level is compared and stored only at `Init`: changing the preference while a city is open adjusts nothing
   until the city is opened again.
2. Reduce keys on the configuration's prop and flora object IDs regardless of the optional flag; increase only
   creates required objects.
3. Two required objects with the same object ID are both created on increase even if one exists already.

## 6. Interface slot table *[compat]*

`cISC4CityDetailManager` (no SDK header), vtable `0x00A87888`:

| Slot | Offset | Method | Address | Pops | Return |
|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x0045D280` | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x004664A0` | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x0045D2E0` | 0 | EAX |
| 3 | `+0x0C` | `Init` | `0x0045DD10` | 0 | AL (true) |
| 4 | `+0x10` | `Shutdown` | `0x00648F00` (shared "return true") | 0 | AL |
| 5 | `+0x14` | `GetCityDetail` *[inferred name]* | `0x004777E0` (shared getter of +8) | 0 | EAX |
| 6 | `+0x18` | deleting destructor | `0x0045D350` | 4 | EAX |

Serializable (`0x00A87870`): 0–2 thunks (`0x0045D330`, `0x006C2270`, `0x0045D340`), 3 `Write` `0x0045D380` (pop 4,
AL true), 4 `Read` `0x0045D3B0` (pop 4, AL), 5 `GetGZCLSID` `0x0045D300` (pop 0, EAX).

Who reaches in: only the city (create, `Read`, `Write`, `Init`, `Shutdown`).

## 7. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (CD1–CD2).

## 8. Conformance tests

1. A new city writes `00 00 00 00 03 00 00 00` with the default preference; with preference 1, `… 01 00 00 00`.
2. A record of version 1 is rejected.
3. Saved 3, current 3: `Init` changes no occupant.
4. Saved 3, current 1: a lot whose configuration no longer lists the object with ID T loses its props and flora
   tagged T; untagged props and props tagged with a listed prop or flora object stay.
5. Saved 1, current 3: a lot whose configuration lists required prop objects A and B, with a prop tagged A
   present, gains exactly one prop tagged B at the position the lot developer gives for B; an optional object is
   not created; one draw per created object.
6. Both directions end with every network lot redeveloped once; a new city redevelops none.
