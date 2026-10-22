# Pending ABI publication: land-value native Step capture

Lane: re-landvalue-step-abi4. Target: append the section below to `docs/abi/layouts.md`.
Status: proposed, not applied. Coordinator review/application and a **full train gate** are required;
do not land this ABI record through a docs-only bypass. No C ABI export/version change is proposed.
The existing `interfaces.tsv` rows already cover every virtual call named below; no TSV changes are requested.
Behaviour, ownership and the unresolved installation boundary are in landvalue §6.1.1 and §11.3.

## Land-value native progress (`cSC4LandValueSimulator`, 1.1.641 GOG, verified 2026-10-22)

All pointers and vtable entries are 4 bytes, little-endian x86. **L** is the successful
`QueryInterface(0xE7781D8C)` result (`cISC4LandValueSimulator`, vtable `0x00AB1670`);
unknown IID 1 returns this same base. **M** is the separately queried message-target IID
`0x452294AA` result (vtable `0x00AB1660`), and **S** is serializable IID `0xE4FDA3D4`
(vtable `0x00AB1648`). Native base relationships: M = L + 4; S = L + 8.
Offsets in the first table are relative to **L**, never M or S. These are a partial object layout,
not an object allocation size or permission to construct an instance.

| Offset | Width (bytes) | Native field / interpretation |
|---|---|---|
| +0x38 | 4 | pipelineState, int32 (0 through 9) |
| +0x3C | 4 | pointer to tract ScanningIterator |
| +0x40 | 4 | pointer to slope ScanningIterator |
| +0x4C | 1 | ignoreZoning, zero / nonzero |
| +0x70 | 4 | developer cursor, node pointer; equals header at end |
| +0x8C | 4 | developer-map header pointer (map base is the address L + 0x8C) |

QueryInterface establishes these bases at `0x006BD4DF–0x006BD4F9`, `0x006BD523–0x006BD52B`;
its success paths AddRef the shared object. Message-target slot 0 adjusts by −4
(`0x006BDDB0–0x006BDDB3`). Step receives L: DoMessage adjusts M by −4 at
`0x006BF113` / `0x006BF15D`. State is read at `0x006BE7CD`; iterator allocation/stores
are `0x006BE5CF–0x006BE5E8` and `0x006BE632–0x006BE648`; slope iteration reads +0x40
at `0x006BECD5`. Byte access to ignoreZoning is `0x006BEBED`, `0x006BEC08`.
Cursor/header dword accesses are `0x006BEEB3–0x006BEEC3`, `0x006BEED9–0x006BEEE5`.

### ScanningIterator (0x20 bytes, no vtable)

Each iterator pointer above points directly to this layout. The native allocation requests 0x20 bytes
(`0x006BE5CF`); construction is `0x0060CE90–0x0060CED0`. Unlisted bytes +0x01..+0x03
are not a field available to the observer.

| Offset | Width (bytes) | Native field |
|---|---|---|
| +0x00 | 1 | state: 0 fresh, 1 yielding, 2 exhausted |
| +0x04 | 4 | x, int32 |
| +0x08 | 4 | z, int32 |
| +0x0C | 4 | step, int32 (constructor sets 1) |
| +0x10 | 4 | x0, int32, inclusive lower bound |
| +0x14 | 4 | z0, int32, inclusive lower bound |
| +0x18 | 4 | x1, int32, inclusive upper bound |
| +0x1C | 4 | z1, int32, inclusive upper bound |

Widths and signed bounds are established by construction, reset `0x0060C860–0x0060C888`
and next `0x0060C8D0–0x0060C92A`. x/z in state 1 identify the last yielded tract;
state 0 has not yielded that stored coordinate. Exhaustion can leave z beyond z1.

### Developer-map nodes (known fields only)

Node offsets are relative to the node pointer, not to L or the developer interface. Header is a
sentinel: its +0x08 is the first node (header itself for an empty map); never interpret its payload
as a developer. Child links use null for absence. No node allocation size, colour field or padding
contract is asserted here.

| Offset | Width (bytes) | Native field |
|---|---|---|
| +0x04 | 4 | parent node pointer; header +0x04 is root |
| +0x08 | 4 | left child pointer; header +0x08 is first node |
| +0x0C | 4 | right child pointer (ordinary nodes) |
| +0x10 | 4 | key, borrowed `cISC4GrowthDeveloper*`, used as its own interface base |
| +0x14 | 4 | weight, IEEE-754 float32 |

Header/first/cursor: `0x006BEEB3–0x006BEEC0`; payload reads and developer slot 13 call:
`0x006BEEE7–0x006BEEFD`. The loaded weight bits go to the callback context at
`0x006BEEF3`; `0x006BD671` consumes them as float32. Links are used by the cursor successor
at `0x005D17C4–0x005D17FA` (despite its exported constructor label). Map lookup uses header root
and node links/key at `0x006A3179–0x006A3196`. Tuning stores the borrowed developer and weight
through that map at `0x006BE2DC–0x006BE2F9`. Developer slot 13 returns a borrowed
`cISC4SimGrid<int16>*`; getter `0x007C9FB0–0x007C9FB6` is a plain pointer read with no AddRef.
The observer must copy map contents while the city/developer is alive; these facts grant no ownership.

### Message-target forwarding ABI

M's slot 3 (+0x0C) is `DoMessage(cIGZMessage2*)`, target `0x006BF0D0`.
Calling convention: x86 thiscall, ECX = **M**, one 4-byte message-pointer argument,
callee pops 4, bool result in AL (native always 1). The saved native slot must receive M,
not L or a proxy pointer. The separate L reference supplies the progress offsets above.
Type is queried through message slot 4; SimIdle data3 through slot 19 (+0x4C), returning a
32-bit budget in EAX (`0x006BF0D6–0x006BF0DC`, `0x006BF155–0x006BF160`). SimNewDay supplies
constant 5 (`0x006BF111–0x006BF116`). DoMessage return sites include `0x006BF11C–0x006BF11F`,
`0x006BF166–0x006BF169`, `0x006BF177–0x006BF17B`.

These addresses/layouts apply only to the named native build. An observer must validate executable
identity and the expected native interface/slot targets before reading private fields; mismatches
disable capture. The measured main-checkout **working copy** SHA256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
That fingerprint is an observation, not certification of a pristine executable or a shipped detector.
No approved hook installation, per-instance vtable mutation or agent re-registration ABI is implied.
