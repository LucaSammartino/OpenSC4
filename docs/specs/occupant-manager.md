# Occupant manager (`cSC4OccupantManager`): behaviour spec

Status: round 1 (re-specs13b, 2026-10-15); OM4–OM8 answered by re-answers16 (2026-10-16). Written from the Windows 1.1.641 executable (code `0x0049D8B0`–`0x004A1C00`).
Every rule below was checked against the instruction bytes. The Mac symbol list was used only for names
(`InternalOccupantHashSet`, `ClipCellRectToWorldBoundaries`, `GetOccupantList`, `RemoveAllOccupants`).

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced, or that a replacement must survive.

Shared material cited and not repeated here:
- streams and object references: foundations §2, §2.7;
- class store and load order: §3.5;
- city component table: §3.6;
- city Init order: §3.7;
- messages: §5;
- overload slot order: §16;
- x87 precision: §18;
- threading model: §19;
- allocator families A, B and C: [docs/abi/allocators.md](../abi/allocators.md);
- `cISC4Occupant::PostOccupantMessage`: flora §9.1.

---

## 1. Role and identity

The occupant manager is the city's **spatial index of occupants**: buildings, props, flora, network pieces, landfill
tiles and every other `cISC4Occupant`. It owns no simulation. It:
- keeps, for each 64 m cell of a fixed 192 × 192 grid, a list of the occupants whose box touches that cell (§4);
- inserts, removes and moves occupants, posting the occupant messages Insert `0x99EF1142`, Remove `0x99EF1143` and
  Move `0x99EF1146` (§5);
- answers point, cell, rectangle and box queries, as a first hit (§7), a list (§8) or a callback walk (§9);
- converts between positions, 64 m "world cells", occupant-manager cells and 16 m standard city cells (§3);
- saves the cell lists as object references (§13).

| Item | Value |
|---|---|
| Class | `cSC4OccupantManager`, object size `0x9C` bytes. Factory `0x004A18A0`, constructor `0x004A0540`, destructor body `0x004A0620`, deleting destructor `0x004A18C0` (slot 49) |
| CLSID | `0x098F964D` (`GetGZCLSID` `0x004A05B0`) |
| IIDs answered | `0x00000001` and `0xC6BB8334` (`cISC4OccupantManager`) → this; `0xE4FDA3D4` (`cIGZSerializable`) → this + 4; `0x452294AA` (`cIGZMessageTarget2`) → this + 8. QueryInterface `0x0049D910`; any other IID returns false |
| City component | #4 in the component table (foundations §3.6), city field `+0x12C`, read with IID `0xC6BB8334` |
| Init order | after the automata controller manager, before the zone manager (foundations §3.7) |
| Vtables | `+0` `0x00A8F7F0` (`cISC4OccupantManager`, 49 slots + deleting destructor); `+4` `0x00A8F7D4` (`cIGZSerializable`); `+8` `0x00A8F7C4` (`cIGZMessageTarget2`) |

The message-target interface exists, but its `DoMessage` is a shared stub (`0x006B8AA0`). The manager subscribes to
nothing and handles no message *[compat]*.

---

## 2. Lifecycle

### 2.1 Creation and city size

`cSC4City::CreateCityComponents` (`0x0045AA10`) creates the manager at `0x0045AAB4`, but only when city `+0x12C`
is still null (a loaded city already has the object from the class store). It then calls `SetCitySize(city+0x100,
city+0x104)` (slot 5) with the city's size in metres.

**SetCitySize(fx, fz)** (slot 5, `0x0049E050`) *[compat]*:
1. Lock.
2. If the manager is already initialised, return **false** and change nothing.
3. Otherwise store the two sizes and the four counts, each truncated toward zero (`_ftol` `0x009EEF04`):
   - manager cell counts: `+0x24` = trunc(fx · 0.015625), `+0x28` = trunc(fz · 0.015625);
   - standard cell counts: `+0x2C` = trunc(fx · 0.0625), `+0x30` = trunc(fz · 0.0625).
4. Return true.

For the three city sizes (1024, 2048 and 4096 m), the manager counts are 16, 32 and 64, and the standard counts
are 64, 128 and 256.

### 2.2 Init (slot 3, `0x004A18E0`)

The city's Init calls it at `0x0045BF2B`. It runs under the lock, and only once:
1. If already initialised, return true.
2. Set the initialised flag (`+0xC`).
3. If the shared filter (`+0x34`) is null, create it (§4.4).
4. If the grid is empty (a new city), build it: 192 rows, each of 192 null cell pointers.
5. Otherwise (the grid was filled by `Read`, §13) call `IterateOccupants` (slot 44) with:
   - callback `0x0049D900`, which calls the occupant's `Init` (occupant slot 3) and returns true;
   - data = the manager;
   - null min, null max and a null filter.

   This initialises every loaded occupant once (§9.2 removes duplicates).
6. Return true.

*[quirk]* Step 5 uses slot 44's defaults, which cover only the city's own cells: world 64 to 63 + count (§3).
A loaded occupant whose cells all lie in the border ring of the grid is not initialised here. Shipped content keeps
occupants inside the city, so this does not happen in practice *[inferred]*.

A loaded occupant posts **no** Create `0x99EF1140` from its `Init`: the base occupant `Read` (`0x005E8D60`) sets
bit 1 of `+0x2E`, and with that bit set the base `Init` calls slot 17 instead of posting (§5.6, §13; corrected by
re-specs14b). Loading posts **no** Insert messages either. An occupant class whose own `Init` posts something is
outside this rule.

### 2.3 Shutdown (slot 4, `0x0049E000`)

Under the lock, when initialised:
1. `RemoveOccupants(null filter, shutdown = true, data4 = 0)` (slot 22): every occupant is removed, gets its Remove
   message and is shut down (§5.3).
2. Release the shared filter and null `+0x34`.
3. Clear the initialised flag.
4. The grid's row vectors are kept; they are all null by then.

It returns true, initialised or not.

The city's Shutdown (`0x0045B826`) calls the manager's Shutdown. It then calls `cISC4App` slot 5,
`RunMessageServer2Pump(100, 500, 5000)` (app global `0x00B43C94`), so that the queued Remove and Destroy messages are
delivered before the manager is released. Finally it releases the manager and nulls city `+0x12C`. The city's
Write stores a reference to it at `0x0045C781`.

### 2.4 Reference count

- `AddRef` (`0x0049DF80`) and `Release` (`0x0049DFB0`) take the lock.
- When `Release` drops the last reference, it calls `Shutdown`, then the deleting destructor (slot 49, `+0xC4`)
  with 1, and returns 0.
- The `cIGZSerializable` and `cIGZMessageTarget2` views forward QI, AddRef and Release to the primary interface
  (`0x004A05C0` / `0x004A05E0` / `0x004A05F0`, `0x004A05D0` / `0x004A0600` / `0x004A0610`).

---

## 3. Coordinate systems and conversions (slots 5–19) *[compat]*

### 3.1 The four systems

| System | Unit | Origin | City range |
|---|---|---|---|
| position | metre (`float`) | city corner (0, 0) | 0 … size |
| **world cell** (the grid index) | 64 m | world cell 0 starts at position −4096 | 64 … 63 + manager count |
| occupant-manager cell | 64 m | = world cell − 64; cell 0 starts at position 0 | 0 … manager count − 1 |
| standard city cell | 16 m | cell 0 starts at position 0 | 0 … standard count − 1 |

The grid always has **192 × 192** world cells (64 rows of border on the low side, then the city, then the rest).
Its size does not depend on the city size: it covers positions −4096 … 8192 on both axes.

### 3.2 Slot table

All take `(in1, in2, out1&, out2&)` unless noted, `__thiscall`, pop 16 (slots 6–8: pop 8).

| Slot | Address | Name (SDK) | Result |
|---|---|---|---|
| 5 | `0x0049E050` | `SetCitySize(fx, fz)` | §2.1. Returns a real bool |
| 6 | `0x0049D990` | `GetOccupantManagerCellSizes(&x, &z)` | 64.0, 64.0 |
| 7 | `0x0049D9B0` | `GetWorldCellCount(&x, &z)` | 192, 192 |
| 8 | `0x0049D9D0` | `GetOccupantManagerCellCount(&x, &z)` | `+0x24`, `+0x28` |
| 9 | `0x0049DB50` | `GetOccupantManagerCellBounds(&x0, &z0, &x1, &z1)` | 64, 64, `+0x24` + 63, `+0x28` + 63: the city's world cells, inclusive |
| 10 | `0x0049D9F0` | `WorldCellToOccupantManagerCell` | x − 64, z − 64 |
| 11 | `0x0049DA10` | `OccupantManagerCellToWorldCell` | x + 64, z + 64 |
| 12 | `0x0049DA30` | `WorldCellToStandardCityCell` | **bug**: outX = 4 · z − 256, outZ = (the value already in outZ) · 4. See §15 quirk 1 and OM3 |
| 13 | `0x0049DA50` | `StandardCityCellToWorldCell` | x / 4 + 64, z / 4 + 64, with C division (rounding toward zero) |
| 14 | `0x0049DA80` | `WorldCellToPosition` | (x − 64) · 64, (z − 64) · 64, as floats: the cell's low corner |
| 15 | `0x0049E0E0` | `PositionToWorldCell(fx, fz, &x, &z)` | §3.3 |
| 16 | `0x0049DAF0` | `OccupantManagerCellToPosition` | x · 64, z · 64 |
| 17 | `0x0049E150` | `PositionToOccupantManagerCell` | as slot 15 without the + 4096 |
| 18 | `0x0049DB20` | `StandardCityCellToPosition` | x · 16, z · 16 |
| 19 | `0x0049E1B0` | `PositionToStandardCityCell` | exact floor(f · 0.0625), §3.4 |

*[quirk]* Slots 6–19 **do not set AL**: the bool they return is whatever was left in AL. No shipped caller tests it
*[inferred]*. A replacement returns true *[choice]*. No conversion checks ranges: out-of-grid inputs give
out-of-grid outputs.

### 3.3 Position to world cell (slot 15) *[compat]*

With the x87 precision control at 24 bits (foundations §18), every step rounds to `float`:

    t = ((f + 4096) · 0.015625) − 0.499          (constants 4096 at 0x00A8F7C0, 0.015625 at 0x00A8F7B8, 0.499 at 0x00A8F7BC)
    cell = round-half-even(t)                    (added to 12582912 = 0x4B400000 at 0x00A8F778; result bits + 0xB4C00000)

This is floor(f / 64 + 64 + 0.001), not plain floor. A position within **0.064 m below** a 64 m boundary already
belongs to the next cell, give or take one float ulp. Slot 17 is the same formula without + 4096. The helper
`0x0049DAC0` is slot 15 with the results stored as bytes; Insert and Move use it (§5).

Values (from the formula above, single precision):

| f | slot 15 | slot 17 |
|---|---|---|
| 0.0 | 64 | 0 |
| 63.9 | 64 | 0 |
| 63.9365 | 65 (first value that switches) | |
| 63.95 | 65 | 1 |
| 64.0 | 65 | 1 |
| −0.064 | 64 | |
| −0.065 | 63 | |
| −0.5 | 63 | −1 |
| 1023.9 | 79 | |
| 1024.0 | 80 | |
| −4096 | 0 | |
| −4200 | −2 (no clamp) | |

### 3.4 Position to standard city cell (slot 19) *[compat]*

t = f · 0.0625 (constant `0x00A8E174`), then n = round-half-even(t − 0.5) (0.5 at `0x00A84D2C`); if t == n + 1, the
result is n + 1, otherwise n. This is an exact floor for every float in range. Examples: 0 → 0, 15.99 → 0,
16 → 1, 32 → 2, −0.1 → −1, −16 → −1.

---

## 4. Data structure

### 4.1 Object layout *[compat for the save record; otherwise internal]*

| Offset | Size | Field |
|---|---|---|
| `+0x00` | 4 | `cISC4OccupantManager` vtable `0x00A8F7F0` |
| `+0x04` | 4 | `cIGZSerializable` vtable `0x00A8F7D4` |
| `+0x08` | 4 | `cIGZMessageTarget2` vtable `0x00A8F7C4` |
| `+0x0C` | 1 | initialised flag |
| `+0x0D` | 1 | "was read" flag: set to 1 by `Read`, never read *[quirk]* |
| `+0x10` | 4 | reference count |
| `+0x14` | 8 | `cRZCriticalSection`, recursive (vtable `+4` Lock, `+8` Unlock) |
| `+0x1C`, `+0x20` | 4 + 4 | city size X, Z (`float`, metres) |
| `+0x24`, `+0x28` | 4 + 4 | manager cell counts (`int32`) |
| `+0x2C`, `+0x30` | 4 + 4 | standard cell counts (`int32`) |
| `+0x34` | 4 | shared one-type filter (§4.4), null before `Init` and after `Shutdown` |
| `+0x38`–`+0x40` | 12 | the grid: an STLport vector {begin, end, capacity} of 192 rows; each row is a vector of 192 cell pointers. Rows are indexed by **x**, cells by **z** |
| `+0x44` | … | the shared `InternalOccupantHashSet`, a hash set of occupant pointers, emptied after each use |

### 4.2 Cells, lists and nodes

- A cell pointer is null (empty cell) or points to a 4-byte **holder** (allocator C, `0x005E55E0`).
- The holder points to the 12-byte head of a circular doubly linked list (allocator B, `0x0090CF54`).
- Each node is 12 bytes, `{next, prev, value}`, allocator B.
- `value` = occupant pointer | bit 0. **Bit 0 = 1** means "this occupant's box is a single cell" (x1 ≤ x0 and
  z1 ≤ z0).
- Queries skip the duplicate check for bit-0 nodes, since such an occupant appears in no other cell. Bit 0 is an
  optimisation hint only. A replacement may always use 0 and run the duplicate check, with identical results.
- When the last node of a cell is removed, the head (`0x005650A0`) and the holder (C, `0x005E5620`) are freed and
  the cell pointer is nulled. Only Remove, RemoveOccupants and Move do this.

**List order *[compat]*.** Insert and Move add nodes at the **front**. Every query walks from the front, so within
a cell the **newest occupant comes first**. `Read` appends at the back, which preserves the saved order (§13).

### 4.3 Occupant box

Every occupant stores its manager box: 4 bytes (x0, z0, x1, z1) of world cells, inclusive.
- `SetOccupantManagerBBox` is occupant slot 26 (`+0x68`).
- `GetOccupantManagerBBox` is occupant slot 25 (`+0x64`). The base version (`0x005E8BF0`) fails when the first byte
  (`+0x28` of the occupant) is `0xFF`, its "no box" mark.
- Remove and Move use the stored box to find the occupant's cells; they never recompute it from the occupant's
  current geometry.

### 4.4 The shared one-type filter

`Init` creates it: 16 bytes from allocator C.
- Base constructor `0x005EA080`.
- Vtables `0x00A8F79C` (`cISC4OccupantFilter`: QI, AddRef `0x004BDF00`, Release `0x007CB920`, the three methods
  below, deleting destructor `0x006C4A00`, which frees with C) and `0x00A8F784` (a `cIGZSerializable` view at +4:
  Write `0x005E9890`, Read `0x005E98E0`, `GetGZCLSID` `0x005E9730` = `0xE9C75ED1`).
- The reference count is at `+0x8` and the type at `+0xC`, initially 0.
- The manager holds one reference. The filter is never saved by the manager.

Methods:

| Method | Address | Result |
|---|---|---|
| `IsOccupantIncluded(o)` | `0x004BDF90` | `IsOccupantTypeIncluded(o.GetType())` (occupant slot 7) |
| `IsOccupantTypeIncluded(t)` | `0x005E9740` | t == type |
| `IsPropertyHolderIncluded(p)` | `0x004FC900` | true |
| QueryInterface | `0x005E95D0` | 1 and `0x46A80026` → this; `0xE4FDA3D4` → this + 4 |

The typed overloads write their type into this one object and pass it on (§10). The object is shared, so it is
**not re-entrant** (§14).

---

## 5. Insert, remove, move

### 5.1 InsertOccupant(occ, data4) (slot 20, `0x0049EF60`, pop 8) *[compat]*

1. If `occ` is null, return false.
2. If `occ.IsFlagSet(1)` (occupant slot 32), the occupant is already inserted: return false. This check takes no
   lock and no reference.
3. Lock. `occ.AddRef()`. Call `occ.Init()` (occupant slot 3) before any other check. A not-yet-initialised
   occupant posts Create `0x99EF1140` here (§5.6), **even if the insert then fails**.
4. If `occ.AsPropertyHolder()` (slot 6) is null: unlock, `occ.Release()`, return false.
5. `occ.GetBoundingBox(min, max)` (slot 10). For each of x and z separately: if `max − min > 1.0` (strict), then
   `min += 0.1` and `max −= 0.1`. A box 1 m wide or less is not shrunk.
   - *[compat]* Both constants are `float` (OM4): 0.1 is `0x3DCCCCCD` at `0x00A8F768`, 1.0 is `0x3F800000` at
     `0x00A81228`, each read as a 4-byte operand (`0x0049EFEA`–`0x0049F040`).
   - Each shrunk bound is stored back as a `float`, so the shrink is a single-precision add or subtract.
   - The width `max − min` is compared without being stored. Under the 24-bit precision control (foundations §18)
     it is a single-precision subtraction; compute it in `f32`.
   - Move (`0x0049F43A`–`0x0049F490`) and the FindOccupant helper (`0x0049F821`–`0x0049F846`, x only) use the same
     two constants.
6. Cells: (x0, z0) = slot 15(min.x, min.z), (x1, z1) = slot 15(max.x, max.z), each stored as a byte, **with no
   clamp** (§15 quirk 4). Out-of-grid boxes are §5.7.
7. `occ.SetOccupantManagerBBox(x0, z0, x1, z1)` (slot 26).
8. value = occ | (1 if x1 ≤ x0 and z1 ≤ z0).
9. For x from x0 to x1, then z from z0 to z1 (x outer):
   - if the cell pointer is null, create a holder and an empty list. If the holder allocation fails, the cell is
     skipped.
   - **in the first cell only**, look for a node with this occupant. If one is found: unlock, `occ.Release()`,
     return false. The box written in step 7 stays *[quirk]*. Step 2 makes this path unreachable in practice.
   - add the node at the **front**.
10. Unlock.
11. `occ.SetFlag(1)` (slot 29).
12. `occ.PostOccupantMessage(0x99EF1142, data4)` (slot 15): Insert, queued, data3 = occupant type, data4 = the
    argument (flora §9.1).
13. Return true. The manager keeps the reference taken in step 3.

Known data4 values (ZQ14, answered by re-answers11 in zones.md §5.2):
- the network tool passes its field `+0x208`: 1 for a network the player draws, 3 inside `PlaceNetworkOccupants`
  (every network laid by code);
- 0 from `InsertOccupantsForSolvedCells`, building development, the landfill and the other managers;
- nothing passes 2.

### 5.2 RemoveOccupant(occ, shutdown, data4) (slot 21, `0x0049F240`, pop 12) *[compat]*

1. If `occ` is null, return false. Nothing else happens; `Shutdown` is not called.
2. `occ.AddRef()`, a local hold for the whole call.
3. If `!occ.IsFlagSet(1)` (not inserted): go to step 8. The result is false.
4. Lock. If `occ.AsPropertyHolder()` is null or `occ.GetOccupantManagerBBox` fails: unlock, go to step 8, result
   false. Flag 1 stays set.
5. For each cell (x outer, z inner) of the **stored** box: if the cell has a list, unlink and free (B) the first
   node whose value without bit 0 is `occ`. If the list is now empty, free the head and holder and null the cell.
   A cell without a node for `occ` is skipped silently.
6. The result is true, even if no cell held the occupant. Unlock.
7. In this order:
   1. `occ.PostOccupantMessage(0x99EF1143, data4)`: Remove, queued;
   2. `occ.ClearFlag(1)` (slot 31);
   3. `occ.Release()`: the manager's own reference from Insert.
8. If `shutdown`: `occ.Shutdown()` (slot 4). This happens **whether or not the remove succeeded** *[quirk]*.
9. `occ.Release()` (the local hold). Return the result.

So a successful remove with shutdown produces, in this order: Remove `0x99EF1143`, then Destroy `0x99EF1141`,
posted from the occupant's `Shutdown` (§5.6).

### 5.3 RemoveOccupants(filter, shutdown, data4) (slot 22, `0x0049FCD0`, pop 12) *[compat]*

The Mac name is `RemoveAllOccupants`. It has no init check. Before `Init` the grid has no rows, so the call reads
past an empty vector *[quirk]*.

1. Lock.
2. Walk the grid: x from 0 to 191 (outer), z from 0 to 191. For each cell with a list:
   1. Collect, in list order, the occupants that the filter includes (filter slot 3; a null filter includes all)
      into a scratch vector (allocator A, cached service `0x00B21750`).
   2. For each collected occupant that has a property holder and a stored box:
      - remove its node from **every** cell of its box (as §5.2 step 5);
      - append it to a local "removed" list (allocator A, `0x00B207E4`);
      - `ClearFlag(1)` at once, **inside the lock**.

   An occupant that spans several cells is removed when its first cell in this x-major walk is visited, so it is
   collected once.
3. Unlock.
4. For each removed occupant, in removal order:
   1. `PostOccupantMessage(0x99EF1143, data4)`;
   2. if `shutdown`, `Shutdown()`;
   3. `Release()`: the manager's reference.

   The messages therefore interleave per occupant: Remove(o1), Destroy(o1), Remove(o2), Destroy(o2), …
5. Free the removed list. Return **true**, always.

Unlike slot 21, flag 1 is cleared **before** the Remove message is posted (and before the lock is dropped). Both
orders are invisible to message handlers, because the message is queued (flora §9.1).

**Slot 23**, `RemoveOccupants(type, shutdown, data4)` (`0x0049DB80`, pop 12):
- writes `type` into the shared filter and tail-jumps to slot 22 with it;
- it has no init check and no type-0 special case;
- type 0 removes only occupants whose `GetType()` is 0, which in practice is none;
- *[quirk]* before `Init` or after `Shutdown` the filter pointer is null, and the write crashes.

### 5.4 MoveOccupant(occ, post) (slot 24, `0x0049F3C0`, pop 8) *[compat]*

Call it after an occupant's geometry has changed. It takes no reference.

1. If `occ` is null or `!occ.IsFlagSet(1)`, return false.
2. Lock.
3. If there is no property holder, or `GetOccupantManagerBBox` (the **old** box) fails: unlock, return false.
4. Compute the new box from `GetBoundingBox` exactly as Insert steps 5–6 do, and `SetOccupantManagerBBox(new)`.
5. If new == old (all four bytes), the result is true; go to step 9.
6. Remove the node from every cell of old that is not in new. Empty cells are freed as in §5.2. There is no
   null-cell check, which is safe because the occupant is in those cells.
7. Add a node at the **front** of every cell of new that is not in old. The cell lookup `0x0049EE80` (create = 1)
   clamps the indices to 0…191 and creates missing holders. There is no duplicate check.
8. If the single-cell bit changed, rewrite the value of the nodes in the cells common to old and new.
9. Unlock. If `post`: `occ.PostOccupantMessage(0x99EF1146, 0)`: Move, data4 = 0. Per flora §9.1, Move is **sent
   synchronously** when occupant `+0x30` is non-null, and otherwise posted. This happens even when the box did not
   change.
10. Return true.

*[compat]* A moved occupant goes to the front of its **new** cells only. In cells it already occupied, it keeps its
place. Callers that need a Change message post `0x99EF1145` themselves (landfill §4, prop developer).

Step 6 indexes the grid with the old box's raw bytes: no clamp, and no null check on the cell (`0x0049F561`–
`0x0049F56E`). Step 8 uses the clamping lookup with create = 1, so it can create an empty holder in a common cell
that has none.

### 5.7 Uninitialised manager and out-of-grid boxes (OM6) *[compat where defined]*

The original checks none of these cases. Where it has no defined result (a crash or a wild memory access), the
replacement's behaviour is a *[choice]*; the choices below are the ones accepted for OM6.

**Before `Init`.** The constructor (`0x004A0540`) leaves the grid vector null (`+0x38`–`+0x40` = 0).
- Insert has no init check. It still runs steps 1–7, so the occupant is AddRef'd, an uninitialised occupant posts
  Create, and the box is stored. The first cell lookup then reads through the null row array (`0x0049F0F7`) and
  crashes, with the lock held.
- *[choice]* The replacement inserts no nodes.
- **After `Shutdown`** the rows still exist (§2.3), so Insert, Remove and Move work normally on the original even
  though the initialised flag is clear.

**Out-of-grid boxes.** The stored box holds bytes (§4.3), so a cell below 0 wraps to cell + 256 (−1 → 255), and a
cell from 192 to 255 stays as it is. Insert (`0x0049F0BC`–`0x0049F145`), Remove (`0x0049F2A4`–`0x0049F2D6`) and Move step 6
index the grid with these bytes unchecked.
- **Wrapped box.** When x0 > x1 or z0 > z1 as bytes, the loops run zero times (`0x0049F0C8`, `0x0049F0EA`).
  - Example: a low corner below position −4096 gives byte 255, while the high corner gives 0 or 1.
  - Insert then succeeds with no nodes: flag 1 is set and Insert is posted.
  - Remove visits no cells, returns true and posts Remove.
- **Overrun.** A byte from 192 to 255 inside a running loop reads past the 192-entry row array (or a row's
  192 cells) into heap memory. It may write a new holder there. The result is undefined.
  - *[choice]* The replacement skips such cells. Insert still sets flag 1 and posts Insert.

**Slot 23 before `Init` or after `Shutdown`.** The shared filter is null. The type write goes to address `0xC`
(`0x0049DB87`) and crashes. *[choice]* The replacement returns true and does nothing.

**Slot 28 outside the grid.** The world cell from slot 15 is used unclamped (`0x0049E511`–`0x0049E522`), so the
read lands outside the row arrays and the result is undefined. The same holds for slot 32, through the cell of its
low corner (§7). *[choice]* The replacement finds nothing and returns false.

### 5.5 Message summary *[compat]*

| Message | ID | Posted by | data4 | When |
|---|---|---|---|---|
| Create | `0x99EF1140` | occupant `Init` (base `0x005E8C90`) | 0 | the first `Init` of a new occupant, from Insert step 3. Not after a load: a loaded occupant has bit 1 of `+0x2E` set (§13) |
| Insert | `0x99EF1142` | Insert, step 12 | the caller's | after the lock is dropped and flag 1 is set |
| Remove | `0x99EF1143` | Remove step 7.1; RemoveOccupants step 4.1 | the caller's | after the cells are updated and the lock is dropped |
| Destroy | `0x99EF1141` | occupant `Shutdown` (base `0x005E8980`) | 0 | only when Remove or RemoveOccupants is called with `shutdown`, after Remove |
| Move | `0x99EF1146` | Move step 9 | 0 | only with `post` |

All are built by `PostOccupantMessage`:
- void1 and the counted IGZUnknown = the occupant;
- data2 = `IsFlagSet(4)`;
- data3 = `GetType()`.

Every message except a synchronous Move is queued, so a handler sees the manager's state as it is when the queue
is pumped, not as it was when the message was posted. Classes that override `PostOccupantMessage` (buildings
`0x00458D10`, and `0x0065A980`, `0x00654180`) post a second, class-specific message after the standard one
(flora §9.1).

### 5.6 The occupant side

- **Base `Init`** (`0x005E8C90`):
  - if already initialised (occupant slot 5), return true;
  - otherwise set bit 0 of occupant `+0x2E`;
  - unless bit 1 of `+0x2E` is set, post Create `0x99EF1140` with data4 0;
  - with bit 1 set, call occupant slot 17 (`+0x44`) with (0, 0) instead;
  - return true.
- **Base `Shutdown`** (`0x005E8980`):
  - if initialised, clear bit 0 and post Destroy `0x99EF1141` with data4 0;
  - return true.

The init bit and flag 1 ("inserted") are different flags. Flag 1 means "in the occupant manager" (DM4, answered by
re-answers11 in demolition.md §4.6; this section agrees).
- Insert refuses an occupant that has it, and sets it after success.
- Remove clears it after its post.
- RemoveOccupants clears it inside the lock.
- Nothing else in the manager writes it.

---

## 6. Cell helpers

### 6.1 IsCellEmpty(x, z) (slot 25, `0x0049E490`, pop 8)

Returns true when x or z is outside 0…191, when the cell pointer is null, or when the cell's list is empty. Indices
are **world cells**. It takes no lock and does no init check; before `Init` the grid has no rows *[quirk]*.

### 6.2 GetBoundingCells(x, z, cells[8][2], &n) (slot 26, `0x0049DBA0`, pop 16)

This slot does not touch the grid. It writes the eight neighbours of (x, z), in this order:

    (x−1, z−1), (x−1, z), (x−1, z+1), (x, z−1), (x, z+1), (x+1, z−1), (x+1, z), (x+1, z+1)

- **Fast path:** when 1 ≤ x ≤ 190 and 1 ≤ z ≤ 190, it writes all eight and n = 8.
- **Otherwise:** it uses the same order, skipping neighbours outside 0…191. The cell itself is never written.

It stores n through the reference and also returns n in EAX.

### 6.3 FindOccupant(occ, searchAll, &x, &z) (slot 27, `0x0049FB30`, pop 16)

1. Lock.
2. The helper `0x0049F7C0` picks a guess cell:
   - the stored box's corner (x0, z0) when `GetOccupantManagerBBox` succeeds;
   - otherwise a cell from `GetBoundingBox`, with the X shrink only;
   - with no property holder the guess is uninitialised stack data *[quirk]*: FindOccupant passes two of its own
     locals, which it never sets (`0x0049FB51`–`0x0049FB5E`).

   The guess is clamped to 0…191, and the helper returns that cell's holder. FindOccupant then looks for the
   occupant in that cell's list.

   Insert (step 4) and Move (step 3) never place an occupant without a property holder; only `Read` can. So for
   such an occupant the fast path practically never hits. The replacement skips the guess and goes to step 4
   (OM6, accepted).

   Neither `occ` nor the init flag is checked: a null occupant crashes in the helper, and a full scan before
   `Init` reads through the null grid.
3. If it does, return true **without writing x and z** *[quirk]*. The caller must already know the position, or
   pre-set x and z to the box corner.
4. Otherwise, if `searchAll`: scan the whole grid, x-major from (0, 0). At the first cell holding the occupant,
   write x and z (world cells; each only when its pointer is non-null, `0x0049FBFB`/`0x0049FC05`) and return true.
5. Otherwise return false. Unlock on every path.

---

## 7. First-occupant queries (slots 28–35) *[compat]*

Each overload returns the **first** match in list order (§4.2, newest first) and walks cells in the order given.
On a hit it does `AddRef`, writes `*out` and returns true: **the caller owns one reference**. On a miss it returns
false and leaves `*out` untouched. Each takes the lock and returns false when not initialised (checked after
locking). Pop 16.

| Slot | Address | Overload | Cells walked | Test |
|---|---|---|---|---|
| 28 | `0x0049E4D0` | `GetFirstOccupantByPosition(out, fx, fz, filter)` | the one world cell slot 15(fx, fz), **not clamped** *[quirk]*; outside the grid see §5.7 | filter, then the point is inside the occupant's box: min.x ≤ fx < max.x and min.z ≤ fz < max.z |
| 30 | `0x0049E5E0` | `GetFirstOccupant(out, x, z, filter)` | world cell (x, z); outside 0…191 → false | filter only |
| 32 | `0x0049E690` | `GetFirstOccupantByStandardCityCell(out, x, z, filter)` | the one world cell slot 15(x·16 + 0.1, z·16 + 0.1), **not clamped** (`0x0049E736`). The box x·16 + 0.1 … x·16 + 16 − 0.1 (same for z) always lies in that cell, so this equals walking the box's cells | filter, then strict box overlap (§8.2) with that box, on x and z |
| 34 | `0x0049E800` | `GetFirstOccupantByStandardCityCells(out, min*, max*, filter)` | min and max are standard cells. A non-null corner is clamped to **0…count − 1** on each axis (min `0x0049E827`–`0x0049E868`, max `0x0049E876`–`0x0049E8B7`; OM5). Null min = (0, 0); null max = **(count X, count Z)**, unclamped, one cell past the city edge as in slot 40. The box is min·16 + 0.1 … (max+1)·16 − 0.1, and its world cells (slot 15 of the two corners, not clamped) are walked x outer, z inner | filter, then strict box overlap on x and z |

The 0.1 of slots 32, 34, 40 and 47 is the same `float` constant as Insert's (`0x00A8F768`, OM4; slot 32
`0x0049E6FA`–`0x0049E72C`, slot 34 `0x0049E90B`–`0x0049E93D`, slot 40 `0x0049E2F8`–`0x0049E34D`, slot 47
`0x0049E418`–`0x0049E46D`). The cell corners are converted to `float` exactly (an integer times 16). Each bound is
stored as a `float` after one add or subtract, so these are single-precision operations.

The **typed** thunks 29, 31, 33 and 35 (`0x0049DC80`, `0x0049DCB0`, `0x0049DCE0`, `0x0049DD10`):
- return false if not initialised;
- otherwise write the 4th argument into the shared filter's type, **including 0**, and tail-jump to the filter
  overload (slot − 1) with the shared filter;
- type 0 therefore finds only occupants whose `GetType()` is 0 (§10).

The test of slot 28 uses `GetBoundingBox` (occupant slot 10), the occupant's real box, not its cell box.

---

## 8. List queries (slots 36–41) *[compat]*

### 8.1 Common contract

`(list&, min, max, filter-or-type, maxCount)`, pop 20.
- The list is an `ilist<cISC4Occupant*>`: allocator A, 12-byte head and nodes (§12).
- The call does **not** clear the list. Each match is AddRef'd and appended at the back.
- The call stops at once when the number appended **in this call** reaches `maxCount`. Callers pass −1 for "all"
  or 1 for "any" (demolition §4.7, area mode).
- Return value:
  - false, list untouched, when not initialised or `maxCount` is 0;
  - otherwise **true, whether or not anything was appended**. This corrects allocators.md, which says "true when
    something was appended" (§17).
- Order of results: cells x outer, z inner; within a cell, list order (newest first).
- An occupant spanning several cells appears **once**, at its first cell in that walk.

### 8.2 GetOccupantsByBBox(list, min3*, max3*, filter, maxCount) (slot 36, `0x004A0670`)

1. Lock. Not initialised or `maxCount` == 0 → unlock, false.
2. Defaults:
   - null min → (0, −FLT_MAX, 0);
   - null max → (size X, +FLT_MAX, size Z).
3. Cells: slot 15 of (min.x, min.z) and of (max.x, max.z), each clamped to 0…191.
4. **Single-cell query** (both corners in one cell): for each node in list order, apply the filter, then the
   overlap test.
5. **Multi-cell query:** for each node, first the duplicate check through the shared hash set `+0x44`. The check is
   skipped for bit-0 nodes. An occupant already seen is skipped; otherwise it is inserted into the set. Then the
   filter, then the overlap test.
6. Overlap (`0x0049D8B0`; inline at `0x004A08CC`–`0x004A0938`), with the occupant's `GetBoundingBox` (omin, omax):
   **omin < qmax and qmin < omax**, on x, y and z.
   - Strict: boxes that only touch do not overlap.
   - A degenerate query (qmin == qmax) finds an occupant only when the query lies strictly inside it on that axis
     (LD1).
   - A NaN coordinate makes a comparison false, and the code keeps the occupant on that path, so NaN passes
     *[quirk]*.
7. On a match: `AddRef`, append (node from A via `0x00B207E4`), count += 1; stop when count ≥ `maxCount`.
8. Empty the shared hash set, unlock, return true.

The duplicate check runs **before** the filter. An occupant rejected by the filter in one cell is not tested again
in another; the result is the same, because the filter does not depend on the cell.

### 8.3 GetOccupantsByOccupantManagerCells(list, int2* min, int2* max, filter, maxCount) (slot 38, `0x004A0A70`)

The arguments are **world cells**, despite the name.
- Null min → (64, 64); null max → (count X + 63, count Z + 63): the city's cells.
- Both corners are clamped to 0…191, and the range is **inclusive**.
- Filter and duplicate check only, with no geometry test.
- Same contract as slot 36 otherwise.

### 8.4 GetOccupantsByStandardCityCells(list, int2* min, int2* max, filter, maxCount) (slot 40, `0x0049E250`)

1. Not initialised → false. This slot takes no lock of its own; slot 36 locks.
2. min: null → (0, 0); otherwise clamped to 0…count − 1.
3. max: null → **(count X, count Z)**; otherwise clamped to 0…count − 1 *[quirk]*. A null max therefore reaches one
   standard cell past the city edge. Slot 36 clamps the grid indices, so this is harmless.
4. Box, built with slot 18 (×16):
   - x from min.x·16 + 0.1 to (max.x + 1)·16 − 0.1, and the same for z;
   - y from −FLT_MAX to +FLT_MAX.
5. Call slot 36 **through the vtable** (`call [vtbl+0x90]` at `0x0049E357`) with the same filter and `maxCount`.

The 0.1 m shrink with the strict test means: an occupant is returned exactly when its box reaches **more than
0.1 m** into the rectangle of standard cells (NG39).

### 8.5 Typed thunks 37, 39, 41

`0x0049DD40`, `0x0049DD80`, `0x0049DDC0`, pop 20. The type is the 4th argument.
- Not initialised → false.
- Type ≠ 0 → write it into the shared filter and tail-jump to slot 36, 38 or 40 with the shared filter.
- Type 0 → tail-jump with a **null filter**: everything matches (§10).

---

## 9. Iteration (slots 42–48) *[compat]*

The callback is `bool cdecl fn(cISC4Occupant*, void* data)`. Returning false stops the walk. The occupant passed to
the callback is held by the manager for the duration of the call; a callback that keeps it must `AddRef`.

### 9.1 IterateOccupantsByBBox(fn, data, min3*, max3*, filter) (slot 42, `0x004A0D70`, pop 20)

1. The init check comes **before** any lock. Not initialised → return **true** and do nothing *[quirk]*.
2. min and max must not be null; they are dereferenced unchecked.
3. Cells as slot 36 (slot 15, clamped).
4. Multi-cell walks use a **local** hash set (`0x0049FA80`), not the shared one.
5. Per cell, in x-major order:
   1. lock;
   2. collect the matches (filter, overlap test as §8.2), each AddRef'd, into a temporary list (A);
   3. unlock;
   4. for each collected occupant: call `fn` if the continue flag is still set, then `Release` the occupant. After a
      false return, the remaining occupants of this batch are only released;
   5. stop if the flag is clear.
6. Return true.

Callbacks run **unlocked**. They may insert, remove or move occupants. Changes to cells not yet visited are seen by
the walk; an occupant removed after it was collected is still called, because it is held.

### 9.2 IterateOccupants(fn, data, int3* min, int3* max, filter) (slot 44, `0x004A1360`, pop 20)

1. Take the **outer lock**. It is held for the whole walk, callbacks included. Not initialised → go to step 5.
2. min and max are **three-int vectors**: x = element 0, z = element 2, and element 1 is ignored. Null min →
   (64, 64); null max → (count + 63). Values are clamped to 0…191, inclusive. They are world cells.
3. **Single-cell walk** (min cell == max cell, `0x004A1438`–`0x004A1442`):
   1. take the inner lock;
   2. collect every node that passes the filter, in list order. There is **no duplicate check**. Each collected
      occupant is AddRef'd into a temporary list (allocator A);
   3. drop the inner lock;
   4. call `fn` for each collected occupant while the continue flag is set, and `Release` each one.
4. **Multi-cell walk**, x outer, z inner (`0x004A1640`–`0x004A17BB`). Per cell:
   1. take the inner lock;
   2. for each node, in list order:
      - run the duplicate check against a **local** hash set, unless the node has bit 0 set (`0x004A1689`–
        `0x004A1695`; OM7);
      - then the filter;
      - each match is AddRef'd into the batch list;
   3. drop the inner lock;
   4. call `fn` for each match while the continue flag is set. After a false return, the rest of the batch is only
      released;
   5. free the batch, and stop when the flag is clear.

   No geometry test is made.
5. Drop the outer lock and return true, initialised or not; uninitialised walks find no cells.

So slot 44 has slot 42's batch structure. The only difference is that slot 44 keeps the outer lock across the
callbacks.
- A callback runs under that lock, on the same thread, so it does not deadlock.
- It may insert, remove or move occupants, including the one it was given, because the batch holds references.
  Earlier text said that unlinking the visited node invalidates the walk; that was wrong.
- Changes to cells not yet visited are seen by the walk.
- The manager's own use (§2.2) only calls `Init`.

*[compat]* OM7: skipping the duplicate check for bit-0 nodes never changes the result, because a single-cell
occupant has nodes in one cell only. A replacement that checks every node returns the same sequence.

### 9.3 IterateOccupantsByStandardCityCell(fn, data, int2* min, int2* max, filter) (slot 47, `0x0049E370`)

Not initialised → false (`0x0049E487`), unlike slot 42. The min/max conversion is the same as slot 40 (§8.4),
including the null-max quirk. It then calls slot 42 through the vtable (`call [vtbl+0xA8]` at `0x0049E477`) and
returns its result.

### 9.4 Typed thunks 43, 45, 46

`0x0049DE00` (→ 42), `0x0049DE40` (→ 44), `0x0049DE80` (→ **47**), pop 20. The type is the **5th** argument.
- Not initialised → false. Note that slots 42 and 44 themselves return true in that case.
- Type ≠ 0 → shared filter with that type; type 0 → null filter.

*[compat]* **For this last pair the typed overload comes first:** slot 46 is typed and slot 47 takes the filter.
Every other pair has the filter overload first.
- The SDK declares the typed overload before the filter overload for all pairs. Under MSVC's reversal rule
  (foundations §16), that gives the right layout for every pair except this one.
- An SDK plugin that calls the filter overload of `IterateOccupantsByStandardCityCell` reaches slot 46: its filter
  pointer is taken as a type, and nothing matches.
- A replacement must place them as the game does.

### 9.5 ReleaseOccupantList(list) (slot 48, `0x0049EF30`, pop 4)

1. `Release` every element once (occupant slot 2).
2. Free every node with A (`0x00413E60`), leaving the head self-linked.
3. Do **not** free the head.

It takes no lock. Use it only on lists filled by slots 36–41 or by other A lists with one reference per element
(allocators.md).

---

## 10. What "type" means in each typed slot *[compat]*

| Slots | Type argument | Init check | Type 0 |
|---|---|---|---|
| 23 (RemoveOccupants) | 1st | none; crashes when uninitialised | filter with type 0 (matches nothing in practice) |
| 29, 31, 33, 35 (first occupant) | 4th | false | filter with type 0 |
| 37, 39, 41 (lists) | 4th | false | **null filter: all occupants** |
| 43, 45, 46 (iteration) | 5th | false | **null filter: all occupants** |

Every non-zero type is written into the one shared filter (`+0x34`, type at `+0xC`). The value persists after the
call; nothing resets it.

---

## 11. Slot table *[compat]*

`cISC4OccupantManager`, vtable `0x00A8F7F0`, compiled order. "L" = takes the manager lock. "I" = what happens when
the manager is not initialised.

| Slot | Off | Address | Method | Pop | Return | L | I |
|---|---|---|---|---|---|---|---|
| 0 | +0x00 | `0x0049D910` | QueryInterface(iid, void**) | 8 | bool | | |
| 1 | +0x04 | `0x0049DF80` | AddRef | 0 | count | L | |
| 2 | +0x08 | `0x0049DFB0` | Release | 0 | count | L | |
| 3 | +0x0C | `0x004A18E0` | Init | 0 | true | L | |
| 4 | +0x10 | `0x0049E000` | Shutdown | 0 | true | L | no-op |
| 5 | +0x14 | `0x0049E050` | SetCitySize(fx, fz) | 8 | bool | L | works; refused once initialised |
| 6 | +0x18 | `0x0049D990` | GetOccupantManagerCellSizes(&f, &f) | 8 | AL undefined | | |
| 7 | +0x1C | `0x0049D9B0` | GetWorldCellCount(&n, &n) | 8 | AL undefined | | |
| 8 | +0x20 | `0x0049D9D0` | GetOccupantManagerCellCount(&n, &n) | 8 | AL undefined | | |
| 9 | +0x24 | `0x0049DB50` | GetOccupantManagerCellBounds(&, &, &, &) | 16 | AL undefined | | |
| 10 | +0x28 | `0x0049D9F0` | WorldCellToOccupantManagerCell | 16 | AL undefined | | |
| 11 | +0x2C | `0x0049DA10` | OccupantManagerCellToWorldCell | 16 | AL undefined | | |
| 12 | +0x30 | `0x0049DA30` | WorldCellToStandardCityCell (bug, §3.2) | 16 | AL undefined | | |
| 13 | +0x34 | `0x0049DA50` | StandardCityCellToWorldCell | 16 | AL undefined | | |
| 14 | +0x38 | `0x0049DA80` | WorldCellToPosition | 16 | AL undefined | | |
| 15 | +0x3C | `0x0049E0E0` | PositionToWorldCell | 16 | AL undefined | | |
| 16 | +0x40 | `0x0049DAF0` | OccupantManagerCellToPosition | 16 | AL undefined | | |
| 17 | +0x44 | `0x0049E150` | PositionToOccupantManagerCell | 16 | AL undefined | | |
| 18 | +0x48 | `0x0049DB20` | StandardCityCellToPosition | 16 | AL undefined | | |
| 19 | +0x4C | `0x0049E1B0` | PositionToStandardCityCell | 16 | AL undefined | | |
| 20 | +0x50 | `0x0049EF60` | InsertOccupant(occ, data4) | 8 | bool | L | no check; crash before Init *[quirk]* |
| 21 | +0x54 | `0x0049F240` | RemoveOccupant(occ, shutdown, data4) | 12 | bool | L | |
| 22 | +0x58 | `0x0049FCD0` | RemoveOccupants(filter, shutdown, data4) | 12 | true | L | crash before Init |
| 23 | +0x5C | `0x0049DB80` | RemoveOccupants(type, shutdown, data4) | 12 | true | → 22 | crash (null filter) |
| 24 | +0x60 | `0x0049F3C0` | MoveOccupant(occ, post) | 8 | bool | L | |
| 25 | +0x64 | `0x0049E490` | IsCellEmpty(x, z) | 8 | bool | | crash before Init |
| 26 | +0x68 | `0x0049DBA0` | GetBoundingCells(x, z, int[8][2], &n) | 16 | int n | | |
| 27 | +0x6C | `0x0049FB30` | FindOccupant(occ, searchAll, int*, int*) | 16 | bool | L | |
| 28 | +0x70 | `0x0049E4D0` | GetFirstOccupantByPosition(out, fx, fz, filter) | 16 | bool | L | false |
| 29 | +0x74 | `0x0049DC80` | GetFirstOccupantByPosition(out, fx, fz, type) | 16 | bool | → 28 | false |
| 30 | +0x78 | `0x0049E5E0` | GetFirstOccupant(out, x, z, filter) | 16 | bool | L | false |
| 31 | +0x7C | `0x0049DCB0` | GetFirstOccupant(out, x, z, type) | 16 | bool | → 30 | false |
| 32 | +0x80 | `0x0049E690` | GetFirstOccupantByStandardCityCell(out, x, z, filter) | 16 | bool | L | false |
| 33 | +0x84 | `0x0049DCE0` | GetFirstOccupantByStandardCityCell(out, x, z, type) | 16 | bool | → 32 | false |
| 34 | +0x88 | `0x0049E800` | GetFirstOccupantByStandardCityCells(out, min*, max*, filter) | 16 | bool | L | false |
| 35 | +0x8C | `0x0049DD10` | GetFirstOccupantByStandardCityCells(out, min*, max*, type) | 16 | bool | → 34 | false |
| 36 | +0x90 | `0x004A0670` | GetOccupantsByBBox(list, min3*, max3*, filter, maxCount) | 20 | bool | L | false |
| 37 | +0x94 | `0x0049DD40` | GetOccupantsByBBox(list, min3*, max3*, type, maxCount) | 20 | bool | → 36 | false |
| 38 | +0x98 | `0x004A0A70` | GetOccupantsByOccupantManagerCells(list, int2*, int2*, filter, maxCount) | 20 | bool | L | false |
| 39 | +0x9C | `0x0049DD80` | GetOccupantsByOccupantManagerCells(…, type, maxCount) | 20 | bool | → 38 | false |
| 40 | +0xA0 | `0x0049E250` | GetOccupantsByStandardCityCells(list, int2*, int2*, filter, maxCount) | 20 | bool | via 36 | false |
| 41 | +0xA4 | `0x0049DDC0` | GetOccupantsByStandardCityCells(…, type, maxCount) | 20 | bool | → 40 | false |
| 42 | +0xA8 | `0x004A0D70` | IterateOccupantsByBBox(fn, data, min3*, max3*, filter) | 20 | true | per cell | **true** |
| 43 | +0xAC | `0x0049DE00` | IterateOccupantsByBBox(fn, data, min3*, max3*, type) | 20 | bool | → 42 | false |
| 44 | +0xB0 | `0x004A1360` | IterateOccupants(fn, data, int3*, int3*, filter) | 20 | true | whole walk | **true** |
| 45 | +0xB4 | `0x0049DE40` | IterateOccupants(fn, data, int3*, int3*, type) | 20 | bool | → 44 | false |
| 46 | +0xB8 | `0x0049DE80` | IterateOccupantsByStandardCityCell(fn, data, int2*, int2*, **type**) | 20 | bool | → 47 | false |
| 47 | +0xBC | `0x0049E370` | IterateOccupantsByStandardCityCell(fn, data, int2*, int2*, **filter**) | 20 | true | via 42 | false (`0x0049E487`) |
| 48 | +0xC0 | `0x0049EF30` | ReleaseOccupantList(list) | 4 | AL undefined *[inferred]* | | |
| 49 | +0xC4 | `0x004A18C0` | (deleting destructor, flags) | 4 | this | | |

Slot 20 "I": Insert has no init check. Before `Init` the grid has no rows, and the first cell lookup reads past an
empty vector.

The other two vtables:
- `cIGZSerializable` `0x00A8F7D4`:
  - QueryInterface `0x004A05C0`, AddRef `0x004A05E0`, Release `0x004A05F0`;
  - Write `0x0049ECF0`, Read `0x004A1A20`, GetGZCLSID `0x004A05B0`.
- `cIGZMessageTarget2` `0x00A8F7C4`:
  - QueryInterface `0x004A05D0`, AddRef `0x004A0600`, Release `0x004A0610`;
  - DoMessage `0x006B8AA0` (shared stub).

---

## 12. Containers and allocators *[compat]*

These rows are written in the style of allocators.md. "Crosses" means the memory crosses the game–plugin boundary.

| Container | Who fills / frees | Allocator | Crosses? | Rule | Evidence |
|---|---|---|---|---|---|
| result list of slots 36–41 | manager fills, caller frees | A (`0x00B207E4`), 12-byte head and nodes | **yes** | does not clear; appends; one reference per element; free with slot 48 (or release each + `0x00413E60`), then free the head with A | `0x004A0991` (slot 36 node), `0x004A0C8B` (slot 38) |
| `*out` of slots 28–35 | manager writes | none | yes (a reference) | one AddRef on a hit; untouched on a miss | §7 |
| `cells[8][2]` of slot 26 | caller's array | none | yes (plain ints) | up to 8 pairs; n written and returned | `0x0049DBA0` |
| grid rows | manager | B (outer and inner STLport vectors; `0x005D1290` → `0x004A0350` / `0x00733450` free with `0x0090CF63`) | no | — | `0x004A196C` |
| cell holder | manager | C, 4 bytes | no | created on first insert into the cell, freed when the list empties | `0x005E55E0` / `0x005E5620` |
| cell list head and nodes | manager | B, 12 bytes each | no | front insertion (Insert, Move), back insertion (Read) | `0x0090CF54` / `0x0090CF63`, head free `0x005650A0` |
| shared filter | manager | C, 16 bytes | lent to callers of the typed overloads only as an argument | created by `Init`, released by `Shutdown` | `0x004A190F` |
| shared hash set `+0x44`, local hash set of slot 42 | manager | A (`0x00B21750`) buckets and nodes | no | emptied after each query | `0x0049FA80` |
| RemoveOccupants scratch vector and removed list | manager | A (`0x00B21750`, `0x00B207E4`) | no | freed before return | `0x0049FCD0` |
| iteration batch list (slot 42) | manager | A | no | each element released after its callback | `0x004A0D70` |

---

## 13. Save record *[compat]*

`Write` (`0x0049ECF0`, on the `cIGZSerializable` view):

| # | Type | Field |
|---|---|---|
| 1 | `uint16` | version = **1** |
| 2–3 | `float` × 2 | size X, size Z |
| 4–7 | `int32` × 4 | `+0x24`, `+0x28`, `+0x2C`, `+0x30` (the four counts) |
| 8 | `uint32` | number of rows (192) |
| per row | `uint32` | number of cells in the row (192) |
| per cell | `uint32` | number of occupants: 0 for a null or empty cell |
| per occupant | reference | `SC4WriteGZSerializable` (foundations §2.7), in list order (front first) |

- An occupant that spans k cells is referenced **k times**, once in each cell.
- The record is 2 + 8 + 16 + 4 + 192 · (4 + 192 · 4) = 148 254 bytes, plus 8 bytes per reference (object ID and
  class ID, foundations §2.7).
- Returns `GetError() == 0`.
- Neither `Write` nor `Read` takes the manager's lock (OM8): no lock call in `0x0049ECF0` or `0x004A1A20`.

`Read` (`0x004A1A20`):
1. Set `+0xD`. The version must be 1; otherwise return false.
2. Read the sizes and counts **raw**. Nothing is validated or recomputed.
3. Read the number of rows and resize the grid (`0x005D1290`). For each row, read its cell count and resize the row.
4. For each cell with count > 0, create a holder and list head. For each reference:
   1. `SC4ReadGZSerializable(stream, 0x26BB767E (cISC4Occupant), &p, 0)`. A reference that fails to resolve is
      skipped.
   2. bit 0 = 1 when `p.GetOccupantManagerBBox` succeeds and the box is a single cell.
   3. Append the node at the **back**, which preserves the saved order.
   4. If bit 0 is 0: insert `p` into the hash set `+0x44`. If it was already there, `Release` the extra reference.
      The manager thus holds **one** reference per occupant.
5. Empty the hash set. Return `GetError() == 0`.

At load time bit 0 is usually 0. Occupant classes are read after the manager (ascending class ID, foundations §3.5),
so their box is not loaded yet and `GetOccupantManagerBBox` fails. Queries still return correct results (§4.2).

`Read` posts no message and sets no occupant flag. Flag 1 and the stored box come from the occupants' own records
(answers OM2, re-specs14b). The base occupant record (`Write` `0x005E8CD0` / `Read` `0x005E8D60`, version u16 4) holds,
in order:

1. the u16 at occupant `+0x2C`;
2. the highlight byte;
3. the flags byte (occupant `+0x2F`) masked with `0xDD`, so flag 1 (in the manager) **is** saved;
4. the u32 at `+0x24`;
5. the 4-byte stored box at `+0x28`–`+0x2B`;
6. the property holder.

`Read` sets the state byte `+0x2E := (highlight << 4) | 2`. This clears bit 0 (not initialised) and sets bit 1.
`Init` (§2.2) then initialises every occupant. With bit 1 set, the base `Init` calls slot 17 (0, 0) and posts **no**
Create. 16 occupant classes call this base `Read`.

---

## 13.1 The occupant base class (`cSC4Occupant`, `0xA9BC9AB6`)

Every object the manager holds derives from this class. The manager's slots treat it through `cISC4Occupant`;
this section specifies the class itself (the bare instances' save record is lot-records.md §2.3 / §12).

**Layout** (constructor `0x005E9220`, 85 bytes of code; object size from the derived classes):

| Offset | Type | Role |
|---|---|---|
| `+0x00` | vptr | `cISC4Occupant` view, vtable `0x00AA41D8`, 34 slots (+0x00…+0x84, no slot 34) |
| `+0x04` | vptr | `cIGZSerializable` view, vtable `0x00AA41BC` |
| `+0x08` | sub-object | the property holder (`cSCExemplarPropertyHolder`); its ctor `FUN_005FF1C0` zeroes +0x10…+0x28 relative to the holder, installs the holder vptrs (`0x00AA6D50` / `0x00AA6D3C` at the holder's +0x00/+0x14) and the `cISCExemplarPropertyHolder` view vptr `0x00AA4140` at occupant `+0x1C` |
| `+0x20` | u32 | exemplar-holder payload (the holder's own field; base Write saves it via slot 0x24) |
| `+0x24` | u32 | **occupant type** (ctor writes `0x74758925`; the base record field 5) |
| `+0x28` | 4 × u8 | the stored manager box (ctor writes 0xFF to the first byte = "not indexed"; the base record field 6) |
| `+0x2C` | u16 | removal counters: low byte = holders of removal flag 2, high byte = flag 1 (`SetRemovalFlags` slot 12 / `CanRemove` slot 14) |
| `+0x2E` | u8 | status: bit 0 = initialised, bit 1 = loaded; the highlight nibble lives in the top bits (`GetHighlight` slot 16 = `+0x2E >> 4`) |
| `+0x2F` | u8 | flags byte (ctor writes 0x04 = visible; flag 1 = inserted in the manager, flag 2 = visible) |
| `+0x30` | ptr | placeable object (`GetOrCreatePlaceableObject` slot 20 creates it lazily) |

All offsets above are from the **object base**. Derived classes that embed the base at +4 (props) or +8
(construction) shift them accordingly; lot-records.md §9.1 and §10.3 state the shift per class.

**The `cISC4Occupant` table** (vtable `0x00AA41D8`, all slots `__thiscall`; pop = stack bytes):

| Slot | Method | Body |
|---|---|---|
| 0 | `QueryInterface(iid, out)` pop 8 | `0x005E89C0`: 1 and `0x26BB767E` (cISC4Occupant) → this; `0xE4FDA3D4` (cIGZSerializable) → this+4 and AddRef; else `ADD ECX,8` and tail-jump `cSCExemplarPropertyHolder::QueryInterface` `0x005FD310`, which answers `0x0AC2B5F7` with the holder at this+0x10 (and AddRefs it) |
| 1 | `AddRef()` | `0x005E89B0`: `ThreadSafeIncrementReturn` (`0x00933015`) on `+0x0C` |
| 2 | `Release()` | `0x005E8DF0`; at 0, calls vtable slot `+0x58` with argument 1 (the deleting destructor) |
| 3 | `Init()` | `0x005E8C90` (§2.4 / §5.6) |
| 4 | `Shutdown()` | `0x005E8980`: if initialised, clear bit 0 and post `0x99EF1141` Destroy (data4 0); return 1 |
| 5 | `IsInitialized()` | `0x00458A40`: AL = bit 0 of `+0x2E` |
| 6 | `AsPropertyHolder()` | `0x006C2040`: returns this+8 |
| 7 | *(folded)* | `0x006BDD80` |
| 8 | `GetPosition(out)` pop 4 | `0x005E8A20`: zeroes the three floats (the base has no position; derived classes override) |
| 9 | *(folded)* | `0x00648F10` |
| 10 | `GetBoundingBox(out)` pop 8 | `0x005E8A40`: zeroes both boxes |
| 11 | `GetBoundingCityCells(out)` pop 4 | `0x005E8E20`: from the stored box shrunk by 0.01 (`0x00A823A4`), scaled by 0.0625 (`0x00AA40B0`), with the integer-magic constant `0x00AA40B4` = 12582912.0f |
| 12 | `SetRemovalFlags(u32)` pop 4 | `0x005E8A60`: bit 1 → ++ byte `+0x2D`, bit 2 → ++ byte `+0x2C` |
| 13 | `UnsetRemovalFlags(u32)` pop 4 | `0x005E8AA0`: the reverse |
| 14 | `CanRemove()` pop 4 | `0x005E8AE0`: bit 1 → byte `+0x2D` == 0, bit 2 → byte `+0x2C` == 0, else 1 |
| 15 | `PostOccupantMessage(type, data4)` pop 8 | `0x005E9010` (below) |
| 16 | `GetHighlight()` | `0x00458A50`: `+0x2E >> 4` |
| 17 | `SetHighlight(level, post)` pop 8 | `0x005E90E0`: level → the high nibble of `+0x2E`; when initialised and post, posts `0x99EF1145` with data4 0 |
| 18 | `SetVisibility(bool, post)` pop 8 | `0x005E8B10`: arg1 sets/clears flag 2 of `+0x2F`; when arg2, posts `0x99EF1145` with data4 0 |
| 19 | *(folded)* | `0x006AAC50` |
| 20 | `GetOrCreatePlaceableObject()` | `0x005E91C0`: lazily creates via `FUN_005EBDF0`+`FUN_005EC370` into `+0x30` |
| 21 | `SetPlaceableObject(p)` pop 4 | `0x0078DAF0` |
| 22 | `IsOccupantGroup(groupID)` pop 4 | `0x005E92D0`: property `0xAA1DD396` (via the holder, `cSCPropertyHelp::GetPropertyValue` `0x005FF510`); true iff the returned u32 vector contains the ID |
| 23 | `AddOccupantGroup(groupID)` pop 4 | `0x005E9350`: appends to the `0xAA1DD396` vector and writes it back (`cSCPropertyHelp::SetPropertyValue` `0x005FE6B0`); then `PostOccupantMessage(0xEBFF18A3, oldCount)` |
| 24 | `GetOccupantGroups(out)` pop 4 | `0x005E9410`: fills the caller's set from the `0xAA1DD396` vector. Overload `0x005E9500` resolves the group names through the persist-resource manager `0x00B43CA8` slot 0xC with IID `0xA52160F5` *(semantics: OP10)* |
| 25 | `GetOccupantManagerBBox(out)` pop 8 | `0x005E8BF0`: false when byte `+0x28` == 0xFF (not indexed) |
| 26 | `SetOccupantManagerBBox(...)` | `0x005E8C20` |
| 27 | *(folded)* | `0x005DEB40` |
| 28 | *(folded)* | `0x00648F10` |
| 29 | `SetFlag(bits)` pop 4 | `0x005E8B70`: OR into `+0x2F`; on change tail-jumps slot 30 |
| 30 | `SetAllFlags(bits)` pop 4 | `0x005E8B90` |
| 31 | `ClearFlag(bits)` pop 4 | `0x005E8BA0`: AND-NOT; tail-jumps slot 30 |
| 32 | `IsFlagSet(bits)` pop 4 | `0x005E8BC0` |
| 33 | `GetFlags()` | `0x005E8BE0` |

**`PostOccupantMessage`** `0x005E9010` (207 bytes): allocates a `cRZMessage2Standard` (0x2C bytes, ctor
`0x009134D6`), sets type = arg1, void1 and data2 = this (message slots 0x30 and 0x84), calls the occupant's own
slot 32 with 4 and stores the zero-extended result into the message's `+0x40` dword, then calls the occupant's
slot 7 (`+0x1C`, the folded getter) into the message's `+0x54`, then data4 = arg2 at `+0x68`. When type ==
`0x99EF1146` (Move) and the placeable object `+0x30` is set, the message is delivered **synchronously** through
the manager's slot 0xC; otherwise it is queued through `0x00B23194` (defaulted from `0x00B43CCC`) slot 0x10 with 0.
The message constants: `0x99EF1140` Create, `0x99EF1141` Destroy, `0x99EF1142` Insert, `0x99EF1143` Remove,
`0x99EF1144` Show, `0x99EF1145` highlight/visibility, `0x99EF1146` Move (§5.5).

**The base save record**: Write `0x005E8CD0` / Read `0x005E8D60` (lot-records.md §2.3): u16 version 4; the u16
at `+0x28` (removal counters) via slot 0x1C; `GetHighlight()` via slot 0x40 → u8 (slot 0x14); the flags byte
`+0x2B` masked `0xDD` → u8; the type `+0x24` via slot 0x24; four bytes `+0x28…+0x2B` (the stored box) via slot
0x14 each; then `cSCExemplarPropertyHolder::Write`/`Read` on the holder at +8. Read sets
`+0x2E = (highlight << 4) | 2` (loaded, not initialised). 16 occupant classes call this base `Read`.

---

## 13.2 Occupant filters (`cISC4OccupantFilter`, `cSC4OccupantFilter` and the family)

The query slots 20–22, 36–48 all take one of these. An occupant filter answers "is this occupant in?" for the
manager's queries and for the views. Every filter in the game derives from one small base class with a reference
count and a save record; ten classes, the base among them, can be made by class ID (§13.2.2), and the others are
made by the code that uses them. The 3D view's view filter is a realistic filter (§13.2.3), and a text form
(§13.2.5) builds a filter from words such as `Building` or `SubwayLevel`.

*Rewritten 2026-10-21 (re-occfilters), from the instruction bytes. It corrects the earlier table of this section:*
- the default and realistic filters' group tests are occupant slot 22 `IsOccupantGroup` (`+0x58`), not holder
  properties, and the default filter's holder test is holder slot 3 `HasProperty`, not a QI;
- the realistic filter drops a lot base texture only when `+0xC` is clear and the lot under it is ready (state 1),
  and a signpost or text decal only when `+0xE` is clear;
- `AreModelsAvailableForZoomAndRotation` returns 0 when the first model is **found** and 1 when it is missing, so the
  missing-plugin-pack filter takes the occupants whose model is missing;
- the hover filter needs property `0x8A416A99` for props as well as for flora;
- the demolish-by-networks filter **excludes** the occupants that answer `0xA9764C8C`;
- the inside-lot box is the lot's cell rectangle (lot slot 14), not its centre;
- the class names are the exe's own ID table's ([re/class_id_table.tsv](../../re/class_id_table.tsv)): `0xE9C75ED1`
  is `cSC4OccupantTypeFilter`, `0xE9C75EE8` `cSC4OccupantTypesFilter`, `0xAA25CB90` `cSC4OccupantTypesFilterHashed`
  and `0xEAC3A3D2` `cSC4NetworkOccupantFilter`;
- the base's deleting destructor is `0x005E9790`; `0x006C4A00` is the one several subclasses share (the one-type
  filter of §4.4 and the default filter among them).

It also answers OP8 (the types, hashed and `0xCA0E3852` filter bodies) and OP9 (§13.2.7).

### 13.2.1 The interface and the base class *[compat]*

**`cISC4OccupantFilter`** IID `0x46A80026`, vtable 7 slots (+0x00…+0x18): slot 0 `QueryInterface`, 1 `AddRef`,
2 `Release`, 3 `IsOccupantIncluded(occ)`, 4 `IsOccupantTypeIncluded(type)`, 5 `IsPropertyHolderIncluded(holder)`
(each pop 4, result in AL), 6 the deleting destructor.

The base **`cSC4OccupantFilter`** (CLSID `0xA9C74DC9`) is 12 bytes: `+0x00` the filter view, `+0x04` a
`cIGZSerializable` view (IID `0xE4FDA3D4`), `+0x08` the reference count.

| Part | Address | Behaviour |
|---|---|---|
| constructor | `0x005EA080` | vtables `0x00AA42C8` / `0x00AA42B0`; count 0 |
| factory | `0x005EA130` | 12 bytes from the global heap; null when the allocation fails |
| `QueryInterface` | `0x005E95D0` | 1, `0x46A80026` → the object; `0xE4FDA3D4` → `+4`; a reference added; else false |
| `AddRef` | `0x004BDF00` | count + 1 (a body shared with many classes) |
| `Release` | `0x007CB920` | count > 1 → count − 1, returned; otherwise the deleting destructor (slot 6), 0 |
| slot 3 | `0x004BDF90` | `IsOccupantTypeIncluded(occupant.GetType())` (occupant slot 7) |
| slots 4 and 5 | `0x004FC900` | true |
| destructor | `0x005E95C0` (body), `0x005E9790` (slot 6) | the vtables set back to the base's; the deleting form frees the object |
| `Write` / `Read` | `0x005E97C0` / `0x005E97E0` | a u16 version 3; `Read` fails on another version |
| `GetGZCLSID` | `0x005E95B0` | `0xA9C74DC9` |

`Write` and `Read` of every filter report success when the stream has no error (stream slot 18). The reference
count is never saved. The `+4` view's `QueryInterface`, `AddRef` and `Release` lead to the object's (`0x00424EE0`,
`0x005C0500`, `0x00424EF0`). Most filters reuse the base `Write`, `Read` and `GetGZCLSID`; the ones with their own
are named below.

### 13.2.2 The creatable filters *[compat]*

The game's COM director registers these classes (`0x00460863`–`0x00460912`), so a plugin may create them by class
ID:

| Class ID | Class | Factory | Size | Starts as | Section |
|---|---|---|---|---|---|
| `0xA9C74DC9` | `cSC4OccupantFilter` | `0x005EA130` | 12 | accepts every occupant | §13.2.1 |
| `0xC9C75EAE` | `cSC4DefaultOccupantFilter` | `0x005EA160` | 12 | | §13.2.3 |
| `0xE9C75ED1` | `cSC4OccupantTypeFilter` | `0x005EA190` | 16 | type 0 | §4.4 |
| `0xE9C75EE8` | `cSC4OccupantTypesFilter` | `0x005EA330` | 24 | an empty list | §13.2.3 |
| `0xAA25CB90` | `cSC4OccupantTypesFilterHashed` | `0x005EB970` | 32 | an empty set | §13.2.3 |
| `0x8A7A1C2C` | `cSC4RealisticOccupantFilter` | `0x005EA240` | 16 | the three flags clear | §13.2.3 |
| `0xEAC3A3D2` | `cSC4NetworkOccupantFilter` | `0x005EA1C0` | 16 | mask `0x1FFF` | §13.2.3 |
| `0xAAC3C01D` | `cSC4ViewLevelOccupantFilter` | `0x005EA200` | 16 | level 1 | §13.2.4 |
| `0x0BCEB6C2` | `cSC4StringSpecificationOccupantFilter` | `0x005EBBD0` | 72 | an empty text | §13.2.5 |
| `0xCA0E3852` | the data view filter | `0x007A0420` | 48 | the default byte set | §13.2.8 |

A factory returns null when the allocation fails. The one-type filter's methods are §4.4's, with `Write` / `Read`
`0x005E9890` / `0x005E98E0` (u16 3, u16 3, u32 type).

### 13.2.3 Content filters *[compat]*

**The default filter** (`cSC4DefaultOccupantFilter`, vtable `0x00A87FC0`).
- **`IsOccupantIncluded(o)`** (`0x005E9630`) is false when `o` answers `0x4A10AFC2` (the underground occupants) or
  `0x8A139184` (the underground connection occupants, `kcSC4UndergroundConnectionOccupant` in the SDK), or is in
  occupant group `0x4217`, `0x4208` or `0x421D` (occupant slot 22, `IsOccupantGroup`). Otherwise it is the filter's
  own slot 5 with `o`'s property holder (occupant slot 6), so a subclass's slot 5 is used. Each interface answered
  is released at once.
- **`IsPropertyHolderIncluded(h)`** (`0x005E96E0`) is false when `h` has property `0x49512A09` or `0x8A15F1BD`
  (holder slot 3, `HasProperty`); otherwise it is true unless the bool property `0xCAA45670` is true.
- Record (`0x005E9810` / `0x005E9840`): two u16 of 3. `GetGZCLSID` `0x00466480`.

**The realistic filter** (`cSC4RealisticOccupantFilter`, vtable `0x00A88084`). Three byte flags, all 0 when made
(constructor `0x004666C0`, factory): `+0xC` keep empty lots' base textures, `+0xD` keep group `0x5001`, `+0xE` keep
signs and labels *[inferred names, from the tests below]*. **`IsOccupantIncluded(o)`** (`0x005E9990`) is false when
the default filter's test is false. Then:

1. `+0xD` clear and `o` in occupant group `0x5001` → false.
2. `o` of type `0x497F6D9D` (a lot base texture) with `+0xC` clear: when `o` answers `0x097F6D98`
   (`cISC4LotBaseTextureOccupant`), its position (occupant slot 8) gives a cell (the city's slot 95,
   `PositionToCell`); when the lot manager (`0x00B43D08`) has a lot there (slot 6, with 0) whose `GetState()` (lot
   slot 33) is 1 (ready, an empty lot), false.
3. `o` a prop (`0xA823821E`) with `+0xE` clear: a signpost (answers `0x4B44FBE2`) or a text decal (answers
   `0x2B3B7D86`) → false.

Otherwise true; slot 5 is the default's. Record (`0x005E9AF0` / `0x005E9B50`): u16 4, u8 `+0xC`, u8 `+0xD`, u8
`+0xE`, then the default's record. `Read` fails below version 3; a version-3 record has no third byte and copies
`+0xD` into `+0xE`. `GetGZCLSID` `0x004666F0`, which the demolition filter shares. Users: the 3D view's first view
filter has all three flags set (`DoPreCityInit`, `0x007F42C5`); the cheats `Zoneria` (`0x007F365D`:
`+0xC` flipped, `+0xD` copied, `+0xE` clear) and `TastyZots` (`0x007F391F`: `+0xD` flipped, the others copied) replace
it (view3d-window.md §3.1; OM10); the inside-lot filter (§13.2.6); the region-view
thumbnail writer (`0x005DE18C`).

**The demolition filter** (`cSC4DemolitionOccupantFilter`, 0x30 bytes, [demolition.md](demolition.md) §4.2) extends
the realistic filter. `IsOccupantIncluded` `0x00466730`: Realistic test → continue; Default test → **exclude**;
`+0x2C` filter slot 3 false → exclude; byte `+0x10` → box-containment test vs `+0x14…+0x20` (FCOMP); byte `+0x24` →
`[occ vtable+0x38](+0x28)` continue; type switch `0x68FD0C69` foundation (QI `0xC8FCE101` → lot slot 0x24 == 1),
`0x096E763C` construction, `0x278128A0` building, `0x74758926` flora, `0xA823821E` prop, `0xC772BF98` network →
accept; else exclude; accept path: `+0x30` filter slot 3 false → exclude; network (`0xC772BF98`): `0x00B43DC4` slot
0x58 == 1 skips the `0xA821EF94` QI + slot 0x24 (`0x1F4F`) test, else that test true → exclude. Its destructor body
(`0x00466EC0`) releases the caller and view filters (`+0x2C`, `+0x30`).

**The type list** (`cSC4OccupantTypesFilter`, vtable `0x00A8EDBC` / `0x00A8EDA4`). A vector of u32 types at `+0xC`.
Slot 3 is the base's; **`IsOccupantTypeIncluded(t)`** (`0x005E9950`) is true for an empty list and otherwise when `t`
is in it. Only the factory makes one and the class has no setter, so only `Read` fills the list. Record (`0x005EA0A0`
/ `0x005EA490`): u16 3, u16 3, u32 `n`, `n` u32 types; `Read` needs both versions 3. `GetGZCLSID` `0x005E9760`; the
destructor (`0x005EA420`, body `0x00491210`) frees the list.

**The hashed type set** (`cSC4OccupantTypesFilterHashed`, vtable `0x00A8EE38` / `0x00A8EE20`). A hash set of u32
types at `+0xC` (64 buckets; made with a size hint of 100). **`IsOccupantTypeIncluded(t)`** (`0x00493700`): `t` is
in the set. Record (`0x005EA370` / `0x005EA550`): u16 3, u16 3, u32 count, then the types in the set's order.
`GetGZCLSID` `0x005E9770`; destructor `0x00493720`. `cSC4ModelMaker::CreateOccupantFilters` (`0x00493EF0`) makes
two and fills them itself: filter1 with {building `0x278128A0`, foundation `0x68FD0C69`, wall `0x895D1169`, flora
`0x74758926`, prop `0xA823821E`, base texture `0x497F6D9D`, network `0xC772BF98`, pole `0x2890D4DE`, `0x2A183167`,
`0x8A13918A`, landfill `0x6A11BD2B`}, filter2 with a subset starting {`0x68FD0C69`, `0x497F6D9D`, `0xC772BF98`,
`0x2A183167`, `0x8A13918A`}.

**The network filter** (`cSC4NetworkOccupantFilter`, vtable `0x00AA42FC`). A u32 mask at `+0xC`.
**`IsOccupantIncluded(o)`** (`0x005E9BD0`): `o` answers `cISC4NetworkOccupant` (`0xA821EF94`) and its
`GetNetworkFlag()` (network occupant slot 12) has a bit of the mask. Record (`0x005E9C30` / `0x005E9C60`): u16 1,
u32 mask, without the base's record. `GetGZCLSID` `0x005EA120`.

**The group filter** (`cSC4OccupantGroupFilter`, vtable `0x00AA4330`). A vector of group IDs at `+0xC`. The
constructor (`0x005EA280`) takes one group and adds it when it is not 0; slot 7 `AddGroup(g)` (`0x005EA2F0`) does the
same. **`IsOccupantIncluded(o)`** (`0x005EA020`): `o` is not null and is in one of the groups, tested in order; an
empty list includes nothing. A city-situation filter derives from it (`0x00526610`).

### 13.2.4 View levels *[compat]*

**`SC4GetOccupantViewLevel(o)`** (`0x005E9CA0`, 401 bytes, free function): 1 the surface, 2 the subway layer, 3 the
pipe layer. Starting from 1, these tests run in this order and each later hit replaces the result:

1. `o` answers `0x49C05B9F` (a pipe) → 3; `0xEA0A5ACF` (a pipe connection) → 3.
2. Its property holder has `0x49512A09` → 3.
3. `o` answers `0xCA16374F` (a subway) → 2; `0xCA15ED6B` (a subway connection) → 2.
4. `o` is in group `0x4217`, `0x4208` or `0x421D` (occupant slot 22) → 2.
5. The holder has `0x8A15F1BD` → 2, and the result is final.
6. Only when the result is still 1: with the bool properties `p72` = `0xCAA45672`, `p71` = `0xCAA45671`, `p70` =
   `0xCAA45670` (false when absent): `p72` → 3; then `p71` → 2; then, when `p70` is false, 1 again.

So a prop must have `0xCAA45670` for its `0xCAA45671` or `0xCAA45672` to move it below the surface
([neighbors.md](neighbors.md)'s connection markers set them in pairs).

**The view-level filter** (`cSC4ViewLevelOccupantFilter`, vtable `0x00A8FAD4`). A level at `+0xC`.
**`IsOccupantIncluded(o)`** (`0x005E9E40`): `SC4GetOccupantViewLevel(o)` equals it. Record (`0x005E9E60` /
`0x005E9E90`): u16 1, u8 level (widened when read), without the base's record. `GetGZCLSID` `0x005E9780`. The
regional city builds one for its connections (`0x004AAE5E`).

### 13.2.5 The string specification filter (`cSC4StringSpecificationOccupantFilter`, vtable `0x00AA442C`) *[compat]*

Fields: `+0x0C`–`+0x14` the box minimum x, y, z (floats), `+0x18`–`+0x20` the box maximum, `+0x24` the type list,
`+0x30` the group list, `+0x3C` the level list (vectors of u32).

**Building it.** Two constructors: the factory's (`0x005EBB10`), which builds from an empty text, and one from a word
list (`0x005EB850`). `BuildSpecification(text)` (`0x005EBA70`) splits a text into at most 64 words with
`RZParseDelimitedText` and a space ([command-system.md](command-system.md) §2), so a word holding spaces must be
quoted, then calls `BuildSpecification(words)` (`0x005EA610`, 4,670 bytes; the words it knows are at
`0x00AA4354`…`0x00AA4408`). It sets the six box floats to −FLT_MAX and empties the three lists. Then **every word
goes through every test below**; a test matches when the word starts with its prefix, ignoring case. The numbers
after a box prefix are the rest of the word split at spaces, each read as a whole number (`atol`, so `1.5` reads 1)
and stored as a float:

| Prefix | Numbers | Box |
|---|---|---|
| `Position:` | x z | (x, −FLT_MAX, z)–(x, +FLT_MAX, z) |
| `Rect:` | x0 z0 x1 z1 | (x0, −FLT_MAX, z0)–(x1, +FLT_MAX, z1) |
| `BBox:` | x0 y0 z0 x1 y1 z1 | as given |
| `Cell:` | x z | as `Position:`, each number first divided by 16 (whole division toward 0) *[quirk]* |
| `CellRect:` | x0 z0 x1 z1 | as `Rect:`, each number divided by 16 *[quirk]* |

Then the first type name the word starts with adds its type to the type list: `Network` `0xC772BF98`, `PowerPole`
`0x2890D4DE`, `PowerLine` `0x0990B2EA`, `Building` `0x278128A0`, `Flora` `0x74758926`, `Prop` `0xA823821E`, `Vehicle`
`0xC7CC75F1`, `Pedestrian` `0x496E5783`, `Aircraft` `0x480E6151`, `Watercraft` `0x897F7EC2`, `Pylon` `0xA9B6D7DE`,
`Fire` `0x69D25C5F`, `Automaton` `0x49D57875`, `Construction` `0x096E763C`, `Foundation` `0x68FD0C69`,
`LotBaseTexture` `0x497F6D9D`, `LotRetainingWall` `0x895D1169` (tested in this order). A word starting with `0x`
adds a group, but the number read is that of the two-character text `0x` itself, so the group is always 0
(`0x005EB4EF`) *[quirk]*. `SurfaceLevel`, `SubwayLevel` and `PlumbingLevel` add the levels 1, 2 and 3.

**Using it.**
- **`IsOccupantIncluded(o)`** (`0x005E9F20`): three tests, all needed: the type list is empty or holds `o`'s type;
  the group list is empty or `o` is in one of its groups (occupant slot 22); the level list is empty or holds
  `SC4GetOccupantViewLevel(o)`. The box is not tested here.
- **`GetOccupants(list, max)`** (`0x005E9EE0`): the occupant manager's `GetOccupantsByBBox(list, min, max, this,
  max)` (slot 36, §8.2) with the box; false without the manager. Without a box word the box is empty (both corners
  −FLT_MAX), and nothing is found *[inferred, from the strict overlap test]*.
- A debug command of the command support object (`0x005C955C`) builds one from its words and calls `GetOccupants`
  with no limit.
- Record: u16 1 only (`0x0042E440` / `0x006B8A70`, bodies shared with other classes): the specification is not saved
  *[quirk]*. `GetGZCLSID` `0x005EB8F0`; destructor `0x005EB900` (body `0x005EB920`, the three lists freed).

### 13.2.6 Purpose filters *[compat]*

These classes are made by their users and keep the base's record and class ID unless noted.

**Inside a lot** (`cSC4InsideLotOccupantFilter`, vtable `0x00AB1830`, 0x38 bytes). The constructor (`0x006BFB20`)
embeds a default filter at `+0x1C` and a realistic filter at `+0x28` (flags clear). **`SetLot(lot)`**
(`0x006BFAC0`): the lot's rectangle (lot slot 14, cells `x0, z0, x1, z1`) becomes a world box through the city's
`CellCornerToPosition` (slot 96, called twice): `(x0, z0)` → minimum (`+0xC`, `+0x10`), `(x1 + 1, z1 + 1)` → maximum
(`+0x14`, `+0x18`). **`IsOccupantIncluded(o)`** (`0x006BFB80`): `o`'s position (occupant slot 8) lies in the box, all
four sides inclusive (a NaN coordinate is outside), and the default and the realistic filters both accept `o`.
`cSC4Lot::GetLotOccupants` (`0x006BFF70`, [lot-records.md](lot-records.md) §3.1) makes one, sets the lot and asks
the manager's slot 40 (`GetOccupantsByStandardCityCells`) with maxCount −1 (`0x006BFFA7`). Destructor `0x006BFC10`.

Each entry below gives the vtable, the test `IsOccupantIncluded(o)` and the users.

- `cSC4OccupantImmovabilityFilter` (`0x00A88050`, `0x00466530`): a prop, building, power pole or network occupant
  whose bool property `0x29F013E6` is false or absent. Terrain levelling ([demolition.md](demolition.md) §4.7).
- `cSC4ActiveFireOccupantFilter` (`0x00A949B8`, `0x0046FD70`): a fire occupant (`0x69D25C5F`) with a target (its
  interface `0x69D25C60`, slot 5). Fire crews, trucks and planes ([disasters.md](disasters.md) §5.7).
- `cSC4MissingPluginPackOccupantFilter` (`0x00A8ED08`, `0x0048C4D0`): below. The missing-pack check (`0x0048C13E`).
- `cSC4ZoomRotationModelChangeOccupantFilter` (`0x00A8EEA0`, constructor `0x00496790`, `0x004967E0`): below. The
  model maker ([model-maker.md](model-maker.md)) and its loader.
- `cSC4HoverQueriableOccupantFilter` (`0x00A909B0`, `0x004C37B0`): below. The query tool's hover (`0x004CBCE5`).
- `cSC4QueriableOccupantFilter` (`0x00A90B18`, `0x004C46C0`): false when the holder has `0xCAA9AB92`, else the hover
  test. The query tool's pick (`PickQueriableOccupant` `0x004C4C90`, `0x004D42A4`).
- `cSC4DemolishByNetworksOccupantFilter` (`0x00AA825C`, `0x0061F590`): true unless `o` answers `0xA9764C8C` (the
  underground occupants and the power poles do); the QI output is released when non-null. The network routines
  ([network-routines.md](network-routines.md) §5 has the full contract) and the network tool.
- `cSC4WaterOccupantFilter` (`0x00AA7A54`, `0x00648C40`) and `cSC4SubwayOccupantFilter` (`0x00AA7A88`,
  `0x00653C00`): below. The network manager's slots 18 and 19 ([network-manager.md](network-manager.md)).
- `cSC4BlockingLotOccupantFilter` (`0x00AB1B28` / `0x00AB1B10`, constructor `0x006C3C00`): the base's test, with
  slot 4 true only for flora `0x74758926` (`0x006C3C20`). The lot developer ([lot-developer.md](lot-developer.md)).
- `cSC4SurfaceNetworkOccupantFilter` (`0x00AB8340`, constructor `0x007A7900`, `0x007A7970`): below. The mini map
  (`0x007A8765`).
- `cSC4FloraBlockingOccupantFilter`: [flora.md](flora.md) §6.4.1 (`IsOccupantIncluded` `0x006AFD10`).

The hover and query filters' slot 5 is false (`0x005DEB40`); the water and subway filters have their own (below);
the others keep the base's true.

**Missing plugin packs.** The filter holds a zoom (`+0xC`) and a rotation (`+0x10`), both 0 when made.
**`IsOccupantIncluded(o)`**: false when `o`'s holder lacks property `0x6A871B82`; otherwise true exactly when
`AreModelsAvailable(o)` is 1.
- **`AreModelsAvailableForZoomAndRotation(o, zoom, rotation, …)`** (`0x0048C300`): the model instances of `o` at that
  zoom and rotation (`cSC4ModelMakerUtility::GetAllModelInstanceIDs`, graphics-formats.md GF7). No non-zero instance →
  2. Otherwise the first non-zero one is looked up in the resource manager (`0x00B43CA8`, slot 3, IID
  `cIGZPersistResource` `0x456B8F1D`) with the type and group the utility returned: found → 0, else 1.
- **`AreModelsAvailable(o)`** (`0x0048C420`): the lookup above with `o`'s orientation and state
  (`GetOccupantOrientationAndState` `0x00496670`) and whether the holder has `0xE83A081D`. The filter's zoom and
  rotation first; on 2, the zooms 4, 3, 2, 1, 0 except the filter's, at rotation 0, until one answer is not 2; 2 when
  all are.

So the filter takes the occupants marked with `0x6A871B82` whose first listed model is missing.

**Zoom and rotation change.** **`SetZoomAndRotation(z, r)`** (`0x004967B0`): mask := `(1 << (z + 2)) + (1 << (r +
7))`. **`IsOccupantIncluded(o)`**: false when `o` has a placeable object (occupant slot 19) whose `+0x2C` bits,
masked, include both a zoom bit (`0x7C`) and a rotation bit (`0x780`); true otherwise, also without an object.

**Hovering and querying.** The hover test (`HoverQueriable` `0x004C37B0`): buildings, power poles, landfill
(`0x6A11BD2B`) and network occupants are in; flora and props only when their holder has `0x8A416A99` (an item name,
building-development.md); every other type is out.

**Water and subway views.**
- **Water** (`0x00648C40`): a network occupant with a flag of `0x1F4F` (the surface networks) or a pipe connection
  (`0xEA0A5ACF`) is in; otherwise slot 5 (`0x00648CC0`): a holder with property `0x88ED3303` (water production) or
  `0x49512A09` is in, else the bool `0xCAA45672`.
- **Subway** (`0x00653C00`): a network occupant with a flag of `0x1F4F`, a subway connection (`0xCA15ED6B`), a subway
  station (below) or a vehicle (`0xC7CC75F1`) in group `0x4217` or `0x4208` is in; otherwise slot 5 (`0x00653CC0`): a
  station, else the bool `0xCAA45671`.
- **`IsOccupantSubwayStation(o)`** (`0x00653B80`): the transit-switch property `0xE90E25A1` of `o`'s holder, read as
  bytes (traffic-network.md §4.7), has **any** byte equal to 6 *[inferred: 6 the subway travel type]* *[quirk: every
  byte is compared, not only the from and to bytes]*.

**Surface networks.** `cSC4SurfaceNetworkOccupantFilter` is a separate small class (a reference count at `+4`,
`QueryInterface` `0x007A7930` answering 1 and `0x46A80026`, 6 slots). **`IsOccupantIncluded(o)`**: `o` answers
`0xA821EF94` and its network flag has a bit of `0x1F4F`. Slots 4 and 5 are false. The interface is released before
its flag is read, which is harmless because the caller holds `o` *[quirk]*.

### 13.2.7 Filters outside the family

These answer the same interface and are specified with their owners: `cAmbienceOccupantFilterIsNature::IsTrueOccupant
0x004E2E10` (QI `0xC83DD057`, cached at +4/+8), `cAutomataOccupantFilter::IsOccupantIncluded 0x00562C90` (type ∈
{`0xC7CC75F1`, `0x496E5783`, `0x480E6151`, `0x897F7EC2`}: the vehicle, pedestrian, aircraft and watercraft types,
which is how the string specification of §13.2.5 names them; answers OP9), `tCrimePropOccupantFilter::IsOccupantIncluded
0x0069F490` (type `0xA823821E` ∧ holder HasProperty `0x8AA864D5`).

### 13.2.8 The data view filter (`0xCA0E3852`) *[compat]*

The data map view ([map-view.md](map-view.md) §2) gives the 3D city view this filter. It is 0x30 bytes, vtables
`0x00AB7E44` / `0x00AB7E2C` (7 slots: the base's QI, AddRef, Release and slots 4 and 5; slot 3 `0x0079F7F0`; slot 6
`0x007A0480`, body `0x007A04A0`); the exe's ID table does not name the class *[we call it the data view filter]*.
The factory (`0x007A0420`) and the constructor (`0x0079FE60`, which the map view calls at `0x007A4C05`) clear every
field and set `+0x2E` to 1.

| Offset | Field |
|---|---|
| `+0x0C`–`+0x14` | a type list (vector of u32; `AddType` `0x0079FB90` appends) |
| `+0x18`–`+0x20` | a group list (vector of u32) |
| `+0x24`, `+0x28` | two optional filters |
| `+0x2C` | invert the type test |
| `+0x2D` | invert the group test |
| `+0x2E` | keep empty lots' base textures and the signposts and text decals |

**`IsOccupantIncluded(o)`** (`0x0079F7F0`), with `t` = `o`'s type:

An empty-lot base texture has priority over all other rules: for type `0x497F6D9D`,
the lot is found at the low corner of the occupant's cell rectangle (occupant slot
11; lot manager `0x00B43D08` slot 6, with 0). With no lot, or with lot state below 2
(lot slot 33), inclusion equals `+0x2E` and optional filters cannot override it.

For other occupants, define type acceptance as membership in the type list XOR
the invert-type flag `+0x2C`. A type-accepted network (`0xC772BF98`) that answers
`0xA821EF94` additionally needs group acceptance: membership in any listed group
(network slot 22, list order) XOR `+0x2D`. Non-networks and networks lacking that
interface need only type acceptance for this route to inclusion.

An occupant rejected by that route is still included if either optional filter
accepts it (slot 3), or if `+0x2E` is set and it is a prop (`0xA823821E`) answering
`cISC4PropOccupant` (`0xE9793A65`) whose prop type (slot 7) is signpost
`0xAB72FBB3` or text decal `0xCB79919B`. Otherwise it is excluded.

The map view builds it from the data view exemplar ([map-view.md](map-view.md) §2): the non-zero types of
`0x4A0B47E0` go to the type list; `+0x2C` is the bool `0x4A0B47E1` when present, else 1 without `0x4A0B47E0` and 0
with it; `0x4A0B47E2` true clears `+0x2E`. The group list and the two filters stay empty. So a data view without a
type list shows every occupant, one with a list shows only those types (or, with `0x4A0B47E1`, all but those), and
the base textures of empty lots and the signs and labels stay unless `0x4A0B47E2` hides them.

The shared one-type filter of §4.4 is the family member the manager itself owns; the typed slots write their type
into it and pass it on (§10).

---

## 14. Threading *[compat]*

The occupant manager is one of the few city components with a lock: a recursive `cRZCriticalSection` at `+0x14`.
This is an exception to foundations §19.1's "no locking on the simulation path". The thread that competes for it
is the **background model loader** (OM1):
- `cSC4ModelBackgroundLoader::Init` (`0x0048E900`) takes a reference to the occupant manager from the global
  `0x00B43D0C` into its `+0x20` (`0x0048E985`).
- It then starts its `cRZThread` (`+0x60`) on `0x0048E8F0`, which runs `ThreadFunction` (`0x0048E460`).
- On that thread, `ContinueLoadTask` (`0x0048DCD0`, called at `0x0048E620`) walks standard cells in a spiral
  around the view (`SpiralIterator::GetNext`, `0x00440850`).
- When the loader's filter flag (`+0xF0`) is set, it calls slot 40 (`+0xA0`, `0x0048DDBC`) for each cell. The cell
  is passed as both min and max, with the loader's filter (`+0xEC`) and no count limit. Slot 40 reaches the locked
  slot 36. Each occupant found goes to `LoadResourcesForOccupant` (`0x0048D630`) on the same thread.

So queries from the main thread, including Insert, Remove and Move, can wait for one cell's query from the loader
thread, and the other way round. No other worker thread holding the manager was found *[inferred: the renderer and
the simulators run on the main thread, foundations §19.1]*.

| Slots | Locking |
|---|---|
| 1–5, 20–22, 24, 27, 28, 30, 32, 34, 36, 38 | the whole call |
| 42 | per cell, released around the callbacks |
| 44 | the whole walk, callbacks included |
| 40, 47, typed thunks | no lock of their own; they reach a locking slot |
| 6–19, 25, 26, 48, `Write`, `Read` | none (OM8: slot 25 `0x0049E490`, `Write` `0x0049ECF0` and `Read` `0x004A1A20` were checked) |

Hazards a replacement must keep in mind:
- **The shared filter is not re-entrant.** A typed call made from inside a slot-42 or slot-44 callback rewrites
  the shared filter's type. If the outer walk was itself typed, it uses the new type for its remaining cells
  *[quirk]*.
- The shared hash set (`+0x44`) is used only by slots 36 and 38 and by `Read`, each emptying it before returning.
  Slots 42 and 44 use a local set (`0x0049FA80`), so a list query nested in an iteration callback is safe.
- Slot 42 callbacks run unlocked. Slot 44 callbacks run with the lock held: another thread that needs the manager
  waits for the whole walk.
- Insert's flag-1 check (§5.1 step 2) is outside the lock. Two threads inserting the same occupant can both pass it;
  the first-cell duplicate check under the lock then rejects the second.

*[choice]* FreeCity4 keeps one lock per manager, re-entrant on the same thread, and holds it exactly where the table
says. Its typed overloads use a per-call filter value. The results are the same as the original's for every
non-nested call. In the nested case FreeCity4 does **not** reproduce the leaked type: it is accepted as a logged
deviation, because no shipped callback nests a typed call *[inferred]*.

---

## 15. Quirks (summary)

1. **Slot 12 is wrong** (§3.2). outX = 4·z − 256 uses the z input, and outZ is the old outZ times 4. The Mac build
   has the same code. Nothing in the game calls it (OM3). Reproduce it anyway; nothing can observe it.
2. Slot 13 rounds toward zero: standard cell −1 → world cell 64, not 63.
3. Slots 6–19 return an undefined AL.
4. Insert does not clamp the cell box (§5.1 step 6). Bytes wrap, so an occupant beyond position −4096 or 8192 indexes
   past the grid. Move clamps its new cells (step 7), but stores the unclamped box. Slot 28 does not clamp either.
5. Insert calls `occ.Init()` before its property-holder and duplicate checks, so a failed insert can still post
   Create.
6. Remove calls `Shutdown` when asked, even if the occupant was not inserted or the remove failed.
7. FindOccupant's fast path returns true without writing x and z.
8. Slot 40 and slot 47: a null max becomes (count, count), not count − 1.
9. Slot 44 reads three-int vectors (x at 0, z at 2).
10. Slots 42 and 44 return true when not initialised; their typed thunks and slot 47 return false.
11. Slot 23 and slots 29–35 filter on type 0 literally; slots 37–46 treat type 0 as "no filter".
12. The typed/filter order is reversed for slots 46/47 compared with every other pair.
13. `Init` after a load initialises only occupants found in the city's cells.
14. `+0xD` is written by `Read` and never read.
15. Strict overlap with NaN passes (§8.2).
16. After a load, single-cell nodes carry bit 0 = 0 (§13). They are harmless but differ in memory from a fresh
    insert.
17. Slot 32 does not clamp its world cell either (§7), and slot 34 clamps a given max to count − 1 but turns a null
    max into count (OM5).
18. A box whose low corner wraps below byte 0 is inserted with no nodes and still counts as inserted (§5.7).
19. A string specification's `0x` word always adds group 0 (§13.2.5).
20. `Cell:` and `CellRect:` divide their numbers by 16 instead of turning cells into metres (§13.2.5).
21. String specification box numbers are read as whole numbers (§13.2.5).
22. A string specification filter saves only its version, not its specification (§13.2.5).
23. The subway station test looks at every byte of the transit-switch rules (§13.2.6).

---

## 16. Open questions

- ~~**OM1:** which thread contends for the lock? [*inferred*: the renderer or the model loader; settled by a
  breakpoint on `0x0049DF80`/`0x004A0670` filtered by thread ID in a turbo run]~~ **ANSWERED 2026-10-16
  (re-answers18):** **MATCHES** for the model loader. The background model loader's thread (`cRZThread` at loader
  `+0x60`, entry `0x0048E8F0` → `ThreadFunction` `0x0048E460` → `ContinueLoadTask` `0x0048DCD0`) calls slot 40 at
  `0x0048DDBC` on its occupant-manager reference (`+0x20`, taken from `0x00B43D0C` at `0x0048E985`), one standard
  cell at a time around the view. The renderer is on the main thread. §14.
- ~~**OM2:** the loaded occupants' flag 1 and stored box: are they read from the occupant records, or restored some
  other way?~~ **Answered (re-specs14b):** from the occupant base record (§13): flag 1 is saved in the flags byte
  (mask `0xDD`) and the box in the 4 bytes at `+0x28`. The bracket MATCHES. A loaded occupant does not post Create
  (§2.2 corrected).
- ~~**OM3:** is slot 12 ever called? [not searched exhaustively: virtual calls through `+0x30` are common. A turbo run
  with a breakpoint on `0x0049DA30` would settle it; until then reproduce the bug]~~ **ANSWERED 2026-10-16
  (re-answers18):** no caller was found. The bracket stands: reproduce the bug.
  - `0x0049DA30`'s only reference is its vtable entry `0x00A8F820`; there is no direct call.
  - Of the 2436 `call [reg+0x30]` sites, none gets its receiver from the manager global `0x00B43D0C` or from the
    city's `GetOccupantManager` (`+0x88`).
  - Every 4-argument site that passes an address goes to another interface (network manager, terrain, zone
    manager, lighting).

  A pointer passed through a field or parameter cannot be ruled out statically *[inferred]*. Either way the bug is
  not observable through any shipped caller. §15 quirk 1.
- ~~**OM4:** is the 0.1 m shrink constant (§5.1 step 5 and the standard-cell boxes of slots 32/34/40) `float` or
  `double`, and where is it? [float]~~ **ANSWERED 2026-10-16 (re-answers16):** MATCHES. It is `float`
  `0x3DCCCCCD` at `0x00A8F768`, read as a 4-byte operand everywhere; the 1.0 width limit is `float` at `0x00A81228`.
  Every shrunk bound is stored as a `float` (§5.1 step 5, §7).
- ~~**OM5:** slot 34 clamps its corners to the standard counts: 0..=count or 0..=count−1? [0..=count]~~
  **ANSWERED 2026-10-16 (re-answers16):** **DIFFERS**. A non-null corner is clamped to 0…count − 1
  (`0x0049E827`–`0x0049E8B7`). Only a **null** max becomes (count X, count Z), one cell past the edge, as in slot 40
  (§7).
- ~~**OM6:** out-of-grid and crash paths: Insert before Init, boxes with bytes past 191 or negative cells, slot 23
  with a null filter, slot 28 outside the grid, FindOccupant on an occupant without a property holder. [no nodes /
  flag 1 set and Insert posted / true and no-op / none / no guess]~~ **ANSWERED 2026-10-16 (re-answers16):** see
  §5.7 and §6.3. The wrapped box (low byte > high byte) **MATCHES**: no nodes, flag 1 set, Insert posted. The no-guess
  FindOccupant MATCHES in effect, because such an occupant is never inserted. The other three cases crash or read
  wild memory in the original, so the brackets are accepted as choices:
  - Insert before `Init` crashes on the null row array (`0x0049F0F7`) after Create and the stored box; after
    `Shutdown` Insert works;
  - a byte overrun reads and writes past the grid;
  - slot 23 writes to address `0xC`;
  - slot 28 (and slot 32) read outside the grid.
- ~~**OM7:** does slot 44 skip the duplicate check for single-cell nodes like slots 36/38? [checks every node]~~
  **ANSWERED 2026-10-16 (re-answers16):** **DIFFERS** in mechanism, same results. It skips the check for bit-0 nodes
  (`0x004A1689`–`0x004A1695`), and a single-cell walk has no check at all. §9.2 is rewritten: slot 44 collects an
  AddRef'd batch per cell under an inner lock and calls back after dropping it, as slot 42 does, but inside the
  outer lock.
- ~~**OM8:** do `Write`/`Read` and slot 25 take the manager's lock? [yes]~~ **ANSWERED 2026-10-16
  (re-answers16):** **DIFFERS**: none of the three does (`0x0049ECF0`, `0x004A1A20`, `0x0049E490`). Taking the
  recursive lock there is harmless (same thread, no callbacks inside). The choice may stay; it is not observable.
- **ZQ14** is answered in zones.md §5.2 (re-answers11). The trace in this lane agrees: the calls at `0x006277FA`,
  `0x00627A16` and `0x00627A38` pass the network tool's `+0x208`, which is set to 3 at `0x0062924A` and to 1 at
  `0x00629980` and `0x0062C782`.

---

## 17. Corrections to other documents

- **allocators.md**, the slots 36/38 row: "Returns a bool in AL (true when something was appended)" is wrong. It
  returns **true whenever it ran** (initialised and `maxCount` ≠ 0), even with nothing appended. The argument named
  `flags` there is a **maximum count** of results appended in this call. Proposed text in the ABI rows file.
- **allocators.md**, the typed-thunk row: correct, but add that the type is the 4th argument, and that slots 43, 45
  and 46 (iteration) follow the same type-0 rule with the type as the 5th argument.
- **interfaces.tsv** slot 37: the method is `GetOccupantsByBBox(list, min, max, type, maxCount)`. The 5th argument
  is not a filter. The note's "strict overlap" is right.
- **interfaces.tsv** slot 48: "releases each entry of a slot-41 list" is right for any slot 36–41 list.
- **interfaces.tsv** slots 46/47: slot 46 is typed (`→ +0xBC`), slot 47 is the filter overload.

---

## 18. Conformance tests (must exist in `engine/conformance`)

Unless a test says otherwise, use a 1024 m city: manager counts 16, standard counts 64, city world cells 64…79.
"Box" means `GetBoundingBox` (min, max).

1. **Counts.** `SetCitySize(1024, 2048)` gives `+0x24` = 16, `+0x28` = 32, `+0x2C` = 64, `+0x30` = 128.
   - Slot 9 gives (64, 64, 79, 95).
   - After `Init`, a second `SetCitySize` returns false and changes nothing.
2. **Position to world cell.** Slot 15 must give every value of the table in §3.3. Slot 17 must give 0, 0, 1, 1
   and −1 for 0.0, 63.9, 63.95, 64.0 and −0.5.
3. **Standard cell floor.** Slot 19 must map 0, 15.99, 16, 31.99, 32, −0.1 and −16 to 0, 0, 1, 1, 2, −1 and −1.
4. **Slot 12 bug.** `WorldCellToStandardCityCell(70, 66, &a, &b)` with b preset to 5 gives a = 8, b = 20.
   Slot 13 of (−1, 5) gives (64, 65).
5. **Insert cells.**
   - A box x 100.0…164.0, z 0.0…64.0 is shrunk to x 100.1…163.9 and z 0.1…63.9. Cells: x 65…66, z 64…64.
     The stored box is (65, 64, 66, 64); it is not single-cell, so the nodes have bit 0 = 0.
   - A box x 128.0…128.5, z 128.0…128.5 is not shrunk (width ≤ 1) and gives the single cell (66, 66).
   - A box x 63.9…64.5 (width 0.6, not shrunk) spans cells 64…65.
6. **Insert and its messages.** Insert a new, uninitialised occupant with data4 = 7. The queue must then hold, in
   order:
   - Create `0x99EF1140`, data4 0;
   - Insert `0x99EF1142`, data3 = the occupant's type, data4 7.

   After the call, `IsFlagSet(1)` is true, the stored box is set, and the reference count is +1. Inserting it again
   returns false and posts nothing.
7. **Insert failure.** An occupant with no property holder: Insert returns false and Create is still posted. The
   reference count is unchanged and flag 1 is clear.
8. **Newest first.** Insert A, then B, then C, all in cell (66, 66).
   - `GetFirstOccupant(out, 66, 66, null)` returns C.
   - Slot 38 over that cell returns [C, B, A].
   - Remove C. Slot 30 then returns B.
9. **Dedup and order across cells.** Insert D spanning cells x 65…66, z 64. Insert E in (66, 64) after D.
   - Slot 38 over (65, 64)…(66, 64) returns [D, E]: D once, at its first cell.
   - With `maxCount` = 1 it returns [D] and still returns true.
   - With `maxCount` = 0 it returns false and leaves the list untouched.
   - With a filter that excludes D, it returns [E] and still returns true.
   - Over an empty area it returns true with no elements.
10. **Strict box test.**
    - Occupants F (box x 0…16, z 0…16) and G (x 16…32, z 0…16). Slot 40 over standard cell (0, 0)…(0, 0) returns
      [F] only.
    - Slot 36 with min = max = (8, 0, 8) returns F only when F's y range strictly contains 0 (LD1).
11. **Null max quirk.** Slot 40 with a null max on a 1024 m city converts to an x range up to 65 · 16 − 0.1 =
    1039.9 m (world cell 80): an occupant placed at x 1030 is found.
12. **Typed type 0.**
    - Slot 41 with type 0 returns occupants of every type.
    - Slot 31 with type 0 finds nothing (unless an occupant has type 0).
    - Slot 23 with type 0 removes nothing.
    - After slot 41 with type T, the shared filter's type is T.
13. **Remove order.** Remove an inserted occupant with shutdown = true and data4 = 3.
    - The queue holds Remove `0x99EF1143` (data4 3), then Destroy `0x99EF1141`.
    - After the call flag 1 is clear and the reference count is one lower than before (the manager's reference is
      gone; the local hold is balanced).
    - Its cells are empty and their holders freed (slot 25 true).
    - Remove of a non-inserted occupant with shutdown = true returns false but still posts Destroy (if initialised).
14. **RemoveOccupants order.** Insert H in (64, 64) and J spanning (64, 65)…(65, 65), then call slot 22(null, true, 0).
    - The queue order is Remove(H), Destroy(H), Remove(J), Destroy(J): walk order x 64 (z 64, then z 65), and J is
      first met at (64, 65).
    - It returns true. Every cell is empty.
15. **Move.** Move an occupant from box cells (65, 64)…(66, 64) to (66, 64)…(67, 64) with post = true.
    - Cell 65 no longer holds it.
    - In 66 it keeps its old position in the list; in 67 it is at the front.
    - Move `0x99EF1146` with data4 0 is delivered (synchronously when occupant `+0x30` is set).
    - Moving without a box change still posts Move.
16. **FindOccupant.** An occupant at stored box (70, 70, 71, 70), with x = z = −1 before the call: slot 27 returns
    true and x, z stay −1. After its stored box is overwritten to (0, 0, 0, 0) without moving its nodes, slot 27 with
    searchAll = true writes (70, 70).
17. **GetBoundingCells.** (5, 5) gives n = 8 with the order of §6.2. (0, 0) gives n = 3: (0, 1), (1, 0), (1, 1).
    (191, 5) gives n = 5.
18. **Iteration.**
    - Slot 44 with min = (64, 999, 64) and max = (64, −5, 64) visits only cell (64, 64): element 1 is ignored.
    - A slot-42 callback that returns false on the first occupant of a cell with three occupants is called once,
      and all three references are balanced.
    - Slot 42 before `Init` returns true; slot 43 before `Init` returns false.
19. **Save round-trip.** Insert K (single cell) and L (spanning two cells). Write, then Read into a fresh manager.
    - The cell lists are equal in order.
    - L holds exactly one manager reference.
    - The record starts `01 00`, then the two sizes and four counts, then 192 and 192 × (192, counts…).
    - Read with version 2 returns false.
20. **Shutdown.** `Shutdown` posts Remove and Destroy for every occupant, returns true, clears the initialised flag
    and nulls the filter. A second `Shutdown` is a no-op.
21. **Slot 34 clamp (OM5).** Occupant M with box x 1000…1020, z 0…16 on a 1024 m city (standard count 64).
    - Slot 34 with min = (63, 0) and max = (70, 0): max is clamped to (63, 0), so the box is x 1008.1…1023.9, and M
      is found.
    - An occupant N at x 1030…1035, z 0…16, is **not** found by that call.
    - After M is removed, slot 34 with min = (63, 0) and a null max finds N: the box reaches 65 · 16 − 0.1 = 1039.9.
22. **Wrapped box (OM6).** An occupant whose bounding box spans x −4100…−4000, z 100…120 (shrunk to −4099.9…−4000.1):
    - its low x cell is −1, stored as byte 255; its high x cell is 1;
    - Insert returns true, sets flag 1 and posts Insert, but no cell holds it;
    - Remove returns true and posts Remove.
23. **Slot 44 batches (OM7).** A slot-44 callback that removes the occupant it was given (slot 21), in a cell with
    three occupants, is called three times. The walk then continues with the next cell.
24. **Shrink precision (OM4).** A box x 100.0…164.0 stores min.x = `float`(100.0 + 0.1f) = 100.09999847… and max.x
    = `float`(164.0 − 0.1f) = 163.89999389….
25. **Filter release.** A filter with count 2 returns 1 from `Release`; at count 1 it is destroyed and 0 is returned.
26. **Default filter.** A pipe (answers `0x4A10AFC2`) is excluded; a building with `0xCAA45670` true is excluded.
27. **Realistic record.** A version-3 record with bytes (1, 0) reads as flags (1, 0, 0).
28. **View level.** A prop with `0xCAA45670` and `0xCAA45671` true has level 2; with only `0xCAA45671`, level 1.
29. **Specification.** The words `Building` and `SubwayLevel` include a building whose view level is 2 and exclude
    a prop at level 2 and a building at level 1.
30. **Cell box.** `"Cell:32 48"` gives the box (2, −FLT_MAX, 3)–(2, +FLT_MAX, 3); the word `0x1234` adds group 0.
31. **Inside lot.** For the lot rectangle (2, 3)–(4, 5) with 16 m cells, a building at (80, 96) is inside and one
    at (80.5, 96) is not.
32. **Data view filter.** With the type list {building} and `+0x2C` clear, a building is in and a flora occupant is
    out; with `+0x2C` set, the reverse; a signpost prop is in while `+0x2E` is set.
