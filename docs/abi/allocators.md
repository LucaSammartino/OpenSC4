# Memory ownership across the game boundary

These are interface facts (see [README.md](README.md)): which allocator owns the memory of containers that the
game and the plugin both touch. They were read from the target executable (SimCity 4 Deluxe 1.1.641, GOG) and
confirmed in-game (runs 12–16 in `docs/manual-checks.md`). No logic is recorded here.

## Three allocator instances

The game has three instances of the same small-block pooled allocator class. **Each instance owns its own
per-size-class pools.** A block must be freed to the instance that allocated it. A block freed to another instance
is linked into that instance's pools while still belonging to the first one. Later, both instances hand out the
same memory, and the game crashes much later in an unrelated place (seen as an access violation at exe
`0x005E16F5` in the pool allocator).

| | Instance | Allocate | Free | Used by |
|---|---|---|---|---|
| **C** | **Global heap** (static object at `0x00B230E0`) | `0x005E55E0` (cdecl `void* (uint32_t size)`) | `0x005E5620` (cdecl `void (void*)`) | the game's `operator new` / `operator delete` |
| **B** | **STL node allocator** (static object at `0x00B62698`) | `0x0090CF54` (cdecl `void* (uint32_t size)`) | `0x0090CF63` (cdecl `void (void*)`) | the game's standard containers declared with the default allocator (`std::allocator`), and the `cRZString` character buffer |
| **A** | **Framework allocator service** (a heap object created at start-up; service ID `0x3AE4BEA3`, IID `0x3AE4BEAB`) | `cIGZAllocatorService::Allocate`, slot 3 | `cIGZAllocatorService::Deallocate`, slot 4 | containers declared with `cGZAllocatorServiceSTLAllocator` (the game's `ilist<>`), and the gzcom-dll SDK's `SC4List`, `SC4Vector`, `SC4HashSet` and `SC4String` |

The SDK containers always use the framework service. They are therefore only compatible with game containers
whose allocator is also the service.

A rule of thumb that held for every crossing below: a game signature that declares `std::list<…>` or
`std::vector<…>` with the default allocator uses **B**; one that declares `ilist<…>` uses **A**. The type names
come from the Mac build's symbols, which give the declared container type of every method listed here.

Every block from these allocators has a 4-byte header in front of it. The byte just before the returned pointer is
the size class. The headers are the same in every instance, so they cannot tell the instances apart.

Only requests of up to 2044 bytes (size + 4-byte header ≤ 2048) come from the per-instance pools. Larger requests
go to the C runtime `malloc` (`0x009F066E`) with a header of `0x80000000 | size`, and the free functions pass such
blocks to the C runtime `free` (`0x009F015B`) whichever instance they are given to (checked for B, `0x005051C0`,
and C, `0x005E5620`). A large block freed to the wrong instance therefore survives, which can hide the bug.
Containers are usually small, so do not rely on this.

## Containers filled or freed by the other side

Legend: **A** framework service, **B** STL node allocator, **C** global heap (see above). Every list here, game
or SDK, is one pointer to a 12-byte head node; every node is 12 bytes `{next, prev, value}`. Every vector is
`{begin, end, capacity_end}`. The layouts match the SDK's, so only the allocator decides compatibility.
"One reference each" means the method AddRefs every element it puts in the container, and whoever destroys the
container must release each element once.

### (1) The game fills the container, the plugin frees it

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence |
|---|---|---|---|---|---|
| `cISC4LotManager::GetLotsInCellRect` (7) | list of lots | B | **no**: free the nodes with `0x0090CF63` | clears the list first: **releases every element** (slot 2) and frees its node with B, keeping the head node; then appends with one reference each and returns the number appended. See below | `0x0072E080` → clear `0x00473EC0` |
| `cISC4LotManager::GetLotsSurroundingCell` (8, the `(x, z, list, flags)` overload) | list of lots | B | **no** | does not clear; appends; one reference per appended lot. Reaches slot 10 through slots 6 and 9 via the vtable | `0x0072A230` |
| `cISC4LotManager::GetLotsSurroundingLot` (9, the `(lot, list, flags)` overload) | list of lots | B | **no** | does not clear; appends; one reference each. Reaches slot 10 through the vtable | `0x0072A750` |
| `cISC4LotManager::GetLotsSurroundingRect` (10) | list of lots | B | **no** | does not clear; appends only lots that do **not intersect** the rect; uniqueness is checked against the whole list, including elements present before the call; one reference each; returns the number appended in this call | `0x0072E2B0` |
| `cISC4Lot::GetLotOccupants` (49) | list of occupants | A | yes | does not clear; appends; one reference each | `0x006BFF70` → occupant manager slot 40 `0x0049E250` → slot 36 `0x004A0670`; nodes `0x00413DD0` |
| `cISC4NeighborsSimulator::GetAllConnections` (16) | list of neighbour connections | A | yes | clears first (A, releases each); one reference each | `0x006CFF00`; clear `0x00411D70`; insert `0x004A86F0` |
| `cISC4NeighborsSimulator::GetConnectionsOfType` (17), `GetConnectionsOfTypeToCity` (18), `GetConnectionsToCity` (19) | list of neighbour connections | A | yes | clears first; one reference each | `0x006CFF30`, `0x006D0010`, `0x006D0100`; clear `0x00411D70` |
| `cISC4NeighborsSimulator::GetAdjacentCities` (20), `GetConnectedCities` (21), `GetAllConnectedCities` (22) | list of regional cities | A | yes | clears first; one reference each | `0x006D01E0`, `0x006D1E00`, `0x006D1AB0`; clear `0x00411DF0`; nodes `0x00411B60` |
| `cISC4NeighborsSimulator::ProposeDeals` (25) | list of **neighbour deals** | A | yes | clears first; one reference each | `0x006D21A0`; clear `0x00421660`; nodes `0x00420FD0` |
| `cISC4NeighborsSimulator::FindDealsAll` (28) | list of neighbour deals | A | yes | list assignment: existing nodes are reused, surplus nodes are released and freed (A), missing ones are appended (A); one reference each | `0x006D4670` → `0x004A9150` (`0x004A8660`, `0x004A8790`) |
| `cISC4NeighborsSimulator::FindDeals` (29, 3 arguments; 30, 2 arguments) | list of neighbour deals | A | yes | clears first; one reference each | `0x006D0840`, `0x006D0760`; clear `0x00421660` |
| `cISC4RegionalCity::GetAllConnections` (21) | list of neighbour connections | A | yes | clears first; one reference each | `0x004A8DB0`; clear `0x00411D70`; insert `0x004A86F0` |
| `cISC4RegionalCity::SetLocalDeals` (69) | list of neighbour deals (input) | A | yes | copies the elements into the city's own list (A, one reference each). It may also **erase nodes from the list it is given** (release the element, free the node with A) | `0x004A90D0` → `0x004A8EC0`; erase `0x004216E0` |
| `cISC4RegionalCity::GetLocalDeals` (70) | list of neighbour deals | A | yes | list assignment, as `FindDealsAll` | `0x004AA0C0` → `0x004A9150` |
| `cISC4RegionalCity::GetPointsOfInterest` (72) | vector, **12-byte elements** | B | **no** | vector assignment: when the type is present, reuses the caller's capacity or frees the old buffer with B and allocates a new one with B; when absent, the vector is left untouched. The elements are 12 bytes, not `uint32_t` | `0x004A8A40` → `0x007AD7A0` |
| `cISC4ResidentialSimulator::GetSchoolSystemTotals` (14), `GetHospitalSystemTotals` (15) | vector of `uint32_t` (input) | none | n/a | the vector is a read-only input (const reference); the query-data argument receives plain values. No memory crosses | `0x006FA240`, `0x006FA260` → `0x006FA070` |
| `cISC4ResidentialSimulator::GetSchoolBuildings` (47), `GetHospitalBuildings` (49) | list of occupants; vector of `uint32_t` (input, may be null) | A (list) | yes | does not clear; appends; one reference each. The vector is read-only | `0x006FADE0`, `0x006FAE50` → `0x006FACE0` |
| `cISC4BuildingDevelopmentSimulator::GetAllBuildingTypes` (11), `GetAllBuildingFamilyTypes` (13) | vector of `uint32_t` | B | **no** | clears first (end = begin, keeps the buffer), then appends in hash order with B. EAX is garbage on return | `0x00697C50`, `0x00697D10` |
| `cISC4BuildingDevelopmentSimulator::GetAllFoundationTypes` (21), `GetAllLotRetainingWallTypes` (27) | vector of `uint32_t` | B | **no** | clears first, then appends with B; returns the count | `0x00698050`, `0x00698180` |
| `cISC4BuildingDevelopmentSimulator::GetBuildingTypesByFamily` (18, the vector overload) | vector of `uint32_t` | B | **no** | for a family ID: vector assignment from the family's member list (B, may free the caller's buffer), returns the count; for a building type: clears, pushes one, returns 0; otherwise untouched | `0x00697EF0`; assign `0x00462C10` |
| `cISC4BuildingDevelopmentSimulator::GetBuildingTypesByFilter` (17), `GetBuildingTypesByFamily` (19, the set overload) | STLport `hash_set<uint32_t>` (not `std::unordered_set` as the SDK declares) | B (nodes and bucket vector) | **no** | does not clear; inserts; returns a count | `0x00699C50`, `0x00699D20`; insert `0x00624DD0`, resize `0x004F0940` |
| `cISC4OccupantManager::GetOccupantsByBBox` (36) and `GetOccupantsByOccupantManagerCells` (38): the **filter** overloads `(list&, min, max, cISC4OccupantFilter*, uint32_t flags)` | list of occupants | A | yes | does not clear; appends; one reference each (AddRef before the node is linked). The filter may be null (no filtering). Returns a bool in AL (true when something was appended). Returns false without touching the list when the manager is not initialised or `flags` is 0. Pop 20 | `0x004A0670` (AddRef `0x004A0942`, node `0x004A0991`); `0x004A0A70` (AddRef `0x004A0C3C`, node `0x004A0C8B`) |
| `cISC4OccupantManager::GetOccupantsByStandardCityCells` (40): the **filter** overload; the 4th argument is a `cISC4OccupantFilter*`, so the call `(list, &min, &max, 0, -1)` means "no filter, all flags" | list of occupants | A | yes | converts the cell rectangle to a box and calls slot 36 through the vtable: same contract as slot 36. Pop 20 | `0x0049E250` → `call [vtbl+0x90]` at `0x0049E357`. Pathfinder caller `0x006D7CDC`: head allocated with A (12 bytes, cached service pointer `0x00B207E4`), list destroyed by `0x00413FE0` |
| `GetOccupantsByBBox` (37), `GetOccupantsByOccupantManagerCells` (39), `GetOccupantsByStandardCityCells` (41): the **typed** overloads `(list&, min, max, uint32_t occupantType, uint32_t flags)` | list of occupants | A | yes | thunks. A non-zero type is written into a filter object owned by the manager, and the call is forwarded (tail jump) to slot 36, 38 or 40 with that filter. Type 0 forwards with a null filter. Same contract as the target slot. Pop 20 | `0x0049DD40`, `0x0049DD80`, `0x0049DDC0` |
| `cISC4OccupantManager::ReleaseOccupantList` (48) | list of occupants | A | yes | releases every element once (vtable slot 2), then frees every node with A (`0x00413E60`) and leaves the head node self-linked. It does **not** free the head node. Pop 4. Use it only on lists whose nodes are A blocks and whose elements each hold one reference | `0x0049EF30` → `0x00413E60` |
| `cISC4Demolition::DemolishLots` (7) | `std::list<cRZAutoRefCount<cISC4Lot>> const&` (input) | none used (B by declaration) | yes, read-only | only walks the list: no erase, no node allocation or free, no AddRef or Release of the elements, and the list object is never written. An empty list returns true at once. Each element is one `cISC4Lot*` (a `cRZAutoRefCount` is a single pointer). Any allocator works, because the game never frees a node of this list; the caller keeps its references. Pop 32 | `0x004670C0`: list reads at `0x004670CC`, `0x004670FC`, `0x0046722F`; no allocator call touches the list |
| `cISC4DispatchManager::GetDispatchIDArray` (12) | caller's `uint32_t` array | none | n/a | nothing is allocated. See *Dispatch ID array* below. Pop 12 | `0x00469DA0` |
| `cISC4DispatchManager::GetDispatchInfo` (10) | caller's 0x2C-byte struct | none | n/a | plain 11-dword copy (layout in [layouts.md](layouts.md)); no reference is taken on the pointers it copies. Pop 8 | `0x00469D40` |
| `cISC4LotConfigurationManager::GetLotConfigurationsByFilter` (9) | STLport `hash_set<cISC4LotConfiguration*>` (the SDK says `SC4HashSet`) | B (nodes and buckets) | **no** | does not clear; inserts; one reference each, the caller releases each | `0x0048B1A0`; caller `0x00709650` |
| `cISC4LotConfiguration::GetRequiredRoads` (21) | `std::vector<bool>` | B (bit buffer) | **no** | the callee fills; the caller frees the bit buffer with B | callers `0x00705E64`, `0x00706838` |
| `cISC4Demolition::DemolishLots` (7) | list of lots (input) | B (head and nodes) | **no** | the caller (tract developer) builds the list with one referenced lot and frees it afterwards; the demolition code reads it *[inferred: no erase seen]* | callers `0x0070B519`, `0x007069E4`, `0x0070AB59`; original `0x007E98C0` |
| `cISC4PathFinder::GetCompressedRoute` (32) | `std::vector<uint8_t>` plus a `uint32_t&` mask | B | **no** | clears first (end = begin, buffer kept), mask := 0, then appends with B growth (`0x00461A10` → `0x0090CF54`; the old buffer is freed with `0x0090CF63`) | `0x006D7070` |
| `cISC4PathFinder::FindPath` (23), into the deque registered with `SetRouteInfo` (8) | the caller's `std::deque<RouteData>` (3-byte elements, 0x7E-byte node buffers) | B | **no** | slot 8 stores the pointer. On success FindPath clears the deque (node buffers freed with `0x0090CF63`, `0x006DB440`) and pushes each route node at the front (node buffer `0x0090CF54` at `0x006D885D`, map regrowth `0x004321B0`) | `0x006D8710` |
| `cISC4TrafficSimulator::GetTransitSwitches` (28) | `ilist<cISC4Occupant*>` | A | yes | clears first (releases each element, frees the nodes with A via `0x00413E60`), then appends; one reference each. AL is undefined on return | `0x00714460` |
| `cISC4TrafficSimulator::GetAllFerryRoutes` (30) | list of `vector<uint8_t>` (compressed routes) | B (0x14-byte nodes and the vector buffers) | **no** | clears first (`0x006FCBD0`), then appends copies | `0x00718C40` |
| `cISC4TrafficSimulator::GetFerryRoutesInUse` (31) | list of `FerryRouteInfo` (0x3C-byte nodes, each holding two lot pointers and two `vector<uint8_t>`) | B (nodes and vector buffers) | **no** | clears first, then appends copies; one reference on each of the two lots | `0x0071C250`; copy constructor `0x0056D770` |
| `cISC4TrafficSimulator::GetWaterRoute` (33) | `vector<uint8_t>` | B | **no** | clears first (end = begin), then appends with B | `0x0070EC10` → `0x00724C50` |
| `cISC4TrafficSimulator::GetSubnetworksForLot` (39) | `vector<uint32_t>` | B | **no** | clears first, then fills from lot property `0x2A4F9C74` | `0x00711B50` |
| `cISC4TrafficSimulator::GetSubnetworksInRectangle` (40), `GetSubnetworksInRegion` (41) | `vector<uint32_t>` | B | **no** | clears first, then appends with B. AL is undefined on return | `0x00718D20`, `0x00718F90` |
| `cISC4TrafficSimulator::GetOccupantCountForAllSubnetworks` (42) | `vector<uint32_t>` | B | **no** | `assign(size, 0)` with B, then counts. AL is undefined on return | `0x007143B0` |
| `cISC4ZoneManager::PlaceZone` (11), argument 10 `outLots` | `std::set<cRZAutoRefCount<cISC4Lot>>*` (MSVC tree; the SDK says `intptr_t`) | B (nodes) | **no** | does not clear; inserts each lot to be demolished (unique); AddRef only on a new insertion. The caller releases each element and frees the nodes with B | `0x004DDCBB` → `0x00467950` → `0x00467720` |
| `cISC4ZoneManager::PlaceZone` (11), argument 1 `region` | `SC4CellRegion<long>&` (in/out) | C (row array and rows) | n/a | the game only clears the bits of rejected cells; it never reallocates or frees the region | `0x004DD780` |
| `cISC4ZoneDeveloper::HighlightParcels` (5), argument 4 `out` | `SC4CellRegion<long>*` stored into `*out` | C (0x24-byte object, row array, rows) | n/a | the game allocates a new region (when it narrows the result it frees its first region and stores a new one). The caller owns the final region and frees rows, row array and object with C *[inferred: caller not traced]* | `0x00733A1C`, `0x00733E89`, `0x00733891` |
| `cISC4LotManager::GetSafeLotIterator` (15) | 8-byte iterator block `{current node, owner}` | C | n/a (raw struct) | registered in the owner's iterator list (owner + 4, B node, insert `0x0072BA10`) and advanced by lot removal. The caller walks the owner's list nodes raw, then unregisters it (`std::list::remove` `0x00809580` on owner + 4) and frees the block with C (`0x005E5620`) | `0x00734850` → `0x00734860`; callers `0x006CD7A3`, `0x006C77E8`; freed in the lot developer's `Shutdown` `0x006C7380` |
| `cISC4LotManager::CreateLot` (23), `CreateLotIfPossible` (22), argument `out` | `cISC4Lot*&` | n/a | n/a | one reference for the caller, who releases it; the manager keeps its own | `0x0072E520` |
| `cISC4LotManager::GetLot` (5, 6), `GetRandomLot` (13), `GetOccupantLot` (14) | `cISC4Lot*` | n/a | n/a | **no AddRef**; valid until the lot is deleted | `0x0072B410`, `0x0072B3C0`, `0x0072A110` |
| message ZoningChange `0x07699455`, void2 (posted by `PlaceZone`) | `SC4CellRegion<long>*` | C | n/a | owned by the message; its destructor `0x004DCC30` frees it with C; receivers must neither free nor keep it | `0x004DD780`, `0x004DCC30` |
| `cISC4TrafficSimulator::GetTravelStrategyPercentages` (26) | `std::vector<float>*` (the SDK passes an `SC4Vector` by value and returns `bool`) | B | **no** | resized to 3 with B, then filled; no return value. Pop 8 | `0x00718BB0` → `0x007169C0` |
| `cISC4PropManager::GetLotProps` (24) | `ilist<cISC4Occupant*>` | A | yes | appends through occupant manager slot 40 (one reference each), then erases (release, free node with A) every element outside the lot's world rectangle, over the **whole** list, including elements present before the call | `0x004A3240`; caller `0x006C9780` |
| `cISC4LotConfigurationManager::GetObjectIDArray` (27) | `std::vector<uint32_t>` | B | **no** | clears first (end = begin), then appends the object's IDs with families expanded in hash order | `0x0048BA10`; caller `0x006C6D70` |

The list in slots 25 and 28–30 holds `cISC4NeighborDeal` references, not `cISC4NeighborConnection`.

### (2) The plugin fills the container (it implements the method), the game frees it

The game's callers destroy the container themselves, with the allocator of the declared type: B for
`std::vector`, `std::list` and `hash_set` with the default allocator, A for `ilist<>`. Several B callers pass a
container that already owns a B buffer, so the implementation also frees or regrows memory it did not allocate.

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence (game callers; original implementation) |
|---|---|---|---|---|---|
| `cISC4BudgetSimulator::GetAllGroups` (29) | vector of `BudgetGroupInfo`, **0x28-byte** elements: id at +0, `cRZString` name at +4 (0x14 bytes), two `int64_t` at +0x18 and +0x20 (same as the SDK struct) | B for the buffer **and** for each name's character buffer | **no**, for both the vector and the `SC4String` | the callers pass an empty vector. They destroy each name inline: they restore the `cRZString` vtable and free its character buffer with `0x0090CF63`, with no virtual call. Then they free the buffer with `0x0090CF63` | callers `0x00767270` (budget window), `0x007876B0`, `0x007898A0` (sub-budget window); original `0x00694990` |
| `cISC4BudgetSimulator::GetDepartmentBudgetsInGroup` (31) | vector of `cISC4DepartmentBudget*` (4-byte) | B | **no** | the original clears first (end = begin, keeps capacity) and takes one reference each | callers `0x007876B0`, `0x007898A0`; original `0x00691D00` |
| `cISC4BudgetSimulator::GetAllLoans` (46) | vector of `LoanInfo`, **0x20-byte** elements (same as the SDK struct) | B | **no** | the budget window passes a vector it keeps as a member and reuses: it arrives emptied (end = begin) but still owning a B buffer. The window destructor frees it with B | caller `0x00765C30`; freed in `0x00764C70`; original `0x00692C70` |
| `cISC4BudgetSimulator::GetBudgetItemInfo` (50) | vector of `BudgetItem`, **0x20-byte** elements (same as the SDK struct) | B | **no** | the callers pass an empty vector | callers `0x0069B910`, `0x006AB1E0`, `0x006ABB50`; original `0x006937B0` |
| `cISC4DepartmentBudget::GetAllLineItems` (21) | vector of `cISC4LineItem*` (4-byte) | B | **no** | the sub-budget window reuses one vector for every department: from the second call on it arrives emptied but owning a B buffer. The original clears first, reserves with B (which can free the old buffer with `0x0090CF63`) and takes one reference each | callers `0x007876B0`, `0x007898A0`; original `0x006AAD50` |
| `cISC4DepartmentBudget::GetLocallyFundedItemsByPurpose` (32) | list of property holders | B (nodes and head) | **no** | the caller allocates the head node with `0x0090CF54`. Afterwards it releases each element and frees each node, and then the head, with `0x0090CF63`. The original does not clear; it appends with one reference each | callers `0x00785740`, `0x00786690`; original `0x006AA9C0` |
| `cISC4Simulator::EnumerateAgentsByName` (24) | vector of `cRZString` (0x14-byte elements) | B for the buffer and for each string's character buffer | **no** | no game caller found (see *Unresolved*). The original clears first (destroys the strings, freeing their character buffers with B, and keeps capacity) and appends with B | original `0x00704450`; clear `0x004863F0`; append `0x0044EDA0` |
| `cISC4PowerSimulator::GetPowerPlants` (25) | `ilist<cISC4Occupant*>` | A | yes | the original does not clear; it appends one node per plant (A) with one reference each. The caller found passes a fresh, empty list (head allocated with A), then calls `ReleaseOccupantList` (occupant manager slot 48: one Release per element, nodes freed with A) and frees the head with A | caller `cSC4WinMapView::UpdateHighlights` `0x007A1A00` (head `0x007A1A5C`, call `0x007A1F50`, slot 48 `0x007A1FDF`, head freed `0x007A2044`); original `0x006ED120` |
| `cISC4PlumbingSimulator::GetWaterProducers` (28) | `ilist<cISC4Occupant*>` | A | yes | as `GetPowerPlants`: appends, one reference each | caller `0x007A1F81` (same function, same clean-up); original `0x006DC7D0` |
| `cISC4FireProtectionSimulator::GetStrikingStations` (17), `cISC4PoliceSimulator::GetStrikingStations` (16) | `ilist<cISC4Occupant*>` | A | yes | the originals do not clear; they append (A) with one reference each and return the number appended in EAX. The police version returns 0 without touching the list when no strike is on | originals `0x006AE170`, `0x006E1A00`; no game caller found (see *Unresolved*) |
| `cISC4FireProtectionSimulator::GetFireStations` (22), `cISC4PoliceSimulator::GetPoliceStations` (21) and `GetJails` (32) | `ilist<cISC4Occupant*>` | A | yes | the original first empties the caller's list with `0x00413E60`, which frees the nodes with A **without releasing the elements**, then appends a copy of its own list (A), one reference each, and returns the count in EAX. Callers test EAX ≠ 0 before walking the list, and the walk is where they release, so the count must be non-zero whenever something was appended. Every caller found passes a fresh, empty list | callers `0x007A1A8E` / `0x007A1AF0` (map highlights; slot 48 afterwards), `0x00416189` / `0x004164B1` (safety advisor; slot 48 afterwards), `0x007D21C0` / `0x007D22F8` (coverage-radius helper `0x007D20E0`: releases each element itself, then `0x00413E60`, then frees the head with A); originals `0x006E0180` (shared by fire 22 and police 21), `0x006E01A0` (police 32; `interfaces.tsv` calls it `IsJail`, but it is `GetJails`) → `0x006DFC70` |
| `cISC4PoliceSimulator::GetJailbreakBuildings` (42) | `ilist<cISC4Occupant*>` | A | yes | as slot 21, except that when the simulator's own jailbreak list is empty it returns 0 and leaves the caller's list untouched (it is not emptied) | original `0x006E08A0`; no game caller found |
| `cISC4DispatchSource::GetStations` (6; IID `0xC9C00435`; fire and police simulators and both air sources) | `ilist<cISC4Occupant*>` | A | yes | empties, then copies, as slot 21, but returns a **bool** (AL = count ≠ 0). The dispatch manager passes a fresh, empty list and, when the result is true, ends with `ReleaseOccupantList` (slot 48) | callers `0x0046AF10` (`GetAvailablePylonCount`), `0x0046B060` (`BeginDispatchTool`); originals `0x006F6A50` (protection simulators), `0x006E0010` (air sources) |
| `cISC4FloraSimulator::GetAllFloraTypes` (8, the vector overload), `GetAllFloraFamilyTypes` (10) | `std::vector<uint32_t>` | B | **no** | the original clears first (end = begin, keeps the buffer) and appends with B growth (`0x0051CA60`: allocates with `0x0090CF54`, frees the old buffer with `0x0090CF63`). The lot editor passes a zeroed vector (no buffer) and afterwards frees the buffer with `0x0090CF63` | caller `0x00482DC0` (vector zeroed at `0x00482E57`, call `0x00482E6A`, freed `0x004832B2`); originals `0x006B0E90`, `0x006B0F10` |
| `cISC4FloraSimulator::GetFloraTypesByFilter` (13), `GetFloraTypesByFamily` (14) | STLport `hash_set<uint32_t>` (see *hash_set* below) | B (nodes and bucket vector) | **no**, for both the allocator and the layout | the originals do not clear; they insert (a union). Slot 13's filter may be null. The callers build a fresh set with 193 buckets and destroy it with `0x008EBA60` (frees each node with B) and `0x0090CF63` (bucket vector) | callers `0x006CD010` (`cSC4LotDeveloper::PickFloraTypeForLot`), `0x0048B860` (`cSC4LotConfigurationManager::GetMaxFootprintForFloraFamilies`); originals `0x006B23A0`, `0x006B2460` (insert `0x00624DD0`, resize `0x004F0940`) |
| `cISC4TractDeveloper::Grow` (5), called by the building development simulator's daily `Grow` | list of outcomes (contents never read by the caller) | B (nodes and head) | **no** | the caller allocates the head node with `0x0090CF54`, the tract developer appends, then the caller frees every node and the head with `0x0090CF63`. The original appends int32 outcome codes and never clears | caller `0x0069B0B0`; original appends at `0x0070D557`, `0x0070D65A`, `0x0070D727` |
| `cISC4ZoneDeveloper::DoParcellization` (6), argument 1 | `const SC4CellRegion<long>&` | C | n/a | the zone manager passes the region it then attaches to ZoningChange (freed by the message with C); an implementation must neither free nor keep it | caller `0x004DD780`; original `0x00733FA0` |
| `cISC4ZoneManager::PlaceZone` (11): the ZoningChange region it posts | `SC4CellRegion<long>*` | C | n/a | a replacement zone manager must allocate the region with C in the zones.md §11.4 layout (the game's message destructor frees it), or post its own message class | consumers: growth developers `0x006B4B90`; original `0x004DD780` |

A compatible implementation of the B methods must allocate the list nodes, vector buffers and hash nodes and
buckets with `0x0090CF54`. It must free any buffer it replaces, including one it was handed, with `0x0090CF63`.
Every string inside an element must have a B character buffer (see *Strings*). `SC4Vector`, `SC4List` and
`SC4HashSet` cannot be used for these containers. The `ilist<>` methods (A) can use `SC4List`: its nodes come
from the same service.

The game keeps several cached copies of the framework service pointer (one per `ilist<>` instantiation, for
example `0x00B207E4` and `0x00B207DC`). They all point to the same service object, so they are one instance (A).

### (3) Internal containers returned by pointer or reference

These methods return the implementing object's own container. The caller reads it in place and never frees or
modifies it *[inferred: no modifying caller found]*. An implementation in a plugin must therefore hand out a
container whose layout the game can walk (STLport/MSVC as noted) and must not expect the game to free it; the
allocator matters only if the plugin's own code later grows or frees that container while the game holds no
iterator into it.

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence |
|---|---|---|---|---|---|
| `cISC4GrowthDeveloper::GetGrowthList` (11), `GetDeclineList` (12) | vector of 12-byte `GrowthItem {int32 x, int32 z, int32 maxStage}`, by reference | B | **no** | read by the building development simulator (begin/end in place) | original `0x00798040`, `0x006B7540`; push `0x006B56F0` → `0x00626240` → `0x0090CF54`; reader `0x00699440` |
| `cISC4GrowthDeveloper::GetStylePercentages` (26) | `std::map<uint32_t, SC4Percentage>*` (the SDK says a map of `uint32_t`) | B (nodes) | **no** | read by the tract developer *[inferred]* | `0x006B7590`; destructor `0x004C56F0` |
| `cISC4TractDeveloper::GetActiveStyles` (9) / `SetActiveStyles` (10) | `std::vector<uint32_t>`: Get returns the member by reference; Set takes the caller's vector and assigns it into the member | B | **no** | Set's vector assignment (`0x00462C10`) may free and reallocate the member's buffer with B | `0x0070C840`, `0x00707470` |
| `cISC4TrafficSimulator::GetFerryRouteBetweenTiles` (29) | pointer to a terminal link's or neighbour link's `vector<uint8_t>` (the SDK says `int32`) | B | **no** | owned by the simulator; the caller must neither free it nor keep it past the next rebuild | `0x00713B70` |
| `cISC4TrafficSimulator::GetDesiredLotInsertionPoint` (22) | pointer to the lot manager's safe lot iterator (from lot manager slot 15; the SDK says `uint32`) | n/a | n/a | handed back to the lot manager by the caller | `0x00720010` |
| `cISC4ZoneManager::GetZoneGrid` (5) | the `int8` zone grid | n/a | n/a | game grid object, no AddRef (simgrid §6.4) | `0x00698CF0` |
| `cISC4LotManager::GetMaximumCapacityGrid` (27) | the `uint16` grid (class `0x49B9E604`; the SDK says `short`) | n/a | n/a | game grid object, no AddRef (simgrid §6.4) | `0x00809240`; constructor `0x0045F9B0` |
| `cISC4TrafficSource::GetTripDataList` (3) | the source's `std::list<cSC4TripData>` (0x34-byte nodes), by reference (the SDK says `SC4List`) | B | **no** | owned by the lot or neighbour connection; the traffic simulator appends, erases and inserts split copies in it with B, and each trip's route and return-route `vector<uint8_t>` are B buffers filled by `GetCompressedRoute`. A replacement traffic source must hand out a list with this layout and allocator | `0x0080BC20`; `0x006C27E0`, `0x006D4E6B`, `0x0071D440`, `0x0071C6C0` |
| `cISC4TrafficSimulator::GetTrafficEdgeDensity` (12) | pointer into the simulator's edge-density map, or to a shared zero record | n/a | n/a | borrowed; the caller owns nothing | `0x00711460`; zero record `0x00AB31D0` |
| `cISC4BuildingDevelopmentSimulator::GetDevelopers` (34) | `std::map<uint32_t, cISC4GrowthDeveloper*>*` | B (nodes) | **no** | no AddRef; iterated by the demand simulator | `0x00698CD0` |
| `cISC4BuildingDevelopmentSimulator::GetSpecialBuildings` (44) | `ilist<cISC4Occupant*>*` (this + 0x4C) | A | yes | each entry holds one reference owned by the simulator. The growth developer copies the elements into its own list (one AddRef each) | `0x00520080`; filled by `Read` `0x0069ACB0` (`ilist` ctor `0x00411870`); copied at `0x006B6569`–`0x006B65C1` |
| `cISC4BuildingDevelopmentSimulator::GetExemplarName` (37), `GetBuildingName` (38) | `cRZString` inside a static hash map | B (character buffer) | n/a | an unknown type inserts an empty string; the caller must not release or free it | `0x0069AB20`, `0x0069AB00` |
| `cISC4BuildingDevelopmentSimulator::GetBuildingProfile` (39), `GetBuildingProfilePtr` (40) | profile struct holding a vector of `map<uint32_t, uint16_t>` | B | **no** | read only; slot 39 inserts a zero profile for an unknown type | `0x0069CA60`, `0x006985C0` |
| `cISC4PowerSimulator::GetMonthlyConsumptionMapPtr` (18) | `std::vector<std::vector<int16_t>>*`, through an out parameter (`vector<…>*&`); outer elements are 12-byte vectors `{begin, end, capacity_end}` of `int16_t` | B (outer buffer and every row) | **no** | the original stores a pointer to its own member (`this + 0x84`) and allocates nothing; a caller would read in place. **No game caller found** (see *Unresolved*) | original `0x006EAD40` |

## Dispatch ID array

`cISC4DispatchManager::GetDispatchIDArray(type, uint32_t* ids, uint32_t& n)` (slot 12, `0x00469DA0`, pop 12):

- Nothing is allocated; the caller owns the array.
- `n` is **read only**: it is the capacity. The method never writes it.
- It writes the ID of every active dispatch of that type to `ids[0], ids[1], …` and returns the number written in
  EAX.
- It stops when the number written equals `n`. The comparison comes after each write, so **`n = 0` is not "no
  room"**: it means "no limit", and every matching ID is written.
- To size the array, call `GetDispatchCount(type)` (slot 11, `0x00469D80`, pop 4, EAX = count) first, and pass a
  non-zero `n`.

## hash_set

The game's `hash_set<uint32_t>` is STLport's, declared with the default allocator: `hash_set<uint32_t,
hash<uint32_t>, equal_to<uint32_t>, allocator<uint32_t>>`. Users: flora slots 13 and 14, building development
slots 17 and 19.

The object is 0x14 bytes:

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | the hasher, key-equal and key-extractor objects (empty, padded to 4 bytes; the code never reads or writes them) |
| +0x04 | 4 | bucket vector `begin` (`Node**`) |
| +0x08 | 4 | bucket vector `end`; the bucket count is n = (end − begin) / 4 |
| +0x0C | 4 | bucket vector `capacity_end` |
| +0x10 | 4 | element count |

- **Node**: 8 bytes, `{Node* next; uint32_t value}`, allocated with B (`0x0090CF54(8)`, insert `0x00624DD0`) and
  freed with B.
- **Buckets**: the bucket vector's buffer is a B block (`0x0051C950`). An empty bucket holds null; each bucket
  is a singly linked chain ending in null.
- **Hash**: the identity. A value v belongs to bucket v mod n (unsigned division).
- **Bucket counts**: the game only creates counts from STLport's 28-entry prime table at `0x00A829B0`: 53, 97,
  193, 389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317, 196613, 393241, 786433, 1572869, 3145739, 6291469,
  12582917, 25165843, 50331653, 100663319, 201326611, 402653189, 805306457, 1610612741, 3221225473, 4294967291.
  A request for h buckets gives the first prime ≥ h (4294967291 above the table). Construction with a hint
  (`0x00829BC0`) does this; the flora callers pass 100 and get 193 buckets.
- **Growth**: before each insert the game calls resize(count + 1) (`0x004F0940`). When count + 1 exceeds n, it
  allocates a new, zeroed bucket vector with B for the first prime ≥ count + 1, moves every existing node (the
  nodes themselves are kept) to the chain of v mod (new n), and frees the old bucket buffer with B.
- **Insert**: a value already present is not inserted again. A new node goes to the **head** of its bucket chain,
  and the count goes up by one.
- **Destroy** (the callers): `0x008EBA60` frees each node with B, nulls the buckets and zeroes the count; then the
  bucket buffer is freed with B.

An implementation that inserts into the game's set must leave a state the game's own lookup, iteration, insert,
clear and destroy accept:

1. every node holding v is in the chain of bucket v mod n, where n = (end − begin) / 4;
2. no value appears twice;
3. the count equals the number of nodes;
4. every node is an 8-byte B block, and the bucket buffer is a B block;
5. n ≥ 1 (the game divides by n); `capacity_end` ≥ `end` and lies within the bucket block;
6. if it grows the bucket vector, it frees the old buffer with B and relinks the existing nodes; keeping n a
   prime from the table and count ≤ n gives a state the game itself can produce.

Iteration order (bucket 0 to n − 1, then chain order) is observable; head insertion and the table's primes
reproduce the game's order.

**The SDK's `SC4HashSet` is not compatible**: it uses A, and its members start at +0 (`mpStart` at +0, 0x10
bytes), while the game's object has a 4-byte member first and the bucket vector at +4. The header's own comments
give the game's offsets (+0x04 to +0x10), but the struct lacks the leading member.

## Strings

`cRZString` is 0x14 bytes: `{vtable, begin, end, capacity_end, reference count}`. This is the layout of the SDK's
`SC4String` too. Its vtable is at `0x00A80810`; identical copies of the vtable are at `0x00A812F8`, `0x00A85478` and
`0x00A86258`.

- **The character buffer is allocated and freed with B** (`0x0090CF54` / `0x0090CF63`). Even an empty string owns
  a buffer: the default constructor allocates 8 bytes. Evidence: default constructor `0x00578390`; constructor from
  `char*` `0x00408480` (→ `0x00407FA0`); assign/grow `0x0040EBC0`; deleting destructor (vtable slot 28)
  `0x0044BF10`.
- **Buffer pointers** (the STLport string layout):
  - `begin` is the first character, and `end` points at the terminating zero byte, so the length is `end − begin`.
  - `capacity_end` is `begin` plus the number of bytes allocated, so it points one past the last usable byte,
    including the terminator's slot. The usable capacity is therefore `capacity_end − begin − 1`.
  - A string built from n characters allocates exactly n + 1 bytes and sets `capacity_end = begin + n + 1`
    (constructor from a range, `0x00407FA0`).
  - The default constructor allocates 8 bytes and sets `capacity_end = begin + 8` (`0x00578390`).
  - Over-allocating is harmless, provided `capacity_end` does not point past the allocation.
- **Reference count** (+0x10): the game's constructors set it to **0** (`0x00578390`, and the `char*` constructor).
  A `cRZString` embedded in a container element is never reference-counted. It is destroyed inline (see below),
  so 0 is the value to store.
- **A heap-allocated `cRZString` object** is freed with C (`0x005E5620`) by the deleting destructor. This happens
  when `Release` drops the last reference (`0x0090E1F3` → slot 28).
- **The game often destroys strings inline**, without calling through the vtable: it stores `0x00A80810` in the
  vtable slot and frees `begin` with `0x0090CF63`. It does this for strings inside vectors that it destroys (for
  example the group-name loop in `0x00767270`). A string object's own vtable therefore does not protect its buffer.

The SDK's `SC4String` allocates and frees its character buffer with **A**
(`engine/third_party/gzcom-dll/gzcom-dll/src/SC4String.cpp`, `allocate`/`deallocate`). It is therefore **not
compatible** whenever one side allocated a string's buffer and the other side frees or regrows it:
- an `SC4String` in an element the game destroys (`BudgetGroupInfo::groupName` in `GetAllGroups`, the elements of
  `EnumerateAgentsByName`) has its A buffer freed with B;
- a game `cRZString` (a B buffer) that SDK code frees, reallocates or assigns into has its B buffer freed with A.

A string that only one side ever creates, grows and destroys is safe with either type. Each side may read the
other side's strings.

### Strings passed into the game

- `cIGZCheatCodeManager::RegisterCheatCode(uint32_t id, const cIGZString& name)` (slot 3, `0x00921B38`, pop 8)
  only reads the string: it compares it against the registered names through the string's own vtable (slot 10,
  `+0x28`, two arguments), then copies it into a new `cRZString` (`0x00578390`, then that string's slot 13,
  `+0x34`) and stores the copy. It never AddRefs, Releases or frees the argument. The caller keeps ownership and
  may destroy the string after the call. The argument must be a complete `cIGZString`, since its virtual methods
  are called. Returns AL = 1, or 0 when the name is already registered.
- `cIGZCommandServer::RegisterCommand` (slot 6, `0x0092B075`, pop 12) takes **`(uint32_t id, const char* name,
  const char* parameterInfo)`**, not a `cIGZString`. It copies `name` into a `cRZString` of its own (`0x00408480`,
  B buffer) and passes `parameterInfo` to slot 8 (`RegisterCommandParameterInfo`, `0x0092AD10`, pop 8), which also
  copies it (`0x00408480`). Neither pointer is kept or freed; both may be temporary. Returns AL = 0 for id 0, a
  null name or an id already registered, else 1.

## GetLotsInCellRect

With the last argument `false`, the method:
1. first clears the list it is given: it **releases every element once** and frees every node with the STL node
   allocator (`0x0090CF63`), keeping the head node (`0x00473EC0`). Never pass a list holding pointers the caller
   has not referenced;
2. then appends one node per lot (lots spanning several cells appear once), allocated with `0x0090CF54`;
3. takes one reference on each lot;
4. returns the number of lots appended.

The caller owns the nodes and the references. The caller must therefore:
- release each lot once;
- free each node with `0x0090CF63`;
- free its own head node with whichever allocator allocated it.

Never pass a list that already contains nodes from another allocator: step 1 frees them with the STL node
allocator.

The other lot-manager queries (slots 8–10) do not clear the list: they only append nodes from the STL node
allocator. Their caller has the same obligations.

## Unresolved

- `cISC4Simulator::EnumerateAgentsByName` (slot 24): no game caller was found. Neither the Windows code (call
  sites through slot 24 that use a 0x14-byte-element vector) nor the Mac pseudocode (calls through the simulator
  pointer at slot 24, or functions that use a `cRZString` vector) has one. The classification (B) comes from the
  original implementation and its declared type `std::vector<cRZString>`. Callers built on the SDK pass an
  `SC4Vector<SC4String>` (A), which was already incompatible with the original game.
- `cISC4PowerSimulator::GetMonthlyConsumptionMapPtr` (slot 18): no game caller was found. On Windows there is no
  call through the power simulator global (`0x00B43DA0`, set by `SL::OnCityInit` `0x00601F54`) at `+0x48`. In the
  Mac pseudocode no user of `vector<vector<short>>` outside the power simulator and `cSC4SimGrid<short>` calls the
  power simulator at that slot. A replacement need only return a pointer to a valid object of that type that it
  owns.
- No game caller was found for fire slot 17, police slots 16, 32 and 42, and flora slots 8 and 13, either through
  the simulator globals on Windows or in the Mac pseudocode. Their rows rest on the declared types (`ilist<>`,
  `std::vector`, `hash_set` with the default allocator) and on the original implementations, which use the same
  allocators as their siblings with known callers.
- The callers in table (2) are the ones found by scanning for calls through the budget simulator pointer and in
  the budget windows. Other callers may exist, but the declared types (`std::vector`/`std::list` with the default
  allocator in every signature) mean they would use B too.
