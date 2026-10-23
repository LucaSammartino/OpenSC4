# City-detail text decal occupant

## 1. Scope, evidence and receiver frames

*[compat]* This is `cSC4TextDecalOccupant`, class `0x8B79C707`, the city-detail terrain label.
It is separate from the region signpost. This contract supersedes the mixed-frame interpretation
in [lot-records §10.4](lot-records.md#104-csc4textdecaloccupant-0x8b79c707-compat).
Evidence is static Windows 1.1.641 instruction analysis, not a live-save measurement.
The canonical export identifies SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, image base `0x00400000`.
Address citations below identify instruction evidence; decompiler structure names are not field evidence.

Let **O** be the complete-object address. Offsets in this spec are from O unless explicitly qualified.

| View / entry | Receiver | Evidence |
|---|---|---|
| TextDecal, constructor, setters except SetPosition, internal UpdateDecal/UpdateOverlayManager | O | constructor `005F8302–005F833D`; class table `00AA5680` |
| embedded prop | O+4 | constructor calls prop constructor on O+4 at `005F8305–005F8310` |
| message target | O+8 | QI thunk `005F8450` subtracts 8 |
| occupant, Init/Shutdown/SetPosition/SetAllFlags | O+12 | QI thunk `005F8460`; SetPosition `005F7AEF` subtracts 12 before UpdateDecal |
| serializable, Write/Read | O+16 | QI thunk `005F8470`; vtable `00AA556C` points directly at Write/Read |
| property holder | O+20 | QI thunk `005F8480`; record thunks `005F84B0/005F84C0` subtract 4, yielding O+16 |
| exemplar-property-holder view | O+40 | constructor `005F833D` (shared embedded-prop contract) |

TextDecal QI `005F7650` answers IID `0x2B3B7D86` with O and AddRefs through O+4;
other IIDs delegate to embedded-prop QI with O+4. Its success is **AL**, not an asserted full EAX bool.
`AsOccupant` `005F8400` returns O+12 for nonnull O, otherwise null. Shared prop ownership,
lot tags and property operations retain [prop-occupant](prop-occupant.md)'s foreign-base contract.

## 2. State and OP1 reconciliation

*[compat]* Constructor `005F8300` establishes these fields, after constructing the embedded prop.

| O offset | Storage / default | Role |
|---|---|---|
| +6C…+7F | string object, initially empty | text, not lot tag or decal IDs; +70/+74/+78 are string allocation bookkeeping; +7C = 0 |
| +80 | f32 `3F800000` (1) | text height |
| +84, +88 | two 32-bit `FFFFFFFF` | IDs A, B; sentinel means absent |
| +8C | byte 0 | dynamic; only this byte is initialized here |
| +90 | f32 `42C80000` (100) | cached decal size |
| +94, +98 | f32 +0, +0 | label center x, z |
| +9C, +A0, +A4 | f32 `437F0000` each (255) | RGB |
| +A8 | f32 1 | alpha |
| +AC | u32 1 | orientation |
| +B0 | null pointer | owned text texture binding |
| +B4, +B8 | null pointers | owned overlay-manager references A, B |
| +4C, +50, +54 | f32 `3DCCCCCD` each | embedded prop box maxima, constructor overwrite only |

The constructor writes the string allocation's end pointer to O+78 (`005F835C–005F835F`),
not an overlay ID. SetPosition's receiver-relative +88/+8C are O+94/+98 (`005F7AAA–005F7AB6`).
Shutdown's receiver-relative +78/+7C are O+84/+88, and its +A8/+AC are O+B4/+B8
(`005F7728–005F778E`). SetOrientation uses O+AC, not that Shutdown-relative +AC.
UpdateDecal and UpdateOverlayManager use O directly (`005F7814`, `005F7C82`), so their
+84/+88 are indeed IDs and +8C is indeed dynamic. **OP1 is statically resolved: DIFFERS**
from the proposed field-role conflict; no live save is needed to identify these aliases.

The same conversion applies to records: Write/Read receiver +5C is O+6C, the string;
+70 is O+80 height; +80 is O+90 size; +84/+88 are O+94/+98 position; +8C…+94
are O+9C…+A4 RGB; +98 is O+A8 alpha; +9C is O+AC orientation's low byte.
The repeated record pair is **position**, not IDs or height. Evidence: `005F7DD0–005F7E34`,
`005F7E79–005F7F1D`, pair/triple helpers `005DA520/005DA580` use stream slot +30 on
successive 4-byte fields. Read writes the first position pair, height, size, then overwrites
position with the second pair. Full stream details and vectors follow below.

## 3. Publication boundary

This is a bounded static contract under construction, not complete class coverage or byte credit.
No original execution, implementation oracle or live reference traces have been used.
