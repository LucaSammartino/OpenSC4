# Non-virtual game functions called by address

Interface facts (see [README.md](README.md)). Some original behaviour can only be reached through non-virtual
functions of the executable, which no interface exposes. A replacement that needs it calls the function at its
address, behind a check that the executable is SimCity 4 Deluxe 1.1.641 (GOG). This file gives what such a call
needs: the calling convention, the arguments, the bytes the callee pops, the return channel and exactly which
`this` pointer the function expects. No logic is recorded here.

All of these functions are `__thiscall`: `this` in ECX, the arguments on the stack, popped by the callee. A
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
