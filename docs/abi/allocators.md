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
| `cISC4LotManager::GetLotsInCellRect` (7) | list of lots | B | **no**: free the nodes with `0x0090CF63` | clears the list first (B, keeps the head node); one reference each. See below | `0x0072E080` |
| `cISC4LotManager::GetLotsSurroundingCell` (8, the `(x, z, list, flags)` overload) | list of lots | B | **no** | does not clear; appends; one reference per appended lot. Reaches slot 10 through slots 6 and 9 via the vtable | `0x0072A230` |
| `cISC4LotManager::GetLotsSurroundingLot` (9, the `(lot, list, flags)` overload) | list of lots | B | **no** | does not clear; appends; one reference each. Reaches slot 10 through the vtable | `0x0072A750` |
| `cISC4LotManager::GetLotsSurroundingRect` (10) | list of lots | B | **no** | does not clear; appends; one reference each | `0x0072E2B0` |
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

The list in slots 25 and 28–30 holds `cISC4NeighborDeal` references, not `cISC4NeighborConnection`.

### (2) The plugin fills the container (it implements the method), the game frees it

The game's callers declare these with the default allocator. They destroy the container themselves, freeing the
buffer or nodes with B. Several pass a container that already owns a B buffer, so the implementation also frees
or regrows memory it did not allocate.

| Method (slot) | Container | Game allocator | SDK container compatible? | Notes | Evidence (game callers; original implementation) |
|---|---|---|---|---|---|
| `cISC4BudgetSimulator::GetAllGroups` (29) | vector of `BudgetGroupInfo`, **0x28-byte** elements: id at +0, `cRZString` name at +4 (0x14 bytes), two `int64_t` at +0x18 and +0x20 (same as the SDK struct) | B for the buffer **and** for each name's character buffer | **no**, for both the vector and the `SC4String` | the callers pass an empty vector. They destroy each name inline: they restore the `cRZString` vtable and free its character buffer with `0x0090CF63`, with no virtual call. Then they free the buffer with `0x0090CF63` | callers `0x00767270` (budget window), `0x007876B0`, `0x007898A0` (sub-budget window); original `0x00694990` |
| `cISC4BudgetSimulator::GetDepartmentBudgetsInGroup` (31) | vector of `cISC4DepartmentBudget*` (4-byte) | B | **no** | the original clears first (end = begin, keeps capacity) and takes one reference each | callers `0x007876B0`, `0x007898A0`; original `0x00691D00` |
| `cISC4BudgetSimulator::GetAllLoans` (46) | vector of `LoanInfo`, **0x20-byte** elements (same as the SDK struct) | B | **no** | the budget window passes a vector it keeps as a member and reuses: it arrives emptied (end = begin) but still owning a B buffer. The window destructor frees it with B | caller `0x00765C30`; freed in `0x00764C70`; original `0x00692C70` |
| `cISC4BudgetSimulator::GetBudgetItemInfo` (50) | vector of `BudgetItem`, **0x20-byte** elements (same as the SDK struct) | B | **no** | the callers pass an empty vector | callers `0x0069B910`, `0x006AB1E0`, `0x006ABB50`; original `0x006937B0` |
| `cISC4DepartmentBudget::GetAllLineItems` (21) | vector of `cISC4LineItem*` (4-byte) | B | **no** | the sub-budget window reuses one vector for every department: from the second call on it arrives emptied but owning a B buffer. The original clears first, reserves with B (which can free the old buffer with `0x0090CF63`) and takes one reference each | callers `0x007876B0`, `0x007898A0`; original `0x006AAD50` |
| `cISC4DepartmentBudget::GetLocallyFundedItemsByPurpose` (32) | list of property holders | B (nodes and head) | **no** | the caller allocates the head node with `0x0090CF54`. Afterwards it releases each element and frees each node, and then the head, with `0x0090CF63`. The original does not clear; it appends with one reference each | callers `0x00785740`, `0x00786690`; original `0x006AA9C0` |
| `cISC4Simulator::EnumerateAgentsByName` (24) | vector of `cRZString` (0x14-byte elements) | B for the buffer and for each string's character buffer | **no** | no game caller found (see *Unresolved*). The original clears first (destroys the strings, freeing their character buffers with B, and keeps capacity) and appends with B | original `0x00704450`; clear `0x004863F0`; append `0x0044EDA0` |
| `cISC4TractDeveloper::Grow` (5), called by the building development simulator's daily `Grow` | list of outcomes (contents never read by the caller) | B (nodes and head) | **no** | the caller allocates the head node with `0x0090CF54`, the tract developer appends, then the caller frees every node and the head with `0x0090CF63` | caller `0x0069B0B0` |

A compatible implementation of these methods must allocate the list nodes and vector buffers with `0x0090CF54`.
It must free any buffer it replaces, including one it was handed, with `0x0090CF63`. Every string inside an
element must have a B character buffer (see *Strings*). `SC4Vector` and `SC4List` cannot be used for these
containers.

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
| `cISC4BuildingDevelopmentSimulator::GetDevelopers` (34) | `std::map<uint32_t, cISC4GrowthDeveloper*>*` | B (nodes) | **no** | no AddRef; iterated by the demand simulator | `0x00698CD0` |
| `cISC4BuildingDevelopmentSimulator::GetSpecialBuildings` (44) | `ilist<cISC4Occupant*>*` (this + 0x4C) | A | yes | each entry holds one reference owned by the simulator. The growth developer copies the elements into its own list (one AddRef each) | `0x00520080`; filled by `Read` `0x0069ACB0` (`ilist` ctor `0x00411870`); copied at `0x006B6569`–`0x006B65C1` |
| `cISC4BuildingDevelopmentSimulator::GetExemplarName` (37), `GetBuildingName` (38) | `cRZString` inside a static hash map | B (character buffer) | n/a | an unknown type inserts an empty string; the caller must not release or free it | `0x0069AB20`, `0x0069AB00` |
| `cISC4BuildingDevelopmentSimulator::GetBuildingProfile` (39), `GetBuildingProfilePtr` (40) | profile struct holding a vector of `map<uint32_t, uint16_t>` | B | **no** | read only; slot 39 inserts a zero profile for an unknown type | `0x0069CA60`, `0x006985C0` |

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

## GetLotsInCellRect

With the last argument `false`, the method:
1. first clears the list it is given, freeing every node with the STL node allocator (`0x0090CF63`) but keeping
   the head node;
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
- The callers in table (2) are the ones found by scanning for calls through the budget simulator pointer and in
  the budget windows. Other callers may exist, but the declared types (`std::vector`/`std::list` with the default
  allocator in every signature) mean they would use B too.
