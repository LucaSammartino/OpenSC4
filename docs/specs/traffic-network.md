# Traffic network model (behaviour spec)

Status: M5 behaviour spec. Every fact was checked on the Windows executable `bin/SimCity4.exe` (1.1.641, image base
`0x00400000`), and every address is a Windows address. The earlier write-up in `docs/subsystems/traffic.md` §2.4, §3
and §8 was the starting point (mostly Mac). Corrections to it are listed in §13. Implementers work from this spec, the
public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled
code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags:

- *[compat]*: observable; a replacement must match it.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: consistent with all the evidence, but not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced.

`r24(e)` is `e` rounded to float32 (foundations §18: the x87 unit runs at 24-bit precision, so every add, subtract,
multiply, divide and square root rounds to float32).

Related specs (cited, not repeated):

- [traffic-simulator.md](traffic-simulator.md): the owning class, its agent, work cycle, trip generation, tuning
  exemplar, save record and full slot table.
- [pathfinder.md](pathfinder.md): the search that walks the graph defined here. It also defines the compressed route
  format, §4.15.
- [foundations.md](foundations.md): §3.6 city components, §4 response curves, §5 messages, §6 random generators,
  §16 slot order, §18 precision.
- [simgrid.md](simgrid.md): grid classes; §6.4 grid ownership rule; §7 shared helpers.
- [simulator.md](simulator.md): work slicing.
- [neighbors.md](neighbors.md): neighbour connections.
- [landfill.md](landfill.md) §4.7.2: a caller of the subnetwork queries.
- [docs/abi/allocators.md](../abi/allocators.md).

---

## 1. Role and identity

The traffic simulator (`cSC4TrafficSimulator`) keeps a **cell graph** of the city's transport networks. This spec
covers that graph and everything built on it:

- **Network info lists**: for each cell, at most one record per *transit network*. Each record holds the capacity,
  the network type, the extra step cost and the subnetwork labels.
- **Cell connection records**: for each cell and each of the 9 travel types, which exit sides may be used from which
  entry side. Each record also holds the transit-switch type changes, the car lane-continuity bits and the
  *wormhole* bits.
- **Transit switches**: stations and other lots that change the travel type.
- **Tunnels**.
- **Ferry terminals**, together with their water routes and the **water pathfinder** that computes those routes.
- **Subnetworks**: connected components of the car graph, with per-subnetwork population and destination capacity.
- The **queries** built on these structures (traffic simulator slots 16, 22, 23, 25, 28–33, 36–42).

All of this lives inside the traffic simulator object. It is **not** a separate class and has no class ID of its own.

| Fact | Value |
|---|---|
| Owner | `cSC4TrafficSimulator`, class `0x6990C1AA`, city component #37, IID requested on read `0x68027F37` (foundations §3.6) |
| Interface | `cISC4TrafficSimulator`, vtable `0x00AB33D0` (43 slots in docs/abi; slot table in traffic-simulator.md, our slots in §7) |
| Water pathfinder | `cSC4WaterPathFinder`, an internal object of `0x305C` bytes (constructor `0x007250F0`, Init `0x00725490`). It is not a GZCOM class and is not reachable through any interface. |
| Cell connection array | fixed 256 × 256 records, indexed `(x << 8) | z`, allocated with the global heap (allocator C) |

**Init** (traffic simulator `Init`, `0x0071E7E0`; its order belongs to traffic-simulator.md). The parts that concern
this spec:

1. City size `X = CellCountX`, `Z = CellCountZ` (city slots 93 and 94). Max indices `X−1`, `Z−1`. City bounds
   rectangle `[0, X−1] × [0, Z−1]`.
2. Network info grid: X × Z empty lists.
3. Destination bitmask map: `(X+2) × (Z+2)` bytes (owned by traffic-simulator.md and pathfinder.md §4.10).
4. Cell connection array: every one of the 65 536 records gets `conn[t] = 0`, `switch[t] = 0xFFFF` and
   `wormhole = 0`. The original does not initialise the car-continuity words (§2.1). FreeCity4 sets them to
   `0xFFFF` *[choice]*. No reader sees the difference, because every record of a city cell is rewritten by the first
   full rebuild (§4.2) before any search runs.
5. Transit-switch maps: two grid objects, allocated and constructed in place. Each gets `SetTractSize(1)` and an
   **instance ID** through grid slot 6 `SetInstanceID` (`0x0071EF56`, `0x0071EFD2`):
   - a `uint8` *switch mode mask* grid (element class `0x49B9E602`), instance ID `0x69D5C424`;
   - a `uint32` *switch ID* grid (element class `0x49B9E606`), instance ID `0x69D5C42D`.

   Both IDs are instance IDs, not classes (TN13). `DoLotActivated` writes the mode mask into the first grid
   (`0x00723E2B`) and the switch ID into the second (`0x00723E1F`). The *wormhole* bits are a field of the cell
   connection record (§2.1), not a grid. The name "wormhole byte map" for `0x69D5C424` (traffic-simulator.md,
   save-records.md) was wrong and has been corrected there.
6. Water pathfinder: constructed, then `Init`. Init creates its depth grid and subscribes it to messages (§4.10).
7. Next switch ID = 1 (set by the constructor).

The full rebuild runs in **DoPostCityInit** (message `0x26D31EC1`, `0x00720430`), in this order *[compat]*:

1. Re-read the rules of every loaded transit switch (§4.7.2).
2. Refresh the switches' subway access points.
3. Rebuild the whole city with `DoConnectionsChanged(0, 0, X−1, Z−1)` (§4.2).
4. Register the neighbour connections (traffic-simulator.md).
5. The destination-map boundary pass (traffic-simulator.md).
6. Clear the subnetwork labels (§4.11).
7. Run `UpdateSubnetworks` (§4.11), then build the subnetwork info synchronously with no time limit (§4.12) and
   publish it.
8. Mark every ferry terminal for re-routing and start the ferry-route pass at the first terminal (§4.9.3).

---

## 2. State

### 2.1 Cell connection record (one per cell, 256 × 256)

| Field | Type | Meaning |
|---|---|---|
| `conn[t]`, t = 0…8 | u16 | **Connection word** of travel type `t`. Nibble `e` (bits `4e … 4e+3`) belongs to the **entry side** `e`. Bit `x` of that nibble is set when a type-`t` traveller that entered the cell from side `e` may leave through side `x`. Bit index = `x + 4e`. |
| `switch[t]`, t = 0…8 | u16 | **Switch word**. Nibble `x` = the travel type a type-`t` traveller becomes when it leaves through side `x`. `0xF` = no change. Written only by transit switches (§4.7.4). |
| `priorCar[e]`, e = 0…3 | u16 | **Car lane continuity**. Word `e` belongs to the entry side `e` of this cell. Bit `p + 4x` set = a car that entered this cell from side `e` may leave through side `x` when it entered the *previous* cell (the one through side `e`) from side `p`. The pathfinder checks it for travel types on the car network when the node has a predecessor (pathfinder.md §4.8). |
| `wormhole` | u8 | Bit 1 = subway station (read by the pathfinder, **never set**, §10 Q1). Bit 2 = tunnel portal. Bit 4 = ferry terminal. |

**Sides**: 0 = west (x−1), 1 = north (z−1), 2 = east (x+1), 3 = south (z+1). The neighbour through side `d` is
`(x + DX[d], z + DZ[d])` with `DX = {−1, 0, 1, 0}` and `DZ = {0, −1, 0, 1}`. The side of the neighbour that faces
back is `(d + 2) mod 4`.

**Travel types** `t` (SDK `TravelType`): 0 Walk, 1 Car, 2 Bus, 3 PassengerTrain, 4 FreightTruck, 5 FreightTrain,
6 Subway, 7 ElevatedTrain, 8 Monorail.

**Transit networks** `n`:

| n | Network | Travel types on it (`net(t)`) | Path type `pt(n)` used to ask occupants | Representative travel type `rep(n)` |
|---|---|---|---|---|
| 0 | pedestrian | 0 | 2 | 0 |
| 1 | car | 1, 2, 4 | 1 | 1 |
| 2 | train | 3, 5 | 3 | 3 |
| 3 | subway | 6 | 4 | 6 |
| 4 | elevated (light) rail | 7 | 6 | 7 |
| 5 | monorail | 8 | 7 | 8 |

`net(t)` for t = 0…8 is {0, 1, 1, 2, 1, 2, 3, 4, 5}.

### 2.2 Network info (per cell, per transit network)

Each cell has a list of network-info records, at most one per transit network, in creation order. Each record holds:

| Field | Type | Meaning |
|---|---|---|
| `invCap` | f32 | Inverse capacity: `invCapTable[type]` (§4.5), possibly reduced near intersections (§4.6). |
| `connCount` | u8 | Number of connections counted at the last rebuild (§4.2), truncated to 8 bits. |
| `type` | u8 | Network type 0…12 (eNetworkType: 0 Road, 1 Rail, 2 Highway, 3 Street, 4 WaterPipe, 5 PowerPole, 6 Avenue, 7 Subway, 8 LightRail, 9 Monorail, 10 OneWayRoad, 11 DirtRoad, 12 GroundHighway). |
| `net` | u8 | Transit network `n` 0…5. |
| `flags` | u8 | Bit 0 = an occupant here has network flag `0x40000` (meaning unknown; carried for the simulator). |
| `extraCost` | f32 | Extra step cost: the "damaged road extra step cost" on a damaged car cell, else 0.0. |
| `subnet[e]`, e = 0…3 | u16 | Subnetwork label per **entry side** (§4.11). 0 = unlabelled. Car network only. |

The pathfinder reads `invCap`, `type` and `extraCost` through `GetNetworkInfo(n, x, z)` (non-virtual `0x0070FB30`).
It returns the first record of network `n` in the cell's list, or none.

**Creation quirk** *[quirk]*. Appending a new record at the tail of a cell's list also clears the `flags` byte of the
record that was the tail before it. In practice the rebuild (§4.2) creates the records of a cell in network order and
sets each `flags` right after creating it. The quirk is visible only when one cell gets records from two occupants for
the same network (the second occupant reuses the existing record), or from the ferry phase. *[compat]* where it
changes `flags`.

### 2.3 Other members

| Member | Persisted | Initial | Meaning |
|---|---|---|---|
| network info grid | no | empty | rebuilt by §4.2 at load |
| cell connection array | no | see §1 | rebuilt by §4.2 at load |
| switch ID grid (`uint32`, instance ID `0x69D5C42D`) | yes (traffic save) | 0 | switch ID per cell covered by a transit-switch lot |
| switch mode-mask grid (`uint8`, instance ID `0x69D5C424`) | yes | 0 | OR of `1 << from` over the switch's kind-0 rules |
| next switch ID | yes | 1 | incremented once per created switch |
| transit-switch table | yes (each record, traffic-simulator.md) | empty | SGI `hash_map<u32 id, TransitSwitch>`, bucket hint 100 |
| tunnel table | yes (corrected, traffic-simulator §6.1 row 17) | empty | SGI `hash_map<u16 portalKey, TunnelInfo>`, hint 100. Rebuilt from occupant-insert messages. |
| ferry terminal list | no | empty | list of FerryTerminal (§2.6) |
| ferry departure / arrival tables | no | empty | two SGI `hash_map<u16 cellKey, list position>`, hint 100 each |
| ferry terminal count per ferry type [3] | no | 0 | at most 16 terminals per type |
| ferry pass state | no | idle | current terminal, inner position, neighbour-cell vector, phase (§4.9.3) |
| subnetwork counter | no | 1 | next label to assign (u16) |
| subnetwork info, working and published | no | empty | vectors of SubnetworkInfo (§4.12) |
| `invCapTable[13]`, `rawCapTable[13]` | no | 0 (`invCapTable` is not set by the constructor) | §4.5 |
| intersection-effect count `N` and values `[N]` | no | 0 / empty | §4.6 |
| damaged-road extra step cost | no | 32.0 | §4.5 |
| intersection dirty rectangle and flag | no | clear | §4.6 |
| "subnetworks stale" flag | no | — | set by every rebuild; the work cycle reacts to it (traffic-simulator.md) |
| "network changed" flag | no | — | set at Init and by every rebuild; consumed by traffic-simulator.md *[inferred]* |

**Keys.** A cell key is the u16 `(x << 8) | z`. Only the low 8 bits of each coordinate are used.

### 2.4 Transit switch

| Field | Meaning |
|---|---|
| building | the occupant that carries the rules (one reference held) |
| load | f32, starts at 0.0. Traffic through the switch, maintained by the pathfinder and simulator. |
| time factor | f32, building property `0xE90E25A2` |
| capacity | f32, building property `0xE90E25A3` |
| flags | u8. From the "to" types of the rules: 2 → 1, 3 → 2, 6 → 4, 7 → 8, 8 → 0x10. Plus 0x40 when a rule links subway and elevated train. |
| has access point, access cell | u8, u8 x, u8 z: the subway access point (§4.7.1) |
| two f32 counters | saved, owned by traffic-simulator.md |
| rules | vector of Rule records (below) |
| kind-0 "to" mask | u16, OR of `1 << to` over the kind-0 rules |
| property `0x0BFC0A4C` | u32, simulator use |
| counts as intersection | bool, building property `0x4BFC47B0` (§4.6) |

**Rule**: `kind` (0 or 1), `side[4]` (bool per city side), `from` (travel type), `to` (travel type).

### 2.5 Tunnel record

The tunnel table is keyed by the portal's cell key. Each record holds:

- the other portal's cell (x, z);
- a travel-type mask (always `0x1FE`, §4.8);
- `cost[9]` (f32);
- the network type.

### 2.6 Ferry terminal, links and neighbour links

**FerryTerminal**:

| Field | Meaning |
|---|---|
| departure cell (x1, z1) | cell of the terminal's marker prop `0x2C520000`. Key of the departure table. Gets wormhole bit 4. |
| arrival cell (x2, z2) | cell of the marker prop `0x2C530000`. Key of the arrival table. |
| water cell (x3, z3) | the first water cell next to the marker prop `0x2C540000` that is outside the lot (§4.9.1) |
| ferry type | 0…2, building property `0x0BD86FD3` |
| travel mask | u16 from the table `{1, 3}` by ferry type: type 0 = Walk, type 1 = Walk and Car. Type 2 reads past the table and gets `0xD70A` *[quirk]*. |
| terminal links | list of TerminalLink |
| neighbour links | list of NeighborLink |
| riders (last period, current period) | u16 each (simulator, §4.9.3) |
| lot cell | the lot's location (x, z) |
| re-route flag | u8 |

**TerminalLink** (from A to B):

- B's departure key;
- B's arrival cell (x2, z2) as two u16;
- cost: f32 = `r24(cells × 0.01)`;
- route: `vector<u8>` water route A → B (§4.10.3);
- four u16 usage counters: (last, current) for two counts.

**NeighborLink**:

- the off-map cell (x, z) as i16;
- the in-city edge cell as two u16;
- cost: f32, as for terminal links;
- route to the edge: `vector<u8>`;
- route from the edge: `vector<u8>`;
- four u16 usage counters.

The pathfinder uses these as follows (pathfinder.md §4.8.3):

- At a cell with wormhole bit 4, it looks the cell up in the **departure** table.
- Each terminal link becomes an edge to B's **arrival cell**, with the link cost.
- Each neighbour link becomes an exit to its off-map cell, with the link cost.
- Links are tried in list order: new links are appended, and a re-route keeps an existing link in place.

### 2.7 Neighbour connection edge records

There are four arrays of 8-byte records, one per map edge:

- x < 0, indexed by z;
- x > X−1, indexed by z;
- z < 0, indexed by x;
- z > Z−1, indexed by x.

Each record holds:

- a u16 mask of neighbour-connection types (bit = type);
- a u16 neighbour pool ID;
- an f32 cost.

Add and remove (messages `0x8A0D15DA` and `0xEA0D163E`), the pools and the costs belong to traffic-simulator.md.
The graph uses these records only through **HasMapExit(x, z)**, which is defined for an off-map cell next to the map:

- Map the off-map cell to its in-city cell and the side facing out (`GetNeighborConnectionInnerCell`, below).
- The result is true if any travel type's connection word at that in-city cell has an exit through that side, for
  any entry side.
- Otherwise it is true if the edge record at the off-map cell has mask bit 9 set (a water/ferry neighbour
  connection).

`GetNeighborConnectionInnerCell(x, z)` (`0x0070DDD0`):

- Exactly one of x and z must be off-map (x ∉ [0, X−1] xor z ∉ [0, Z−1]). Otherwise the result is −1.
- x < 0 → (0, z), side 0.
- x > X−1 → (X−1, z), side 2.
- z < 0 → (x, 0), side 1.
- z > Z−1 → (x, Z−1), side 3.

---

## 3. Messages handled (network part)

The traffic simulator's `DoMessage` (`0x007241B0`) dispatches these. Only the network-model effects are listed here;
traffic-simulator.md has the full list.

| Message | Condition | Effect |
|---|---|---|
| `0x99EF1142` occupant inserted / `0x99EF1143` removed | data3 = `0xC772BF98` (network occupant), void1 = occupant, QI `0xA821EF94` (cISC4NetworkOccupant) succeeds | Simulator bookkeeping first: per-network-type counts and budget (DoRoadChanged for types 6, 0, then the first of 10/3/2/12; DoRailChanged for 1; DoSubwayChanged for 7 unless network flag `0x80000000`). If the occupant is of any network type that takes part (§4.1): (a) if it is a tunnel (QI `0x8A4BD52B`), §4.8; (b) cell = city `PositionToCell(occupant position)`; on **remove**, erase the 18 edge-density keys of that cell (traffic-simulator.md); (c) `DoConnectionsChanged(x, z, x, z)`. Always sets the simulator's "occupants changed" byte. |
| `0xE8BDA2E7` lot state changed | void1 QI `0xE818FCED` (a **copy** of the lot with empty trip list, lot-developer.md §3.2) | data1 = 3 → **lot deactivated** (§4.7.5, §4.9.4). Otherwise data2 = 3 → **lot activated** (§4.7.1, §4.9.1). |
| `0x26D31EC1` post city init | | DoPostCityInit (§1) |

The water pathfinder subscribes separately (§4.10.1).

---

## 4. Behaviour

### 4.1 Constant tables *[compat]*

| Table | Values |
|---|---|
| network type takes part in traffic (`0x00AB3164`) | Road, Rail, Highway, Street, Avenue, Subway, LightRail, Monorail, OneWay, DirtRoad, GroundHighway = yes. WaterPipe, PowerPole = no. |
| transit-network mask per network type (`0x00AB3174`) | Road 3, Rail 4, Highway 2, Street 3, Pipe 0, Pole 0, Avenue 3, Subway 8, LightRail 0x10, Monorail 0x20, OneWay 3, Dirt 3, GroundHighway 2. Bit `n` = the type carries transit network `n`. |
| `net(t)` (`0x00AB31A8`) | {0, 1, 1, 2, 1, 2, 3, 4, 5} |
| `pt(n)` (`0x00AB3214`) | {2, 1, 3, 4, 6, 7} |
| `rep(n)` (`0x00AB3244`) | {0, 1, 3, 6, 7, 8} |
| opposite side (`0x00AB3394`) | {2, 3, 0, 1} |
| switch rotation `ROT[facing][i]` (`0x00AB3280`) | facing 0: {2, 3, 0, 1}; 1: {1, 2, 3, 0}; 2: {0, 1, 2, 3}; 3: {3, 0, 1, 2} |
| ferry travel mask by type (`0x00AB32C8`) | {1, 3} (type 2 reads `0xD70A`) |

**Side-of helper** `SideOf(a → b)`: the side of cell `b` that faces cell `a`.

- `a.x < b.x` → 0;
- `a.x > b.x` → 2;
- otherwise `a.z ≥ b.z` → 3, else 1.

Equal cells give 3. This is the same rule as pathfinder.md's EdgeTo.

### 4.2 DoConnectionsChanged(x1, z1, x2, z2) (`0x0071A860`) *[compat]*

This rebuilds the graph over a rectangle. In this order:

1. **R** = the rectangle grown by 2 on every side, clamped to `[0, X−1] × [0, Z−1]`. Add R to the intersection
   dirty rectangle (union) and set the dirty flag (§4.6).
2. For every cell of R:
   - delete its network-info records of all six networks;
   - reset `conn[t] = 0` and `switch[t] = 0xFFFF` for all 9 types;
   - reset `priorCar[0…3] = 0xFFFF` and `wormhole = 0`.
3. Get all occupants of type `0xC772BF98` in R. Use occupant manager slot 41, `(list, &nw, &se, type, −1)`. The list
   uses allocator A and is internal. For each occupant, in list order, that is of at least one taking-part network
   type and does **not** have network flag `0x200000`:
   - c = `PositionToCell(occupant position)`.
   - For n = 0…5 (in this order), with `p = pt(n)` and `count = 0`:
     - For each entry side e = 0…3:
       - For each exit x = 0…3: if the occupant's `PathConnection(p, e, x)` (cISC4NetworkOccupant slot 53, §7.2)
         is true:
         - `count += 1`;
         - for every travel type t with `net(t) = n`: `conn[t] |= 1 << (x + 4e)`;
         - when t = 1 (Car): **LaneContinuity**(c, occupant, e, x) (§4.4).
       - If no exit was found for e: ask the occupant's path info (cISC4NetworkOccupant slot 49 → path info slot 4,
         `(p, e, 0xFF, 0, 0)`). If that is non-zero (the piece allows a turn-around): `count += 1` and
         `SetConnection(c, e, e, t)` for every t on n (§4.3).
     - If `count ≠ 0` (as u16):
       - get or create the record of network n in c;
       - `connCount = count` (u8);
       - `extraCost` = the damaged-road extra step cost if `n = 1` and `IsRoadDamaged(c)` (slot 16), else 0.0;
       - if the occupant has network flag `0x40000`: `flags |= 1`;
       - choose the network type: start "none". For k = 0…12 where `mask[k]` has bit n and the occupant
         `IsOfType(k)`, set `type = k`, with two exceptions: Street (3) is taken only while still "none", and Road (0)
         does not replace Highway (2). The last match wins. If still "none": type 0 and `invCap = 1.0`, else
         `invCap = invCapTable[type]`;
       - create the edge-density entries of the cell for every t on n, if missing (traffic-simulator.md).
4. **Transit switches** over **R1** = R grown by 1 more and clamped (so the original rectangle + 3). See §4.7.4.
5. **Tunnels**: every cell of R1 that is a key of the tunnel table gets `wormhole |= 2`.
6. **Ferries**: every terminal is processed in list order (§4.9.2).
7. Set the "subnetworks stale" and "network changed" flags. Free the occupant list.

The occupants' order decides which occupant's `flags` and type win in a shared cell. The occupant manager's order is
specified in its own spec.

### 4.3 SetConnection(x, z, e, x', t) (`0x0070EAB0`)

1. `conn[t] |= 1 << (x' + 4e)` at (x, z).
2. If `net(t) = 1` and the neighbour through `x'` is in the city: that neighbour's
   `priorCar[(x' + 2) mod 4] |= 0x1111 << e`.

Step 2 allows every exit of the neighbour for cars arriving from this side. Plugins (NAM) hook this function at
`0x0070EAE7`. That hook point is not part of FreeCity4's contract.

### 4.4 Lane continuity (RemoveBadElevatedPaths, `0x00715940`)

Called for each car connection (e → x) of an occupant at cell c:

1. From the occupant's path info, get the car paths (path type 1) that start at entry e (path info slot 14). Keep
   those whose exit byte equals x. Collect the first vertex of each kept path (path info slot 3). If none is
   collected, stop: **nothing changes**.
2. Let P = the neighbour through side e (no bounds check is done; the occupant query returns nothing off-map).
   For each network occupant in P, in list order, that has any of the network flags `0x1F4F` (slot 9) and a path
   info:
   - `M = 0`;
   - for each collected vertex v: query path info slot 7 `(path type 1, v, any entry 0xFF, exit (e+2) mod 4,
     radius 4.0)`. This returns the car paths of P that end within 4 m of v. `M |= 1 << entry` for each one;
   - write nibble x of `priorCar[e]` at c: `:= M`.

The **last** qualifying occupant in P decides *[quirk]*. With no flagged occupant in P, the word keeps `0xFFFF`
from the reset. So the bits are only restricted next to occupants with those flags: elevated and highway pieces
*[inferred]*.

### 4.5 Capacity tuning (read by ReadSimulatorTunableValues `0x00716AE0`, exemplar in traffic-simulator.md)

| Property | Name (re/tuning) | Use | Missing → | Shipped |
|---|---|---|---|---|
| `0xA92356B0` | Damaged road extra step cost | `extraCost` on damaged car cells | unchanged (code default 32.0) | 0.1 |
| `0xA92356B3` | Network Traffic Capacity | 13 floats | arrays unchanged (`invCapTable` then undefined before the first read *[choice]*: FreeCity4 uses 1.0) | 1000, 3000, 4000, 100, 0, 0, 2500, 3000, 3000, 3000, 2000, 0, 4000 |
| `0xA92356BB` | Intersection and Turn Capacity Effect | N = count (u8), `values[i] = r24(1.0 / v[i])`, no zero guard | unchanged | 0.7, 0.8, 0.9 |

When `0xA92356B3` is present, for k = 0…12:

- `v = (k < count) ? value[k] : 0.0`;
- `rawCapTable[k] = v`;
- `invCapTable[k] = r24(1.0 / v)` if `|v| > 1e-10`, else 1.0.

Shipped `invCapTable` bit patterns:

| Capacity | `invCapTable` |
|---|---|
| 1000 | `0x3A83126F` |
| 3000 | `0x39AEC33E` |
| 4000 | `0x3983126F` |
| 100 | `0x3C23D70A` |
| 0 | `0x3F800000` |
| 2500 | `0x39D1B717` |
| 2000 | `0x3A03126F` |

**GetCapacity(n, x, z)** (slot 23):

- returns `r24(1.0 / invCap)` of the first record of network n at (x, z), as x87 float;
- 0.0 when there is no record;
- no bounds check (x, z must be in the city).

So it reports the capacity after the intersection reduction. The round trip is not exact. For example, 1000 gives
999.99994 (`0x4479FFFF`), 3000 gives 3000.0 and 4000 gives 3999.9998. *[compat]*

**GetConnectionCount(n, x, z)** (slot 25): `connCount` of that record, or 0. Returned zero-extended in EAX.

### 4.6 Intersection capacity reduction (UpdateIntersectionData `0x0071B9A0`)

This runs at the start of the next work cycle while the dirty flag is set (traffic-simulator.md). It runs over the
dirty rectangle grown by N and clamped, then clears the flag.

For every car record (n = 1) whose type is not Highway:

1. Find `best`, the distance to the nearest intersection:
   - If `connCount > 2`, `best = 0`.
   - Otherwise, search depth-first from the cell along its car (`conn[1]`) exits, union over entry sides. Stop
     extending at depth `best`; best starts at N + 1. Do not step through tunnel cells (wormhole bit 2) or cells
     without car info.
   - A reached cell is an intersection if its car record has `connCount > 2`, or if it has no car record but a
     transit switch whose "counts as intersection" flag is set.
   - The search's bounds check compares both coordinates with X−1 *[quirk]*. It matters only on non-square maps.
2. Set `invCap = invCapTable[type]`. If `best < N`, then `invCap = r24(invCap × values[best])`.

With the shipped values, a Street (100) gives:

| Position | `invCap` | Capacity |
|---|---|---|
| intersection | `0x3C6A0EA1` | 70.0 |
| distance 1 | `0x3C4CCCCC` | 80.00001 |
| distance 2 | `0x3C360B61` | 90.0 |
| distance 3 or more | `0x3C23D70A` (unchanged) | 100 |

### 4.7 Transit switches

#### 4.7.1 Activation (DoLotActivated `0x00723AF0`, network part)

This applies when the lot's building has property `0xE90E25A1` (a u32 array):

1. If the property's byte size is not a multiple of 4, stop.
2. Build a switch:
   - the building reference;
   - properties `0xE90E25A2`, `0xE90E25A3`, `0x0BFC0A4C`, `0x4BFC47B0`;
   - the rules (§4.7.2) and the flags.
3. If flag 4 (a Subway "to" rule) is set: list the occupants of type `0x8A13918A` in the lot rectangle. The first
   one's cell is the access point. Without one, use the NW cell of the building's bounding cells. Set "has access
   point".
4. Mode mask = OR of `1 << from` over the kind-0 rules with `from < 8`.
5. Write `id = next switch ID` into the ID grid and the mode mask into the mask grid over the lot rectangle. Insert
   the switch under `id`, then increment the next switch ID.
6. If the building has `0x0BD86FD3`, run ferry activation (§4.9.1), then `DoConnectionsChanged(lot rectangle)`.
   Otherwise, if a switch was created, run `DoConnectionsChanged(lot rectangle)`.
7. The traffic-source registration and the one random draw that follow belong to traffic-simulator.md (§12).

#### 4.7.2 Rules (ReadTransitSwitchRules `0x00715CB0`)

Find the lot at the NW corner of the building's bounding cells (lot manager slot 6, bool false). With no lot, no
rules are read *[quirk]*. Otherwise `f = lot.GetFacing()` (slot 23).

For each u32 entry of `0xE90E25A1`, taking bytes `b0…b3` with b0 the least significant:

- `kind = (b0 = 0x82) ? 1 : 0`;
- for i = 0…3: `side[ROT[f][i]] = bit (7 − i) of b1`;
- `from = b2`, `to = b3`.

Kind-0 rules also OR `1 << to` into the "to" mask.

**After a load (TN3, from the exe).** The save **does** contain the rules: `Read` reads them for record versions
above `0x1C` (§6). `DoPostCityInit` still calls this function for **every** switch in the table (`0x00720466`). The
function appends and never clears: `0x00711CD0` only reserves room, and each rule is pushed after the existing ones.
In order:

1. The "to" mask is zeroed, before the lot lookup.
2. With no lot at the NW corner it returns at once. The rules and flags stay as `Read` left them, and the mask
   stays 0.
3. Otherwise the property's n entries are appended and the mask is rebuilt from them. The flags are reset and
   rebuilt from the **first n rules of the vector**, which after a load are the saved copy.

After loading a current save, each switch therefore holds its saved rules followed by the same rules again
*[quirk]*. Nothing rebuilt depends on the duplicates. §4.7.4 applies the same rules twice, in the same order, and so
writes the same values, and the flags use the first n. The duplicates show only in the next save, which writes the
doubled list, so the record grows by n rules per load. *[choice]* FreeCity4's re-read **replaces** the rules (clear,
then read), so a load followed by a save reproduces the file.

For saves of version ≤ `0x1C` (most of the corpus is version 19), `Read` reads no rules and the re-read is their only
source. `Read` also computes its own mask and flags. They survive only when the NW lot is missing, and they differ
from the rules above *[quirk]*:
- the mask ORs the raw `to` of the **kind-1** rules;
- the flags also map `to = 5` to `0x20` (`0x007235FC`).

#### 4.7.3 TransitSwitchEntryCost(id) (non-virtual `0x0071CB20`, x87 return; used by pathfinder.md §4.8)

1. Look up the switch with insert-if-missing semantics *[quirk]*: an unknown ID inserts a default switch.
2. `q = r24(load / capacity)`.
3. If `q < 5.0` (ordered): cost = `r24(r24(Curve(q) × transitMultiplier) × timeFactor)`. Curve is the
   simulator's congestion → inverse-speed curve (`0x00B4B2F0`). `transitMultiplier` is simulator-owned (1.0 at
   construction, then set from a curve in DoPostCityInit and restored by Read; traffic-simulator.md).
4. Otherwise (q ≥ 5, NaN from 0/0, or +∞): cost = `1.7014117e38` (`0x7EFFFFFF`).

#### 4.7.4 Switch phase of DoConnectionsChanged (over R1)

For each cell of R1 with switch ID ≠ 0, let S be the switch (looked up with insert-if-missing).

1. For every travel type except 6 (Subway): `conn[t] = 0` and `switch[t] = 0xFFFF` in that cell.
2. For each side d = 0…3 whose neighbour N = cell + side d is in the city:
   - **Same switch** (N has the same ID): for each kind-0 rule with `side[d]`: `SetConnection(cell, k, d, to)` for
     every k with `side[k]`.
   - **Other cell**: `ns = SideOf(cell → N)`. For each rule with `side[d]`:
     - **Kind 1 (entering the switch)**:
       - If N has a record of `net(to)`: when `from = to`, `SetConnection(cell, k, d, from)` for k = 0…3;
         otherwise set nibble d of the switch cell's `switch[from]` to `to`.
       - Then for each k ≠ ns such that N's `conn[to]` has an exit k from some entry: `SetConnection(N, ns, k, to)`.
     - **Kind 0 (leaving the switch)**:
       - If N has a record of `net(from)`: when `from ≠ to`, set nibble `ns` of N's `switch[from]` to `to`;
         otherwise for each entry j of N with any exit in `conn[from]`, `SetConnection(N, j, ns, from)`.
       - If `from = 1` (Car): for each k ≠ ns such that N's `conn[1]` has an exit k, `SetConnection(N, ns, k, 1)`.

The order is cells in R1 order (x outer, z inner, as the rectangle loop), sides 0…3, rules in stored order.
*[compat]* where several rules write the same switch nibble (the last write wins).

#### 4.7.5 Deactivation (DoLotDeactivated `0x0071E2F0`, network part)

1. Read the switch ID at the lot rectangle's NW cell. If the switch exists:
   - release the building reference;
   - erase the switch;
   - zero the ID and mask grids over the lot rectangle;
   - `DoConnectionsChanged(x1−1, z1−1, x2+1, z2+1)`.
2. Ferry terminal removal (§4.9.4).

### 4.8 Tunnels (DoTunnelChanged `0x007140E0`)

The key is the portal's cell key.

**Remove**: erase the key. No rebuild of the tunnel bit happens here: the rebuild that follows (§3, step c) clears
wormhole bit 2.

**Insert**:

1. The other portal is the tunnel object's paired portal. The original reads it from the concrete
   `cSC4NetworkTunnelOccupant`, not through an interface (§9). Its occupant position gives cell (ox, oz).
2. Network type = the first of 2, 6, 0, 1, 10, 12, 9 that the tunnel `IsOfType`. None → no record.
3. Travel mask = `0x1FE` (every type but Walk) *[quirk]*. The test that was meant to use the type's transit networks
   always passes.
4. `d = r24(√ r24(r24(r24(dz²) + r24(dy²)) + r24(dx²)))`, with (dx, dy, dz) = this portal's 3-D position minus the
   other's, in world units.
5. `cost[t] = r24(r24(d × invMaxSpeed[t][type]) × 0.0625)` for t = 0…8. `invMaxSpeed` is the class static
   (`0x00B4B110`) filled from the shipped "Max speed by Network" tuning (traffic-simulator.md):
   `invMaxSpeed = r24(1 / r24(r24(v × 16.666666) / CellWidthX))`, or FLT_MAX when v ≤ 0.

For types with zero speed on that network the cost is +∞. Example: Road, Car (v = 31, cell width 16), with portals
64 m apart: `invMaxSpeed = 0x3CFDB011` (0.030967744), cost = `0x3DFDB011` (0.123870976).

Each portal's insert message creates its own record. A tunnel works in both directions once both portals are
inserted.

### 4.9 Ferries

#### 4.9.1 Terminal activation (DoFerryTerminalActivated `0x00723620`)

1. `type` = property `0x0BD86FD3` (u8) of the building. Stop if it is missing, if `type > 2`, or if there are
   already 16 terminals of that type.
2. Get the prop occupants (type `0xA823821E`, QI `0xE9793A65`) in the lot rectangle. Classify them by
   `cISC4PropOccupant::GetPropType` (slot 7):
   - `0x2C520000` → departure cell;
   - `0x2C530000` → arrival cell;
   - `0x2C540000` → water cell: the first side d = 0…3 whose neighbour is outside the lot rectangle and is under
     water (terrain slot 34 `+0x88`, `(x, z)`).

   Later props of the same kind overwrite earlier ones. All three must be found, otherwise stop.
3. Append the terminal and count it. Insert its departure key and arrival key in the two tables.
4. For every other terminal of the **same type**, in list order: **RouteAndLinkTerminals**(new, other) (§4.9.3).
5. For every neighbour connection of type 9 (neighbours simulator slot 17 `GetConnectionsOfType(9, list)`; the
   list uses allocator A and is internal): take its cell (connection slot 6) and run
   **RouteTerminalToNeighbor**(new, x, z).

#### 4.9.2 Ferry phase of DoConnectionsChanged

For each terminal, with departure cell T = (x1, z1) and travel mask m:

1. If T is in R1:
   - `wormhole |= 4` at T;
   - make sure T has records for networks 0 and 1. A new record gets `connCount 1`, type 0,
     `invCap = invCapTable[0]` and extraCost 0.
   - For each side d whose neighbour N is in the city, with `ns = SideOf(T → N)`, and for t = 0…8:
     - if m has t: for each entry e ≠ ns of N with any exit in `conn[t]`, `SetConnection(N, e, ns, t)` (roads
       around the terminal lead into it);
     - else clear bit ns from every nibble of N's `conn[t]`.
2. If the arrival cell A = (x2, z2) is in R1: for each side d whose neighbour N is in the city, with
   `ns = SideOf(A → N)`, and for t = 0…8:
   - if m has t: for each pair (ns → x) present in N's `conn[t]` with x ≠ ns, `SetConnection(N, ns, x, t)`. If
     any were set, also `SetConnection(A, s, d, t)` for s = 0…3;
   - else clear N's `conn[t]` nibble ns.

#### 4.9.3 Routing

**RouteAndLinkTerminals(A, B)** (`0x0071BE50`):

1. Water FindPath from A's water cell to B's, giving `cells`, route A → B and route B → A (§4.10).
2. **Failure**: remove A's link to B and B's link to A, if present.
3. **Success**: find or append A's link keyed by B's departure key and set:
   - B's arrival cell;
   - `cost = r24(cells × 0.01)`;
   - route = A → B.

   Do the same for B's link to A, with route B → A. A new link starts with zero counters.

**RouteTerminalToNeighbor(T, x, z)** (`0x0071C0C0`):

1. (ix, iz) = GetNeighborConnectionInnerCell(x, z). Stop if the result is −1.
2. Water FindPath from T's water cell to (ix, iz).
3. **Failure**: remove T's neighbour link for (x, z).
4. **Success**: find or append it and set (x, z), (ix, iz), `cost = r24(cells × 0.01)`, route to the edge and route
   from the edge.

**Periodic re-routing** (WorkOnFerryRoutes `0x0071DC90`, work-cycle state 19, time-sliced like §4.12; cycle order in
traffic-simulator.md).

Entering the state (`0x0072134C`):

1. For every terminal: last riders := current riders and current := 0. The simulator adds the last riders to a city
   total. Each link's last := current and current := 0, for both counter pairs.
2. Set every terminal's re-route flag.
3. Rebuild the vector of type-9 neighbour connection cells.
4. Start at the first terminal.

For each terminal A in list order:

1. For every other terminal B of the same type whose re-route flag is still set: RouteAndLinkTerminals(A, B).
2. Clear A's flag. So each pair is routed once.
3. RouteTerminalToNeighbor(A, cell) for each cell of the vector.

When the list is exhausted, the pass posts message `0x2C06BA82` (§5).

DoPostCityInit sets every flag and starts the pass, so links are computed after load.

#### 4.9.4 Terminal removal (in DoLotDeactivated)

The first terminal (list order) whose lot cell lies in the lot rectangle is removed. At most one terminal per call:

1. Decrement its type count (not below 0).
2. In every terminal reached through its terminal links, delete the link back to it.
3. If the ferry pass is running and points at it, advance the pass.
4. Erase its departure and arrival keys and free it.

No DoConnectionsChanged is run *[quirk]*. The departure cell keeps wormhole bit 4 and its network records until a
later rebuild covers that cell. A search that reaches it then finds no terminal in the departure table and adds no
ferry edges.

#### 4.9.5 Queries

**GetFerryTerminalCount(type)** (slot 32): the count for `type`. There is no bounds check.

**GetFerryRouteBetweenTiles(x1, z1, x2, z2)** (slot 29) returns a pointer to a stored route vector (internal; §9),
or null:

| Case | Lookup | Returns |
|---|---|---|
| (x1, z1) off-map | terminal by **departure** key (x2, z2); its neighbour link with off-map cell (x1, z1) | its route *from* the edge |
| (x1, z1) in city, (x2, z2) off-map | terminal by departure key (x1, z1); neighbour link (x2, z2) | its route *to* the edge |
| (x1, z1) and (x2, z2) in city, (x2, z2) **not** an arrival key | terminal by departure key (x1, z1); neighbour link whose in-city edge cell is (x2, z2) | route to the edge |
| (x1, z1) and (x2, z2) in city, (x2, z2) an arrival key of B | terminal by departure key (x1, z1); its link whose key is B's departure key | that link's route |

"Off-map" means outside `[0, X−1] × [0, Z−1]`.

**GetAllFerryRoutes(list<vector<u8>>&)** (slot 30):

1. Clear the list.
2. For each terminal: append a copy of every terminal link's route, then every neighbour link's route *to* the
   edge.

**GetFerryRoutesInUse(list<FerryRouteInfo>&)** (slot 31):

1. Clear the list. Keep a temporary set of visited terminal keys.
2. For each terminal A, in list order:
   - For each terminal link to B with a non-zero *last* counter (either pair), unless B was already visited: append
     a record with:
     - lot A and lot B (lot manager slot 6 at the lot cells, bool false, each AddRef'd);
     - type;
     - A→B counters, and B→A counters from B's link to A (0 when absent);
     - "neighbour" = 0;
     - neighbour cell = (−2, −2);
     - route A→B and route B→A.
   - For each neighbour link with a non-zero last counter: append a record with:
     - lot A, lot B = null;
     - type;
     - both counter fields of each pair = the link's counter;
     - "neighbour" = 1;
     - neighbour cell;
     - route to the edge and route from the edge.
   - Mark A visited.

FerryRouteInfo layout (`0x34` bytes, copy constructor `0x0056D770`). Its users are cSC4WatercraftManager and the
query tool:

| Offset | Field |
|---|---|
| +0x00 | `cISC4Lot*` A (referenced) |
| +0x04 | `cISC4Lot*` B (referenced, may be null) |
| +0x08 | u32 ferry type |
| +0x0C, +0x0E | u16 counter 1: A→B, B→A |
| +0x10, +0x12 | u16 counter 2: A→B, B→A |
| +0x14 | u8 neighbour flag |
| +0x16, +0x18 | i16 neighbour x, z |
| +0x1C | `vector<u8>` route A→B |
| +0x28 | `vector<u8>` route B→A |

**GetWaterRoute(x1, z1, x2, z2, vector<u8>&)** (slot 33): water FindPath from (x1, z1) to (x2, z2), writing the
forward route into the caller's vector. Returns the success bool in AL. Its caller is
`cSC4WatercraftManager::CreateMarinaBoat`.

### 4.10 Water pathfinder (cSC4WaterPathFinder)

#### 4.10.1 Depth map

The depth map is a `uint8` grid over the city. It is created directly (vtable `0x00A878C0`, not through GZCOM), with
type `0x49B9E602`, and given the ID `0xCBDF6BF8` by its slot 6.

For each cell:

- If surface water `IsNavigable(x, z, 0, null)` (cISTESurfaceWater slot 8) is false: 0.
- Otherwise let `w = GetWaterDepthForCell(x, z)` (slot 6, x87). If `w ≤ 0` or w is NaN: 0.
- Otherwise `byte = ceil(r24(min(w, 40.0) × 5.0))`, giving 1…200. Here min takes 40.0 only when `40.0 < w`.
- The ceiling is computed with the 1.5·2²³ idiom: `r = r24(r24(0.5 + p) + 12582912)` (round-to-even integer), and
  `r − 1` is used when `p = r − 1` exactly. This equals `ceil(p)` for 0 ≤ p ≤ 200.

Updates are lazy. A dirty rectangle (with a flag) is updated before each FindPath and on message `0x6A3B0AFD`.

| Message | Effect |
|---|---|
| `0x26D31EC1` | whole map dirty and updated at once |
| `0x278E6003`, `0x678E6E3A` | dirty ∪= (data1 >> 16) − 1 … data1 & 0xFFFF, (data2 >> 16) − 1 … data2 & 0xFFFF, clamped to the map (terrain change) |
| `0x99EF1142` / `0x99EF1143` with data3 = `0xC772BF98` | if the network occupant's cell is under water (terrain slot 34), dirty ∪= the occupant's cell rectangle (bridges) *[inferred]* |

#### 4.10.2 FindPath(sx, sz, tx, tz, &cells, fwd, back) (`0x00725170`, pop 28, AL)

1. If the map is dirty, update it.
2. Both cells must have the same non-zero water-body ID (surface water slot 9 `GetBodyOfWaterMap`, then grid slot 7
   at the cell). Otherwise fail.
3. A\* over cells:
   - **Nodes**: key = cell key; predecessor; `g`; `f`; open and closed flags.
   - **Node store**: a 3079-bucket table (`(key × 0x9E3779B1) mod 3079`) over pages of 4096 nodes (allocator C).
     It never fails and is reset per call.
   - **Start node**: `g = 0`, `f = r24(√ (float)(dx² + dz²))`, with integer (dx, dz) = start − target.
   - **Open list**: a sorted singly linked list. Insert before the first node with a strictly greater f, so equal f
     is FIFO. A NaN f is treated as "not less". To update a node, unlink it and insert it again.
4. Loop: pop the head. If it is the target, succeed. Otherwise, for directions d = 0…3 in order (DX, DZ as §2.1):
   - Skip b if it is outside `[0, X−1] × [0, Z−1]`.
   - Skip b if its depth byte is 0, unless b is the target.
   - Skip b if it is the current node's predecessor.
   - `g' = r24(r24(r24(r24(40.0 − r24(depth × 0.2)) × 0.1) + g) + 1.0)`. 0.1 is the static at `0x00B0D490`, never
     written. Examples:

     | Depth byte | Step cost |
     |---|---|
     | 200 | 1.0 |
     | 100 | 3.0 |
     | 0 (target on land) | 5.0 |

   - Get or create b. If b is open and not `g' < b.g`, skip. If b is closed and not `g' < b.g`, skip.
   - Otherwise set closed = false, `b.g = g'`, `b.f = r24(√ (float)(Δx² + Δz²) + g')` (to the target),
     predecessor = current. Reposition b if it was open; else insert it and mark it open.
   - Close the current node.
5. The open list empties → fail (AL 0).

**Success**:

- `cells` = the number of nodes on the path, including both ends.
- If `fwd` is given, it receives the route start → target.
- If `back` is given, it receives the route target → start.

#### 4.10.3 Water route format (MakeFinishedPath `0x00724C50`)

The vector is cleared (end = begin) and filled with allocator B:

- `[0, x, z]` of the first cell;
- then runs `[0x40, count, ⌈count/4⌉ bytes]` of unit-step codes (0 = −x, 1 = −z, 2 = +x, 3 = +z, four per byte,
  least significant first). A run holds at most 255 steps; the 256th step starts a new run.

This is the pathfinder's compressed format with travel type 0 (pathfinder.md §4.15). The watercraft code reads it
with `cSC4CompressedRouteReader`.

### 4.11 Subnetworks (UpdateSubnetworks `0x007183B0`, work-cycle state 12)

**Clear** (`0x0070FCE0`): subnetwork counter := 1, and every record's `subnet[0…3]` := 0.

**Seeds**:

1. Forward pass: x = 0 … X−2 (outer), z = 0 … Z−2 (inner). For the cell's first car record (n = 1): if
   `subnet[0] = 0`, Fill(x, z, 0); then if `subnet[1] = 0`, Fill(x, z, 1).
2. Backward pass: x = X−2 … 0, z = Z−2 … 0, the same with entry sides 2 and 3.

The last row and the last column are never seeds *[quirk]*. They are still labelled when reached from other cells.

**Fill(x, z, e)** labels the (cell, entry side) states reachable along directed car edges. It uses travel type
`rep(1)` = Car and an explicit stack (a deque, allocator B, internal).

1. Start only if:
   - the cell has a car record with `subnet[e] = 0` and `connCount ≤ 2`;
   - and the cell is not a tunnel cell (wormhole bit 2).
2. The state is (cell, entry e, next k). For k = 0…3, if `conn[1]` has bit `k + 4e`:
   - `subnet[e] := counter` in this cell's record, and remember that something was labelled.
   - Find the next state:
     - **Off-map neighbour**: if the cell also has bit `e + 4k` (the connection from k back to e) and
       `subnet[k] = 0`, continue in the same cell with entry k. This turns around at map edges.
     - **In-city neighbour N with a car record**, where `subnet[(k+2) mod 4] = 0`: continue at N with entry
       `(k + 2) mod 4`.
     - **N without a car record but with switch ID s ≠ 0**: step further in direction k while the cells stay in the
       city and carry ID s. At the first cell with another ID: if it has a car record with `subnet[(k+2) mod 4] = 0`,
       continue there. This skips straight across a switch lot.
   - When a next state is found, push the current state with k + 1 and move to the next state (k = 0).
3. After k = 3, a tunnel cell (wormhole bit 2) whose tunnel record allows Car:
   - `subnet[e] := counter`;
   - if the other portal's car record has `subnet[e] = 0` (the **same** entry side), push and continue there with
     entry e.
4. Pop and resume. When the stack is empty, `counter += 1` if anything was labelled.

Labels are "first reached" labels, not strongly connected components. A one-way road reached first from seed A
keeps A's label. *[compat]*

### 4.12 Subnetwork info (UpdateSubnetworkInfo `0x00716180`, state 13; WorkOnUpdateSubnetworkInfo `0x0071DC50`)

**SubnetworkInfo** (`0x1C` bytes):

- `population`: `map<u32 developerType, u32>`;
- `capacity[4]`: u32.

When state 13 is entered (in the work cycle, and synchronously in DoPostCityInit):

1. Clear the working vector and resize it to `counter` default entries. Index 0 is unused.
2. Reset the scanning region to the whole city (cell order: x inner, z outer).

For each cell (x, z) of the region with `lot = GetLot(x, z, true)` (lot manager slot 6; only the lot's anchor cell
answers, so each lot is visited once):

1. Rectangle `r` = the lot's bounding rectangle (lot slot 14). Ring = the boundary (BoundaryIterator `0x0060CEE0`)
   of `r` grown by 1 and clamped to the city. Skip the ring cells whose x is outside `[r.x1, r.x2]` **and** whose z
   is outside `[r.z1, r.z2]`, which are the four corners.
2. `ids` = the distinct non-zero `subnet[0…3]` values of the first car record of each remaining ring cell, in order
   of discovery.
3. If `ids` is empty: remove lot property `0x2A4F9C74` (property holder slot 17). Done.
4. Otherwise:
   - set lot property `0x2A4F9C74` = `ids` (u32 array);
   - for every (type, count) in every map of `lot.GetPopulations()` (slot 54): `info[id].population[type] += count`
     for each id;
   - for k = 0…3: `info[id].capacity[k] += GetMaxTripCapacity(lot's property holder, k)` (slot 20) for each id.

After the last cell, the neighbour connections are added. For every connection of the neighbours simulator (slot 16
`GetAllConnections`; list with allocator A, internal), with cell (x, z) from connection slot 6:

1. Map (x, z) to the in-city cell: x = −1 → 0; x = X → X−1; else z = −1 → 0; z = Z → Z−1.
2. If that cell has a car record and `GetNeighborPoolPropertyHolder(1, x, z)` (non-virtual `0x007114E0`) gives a
   holder:
   - `c[k] = GetMaxTripCapacity(holder, k)`;
   - add `c` to `info[subnet[e]].capacity` for **each** e = 0…3 with `subnet[e] ≠ 0`. This is not deduplicated: a
     label shared by two entry sides is counted twice *[quirk]*.

Then the working vector is copied to the published vector. All queries read the published vector.

The work is time-sliced with an adaptive work-unit counter (statics `0x00B4B33C`, `0x00B4B340`, start 1024; the same
scheme as pathfinder.md §4.11). Slicing changes only when the result is published, not the result itself.

### 4.13 Connectivity queries (slots 36–42)

The lot's subnetwork list is its property `0x2A4F9C74`, as written by §4.12. A lot built after the last pass has no
list until the next pass.

| Slot | Query | Result |
|---|---|---|
| 36 | `AreLotsConnected(A, B)` | AL true when both lots have a non-empty list and the lists share an ID. |
| 37 | `GetConnectedOccupantCount(lot, type)` | Maximum over the lot's IDs (with `id < size`) of `published[id].population[type]` (0 when absent). 0 without a list. |
| 38 | `GetConnectedDestinationCount(lot, k)` | k > 3 (signed) → 0. Otherwise the maximum over the lot's IDs (with `id < size`) of `published[id].capacity[k]`. 0 without a list. `cSC4Lot::HasJobAccess` (`0x006BFCE0`) is true when this is > 0 for k = 0 or 1. |
| 39 | `GetSubnetworksForLot(lot, vector<u32>&)` | Clears the vector, then reads the property into it. AL = whether the property exists. |
| 40 | `GetSubnetworksInRectangle(rect, vector<u32>&)` | The rectangle is inclusive (x1, z1, x2, z2). Collect the distinct non-zero `subnet[0…3]` of the first car record of every in-city cell of rect into an SGI `hash_set<u32>` (hint 100 → 193 buckets, identity hash). Clear the vector and append the set in hash order (bucket order; within a bucket, most recent first). For fewer than 194 IDs all below 193, this is ascending order. |
| 41 | `GetSubnetworksInRegion(region, vector<u32>&)` | As slot 40, over the cells of an SC4CellRegion: its bounding rectangle filtered by its bitmap. Row `x − x1`, bit `(z − z1)`, 32 bits per word, least significant first; cells outside width × height are excluded. |
| 42 | `GetOccupantCountForAllSubnetworks(type, vector<u32>&)` | `vector.assign(size, 0)`, then `v[id] = published[id].population[type]` (if present) for id = 1 … size−1. |

Slots 40, 41 and 42 return garbage in AL. The SDK declares bool. A replacement returns true *[choice]*. The landfill
spec's IsZoneConnected uses slots 41 and 42 (landfill.md §4.7.2).

### 4.14 GetDesiredLotInsertionPoint (slot 22)

This returns the simulator's *safe lot iterator* while the work cycle is in state 1 (the lot sweep), else null. The
iterator is obtained once at Init from lot manager slot 15 (`GetSafeLotIterator`).

The lot manager's lot insertion (`0x0072DB60`) asks for it. When non-null, the new lot is inserted into the lot list
through that iterator (just after the iterator's current element, with the iterator stepped back). Otherwise the lot
is appended. Inserting this way keeps an ongoing sweep consistent *[inferred]*. The exact list effect is the lot
manager's (its spec).

### 4.15 IsRoadDamaged(x, z) (slot 16)

True when the cell key is in the simulator's pothole table. The table is maintained by ApplyPotholeChanges
(`0x00715FB0`, traffic-simulator.md). §4.2 reads it for `extraCost`.

---

## 5. Messages posted

| ID | When | Data |
|---|---|---|
| `0x2C06BA82` | the periodic ferry pass (§4.9.3) has finished all terminals | a cRZMessage2Standard posted to the simulator's message server (slot 4): void1 = the simulator (slot 12), and the simulator again through slot 33. Consumers are not identified *[inferred: watercraft manager]*. |

The rebuilds post nothing.

---

## 6. Persistence

The traffic simulator's save record belongs to traffic-simulator.md. For this spec:

- **Saved**:
  - the switch mode-mask grid and the switch ID grid (as serialisable grids);
  - the next switch ID;
  - the switch table: count, then per switch its ID, building reference, load, time factor, capacity, the access
    flag and cell, the two counters, and the rules (count, then kind, four sides, from, to; read only for versions
    above `0x1C`). This is right and the old §4.7.2 remark was wrong (TN3). `DoPostCityInit` re-reads the rules
    anyway and appends them (§4.7.2);
  - the **ferry terminals** (corrected): `Write` `0x00712400` writes the terminal list after the per-type tables
    (count, per-terminal fields, link lists with their f32 costs, s16 cells, u16 counters and route vectors), and
    `Read` `0x00721990` rebuilds them through the same helper as `DoFerryTerminalActivated` (`0x00713DF0`, called
    at `0x00723593` and `0x007235B1`). The byte layout is in traffic-simulator.md §6.
  - the **tunnel table** (corrected): it is record row 17; `Read` fills the `hash_map<u16, TunnelInfo>` at
    `0x00722E18` (save-records.md §7.1.1). The occupant-insert messages during load may add the same keys again.
- **Not saved** (rebuilt at load):
  - cell connection records and network info (§1, DoPostCityInit);
  - subnetwork labels and info;
  - the water depth map (recomputed on message `0x26D31EC1`).
- **Lot property** `0x2A4F9C74` is saved with the lot as an ordinary property. It is rewritten by the next §4.12 pass.

---

## 7. Interface slots

### 7.1 cISC4TrafficSimulator (vtable `0x00AB33D0`), slots owned by this spec

Pop and return channel were checked with `abi_extract.py --check` and by reading the epilogues.

| Slot | Offset | Address | Signature as compiled | Pop | Return | SDK |
|---|---|---|---|---|---|---|
| 16 | +0x40 | `0x00720070` | `bool IsRoadDamaged(int x, int z)` | 8 | AL | ok |
| 22 | +0x58 | `0x00720010` | `cSC4SafeListIterator<cISC4Lot*>* GetDesiredLotInsertionPoint()` | 0 | EAX pointer | **SDK error**: declares `uint32_t`. It is a pointer (or null). |
| 23 | +0x5C | `0x00710320` | `float GetCapacity(uint8 transitNetwork, int x, int z)` | 12 | **ST0** | **SDK error**: declares `uint32_t GetCapacity(uint32_t networkType, …)`. It returns a float on x87, and the first argument is a *transit network* 0…5 compared as a full u32 with the record's byte. |
| 25 | +0x64 | `0x00710370` | `uint8 GetConnectionCount(uint32 transitNetwork, int x, int z)` | 12 | EAX (zero-extended u8) | argument is a transit network, not a network type (SDK name `networkType`) |
| 28 | +0x70 | `0x00714460` | `void GetTransitSwitches(ilist<cISC4Occupant*>&)` | 4 | garbage | SDK says bool. Clears the list (releasing elements, `0x00413E60`), appends every switch building in table order (AddRef, allocator A nodes). |
| 29 | +0x74 | `0x00713B70` | `SC4Vector<uint8>* GetFerryRouteBetweenTiles(long, long, long, long)` | 16 | EAX pointer | **SDK error**: declares `int32_t`. It returns a pointer to an internal vector. |
| 30 | +0x78 | `0x00718C40` | `void GetAllFerryRoutes(SC4List<SC4Vector<uint8>>&)` | 4 | garbage | SDK says bool |
| 31 | +0x7C | `0x0071C250` | `void GetFerryRoutesInUse(SC4List<FerryRouteInfo>&)` | 4 | garbage | SDK says bool |
| 32 | +0x80 | `0x0071FFF0` | `uint32 GetFerryTerminalCount(uint32 type)` | 4 | EAX | ok (no bounds check) |
| 33 | +0x84 | `0x0070EC10` | `bool GetWaterRoute(long x1, long z1, long x2, long z2, SC4Vector<uint8>&)` | 20 | AL | ok |
| 35 | +0x8C | `0x0071CB80` | `bool GetTransitSwitchQueryData(x, z, TransitSwitchQueryData&)` | 12 | AL | SDK names the first two arguments `uint32_t unknown1, unknown2`: they are a cell. Lookup by the switch ID grid, else by a ferry terminal whose lot cell is in the lot at (x, z). The data fields belong to traffic-simulator.md. |
| 36 | +0x90 | `0x00710540` | `bool AreLotsConnected(cISC4Lot*, cISC4Lot*)` | 8 | AL | ok |
| 37 | +0x94 | `0x00711A70` | `uint32 GetConnectedOccupantCount(cISC4Lot*, uint32 developerType)` | 8 | EAX | ok |
| 38 | +0x98 | `0x007106C0` | `uint32 GetConnectedDestinationCount(cISC4Lot*, int k)` | 8 | EAX | ok |
| 39 | +0x9C | `0x00711B50` | `bool GetSubnetworksForLot(cISC4Lot*, SC4Vector<uint32>&)` | 8 | AL | ok |
| 40 | +0xA0 | `0x00718D20` | `void GetSubnetworksInRectangle(SC4Rect<int> const&, SC4Vector<uint32>&)` | 8 | garbage | SDK says bool |
| 41 | +0xA4 | `0x00718F90` | `void GetSubnetworksInRegion(SC4CellRegion<long> const&, SC4Vector<uint32>&)` | 8 | garbage | SDK says bool |
| 42 | +0xA8 | `0x007143B0` | `void GetOccupantCountForAllSubnetworks(uint32, SC4Vector<uint32>&)` | 8 | garbage | SDK says bool |

A replacement returns true wherever the original returns garbage *[choice]*.

### 7.2 cISC4NetworkOccupant (vtable `0x00AA8080`), slots used by the rebuild

| Slot | Offset | Address | Behaviour | Pop | Return |
|---|---|---|---|---|---|
| 53 | +0xD4 | `0x006196A0` | `PathConnection(pathType, entry, exit)`: makes sure path info exists (calls GetPathInfo unless network flag `0x2000` is set), then tests the connection table. With `0xFF` wildcards it returns the number of matches; otherwise a bool. | 12 | AL (u8) |
| 54 | +0xD8 | `0x00619790` | `AddPathConnection(pathType, entry, exit)`: sets the bit for `pathType` and also for path type 0, then sets network flag `0x2000`. | 12 | none |

**SDK error**: the SDK declares slot 53 as `AddPathConnection` (void) and slot 54 as `PathConnectionExists`. The
compiled order is the reverse (checked at `0x006196A0` and `0x00619790`).

---

## 8. Dependencies

| Interface (vtable) | Slot / offset | Method | Pop | Return | Used in |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 93 +0x174, 94 +0x178 | CellCountX / Z | 0 | EAX | Init, water |
| | 95 +0x17C | `PositionToCell(x, z, &cx, &cz)` (floats by value) | 16 | — | §3, §4.8, §4.9 |
| | 91 +0x16C | `CellWidthX` | 0 | ST0 | tuning (traffic-simulator.md) |
| | 83 +0x14C | GetSurfaceWater (cached in `0x00B43CF0`) | 0 | EAX | water |
| `cISC4OccupantManager` (`0x00A8F7F0`, global `0x00B43D0C`) | 41 +0xA4 | GetOccupantsByStandardCityCells, typed | 20 | AL | §4.2, §4.4, §4.7.1, §4.9.1 |
| `cISC4LotManager` (`0x00AB37C0`, `0x00B43D08`) | 6 +0x18 | `GetLot(x, z, bool anchorOnly)` | 12 | EAX | §4.7.2, §4.9.5, §4.12 |
| | 15 +0x3C | GetSafeLotIterator | 0 | EAX | §4.14 |
| `cISC4Lot` (`0x00AB19B0`) | 3 +0x0C | property holder | 0 | EAX | §4.12, §4.13 |
| | 14 +0x38 | GetBoundingRect | 4 | — | §4.7, §4.9, §4.12 |
| | 21 +0x54 | GetLocation | 8 | — | §4.9.1 |
| | 23 +0x5C | GetFacing | 0 | EAX | §4.7.2 |
| | 48 +0xC0 | GetBuilding | 0 | EAX | §4.7.1, §4.9.1 |
| | 54 +0xD8 | GetPopulations (vector of maps) | 0 | EAX pointer | §4.12 |
| `cISC4NetworkOccupant` (`0x00AA8080`) | 9 +0x24, 8 +0x20 | HasAnyNetworkFlag / HasNetworkFlag | 4 | AL | §3, §4.2, §4.4 |
| | 22 +0x58 | IsOfType | 4 | AL | §3, §4.2, §4.8 |
| | 49 +0xC4 | GetPathInfo | 0 | EAX | §4.2, §4.4 |
| | 53 +0xD4 | PathConnection | 12 | AL | §4.2 |
| path info (no SDK; vtable not curated) | 3 +0x0C, 4 +0x10, 7 +0x1C, 14 +0x38 | get path, turn-around test, paths near a point, paths by entry | — | — | §4.2, §4.4 |
| `cISC4PropOccupant` (`0x00AA4900`) | 7 +0x1C | GetPropType | 0 | EAX | §4.9.1 |
| `cISC4Occupant` (`0x00AA41D8`) | 8 +0x20 | GetPosition | 4 | — | §3, §4.8, §4.9.1 |
| `cISCPropertyHolder` (`0x00AA6CD8`) | 17 +0x44 | RemoveProperty | 8 | AL | §4.12 |
| `cISC4NeighborsSimulator` (`0x00AB1EE0`, `0x00B43D8C`) | 16 +0x40, 17 +0x44 | GetAllConnections, GetConnectionsOfType(9) | 4 / 8 | AL | §4.9, §4.12 |
| `cISC4NeighborConnection` (`0x00AB2034`) | 6 +0x18 | cell (x, z) | 8 | — | §4.9, §4.12 |
| `cISTETerrain` (`0x00AB3D18`, `0x00B43CF4`) | 34 +0x88 | cell under water | 8 | AL | §4.9.1, §4.10.1 |
| **`cISTESurfaceWater`** (`0x00AB3B00`, 15 slots, **new**) | 6 +0x18 | `GetWaterDepthForCell(x, z)` | 8 | ST0 | §4.10.1 |
| | 8 +0x20 | `IsNavigable(cellX, cellZ, radius, lot)` | 16 | AL | §4.10.1 |
| | 9 +0x24 | GetBodyOfWaterMap → `cISC4SimGrid<uint16>` | 0 | EAX | §4.10.2 |
| `cISC4SimGrid<T>` | 24 +0x60 GetGridData; 7 +0x1C get value | | 0 / 8 | EAX | §4.6, §4.7, §4.10 |
| `cIGZMessageServer2` (`0x00B43CCC`) | 5 / 6 | Add / Remove notification | 8 | AL | water pathfinder |

Non-virtual helpers that also serve pathfinder.md:

- `GetNetworkInfo` `0x0070FB30`;
- `TransitSwitchEntryCost` `0x0071CB20`;
- `GetNeighborConnectionInnerCell` `0x0070DDD0`;
- `GetNeighborPoolPropertyHolder` `0x007114E0`;
- `GetTrafficNeighborConnection` `0x0070F970`;
- `SetConnection` `0x0070EAB0`;
- `HasMapExit` `0x0070FA40`.

---

## 9. Replaceability

- **Not separable.** The model is internal to `cSC4TrafficSimulator`. A replacement is a replacement traffic
  simulator (class `0x6990C1AA` in the GZCOM registry). The pathfinder reads the following directly (pathfinder.md
  §8, §9):
  - the cell connection array, network info lists, tunnel and ferry tables, switch table and destination map;
  - non-virtual functions of the class.

  So the two classes are replaced together.
- **Raw grid reads** (simgrid.md §6.4):
  - The switch ID grid is read raw (`GetGridData`) by the pathfinder and by §4.6, §4.11 and §4.7.5.
  - The mode-mask grid is written raw by §4.7.1 and §4.7.5.
  - The water depth map is internal, and the original creates it without GZCOM. A replacement may keep it private
    *[choice]*.

  Under the grid-ownership rule, a replacement creates the switch grids as the game's grid classes. They are saved.
- **Memory reached into by this class**:
  - the paired portal of `cSC4NetworkTunnelOccupant` (a member of the concrete class, not an interface method), so
    a replacement tunnel occupant breaks the original simulator;
  - the lot manager's safe-iterator object, returned by slot 22 and used by the lot manager.
- **Containers crossing the interface**: slots 28–31, 33 and 39–42, listed in the final report's allocator rows.
  `GetFerryRouteBetweenTiles` returns a pointer into internal storage (section (3)).
- **Internal only**: occupant lists in the rebuild (A), neighbour connection lists (A), path-info vectors (A/B),
  the Fill stack (B), lot ID vectors (B), hash sets in slots 40/41 (B), water node pages (C) and the cell array (C).
- **Plugins**: NAM hooks SetConnection (`0x0070EAE7`) and the tables `0x00AB3174` and `0x00B09628` (re/prior_art.md
  §3.2). FreeCity4 exposes no such hooks.

---

## 10. Quirks and open questions

Quirks reproduced: the network-info append clears the old tail's flags; the last flagged occupant decides the lane
continuity; the tunnel mask is always `0x1FE` and the tunnel cost is +∞ for types without speed; ferry type 2 reads
the travel mask `0xD70A`; ferry removal does not rebuild the graph; subnetwork seeds skip the last row and column;
neighbour capacity is counted once per labelled entry side; the intersection search compares z with X−1; switch
entry cost inserts unknown IDs; the switch rules need a lot under the building's NW cell.

Open questions:

- **Q1** Wormhole bit 1 (subway station) is tested by the pathfinder (pathfinder.md §4.8.3), but no code in the
  executable sets it. A scan found every write to that byte: the reset, the tunnel bit and the ferry bit. The
  subway-station branch is therefore dead in 1.1.641 *[inferred]*. Subway access works through transit switches
  instead. FreeCity4 never sets the bit *[compat]*.
- **Q2** *Answered*: ferry terminals are saved and re-read (§6), so they do not depend on the lot-activated message
  being re-sent at load.
- **Q3** Network flag meanings: `0x40000` (record flag bit 0), `0x200000` (excluded from the graph), `0x1F4F` (lane
  continuity).
- **Q4** The off-map reads by pathfinder switch moves (pathfinder.md Q4) index the cell array and the switch grid
  outside the city. The cell array is 256 × 256, so x = −1 wraps to row 255 *[inferred]*. FreeCity4 treats such
  reads as empty.
- **Q5** The consumers of `0x2C06BA82` are not identified.

---

## 11. Conformance tests

Unless stated otherwise:

- the city is X = Z = 4;
- tuning as shipped, so `invCapTable[Street] = 0x3C23D70A` and N = 3, values `0x3FB6DB6E`, `0x3FA00000`,
  `0x3F8E38E4`;
- stub network occupants answer `PathConnection` only for path type 1 (car), as listed;
- the path-info turn-around test returns 0 unless listed;
- occupants have no network flags and no lane-continuity neighbours.

1. **Three-cell street (hand-checkable).**

   Setup: Street occupants (IsOfType(3) only) at:
   - A = (1, 1): no car connections; turn-around at entry 2;
   - B = (2, 1): (0 → 2) and (2 → 0);
   - C = (3, 1): turn-around at entry 0.

   Insert them in the order A, B, C, then run DoPostCityInit. After the rebuild:

   | Cell | `conn[1]` = `conn[2]` = `conn[4]` | `connCount` | type | `invCap` | `extraCost` |
   |---|---|---|---|---|---|
   | A | `0x0400` | 1 | 3 | `0x3C23D70A` | 0.0 |
   | B | `0x0104` | 2 | 3 | `0x3C23D70A` | 0.0 |
   | C | `0x0001` | 1 | 3 | `0x3C23D70A` | 0.0 |

   - Every other `conn` word is 0, and `switch[*]` is `0xFFFF`.
   - `priorCar[0…3]` is `0xFFFF` in all cells.
   - There are no records for networks other than 1.
   - The intersection pass leaves `invCap` unchanged (no `connCount > 2`).
   - `GetCapacity(1, 2, 1)` = 100.0 (`0x42C80000`). `GetConnectionCount(1, 2, 1)` = 2. `GetCapacity(0, 2, 1)` =
     0.0.

2. **Subnetworks of test 1.**
   - A.subnet = [0, 0, 1, 0], B.subnet = [1, 0, 1, 0], C.subnet = [1, 0, 0, 0]; counter = 2.
   - The working and published vectors have size 2.
   - A seed at A with entry 0 labels nothing and does not advance the counter.
   - C is never a seed, because x = 3 = X−1.

3. **Lot lists and queries on test 2.**
   - Lot L1 at (1, 2), 1 × 1: its ring (corners excluded) is (1, 1), (1, 3), (0, 2), (2, 2). Its list is [1].
   - Lot L2 at (2, 0): its ring contains (2, 1). Its list is [1].
   - Stub `GetMaxTripCapacity` gives (5, 0, 0, 0) for L1 and (0, 7, 0, 0) for L2. Populations: L1 {type 0x1010: 3},
     L2 none. No neighbour connections.

   Results:
   - `AreLotsConnected(L1, L2)` = true.
   - `GetConnectedDestinationCount(L1, 1)` = 7 and `(L1, 0)` = 5; `(L1, 4)` = 0.
   - `GetConnectedOccupantCount(L2, 0x1010)` = 3.
   - `GetOccupantCountForAllSubnetworks(0x1010)` → [0, 3].
   - `GetSubnetworksInRectangle((0,0,3,3))` → [1].
   - A lot at (0, 3) (ring (0, 2), (1, 3), plus the clamped cells) has no list. `AreLotsConnected` with it is false,
     and property `0x2A4F9C74` is removed from it.

4. **Intersection reduction.**
   - A street cross centred at (2, 2), with every arm piece straight and the centre having
     `connCount = 12 > 2`:
     - centre `invCap = 0x3C6A0EA1`, so GetCapacity = 70.0;
     - the arm cells next to it: `0x3C4CCCCC`, GetCapacity 80.00001;
     - two cells away: `0x3C360B61`, GetCapacity 90.0.
   - Roads (1000): `0x3ABB3EE8` / `0x3AA3D70B` / `0x3A91A2B5`.
   - A Highway record is never reduced.

5. **Capacity table.** With shipped tuning, `invCapTable` = the §4.5 table. `GetCapacity` of an unreduced record of
   type Road = 999.99994 (`0x4479FFFF`), Highway = 3999.9998 (`0x4579FFFF`), OneWay = 1999.9999 (`0x44F9FFFF`).
   Types with capacity 0 (pipe, pole, dirt) → `invCap` 1.0 and capacity 1.0.

6. **Turn-around via SetConnection.** At cell (0, 0), a turn-around at entry 2 for car sets `conn[1] |= 0x0400`
   and the east neighbour's `priorCar[0] |= 0x4444`. A turn-around at entry 0 at (0, 0) sets only `conn[1] |= 1`,
   because the west neighbour is off-map.

7. **Tunnel cost.** A Road tunnel (IsOfType 0, not 2 or 6) with portals at world positions (8, 0, 24) and (72, 0, 24),
   with shipped speeds and CellWidthX 16:
   - both records have mask `0x1FE`;
   - `cost[Car] = 0x3DFDB011` (0.123870976);
   - `cost[Walk] = 0x3F8C6F2E` (1.0971429): stored, but unused because of the mask;
   - `cost[PassengerTrain] = +∞`.

8. **Water route.**
   - Water at depth 3.3 → byte 17, depth 40.7 → 200, depth 0.1 → 1, depth 2.1 → 11.
   - City X = Z = 8 (overriding the 4 × 4 default). Only row z = 0 is water, all bytes 200, one water body. From
     (0, 0) to (5, 0): cells = 6, each step costs 1.0 so the target
     g = 5.0, link cost = `r24(6 × 0.01)` = `0x3D75C28F` (0.06).
   - Forward route bytes: `00 00 00 40 05 AA 02`.
   - Backward route bytes: `00 05 00 40 05 00 00`.
   - Different water-body IDs at start and target → failure, and the vectors are untouched.

9. **Ferry cost encoding.** Path lengths 1, 5, 37, 100, 250 give costs `0x3C23D70A`, `0x3D4CCCCC`, `0x3EBD70A4`,
   `0x3F800000`, `0x40200000`.

10. **Switch entry cost.**
    - load 0, capacity 100: q = 0 → `Curve(0) × multiplier × timeFactor`.
    - load 600, capacity 100: q = 6 → `0x7EFFFFFF`.
    - load 0, capacity 0: NaN → `0x7EFFFFFF`.
    - Unknown ID: inserts a default switch (capacity 0) → `0x7EFFFFFF`.

11. **Rule rotation.** Facing 1, rule u32 `0x01_01_80_00`:
    - b0 = 0x00 → kind 0;
    - b1 = 0x80 → i = 0 → side `ROT[1][0]` = 1 (north);
    - from = 1, to = 1.

    The "to" mask gets `0x0002`, and the mode mask gets bit 1.

12. **Seeded RNG (no draws).** Seed a cRZRandomSuper with 12345. Run tests 1–3, 8 and a DoConnectionsChanged over the
    whole city. Then the generator's next `RandomUint32Uniform(100)` equals that of a fresh generator seeded with
    12345: the network model makes no draws.

    For comparison, DoLotActivated on a switch lot makes exactly one draw on the simulator's own generator (§12),
    after the rebuild.

Test 8's route bytes:

- Forward: header `[0, 0, 0]`, then run `[0x40, 5]`, then codes 2, 2, 2, 2, 2 packed as `0xAA, 0x02`.
- Backward: header `[0, 5, 0]` (start = target cell), then five code-0 steps, packed as `0x00, 0x00`.

---

## 12. Random draws

None. The rebuild, the subnetworks, the queries, the water pathfinder and the ferry routing make no random draws and
call no `rand()`.

The only draw near this code is in DoLotActivated: after the switch, ferry and rebuild steps,
`RandomUint32Uniform(100)` on the simulator's `cRZRandomSuper` is stored in lot property `0x4A611773`. That
generator is seeded from the wall clock at Init (foundations §6.2/§6.3), and the draw belongs to traffic-simulator.md.

Hash iteration orders are deterministic: SGI hash containers with identity hashes, bucket hint 100 → 193 buckets.

---

## 13. Corrections to earlier write-ups

- `docs/subsystems/traffic.md`: the connection word's nibble is the **entry** side and its bit the exit side. The
  car word is indexed by entry side with bits `p + 4x`. GetCapacity returns a float of `1/invCap`, not a raw
  capacity.
- The SDK (§7): slots 22, 23 and 29 have wrong return types, and cISC4NetworkOccupant slots 53/54 are swapped.
