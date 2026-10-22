# Native lot configuration editor

Source: SimCity 4 Deluxe GOG 1.1.641, Windows executable; export and
`decomp/cSC4WinLotConfigurationEditor.c` read from the main checkout, 2026-10-22.
This is the native `cSC4WinLotConfigurationEditor`, class ID `0x4918C704`
(registration at `0x00B08A28`). It is separate from the resource/exemplar/value
editors in [resource-exemplar-editor.md](resource-exemplar-editor.md).

**Coverage status: partial, no complete-owner spec credit.** The fresh owner
inventory has 111 entries, including 92 named editor methods. The named bodies
sum to 51,054 bytes; the other 19 entries contribute 1,826 attributed bytes.
The old snapshot totals **52,880 attributed bytes**; reconciliation against
current exported entries gives **109 entries / 52,646 body bytes**, matching
the brief. These are inventory counts, not a claim of complete contracts;
see §6 and the inventory companion for the exact difference.
Sections below distinguish established dispatch/state rules from unresolved
dialog and numeric details. An address citation identifies native evidence,
not an executable conformance fixture.

## 1. State and selection *[compat]*

The editor holds a current lot configuration, a dirty byte, a selected object
pointer and a selected object index. Object records are 52 bytes apart. Native
offsets used here are evidence labels, not requirements on a replacement's
memory layout.

`SetDataHasChanged` (`0x00480590`, 18 bytes) stores the caller's byte at editor
`+0x12C`, then calls `SetTitle` (`0x0047F0B0`). The title begins `Lot Editor -- `;
its dirty suffix is ` *` (native string bytes at `0x00A8DCC0`).

`SetSelectedLotConfigurationObject` (`0x0047B100`, 238 bytes): changing the
pointer ends a pending drag (`+0x1B8`) and releases mouse capture if held. Null
selection sets the index (`+0x1B4`) to zero. A non-null pointer is searched by
identity in the current configuration's record vector (`+0x90..+0x94`). A
pointer not in that vector is retained as the selected pointer but its index
falls back to zero; it is not rejected or replaced by the first record.
Selecting the same pointer leaves drag/capture alone.

## 2. Edit-area popup *[compat]*

`DoEditAreaMenu` (`0x004871B0`, 2,425 bytes) builds a string list, obtains popup
coordinates through the UI service, shows the popup and restores focus to the
editor before dispatch. The returned zero-based row is used directly; separator
rows count. Cancellation or an unrecognised row causes no command. Temporary
menu strings are destroyed after dispatch.

| Row | Action |
|---|---|
| 0 | Save with both Boolean options true. |
| 1 | Save As with `(copy=false, option2=true, option3=true)`. |
| 2 | Save a Copy As with `(copy=true, option2=true, option3=true)`. |
| 3 | Separator. |
| 4 | Set lot name dialog. |
| 5 | Set lot size dialog. |
| 6 | Set lot foundation, passing the current lot-foundation ID and no building object. |
| 7 | Set retaining wall dialog. |
| 8 | Set lot compatibilities dialog. |
| 9 | Separator. |
| 10 | Add building. |
| 11 | Add flora. |
| 12 | Add prop, passing both selector options true. |
| 13 | Add lot base texture. |
| 14 | Add network node. |
| 15 | Add water constraint. |
| 16 | Add terrain constraint. |
| 17 | Add/edit fences. |
| 18 | Separator. |
| 19 | Go to 3D view. |
| 20 | Refresh 3D view through editor virtual slot `+0x27C`. |
| 21 | Cycle lot condition through `0x0047A8C0`. |
| 22 | Set 3D view slope dialog. |
| 23 | Separator. |
| 24 | Help page 0. |

The row labels advertise Control-S, Control-Shift-S, Control-Alt-S, G, R, C and
F1. Keyboard handling is a separate contract; a label alone does not establish
which modifiers the keyboard handler accepts.

## 3. Selected-object popup *[compat]*

`DoLotConfigurationObjectMenu` (`0x00487B30`, 5,971 bytes) chooses a menu by the
record's first byte. It restores editor focus after each popup. This section
establishes command dispatch and direct flag effects; bounding-box arithmetic,
network edge labels and two constraint branches remain to be reconciled.

| Object type | Row-to-action mapping (zero based, separators included) |
|---|---|
| 0, building | 0 families; 1 building foundation; 2 delete building foundation; 3 rotation instructions; 4 duplicate; 5 delete; 6 separator; 7 help page 1. |
| 1, prop | 0 families (both options true); 1 rotation instructions; 2 elevation instructions; 3 toggle low flag bit; 4 duplicate; 5 delete; 6 separator; 7/8/9 detail low/medium/high; 10 separator; 11 help page 4. |
| 2, base texture | 0 textures; 1 rotation instructions; 2 mirror; 3 duplicate; 4 delete; 5 separator; 6/7/8 detail low/medium/high; 9 separator; 10 help page 6. |
| 4, flora | 0 families; 1 duplicate; 2 delete; 3 separator; 4/5/6 detail low/medium/high; 7 separator; 8 help page 2. |
| 6 | No action. |
| 7, network node | 0 delete; 1 network type submenu; 2 minimal edges submenu; 3 maximal edges submenu; 4 network path dialog; 5 separator; 6 help page 7. |

Building-foundation deletion clears record `+0x20` only when it was nonzero,
then sets dirty, updates the title and requests a redraw. Accepting a family
or texture selector replaces the record's ID vector, recomputes its bounds,
sets dirty, updates the title and requests a redraw. A cancelled selector does
none of these writes. Duplicate and delete delegate to their respective editor
methods; they are not implemented as raw vector operations by this menu.

Rotation rows show instructions to use Page Up and Page Down. Prop elevation
shows instructions to hold Space while pressing Up/Down; these rows do not
directly rotate or move an object.

Detail choices replace byte `+1` by `(previous byte AND 1)` plus the chosen
detail value: low `0`, medium `0x10`, high `0x20`. Thus other bits, including
`0x02`, are cleared. The prop toggle XORs that byte with `1`. These direct flag
branches do **not** call the dirty/title/redraw helpers. Preserve that observable
distinction rather than treating every popup choice as a generic edit commit.
The current-marker labels inspect mask `0x30`; their unusual low-detail/high-detail
combination still requires full branch reconciliation.

Network type submenu presents 13 types, followed by a separator and an uncertain/
try-later choice. Only an unsigned result below 13 replaces the first network
payload word. The edge submenus accept only results below 20; each uses a
key/label table at `0x00A8E6C8`, writing the selected key to payload word 1
(minimal) or 2 (maximal). Cancellation and trailing choices preserve the word.
These payload writes also bypass dirty/title/redraw helpers. Exact edge names
are deliberately not inferred from their numbers.

## 4. Lot compatibility dialog *[compat]*

`DoSetLotCompatibilitiesUI` (`0x00480D10`, 6,818 bytes) calls the script-window
factory with selector `0xE9264B49` and key words `(0, 0x96A006B0, 0xE9263D4D)`
(`0x00480D25..0x00480D58`). Creation failure bypasses all model changes.
It registers the created window with the modal service, runs it through the
editor's dialog host, unregisters it and restores focus. Only modal result
`0x42B7C356` enters the write-back branch (`0x004823B1..0x004823F2`).

The following numeric/control rules are instruction-verified; property names
are not assigned without a confirmed interface mapping:

| Child ID / queried interface | Initial source | Accepted write-back |
|---|---|---|
| `0x294053DB` / `0x8810` | Configuration virtual `+0x38`, to child `+0x24`. | No write-back: this control is never queried in the accepted branch. |
| `0x294053DA` / `0x612CE0C3` | Configuration virtual `+0x28`, x87-to-integer conversion, child `+0x24`; clear child flag `0x4000`. | Signed 32-bit child value becomes binary32 at configuration `+0x38`. |
| `0xE9B98665` / `0x612CE0C3` | Configuration virtual `+0x20`, same conversion/flag handling. | Signed 32-bit child value becomes binary32 at configuration `+0x30`. |
| `0xE9B98666` / `0x612CE0C3` | Configuration virtual `+0x24`, same conversion/flag handling. | Signed 32-bit child value becomes binary32 at configuration `+0x34`. |
| `0xC9B981C9` / `0x612CE0C3` | Low byte of configuration virtual `+0x2C`, same flag handling. | Take the low byte of the child integer; values 0 and 1 become 1, values 2..255 remain that byte; store configuration `+0x3C`. |

The last row truncates **before** its unsigned comparison: 256 becomes 1,
258 becomes 2, and -1 becomes 255. This is not a signed lower-bound clamp of
the original integer (`0x004826A3..0x004826CF`). The three float writes use
`FILD` signed dword then `FSTP` float (`0x00482649..0x00482698`).

Four optional checkboxes, IDs `0xCA4A929A+i` for i=0..3, initially reflect bits
0..3 of configuration byte `+0x8E`. On acceptance that byte is first cleared;
only present checked controls contribute their bit. Missing controls therefore
clear their bit, and bits 4..7 are lost. The loop is bounded to four
(`0x0048235B..0x004823AE`, `0x004826D1..0x0048270C`). Acceptance sets dirty
and updates the title even if the values did not change
(`0x0048270E..0x00482717`). No redraw helper follows that commit.

### 4.1 Multi-selection grids and acceptance order

All three grids query IID `0xDAA6B9BE`, initialise with `(2,32,0)` through
`+0x78`, then clear their rows through `+0xB8`. Each has two columns: column 0
contains decimal text `1` through its last model bit (format `%d` at
`0x00A8281C`), column 1 contains the description below. Row numbers are zero
based; **row r maps to model bit r+1**. Initial selection tests that bit in the
corresponding bit vector and selects column 0 of the row through `+0xB0`.
The full initialisation is at `0x00480F1A..0x00482359`.

| Row | Bit | `0x49405340`, vector `+0x44` | `0x49405342`, vector `+0x58` | `0x49405343`, vector `+0x6C` |
|---:|---:|---|---|---|
| 0 | 1 | Low Density Residential | Low Wealth | Residence |
| 1 | 2 | Medium Density Residential | Medium Wealth | Services |
| 2 | 3 | High Density Residential | High Wealth | Office |
| 3 | 4 | Low Density Commercial | — | Tourism |
| 4 | 5 | Medium Density Commercial | — | Agriculture |
| 5 | 6 | High Density Commercial | — | Processing |
| 6 | 7 | Resource Industrial | — | Manufacturing |
| 7 | 8 | Light Industrial | — | High Tech |
| 8 | 9 | Heavy Industrial | — | — |
| 9 | 10 | Military | — | — |
| 10 | 11 | Airport | — | — |
| 11 | 12 | Seaport | — | — |
| 12 | 13 | Spaceport | — | — |
| 13 | 14 | ` Landfill` (leading space) | — | — |
| 14 | 15 | Plopped Building | — | — |

The one-based bit indices and vector bases are direct operands of the native
bit-access calls, e.g. `0x0048102F..0x00481043`,
`0x00481B5F..0x00481B73`, `0x0048230F..0x00482323`.
There are no rows for bit 0 or for bits beyond the listed last bit.

Acceptance processes grids in the table's left-to-right order. For each it
requests up to 32 selection records through `+0xA4`, clears the **entire
current logical bit-vector range**, then sets bit `selectedRow+1` for each
returned record. It reads the row word of each 8-byte record, not the displayed
decimal string and not a separate row-ID map. Column is ignored. Consequently
an empty selection clears the vector, formerly set bit 0 and other undisplayed
bits are lost, and repeated records merely set the same bit again
(`0x004823F8..0x0048263F`). This body does not resize the vector or validate a
returned row against the displayed rows.

Then it writes the four available numeric controls in table order excluding
`0x294053DB`, rebuilds the checkbox byte, and marks dirty/updates title.
`0x294053DB` is seeded and later Released, with **no accepted getter or model
write** (`0x00480D74..0x00480DEA`, `0x004823F8..0x00482734`). Changing its
displayed value alone therefore does not change that model field, although
acceptance still marks the editor dirty.

Missing scalar controls skip their initialisation and write-back. Missing
grids skip initialisation, but accepted write-back **unconditionally
dereferences all three grid pointers**. Thus a missing first grid faults before
vector writes; a missing second/third may fault after earlier vectors changed.
Cancellation skips these dereferences and all model changes. This is not a
supported optional-grid configuration. On normal exit the editor Releases
each non-null acquired scalar/grid/checkbox reference, shuts down and Releases
the dialog window (`0x00482723..0x004827B8`). Return is the acceptance byte;
creation failure and cancellation leave it false.

## 5. Independent conformance vectors

These expected outcomes come from native control flow, not an implementation.

| Initial state / input | Action | Expected result |
|---|---|---|
| Drag active; capture held; selection A. | Select B. | Drag ends, capture releases, B is retained; index is searched in the record vector. |
| Drag active; selected A. | Select A again. | Drag and capture remain unchanged. |
| A pointer outside the record vector. | Select it. | Pointer is retained; index becomes zero. |
| Object-popup detail flag byte `0xA3`, prop. | Choose medium detail. | Byte becomes `0x11`; dirty/title/redraw are untouched by this branch. |
| Flag byte `0x10`, prop. | Choose toggle row. | Byte becomes `0x11`, with no dirty helper call. |
| Nonzero building foundation; dirty false. | Delete building foundation. | Foundation becomes zero, dirty true, title updated, redraw requested. |
| Existing family vector and bounds. | Cancel family selector. | Existing vector/bounds/dirty state preserved. |
| Network type 4. | Submenu returns 12, then separately 13 or cancellation. | 12 is stored in the first case; 4 remains in the latter cases. |
| Current compatibility values, dirty false. | Dialog returns another result than `0x42B7C356`. | No compatibility write-back or dirty update. |
| Integer control values -1, 256, 258 (separate runs). | Accept compatibility dialog. | Stored byte `+0x3C` is respectively 255, 1, 2. |
| Compatibility byte `+0x8E=0xFF`; all four controls absent. | Accept dialog. | Byte becomes zero; dirty true and title updated. |
| Edit-area popup returns 3, 9, 18, 23, -1, or 25. | Dispatch. | No editor command is invoked. |

## 6. Inventory and remaining work

The [inventory companion](lot-configuration-editor-inventory.md) records every
entry in the measured owner set, its attributed byte count and contract status.
LE1 and LE3–LE5 track the unresolved behavior; LE2 is answered in §8.
No ABI changes or game-run evidence.

## 7. Interfaces, creation and lifetime *[compat]*

`QueryInterface` (`0x0047A830`, 64 bytes) recognises IID `0xC6AE7085`, returns
the interface subobject at editor `+0x128`, AddRefs the editor and returns true.
Other IIDs delegate to `cGZWinGen::QueryInterface`. There is no safe null-output
case. The null-this branch writes null and still attempts the virtual AddRef;
do not treat it as a supported way to query capabilities
(`0x0047A830..0x0047A86F`). Main-vtable AddRef/Release entries are the shared
bodies `0x008AE960`/`0x008AE970` (native table `0x00A8D888`). This spec does not
assign the private callback IID a public lot-editor API name.

The constructor (`0x0047DA80`, 375 bytes) starts clean, with null selected
pointer, index zero, drag false, placement mode zero, empty pending-ID vector,
null current configuration, null service/view/chooser/network references and
empty owned cursor containers. The pixel-to-object scale starts at binary32
1.0 (`+0x1AC`); view-condition/slope state starts at zero. It installs the main
window, auxiliary window-interface and dialog callback vtables. Construction
alone does not create/load a configuration.

`Init` (`0x004892B0`, 295 bytes) invokes static initialisation if its shared
flag is unset. The base-window initialised flag gates the rest: first Init
sets window/interface flags, invokes base Init, obtains an Arial size-12 font
if missing, replaces the manager reference with the global configuration
manager using AddRef-before-Release, posts message `0x0920B687`, loads cursors
and installs the normal cursor. Repeated Init returns true without repeating
that block. Exact flag meanings/static resources are LE5, not inferred from
their numeric masks.

The main `DoMessage` (`0x0047BA80`, 73 bytes) consumes all category-3 messages.
Within that category, command `0x287259F6` and control `0x42B7C353` call editor
virtual `+0x264` with `(true,true)` (Close). Message `0x0920B687` calls PostInit
and returns true. Other categories delegate to the base procedure.

`PostInit` (`0x0047B890`, 488 bytes) lazily acquires view interfaces. It also
obtains network-manager objects for type selectors 3 and 0 with the second
option true; for each missing stored object it AddRefs and invokes `+0x0C`.
Repeated calls leave already-stored references alone. The central view/alpha
setup has broken stack recovery and remains LE5.

`Shutdown` (`0x0047C510`, 322 bytes), when initialised, first notifies a stored
chooser that this editor closed, then releases and clears the font, view/service
references, chooser, configuration-manager and identity references. For each
stored network object it invokes `+0x10`, then Release, then clears the pointer.
It Releases each non-null object in the owned reference array
`+0x1D0..+0x1D4`. Base Shutdown runs even if the initialised flag was false.
This body does not release the current configuration directly; destruction and
container cleanup must not be invented from this Shutdown list (LE5).

## 8. New, open, save and close *[compat]*

### 8.1 Configuration creation and opening

`CreateNewLotConfiguration` (`0x00483B20`, 342 bytes) clears all identity/key
words and path text, Releases and clears both identity references, constructs
a configuration and replaces the prior configuration with AddRef-before-Release.
The new configuration has ID zero, dimensions 1 by 1, float fields `+0x38=5.0`,
`+0x30=90.0`, `+0x34=0.0`, and byte `+0x42=0`. It clears dirty, updates the
title and returns true. Constants are direct native writes at
`0x00483C26..0x00483C62`. The name is the literal seven-character `New Lot`:
`0x00483BCB..0x00483C0E` constructs the range `0x00A8E020..0x00A8E027`
and assigns it to configuration `+0x10`; it is not obtained from a chooser,
path or generated counter. Allocation failure installs a null configuration
and then dereferences it at `0x00483BD9`; there is no graceful false-return path.

`ReadLotConfigurationExemplar` (`0x00483C80`, 281 bytes) returns false if the
global manager is absent or its read fails, preserving the prior current
configuration. A successful read replaces the current pointer with
AddRef-before-Release and asks the manager to reconcile its objects. Result
zero returns success without setting dirty. Nonzero results set dirty, update
the title and show `This lot has components that have changed since the last
edit.` with the following reason when recognised:

| Result | Reason text |
|---|---|
| 1 | The family has changed. |
| 2 | The family is now empty. |
| 3 | The footprint has changed. |
| 4 | The footprint was out of bounds and needs to be fixed. |
| 5 | The center has changed. |
| 6 | The retaining walls have changed. |

Other nonzero values retain only the introductory message. This is a warning
after successful replacement, not a rollback of that replacement.

`Open` (`0x00486530`, 145 bytes) calls PostInit, then uses the first word of
the supplied key to choose new configuration (zero) or read exemplar (nonzero).
Failure of that operation returns false. For nonzero keys, failure to obtain
identity information also returns false; it does not undo the successful
configuration replacement. Then, in order: update title; SetLotSize using the
configuration's two dimension bytes with its third option false; remove
out-of-bounds objects; prepare 3D view; go to 3D view; invoke refresh virtual
`+0x27C`; return true. The post-load cleanup can therefore alter loaded data.

### 8.2 Save and Save As

`Save` (`0x004852A0`, 238 bytes) has separate success-message and error-message
options. If identity word `+0x168` is zero, it temporarily sets shared byte
`0x00B216B8=1`, calls Save As with `(copy=false, showSuccess, showError)`, clears
the shared byte, and returns that result.

For an existing identity it first calls bounds verification with both options
true. Refusal returns false without writing. If verification permits the save,
a missing manager or false result from persistence virtual `+0x268` fails;
when enabled, the error message is `Error saving lot configuration.` with
message-box flag `0x10000`. On successful persistence it clears dirty, updates
the title, passes the identity starting at `+0x168` to the service obtained from
global `0x00B43CEC` virtual `+0x94`, and optionally shows
`Lot configuration saved.` with flag zero. Suppressing the success message does
not suppress the service notification. No success is inferred from the upper
24 bits of the native return register: callers test AL.

The persistence virtual points to `0x0047AB70` (43-byte supplemental helper,
outside the attributed set); it forwards the identity starting at `+0x168`,
the current configuration and identity reference `+0x188` to `0x00489700`,
or returns false in AL if the manager is absent. Its downstream file format is
outside this owner's contract.

**Save As entry correction:** the main-vtable word at `0x00A8DAE8`
(`+0x260`) is **`0x00485390`**. The exported label `0x00485445` begins in the
second byte of the instruction at `0x00485444`; its initial interrupt/arithmetic
pseudocode is spurious. Native bytes `0x00485444..0x0048544B` encode the view
flag call after the file-browser setup. The export's 180-byte prefix at
`0x00485390`, unassigned byte at `0x00485444`, and 700-byte labelled continuation
must be treated together for this workflow, not as three editor operations.

The browser is created as a file browser, sized 200 by 200, configured with
mode 2 and an initial directory: default path if no existing identity, otherwise
the current path's directory. The default path helper (`0x0047EFF0`, 83 bytes)
appends `Exemplar` and `Lot Templates` to the data directory using backslash
separators. After modal selection, result retrieval failure reports
`Unknown error in file selection.`; an item kind other than 2 reports
`You must select a file.`; failure to resolve the selected path reports
`The file you selected was somehow invalid.`. Dialog cancellation does not save.

For a resolved selection, Save As copies the old identity, installs the new
identity, writes its ID word into configuration `+0x08`, and calls Save with
the supplied message options (`0x004855C6..0x004855FF`). Failure restores the
old identity and configuration ID. Success queries the manager for the saved
ID, registers the configuration with option true if absent, and refreshes the
chooser list if a chooser exists. Successful **copy** also restores the old
identity and configuration ID; successful ordinary Save As retains the new
identity (`0x00485605..0x004856AD`). Copy does not restore the prior dirty byte:
the inner successful Save has already cleared it. The identity and reference
rules below complete this workflow. There is no title refresh after restoring
the old identity in the copy/failure branch; the inner Save's effects have
already occurred.

The browser window flag call is `(0x100,true)` at `0x0048543D..0x0048544B`,
through window virtual `+0x110`. This is confirmed from the executable bytes
across the false export boundary. Browser virtual `+0x14` receives mode 2.
The browser allocation failure branch sets its pointer to null and immediately
dereferences it (`0x004853EE..0x00485419`); it does not return a clean failure.

The browser object receives one explicit AddRef before use. After the modal
attempt, including cancellation and all three selection-error branches, the
editor unregisters the window from the modal service, removes it through the
dialog host, Releases the browser and restores editor focus, in that order
(`0x00485586..0x004855A8`). Selection errors use message-box option zero,
independently of the Save error-message option. The browser is already gone
before any identity replacement or persistence attempt.

Identity snapshots copy their path string but **copy reference pointers
without AddRef or Release** (`0x0047E5F0..0x0047E64E`,
`0x0047F050..0x0047F0A6`). Save As therefore transfers ownership explicitly:

| Outcome after installing the selected identity | Reference and identity cleanup |
|---|---|
| Save succeeds, ordinary Save As | Keep selected identity; Release each non-null old reference from the snapshot. |
| Save succeeds, copy | Restore snapshot; Release each non-null selected reference; reset configuration ID from restored identity. |
| Save fails | Same restoration and selected-reference cleanup as copy; return false. |
| Cancellation or invalid selection | Never install selected identity; return false. |

The two references are independently Released, even if their pointer values
coincide. The local selected-reference slots are cleared after release on the
restoration path. Snapshot/path string buffers are freed on their applicable
exit paths (`0x0048564A..0x00485700`). This is not a general reference-owning
assignment operation: a caller reusing a populated identity output must arrange
its old-reference cleanup separately.

### 8.4 Resolving identities

The identity record, relative to editor `+0x164`, consists of a separate ID at
`+0`, a three-word key at `+4/+8/+0x0C`, path string at `+0x10`, and two retained
references at `+0x24/+0x28`.

`GetIdentityInfoForLotConfigurationExemplarPath` (`0x0047F200`) starts with
success false. It extracts the selected path's directory; a nonexistent
directory or absent global provider manager fails without installing an
identity. It enumerates manager virtual `+0x90` in two calls (count, then array),
lowercases the extracted directory, and scans providers in enumeration order.
Each provider must supply IID `0x856C8CF7`. Its directory, obtained through
that interface's `+0x20`, is lowercased and compared by equal byte length and
equal bytes (`0x0047F2BB..0x0047F365`). This is full directory equality, not a
prefix test. No extra slash or path normalisation is performed by this body.

For an equal directory it splits the original selected path into filename stem
and extension and calls the queried interface's `+0x4C` with those two strings
and the output key. Only a true AL from that call establishes success. The
provider supplies the key; this editor does not itself hash, parse or invent
an instance ID. On success the editor copies the original, case-preserving
selected path, copies key word `+0x0C` into the separate ID, stores both provider
and queried-interface references and AddRefs both (`0x0047F37D..0x0047F3D8`).
A matching directory whose key derivation fails does not prevent trying later
providers. After the first success, later providers are only Released.

Every successful queried-interface acquisition is Released after its attempt;
every enumerated provider reference is Released, including providers after
success. The enumeration allocation and temporary strings are freed. The final
return reloads the saved success byte (`0x0047F430..0x0047F479`), contradicting
the corrupted pseudocode's unconditional failure. Failure does not promise to
clear an output key partly written by a failing provider. The retained path,
ID and reference installation occur only after provider success.

`GetIdentityInfoForLotConfigurationExemplarKey` (`0x0047B1F0`) fails if the
provider manager is absent, its `+0x80` lookup fails, or the returned provider
cannot supply IID `0x856C8CF7`. After successful acquisition it asks that
interface's `+0x48` for the path, copies all three input key words, copies the
instance word into the separate ID, retains/AddRefs the two references, Releases
the temporary acquisitions and returns true. **The path query's return value
is not tested** (`0x0047B231..0x0047B283`). Thus a failed path query does not
by itself make this helper return false.

`GetIdentityInfoForLotConfigurationType` (`0x0047BAD0`) first requires the
configuration manager and a successful type lookup (`+0x20`). It reads that
configuration's ID through `+0x0C`, constructs key
`(0x6534284A,0xA8FBD372,ID)`, then returns the key helper's result. The earlier
temporary group word `0x67CD5FA1` is overwritten before the key-helper call
(`0x0047BAD0..0x0047BB34`); it is not the final lookup group.

### 8.3 Closing and failure ordering

`Close` (`0x004865D0`, 191 bytes) takes independent `ask` and `save` flags.
If ask is true and dirty is nonzero, it asks
`Do you want to save this lot configuration?` with flag 4.

| Result | Effect |
|---|---|
| `0x5301814C` | Permit the normal save-flag path. |
| `0x53018149` | Close immediately without saving, regardless of save flag. |
| `0x53018147` | Return false immediately; no close notification/removal. |
| Other | Initial permission remains false, but a requested dirty Save can still replace it with Save's result. |

Without an ask, or when clean, permission starts true. On the normal path,
`save && dirty` invokes Save with `(showSuccess=false, showError=true)` and uses
its Boolean result. False leaves the window open. True notifies the chooser,
if present, then calls parent virtual `+0x40` with this editor to remove it.
This does not itself undo edits. A later Shutdown independently notifies a
still-held chooser again; this body does not clear that reference on Close.

## 9. Object edits and bounds *[compat]*

Record type byte values are 0 building, 1 prop, 2 base texture, 4 flora and
7 network node in the verified menu/add branches. Geometry uses binary32
center `(x=+0x04, y=+0x08, z=+0x0C)` and bounds
`xmin=+0x10, zmin=+0x14, xmax=+0x18, zmax=+0x1C`. Lot extents are unsigned
dimension bytes multiplied by 16. The rules below concern finite input values;
NaN/unordered x87 branches remain LE3.

### 9.1 Translation, rotation and mirroring

The key-based MoveObject (`0x00485040`, 353 bytes) starts with step 0.25;
modifier bit 1 changes it to 0.5, bit 0 multiplies it by 4, then bit 2
multiplies it by 8. The modifier meanings are assigned by the key handler,
not guessed here. Native operations spill the step to binary32 after each
multiply (`0x00485048..0x0048508B`, multiplier words at `0x00A8E178/17C`).
Left subtracts from x and both x bounds; Right adds. Up subtracts from z and
both z bounds; Down adds. With Space held, Up/Down instead add/subtract y
**only for props**; non-props still reach the post-edit processing.

The position-based MoveObject (`0x004851B0`, 119 bytes) computes dx=requested
x−old x and dz=requested z−old z, adds them to the corresponding center and
bound coordinates, then runs the same post-edit processing. Both overloads
call the boundary mover with `allowPartial=(type != flora)`, set dirty, update
the title, draw the current configuration and invoke window virtual `+0x170`.
The key overload reaches that processing even for an unsupported key or a
Space+Up/Down non-prop which did not change coordinates.

RotateObject (`0x00484D40`, 130 bytes) increments rotation byte `+2` modulo 4.
Using pre-rotation bounds, define hZ=(zmin−zmax)×0.5 and hX=(xmin−xmax)×0.5.
It writes xmax=x−hZ, zmax=z−hX, xmin=x+hZ, zmin=z+hX, in that order, then
sets dirty, updates title, draws and invokes `+0x170`. The native code keeps
hZ in the x87 register stack but spills hX to binary32 before the z writes
(`0x00484D5F..0x00484D9E`, half constant bits `0x3F000000`). There is no boundary
mover call in this method. MirrorObject (`0x00484DD0`, 58 bytes) acts only on
type 2: byte `+3` becomes 1 if zero, otherwise 0, then the same dirty/draw
sequence. Other types return without mutation.

### 9.2 Boundary mover and save verification

MoveObjectToWithinLotBounds (`0x0047B480`, 443 bytes) uses epsilon equal to
binary32 bits `0x3D4CCCCD` (approximately 0.05, constant `0x00A8CF94`).
With allowPartial false, in order it translates the complete x triple until
the center reaches epsilon if below it; translates the z triple similarly;
translates x back if the center exceeds width−epsilon; then translates z back
if its center exceeds depth−epsilon. With allowPartial true the same sequence
tests xmax/zmax against epsilon and xmin/zmin against extent−epsilon instead.
Thus the latter mode requires intersection with the lot, not full containment.
Each translation preserves the relative center/bounds offsets subject to the
individual binary32 stores; the delta remains on the x87 stack. Finally, a
negative x center alone is set to zero without adjusting its bounds. There is
no matching final z-center clamp. Width/depth are converted from unsigned bytes
shifted left four (`0x0047B489..0x0047B4AD`); the two operation sequences and
final x-only clamp are `0x0047B4B7..0x0047B63A`.

The predicate helper `0x0047B640` (185 attributed bytes) has no epsilon. In
full-containment mode require xmin,zmin≥0 and xmax≤width,zmax≤depth; in partial
mode require xmax,zmax≥0 and xmin≤width,zmin≤depth. Equality is allowed for
finite values. It does not move anything.

VerifyObjectsAreWithinLotBounds (`0x0047D6B0`, 212 bytes) checks only types
0, 4 and 1, in vector order, using that predicate and its supplied mode. Empty
vectors and vectors containing only other types pass. The first failure either
returns false silently or, when prompting is enabled, asks whether to continue
or cancel. Only result `0x5301814A` permits continuation. No record is repaired
by this verification. Save supplies partial mode true and prompting true.

RemoveOutOfBoundsLotConfigurationObjects (`0x00484330`, 494 bytes) removes
types 0/4/1 only when their rectangles fail the inclusive intersection test;
partially overlapping objects remain. Erasure preserves remaining order and
continues at the erased index. It also filters a separate packed fence vector,
then selects the first record or null and redraws/invalidates. Fence-coordinate
signedness, edge equality and dirty effects remain LE3; do not use this partial
contract as a fence validator.

### 9.3 Addition, duplication and deletion: established subset

AddNewLotConfigurationObject (`0x00484850`, 1,236 bytes) switches on placement
mode. Modes 1/2/3 create types 0/4/1 from the pending-ID vector and do nothing
if that vector is empty. Mode 4 creates type 7 with rotation 2 and a network
payload whose meaningful initial words include 0, 0, `0x02000000` and 0;
the exact allocation/size reconciliation is LE3. Mode 5 creates type 2 and
requires IDs. Modes 6/7 create types 5/6 from the pending vector without that
nonempty precondition. Other modes do nothing. New objects start at the scaled
click position, y=0, flags/mirroring zero, and a clock-derived creation word.
Type 7 starts rotation 2; others start zero. Non-building/flora/prop bounds
start at center ±8. Family-based bounds come from manager slots `+0x60/+0x64/
+0x68`. The editor boundary-moves the result, inserts it into the record vector,
selects the first vector record, frees the temporary record/payload, sets dirty,
updates title, draws and invalidates. Texture insertion order and failed
allocation paths remain LE3.

DuplicateObject (`0x00484E10`, 557 bytes) accepts only types 0,1,2,4; it copies
the object, modifies placement through helper `0x0047BB40`, inserts at the
vector front, selects the first record, sets dirty, updates title, draws and
invalidates. The helper's offset sequence is LE3, so no exact duplicate
coordinates are specified here.

DeleteLotConfigurationObject (`0x00480170`, 411 bytes) optionally asks
`Are you sure you want to delete this %s?` with flag 3. Result `0x53018149`
aborts. Otherwise it searches by pointer identity and erases the matching
record, destroying the vacated tail payload. If the selected pointer equals
the requested pointer it clears that pointer; it does not reset selected index
or drag/capture in this method. It draws and invalidates even when the pointer
was absent. **This method does not set dirty or update the title.** Do not
silently repair that omission in the compatibility contract.

## 10. Additional independent conformance vectors

| Initial state / input | Action | Expected result |
|---|---|---|
| Clean editor, no configuration. | Construct. | Null selection, scale 1.0, empty pending IDs, dirty false; no configuration loaded. |
| Old configuration A; successful read B, failed identity lookup. | Open B. | Returns false with B still installed; later open steps do not execute. |
| Dirty existing identity; valid bounds; persistence false; showError false. | Save. | Returns false, keeps dirty, does not show error or send success notification. |
| Successful Save; showSuccess false. | Save. | Dirty clears, title updates, success notification still occurs, no success box. |
| Dirty; ask/save both true; prompt result `0x53018149`. | Close. | Skips Save, notifies chooser, removes editor and returns true. |
| Dirty; ask false, save true; Save As cancelled. | Close. | Save returns false; editor remains open and chooser is not notified by Close. |
| Successful Save a Copy As from dirty original. | Save copy. | Original identity/ID restored, saved copy registered/chooser refreshed, dirty remains cleared by inner Save. |
| Step modifier masks 0,1,2,3,4,7. | Move Right, away from bounds. | x and both x bounds increase by 0.25,1,0.5,2,2,16 respectively. |
| Non-prop selected; Space held. | Key Move Up. | No y/z change, but boundary pass/dirty/title/draw/invalidating postlude still runs. |
| Rect xmin=−2,xmax=2,zmin=1,zmax=3 in 16×16 lot. | Verify partial/full modes separately, no prompt. | Partial passes, full fails. |
| Same partially overlapping rectangle. | Post-open cleanup. | Object remains; cleanup is based on intersection. |
| x center 5,z center 6; bounds x=[3,7],z=[5,7]; rotation 3. | Rotate. | Rotation 0; x=[4,6],z=[4,8]; dirty/title/draw sequence runs. |
| Dirty false; selected object A with index n. | Delete A without prompt. | A erased, selected pointer null, index n retained, dirty still false; draw/invalidating calls run. |
| Type 5 selected, dirty false. | Mirror. | No mutation, dirty/title/draw untouched. |

## 11. Keyboard and edit-mouse state: established subset *[compat]*

`GZOnKeyDown` (`0x00485710`, 717 bytes) returns true for every key, including
unrecognised keys. While this editor is capturing the mouse it performs none
of its key actions. Otherwise it clears the drag byte before dispatch.
Modifiers below are raw mask bits; the Save labels corroborate their usual
Shift/Control/Alt meanings, but no modifier-name ABI is imposed here.

| Key | Action when not capturing |
|---|---|
| Escape | Placement mode zero; install normal cursor. Does not close or discard configuration edits. |
| C | Increment condition byte; when the new byte is exactly 7 reset to zero; invoke refresh `+0x27C`. |
| B / T | Add building / base texture selector. |
| P | Prop selector: bit0 set gives `(false,true)`; bit0 clear and bit1 set gives `(true,false)`; otherwise `(true,true)`. |
| F | No bits0/1: flora selector; either bit set: add/edit fences. |
| S | Requires bit1. If bit0 set: ordinary Save As; else bit2 set: Save a Copy As; else Save. Both message options true. Without bit1 no action. |
| G / R / F1 | Go to 3D view / refresh / help page 0. |
| Arrow keys with selection | Pass key and modifier word to key MoveObject. |
| Page Up or Page Down with selection | Call the same RotateObject; both increment rotation. |
| M with selection | MirrorObject (which acts only on textures). |
| D with selection | Duplicate only if bit1 is set. |
| Delete or Backspace with selection | Delete with confirmation enabled. |
| Home with selection | If elevation is nonzero, write zero, set dirty, update title and request redraw; this is not limited to props. |
| N with selection | CycleSelectedObject. |

Recognised global keys precede selected-object keys. For example Escape cancels
placement mode even if an object is selected. Condition cycling assumes normal
state 0..6: it is an equality reset at 7, not a general modulo-7 normalisation.
Save argument pushes are at `0x00485846..0x00485881`; rotation keys share the
one incrementing method specified in §9.1. Selection cycling and selector/
refresh internals remain LE4/LE5.

`DoEditMouseUpL` (`0x0047A890`, 42 bytes) returns true. If the drag byte is set,
it clears it and releases capture only if currently capturing. If drag was
already false it leaves even an existing capture untouched.

`DoEditMouseMove` (`0x004859E0`, 286 bytes) moves the selection only while both
drag and capture are active. It then updates status text for every move:
`%08x: Pixel: (%d,%d) World: (%.1f,%.1f)`, using zero for the timestamp word
when no object is selected and selected record `+0x24` otherwise. World values
are pixel coordinates scaled by `+0x1AC`. The precise drag delta/pixel
conversion and status-interface argument recovery remain LE3/LE5.

`DoEditMouseDownL` (`0x00486770`, 434 bytes) starts by hit-testing. A no-hit
clears an existing selection and redraws/invalidates. Hit code 1 with placement
mode zero does the same. Hit code 2 with mode zero opens the lot-foundation
dialog with the current foundation ID and no building, then clears selection;
with nonzero mode it does nothing. Hit code 3 in mode 8 toggles a fence; other
nonzero modes do nothing for this hit. In mode zero, hit code 3 clears any
selection and requests a redraw. Object codes ≥100 in mode zero select the
returned object and proceed into drag setup. Hit-code assignment, placement
clicks and the remaining drag/capture details remain LE5, not inferred here.

| Initial state / input | Action | Expected result |
|---|---|---|
| Mouse capture active; dirty false. | Press Escape, S or Delete. | Handler returns true without its key actions, including no drag-byte clear. |
| Placement mode nonzero, configuration dirty. | Press Escape without capture. | Placement mode zero, normal cursor; dirty/current configuration preserved. |
| Rotation 0, selected object. | Page Up, then Page Down. | Rotation becomes 1, then 2; Page Down does not undo Page Up. |
| Capture held, drag false. | DoEditMouseUpL. | Returns true and leaves capture held. |
| Capture held, drag true. | DoEditMouseUpL. | Drag clears and capture releases. |
| Condition 6 versus 7 (separate runs). | Press C. | 6 becomes 0; 7 becomes 8. Both invoke refresh. |
