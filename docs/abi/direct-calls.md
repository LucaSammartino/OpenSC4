# Non-virtual game functions called by address

Interface facts (see [README.md](README.md)). Some original behaviour can only be reached through non-virtual
functions of the executable, which no interface exposes. A replacement that needs it calls the function at its
address, behind a check that the executable is SimCity 4 Deluxe 1.1.641 (GOG). This file gives what such a call
needs: the calling convention, the arguments, the bytes the callee pops, the return channel and exactly which
`this` pointer the function expects. No logic is recorded here.

Unless a section says otherwise (the lot pool allocator, the retaining-wall helpers and the confirmation dialog are
`__cdecl`), these functions are `__thiscall`: `this` in ECX, the arguments on the stack, popped by the callee. A
`bool` argument is passed as a full 4-byte stack slot; only its low byte is read.

## Finding the `this` pointer

The game's objects inherit several interfaces, so one object has several sub-object pointers. The functions below
expect the object **base**, which is not the `cISC4Occupant*` that occupant-manager queries return. Two ways to get
the base:
- QueryInterface the occupant for an IID that the class answers with its base (listed per function). This also
  checks that the object really is that original class, which the direct call assumes.
- Or subtract the fixed offset of the `cISC4Occupant` sub-object, but only after such a check.

## cSC4PowerPoleOccupant::RemoveConnection `0x0064DA00`

`void RemoveConnection(cSC4PowerPoleOccupant* other, bool flag)`; the call the power spec needs is
`firstPole.RemoveConnection(secondPole, false)`.

| | |
|---|---|
| Convention | `__thiscall` |
| `this` | the **base** of the first pole object |
| Arguments | `other`: the **base** of the second pole object (same class); `flag`: `bool` in a 4-byte slot (pass 0) |
| Pops | 8 |
| Return | none (EAX is garbage) |
| Null `other` | returns at once |

Pole object layout (constructor `0x0064DB30`; the copy at `0x0064DFC0` sets the same table):

| Offset | Sub-object | vtable |
|---|---|---|
| +0x00 | `cISC4PowerPoleOccupant` (primary: QueryInterface `0x0064A030` in slot 0) | `0x00AA9EB0` |
| +0x04 | `cIGZMessageTarget2` (4 slots) | `0x00AA9EA0` |
| +0x08 | `cISC4Occupant` (the `cSC4Occupant` base; its QueryInterface thunk `0x0064DF00` subtracts 8) | `0x00AA9E18` |
| +0x0C | `cIGZSerializable` | `0x00AA9DFC` |
| +0x10 | property holder | `0x00AA9D98` |

So **base = `cISC4Occupant*` − 8**. QueryInterface (`0x0064A030`) returns the base for IIDs `0xA8C86142`
(`cISC4PowerPoleOccupant`), `0x09C05C6A` (the class ID, `GetGZCLSID` `0x0064DD50`) and `0xA9764C8C`, and
base + 4 for `0x452294AA`. Any other IID is passed to the `cSC4Occupant` base at +8.

The function calls `other`'s vtable slot 8 (`+0x20`, two out-pointers, pop 8), which is the primary vtable's
slot. That is why `other` must also be a base pointer and not a `cISC4Occupant*`.

## cSC4PipeOccupant::SetWateredState `0x00648540`

`void SetWateredState(bool watered)`

| | |
|---|---|
| Convention | `__thiscall` |
| `this` | the **base** of the pipe object |
| Arguments | `watered`: `bool` in a 4-byte slot |
| Pops | 4 |
| Return | none. When the state changes, the function ends with a tail call to the object's own vtable slot 100 (`+0x190`, one argument, pop 4), whose EAX it leaves behind; treat it as void |

Pipe object layout (constructor `0x006488F0`):

| Offset | Sub-object | vtable |
|---|---|---|
| +0x00 | primary: the network occupant (`cSC4NetworkOccupant`, at least 101 slots; slot 59 is `GetOccupiedCell`) | `0x00AA97B8` |
| +0x04 | `cISC4Occupant` (the `cSC4Occupant` base; its QueryInterface thunk `0x006485C0` subtracts 4) | `0x00AA9730` |
| +0x08 | `cIGZSerializable` (thunk subtracts 8) | `0x00AA9714` |
| +0x0C | property holder | `0x00AA96B0` |

So **base = `cISC4Occupant*` − 4**. The pipe's QueryInterface (`0x006484D0`) returns the base for `0x49C05B9F`
(the pipe's class ID, `GetGZCLSID` `0x00648F40`) and `0xA9764C8C`. The network-occupant QueryInterface it falls
back to (`0x0065A080`) returns the base for `0x4A10AFC2` and `0xA821EF94` (`cISC4NetworkOccupant`). Only the pipe answers `0x49C05B9F`, so a successful
QueryInterface for it both identifies a pipe and yields the right pointer. The plumbing simulator calls the
function with the pointer it keeps in its pipe list and uses for network-occupant slot 59 (`0x006DE677` →
`0x006DE6AD`), which is that base.

## Air dispatch sources: fire plane and police helicopter

| Class | CLSID | Factory | Dispatch type (+0x14) | vtable at +0 |
|---|---|---|---|---|
| `cSC4FirePlaneDispatchSource` | `0x4C0AE114` | `0x006ACEA0` | 5 | `0x00AB1008` |
| `cSC4PoliceHelicopterDispatchSource` | `0x0C0B06E8` | `0x006E0030` | 6 | `0x00AB22D0` |

### Creation through GZCOM

Both classes are registered with the GZCOM class registry. The game's COM director constructor
(`cSC4COMDirector`, `0x00460090`) calls its `AddCls` (`0x0090E133`) with `(0x0C0B06E8, 0x006E0030)` at
`0x00460B33`–`0x00460B40` and with `(0x4C0AE114, 0x006ACEA0)` at `0x00460B45`–`0x00460B52`. The framework's
`CreateInstance` / `GetClassObject` can therefore create them. (The Mac build registers them the same way,
`cSC4COMDirector::cSC4COMDirector` `0x00711BBE`.)

- The factory allocates 0x28 bytes from the global heap (C, `0x005E55E0`), runs the constructor and returns the
  base with a **reference count of 0**. The framework then QueryInterfaces it for the requested IID.
- **Request IID `0xC9C00435`** (`cISC4DispatchSource`). The class's QueryInterface (`0x006AC550`, shared by both
  classes) answers only `0xC9C00435` (base), `0x452294AA` (`cIGZMessageTarget2`, base + 4) and `0xE4FDA3D4`
  (`cIGZSerializable`, base + 8). It does **not** answer the generic `cIGZUnknown` IID, so a request for that
  fails.
- The original simulators do not use GZCOM: they construct the objects directly and keep them in a member
  (fire `+0xD0`, police `+0xC8`).

Object layout (constructors `0x006ACE00`, `0x006DFF70`):

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | `cISC4DispatchSource` vtable (primary) |
| +0x04 | 4 | `cIGZMessageTarget2` vtable |
| +0x08 | 4 | `cIGZSerializable` vtable |
| +0x0C | 1 | initialised flag (`bool`), followed by 3 bytes of padding |
| +0x10 | 4 | reference count (AddRef `0x004F7BA0`; Release `0x006DFAF0` calls Shutdown and then the deleting destructor, slot 12 `+0x30`, when the count is ≤ 1) |
| +0x14 | 4 | dispatch type handled (5 or 6, see above; `CanHandleDispatchType`, slot 3, compares with it) |
| +0x18 | 4 | the city, a counted reference taken by Init |
| +0x1C | 4 | `ilist<cISC4Occupant*>` of stations: a pointer to an A head node |
| +0x20 | 4 | the message server, a counted reference taken by Init |
| +0x24 | 4 | `uint32_t`, 1 after construction |

### Init `0x006DF8C0` and Shutdown `0x006AC860`

Neither function is in any vtable: they are **direct calls only**. Each is one body shared by both classes
(identical code folded), so the same address serves the fire plane and the helicopter.

| | Init `0x006DF8C0` | Shutdown `0x006AC860` |
|---|---|---|
| Convention | `__thiscall` | `__thiscall` |
| `this` | the object **base** = the `cISC4DispatchSource*` (IID `0xC9C00435`), which is also what the factory returns | same |
| Arguments | none | none |
| Pops | 0 (`ret`) | 0 (`ret`) |
| Return | `bool` in AL, always 1 | `bool` in AL, always 1 |
| Repeated calls | a second Init does nothing (flag at +0x0C) | does nothing unless initialised; clears the flag |

What Init needs from the game: it reads the city (`0x00B43CEC`), the message server (`0x00B43CCC`) and the
dispatch manager (`0x00B43D04`) from the game's globals, which are set while a city is loaded. It takes a
reference on the city and the message server, registers six message notifications for the `cIGZMessageTarget2`
sub-object (base + 4), and registers the object with the dispatch manager (`RegisterDispatchSource`, slot 13,
with the base). Shutdown undoes these: it unregisters from the city's dispatch manager (city slot 39 `+0x9C`, then
slot 14 `+0x38`), hands the station list at +0x1C to the occupant manager's `ReleaseOccupantList` (city slot 34
`+0x88`, then slot 48), removes the notifications and releases the message server and the city. Shutdown is
therefore safe before the last Release, which would call it anyway.

Callers in the original: fire `Init` `0x006AE373`, police `Init` `0x006E290D`; fire `Shutdown` `0x006AD200`,
police `Shutdown` `0x006E0334`, and the classes' `Release` `0x006DFB03`.

## STLport `map<uint32_t, 4-byte value>`: `operator[]` `0x00497A00` and subtree free `0x004A79A0`

A replacement of `cISC4DemandSimulator::GetLocalPopulationSummary` (slot 11) must empty and fill a
`std::map<uint32_t, int32_t>` that the game owns. The layout, the invariants and the original's exact contract
are in [allocators.md](allocators.md) (*map and set*, *GetLocalPopulationSummary*). These two game functions do
the work with the game's own code. Both bodies are shared by every game map of `uint32_t` to a 4-byte value
(0x18-byte nodes), and both are pure container code:
- they touch only the map in ECX, its nodes and the STL node allocator (B, `0x0090CF54` / `0x0090CF63`);
- they make no virtual calls and take or release no references;
- they read no other global.

The allocator brackets each allocation and free with its instance's virtual slots 1 and 2 (a lock and an
unlock *[inferred]*). **The map itself is not locked**: call them on the thread that called the plugin's method,
as the original does.

### operator[] `0x00497A00`

`T& map<uint32_t, T>::operator[](const uint32_t& key)`, for any 4-byte T (`int32_t` here)

| | |
|---|---|
| Convention | `__thiscall` |
| `this` | the game's 12-byte map object (the reference the game passed; for slot 11, regional city + 0x3C). Never a map built by an MSVC `std::map` or by the SDK |
| Arguments | `key`: a **pointer** to the `uint32_t` key (a reference) |
| Pops | 4 |
| Return | EAX = pointer to the value (node + 0x14). Write the `int32_t` through it |
| Key present | returns the existing node's value; nothing is allocated |
| Key absent | allocates one 0x18-byte node with B, stores (key, 0), links and rebalances it, adds 1 to the count, and returns the new value's address |
| Frees | nothing. The map owns the new node, and the game frees it later with B |

This is the call the original uses (`0x006A5FEE` and `0x006A6005` in `0x006A5F80`).

Alternative: `insert_unique` `0x004FB6C0` (`__thiscall`, stack `(result* out, const pair* v)`, pop 8; EAX = `out`;
`out+0` = node, byte `out+4` = inserted). It does not overwrite an existing key's value.

### Subtree free `0x004A79A0`

`void _M_erase(node* top)`: frees `top` and every node below it with B.

| | |
|---|---|
| Convention | `__thiscall` |
| `this` | any value: it is only passed on to the recursive calls. Pass the map object |
| Arguments | `top`: the root, header + 4 (`header.parent`); null is allowed and does nothing |
| Pops | 4 |
| Return | none (EAX is garbage) |
| Does **not** | touch the header or the count. Values are freed as raw bytes, with no destructor |

To empty a map the way the original does (`0x006A5F95`–`0x006A5FB7`), do these steps when the count (map + 4) is
not 0:
1. call `0x004A79A0` with the header's `parent`;
2. set header.left (+8) := header;
3. set header.parent (+4) := null;
4. set header.right (+0xC) := header;
5. set the count := 0.

Never free the header: the game keeps using this map object.

**Judgement: safe for a plugin**, behind the usual executable check:
- both functions keep the map in the state the game's own code leaves;
- they allocate only from B, the allocator that the game's own clear of this map frees with (the original's
  next call, `0x006A5FA0` → `0x004A79A0` → `0x0090CF63`);
- they have no side effects outside the map.

They require a live, consistent game map in ECX: header non-null, count equal to the number of nodes.
`0x00497A00` must not be used on a map whose value is not 4 bytes: the `uint16_t` maps have their own `0x00699030`.

## The lot pool and the lot constructor: `0x006BF430` and `0x006C1E50`

The zone developer's `CreateLot` is the only maker of new lots (docs/specs/zones.md §8.5, §8.5.1, §13.1). A
replacement lot manager that creates the game's own lots must take their memory from the lot pool and construct
them with the game's constructor, because the lot's deleting destructor (reached from its last `Release`) gives
the block back to that pool by address.

### Allocate `0x006BF430`

| | |
|---|---|
| Convention | `__cdecl` (like the retaining-wall helpers and the confirmation dialog below) |
| `this` | none |
| Arguments | one 4-byte slot: the size; the original pushes `0x8C` and the callee never reads it |
| Pops | 0 (the caller pops 4) |
| Return | EAX = a 0x8C-byte block of the lot pool. There is no failure path |

### Constructor `0x006C1E50`

`cSC4Lot(int32 x, int32 z, int32 w, int32 h, int32 facing)`

| | |
|---|---|
| Convention | `__thiscall` |
| `this` | the block `0x006BF430` returned; the constructed object's **base** is its `cISC4Lot` sub-object (vtable `0x00AB19B0` at +0), so the block address is the `cISC4Lot*` |
| Arguments | five int32 slots, stored as bytes (low 8 bits) |
| Pops | 20 |
| Return | EAX = `this` |
| Reference count | 0 (at +0x0C, shared by every interface): the caller AddRefs |
| Not written | +0x83 (the vacate reason) keeps the block's old contents; zones.md §8.5.1 gives every field written |

The constructor reads the active zone manager's `GetZoneType` at the location cell through the global
`0x00B43D14`, so it must run while a city is loaded.

## cSC4LotRetainingWallOccupant retaining-wall helpers `0x005E3070`, `0x005E37F0`, `0x005E3960` (LD3)

Static functions, not methods: `__cdecl`, arguments on the stack, **the caller pops**; no `this`. The lot argument
is the `cISC4Lot*` interface pointer (the pointer the lot developer's slots receive). They read the global occupant
manager `0x00B43D0C`, lot manager `0x00B43D08` and BDS `0x00B43D68`, so the original city services must be in
place (or replacements that answer the same interfaces; `0x005E3960` frees the lot manager's slot 9 list nodes with
the STL node allocator B, allocators.md).

| Function | Signature | Caller pops | Return |
|---|---|---|---|
| `AddRetainingWallOccupant` `0x005E3070` | `bool (cISC4Lot* lot, bool add)`; `add` a 4-byte slot, low byte read | 8 | AL: true when a wall occupant was removed (`add = false`) or built and inserted (`add = true`) |
| `UpdateRetainingWalls` `0x005E37F0` | `bool (cISC4Lot* lot)` | 4 | AL: no defined value (the last call's); lot developer slot 62 returns it |
| `UpdateSurroundingRetainingWalls` `0x005E3960` | `void (cISC4Lot* lot)` | 4 | none |

Callers in the lot developer: `0x006C8A3D` and `0x006C8A62` (`ClearLot`), `0x006C8C23` and `0x006C8C34` (slot 55),
`0x006C4195` (slot 62).

## cSC4PropOccupantBurntBuilding::SetBoundsFromOccupant `0x005EEB70` (LD14)

`void SetBoundsFromOccupant(cISC4Occupant* source)`: `__thiscall`, `this` = the **base** of a
`cSC4PropOccupantBurntBuilding` (class `0xC9EFB016`; `QueryInterface` for `cISC4PropOccupant` `0xE9793A65` returns
the base), `source` = the building's `cISC4Occupant*`; pops 4; no return. Caller: `ExplodeLot` `0x006CC33C`.

## nSC4UI::GenericConfirmationDialog `0x0078DFF0` (NG35)

`bool GenericConfirmationDialog(cIGZString* title, cIGZString* text, cIGZString* yesLabel, cIGZString* noLabel)`:
a free function, not a method. The neighbours' RequestCreateConnection calls it at `0x006D16DA` with both labels
null (docs/specs/neighbors.md §15.4 NG35).

| | |
|---|---|
| Convention | `__cdecl`, no `this` |
| Arguments | four 4-byte slots: `title` and `text` are `cIGZString*`; each label is a `cIGZString*` or null for the default Yes / No |
| Pops | 0 (the caller pops 16) |
| Return | AL: true only when the dialog was closed with the Yes button (ID `0x8A8DFD4E`) |
| Popups off | returns false at once (gate a, `SC4GetPopupModalDialogsEnabled`, NG36) |
| Popups on | takes the hidden pause (simulator and clock) and runs the dialog modally: the caller must not hold a lock the game's other threads or the dialog's own callers need |

## Tutorial registry and window protocol (TU6?TU9, 2026-10-22)

Native entries proved by `re-tutorial-contracts4`; see `docs/specs/tutorial.md` ??4.2.1, 4.2.2, 4.3.1 and 4.4.1 for exact order, callbacks and ownership. These are native call contracts, not new C exports. A discarded result certifies only that the native caller reads no return value; it does not invent a bool/pointer/void result for the callee.

The original tutorial window reads the borrowed `cISC4TutorialSystem*` at supported-game VA `0x00B20E88`. A replacement must bind the actual interface view through its documented lifetime and preserve reentrant callbacks. This record provides no concurrency or safe installation claim by itself. Runtime code must use version-gated compatibility addresses and test the binding/restoration and ownership before promotion.

```tsv
method	address	convention	this	arguments	callee_pop	return	ownership	evidence
cSC4TutorialRegistry::GetItem	004b5d10	__thiscall	cISC4App slot49 result, already app+0x1c0	GUID:uint32 by value	4	EAX:item* or null	borrowed list-node+8; no AddRef/free	004b5d1d,004b5d60-004b5d7c; tutorial.md 4.2.1
cSC4TutorialRegistry::SetActive	004b5d80	__thiscall	cISC4App slot49 result, already app+0x1c0	GUID:uint32 by value; active:bool byte in 4-byte slot	8	AL:changed-state bool; upper EAX unspecified	no ownership transfer; synchronous preferences save on cell modification	004b5d82-004b5d99,004b5dcd-004b5e02; tutorial.md 4.2.1
cSC4TutorialUIWinProc::constructor	00442920	__thiscall	100-byte heap-C allocation base	none	0	EAX:same base	count at base+4 starts 0; caller takes two explicit references	00442920-00442991;0090da1e-0090da2a;0043f08c-0043f0d2
cSC4TutorialUIWinProc::Init	00443e60	__thiscall	constructed window base	none	0	discarded; caller reads no return value	uses global00b20e88 borrowed live cISC4TutorialSystem*; no owner argument	0043f0cf-0043f0d2;00443eda-00443ee2;00443f37-00443f6b;00444152
cSC4TutorialUIWinProc::Shutdown	00442c80	__thiscall	constructed window base	none	0	discarded; caller reads no return value	close before releasing the two owner references	00442c80-00442e34;0043f110; tutorial.md 4.2.2
cSC4TutorialUIWinProc::OnTutorialEvent	00443970	__thiscall	constructed window base	event:uint32 by value	4	AL:true	no ownership transfer; temporarily retains current task	00443989;004439e1-004439e7
cSC4TutorialUIWinProc::moveToNext	00443bd0	__thiscall	constructed window base	none	0	discarded; caller reads no return value	temporarily retains current task and global tutorial-system interface; may post end message	0043faa6-0043faad;00443c10-00443c1c;00443ce3-00443cf6;00443e0f
class	vtable	slot	offset	target	method	callee_pop	arg_dwords	ret	ret_confidence	notes
cSC4TutorialUIWinProc::vftable	00a8638c	1	+0x4	0057eb00	cSC4TutorialUIWinProc::AddRef	0	0	eax	high	this=base; tail00968b60 increments base+4; tutorial.md4.2.2
cSC4TutorialUIWinProc::vftable	00a8638c	2	+0x8	0057eb70	cSC4TutorialUIWinProc::Release	0	0	eax	high	this=base; tail0090d997; final release invokes deleting destructor
cSC4TutorialUIWinProc::vftable	00a8638c	6	+0x18	00443e10	cSC4TutorialUIWinProc::FilterMessagePayload	4	1	void	high	this=base; uint32 payload by value; owner consumes no result;00443e3c-00443e55
cSC4TutorialUIWinProc::vftable	00a8638c	7	+0x1c	00443180	cSC4TutorialUIWinProc::ShowUIHintsForUsers	0	0	void	high	this=base; no arguments/result consumed;004433ad; owner0043f440
cSC4TutorialUIWinProc::vftable	00a8638c	8	+0x20	00442340	cSC4TutorialUIWinProc::HandleCatalogView	4	1	void	high	this=base; borrowed catalog-menu interface pointer;00442348-00442354;00442428
cSC4TutorialUIWinProc::vftable	00a8638c	9	+0x24	00442320	cSC4TutorialUIWinProc::GetCurrentTask	0	0	eax	high	this=base; borrowed task* or null, no AddRef;00442320-00442339
cSC4TutorialUIWinProc::vftable	00a8638c	10	+0x28	00441c50	cSC4TutorialUIWinProc::FilterLeftMouseClick	12	3	al	high	this=base; two borrowed pointers to32-bit coordinate values then int32 stage;00441c83-00441ca8;00441d94-00441da5
cSC4TutorialUIWinProc::vftable	00a8638c	11	+0x2c	00443b40	cSC4TutorialUIWinProc::ResetTutorialNumAndIndex	8	2	eax	high	this=base; GUID:uint32,relativeIndex:int32 by value; signed relative index or -1;00443bb7-00443bcc
```

## Power pole occupant and power line tool entries (NO44, re-no28, 2026-10-26)

Native entries the replacement power pole and the native power-line tool cross (see `docs/specs/network-occupants.md`
section 4.6.3 and the NO44 answer in `docs/specs/questions/answered/network-occupants.md`). These are native call
contracts, not new C exports. Every function is `__thiscall` and takes the concrete **base** of the primary pole object
(or, for `CreatePowerPole`, the native tool base) in ECX; none of them is reachable through a `cISC4Occupant*`. The two
`GetLineCells*` entries return in AL only; the other pole entries leave no result for their callers (EAX is not
consumed), except `GetAttachmentPoints`, which returns AL = 1 (`mov al, 1` immediately before its only `ret 8`).
Bytes were checked against the supported-game executable by the architect.

```tsv
method	address	convention	this	arguments	callee_pop	return	ownership	evidence
cSC4PowerPoleOccupant::FUN_00649910	00649910	__thiscall	concrete primary pole base	other:non-null primary pole base; flag:bool low byte in a 4-byte slot	8	none; caller reads no result	no ownership transfer	callers 0064f4f3,0064f511,006518e6,00651a2e; body 00649916-006499a4; network-occupants 4.6.3
cSC4PowerPoleOccupant::AddConnection	0064e3e0	__thiscall	concrete primary pole base	other:nullable primary pole base; wires:nullable pointer to a 12-byte native wire-vector header (item stride 16, line primary pointer at item+8)	8	none; caller reads no result	caller keeps the input container and its references; the owner retains `other` once and each line once	body 0064e3e9-0064e47f,0064e804-0064e91c; network-occupants 4.6.3
cSC4PowerPoleOccupant::GetLineCells	0064be50	__thiscall	primary pole (primary slot 5)	index:signed int32; out:writable native cell-vector header (begin,end,capacity; cell stride 8)	8	AL only (bool); upper EAX unspecified	caller owns the game-allocated output buffer; failure preserves the output, success replaces its logical content	body 0064be58-0064beed,0064bfb2-0064bfc8; network-occupants 4.6.3
cSC4PowerPoleOccupant::GetLineCellsTo	00649dd0	__thiscall	primary pole (primary slot 16)	other:primary pole base; out:writable native cell-vector header	8	AL only (bool); upper EAX unspecified	the first exact pointer match dispatches slot 5; no match preserves the output	body 00649dd0-00649e43; network-occupants 4.6.3
cSC4PowerLineTool::CreatePowerPole	00650140	__thiscall	native power-line tool base	info:pointer to a 48-byte pole-info record	4	none; caller reads no result	the factory is bypassed; fixed pole views at occupant+8 and exemplar+24h; kind dword at tool+b8h	body 00650140-00650241; network-occupants 4.6.3
cSC4PowerPoleOccupant::GetAttachmentPoints	00649ba0	__thiscall	concrete primary pole base	orientation:low byte of a 4-byte slot; out:caller-owned buffer of four xyz float triplets (48 bytes)	8	AL = 1 (always true); callers consume only the output buffer	reads base+e4h, base+a0h, base+a4h, base+a8h; no ownership transfer	body 00649bb7-00649cf8; network-occupants 4.6.3
cSC4PowerPoleOccupant::RebuildWires	0064d510	__thiscall	concrete primary pole base	other:primary pole base	4	none; caller reads no result	a native exemplar update enters both endpoint bases directly	body 0064d51a-0064d58b,0064d685-0064d695; network-occupants 4.6.3
```
