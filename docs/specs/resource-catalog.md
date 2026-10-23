# Resource catalog and key picker: native behavior contract

Status: first behavior publication, re-resource-catalog4, 2026-10-23. Partial contracts, not whole-owner coverage.
Source: original Windows 1.1.641, image base `00400000`, executable/export SHA256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b` (independently matched).
Addresses below cite native export instructions, checked with bytes. Local decompilation fails on many of these
bodies; no successful live pseudocode query is claimed or required. No implementation or portable test output was used.

## 1. Scope and identity

This is the catalog-owner supplement to [resource-exemplar-editor.md](resource-exemplar-editor.md).
It does not reopen the property/value/curve algorithms or specify widget internals, DBPF enumeration internals,
filesystem dialogs or the modal event loop. A foreign primitive may supply raw enumeration entries, widget row
permutations or modal results; it must not supply the catalog's filtering, selection or output algorithm wholesale.

| Owner | Primary identity and adjustment |
|---|---|
| cSCResourceCatalogView | Director registration `00664968–00664974` pairs class `1A368ED8` with factory `00664280`. Factory allocates `0x114` bytes, constructs at allocation start, returns null on allocation failure. Constructor `00686450` constructs cGZWin at catalog+4 and installs catalog table `00AAFE7C`, window table `00AAFC20`. |
| cSCResCatalogPickerView | Registration `006649CE–006649DA` pairs class `671E9725` with factory `00664750`. Allocation is `0x12C` (300) bytes; null allocation returns null. Catalog subobject is picker+4, window subobject picker+8, variant picker+`0x118`. Tables are `00AAD0D8`, `00AAD0B4`, `00AACE58` respectively. Constructor `006645F0` agrees with factory body. |
| cSCResCatBaseView | Separate window owner: `00675AF0` installs its own window table and destroys cGZWin. Do not infer from its name that the catalog constructor calls its Init or inherits its behavior: `00686450` directly constructs cGZWin. Its UI construction remains partial here. |
| cSCResKeyInputView | Constructor `00682330` directly constructs cGZWin, installs table `00AAF510`, copies three input dwords at `0068234B–0068235F` to offsets `EC/F0/F4`. No independent registered class ID is asserted. |

Catalog QI (`006855E0`) recognizes `7A368EE3`, writes the catalog pointer, then AddRefs, and returns true in AL.
Other IDs delegate to cGZWin QI at catalog+4; its failure/output policy is foreign. Picker QI (`006759B0`)
recognizes `06C5FF7B`, AddRefs **before** writing the picker pointer, returns true in AL; other IDs delegate to
catalog QI at picker+4. Output pointers must be valid; neither success branch null-checks them.
Catalog AddRef/Release (`00762470/00762480`) adjust +4 and delegate; picker AddRef/Release
(`00664660/00664670`) adjust +4 again. These are forwarding bodies, not independent counters.
The exported name at the shared Release destination is not evidence that an editor-dialog constructor runs.

Throughout §§2–4, a key is three unsigned 32-bit words `(T,G,I)`, copied in that order. The picker variant
uses a distinct raw order `(G,T,I)` (§4). Do not conflate these representations.

## 2. Row rebuild, filter and order

`updateKeyList` (`00685AE0`, catalog receiver) uses the selected descriptor index at +`F4` and a contiguous
descriptor range +`100/+104`. Each descriptor is 32 bytes; its first two words supply the resource type and group.
An empty descriptor range, an index outside the unsigned descriptor count (including `FFFFFFFF`), or a null
list pointer (+`110`) returns before clearing rows. Those cases retain the previous widget contents.

After those checks it clears the list through slot `188` (`00685B3D`), then asks the global resource service
for an enumeration of the descriptor's type through slot `48` (`00685B4D`). A zero return ends the rebuild
after the clear. The count and indexed raw-key access are foreign slots `24/28`; the count is queried again
after each iteration. This contract assumes valid returned key pointers and stable entries for that traversal.

For every raw entry, compare its **second dword** to the descriptor group (`00685B82`). Only exact equality
passes. There is no group wildcard in this loop, no instance filter, and no deduplication. Entries with other
groups are skipped. The type is a service request parameter, not rechecked against each returned key's first word.
Passing entries receive consecutive row numbers starting at zero; a name helper `006695A0` supplies display text.
If that helper returns false in AL, text is the instance word formatted as `0x%08X` (`00685BA7–00685BB5`), e.g.
`0x0000002A`. It writes text to column 0 and stores the **raw key pointer**, not a copied key, as row data
(`00685BDE/00685BF0`). It does not check those widget calls' return values. Temporary display strings are destroyed.

After enumeration, even with zero passing rows, it calls list slot `20C` with arguments `(0,0,1,0)`
(`00685C32–00685C3A`). This is an explicit foreign ordering operation. Its comparator, stability, locale and
tie behavior are not established here: do not replace it by a claimed numeric T/G/I sort or promise raw
enumeration order on screen. The following owned selection scan uses the rows **after** this operation.

If no rows passed, it does not select or call `setViewKey`; the earlier preview is not explicitly cleared by this
body. If rows passed, it sends list slot `10C` the arguments `(0,rowCount,15)` and scans row data from row zero
upwards. Row zero is always selected and copied to a local key. Each later full three-word equality with the
requested key is also selected and copied. After the scan it calls `setViewKey` once with the last copied key
(`00685C80–00685CDD`). Thus absent requested keys fall back to row zero; duplicate full keys finish at the last
matching displayed row. The scan does not stop at the first match. Nonnull row data is a precondition here.

The body has no enumeration Release call and no per-key AddRef/Release. It retains service-provided pointers
as widget data. This proves call effects, not a safe assumption that the enumeration is owned or leaked: its
ownership and pointer validity beyond the traversal remain unresolved.

`SetFilterForType/Group` (`00664640/00664650`) only store one 32-bit word at catalog+`E0/E4`.
`SetReadOnly` (`00664630`) only stores the argument byte at +`DC`. None rebuilds rows immediately. The constructor
sets these three fields to zero and byte +`DD` to one (`00686473–00686486`). Descriptor registration and the
later uses of these fields need separate attribution; these setters alone do not prove wildcard semantics.

## 3. Selection read and refresh

`GetSelectedKey` (`006857E0`) asks the list for selected coordinates (slot `A8`), then row data (slot `1E0`).
It does not test the coordinate-call result. Null row data returns false in AL and leaves all output words
unchanged (`00685816–0068583C`). Nonnull row data copies T, then G, then I to the output and returns true.
It does not validate the list pointer, key pointer length, or output pointer; uninitialized/malformed objects
have no defined failure contract here.

`RefreshKeyList` (`00685CF0`) zeros a temporary triple, calls virtual GetSelectedKey, and calls updateKeyList
only when AL is nonzero, using the copied key. Selection failure therefore does **not** clear/re-enumerate rows.
Refresh is not an unconditional rescan, and an empty selection does not gain a first selection through this path.

`setViewKey` (`00685840`) chooses the preview interface from descriptor+`1C`. If it differs from the current
preview (+`10C`), it invokes old preview slot `14`, hides its window (slot `118`), stores the new interface,
and shows its window (slot `114`). A nonnull current preview receives the key via slot `10`, then the window
manager receives a focus request for the list window. Matching preview interfaces skip the switch/hide/show
but still receive the key and focus request. There is no AddRef/Release in this switching body; preview internals
and failure returns are foreign. Descriptor indexing here is not independently bounds-checked.

## 4. Picker input, output and close results

Direct PE table data at `00AAD0D8` maps picker slots `18/1C` to unnamed native bodies `00675A00/00675A60`.
These are catalog-owner adapter bodies despite their missing exported method names.

The input adapter (`00675A00`) assigns the supplied variant to picker+`118`, calls catalog SetReadOnly(false),
calls catalog slot `0C` with zero (the shared body is not semantically named here), then reads the stored
variant's raw data via slot `19C`. It sets type from raw word 1 and group from raw word 0. It does not rebuild
the list in this body. Variant type conversion/validation belongs to the foreign variant primitive.

The output adapter (`00675A60`) starts with a zero `(T,G,I)` triple, calls catalog GetSelectedKey, **ignores**
its boolean result, writes `(G,T,I)` into the stored variant data, then assigns that variant to its output.
It does so regardless of any modal result. Selection failure therefore outputs three zero words, provided
the variant has valid writable data. Null output/invalid variant storage are not guarded here.

`doButtonPress` (`006861F0`, catalog receiver) maps IDs using native jump-table data at `0068624C`:

| Button ID | Owned effects | AL |
|---|---|---|
| `24C93018` | If current preview exists, invoke preview slot `14`; then KillSelf(1). | false |
| `24C93019` | Invoke addResource. Its algorithm is not covered by this close contract. | true |
| `24C9301A` | KillSelf(0), without preview slot `14`. | false |
| `24C9301B` | No action. | true |
| Other | No action. | false |

`KillSelf` (`00685780`) checks window flag `1000`. When set, it calls window-manager slot `24` with
`(15, catalogWindow, suppliedResult)`. Otherwise it asks the window for its parent and delegates removal through
slot `248`; it does not send the result argument on that branch. The modal machinery is a foreign primitive.
The key-table Add Key caller (`006843CE–00684426`) treats a zero modal return as acceptance and a nonzero return
as cancel; it still invokes the picker output adapter before checking that result. It converts the raw variant
back from G/T/I to T/G/I only on the accepted branch. Output availability and caller acceptance are separate.
The existing consumer's 575×475 and 300-byte facts remain consumer facts; they do not specify catalog geometry.

## 5. Literal independent vectors

These are instruction-derived expected outcomes, not executed game results. Enumeration inputs and post-order
widget rows are independent primitive inputs; owner decisions must be reproduced locally.

| Case | Inputs | Expected owned result |
|---|---|---|
| Group filter and duplicate | Descriptor `(T=00000010,G=00000020)`; service entries `(10,20,2A)`, `(10,21,01)`, `(10,20,2A)` (hex). | Two append operations, both data pointers retained, no dedup; fallback text for each is `0x0000002A` if name lookup fails. |
| Exact zero group | Descriptor group `00000000`; entries have groups 0 and 1. | Only group 0 passes; zero is not a wildcard in the row loop. |
| Missing requested key | Post-order rows `(10,20,09)`, `(10,20,03)`; request `(10,20,FF)`. | Select row 0; preview `(10,20,09)`, regardless of numeric instance ordering. |
| Duplicate selection | Post-order rows `(10,20,2A)`, `(10,20,03)`, `(10,20,2A)`; request `(10,20,2A)`. | Select row 0 then row 2; final row 2, one final preview call with `(10,20,2A)`. |
| Reorder boundary | Same entries; primitive returns reversed rows `(10,20,03)`, `(10,20,09)`; missing request. | Select new row 0 `(10,20,03)`. Owner scan respects primitive order. |
| Invalid descriptor | Index `FFFFFFFF`, prior rows present. | Return before clear/service request; rows retained. |
| Service failure | Valid descriptor/list; enumeration service returns zero. | Clear first; no append, ordering call, selection or preview call. |
| Empty/group-miss | Valid enumeration with zero passing rows. | Clear, invoke ordering primitive, no selection or setViewKey. |
| Null selected data | Output initially `(AAAAAAAA,BBBBBBBB,CCCCCCCC)`; row data null. | GetSelectedKey false; output unchanged; RefreshKeyList makes no rebuild call. |
| Picker conversion | GetSelectedKey succeeds with `(11223344,55667788,99AABBCC)`. | Variant raw output `(55667788,11223344,99AABBCC)`. |
| Picker missing selection | GetSelectedKey fails, valid variant storage. | Output variant raw words `(0,0,0)`; no error branch. |
| Accept vs cancel | Modal return 0 or 1, same selection `(10,20,2A)`. | Output adapter runs in both cases; key-table consumer uses converted key only for return 0. |
| QI lifetime | Valid catalog/output, IID `7A368EE3`. | Write catalog pointer, one AddRef, AL=true; picker IID `06C5FF7B` AddRefs before writing. |

## 6. Attribution and remaining boundaries

No byte total or whole-owner coverage follows from this publication. The historical family figure 16686 is an
identity attribution, not 16686 bytes of proven algorithms. The checklist below is disjoint by native body;
shared/folded bodies must not be counted as independent catalog implementations. Unknown portions of partial
bodies remain unknown even when one branch is specified above.

| Category | Native bodies | Extent |
|---|---|---|
| Supported | `00664630/40/50`, `006855E0`, `006857E0`, `00685CF0`, `006759B0`, `00675A00`, `00675A60`, `00664680` | Setters; QI; selection/refresh; picker adapters. `00664680` writes `FFFFFFFF` to its output word and returns; no modal meaning inferred. |
| Partial | `00664280`, `00664750`, `006645F0`, `00686450`, `00682330`, `00685AE0`, `00685840`, `006861F0`, `00685780` | Allocation/identity, row algorithm with foreign enumeration/order/name/lifetime boundaries, preview dispatch, close dispatch. Constructor auxiliary allocator failure is not specified. |
| Folded/forwarding | `00664660/70/90`, `00762470/80`, `006759F0`, `006864F0` | Adjusting thunks and inherited initialization; no independent owner outcome. Undecoded neighboring thunks are unknown below. |
| Foreign | cGZWin lifetime/QI/widget state; resource service slots `48`, enumeration `24/28`; name helper `006695A0`; list ordering `20C`; variant storage/assignment; modal loop | Only call boundary effects described; not owner callbacks returning a whole catalog outcome. |
| Unknown | Catalog `00685610/60/B0/C0`, `006858E0`, `006859B0`, `00685D70`, `00686310`, `00686410`, `00686510/B0`, `006866C0/E0`, `00686B90`, `00686D30`, `00687890`, `0079B0E0`, `00664720`; picker `00664700`, undecoded `006646A0/B0/C0/D0/E0`; base `00675AF0/B00/C80/C90/DC0/DF0`; key-input `00682060`, `00682140/F0`, `00682230/90`, `006823D0`, `006824A0`, `00682590`, `00682632/E0`, `00682730`, `00683130` | Identity/read discovery only at this milestone, not behavior coverage. Registration, remaining initialization/failure/lifetime, widget construction/geometry, add-resource and key-text paths need their own evidence. |
