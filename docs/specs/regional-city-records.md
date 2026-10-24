# Spec: regional-city records (`cSC4RegionalCity` in the city file: pending connection changes, `CreateSymmetricConnections`, the thumbnails)

Status: **behaviour spec**, round 1 (re-regional-city-records, 2026-10-21), clean-room input. This document describes
*behaviour*, not code. It was written by the analysis team from the Windows 1.1.641 executable (the GOG build,
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, read in the instruction bytes and in a
Ghidra 12.1.4 project), and checked against the six regions the game ships (`Regions\Berlin`, `Fairview`, `London`,
`New York`, `San Francisco`, `Timbuktu`: 417 city files). Implementers must work from this spec, the public LGPL
gzcom-dll SDK headers, the game's data files and general knowledge only. They must **not** consult decompiled or
disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

**This spec complements [regional-city.md](regional-city.md) and does not repeat it.** That spec owns the object
(identity, fields, slots, tax rates, population maps, commodities, position, the city cache, the quirks). This one adds
what it lacks: how a record is read from and written back into a city file, the queue of pending connection changes
and the full rules of `CreateSymmetricConnections`, the message-handler details, and the city thumbnails stored in the
same file. Read regional-city.md first; where this document says "see regional-city.md §N" the rule is not restated.

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven (read from the bytes, not seen running);
*[quirk]* is surprising original behaviour that is kept on purpose; *[compat-limit]* depends on something
FreeCity4 cannot reproduce (the host's temp folder, the window state), so only the rule is matched, not each value.

Related specs: [regional-city.md](regional-city.md) (the object; §6.2 is the outline this spec completes),
[foundations.md](foundations.md) (§11 the byte layout of the record, the deal lists, TerrainReconciliation and
ConnectionsReady; §3.7 the save sequence; §2.3 strings), [region.md](region.md) and [region-model.md](region-model.md)
(the region that loads, places and reloads the records; region.md §8 and region-model.md §5.1 describe the path form of
the read), [neighbors.md](neighbors.md) (§2.2 the cell conversions, §6.3 and §6.4 creating and deleting a connection,
§9 adjacent cities), [application.md](application.md) (§11 city life cycle: `NewCity`, `SaveCity`, `Read`),
[save-records.md](save-records.md) (the application version record and the other records of a city file),
[network-manager.md](network-manager.md) (the networks that edge connections are laid on).
Edge reconciliation (which neighbouring terrain and network edges must match, the mismatch dialogs) is not covered;
this spec only says which messages and functions reach it.

## 0. Summary

A city file carries a small **regional-city record** that the game can read and rewrite without loading the city, and
up to four **thumbnail pictures**. This spec covers:

- **§2** reading and writing the record in a package, in place; the application's own copy; the placeholder object for
  an empty location;
- **§3** the **pending connection change queue**: entry layout, who appends, who drains, and real examples;
- **§4** `CreateSymmetricConnections` (`0x004AA6D0`), rule by rule (answers regional-city.md question RY1);
- **§5** the message handler and the cache, as far as regional-city.md leaves them open: which object reacts, the
  figures the cache does not touch, `GetWorkforcePercentage`;
- **§6** the city thumbnails: record types, sizes, when they are written and read, the default thumbnail;
- **§7** what was checked against the shipped files; **§8** quirks; **§9** questions; **§10** conformance tests.

Four results that are easy to get wrong:

1. A pending entry is **not a diff of connections**. It is a note written into the file of the city that must apply it,
   in that city's own edge-cell coordinates, by the neighbour that created or deleted a connection (§3.2). Three
   shipped files carry entries (§3.4), and one carries a remove-then-add pair for the same cell.
2. `CreateSymmetricConnections` keeps the **last** of any entries with the same type and cell, sorts the rest by edge,
   treats highway, avenue and ground-highway additions as **pairs** of which only the second half builds anything,
   and **rolls a new connection back** when no network piece ends up beside the edge (§4).
3. A thumbnail is **not a separate file**: it is up to four PNG records inside the city file, and a flagged save copies
   the old pictures instead of rendering (§6).
4. While a city is open the original holds **two regional-city objects** for it, and only the application's copy reacts
   to the messages (§5.1).

## 1. Differences from regional-city.md (and one from application.md)

No outright contradiction with regional-city.md was found. These are the places where the executable gives more than,
or reads differently from, the spec as written. The maintainer may fold them into regional-city.md; the matching
question entries are RY3 to RY6 in [questions/regional-city.md](questions/regional-city.md).

| Where | regional-city.md says | Verified behaviour |
|---|---|---|
| §6.2, pending flag | `1` creates, `0` removes *[inferred]* | Confirmed by the code (the helper that treats incoming connections as new writes 1, the removal helper writes 0, and `CreateSymmetricConnections` creates for non-zero and removes for zero) and by shipped files (§3.4). No longer inferred. |
| §6.2, `CreateSymmetricConnections` | "duplicate pending records are removed" | Of entries with equal type and cell (the flag is not compared) the **earlier** ones are removed and the **last** stays (§4.1). The outline is expanded into §4. |
| §3 slot 13 | listed with the "getters shared with other classes" | `GetWorkforcePercentage` returns the **address** of the stored float, not a float value (§5.4). |
| §5, commodity map | produced, demanded, exported, imported, cost per unit | That is the in-memory order of the five doubles. In the **file** the order is produced, demanded, **imported, exported**, cost per unit (as foundations.md §11.2 says). An implementation that stores them in one order must write them in the other. |
| §1, `SC4ReadRegionalCity` | stream form only | The path form has a version gate and a finish step that can turn a good read into a failure (§2.2). |
| application.md §11.9 | `cSC4App::Read` "returns true only when all three succeed" | It returns **true even when the regional-city read fails**; the previous regional city then stays (§2.4). |

## 2. Reading and writing a record in a package

### 2.1 Stream forms *[compat]*

The regional-city.md §1 text for `SC4ReadRegionalCity(package, &out)` stands. Two additions: when the record cannot be
opened, or the object cannot be created or fails its `Read`, `*out` is **not touched**; and `Init` runs inside the
read, so the object is subscribed to its messages before the caller sees it.

`SC4WriteRegionalCity(package, city)` (`0x005DC1A0`) opens the record `0xCA027EDB / 0xCA027EE1 / 0` for writing with
the helper that puts the `u16 1` prefix in front (the original passes one extra flag, value 1, whose meaning is not
decoded: RR9). If the record cannot be opened it returns false. It asks the object for its serializable interface
(IID `0xE4FDA3D4`) and returns `Write`'s result; an object that does not answer returns false.

`Write` emits the layout of foundations.md §11.2, version 13, always. The counts of the connection list and the two
deal lists are taken from the lists, not from what each element managed to write; an element that does not answer the
serializable interface writes nothing, so a count can then exceed the records that follow *[quirk]*. The maps and
the pending queue are written exactly as they are in memory (the PreSave handler recomputes the maps first,
regional-city.md §4.3).

### 2.2 Path forms: update one record of a city file *[compat]*

| Form | Behaviour |
|---|---|
| `SC4ReadRegionalCity(path, &out)` (`0x005DC220`) | Create a package object; open the file for **reading** (a missing, unreadable or non-package file fails); read the **application version record** (`0xEA3FF4F5 / 0xEA3FF506 / 0xEA3FF4F5`, a bare `u32`); the value must be **62 to 65** inclusive; run the stream form; finish and close. |
| `SC4WriteRegionalCity(path, city)` (`0x005DC2E0`) | The same, but the file is opened for **update** (it must already exist), and the stream form is the write. |

Details that region.md §8 and region-model.md §5.1 do not give:

1. **The version value.** If the version record cannot be read, the result depends on whether the record exists: a
   record that is **absent** counts as **65**; one that **exists but cannot be read** counts as **0**, which fails the
   gate. (The exists-test is a second package call made only after the first read reports an error.) This is the rule
   region-model.md uses; the bytes confirm it. The game writes **65** into every file it saves
   (`SC4WriteAppLoadSaveVersion`, `0x005DBFA0`).
2. **A file outside 62 to 65 is never patched.** The write fails and the file is left as the package layer leaves it,
   so a neighbour saved by an older or newer build keeps its old queue and deals *[compat]*.
3. **The finish step decides the result.** After the stream form the package is told to commit (a call on the package,
   its slot `+0x1C`, meaning *[inferred]*); only if that succeeds are the remaining close calls made and the stream
   result returned. If the commit fails the path form returns **false even though the record was read or written**
   (in the read case `*out` has already been set and keeps its reference).
4. **Only the one record is replaced** in the write form; every other record of the file is untouched. How the package
   layer rebuilds its index is its business and not decoded (RR9). An implementation may rewrite the package.
5. The read form does not set the object's file path; callers do (region-model.md §5.1, `SetFilePath`).

Callers of the path read: the region's `LoadCitiesFromDirectory` and `ReloadCity`, `MoveCity`, the nation's
`ResetTutorialCities`, the region screen's launch command, and `SC4CreateSavePathForCity` (to learn an existing file's
serial number). Callers of the path write: `cSC4App::SaveCity` (the neighbour rewrite, foundations.md §11.1 and §3.7
step 7, application.md §11.16 step 13) and `cSC4Region::MoveCity`.

### 2.3 What a save writes where *[compat]*

- The **current** city's record goes into the *new* package during the save, right after the app version record and
  before the root record (foundations.md §3.7 step 3). The object written is the application's own copy (§5.1).
- After the save succeeded, the record of **every adjacent city, established or not, that has a save path** is
  rewritten in its own file with the path write above, one city at a time. This is the only way pending entries
  (§3) and deals reach a neighbour's file.
- The thumbnails are written separately at PreSave (§6.3), not by this record.

### 2.4 The application's copy: `cSC4App::Read` and `Write` *[compat]*

`cSC4App::Read(stream, package)` (`0x00454620`) is called while a city file loads. It reads a `u16` version that must
be 2, then the city's serialised object (class `0x66BA4DB5`); if either fails it returns false. Then it reads the
regional-city record from the package with the **stream form**. Only when that read succeeds does it release the
application's previous regional city (its `Shutdown`, slot 4) and store the new object as the application's
regional city. In every case where the first two steps succeeded it returns **true**, even if the regional-city read
failed *[quirk]*; the previous regional city (if any) then stays. `Write` is the mirror: version 2, the city, then the
regional city when the application holds one.

### 2.5 The placeholder object of an empty location *[compat]*

When the region screen needs a regional city for a location that has none and was asked to create one
(`cSC4WinRegionScreen::GetOrCreateRegionalCity`, `0x007AEA00`), it makes an **unestablished** object: construct it
directly, `Init`, `SetCitySize(2^k, 2^k)` with `k` the location's size exponent, `SetPosition(x, z, false)` (so nothing
is repaired or marked moved), and `SetCityName` to the localized default name (the string of group `0x6A231EAA`,
instance `0xAA738C4E`, loaded with the game's string loader; "New City" in the English data). It is inserted into the
region's cell map and the cell is looked up again to give the result. It has **no file path**, serial number 0, no
connections and no deals. A city later founded on the location gets its name, serial number and file path from the
city (regional-city.md §7).

## 3. The pending connection change queue

### 3.1 Entry layout *[compat]*

The queue (object offset `+0x94`, a vector of 16-byte entries) holds entries `{ type, x, z, add }`:

| Field | In memory | In the file | Meaning |
|---|---|---|---|
| type | `u32` | `u32` | connection type (neighbors.md §3): Rail 1, Road 2, Highway 3, PowerPole 4, Pipe 5, Subway 6, LightRail 7, Avenue 8, Ferry 9, DirtRoad 10, Monorail 11, GroundHighway 12 |
| x, z | `s32` each | `s32` each | an edge cell of **this** record's city, in connection-cell coordinates (neighbors.md §2.2): `x = -1` west, `z = -1` north, `x = 64·sizeX` east, `z = 64·sizeZ` south |
| add | byte | `u8` | 1: a neighbour created a connection here; 0: a neighbour deleted it |

On disk an entry is 13 bytes with no padding, after a `u32` count, between the connection list and the budget
(foundations.md §11.2). `Read` appends the entries as stored, with no validation and no duplicate removal; the count
limit is `0x100000`.

### 3.2 Who appends *[compat]*

There are exactly three producers; all append at the end without checking for duplicates.

| Producer | Entry written |
|---|---|
| `ChangeSymmetricConnection(connection, add)` (slot 22, `0x004A8DF0`), called on the **neighbour's** regional city by the neighbours simulator when it creates (`add` true) or deletes (`add` false) a connection (neighbors.md §6.3 and §6.4) | The connection's type, its cell converted from the calling (current) city's coordinates into **this** record's city with the conversion of neighbors.md §2.2 (neighbour region cell, then the neighbour's connection cell), and `add`. |
| The removal helper (`0x004A9580`; regional-city.md §6.2 calls it `MarkConnectionsForRemoval`) | For each own connection whose cell has a regional city on the other side that is **established**: `{type, the connection's own cell, 0}`. |
| The incoming helper (`0x004A9220`, `TreatAllIncomingConnectionsAsNew`) | For each adjacent regional city, established or not, and each connection of that city whose other-side city is this record: `{type, that connection's cell converted into this city's cell, 1}`. |

So a pending entry means: "a neighbour has just created (or deleted) a connection of this type at **your** edge cell;
mirror it when you are next loaded". The queue travels in the file of the city that must apply it because the
neighbour's save rewrites that file (§2.3).

### 3.3 Who drains *[compat]*

The queue is read and emptied in one place only: the TerrainReconciliation branch of the message handler (§5.2),
which runs `CreateSymmetricConnections` (§4). Nothing else reads it. Its other uses are the three producers above, the
"clear" helper (`0x004A8570`, which sets the end of the vector to its start and has one caller, the same branch), and
the file reader and writer. `CreateSymmetricConnections` and the three helpers each have exactly one caller, the
handler. The clear helper receives the **vector subobject** in ECX, not the regional-city base: the call site at `0x004ABE09` passes the message-target view plus `0x90`; the helper stores the new end at subobject `+4`. Its descriptive symbol deliberately does not claim a regional-city member calling convention.

On the **current** city's record (the application's copy, §5.1) the handler does the following. `established` is the
city's established flag, `forced` is `data3 ≠ 0` of the message, `moved` is the record's flag:

| Case | Steps before `CreateSymmetricConnections` |
|---|---|
| established, not forced, not moved | none: the queue read from the file is applied as it is |
| anything else (a new city, a forced pass, a moved city) | clear the queue; if `moved`, run the removal helper; then run the incoming helper |

After `CreateSymmetricConnections` the queue is empty and `moved := 0`. So a **new** city starts with an "add" entry
for every connection its neighbours already have toward it; a **moved** city first drops what it had toward
established neighbours and then adds what the neighbours at the new place have; and an **established, unmoved** city
applies only what its neighbours queued in its file since it was last loaded. foundations.md §11.6 gives who posts
TerrainReconciliation.

The queue is **not** applied in the order it was written: §4.1 and §4.2 remove duplicates and sort it first. An applied
entry stays in the file until the next save of that city writes the emptied queue.

### 3.4 What the shipped files show

Of the 417 files, three have entries (found by locating the record's commodity block and reading the 13-byte entries
back from it):

| File (region) | Entries | How they arise |
|---|---|---|
| `City - Konradshohe.sc4` (Berlin), cell (4, 5), 2 x 2 | `{Ferry, (100, 128), add}`, `{Ferry, (99, 128), add}` | Ferries on the north-edge cells `(99, -1)` and `(100, -1)` of the city to the south (`City - Spandau.sc4`, cell (4, 7)). North-edge cell `(x, -1)` gives the cell above, `(4 + x/64, 6)`, which is Konradshohe; the converted cell is `(x, 128)`. |
| `City - Kensington.sc4` (London), cell (2, 6), 2 x 2 | ten entries: Road removes at x = 34, 47, 56, 73, then Road adds at x = 5, 34, 47, 56, 73, then `{PowerPole, (43, 128), remove}`; all at `z = 128` | The city to the south, `City - Fulham.sc4` (cell (2, 8)), has Road connections at its north-edge cells `x = 5, 34, 47, 56, 73`; converted, they are Kensington's `(x, 128)`. The pairs of remove and add for the same cell show that Fulham deleted and recreated four roads between two saves of Kensington. The pole removal has no counterpart in Fulham's current list (its origin was not traced). |
| `City - New City (57).sc4` (London), cell (8, 10), 2 x 2 | `{PowerPole, (-1, 119), remove}` | A west-edge pole removal; the cell to the west holds `City - Sydenham.sc4` (cell (6, 10)); the origin was not traced further. |

The Kensington queue is the case that decides the ordering rule of §4.1: removing and adding the same road must end
as an addition, and it does, because the later entry wins. All other shipped files have an empty queue (the record of a
city whose neighbours never changed their connections).

## 4. `CreateSymmetricConnections` (`0x004AA6D0`) *[compat]*

This answers regional-city.md question RY1. The network-manager and demolition calls are described by what they do to
the edge cell; their exact arguments are not decoded and the network part is *[inferred]* from the call sequence
(RR3).

Inputs: the record's queue, the current city and its services (the neighbours simulator `N`, the network manager, the
occupant manager, the demolition utility, the zone manager, the traffic network map). The function reads these city
services without a null test; its only caller (§3.3) checks that a city exists. Output: connections created or deleted
in the current city, edge pieces placed or updated, the queue emptied, and ConnectionsReady posted (foundations.md
§11.6).

### 4.1 Remove duplicates: the last entry wins

Walk the queue from the start. If a **later** entry has the same type, x and z (the `add` flag is **not** compared),
delete the **earlier** one and re-examine the entry that took its place. Result: for each `(type, x, z)` only the last
entry remains, survivors keep their relative order, and an add followed by a remove leaves the remove while a remove
followed by an add leaves the add.

### 4.2 Sort by edge

Sort the remaining entries by **side**, then `x`, then `z`, all ascending; the type is not part of the key, and the
sort (an introsort) is not stable, so entries that tie on all three keys but differ in type have no defined order. The
side ranks are West 0, North 1, East 2, South 3. The side of a cell is decided in this order: `x < 0` West; else
`z < 0` North; else `x < cellCountX` South; else East. (`cellCountX` is `64 ·` the city's width in region cells. For a
cell on the outer ring the second and later tests identify the side uniquely.)

### 4.3 Pair the wide types, give each entry a direction bit

Only entries with `add` set and a type of **Highway (3), Avenue (8) or GroundHighway (12)** are *wide*. Count them in
sorted order across the whole queue, whether or not they are adjacent: the 1st, 3rd, 5th, ... are the **first half**
of a pair (`pairFlag = false`), the 2nd, 4th, ... the **second half** (`pairFlag = true`). Every other entry has
`pairFlag = true`. Each entry also gets a direction bit:

```
non-wide entries:     dir := false
wide add entries:     dir := pairFlag
                      if the cell is on the North or East edge:  dir := not dir
                      if the drive-side property is false:       dir := not dir
```

"North or East edge" is the test `x ≥ 0` and `z ≤ cellCountZ - 1`, which holds for exactly those two sides. The
**drive-side property** is the boolean exemplar property `0x49BE9565` of `0x6534284A / 0xA998D30B / 1`, read once when
the automata manager starts (global `0x00B21CC1`); the shipped data hold **true**, so only the North/East flip applies
(its meaning is open: RR4). The direction bit is passed to `CreateConnection` as its `flag` (neighbors.md §6.3), where
it flips the orientation of the marker prop.

### 4.4 Connection type to network type

| Connection type | Network | Connection type | Network |
|---|---|---|---|
| Rail 1 | 1 | Avenue 8 | 6 |
| Road 2 | 0 | DirtRoad 10 | 11 |
| Highway 3 | 2 | Monorail 11 | 9 |
| PowerPole 4 | 5 | GroundHighway 12 | 12 |
| Pipe 5 | 4 | **Ferry 9**, and any type 0 or above 12 | none |
| Subway 6 | 7 | | |
| LightRail 7 | 8 | | |

For Pipe the occupant filter layer is **3** and for Subway **2**; for all other networks it is **1** *[inferred]*.
(These network numbers are the same ones neighbors.md §6.5 uses for the reverse mapping.)

### 4.5 Apply each entry, in sorted order

| Entry | Behaviour |
|---|---|
| a type with **no network** (Ferry, or an out-of-range type) | **Add and remove alike** call `CreateConnection(type, x, z, dir)` and nothing else; that call does nothing when the connection already exists (neighbors.md §6.3). A pending Ferry **removal** therefore never deletes anything, and creates the ferry when it is absent *[quirk]*. |
| the network manager has **no network** of that type | The entry is skipped (an add and a remove alike). |
| **add**, a connection of that type and cell **already exists** (`FindConnection` on `N`) | Skipped. |
| **add**, none exists, `pairFlag` **false** (first half of a wide pair) | Only `CreateConnection(type, x, z, dir)`. Nothing is demolished and no network piece is placed. |
| **add**, none exists, `pairFlag` **true** (every other add, including the second half of a wide pair) | The five steps of §4.6. |
| **remove**, no such connection | Skipped. |
| **remove**, the connection exists | `DeleteConnection(type, x, z)` on `N` (neighbors.md §6.4), then the side clean-up of §4.7. |

### 4.6 Adding a connection with `pairFlag` true *[inferred]*

Let the **edge cell** be the entry's cell and the **inner cell** the cell just inside the city on that edge
(`x + 1`, `z + 1`, `x - 1` or `z - 1` according to the side). Let the **strip** be the inner cell and the two cells
beyond it in the same direction (three cells in a line, perpendicular to the edge, at the entry's lateral
coordinate). Each entry clears its own strip, at its own lateral coordinate.

1. **Select the cells to clear.** Start with all cells of the strip. For a **PowerPole** entry, remove from the set every
   cell that already holds an occupant of type `0x2890D4DE` (power pole) or `0x0990B2EA` (power line piece). For every
   other type, remove every cell for which the traffic network map reports that something is already there.
2. **Demolish.** Through the demolition utility, demolish what stands on the remaining cells, with an occupant filter
   whose layer value is the one of §4.4 (1 for roads and the like, 3 for pipes, 2 for subways). The call does not
   ask for confirmation and does not charge money.
3. **Dezone.** For a PowerPole entry only, ask the zone manager to dezone the same cells.
4. **Create the connection:** `CreateConnection(type, x, z, dir)`.
5. **Place the edge piece.** Ask the network manager's network of that type to lay a segment from the strip's first cell
   to its last (the call that places a segment between two cells, with the same arguments the road tool would use);
   then look for the result at the **inner cell**: for a PowerPole, an occupant of type `0x2890D4DE` there; for other
   types, a network occupant (type `0xC772BF98`) of this network. **If none is found, the connection just created is
   deleted again** (`DeleteConnection(type, x, z)`): a connection never survives without a piece next to its edge.
   Then, for every network occupant of this network at the inner cell, set the edge byte of its side to **2** (see
   below). If any of those occupants had an edge byte of **4** on some side (a link to the next cell), the same side
   byte is set to 2 on the occupants of the neighbouring cell in that direction, searched in the order West, North,
   East, South (the first side holding a 4 wins). Finally the segment placement of the first sentence is run again so
   the network recomputes its pieces.

The **edge byte** is part of a network occupant's connection word: a 32-bit value with one byte per side, West in the
low byte, then North, East, South (a side is shifted by `8 × rank`). The mask `0x04040404` picks out the "link" value 4
in each byte. The meaning of the other values (the written 2 on an add, 0 on a remove) is not decoded (RR3).

### 4.7 Removing a connection *[inferred]*

After the connection is deleted, for every network occupant of the entry's network at the **inner cell** (the
occupants returned for that cell with the network occupant filter), the edge byte of the entry's side is set to
**0**. If any of them had a link value 4 on a side, the same byte is cleared on the occupants of the neighbouring cell
in that direction (same search order as §4.6). Then the segment placement is run once more to recompute. A removal
**never demolishes** anything and does not touch the power pole occupants, which carry no edge byte.

### 4.8 Finish

The queue is emptied (its end set to its start) and ConnectionsReady `0x6AC284F3` is posted (foundations.md §11.6).
That happens on every call, whatever was found, including an empty queue.

## 5. Messages and the cache, beyond regional-city.md

### 5.1 Two objects for the current city *[compat]* (*[inferred]* that they are never merged)

The original holds **two distinct regional-city objects** for the city that is open:

- the **region's** object, built when the region loaded (`LoadCitiesFromDirectory`) and held in the region's cell map;
  it is the one the neighbours simulator reaches through `GetCity` (neighbors.md §9.2) and the one that
  `ChangeSymmetricConnection` and `SetLocalDeals` are called on;
- the **application's** object, read from the city file by `cSC4App::Read` while the city loads (§2.4) and returned by
  `cISC4App::GetRegionalCity()`.

Every handler that works "on the current city's record" (PreSave, PostCityInit, TerrainReconciliation, and
PostCityShutdown) acts only when the receiving object is the application's. This follows from the tests the handler
makes; no code that makes the two the same object was found. The consequences: the cache update, the potential and
extrapolated rewrite and the queue draining happen to the application's object, and that object is what the save
writes into the new package (§2.3). Connections the open city creates are queued in the **neighbours'** region objects
(§3.2), and reach their files at the save. The region's own object for the open city keeps its stale statistics until
the region reloads it (`ReloadCity` after the city closes, application.md §11.2).

### 5.2 `DoMessage` (`0x004ABD40`) *[compat]*

| Message | Which objects react | Action |
|---|---|---|
| PostLoad `0x26C63342` | every subscribed object | move the whole of the local deal list L to the end of O, taking L's last element each time (foundations.md §11.3) |
| PostCityShutdown `0x26D31EC3` | every object, but only when the message names the current city | the same move |
| PreSave `0x26C63343` | only the application's object, and only while a city exists | `UpdateCityCache(city)`, then the potential rewrite, then the extrapolated reset (regional-city.md §4.3) |
| PostCityInit `0x26D31EC1` | same | `UpdateCityCache(city)` |
| TerrainReconciliation `0xCA63D517` | same | §3.3 |

Every message returns true. The handlers read data1 and data2 of TerrainReconciliation and ignore them; `forced` comes
from data3.

### 5.3 What the cache leaves alone *[compat]*

`UpdateCityCache(city)` (regional-city.md §7.2) does **not** write the budget or income (`+0x170`, `+0x178`): those
change only through the setters called by the neighbours simulator's `UpdateCommodityStats` (neighbors.md §8.2), which
the cache runs. It also does not touch the extrapolated and potential maps (only the PreSave handler does), the
languages and countries, the birth date, the world coordinates and the three unknown strings. It **replaces** the
connection list with references to the city's **live** connection objects, and it clears and refills the population
map from the demand simulator's summary rather than adding to it. `UpdateImportExport` is exactly foundations.md §11.3;
it resets only imported and exported of the three commodities and leaves produced, demanded and cost per unit alone.

### 5.4 `GetWorkforcePercentage` *[compat]*

Slot 13 returns, in the integer result register, the **address** of the stored float (object offset `+0x30`); it does
not return a float value. The residential simulator's counterpart, which `UpdateCityCache` reads, does the same. A
caller of the original must dereference the result. An implementation that replaces this class must keep the address
convention for plug-ins compiled against the original (the SDK header declares a plain float return: RR6).

## 6. City thumbnails

### 6.1 Where they live and what they hold *[compat]*

A region folder holds no thumbnail file (region-model.md §2). The pictures are records of the city's own package:

| Record (type / group / instance) | Content |
|---|---|
| `0x8A2482B9 / 0x4A2482BB / 0` | picture 0: the city seen from above in the region view's projection |
| `0x8A2482B9 / 0x4A2482BB / 2` | picture 2: the border mask of picture 0 |
| `0x8A2482B9 / 0x4A2482BB / 4` | picture 4: a second rendering (optional) |
| `0x8A2482B9 / 0x4A2482BB / 6` | picture 6: the border mask of picture 4 (optional) |
| `0x2026960B / 0x4A2482BB / 1, 3, 5, 7` | one companion string per picture: always the 12 bytes `08 00 00 00` followed by `Temp.png` |

Each picture record is the bytes of a **PNG** file, unmodified (8-bit RGBA, PNG colour type 6, in all 854 pictures of
the shipped files). The companion string names the format by its extension; the reader takes the temporary file's
extension from it and assumes `.png` if the string is missing.

**The four-picture rule.** Pictures 0 and 2 exist in all 417 shipped files. Pictures 4 and 6 (with their strings) exist
in exactly the ten files that carry a version-13 regional-city record: the files whose application version record is 64
or 65. The 407 files with version 62 have two pictures. The pairing is strict: in every shipped file a picture record
exists exactly when its companion string does.

**Sizes** (measured on all 417 files). The width is `128 · sizeX + 2` pixels: **130** for a 1 x 1 city, **258** for
2 x 2, **514** for 4 x 4 (pictures 2, 4 and 6 have the same width as picture 0). The height is not fixed: 85 to 127
for the small, 149 to 184 for the medium, 277 to 290 for the large, depending on where the city block falls on the
screen at capture. Neither the reader nor the writer accepts a picture of **5,000,000 bytes or more**.

**What they show** *[inferred]*. Picture 0 has real alpha: the city block on a transparent ground. Pictures 2, 4 and 6
have alpha 0 in every pixel. Picture 2 (and 6) is a flat blue field with the outer ring of the city's border cells in
white and the second ring in cyan, the cells drawn at the terrain height by the border-mask renderer
(`SC4RenderRegionBorderMasks`, `0x005DCCF0`; colours `0xFFFFFFFF` and `0xFF00FFFF`). Picture 4 shows terrain and
streets without buildings for the one city looked at (Konradshohe). What produces picture 4 and what the region screen
does with it are not decoded (RR2).

### 6.2 Reading *[compat]*

`SC4ReadCityRegionViewThumbnails(package, &p0, &p2, &p4, &p6)` (`0x005DD480`; `p4` and `p6` may be null). It clears
`p0` and `p2` and then handles the pictures in the order 0, 2, 4, 6:

1. Read the companion string 1 for the temporary file's extension, default `.png`; make a temporary path with that
   extension.
2. For each picture: the record must exist and be smaller than 5,000,000 bytes; its bytes are written to the
   temporary file and decoded by the image library into a 32-bit-per-pixel buffer
   (`SC4ReadCityRegionViewThumbnailBitmapFile`, `0x005DC910`, colour type 9, the type region-model.md §4.1 uses for
   `config.bmp`). The temporary file is removed at the end.
3. **Picture 0 is mandatory**: missing, too large or undecodable, the call fails and every output stays null.
   Pictures **2, 4 and 6 are optional**: a missing record, or one of 5,000,000 bytes or more, leaves that output null
   and the call still succeeds. A record that exists and is small enough but cannot be written out or decoded fails the
   whole call *[quirk]*. Picture 4 is read only if `p4` is non-null, and picture 6 only if `p6` is non-null and
   everything before it succeeded.
4. On success each buffer is stored, with one reference, into its non-null output.

The path form (`0x005DDA40`) opens the file with the version check (the same 62 to 65 window as §2.2, through
`SC4OpenSaveGame`, `0x005DBFE0`) and closes it afterwards; a file outside the window gives no thumbnails.
Callers: the region screen's `PutCitiesInView` (once per city; it falls back to its default pictures when the read
fails, and *[inferred]* drops pictures 4 and 6 when their sizes disagree with each other or with picture 0) and the
progress screen at city load (pictures 0 and 2; the default pictures of the game data on failure).

### 6.3 Writing *[compat]*

**When.** On PreSave, the city (`cSC4City::DoMessage`) calls `WriteRegionViewThumbnail(package, oldPath, flag)`
(`0x00459B20`), where `package` is the package being written, `oldPath` is the path of the file the save will replace
(a string carried by the message) and `flag` is "data4 of the message is non-zero" (the caller's flag of
`cSC4App::SaveCity`; the same value foundations.md §3.7 calls data4). The city keeps a **thumbnail-written** bit, clear
for a city just loaded or created.

1. If `flag` is non-zero the bit is cleared.
2. If the bit is clear, the old file exists and opens (with the version check): **copy** picture 0's record into the
   new package. If that worked, also copy string 1 and, when present, picture 2 and string 3. Pictures 4 and 6 are
   **not** copied. If picture 0 was copied, the thumbnails are done.
3. Otherwise **render** a new set (§6.4); if the render reports failure, write the **default** (§6.5).
4. In every case the bit is then set.

So the first save after a load, or any save with the flag, **reuses** the old pictures (when there are any); a second
save in the same session without the flag renders again. What the flag is meant to stand for is open (RR1).

**Writing a set.** `SC4WriteCityRegionViewThumbnails(package, b0, b2, b4, b6)` (`0x005DDA90`): `b0` must be non-null,
the others may be null. It makes a temporary `.png` path and, for each non-null buffer in the order 0, 2, 4, 6 and only
while everything before succeeded: encode the buffer to the temporary file as PNG (quality argument 100, which PNG
ignores), read the file back (`SC4WriteCityRegionViewThumbnailBitmapFile`, `0x005DCA10`: it must be shorter than
5,000,000 bytes) and store its bytes as the picture record, then write the companion string `Temp.png` at the
following instance. The temporary file is deleted. The result is the success of the last step that ran.

### 6.4 Rendering a set *[inferred]*

`SC4WriteCityRegionViewThumbnail` (`0x005DDEC0`) renders the **current** city from the 3D view into off-screen buffers
and hands them to §6.3. It pauses the game while it runs and restores the camera, zoom and view mode afterwards. The
capture is `128 · sizeClass + 2` pixels wide (which is the width of §6.1) and runs from the top of the screen down to
the city block's centre plus 32, 64 or 128 pixels for the small, medium and large city. The picture is composed of
four renders with the camera moved by the offsets `(1/6, 1/3)`, `(1/3, -1/6)`, `(-1/6, -1/3)`, `(-1/3, 1/6)` of the
city, each with a border-mask twin, and a second pass with the view manager in mode `0xF3` produces pictures 4 and 6.
The capture is only valid while the game window stays active: if a flag object shows it was not, the function waits up
to 10 seconds (polling every half second) and then repeats the whole render, with at most three nested attempts; after
that it writes the default thumbnail. It also writes the default itself when it cannot set up the capture, and it
returns true on every path, so the caller's own fallback to the default does not appear to trigger.
Because the camera, composition and view mode are not specified (RR2), an implementation may draw its own pictures
*[choice]* as long as the records have the format of §6.1 and the widths of §6.1 hold. *[compat-limit]*

### 6.5 The default thumbnail *[compat]*

`SC4WriteDefaultCityRegionViewThumbnail` (`0x005DDDC0`) writes pictures 0 and 2 only. They come from the game data,
type `0x856DDBAC`, group `0x6A1EED2C` (the region templates' group), chosen by the **city size class** (0 small,
1 medium, 2 large):

| Class | Picture 0 instance | Size | Picture 2 instance | Size |
|---|---|---|---|---|
| 0 (1 x 1) | `0x6A231946` | 128 x 87 | `0x6A231947` | 128 x 87 |
| 1 (2 x 2) | `0xEA23195D` | 256 x 151 | `0xEA23195E` | 256 x 151 |
| 2 (4 x 4) | `0x0A2312D9` | 512 x 279 | `0x0A2312D8` | 512 x 279 |

All six exist in `SimCity_1.dat` as PNGs of the sizes shown (measured). The data pictures are decoded and written back
through the set writer of §6.3, so they take the same temporary-file route as rendered ones. Pictures 4 and 6 are not
written, so a city with a default thumbnail has two pictures. If either data record is missing the function returns
false and writes nothing. The default pictures are `128 · size` wide, not `128 · size + 2`.

### 6.6 The render helpers *[compat]*

Added 2026-10-24 (re-thumbs7): the five helpers of `SC4WriteCityRegionViewThumbnail` (§6.4) that the owner
`fn:SC4WriteCityRegionViewThumbnail` holds besides the renderer itself.

- **Opaque bounds** (`0x005DCAD0(&rect)`, buffer in a register): under lock `0x8080`, every pixel whose low three
  bytes OR to more than 3 (any colour channel above 3) gets alpha `0xFF` (slots 21, 22) and grows the bounding box,
  which is returned half-open (`(0, 0, 0, 0)` when no pixel qualifies). True unless the lock fails.
- **Crop** (`0x005DCC00`): when the rectangle is the whole buffer, the buffer itself (with a reference); otherwise a
  new buffer of the rectangle's size and the same colour type (surface service `0x00B43C9C` slot 3), cleared to 0
  (slot 12) and filled with that part of the source by one blit (slot 29) under lock `0x8010`; null on failure.
- **Average of four** (`0x005DD180(&buffers[4])`): a new 32-bit buffer (colour type 9) of the first buffer's size whose
  every pixel is the per-channel mean of the four sources' pixels, `(a + b + c + d + 2) / 4` on each of the three
  colour bytes (the alpha byte becomes 0); all five buffers are locked meanwhile (`0x40`, `0x8040`); null on failure.
  The renderer uses it to combine four captures.
- **Foreground test** (`0x005F85F0`): Win32 `GetActiveWindow() == GetForegroundWindow()`; the renderer checks it
  before and after capturing.
- **Capture call** (`0x005F9510(a, b, c, d)`): the graphics layer (`0x00B43CA4`) as interface `0xE69BFE2A`, its slot 3
  `(a, b, c, d, 0)`, released; 0 without the layer or the interface.

## 7. What was checked against the shipped files

On the 417 city files of the six shipped regions (DBPF index, QFS decompression where flagged, then the layouts above):

| Fact | Result |
|---|---|
| application version record | 62 in 407 files, 64 in 1, 65 in 9. (save-records.md counts 403, 1 and 13 over its own corpus; the figures here are the files of the install.) |
| regional-city record version | 9 in 373, 10 in 34, 13 in 10; the ten version-13 files are exactly the ten with four thumbnails |
| thumbnail pictures | pictures 0 and 2 in 417, pictures 4 and 6 in 10; every picture is a PNG of colour type 6; width `128 · sizeX + 2` for pictures 0 and 2 in all files; a companion string exists exactly when its picture does, and every companion string is `08 00 00 00 "Temp.png"` |
| size and pending entries | sizes (1,1) 157, (2,2) 250, (4,4) 10; pending entries in three files (§3.4) |
| potential population | `0` for a type with no population, else `trunc(max(1.1f · population, 1000))` in a 32-bit float multiply: holds in all 417 (regional-city.md §4.3) |
| extrapolated population | equals the population map in 416 of 417 files |

## 8. Quirks

1. A pending Ferry removal creates the ferry when it is absent and never deletes one (§4.5).
2. A new road connection with no network piece in the inner cell is created and then deleted again (§4.6 step 5).
3. Of a wide pair only the second half demolishes and places a piece; the first half only creates the connection (§4.5).
4. `cSC4App::Read` returns true when the regional-city read failed (§2.4).
5. A path write whose finish step fails reports false although the record was written (§2.2).
6. A save with the flag set reuses the old pictures and does not render (§6.3).

## 9. Open questions

See [questions/regional-city-records.md](questions/regional-city-records.md): RR1 (the PreSave flag), RR2 (the second
capture pass and the border-mask colours), RR3 (arguments and values of the network calls in §4), RR4 (the drive-side
property), RR5 (network numbering and the unpaired half), RR6 (`GetWorkforcePercentage` in the SDK header), RR7
(unwritten fields), RR8 (callers of the new-city preference slots), RR9 (the stream flag of the writer and the package
update). The answers and refinements for regional-city.md are filed as RY3 to RY6 in
[questions/regional-city.md](questions/regional-city.md).

## 10. Conformance tests

1. **Path-form gate.** A package whose application version record is 61 or 66 fails both path forms and the write
   leaves the file untouched; a package with no version record passes; one whose record exists but is unreadable fails.
2. **Application read.** `cSC4App::Read` on a package whose regional-city record is missing returns true and keeps the
   previous regional city; if the package's commit fails after a good stream write, the path write returns false.
3. **Pending entry layout.** The entries `{9, 100, 128, 1}` and `{9, 99, 128, 1}` are the 26 bytes
   `09000000 64000000 80000000 01` and `09000000 63000000 80000000 01`, preceded by the `u32` count 2.
4. **Conversion.** With Fulham at (2, 8) as the current city and Kensington at (2, 6), both 2 x 2: a Road created at
   Fulham's `(34, -1)` appends `{Road, 34, 128, add}` to Kensington's queue; deleting it appends the same with
   `remove`.
5. **Duplicates.** The queue `[(Road,-1,5,add), (Road,-1,5,remove)]` becomes `[(Road,-1,5,remove)]`; the queue in the
   other order becomes `[(Road,-1,5,add)]`. `[(Road,3,-1,add), (Road,3,-1,add), (Road,3,-1,remove)]` becomes the single
   remove.
6. **Sort.** In a 1 x 1 city (`cellCount = 64` both ways) the entries `(64,10)`, `(-1,5)`, `(3,-1)`, `(7,64)` of one
   type sort to `(-1,5)`, `(3,-1)`, `(64,10)`, `(7,64)`.
7. **Pairs.** In a 1 x 1 city with the drive-side property true, Highway adds at `(-1,10)`, `(-1,11)`, `(10,-1)`,
   `(11,-1)` sort in that order and have `pairFlag` false, true, false, true and direction bits false, true, true, false.
   With the property false every direction bit is inverted. Only the 2nd and 4th entries demolish and place pieces.
8. **Ferry.** A pending `{Ferry, (x,z), remove}` on a city with no ferry at `(x,z)` creates one; on a city that has
   one, nothing changes.
9. **Rollback.** A Road add whose network placement leaves no road occupant in the inner cell leaves no connection in
    the list.
10. **Handler cases.** An established, unmoved, unforced record keeps its queue through the handler's preparation; a
    new, forced or moved record has its queue cleared and refilled; a moved record also gets removal entries for its
    connections toward established neighbours; `moved` is 0 afterwards in all cases.
11. **Thumbnails, shipped files.** For each shipped file pictures 0 and 2 exist with width `128 · sizeX + 2`; pictures 4
    and 6 exist exactly in the ten files with application version 64 or 65; each companion string is the 12 bytes
    `08 00 00 00 "Temp.png"`.
12. **Default thumbnail.** For size class 1, pictures 0 and 2 are written from instances `0xEA23195D` and `0xEA23195E`
    (256 x 151) and nothing else.
13. **`GetWorkforcePercentage`.** The slot's result, read as an address, points at a float equal to the stored
    workforce fraction.
