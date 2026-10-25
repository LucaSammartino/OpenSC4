# Small framework owners 3: record copies, small constructors, type IDs, allocator helpers (behaviour spec)

Scope: 24 small owners as tools/coverage.py assigns them, most of one to five functions: the drag data, the ferry route
info, the MySim template, the sim-grid row constructors, the building profile, the conditional building status, the
residential query data, `cRZRefCount`, `cGZCOMLibrary`, `cCompletionObj`, `cGZKeyMessage`, `SignalPlatformData`,
`cGZAny`, `cRZFastSTLAllocator`, `ibasic_string`, `cGZCommandServer::CommandInfo`, `RZFileFindData`,
`FileChangeNotification`, `nGZLog`, `cRZCellMap`, `cRZLock`, `cIGZMessageTarget2`, `cIGZWinProc` and
`cIGZSystemService`. Many are folded or template bodies whose owner name is the first class that used them; each row
says what the body does.

Related: widget-dragdrop.md §1 (the drag data layout), mysims.md §4.12 (MySim templates), demand.md (building profiles),
zones.md (cell maps), framework-misc2.md (the reference-counting bases, the pooled messages, the locks), foundations.md
§9 (variants), framework-rz-files.md (file finding, change notifications), framework-gz-services.md (thread signals).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile and the small ones in the disassembly. Written
2026-10-24 (re-fwmisc7c).

## 1. Record copies and constructors *[compat]*

| Owner | Address | Behaviour |
|---|---|---|
| `cIGZWinDragAndDropManager::DragData` | `0x009CC3DD` | constructor: type `+0` 0, an empty variant `+4`, four buffer references (`+0x18`, `+0x24`, `+0x30`, `+0x34`) null, the other dwords 0, byte `+0x3C` 0, `+0x40` = `0xAC0C7F4A` (widget-dragdrop.md §1 layout) |
| | `0x009CC53A` | destructor: releases the four buffers (in reverse order), then the variant |
| | `0x009CC576` | assignment: every field copied, the variant by value, the buffers with reference counting |
| `cISC4TrafficSimulator::FerryRouteInfo` | `0x0056D7F0` | assignment: two counted references (`+0`, `+4`), the scalar fields `+8`…`+0x18`, then two byte vectors copied |
| `cISC4MySimPool::tMySimTemplate` | `0x008B5E70` | constructor: two empty strings (`+4`, `+0x18`), `+0x14` and `+0x28` 0 (mysims.md §4.12) |
| | `0x0043B180` | destroys a range of 48-byte templates (both strings each) |
| `cISC4SimGrid<u8>`, `<s16>` | `0x004610E0`, `0x004623B0` | the grids' row-vector resize with an empty row (the rows are emptied) |
| `cISC4BuildingOccupant::BuildingProfile` | `0x004D1230` | copy constructor (demand.md "Copied maps"): the scalar header, the vector of maps, then the remaining fields |
| `cISC4CivicBuildingSimulator::ConditionalBuildingStatus` | `0x0069E3A0` | copy constructor: byte `+0`, `u16` `+2`, the string (`+8` of the source) into `+4`, `+0x14` = 0 |
| `cISC4ResidentialSimulator::tQueryData` | `0x00414210` | constructor: an empty string `+0`, `+0x28`, `+0x2C`, `+0x34` = 0 |
| `cGZCommandServer::CommandInfo` | `0x0092AA36` | destructor: the strings `+0x18` and `+4` |
| `RZFileFindData` | `0x0091ADEC` | destructor: the strings `+0x18` and `+0` |
| `SignalPlatformData` | `0x00932E47` | constructor: handle `+0` and `+4` 0, a name string `+8`, a critical section `+0x1C` (the thread signal's platform state, framework-gz-services.md) |
| `cGZKeyMessage` | `0x00989C10` | constructor of the pooled key message (framework-misc2.md §3.3): type `0x7A104750`, `+0xC` = −1, `+0x10`, `+0x14` = 0 |

## 2. Reference counting, interfaces and type IDs *[compat]*

| Owner | Address | Behaviour |
|---|---|---|
| `cRZRefCount` | `0x0090D957` | constructor: count `+4` = 0, vtable `0x00ACE624` (deleting destructor at slot 0) |
| | `0x0090D964` | destructor body (vtable only) |
| | `0x0090D96B` | `Release`: a count above 1 is decremented and returned; else slot 0 with 1 (delete) and 0 |
| | `0x00968B60` | `AddRef`: count + 1, returned |
| `cGZCOMLibrary` | `0x0087B319` | `QueryInterface`: IIDs `0x07D4922E` and 1 |
| `cCompletionObj` | `0x00968B3B` | `QueryInterface`: IIDs `0x1B3637C6` and 1 |
| `cIGZWinProc` | `0x0041D4C0` | the default `DoWinMsg`: false, five arguments |
| `cIGZSystemService` | `0x004EA510` | a thunk to the object's slot 5 |
| `nGZLog` | `0x00934732` | the log service's deleting-destructor thunk (face `+4`) |
| `cGZAny<T>::TypeID` | `0x009378CD`, `0x0093AAAF`, `0x0093FFCE`, `0x00940BBF`, `0x00940DA2` | the type IDs of the "any" value holders: `AnyUint32` `0xEB61E4F7`, `AnyFloat32` `0xABC78708`, `AnyString` `0x0B8BEA18`, `AnyBoolean` `0xCBA908E1`, `AnySint32` `0x0C264712` |

## 3. Allocation, locks, cells and files *[compat]*

| Owner | Address | Behaviour |
|---|---|---|
| `cIGZMessageTarget2` | `0x0090CF54` | the framework's general `operator new(size)`: the fast allocator at `0x00B62698` under its lock (framework-misc2.md §2.1) |
| `cRZFastSTLAllocator<cGZPersistResourceKey>` | `0x005061E0` | `construct(p, key)`: copies the 12-byte key when `p` is non-null |
| `ibasic_string<char>` | `0x0093415E` | constructs a `std::string` whose allocator is the allocator service's (framework-misc2.md §2.2) |
| `cRZLock` | `0x0048C9F0` | the scoped lock's constructor (named `~cRZLock` in the export): vtable `0x00A8ED58`, stores the lockable and calls its `Lock` (slot 1) |
| `cRZCellMap<bool>` | `0x00441AA0` | `Fill(value)`: every word of every row (`+4` rows of `+0xC` 32-cell words, row table `+0x10`) := 0, or `0xFFFFFFFF` for true (padding bits included) |
| | `0x00441A60` | destructor: frees each row, then the row table (zones.md) |
| `FileChangeNotification` | `0x005C0BD0` | `GetWatchedDirectory(out)`: assigns the watched directory to the string `out` (its slot 14) |

## 3a. Leftovers (re-comdirs7) *[compat]*

| Owner | Address | Behaviour |
|---|---|---|
| `cGZCOMDirector` | `0x0087B1C3`, `0x0087B1F6` | the framework's own COM director (ID `0xA3CAD79A`); without a GZCOM yet, the constructor takes the framework's (framework slot 23) |
| `fn:NULL_about` | `0x0090CDEA` | the Gimex "NULL" codec's about record (gimex.md): 256 zeroed bytes with the tag `NULL`, size `0x100`, version `0x159`, a flag word `0x1000`, `+0x18` = 100, and the strings `Colin Andrews` and `Not Gimex` among its text fields |
| `fn:SetTableNumber` | `0x0043D1F0` | on the table object in ESI (used by the MySim code): slot 33 with the index, slot 31 with the value as a double, slot 46 with −3 |
| `PieGraphSeriesInfo` | `0x009B750D` | constructor: `+0` = 0, colour `+4` = `0x80000000` (unset), an empty label |
| `nGZDrawSoftRender::cISpanRender` | `0x0082FF00` | the span renderers' shared deleting destructor (span-render-4444.md, span-render-1555.md) |

## 4. Function inventory

`cIGZWinDragAndDropManager::DragData`: `0x009CC3DD`, `0x009CC53A`, `0x009CC576`.
`cISC4TrafficSimulator::FerryRouteInfo`: `0x0056D7F0`. `cISC4MySimPool::tMySimTemplate`: `0x0043B180`, `0x008B5E70`.
`cISC4SimGrid`: `0x004610E0`, `0x004623B0`. `cISC4BuildingOccupant::BuildingProfile`: `0x004D1230`.
`cISC4CivicBuildingSimulator::ConditionalBuildingStatus`: `0x0069E3A0`. `cISC4ResidentialSimulator::tQueryData`:
`0x00414210`. `cRZRefCount`: `0x0090D957`, `0x0090D964`, `0x0090D96B`, `0x00968B60`. `cGZCOMLibrary`: `0x0087B319`.
`cCompletionObj`: `0x00968B3B`. `cGZKeyMessage`: `0x00989C10`. `SignalPlatformData`: `0x00932E47`.
`cGZAny`: `0x009378CD`, `0x0093AAAF`, `0x0093FFCE`, `0x00940BBF`, `0x00940DA2`. `cRZFastSTLAllocator`: `0x005061E0`.
`ibasic_string`: `0x0093415E`. `cGZCommandServer::CommandInfo`: `0x0092AA36`. `RZFileFindData`: `0x0091ADEC`.
`FileChangeNotification`: `0x005C0BD0`. `nGZLog`: `0x00934732`. `cRZCellMap`: `0x00441A60`, `0x00441AA0`.
`cRZLock`: `0x0048C9F0`. `cIGZMessageTarget2`: `0x0090CF54`. `cIGZWinProc`: `0x0041D4C0`. `cIGZSystemService`:
`0x004EA510`. `cGZCOMDirector`: `0x0087B1C3`, `0x0087B1F6`. `fn:NULL_about`: `0x0090CDEA`. `fn:SetTableNumber`:
`0x0043D1F0`. `PieGraphSeriesInfo`: `0x009B750D`. `nGZDrawSoftRender::cISpanRender`: `0x0082FF00`.
