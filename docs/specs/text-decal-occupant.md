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

## 3. Inputs, comparisons and rebuild triggers

*[compat]* The TextDecal interface takes binary32 height, RGB and alpha, a byte dynamic argument,
and a 32-bit orientation. These setters have no defined success bool. Geometry and comparisons
use x87, with binary32 field/stack stores at the points below, not an all-binary64 algorithm.
Nonfinite rules below assume masked floating-point exceptions; trap behavior is outside this contract.

| Operation | Mutation and following action |
|---|---|
| SetText (`005F82E0`) | assign argument string into O+6C through its +34 operation; always UpdateTexture, including equal text |
| GetText (`005F8410`) | assign O+6C into caller's output string through output's +34 operation; no binding/refcount operation on the text source |
| SetTextHeight (`005F7B10`) | accept strictly `0 < h < 8`, store input bits at +80 and UpdateDecal, including equal accepted height; reject 0, 8, infinities and NaN |
| SetTextColor (`005F7B50`) | compare absolute new-minus-old in R, G, B order; if **any difference is >= epsilon or unordered**, store all three raw inputs; dynamic: SetColor on live A then B; static: UpdateDecal |
| SetAlpha (`005F7C00`) | compare numerically with old alpha; equal, including opposite signed zeros, does nothing; unequal or unordered stores raw input; dynamic: SetAlpha on live A then B; static: UpdateDecal; no clamp |
| SetOrientation (`005F7D60`) | compare all 32 bits; equal does nothing; otherwise store and UpdateDecal; no clamp |
| SetDynamic (`005F8170`) | compare/store the argument's low byte, without canonicalizing it; equal does nothing; changed value invokes UpdateOverlayManager; routing uses nonzero truth |

Epsilon is exactly the binary32 constant `38D1B717` at `00AA54A0`, namely
`13743895 / 137438953472` (about 0.0001). Equality **does trigger** a color update:
the parity branches at `005F7B6E–005F7B9D` distinguish ordered less from equal/greater/unordered.
The old strict-greater wording is superseded. NaN in any tested color difference causes mutation;
the remaining channels are then stored without further comparisons. Alpha uses `FUCOMPP`
at `005F7C0D` and its equality/unordered distinction at `005F7C11–005F7C14`.
Height's two comparisons are `005F7B14–005F7B30` against binary32 0 and 8.

SetPosition (`005F7AA0`, occupant receiver) copies input x/z bits into O+94/+98, then
calls the established embedded-prop box-translation operation with the entire input vector.
It calls terrain +17C with `(x,z,out1,out2)` even for equal positions; neither output is used
in the later label decision. Nonzero dynamic calls MoveDecal; static calls UpdateDecal.
MoveDecal (`005F7A50`) itself does nothing when static; otherwise it calls A.SetPosition
`(idA,&centerXZ)` then B.SetPosition `(idB,&centerXZ)` for nonsentinel IDs only.
It does not change cached size or notify the occupant manager itself. These are manager slot +30
calls, not full decal rebuilds. A static rebuild overwrites the translated box using terrain altitude;
the dynamic path retains the shared translation result. TextDecal is therefore not covered by the
plain prop's claim that no later terrain-dependent box writes occur.

## 4. Owned resource, routing and decal lifetime

*[compat]* Define A = O+B4, B = O+B8; source = O+B0. The source is the text texture
**source object**, passed to overlay slot +0C, not an already computed placement or bounding box.
The overlay primitive contract is [overlay-manager §3](overlay-manager.md#3-the-interface-0x00ab39e8-compat).
The owner retains all decisions below. A provider may supply raw source identity/dimensions,
terrain altitude, renderer level and opaque manager identities/IDs; it must not compute the owner's
rebuild, replacement, box, routing or callback order.

RemoveDecals (`005F77C0`) calls A.Remove(idA) if idA != `FFFFFFFF`, then stores that sentinel;
then does the same for B. It retains managers, source, size, box, text and position.
UpdateDecal begins with exactly the same A-then-B removal, before checking source or flags.
The individual stores follow the respective Remove calls, not both calls together.
IDs are opaque 32-bit values: only `FFFFFFFF` is rejected, not all values with the high bit set.
These paths assume a valid manager whenever its ID is live; no null-manager guard exists.

UpdateTexture (`005F8190–005F81E8`): null source field and Release old source if present.
The native body checks the field a second time and nulls/releases it if nonnull; ordinary
non-reentrant Release leaves it null, so there is one release. Then call the text resource factory
at service `00B43CE4`, slot +14, with `(0xCC1FD62C,&text,&sourceField)`.
Only the resulting pointer is tested; a separate factory return value is ignored.
Nonnull output invokes UpdateDecal. Null output returns without removing existing IDs or changing
the box/size. Thus failed replacement releases the old source and keeps old IDs until another
remove/rebuild; it is not rollback to the old source. Source reference transfer is the factory-output
contract; TextDecal does not explicitly AddRef the new source here. No OOM, exception or reentrancy
guarantee is inferred from this ordinary path.

UpdateOverlayManager (`005F7C80–005F7D51`) removes old A then old B IDs first. It obtains the
terrain view (`00602160`) and requests slot +48 with index **1 static / 3 dynamic**, producing new B.
If the pointer differs: AddRef nonnull new B, publish B, Release nonnull old B, in that order.
Equal pointer causes neither AddRef nor Release. It next obtains the terrain view again and requests
index **0 static / 2 dynamic**, applying the same replacement ordering to A. Finally UpdateDecal.
Equal managers do not suppress removal/rebuild. Null replacements are stored and old references
released; no fallback manager is selected. A null-manager state can safely reach the early exit of
UpdateDecal (absent source or flag 2 clear), but adding with either manager null is unsupported.
This publication does not invent safe behavior for that dereference.

## 5. UpdateDecal geometry and primitive arguments

*[compat]* Following removal, absent source returns immediately. Otherwise ask the occupant
view O+12 slot +80 whether flag mask **2** is set, testing AL; false returns. Both early exits
leave cached size and box unchanged. Neither dynamic nor initialized is a further gate.

For the remaining path let the source's slot +0C output two u32 dimensions `(w,h)` in that order.
The owner converts each to x87 numeric value with signed FILD plus `2^32` for negative signed
interpretations (`005F788A–005F78AE`), then computes the following in this order. Ordinary supported
dimensions are positive and nonzero; malformed source dimensions have no finite-geometry guarantee.
Use `r32` for a binary32 store, and `P` for the active x87 arithmetic precision/rounding.

1. Compute `q = (w / h) * textHeight * 16`, with an x87 rounding at each arithmetic operation.
   Store `size = r32(q)` at O+90 **without popping q** (`005F78AE–005F78D0`).
2. Store `hx = r32(q * 0.5)` and `hz = r32(textHeight * 8)` to stack binary32 slots
   (`005F78DC–005F78F4`). The first expression uses retained q, not reloaded size.
3. Query terrain service `00B43CF4` slot +34 with **(centerX,centerZ)**; store its x87 return
   to a binary32 altitude temporary (`005F78D6–005F7901`). This is distinct from SetPosition's +17C query.
4. If orientation is zero, exchange hx and hz. Every nonzero orientation keeps them.
5. Renderer service `00B43DD0` slot +28 returns a view object; load its u32 word +18 as level.
   Select binary32 **turns** from `00B0A978 + 16*level + 4*orientation`
   (`005F7919–005F7939`). This is a two-dimensional table, not `table[level]` or a size scale.
6. Write minimum `(r32(x-hx), r32(altitude-m), r32(z-hz))` at O+40/+44/+48;
   maximum `(r32(hx+x), r32(altitude+m), r32(hz+z))` at O+4C/+50/+54.
   Here `m` is binary32 `3DCCCCCD`, not mathematical 1/10. Instruction evidence:
   `005F793D–005F7985`. No renderer-table value enters these bounds.
7. Call A.AddDecal **(source,&centerXZ,size,turns)**, store returned ID at O+84 even on failure.
   If nonsentinel: A.SetColor `(idA,&RGB)` then A.SetAlpha `(idA,alpha)`.
   Then B.AddDecal with the same four arguments, store O+88; if nonsentinel set its color then alpha.
   Failure of A does not skip B; failure of B does not remove successful A.
8. Finally occupant-manager service `00B43D0C` slot +60 **(O+12,0)**, even if both Adds
   returned sentinel. Evidence: `005F7988–005F7A3C`. The box and size precede either Add.

| renderer level | orientation 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| 0 | 1/4 | 0 | 1/8 | 7/8 |
| 1 | 3/4 | 0 | 5/8 | 7/8 |
| 2 | 3/4 | 1/2 | 5/8 | 3/8 |
| 3 | 1/4 | 1/2 | 1/8 | 3/8 |

Those 16 source-image floats are exact binary fractions. Valid table access in this bounded contract
requires level and orientation in 0…3. The setter does not enforce that range; no clamping or fallback
is authorized for other values. Zero height and arbitrary float bits may enter through Read even
though the ordinary setter rejects them. x87 precision/control-word setup across all callers is not
proved here. Exact arbitrary-input results must preserve the explicit store points and be conditional
on that environment; host f64 evaluation alone is not a native oracle.

## 6. Initialization, flags, messages and shutdown

*[compat]* Init (`005F7F40`, occupant receiver) first tests IsInitialized (AL); already true
returns AL=1 without other work. Otherwise call embedded prop slot +20 with `0xCB79919B`,
then shared prop Init with O+12 (its return is not tested). If byte O+3A bit `02` is clear:
UpdateOverlayManager; get a temporary filter from service `00B43DC4` slot +48 (out starts null).
If nonnull, call filter slot +0C **(O+12)**; if its AL is true, ask city-config `00B43DB8`
slot +28; if that AL is true, call occupant slot +74 **(2)**. Release the filter.
Regardless of that bit/temporary filter, subscribe through message-server `00B43CCC` slot +14
with target **O+8**, in order `A6B79602`, `06CDB65B`, `26D31EC1`. Return AL=1.
No direct UpdateTexture call occurs in Init; setting flag 2 may cause one via SetAllFlags.

SetAllFlags (`005F81F0`, occupant receiver) snapshots byte O+3B and compares its bit 2 with
the requested flags. When changing 2 from off to on, query city-config +28; false clears bit 2
in the request and recomputes the change mask. Call shared base SetAllFlags with the effective
request before any decal work. If bit 2 did not change, return. If turning off, RemoveDecals.
If turning on: both managers nonnull -> UpdateDecal; otherwise UpdateOverlayManager. After this,
if source is null and text length (text slot +1C) is unsigned greater than zero, UpdateTexture.
Thus routing/rebuild may precede source acquisition, and even a same-source rebuild can happen
before the later source check. Native branches: `005F81F8–005F829A`.

DoMessage (`005F8030`, target O+8) reads type through message +10. Owner work precedes
the established embedded-prop DoMessage (called with target O+8); its return is passed through.

| Type | TextDecal work before delegation |
|---|---|
| `A6B79602` | UpdateDecal |
| `26D31EC1` | UpdateOverlayManager; obtain temporary filter as in Init; nonnull filter +0C(O+12) and true city-config +28 cause occupant +74(2); Release filter. A false/null result does **not** clear an already set flag here |
| `06CDB65B` | message +3C returns address of a filter pointer; dereference it and call filter +0C(O+12). If its AL and city-config AL are true, set flag 2 through +74 only if currently clear. Otherwise clear it through +7C only if currently set |
| other | no additional owner work |

The last filter is borrowed from the message, with no local Release; its pointer chain must be valid.
No whole routing or message-output callback substitutes for these owner decisions.

Shutdown (`005F76C0`, occupant receiver) returns AL=1 immediately if not initialized.
Otherwise unsubscribe the same three types in the same order, target O+8. For A then B:
if manager nonnull, Remove nonsentinel ID, set sentinel **after** Remove; null manager field
before Release(manager). A null manager skips the ID reset too. Next null source before its
Release, if nonnull. Finally shared embedded-prop shutdown `005EE160` with O+12; return AL=1.
The explicit lifecycle relies on that shared shutdown clearing initialized. Repeated Shutdown
after a valid completed shutdown therefore performs no owner releases or unsubscriptions.
It does not reset text, height, size, color, alpha, orientation or center.

Destruction is separate: body `005F8570` releases nonnull B, A, source in that order, without
nulling them or Remove/Unsubscribe/Shutdown calls; destroys the text allocation; then delegates
base teardown with O+12. Deleting wrapper `005F8510` also returns the object to its pool if
argument bit 0 is set. Call Shutdown before destroying a live initialized subscription owner.
Repeated destruction is unsupported; destruction after valid Shutdown has no manager/source
Release calls because those fields are already null. Factory `005F84D0` allocates from the
class pool then constructs at O; operator new `005F7D80` only allocates. The old `0x40` wording
is not a valid object-size claim: the constructor demonstrably writes through O+B8.

## 7. TextDecal-only stream semantics (OP2)

*[compat]* Stream slot +30 is Float32 and +3C is GZStr, not Uint32. This is not inferred from
NaN-looking IDs: compiled `cRZOStream::SetFloat32` `0092B276` writes exactly four argument
bytes, and its compiled vtable slot is 12/+30 (`00AD1818`); SetGZStr `0092B765` is
slot 15/+3C and calls string length (+1C) and bytes (+14). Compiled GetFloat32 `0092B2ED`
reads exactly four bytes and occupies +30 (`00AD8528`); TextDecal pair/triple read helpers
`005DA4F0/005DA550` pass addresses at 4-byte spacing. The established primitive encodings
and string contract remain [foundations §§2.2–2.3](foundations.md#22-primitive-encodings-compat).
These compiled-width facts plus the O+16 receiver resolve OP2 **for TextDecal only**.

| Order after outer version | v3 Write / corresponding Read | Complete-object destination |
|---|---|---|
| 1 | complete embedded prop record (shared record contract) | serializable receiver O+16 is also the embedded prop serializable view |
| 2 | GZStr | text O+6C |
| 3 | two Float32 | x,z at O+94/+98 |
| 4 | Float32 | height O+80 |
| 5 | Float32 | cached size O+90 |
| 6 | two Float32, same addresses again | x,z again; second pair wins on successful Read |
| 7 | three Float32 | RGB O+9C/+A0/+A4 |
| 8 | Float32 | alpha O+A8 |
| 9 | Uint8 | low byte of orientation on Write; zero-extended into whole O+AC on Read |

Write begins with Uint16 version 3. Read reads Uint16 version, then the prop record and text;
if version <3, consumes an extra Uint32 and discards it. In version exactly 1 the height is
Uint32 numerically converted unsigned-to-x87 then stored f32 (signed FILD plus 2^32 correction);
all other versions use Float32 height. No outer maximum-version rejection is present, and Read
does not invoke setters, UpdateTexture, rebuild or routing. Dynamic, source, managers and IDs
are not in this extension record and are not implicitly reset by Read.

If embedded prop Read/Write returns AL=false, skip the extension fields, then still return
whether the stream GetError **AL** is zero. Both functions normalize the final result to full
EAX=0/1 (`005F7E3C–005F7E48`, `005F7F28–005F7F35`). They do not directly return the
embedded prop bool. Earlier extension failures are not individually branched on; rely on the
stream's established error semantics. There is no owner-level rollback of already read fields.

## 8. Coverage boundary

The following is bounded static publication, not complete class byte credit. Literal proposed
vectors are in [text-decal-vectors.md](text-decal-vectors.md); all are **UNEXECUTED**.
No original execution, implementation oracle or live reference traces have been used.

### 8.1 Native-body, thunk, folded and foreign checklist

The old metadata attribution "35 / 3545B" is not a denominator or proof of coverage.
The inventory below is constructed from constructor-installed views, address searches and
instruction pages, and separates physical bodies from delegated contracts. Checked means
statically inspected for the bounded rules above, not executed or proved over invalid storage.

| Native entries checked | Contract |
|---|---|
| `005F7650` QI | §1 |
| `005F76C0` Shutdown; `005F77C0` RemoveDecals | §§4,6 |
| `005F7810` UpdateDecal; `005F7A50` MoveDecal; `005F7AA0` SetPosition | §§3–5 |
| `005F7B10` height; `005F7B50` RGB; `005F7C00` alpha | §3, including parity/NaN domains |
| `005F7C80` routing; `005F7D60` orientation; `005F8170` dynamic | §§3–4 |
| `005F7D80` allocation; `005F84D0` factory | §6, pool primitive only; no guessed size/OOM contract |
| `005F7DB0` Write; `005F7E50` Read | §7 |
| `005F7F40` Init; `005F8030` message; `005F81F0` flags | §6 |
| `005F8190` source replacement; `005F82E0` text | §§3–4 |
| `005F8300` constructor | §§1–2 |
| `005F8410` text output; `005F8430` CLSID | §§1,3; CLSID full EAX=8B79C707 |
| `005F8510` deleting destructor; unlabeled `005F8570` destructor body | §6; checked separately, neither silently invokes Shutdown |

That is 25 named/search entries (including factory/deleting wrapper) plus one separately
identified destructor body. Nine distinct adjusting thunks were checked: QI `005F8440`,
`005F8450`, `005F8460`, `005F8470`, `005F8480`, `005F84A0` subtract respectively
4,8,12,16,20,40; deleting-destructor thunk `005F8490` subtracts20; record thunks
`005F84B0/005F84C0` subtract4. The folded `005F8400` AsOccupant body is separately
checked, not a newly owned algorithm (nor evidence for another class's whole owner).
The resulting 36-entry local checklist is not a complete-class-byte coverage claim.

Foreign mappings below are **delegated established embedded-prop/base/holder contracts**, not
newly analyzed owner bodies. Slot numbers are zero-based, from the constructor-installed vtables.
This records which dispatch remains outside this owner's algorithms, including folded names
whose export attribution is another class; it does not promote names into layout proof.

| View | Foreign slots and targets |
|---|---|
| TextDecal `00AA5680` | 1 `005ECAF0`, 2 `005ECB00`: checked adjustments +12 into shared occupant AddRef/Release; 3 folded `005F8400` above |
| prop `00AA5620` | 1–2 `005ED5C0/005ED5D0`; 3–8 `005EFE60,00458C60,004E13E0,00648F10,005ED280,005EF4C0`; 9–14 `005EC9D0,005EC9E0,005ED290,005ED560,005ED400,005ECA10`; 15–23 `005EEFF0,005EE230,007C2740,005ED570,005ED580,005ED590,005ECA50,00477120,005ECAE0` |
| message `00AA5610` | 1–2 `005ED5F0/005ED600`; 0 is own QI thunk, 3 own DoMessage |
| occupant `00AA5588` | 1–2 `005ED620/005ED630`; 5–8 `00458A40,006C2040,006BDD80,005ED1F0`; 10–19 `005DED60,005E8E20,005E8A60,005E8AA0,005E8AE0,005E9010,00458A50,005E90E0,005E8B10,006AAC50`; 20–29 `005E91C0,0078DAF0,005E92D0,005E9350,005E9410,005E8BF0,005E8C20,005ECAC0,004302C0,005E8B70`; 31–33 `005E8BA0,005E8BC0,005E8BE0`; 0/3/4/9/30 are own entries already checked |
| serializable `00AA556C` | 1–2 `005ED650/005ED660`; 0/3/4/5 own entries |
| holder `00AA5508` (25 slots) | 1–2 `005ED680/005ED690`; 3–10 `005FEC20,005FECA0,005FD290,005FD2D0,005FD260,005FD230,005FEDA0,005FFA40`; 11–21 `005FF950,005FF860,005FF770,005FDEF0,005FF6D0,005FDFD0,005FEBC0,005FF3C0,005FEF00,005FEE10,005FF160`; 0/22/23/24 own thunks |
| exemplar holder `00AA54F0` | 1–4 `005ED6C0,005ED6D0,00458E60,0060CE80`; 0 own QI thunk; following null word not a method |

Primitive source/font rendering, terrain altitude queries, renderer view acquisition, manager
Add/Remove/Set and AddRef/Release remain foreign primitives. The owner algorithms in §§3–7
must compute their arguments and order. Complete physical layout compatibility also requires
the established embedded prop; unrelated storage masquerading as its views is unsupported,
not a reason to fall back to plain prop behavior. Global floating-point environment proof is
still open as **TD1**; owner status is **partial for arbitrary-input bit exactness**, with a
coherent normal-domain contract and exact-environment-conditioned vectors.
