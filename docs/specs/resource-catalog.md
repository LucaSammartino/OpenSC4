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
Base-view table `00AAE630` and key-input table `00AAF510` both start with shared forwarding bodies
`00682050/007BE160/007BE170`: QI/AddRef/Release delegate without adjustment. They add no owner-specific QI ID.
For these delegations, native cGZWin QI `0099B774` recognizes `00000001`, `22BA0121`, `E98B2F57`, writes
the window pointer and AddRefs; failure returns false without writing the output. AddRef `0094C47C` increments
the window's 32-bit count at +`0C`. Release `0099B7A3` decrements only when the unsigned count exceeds one;
otherwise it invokes virtual deleting slot `250` with flag 1 and returns zero. These shared facts explain
the reference effects at these owners' boundaries; they do not grant whole-widget or editor-dialog coverage.

For offsets, C denotes the catalog interface, W=C+4 its window interface, and P=C−4 the enclosing picker when
present. Window methods such as Init, Shutdown, DoMessage and GZOnCommand receive W, not C. This distinction is
essential when auditing shared bodies. In particular, the exported `00664720` “catalog destructor” is actually
the picker destructor (installs picker tables, destroys P+118 variant, then destroys C); `00686510`, exported
as a vector destructor, is the catalog destructor (installs catalog/window tables and finally destroys cGZWin).
These attributions follow actual instructions, not public symbol ownership.

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
Catalog slot `0C` is a shared body at `0098AD30`: it stores its argument byte at C+`DD`. The exported
page-flipping name describes another folded use, not a catalog operation.

### Descriptor filtering during Init

The descriptor loop in Init (`00687289–006873F4`, receiver W) reads the stored type filter from W+`DC`=C+`E0`
and group filter from W+`E0`=C+`E4`. Zero disables that dimension of this **descriptor** filter. It admits a
descriptor exactly when each nonzero filter equals the corresponding descriptor word. Thus `(0,0)` admits all,
`(T,0)` admits all groups of that type, `(0,G)` admits all types of that group, `(T,G)` requires both.
This is distinct from the exact-group row loop, where descriptor group zero is literal.

Descriptors are visited in vector order, never sorted by this loop. For each admitted descriptor, it passes the
read-only byte C+`DC` to preview slot `18`, attaches/hides the preview window, appends descriptor name (+8) to
the selector with argument `-2`, and records a map from consecutive **visible selector index** to original
descriptor index (`0068739E–006873CD`). It does not discard excluded descriptors from the backing vector.
Init selects selector row zero and retrieves map entry zero for the selected descriptor (`0068741E–0068743B`).
The empty-map outcome depends on the foreign map insertion primitive and remaining initialization; no claim of
a valid descriptor on an empty filtered set follows. Ini-to-descriptor registration remains unknown (§8).

GZOnCommand (`00686310`, receiver W) routes command `287259F6` to doButtonPress with C=W−4 and returns its AL.
For command `E8711255` with control ID `04C9303E`, it finds the child and queries interface `412CE496`.
Successful QI reads selector slot `78`, resolves that visible index through the map, stores the descriptor
index at C+`F4`, and Releases the queried interface (`00686370–0068639B`). Missing child returns true immediately.
After the QI attempt it rebuilds with the all-zero requested key even if QI failed, preserving the previous
descriptor index in that failure case. It then requests list focus if the list exists and returns false.
Other command/control pairs return false. Unexpected selector indices and their map insertion effects remain
unsupported; the handler performs no explicit range validation before lookup.

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
sets byte C+`DD` to zero through catalog slot `0C`, then reads the stored
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

## 5. Key-input notifications and filename

The key-input constructor copies `(T,G,I)` into +`EC/F0/F4`, initializes its reentrancy byte (+`E8`) to zero,
and initializes the three held UI pointers +`10C/110/114` to null (`00682330`). The algorithms below require
initialized controls; null controls and invalid message pointers are not converted into clean false results.

`DoMessage` (`00683130`) routes message `682876C9` to doSpinnerChange using message words +4/+8. Message
`4348B111` is forwarded to the message service slot `10` with `(message,0)` and returns true; unmatched messages
(including `0D`) delegate to cGZWin. For `9B1552D8`, it gets the source control ID through message+4's window.
ID `283B52CE` calls setFileName directly. ID `E83B22EE` reads text from held control +`10C`, calls
`RZConvertToUint32A` (`0090FE93`), and stores the returned 32-bit word as I **without** checking conversion
success, collision, or the spinner's nominal range (`006831AD–006831CD`). It finds control `04C93038`, sets
reentrancy byte to one, QIs `612CE0C3`, and on success writes I via slot `24`. Then it clears the byte and calls
setFileName. There is **no Release after this successful QI in this text-notification branch**, unlike the spinner
branch below. This is a call-effect difference; foreign QI ownership policy is not reconstructed here. Unrecognized
source IDs for this message return true without changing I. Malformed text has whatever uint32 the foreign
conversion returns; rejection, prefixes, overflow and whitespace rules are not specified by this owner.

`getNextID` (`00682290`) constructs candidate `(held T,held G,requested I)`. Step direction is −1 when the raw
requested word is **unsigned less than** current I, otherwise +1 (`CMP/SBB`, `006822AC–006822B7`). Initial
normalization uses **signed** comparisons: requested values below 1 become `00007FFF`; above `00007FFF` become
1. It queries resource-service slot `38` for the candidate. A false AL returns that candidate. On true AL it
adds the step, then maps signed values <=0 to `00007FFF`, and signed values >`00007FFF` to **zero**, before
querying again (`006822F0–00682319`). The latter differs from initial normalization: an increasing search can
return instance zero. There is no bounded probe count, failure result or occupied-set cycle detection.

`doSpinnerChange` at `00682590` recognizes only control ID `04C93038`. A set reentrancy byte returns true
immediately. Otherwise it calls getNextID and stores the returned I. If this differs from the supplied spinner
value, it finds the spinner, sets the reentrancy byte, QIs `612CE0C3`, writes through slot `24` and Releases the
interface on QI success, then clears the byte (`006825CC–00682613`). Subsequent code formats I as signed decimal
`%d`, updates the instance text, requests its selection/focus, calls setFileName and returns false. The export
incorrectly cuts this body at `0068262E` and starts a spurious function at `00682632` in the middle of an
instruction. Only the decoded continuation from `0068263D` onward supports these later effects; allocator setup
across that split is not assigned an invented failure rule.

`setFileName` (`006823D0`) checks length of held name control +`110`. For a nonempty name it copies its text
and replaces **each byte other than ASCII A–Z/a–z with underscore**, including digits, punctuation and whitespace
(`00682410–0068242A`). It formats the result as `%s_%08x`, with the current I in lower-case, zero-padded hexadecimal.
For an empty name it uses `%08x` alone (literal bytes at `00A8DB8C`). The generated string is stored at +`F8`
and sent to the display obtained from held pointer +`114` (`0068247A–0068248A`). This is display-name generation,
not a file write; there is no path, extension or filesystem validation in this body.

Key-input `GZOnCommand` (`00682230`) accepts command `287259F6` with IDs `24C93018/24C93019` and sends
window-manager slot `24` `(15,this,0)` / `(15,this,1)` respectively. Other pairs return false. The recognized
branches return the foreign call's AL rather than forcing true. These IDs differ from the catalog's close-ID
mapping (§4); do not copy that mapping into the key-input owner.

## 6. Shutdown and reference effects

Catalog Shutdown (`00687890`, receiver W) first hides the window and calls slot `158` with zero. It destroys
all descriptors in C+`100..104` using helper `00685D30`, sets end to begin, clears current preview C+`10C`,
then Releases and clears the list C+`110` if nonnull, and delegates cGZWin Shutdown. Helper `00685D30` visits
each 32-byte descriptor, Releases its nonnull preview interface (+`1C`) once, then destroys its name storage.
The clearing operation leaves the vector allocation for later destruction. It does not Release current preview
as a separate reference in addition to the descriptor references. Widget shutdown and map internals remain foreign.

Catalog destructor `00686510` installs catalog/window tables, destroys the remaining descriptor range through
the same helper, destroys vector/map storage, then destroys cGZWin at C+4. It does **not** explicitly call catalog
Shutdown. Deleting wrapper `006866C0` calls that destructor, then GlobalHeapFree only when flag bit 0 is set.
Picker destructor `00664720` reinstalls picker tables, destroys variant P+`118`, then calls that catalog
destructor at P+4. Picker deleting wrapper `00664700` conditionally frees P on flag bit 0. This correction of
exported owner names is body-level attribution, not an ABI change.

Key-input Shutdown (`00682060`) hides the window and calls slot `158` with zero; for held +`10C`, then +`110`,
each nonnull pointer receives slot `10`, then Release, then is cleared. Nonnull +`114` receives window slot `14`
through its slot `0C`, then Release and is cleared. Finally it delegates cGZWin Shutdown. Key-input deleting
destructor (`006826E0`) installs its table, explicitly calls this Shutdown, destroys filename storage, destroys
cGZWin, and frees the object only for flag bit 0. A second Shutdown has no additional held-pointer Releases,
but still calls the foreign window operations; foreign idempotence is not implied.

Base-view Init (`00675B00`) checks cGZWin's initialized flag first: set means return true without building its UI.
Otherwise it configures colors/flags and attempts GZCOM creation with class/interface `C2AFA76E/C2AFA76F`.
Failure skips child configuration but **still returns true** (`00675BE1–00675C6F`). Success configures/adds the
child and Releases the creation interface. Geometry and widget construction are not covered. Base Shutdown
(`00675C80`) is solely cGZWin Shutdown; destructor `00675AF0` installs `00AAE630` then destroys cGZWin, and
deleting wrapper `00675DC0` additionally frees only for flag bit 0.

## 7. Literal independent vectors

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
| Descriptor wildcard vs row filter | Descriptors `(10,20)`, `(10,21)`, `(11,20)`; type/group filters `(10,0)`. | Admit descriptor indices 0 and 1, map visible indices 0→0, 1→1; no descriptor sorting. Later each row loop still requires its descriptor's exact group. |
| Group-only descriptor filter | Same descriptors; filters `(0,20)`. | Admit descriptor indices 0 and 2; visible map 0→0, 1→2. |
| Filename sanitation | Name `A9-b c`, I=`00001234`. | Display `A__b_c_00001234`; digits in name become underscores. |
| Empty name | Name empty, I=`ABCDEF01`. | Display `abcdef01`, no underscore prefix or `0x`. |
| Initial high/signed normalization | Requested `00008000` / `FFFFFFFF`; occupancy always false. | Return 1 / `00007FFF` respectively. |
| Search wrap to zero | Current I=`00007FFE`, request `00007FFF`; service says `7FFF` occupied, zero free. | Direction +1; probe `7FFF`, then zero; return zero. |
| Decreasing search | Current I=2, request 1; service says 1 occupied, `7FFF` free. | Direction −1; probe 1 then `7FFF`; return `7FFF`. |
| Text conversion boundary | Parser primitive returns `FFFFFFFF` for supplied text. | Store that exact word; spinner write receives it, filename suffix `ffffffff`; no owner range rejection. This does not assert which malformed text produces that parser result. |
| Spinner reentrancy | ID `04C93038`, byte E8=1. | Return true; no getNextID, text write or filename refresh. |
| Shutdown lifetime | Two descriptors share one preview pointer; each owns a reference; list nonnull. | Two descriptor Releases, then one list Release; clear preview without an extra Release. A later catalog destructor sees an empty descriptor range. |
| Base creation failure | Init flag false; GZCOM creation returns false. | Return true despite absent child. |

## 8. Attribution and remaining boundaries

No byte total or whole-owner coverage follows from this publication. The historical family figure 16686 is an
identity attribution, not 16686 bytes of proven algorithms. The checklist below is disjoint by native body;
shared/folded bodies must not be counted as independent catalog implementations. Unknown portions of partial
bodies remain unknown even when one branch is specified above.

| Category | Native bodies | Extent |
|---|---|---|
| Supported | `00664630/40/50`, `006855E0`, `006857E0`, `00685CF0`, `006759B0`, `00675A00`, `00675A60`, `00664680`, `00682290`, `006823D0`, `00682230`, `00683130`, `00685D30`, `00664700`, `006866C0`, `00675AF0/DC0` | Owned rules under the explicit primitive/precondition boundaries above; not foreign internals. `00664680` writes `FFFFFFFF` to its output word and returns; no modal meaning inferred. |
| Partial | `00664280`, `00664750`, `006645F0`, `00686450`, `00682330`, `00685AE0`, `00685840`, `006861F0`, `00685780`, `00686310`, `00686D30`, `00687890`, `00686510`, `00664720`, `00682060`, `006826E0`, `00675B00`, `00682590` with continuation `0068263D–006826DC` | Allocation/identity, row/preview/close algorithms, descriptor filtering/mapping, shutdown/reference effects, key input; remaining widget/allocator/registration and export-split boundaries are not covered. |
| Folded/forwarding | `00664660/70/90`, `00762470/80`, `006759F0`, `006864F0`, `00686500`, `00675C80`, `0098AD30`, `00682050`, `007BE160/170`, `0099B774`, `0094C47C`, `0099B7A3` | Adjusting thunks, inherited initialization/shutdown, shared QI/reference count and byte setter; `00686500` adjusts W−4 to the catalog deleting wrapper. No independent foreign implementation credit. Undecoded neighboring thunks are unknown below. |
| Foreign | cGZWin lifetime/QI/widget state; resource service slots `48`, enumeration `24/28`; name helper `006695A0`; list ordering `20C`; variant storage/assignment; modal loop | Only call boundary effects described; not owner callbacks returning a whole catalog outcome. |
| Unknown | Catalog `00685610/60/B0/C0`, `006858E0`, `006859B0`, `00685D70`, `00686410`, `006865B0`, `006866E0`, `00686B90`, `0079B0E0`; picker undecoded `006646A0/B0/C0/D0/E0`; base `00675C90/DF0`; key-input `00682140/F0`, `006824A0`, `00682730`; alleged function start `00682632` | No behavior coverage, even if read during discovery. Registration grammar, remaining UI construction/paint/geometry, add-resource, container helper internals and undecoded thunks remain unsupported. `00682632` is not accepted as an independent owner body. |

This checklist inventories discovered owner bodies and adjacent helpers, not all inherited cGZWin slots or a
coverage-map regeneration. Open boundaries are tracked in [questions/resource-catalog.md](questions/resource-catalog.md).
