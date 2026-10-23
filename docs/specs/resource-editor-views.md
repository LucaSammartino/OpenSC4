# Resource editor views and default conversion

Status: behavior publication, re-resource-editor5, 2026-10-23. **Partial owner coverage**: the
contracts below can be implemented independently, but the open questions are not completion claims.
Source: Windows SimCity 4 Deluxe 1.1.641, image base `00400000`. Addresses identify evidence,
not portable implementation entry points. Numeric statements below were checked against instruction
bytes and, for constants and secondary tables, the executable's data. No live REA result or game run
is claimed. The available decompilation fails for most catalog/key-input bodies; those contracts
use exported instructions and the earlier catalog publication.

## 1. Scope, dependencies and host boundary

This extends [resource-exemplar-editor.md](resource-exemplar-editor.md), especially §§2–7
(Ingred.ini, definitions, editor selection and write-back), §8 (class factories), §9 (value dialog),
§11 (16-column grid) and §12 (resource-key table). The catalog's row enumeration, descriptor
filtering, selection/preview switching, close results, key probing, filename generation and
shutdown are specified in [resource-catalog.md](resource-catalog.md) §§1–6. Those rules remain
normative; this document adds owner inventories, shared painting, message dispatch and curve/default
contracts rather than duplicating them. [ui-windows.md](ui-windows.md) owns window-manager,
widget and drawing primitives; [modal-dialogs.md](modal-dialogs.md) owns modal-loop behavior.

*[compat]* These are editor views, not simulation response-curve evaluation services. A curve's
GetValue produces a property variant; painting and editing do not write a DAT file, install a resource
or mutate the target exemplar directly. PropertyView's Done is a separate write-back operation.
The catalog delegates resource creation to its addResource path; this publication does not establish
that path's persistence contract (REV4).

*[choice]* A portable core may expose state and an ordered stream of host operations rather than
instantiate native widgets. Preserve child IDs, dispatch results, value shape, conversion and
observable ordering. Host operations include child lookup, QI, retained references, text access,
show/hide, modal completion, resource enumeration and graph updates. A missing native range check
does not establish a defined portable outcome for an invalid pointer or malformed array.

## 2. Interface and method inventory *[compat]*

Slots are **byte offsets**, with the zero-based slot number in parentheses. Unlisted window slots
inherit the window/base editor contract. Exported folded function names are not reliable owner names.

| Owner / table receiver | Slot → body |
|---|---|
| PropertyView, table `00AAE270`, object start | Init `10` (4) → `006751F0`; Shutdown `14` (5) → `0066E870`; paint `160` (88) → shared `00675D80`; KeyDown `208` (130) → `00682100`; KeyUp `20C` (131) → `00682120`; command `23C` (143) → `0066F740`; deleting destructor `250` (148) → `0066EE60` |
| ResourceCatalogView window, table `00AAFC20`, **C+4** | DoMessage `0C` (3) → `006858E0`; Init `10` → `00686D30`; Shutdown `14` → `00687890`; paint `160` → shared `00675D80`; mouse down `218` → `00685610`, up `220` → `006856B0`, move `228` → `00685660`; command `23C` → `00686310` |
| ResourceCatalogView catalog interface, table `00AAFE7C`, **C** | QI `00` → `006855E0`; AddRef `04` → `00762470`; Release `08` → `00762480`; byte option `0C` → shared `0098AD30`; read-only `10` → `00664630`; type filter `14` → `00664640`; group filter `18` → `00664650`; GetSelectedKey `1C` → `006857E0`; RefreshKeyList `20` → `00685CF0` |
| ResKeyInputView, table `00AAF510`, object start | DoMessage `0C` → `00683130`; Init `10` → `00682730`; Shutdown `14` → `00682060`; paint `160` → `006821F0`; command `23C` → `00682230`; deleting destructor `250` → `006826E0` |
| GeneralResponseCurveEntryView, table `00AADEE8`, object start | QI `00` → `0066B190`; Init `10` → `0066D240`; inherited Shutdown `14`; paint `160` → `0066B1D0`; command `23C` → `0066DCE0`; deleting destructor `250` → `0066B990` |
| HappinessCurveEntryView, table `00AAD130`, object start | QI `00` → `0066B110`; Init `10` → `0066C5F0`; inherited Shutdown `14`; paint `160` → shared `00675D80`; deleting destructor `250` → `006648A0`; button `25C` (151) → `0066B9B0`; redraw `260` (152) → `0066BB50`; add point `264` (153) → `0066CDB0`; remove point `268` (154) → `0066BD30` |

Both curves derive from cSCStandardTypeEntryView (`00687C20`) and expose value-editor IID
`06C5FF7B` at object+`D8`. Their extra QI recognizes IID `273B784C`, returns object+`100`,
and AddRefs through the object. Other IIDs delegate to the base; Happiness first clears the
output. The extra four-slot interfaces have tables `00AADEB0` (general) / `00AAD0FC`
(happiness): QI `00`, AddRef `04`, Release `08`, GetPropertyValueString `0C`. Release
`0066B6A0` adjusts the receiver by −`100` before object Release. These are interface facts,
not a proposed new public C ABI.

Value-editor tables `00AADEC0` (general) / `00AAD10C` (happiness) have QI `00`, AddRef `04`,
Release `08`, GetWindow `0C`, read-only `10`, base initializer `14`, SetValue `18`, GetValue `1C`. SetValue is shared
`0066B180` → `00687950`, which copies the supplied variant into the base's held variant.
GetValue is `0066B220` / `0066BD70` and serializes edited point state, as specified below.
Do not confuse the happiness window slot `268`, whose export is named SetValue but removes a point,
with the value-editor interface's variant SetValue. SetValue alone does not import point state or redraw;
Init performs import. Repeated Init returns true without rebuilding either curve's controls.

### 2.1 Nonvirtual/private owner methods

The following inventory includes named bodies and additional bodies needed to explain interface dispatch.
Addresses grouped in one cell identify distinct methods, not an address range promised as complete code coverage.

| Owner | Methods / addresses |
|---|---|
| PropertyView construction/lifetime | constructor `0066EB30`, destructor `0066EC00`; definition-record constructor/destructor `0066E900/0066E990`, assignment `00671080` |
| PropertyView widgets/resources | createSpinner `0066DD00`, createLabel `0066DDB0`, getResource `0066DEE0`, createLineInput `0066DF60`, getStringResource `0066E180`, getDebugNameResource `0066E210`, createButton `0066E9F0`, getFilenameFromKey `0066F360` |
| PropertyView values/commands | GetPropertyNameStr `0066E3E0`, GetHelpText `0066E430`, assignValuesToProperty `0066EC80`, doButtonPress `0066F530`, GetValueEditor `00672D60`, mapPropertiesToViews `00672F70`, GetPropertyValueStr `00674140`, GetPropertyValueMap overloads `006750F0/00675170` |
| PropertyView configuration | aliasesSectionCallback `00672360`, convertStringToFilter `00672420`, getFilter `00672480`, propertiesSectionCallback `00672670`, Uint8/Uint32ValueMapSectionCallback `00673A70/00673BB0`, processINI `00673CF0` |
| Catalog | constructor `00686450`, actual catalog destructor body `00686510`, deleting wrapper `006866C0`; createLabel `006856C0`, KillSelf `00685780`, setViewKey `00685840`, createButton `006859B0`, updateKeyList `00685AE0`, addResource `00685D70`, doButtonPress `006861F0`, registerTypesFromIni `006866E0`, registerTypes `00686B90` |
| Key input | constructor `00682330`; createLabel `00682140`, getNextID `00682290`, setFileName `006823D0`, createButton `006824A0`, doSpinnerChange `00682590` (export boundary splits its continuation) |
| General curve | constructor `0066B630`, destructor body `0066B6B0`; GetPropertyValueString `0066BF90`, SetCurrentPoint `0066C060`, UpdateCurveDisplay `0066C180`, DoButtonPress `0066CE10`; vector resize helper `0066D1E0` |
| Happiness curve | constructor `006647F0`; GetPropertyValueString `0066BE40`; value serialization `0066BD70`; remaining operations are window virtuals listed above |
| Default parser | cdecl `getDefaultValue` `00670940`, four dword arguments `(output variant, type, signed count, text)`; bool array helper `00670790`; uint8/16/32 helpers `0066F8E0/0066FA50/0066FBC0`; sint8/16/32/64 helpers `0066FD30/0066FEA0/00670010/00670180`; float32 helper `00670300` |

The catalog export also labels `0079B0E0` as catalog GZPaint. The catalog window table's actual
paint entry is `00675D80`; do not assign the unrelated labelled body to this receiver.
Likewise `00664720` belongs to picker destruction; use the attribution in resource-catalog §6.

## 3. PropertyView state and interaction

*[compat]* Construction (`0066EB30`) creates the window, an empty row-ID vector at `DC/E0/E4`,
an empty editor map at `E8/EC`, a null current editor at `F4`, an embedded property at `F8`,
and holds the supplied target property pointer at `114`. The view filter is stored at `D8`;
the constructor's filter lookup falls back to `FFFFFFFF` when no record is found. The target
pointer is stored directly; this constructor does not establish an additional retained reference.

The authoritative definition/filter loading and view-building behavior is resource-exemplar-editor
§§2–6. Row order, value precedence on all reference-resolution failures, and the complete helper
resource/formatter behavior remain partial there (its RE4/RE5); this document does not silently
choose answers for them. Init's existing-initialized path returns true (`00675280` vicinity).
Construction is not equivalent to successful widget creation.

The command contract is resource-exemplar-editor §6.1: button message `287259F6` dispatches
Done `24C93018` / Cancel `24C93019`; list message `E8711255` recognizes property-list ID
`FA12B77F`. A valid selection hides the old editor, displays the selected editor if any,
and updates help control `87C30D17` through text IID `212CDC1F`. Done writes only the selected
property through the editor's GetValue; Cancel writes none. Out-of-range row indices are ignored.
Those commands do not commit every editor in the map.

Shutdown (`0066E870`) hides/detaches through the window operations, clears the editor map (releasing
its retained entries), resets the row-vector end to its begin, then delegates to cGZWin Shutdown.
The row allocation survives this clear. It does not explicitly clear `F4` in this body; do not
use that pointer after shutdown as a retained editor. Destructor `0066EC00` calls Shutdown,
destroys the embedded property and remaining containers, then destroys the window. Deleting wrapper
frees the object only for flag bit 0. INI definitions are shared process state rather than being
re-read for every new view (resource-exemplar-editor §2).

## 4. Catalog and key-input owner supplements

### 4.1 Catalog state and dispatch *[compat]*

Use **C** for the catalog interface receiver and **W=C+4** for the window receiver. Construction
allocates `114` bytes, with read-only byte C+`DC` false, option C+`DD` true and type/group filters
C+`E0/E4` zero. Other important state: selected descriptor C+`F4`, descriptor vector
C+`100/104/108`, current preview C+`10C`, retained list C+`110`. Descriptor entries are 32 bytes;
their preview interface is at entry+`1C`. Their order and reference lifetime are resource-catalog
§§2, 3 and 6. Selector index is mapped to original descriptor index; filtering does not compact
the backing descriptor vector.

| Window DoMessage input | Result and host effects |
|---|---|
| Message type `4348B111` | Submit the same message to message-server slot `10`, with trailing argument zero; return true. |
| `9A1552D2` or `9A1552D5` | Read control from message+4, obtain its window via slot `14`, and check window ID `04C9303F`. If it matches, require control slot `C8` to return exactly 1; then ask held list for selected coordinates (`A8`), require success, read row data (`1E0`), and if nonnull call setViewKey. Return true even when these tests skip the preview update. |
| `0D` and all other types | Delegate cGZWin DoMessage and preserve its result. |

Evidence: `006858E0–006859AA`. The message handler's list pointer is W+`10C` = C+`110`.
This handler checks coordinate success; GetSelectedKey's separate method does not (resource-catalog §3).
The message server call is a host notification, not resource persistence.

GZOnCommand's button/selector behavior is resource-catalog §2; KillSelf/close results are §4.
Read-only/filter setters only store values; their calls do not immediately rebuild the rows.
There is no evidence here that all manually injected mutation commands enforce read-only. Mouse drag
capture/movement, complete descriptor INI grammar, empty-filter Init, widget layout and addResource
failure/ownership behavior remain REV4. Do not invent a sorted descriptor order or recovery descriptor.

### 4.2 Key input state, messages and output *[compat]*

Constructor `00682330` copies the caller's `(T,G,I)` dwords into `EC/F0/F4`, clears reentrancy
byte `E8`, initializes an empty filename string at `F8`, and clears held controls `10C/110/114`.
It does not choose a new instance ID during construction. getNextID, doSpinnerChange and
setFileName are resource-catalog §5: collision checks use the held type and group, and name sanitation
retains ASCII letters only before appending the lower-case eight-digit instance ID. That text is
an output/display value, not a filesystem operation. Shutdown and destruction are §6.

| DoMessage input | Result and state/host effects |
|---|---|
| `4348B111` | Submit same message to message-server slot `10` with trailing zero; true. |
| `682876C9` | Pass message+4 and message+8 as control ID/value to doSpinnerChange; preserve that method's result. |
| `9B1552D8`, source window ID `E83B22EE` | Read held instance text control (`10C`) via `B8`, convert with RZConvertToUint32A and store I. Set reentrancy byte, find spinner `04C93038`, QI `612CE0C3`; if successful set its value through `24`. Clear the byte, then regenerate filename; true. This route does not call getNextID. |
| `9B1552D8`, source window ID `283B52CE` | Regenerate filename; true. |
| `9B1552D8`, other source ID | No owner update; true. |
| `0D` and other types | Delegate cGZWin DoMessage and preserve its result. |

Evidence: `00683130–00683218`. Source identification obtains the message's control at +4,
its window with slot `14`, then the ID with `FC`. This path assumes those objects and the held
instance control are valid. Unlike doSpinnerChange, the text-edit route shows **no Release**
after successful spinner QI (`006831DE–00683204`). A portable implementation need not reproduce
a reference leak, but an ABI adapter must not infer a Release that is absent from the call trace.

GZOnCommand recognizes button message `287259F6` with IDs `24C93018` / `24C93019`, ending
the modal loop with result 0 / 1 respectively; it returns the foreign completion call's AL.
Other command/control pairs return false. Init's complete child creation, initial probe timing,
allocation failures and caller-side copying on acceptance remain REV5. In particular neither a
direct instance text edit nor pressing Done is proven here to reject a duplicate key.

## 5. Shared drawing contract *[compat]*

PropertyView, catalog W and Happiness use paint body `00675D80`. Through their held surface at
window+`68`, they begin drawing with flags `8001` (surface slot `18`). If successful, they call
surface slot `50` with `(0, window rectangle at +24, 0)` and then surface slot `1C` with `8001`.
If Begin fails neither operation is called. The shared window paint continuation `00949ADE`
is invoked in either case. Its misleading folded export name is not evidence of app shutdown.

Key input `006821F0` follows the same sequence but passes its rectangle at `D8`. General curve
`0066B1D0` obtains the fill argument from window slot `19C` using `(rectangle at +24,0)` before
surface `50`, then ends drawing and invokes the same continuation. These contracts define calls
and draw-lock pairing, not an invented pixel format or color interpretation. Child graphs/text
paint through their own host implementations. Surface/child null pointers are not validated by
these paint bodies; native failure on uninitialized receivers is not a benign no-op contract.

## 6. General response curve

### 6.1 Construction, state and variant I/O *[compat]*

Factory class `E8DDA013` is listed in resource-exemplar-editor §8. Constructor `0066B630`
constructs the standard editor and initializes the point vector `104/108/10C` empty. Each point
is two binary32 numbers `(x,y)`, in vector order. The underlying held property variant is base
state, distinct from this editable vector.

On first Init (`0066D240`), the owner initializes the window, sizes its vector for input pairs,
and copies alternating FLOAT32 array elements into `(x,y)` points. Input is expected to be an
even-length FLOAT32 array. There is no demonstrated odd-count rejection; the import loop reads
the following y element. Do not turn malformed input into a silently truncated valid curve.
No sorting of imported pairs was established. Editing maintains ascending x when the imported
sequence already has that order; GetValue preserves actual vector order.

GetValue (`0066B220`, receiver object+`D8`) constructs a FLOAT32 **array**, count `2 × points`,
with elements `x0,y0,x1,y1,…`; it copies the binary32 words, calls variant array setter `1B8`,
then frees its temporary allocation. One point still produces a two-element array; no points
produces an empty array. This is not the base held variant returned unchanged. Allocation failure
has no explicit recovery check in the copying path.

GetPropertyValueString (`0066BF90`) clears the destination and formats each complete pair in
the **supplied variant** as `(%.2f,%.2f) `, including a trailing space. It uses floor(count/2)
pairs and promotes each binary32 input to double for formatting; returns true. This formatter's
odd-tail omission must not be generalized to Init's memory safety.

### 6.2 Controls and editing *[compat]*

| ID | Meaning |
|---|---|
| `26C10A3A` | Current property-value text display |
| `26C10A3B` | Add/update point |
| `26C10A3C` | Delete point |
| `26C10A3D` | Graph |
| `26C10A3E`, `26C10A3F` | Input x / output y line inputs, text interface `231A1C57` |
| `26C10A40`, `26C10A41` | Previous / next point, wrapping at the ends |

GZOnCommand (`0066DCE0`) dispatches only `287259F6` to DoButtonPress. Other command types return
false. Input numbers use RZAtofA, then are stored as binary32. Matching an existing x means
`abs(stored_x − input_x) < binary64(0.00001)`, using x87 subtraction/absolute/comparison of
binary32 operands (`0066CF50–0066CF63`; constant `00AADEA0`). Equality at the tolerance is not a
match. Do not use a relative tolerance, integer comparison or decimal string equality.

- Add/update: find the first matching point, change only its y and redraw. If there is no match,
  insert before the first point with greater x, or append; redraw. Successful add/update returns
  true. This operation has no x/y range clamp and does not enforce a minimum point count.
- Delete: erase the first matching point and redraw. If points remain, place the successor's x/y
  into the line inputs; when deleting the last point, use the new last point. Deleting the final
  point leaves no replacement selection. A successful deletion returns true; a missing x returns
  false. Deletion is permitted with one or two points.
- Previous/next: require a matching current x. Previous wraps index 0 to the last; next wraps
  the last to 0. SetCurrentPoint updates the input text using `%f` for both binary32 coordinates.
  Navigating changes the inputs, not point values. Successful navigation returns true.
- An unknown button or absent required x input returns false. A missing y control after a found
  x control does not establish a transactional failure: the native path can reuse text state.
  That malformed-widget case and failed QI outcomes are REV2.

Evidence for insertion/matching: `0066CE10–0066CFCC`; deletion/navigation
`0066D000–0066D1B5`; input formatting `0066C060`, format string `00AAE160`.

### 6.3 Display and lifecycle *[compat unless marked]*

UpdateCurveDisplay (`0066C180`) reads current edited data through the editor interface for the
property text, locates the graph, clears/reconfigures it, sends one scatter point per vector entry
in vector order (point slot `3C`, arguments x, y, `FFFFFFFF`, `FFFFFFFF`), then requests graph
update slot `6C` with zero. Queried graph interfaces are released at the end. Empty input is
handled separately by the axis setup. The full axis-padding, tick-spacing and graph-service
dependency contract remains REV3; it must not be inferred from unreliable pseudocode. A core
may expose the point stream now and leave axis layout to an explicit host primitive *[choice]*.

The graph is a cGZScatterGraph; inputs are labelled Input/Output. Init does not rebuild after
the init flag is set. Value-editor SetValue copies only base variant state, so replacing that
variant after Init is not a promise that the editable vector or display changes. The deleting
destructor calls body `0066B6B0` and frees only when flag bit 0 is set. Exact child ownership
and destruction ordering within the body remain REV2; do not add disk writes on destruction.

## 7. Happiness curve

### 7.1 State, import and output *[compat]*

Factory class `071E971B` constructs the standard editor, an empty ordered map at `104` with
size at `108`, and its two extra interfaces (`006647F0`). Map keys are signed 16-bit x values;
map values are binary32 weights. Native import does **not** call the bounded addPoint operation.

On first Init (`0066C629–0066C68A`), each UINT32 array element is unpacked:

| Part of packed word | Editable interpretation |
|---|---|
| High 16 bits | Map key (signed 16-bit key storage) |
| Low 16 bits | **Signed** 16-bit integer multiplied by binary32 `0.01` (`3C23D70A`), rounded to binary32 on storage |

Later input words with the same key replace the earlier weight. The map orders by signed key;
there is no import-time domain check or preservation of duplicate knots. Init builds a line graph
labelled Bar Value (x) and Weight (y), x range 0..255, y range 0..30 and y tick spacing 5, calls
updateCurveDisplay, then finishes window initialization. Already initialized returns true.

GetValue (`0066BD70`, receiver object+`D8`) traverses the map in ascending signed-key order and
produces a UINT32 **array**, one packed word per map entry, through variant setter `198`.
For each binary32 weight, calculate in x87 precision `weight × binary32(100) + binary32(12582912)`,
then round once to binary32; take that float's raw low 16 bits. Add those bits to `(signed key
shifted left 16)` to form the output word. This describes the native rounding trick; it is not
truncation of `weight × 100`. Under round-to-nearest-even and weights in 0..30, it yields rounded
hundredths, including even rounding at exact half-integers. The magic value is `4B400000` at
`00AADEAC`; instruction order is `0066BDA0–0066BDC9`. No per-entry clamping precedes serialization.
After the array setter the temporary buffer is freed (`0066BE0A–0066BE20`). No allocation-failure
recovery is established. Caller-supplied arbitrary weights or a changed floating-point rounding
mode require the literal operation above, not a claim that all inputs fit a u16 hundredths value.

GetPropertyValueString (`0066BE40`) independently traverses the **supplied packed variant** in
its original array order. It reads x as signed high-half and y as **unsigned** low-half multiplied
by binary32 `0.01`, promoted through x87 to double; each item is `(%d,%.2f) `, trailing space
included. It assigns the combined text and returns true, including for zero items. Thus packed
`0001FFFF` displays `(1,655.35) `, while importing it stores a negative weight near −0.01.
This discrepancy is native behavior, not a reason to change the packed format.

### 7.2 Mutation and dispatch *[compat]*

addPoint (`0066CDB0`, window slot `264`) accepts x in **0..255 inclusive** and weight in
**0..30 inclusive** for ordered numeric weights. x is the signed low 16 bits of its argument.
Out-of-range x or ordered out-of-range weights leave state and display unchanged. The x87 status
tests also admit an unordered NaN weight; there is no explicit finite-value guard. For accepted inputs, insert or replace
the map value and call redraw even when the stored value equals the input. The float comparisons
and input width are checked at `0066CDB0–0066CE08`.

Window slot `268` (`0066BD30`, misleading export SetValue) removes an existing map key only
when x is in 0..255. A successful erase redraws; a missing or out-of-range key does not. This
direct operation has **no minimum-size guard**. Its map erase helper is `0066B710` (another
misleading folded name).

The button callback (`0066B9B0`) recognizes add `26C10A3B` and delete `26C10A3C`; it uses x
text control `26C10A3E` with CRT 32-bit `atol`, and weight control `26C10A3F` with RZAtofEnglishA
then binary32 storage. Add calls window slot `264`. Delete calls `268` only when map size is
greater than two. That guard constrains the button path, not the direct method. Unknown buttons
return true too. Missing x control suppresses the mutation; missing weight control on Add leaves
the initialized default weight zero. Complete inherited GZOnCommand dispatch/read-only enforcement
is REV1; the button callback's return must not be generalized to arbitrary message types.

### 7.3 Line-graph sampling and failure domain *[compat]*

updateCurveDisplay (`0066BB50`) refreshes the property text through the edited value, clears
the graph and emits **256 samples**, for integer x=0..255. At a knot it emits its exact stored
binary32 weight; between surrounding knots `(a,A)` and `(b,B)` it emits

`binary32(A + ((B − A) × (x − a)) / (b − a))`.

Subtraction, multiplication, division and addition occur in that order in x87 precision; only
the final result is stored to binary32 before graph slot `50` `(value,0,FFFFFFFF)`.
Evidence `0066BCCC–0066BD0C`: integer deltas multiply/divide using FIMUL/FIDIV, with no intermediate
binary32 rounding. Graph update `6C` follows. The intended safe sampling domain is an ordered
map with bounding knots at 0 and 255; no extrapolation/clamp rule is established for missing
endpoints or an empty map. Traversal can reach a sentinel rather than a valid next knot.
Calling add/remove or importing malformed endpoint data is not proof that a usable graph results.
REV1 covers these boundary failures. The core may report an explicit unsupported malformed-curve
result *[choice]*; it must not present that choice as native endpoint recovery.

## 8. getDefaultValue contract *[compat]*

This supersedes the inferred item-conversion paragraph in resource-exemplar-editor §7. Ingred.ini's
outer quoted-field extraction is separate from the numeric array tokenization described here.
getDefaultValue is editor-only and has no drawing, message or file-I/O side effects.

### 8.1 Shape, accepted types and return

The arguments are an output variant, integer type tag, **signed** count, and text. Exactly count
`+1` selects a scalar setter. All other supported numeric counts select an array setter, using
the absolute count as array length. Therefore count `−1` means a one-element array, not a scalar.
Count zero and signed-minimum overflow are outside the safe contract: the helper decrements an
unsigned loop bound and does not establish a useful empty-array result. Do not infer zero-count
success just because arrays elsewhere can be empty.

STRING `800E` is special: assign the supplied text through variant setter `E0` before trimming
or checking emptiness; count does not affect shape. Empty string succeeds. Other types first
left-trim their text; empty after this trim returns false without a value setter. Unsupported
type returns false. Supported nonempty numeric/BOOL text returns true after conversion and
setting the value; there is **no conversion-success check**, range validation or full-token
consumption check in this dispatcher. Host conversion and allocation failures are separate
dependencies. Evidence `00670969–006709D2` and common success/failure exits `00670D9B–00670DD8`.

| Type tag | Scalar conversion / setter | Array helper / setter |
|---|---|---|
| `01` BOOL | Uppercase text; first character T or unsigned numeric conversion nonzero → true; `2C` | `00670790` / `170` |
| `02` UINT8 | RZConvertToUint32A, keep low 8 bits; `38` | `0066F8E0` / `178` |
| `03` SINT8 | CRT 32-bit atol, keep low 8 bits; `44` | `0066FD30` / `180` |
| `04` UINT16 | RZConvertToUint32A, keep low 16 bits; `50` | `0066FA50` / `188` |
| `05` SINT16 | CRT 32-bit atol, keep low 16 bits; `5C` | `0066FEA0` / `190` |
| `06` UINT32 | RZConvertToUint32A; `68` | `0066FBC0` / `198` |
| `07` SINT32 | CRT 32-bit atol; `74` | `00670010` / `1A0` |
| `09` SINT64 | CRT **32-bit atol**, sign-extend to 64 bits; `8C` | `00670180`, same 32-bit conversion and sign-extension / `1B0` |
| `0A` FLOAT32 | RZAtofEnglishA, round to binary32 before setter `98` | `00670300` / `1B8`, binary32 storage per item |
| `800E` STRING | Assign whole text / `E0` | No array path |

Other tags, including `08`, `0B` and object/reference types, are rejected here. This is the
default parser's supported set, not the complete variant type system. Unsigned hex behavior
belongs to RZConvertToUint32A; signed values use decimal CRT atol, so `0x10` yields zero on that
path. Large values do not gain 64-bit precision merely from the SINT64 destination. CRT overflow,
special float values and locale edge cases remain explicit runtime dependencies (REV6).

### 8.2 Array text

For a positive array length N, each helper left-trims the remaining text and extracts a token
using cRZString SplitToken with delimiter string **one ASCII space** for the first N−1 items.
It converts those tokens to the destination width. For the final item it left- and right-trims
the entire remainder and converts it as one numeric input; extra suffix text is handled only
by the conversion primitive. It does not split on commas, validate N tokens, or call
getUnquotedString per array item. Repeated spaces are removed by the next left trim. Tabs are
not declared token delimiters by these calls; their interaction with cRZString's trim/token
primitives remains REV6. A portable implementation must not silently substitute general
whitespace tokenization for the host's SplitToken contract.

BOOL uppercases the full text before either path, then tests first character T or the numeric
conversion for each token. Consequently `turtle` is true and `FALSE` is false under the ordinary
nonnumeric-to-zero converter. Empty numeric text for the entire default is rejected up front;
an empty **interior** token is not rejected by these helpers. The precise exhausted-token behavior
is REV6, since SplitToken ownership/empty-return behavior is a host primitive.

Numeric evidence: uint8 delimiter construction/conversion/store `0066F907–0066FA01`; FLOAT32
array conversion/store `006703D7–00670428`; SINT64 array `atol`, CDQ and low/high word stores
`00670266–006702B0`; scalar SINT64 CDQ `00670C4D–00670C5A`.

## 9. Conformance vectors

These are proposed portable test vectors, **not executed tests or captured game traces**. Host
primitives are injected, with call order recorded. Numeric conversions below use ordinary ASCII
prefix semantics and round-to-nearest-even. No test needs engine source or native pointers.
Use the catalog publication's §7 vectors for row ordering/filtering and collision/name rules.

| ID / contract | Input or setup | Expected result |
|---|---|---|
| REV-P01 / §3 | Two row editors; select second, then Cancel | First hidden, second shown, help updated; close result 1; no target write. |
| REV-P02 / §3 | Selected property has no editor; Done | No target ID/value setter; close result 0. |
| REV-P03 / §3 | Shutdown with two retained map entries | Hide/detach before releasing entries; row count becomes zero; delegate window shutdown. |
| REV-C01 / §4.1 | Catalog notification `9A1552D2`; right list ID; coordinate query false | Return true; no row-data read and no preview call. |
| REV-C02 / §4.1 | Same, coordinate success and nonnull row key K | Pass K to setViewKey; return true. |
| REV-K01 / §4.2 | Instance text event from `E83B22EE`; converter returns 42; spinner QI succeeds | I=42; set guard, spinner SetValue(42), clear guard, regenerate filename; no collision probe. |
| REV-K02 / §4.2 | Spinner notification `682876C9` while guard is set | doSpinnerChange returns true immediately; no corrective recursive change. |
| REV-K03 / §4.2 | Key-input Cancel button | Complete modal result 1; preserve host completion AL. |
| REV-D01 / §5 | Shared paint Begin returns false | Begin(8001), then continuation; no fill/End. |
| REV-D02 / §5 | Key paint Begin succeeds | Begin(8001), fill(0, rectangle D8,0), End(8001), continuation. |
| REV-G01 / §6.1 | General editable points `(1,2),(3,4)`; GetValue | FLOAT32 array `[1,2,3,4]`, count 4, even for a formerly scalar/base value. |
| REV-G02 / §6.1 | Supplied FLOAT32 array `[1,2,3]`; formatter | `(1.00,2.00) `; formatter ignores odd tail. Do not use this as an Init vector. |
| REV-G03 / §6.2 | Points `(0,1),(2,3)`; Add x=1,y=9 | `(0,1),(1,9),(2,3)`; redraw once; true. |
| REV-G04 / §6.2 | Point `(1,2)`; Add x=`1 + 2^-18`, y=7 | Retain x bits of 1, set y=7; one point, redraw, true. |
| REV-G05 / §6.2 | One point `(1,2)`; delete x=1 | Empty edited vector; redraw; no replacement input selection; true. |
| REV-G06 / §6.2 | Points `(0,1),(2,3)`; current x=0; Previous | Inputs become `%f` text of `(2,3)`; vector unchanged; true. |
| REV-G07 / §6.2 | Unknown general command type | False; no mutation. |
| REV-H01 / §7.1 | Packed words `[00000064,00FF00C8,000000C8]`; import | Two knots: x=0 weight near 2, x=255 weight near 2; later duplicate wins. |
| REV-H02 / §7.1 | Packed word `0001FFFF` | Formatter `(1,655.35) `; import weight is binary32(−1 × binary32(0.01)). |
| REV-H03 / §7.1 | Map inserted out of order: `(255,2),(0,1.25)`; GetValue | UINT32 array `[0000007D,00FF00C8]`, ascending x. |
| REV-H04 / §7.2 | Valid bounding map; add x=256 or weight=31 | State unchanged; no redraw. With in-range x and NaN weight, store NaN and redraw instead. |
| REV-H05 / §7.2 | Two knots; Delete button for existing key | No deletion due to size guard; callback true. Direct remove of same key does erase/redraw. |
| REV-H06 / §7.3 | Knots `(0,0),(255,25.5)` | 256 graph samples; sample 0=0, 10=1, 255=25.5; update after last sample. |
| REV-F01 / §8.1 | UINT8, count +1, text `300` | True; scalar UINT8 44, no range error. |
| REV-F02 / §8.1 | UINT8, count −1, text `300` | True; UINT8 array `[44]` of length 1. |
| REV-F03 / §8.1 | STRING, any count, empty text | True; empty scalar string. |
| REV-F04 / §8.1 | UINT32, count +1, all spaces; or unsupported tag 08, text `1` | False; no value setter. |
| REV-F05 / §8.1 | BOOL, count +1, text `turtle` / `FALSE` | True return in both; values true / false respectively. |
| REV-F06 / §8.1 | SINT64, count 2, text `-1 2147483647` | Array `[-1,2147483647]`, 32-bit conversions sign-extended; no 64-bit parser call. |
| REV-F07 / §8.2 | UINT32, count 2, text `1,2 3` | Space splitter supplies `1,2` then `3`; with prefix converter results `[1,3]`. Comma is not an array separator. |
| REV-F08 / §8.2 | FLOAT32, count 2, text `1.5 2.5 suffix` | Array `[1.5,2.5]` with prefix float converter; no suffix validation. |

## 10. Remaining work

Questions REV1–REV6 are in [questions/resource-editor-views.md](questions/resource-editor-views.md).
Do not award whole-owner coverage from the inventory or infer successful native allocation/error
recovery for unchecked calls. Existing resource-exemplar-editor questions RE1, RE5 and RE7 are
partly informed here but remain open: each contains additional unresolved requirements. No question
has been moved to answered on the basis of a partial answer.
