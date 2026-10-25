# cSC4NetworkOccupant and its family (behaviour spec)

Status: network-occupants spec, written 2026-10-20 (re-netocc lane). Every fact was checked on the Windows
executable `bin/SimCity4.exe` (1.1.641, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`); every address is a Windows address. Numeric
details (float or double, operation order, widths) were checked against the instruction bytes. Implementers work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult
decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags: *[compat]* is required for compatibility with the original game and its plugins; *[choice]* is a behaviour we
chose where the original was arbitrary or undefined; *[inferred]* is consistent with all evidence but not proven;
*[quirk]* is surprising original behaviour that is kept on purpose.

`r24(e)` is `e` rounded to float32 (foundations §18). All field offsets are **from the object base**, which is the
class's primary interface pointer (`cISC4NetworkOccupant` for the network-piece classes, the class's own interface
for the others), unless a section says otherwise.

**Scope.** The in-world objects the network tools place: the *network occupant* family. The piece occupant
(`cSC4NetworkOccupant` and its subclasses) is the geometry of a network tile — the object that carries the piece's
model, its terrain-following triangle soup, its connection matrix and its network flags. Alongside it: the power
pole and power line occupants, the pipe and subway occupants, the underground connection occupants, and the
dispatch pylon. This spec gives the identity of every class, the object layout, the lifecycle (`Init` /
`Shutdown`), the interface slot tables with return channels, what each class places on the grid, and the save
record.

**What is not here:** the tools that decide *which* piece to build (network-tools.md, network-geometry.md), the
piece solve and height solver (network-geometry.md §8–§9), supports and bridges (network-geometry.md §10,
network-bridges.md), the network manager and lot manager (network-manager.md), the traffic rebuild that consumes
paths (traffic-network.md), the power and water simulations that consume poles, lines and pipes (power.md,
plumbing.md).

**Shared material cited and not repeated here:**

- [foundations.md](foundations.md) §2.3 (`gzstr`), §2.7 (references and the class store), §3.3, §3.5 (save/load
  order), §10.3 (exemplar property holders), §18 (x87 precision).
- [occupant-manager.md](occupant-manager.md) (insertion, the spatial index, the flags byte, threading).
- [lot-records.md](lot-records.md) §2.3 (the occupant base record), §14 (references and resolution on load).
- [network-manager.md](network-manager.md) §5 (the network flags table — this spec does not repeat it), §6.1
  (the `managed` list), §8.5–§8.6 (how the lot manager finds and decorates a piece).
- [network-geometry.md](network-geometry.md) §2.3/§2.4 (the network type info tables), §8.1 (corner heights),
  §10 (creation, supports, flags).
- [network-bridges.md](network-bridges.md) §11.2 (bridge piece creation), [traffic-network.md](traffic-network.md)
  §4.8 (tunnel portals), §7.2 (slots 48–54).
- [power.md](power.md) §5.1/§5.4 (pole and line consumers), [plumbing.md](plumbing.md) §5.2/§9 (pipe consumers),
  [dispatch.md](dispatch.md) §4.1 (pylon slots), [demolition.md](demolition.md) §4.2/§4.3 (demolition dispatch).

---

## 0. Summary

The network-piece family is rooted in `cSC4NetworkOccupant` (class ID `0xC9C05C6E`), which derives from
`cSC4Occupant` and adds the `cISC4NetworkOccupant` interface (81 slots in the shipped SDK order, extended by the
underground subclasses). On top of it sit the pre-built-model subclass (which owns a real exemplar model), the
bridge and tunnel pieces, and the three underground pieces (base, pipe, subway). The power pole, power line,
underground connection and pylon classes do **not** derive from `cSC4NetworkOccupant` — they derive directly from
`cSC4Occupant` and answer their own interfaces.

One object, many roles. A single road tile's occupant holds: the four corner points the tool solved (slot 3), the
cell's network flags (`+0x13C`), the 4 × 4 connection byte matrix (`+0x144`), the four road quads and the triangle
soup that draw the tile, the cached path info the traffic simulator walks (`+0x140`), and the demolition cost
(`+0x170`). The network lot manager finds lots *through* these objects' flags, not through a back-pointer
(network-manager.md §8.5); a network lot's `occupant` reference points at the piece that covers its cell.

Implementation size: about 2 500 lines for the whole family — the base class and its record (≈600), the prebuilt
subclass (≈500), the underground family (≈700), the pole and line pair (≈500), the connection occupants (≈150,
mostly shared) and the pylon (≈200) — plus the type-info/routines tables (§11, §12) which are data and one
function.

---

## 1. Identity and inheritance

### 1.1 The class table

| Class | Class ID | Occupant type (u32, `GetType` slot 7) | Primary interface (vtable) | Object notes |
|---|---|---|---|---|
| `cSC4Occupant` (base of all) | `0xA9BC9AB6` | `0x74758925` (bare occupant) | `cISC4Occupant` `0x00AA41D8` | ctor `0x005E9220` |
| `cSC4NetworkOccupant` | `0xC9C05C6E` | `0xC772BF98` (network piece) | `cISC4NetworkOccupant` `0x00AA8080` | ctor `0x0061BDD0` |
| `cSC4NetworkOccupantWithPreBuiltModel` | `0x49C1A034` | `0xC772BF98` | `cISC4NetworkOccupant` `0x00AA7038` | ctor `0x0061E1A0`; sets flag `0x400000` at `+0x13C` |
| `cSC4NetworkBridgeOccupant` | `0x49CC1BCD` | `0xC772BF98` | `cISC4NetworkOccupant` `0x00AA7300` | ctor via prebuilt `0x00605180` (`0x006051F0`'s callee) |
| `cSC4NetworkTunnelOccupant` | `0x8A4BD52B` | `0xC772BF98` | `cISC4NetworkOccupant` `0x00AA9308` | ctor `0x006476F0` |
| `cSC4UndergroundOccupant` | `0x4A10AFC2` | `0xC772BF98` (no override; §3.2) | `cISC4NetworkOccupant` `0x00AAAD40` | extended underground interface (106 slots) |
| `cSC4PipeOccupant` | `0x49C05B9F` | `0xC772BF98` | `cISC4NetworkOccupant` `0x00AA97B8` | 110 slots |
| `cSC4SubwayOccupant` | `0xCA16374F` | `0xC772BF98` | `cISC4NetworkOccupant` `0x00AAA3C8` | 112 slots |
| `cSC4UndergroundConnectionOccupant` | `0x8A139184` | none set by the base ctor (subclasses set it, §6.4) | `cISC4UndergroundConnectionOccupant` `0x00AAA840` (25 slots) | ctor `0x00654F00` |
| `cSC4PipeConnectionOccupant` | `0xEA0A5ACF` | `0x2A183167` (set by `Init`) | `0x00AA9578` (45 slots) | ctor `0x006483E0` |
| `cSC4SubwayConnectionOccupant` | `0xCA15ED6B` | `0x8A13918A` (set by `Init`) | `0x00AAA188` (45 slots) | ctor `0x00652C50` |
| `cSC4PowerPoleOccupant` | `0x09C05C6A` | `0x2890D4DE` | `cISC4PowerPoleOccupant` `0x00AA9EB0` (19 slots) | ctor `0x0064DB30`, object `0x124` bytes |
| `cSC4PowerLineOccupant` | `0xC9C05C5D` | `0x0990B2EA` | `cISC4PowerLineOccupant` `0x00AA9C80` (8 slots) | ctor `0x0064A270` |
| `cSC4PylonOccupant` | `0x49B6D69B` | `0xA9B6D7DE` | `cISC4PylonOccupant` `0x00AA5150` (21 slots) | ctor `0x005F1730` |

All eleven class IDs of §1.1 are registered in the exe's class-id table between `0x00B08D40` and `0x00B08E18`
(`re/class_id_table.tsv`): pylon `0x00B08D40`, bridge `0x00B08DC8`, plain `0x00B08DD8`, prebuilt `0x00B08DE8`,
tunnel `0x00B08DF0`, pipe `0x00B08DF8`, power line `0x00B08E00`, pole `0x00B08E08`, subway `0x00B08E10`,
underground `0x00B08E18`. The two connection classes (`0xEA0A5ACF`, `0xCA15ED6B`) and the connection *types* are
not in that table — they reach saves as class IDs on their records instead (save-records.md §3.2 Q5, answered:
the classes exist and are read from saves).

The occupant types are what `cISC4Occupant::GetType` (slot 7) returns and what the demolition dispatcher and the
simulators dispatch on. The plain network-piece classes all share `0xC772BF98`: they are told apart only by
`QueryInterface` (§1.2) — there is no per-subclass occupant type for bridge, tunnel, prebuilt, underground, pipe
or subway *[inferred from the absence of a type write in their constructors and from the consumers, which all QI]*.

The type constants live in the *type info* table `0x00AA7CE8` (13 dwords, one per network type index, §11) and in
`IsOfType` (§1.4).

### 1.2 Inheritance and `QueryInterface`

Inheritance, from the constructors' vtable writes (each row lists the object's sub-object pointers):

| Class | `+0x00` | `+0x04` | `+0x08` | `+0x0C` | other |
|---|---|---|---|---|---|
| `cSC4NetworkOccupant` | `cISC4NetworkOccupant` `0x00AA8080` | `cISC4Occupant` `0x00AA7FF8` | `cIGZSerializable` `0x00AA7FDC` | property holder `0x00AA7F78` | `cS3DModelInstanceBasic` at `+0x68` |
| `…WithPreBuiltModel` | `0x00AA7038` | `0x00AA6FB0` | `0x00AA6F94` | `0x00AA6F30` | as the base |
| `…BridgeOccupant` | `0x00AA7300` | `0x00AA7278` | `0x00AA725C` | `0x00AA71F8` | as the base |
| `…TunnelOccupant` | `0x00AA9308` | `0x00AA9280` | `0x00AA9264` | `0x00AA9200` | as the base |
| `cSC4UndergroundOccupant` | `cISC4NetworkOccupant` `0x00AAAD40` | `cISC4Occupant` `0x00AAACB8` | `cIGZSerializable` `0x00AAAC9C` | property holder `0x00AAAC38` | model instance at `+0x68` |
| `cSC4PipeOccupant` | `0x00AA97B8` | `0x00AA9730` | `0x00AA9714` | `0x00AA96B0` | as the base |
| `cSC4SubwayOccupant` | `0x00AAA3C8` | `0x00AAA340` | `0x00AAA324` | `0x00AAA2C0` | as the base |
| `cSC4UndergroundConnectionOccupant` | `cISC4UndergroundConnectionOccupant` `0x00AAA840` | `cIGZSerializable` `0x00AAA824` | property holder `0x00AAA7C0` | `cISC4Occupant`-shaped `0x00AAA7A8` at `+0x1C` | model instance at `+0x38` |
| `cSC4PipeConnectionOccupant` | `0x00AA9578` | `0x00AA955C` | `0x00AA94F8` | — | as the connection base |
| `cSC4SubwayConnectionOccupant` | `0x00AAA188` | `0x00AAA16C` | `0x00AAA108` | — | as the connection base |
| `cSC4PowerPoleOccupant` | `cISC4PowerPoleOccupant` `0x00AA9EB0` | `cIGZMessageTarget2` `0x00AA9EA0` | `cISC4Occupant` `0x00AA9E18` | `cIGZSerializable` `0x00AA9DFC` | property holder `+0x10` `0x00AA9D98`; 5-slot table `+0x24` `0x00AA9D80`; model instance `+0x3C` |
| `cSC4PowerLineOccupant` | `cISC4PowerLineOccupant` `0x00AA9C80` | `cISC4Occupant` `0x00AA9BF8` | `cIGZSerializable` `0x00AA9BDC` | property holder `0x00AA9B78` | |
| `cSC4PylonOccupant` | `cISC4PylonOccupant` `0x00AA5150` | `cIGZMessageTarget2` `0x00AA5140` | occupant view `0x00AA50B8` | `cIGZSerializable` `0x00AA509C` | property holder `+0x10` `0x00AA5038`; model instance `+0x3C` |

So the inheritance chain is: `cSC4NetworkOccupant` → `cSC4Occupant`; the prebuilt, bridge, tunnel and underground
classes derive from `cSC4NetworkOccupant`; pipe and subway derive from `cSC4UndergroundOccupant`; the connection
occupants derive **directly** from `cSC4Occupant` (their constructor calls `cSC4Occupant::cSC4Occupant`
(`0x005E9220`) only — there is no network-occupant base). The pole, line and pylon also derive directly from
`cSC4Occupant`.

`QueryInterface` per class (each is `__thiscall`, one IID and one `void**`, callee pops 8, returns `al`, and
`AddRef`s on success):

| Class (address) | IID → result |
|---|---|
| `cSC4NetworkOccupant` (`0x006192C0`) | `0xA821EF94` (`cISC4NetworkOccupant`) or `0xA9764C8C` (class ID) → `this`; else the `cSC4Occupant` QI (`0x005E89C0` at `+4`): IID 1 (`cIGZUnknown`) or `0x26BB767E` (`cISC4Occupant`) → `this + 4`; `0xE4FDA3D4` (`cIGZSerializable`) → `+8`; anything else → the property holder (`+0xC`, through `0x005FD310`); then the model instance at `+0x68` (`0x00801940`) |
| `…WithPreBuiltModel` (`0x0061D060`) | `0x49C1A034` → `this`; else the base QI |
| `…BridgeOccupant` (`0x00604BC0`) | `0x49CC1BCD` → `this`; else the prebuilt QI |
| `…TunnelOccupant` (`0x00647170`) | `0x8A4BD52B` → `this`; else the prebuilt QI |
| `cSC4UndergroundOccupant` (`0x0065A080`) | `0x4A10AFC2`, `0xA821EF94` or `0xA9764C8C` → `this`; else the base QI, then the model instance |
| `cSC4PipeOccupant` (`0x006484D0`) | `0x49C05B9F` or `0xA9764C8C` → `this`; else the underground QI |
| `cSC4SubwayOccupant` (`0x00652D00`) | `0xCA16374F` → `this`; else the underground QI |
| `cSC4UndergroundConnectionOccupant` (`0x00654030`) | `0x8A139184` or `0xA9764C8C` → `this`; else the `cSC4Occupant` QI, then the model instance (`+0x38`) |
| `cSC4PipeConnectionOccupant` (`0x00648080`) | `0xEA0A5ACF` → `this`; else the connection QI |
| `cSC4SubwayConnectionOccupant` (`0x00652AA0`) | `0xCA15ED6B` → `this`; else the connection QI; its slot 44 (`QueryOccupantInterface`, `0x00652BD0`) answers QI `0xCA16374F` then `0x4A10AFC2` — "is it an underground occupant" |
| `cSC4PowerPoleOccupant` (`0x0064A030`) | `0xA8C86142` (`cISC4PowerPoleOccupant`), `0x09C05C6A` or `0xA9764C8C` → `this` (null-`this` quirk: out = 0, returns 1); `0x452294AA` (`cIGZMessageTarget2`) → `+4`; `0x26A68448` → dispatches through the delegate at `+0xBC` (§7.5); else the `cSC4Occupant` QI (`+8`), then the model instance |
| `cSC4PowerLineOccupant` (`0x00649350`) | `0xC98F3B24`, `0xC9C05C5D` or `0xA9764C8C` → `this`; else a tail jump to the `cSC4Occupant` QI with `ECX = this + 4` (**no** model-instance fallback) |
| `cSC4PylonOccupant` (`0x005F0820`) | `0xC9B6D7D6` (`cISC4PylonOccupant`) → `this`; `0x452294AA` → `+4` (null-`this` quirk: out = 0, returns 1); else the `cSC4Occupant` QI, then the model instance |

The IID `0xA9764C8C` accepted by every class is the family's shared "class ID as IID" (compare prop-occupant.md
§1.1). The pole additionally answers `0x452294AA` by returning `this + 4` even when `this` is null *[quirk]*:
the body tests `this` only after writing the out-pointer.

### 1.3 `GetGZCLSID` and the record class IDs

Every class answers `GetGZCLSID` (serializable slot 5) with its class ID:

| Class | Body | Returns |
|---|---|---|
| `cSC4NetworkOccupant` | `0x00619040` | `0xC9C05C6E` |
| `…WithPreBuiltModel` | `0x00604E50` | `0x49C1A034` |
| `…BridgeOccupant` | `0x00604BB0` | `0x49CC1BCD` |
| `…TunnelOccupant` | `0x006472D0` | `0x8A4BD52B` |
| `cSC4UndergroundOccupant` | `_purecall` `0x005D4A10` | — the base class is abstract; only pipe and subway are serializable |
| `cSC4PipeOccupant` | `0x00648F40` | `0x49C05B9F` |
| `cSC4SubwayOccupant` | `0x00653D90` | `0xCA16374F` |
| `cSC4UndergroundConnectionOccupant` | `0x00654E70` | `0x8A139184` |
| `cSC4PipeConnectionOccupant` | `0x00648040` | `0xEA0A5ACF` |
| `cSC4SubwayConnectionOccupant` | `0x00652A70` | `0xCA15ED6B` |
| `cSC4PowerPoleOccupant` | `0x0064DD50` | `0x09C05C6A` |
| `cSC4PowerLineOccupant` | `0x00649260` | `0xC9C05C5D` |
| `cSC4PylonOccupant` | `0x005F1A10` | `0x49B6D69B` |

`cSC4UndergroundOccupant`'s pure virtual `GetGZCLSID` (and `_purecall` slots 101/102/105 of its primary vtable)
make it an abstract class: it is never instantiated directly; the tools create pipe and subway occupants.

### 1.4 `IsOfType` and the type table

`cISC4NetworkOccupant` slot 22 (`0x00619520`): with the argument `t`, reads `EDX = [t*4 + 0x00AA7CE8]` and
tail-calls primary slot 8 (`HasAllNetworkFlags`, all bits). So **`IsOfType(t)` ≡ the occupant carries network flag
`1 << t`** — the same 13-entry table the network manager's own type queries use (network-manager.md §5: bit `1<<t`
= network type `t`). The table at `0x00AA7CE8` holds `1<<0` … `1<<12` in order. An implementer needs no table at
all: `IsOfType(t)` is `GetNetworkFlags() & (1 << t)`.

---

## 2. Object layout: `cSC4NetworkOccupant`

All offsets from the object base (= the `cISC4NetworkOccupant` pointer). The constructor is `0x0061BDD0`.

*Rewritten 2026-10-21 (re-ugbase), from the instruction bytes and the SDK's slot order. It corrects the earlier
§2.1–§2.7 (offsets there were partly view-relative):*
- `+0xFC` is the **piece id** (slot 23), `+0x100` the **under-texture** (slot 29), `+0x104` the **rotation and
  flip** byte (slots 24–26, 74), `+0x105` the **variation** (slots 27, 28); slot 27 is `SetVariation` and slot 29
  `SetUnderTexture` (SDK), not `SetRotation` / `SetVariation`;
- `+0x108` is the **edge store** (`cSC4EdgeConnectionStore`, slots 37–45, §2.10); `+0x118` holds the **parts**
  (slot 31 `AddPart`); the occupant type is at `+0x28` (`0x0061BF85`), not `+0x24`; the model-instance properties
  set to `0x1A` by `Init` are at `+0x4C`;
- `Init` **clears** network flag `0x2000` (primary slot 11, `0x0061CB7A`); it adds the piece to the retrofit list only
  when `CheckForRetrofit` (slot 6, property `0x4835C6AC`) is true; on a second `Init` it clears flags `0x10000000`
  and `0x800000` directly and sets them again through `SetNetworkFlag` (slot 10), so their change messages are
  posted again;
- `StaticInit` gets the texture manager by its service ID (it subscribes to nothing) and fills the avenue
  intersection set with 13 piece ids (§2.5);
- the path connections are a byte per (path type, entry side) with a bit per exit side (§2.6);
- `GetPosition` is the corners' mean x and z and their highest y (§2.9); `GetOccupiedCell` rounds to nearest.

### 2.1 Sub-object map and identity

| Offset | What |
|---|---|
| `+0x00` | `cISC4NetworkOccupant` (vtable `0x00AA8080`, 81 slots, §2.4) |
| `+0x04` | `cISC4Occupant` base sub-object (its ctor runs with `this = object+4`; vtable `0x00AA7FF8`, 34-slot `cISC4Occupant` interface) |
| `+0x08` | `cIGZSerializable` (`0x00AA7FDC`: Write `0x0061A190`, Read `0x0061CC00`, `GetGZCLSID` `0x00619040`) |
| `+0x0C` | exemplar property holder (`0x00AA7F78`, 25 slots; foundations §10.3) |
| `+0x20` | a small interface (`0x00AA7F60`: `QueryInterface` / `AddRef` / `Release` thunks and two slots) *[summary]* |
| `+0x38` | the model instance part (vtable `0x00AA7F5C`; `cS3DModelInstanceBasic` ctor `0x00801CF0` runs with `this = object+0x38` at `0x0061BE07`); its table pointer `+0x3C` locates the virtual base |
| `+0x17C` | the virtual base: the model instance interface (`0x00AA7F10`, model-instances.md §1) |

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`, `0x00619040`) | `0xC9C05C6E` |
| `QueryInterface` (`0x006192C0`) | `0xA821EF94` (`cISC4NetworkOccupant`) and `0xA9764C8C` give the main interface, counted; any other IID goes to the occupant base, then to the model instance base |
| Occupant type (occupant slot 7, the view's `+0x24`) | `0xC772BF98`, written by the constructor at object `+0x28` (`0x0061BF85`) over the base ctor's `0x74758925` |
| Factory | `GZCOM_CreatecSC4NetworkOccupant` (`0x0061C480`), registered by `cSC4COMDirector`: a block from the class's pool (`operator new` `0x00619E90`, pool `0x00B0B398`), constructed as most-derived; returns the main interface |
| Destructor | body `0x0061C170`; deleting destructor `0x0061C4C0` (returns the block to the pool) |

The pool is an implementation detail *[choice]*: FreeCity4 may allocate the objects in any way. The other vtables'
slots reach the methods through this-adjustor thunks (`0x0061BFC0`…`0x0061C160`).

The model instance interface's slots: 0–2 the main object's `QueryInterface`, `AddRef`, `Release`; 3 `GetOwner`
(the interface itself, no reference; `0x006193A0`); 4–6, 13–16, 18 the basic model instance's (owner, custom data,
properties, draw attributes, `GetResModel`); 7 `GetBounds(box)` (the occupant's box in the instance's own frame,
the inverse transform applied *[summary]*; `0x0061C110`, same body as `0x0061B1D0`); 8 `GetWorldBounds(box)` (the
occupant's box, `0x0061C0A0`); 9, 10 `Get/SetTransform` (the set adds 1 to the change counter `+0x64`); 11
`Pick(ray)` (§2.9); 12 `Draw` (§2.11); 17 `GetSortValue` (§2.11).

### 2.2 Fields *[compat]*

| Offset | Type | Meaning |
|---|---|---|
| `+0x28` | u32 | the occupant type `0xC772BF98` (§2.1) |
| `+0x4C` | u32 | the model instance's properties: `0x1A` after `Init` (§2.5); the prebuilt `Init` also clears `0x10` and sets `0x40` (§2.8) |
| `+0x50`…`+0x63` | 20 bytes | the model instance's draw attributes (model-instances.md §1 slots 15, 16): byte `+0x50` (bit `0x02` translucent), byte `+0x51`, the opacity `+0x60` (f32) |
| `+0x64`…`+0x9B` | `S3DTransform` (0x38) | the transform (NO15): `+0x64` u8 change counter, `+0x65` u8 part mask (`4` rotation, `1` translation, `2` scale), `+0x68`…`+0x8B` 9 f32 rotation rows, `+0x8C`/`+0x90`/`+0x94` f32 translation, `+0x98` f32 scale; the ctor zeroes `+0x64`/`+0x65` (`0x0061BE52`) and seeds the matrix from `0x00B4525C` |
| `+0x9C`, `+0xB4`, `+0xCC`, `+0xE4` | 4 × 0x18 | the four **corners** of the textured quad: f32 x `+0`, y `+4`, z `+8`, colour dword `+0xC` (bytes B, G, R, A at `+0xC`…`+0xF`), f32 u `+0x10`, v `+0x14` (NO16) |
| `+0xFC` | u32 | the **piece id** (slot 23) |
| `+0x100` | u32 | the **under-texture** (slot 29) |
| `+0x104` | u8 | the **rotation** (low 7 bits) and the **flip** (bit 7) (slots 24–26, 74) |
| `+0x105` | u8 | the **variation** (slots 27, 28) |
| `+0x108`…`+0x117` | 16 bytes | the **edge store** (§2.10): u8 count `+0x108`, first entry u8 type `+0x10C` and u32 flags `+0x110`, heap array `+0x114` of `count` 8-byte entries {u8 type `+0`, u32 flags `+4`}; slot 44 (`0x00604E00`) is `ADD ECX,0x108; JMP 0x006199D0` (free + copy) |
| `+0x118`/`+0x11C`/`+0x120` | vector | the **parts** (slot 31, §2.11): begin/end/capacity, element stride **0x4C**: u32 texture id `+0`, then three vertices of 0x18 in the corner format (`+4`, `+0x1C`, `+0x34`) |
| `+0x124`…`+0x13B` | 6 floats | the box: min x/y/z `+0x124/+0x128/+0x12C`, max x/y/z `+0x130/+0x134/+0x138` (slot 80, §2.9; `GetBoundingBox` `0x00619360` on the `+4` view reads view `+0x120`…`+0x134`) |
| `+0x13C` | u32 | the **network flags** (ctor `0x6000000`); the flag table is network-manager.md §5 |
| `+0x140` | pointer | the **cached path info** (slots 49, 50; traffic-network.md §7.2); saved as its resource key (§2.3 field 12) |
| `+0x144`…`+0x163` | 8 × 4 bytes | the **path connections** (§2.6): a row of 4 bytes per path type |
| `+0x164` | u32 | the path-id override (slots 16, 51, 52); saved (field 11) and the trigger of the v < 8 read tail |
| `+0x168`, `+0x16C` | 2 × f32 | the **path origin offset** (x, z), meaningful only when `+0x164` ≠ 0. `GetPathInfo` (`0x0061B510`, `0x0061B635…0x0061B64B`) passes the two dwords by value as (x, 0, z) to the path info's slot `+0x74` (`OffsetAndScissor`, network-rules.md); the prebuilt `GetModelInstances` loads them as f32 and negates them into the `Deform` origin (§10.3, `0x0061EEC9…0x0061EEE0`). The ctor stores 0 in both (`0x0061BFAC`/`0x0061BFB2`). Not saved |
| `+0x170`, `+0x174` | s64 | the **cost of demolition** (slots 63, 64), saved as field 13 |

**Offsets in the record.** `Write` and `Read` sit in the `+8` vtable (`0x00AA7FDC`) with no thunk, so they run with
`this = object+8`; every offset the record code uses is 8 less than the object offset in this table (NO16 c,
NO17 b). The rows above give object offsets.

**Properties.** Some state lives in the property holder (`+0xC`), so it is saved with the occupant record's
property list (answers NO5: the slots that use them carry the SDK names):

| Property | Value | Slots (SDK name) |
|---|---|---|
| `0x4835C6A7` | 2 × u32 (an array variant) | 13, 14: `Get/SetConnectionNodeEdgeTypes` |
| `0x4835C6A8` | f32 | 67, 68: `Get/SetPassageCeiling` |
| `0x4835C6A9` | u32 | 15, 16: `Get/SetConnectionPathIDOverride` |
| `0x4835C6AA` | u8 | 69, 70: `Get/SetOneWayDirections` |
| `0x4835C6AB` | u32 `(z << 16) + x` | 71, 72: `Get/SetComplexPieceOrigin` |
| `0x4835C6AC` | an empty variant | 6, 7: `CheckForRetrofit` / `SetRetrofitFlag` (present = set) |

The property holder's `HasProperty` (`0x00619500`) is true for `0x88696402` without looking; other IDs go to the
exemplar property holder (NO38).

### 2.3 The base save record *[compat]*

`Write` `0x0061A190` / `Read` `0x0061CC00`, both `__thiscall` (callee pops 4), both returning a clean 0/1 in EAX
(from the stream's good flag, `NEG AL; SBB EAX,EAX; INC EAX`). `Write` emits version **8**; `Read` accepts any
version ≥ 4 (below that it fails at once) and has conversions for 4/5/6/7. In order:

| # | Field | Type | Notes |
|---|---|---|---|
| 1 | version | u16 = **8** | `Read` rejects < 4 with nothing more read |
| 2 | the base occupant record | — | `cSC4Occupant::Write` / `Read` (lot-records.md §2.3) |
| 3 | transform | `S3DTransform` stream form | object `+0x64` (view `+0x5C`). Write `operator<<` `0x005DA8C0`: u8 mask = the transform's own mask byte `+0x65`, copied, never recomputed; then if `mask & 4` the 9 rotation f32 (`+0x68`…`+0x88`, row by row), if `mask & 1` the 3 translation f32 (`+0x8C`, `+0x90`, `+0x94`), if `mask & 2` the scale f32 (`+0x98`). Read: v ≥ 5 `operator>>` `0x005DA710`, v 4 `S3DTransformLegacyLoad` `0x005DA930`; both rules below the table (NO15) |
| 4 | the four corners | 4 × { 5 × f32, 4 × u8 } = 96 bytes | object `+0x9C`…`+0xFB`, one corner per iteration (stride 0x18, `0x0061A1C0`…`0x0061A23E`): f32 x `+0`, y `+4`, z `+8`, u `+0x10`, v `+0x14`, then the colour bytes `+0xE`, `+0xD`, `+0xC`, `+0xF` — i.e. **R, G, B, A** of the little-endian ARGB dword `+0xC`. Stream slot `+0x30` is `SetFloat32`/`GetFloat32` (NO16 a) |
| 5 | piece id, variation, under-texture, rotation and flip | u32, u8, u32, u8 | `+0xFC`, `+0x105`, `+0x100`, `+0x104`, in that order |
| 6 | the edge store | u16 = 2, u8 count, u8 first type, u32 first flags, count × { u8 type, u32 flags } | helper `0x00619C10` with `this = object+0x108` (§2.10). Read `0x00619C90`: a version other than 2 returns 0 after consuming only the u16, and **the caller ignores that result** (the stream is then misaligned) *[quirk]*; otherwise it frees the old array, reads the header fields and allocates `count × 8` bytes before reading the entries (count 0: no array) |
| 7 | part count | u32 | `(end − begin) / 0x4C` over `+0x118`/`+0x11C` (magic division `0x6BCA1AF3`, `SAR 5`). Read resizes the vector to the count with zero-filled elements (`0x0061CAF0`) |
| 8 | per part | u32 + 3 × { 5 × f32, 4 × u8 } = 76 bytes | the texture id, then the three vertices at element `+4`, `+0x1C`, `+0x34` in the field-4 form. Read forces each vertex alpha byte to `0xFF` after reading it (`0x0061CE04`). The element is 0x4C bytes in memory and 76 in the stream (NO16 b) |
| 9 | the box | 6 × f32 | interleaved min/max per axis: min x `+0x124`, max x `+0x130`, min y `+0x128`, max y `+0x134`, min z `+0x12C`, max z `+0x138` (loop of 3 at `0x0061A3B8`, slot `+0x30`) (NO16 c) |
| 10 | the network flags | u32 | object `+0x13C`. Read: below v6 converted by `ConvertV5FlagsToCurrentFlags` (rule below), then always `& 0xFFFF1FFF`, which clears `0x2000` (path connections built), `0x4000` and `0x8000` (lighting up to date) |
| 11 | the path-id override | u32 | object `+0x164` |
| 12 | the cached path info's key | 0 or 3 × u32 | `SC4WriteGZPersistRef` (`0x005D9940`): only through a database segment stream — the stream is asked for `cISC4DBSegmentOStream` (`0x6999A8FB`); when it refuses, **nothing** is written. Otherwise, when the path info `+0x140` answers `cIGZPersistResource` (`0x456B8F1D`), its slots 5, 6 and 7 are written *[inferred: type, group, instance]*; with no path info, or one that does not answer, three zeros. `SC4ReadGZPersistRef` (`0x005D9820`) reads the three u32 only through `cISC4DBSegmentIStream` (`0x4999A8F5`) and asks the resource manager (`0x00B43CA8`, slot 3) for that key as interface `0xA9667793`; the result (or null) becomes the cached path info. Read in **every** accepted version. Where the city file keeps the piece's own path info is NO39 |
| 13 | the cost of demolition | s64 (`SetSint64`, slot `+0x28`) | low dword `+0x170`, high `+0x174`. Read in every accepted version |

`Write` returns early (to the error-flag return) when the occupant base record fails; otherwise it writes all 13
fields and returns 1 iff the stream's error flag is clear. Byte count of a version-8 record written to a segment
stream: 2 (version) + the base occupant record + transform (1 + 36·[mask&4] + 12·[mask&1] + 4·[mask&2]) + 96 + 10
+ (8 + 5·count) + 4 + 76·parts + 24 + 4 + 4 + 12 + 8.

**Transform read rules (NO15).** `operator>>` `0x005DA710` reads the u8 mask; if the live mask byte `+0x65` is
non-zero it first resets the transform to identity (rotation = identity, translation 0, scale 1.0), sets the mask
to 0 and increments the counter `+0x64`; then for each mask bit present it reads that part, ORs the bit into `+0x65`
and increments the counter. A part the stream omits keeps its value from before the call (identity after the
reset; untouched when the live mask was already 0). The scale is stored as read, but bit 2 is set only when the
scale differs from 1.0 (constant `0x00A81228`; a NaN counts as different) and cleared when it equals 1.0. So a
written mask of 7 with scale 1.0 reads back as mask 5. Worked example: a transform with mask `0x01` and
translation (8, 2.5, 8) writes the 13 bytes `01 | 00 00 00 41 | 00 00 20 40 | 00 00 00 41`; mask `0x07`, identity
rotation, translation 0 and scale 2.0 writes 1 + 36 + 12 + 4 = 53 bytes ending `00 00 00 40`.
`S3DTransformLegacyLoad` (v 4) is fixed-size, 56 bytes: 9 f32 rotation (rows, → `+0x68`…`+0x88`, mask `|= 4`),
3 f32 translation (→ `+0x8C`…`+0x94`, mask `|= 1`), 1 f32 scale (→ `+0x98`, bit 2 by the 1.0 test as above), then
a u32 whose **low byte replaces the counter `+0x64`** (the three increments are overwritten). It never resets the
mask first: bits already set stay set.

**`ConvertV5FlagsToCurrentFlags` `0x006191A0` (NO16 d)** maps each old bit to a new bit, starting from 0:

| old | new | old | new | old | new |
|---|---|---|---|---|---|
| `0x1` | `0x1` | `0x400` | `0x1000000` | `0x100000` | `0x80` |
| `0x2` | `0x2` | `0x800` | `0x2000000` | `0x200000` | `0x100` |
| `0x4` | `0x4` | `0x1000` | `0x4000000` | `0x400000` | `0x10000` |
| `0x8` | `0x8` | `0x2000` | `0x40` | `0x800000` | `0x20000` |
| `0x10` | `0x10` | `0x4000` | `0x20` | `0x1000000` | `0x100000` |
| `0x20` | `0x40000` | `0x8000` | `0x400000` | `0x4000000` | `0x80000000` |
| `0x40` | `0x80000` | `0x10000` | `0x10000000` | `0x20000000` | `0x200000` |
| `0x200` | `0x800000` | `0x40000` | `0x20000000` | | |
| | | `0x80000` | `0x40000000` | | |

Old bits `0x80`, `0x100`, `0x20000`, `0x2000000`, `0x8000000`, `0x10000000`, `0x40000000`, `0x80000000` produce
nothing directly. Last step: if old bit `0x80` is **clear**, the result loses `0x6000000` and gains `0x8000000`.
Worked example: old `0x0000_0283` → `0x1 | 0x2 | 0x800000` = `0x00800003` (bit `0x80` set, so no last step); then
the read mask gives `0x00800003`. Old `0x0000_1001` → `0x1 | 0x4000000`, then bit `0x80` clear → `0x08000001`.

**The read tail (NO16 e).** After field 11, only when the path-id word `+0x164` is non-zero:
- v < 7: the record holds **two more f32** here (offsets `dx`, `dz`; current writers never emit them). Read computes
  a cell `x = floor(((v0.x + v2.x) · 0.5 + dx) · 0.0625)` and `z = floor(((v0.z + v2.z) · 0.5 + dz) · 0.0625)` from
  corners 0 and 2 (object `+0x9C`/`+0xCC` for x, `+0xA4`/`+0xD4` for z), in x87 then rounded through f32
  (`+ 12582912.0` trick and a fix-up that makes exact integers floor to themselves, `0x0061CEBD`…`0x0061CF5B`), and
  calls primary slot 72 (`SetComplexPieceOrigin`, `0x0061BA90`) with `(x, z)`, which sets property `0x4835C6AB` to
  `(z << 16) + x`.
- v < 8 (so also after the v < 7 step): primary slot 7 (`SetRetrofitFlag`, `0x0061A120`) with 1, which adds the
  empty property `0x4835C6AC`.
Then fields 12 and 13 are read in every version. Finally Read fetches the draw attributes from the model-instance
base (its slot `+0x3C`); if corner 0's alpha `+0xAB` is not `0xFF` or their translucent bit (2) is set, it forces
all four corner alphas to `0xFF`, clears bit 2 and stores `255 · (1/255)` = 1.0 as the opacity before handing them
back (slot `+0x40`), so a loaded piece is never translucent. The return is 1 iff the stream's error flag is clear;
version < 4 returns 0 having read only the u16.

The subclass records add fields after the base block; §3.4, §5.4, §6.4, §7.4, §8.5 give each.

### 2.4 Interface slots (`cISC4NetworkOccupant`, vtable `0x00AA8080`) *[compat]*

The SDK names slots 3–73 (its slot order matches the exe except that 53 and 54 are swapped, traffic-network.md
§7.2; this spec keeps the compiled order). Slots 74–80 are past the SDK.

| Slot | Address | Behaviour |
|---|---|---|
| 3 | `0x00619050` | `Init(corners[12])` (§2.5) |
| 4 | `0x00619100` | `Shutdown`: the occupant base `Shutdown` |
| 5 | `0x00648EF0` | `IsManagedGlobally`: false (§2.7) |
| 6 | `0x00619180` | `CheckForRetrofit`: property `0x4835C6AC` present |
| 7 | `0x0061A120` | `SetRetrofitFlag(b)`: true adds the property (an empty value), false removes it |
| 8 | `0x00604D10` | `HasNetworkFlag(m)`: `(flags & m) = m` |
| 9 | `0x00604D30` | `HasAnyNetworkFlag(m)`: `(flags & m) ≠ 0` |
| 10, 11 | `0x00619EC0`, `0x00619F70` | `SetNetworkFlag`, `ClearNetworkFlag` (§2.7) |
| 12 | `0x006E3000` | `GetNetworkFlag`: the flags |
| 13 | `0x00619110` | `GetConnectionNodeEdgeTypes(&a, &b)`: from property `0x4835C6A7`; false without it |
| 14 | `0x0061A020` | `SetConnectionNodeEdgeTypes(a, b)`: sets the property |
| 15 | `0x00619150` | `GetConnectionPathIDOverride`: property `0x4835C6A9`, or 0 |
| 16 | `0x0061A090` | `SetConnectionPathIDOverride(id)`: 0 removes the property, else sets it; then `+0x164 := id`, `ReleasePathInfo`, clear flag `0x2000` |
| 17, 18 | `0x00604D50`, `0x00604D60` | `IsImmovable` / `SetImmovable(b)`: flag `0x400000` |
| 19, 20 | `0x00604D80`, `0x00604D90` | `IsUsable` / `SetUsable(b)`: flag `0x10000000` |
| 21 | `0x00604DB0` | `IsPlaced`: the occupant's flag bit 0 (in the occupant manager) |
| 22 | `0x00619520` | `IsOfType(t)`: `HasNetworkFlag(table[t])`, table `0x00AA7CE8` = `1 << t` for `t` 0–12; no bounds check *[quirk]* |
| 23 | `0x00459960` | `PieceId`: `+0xFC` (a body shared with other classes) |
| 24, 25, 26 | `0x00604DC0`, `0x00604DD0`, `0x00604DE0` | `GetRotation` (`+0x104 & 0x7F`), `GetFlip` (`+0x104 >> 7`), `GetRotationAndFlip` (the byte) |
| 27, 28 | `0x0061B7F0`, `0x00619880` | `SetVariation(v)` / `GetVariation`: `+0x105`; a change posts ChangeOccupant (§2.7) |
| 29 | `0x0061B880` | `SetUnderTexture(t)`: `+0x100`; a change posts ChangeOccupant |
| 30 | `0x0061B490` | `SetQuadDraw(piece, rotation)` (§2.11) |
| 31 | `0x0061C800` | `AddPart(positions, uvs, indices[3], texture)` (§2.11) |
| 32 | `0x00648EF0` | `HasViewDependentModel`: false |
| 33 | `0x0061AF90` | `GetModelInstances(…, out[], …, &count)`: translation `:=` the occupant's position (occupant slot 8), mask bit 1 set, change counter + 1; `out[0] :=` the model instance (counted), `count := 1` |
| 34, 35, 36 | `0x0065A6C0`, `0x006194C0`, `0x00619650` | `GetVertexHeights(h[4])`, `GetVertexHeight(i)` (ST0), `SetVertexHeights(h[4])`: the corners' y; the set does not recompute the box |
| 37–43 | | the edge store (§2.10) |
| 44, 45 | `0x00604E00`, `0x00798030` | `SetEdgeStore(store)` (a copy), `GetEdgeStore` (`+0x108`) |
| 46 | `0x00619540` | `MarkLightingUpdateNeeded`: clear flag `0x8000` |
| 47 | `0x00619320` | `ConnectsToNeighbor(side, t)`: `IsOfType(t)` and side `side`'s byte of `GetEdgeTypesFlag(t)` has bit 0 or 1 |
| 48, 49 | | `AsOccupant`, `GetPathInfo` (§2.6; traffic-network.md §7.2) |
| 50 | `0x00619680` | `ReleasePathInfo`: release and forget `+0x140` |
| 51, 52 | `0x006197D0`, `0x006197E0` | `SetPathIDOverride(id)` / `GetPathIDOverride`: `+0x164` |
| 53, 54 | `0x006196A0`, `0x00619790` | `PathConnectionExists`, `AddPathConnection` (§2.6) |
| 55 | `0x0061BD80` | `IsIntersection`: an avenue piece (flag `0x40`, `IsOfType(6)`) whose piece id is in the static set (§2.5) |
| 56, 57 | `0x006197F0`, `0x0061B6C0` | `GetAltitude(x, z)`, `GetNormal(out, x, z)` (§2.9) |
| 58 | `0x0061B750` | `SetTranslucency(on, alpha)` (§2.11) |
| 59 | `0x0061B010` | `GetOccupiedCell(&x, &z)` (§2.9) |
| 60, 61, 62 | `0x00619890`, `0x0061B910`, `0x00648EF0` | `IsLit` (flag `0x20000000`), `SetLit(b)` (§2.7), `CanBeLit` (false) |
| 63, 64 | `0x00604E10`, `0x00604E20` | `Get/SetCostOfDemolition`: the s64 `+0x170` (returned in `EDX:EAX`); the prebuilt primary `0x00AA7038` has the same body at `+0xFC` |
| 65 | `0x00648F10` | `TransformNode`: nothing |
| 66 | `0x006198A0` | `IsPartOfTheSameStructure(o)`: the two complex piece origins are equal (§2.9) |
| 67, 68 | `0x00619900`, `0x0061B9A0` | `GetPassageCeiling(&f)` (false without the property) / `SetPassageCeiling(f)` |
| 69, 70 | `0x00619940`, `0x0061B9F0` | `GetOneWayDirections` (0 without the property) / `SetOneWayDirections(d)`: with the property its value is set (0 included); without it, `d ≠ 0` adds it |
| 71, 72 | `0x00619970`, `0x0061BA90` | `GetComplexPieceOrigin(&x, &z)` (false without the property; `x := v & 0xFFFF`, `z := v >> 16`) / `SetComplexPieceOrigin(x, z)` (`v := (z << 16) + x`) |
| 73 | `0x006199C0` | `IsSupportPylonPresent`: flag `0x40000000` |
| 74 | `0x00604DF0` | `SetRotationAndFlip(b)`: the byte, nothing else |
| 75 | `0x006193D0` | `GetPosition(out)` (§2.9) |
| 76 | `0x0061AC70` | `DrawOneWayArrow(context, selfLit, state, directions)` (§2.11) |
| 77 | `0x0061C950` | `GetTexture(id, fallback)` (§2.11) |
| 78 | `0x0061C5F0` | `SetTextureZoom(context)` (§2.11) |
| 79 | `0x0061BB60` | `UpdateLighting(lighting)` (§2.7) |
| 80 | `0x0061A430` | `ComputeBox(&min, &max)` (§2.9) |

### 2.5 Lifecycle *[compat]*

**Construction.** The constructor sets the defaults of §2.2: identity transform, mask 0, zero corners and parts,
piece id, under-texture, rotation and variation 0, an empty edge store, flags `0x06000000`, no path info, no path
connections, no override, path origin offset 0, cost 0, type `0xC772BF98`.

**`StaticInit`** (`0x0061C690`, once; latched by the byte at `0x00B45250`):
1. hold the resource manager (`0x00B43CA8`, at `0x00B45254`) and the texture manager (system service `0x1AC0E11A`,
   interface `0xFAC0E219`, at `0x00B45258`);
2. register the static shutdown (`0x0061C520`) with the application (`0x00B43C94`, slot 69, `+0x114`);
3. pack three render states (draw-context.md §5.1 order): **A** `0x00B45260` = (1, 1, 1, 1, 1, 1, 1, 3, 4, 5, 4,
   0.1, 1, −1); **B** `0x00B45268` = (1, 1, 1, 0, 0, 1, 1, 3, −1, −1, −1, −1.0, 1, −1); **C** `0x00B45270` = (1, 1,
   1, 1, 1, 1, 1, 2, 1, 1, 4, 0.1, 1, −1);
4. empty the avenue intersection set (`0x00B4528C`) and fill it with the 13 piece ids of `0x00AA7D40`:
   `0x04003500`, `0x04003600`, `0x04005200`, `0x04005300`, `0x04005400`, `0x04006900`, `0x04007100`, `0x04007400`,
   `0x04007500`, `0x04008500`, `0x04009000`, `0x04009300`, `0x04009400` (slot 55).

The static shutdown releases the cached textures (§2.11), the texture manager and the resource manager, and clears
the latch.

**Main slot 3 `Init(corners)`** (`0x00619050`): the occupant `Init` (below); when it fails, false. Otherwise the 12
floats become the corners' x, y, z (corner 0 first), the box is recomputed (slot 80), true. (The SDK declares a
3-float array; it is 4 × 3.)

**Occupant slot 3 `Init()`** (`0x0061CB50`; the `cISC4Occupant` view's slot 3, `this = object+4`): `StaticInit` if
needed. When the occupant is not yet initialised (occupant slot 5): `ClearNetworkFlag(0x2000)` (primary slot 11);
model instance properties `+0x4C := 0x1A`; when `CheckForRetrofit` (primary slot 6) is true, the network manager's
`AddToRetrofitList(this)` (manager slot 23, `+0x5C`; network-manager.md §7.10); then `cSC4Occupant::Init`
(`0x005E8C90`, which posts the `CreateOccupant` message only for fresh occupants, lot-records.md §2.4), whose
result is returned. When it is already initialised (the re-`Init` after a load): if flag `0x10000000` is set, it is
cleared in `+0x13C` and set again through `SetNetworkFlag` (slot 10), and the same for `0x800000`, so their change
messages are posted again (§2.7); true.

**`Shutdown`** (slot 4, `0x00619100`) = `cSC4Occupant::Shutdown` only; the base class holds no model to release.

### 2.6 Path connections *[compat]*

The table `+0x144` has a row of 4 bytes per path type (8 rows); byte `entry` of a row has bit `exit` set for a
connection from side `entry` to side `exit` (answers NO12).

- **`AddPathConnection(type, entry, exit)`** (slot 54, `0x00619790`): unless `entry` is `0xFF`, ORs `1 << exit`
  into byte `+0x144 + type·4 + entry` and into byte `+0x144 + entry` (row 0, path type 0, is always set too); then
  `SetNetworkFlag(0x2000)` (in every case).
- **`PathConnectionExists(type, entry, exit)`** (slot 53, `0x006196A0`): first builds the path info when flag
  `0x2000` is clear (`GetPathInfo`); then, with `0xFF` as a wildcard: both wild → the number of set bits 0–3 over
  the row's 4 bytes; only `entry` wild → the number of bytes with bit `exit`; only `exit` wild → the number of bits
  0–3 set in byte `entry`; neither → bit `exit` of byte `entry`.
- **`GetPathInfo`** (slot 49, `0x0061B510`): returns the cache at `+0x140` without `AddRef` (TG24); builds it on
  first call. **`ReleasePathInfo`** (slot 50, `0x00619680`) and the reset path (`pathinfo` slot 0xA0, zeroing
  `+0x144…+0x160`, pathinfo slot 0x6C) belong to the traffic rebuild (traffic-network.md §7.2).

### 2.7 Flags, lighting and messages *[compat]*

- `SetNetworkFlag` `0x00619EC0` / `ClearNetworkFlag` `0x00619F70` post the queued message `0x89D3E9F3` (usable
  change, data1 = 1/0) exactly as network-manager.md §5 says: the set only when the occupant is in the city
  (`GetFlags() & 1`, primary slot 0x84); the clear unconditionally tests. Setting `0x4000` also calls slot 58
  (`SetTranslucency(1, 0x80)`).
- `SetLit(b)` (slot 61, `0x0061B910`): when `CanBeLit` (slot 62 = `0x00648EF0`, false for the base class) and `b`
  differs from flag `0x20000000`: toggle the flag and post (queued) `0x09D8E0B6` with the occupant (slot 48) as its
  object. The base `CanBeLit` is false, so only subclasses light.
- `UpdateLighting(lighting)` (slot 79, `0x0061BB60`): each corner gets its colour from the lighting's slot 12 (its
  position; red, green, blue into `+0xE`, `+0xD`, `+0xC`). Each part gets the normal of its triangle
  (`SC4GeometryUtilities::GetNormalToTriangle`), scaled to length 1, and the lighting's slot 19 (first vertex,
  normal; red, green, blue), written to all three vertices. Then flag `0x8000` is set (primary slot 10).
- `IsManagedGlobally` (slot 5) = `0x00648EF0`, **false** for the plain class and for the underground, pipe and
  subway classes (their vtables `0x00AAAD40`, `0x00AA97B8`, `0x00AAA3C8` hold the same body; answers NO9). The
  prebuilt (`0x00AA7038`), bridge (`0x00AA7300`) and tunnel (`0x00AA9308`) primaries answer **true** — that is what
  puts a piece on the network manager's `managed` list (network-manager.md §6.1).
- Messages: `SetVariation` and `SetUnderTexture`, on a change, post (queued) ChangeOccupant `0x99EF1145` built by
  hand: data1 (message slot 11, not void1) and the object are the occupant, data2 is **1**, data3 the type
  `0xC772BF98`, data4 0 (`PostOccupantMessage`, flora.md §9.1, would set void1 instead and put the occupant's flag 4
  in data2). `SetTranslucency` uses `PostOccupantMessage(0x99EF1145, 0)`.

### 2.8 The prebuilt subclass *(cSC4NetworkOccupantWithPreBuiltModel)*

The constructor `0x0061E1A0` calls the base ctor, sets network flags `0x400000` (`IsImmovable`, zones.md §8.4.1)
at `+0x13C`, and adds:

| Offset | Meaning |
|---|---|
| `+0x170`/`+0x174`/`+0x178` | u32 triplet (written by the record; NO4, §12) |
| `+0x17C` | u32 |
| `+0x180` | byte |
| `+0x184`…`+0x19C` | 7 floats — the **shear transform** inputs; `RecomputeShearTransform` `0x0061D230` uses the constant `0.0625` (`0x00AA81E4`); the ctor seeds `+0x184…+0x1AC` from `0x00B45298` and then zeroes `+0x184…+0x19C` |
| `+0x194` | float gate: if ≠ 0, `+0x198…+0x1A4` hold 4 more floats; v < 4 records used the legacy `+0x1A8`/`+0x1AC` pair + 2 more |
| `+0x1B0` | vector (u32) |
| `+0x1B4` | byte |
| `+0x1B8`/`+0x1BC` | int pair, ctor sets both to **−1** |
| `+0x1C0`/`+0x1C1` | bytes |
| `+0x1C4`/`+0x1C8` | pointers (the model side; the vtable patch at `+0x3C`-relative `0x00AA6EC8`/`0x00AA6FB0` pairs) |

`Init` `0x0061DB00`: the base `Init`; then `+0x48 & ~0x10 | 0x40`; city `0x00B43DDC` slot 0x24 → primary slot
0xF4 (`SetModel`, the prebuilt model from the exemplar); `RecomputeShearTransform`.

`GetPathInfo` `0x0061D380` (overrides the base): asks its model instance first (`+0x20` list slot 0x10
`GetCount`); a null model → null (traffic-network.md §7.2); otherwise like the base, honouring the `+0x164`
override and using `FUN_00466590`; caches into `+0x140`; if the result lacks `HasNetworkFlag(0x100000)` it calls
pathinfo slot 0x68; then `ResetPathInfo`.

`Shutdown` `0x0061D0D0` releases the model. `Write` `0x0061D8E0` (version 4) / `Read` `0x0061D9B0` (versions
3…4) are the base record plus the prebuilt fields; the field list is §12 (NO4).

The **bridge** subclass adds: `QueryInterface` `0x00604BC0` (class ID `0x49CC1BCD`), `Init` `0x00604BF0` = the
prebuilt `Init`, `Shutdown` `0x00604C10` releases the `+0x1D0` model/structure list then the prebuilt
`Shutdown`, and an extra constructor part `0x00604C60` allocating a 0x42-entry zeroed array (the `0x1F0`-stride
region). Creation is network-bridges.md §11.2 (class `0x49CC1BCD` through `CreateNetworkOccupant` slot 54, corner
points via slot 3, position = mean x/z with the highest corner, flags, slot 30, slot 68 passage ceiling when the
property `0xABE1B5A3` is absent, insert after the triangles). Its primary vtable `0x00AA7300` has one extra slot
(81, `0x00605070`, a helper over `+0x1D0`) before its RTTI column.

The **tunnel** subclass ctor `0x006476F0` runs the prebuilt ctor then sets `+0x13C &= ~0x400 | 0x10C00000`
(clearing the prebuilt bit, setting usable `0x10000000` + `0x0C00000`), stores the **paired portal** at `+0x1D0`
(traffic-network.md §4.8 TG25: counted ref, non-virtual setter `0x00647530`, getter `0x00647570` folded, released
by `Shutdown` `0x006474F0`); `Write` `0x00647580` (v2) writes the has-pair bool and the ref (read with IID
`0xE4FDA3D4`); `Read` `0x00647610` (v2) uses `SC4ReadGZSerializable` IID `0x8A4BD52B`; `Init` `0x006471C0` = the
prebuilt `Init` + `ClearNetworkFlag(0x4000)`. Creation and pairing are traffic-network.md §4.8 and
network-geometry.md §6.


### 2.9 Geometry *[compat]*

*Added 2026-10-21 (re-ugbase).* The four corners sit on the piece's 16 m cell: relative to corner 0 at `(x0, z0)`,
corner 1 is at `(x0, z0 + 16)`, corner 2 at `(x0 + 16, z0 + 16)` and corner 3 at `(x0 + 16, z0)`. Their texture
coordinates come from the rotation (§2.11). With `dx := x − x0`, `dz := z − z0` and `y_i` the corners' heights:

- **`GetAltitude(x, z)`** (slot 56): if `dx ≤ dz`, `y0 + (dx × (y2 − y1) + dz × (y1 − y0)) × 0.0625`; otherwise (and
  when the comparison is unordered) `y0 + (dx × (y3 − y0) + dz × (y2 − y3)) × 0.0625`. The cell is split along its
  `dx = dz` diagonal, whatever flag `0x1000000` says for drawing (§2.11) *[quirk]*.
- **`GetNormal(out, x, z)`** (slot 57): an unnormalised vector with `out.y = 16`. When `dz > dx`: `out.x = y0 − y3`,
  `out.z = y0 − y1`; otherwise (`dz ≤ dx`, or unordered): `out.x = y1 − y2`, `out.z = y3 − y2`. The z components are
  those of the triangle `GetAltitude` uses, the x components the other triangle's *[quirk]*; on a planar quad they
  agree.
- **`GetOccupiedCell(&x, &z)`** (slot 59, `0x0061B010`): corner 0's x and z (`+0x9C`, `+0xA4`) times `0.0625`
  (`0x00AA7D78`), plus `12582912.0` = 1.5 × 2²³ (`0x00AA7D7C`), stored as f32, whose bits minus `0x4B400000` are the
  cell: **round to nearest, ties to even**, in float32 (foundations §18).
- **`IsPartOfTheSameStructure(o)`** (slot 66): both pieces' `GetComplexPieceOrigin` (slot 71) are called into
  locals and the x and z compared. The results are not checked, so a piece without an origin compares uninitialised
  values *[quirk]*; FreeCity4 returns false when either piece has no origin *[choice]*.
- **`GetPosition(out)`** (slot 75, `0x006193D0`, and the occupant's slot 8 through it): x and z are the means of the
  four corners' x and z (sum × 0.25), y the largest corner height (from `−FLT_MAX`, `0x00AA7D74`).
- **`ComputeBox(&min, &max)`** (slot 80, `0x0061A430`): both start at the occupant's position and are widened by
  every corner and every part vertex. Slot 3 and slot 31 call it with `+0x124`, `+0x130`; the occupant's
  `GetBoundingBox` (slot 10, `0x00619360`) returns those two.
- **`Pick(ray)`** (model instance slot 11): the box with its top raised by 0.1 is tested against the ray
  (`RayIntersectsBoundingBox`); on a hit at a distance not above the ray's current best (`+0x28`), that distance
  becomes the best and the result is true.

### 2.10 The edge store (`cSC4EdgeConnectionStore`, `+0x108`) *[compat]*

*Added 2026-10-21 (re-ugbase).* A small map from network type to a u32 of edge types, one byte per side 0–3: a
first entry (u8 type `+0x10C`, u32 flags `+0x110`) and a heap array (`+0x114`) of `count` (`+0x108`) further
entries of 8 bytes {u8 type, u32 flags}. In an edge byte, bits 0–1 are the edge type and bit 2 marks a grouped
edge.

- `SetEdgeTypesFlag(t, f)` (slot 37, `0x00619A40`): `t` is the first type → its flags `:= f`. Else, when the first
  flags are 0, the first entry becomes `(t, f)`. Else `f = 0` removes `t`'s entry from the array (the last entry
  takes its place; the array is freed when it empties); `f ≠ 0` sets `t`'s entry, or appends one.
- `GetEdgeTypesFlag(t)` (slot 38, `0x00619B70`): the first entry's or the array entry's flags; 0 when absent.
- `SetEdgeType(t, side, e)` (slot 39): byte `side` of `GetEdgeTypesFlag(t)` replaced by `e`, then set.
- `GetActiveEdges` (slot 40): a 4-bit mask of the sides whose byte, in the OR of all entries, has any of bits 0–2.
  `GetActiveEdgesForType(t)` (41): the same over `t`'s flags. `GetActiveEdgesForTypeNoGroup(t)` (42): any of bits
  0–1. `GetGroupedEdges` (43): bit 2, over all entries.
- `SetEdgeStore` / `GetEdgeStore` (slots 44, 45): a copy in, the store's address out. The record form is §2.3
  field 6.

### 2.11 Drawing *[compat]*

*Added 2026-10-21 (re-ugbase).*

**The textured quad.** Two triangles over the four corners: with flag `0x1000000` clear they are (0, 1, 2) and
(0, 2, 3) (index list `0x00AA7D28`); with it set, (0, 1, 3) and (3, 1, 2) (`0x00AA7D34`). Flag `0x2000000` draws
the first triangle and `0x4000000` the second (so these two bits are "draw the surface quad's halves"; answers
UG2). The constructor sets both bits.

**`SetQuadDraw(piece, rotation)`** (slot 30, `0x0061B490`): unless the piece is a lot connection node (flag
`0x200000`), set flags `0x2000000` and `0x4000000`. Then release the path info, clear flag `0x2000`, store the piece
id and the rotation byte, set the corners' texture coordinates (below) and make the four corner alphas `0xFF`.

**Texture coordinates** (`SetTextureCoordinates(rotation)`, `0x00619550`): with `U = (0, 0, 1, 1)`,
`V = (0, 1, 1, 0)` (`0x00AA7D98`, `0x00AA7D88`) and `r := rotation & 3`, corner `k` gets `(U[(r + k) & 3],
V[(r + k) & 3])`. When the byte is `0x80` or more (flipped): for an odd `r` the v of corners 0 and 3 and of corners 1
and 2 are swapped; for an even `r` their u.

**`AddPart(positions, uvs, indices, texture)`** (slot 31, `0x0061C800`): appends one triangle whose vertex `k` is
`positions[indices[k]]` with `uvs[indices[k]]` and white opaque colour, tagged with `texture`; then the box is
recomputed (slot 80) and the lighting marked out of date (slot 46).

**`GetSortValue(out)`** (model instance slot 17): with any of flags `0x06000000`, the piece id; otherwise, with
parts, the first part's texture id; otherwise `0xA821EF94`.

**Textures** (`GetTexture(id, fallback)`, slot 77, `0x0061C950`): a texture id whose low 4 bits are 0 gets the
current zoom offset added (`0x00B0B374`, 5 at start). The id is looked up in a static cache; a miss loads the
texture `{type 0x7AB50E44, group 0x1ABE787D, instance id}` through the texture manager (slot 6) and caches the
result. When that fails and `fallback` is non-zero and differs from `id`, the fallback (with the same zoom rule) is
tried the same way. The result is the binding or null. `SetTextureZoom(context)` (slot 78, `0x0061C5F0`) sets the
zoom offset to the context's `+0x14` and releases every cached binding.

**`SetTranslucency(on, alpha)`** (slot 58, `0x0061B750`): read the draw attributes; `on` sets their bit `0x02` and
makes the four corner alphas `alpha`; off clears the bit and makes them `0xFF`. The opacity becomes corner 0's
alpha / 255. Write the draw attributes back and post `PostOccupantMessage(0x99EF1145, 0)`.

**`Draw(context, selfLit)`** (`0x0061A850`):
1. When the context's zoom (`+0x14`) differs from the cached one: slot 78.
2. When flag `0x8000` is clear: `UpdateLighting` (slot 79) with the context's lighting (`+0x44`).
3. Render state A, unless the piece is a portal (flag `0x80000`), or is elevated (`0x40000000`) with corner 0
   opaque (`+0xAB = 0xFF`): then the OR of all the edge store's flags is taken, and when it is none of
   `0x03010000`, `0x00030100`, `0x00000301`, `0x01000003`, `0x01000000`, `0x00010000`, `0x00000100`, `0x00000001`,
   `0x03000000`, `0x00030000`, `0x00000300`, `0x00000003`, render state B.
4. Set the state and the model transform. With `selfLit`: when the occupant has no highlight (occupant slot 16),
   stop (result 1); otherwise the self-lit shade `(1, 0.94, 0.55)` (`0x00B0B388`). Without: reset the shade.
5. With any of flags `0x06000000`: the under-texture (when `+0x100 ≠ 0`, `GetTexture(+0x100, 0)`) and the piece
   texture (`GetTexture(piece + 16 × variation, piece)`). Without a piece texture, skip to step 6. Otherwise wrap
   modes (2, 2, 0), filtering (1, 1, 0), the corners as a 4-vertex buffer (format 10); for the under-texture and
   then the piece texture, when present: set it and draw the quad's triangles; with `selfLit`, draw them a second
   time with state C, wrap (2, 2, 0) and filtering (1, 1, 0), then restore step 3's state. Then, when the one-way
   directions (slot 69) are not 0, `DrawOneWayArrow(context, selfLit, state, directions)`.
6. Parts: wrap (3, 3, 0), filtering (1, 1, 0). When the terrain shows its water surface (terrain slot 83), a part
   whose first vertex is below the water level there (terrain slot 30 at its x, z) is skipped. Each other part with
   a texture (`GetTexture(id, 0)`) is drawn as one triangle (format 10); with `selfLit`, a second time with state C.
7. With `selfLit`, reset the shade. The result is 1.

**One-way arrows** (`DrawOneWayArrow`, slot 76, `0x0061AC70`):
1. `e := GetEdgeTypesFlag(10)` (one-way road). When `e` has a bit outside bits 0–1 of each byte, stop.
2. `m :=` the mask of sides with an edge; the table `0x00AA7EE0` gives two rotation bytes per mask: 3 → (`0x83`,
   2), 5 → (1, 3), 6 → (3, `0x82`), 9 → (1, `0x80`), 10 → (2, 0), 12 → (`0x81`, 0); any other mask stops.
3. `e` must be one of `0x00020002`, `0x02000200` (straight) or `0x01000003`, `0x00000301`, `0x00030100`,
   `0x03010000` (curves); otherwise stop.
4. Texture `0x0BE35EB0`; for the curve masks (3, 6, 9, 12) texture `0x0BE35EC0`, and in `directions` the two bits of
   side 1 are swapped and so are those of side 2.
5. `n :=` the sum over the four 2-bit fields of `directions` (1 → +1, 2 → −1). The rotation byte is the table's
   first, or its second when `n < 0`.
6. `GetTexture(texture, 0)`; when missing, stop. State A, the texture, `SetTextureCoordinates(rotation byte)`, the
   quad's triangles; then state C, wrap (2, 2, 0), filtering (1, 1, 0) and the triangles again only with `selfLit`;
   then `state` (twice) and the corners' own coordinates back (`SetTextureCoordinates(+0x104)`).
---

## 3. The underground pieces

### 3.1 `cSC4UndergroundOccupant` — identity, layout and slots

*Rewritten 2026-10-21 (re-ugbase), from the instruction bytes: the constructor is `0x0065C330` (the pipe and subway
constructors call it, `0x00648910`, `0x006530D0`) and `0x0065C0C0` the destructor body; the slots below are named
by what their bodies do; the texture managers `Init` fetches are service lookups, not subscriptions; the
descriptor from slot 101 goes to `+0x26C`.*

A network occupant (§2) with 25 more main slots. It has no factory and no class ID of its own (its serialisation
slot 5 is pure); the pipe and subway are its concrete classes.

| Item | Value |
|---|---|
| `QueryInterface` (`0x0065A080`) | `0x4A10AFC2`, `0xA821EF94` and `0xA9764C8C` give the main interface, counted; others go to the occupant base, then the model instance base |
| Main vtable | `0x00AAAD40` (106 slots: the base 81 plus 25; 101, 102 and 105 are pure, `0x005D4A10`, the subclasses fill them) |
| Other vtables | `+0x04` `cISC4Occupant` `0x00AAACB8`; `+0x08` `cIGZSerializable` `0x00AAAC9C` (Write `0x0065B140`, Read `0x0065E510`); `+0x0C` property holder `0x00AAAC38` |
| Constructor | `0x0065C330` (the network constructor, then the defaults below) |
| Destructors | body `0x0065C0C0`, deleting `0x0065C650` |
| Virtual base | `+0x274` (moved by the subclasses to `+0x27C` and `+0x2D4`) |

Fields past the network occupant's (§2.2):

| Offset | Type | Field |
|---|---|---|
| `+0x178` | binding | the face texture |
| `+0x17C` | u32 | the number of face lists in use (0–4) *[inferred]* |
| `+0x180`, `+0x18C`, `+0x198`, `+0x1A4` | 4 vertex lists | the model's faces, drawn as quads (§3.2) |
| `+0x1B0` | binding | the cap texture |
| `+0x1B4` | vertex list | the cap: the four corners (slot 90) |
| `+0x1C0` | vertex list | the highlight geometry, drawn in the self-lit pass |
| `+0x1CC`…`+0x1E0` | 3 × 2 bindings | the wall textures: normal, distressed (flag `0x10000`), burst (`0x20000`) (slot 104) |
| `+0x1E4`, `+0x1F0` | 2 vertex lists | the walls (slot 89); not saved |
| `+0x1FC`…`+0x23B` | 16 × f32 | the mesh transform, a 4 × 4 matrix (identity; slot 85) |
| `+0x23C`, `+0x240` | 2 × s32 | the cell (from the position through the city's `PositionToCell`, slot 95, `+0x17C`) |
| `+0x244` | pointer | the mesh (a model resource, interface `0x9AD0E7E3`) |
| `+0x248`, `+0x249`, `+0x24A` | 3 × bool | the quad's diagonal, "hide the first triangle", "hide the second triangle" (slot 88) |
| `+0x24C`…`+0x258` | 4 × f32 | the corners' **original** heights (the terrain heights at `Init`; slots 94, 95) |
| `+0x25C` | f32 | the ceiling height (§3.2) |
| `+0x260`, `+0x261` | 2 × bool | refresh pending, model rebuild needed (slot 100, §3.2) |
| `+0x262` | bool | cleared by `Read`; the subway builds its model before its paths while it is clear |
| `+0x264` | u32 | the layer's class ID (`0x49C05B9F` pipe, `0xCA16374F` subway), given to `Init` (slot 82) |
| `+0x268` | u32 | opaque saved model-key word, returned unchanged by slot 96 (§3.6.3) |
| `+0x26C` | pointer | the static model descriptor (slot 101) |

A vertex list is a vector of the 24-byte vertices of §2.2 (begin/end/capacity).

Slots (main unless noted):

| Slot | Address | Behaviour |
|---|---|---|
| 10, 11 | `0x0065A900`, `0x0065A940` | the base `Set` / `ClearNetworkFlag`, and "refresh pending" `:= true` when the bits `0x30000` change |
| 21 `IsPlaced` | `0x00648470` | in the occupant manager, and a mesh or a descriptor is present |
| 30 `SetQuadDraw(piece, rotation)` | `0x0065ADB0` | stores both, then the ceiling (§3.2); no flags, no texture coordinates |
| 33 `GetModelInstances` | `0x00659D70` | `out[0] :=` the model instance (counted), count 1; the translation is left alone. Also slot 36 of the connections |
| 78 | `0x0065CF20` | the base's slot 78, except that each cached texture is released and loaded again, as instance (its cache key + the new zoom offset) |
| 80 `ComputeBox` | `0x00659E00` | the box of the corners and the model's vertices *[summary]* |
| 81 | `0x0065C6B0` | `Init(corners, classID, descriptor)` (below) |
| 82 | `0x00648460` | the layer's class ID |
| 83 | `0x00657730` | nothing (3 arguments) |
| 84 | `0x0065AE30` | `SetCornerHeights(h[4])`: the corners' y (`+0xA0`, `+0xB8`, `+0xD0`, `+0xE8`); the ceiling; slots 89 and 90 with true |
| 85 | `0x0065B700` | the mesh transform `+0x1FC` from the rotation, flip and position (`GetObjectYRotationAngle`) *[summary]* |
| 86 | `0x0065ADD0` | `Rebuild(s)`: the ceiling, slot 85, slot 87 (rotation, flip, `s`), slot 89 (`s`), slot 90 (`s`) |
| 87 | `0x0065D810` | `CreateUndergroundModel(rotation, flip, s)` (§3.2; NO40) |
| 88 | `0x0065A3F0` | the quad's triangles (§3.2) |
| 89 | `0x0065E260` | `CreateWalls(s)` (§3.2; NO40) |
| 90 | `0x0065E410` | `BuildCap(s)`: the cap list `:=` the four corners with white colour and their own alpha; with `s`, slot 100 with true |
| 91 | `0x00648F10` | nothing |
| 92 | `0x0065A6F0` | `GetQuadBools(&diagonal, &hideFirst, &hideSecond)` |
| 93 | `0x0065AA70` | `GetCeiling(&h, &flat, side)` (§3.2) |
| 94, 95 | `0x00659DD0`, `0x0065B6B0` | get / set the original heights; the set recomputes the box (slot 80) and the ceiling (`0x0065AB50`) |
| 96 | `0x0065A810` | `GetModelKey(&piece, &value, h[4], &ceiling)`: the piece id, `+0x268`, the corners' y, the ceiling |
| 97 | `0x00659DB0` | `SetCeilingParameters(depth, useOriginal, lift)`: the three statics of §3.2 (shared by every underground class) |
| 98 | `0x0065A120` | `CountRun(o, side, depth)` (§3.2) |
| 99 | `0x0065A200` | `GetZipperPieceType(&side)` (§3.2) |
| 100 | `0x0065AF10` | `Refresh(propagate)` (§3.2) |
| 103 | `0x0065B020` | `GetTint(out)`: (1, 1, 1) |
| 104 | `0x0065A880` | `GetWallTextures(&a, &b)`: the burst pair with flag `0x20000`, else the distressed pair with `0x10000`, else the normal pair (plumbing.md cites `0x0065A880`) |
| occupant 8 `GetPosition` | `0x0065A720` | §3.2 |
| occupant 15 `PostOccupantMessage(type, v)` | `0x0065A980` | the base's; then for Insert `0x99EF1142` and Remove `0x99EF1143` also `0x8B87A974` / `0x8B87A975` with the base's layout (flora.md §9.1), posted |
| drawable 17 `GetSortValue` | `0x0065A0E0` | the piece id (`+0xFC`, read as `[this − 0x178]` from the drawable at `+0x274`) with any of flags `0x06000000`, else `0x4A10AFC2` (answers NO7) |

**Life cycle.**
- **Occupant slot 3 `Init()`** (`0x0065C070`, the pipe and subway `Init` call it): the static set-ups once (below,
  then the network occupant's when it has not run); unless the occupant is initialised, the network occupant's
  `Init` (§2.5), then the descriptor `+0x26C :=` slot 101 (`+0x194`; stored at `this + 0x268` with `this = object+4`).
  True.
- **Main slot 81 `Init(corners, classID, descriptor)`** (`0x0065C6B0`): the static set-ups; unless initialised:
  1. the layer's class ID and the descriptor are stored; the original heights `:=` the corners' y;
  2. the network occupant's `Init(corners)` (§2.5);
  3. the corners' texture coordinates `(0, 0)`, `(0, 1)`, `(1, 1)`, `(1, 0)` (`+0xAC`…`+0xF8`);
  4. the cell from the occupant's position (occupant slot 8) through the city's slot 95;
  5. the identity mesh transform; the three quad bools false; the descriptor `:=` slot 101;
  6. the cap texture `{type 0x7AB50E44, group 0x891B0E1A, instance =` descriptor word 35`}` through the texture
     manager (system service `0x1AC0E11A`, interface `0xFAC0E219`, slot 6) into `+0x1B0`; the face texture into
     `+0x178` *[summary]*;
  7. the face count, the six wall bindings, the lists and both bools of slot 100 cleared; the box recomputed;
  8. the descriptor's ratio tables recomputed (§3.2). True.
  The pipe and subway `Init(arg)` call it with their class IDs and descriptors (§3.3, §3.5).
- **`Shutdown`** (main slot 4, `0x0065CD90`): release the eight texture bindings and the mesh, empty the eight
  lists, then the network occupant's `Shutdown`.
- **Static set-up** (`0x0065AE80`, once; flag `0x00B46870`): when the renderer (`0x00B43DD0`) exists, its slot 10,
  then three packed states (draw-context.md §5.1 order): **D** `0x00B46878` = (1, 1, 1, 0, 0, 1, 0, 3, −1, −1, −1,
  −1.0, 1, −1); **E** `0x00B46880` = (1, 1, 1, 0, 1, 1, 0, 3, 4, 5, −1, −1.0, 1, −1); **F** `0x00B46888` = (1, 1, 1,
  0, 1, 0, 0, 3, 4, 5, −1, −1.0, 1, −1).

### 3.2 Underground behaviour *[compat]*

**The quad's triangles** (main slot 88, `0x0065A3F0`). The network occupant draws its textured quad with flags
`0x1000000` (the other diagonal), `0x2000000` (first triangle) and `0x4000000` (second triangle) (§2.11). This slot
sets the three bools from the piece id and rotation byte and then the flags from them: diagonal → `0x1000000` set,
else cleared; "hide first" → `0x2000000` cleared, else set; "hide second" → `0x4000000` cleared, else set.

| Piece id | Bools (diagonal, hide first, hide second) |
|---|---|
| `0x200` | by the rotation byte: 0 or `0x83` → (0, 1, 0); 1 or `0x82` → (1, 1, 0); 2 or `0x81` → (0, 0, 1); 3 or `0x80` → (1, 0, 1); any other byte leaves them |
| `0x800`, `0xA00`, `0xC00`, `0x1400`, `0x1E00` | with `r :=` the rotation (slot 24): diagonal `:=` byte `r` of `0x00AAAB9C` = (1, 0, 1, 0); the mode word `r` of `0x00AAABA0` = (1, 0, 0, 1): 0 → hide (first, not second), 1 → hide (second, not first); then, when the piece is flipped (slot 25), all three are inverted |
| any other | (0, 1, 1): no quad |

So an underground piece shows the surface quad only over the half-cells that one of these pieces leaves open, and its
sort value is its piece id exactly then.

**The ceiling height** (`ComputeModelYPosition`, `0x0065AB50`). Three statics (slot 97): `depth` 3.0
(`0x00B0CFD0`), `useOriginal` true (`0x00B0CFD4`), `lift` 10.0 (`0x00B0CFD8`). With `h_k` the corners' y and `o_k`
the original heights:
- pieces `0x200`, `0x800`, `0xA00`, `0x1400`: corners `(s, i, j)` by the rotation byte: 0 or `0x81` → (2, 1, 3);
  1 or `0x80` → (1, 0, 2); 2 or `0x83` → (0, 1, 3); 3 or `0x82` → (3, 0, 2). With `useOriginal`:
  `min(o_s, (o_i + o_j) × 0.5) − depth`; otherwise `min(h_s, (h_i + h_j) × 0.5) + lift`. The minimum keeps `o_s` (or
  `h_s`) when the comparison is unordered.
- pieces `0x300`, `0x4B00`: with `useOriginal`, the position's y (the lowest original height) `− depth`; otherwise
  the mean of the four `h_k` `+ lift`.
- any other piece: `h_0 + lift`.

`GetCeiling(&h, &flat, side)` (slot 93): `h :=` the ceiling and `flat := 0`; for pieces `0x200`, `0x800`, `0xA00`,
`0x1400` and `0x14E00`, `flat := 1`; for `0x300` and `0x4B00`, `h := (h_a + h_b) × 0.5 + lift` over the corners of
side `side` (0 → 0, 1; 1 → 0, 3; 2 → 3, 2; 3 → 1, 2).

**The descriptor's ratio tables.** `Init` recomputes, for every occupant, the tables that the pipe's and subway's
static set-ups fill (§3.3, §3.5): with `n :=` descriptor word 25, the points table word 14 and the index lists words
15 and 17, `L_k := ‖p[a_k] − p[b_k]‖` (x and z) for `k < n`; table **T1** (word 18) gets `T1[0] := 1` and
`T1[k] := L_k / (L_{n−k} + L_k)` for `0 < k < n`; table **T2** (word 23) gets `T2[4] := 1` (a fixed index; `n` is 5
in both descriptors) and `T2[n−1−k] := T1[k]`.

**`GetPosition`** (`0x0065A720`, on the `+4` view): the out vector starts as (0, `FLT_MAX`, 0). x and z are the
**average of the four corners** (NO17 f): x = `((v0.x + v1.x) + v2.x) + v3.x` then `× 0.25` (constant `0x00A8821C`),
z the same with the z fields; corner k is at object `+0x9C + 0x18·k` (x) and `+0xA4 + 0x18·k` (z), each partial sum
stored as f32. y = the minimum of the four original heights `+0x24C`, `+0x250`, `+0x254`, `+0x258`, compared in that
order with a strict `<` against the running value (seeded `FLT_MAX`, `0x00AAABB0`), so a NaN height never wins and
all-`FLT_MAX` heights give `FLT_MAX`. Example: corner x 0/16/16/0, z 0/0/16/16, heights 3, 2.5, 4, 2.5 → (8, 2.5,
8).

**Neighbours.** The side offsets (`0x00B0CFE8`) are side 0 `(−1, 0)`, side 1 `(0, −1)`, side 2 `(+1, 0)`, side 3
`(0, +1)` in cells. `FindUndergroundOccupant(x, z, &o, classID)` (`0x006584D0`) and
`FindUndergroundConnectionOccupant` (`0x006585E0`) are the underground tool's lookups (network-manager.md).
- **`IsImaginaryConnection(side)`** (`0x0065A640`, NO17 e): if the piece's own flags (primary slot 12) have bit 31
  (`0x80000000`, a draft) **clear** it returns false at once. Otherwise it looks for an underground occupant of its
  own class ID (`+0x264`) at cell (`+0x23C` + dx[side], `+0x240` + dz[side]); not found → false; found → true iff
  the neighbour's flags have bit 31 clear (the neighbour reference is released). The side indexes the table with no
  bounds check, so a side above 3 reads the bytes after it *[quirk]* (answers NO20).
- **`CountRun(o, side, depth)`** (slot 98): 0 unless `o`'s piece id is `0x4B00` and there is no connection occupant
  of this layer at `o`'s cell; otherwise 1, plus, when `depth > 0` and `o` has a neighbour across `side` of the
  layer, that neighbour's `CountRun(…, side, depth − 1)`.
- **`GetZipperPieceType(&side)`** (slot 99): only for a subway (`+0x264 = 0xCA16374F`) piece `0x4B00` with no
  connection occupant at its cell; otherwise 0. With `r` the rotation, the sides `a := (r + 3) mod 4` and
  `b := (r + 1) mod 4`, each turned by 2 more when the piece is flipped and `r` is odd; `na`, `nb :=` the
  neighbours' `CountRun(…, a or b, 4)` (0 without a neighbour). The result is 2 when both are above 1; 1 with
  `side := 1` when `na = 1` and `nb > 2`; 1 with `side := 0` when `nb = 1` and `na > 2`; otherwise 0 (UG3).

**`Refresh(propagate)`** (slot 100, `0x0065AF10`):
1. When the city does not accept the cell (city slot 99), both bools (`+0x260`, `+0x261`) are cleared and nothing
   else happens.
2. When "refresh pending" is already set, nothing happens.
3. Set "refresh pending" and "model rebuild needed"; recompute the box; when the occupant is visible (its flag 2),
   the renderer (`0x00B43DD0`) slot 31 with the model instance and 3.
4. With `propagate`: for each side 0–3 that `ConnectsToNeighbor(side, network type)` (slot 47, type from slot
   102), the neighbour of the same layer across it gets `Refresh(true)`.
The pending flag stops the recursion; the draw pass clears it.

**Model and drawing** *[summary; NO40]*:
- `CreateUndergroundModel(rotation, flip, s)` (slot 87): empties the lists, releases the six wall textures and the
  mesh, loads the three wall texture pairs through the texture manager, builds the face lists of the piece
  (`MakeCustomUndergroundPiece`, `0x0065D160`, which looks at the neighbours and their ceilings, slot 93), loads the
  mesh (resource manager slot 3, interface `0x9AD0E7E3`) and the cap and face textures.
- `CreateWalls(s)` (slot 89): rebuilds the wall lists with quads (`AddWallQuad`, `0x0065CFF0`) on the sides that have
  no neighbour, no connection occupant and no imaginary connection; with `s`, slot 100.
- `Draw(context, selfLit)` (`0x0065B290`): when "model rebuild needed" is set and the city accepts the cell, slot 87
  rebuilds the model with the rotation and flip and the bool is cleared. Pass 0 (the context's `+4 = 0`): the cap
  (render state D, the cap texture, wrap (2, 2, 0), filtering (1, 7, 0), primitive type 6, quads) and each face list
  (state D, the face texture, quads); the result is 2. Pass 1, unless the piece is a lot connection node: the tint
  (slot 103) as the shade; the two wall lists with the wall textures of slot 104 (state E, type 1, strips); the
  mesh, when present, with the mesh transform, its render states forced to blending (bit `0x10`, factors 4, 5),
  culling off for a flipped piece; then the shade is reset and "refresh pending" cleared. The self-lit pass draws the
  highlight list with state F (strips). Primitive types: view-utilities.md §7.1.
- **The occupant type** is the network occupant's `0xC772BF98`: the underground constructor runs the network
  constructor first (`0x0065C354`), which writes it (§2.1), and nothing later changes it (quirk 1; answers NO3).

### 3.3 `cSC4PipeOccupant` *(0x49C05B9F)*

Layout: `+0` primary `0x00AA97B8` (110 slots), `+4` `0x00AA9730`, `+8` serializable `0x00AA9714` (Write
`0x00648800` v3 / Read `0x00648840` v3 / `GetGZCLSID` `0x00648F40`), `+0xC` property holder `0x00AA96B0`.

`Init` (the vtable slot 3 body `0x00648BF0`; a second `Init` variant `0x00648BC0` exists on the occupant view):
`StaticInit` once `0x00648A90` (wall-slope ratios from the tables `0x00B0BA68`/`0x00B0BC60`/`0x00B0BC88` into
`0x00B46708…0x00B4672C`, ratio `d[i]/(d[i]+d[j])` pattern); `cSC4UndergroundOccupant::Init` (`0x0065C6B0`) with
`(p, classID 0x49C05B9F, param4 0x00B0BED0)`; `+0x270 = 0`.

Record: u16 **3**, the underground record (§3.4, with its own u16 3), then u32 object `+0x270` (code `+0x268`); `Read` accepts exactly 3, reads the same and then sets "refresh pending" (object `+0x260`, code `+0x258`) to 1 (§3.2). The
meaning of `+0x270` is not established (NO6, §12).

`Shutdown` is **shared with the subway**: both classes' slot 4 is `0x00652CF0` (5 bytes, tail to the underground
`Shutdown`) *[quirk: one folded body for two classes]*.

`SetWateredState` is a **non-virtual** member `0x00648540` (direct-calls.md; plumbing.md §9): stores the state and,
when it changes, tail-calls primary slot 100 (`Refresh`, `0x0065AF10`, §3.2). The water simulator calls
it directly (plumbing.md §6.4).

### 3.4 The underground save record *[compat]*

`Write` `0x0065B140` / `Read` `0x0065E510`, in the `+8` vtable with `this = object+8` like the base record (§2.2
"Offsets in the record"); the offsets below are **object** offsets (code offset + 8). The record is its **own
u16 version 3 followed by the complete base network record** (§2.3, with its own u16 8), then the underground tail
(NO17 a):

| # | Field | Type | Object offset |
|---|---|---|---|
| 1 | version | u16 = **3** | `Read` accepts exactly 3; anything else returns 0 having read only the u16 |
| 2 | the base network record | §2.3 | `cSC4NetworkOccupant::Write`/`Read`; on failure the tail is skipped |
| 3 | the draw attributes' first two bytes | u8, u8 | `+0x50`, `+0x51` |
| 4 | the opacity | f32 | `+0x60` |
| 5 | the face count | u32 | `+0x17C` |
| 6 | six vertex lists: the four faces, the cap, the highlight | 6 × list | vectors (begin/end/capacity) at `+0x180`, `+0x18C`, `+0x198`, `+0x1A4`, `+0x1B4`, `+0x1C0` |
| 7 | the mesh transform | 16 × f32 | `+0x1FC`…`+0x23B`, row by row (`operator<<` `0x005DA6E0` / `operator>>` `0x005DA6B0`) |
| 8 | the cell | s32, s32 | x `+0x23C`, z `+0x240` (`SetSint32`, slot `+0x20`) |
| 9 | the three quad bools | u8 × 3 | `+0x248`, `+0x249`, `+0x24A` (`operator<<(u8)` `0x005D9FD0`) |
| 10 | the four original heights | 4 × f32 | `+0x24C`, `+0x250`, `+0x254`, `+0x258` |
| 11 | the ceiling height | f32 | `+0x25C` |
| 12 | the layer's class ID | u32 | `+0x264` (the field `GetZipperPieceType` and `IsImaginaryConnection` read) |
| 13 | one word | u32 | `+0x268` (NO41) |

A **vertex list** (helper `0x0065B070` write / `0x006574A0` read, NO17 c) is a u32 count = `(end − begin) / 0x18`
(magic division `0x2AAAAAAB`, `SAR 2`) followed by `count` vertices in the corner form of §2.3 field 4: f32 x, y,
z, u, v (element `+0`, `+4`, `+8`, `+0x10`, `+0x14`), then the colour bytes `+0xE`, `+0xD`, `+0xC`, `+0xF` (R, G, B,
A); 24 bytes per vertex in the stream and in memory. The reader resizes the vector to `count` first (`0x005E1B30`)
and does **not** force the alpha byte. The face and cap textures, the wall lists and wall textures and the mesh are
not saved: they are rebuilt.

After the tail `Read` sets "model rebuild needed" `+0x261 = 1` and clears `+0x262` (code `+0x259`/`+0x25A`) and
returns 1 iff the stream's error flag is clear. The tail after the base record is 2 + 4 + 4 + 6 × (4 + 24·n) + 64 +
8 + 3 + 16 + 4 + 4 + 4 = 137 + 24·(total list vertices) bytes.

Worked example (the stream after the base record, little-endian): bytes `+0x50 = 1`, `+0x51 = 0`, `+0x60 = 2.0`,
`+0x17C = 7`, six empty lists, an identity matrix, cell (12, 34), bytes 0/0/0, heights 1.0/1.5/1.0/1.5,
`+0x25C = 0`, class `0x49C05B9F`, `+0x268 = 0` give
`01 00 | 00 00 00 40 | 07 00 00 00 | 00 00 00 00` ×6 `| 00 00 80 3F, 00 00 00 00 ×3, ... (16 floats) | 0C 00 00 00
22 00 00 00 | 00 00 00 | 00 00 80 3F 00 00 C0 3F 00 00 80 3F 00 00 C0 3F | 00 00 00 00 | 9F 5B C0 49 | 00 00 00 00`
= 137 bytes.

### 3.5 `cSC4SubwayOccupant` *(0xCA16374F)*

Layout: `+0` primary `0x00AAA3C8` (112 slots), `+4` `0x00AAA340`, `+8` `0x00AAA324` (Write `0x00653010` v2 / Read
`0x00653040` v2: u16 **2**, the underground record (§3.4), **no** extra field; `Read` accepts exactly 2, then sets "refresh pending" (object `+0x260`, code `+0x258`) to 1), `+0xC` `0x00AAA2C0`.

`Init` `0x006537A0`: `StaticInit` `0x00653640` (ratios from `0x00B0C960`/`0x00B0CB58`/`0x00B0CB80` into
`0x00B467F8…0x00B4681C`); `cSC4UndergroundOccupant::Init` with `(classID 0xCA16374F, param4 0x00B0CDC8)`.
`GetPathInfo` has a subway override `0x006537E0` (522 bytes — the subway-specific path build). Everything else is
the underground base.

### 3.6 The underground connection occupant *(cSC4UndergroundConnectionOccupant, 0x8A139184)*

*Rewritten 2026-10-21 (re-ugbase): `Read` is `0x00657590` (`0x00659D70` is the underground `GetModelInstances`);
`Init(classID, descriptor)` (main slot 34) begins at `0x00654710`, and the second arguments its callers pass
(`0x00B0B6D0`, `0x00B0C388`) are the classes' static model descriptors, not strings (answers NO8); offsets are
object offsets (the record code runs with `this = object+4`).*

The constructor `0x00654F00` derives **directly from `cSC4Occupant`** (no network-occupant base). An occupant and a
model instance; object `0x250` bytes. Class ID `0x8A139184` (`GetGZCLSID` `0x00654E70`); `QueryInterface`
(`0x00654030`) answers `0x8A139184` and `0xA9764C8C` itself, others go to the occupant base, then the model instance
base.

| Offset | What |
|---|---|
| `+0x00` | the occupant interface (main vtable `0x00AAA840`, 45 slots: the occupant's 0–33, then 34–44) |
| `+0x04` | serialisation (`0x00AAA824`: Write `0x00654C50`, Read `0x00657590`, `GetGZCLSID` `0x00654E70`) |
| `+0x08` | the property holder (`0x00AAA7C0`, 25 slots) |
| `+0x1C`, `+0x34` | small interfaces (`0x00AAA7A8`; `0x00AAA798` the message target) |
| `+0x38` | the model instance part (`cS3DModelInstanceBasic`); the virtual base at `+0x24C` (`0x00AAA748`) |

No stored occupant type: the subclasses set the occupant type `+0x24` (what occupant slot 7 `GetType` returns)
themselves, before the base `Init`: `0x2A183167` for the pipe connection and `0x8A13918A` for the subway connection
(answers UG4).

| Offset | Type | Field |
|---|---|---|
| `+0x68` | u32 | number of active wall lists (0–4); saved |
| `+0x6C`, `+0x78`, `+0x84`, `+0x90`, `+0xA0`, `+0xAC`, `+0xBC` | 7 vertex lists | the connection's model *[summary]* |
| `+0xC8` | pointer | the **attached** underground occupant (the one at this cell), counted |
| `+0xCC` | pointer | a **linked** occupant (slot 40), counted |
| `+0x150`, `+0x154` | 2 × s32 | the cell |
| `+0x160`…`+0x16C` | 4 × f32 | four heights |
| `+0x170` | f32 | model's upper anchor elevation; saved |
| `+0x174` | f32 | creation-time position y minus the cell size; not a separate record field |
| `+0x178` | s32 | self-lit shade-table index, initially 1; saved |
| `+0x17C` | bool | the model state has been taken into account |
| `+0x180`, `+0x18C` | 2 × 3 × f32 | the box (slot 42) |
| `+0x230` | 3 × f32 | the position (occupant slots 8 `GetPosition`, 9 `SetPosition`) |
| `+0x240` | u32 | creation class ID; saved |
| `+0x244` | bool | set by `Read` |

**Record** (`Write` `0x00654C50`, `Read` `0x00657590`):

| # | Field | Encoding |
|---|---|---|
| 1 | version | u16 **2** |
| 2 | the occupant base record | lot-records.md §2.3 |
| 3 | `+0x68` | u32 |
| 4 | the seven vertex lists, in order | as §3.4 |
| 5 | the cell | s32 x, s32 z |
| 6 | the four heights, `+0x170` | 4 × f32, f32 |
| 7 | `+0x178` | s32 |
| 8 | `+0x17C` | `bool` |
| 9 | the attached occupant | `SC4WriteGZSerializable` ref (its serialisation interface); read as the occupant interface `0x26BB767E`, stored 4 bytes lower (the network occupant's main object) |
| 10 | the linked occupant | ref, read as `0x26BB767E` and stored as is |
| 11 | `+0x240` | u32 |
| 12 | the position | 3 × f32 |

`Write` stops after row 2 when the base fails. `Read` returns false for any version other than 2 before the base;
after the rows it sets `+0x17C` and `+0x244` to true, so the saved row 8 is never used *[quirk]*. Both return true
exactly when the stream has no error. Both classes appear in saves by class ID (save-records.md §3.2: subway
connection `0xCA15ED6B` in 1 save, pipe connection `0xEA0A5ACF` in 4).

*[compat; UG5 record clarification, 2026-10-23]* The seven list writes all use the §3.4 wall helper
(`0x0065B070`, calls at `0x00654C81–0x00654CC0`); all seven reads use `0x006574A0`
(`0x006575D9–0x00657618`). Each is a u32 count followed by that many 24-byte vertices in the helper's
field order, not a byte-counted opaque mesh. Record methods receive **primary+4**. Thus the attached
ref's instruction displacement +C4 is live +C8, and the linked ref's displacement +C8 is live +CC.
Both reads request IID `0x26BB767E`; only the attached nonnull result is adjusted by −4 before storage
(`0x00657693–0x006576D4`). A null attached result stays null. These are the same live references released
by Shutdown, not two additional raw reference fields. The exact-version comparison is at `0x006575A7`.

**Behaviour.**
- `Init()` (main slot 3, `0x00654670`): once (latch `0x00B46838`), hold the resource manager (`0x00B43CA8`, at
  `0x00B46830`) and register a static shutdown (`0x00653DA0`) with the application (`0x00B43C94`, slot 69). Unless
  initialised: the occupant base `Init`, the render state set-up (`SetupPackedRenderState`, `0x00654260`), and
  subscriptions of the message target (`+0x34`) to `0x8B87A974`, `0x8B87A975`, `0x8B8B2CEE` and `0xA6B79602`
  (message server slot 5).
- `Init(classID, descriptor)` (main slot 34, `0x00654710`, 1,341 bytes; the tools' creation path): the same
  one-time set-up and base `Init`; the cell and heights from the position through the city (`PositionToCell`; y from
  the cell size `0x00B0CF9C`); `SetupPackedRenderState`; `+0x178 := 1`, `+0x17C := 1`; the attached occupant
  (`FindUndergroundOccupant` `0x006584D0` at its own cell, else the 8-neighbour loop with `FindUndergroundOccupant` /
  `FindUndergroundConnectionOccupant` `0x006585E0` and the network manager's connections, manager slot 0x14); then
  the same four subscriptions. The pipe connection's `Init` (`0x00648050`) sets its type and calls it with
  `(0xEA0A5ACF, 0x00B0B6D0)`; the subway connection's (`0x00652A80`) with `(0xCA15ED6B, 0x00B0C388)`.
- `Shutdown` (`0x006553A0`): releases the three texture bindings and the lists, the attached and linked occupants,
  removes the four subscriptions, then `cSC4Occupant::Shutdown`.
- `PostOccupantMessage` (`0x00654180`): as the underground occupant's (§3.1).
- `GetHiddenTriangles(&a, &b, &c)` (`0x006540A0`): (0, 1, 1), as an underground piece without a quad.
- `ModelStateChanged` (`0x006542F0`): when `+0x17C` is false, set it, recompute the box (slot 42) and, when visible,
  the renderer's slot 31 with the instance and 3.
- Main slot 40 (`0x00654620`): `SetLinkedOccupant(o)`: holds `o`; with null, the connection removes itself from the
  occupant manager (its slot 21 with 1, 0).
- Main slot 43 (`0x00654160`): the attached occupant's `Refresh(true)` (§3.2).
- `DoMessage` (`0x00656C80`): `0x8B87A974` (an underground occupant inserted) attaches the occupant of this layer at
  the connection's cell and calls `ModelStateChanged`, or, when the message is the connection's own, rebuilds the
  attached occupant's walls (slot 89); `0x8B87A975` (removed) detaches it; `0x8B8B2CEE` for the linked occupant
  forgets it and removes the connection from the occupant manager; `0xA6B79602` with a changed value rebuilds the
  geometry (`CreateGeometry`, `0x006562B0`).
- `Draw` (`0x00656E90`): builds the geometry when needed (`CreateGeometry`: `CreatePieceModel` `0x00655850`,
  `CreateWalls` `0x00655F10`, `CreateFloor` `0x00656010`, from the zoom and rotation; `AddWallQuad` `0x006555E0`),
  then draws the lists with two packed states (`0x00B46864`, `0x00B46854`) (NO40).
- Position accessors `0x006540C0` / `0x006540F0` copy the three words at +230 to/from the caller's vector;
  SetPosition alone does not rebuild geometry, update the cell or notify the renderer. `0x00648360` is the
  pipe connection's QueryOccupantInterface, **not GetTransform** (underground-occupants.md §4).
- `GetModelInstances` (main slot 36, `0x00659D70`) writes count 1 and the model-instance virtual-base pointer
  selected through +3C's adjustment table, and AddRefs the primary object. The first two and fourth arguments
  are unused; it is not a stream reader. It assumes a live object. `0x00655520` (main slot 41) instead reloads
  the shared texture bindings for the draw context's zoom: record that zoom, release/null each old binding,
  then request key `(0x7AB50E44, 0x1ABE787D, entry-key + zoom)` into the binding's slot.

#### 3.6.1 Creation and owner-state details *[compat; UG5 partial]*

The creation Init stores the supplied class ID at +240 and the descriptor pointer at +23C. On an already
initialized object it returns true without rebuilding. Before a fresh initialization it releases/nulls both
occupant refs unless +244 says this is a loaded object; the attached ref is nevertheless released/nullified
again before the own-cell search. A preserved linked ref is not cleared by that search. The creation path's
actual entry is `0x00654710`; its export splits off the first 13 bytes from the body at `0x0065471D`.

It obtains the position, converts x/z to cell coordinates through the city, saves position y as +170 and
f32(position y − cell size) as +174, and obtains four cell-corner heights. Let the city's CellToPosition
result be `(x0,z0)`, its x/z cell extents truncated to integers be `(sx,sz)`, the cell size be S, and those
four heights be h0…h3. The four surface vertices beginning at +1D0 have positions/UVs:

| Corner | Position | UV |
|---|---|---|
| 0 | `(x0, f32(h0−S), z0)` | `(0,0)` |
| 1 | `(x0, f32(h1−S), f32(z0+sz))` | `(0,1)` |
| 2 | `(f32(x0+sx), f32(h2−S), f32(z0+sz))` | `(1,1)` |
| 3 | `(f32(x0+sx), f32(h3−S), z0)` | `(1,0)` |

It also resets the two 4×4 matrices at +D0 and +110 to identity, sets +178=1 and +17C=true and sets up the
packed states. Numeric sources: `0x006547DE–0x006549F3` (f32 height stores, integer cell extents).
These are creation inputs/surface vertices, not a complete description of CreatePieceModel/CreateGeometry.

The own-cell search requests the class returned by main slot 38. An own-cell hit is retained and refreshed
through main slot 43; no neighbor search occurs. Otherwise the eight neighbor offsets are visited in this
exact order: `(-1,0), (0,-1), (1,0), (0,1), (-1,-1), (1,-1), (1,1), (-1,1)`.
For each cell an underground-occupant search comes first, then, only on failure, a connection search.
The first successful search suffices for that cell. Neighbor results are released immediately, never kept
as the attached occupant. On a hit, a fresh object (+244=false) obtains the network interface selected by
main slot 39, AddRefs it, begins a change (+0C), requests a connection (+1C) with arguments
`(ownX,ownZ,neighborX,neighborZ,0,0,1,−1)`, ends the change (+10) and Releases it. A loaded object still
performs the neighbor queries/releases but does not request these connections. After all eight cells it
reacquires its own-cell occupant, refreshes it if present, computes the bounding box and subscribes the four
messages. A neighbor miss does not terminate the loop. Sources: `0x00654A20–0x00654C43`; offset table
`0x00654A54–0x00654A98`, loaded gate `0x00654B30`.

ModelStateChanged consumes a false +17C exactly once: set true, calculate bounds into +180/+18C, then, if
occupant visibility bit 1 at +2F is set, notify renderer +7C with the adjusted model instance and argument 3.
An already-true flag produces no bound calculation/notification (`0x006542F0–0x0065433B`). SetLinkedOccupant
compares pointers first; an equal pointer, including null→null, does nothing. For a changed pointer it
Releases the old, stores the new, then AddRefs a nonnull new pointer. A changed pointer to null instead
removes this connection through occupant manager +54 with `(this,1,0)` (`0x00654620–0x00654665`).

The static shutdown callback Releases/nulls the held resource manager and returns true even if already
null. It **does not clear the one-time latch** (`0x00653DA0–0x00653DBB`), so calling the callback then Init
does not reacquire that static resource through this latch.

SetupPackedRenderState does nothing when the renderer is null. Otherwise it calls renderer +28 and
discards its result, then packs these three tuples in argument order to `PackRenderState`
(`0x00654260–0x006542EE`; float argument is exactly −1.0):

| State address | Positional arguments |
|---|---|
| `0x00B46854` | `(1,1,1,0,0,1,0,3,−1,−1,−1,−1.0,1,−1)` |
| `0x00B4685C` | `(1,1,1,0,1,1,0,3,4,5,−1,−1.0,1,−1)` |
| `0x00B46864` | `(1,1,1,0,1,0,0,3,4,5,−1,−1.0,1,−1)` |

#### 3.6.2 Unattached connection floor and walls *[compat; NO40/UG5 partial]*

These builders use the four surface vertices at +1D0 (stride 24) and the four terrain heights +160,
not the pole's floor or the underground piece's ratio tables. CreateFloor (`0x00656010–0x00656149`)
first clears +A0. With an attached occupant at +C8 it leaves that list empty. Without one it copies
the four surface vertices in order, retaining position/UV/alpha and replacing RGB with white, then
invokes ModelStateChanged with the latch behavior in §3.6.1. Creation's height subtraction has already
happened in the surface inputs; this builder does not subtract another cell size.

CreateWalls (`0x00655F10`) clears the four lists at +6C/+78/+84/+90. Without an attached occupant,
it resets the wall count +68 to zero and builds all four ordered endpoint pairs `(1,0),(0,3),(3,2),(2,1)`,
one per list, then invokes ModelStateChanged. With an attached occupant the cleared lists stay empty;
the old wall count is retained and no ModelStateChanged follows. These are standalone builder rules,
not the descriptor-dependent CreateGeometry dispatch contract.

AddWallQuad (`0x006555E0–0x0065574E`) **appends** four vertices to its destination. For endpoints A/B,
top positions use surface x/z and terrain y; bottom positions use the complete surface positions.
Order is top A, bottom A, bottom B, top B. RGB is white; alpha comes from A for the first two and
B for the last two. UVs are `(0,0),(0,V),(1,V),(1,0)` with the **same**
`V=f32((terrainHeight[A]−surfaceY[A])×0.0625)` at both bottom vertices, even if B's drop differs.
Subtraction and multiplication remain on the x87 stack until the f32 V store; no f32 subtraction
temporary intervenes (`0x00655659–0x0065567F`, `0x0065569C–0x006556EE`). The wall builder clears
its lists before appending; the helper itself does not clear them.

#### 3.6.3 Saved auxiliary words *[compat; NO41]*

The underground base's +268 is an **opaque model-key word**: preserve all 32 bits on record round trips
and return them unchanged as GetModelKey's second output. It is not a descriptor pointer or an inferred
network enum. The descriptor is separately +26C: Init's apparent +268 store is relative to object+4
(`0x0065C0A8–0x0065C0B4`), whereas GetModelKey reads +268 relative to the primary object
(`0x0065A81C–0x0065A826`). No semantic conversion is justified by these methods. The connection's +68
counts the active wall lists: construction gives zero, unattached CreateWalls rebuilds it to four,
attached CreateWalls retains it despite clearing the lists (§3.6.2; `0x00655F5F`). Its +170 is the upper
model anchor height: initially position.y, subsequently refreshed from the linked occupant (or the
own-cell connection lookup) position in CreatePieceModel; +174 is the attached underground ceiling,
or +170 minus the cell size when unattached (`0x006559A6–0x006559E5`,
`0x00655AC1–0x00655B05`). +178 is the signed **shade-table index**, initialized to 1, used as
`0x00B0CFB0 + 12×index` for Draw's self-lit RGB; the saved value is retained without validation.
+240 is the class ID supplied to creation Init, independently of the occupant type and descriptor
(`0x0065476E–0x0065477C`, `0x00656E90–0x00657493`). Thus the opaque base word needs no invented meaning,
and the shade word must not be normalized to a boolean.

The subclasses differ in exactly three slots plus the type: the **pipe connection** ctor `0x006483E0` = the
underground-connection ctor + its own vtables (`0x00AA9578` primary etc.), `Init` `0x00648050`; the **subway
connection** ctor `0x00652C50`, `Init` `0x00652A80`. The `QueryInterface`s are `0x00648080` (pipe) and `0x00652AA0`
(subway); the subway's slot 44 `QueryOccupantInterface` `0x00652BD0` QIs `0xCA16374F` then `0x4A10AFC2` (§1.2). The
primary vtables are 45 slots (`0x00AA9578`, `0x00AAA188`) with the same slot order; only QI (0), Init (3),
GetGZCLSID (37), the folded 38 and slot 44 differ.
---

#### 3.6.4 Connection message decisions *[compat; UG5 partial]*

DoMessage receives the message-target view at primary+34 and always returns true, including for
unknown messages and all ignored payloads. Its four decisions are:

| Message | Payload and action |
|---|---|
| `0x8B87A974` | Read message +28's pointer. For a nonnull pointer accepted by main slot 44 (the layer-specific interface query), ask the returned occupant's slot 59 (+EC) for cell x/z. Only an exact match with live +150/+154 replaces attached +C8: Release old if present, store the query result, AddRef it, then ModelStateChanged. Release the query result afterward even on a cell mismatch. A successful interface query completes this branch; it never falls through to the own-pointer case. |
| `0x8B87A974`, interface query failed | If the original payload pointer equals the connection's **primary pointer**, and +C8 is nonnull, call the attached occupant's slot 89 (+164) with true. Null payload and other pointers do nothing. This is the own-connection insertion notification, not a new attachment. |
| `0x8B87A975` | Query the nonnull payload through main slot 44. If the returned pointer equals attached +C8, Release the stored reference, clear +C8, set +17C=true, get the connection position and store +174=`f32(position.y−cellSize)`. Release the query result regardless of equality. No ModelStateChanged, immediate geometry build or renderer notification is performed here. |
| `0x8B8B2CEE` | Only a nonnull payload exactly equal to linked +CC triggers action: Release it, clear +CC, remove this connection through occupant manager +54 with `(primary,0,0)`, invoke main slot 43 (refresh the attached occupant with true), then main slot 4 (Shutdown). Other pointers do nothing. |
| `0xA6B79602` | Fetch the two pointers through message +28 and +3C; compare their **u32 at +4**. Different values call CreateGeometry; equal values do nothing. The handler contains no null guards for those two context pointers and does not compare the pointers themselves. |

Replacement on insertion does not have an equal-pointer fast path: even reattaching the same
occupant performs Release, store, AddRef, ModelStateChanged, and the final temporary Release.
ModelStateChanged still has its independent +17C latch (§3.6.1), so inserting while that latch is
already true does not by itself rebuild geometry or notify the renderer. The removal branch does
not recompute +170; it changes the lower height +174 only. Sources: insertion
`0x00656D43–0x00656DF0`, removal `0x00656CC2–0x00656D35`, linked removal
`0x00656DFD–0x00656E42`, context comparison `0x00656E50–0x00656E79`.

The model-view method at `0x00654140` subtracts **0x24C** to the primary pointer, then calls
main slot 10 with the supplied box pointer and that pointer+12: it delegates to occupant bounds,
without independently transforming a box or ensuring geometry is current. The function at
`0x00654160` instead calls attached +C8's slot 100 (+190) with true if attached, otherwise does
nothing; it is main slot 43 Refresh, despite the export's GetWorldBounds name. Neither function
establishes a missing general GetTransform/SetTransform contract. Complete geometry, Draw and
view-dependent transform/bounds generation remain open under NO40/UG5.

## 4. The power pole *(cSC4PowerPoleOccupant, 0x09C05C6A)*

*Rewritten 2026-10-21 (re-powerpoles), from the instruction bytes. Offsets are object offsets (the earlier layout
table and record list mixed in view offsets); `+0xBC` is the pole's model instance, to which `0x26A68448` is
forwarded (answers NO2); `+0xC0`/`+0xC4` are the view rotation and zoom the model was made for; slots 5, 9, 13, 16
and 17 are named; slot 10's cost is the tuning property `0x0A2ECED8`, read by `StaticInit`. The wire geometry is
[power-line-tool.md](power-line-tool.md) §5.3 and §5.5.*

A power pole is an occupant with a model (the pole) and a list of **connections** to other poles. Each connection
holds the other pole, the power **line** occupants that stand in the cells under the wire, and, on the pole that
owns it, four **wires**: point lists sampled from a cubic Bézier curve that sags between the two poles' attachment
points. Poles can be distressed (they then show a "powerpolezap" effect) and preview.

### 4.1 Identity and layout

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID` `0x0064DD50`) | `0x09C05C6A` |
| `QueryInterface` (`0x0064A030`) | `0xA8C86142` (`cISC4PowerPoleOccupant`), `0x09C05C6A`, `0xA9764C8C` → main; `0x452294AA` → the message target (`+4`); `0x26A68448` (a model instance's mesh container, model-instances.md) → forwarded to the model instance `+0xBC` when there is one; any other → the occupant base, then the model instance base |
| Occupant type (`+0x2C`, the occupant view's `+0x24`) | `0x2890D4DE` |
| Factory | `GZCOM_CreatecSC4PowerPoleOccupant` (`0x0064E340`): a pool block (`0x006499D0`) constructed as most-derived (`0x0064DB30`); destructor `0x0064E380`; object **0x124** bytes |

Sub-objects: `+0` main (`cISC4PowerPoleOccupant`, vtable `0x00AA9EB0`, 19 slots), `+4` message target
(`cIGZMessageTarget2`, `0x00AA9EA0`, 4 slots), `+8` occupant (`0x00AA9E18`, 34 slots; its QI thunk `0x0064DF00`
subtracts 8), `+0xC` serialisation (`0x00AA9DFC`: Write `0x0064B010`, Read `0x0064EA00`), `+0x10` property holder
(`0x00AA9D98`, 25 slots), `+0x24` the exemplar interface (`0x00AA9D80`: slot 3 `SetDefaultExemplar` `0x0064D910`,
slot 4 the exemplar; `GetOwner` `0x00649190`), `+0x3C` the model instance part, and the model instance interface as
a virtual base at `+0x120` (`0x00AA9D30`, 19 slots).

| Offset | Type | Field |
|---|---|---|
| `+0x54`, `+0x55`, `+0x64` | u8, u8, f32 | the model instance's draw attributes: flags (bit 0 highlighted), highlight, opacity |
| `+0x68`, `+0x69` | u8, u8 | transform change counter, mask |
| `+0x6C`, `+0x90`, `+0x9C` | 9 f32, 3 f32, f32 | rotation, translation, scale (identity; defaults from `0x00B46778`) |
| `+0xA0` | 3 × f32 | the pole's **position** (`GetPosition` `0x00649DA0`) |
| `+0xAC`…`+0xB4` | vector | the **connections** (44-byte records, §4.6) |
| `+0xB8` | u32 | the pole kind, `Init`'s second argument (§4.3) |
| `+0xBC` | pointer | the pole's model instance (counted) |
| `+0xC0`, `+0xC4` | 2 × s32 | the view rotation and zoom the model was made for (from the view utilities `0x00B43DD8` `+0x10`, `+0xC`; −1 at construction) |
| `+0xC8`, `+0xCC` | 2 × s32 | the **cell** (slot 8) |
| `+0xD0` | bool | set by every connection change; true at construction *[inferred: the bounds are stale]* |
| `+0xE4` | pointer | the attachment table (§4.6) |
| `+0xE8`, `+0xF4`, `+0x100` | 3 vertex lists | the floor and walls drawn in the underground view *[summary]* (stride 0x18) |
| `+0x10C` | f32 | the floor height: the lowest corner altitude of the cell (§4.7) |
| `+0x110` | bool | saved; meaning not known (NO42) |
| `+0x111` | bool | subscribed to `0x99EF1142` |
| `+0x112` | bool | **distressed** (slots 11, 12) |
| `+0x114` | pointer | the "powerpolezap" effect while distressed |
| `+0x118` | bool | **preview** (slots 13, 14) |

**Constructor defaults** *[compat]* (`0x0064DC43–0x0064DD33`): identity rotation,
zero translation and scale 1; transform counter/mask +68/+69 both zero; position and cell zero;
connections and all three vertex lists empty; kind +B8=0; delegate +BC=null; rotation/zoom +C0/+C4=−1;
attachment table +E4=null; stale +D0=true; floor height +10C=0; +110, subscription +111, distressed +112
and preview +118 all false; effect +114=null. This does not promise zeroed padding or a usable attachment
table before a supported exemplar selects it.

#### 4.1.1 Native pool contract *[compat; NO33]*

The native factory allocates a **0x124-byte block**, not a block sized from a host class layout.
The pool owner is `0x00B0C0C8`, with its current pool pointer at `0x00B0C0CC`.
Each 32-KiB-aligned pool has a 16-byte header and **112 blocks**: offsets
`0x10 + i×0x124`, i=0…111 (last `0x7EAC`). Free blocks link through their first word;
fresh allocation takes them in increasing address order. Header +8 is the free-list head;
+C is the live-allocation count. Allocating pops the head and increments that count
(`0x006497E0–0x0064981B`, `0x006499D0–0x006499FA`). No block clearing precedes construction.

When the current pool is exhausted, allocation obtains another available pool, creating one
if necessary. The exhausted pool moves to the full-pool ring with `0x40000000` added to its
count (`0x0090CF9A–0x0090CFF8`). Deleting destruction first destroys the object, then finds
its pool with `address & 0xFFFF8000`, prepends the block to that pool's free list and decrements
the count (`0x0064E380–0x0064E3F2`). A marked count reaching **`0x40000054`** (84 live,
28 free) moves the pool back to the available ring and removes the marker
(`0x0090CFF9–0x0090D030`). An unmarked count reaching zero removes/frees the pool if it
has another pool in its ring; a sole available pool is retained (`0x0090D031–0x0090D055`).
Destruction without the deleting flag does not return the block. These are allocator and
native-layout facts; a portable value representation need not reproduce native addresses.

### 4.2 Interface slots *[compat]*

`cISC4PowerPoleOccupant` (vtable `0x00AA9EB0`), pop sizes from the ABI table:

| Slot | Address | Behaviour | Pop |
|---|---|---|---|
| 0 | `0x0064A030` | `QueryInterface` (§4.1) | 8 |
| 1/2 | `0x006491C0`/`0x006491D0` | `AddRef`/`Release` (thunks to the shared bodies) | 0 |
| 3 | `0x006C2040` | `AsOccupant`: `this + 8` (null-safe) | 0 |
| 4 | `0x00649D80` | `GetConnectionCount`: records in the vector (`([+0xB0] − [+0xAC]) / 0x2C`) | 0 |
| 5 | `0x0064BE50` | `GetLineCells(index, out)` (§4.6) | 8 |
| 6 | `0x0064DAE0` | `BreakAllConnections(quiet)` (§4.6) | 4 |
| 7 | `0x006499B0` | `GetConnectedPole(i)`: record `i`'s pole, no bounds check, not counted | 4 |
| 8 | `0x00649210` | `GetCell(&x, &z)` = (`+0xC8`, `+0xCC`) | 8 |
| 9 | `0x0064A650` | `InsertLines`: for each record not inserted, each line into the occupant manager (`InsertOccupant(line, 0)`), then inserted | 0 |
| 10 | `0x0064DD60` | `GetCostOfDemolition`: `EDX:EAX` from `0x00B46770`/`0x00B46774`, the tuning `0x0A2ECED8` as an int64 (§4.3) | 0 |
| 11, 12 | `0x00649230`, `0x0064AA40` | `IsDistressed` (`+0x112`) / `SetDistressed(b)` (§4.7) | 0, 4 |
| 13, 14 | `0x00649240`, `0x00649250` | `IsPreview` / `SetPreview(b)` (`+0x118`; power.md §5.1 ignores a preview pole) | 0, 4 |
| 15 | `0x0064CE40` | `Init(exemplar, kind)` (§4.3) | 8 |
| 16 | `0x00649DD0` | `GetLineCellsTo(pole, out)`: slot 5 for the record of `pole`; false when there is none | 8 |
| 17 | `0x00649D00` | `GetModelInstances(…, out[], …, &count)`: translation `:=` the position, mask bit 1, change counter + 1, `out[0] :=` the model instance interface (counted), count 1 | 20 |
| 18 | `0x0064C6C0` | `ChangeZoomLevel(context)` (§4.7) | 4 |

Others: occupant slot 9 `SetPosition` (`0x0064C540`, §4.7); occupant slot 10 `GetBoundingBox` (`0x00649A00`, with the
cell-size floats `0x00B4675C`/`0x00B46760` that the occupant `Init` reads from city slots 91, 92) *[summary]*;
occupant slot 17 `SetHighlight(h)` (`0x006491E0`) writes the draw attributes only (byte 1 `:= h`, flag bit 0 `:= h ≠
0`), so the occupant's own `GetHighlight` (slot 16, the occupant state byte) stays 0 *[quirk]*; model instance slot 17
`GetSortValue` (`0x00649200`) writes `0xAA9DA78E` to its u32 output (the wire texture); slot 11 `Pick`
(`0x0064A870`) uses the model-and-walls box (§4.7.4), updating only the context's distance at `+0x28`.
`GetBounds` `0x00649E50`, `GetWorldBounds` `0x00649E60`, `GetTransform` `0x00649E80`, `SetTransform`
`0x00649F10`. `RemoveConnection` is non-virtual (§4.6; direct-calls.md).

**Output and transform details** *[compat; NO33]*. Slot 16 searches connection records in order
by exact other-pole pointer, including a null pointer if a record contains one. The first match
calls this pole's slot 5 with its index and returns that result. No match returns false and
leaves the output alone (`0x00649DD0–0x00649E43`). Slot 17 ignores the selection/capacity
inputs: it first writes count=0, copies occupant GetPosition to transform translation +90/+94/+98,
ORs mask bit 1 and increments the u8 counter modulo 256, writes **the pole's own** model-instance
interface (most-derived object+120) to out[0], writes count=1 and AddRefs the pole. It does this
even without a loaded +BC delegate (`0x00649D00–0x00649D71`). The delegate is not the returned object.

GetTransform copies the 13 f32 rotation/translation/scale words to output +4…+34, copies mask
+69 to output byte 1, and increments the **destination's** byte 0 modulo 256; it neither copies
the stored counter nor changes the pole. SetTransform copies those words and mask back, and
increments the **pole's** +68 counter, independently of input byte 0. Neither operation changes
position +A0, cell, floor/walls or delegate (`0x00649E80–0x00649F9A`). GetBounds forwards its
one box output to delegate slot 7 if present, otherwise leaves it untouched; GetWorldBounds
passes the output's min and max triplets to occupant GetBoundingBox (§4.7.2).

SetHighlight stores the low byte of its first argument at +55, but tests the **whole 32-bit
argument** to set/clear +54 bit 0, preserving the other bits. The second argument is ignored;
the occupant-view call pops 8 bytes. Thus h=`0x100` gives highlight byte 0 with the draw flag
set (`0x006491E0–0x006491FD`). The saved +110 byte remains separate: constructor false and
record Read/Write storage (§4.4), with no inferred meaning or coupling to preview/distress.

### 4.3 Tuning, `Init` and the static set-up *[compat]*

The tuning is read from the exemplar type `0x6534284A`, group `0xE7E2C2DB`, instance `0xC911E35B` ("Utilities",
`SimCity_1.dat`): the first six by `InitPowerLineControlVariables` (`0x0064A170`, which writes the defaults first),
the last two by `StaticInit`:

| Property | Name | Default | Shipped | Where, use |
|---|---|---|---|---|
| `0xE9A18D3F` | power line control max horizontal distance | 16 | 16 | `0x00B46754`, power-line-tool.md §5.5 |
| `0x49A1E8C4` | power line control horizontal % | 10 | 10 | `0x00B46750` |
| `0x89A18FC4` | power line control max vertical distance | 10 | 10 | `0x00B4674C` |
| `0xE9A1E8D4` | power line control vertical % | 0.15 | 0.15 | `0x00B46748` |
| `0x49A195C0` | power line curve distance between points | 8 | 8 | `0x00B46744` |
| `0x89A1E8FB` | minimum number of curve points (u32) | 5 | 5 | `0x00B46740` |
| `0xE9AC43E7` | power line scaling factor, one per zoom (5 × f32) | 0.3 each | 0, 1, 0.5, 0.35, 0.2 | `0x00B46798`, the wire width by zoom (§4.7) |
| `0x0A2ECED8` | power pole demolition cost | 0 | 5 | `0x00B46770`, slot 10 (stored as an int64) |

**`StaticInit`** (`0x0064CC20`, once; flag `0x00B46764`): hold the resource manager (`0x00B46768`); register the
static shutdown (`0x0064BD80`, which releases the texture cache) with the application (slot 69); hold the texture
manager (service `0x1AC0E11A`, interface `0xFAC0E219`, `0x00B46758`); read the tuning; put the textures `0xAA9DA78E`,
`0xC9AF0FCD`, `0x0912220E` and `0x08080004` in the texture cache (`0x00B46784`, empty bindings); pack the render
state `0x00B4677C` = (1, 1, 1, 1, 0, 1, 0, 3, 4, 5, 4, 0.1, 1, −1) (draw-context.md §5.1).

- **Main slot 15 `Init(exemplar, kind)`** (`0x0064CE40`, pop 8): the static set-up; unless initialised: the occupant
  `Init` (below); rotation and zoom from the view utilities; `kind` stored (`+0xB8`); the exemplar set
  (`SetDefaultExemplar`); the model loaded (`LoadModel`, §4.7) and the attachment table chosen (§4.6); the three
  vertex lists emptied; draw attributes cleared and opacity 1; when distressed, `SetDistressed(true)`. True.
- **Occupant slot 3 `Init()`** (`0x00649FA0`): the occupant base `Init`; the city's cell width and depth (city
  slots 91, 92) into +`0x00B46760` (x width), +`0x00B4675C` (z depth); unless already subscribed, the message target subscribes to `0x99EF1142` (message
  server slot 5); the model instance is released.
- **The message target** (`+4` slot 3, `0x0064A140`): on `0x99EF1142` whose occupant is this pole, it unsubscribes
  and inserts its connections' lines (slot 9). So the lines join the occupant manager when their pole does.
- **`SetDefaultExemplar`** (`+0x24` slot 3, `0x0064D910`): when the exemplar is not the current one, hold it, reload
  the model and the attachment table, and rebuild the wires of every connection on both poles (`0x0064D510`)
  *[summary]*.
- **`Shutdown`** (occupant slot 4, `0x0064BC80`): `BreakAllConnections(true)`; the stale flag cleared; the model
  instance released; the vertex lists emptied; the subscription removed; the effect stopped and released; the
  occupant base `Shutdown`.


**Demolition quote is independent of those six controls.** The signed 64-bit quote at
`0x00B46770/74` starts at zero. In the same one-time `StaticInit`, a second lookup of the
same control exemplar reads property **`0x0A2ECED8`** through the signed-int64 property
helper (`0x0064CDBA–0x0064CE29`, call at `0x0064CE1B` to `0x005FD4B0`). The preceding
property `0xE9AC43E7` updates the five-entry wire-width vector, not the quote.
The quote is the property's signed-int64 value, without scaling, multiplication by a
pole count, or conversion from `0xE9A18D3F`. Missing exemplar/property leaves the quote
unchanged; failed variant conversion likewise leaves its destination unchanged
(`0x005FD4B0`, `cRZVariant::GetValSint64(out)` `0x009177E4`). Normal SINT64 values copy
both words exactly. Other variant types use the shared variant conversion contract.
The getter loads the low word into EAX and high word into EDX without alteration
(`0x0064DD60–0x0064DD6B`). Repeating `Init` does not re-read the property once
`StaticInit` has run; `InitPowerLineControlVariables` alone only updates the six controls
above. Thus changing `0xE9A18D3F` alone never changes this demolition quote.

`GetBoundingBox` (`0x00649A00`) uses the cell-size floats `0x00B4675C`/`0x00B46760` (`Init` reads city slots
0x16C/0x170 into them).

### 4.4 Save record *[compat]*

`Write` `0x0064B010` writes u16 version **5**. `Read` `0x0064EA00` first ensures
StaticInit, then rejects versions below 4; there is no upper-version rejection. The
record body runs only after successful base occupant serialization. All offsets below
are **primary-relative**: these methods receive the serializable view at primary+0xC,
so add 0xC to their instruction displacements. This resolves the earlier contradictions
between this section and §4.1. In particular, record +B0 is live **+BC (delegate)**,
not the connection vector's live +B0 end pointer. Primary GetCell really reads +C8/+CC
(`0x00649210`); the dirty byte is +D0 (`0x0064DCD1`).

| Order | Field | Encoding |
|---|---|---|
| 1 | base occupant | base record |
| 2 | transform +68 | current transform codec on Write and v5+ Read; legacy on v4 Read |
| 3 | position +A0/+A4/+A8 | three f32, x/y/z order |
| 4 | connection vector +AC | u32 connection count, then the connection bodies below in vector order |
| 5 | exemplar index +B8 | u32 |
| 6 | model IDs +C0/+C4, cell x/z +C8/+CC | four s32 words in that order |
| 7 | four vectors at the **pointer stored in +E4** | f32 xyz at pointer+0, +0C, +18, +24, in that order (48 bytes) |
| 8 | vertex vectors +E8, +F4, +100 | three independent wall-helper lists, described below |
| 9 | model Y +10C | f32 |
| 10 | bool +110 | u8, nonzero normalized on Read |
| 11 | bytes +54/+55 | two raw u8 |
| 12 | field +64 | f32 |
| 13 | distressed +112 | raw u8 |

The four vectors in order 7 are **not inline object fields**. On Read,
`InitConnectionPoints(primary)` runs after order 6, then the four xyz values are read
through the selected +E4 pointer (`0x0064EF06–0x0064EF28`; Write
`0x0064B4B0–0x0064B4C8`). Thus for the supported subtypes they overwrite the first four
entries of the selected shared connection-point table. They do not constitute four
new private vectors or overlap the wall-list headers. Invalid-subtype pointer behavior
is not guarded by this record reader; callers must provide the valid table invariant.

**Connection body.** Each live record has stride 0x2C. Its stream is:

1. Destination pole reference (live record+0). Write passes its serializable view
   at pole+0xC; Read requests **IID `0x09C05C6A`**, exactly that class-ID value.
2. u32 number of power-line references in the linked list at record+4; then that many
   references in list traversal order. Write passes each line's serializable view at
   line+8; Read requests **IID `0xC9C05C5D`** for every list item. These are different
   reference kinds, not alternative IIDs for the same pole reference.
3. Two bool bytes, record+8 then +9 (Read normalizes nonzero to 1).
4. Two s32 words, record+0C then +10 (both preserved, not reset to zero).
5. The nested xyz-vector collection at record+14; then the collection at record+20.
   **Each** is u32 outer count; for each member, u32 inner point count followed by
   that many xyz f32 triples in point order. Empty collections and empty inner lists
   retain their explicit zero counts. Each live outer/inner vector header is 12 bytes;
   header/capacity pointers never go into the stream.

Evidence: `0x0064B0BE–0x0064B139` (reference views and list count),
`0x0064EAEE` / `0x0064EB33` (IIDs), `0x0064EB48–0x0064EB8D` (bools and signed words),
and the nested-list writes between `0x0064B170` and `0x0064B43E`.

**Wall-helper list.** Each of the three headers uses the same helper as §3.4:
u32 vertex count, then per vertex f32 x/y/z, f32 u/v, u8 R/G/B/A. The live vertex is
24 bytes: xyz at +0/+4/+8, BGRA bytes at +0C/+0D/+0E/+0F, uv at +10/+14. The stream
is therefore 24 bytes per vertex **after** a four-byte count; it is not a memory dump
(colour precedes UV in memory but follows it in the stream). Writer `0x0065B070`,
reader `0x006574A0`; pole call sites `0x0064B4CA–0x0064B4E6` and
`0x0064EF2A–0x0064EF46`.

**Failure and cleanup.** There is no rollback. After the version/base gates, Read
resizes the connection/list/vector containers and attempts all body fields without
branching on individual reference/helper returns. `SC4ReadGZSerializable` clears the
destination before lookup (`0x005D9742`); a failed lookup is not preservation of an
old reference. At the end of the reached body, the old delegate live +BC is cleared
and released if non-null, even when the stream has already failed
(`0x0064EF98–0x0064EFB0`). The base-failure path skips all body work and delegate
release. Both Write and Read then return `GetError()==0`; a failed reference lookup
alone is not separately promoted to a false result by the owner. Unsupported version
returns false directly. Allocation failure and malformed excessive counts have no
additional owner validation contract.

### 4.5 Creation and consumption

Poles are created by `cSC4PowerLineTool::CreatePowerPole` `0x00650140` (`FindPoleOccupant` `0x0064FFE0` reuses an
existing pole at the cell; `operator new` pool `0x006499D0`; `SetDefaultExemplar`; `+0xB8` = the exemplar index;
power-line-tool.md). The consumer is the power simulator (power.md §5.1/§5.4: dispatch on type `0x2890D4DE`, QI
`0xA8C86142`, wire bits from `GetCell`, the reference list, `costBase` from the int64 property `0x09132DE8`).

### 4.6 Connections and wires *[compat]*

**A connection** (`tConnection`, 44 bytes): `+0` the other pole (counted); `+4` a list of the power line occupants
under the wire (counted); `+8` bool: the lines have been inserted into the occupant manager; `+9` bool: **this pole
owns the wire** (it alone keeps the wire points and draws them; the one with the smaller `(z, x)` cell); `+0xC`,
`+0x10` the other pole's cell; `+0x14` the **wires**: a vector of point lists (each a vector of 3 × f32); `+0x20` the
coarse curves, made only over water. `AddConnection` (`0x0064E3E0`), the direction helper, the sag control points
and the Bézier sampling are [power-line-tool.md](power-line-tool.md) §5.2, §5.3 and §5.5.

**Attachment points** (`0x00649BA0`). `InitConnectionPoints` (`0x00649B30`) reads the exemplar's resource key: with
`k := (instance >> 8) & 0xF`, `k` 1 or 2 selects table **A** (`0x00B0C0D8`, the straight pole), 3 or 4 table **B**
(`0x00B0C198`, the diagonal pole); another value keeps the current table. Each table has four orientations of four
points (x, y, z):

| `r` | Table A | Table B |
|---|---|---|
| 0 | (0, 31.7, ±4.8), (0, 38.7, ±4.8) | (∓3.3, 31.7, ±3.3), (∓3.3, 38.7, ±3.3) |
| 1 | (0, 31.7, ±6.5), (0, 38.7, ±6.5) | (∓4.5, 31.7, ±4.5), (∓4.5, 38.7, ±4.5) |
| 2 | (∓4.8, 31.7, 0), (∓4.8, 38.7, 0) | (∓4.5, 31.7, ∓4.5), (∓4.5, 38.7, ∓4.5) |
| 3 | (∓6.5, 31.7, 0), (∓6.5, 38.7, 0) | (∓3.3, 31.7, ∓3.3), (∓3.3, 38.7, ∓3.3) |

(each entry is two points, the upper sign first: for example table A `r = 0` is (0, 31.7, 4.8), (0, 31.7, −4.8),
(0, 38.7, 4.8), (0, 38.7, −4.8)). The attachment points of a pole for orientation `r` are the four table points of
`r mod 4` plus the pole's position.

The displayed decimal coordinates are **binary32 constants**. Positive bit patterns are
31.7=`0x41FD999A`, 38.7=`0x421ACCCD`, 4.8=`0x4099999A`, 6.5=`0x40D00000`,
3.3=`0x40533333`, 4.5=`0x40900000`; negatives change the sign bit. The data at the two cited table
addresses confirms the signs/order in the table. Subtype selection uses an unsigned shift/mask;
the jump table at `0x00649B90` maps 1/2 to A and 3/4 to B. Any other subtype **retains** +E4:
on a fresh object it remains null, after selecting A it stays A, and after selecting B it stays B.
No default table is supplied (`0x00649B59–0x00649B89`). The record reader's shared-table overwrite
in §4.4 therefore affects later users of that same table.

**Removing.**
- `RemoveConnection(record, quiet)` (`0x0064A560`): release the record's pole (left null); remove each of its lines
  from the occupant manager (`RemoveOccupant(line, true, 0)`) and release it; empty the list; not inserted; when this
  pole owns the wire and not `quiet`, `PostOccupantMessage(0x99EF1145, 0)`; the stale flag set. The wires stay.
- `RemoveConnection(pole, quiet)` (`0x0064DA00`, non-virtual; power.md §11.2): the record whose pole is `pole` is
  removed as above and erased from the vector.
- `BreakAllConnections(quiet)` (main slot 6, `0x0064DAE0`): for every record, the other pole's
  `RemoveConnection(this, false)`, then this pole's `RemoveConnection(record, quiet)`. The records stay in the vector
  with no pole *[quirk]*, so `GetConnectionCount` does not drop.

**Cells under a wire** (main slot 5, `0x0064BE50`). `GetLineCells(index, out)`: false for an index out of range.
When this pole does not own the wire and the other pole exists, the answer is the other pole's slot 16 for this pole.
Otherwise the output is cleared and rebuilt by the fractional-cell walk below; true.

#### 4.6.1 Cell-list lookup and walk *[compat; NO33/NO43]*

An index is invalid if negative or at least the record count: return false without touching out.
A non-owning record with a non-null destination delegates to that pole's slot 16 with this pole
and out; return its result without clearing out locally. There is no cycle guard. An owning
record, or any record with a null destination, follows the local path (`0x0064BE50–0x0064BEED`).

The local path **replaces**, rather than appends to, out. Its start is
`(f32(position.x / X), f32(position.z / Z))`, with X=`0x00B46760`, Z=`0x00B4675C`.
The end is the record's +C/+10 cell pair converted **unsigned u32 to f32**. It does not use
the stored starting cell +C8/+CC, nor the destination's current position
(`0x0064BEED–0x0064BF3B`). Each endpoint difference is stored f32.

For finite differences dx,dz, let a=abs(dx), b=abs(dz), N=`trunc(max(a,b))+1`.
If a>b, the step is `(1, f32(b/a))`; otherwise it is `(b=0 ? 0 : f32(a/b), 1)`.
Negate an axis's step only when that axis's difference is negative. Start at the f32 start
and emit **N** pairs, each by truncating both current coordinates toward zero to s32.
After each emission, including the last, each accumulator becomes f32(current+step).
There is no floor, deduplication, end-cell correction or terrain test
(`0x00649820–0x006498BA`, `0x0064A4D0–0x0064A53B`, `0x0064BF40–0x0064BFB0`).
For coincident endpoints N=1, so the internal unused z step of 1 does not add a cell.
The count/conversions have no overflow or non-finite recovery branch.

#### 4.6.2 Record ownership and refresh *[compat; NO33]*

Connection relocation/copy makes an independent line-list container and deep copies both
vectors of point lists, but copies pole and line pointer **values without AddRef**. Destruction
of such a container frees its nodes/point buffers without releasing those pointers
(`0x0064CFA0–0x0064D01A`, `0x0064D2D0–0x0064D3CF`, `0x0064D450–0x0064D50A`).
The logical connection owns the refs established at creation/attachment; vector reallocation
does not change their counts. AttachLines (`0x0064B6E0`) appends the line pointer from +8 of
each 16-byte input item, AddRefing each, including duplicates. Null/empty input does nothing.

RemoveConnection(record,quiet) releases the other pole then clears that pointer; for **every**
line, regardless of the inserted flag, calls manager RemoveOccupant(line's occupant view,1,0)
before releasing the line. It empties/frees the list nodes and clears inserted, preserves
owning, destination cell and both sampled-point vectors, and sets +D0=true. It posts
`0x99EF1145` with data 0 only for an owning record and quiet=false (`0x0064A560–0x0064A64A`).
The pointer overload is inert on null/no match; otherwise it removes the first matching record
and erases it. BreakAllConnectionsAtDestination(quiet) visits the original records: for a
non-null other pole, call that pole's pointer overload with this pole and **quiet=false**,
then remove this record with the caller's quiet. Its records remain, so its count does not
fall; the destination's matching record is erased (`0x0064DA00–0x0064DB29`). InsertLines marks
each uninserted record inserted even for an empty list or failed InsertOccupant result;
it does not AddRef, and skips already-inserted records (`0x0064A650–0x0064A6B3`).

RebuildWires(other), `0x0064D510–0x0064D90B`, is inert for null, no matching **owning** record,
or equal starting/destination cells. It obtains the destination's current GetCell, and derives
the start cell by truncating position.x/X and position.z/Z in x87. Otherwise it clears only
the regular sampled-wire vector, obtains the direction and four attachment pairs using the
**same direction index** on both poles, and regenerates four regular curves in attachment
order by the control-point and Bézier rules of [power-line-tool.md](power-line-tool.md) §5.5.
Each regular curve is allocated at the helper's count, begins with the exact start point,
and fills the remainder by EvaluateBezier with count−1. If terrain slot 34 reports water at
either pole's cell, append a three-sample coarse curve for each pair. **The coarse vector is
not cleared here**, so repeated refreshes over water append four more curves; a dry refresh
preserves existing coarse curves. Finally post `0x99EF1145`/0 and set +D0. Line occupants,
inserted/owning bits, destination pointer/cell and connection count are unchanged.
SetDefaultExemplar with an unchanged pointer is inert; a changed exemplar is held before
releasing the old, then LoadModel and InitConnectionPoints run, followed by RebuildWires on
this and the destination for each record (`0x0064D910–0x0064D98C`).

### 4.7 Position, model, effect and drawing *[compat]*

**Position** (occupant slot 9 `SetPosition(p)`, `0x0064C540`): the position becomes `p`; the model is moved
(`SetModelPosition` `0x0064AC70`); the cell becomes `(trunc(p.x × (1/w)), trunc(p.z × (1/w)))` with `w` the terrain's
cell width (terrain slot 6); the floor height becomes the lowest of the cell's four corner altitudes (terrain slot
17), the first corner kept on ties and NaN never taken; the floor and walls are rebuilt (`CreateFloor` `0x0064BFD0`,
`AddWallQuad` `0x0064C2C0`, `CreateWalls` `0x0064C470`); the effect, when present, is moved to `p`. For a cell off
the map the corner read leaves its output untouched, so the floor height comes from uninitialised values *[quirk]*;
FreeCity4 uses the position's y then *[choice]*. Occupant slot 8 `GetPosition` returns the position.
The reciprocal `1/w` and each multiplication remain in x87 until the truncating conversion;
there is no intervening f32 reciprocal/product store (`0x0064C578–0x0064C59D`). SetPosition
does **not** dirty +D0, rebuild connections, or synchronize the pole's own transform translation.
When an effect exists, its new transform has identity rotation, scale 1, translation p,
counter=1 and mask=1 (`0x0064C61D–0x0064C6AA`).

**Model and zoom.** `LoadModel` (`0x0064AD90`) builds the model's resource key from the exemplar's property
`0x27812821`, with the instance from `cSC4ModelMakerUtility::GetModelInstanceID(occupant, zoom, rotation, kind is 1
or 8, 0, 0, 0)`, fetches the model (interface `0x9AD0E7E3`) and makes the pole's model instance, placed by
`SetModelPosition` (the position plus the exemplar's offset `0x4835C6A4`) and turned with the view.
`ChangeZoomLevel` (slot 18): when the context's zoom differs from the cached one (`0x00B0C0A8`), every cached texture
is reloaded; then the rotation and zoom become the context's (`+0x18`, `+0x14`) and the model, floor and walls are
rebuilt.

**Distress.** `SetDistressed(b)` stores the flag. Distressed with no effect: the effects manager (`0x00B43D1C`, slot
7) makes `powerpolezap`, which is placed at the position (identity rotation, scale 1) and started (its slots 8 and 3).
Repaired: a running effect is stopped (slots 5, 4), released and forgotten. The pole decay of power.md §7.6 calls it.

**Drawing.** `Draw` (`0x0064C800`) rebuilds the model, floor and walls when needed, draws the floor and walls
with the textures `0x0912220E` and `0x08080004` and the tint from the lighting, and
`DrawPowerlines` (`0x0064B750`) draws each owned wire with texture `0xAA9DA78E` (and `0xC9AF0FCD` for the coarse
curves), its width scaled by the zoom's scaling factor (§4.3).

---

#### 4.7.1 Floor and wall inputs *[compat; NO33/NO43]*

CreateFloor empties the +E8 list and emits four 24-byte vertices. Without the diagonal-model rotation,
at pole position `(x,y,z)` their positions are `(x−5,y,z−5)`, `(x−5,y,z+5)`, `(x+5,y,z+5)`,
`(x+5,y,z−5)`, with UVs `(0,0),(0,1),(1,1),(1,0)` and white RGBA `(255,255,255,255)`.
This is a 10×10 square at the pole's **position y**, not at +10C. The coordinate inputs are the signed
unit-square corners multiplied by f32 10 and then f32 0.5 in x87. Transformed x is rounded to f32
before adding position.x; transformed z retains x87 precision through addition of position.z.
Both final world coordinates are stored f32. Sources: `0x0064BFDF–0x0064C02C`,
`0x0064C162–0x0064C2A0`; UV arrays `0x00AA9B18/0x00AA9B28`.

There is a separate diagonal-model branch: when exemplar model property `0x27812821` succeeds and the
instance's high word is `0x0503` or `0x0504`, it constructs a Y rotation from the double at `0x00AA9D28`
before transforming the square (`0x0064C086–0x0064C162`). This double is exactly
**0.78539001941680908203125**, not π/4. x87 cosine is stored f32, giving
c=`0x3F350554` (0.7071125507354736328125); the sine terms round to
s=`0x3F350493` (0.707101047039031982421875). The relevant matrix coefficients are
`m00=c, m02=−s, m20=s, m22=c`, with zero y terms. Each coefficient is stored f32.
For a corner's scaled coordinates qx,qz (±5), form x87 sums in the order
`m02×qz + m00×qx + zeroY` and `m22×qz + m20×qx + zeroY`.
World x is `f32(f32(xSum)+position.x)`; world z is `f32(zSum+position.z)`.
The x-only intermediate store is `0x0064C217`; the final stores are
`0x0064C23E`/`0x0064C248`. There is no fused multiply-add or post-rotation reordering.

AddWallQuad clears its destination list and uses floor vertices A/B as the top endpoints. It emits four
vertices in order **top A, bottom A, bottom B, top B**, where each bottom preserves its top's x/z but
has y=+10C. UVs are `(0,0)`, `(0,f32((topA.y−bottomY)×0.0625))`,
`(1,f32((topB.y−bottomY)×0.0625))`, `(1,0)`. The subtraction and multiplication feed the f32 store
without an intervening f32 temporary. RGB becomes white on each vertex; its source endpoint's alpha
is retained. The two endpoints can have different heights/alphas. Negative V is not clamped
(`0x0064C2C0–0x0064C46E`, factor `0x00A8E174` is f32 0.0625).

CreateWalls clears both wall lists and selects these **ordered floor-index pairs** for +F4 then +100:

| Cached view rotation +C0 | First wall | Second wall |
|---|---|---|
| 0 | `(2,3)` | `(1,2)` |
| 1 | `(1,2)` | `(0,1)` |
| 2 | `(0,1)` | `(3,0)` |
| 3 | `(3,0)` | `(2,3)` |

Each list is rebuilt with AddWallQuad, not appended to old geometry (`0x0064C470–0x0064C53B`). The
table is indexed by the cached rotation without a range check; these rules require rotation 0…3.
The cell walk is §4.6.1, occupant bounds are §4.7.2, and model/Pick/Draw contracts follow below.

#### 4.7.2 Occupant bounding rectangle *[compat; NO33/NO43 partial]*

GetBoundingBox (`0x00649A00–0x00649B27`) receives the **occupant view at primary+8**. Its +C8 dirty
test is therefore the primary +D0 flag; its apparent position +98/+9C/+A0 is primary +A0/+A4/+A8.
When dirty, obtain the starting x/z cells by truncating position.x / cell-width and position.z /
cell-depth toward zero, retaining their low 32 bits. Initialize both minimum and maximum cell
coordinates to this cell. Consider only connection records whose sampled-wire vertex list is
nonempty (record +14 differs from +18). Their destination cell x/z at record +0C/+10 expand the
rectangle using **unsigned** comparisons. A smaller coordinate replaces the minimum; otherwise
a larger one replaces the maximum. Empty-wire records do not contribute even when they retain
destination coordinates. Store the four cached extrema at primary +D4/+D8/+DC/+E0 and clear +D0.
No coordinate scan occurs on a clean call (`0x00649A11–0x00649AB1`).

Return min `(signed(minX)×cellWidth, floorHeight, signed(minZ)×cellDepth)` and max
`(signed(maxX+1)×cellWidth, position.y+60, signed(maxZ+1)×cellDepth)`, with 32-bit wrapping
increments and f32 output stores. The extrema are compared as unsigned but converted to floating
point as **signed** integers (`0x00649A6B–0x00649AA4`, `0x00649AB8–0x00649B22`). Each multiplication
is done after signed conversion; the y addition is one x87 addition followed by an f32 store.
This is a cell-covering rectangle, independent of the actual rendered wire extrema and model box.
GetWorldBounds uses this occupant box. GetBounds instead forwards to the held model's +1C method
when a model exists and otherwise leaves the output untouched (`0x00649E50–0x00649E6C`);
the separate Pick path uses a model-derived box.
The floor and wall lists do not expand this occupant rectangle.

#### 4.7.3 Model replacement, zoom and lifetime *[compat; NO33]*

LoadModel first obtains the exemplar; null returns false without changing the delegate.
Its default resource key is type **`0x5AD0E817`**, group **`0xBADB57F1`**, instance 0.
Property `0x27812821` may replace that key, but the instance is then **always replaced** by
GetModelInstanceID(occupant view, cached zoom, cached rotation, kind∈{1,8},0,0,0).
The property helper's success flag is ignored. With no resource manager, or a failed
GetResource(key,`0x9AD0E7E3`,…,0,0), preserve the old delegate. Only after successful resource
acquisition is the old delegate cleared/released; resource slot 5 creates an instance with
argument 0 directly into +BC. A null result leaves +BC null. For a non-null instance,
SetModelPosition runs, then delegate slot 4 receives the occupant view, then its transform
rotation becomes a Y rotation by **minus view-utility +1C** while preserving the just-set
translation/scale. Sine/cosine are evaluated in x87 and matrix entries stored f32; mask bit 4
is ORed and the delegate transform's counter incremented. The temporary model resource is
released on either creation result. **LoadModel returns false even after successful creation**;
callers use its side effects (`0x0064AD90–0x0064B004`, especially `0x0064AFEF`).

SetModelPosition does nothing without a delegate. Otherwise fetch its current transform and
the occupant position; read the float-array property `0x4835C6A4`, then set each translation
component to f32(position+offset), OR mask bit 1, increment the fetched transform counter and
pass it to delegate SetTransform (`0x0064AC70–0x0064AD83`). Rotation and scale are preserved.
The temporary property array is freed. **Three readable offsets are required**: the original
does not check property success or array length before indexing; missing/short data has no
defined fallback offset (`0x0064AD07–0x0064AD36`).

ChangeZoomLevel always copies context +18/+14 to cached rotation/zoom and calls LoadModel,
CreateFloor, CreateWalls, even when both values already match. Separately, when the shared
cached zoom `0x00B0C0A8` differs, update it and visit every static texture-cache entry:
release/clear the old binding, request type `0x7AB50E44`, group `0x1ABE787D`, instance=entry
texture ID. A failed request is retried with the **same key**; no alternate group or explicit
zoom offset is used (`0x0064C6C0–0x0064C7F0`). The texture service supplies zoom-dependent data.

SetDistressed stores its input byte even when resource creation fails. A nonzero request with
an existing effect does not restart/reposition it. With no effect, successful creation of
`powerpolezap` gives it an identity/scale-1 transform at position and starts it with argument 0;
failure leaves the distress flag set and permits a later retry. A zero request tests the
effect's slot 5 (running), calls slot 4 only if running, then clears/releases it
(`0x0064AA40–0x0064AB6F`). Shutdown instead passes **1** to that stop call, removes the message
subscription when its server exists, clears/releases the delegate and clears all three vertex
lists without freeing their capacity. It calls BreakAllConnections(true), clears +D0, invokes
base occupant Shutdown and returns true (`0x0064BC80–0x0064BD72`). Remaining record/point-buffer
and vertex capacities are freed by destruction. Destruction alone releases delegate/effect
and frees containers, but does not perform Shutdown's reciprocal removal, unsubscribe or
effect-stop operations (`0x0064DFC0–0x0064E0DB`).

#### 4.7.4 Pick *[compat; NO33/NO43]*

No delegate means false and no pick-context change. Otherwise obtain its local box, initialized
before the call to min=(1,1,1), max=(0,0,0), then add **pole position +A0** to both endpoints,
rounding each addition f32. The exemplar model-position offset and the pole's own transform
are not applied here. Expand that single box componentwise with **all vertices of +F4 and
+100**, in that order; strict comparisons keep existing endpoints on ties/unordered values.
The floor list and connection wire samples are not scanned (`0x0064A870–0x0064A9DA`).

Call RayIntersectsBoundingBox(context+0, context+C, min, max, hit). On intersection, compute
LengthOfLine(context+0,hit) in x87; accept only an ordered distance **strictly less** than the
f32 context +28. On acceptance store that distance f32 at +28 and return true, otherwise false.
Nothing writes context +2C (`0x0064A9E0–0x0064AA36`). Box intersection and length use
[geometry-utilities.md](geometry-utilities.md) contracts; this is one enclosing-box test,
not separate wire or triangle picks.

#### 4.7.5 Draw and wire submissions *[compat; NO33/NO43]*

Draw returns true. If +BC is null at entry, first try LoadModel and rebuild floor/walls.
If either shared texture zoom or this pole's cached zoom/rotation differs from context,
call ChangeZoomLevel (so a null model and a mismatch can cause two load/build attempts).
For a non-null delegate, pass draw attributes +54 to its slot 16, then Draw(context,false).
Select tint=(1,1,1) from `0x00B0C0B0`; when Draw's highlight argument is nonzero, instead use
the first three f32 values of context +54's **16-byte record indexed by byte +55**.
Set render state `0x00B4677C` (§4.3) and the context's default model transform
(`0x0064C800–0x0064C8D1`). There is **no underground-view, preview, distress or visibility
test inside this Draw body**, and a failed delegate load does not suppress its floor/wall path.

Submit floor then first wall then second wall. For each list, obtain the unnormalized normal
from its first three vertex positions using GetNormalToTriangle (`0x005D6EA0`), normalize with
x87 reciprocal length `1/sqrt((nz²+ny²)+nx²)`, and store each normal component f32. No zero-length
guard is applied. Obtain lighting from view-manager slot 9 then lighting slot 22, with output
RGB, the normalized normal and **the list's first vertex position**. Multiply each returned
RGB component by the selected tint and store f32 for SetShade. This is a fresh lighting query
for each face; do not use the floor's result on the walls (`0x0064C934–0x0064C9CF`,
`0x0064CB34–0x0064CBC2`).

| Submission | Texture | Filtering (min,mag,stage) | Wrap (u,v,stage) | DrawPrims (primitive,format,count) |
|---|---|---|---|---|
| Floor +E8 | `0x0912220E` | (1,1,0) | (2,2,0) | (6,10, list size) |
| Walls +F4, +100 | `0x08080004` | (1,1,0), set once | (3,3,0), set once | (6,10, each list size) |

Bind each texture at stage 0. ResetShade after the floor, then after both walls; the second
wall replaces the first wall's shade. Next call DrawPowerlines, inheriting this render state,
transform, filtering and wrap modes (`0x0064C9D4–0x0064CC1F`). The expected floor and walls
have four vertices each; this body dereferences their initial vertices without an empty-list
guard and requires successfully built geometry and texture bindings.

DrawPowerlines indexes the width vector `0x00B46798` by cached zoom, without a range check.
It returns immediately for width≤0; an unordered width is **not** rejected. In connection
order, only owning records draw. Bind texture `0xAA9DA78E` and, in regular-curve order, emit
one quad per adjacent pair A,B; point lists of size≤1 emit none. For the x/z difference
`dx=Bx−Ax`, `dz=Bz−Az`, compute x87 L=sqrt(dx²+dz²), then store
nx=f32(dx/L), nz=f32(dz/L). There is no zero-length guard. Put P=(nz,−nx) in x/z.
Quad order is **A−width×P, A+width×P, B+width×P, B−width×P**, keeping each endpoint's y.
Each product-plus-coordinate stays x87 through its final f32 vertex store. UVs are
`(0,0),(1,0),(1,V),(0,V)`, where V=f32(sqrt(f32(dx)²+f32(dz)²)×0.125), recomputing
the UV length from the separately stored differences. The normalizing differences themselves
retain x87 precision; their f32 stores do not truncate the live calculation
(`0x0064B824–0x0064B9A7`). Each quad is DrawPrims(6,2,4): xyz+uv, **no colour bytes**.

If the coarse vector is nonempty, bind `0xC9AF0FCD`. For each coarse curve of N points,
emit **N−1 axis-aligned squares**, centered on points 0…N−2, not strips between endpoints.
Half-size is width×4. Vertex x/z signs in order are (−,−),(−,+),(+,+),(+,−), all at the
center's y, with UVs `(0,0),(1,0),(1,1),(0,1)` and DrawPrims(6,2,4)
(`0x0064B9DB–0x0064BB95`). Neither regular nor coarse drawing restores the texture binding.

## 5. The power line *(cSC4PowerLineOccupant, 0xC9C05C5D)*

*Rewritten 2026-10-21 (re-powerpoles): object offsets, and the record read in the bytes (answers NO13).* A power line
occupant is a light occupant at one cell that names its two poles; the power simulator wires its cell.

### 5.1 Layout and slots

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID` `0x00649260`) | `0xC9C05C5D` |
| `QueryInterface` (`0x00649350`) | `0xC98F3B24` (`cISC4PowerLineOccupant`), `0xC9C05C5D`, `0xA9764C8C` → main (counted); others → the occupant base |
| Occupant type (`+0x28`) | `0x0990B2EA` |
| Factory | `GZCOM_CreatecSC4PowerLineOccupant` (`0x0064AB70`), a pool block and the constructor `0x0064A270` |
| Sub-objects | `+0` main (`cISC4PowerLineOccupant`, vtable `0x00AA9C80`, **8 slots**), `+4` occupant (`0x00AA9BF8`, 34), `+8` serialisation (`0x00AA9BDC`: Write `0x0064A310`, Read `0x0064A3A0`), `+0xC` property holder (`0x00AA9B78`), `+0x20` a small interface (`0x00AA9B60`) |

State: position `+0x38` (3 × f32), cell `+0x44`, `+0x48` (2 × s32), the two poles `+0x4C`, `+0x50` (counted), a
bool `+0x54` given with the poles, preview `+0x55`.

**Shutdown forwarding and live offsets** *[compat; NO13 supplement]*. Occupant slot 4 runs
with this=object+4. It calls **main slot 4 virtually** on object with `(null,null,false)`,
then tail-forwards to `cSC4Occupant::Shutdown` (`0x005E8980`) with the original occupant-view
pointer, not object or object+8 (`0x00649270–0x00649289`). The setter processes pole +4C
first, then +50: AddRef the replacement if non-null, Release the old if non-null, store the
replacement; after the second store write +54. Equal old/new pointers still get AddRef then
Release (`0x006492D0–0x0064932A`). Both references may name the same pole and each owns a ref.
On ordinary Shutdown each old pole is therefore released once; +54 becomes false. Position
+38/+3C/+40, cell +44/+48 and preview +55 are preserved. Base Shutdown tests occupant
IsInitialized; when true it clears bit 0 at **object+32** and posts `0x99EF1141`/0 through
occupant slot 15; it always returns true (`0x005E8980–0x005E89A6`). The tail forwarding
preserves that result. The constructor zeros position, cell, both pole pointers and +54/+55
(`0x0064A270–0x0064A304`); no third pole exists in this layout.

| Slot | Address | Behaviour | Pop |
|---|---|---|---|
| main 3 | `0x00649290` | `GetConnectedPoles(&p1, &p2, &b)`: both poles (each counted for the caller), `b :=` `+0x54`. The SDK declares the first out-argument a line; it is a pole | 12 |
| main 4 | `0x006492D0` | `SetConnectedPoles(p1, p2, b)`: holds the new poles, releases the old, stores `b` | 12 |
| main 5 | `0x00649330` | `GetCell(&x, &z)` = (`+0x44`, `+0x48`) | 8 |
| main 6, 7 | `0x00649390`, `0x006493A0` | `IsPreview` / `SetPreview(b)` (`+0x55`) | 0, 4 |
| occupant 3 | `0x005DEA50` | the occupant base `Init` | |
| occupant 4 | `0x00649270` | `SetConnectedPoles(null, null, false)`, then the occupant base `Shutdown` | |
| occupant 8 | `0x005B2B00` | `GetPosition` | |
| occupant 9 | `0x0064A460` | `SetPosition(p)`: the position, and the cell `(trunc(p.x × 0.0625), trunc(p.z × 0.0625))` (`0x00A8E174`, through the float→int helper `0x009EEF04`) | |
| occupant 10 | `0x0064A4A0` | `GetBoundingBox(min, max)`: both the position (a degenerate box) | |

### 5.2 Record and creation *[compat]*

`Write` `0x0064A310` / `Read` `0x0064A3A0`: u16 **2**; the occupant base record; the position (3 × f32, stream slot
`+0x30`); the cell (2 × s32, slot `+0x20`); `bool` `+0x54` (`operator<<(u8)` `0x005D9FD0`); `ref` pole 1; `ref` pole 2
(`SC4WriteGZSerializable` `0x005D99A0`; read as `0xA8C86142`). `Read` accepts only version 2.

Creation: `cSC4PowerLineTool::PlacePoles` `0x00651560` creates lines directly (ctor `0x0064A270` at `0x00651CE4`),
position from `ComputePolePosition` `0x0064EFE0`; x,z use `(signed cell + 0.5) × terrain cell width`.
**NO24:** start height at -FLT_MAX, replace with each strictly greater ordered corner in returned
order, and store y=height+0.5 as float32. All-sentinel corners return y=-FLT_MAX; no failure or skipped
pole. NaN corners are ignored. The special terrain branch samples at stored x,z and adds 0.5.
Evidence: `0x0064F00B`, `0x0064F061–0x0064F0BB`; [full contract](power-line-tool.md) §4.2.
Consumers: power.md §5.4 and §11.

---

## 6. The pylon *(cSC4PylonOccupant, 0x49B6D69B)*

### 6.1 Layout

Constructor `0x005F1730`. Sub-objects: `+0` primary `cISC4PylonOccupant` `0x00AA5150` (21 slots), `+4`
`cIGZMessageTarget2` `0x00AA5140`, `+8` occupant view `0x00AA50B8`, `+0xC` serializable `0x00AA509C` (Write
`0x005F0A10` / Read `0x005F0AF0` / `GetGZCLSID` `0x005F1A10`), `+0x10` property holder `0x00AA5038` (25),
`+0x24` `0x00AA5020`, `+0x3C` model instance, `[+0x40]`-relative `0x00AA501C` (19-slot 3D view). The ctor writes
the occupant type **`0xA9B6D7DE`** at `+0x2C` (`0x005F19F9`) — not the bare type (correcting the earlier
[inferred] guess).

| Offset | Meaning |
|---|---|
| `+0x68` | texture/model dirty bool; constructor sets 1 |
| `+0x6C/+0x70/+0x74` | UI state / dispatch state / dispatch type (32-bit live enums; signed bytes on disk) |
| `+0x78` | source occupant ref (`SetSourceOccupant` `0x005F0D30`; getter folded `0x00407AB0`) |
| `+0x7C` | the single transform; flags at +7C/+7D, identity 3×3 at +80…+A0, zero translation +A4/+A8/+AC, scalar +B0=1 |
| `+0xB4…+0xC8` | cached local box, min initially (1,1,1), max initially (0,0,0) |
| `+0xCC…+0xE0` | cached world box, min initially (1,1,1), max initially (0,0,0) |
| `+0x134/+0x13C` | packed render states |
| `+0x144/+0x148/+0x150` | main texture binding / main texture id / call-number texture id; all initially zero |
| `+0x14C` | u16 call number; constructor 0; setter marks +68 dirty |
| `+0x1A4/+0x1A5` | blink bool / drag byte; constructor zero |
| `+0x1A8` | signed 32-bit bitmap/state field; constructor zero |
| `+0x1AC` | embedded GZ string; constructor empty (not an int64) |
| `+0x1C0` | owner reference; constructor null |

**Initial enum values** *[compat; answers NO44]*: UI state +6C is **1** and dispatch state +70 is
**0**. Dispatch type +74 is **not initialized** by construction: it retains the incoming allocation's
four bytes until a setter or record read supplies it. It has no promised numeric default. The zero in
dispatch state is not a blanket zero-initialization of all three enums. The instruction stores are
`0x005F17C0` (+70=0) and `0x005F17D1` (+6C=1); the entire constructor
`0x005F1730–0x005F1A0E` has no +74 store. Its basic-model base constructor
`0x00801CF0–0x00801D67`, called at primary+3C with virtual-base construction disabled, initializes
only through primary+64 and does not supply that missing value. A deterministic fresh-object dispatch
type is an implementation *[choice]*, not a recovered default; do not use an uninitialized native type
to index texture tables before assigning a supported type.

All offsets in this layout are from the primary object. **Record methods receive the
serializable view at primary+0xC**, so their instruction displacements must have 0xC
added to obtain these live offsets. The serializable QI thunk subtracts 0xC
(`0x005F1A90`); the constructor installs its table at +0xC (`0x005F1789`). In particular,
record displacement +5C is live +68, not another transform, and record +6C is the same
source reference that Shutdown releases at live +78. Call number and drag state are
live +14C and +1A5, not +74 and +7C. Constructor evidence: `0x005F17C0–0x005F19F2`.

### 6.2 Slots and lifecycle *[compat]*

`cISC4PylonOccupant` (vtable `0x00AA5150`; dispatch.md §4.1 cites these):

| Slot | Address | Behaviour |
|---|---|---|
| 0 | `0x005F0820` | `QueryInterface` (§1.2) |
| 3 | `0x006C2040` | `AsOccupant`-folded |
| 4 | `0x005F0930` | `Init` |
| 5 | `0x005F11A0` | `Shutdown` |
| 6 | `0x005F0CA0` | `SetUIState` |
| 8 | `0x005F0900` | `SetBitmapInfo` |
| 9 | `0x005F0D00` | `SetDispatchType` |
| 10 | `0x00407AC0` | `GetDispatchType` (folded) |
| 11 | `0x005F0CD0` | `SetDispatchState` |
| 13/14 | `0x005F1A20`/`0x005F1A40` | `SetCallNumber` / `GetCallNumber` |
| 15/16 | `0x005F1A50`/`0x005F1A60` | `SetDragState` / `GetDragState` |
| 17/18 | `0x005F0D30`/`0x00407AB0` | `SetSourceOccupant` / `GetSourceOccupant` |
| 19 | `0x005F1160` | `SetPylonOwner` |
| others | | `SetBitmapInfo` `0x005F0900`, `GetBounds` `0x005F0D60`, `GetWorldBounds` `0x005F0DA0`, `GetTransform` `0x005F0DE0`, `SetTransform` `0x005F0E70`, `GetPosition` `0x005F0F00`, `SetPosition` `0x005F2670`, `Pick` `0x005F0F40`, `DoAnimationTick` `0x005F0C50`, `SetVisibility` `0x005F12A0`, `DoMessage` `0x005F26F0`, `Draw` `0x005F2830`, `GetSortValue` `0x005F08B0` |

Textures: `BuildMySimTexture` `0x005F12D0`, `RebuildCurrentTexture` `0x005F1610`, `BuildCallNumberTexture`
`0x005F1D00`, `RebuildModel` `0x005F20A0`.

`Init` `0x005F0930` (the `+8` occupant vtable's slot 4): `IsInitialized` via the `+8` vtable +0x14;
`cSC4Occupant::Init`; `0x00B23B78 = 1`; subscribes the three messages `0xA6B79602`, `0x06CDB65B`, `0x26D31EC1`
via `+4`; `cRZSysServPtr<cISC4AnimationTickManager>(0x86F9FB4D/0x86F9FB4C)` slot 0x14 subscribes the tick
`0x80`; if the renderer `0x00B43DD0` exists: its slot 0x28 + `PackRenderState` into `+0x134` and `+0x13C`.

`Shutdown` `0x005F11A0`: `cSC4Occupant::Shutdown`; if byte `+0x37 & 1` (inserted): `PlaceInOccupantManager(0)`
`0x005F1010`; unsubscribe the three messages; release `+0x78` (the source).
An already initialized `Init` returns true immediately, without subscriptions or render-state work
(`0x005F093C–0x005F0941`, `0x005F0A03–0x005F0A07`). The renderer slot +0x28 result is
discarded; it does not decide success or the packed states. With a renderer, the two
14-argument `PackRenderState` tuples, in parameter order, are
`(1,1,1,1,1,1,1,3,4,5,4,0,1,−1)` at +134 and
`(1,0,0,1,1,1,1,−1,4,5,4,0,1,−1)` at +13C
(`0x005F09B2–0x005F09FE`).

#### Placement state *[compat]*

Byte +37 bit 0 means inserted in the occupant manager; bit 1 means inserted in the
renderer. The renderer receives the adjusted 3D-instance view
`primary + 0x40 + signed32([primary+0x40]+4)`, not the primary or occupant view.
Both placement methods first call city slot +88 (even for no-op requests); the first
uses the result as its occupant-manager reference. For Boolean requests:

| Operation | Required effect and result |
|---|---|
| Requested state already matches its bit | true; no insert/remove call |
| Manager insert, bit 0 clear | Null manager: false. Otherwise manager +50(occupant-view +8, 0); only success sets bit 0 and returns true |
| Manager remove, bit 0 set | Null manager: false, preserving bits. If bit 1 set, renderer +74(instance) when renderer exists, then clear bit 1 even if renderer absent. Manager +54(occupant-view +8, 0, 0), clear bit 0, true |
| Renderer insert, bit 1 clear | Null renderer: false. Ensure manager insertion; require resulting bit 0, then occupant slot +80(4), then renderer +70(instance). Only success at both calls sets bit 1. Failure returns false without undoing successful manager insertion |
| Renderer remove, bit 1 set | Null renderer: false, preserving bit 1. Otherwise renderer +74(instance), clear bit 1, true; bit 0 stays set |

Other bits survive every transition. Sources: `0x005F1010–0x005F10A2`,
`0x005F10B0–0x005F1155`. The old description's renderer +78 insertion slot was incorrect.

Shutdown returns true and does nothing when uninitialized. Otherwise, after base
Shutdown, placement removal and the three message unsubscriptions, it unsubscribes
animation ticks (`+4` target, category 0x80); releases MySim texture +150; releases
binding +144 if present, otherwise releases texture +148 if nonzero and the texture
manager exists; clears/releases source +78 and owner +1C0. These are distinct refs;
the source is not a second ref at +6C (`0x005F11A0–0x005F129A`).

#### 6.2.1 State setters, animation and position *[compat; NO36]*

SetUIState, SetDispatchState and SetDispatchType compare the full 32-bit value against +6C, +70 or +74.
A changed value marks +68 dirty and clears blink +1A4; an equal value preserves both bytes, including a
currently active blink. No range check or byte truncation occurs here (only the record uses signed bytes).
SetBitmapInfo always stores its 32-bit first argument at +1A8, assigns its string argument to +1AC and
marks +68 dirty, even if both values match. Sources: `0x005F0900–0x005F0928`, `0x005F0CA0–0x005F0D29`.

SetSourceOccupant and SetPylonOwner use the same replacement policy for +78 and +1C0: an equal pointer
does nothing; otherwise AddRef a nonnull new pointer, store it, then Release a nonnull old pointer.
Neither dirties the model/texture nor changes blink (`0x005F0D30–0x005F0D5E`, `0x005F1160–0x005F1194`).
This order differs from the connection's linked-reference setter (§3.6.1).

DoAnimationTick advances the **shared u32 accumulator** at `0x00B23B80` only when this pylon's dispatch
state is 1 or 4. Add the supplied u32 interval modulo 2³²; compare the unsigned result with 500. Below
500 it only retains the accumulator. At or above 500 reset the accumulator to zero (discard all excess)
and set this pylon's blink to 1 if its old byte was zero, otherwise 0, marking +68 dirty when that byte
changes. Other pylons' blink bytes do not change, although their eligible ticks share the accumulator.
Other dispatch states leave both the accumulator and this pylon unchanged. It is neither a per-object
timer nor a catch-up loop (`0x005F0C50–0x005F0C9B`, unsigned comparison at `0x005F0C73`).

The position is the transform translation at primary +A4/+A8/+AC. GetPosition accepts a null output as
a no-op. SetPosition accepts a null input as a no-op; otherwise its order is removal from the occupant
manager, removal from the renderer, translation assignment, increment the transform change byte +7C
modulo 256, OR bit 0 into transform mask +7D, RebuildModel, insertion into the occupant manager and
insertion into the renderer. It does not restore the old placement state or stop on a placement failure;
even a previously unplaced object is offered for insertion. These accessors receive the occupant view
at primary+8 (`0x005F0F00–0x005F0F3E`, `0x005F2670–0x005F26E9`).

SetVisibility first forwards both arguments to the occupant base. If the resulting live +37 bit 0 says
it is manager-inserted, call PlaceInRenderer with the requested visibility; otherwise no renderer
placement call follows (`0x005F12A0–0x005F12C6`, also occupant-view-relative).

These state operations combine with §§6.2.2–6.2.4 to answer NO36. The setters themselves do not
perform texture composition or update bounds unless a rebuild is explicitly listed.

#### 6.2.2 Bounds, transforms, picking and texture selection *[compat; NO36]*

GetBounds copies local min/max at primary +B4/+C0; GetWorldBounds copies world min/max at +CC/+D8.
The occupant-view GetBoundingBox copies that same world pair to separate outputs. These getters do
not rebuild, clear dirty or transform corners (`0x005F0D60`, `0x005F0DA0`, `0x005F08C0`).
GetSortValue returns the current **texture ID +148**, including zero, rather than the class ID
(`0x005F08B0`). RebuildModel owns updates to the cached bounds; stale getters remain stale.

GetTransform increments the **output** change byte modulo 256, copies the matrix, translation and
scale from +80…+B0, and copies the internal mask +7D. It does not copy the internal counter +7C.
SetTransform increments the internal counter, copies those same 13 floats and the input mask, and
does not copy the input counter. Neither getter nor setter rebuilds the model, updates bounds,
places the occupant or dirties +68 (`0x005F0DE0–0x005F0EFA`).

Pick returns false immediately for UI state zero. Otherwise it starts with the cached world box,
replaces its minimum y with `f32(maxY − ViewportUnitsToWorldUnits(50)/f32(cos(viewPitch)))`,
and ray-tests this box. The conversion/division stays extended until the final min-y store; cosine
is explicitly rounded to f32 first. A hit succeeds only when its ray-origin distance is strictly
less than the pick context's existing distance; only then is the distance replaced. A tie is false
and preserves the old distance. This path neither rebuilds a dirty model nor tests the rendered
triangles (`0x005F0F40–0x005F1006`, cosine store `0x005F0F9F`, distance gate `0x005F0FE3–0x005F0FF0`).

RebuildCurrentTexture first releases/nulls a binding if present; otherwise releases a nonzero raw
texture ID when the texture manager exists. It does **not** zero +148 at that point. For types 0/6
it selects instances from table A, types 1/5 table B; other non-4 types use instance zero. Resource
type/group are `0x7AB50E44/0x1ABE787D`. A successful binding replaces +148 with that binding's
texture ID; a failed/null binding leaves the old +148 unchanged. The return is whether +148 is
nonzero, so it can be true after a failed replacement of a previously nonzero ID. Type 4 calls
BuildMySimTexture instead; §6.2.4 specifies its pixel composition.

| Dispatch state | Table A instance | Table B instance |
|---|---|---|
| 0 | `0xEA3EE100` | `0x14315E00` |
| 1 | `0xEA3EE110` | `0x14315E10` |
| 2 | `0xEA3EE120` | `0x14315E20` |
| 3 | `0xEA3EE130` | `0x14315E30` |
| 4 | `0xEA3EE140` | `0x14315E40` |
| 5 | `0xEA3EE150` | `0x14315E50` |
| 6 | `0xEA3EE120` | `0x14315E20` |

These arrays are indexed without a range check; values outside 0…6 have no supported resource
contract. Sources: `0x005F1610–0x005F1705`, tables `0x00AA4F20/0x00AA4F3C`.
Billboard construction and label/MySim composition are in §6.2.4; Draw and messages are in §6.2.3.

#### 6.2.3 Drawing and message decisions *[compat; NO36]*

Draw (`0x005F2830–0x005F29A4`) returns 1 without submitting anything when its self-lit argument is
true or the byte at `context[+38][+0C]+39C` is nonzero. Otherwise pass zero alone consumes a dirty
model by calling RebuildModel. Both supported passes install the cached +7C transform and clear
texture transform zero. Pass zero sets transparency to f32 bits `0x3ECCCCCD`, uses state +13C,
binds the raw texture ID +148 and submits the four main vertices at +E4 with primitive/format
`(6,2)`. It resets transparency and returns 2. Pass one resets transparency, uses state +134 and
draws the same main quad. It also draws the four label vertices +154 with texture +150 only when
the **dispatch state getter** (primary slot 12) is neither 1 nor 4, call number +14C is nonzero,
and dispatch type +74 is not 4. Pass one does not consume dirty state; its return is 1.
Other pass numbers submit nothing and return 1. There is no texture-ID-zero rejection in these
submissions. The label eligibility is evaluated before a pass-zero rebuild, although pass zero
does not draw the label (`0x005F2864–0x005F2898`, `0x005F28A1–0x005F28AD`,
`0x005F291E–0x005F299A`).

DoMessage (`0x005F26F0–0x005F282C`, receiving primary+4) always returns true, including unrecognized
messages. `0x86AD10EE` obtains the message's +38 u32, divides by **unsigned 1000**, and passes the
quotient to DoAnimationTick. `0x26D31EC1` immediately rebuilds the model. `0xA6B79602` only marks
+68 dirty. For `0x06CDB65B`, the filter obtained from message +3C tests this occupant (+8): rejection
removes visibility flag 2 and calls renderer +74 only if that flag was set; acceptance sets flag 2
and calls renderer +70 only if it was clear. Already-matching visibility causes no renderer call.
The renderer receives the adjusted 3D-model view, not the occupant interface.

#### 6.2.4 Billboard and bitmap construction *[compat; completes NO36]*

Construction initializes both cached boxes to min `(1,1,1)`, max `(0,0,0)`, **not** useful
geometry; the two packed-state pairs are all zero. The main and label vertex arrays have no
constructor initialization contract. RebuildModel supplies them before drawing. The constructor's
model category at primary +50 is 3 (`0x005F196C–0x005F19A9`, `0x005F1A00`). The defaults in §6.1
and the lifecycle/state rules in §§6.2–6.2.3 complete the other requested constructor/owner behavior.

**MySim composition** (`0x005F12D0–0x005F160B`). Load a bitmap with type/group
`0x856DDBAC/0xAB7E5421`, format argument 9 and immediate-load argument 1. Its instance is
`0x2BB06F3F` for dispatch state 1, `0x2BB075B4` for states 0,2,3,4,5,6 (table `0x00AA4F58`);
there is no state-index range check. Missing background returns false without changing the
texture ID. Hold the background for the duration of the operation. A nonzero bitmap selector
+1A8 loads the portrait from type/group `0x856DDBAC/0x46A006B0`, instance equal to that selector,
format 9, explicit Load. A zero selector instead obtains class/IID `0xA2FFB5D3/0x02FFBB60` and
requests a portrait with string +1AC and argument 8 through slot +10. Failure to obtain that
service returns false; a successfully obtained service returning no portrait still permits
uploading the background. Release the portrait/service and temporary resource holders on exit.

The loaded background itself is the destination; this routine makes no private bitmap clone.
For a portrait of width `w`, let `d = (u32(52-w) >> 1)`. Starting from its reported rectangle,
the destination x is `rect.left+d+n` and y is `rect.top+trunc(d/2)`, for `n=52,104,156` in that
order. The source starts at its bitmap data origin, not at that rectangle's top-left offset.
For each placement lock both bitmaps with `0x8040`, visit the full source height/width using
their byte row strides, and unlock both with that same flag. Copy a complete 32-bit source pixel
only if the destination pixel's `0xC0000000` bits are zero and the source RGB is not `0xFF00FF`.
There is no blending, clipping, resizing or width-underflow repair in this routine. In particular,
pixels copied by an earlier placement can protect themselves against a later placement. With
no portrait the pixel walk is skipped. Finally call SetupTexture(background, texture manager,
address of **main** ID +148, 1,0,0,0) and return its boolean; +150 is the call-number ID, not the
MySim ID. Sources: placement `0x005F1436–0x005F1475`, pixel predicate/strides
`0x005F14DA–0x005F15B7`, upload `0x005F15C1–0x005F15E6`.

**Main billboard.** Let `V(p)` be ViewportUnitsToWorldUnits for viewport distance p and let F
denote an explicit float32 store. RebuildModel computes `W=F(V(44))`, half-width
`a=F(W*0.5)`, and height `h=F(V(150)/cos(pitch))`. The height division uses the extended x87
cosine without an intermediate cosine float32 store. It calls RebuildCurrentTexture even if
the current texture appears valid, and continues geometry rebuilding regardless of that result.
The ordered vertices at +E4 are:

| Vertex | Position | UV |
|---|---|---|
| 0 | `(-a,h,0)` | `(k*s,0)` |
| 1 | `(-a,0,0)` | `(k*s,b)` |
| 2 | `(a,0,0)` | `((k+1)*s,b)` |
| 3 | `(a,h,0)` | `((k+1)*s,0)` |

For dispatch type 4, `s=0.203125` and b has float bits `0x3F3145D1`; otherwise `s=0.171875`,
`b=0.5859375`. k is 3 when blink +1A4 is true, otherwise the UI state except that zero becomes
1. k is converted as a signed integer; the products and sum are stored as float32. No UI-state
range clamp is added (`0x005F20AF–0x005F21E9`, constants `0x00AA51AC/0x00AA51B0`).

The rebuild preserves translation. If scale's bits are not exactly `0x3F800000`, set it to 1,
clear mask bit 2, and increment the transform counter. If mask bit 4 is set, first replace the
matrix with identity, clear that bit and increment again. Then turn the horizontal matrix pairs
to face the view: angle is `atan2(-view[+54],-view[+5C])`, c is its cosine rounded to float32,
and t is its sine retained in x87 precision. For each old pair `(x,z)` from offsets
`(+80,+88)`, `(+8C,+94)`, `(+98,+A0)`, replace it with `(x*c-z*t, x*t+z*c)`; the middle
column stays as it was. Rounding is observable: the first pair's new x rounds only the final
difference; the other two round each product before subtracting. The first two new z values
round both products before adding; the last rounds only `z*c` before adding extended `x*t`.
Every final element is float32. Set mask bit 4 and increment the counter once more, modulo 256.
Consequently a second rebuild resets a previously billboard-rotated matrix before applying the
new facing, while an input matrix with bit 4 clear participates in the first rotation
(`0x005F21EF–0x005F2359`).

Local bounds become `(-a,0,0)` and `(a,h,0)`. World bounds are the componentwise extrema of
**only these two transformed endpoints**, not all four corners. For output component i, the stored
matrix triple beginning at +80, +8C or +98 is `(Mi0,Mi1,Mi2)`, respectively. Transform that
component as `(x*Mi0+y*Mi1+z*Mi2)*scale+translation[i]`. Keep the native x87 arithmetic order: for the
first endpoint, x sums `-a*M00+(M02+M01)*0`, y sums `-a*M10+(M12+M11)*0`, and z sums
`-a*M20+(M22+M21)*0`; y/z sums are F-stored before scale, and z is also F-stored after scale.
For the second endpoint, x sums `(h*M01+a*M00)+M02*0`, y sums
`a*M10+(M12*0+h*M11)`, and z sums `a*M20+(M22*0+h*M21)`; its x is F-stored after scale
and its z sum is F-stored before scale. Other intermediate operations stay extended. First x/y
and all second-endpoint coordinates are F-stored after translation. First z stays extended
through the comparisons; final bounds are float32. No extra corners, label bounds or padding
are included (`0x005F2363–0x005F25B1`).

If call number is nonzero, rebuild its texture/vertices as below, with arguments W and h; zero
skips that work and leaves any previous label resources/vertices untouched. Then refresh spatial
registration: with renderer flag 2 set, renderer calls +78 then +7C(model,3); if manager flag 1
is set, notify manager +60(occupant,0). Without renderer flag 2, notify the manager if already
inserted, otherwise attempt PlaceInOccupantManager(true), then attempt PlaceInRenderer(true) if
manager flag 1 is now set. These are refresh/placement operations, not a renderer +28 test.
Finally clear dirty +68 even when texture replacement or placement failed
(`0x005F25B7–0x005F2667`).

**Call-number texture** (`0x005F1D00–0x005F2050`). Obtain font resource `0x68963C4C` from WCM,
hold it, save its slot +3C setting and set that setting to 3 through +40. Format the unsigned
16-bit call number with `%d`, without leading zeros. Let A and B be the signed integer results
of font measurement slots +C0 and +94 respectively, each given the string and its length. The
square bitmap side S is 32 if both measurements are at most 32; otherwise it is the smallest
power of two at least `max(A,B)` (`0x006046B0`, despite the helper's name "nearest"). Release
any old +150 texture and zero that ID before trying to obtain/create the bitmap. Obtain a bitmap
through GS +0C; create S by S, descriptor `(9,32)`. On a successful `0x8001` lock clear via
bitmap +48(0,0), then unlock. Draw the text even if that lock failed: font +CC gets
`(bitmap,0,0,string,length,0xFF000000,0x08000000,0,0)`. If the texture manager exists upload
with SetupTexture(bitmap,manager,&labelID,1,0,0,0), then release the bitmap. Bitmap/service/create
failure skips the corresponding pixel/upload work but still updates label geometry and restores
the font setting/releases it. There is no fallback to the previous label ID.

For label geometry let `r=F(F(V(14))/cos(pitch))`, `j=F(V(9))`,
`q=V(F(A))*0.5`, `y=h-r-j/cos(pitch)`, and `u=F(1/S)`. Neither cosine is F-stored here.
The second argument h is the only geometry input used; the first argument W is unused.
The four `(x,y,z,u,v)` tuples at +154, in order, are
`(-q,y,0.01,0,0)`, `(-q,y-r,0.01,0,B*u)`,
`(q,y-r,0.01,A*u,B*u)`, `(q,y,0.01,A*u,0)`.
Keep q and y extended until each vertex store; A becomes float32 before V and the UV multiplication,
whereas B is converted directly from its signed integer to x87 for its UV product. The reciprocal
u is float32. Zero UVs are produced by multiplying u by zero. Position z uses
bits `0x3C23D70A`. Geometry is rebuilt even when no texture was uploaded; main bounds are not
expanded to include it (`0x005F1EEC–0x005F201A`).

### 6.3 Record *[compat]*

`Write` `0x005F0A10` emits u16 version 5 and the base occupant record, then the following
fields in order. Offsets below are **live primary-relative**, not serializable-relative.

| Field | Stream encoding |
|---|---|
| dirty +68 | bool as one u8 |
| UI +6C, dispatch state +70, dispatch type +74 | three signed bytes, taking each enum's low byte |
| blink +1A4 | bool as one u8 |
| call number +14C | u16 |
| source +78 | SC4 serializable reference; on Read request IID `0x26BB767E` (occupant) |
| transform +7C | current transform codec on Write and v4+ Read; legacy codec only on v3 Read |
| drag +1A5 | bool as one u8 (Read normalizes any nonzero byte to 1) |
| bitmap/state +1A8 | s32; v5+ only |
| string +1AC | stream GZ-string codec, not a numeric scalar; v5+ only |
| owner +1C0 | SC4 serializable reference; on Read request IID 1 (unknown), v5+ only |

This is **one** transform, not two. Bool reader `0x005D9FA0` normalizes its byte;
enum reads sign-extend each signed byte to its 32-bit live field
(`0x005F0B43–0x005F0B78`). The call number uses stream +1C (u16), not u32;
the string uses stream +3C (GZStr), not int64. See writer
`0x005F0A31–0x005F0AD4`, reader `0x005F0B29–0x005F0C30`, and stream table
`0x00AA2280` slots 7, 8, 15. The base and transform/string codecs retain their usual
variable lengths; do not treat the tail as a fixed sequence of machine words.

`Read` rejects versions below 3 before reading the base. It has no upper-version
rejection. A successful base Read enables the body; old source +78 is cleared and
released immediately before its replacement read, not before the version/base gates.
For v5+, old owner +1C0 is similarly cleared/released immediately before its replacement.
For v3/v4, the v5 tail is **untouched**, including any old owner; in a freshly constructed
object that means bitmap/state 0, string empty and owner null. Failed reads are not
transactions: reached fields/releases remain changed, later operations still run under
the stream's error handling, and the final return is `GetError()==0`. The base-failure
path skips the body and also returns that error test. There is no forced restoration or
post-read Init/rebuild (`0x005F0AF0–0x005F0C49`).

---

## 7. What each class places on the grid, and who owns it

| Class | Placed by | What it covers | Owner/consumer |
|---|---|---|---|
| `cSC4NetworkOccupant` + prebuilt/bridge/tunnel | the network tools (`CreateNetworkOccupant` slot 54, network-geometry.md §10.1) | the solved piece's cell(s); the network lot manager keys lots by these cells (network-manager.md §8.5) | network manager `managed` list (only prebuilt/bridge/tunnel, §2.7); traffic simulator (traffic-network.md); demolition (demolition.md §4.2/§4.3, via type `0xC772BF98` and the cost slot 63) |
| `cSC4UndergroundOccupant` (+ pipe/subway) | the pipe/subway tools | the trench walls/floor drawn in the underground view; the pipe's cell feeds the water grid (plumbing.md §6.4) | plumbing.md (distressed/burst model choice §3.2 slot 104), power.md §5.2 (subway wire) |
| `cSC4UndergroundConnectionOccupant` (+ pipe/subway connection) | the pipe/subway tools at connection cells | the connection piece's walls; located by `FindUndergroundConnectionOccupant` | `GetZipperPieceType` (§3.2) and the tools' neighbour loops |
| `cSC4PowerPoleOccupant` | `cSC4PowerLineTool::CreatePowerPole` (§4.5) | its own cell (`GetCell`); the wire overlay is drawn *from* it | power.md §5.1 (the pole list, wire bits, `costBase`) |
| `cSC4PowerLineOccupant` | `cSC4PowerLineTool::PlacePoles` (§5.2) | its cell; up to three connected poles | power.md §5.4/§11 |
| `cSC4PylonOccupant` | the dispatch system | nothing persistent: it is a transient MySim/dispatch marker (dispatch.md §4.1) | dispatch manager (tick `0x80`), the animation tick manager |

### 7.1 The occupant ↔ network-lot relationship *[important]*

A **network lot** (`cSC4NetworkLot`, network-manager.md §2.3) holds a reference to the network occupant of its
cell. `CreateLotObjects` (network-manager.md §8.6) creates **props and flora** for a network lot — never
occupants. The network lot's cells are covered by the network-occupant pieces the tools place; the lot manager
finds the lot at a cell through the pieces' network flags (`GetZoneTypeForNetwork`, network-manager.md §8.2), not
through an occupant back-pointer. There is **no `cSC4NetworkLotOccupant` class** in the binary — the only
`cSC4NetworkLot` objects are the 24-byte lot records of network-manager.md §2.3. A replacement must therefore:

1. place one piece occupant per solved network cell (with the flags, §2.2 `+0x13C`);
2. let the network lot manager key its lots by cell and hold references to the piece occupants;
3. never create a separate "lot occupant" object.

### 7.2 References the records hold *[compat]*

| From class | Field | To | Read IID |
|---|---|---|---|
| network occupant (base) | `+0x138` | a path-info-like object | write via `0x6999A8FB`, read `0xA9667793` |
| tunnel | `+0x1D0` | the paired tunnel portal occupant | `0xE4FDA3D4` on write, `0x8A4BD52B` on read |
| pipe/subway connection | live `+0xC8/+0xCC` | attached underground occupant / linked occupant (null allowed) | both `0x26BB767E`; attached result adjusted −4 (§3.6) |
| power pole | per connection | the pole ref (+ wire entries) | `0x09C05C6A`, `0xC9C05C5D` |
| power line | `+0x44/+0x48/+0x4C` | up to three poles | `0xA8C86142` |
| pylon | `+0x6C`, `+0x1B4` | the source occupant, another ref | as written |

The occupant manager's own record (occupant-manager.md §13) references every one of these objects with IID
`0x26BB767E` (`cISC4Occupant`), once per covered cell.

---

## 8. `cSC4NetworkTypeInfo` — the per-network-type tables

### 8.1 The 13 records

Thirteen static records of stride `0x114` at `0x00B452C8`, constructed in a loop by the static initialiser
`0x00A73D10` (`MOV ESI,0xb452c8; MOV EDI,0xd;` ctor `0x00641F70` per record; the static destructor `___tcf_1`
`0x00A7C140` runs 13 reverse iterations). The layout is network-geometry.md §2.3 (widths, evaluator vector,
bridge types, supports map, placement parameters at `+0x50`, the four derived slope numbers at `+0x100…+0x110`) —
**cite, do not repeat**. What this spec adds is *how the records are filled*.

`ReloadNetworkTypeInfo` (`0x00623310`, called by `cSC4NetworkTool::ReloadRules` `0x0062541B` per tool) uses the
loader family below. This is a label/property inventory, not execution order. The authoritative owned
constructor/default, Clear/destruction, reload order and failure-publication contract is now
[network-type-info.md](network-type-info.md). In particular Clear preserves the loaded byte and placement
state, and failed whole-exemplar acquisition does not prevent derivation or LoadComplete.

| Loader | Address | What it fills | Detail |
|---|---|---|---|
| `LoadPlacementParams` | `0x0063CBE0` | `+0x50` (the placement parameters) | one exemplar per type except power line (type `0x6534284A`, group `0x084344E0`, the instances of network-geometry.md §2.4); the property list is: `0x879D12E7` → p+0x00, `0x279D1707` → p+0x04, `0x07B36C71` → p+0x08, `0xC7B36CA4` → p+0x0C, `0xC7B36CA5` → p+0x10, `0xC804DD86` → p+0x14, `0xC804DD87` → p+0x18, `0xC804DD88` → p+0x1C, `0xC804DD89` → p+0x20 (stored as read, not converted), `0xC804DD90` → p+0x24, `0xC804DD9B` → p+0x50 (default 1000), `0xC804DD92…99` → p+0x28…+0x4C, `0xE974EAA4` → p+0x54 (a vector), `0xE974EAA5` → p+0x60 (fallback: one zero word per p+0x54 entry), `0xE974EAA6` → p+0x6C (fallback: one binary32 10.0 per p+0x54 entry), `0xE974EAA7` → p+0xA0 (tail vector), `0x4A270DB0` (default 1.5), `0x4A270E94` (default 32.0), `0x4A271248` (integer default 0), `0x4A27124F` (integer default 0), `0x4A275748` → p+0x88 (integer default 1), `0x4A2D49B9` → p+0x8C (default 1), `0x8A2D49EA` (default 10), `0x8A2D49F0` (default 20), `0x6A2D49F4` (default 0.1), `0xEA86E3B5` → p+0x9C (failure writes p+0x8C=1). Angles use binary32 constant `0x3C8EFA35` at `0x00AA8C60`; see network-type-info.md §5 for admission, absent-property preservation and repeated conversion |
| `LoadEvaluator` | `0x0063D360` | `+0x24` (the evaluator vector) | factory `0x00622A90` → `cSC4TileEvaluator`, `AddRulesFromResource`, appended to `+0x28` (network-geometry.md §8 step 3) |
| `LoadTileConflictRules` | `0x0063D3E0` | the store `0x00B466D0` | callback `0x0063D1E0` |
| `LoadTilePatternVerifiers` | `0x0063FAF0` | the store `0x00B466DC` | `LoadVerifiersCallback` |
| `LoadSupports` | `0x0063F780` | `+0x3C` (the supports map) | `ReadSupportStructureInfos` `0x0063F700` (group `0x8974F80F`; the properties are network-geometry.md §10.2) |
| `LoadBridgeTypes` | `0x006437A0` | `+0x30` (the bridge-type vector) | `ReadBridgeTypeInfo` `0x00642220` + `ReadBridgeTypeSpecializations` `0x006436D0` (parser `0x00643580`, hash `0x00640E10`; network-bridges.md §2) |
| `LoadComplete` | `0x0063BF90` | `+4 = 1` | the loaded latch; `Clear` `0x006421D0`; dtor `0x00643510` |

The class-object store never constructs `cSC4NetworkTypeInfo` dynamically: the 13 records are process globals.
Internal replacement storage is a choice; native interoperability uses the established stride. The lifecycle
spec qualifies what is cleared versus preserved at `ReloadRules` time. `LoadTileConflictRules` is not called
inside `ReloadNetworkTypeInfo`; the table above does not imply that it is.

---

## 9. `cSC4NetworkRoutines`

One body: `ComputeDemolitionRegionForPlacementCells` `0x0061F7C0` (1 004 bytes). Callers:
`cIntRule::Test` `0x00609DDE`, `UpdateOccupantHighlights` `0x00629C53`, `InsertIsolatedHighwayIntersection`
`0x0062D189` — the caller contract is network-rules.md §10 (cite, do not repeat).

The canonical typed contract, instruction citations, lifetime table and proposed vectors are in
[network-routines.md](network-routines.md). Corrections to the earlier summary:

1. The five-argument helper reads contiguous 32-bit x/z pairs using signed coordinate comparisons. City slot 33
   supplies the lot manager; its slot 6 gets each input's lot, whose slot 14 supplies an inclusive rect. Bounds
   include inputs and whole lots, with an extra high border; the bitmap starts entirely clear.
2. It scans only collected lot rects (duplicates retained), x outer/z inner. For each nonzero zone, occupant
   manager `0x00B43D0C` slot 32, **not a demolition utility**, looks for an occupant accepted by the caller's
   placement filter. A **miss** sets the cell bit and zone-type bit. Set bits select cells for demolition.
3. After all rects, `typeMask & 0xFC00` rejects before cost initialization; a high zone alone does not reject.
4. Otherwise the temporary filter rejects occupants answering QI `0xA9764C8C`; it has no caller-filter state.
   `DemolishRegion(false, region, 1, 0x2001, false, temporaryFilter, cost, demolished, null, 0, 0)` is a preview.
   Its AL is returned; the output region remains caller-owned on either constructed-region exit.

---

## 10. The model side (cSC4NetworkOccupantModelInstance)

`cSC4NetworkOccupantModelInstance` is a `cS3DModelInstance` subclass (ctor `0x00614A30`: primary `0x00AA7CBC`,
paired `0x00AA7C70`, `+0x20` `0x00AA7CE0`, processor at `+0x2C+0x70`) with four overrides:

| Body | Address | Behaviour |
|---|---|---|
| `Deform` | `0x00614A90` | bends the flat piece model to the four corner heights (network-geometry.md §8.1's corner points) |
| `FlipAndLight` | `0x00616800` | the flip (`+0x104` bit 7) and the lighting pass |
| `Clip` | `0x00618B00` | the underground/clipping view |
| `Prelight` | `0x00614350` | the pre-light pass |

**Correction (NO22, NO23; re-networkmodel-answers4, from the bytes).** None of the four bodies is a virtual
override: no vtable or data reference points at any of them. They are ordinary members called only from
`cSC4NetworkOccupantWithPreBuiltModel::GetModelInstances` (`0x0061E8A0`). The instance's own vtables (`0x00AA7CBC`,
9 slots, and `0x00AA7C70`, 19 adjustor thunks) are the plain `cS3DModelInstance` ones plus a destructor
`0x00614830`. The instance is 0xE4 bytes: `new` at `0x0061E9xx`, ctor `0x00614A30` with the virtual-base flag 1,
and a `cS3DModelProcessor` at `+0x9C`. The "Prelight" name is wrong; §10.3 gives the real role of `0x00614350`.

### 10.1 Who builds the instances, and what `GetModelInstances` returns *(NO23)*

`GetModelInstances(a1, a2, outArray, a4, outCount)` (5 stack arguments, `RET 0x14`):

1. **Plain piece (`0x0061AF90`, every class).** It sets `*outCount = 0`, reads the piece position (the `+4` view's
   slot 8, `+0x20`) into the transform translation `+0x8C/+0x90/+0x94`, sets part-mask bit 1 at `+0x65`, and
   increments the change counter `+0x64`. It then stores **the piece's own embedded `cS3DModelInstanceBasic`**
   (the virtual base at `+0x38`, reached through the vbtable at `+0x3C`) in `outArray[0]`, sets the count to 1
   and AddRefs it. So the call never returns zero instances and never returns null: a piece with no exemplar
   model still returns this embedded instance, which draws the §2.2 quad and parts (§2.11). The ctor `0x00614A30`
   is not involved.
2. **Prebuilt piece (`0x0061E8A0`).** It calls step 1 first, so `outArray[0]` is the embedded instance. It then
   appends up to two new `cSC4NetworkOccupantModelInstance` objects:
   - **Main model.** `cSC4ModelMakerUtility::GetModelInstanceID` (`0x00497180`) takes the `+4` view, the
     rotation (primary slot 24), and a bool that is the negation of option `0x27812821`. A zero ID **returns at
     once**: no model instance, and the shadow update of the last step is skipped. Otherwise the resource
     manager (`0x00B45254`, IID `0x9AD0E7E3`) loads the model. On success a fresh instance is built **on every
     call** (`new 0xE4`, ctor, AddRef) and bound to the model by `0x00614890`. That function stores the model
     ref at instance `+0x28`, stores the LOD word 0 at `+0x94`, has the model fill the instance's mesh vector
     `+0x04`, and copies the model bounds to `+0x64`. Then the transform, the passes of §10.2 and the alpha are
     set. The instance is AddRef'd into `outArray[count++]`, passed to primary slot 81 (the prebuilt appendix,
     §2.8), and **cached in the AutoRef at `+0x1C4`**, which releases the previous instance. The model resource
     ref and the construction ref are then released. Net result: one reference for the caller and one in the
     cache. If the allocation fails or `0x00614890` fails, `+0x1C4` is cleared (and its old instance released)
     and nothing is appended. If the resource load fails, `+0x1C4` keeps its old instance and nothing is
     appended.
   - **Second model.** Built only when `+0x19C` is not zero, from the key {type `0x5AD0E817`, group
     `0xBADB57F1`, instance `+0x19C`}, in the same way. It is cached at `+0x1C8`. Its yaw adds f32 `+0x1A0`, its
     translation adds `+0x1A4/+0x1A8/+0x1AC` (on top of `+0x178…`), and it gets no `Deform` or `Clip`.
   - Last step: when `a1 != byte +0x1C0`, it calls `UpdateShadow(1)` and stores `a1` in `+0x1C0`.
3. **Ownership.** The model instance is **a separate heap object**, not the `+0x38`/`+0x68` sub-object of §2.1/§2.2.
   The piece owns it only through the `+0x1C4`/`+0x1C8` cache. Each draw query replaces the instance, so the
   deformation always starts from the flat model (§10.3). **Release (NO26):** the prebuilt `Shutdown`
   (`0x0061D0D0`) first asks the `+4` view's slot 5 (`+0x14`). When that answers true, it clears and releases
   `+0x1C4`, then `+0x1C8`, and tail-calls the base `Shutdown`. When it answers false, it returns 1 and touches
   nothing. `+0x1C0` is not reset. **Destructor:** the prebuilt destructor body is `0x00604CA0` (the export
   labels it `cSC4NetworkOccupant::~cSC4NetworkOccupant`, but it installs the prebuilt vtables `0x00AA7038`,
   `0x00AA6FB0`, `0x00AA6F94`, `0x00AA6F30`, `0x00AA6F18`, `0x00AA6F14`, `0x00AA6EC8`). It releases **`+0x1C8` first,
   then `+0x1C4`** (each: if non-null, call its slot `+8`, Release; `0x00604CEA…0x00604D05`). It does not clear
   them, and it then tail-jumps to the base destructor `0x0061C170`. The deleting destructor `0x00605020` calls
   that body and frees the object (`0x0061E300`) when bit 0 of its argument is set. The tunnel destructor body
   `0x00647260` (also labelled as the prebuilt one) releases its own `+0x1D0` and then runs `0x00604CA0`. A
   piece that was shut down first holds null caches at this point, so nothing is released twice.

**Transform written to each model instance** (before the passes; the instance's slot `+0x24` reads its transform
into a local, slot `+0x28` writes it back):
- the rotation starts from identity;
- yaw = `SC4ViewUtilities::GetObjectYRotationAngle` when the position query's flag byte is set, otherwise
  −(view global `0x00B43DD8` `+0x1C`), plus `+0x1A0` for the second model;
- the 3×3 rotation is built from that yaw (pitch and roll 0), and the mask gets bit `0x400`;
- **flip** (primary slot 25 true): the **first rotation row is negated** (a mirror);
- translation = position + (`+0x178`, `+0x17C`, `+0x180`), and the mask gets bit `0x100`.

When byte `+0xAB` ≠ 0xFF, the draw attributes get alpha `+0xAB × (1/255)` (constant 0.003921569) and flag bit 2.

### 10.2 Pass order *(NO22)*

| Path (main model) | Condition | Calls, in order |
|---|---|---|
| options skip | option `0x6A845768` true (its byte is saved at `0x0061E97B`) | none: the instance draws the shared flat model |
| surface | network-flag test (primary slot 8) `0x40000` false | `Bind` `0x00614350`, then `Deform` `0x00614A90`, then `FlipAndLight` `0x00616800` |
| clipped | `0x40000` true | `Clip` `0x00618B00` **first** (it runs even when the option is set), then `Bind`, then `FlipAndLight` (no `Deform`) |
| second model | option false | `Bind`, then `FlipAndLight` |

`Clip` is called at `0x0061EE60` as `Clip(&lo, &hi, 1)` with
- lo = (−FLT_MAX, h, −FLT_MAX) and hi = (FLT_MAX, FLT_MAX, FLT_MAX);
- h = (lowest of the four terrain heights at the piece's cell corners) − position y (`0x0061EE07`);
- the corner heights come from the terrain at `0x00B43CF4`: slot `+0x38` per integer vertex, or, when slot
  `+0x14C` answers true, slot `+0x78` at world (16·x, 16·z) and (16·x+16, 16·z+16).

### 10.3 The four bodies *(NO21, NO22)*

- **`Bind` (`0x00614350`, misnamed "Prelight").** It calls `cS3DModelProcessor::SetModel` (`0x008038D0`) on the
  processor at `+0x9C`, passing the instance's `+0x20` virtual-base view. SetModel keeps the shared vertex arrays
  (refcounted) and replaces each mesh's array with a **private copy** in the processor's vertex format. The copy
  takes x, y, z, u and v from the source and fills colour with the constant at `0x00B0E37C`; the bounds are
  copied too. A mesh whose source format is not `PackStandardVertexFormat(2)` makes SetModel drop the model.
  Every later pass edits only this private copy.
- **`Deform(rot, flip, P0, P1, origin, diag)` (`0x00614A90`).** The arguments are:
  - `rot` = primary slot 24 & 0xFF;
  - `flip` = slot 25;
  - `P0` = prebuilt `+0x184`, `P1` = `+0x190`, each three f32 {a, b, c};
  - `origin` = (0, 0, 0), or (−f32 `+0x168`, 0, −f32 `+0x16C`) when `+0x164` ≠ 0;
  - `diag` = the network-flag test `0x1000000`.

  It resets the instance bounds `+0x64…+0x78` to the empty sentinel (min 1, 1, 1; max 0, 0, 0). Then, for every
  vertex of every mesh (count u16 at array `+6`, stride u16 at `+4`, data at `+8`; position x, y, z at `+0/+4/+8`):
  1. Rotate the model x, z with the jump table `0x00614CE4`:
     - rot 0 (and any value > 3): x′ = x, z′ = z
     - rot 1: x′ = −z, z′ = x
     - rot 2: x′ = −x, z′ = −z
     - rot 3: x′ = z, z′ = −x
  2. If `flip` is set, x′ = −x′.
  3. Subtract the origin: x′ −= origin.x, z′ −= origin.z.
  4. Compute s = x′ + z′ when `diag` is set, otherwise s = x′ − z′. Use P1 when s ≥ 0 (also when s is NaN),
     otherwise P0 (`0x00614B87`, compared with 0.0 at `0x00A81054`).
  5. Set **y ← ((z′·b + x′·a) + c) + y**, summed on the x87 stack and rounded to f32 once at the store
     (`0x00614B9C…0x00614BAC`).

  Then it recomputes the bounds from all mesh vertices. The first mesh seeds them when max.x < min.x.

  **The planes** come from `RecomputeShearTransform` (`0x0061D230`). Write y0…y3 for the y values of quad
  vertices 0…3 (`+0xA0/+0xB8/+0xD0/+0xE8`) and Y for the piece position y. The constants are 0.5 (`0x00A84D2C`)
  and 0.0625 (`0x00AA81E4`).

  | `0x1000000` | c (both planes) | P0 {a, b} | P1 {a, b} |
  |---|---|---|---|
  | clear | (y0 + y2)·0.5 − Y | ((y2 − y1)·0.0625, (y1 − y0)·0.0625) | ((y3 − y0)·0.0625, (y2 − y3)·0.0625) |
  | set | (y1 + y3)·0.5 − Y | ((y3 − y0)·0.0625, (y1 − y0)·0.0625) | ((y2 − y1)·0.0625, (y2 − y3)·0.0625) |

  These two planes reproduce all three corner heights of their triangles only with the corners at x′, z′ =
  v0 (−8, −8), v1 (−8, +8), v2 (+8, +8), v3 (+8, −8). So the frame is **centred on the piece position with
  extent −8…+8**.
  - Diagonal: v0–v2 (x′ = z′) when the flag is clear, v1–v3 (x′ = −z′) when it is set.
  - **Shape: two triangles, not bilinear.**
  - **The bend is added to the flat y**, relative to Y, and always starts from a fresh private copy of the flat
    model.
  - **Nothing is clamped**: a vertex outside ±8 takes the plane of its side of the diagonal, extrapolated.

  Worked example (flag clear, y = (0, 4, 8, 0), Y = 0): P0 = (0.25, 0.25, 4), P1 = (0, 0.5, 4). Results:
  - vertex (8, 8, flat y 0): s = 0 picks P1, y = 8 (corner v2);
  - (−8, 8): P0, y = 4 (v1);
  - (8, −8): P1, y = 0 (v3);
  - (12, 0): P1, y = 4 (extrapolated);
  - with flip set, model (8, −8) maps to x′ = −8 and gets P0, y = 0.
- **`FlipAndLight(flip)` (`0x00616800`).** When `flip` (slot 25) is set, every mesh gets a new index list, which
  **reverses the winding**:
  - the old indices are copied in reverse order;
  - each primitive range's start becomes N − start − count, where N is the index count (or the vertex count when
    the mesh has no index list);
  - a mesh with no index list gets a synthesized descending list (max − 1 … min) and its ranges are rebased by −min.

  It moves no vertex and changes no UV: the mirror itself is the negated transform row of §10.1. Whether or not
  `flip` is set, it then calls `cS3DModelProcessor::LightModel(*0x00B43DDC, flip)` (`0x00803E10`). LightModel
  builds vertex normals (`0x00803370`) when the processor has none and lights the copy through the instance
  transform; when `flip` is set it negates all nine rotation entries. FlipAndLight then calls `ReleaseModel`
  (`0x00802B30`), which drops the processor's references.
- **`Clip(lo, hi, mode)` (`0x00618B00`, NO25).** Arguments: `lo` at `[esp+4]`, `hi` at `[esp+8]`, `mode` byte
  at `[esp+0xC]`, `RET 0xC`.
  1. **Early out.** A null `lo` returns at once. The bounds are read through slot `+0x1C` of the `+0x20` view
     into a local that starts as min (1, 1, 1), max (0, 0, 0). With `mode` ≠ 0 it cuts when **min.y < lo.y or
     max.y > hi.y**, an unordered compare counting as "cut" (`0x00618B65…0x00618BC0`). With `mode` 0 it tests
     x and z only: min.x < lo.x, min.z < lo.z, max.x > hi.x, max.z > hi.z. If no test fires, it returns with
     nothing changed, the bounds included. At the §10.2 call site (hi = +FLT_MAX), the piece is cut exactly when
     its bounds' min.y < h.
  2. **Per mesh** (slot `+0xC` gives the count, slot `+0x10` the mesh): the source format (dword at the vertex
     array `+0`) selects the path:
     - 2 and `0x80004001`: 20-byte output vertices {x, y, z, u, v} at `+0/+4/+8/+0xC/+0x10`, through the
       pooled clipper `0x00B45240`.
     - 10: 24-byte vertices {x, y, z, colour u32 at `+0xC`, u `+0x10`, v `+0x14`}, through the pool
       `0x00B45244`.
     - Any other format runs no clipping. The mesh still gets the fresh, **empty** vertex array (same format)
       and an empty index list (`0x00618EC0` onward installs them unconditionally).

     The source is read with its own stride (u16 at array `+4`).
  3. **Clipper state:** lo (3 f32), hi (3 f32), the source data pointer and stride, the output vertex vector, the
     output index vector, a map from source index to output index, and `mode` (clipper `+0x4C`).
  4. **Ranges.** Each primitive range (12 bytes: {kind, start, count}) is walked as a **triangle list**:
     - triangle k uses indices start + 3k … start + 3k + 2, taken while 3k < count − 2;
     - with no index list, the indices are the vertex numbers themselves;
     - afterwards the range's start = the output index count before it, and its count = the indices it emitted;
     - ranges that emitted nothing are removed, keeping order (`0x006189E0`);
     - a mesh with no ranges emits nothing and ends empty.
  5. **Outcode per vertex** (`0x00614100`): +1 if x < lo.x, +0x10 if y < lo.y, +2 if z < lo.z, +4 if x > hi.x,
     +0x20 if y > hi.y, +8 if z > hi.z, all strict. A NaN coordinate gives 0 (inside). For a triangle:
     - the AND of its three codes is non-zero: dropped;
     - all three codes are 0: emitted as is;
     - otherwise it is clipped, with `mode` ≠ 0 by the **y-slab** clipper (`0x006175C0` for 20-byte vertices,
       `0x00618030` for 24-byte). `mode` 0 uses `0x00616F60`/`0x00617980` (all six planes). The network call never
       passes `mode` 0, and that path is not specified here.
  6. **Y-slab clip (kept side).** Only the y bits (code & 0x30) count, so a vertex is kept when
     **lo.y ≤ y ≤ hi.y**. At the §10.2 call site that means **the geometry at or above h is kept and everything
     below h is removed**. The edges are walked as cur → next, with (v0 → v1), (v1 → v2), (v2 → v0), and each
     appends to a polygon:

     | cur | next | appended |
     |---|---|---|
     | in | in | cur |
     | in | out | cur, then X(from next toward cur) |
     | out | in | X(from cur toward next) |
     | out, out, a shared y bit | | nothing |
     | below and above (no shared bit) | | X(cur→next, t_min), then X(cur→next, t_max) |

     The polygon (3 to 6 vertices) is emitted as a **fan**: (p0, p1, p2), (p0, p2, p3), … (the tail loop of
     `0x006175C0`, index pushes around `0x00617908`/`0x0061793A`/`0x00617963`). The winding is kept.
  7. **The cut vertex X(from out toward in)** (`0x00614490`, 24-byte `0x006145D0`):
     - t starts at 0; for each set bit, t_b = (plane − out.c) / (in.c − out.c), with plane lo.y for 0x10 and
       hi.y for 0x20;
     - t_b is computed on the x87 stack from f32 operands, compared unrounded, and stored as f32 only when
       t_b ≥ t (the first test is against 0.0 `0x00A81054`; a NaN fails).
     - In the "below and above" case, t0 = (lo.y − cur.y) / (next.y − cur.y) and t1 = (hi.y − cur.y) /
       (next.y − cur.y) share the f32-loaded denominator, are each stored f32, and are swapped when t0 > t1
       (`0x00617738…0x00617773`).
     - Every float attribute is **a′ = (in.a − out.a)·t + out.a**, on x87 and rounded to f32 at the store, in
       the order x, y, z, u, v (`0x00614170`; 24-byte `0x00614200`).
     - Format 10 colour: each byte c of `+0xF`, `+0xE`, `+0xD`, `+0xC` is **trunc(int(in.c − out.c)·t +
       out.c)** through `_ftol2` (`0x0061424F…0x006142A9`), with no rounding to f32 first.
  8. **Output vertices.** The first time a source vertex is used by an emitted triangle, it is copied to the
     output (a per-mesh map, so shared source vertices stay shared). A cut vertex is appended every time, with no
     sharing, even between the two triangles of a shared edge. Its output index is the vertex count just before
     its append. The new vertex count is stored as u16 at array `+6`.
  9. **Bounds after a cut.** With the meshes done, the instance bounds are **clamped to the box, not
     recomputed**: per axis, min = lo if min < lo, and max = hi if hi < max (`0x00618FD2…0x00619024`; an
     unordered compare keeps the old value). This runs for all three axes in both modes. At the call site it
     raises min.y to h.

  **Worked vectors (h = lo.y = 0, hi = +FLT_MAX, format 2, vertex = (x, y, z; u, v)).**
  - A: v0 (0, −2, 0; 0, 0), v1 (4, 2, 0; 1, 0), v2 (0, 2, 4; 0, 1). Codes 0x10, 0, 0.
    - Edge 0→1 gives X(v0→v1), t = 2/4 = 0.5: (2, 0, 0; 0.5, 0).
    - Edge 1→2 gives v1.
    - Edge 2→0 gives v2, then X(v0→v2), t = 0.5: (0, 0, 2; 0, 0.5).
    - Polygon [X01, v1, v2, X02]; triangles (X01, v1, v2) and (X01, v2, X02).
    - Output vertex order: X01, v1, v2, X02.
    - The bounds' min.y becomes 0.
  - B: v0 (0, −2, 0), v1 (4, −2, 0), v2 (0, 2, 0), with u, v = x/4, z/4.
    - Edge 0→1 gives nothing.
    - Edge 1→2 gives X(v1→v2), t = 0.5: (2, 0, 0; 0.5, 0).
    - Edge 2→0 gives v2, then X(v0→v2) = (0, 0, 0; 0, 0).
    - One triangle (X12, v2, X20).
  - C: all three y < 0: dropped. y = 0 exactly counts as inside (strict compare), so a triangle touching h only
    at vertices with y = 0 is emitted unchanged.
  - D: t is taken from the **outside** vertex. Take out.y = −1, in.y = 2. Then t = 1/3 (f32 0x3EAAAAAB), and
    in.x = 3, out.x = 0 give x′ = (in.x − out.x)·t + out.x = 3·t = 1.0000000298… → f32 1.0 (0x3F800000).
    Anchoring on the inside vertex instead gives a different bit pattern. With t′ = (plane − in)/(out − in)
    = f32 2/3 (0x3F2AAAAB), the weighted form in.x·(1 − t′) + out.x·t′ gives 1 − t′ = f32 0x3EAAAAAA, and
    3 × that = 0.99999994 (0x3F7FFFFF), one ulp below the native 1.0. (The inside delta form 3 + (0 − 3)·t′
    happens to round to 1.0 for this vector, but it is not the native order either.) To match bit-for-bit,
    interpolate from the outside end exactly as rule 7 says: t from the outside vertex, stored as f32, then
    (in − out)·t + out (0x00614490/0x00614170).

---

## 11. Quirks (reproduced unless marked *[choice]*)

1. **One type for nine classes** (§1.1): plain, prebuilt, bridge, tunnel, underground, pipe and subway pieces all
   report occupant type `0xC772BF98`; everything downstream QIs instead of dispatching on the type *[inferred from
   the consumers]*.
2. **The pole's null-`this` QI** (§1.2): `QueryInterface` writes 0 to the out-pointer and returns 1 even when
   `this` is null, for the `0xA8C86142`/`0x09C05C6A`/`0xA9764C8C` and `0x452294AA` branches.
3. **`IsOfType` is a flag test** (§1.4): it answers "does the piece carry network flag `1<<t`", not "is the class
   of type t".
4. **The folded `Shutdown`** (§3.3): pipe and subway occupants share one 5-byte shutdown body `0x00652CF0`.
5. **The `SetWateredState` seam** (§3.3): the water simulator drives the pipe through a **non-virtual** member,
   not an interface slot (plumbing.md §6.4; direct-calls.md).
6. **`Read`'s version zoo** (§2.3): the base record is written as 8 and read with conversions for 4–7; the
   underground record is v3, the prebuilt record v4 (read accepts 3), the tunnel record v2, the connection record
   v2, the pole record v5 (read rejects < 4), the line record v2, the pylon record v5.
7. **The prebuilt's `+0x1B8`/`+0x1BC` are −1** out of the constructor (§2.8).
8. **The pylon's occupant type is its own** `0xA9B6D7DE` (§6.1), not the bare occupant type an earlier draft
   guessed from the missing write.
9. **`GetOccupiedCell`'s rounding** (§2.9): `v · 0.0625 + 1.5 × 2^23` stored as float32, minus the bias in its bits:
   round to nearest, ties to even, of corner 0's x and z — reproduce exactly (foundations §18).
10. **The path connections are 8 × 4 bytes at `+0x144`** (§2.6): a byte per (path type, entry side), a bit per exit
    side; path type 0 is always set along with the requested type, and the whole thing is gated by network flag
    `0x2000`.
11. **Pole records use class-ID-valued IIDs for distinct ref kinds** (§4.4): each destination
    pole uses `0x09C05C6A`, each linked-list power line uses `0xC9C05C5D`. Neither is an
    alternative fallback IID for the other kind. Four xyz vectors are saved/read through
    the selected shared connection-point table pointer, not inline object storage.
12. **The demolition cost is an int64** (§2.4, slot 63): returned in `EDX:EAX`, saved as one `SetInt64`
    (§2.3 field 13).
13. **The prebuilt primary's appendix** (§2.8): the vtable at `0x00AA7038` continues past the shared 81 slots with
    a long tail of entries before its RTTI column; the shared interface proper is slots 0…80. *[quirk: the extra
    entries are unreachable through the SDK order; reproduce the first 81 and their overrides.]*

14. **`GetNormal`'s x component** comes from the other triangle than `GetAltitude`'s (§2.9).
15. **`IsPartOfTheSameStructure`** compares uninitialised values when an origin is missing (§2.9).
16. **A wrong edge store version** is ignored and the record continues from there, misaligned (§2.3 field 6).
17. **Loading makes every piece opaque** and every part vertex alpha `0xFF` (§2.3).
18. **`GetAltitude` ignores the drawn diagonal** (flag `0x1000000`) (§2.9).
19. **`IsImaginaryConnection` has no bounds check** on its side (§3.2).
20. **The connection record's row 8 is never used**: `Read` sets `+0x17C` to true after reading it (§3.6).
21. **Loading a pole writes its saved attachment points into the shared table of its kind** (§4.4).
22. **`BreakAllConnections` leaves pole-less records in the vector**, so the connection count does not drop (§4.6).
23. **A pole off the map takes its floor height from uninitialised values** (§4.7).
24. **`GetConnectedPole` has no bounds check** (§4.2).
25. **A pole's `SetHighlight` does not change the occupant's `GetHighlight`** (§4.2).
---

## 12. Open questions

- **NO1.** **Answered 2026-10-21 (re-ugbase):** flag `0x40` is `1 << 6`, the avenue network type (`IsOfType(6)`, §1.4);
  `IsIntersection` is "an avenue piece whose piece id is in the 13-entry avenue intersection set" (§2.4, §2.5).
- **NO2.** **Answered 2026-10-21 (re-powerpoles):** `0x26A68448` is forwarded to the pole's model instance
  `+0xBC` (a model instance's mesh container, §4.1).
- **NO3.** **Answered 2026-10-21 (re-ugbase):** no; the underground constructor runs the network constructor first,
  which writes `0xC772BF98` (§3.2).
- **NO4.** The semantics of the prebuilt `Write` fields `+0x170`/`+0x174`/`+0x178`/`+0x17C`/`+0x180` (§2.8):
  model refs? shear inputs?
- **NO5.** **Answered 2026-10-21 (re-ugbase):** the slots that use them carry the SDK names (§2.2 "Properties");
  `0x4835C6A9` is the connection path-ID override.
- **NO6.** Pipe `Read`'s extra u32 → object `+0x270` (§3.3; code offset `+0x268`): meaning (funding state?
  watered?) unverified. (The byte `+0x260` it sets is "refresh pending", §3.1.)
- **NO7.** **Answered 2026-10-21 (re-ugbase):** the piece id `+0xFC` (§3.1 slot table, drawable 17).
- **NO8.** **Answered 2026-10-21 (re-ugbase):** they are the classes' static model descriptors, the second argument of
  `Init(classID, descriptor)` (§3.6).
- **NO9.** ~~The power-pole `Write` version number~~ — **settled while writing this spec**: it writes version
  **5** (`PUSH 0x5` at `0x0064B01D`); `Read` rejects < 4. No question remains.
- **NO10.** ~~The pylon's occupant type~~ — **settled while writing this spec**: the ctor writes `0xA9B6D7DE`
  at `+0x2C` (`0x005F19F9`). No question remains.
- NO38–NO40 (re-ugbase) and NO42–NO43 (re-powerpoles), 2026-10-21, are in [questions/network-occupants.md](questions/network-occupants.md).
  NO41 is answered in §3.6.3/test 51 and the answered archive.
- NO33 remains open for complete pole owner algorithms; its partial rules and vectors are in
  §§4.1/4.6/4.7.1–4.7.2. NO36 is answered by §§6.1–6.2.4 and tests 40–42, 46–48, 52–53, 56–59.
  UG5 is tracked in
  [questions/underground-occupants.md](questions/underground-occupants.md); §§3.6–3.6.1 specify its verified
  record, creation-input, neighbor and state/ref subset.

---

## 13. Conformance tests

Each test names its section. A "city" is a fresh 64 × 64 city unless stated otherwise.

1. **Identity** (§1.1, §1.2). A newly created road piece answers QI `0xA821EF94` with the object base; QI
   `0xA9764C8C` likewise; QI `0x26BB767E` returns base `+4`; QI `0xE4FDA3D4` returns `+8`; an unknown IID reaches
   the property holder and comes back false. `GetGZCLSID` returns `0xC9C05C6E`.
2. **Types** (§1.1). `GetType` on a road piece returns `0xC772BF98`; on a pole `0x2890D4DE`; on a line
   `0x0990B2EA`; on a pipe connection `0x2A183167` **only after `Init`**; on a subway connection `0x8A13918A`
   after `Init`; on a pylon `0xA9B6D7DE`.
3. **`IsOfType`** (§1.4). For a road piece (flag bit 0 set), `IsOfType(0)` is true and `IsOfType(1)` is false;
   the result equals `GetNetworkFlags() & (1<<t) != 0` for every t in 0…12.
4. **Object sizes** (§1.1, §4.1). A pole allocates `0x124` bytes. The other classes' sizes are whatever their
   fields sum to; only the pole's is load-bearing (its pool).
5. **Occupied cell** (§2.4). `GetOccupiedCell` of a piece positioned at world (800.0, 240.0) returns (50, 15),
   including for negative coordinates down to −4096 (the x87 rounding of quirk 9).
6. **Rotation and flip** (§2.4). `SetRotationAndFlip(2)` (slot 74) then `GetRotation` (slot 24) returns 2; setting
   the flip bit (`+0x104 |= 0x80`) makes `GetFlip` (slot 25) return 1 while `GetRotation` still returns the low 7 bits.
7. **Network flags** (§2.7). `SetNetworkFlag(0x10000000)` on a piece in the city posts `0x89D3E9F3` with data1 = 1
   exactly once; `ClearNetworkFlag(0x10000000)` posts data1 = 0; setting `0x4000` also calls slot 58.
8. **Managed-globally** (§2.7). A plain road piece answers `IsManagedGlobally()` false; a bridge and a tunnel
   piece answer true; the network manager's `managed` list gains exactly the true ones (network-manager.md §6.1).
9. **Path connections** (§2.6). `AddPathConnection(pt, e, x)` sets the matrix bit for `pt` **and** for type 0, sets
   flag `0x2000`, and `PathConnection` then matches; with `0xFF` wildcards it returns the count.
10. **Init/Shutdown round trip** (§2.5). `Init` sets `+0x4C = 0x1A` and clears flag `0x2000`; a second `Init` on a
    piece with `0x10000000` set posts the usable message (data1 = 1) again and keeps the flag; `Shutdown` does not
    crash on a piece with no model.
11. **Base record round trip** (§2.3). Save and reload a city with road/rail/highway pieces: every field of
    §2.3 is identical, the version is 8, and the flag mask `& 0xFFFF1FFF` is applied on read (a save with stray
    high bits reads them cleared).
12. **v4/v5/v6/v7 record reads** (§2.3). A hand-built v5 record with the legacy transform loads; a v6 record's
    flags go through `ConvertV5FlagsToCurrentFlags`; a v7 record skips the position recompute branch.
13. **Underground record** (§3.4). A pipe occupant saves and reloads with object `+0x268`, `+0x270`, the four heights
    `+0x24C`…`+0x258` and the six wall vertex lists identical; after read the bytes `+0x260 = 1`, `+0x261 = 1`,
    `+0x262 = 0`. The stream is u16 3 (pipe), u16 3 (underground), u16 8 (base) in that order; the underground
    tail with empty wall lists is 137 bytes.
14. **Tunnel pairing** (§2.8). Two portals built across a hill save and reload with the `+0x1D0` references
    resolved to each other (traffic-network.md §4.8).
15. **Connection occupants** (§3.6). A pipe connection saves as class `0xEA0A5ACF` with the seven wall writes and
    reloads; `QueryOccupantInterface` (subway, slot 44) QIs `0xCA16374F` then `0x4A10AFC2`.
16. **Pole record** (§4.4). Round-trip a v5 pole with two destination refs. The first
    connection has line list [L1,L2], the second an empty list: reference read IIDs are
    respectively 0x09C05C6A, 0xC9C05C5D, 0xC9C05C5D, 0x09C05C6A, with list counts
    2 and 0. Set the first connection's two nested geometry collections to
    [[(1,2,3)],[]] and [[]]: their count/triple sequence is
    `2,1,(1,2,3),0,1,0`, occupying 32 bytes. Both empty inner vectors survive.
    Signed fields −7 and 19 remain at record+0C/+10 after read; bool bytes 0x80
    normalize to 1. Distinct model IDs 11/12 and cells −3/4 round-trip at live
    +C0/+C4/+C8/+CC; exemplar index at +B8 is not a vector pointer.
    Four sentinel xyz triples read into the chosen +E4 table at offsets 0/0C/18/24;
    the three following wall-vector headers stay independent. Give the first wall
    list one vertex xyz=(1,2,3), uv=(0.25,0.5), RGBA=(0x11,0x22,0x33,0x44): its
    count is 1 and its payload is five f32 followed by those four bytes (28 bytes
    total); live BGRA at vertex+0C is 33/22/11/44. Other empty wall lists consume
    four bytes each. Version 3 rejects before base; v4 uses the legacy transform;
    v5 uses current. With a preexisting delegate D, a failed body read still
    clears/releases D once when cleanup is reached and returns false for sticky
    stream error; base-read failure leaves D untouched. Assert that live vector
    end +B0 is never passed to Release.
17. **Pole cost** (§§4.2–4.3). In separate fresh static-init fixtures, omit the control exemplar,
    supply it without `0x0A2ECED8`, or supply a non-convertible value: the quote stays zero.
    A SINT64 `0x00000001_00000002` returns EDX=1, EAX=2; SINT64 −7 returns
    EDX=`0xFFFFFFFF`, EAX=`0xFFFFFFF9`. Change only `0xE9A18D3F` from 16 to 32:
    the quote is identical. After the first static init, changing `0x0A2ECED8` and
    initializing another pole also leaves the old quote intact. Calling only the
    six-control reload does not reset an already supplied quote.
18. **Line slots** (§5.1). `GetConnectedPoles` returns the three refs with `AddRef`; `SetConnectedPoles` releases
    the old ones; `IsPreview`/`SetPreview` round-trip.
19. **Pylon record** (§6.3). A v5 fixture starts its derived body with dirty=1,
    UI=−1, dispatch state=−128, dispatch type=127, blink=0, call=0x1234:
    bytes `01 FF 80 7F 00 34 12` precede the source reference. Read produces live
    +6C=−1, +70=−128, +74=127 and +14C=0x1234. Expect source IID 0x26BB767E
    and owner IID 1. Exactly one transform is consumed; only v3 uses its legacy
    form. Drag byte 0x80 reads as 1. A v4 Read into an object with bitmap/state 9,
    string "kept" and owner O preserves all three and does not release O; on a
    fresh object the same tail remains 0/empty/null. Version 2 rejects before the
    base and releases no refs. Inject failure after the source replacement step:
    old source has already been released, no rollback occurs, final result false.
20. **Pylon lifecycle and placement** (§6.2). Second Init returns true with no
    subscriptions/packing. Renderer +28 may return either value without changing
    the two packing tuples or Init's result. Starting byte +37=0x80, successful
    renderer insertion traces city+88, manager insertion (including its own
    city+88), occupant+80(4), renderer+70; resulting byte=0x83. If the final renderer
    insert fails it is 0x81 and result false. Removing from the renderer yields
    0x81; removing from the manager while both inserted calls renderer+74 before
    manager+54 and yields 0x80. With a null renderer, direct removal preserves
    bit 1 and fails; manager removal with an available manager clears both bits.
    Matching-state requests return true even without the associated manager or
    renderer. Shutdown of an initialized object removes placement, unsubscribes
    three messages and tick category 0x80, and releases source +78 and owner +1C0.
21. **Type info load** (§8). `ReloadRules` for the road tool fills all 13 records: the widths match
    network-geometry.md §2.3, the placement parameters §2.4, `LoadComplete` sets `+4 = 1`, and the road record's
    p+0x00 slope is 35 degrees → `+0x100 = r24(tan(35°) × 16)`.
22. **Power line has no placement exemplar** (§8, network-geometry.md §2.4): the power line record's parameters
    stay at the defaults and its plane tolerance p+0x20 is 0.
23. **Demolition region** (§9). `ComputeDemolitionRegionForPlacementCells` over two cells whose lots overlap
    returns 0 when either cell's zone type has a bit within `0xFC00` set, and otherwise demolishes exactly the
    merged region (flags `0x2001`).
24. **Model instance** (§10). `GetModelInstances` (slot 33) returns the `cSC4NetworkOccupantModelInstance`; after
    `Deform`, the rendered vertices' y values equal the four corner heights interpolated per quad position.
25. **No network lot occupant** (§7.1). After building a road, the occupant manager holds exactly one occupant
    per road cell (the piece); the network lot manager's lot at that cell references the same piece occupant; no
    separate lot occupant object exists.
26. **Fields** (§2.2). `SetVariation(3)` (slot 27) makes `+0x105` = 3 and posts ChangeOccupant with data2 = 1;
    `SetUnderTexture(t)` (slot 29) writes `+0x100`; `PieceId` (slot 23) reads `+0xFC`.
27. **Old origin** (§2.3). A version 6 record with a path override, corners 0 and 2 at x 32 and 48, z 64 and 80,
    and `dx = dz = 0` gives the complex piece origin (2, 4) and sets the retrofit flag.
28. **Altitude** (§2.9). Corners at heights (0, 16, 32, 16): `GetAltitude(x0 + 4, z0 + 8)` = (4 × 16 + 8 × 16) ×
    0.0625 = 12; `GetAltitude(x0 + 8, z0 + 4)` = (8 × 16 + 4 × 16) × 0.0625 = 12.
29. **Edges** (§2.10). `SetEdgeTypesFlag(0, 0x00020002)` then `GetActiveEdges()` = 5 and
    `GetActiveEdgesForTypeNoGroup(0)` = 5; `SetEdgeType(0, 1, 4)` makes `GetGroupedEdges()` = 2.
30. **Path connections** (§2.6). `AddPathConnection(3, 0, 2)` sets flag `0x2000`; `PathConnectionExists(3, 0, 2)`
    and `(0, 0, 2)` are true; `(3, 0xFF, 0xFF)` is 1.
31. **Texture coordinates** (§2.11). Rotation 1 gives corner 0 the coordinates (0, 1); rotation `0x81` gives
    corner 0 (0, 0).
32. **One-way arrow** (§2.11). A one-way straight piece with edges `0x00020002` and directions with net sum ≥ 0
    is drawn with texture `0x0BE35EB0` at rotation 1; with a negative sum at rotation 3.
33. **Translucency** (§2.11). `SetTranslucency(true, 0x80)` makes the corner alphas `0x80`, the opacity 128/255 and
    posts ChangeOccupant; after save and load the piece is opaque again.
34. **Ceiling** (§3.2). An underground piece `0x300` with original heights 10, 12, 10, 12 and the default statics
    has the ceiling 10 − 3 = 7.
35. **Pole identity** (§4.1). `QueryInterface(0xA8C86142)` succeeds; `QueryInterface(0x26A68448)` on a pole with a
    model instance returns that instance's mesh container, and fails without one.
36. **Line record** (§5.2). A line saves u16 2, the occupant record, 12 bytes of position, 8 of cell, one byte and
    two refs; version 1 fails.
37. **Line position** (§5.1). `SetPosition((40, 3, 70))` gives the cell (2, 4) and a point bounding box at
    (40, 3, 70).
38. **Distress** (§4.7). `SetDistressed(true)` creates one `powerpolezap` effect; a second call creates no other;
    `SetDistressed(false)` stops and releases it.
39. **Break all** (§4.6). After `BreakAllConnections(true)` on a pole with two connections, `GetConnectionCount` is
    still 2, both records have no pole, and the other poles have no record of this one.
40. **Pylon state setters and references** (§6.2.1). With dirty=0, blink=1 and UI=2, SetUIState(2) leaves
    dirty=0/blink=1; SetUIState(3) gives dirty=1/blink=0. Repeat for dispatch type/state, including values
    outside signed-byte range. SetBitmapInfo with equal inputs still dirties. For source and owner A→B,
    trace AddRef(B), store B, Release(A); B→B produces no reference calls; B→null stores null before
    Release(B). All these ref changes leave a clean dirty byte and blink untouched.
41. **Shared pylon blink timer** (§6.2.1). With timer=0, eligible pylons P/Q blinking 0: P tick(300) gives
    timer=300 and no blink change; Q tick(250) gives timer=0, Q blink=1/dirty=1 and P blink=0. P tick(1200)
    toggles P only once and leaves timer=0. A state-2 pylon tick(500) changes nothing. With timer=0xFFFFFFF0,
    an eligible tick(32) wraps to 16 and does not toggle. Reassigning Q's unchanged dispatch state preserves
    its blink; changing it clears the blink without resetting the shared timer.
42. **Pylon position and visibility** (§6.2.1). Null input produces no placement/model calls. From transform
    counter 255/mask 0x80, SetPosition((4,5,6)) gives translation (4,5,6), counter 0/mask 0x81, with remove
    manager → remove renderer → RebuildModel → insert manager → insert renderer in order. Repeat on an
    initially unplaced object: insertion is still requested. SetVisibility on an uninserted object calls
    only the base; on a manager-inserted object it additionally requests renderer placement matching the
    supplied visibility, using the placement semantics in test 20.
43. **Pole constructor/table selection** (§4.1/§4.6). A fresh pole has no connections/delegate/effect,
    no table, rotation/zoom −1, identity transform and stale=true. Exemplar instances `0x00000100`,
    `0x00000200` select A; `0x00000300`, `0x00000400` select B. On a fresh object subtype 0 leaves
    null; A→subtype 0 preserves A; B→subtype 15 preserves B. Compare attachment coordinates as f32
    bits, e.g. A r0 point0 `(0,0x41FD999A,0x4099999A)` and B r0 point0
    `(0xC0533333,0x41FD999A,0x40533333)`, before position addition.
44. **Pole floor/wall geometry** (§4.7.1). With a non-diagonal model, position `(10,20,30)` and floor
    height +10C=4, floor vertices are `(5,20,25),(5,20,35),(15,20,35),(15,20,25)` with the four
    listed UVs, white RGBA. Rotation 0 first wall is `(15,20,35),(15,4,35),(15,4,25),(15,20,25)`
    with UVs `(0,0),(0,1),(1,1),(1,0)`; second wall uses floor endpoints 1/2. Rebuild twice: four
    floor vertices and four per wall, with no growth. Rotations 1/2/3 follow the pair table.
    In an AddWallQuad-only fixture use top A y=36/alpha=64, top B y=20/alpha=128, bottomY=4:
    V is 2 and 1 at the two bottom vertices, alpha sequence 64/64/128/128. With topA y=−12,
    its bottom V is −1, with no clamp.
45. **Pylon constructor enums** (§6.1). Construct over storage whose +6C/+70/+74 words are
    `0x13579BDF/0x2468ACE0/0xA5A5A5A5`: +6C becomes 1, +70 becomes 0, and +74 remains
    `0xA5A5A5A5`. Repeat with a different +74 sentinel; construction must not promise zero there.
    SetDispatchType(4) then makes it 4 with the setter effects in test 40. A portable implementation
    can represent the pre-set dispatch type as unspecified rather than reading uninitialized memory.
46. **Pylon cached bounds and transforms** (§6.2.2). Seed distinct local/world boxes and dirty=1:
    getters return the seeded boxes unchanged and leave dirty=1. Texture ID `0x12345678` is also
    GetSortValue's result. GetTransform into a counter-255 output returns counter 0 and the internal
    mask/13 floats while preserving the internal counter. SetTransform with input counter 99 into
    internal counter 255 gives internal counter 0, copies mask/13 floats, and leaves dirty/bounds intact.
47. **Pylon pick** (§6.2.2). UI=0 never tests the ray. With UI=1, cached maxY=100, view conversion
    of 50 equal to 20 and pitch zero, the tested minY is 80 regardless of cached minY. A hit at
    distance 8 replaces old distance 10 and succeeds; against old distance 8 it fails without changing
    the distance. Dirty stays unchanged in all cases.
48. **Pylon texture replacement** (§6.2.2). Type 6/state 6 requests instance `0xEA3EE120`;
    type 5/state 1 requests `0x14315E10`; type 2 requests zero. With old raw ID 7 and no binding,
    a failed replacement releases 7 but retains +148=7 and returns true. Repeat from +148=0:
    failed replacement returns false. With an old binding, release/null that binding instead of
    separately releasing its raw ID. A successful new binding replaces +148 with its own ID.
49. **Unattached connection builders** (§3.6.2). With no attached occupant and four distinct surface
    vertices, CreateFloor emits exactly four white-RGB copies preserving alpha/UV/position. Calling it
    twice leaves four vertices. With an attached occupant it clears to zero and leaves the latch alone.
    CreateWalls without an attachment emits four lists for `(1,0),(0,3),(3,2),(2,1)` and count 4;
    with an attachment it clears all four lists but retains a seeded count 3 and does not notify.
50. **Connection wall slope/append** (§3.6.2). Surface A=`(2,4,6)`/alpha 64, terrain A y=36;
    surface B=`(8,12,10)`/alpha 128, terrain B y=28. The appended positions are
    `(2,36,6),(2,4,6),(8,12,10),(8,28,10)`; UVs `(0,0),(0,2),(1,2),(1,0)`;
    alphas 64/64/128/128 and white RGB. B's drop 16 does not give a different V.
    Appending to a one-vertex destination leaves five vertices. A negative A drop gives negative V.
51. **Auxiliary saved words** (§3.6.3). Load underground +268=`0x12345678`; GetModelKey and the next
    save both retain that word, independently of the live descriptor pointer. For a connection,
    retain shade index 2 rather than converting it to true/1; pass shade RGB at `0x00B0CFC8` to Draw.
    Starting from wall count 3, attached CreateWalls clears the lists and retains 3; after detachment
    rebuilding gives count 4. A linked position y=40 and cell size=16 gives +170=40,+174=24 when unattached.
52. **Pylon pass and dirty behavior** (§6.2.3). With dirty=true, selfLit=true returns 1 and leaves
    dirty set with zero submissions. With selfLit=false and the context gate clear, pass one also
    leaves dirty set: dispatch state=0, call=7, type=0 submits main and label, while states 1/4,
    call=0 or type=4 suppress only the label. Pass zero rebuilds once, submits just the main quad
    with transparency bits `0x3ECCCCCD`, resets transparency, and returns 2. Unsupported pass 2
    neither rebuilds nor submits. A zero raw texture ID does not suppress the main submission.
53. **Pylon messages** (§6.2.3). Animation payloads 999/1000/1999/2000 deliver tick values 0/1/1/2.
    A visibility rejection while flag 2 is clear produces no renderer removal; when set, it clears
    the flag and removes once. Repeated acceptance adds only on the first clear-to-set transition.
    The view-change message marks dirty without a rebuild; `0x26D31EC1` rebuilds immediately.
54. **Pole occupant bounds** (§4.7.2). Cell extents 16×16, position `(24,50,40)`, floor height=7,
    no sampled wires, dirty=true: box min `(16,7,32)`, max `(32,110,48)`; dirty becomes false.
    An empty-wire record targeting `(9,9)` changes nothing. A nonempty record targeting `(3,1)`
    gives min `(16,7,16)`, max `(64,110,48)`. Changing that record while dirty=false leaves the
    cached x/z rectangle unchanged; changing position.y still changes the returned maximum y.
55. **Pole bounds unsigned quirk** (§4.7.2). Seed dirty=true, position cell `(1,2)`, and a nonempty
    record targeting x=`0xFFFFFFFF`, z=2. With extents 16×16, minX stays 1 and maxX becomes
    `0xFFFFFFFF`; the wrapped increment gives returned maximum world x=0, less than minimum x=16.
    Do not sort the resulting endpoints or clamp the destination cell.
56. **MySim pixel predicate** (§6.2.4). With a one-pixel-wide portrait whose rectangle starts at
    `(0,0)`, d=25: its three copies start at `(77,12)`, `(129,12)`, `(181,12)`. At an eligible
    destination, source `0x12FF00FF` preserves the old pixel; source `0x00112233` replaces it
    completely. Destination `0x40000000` blocks either source. A missing portrait still uploads
    the background; a missing background returns false without upload. State 1 selects
    `0x2BB06F3F`; states 0 and 6 select `0x2BB075B4`.
57. **Main billboard** (§6.2.4). Fixture V is identity, pitch=0, view +54=0,+5C=−1, initial
    matrix identity, scale=1, counter=0, mask=0, position `(10,20,30)`, UI=0, blink=false,
    type=0, call=0. Rebuild gives half-width 22, height 150, vertices
    `(-22,150,0),(-22,0,0),(22,0,0),(22,150,0)`, UV x values 0.171875/0.34375 and bottom
    UV y 0.5859375. Bounds are local `(-22,0,0)..(22,150,0)`, world
    `(-12,20,30)..(32,170,30)`; mask bit 4 set, counter=1, dirty=false. A second rebuild
    resets that rotation and rotates again, giving counter=3. A failed texture replacement
    still produces those vertices and bounds. Blink=true selects k=3 regardless of UI.
58. **Call-number replacement** (§6.2.4). Call=7 formats `7`. For font results A=20,B=10,
    S=32; for A=33,B=10, S=64, not 32. Start labelID nonzero and make GS bitmap acquisition
    fail: release the old texture, retain labelID=0, still rebuild label vertices and restore
    the font setting. A failed initial bitmap clear lock does not suppress the font draw.
    Setting call=0 before RebuildModel instead preserves any old label ID/vertices untouched.
59. **Label geometry and ownership** (§6.2.4). With V identity, pitch=0, h=150, A=20,B=10,
    S=32, the vertices have x=−10/+10, top y=127, bottom y=113, z bits `0x3C23D70A`,
    right u=0.625 and bottom v=0.3125. Changing only the first geometry argument W leaves
    these unchanged. A MySim upload writes main +148; a call-number upload writes +150.
60. **Connection attachment messages** (§3.6.4). Query an inserted occupant with matching cell
    and +17C=true: replace/hold attached, release the temporary query ref, but do not rebuild or
    notify the renderer. Repeat with the same pointer: old Release and new AddRef still occur.
    A cell mismatch releases only the temporary and preserves attached. A failed interface query
    with payload=primary calls attached slot 89(true); a successful query never uses this fallback.
61. **Connection removal messages** (§3.6.4). With position.y=40, cellSize=16, +170=91, and
    a matching queried removal, attached becomes null, +17C=true, +174=24, +170 stays 91;
    no immediate build occurs. A linked-removal message for an unrelated pointer is inert; one
    for linked clears it before manager removal `(primary,0,0)`, attached refresh and Shutdown.
    Equal context +4 words suppress CreateGeometry even for different pointers; unequal +4 words
    request exactly one geometry build. All these cases, and an unknown message, return true.
