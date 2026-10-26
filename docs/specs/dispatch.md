# Spec: the dispatch manager (`cSC4DispatchManager`)

Status: **behaviour spec, short form** (the component is mostly the emergency-dispatch tool and its vehicles; this
spec covers the dispatch records, the messages the simulators react to, and the save record), clean-room input.
This document describes *behaviour*, not code. It was written by the analysis team from observing the original
program and its data files. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general
knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (components §3.6, streams §2, object references §2.7, messages §5),
[protection.md](protection.md) (§8 the dispatch sources, units and call IDs; §5 the begin/active/inactive/end
messages; §15.4 the dispatch-source interface), [advisors.md](advisors.md) (`0xAA1B2002` DisasterPeriodEnded),
[city-situation.md](city-situation.md) (uses the vehicle view), [docs/abi/layouts.md](../abi/layouts.md) (the
0x2C-byte `DispatchInfo`).

## 0. Summary

The dispatch manager keeps the list of **emergency dispatches**: fire engines, police cars, fire planes and police
helicopters that the **player** sends with the dispatch tool. It owns no simulator agent and no timer; it changes
state only when

- the dispatch tool calls it (`BeginDispatchTool`, `Dispatch`, `RevokeDispatchByPylon`, `EndDispatchTool`),
- a dispatched vehicle (automaton) reports an event,
- an occupant is removed, or a disaster period ends.

The game never dispatches by itself: no shipped code path calls `Dispatch` except the dispatch tool
(`0x004BA1B0`). In an unattended run the list stays empty and the record is the 22-byte empty form of §7.

The simulation side is small: the dispatch **sources** (the fire and police simulators and their air sources,
protection.md §8) hand out units; the manager posts four messages that the protection simulators use for their
dispatch cones (protection.md §5). Everything else (pylons, vehicles, the vehicle view) is presentation and
automata.

Implementation size: the record, the list and the messages are small (about 300 lines); the pylon layout and the
automata hand-off are UI/automata work outside the simulation milestone.

## 1. Identity

| Item | Value |
|---|---|
| Class | `cSC4DispatchManager`, class ID `0x4990BD93` |
| Component | city component **3**, interface `cISC4DispatchManager` (`0x694028EC`); `cISC4City` slot 39 (`+0x9C`, `GetDispatchManager`) |
| Global | `0x00B43D04` |
| Factory | `0x0046A7B0`, registered by the game COM director; global-heap allocation of **0x4C** bytes, constructor `0x0046A510` |
| QI (`0x004695B0`) | `1`, `0x694028EC` → `+0`; `cIGZMessageTarget2` `0x452294AA` → `+4`; `cIGZSerializable` `0xE4FDA3D4` → `+8`; else false |
| Vtables | primary `0x00A88108` (24 slots, §8); message target `0x00A880F8` (`DoMessage` `0x0046A320`); serializable `0x00A880E0` (`Write` `0x0046A250`, `Read` `0x0046A820`, `GetGZCLSID` `0x004695A0`) |
| Reference count | `+0x10`; `AddRef` shared (`0x004F7BA0`); `Release` (`0x00469760`) calls `Shutdown` and the deleting destructor (slot 23) when the count would drop from 1 |

## 2. State

| Offset | Field | Initial | Saved |
|---|---|---|---|
| `+0x0C` | `initialized` (byte) | 0 | no |
| `+0x10` | reference count | 0 | no |
| `+0x14` | `loaded` (byte): set by `Read` (before the version check) | 0 | no |
| `+0x18` | message server | null | no |
| `+0x1C` | city | null | no |
| `+0x20` | **dispatch list**: STLport `list<DispatchInfo>` (nodes of 0x34 bytes from the STL node allocator, the record at node `+8`) | empty | **yes** |
| `+0x24` | `nextID`: the last dispatch ID handed out | 0 | **yes** |
| `+0x28` | the dispatch **vehicle view** (`cSC4DispatchVehicleView`, 0xA0 bytes, registered with the renderer `0x00B43DD0` at layer 5, priority 1000) | null | no |
| `+0x2C` | the **active pylon** (the one the player is dragging) | null | **yes** (ref) |
| `+0x30`–`+0x38` | **available pylons**: vector of `cISC4PylonOccupant*` (one per free unit of the open tool's type) | empty | **yes** (refs) |
| `+0x3C` | `toolType`: the dispatch type whose tool is open; **8 = none** | 8 | **yes** |
| `+0x40`–`+0x48` | registered dispatch **sources**: vector of `cRZAutoRefCount<cISC4DispatchSource>` | empty | no (sources register again at their `Init`) |

The record `DispatchInfo` (0x2C bytes) is specified in docs/abi/layouts.md: type, ID, call ID (the station's unit
number), vehicle-view handle, `active` byte, target x/y/z, station occupant, automaton, pylon.

**Dispatch types** *[inferred]* from the sources: **0** police car, **1** fire engine, **5** fire plane, **6**
police helicopter; **8** means "no tool". No shipped source handles 2, 3, 4 or 7 (DP1, verified): the only four
`cISC4DispatchSource` classes (the Mac build's symbols list no other `CanHandleDispatchType`) are the police
simulator (type 0, `0x006E0210`), the fire simulator (type 1, `0x006ACFA0`) and the two air sources, whose shared
slot 3 (`0x006DF760`) compares with `+0x14`, set to 5 by the fire plane constructor (`0x006ACE33`) and to 6 by the
police helicopter constructor (`0x006DFFA3`).

## 3. Agents, messages and cadence

No simulator agent, no `AddAgent` name, no animation tick.

Subscriptions (`Init`; removed in `Shutdown`):

| Message | Handling |
|---|---|
| `0x99EF1143` RemoveOccupant (data1 = occupant) | every record whose **station** is that occupant is dropped **without** `EndDispatch`: if it has a vehicle, untrack it in the vehicle view (when the view exists), despawn it (automaton slot 13 `Remove` with 0, `+0x34`) and **release** the record's reference; if it has a pylon, `SetActivePylon(null)` when it is the active one, then shut the pylon down and release it; release the station; erase the record. **No end message, no inactive message, no unit returned** *[quirk]*. Then every available pylon whose source occupant (pylon slot 18) is that occupant is shut down, released, `SetActivePylon(null)` if it is the active one, and erased from the vector (DP7; `DoMessage` `0x0046A320`) |
| `0xAA1B2002` DisasterPeriodEnded | every record's vehicle is told to go home (automaton slot 29); records stay until the vehicles report back |
| `0xA6B79602` view changed | the vehicle view refreshes (presentation) |

Direct deliveries (no subscription): each vehicle automaton the manager creates gets the manager as its event
target, so the automaton's **`0x09D92FC3`** events arrive at `DoMessage`. data1 = the automaton, data2 = event
code:

- **3** (the vehicle is gone): `RevokeDispatchByID` of its record;
- **7** (state change), data3 = the new dispatch state: `UpdateDispatchState(record, state)` (§4.3); if the new
  state is **0**, also `RevokeDispatchByID`.

Other event codes, or a null automaton, return false; an automaton with no record returns true.

Posted (`MessagePost`, queued, priority 0; IGZUnknown = the manager, data1 = dispatch type, data2 = dispatch ID):

| Message | When |
|---|---|
| `0xE94671EB` begin | `BeginDispatch` succeeded (§4.2) |
| `0xE94671EC` active | the record's state becomes 5 |
| `0xE94671ED` inactive | the record's state leaves 5 |
| `0x894671F6` end | `EndDispatch` (§4.4) |

The protection simulators react to them (protection.md §5 table, §8.3). Because they are **queued**, a handler that
calls `GetDispatchInfo(data2)` may find the record already gone.

## 4. Behaviour

### 4.1 `Init` (`0x0046A050`), `Shutdown` (`0x0046A1A0`)

`Init` (once): hold the message server and subscribe (§3); create the vehicle view and register it with the
renderer; hold the city. If `loaded`: `toolType := 8`, `SetActivePylon(null)`, clear the available pylons, and
re-track every record's vehicle in the vehicle view (types 0 and 6 as kind 1, types 1 and 5 as kind 0, other
types untracked; the new handle goes to record `+0x0C`). Return AL = 1.

`Shutdown`: unsubscribe and release the message server; shut down and release the active pylon (no UI state
change); clear the available pylons; unregister and release the vehicle view; release the city. Records are **kept** (they are released by the
destructor).

**Pylon calls** (`cISC4PylonOccupant`, vtable `0x00AA5150`): slot 3 the occupant (slot 9 of which is `SetPosition`);
slot 5 `Shutdown` (`0x005F11A0`: when the pylon is initialized, it also clears and releases its source occupant
`+0x78`); slot 6 `SetUIState`; slot 9 `SetDispatchType`; slot 10 `GetDispatchType` (`+0x74`); slot 11
`SetDispatchState`; slot 13 `SetCallNumber`; slot 14 `GetCallNumber`; slot 17 `SetSourceOccupant`; slot 18
`GetSourceOccupant` (`0x00407AB0`, `+0x78`).

**Dropping pylons** (DP4, DP5). Three different operations, each with its own exact steps:

- *Clear the available pylons* (`0x00469E10`; `Init` when loaded, `Shutdown`, `BeginDispatchTool`,
  `EndDispatchTool`): walk the vector from the back; each non-null pylon is shut down (slot 5) and released, then
  popped. A pylon that is also the active one is shut down too; the active pointer keeps its own reference.
- *`SetActivePylon(p)`* (`0x00469630`): if `p` differs from the active pylon, the old one gets `SetUIState(1)` and
  is released (**not** shut down); `p` is held and gets `SetUIState(2)`.
- *`Shutdown`'s drop of the active pylon*: shut down and released, no UI state.

`Init` when loaded does `SetActivePylon(null)` first, then clears the available pylons. `EndDispatchTool` clears
only the available pylons and leaves the active pylon alone.

#### 4.1.1 Native pylon construction and callable prefix (DP11) *[compat]*

The native class is `0x49B6D69B` (`GetGZCLSID`, `0x005F1A10`), with registered factory
`0x005F2060`: allocate `0x1CC` bytes through `GlobalHeapAlloc`, then invoke constructor
`0x005F1730` with ECX = allocation and one stack argument 1. Allocation failure returns null.
The factory returns the object start without calling AddRef or Init. Dispatch uses this same
allocation/constructor path directly, then primary slot 1 AddRef and slot 4 Init, in that order
(`0x0046B30B`–`0x0046B340`). The tool dereferences a null construction result; it does not turn
allocation failure into a false result. A factory client must acquire its own reference before
retaining the result; normal GZCOM acquisition uses QueryInterface.

Primary QueryInterface `0x005F0820` accepts pylon IID `0xC9B6D7D6` at object start, and
message-target IID `0x452294AA` at `+4`, acquiring a reference on success. Other IIDs are tried
through the occupant at `+8`, then the model-instance base at `+0x6C`. Slot 3 is a borrowed
occupant pointer at **+8** (`0x006C2040`), not Init. No extra QueryInterface is required for
AsOccupant, and its result must not be independently released. The callable primary prefix
(`0x00AA5150`) is below. All stack arguments occupy dwords; offsets are relative to the primary
pointer, not the occupant subobject. Setter return values are unused by dispatch unless stated.

| Slot | Contract / stack bytes popped |
|---|---|
| 0 | QueryInterface(IID, out interface), AL success, acquired / 8 |
| 1, 2 | AddRef, Release on the common object lifetime / 0 |
| 3 | AsOccupant, EAX borrowed `this+8` / 0 |
| 4, 5 | Init (`0x005F0930`), Shutdown (`0x005F11A0`), AL true / 0 |
| 6, 7 | SetUIState(u32), GetUIState() → EAX, member `+0x6C` / 4, 0 |
| 8 | SetBitmapInfo(u32 bitmap value, const cIGZString& label), `0x005F0900`; copies the label into the embedded cRZString; no success return / 8 |
| 9, 10 | SetDispatchType(u32), GetDispatchType() → EAX, member `+0x74` / 4, 0 |
| 11, 12 | SetDispatchState(u32), GetDispatchState() → EAX, member `+0x70` / 4, 0 |
| 13, 14 | SetCallNumber(u16 in low word), GetCallNumber() → **AX only**, member `+0x14C`; upper EAX is unspecified / 4, 0 |
| 15, 16 | SetDragState(bool in low byte), GetDragState() → low-byte bool, member `+0x7C` / 4, 0 |
| 17, 18 | SetSourceOccupant(pointer), GetSourceOccupant() → borrowed EAX, member `+0x78` / 4, 0 |
| 19, 20 | SetPylonOwner(pointer), GetPylonOwner() → borrowed EAX, member `+0x1C0` / 4, 0 |

SetSourceOccupant (`0x005F0D30`) and SetPylonOwner (`0x005F1160`) do nothing on identical
pointers; otherwise AddRef the new non-null pointer, store it, then Release the old one.
Initialized Shutdown clears and releases both held pointers after unsubscribing/releasing its
presentation resources; repeated Shutdown succeeds without repeating this work. Init subscribes
the `+4` message target to `0xA6B79602`, `0x06CDB65B`, `0x26D31EC1` and animation tick `0x80`;
it is idempotent and returns true. Position is supplied through AsOccupant slot 9, one pointer
to three consecutive float32 values `(x,y,z)`, callee pops 4; dispatch ignores its return.

#### 4.1.2 Station geometry and return predicate (DP11) *[compat]*

The station **occupant**, not a lot interface, supplies the placement bounds. Occupant slot 11
`GetBoundingCityCells(region&)` writes four signed dwords `(minX,minZ,maxX,maxZ)`. City slot 96
`CellCornerToPosition(i32 x,i32 z,float* outX,float* outZ)` pops 16 and has no result channel.
Convert `(minX,minZ)` to corner A and `(maxX+1,maxZ+1)` to corner B; obtain each Y from city
slot 84's terrain slot 13 GetAltitude(x,z), ST0 stored to float32. No rotation, lot transform,
road heading, or half-cell offset is requested (`0x0046B1A2`–`0x0046B22C`).

For BeginDispatchTool count < 2, A is instead station occupant slot 8 GetPosition(vec3&),
direction and spacing are zero. Otherwise use D = B−A, length = sqrt(Dx²+Dy²+Dz²),
spacing = length / unsigned32(count+1). Normalize D only if length > 0. Each free unit i uses
`A + (Dnormalized * spacing) * (i+1)`; occupied indices still leave their parking gap. The
loop index is a byte, the free-mask shift uses its low five bits, and the stored call number
is the low 16 bits of `base+i`; count ≥ 256 can make the native byte loop nonterminating.

Numeric order is compatibility-visible: differences and normalized components are stored as
float32; BeginTool's length is retained in x87 while computing spacing, which is stored as
float32. Normalization uses the reciprocal in x87, not a separately stored float32 reciprocal.
X/Y step-times-index are stored before adding A; Z step is stored before multiplying by index.
See `0x0046B230`–`0x0046B2BC`, `0x0046B343`–`0x0046B3C4`; do not simplify to D/(count+1).

ReturnPylonToStation always recomputes the diagonal from the bounds, including count 0 or 1;
it does not use BeginTool's single-unit position shortcut. Its length is stored as float32,
and its reciprocal and each step component are also stored as float32 before positioning
(`0x0046AB2E`–`0x0046ABFE`). Its parking multiplier is
`1 + u8(low8(callNumber) - low8(base))`, **not** full-width callNumber−base+1
(`0x0046AC02`–`0x0046AC0D`). It resets dispatch/UI state only after successful slot-7 unit-state
lookup, but inserts an absent pylon into the available list with AddRef even when that lookup
fails. Existing null/missing-source cleanup rules remain as §4.4 states.

The input tool's return predicate (`0x004BA1B0`) uses the active pylon's current occupant
position and its source occupant's same bounds converted to corners. It revokes when
`minWorldX <= p.x < maxWorldX` and `minWorldZ <= p.z < maxWorldZ`, ignoring height. The upper
edges are exclusive (`0x004BA248`–`0x004BA28D`). This is an axis-aligned footprint test,
not GetOccupantLot or a rotated-lot containment call. It then clears the active pylon.

DP11's callable prefix and geometry are resolved here. Slot 8 requires a non-null string
interface pointer, not a char buffer, resource key, or texture-binding interface. It stores
the bitmap dword at `+0x1A8`, copies into the embedded string at `+0x1AC`, then marks the
presentation dirty (`+0x68 := 1`). The copy invokes source string slot 7 Strlen, then source
slot 6 ToChar, then destination slot 3 FromChar(buffer,length), in that order
(`0x005F0900`–`0x005F0926`, `0x0090E230`–`0x0090E23F`, `0x0090E20A`–`0x0090E22D`).
It neither retains nor releases the source interface and does not accept null. Empty strings
are valid; the explicit length controls copying, including bytes after an embedded NUL.
The caller may destroy or change its string after the call. There is no boolean success
channel; the wrapper's incidental EAX is the embedded destination string pointer.

The prefix's narrow returns matter: GetCallNumber writes AX, without clearing the high
word of EAX (`0x005F1A40`); GetDragState writes AL only (`0x005F1A60`). Its setter copies
the input low byte literally, with no boolean normalization and no dirty flag
(`0x005F1A50`). SetCallNumber writes the low word and always marks dirty (`0x005F1A20`).
UI/dispatch state/type setters dirty the presentation and clear `+0x1A4` only when the
dword value changes (`0x005F0CA0`, `0x005F0CD0`, `0x005F0D00`); their return registers
are not success results. No shifted SDK method numbering is compatible with this prefix.

#### 4.1.3 Vehicle-view native boundary (DP10, partial) *[compat]*

Manager slot 22 returns the **borrowed object-start pointer** to its concrete vehicle view,
not an acquired unknown interface. `Init` allocates `0xA0` through GlobalHeapAlloc, invokes
constructor `0x0046F120`, calls primary slot 1 AddRef, direct `Init` at `0x0046B550`, then renderer
slot 32 `(view,5,1000)`; it ignores Init/registration return values and has no null-allocation
recovery (`0x0046A0A9`–`0x0046A0F0`). No view CLSID/factory or dedicated view IID is requested.
Shutdown calls renderer slot 33 `(view)` **before** direct view Shutdown `0x0046D810`, then
primary Release and clears its member (`0x0046A214`–`0x0046A233`).

Exported data references define the following primary prefix at `0x00A88248`:
slot 0 QueryInterface `0x0046B500`, slot 1 AddRef `0x005BE3E0`, slot 2 Release `0x005BCB30`,
slot 3 Update `0x0046D990`, slot 4 the always-false, four-dword-pop pick function `0x00735290`.
The prefix does **not** contain Init, Shutdown, TrackAutomaton or UntrackAutomaton slots.
Those names must not be invented to complete a public SDK interface.

QueryInterface(IID,out) zeros out first. Message-target IID `0x452294AA` yields acquired
`this+0x0C`; otherwise it delegates to cRZUnknown at `this+4`. That base accepts IID 1 and
returns acquired `this+4`, not the primary pointer (`0x0046B500`–`0x0046B540`, `0x009457C6`).
Primary AddRef/Release adjust ECX by +4. Message-target QI/AddRef/Release use their adjustment
thunks, and message slot 3 is DoMessage `0x0046D0F0` (data references `0x00A88220`–`0x00A88238`).

Tracking is direct `TrackOccupant` at `0x0046F240`, ECX = view start, four stack arguments:
occupant, kind, display/call number, optional label pointer. It returns a **32-bit EAX handle**,
0 for a null occupant or a reached nonzero kind limit. Kinds used by dispatch are 0/1 with
null label; numbers come from the vehicle's low-16-bit call number on new creation. Tracking
acquires one occupant reference (`0x0046F608`–`0x0046F60C`). A successful track increments a
process-global dword at `0x00B215FC` and stores that value as the handle, plus a per-kind count
(`0x0046F61B`–`0x0046F646`). This is not a dispatch ID, pointer token, per-manager counter, or
saved value. New manager construction does not itself allocate a handle. Re-targeting does
not track again. Limits initialize to `0xFFFFFFFF` in Init.

Direct `UntrackOccupant` at `0x0046EFB0` takes `(u32 handle,bool immediate)` and pops 8;
AL false for zero/absent handle, true for found. Non-immediate removal of an entry with
flag bit 3 sets bit 4 and returns true without immediate removal. Otherwise it temporarily
AddRefs the occupant, removes track info, sorts remaining indicator priority, then Releases
that temporary hold. View Shutdown releases every retained tracked occupant and clears its
map. Further update/removal contracts are below; DP10 remains open for the complete drawing
protocol. These boundary facts alone do not establish a complete native view provider.

**Update, refresh and delayed removal (DP10).** Primary slot 3 is a thiscall taking one
borrowed `SC4DrawContext*`, callee pops 4, AL true on every normal exit (`0x0046D990`,
`0x0046EFA5`). It is not a no-argument refresh or an elapsed-time callback. It reads the
context at `+0x38`, then that object's `+0x0C` pointer and byte `+0x39C`; a set byte skips
all per-track update/drawing. Otherwise it also requires global `0x00B43DC4` slot 22 to
return 1 (`0x0046D9F8`–`0x0046DAA8`). These are separate gates, before the track walk.
The internal static timer stops and samples elapsed time before either gate; it restarts
only after the normal track walk. Consequently removal is not guaranteed to complete while
a gate suppresses the walk (body `0x0046D990`).

TrackInfo's dword flags are at `+0xFC` and fade value at `+0xF8`, relative to the info,
not the tree node (which prefixes 0x14 bytes). Dispatch kinds 0 and 1 start with flags
`0x2A` and fade zero (`0x0046F240` kind switch). Non-immediate untrack sets bit 4 only;
it does not change the count, release the occupant, or allocate another handle.
For an info with bit 3 set, each eligible Update decreases fade for pending removal,
clamps at zero, stores float32, and tests the still-live x87 result against float32
0.01. Removal occurs strictly below 0.01, not at equality. A nonpending entry increases
fade and clamps at 1.0. Delta is the timer's float32 milliseconds multiplied in x87 by
float32 0.001 and float32 2.0 at `0x00B07F84`, then stored as float32 before use
(`0x0046D9EB`–`0x0046DA15`, `0x0046DC1E`–`0x0046DC51`, `0x0046DC92`–`0x0046DCB4`).
Do not round the subtraction to float32 before the threshold comparison.

Completion holds a temporary occupant reference, calls RemoveTrackInfo, sorts priorities
with that occupant, then releases the temporary hold (`0x0046DC53`–`0x0046DC8D`). Both
immediate untracking and delayed completion use the same removal body `0x0046D650`:
decrement the per-kind count if nonzero; release the generated texture if nonzero; QI
automaton IID `0xA9B40F05` and, on success, clear flag `0x200000` through automaton slot 30
`(0x200000,false)`; release the held animated model (with its virtual-base adjustment),
destroy/free its model processor if present, Release the tracked occupant, erase the
tree entry, then Release the temporary QI result. Texture/model release therefore precedes
the retained occupant release; the QI hold survives erasure (`0x0046D650`–`0x0046D721`).

Direct HandleViewChanged `0x0046D2B0` takes no arguments and rebuilds each retained info's
call-number display with `0x0046C8B0`, in map order. Message-target slot 3 DoMessage receives
one message pointer with ECX = primary+12; message ID `0x8ACAA95F` performs the same rebuild,
others do nothing; AL true (`0x0046D0F0`–`0x0046D16F`). Init subscribes that adjusted target
to this ID (`0x0046B743`–`0x0046B75B`). This does not re-track occupants or advance handles.

Init loads seven texture-resource handles through service class `0x1AC0E11A`, IID
`0xFAC0E219`, slot 6 `(key3&,out handle)`. Keys use type `0x7AB50E44`, group `0x1ABE787D`,
with instance IDs, in member order `+0x2C` through `+0x44`:
`144161A1,144161A2,144161A3,4BB0ECF3,2BB12D1F,4BB0ECF3,144161A2`
(`0x0046B61C`–`0x0046B72B`). Kinds 0/1 use the first/second handle respectively and use
their number as the call display, with flags `0x2A`; they do not require the optional label.
The number branch, rectangle protocol and final 2D submission are specified below.
The exact remaining geometry/alternate-branch scope is parked at the end of this section;
none of these facts authorizes inventing SDK view slots.

**Call-number texture rebuild (DP10; instruction-verified).** Direct
BuildCallNumberDisplay `0x0046C8B0` operates on one stack TrackInfo pointer, pops 4,
and has no defined success return channel (`0x0046D0E3`–`0x0046D0EF`). Flag bit 6 causes
an immediate return, before releasing or modifying its existing texture. Otherwise a
non-null texture at info `+0x0C` is passed to ReleaseTexture `(global 0x00B43CA4,texture)`
and that member is cleared first (`0x0046C8CB`–`0x0046C91A`). Rebuild failure therefore
does not preserve the previous texture. Kinds 3/4 have separate image branches; for the
dispatch kinds 0/1, flag bit 1 selects the following number branch.

WCM slot 5 receives `0x68963C4C`; its returned font is AddRef'd. Font slot 15 returns the
old style, then slot 16 receives 3. The dword display number at info `+0x10` is formatted
with signed-decimal format `%d` at `0x00A8281C` (original PE bytes). Font slot 48 receives `(bytes,length)` for width, then
slot 37 receives the same pair for height. Start the square surface side at 32; only
when width or height is greater than 32, replace it with the smallest power of two at
least their signed maximum. This is ceiling-to-power-of-two, despite the helper's
name GetNearestPowerOfTwo (`0x0046C950`–`0x0046CA08`; helper `0x006046B0`).

If global surface service `0x00B43C9C` is non-null, its slot 3 is called with an output
surface pointer. Only AL success continues. On that surface, slot 3 receives
`(side,side,descriptor8)`; the 8-byte by-value descriptor is constructed from value 9
by `0x0044A490`. Only AL success continues. Slot 6 receives `0x8001`; if AL succeeds,
slot 18 receives `(0,0)` and slot 7 receives `0x8001`. A false slot-6 result skips
those two calls but **does not** skip text drawing. Font slot 51 then receives
`(surface,0,0,bytes,length,0xFF000000,0x08000000,0,0)`, with its return ignored.
If global texture service `0x00B43CA4` exists, SetupTexture is called with
`(surface,service,&info.texture,1,0,0,0)`; its return is ignored. The surface is Released
after slot-3 success from the service, including when surface initialization fails
(`0x0046CA08`–`0x0046CAD6`). These are observed native slot calls, not proposed SDK names.

Every normal number-branch exit restores the old font style through slot 16, Releases
the acquired font and destroys the temporary string. Even when surface creation or
texture setup fails, info `+0xD0` is set to the measured width converted to float32 and
`+0xD4` to float32 14.0. The branch's geometry depths are float32 0 and
`0x3E99999A` (approximately 0.3); absence of a texture does not suppress geometry
construction (`0x0046CADD`–`0x0046CB34`). If neither the kinds-3/4 branch nor bit 1
applies, both dimensions are zero (`0x0046CB39`–`0x0046CB4D`).

Independent distinguishing vectors: measured sizes (31,32), (32,32), (33,14) and
(65,14) require sides 32,32,64,128 respectively. With an old texture, a surface-service
failure must release/clear the old texture, restore the font style, Release the font,
retain the measured dimensions and leave no generated texture. A surface slot-6 false
result must still draw text and attempt SetupTexture; a bit-6 entry must perform none
of those calls. These vectors derive from native branches, not replacement outputs.

**Number/icon geometry before camera rotation (DP10).** The following describes
the intermediate quads, before the in-place camera rotation at `0x0046CF41` onward.
It must not be used as a contract for the final rotated vertex bytes. Both arrays
contain four `(x,y,z,u,v)` float32 vertices: number at info `+0x30`, icon at `+0x80`.
Use the global view utility at `0x00B43DD8`; its conversion U(p) returns the x87
product of float32 p and its float32 member `+0x150`, with no intervening float32
store (`0x007F6690`). Read zoom through GetZoomAndRotation `0x007F66A0` and use that
index into the float32 table `0x00A88170`: entries 0..5 are 20,30,40,50,60,60.
Read the pitch as float32 utility member `+0x18` and evaluate its cosine in x87.

For ordinary finite inputs, let R denote a float32 store/reload, and let:
separation = R(U(table[zoom])/cos(pitch));
iconHeight = R(U(64)/cos(pitch));
iconHalfWidth = R(U(64)*0.5);
iconHalfHeight = R(iconHeight*0.5);
numberHalfWidth = U(measuredWidth)*0.5 (retained in x87);
numberHalfHeight = R(R(U(measuredHeight))/cos(pitch)*0.5).
The conversion of measured width/height to U's argument is itself stored float32.
U(64) remains live for the iconHalfWidth multiplication after storing iconHeight;
it is not replaced by a division-rounded value (`0x0046CCF4`–`0x0046CDA8`).

| Quad | Vertices, in submitted order, before rotation |
|---|---|
| Icon | `(-iconHalfWidth, R(iconHeight+separation), 0, 0, 0)`; `(-iconHalfWidth, separation, 0, 0, 1)`; `(iconHalfWidth, separation, 0, 1, 1)`; `(iconHalfWidth, R(iconHeight+separation), 0, 1, 0)` |
| Number (bit 1) | x values `-numberHalfWidth,-numberHalfWidth,+numberHalfWidth,+numberHalfWidth`, individually stored float32; top y `R((numberHalfHeight+iconHalfHeight)+separation)`, bottom y `R((iconHalfHeight+separation)-numberHalfHeight)`, in top/bottom/bottom/top order; z is `0x3E99999A` at all four vertices |

For the number UVs, first store float32 reciprocal `1/side`. Left u and top v
are that reciprocal multiplied by float32 zero; right u is the measured width
times the stored reciprocal, stored float32; bottom v is the measured height
times the stored reciprocal, stored float32. Number UV order is left/top,
left/bottom, right/bottom, right/top. The quad's geometric measured height is
the font height, while the info `+0xD4` dimension remains the literal 14.0
specified above (`0x0046CDB2`–`0x0046CF0F`). This distinguishes a cropped text
quad from stretching a whole power-of-two texture over a 14-pixel rectangle.

Worked intermediate example: U's scale 1, pitch 0, table index 0, measured
number size 10 by 12 and side 32 give icon x ±32, y 20..84; number x ±5,
y 46..58, z `0x3E99999A`, right u 0.3125 and bottom v 0.375. The number's
stored display dimension remains height 14. All values here precede rotation.

**Last screen rectangle and 2D draw submission (DP10).** Direct thiscall
GetLastScreenIconRect `0x0046BFA0` takes one u32 handle, pops 4 and returns EAX:
null for an absent handle, or when either rectangle extent is zero; otherwise a
**borrowed pointer** into its tree node. Node `+0xEC` is info `+0xD8`; its four signed
dwords are left, top, right, bottom. The getter tests `right-left` and `bottom-top`
for zero, not positivity: reversed extents are not rejected. There is no output-copy
argument, acquisition, or view-handle allocation. Mutation/removal can invalidate
the pointer (`0x0046BFA0`–`0x0046BFDF`).

The 2D placement branch stores the chosen integer rectangle at info `+0xD8..+0xE4`,
and its float32 anchor at `+0xE8..+0xF0`. Unless bit 7 is set, the anchor is
`(left+(right-left)*0.5, top+(bottom-top)*0.5, 0)`; with bit 7 set, the previous
anchor is retained. The native integer differences are converted before x87
multiplication and addition, and each anchor component is stored float32
(`0x0046E7ED`–`0x0046E811`, `0x0046E9DF`–`0x0046EA45`). Do not confuse these
members with fade at `+0xF8` or the flags at `+0xFC`.

The visible rectangle used for automatic placement is inset by 32 integer pixels
on each side. Its horizontal center is `(right-left)*0.5+left`, with the inset
values, and likewise vertically (`0x0046E296`–`0x0046E321`). Before overlap search,
the prospective icon is made 42 by 42 pixels using float32 additions; its four
edges are individually converted to signed integers with `_ftol2` (truncation
toward zero, not floor or round-to-nearest; `0x0046E38E`–`0x0046E3E4`). The overlap
search may then move it, using the ordered protocol below.

The final 2D submission sequence is instruction-verified at
`0x0046EE02`–`0x0046EEE5`. All context calls use the **same borrowed Update context**:

| Order | Native call/effect |
|---|---|
| 1 | DrawContext SetModelTransform `0x007D2D20` with no stack arguments; ClearTexTransform `0x007D4420` with stage 0 |
| 2 | Context `+0x34` service, slot 25, with no stack arguments |
| 3 | SetTransparency `0x007D2A20` with info's float32 fade; SetRenderState `0x007D4530` with pointer to view `+0x14` |
| 4 | Submit icon and optional call-number quads in the order below |
| 5 | ResetTransparency `0x007D2A30`; context `+0x34` service, slot 24, no stack arguments |

The icon's resource-handle pointer is info `+0x2C`. Its texture argument is the
dword reached by **two dereferences**, whereas the generated call-number texture
argument is info `+0x0C` directly. SetTexture `0x007D4070` receives `(texture,0)`;
every quad uses DrawPrims `0x007D2990` with `(6,2,4,vertices)`. The two temporary
vertex arrays each contain four 20-byte `(x,y,z,u,v)` records (the transform loop
advances 0x14 and terminates at 0x50; `0x0046ED10`–`0x0046EDEF`). There is no null
texture test at submission: when bit 1 requests a number, its quad is submitted
even if number-texture creation left `+0x0C` zero.

If the icon resource-handle pointer equals view `+0x34`, draw the number quad first
when bit 1 is set, then the icon. Otherwise draw the icon first, then the number
when bit 1 is set (`0x0046EE35`–`0x0046EED2`). This is a **handle-pointer identity**
comparison, not equality of texture IDs, type numbers, or number text. Thus the
first two dispatch kinds need not share draw order. Submission does not AddRef
the context, acquire an occupant, increment the process-global handle counter,
or serialize any of the rectangle/anchor data.

Independent vectors: missing handle and rectangles `(1,2,1,9)` or `(1,2,9,2)`
return null; `(9,8,1,2)` returns the retained rectangle pointer. Rectangle
`(-3,-5,40,38)` anchors at `(18.5,16.5,0)` with bit 7 clear, but preserves an
existing anchor with bit 7 set. Two different handle pointers referring to the
same texture must still select the identity-based order above. A bit-1 entry
with null generated texture must still emit the number DrawPrims call.

**Rectangle conflict and relocation (DP10).** The cdecl helper `0x0046B830` takes
`(candidate20*,obstacleList*,bounds16*,outConflict20*)`; the caller removes its
four dword arguments. A 20-byte record is four signed rectangle edges followed
by a category byte (the remaining three bytes are not used for category tests).
AL **true means conflict**, not success. It first tests containment against
bounds, inclusive of equal edges. Out-of-bounds writes the bounds rectangle and
category `0xFF` to outConflict and returns true. Otherwise it walks obstacles in
list order. An obstacle participates only if its category is `0xFF` or equals
the candidate category. Rectangles overlap when both closed intervals overlap;
**touching edges conflict**. The first matching overlap copies all 20 bytes to
outConflict and returns true. No match returns AL false without initializing
outConflict (`0x0046B830`–`0x0046B8EF`). This helper does not alter the candidate.
The category table at `0x00A881AC` is `1,1,1,1,2,2,3` for kinds 0 through 6
(original PE bytes), so fire/police indicators compete with one another.

For the relocation branch, first save the prospective integer rectangle as a
fallback. Set a float32 moving center to `(integerLeft+21, integerTop+21)`.
Let D be its vector toward the inset bounds' center. Compute its length L in
x87, store `1.75*L` as the float32 travel limit, and store the two components
of `D/L` as float32 step components. Progress begins at zero. The native code
does not special-case L=0 before division (`0x0046E3E9`–`0x0046E495`).

While progress is below that limit, make a main candidate by truncating each
`centerX±21,centerY±21` to signed integer, retaining the entry's category, and
test it with the conflict helper. A conflict-free candidate ends the search.
When the first conflict's category is less than `0xFF`, test these **four**
additional rectangles, in order, each derived from that first conflict:
translate vertically by +43, vertically by −43, horizontally by +43,
horizontally by −43. Stop at the first conflict-free rectangle. These are not
four offsets from the original candidate, and subsequent conflicts returned
by those probes do not replace the first conflict used to construct them.
An out-of-bounds (`0xFF`) first conflict skips all four probes. If all attempted
rectangles conflict, advance each moving-center component by twice its stored
step component, storing float32, and add float32 2.0 to progress before the
next limit test. Progress is stored float32 but the limit comparison uses the
still-live x87 sum, not a reload of that store (`0x0046E727`–`0x0046E744`).
The candidate/probe order is at `0x0046E4A0`–`0x0046E744`; constants 43 at `0x00A88260`,
1.75 at `0x00A88264`, 2 at `0x00A8825C`, verified in the original PE.

After the loop the helper is called once more on the current candidate. If it
still conflicts, restore the **original saved integer rectangle**, even when
it overlaps or is outside the inset bounds. Append the selected rectangle and
category to the frame's obstacle list in either case. This is bounded search
with an overlapping fallback, not a guarantee of collision-free placement
(`0x0046E74A`–`0x0046E7D0`).

Independent vectors for the helper: with bounds `(0,0,100,100)` and a category-1
obstacle `(10,10,20,20)`, category-1 candidate `(20,12,30,18)` conflicts by edge
contact, whereas `(21,12,31,18)` does not. Changing the obstacle to category 2
removes that conflict; category `0xFF` restores it. Candidate `(-1,0,10,10)`
returns bounds/category `0xFF` before inspecting obstacles; `(0,0,10,9)` is
contained. Ordered same-category obstacles must return the first overlap,
not the nearest or highest-priority one. A fully blocked relocation must
append the initial fallback rather than omit the indicator or keep the last
failed probe. These checks distinguish the native failure policy independently.

**DP10 final-round residual (PARK, 2026-10-22).** The contracts above resolve
the dispatch number-texture branch, intermediate quads, borrowed rectangle
getter, relocation helper/search/fallback, and final 2D draw submission. DP10
is still open for the exact final vertex bytes and complete visible-entry
protocol. The remaining bounded evidence targets are:

- BuildCallNumberDisplay's camera rotation, including x87 coefficient stores
  and in-place vertex operation order: `0x0046CF41`–`0x0046D0DB`. The angle is
  formed with FPATAN from negated view-utility members `+0x54,+0x5C`; do not
  replace it with an assumed rotation-index formula.
- BuildCallNumberDisplay kinds 3/4 resource branches and their texture/UV
  geometry/failure exits: `0x0046CB52`–`0x0046CCDA`. These are separate from
  the dispatch kinds 0/1 number branch documented here.
- Update's complete eligibility/priority and occupant/terrain/projected-point
  preparation before rectangle placement (`0x0046DCBA`–`0x0046E296`), plus
  construction/rotation of the final 2D icon/number vertex arrays
  (`0x0046EA4A`–`0x0046EDF1`). The final submission call order is already proved;
  its input vertex values are not yet fully specified.
- The alternate world-space two-pass drawing path `0x0046E845`–`0x0046E9DA`
  and bit-6 3D-model-indicator branch `0x0046DE07` are not covered by the final
  2D submission table. Do not conflate them with dispatch bit-1 number drawing.

No fourth round is authorized. Preserve the worktree and these partial native
contracts; no complete view-provider, complete-owner, exact-shadow or supported
replacement credit follows while this residual remains. No new runtime trace
was requested or performed to replace these missing static contracts.

**DP10 closed (2026-10-25, re-dispatchview8).** Every range of the residual above is
specified in [dispatch-vehicle-view.md](dispatch-vehicle-view.md): the camera rotation
(§4.2), the kinds 3/4 images (§4.1), the complete per-entry walk with the building ring,
the bounds clamp and the choice between paths (§5.2), the final 2D vertices (§5.3), the
world-space two-pass path (§5.4) and the 3D-model indicator (§5.7). It also specifies the
view's other entry points and tracking kinds 2–6, and the dispatch tool (§6).

### 4.2 `Dispatch(type, x, z)` (`0x0046A630`, slot 6) → ID *[compat]*

**BeginDispatch's native call layout (DP12, partial).** All calls below use ECX for this;
stack arguments are in source order, each occupying a dword. Source slot 11 takes
`(station, u32* group, key3* modelKey, u32* flags, vec3* target)`, pops 20 and returns AL.
BeginDispatch initializes group, model key and flags to zero, takes target x/z from the record
and refreshes target y from terrain ST0 into float32, then calls slot 11 and **ignores AL**
(`0x00469914`–`0x00469979`). The three dwords of modelKey are not a spawn-position output.
The shipped sources ignore station and target: fire ground group `0x4202`, police ground
`0x4207` during disaster else `0x4228`, fire plane `0x4303`, police helicopter `0x4301`;
all write key `(0,0,0)` and flags `0x11882` (`0x006AD5B0`, `0x006E0C00`,
`0x006ACB30`, `0x006DFAC0`).

`GetAutomataManagerForGroup(group)` at `0x0050CCA0` tries city slots 43,44,45,46 in that order,
accepting the first non-null manager whose slot 30 `(group)` returns AL true. No AddRef is
taken on the manager. If none accepts, the helper runs its diagnostic path and returns the
last candidate (ESI), which is not forcibly nulled (`0x0050CD90`–`0x0050CD98`). BeginDispatch
tests only the returned pointer; a non-null last candidate therefore proceeds even after rejection.

BeginDispatch then passes these two temporary native descriptors by address:

| Descriptor | Layout |
|---|---|
| source | u32 tag 0 at +0, station occupant pointer at +4 |
| destination | u32 tag 1 at +0, target float32 x/y/z at +4/+8/+12 |

New vehicle: manager slot 18 `(+0x48)` arguments are
`(group, &source, &destination, flags, 0, &modelKey, 0xFF)`, pops 28, returns a borrowed
automaton in EAX or null (`0x0046999E`–`0x004699F3`). The traffic-manager slot body is
`0x0054DBF0`, **not** the similarly named `CreateDispatchAutomaton` at `0x0054F8A0`, which
has a different signature. Slot 18 creates the automaton internally, adds it to the manager,
routes through slot 17, releases its creation reference, and on route failure removes it
through manager slot 8 `(automaton,true)` and returns null (`0x0054DC79`–`0x0054DCBF`).

On non-null return, BeginDispatch AddRefs it, calls automaton slot 39 `(manager+4)` to add
the dispatch manager's message-target callback, stores the vehicle in the record, then calls
slots 28 `(dispatchID)`, 24 `(low16(callID))`, 25 `(3)` in order. It subsequently tracks its
occupant, stores the handle and updates pylon call/state if present. Re-targeting instead
calls vehicle slot 25 `(3)`, manager slot 17
`(vehicle,&source,&destination,flags,2)` (pop 20, return ignored), then vehicle slot 39
`(manager+4)`. It does not repeat the creation AddRef, dispatch/call-ID setters or view tracking
(`0x00469AF2`–`0x00469B40`). **On re-targeting only**, source is `(tag 0, null occupant)`,
not the record's station; `0x00469B20`–`0x00469B24` zero both source descriptor dwords.
Both paths update the pylon's state from vehicle slot 26 and
post begin only after those calls. Source absence, unavailable manager or null creation returns
AL false; the enclosing Dispatch revokes its already-appended record through the existing
§4.4 end path. There is no separate BeginDispatch station rollback before that revoke.

DP12 remains open for the internal station-to-road spawn position/heading and complete
slot-17 route failure/ownership contract. The descriptor/caller contracts above do not
authorize treating zero modelKey as a world position or guessing that missing routing rule.

#### 4.2.1 Native traffic creation and routing protocol (DP12)

This is the actual owner chain, not a second dispatch-manager forwarder. Traffic manager
primary slot 18 is `0x0054DBF0`, slot 17 is `0x00551730`; exported data references at
`0x00A9A4B4`/`0x00A9A4B8` establish their numbering relative to `0x00A9A470`.
The other two tables containing this pair use the same bodies (`0x00A9BDCC`/`0x00A9BDD0`,
`0x00A9D224`/`0x00A9D228`). All pointers below are borrowed unless an acquisition is stated.

**Creation wrapper.** Slot 18's fifth argument is a route-entry minimum (low byte), not
a position, heading or priority; its seventh is creation priority (low byte). It first copies
the three model-key dwords. If instance (third dword) is zero, it calls manager slot 20
`(source descriptor, out rect4i)`. Only when that returns AL true does it call slot 25
`(group,0,rect.minX,rect.minZ)` to choose an exemplar, copying its key at exemplar `+0x10`
when non-null. The rect's max coordinates are not passed. Otherwise it keeps the supplied key.
Then call manager slot 46 `(group,flags,&localKey,exemplar-or-null,priority)`; EAX null
ends creation immediately (`0x0054DBF0`–`0x0054DC7D`). Slot 46 here is the base
CreateAutomatonInternal `0x00510690`, **not** CreateDispatchAutomaton `0x0054F8A0`.
It obtains an acquired automaton IID `0xA9B40F05` through GZCOM using the selected group
template's class ID, falling back to the manager's default class (`0x00510773`–`0x005107CD`).
The broader group/exemplar/model setup is automata.md §7.2; no station position or heading
is passed to slot 46, and creation here is not a spawn-at-station call.

On success, call manager slot 7 `(vehicle)`, then slot 17
`(vehicle,source,destination,flags,minimum)`, and **then** vehicle Release, regardless of
the routing result. Null routing result calls manager slot 8 `(vehicle,true)` and returns
null. This ordering releases the initial GZCOM reference before asking the manager to remove
its retained entry. Non-null result remains borrowed from the manager. If the manager's
byte `+0x4C` is false and the created vehicle's slot 31 `(0x80)` is false, call the returned
automaton's slot 9 `(false)` before returning it (`0x0054DC7F`–`0x0054DCF9`). This extra
activation call does not change the returned pointer. BeginDispatch subsequently acquires
its separate record reference, installs its callback, sets IDs/state, and tracks, as §4.2 says.

**Source geometry for the route.** Manager slot 20 (`0x00506F50`) accepts a tagged
descriptor and writes four signed dwords `(minX,minZ,maxX,maxZ)`; pops 8, AL success.
For tag 0 a null occupant fails. A building-class occupant (`GetType` = `0x278128A0`)
supplies slot 11 bounds, followed by city slot 33's lot-manager slot 6
`(minX,minZ,0)`; if it finds a lot, lot slot 14 overwrites the bounds. A different occupant
supplies slot 8 GetPosition, then city slot 95 `(x,z,&cellX,&cellZ)`; both max coordinates
equal their minimum. Tag 1 converts its x and z to the containing 16-unit cells and likewise
makes a point rect; it does not use Y. Tags 2 and 3 copy a cell pair or rect respectively.
Unknown tags fail. Unlike pylon parking (§4.1.2), this route conversion can consult the lot
manager (`0x00506F6D`–`0x00507021`, `0x0050702E`–`0x0050710E`).

**Five-argument route method.** Slot 17 returns a borrowed vehicle pointer on success,
null on failure, pops 20. A null vehicle fails immediately. Convert source through slot 20;
if that fails (including retarget's tag0/null), use the vehicle's current position from
automaton slot 7, convert x/z to cells, form a point rect, and reject out-of-city cells
using city slots 93/94. Failure converting the destination through slot 20 always fails.
Acquire a pathfinder through the pointer at manager `+0xA88`, slot 6 `(out pathfinder)`;
null output fails. Reject non-traffic automata: the native check uses automaton slot 5's
occupant type, accepting only `0xC7CC75F1` or `0x496E5783` (`0x00551730`–`0x00551885`,
`0x0054D150`–`0x0054D191`). The non-traffic rejection jumps straight to the early null
return, **without releasing** an already acquired non-null pathfinder *[quirk]*. After that
gate, every routing/placement exit releases it (`0x00551885`, `0x00551A21`, `0x00551A3A`).

The exact issued sequence is below. Rects are passed as four **by-value** dwords, not
pointers. The pathfinder method names are deliberately not invented from a public SDK.
There is no boolean check of any setter here (`0x00551887`–`0x00551958`).

| Receiver | Slot | Arguments / observation |
|---|---|---|
| vehicle | 30 | `(flags,true)` |
| pathfinder | 19 | four source-rect dwords |
| pathfinder | 13 | `(1 << (manager[+0xA90] & 31),true)` |
| pathfinder | 18 | `(0)` |
| pathfinder | 5 | `(1)`, then `(2)`, then `(16)` in separate calls |
| pathfinder | 9 | `(3)` |
| pathfinder | 8 | address of the vehicle's route deque, vehicle `+0x22C` |
| pathfinder | 20 | four destination-rect dwords |
| vehicle | 30 | `(4,false)` |
| pathfinder | 23 | no arguments; EAX must equal **1**, not merely nonzero |

If the result is 1, compare route deque size **unsigned** against the zero-extended low byte
of the fifth argument (`0x00551964`–`0x00551972`, deque-distance body `0x00431DB0`).
Too few entries fails. New creation uses minimum 0; retarget uses 2. Only then call vehicle
slot 142 `(false)` BeginRoute; AL false fails. No rollback restores earlier flag/deque changes
on any routing failure. On successful placement, set flag `0x40`, clear flags `0x2000`,
`0x100`, `0x200`, `0x20` in that order through slot 30, then slot 160 ReturnToDefaultSpeed.
Finally manager slot 19 (`0x00507120`) converts the source descriptor to a world vec3.
For tag0/non-null it uses occupant slot 8 GetPosition; for tag1 it copies the three floats.
On conversion success, vehicle slot 80 receives `(world source vec3&, source occupant)`;
the latter is descriptor `+4` only for tag0, otherwise null. Conversion failure skips slot 80
but still returns the placed vehicle. Thus retarget's tag0/null preserves the vehicle's
previous return source (`0x00551978`–`0x00551A4E`).

**Road placement and heading.** BeginRoute at `0x00554AB0` (vehicle slot 142, pops 4,
AL success) obtains the traffic-network map from city slot 40. Zero route entries, from
vehicle slot 174, fail. Otherwise clear lookahead, call slot 90 `(true)`, clear flag `0x2000`.
A true input would return here without road placement; dispatch passes false. Save and pop
the first three-byte route entry `(u8 cellX,u8 cellZ,u8 extra)`. With a following entry, choose
direction 1 for negative ΔZ, 3 for positive ΔZ, otherwise 0 for negative ΔX and 2 otherwise;
with no following entry use 255. Fetch the cell's network list with traffic-network slot 14
`(cellX,cellZ)`, walk links `+0x54` in order, skipping null occupants or null occupant slot 49
path info. Ask manager slot 68 `(vehicle)` for a path type, then path-info slot 9
`(pathType,255,direction,&randomSuper[0x00B21D78],&outPathIndex,0)`, then path-info slot 3
`(returned path ID)`. First non-null path wins. Exhaustion returns false, after the deque
has already been popped (`0x00554AB0`–`0x00554C25`).

For the winner, vehicle slot 132 `(network occupant,pathIndex,path pointer,true)` attaches
it. Heading comes from the last path point minus the first path point in **three dimensions**,
normalized when length exceeds float32 0.00001 (bits `0x3727C5AC`, `0x00A9BCD8`);
otherwise `(0,0,1)`.
Y difference is rounded to float32 before the squared sum; X/Z differences remain in x87;
each normalized output is stored float32 (`0x00554C3B`–`0x00554CB7`). If no route remains,
path-info slot 22 `(path,&world position)` chooses position and flag 4 is set; otherwise use
the path's first point. Call vehicle slot 16 `(position vec3&,heading vec3&)`, then AL true
(`0x00554CB7`–`0x00554D13`). It is not the station/pylon diagonal, a random road heading,
or a separately invented station-spawn SDK function.

The selector argument at `0x00B21D78` is the shared **cRZRandomSuper object**, not a vec3.
Its initializer constructs it using `cRZTime`'s first dword XOR `0x4C09C170`
(`0x00A727C0`–`0x00A727DC`); e.g. `0x005057AB` and `0x0050CFF9` use the same object
for integer/double uniform draws. Pass it as a borrowed mutable generator; do not copy it,
seed a per-dispatch generator, or treat its storage as coordinates. The matching path
choice remains the path-info provider's operation, with its exact inputs above.

For the point-descriptor conversion, the native cell operation is precise: retain
`q = float32Coordinate * float32(1/16)` in x87; store
`b = float32((q - float32(0.5)) + float32(12582912))`; let candidate be the signed32
result of wrapping `bits32(b) - 0x4B400000`. If q equals signed32(candidate+1), increment
candidate (ordered equality only); otherwise keep candidate. Apply independently to X/Z.
For ordinary city coordinates this is floor(coordinate/16), including exact cell edges,
but the bit algorithm defines the operation outside that range too; do not use truncation
toward zero. Slot-17's current-vehicle fallback issues the same operation before its city
range checks (`0x0050702E`–`0x005070B7`, `0x00551780`–`0x00551838`). Float32 constants
are respectively bits `0x3D800000`, `0x3F000000`, `0x4B400000` (file-backed values at
`0x00A952F0`, `0x00A84D2C`, `0x00A952F4`; the traffic copy uses `0x00A9BCF4`/`0x00A9BCF8`).
No intermediate float32 store rounds q or q−0.5. Heading similarly sums squared Z and
stored Y differences before squared X in x87, takes sqrt, uses x87 reciprocal 1.0/length,
and rounds each output component only on store (`0x00554C3B`–`0x00554C99`).

DP12 is answered by §4.2 and this native owner chain. It defines the issued calls and
provider boundaries; an implementer must still provide the real automata/path/network
objects. It does not authorize forwarding to the original dispatch manager or substituting
a successful ID without a vehicle. DP10's rendering completion remains separate.

The enclosing Dispatch sequence is:

1. `source := GetDispatchSource(type)` (the first registered source whose `CanHandleDispatchType(type)` is true).
   If none, or `source.CanDispatch(x, z, &allBusy)` is false (protection.md §8.2): return **0**.
2. `pylon := activePylon`. If null and `allBusy`: return 0. If non-null and a record already holds this pylon
   (the player dragged a unit that is out): set that record's target x and z and its y := terrain altitude at
   (x, z), and go to step 5 with that record.
3. New record, all zero: `type`; `ID := ++nextID` (consumed even if the dispatch then fails); target `(x,
   altitude(x, z), z)`.
4. `InitDispatch(record)`; if false, return 0 (the record is discarded). Otherwise append it to the list.
   - Without a pylon: `station := source.DispatchFromNearestStation(x, z, &record.callID)`; null → false. Hold
     the station. If `type = toolType`, the available pylon whose call number equals the call ID is taken out of
     the list and becomes the record's pylon. Otherwise a new pylon is created, initialised and given the station
     as its source, **but not stored** — the record's pylon stays null and the new pylon is leaked *[quirk]*.
   - With a pylon: `station := pylon's source occupant` (held); the record holds the pylon; `callID := pylon's
     call number` (low 16 bits); `source.DispatchFromStation(station, &callID)` (result ignored); the pylon leaves
     the available list.
   - If the record has a pylon: move it to the target and set its dispatch type. Return true.
5. `BeginDispatch(record)` (`0x004698F0`): find the automata manager for the source's vehicle group
   (`GetDispatchAutomataProperties`, protection.md §8.2), create the vehicle (or re-target the existing one), set
   its dispatch ID, call ID and state 3, track it in the vehicle view, and post **begin**. On success return the
   record's ID; on failure `RevokeDispatchByID(ID)` and return 0.
   - A **new** vehicle is tracked with kind 0 when its occupant group is `0x4202` or `0x4303`, else kind 1, and the
     handle is stored at record `+0x0C`.
   - **Re-targeting** (DP9): with a record that already has a vehicle (step 2), the vehicle gets
     `SetDispatchState(3)` (automaton slot 25), a new path from the automata manager (slot 17, mode 2) and its event
     target again (slot 39). It is **not** tracked again: the record keeps its vehicle-view handle. The pylon's
     dispatch state is then set from automaton slot 26, and **begin** is posted again with the **same** ID.

### 4.3 `UpdateDispatchState(record, state)`

If `(state = 5) ≠ record.active`: `record.active := (state = 5)` and post **active** or **inactive**. Then, if the
record has a pylon, set the pylon's dispatch state.

**The dispatch states** (answers DP3). The vehicle automata set their own state through `SetDispatchState`
(automaton slot 25; `GetDispatchState` is slot 26, `0x00505020`, the field `+0x2CC`). The emergency vehicle's
`SetDispatchState` (`0x0052FF60`; the fire truck `0x005336C0` and patrol car `0x00543F60` call it first, the
helicopter has its own, `0x0053A360`) ignores a call that does not change the state, or while automaton slot 57
with `0x10` is non-zero; otherwise it stores the state and sends the manager event 7 with it (§3). Who sets what:

| State | Meaning | Set at |
|---|---|---|
| 0 | done, nothing more to report: the manager revokes the record at once (§3) | helicopter `0x0053C454` (instead of 6 when it has nowhere to return to *[inferred]*); airplane `0x005047B3`. **Ground vehicles never set 0** |
| 1 | the route is blocked: the vehicle reached the end of its path but not the target | arrival handler (emergency vehicle slot 79, `0x0052F210`) at `0x0052F2B2`; `SetDispatchState(1)` also sends the path-blocked message |
| 2 | finished at the scene | fire truck `0x00533679` (handling tick, the fire is out) and `0x0053307D` (slot 199); patrol car `0x00543C46` (slot 197) and `0x00543F5A` (slot 199); helicopter `HandleExitState` `0x00535518` |
| 3 | dispatched, on the way | `BeginDispatch` (`0x00469A30`, `0x00469AF2`); a successful re-path (`DoRepathingTick` `0x0052FEDA`); helicopter `0x0053C3A2`; airplane `0x005045D9`, `0x00504654` |
| 4 | (airplane only) | airplane `0x00504621` |
| 5 | working at the scene: the only state with `active` set | arrival handler at `0x0052F2A9` (the vehicle is at the target, slot 146); fire truck handling tick `0x00533644`; helicopter `0x0053C321`; airplane `0x00504729` |
| 6 | going home | "go home" (slot 29, `0x0052F330`, at `0x0052F353`; the DisasterPeriodEnded handler calls it, §3); the after-work step (slot 198, `0x0052F570`, at `0x0052F66A`); helicopter `0x0053C465`; airplane `0x00504796` |

The after-work step (slot 198) does nothing while the vehicle's tween value is above 0.6 or automaton slot 85 is
true; then a vehicle without a home station (`+0x2C4`), or nearer to it than a tuning distance (the byte `0x00B21CB8` × 16, compared with the 3D distance, `0x0052F60E`), is
removed at once (automaton slot 13); otherwise it gets state 6 and a path home (automata manager slot 17, mode 2),
and is removed when that path cannot be made. A removed vehicle sends event 3, and **event
3 is what ends a ground vehicle's dispatch**: the record is revoked then (unit returned, pylon back, **end**).

A fire engine therefore reports 3 (begin) → 5 (at the fire, **active**) → 2 (fire out, **inactive**) → 6 (driving
home, or nothing when the station is near) → event 3 (removed: revoke, **end**). A blocked route gives 3 → 1 and the
record stays until the vehicle is removed or re-targeted. Only the change into and out of 5 posts anything.

### 4.4 Ending and revoking

- **`EndDispatch(record)`** (`0x0046ACE0`): `record.active := false` **without** posting inactive *[quirk]*;
  `source := GetDispatchSource(type)`, null → return false. `source.RevokeDispatchFromStation(station, callID)`
  (returns the unit, protection.md §8.2). If the record has a pylon: clear the active pylon if it is this one; if
  `type = toolType`, return the pylon to its station (below), else shut it down; release it. Untrack and release the
  vehicle; release the station; post **end**; return true.
  The order (`0x0046ACE0`): `active := 0`; the source (none → return false, nothing released);
  `RevokeDispatchFromStation`; the pylon (clear active, return or shut down, release); untrack the vehicle and
  release it (**no** despawn call); release the station; post **end**; return true.
- **`RevokeDispatchByID(id)`** (slot 7, `0x0046AEC0`): `EndDispatch` on the first record with that ID, then erase it
  whatever `EndDispatch` returned; AL = the `EndDispatch` result, or false when no record has the ID.
- **`RevokeDispatchByPylon(p)`** (slot 8, `0x0046AE70`): if a record holds `p` (record pylon at node `+0x30`, the
  first match), `RevokeDispatchByID(its ID)`; in every case, also when no record holds it, then
  `ReturnPylonToStation(GetDispatchSource(p.GetDispatchType()), p)`. With no source for that type the return does
  nothing (below). AL = the revoke result, false when no record (DP8).
- **`RevokeAllDispatches(type)`** (slot 9): revokes **every** record, whatever its type *[quirk]*; AL = 1.
- **Return a pylon to its station** (`ReturnPylonToStation(source, p)`, `0x0046AA20`, `ret 8`):
  - `p` or `source` null: nothing at all (no shutdown, no add).
  - `p` has no source occupant (slot 18 null): shut down and **released** (`0x0046ACBA`). That release drops a
    reference the caller (`EndDispatch`) releases again afterwards, so the original over-releases *[quirk]*
    (DP6). Unreachable in shipped play *[inferred]*: the tool gives every pylon its station as source, the
    pylon's `Shutdown` is what clears it, and a station's removal erases its records first. FreeCity4 shuts it down without the extra release *[choice]*.
  - Otherwise: `source` slot 7 (`+0x1C`) gives the station's unit state `(mask, base, count)`; if it succeeds, `p`
    is placed at its parking spot (byte-wrapped multiplier in §4.1.2), with dispatch state 0 and UI
    state 1. Whether or not that call succeeded, `p` is looked up in the available vector (`0x004696C0`) and, if
    absent, pushed at the back with a reference (`0x007EBD30`).
  - A record whose type is not `toolType` has its pylon shut down and released by `EndDispatch`; a following
    `RevokeDispatchByPylon` return then finds no source occupant and over-releases as above (edge case).

A revoke therefore never posts **inactive**: a dispatch revoked while active leaves its cone to the protection
simulators' next full rebuild (protection.md §8.3 assumes inactive comes first).

### 4.5 The dispatch tool *[compat for the slots; presentation for the layout]*

- **`BeginDispatchTool(type)`** (slot 16): if `type = toolType`, AL = 1. If there is no source for `type`, AL = 0.
  Otherwise `toolType := type`, clear the available pylons, and for each of the source's stations (`GetStations`)
  with unit state `(mask, base, count)`: lay the station's units out along its lot diagonal (a station with fewer
  than two units uses its own position), and create one pylon per **free** unit i (mask bit i set) with call number
  `base + i`, dispatch state 0, the tool's type, UI state 1, the station as source, at position `start + step ×
  (i + 1)`. AL = 1.
- **`EndDispatchTool(type)`** (slot 17): if `type = toolType`, clear the available pylons, `toolType := 8`, AL = 1;
  otherwise AL = 0.
- **`SetActivePylon(p)`** (slot 19): when it changes, the old pylon gets UI state 1 and is released; the new one is
  held and gets UI state 2.
- **`GetAvailablePylonCount(type)`** (slot 21): the number of free units (mask bits below `count`) summed over the
  source's stations; 0 without a source.
- The tool itself (`cSC4ViewInputControlDispatch`): opening calls `BeginDispatchTool`, closing `EndDispatchTool`;
  dropping the active pylon on its own station's lot calls `RevokeDispatchByPylon`, elsewhere `CanDispatch` then
  `Dispatch`; afterwards `SetActivePylon(null)`.

#### 4.5.1 Air station enumeration and unit state (DP13) *[compat]*

Both air sources use the object-start dispatch-source interface (IID `0xC9C00435`), not their
message-target or serializable subobjects. Fire's primary table is `0x00AB1008`; police's is
`0x00AB22D0` (constructor stores at `0x006ACE19`, `0x006DFF89`). Exported data references
at `0x00AB1020` / `0x00AB22E8` select **the same** slot-6 function, `0x006E0010`.

| Slot | Arguments, in order | Return and ownership |
|---|---|---|
| 6 `GetStations` | writable native occupant-list reference; callee pops 4 bytes | AL = copied count nonzero, **not** a count in EAX. Each copied station has one acquired reference. |
| 7 `GetDispatchesAtStation` | station occupant pointer, optional `u32* mask`, optional `u32* base`, optional `u32* count`; callee pops 16 bytes | AL success; no acquired reference. Fire `0x006AC710`, police `0x006DF770`. |

Slot 6 passes the source's list at object `+0x1C` to `CopyOccupantList` (`0x006DFC70`). It first
clears the destination's **nodes only**, without releasing their old element pointers
(`0x00413E60`); callers must balance any previously acquired elements before reusing the list.
It walks source order, calls each station's slot 1 AddRef **before** appending the pointer, and
returns whether at least one was copied (`0x006DFC90`–`0x006DFC97`, `0x006DFD10`–`0x006DFD17`,
`0x006E001E`–`0x006E0026`). The native list is a dword sentinel pointer; each node contains next,
previous and occupant at offsets 0, 4 and 8. It is not an output array or an arbitrary STL layout.
Null station entries and allocation failure have no recoverable path here.

Balance **one** Release per returned station reference. A list wrapper whose destructor releases
its elements can perform that balance itself; explicit releases followed by that same destructor
require clearing the nodes first. The earlier crash in the question was corrected as a caller
cleanup error and supplies no contrary air-source ownership evidence.

Slot 7 reads the station's property holder (occupant slot 6). It always reads count first, even if
all output pointers are null. It writes count if requested, reads and writes mask only if
requested, and finally reads and writes base only if requested. The property IDs are:

| Source | Count | Free mask | Base call number |
|---|---|---|---|
| Fire plane | `0x0C0AFDD7` | `0xCC0AFDDB` | `0x0C0AFF3F` |
| Police helicopter | `0xCC0B0705` | `0xEC0B0709` | `0xAC0B070E` |

On a failed property read AL = 0 immediately; earlier output writes remain and later ones are
untouched. On success the written mask is `rawMask & ((1u << (count & 31)) - 1u)`, with 32-bit
shift/subtract semantics: count 32 yields mask zero, count 33 retains bit 0. This is verified by
the CL shift and dword decrement/AND at `0x006AC775`–`0x006AC78B` and
`0x006DF7D5`–`0x006DF7EB`. No station/source relationship validation or reference operation
occurs in this getter. See protection.md §8.4 for the matching air-source contract.

### 4.6 Queries

`CanDispatch(type, x, z)` (slot 5): the source's `CanDispatch`, or false without a source. `GetDispatchInfo(id,
&info)` (slot 10): docs/abi/layouts.md (the SDK names the first parameter `type`; it is the **ID**).
`GetDispatchCount(type)` (slot 11): records of that type. `GetDispatchIDArray(type, buf, &n)` (slot 12): writes the
IDs of that type in list order and returns the number written in EAX. It stops when that number equals `n`; the
comparison follows each write, so `n = 0` means no limit (docs/abi/allocators.md, *Dispatch ID array*). `n` is read,
never written. `GetAutomatonFromPylon(p)` (slot 20): the vehicle of the record that holds
`p` (borrowed), or null. `GetDispatchSource(type)` (slot 15): borrowed, may be null.

### 4.7 Worked example

Nothing dispatched, a fire station with 3 units (base 1). The player opens the fire tool: three pylons, call
numbers 1–3. They drop pylon 2 on a burning building: `CanDispatch(1, x, z)` is true; `Dispatch(1, x, z)` → ID 1
(`nextID` 0 → 1); the record takes pylon 2, `DispatchFromStation(station, 2)` clears unit 2's bit, a fire engine
automaton is created; **begin** (1, 1) is posted. On arrival the automaton reports state 5: **active** (1, 1);
the protection simulator adds the dispatch cone. When the fire is out it reports state 2: **inactive** (1, 1); then
state 6 while it drives home (nothing posted). When it is removed at the station it sends event 3:
`RevokeDispatchByID(1)`: unit 2 returns, pylon 2 goes back to the station (the tool is still open), **end**
(1, 1). Saved now, the record is `02 00 | 00000000 | 01000000 | 00000000 | 03000000 + three pylon refs | 01000000`
(tool still open, type 1).

## 5. Random numbers

None.

## 6. Tuning

None of its own. Unit counts, call-number bases and the automata groups come from the dispatch sources
(protection.md §8).

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x4990BD93`) *[compat]*

| Field | Encoding |
|---|---|
| version | u16, **2** |
| record count n | u32 |
| n records | each: `type` s32; `ID` u32; `callID` u32; `active` u8 (0/1); target x, y, z float32 ×3; station ref (IID `0x26BB767E`); automaton ref (`0xA9B40F05`); pylon ref (`0xC9B6D7D6`) — refs per foundations §2.7 (4 bytes when null, else 8). The vehicle-view handle is **not** saved |
| `nextID` | u32 |
| active pylon | ref (`0xC9B6D7D6`) |
| available pylon count m | u32 |
| m pylons | refs (`0xC9B6D7D6`) |
| `toolType` | u32 |

`Write` returns `GetError() = 0` (EAX 0/1). `Read` sets `loaded` first; a version other than 2 returns false; the
records are **appended** to the current list (it is not cleared), the pylon refs pushed onto the current vector;
returns `GetError() = 0`. `Init` then resets `toolType` to 8 and drops the active and available pylons (§4.1), so
the saved tool state has no effect.

**Corpus** (421 saves): every record has no dispatch and no pylon, so it is 22 bytes:
`02 00 | 00 00 00 00 | nextID | 00 00 00 00 | 00 00 00 00 | toolType`.

| `nextID` | `toolType` | Saves |
|---|---|---|
| 0 | **4** | 409 (the bundled regions' cities) |
| 0 | 8 | 8 (Berlin Spandau and Tegel, FC4 Test Spandau and Tegel, London Fulham and Kensington, Big City and Rush Hour tutorials) |
| 1 | 8 | 2 (Berlin Konradshohe; Making Money Tutorial): one dispatch was made and ended |
| 0 | 5 | 1 (Getting Started Tutorial) |
| 0 | 7 | 1 (New City (32)) |

`toolType` 4, 5 and 7 cannot be produced by the shipped tool's "none" (8) unless the tool was open; the 409 saves
with 4 were presumably written by an earlier build whose "none" was 4 *[inferred]*. All values load identically.
This build cannot write 4 (DP2): apart from `Read`, the field is written only with 8 (`Init` `0x0046A116`,
`EndDispatchTool` `0x0046A4F6`, `0x0046A56B`) and with the tool's type in `BeginDispatchTool` (`0x0046B0AA`), which
first requires a source for that type (0, 1, 5 or 6).

## 8. Interface slot table *[compat]*

`cISC4DispatchManager` (vtable `0x00A88108`).

| Slot | Offset | Method | Original | Args | Pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x004695B0` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x004F7BA0` | 0 | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x00469760` | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init` | `0x0046A050` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown` | `0x0046A1A0` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `CanDispatch(type, float x, float z)` | `0x00469680` | 3 | 12 | AL |
| 6 | `+0x18` | `Dispatch(type, float x, float z)` → ID | `0x0046A630` | 3 | 12 | EAX (0 = failed) |
| 7 | `+0x1C` | `RevokeDispatchByID(id)` | `0x0046AEC0` | 1 | 4 | AL |
| 8 | `+0x20` | `RevokeDispatchByPylon(pylon)` | `0x0046AE70` | 1 | 4 | AL |
| 9 | `+0x24` | `RevokeAllDispatches(type)` — **SDK error**: the SDK declares no argument | `0x00469D10` | 1 | 4 | AL (1) |
| 10 | `+0x28` | `GetDispatchInfo(id, DispatchInfo&)` | `0x00469D40` | 2 | 8 | AL |
| 11 | `+0x2C` | `GetDispatchCount(type)` | `0x00469D80` | 1 | 4 | EAX |
| 12 | `+0x30` | `GetDispatchIDArray(type, u32* buf, u32& n)` | `0x00469DA0` | 3 | 12 | EAX (number written) |
| 13 | `+0x34` | `RegisterDispatchSource(src)` (holds a reference; no duplicate check) | `0x0046AE00` | 1 | 4 | none |
| 14 | `+0x38` | `UnregisterDispatchSource(src)` (first match; releases) | `0x0046A5D0` | 1 | 4 | none |
| 15 | `+0x3C` | `GetDispatchSource(type)` | `0x004698B0` | 1 | 4 | EAX (borrowed) |
| 16 | `+0x40` | `BeginDispatchTool(type)` | `0x0046B060` | 1 | 4 | AL |
| 17 | `+0x44` | `EndDispatchTool(type)` | `0x0046A4E0` | 1 | 4 | AL |
| 18 | `+0x48` | `GetActivePylon()` | `0x004ABAD0` (shared getter of `+0x2C`) | 0 | 0 | EAX (borrowed) |
| 19 | `+0x4C` | `SetActivePylon(pylon)` | `0x00469630` | 1 | 4 | none |
| 20 | `+0x50` | `GetAutomatonFromPylon(pylon)` | `0x00469DE0` | 1 | 4 | EAX (borrowed, 0 if none) |
| 21 | `+0x54` | `GetAvailablePylonCount(type)` | `0x0046AF10` | 1 | 4 | EAX |
| 22 | `+0x58` | `GetDispatchVehicleView()` | `0x0060FB80` (shared getter of `+0x28`) | 0 | 0 | EAX (borrowed) |
| 23 | `+0x5C` | deleting destructor `(flags)` (not in the SDK) | `0x0046A7D0` | 1 | 4 | EAX |

## 9. Who reaches in

| Caller | Slots |
|---|---|
| fire and police simulators (protection.md) | 13/14 at `Init`/`Shutdown`; 10 on the active/inactive/end messages; 11 inside their `CanDispatch` |
| fire-plane and police-helicopter sources (protection.md §8.4) | 13/14, 10 |
| the dispatch tool `cSC4ViewInputControlDispatch` (`0x004BA1B0`, `0x004BA510`) | 5, 6, 8, 16, 17, 18, 19 |
| MySims, the My Street sim, the player-drive control, the city situation manager, the automata manager | 22 (track and untrack vehicles in the vehicle view) |

## 10. Quirks (reproduced)

1. A station removed while it has dispatches drops them silently: no end message, no unit returned (§3).
2. `EndDispatch` clears `active` without posting inactive (§4.4).
3. `RevokeAllDispatches(type)` ignores `type` (§4.4).
4. `InitDispatch` without a pylon and outside the open tool creates a pylon it never stores (§4.2).
5. `nextID` advances even when the dispatch fails.
6. `Read` appends to the existing list and sets `loaded` before checking the version (§7).
7. `GetDispatchIDArray` with `n = 0` writes every matching ID: the limit is checked only after a write.

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (DP1–DP3), and the cores12a questions DP4–DP9 filed in the
same section; all answered (DP3 by re-answers19, §4.3).

## 12. Conformance tests

DP13 independent vectors (specified, not claimed executed):

- Empty air source into an empty native list: AL false, no element AddRef. Two source stations A, B:
  output A then B, one AddRef each, AL true. Releasing each once and clearing nodes leaves net zero.
- Destination initially contains an acquired X: slot 6 discards its node without releasing X;
  this distinguishes node replacement from reference-owning list assignment.
- Fire count 3, raw mask `0xFFFFFFFF`, base 17: outputs `(7,17,3)`; police uses its own three IDs.
- Count 32 / 33 with raw mask all ones gives mask 0 / 1. Do not substitute a saturating mask.
- Missing count leaves all sentinel outputs unchanged; missing mask writes count only; missing base
  writes count and masked mask. With null mask output, a missing mask property does not cause failure.

DP11 independent vectors (specified, not claimed executed):

- A=(0,0,0), B=(40,0,0), count 3, mask 5: pylons at x=10 and 30, with calls base and base+2.
  With count 1 and station position x=7, BeginTool uses x=7; return uses diagonal midpoint x=20.
- Call 1, base 257: return multiplier 1, not −255. Source replacement A→B acquires B before releasing A;
  assigning B again performs neither reference operation.
- Drop at the minimum X/Z corner revokes; at either maximum edge it does not. Vary height alone and
  keep x/z inside: still revokes. Bounds come from the source occupant, even if a lot lookup would differ.
- Query pylon IID and AsOccupant: acquired primary pointer and borrowed pointer+8 respectively;
  the occupant's slot 9 receives the three-float position. A shifted SDK prefix must fail this vector.
- Supply a string interface reporting length 3 and bytes A,NUL,B. Observe source slots 7 then 6,
  a copied three-byte destination, and no source AddRef/Release; mutate the source afterward.
  Passing the character buffer itself cannot satisfy this contract. Empty string still makes both calls.
- Set call number 0x12345678: GetCallNumber exposes AX=0x5678; do not compare upper EAX.
  Set drag byte 2: GetDragState exposes AL=2 and does not mark dirty. These are narrow channels.

DP10/DP12 boundary vectors (specified, not claimed executed; remaining open contracts still apply):

- Start a view, track two different occupants with an unchanged global counter between calls:
  two consecutive nonzero handles, one acquired occupant reference each. A second manager must
  not reset the counter. A null occupant returns zero without advancing it. Neither dispatch ID
  nor a serialized record seeds the counter. Do not derive expected handles from candidate output.
- QI message target returns primary+12; QI unknown returns primary+4; unsupported IID clears out
  and returns false. Neither equals manager slot 22's borrowed primary pointer.
- With a source returning group G, key K, flags F, capture manager slot 18 arguments independently:
  G, source tag0/station, destination tag1/terrain-adjusted target, F, 0, K, 255. Retarget captures
  slot 17 with source tag0/**null**, mode2, then callback; it leaves the view handle unchanged.
- Return AL false from source slot 11 after writing the outputs: creation still uses those outputs.
  Return null from creation: no vehicle AddRef/callback/tracking/begin; enclosing revoke performs end.
- Native creation wrapper: supply model instance zero and a source rect with different min/max.
  Capture exemplar slot 25 receiving minX/minZ only; change the instance to nonzero and observe no
  source-rect/exemplar call before slot 46. A failed source conversion still attempts creation.
- Supply pathfinder result 2, with a nonempty route: routing fails (only 1 succeeds). With result 1
  and one entry, minimum 0 permits BeginRoute; minimum 2 rejects it. Minimum 258 is narrowed to 2.
  On wrapper failure observe vehicle Release before manager slot 8 `(vehicle,true)`; no tracking.
- Retarget with tag0/null: source rect comes from current vehicle position; destination conversion
  is mandatory; successful routing skips return-source setter slot 80. Keep an independent original
  return source and verify it survives. Do not replace null with the station pointer.
- Give two network occupants in the first route cell: first has no matching path, second does.
  Choose the second, derive heading from its path endpoints, and pop the first route entry once.
  With no match, fail after the pop; do not silently restore the deque or choose a nearby road.
- Point descriptors x=−0.01, 0, 15.99, 16 map to cells −1, 0, 0, 1 respectively; fallback source
  rejects the negative cell. Source tag0/non-null uses occupant/lot bounds instead. A degenerate
  chosen path gives heading (0,0,1); a path from (0,0,0) to (0,3,4) gives (0,0.6,0.8), with
  the specified float-store order. Capture the shared RNG pointer passed to the path selector.
- Non-traffic automaton rejection after pathfinder acquisition returns null without Release;
  a traffic automaton whose pathfinder result is 2 releases it once. These paths distinguish
  the native exceptional leak from ordinary failed-routing cleanup.
- Track a kind-0 occupant and request delayed removal before any eligible Update: it remains held
  and counted. A context gate can postpone completion; an eligible update below fade 0.01 removes
  it with texture/model release before the retained occupant Release. Refresh leaves the handle intact.

1. Empty record: a fresh manager writes `02 00 00000000 00000000 00000000 00000000 08000000`.
2. Round trip of the 22-byte corpus forms, including `toolType` 4; after `Init`, `toolType` is 8.
3. `Read` of version 1 or 3 fails; `loaded` is still set.
4. `Dispatch` with no registered source returns 0 and `nextID` is unchanged; with a source whose `CanDispatch`
   is true and a free unit, returns `nextID + 1`, appends one record and posts begin once.
5. A failed `InitDispatch` (no station found) still advances `nextID`.
6. Automaton event 7 with state 5 posts active once; a second state 5 posts nothing; state 0 posts inactive, then
   end, and removes the record.
6a. Event 7 with state 2 after state 5 posts inactive only; the record stays. A later event 3 posts end and removes
   it. States 1, 3, 4 and 6 post nothing unless the record was active.
7. `RevokeDispatchByID` of an active record posts end only (no inactive) and returns the unit
   (`RevokeDispatchFromStation` called once).
8. `RevokeAllDispatches(1)` also removes type-0 records.
9. RemoveOccupant of a station with two records removes both, posts nothing and never calls
   `RevokeDispatchFromStation`.
10. `BeginDispatchTool(1)` with one station of 3 units, mask `0b101`: two pylons (call numbers base and base + 2);
    `GetAvailablePylonCount(1)` = 2; `EndDispatchTool(0)` → false, `EndDispatchTool(1)` → true and the list is empty.
