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
its dirty suffix is ` *` (native string bytes at `0x00A8DCC0`). §25.3 gives
the complete title composition and refresh contract.

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

### 7.1 Shutdown ownership and observable order (LE8)

The initialized Shutdown protocol is ordered as follows. Every listed operation
skips a null reference; offsets identify ownership slots, not portable layout
requirements (`0x0047C520..0x0047C648`).

| Order | Reference or collection | Effect |
|---:|---|---|
| 1 | Chooser `+0x15C` | Notify editor closed, while the chooser reference is still held. |
| 2 | Font `+0x130` | Release, then clear slot. |
| 3 | View/service `+0x134`, `+0x138`, `+0x13C`, in that order | Clear each slot before its Release. |
| 4 | Chooser `+0x15C`, then configuration manager `+0x160` | Clear each slot before its Release. |
| 5 | Identity references `+0x188`, then `+0x18C` | Release each, then clear its slot. |
| 6 | Network objects `+0x194`, then `+0x198` | For each: invoke stop/shutdown slot `+0x10`, Release, then clear. |
| 7 | Owned reference array, begin `+0x1D0`, end `+0x1D4` | Release each non-null element in forward order. Do not clear the elements, change the logical end, or free storage here. |
| 8 | Base window | Invoke base Shutdown, including when the editor was not initialized. |

All editor-specific steps are skipped when the base initialized flag is false.
The seven cursor slots `+0x140..+0x158`, the current configuration `+0x190`,
identity path/key/ID and collection storage survive this body. The destructor
in §21.1 subsequently Releases the cursors in reverse slot order. The font is
`+0x130`, not one of `+0x134/+0x138/+0x13C`. The array Release loop leaves
dangling element values until storage destruction; raw destruction alone omits
those Releases. Do not add an extra Release pass or promise idempotence for
an initialized reentrant call. These distinctions follow the native stores
around Release (`0x0047C533..0x0047C5DA`) and the unchanged array loop
(`0x0047C61B..0x0047C648`).

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
dimension bytes multiplied by 16. Unless explicitly extended below, numeric
rules concern finite values. §9.4 establishes unordered cases for containment
and cleanup; movement/hit-testing unordered cases remain LE3.

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
types 0/4/1 when xmax>width, xmin<0, zmax>depth or zmin<0. Equality is allowed.
**Correction to the earlier partial contract:** this is full containment,
not intersection; partially overlapping objects are removed. It examines no
other object types. Erasure preserves remaining order and continues at the
erased index, destroying the vacated tail payload (`0x00484396..0x0048445A`).

It next filters four-byte fence records in vector `+0xB4..+0xB8`. Record bytes
2 and 3 are **signed** coordinates; dimensions remain unsigned. Remove if
signed byte 2 > width or signed byte 3 > depth. There is no lower-bound check;
negative coordinates and exact upper-edge equality survive. It compacts in
place, preserving order. The return count includes both removed objects and
fences. A nonzero count sets dirty and updates title once, after both passes;
zero does neither (`0x00484460..0x00484520`). This body does **not** select a
replacement object, redraw or invalidate, and does not repair selection pointers
invalidated by erasure. Callers must account for those separate operations.

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
updates title, draws and invalidates. §9.4 specifies texture insertion order,
record-allocation failures and the type-7 logical-size discrepancy.

DuplicateObject (`0x00484E10`, 557 bytes) accepts only types 0,1,2,4; it copies
the object, modifies placement using containment helper `0x0047BB40`, inserts at the
vector front, selects the first record, sets dirty, updates title, draws and
invalidates. Relative to the original geometry, it tries the translations
`(+4,+4)`, `(-4,-4)`, `(0,-4)` in that order, stopping on the first containment
success. If all three fail it inserts at `(-8,+4)` **without testing that final
position**. Each pair is `(x,z)` and moves center and both bounds; y is preserved.
The first test is `0x00484EAE`, the second `0x00484F1A`, the third
`0x00484F74`; final untested adjustment/insertion is `0x00484F7D..0x00484FED`.
Each add/subtract spills separately to binary32. Constants at `0x00A8E17C`
and `0x00A8E178` are binary32 4 and 8. These are sequential rounded operations,
not a single final displacement for extreme numeric inputs.

DeleteLotConfigurationObject (`0x00480170`, 411 bytes) optionally asks
`Are you sure you want to delete this %s?` with flag 3. Result `0x53018149`
aborts. Otherwise it searches by pointer identity and erases the matching
record, destroying the vacated tail payload. If the selected pointer equals
the requested pointer it clears that pointer; it does not reset selected index
or drag/capture in this method. It draws and invalidates even when the pointer
was absent. **This method does not set dirty or update the title.** Do not
silently repair that omission in the compatibility contract.

### 9.4 Containment unordered cases and insertion details

Helper `0x0047BB40` constructs the binary32 rectangle `(0,0,16*width,16*depth)`
from unsigned dimension bytes and calls `0x0047B2A0`. The latter requires
object xmax≤right, left≤xmin, zmax≤bottom and top≤zmin for finite values.
Its x87 status tests allow unordered comparison at **each** check
(`0x0047B2BB..0x0047B2F4`: three tests of status mask `0x41` for zero, one
parity test of mask `0x05`). Thus, with x87 invalid exceptions masked, a quiet
NaN coordinate does not itself reject containment; other coordinates can still
fail it. Infinity follows the ordinary ordered comparisons. This helper does
not test centers, type, finite-ness or rectangle orientation.

The cleanup comparisons likewise let unordered values pass their individual
checks (`0x004843B8..0x004843FE`, masks `0x05` and `0x41`). This rule is confined
to these helpers; it does not establish movement, hit-test or conversion rules.

For type 7 addition, the editor resizes the initially empty dword payload to
**three elements**, then stores words at byte offsets 0,4,8,12 as
`0,0,0x02000000,0` (`0x00484BCA..0x00484BEE`). The last store is beyond the
logical end. Do not silently describe it as a four-element payload or a safe
16-byte payload allocation. §25.1 establishes the allocation request and copy
extent; effects on adjacent allocator storage are outside that contract.
All six record-allocation paths set the result to null on
allocation failure, then write its type/fields without a guard; these are not
clean no-op exits (`0x004848BB..0x004848D4`, `0x0048498F..0x004849A8`,
`0x00484A64..0x00484A7D`, `0x00484B39..0x00484B52`,
`0x00484B93..0x00484BB0`, `0x00484C06..0x00484C2E`).

The common default-bounds block stores `x+8` at `+0x10`, `z+8` at `+0x14`,
`x−8` at `+0x18`, `z−8` at `+0x1C` (`0x00484C5D..0x00484C8C`). These are
reversed relative to the min/max field convention above; do not normalise them
when reproducing this block. The boundary-mover option is false for flora
type 4 and true otherwise. Its result is ignored. Type 2 appends at the object
vector's end, through the misleadingly named `0x00479BE0` helper; other types
insert at the front. **Selection always takes the vector's first record**, so
adding a base texture to a nonempty vector selects an older object, not the new
texture (`0x00484C8F..0x00484CDD`, append proof `0x00479BE0..0x00479C1A`).

Constructor helper `0x00478360` is reached directly by these allocations. It
copies shared word `0x00B21670` into x/y/z, sets bounds to `(0,0,1,1)`, and
zeroes the three payload-vector words; it leaves the type/flag bytes and
creation word untouched. This proves a record-construction dependency, not
exclusive editor ownership of this shared helper.

### 9.5 Fence toggles and lot foundations

The record overload of ToggleFence (`0x0047E890`) compares **all four bytes**
against fence records in forward order. On the first exact match it removes
only that record and shifts later records down; otherwise it appends the input
four bytes. Reversed endpoints are distinct. It performs no coordinate,
adjacency or lot-bound validation. In either case it draws the current lot and
calls editor virtual slot `+0x170`; it does not set dirty or update title
(`0x0047E8BE..0x0047E947`, `0x0047E9ED..0x0047EA06`). Capacity growth is
one record from empty, otherwise twice the current record count. Its raw
allocation/copy path has no transactional failure return; do not substitute a
successful no-op on allocation failure (`0x0047E94C..0x0047E9E7`).

The pixel overload (`0x00484700`) consumes x and y as signed 32-bit integers;
its third stack argument is unused (callee discards 12 argument bytes). Let
`s` be editor binary32 `+0x1AC`. First scan vertical grid lines `i=0..width`
in increasing order, with pixel coordinate truncation-toward-zero of
`(16*i) * binary32(1/s)`. The reciprocal is rounded to binary32 before the
loop. The first line within **inclusive ±3 pixels of x** wins. Only if no
vertical line matches, scan horizontal lines `j=0..depth` similarly against y.
There is no nearest-line comparison and no rectangle-origin subtraction.

For a vertical match the fence bytes are `(i,t,i,t+1)`, where t is the low
byte of truncation-toward-zero of `(signed y * s) * 0.0625`; for a horizontal
match they are `(t,j,t+1,j)`, using x instead. Increment wraps within one byte.
The multiply chain stays in x87 until conversion, without a binary32 spill
between the two multiplications. There is no clamping of the other coordinate.
It calls the record overload, then sets dirty and updates title; no match
does none of these operations. Thus its draw/invalidation precedes dirty/title
(`0x00484718..0x00484839`). Constants are binary32 1 at `0x00A81228` and
1/16 at `0x00A8E174`. Conversion is helper `0x009EEF04`, with signed 64-bit
intermediate and low-byte use here. §28 completes invalid/overflow conversion.

LotFoundationIsPresent (`0x0047B420`) returns the `+0x20` ID of the **first**
type-3 object, or zero if none; it is not a Boolean test and a zero-ID first
foundation hides subsequent foundations (`0x0047B455..0x0047B470`).
AddLotFoundation (`0x00484530`) likewise considers only the first type-3 record:

| Existing first type-3 / enable argument | Result |
|---|---|
| None / zero | No change. |
| Present / zero | Remove that record, preserving survivor order and releasing the vacated tail payload. |
| Present with requested ID / nonzero | Return immediately, without replacing or moving it. |
| Present with different ID / nonzero | Remove it, then append a new type-3 record at the end. |
| None / nonzero | Append a new type-3 record. |

The appended record has type byte 3, requested ID at `+0x20`, zero orientation,
flags, position/bounds and payload pointers/counts. The spare dword `+0x24`
is not initialized by this stack construction and is not assigned a zero rule.
Additional type-3 records remain untouched. Removal/replacement does not repair
the selected pointer. This method itself does not set dirty, update title,
draw, invalidate or select (`0x00484570..0x004846FD`). These effects belong to
its callers; adding them here changes same-ID and deletion behavior.

### 9.6 Conversion stores and unordered movement

PixelUnitsToObjectUnits (`0x0047A8F0`) reads each pixel coordinate as signed
32-bit, multiplies by binary32 scale `+0x1AC`, and stores each product to its
binary32 output. It then subtracts rectangle `+4` from the stored x and
rectangle `+0xC` from the stored y, storing each result again to binary32
(`0x0047A8F0..0x0047A920`). The intermediate rounding is observable; replacing
it with one multiply-subtract or subtracting the origin before scaling differs.

With x87 invalid exceptions masked, **every** unordered comparison in
MoveObjectToWithinLotBounds skips its associated translation; the final
x-only negative clamp also skips for unordered x. Each test uses status bits
`0x05` with a parity branch (`0x0047B4C0..0x0047B554`,
`0x0047B56B..0x0047B5FF`, `0x0047B627..0x0047B62E`). This is independent
per test: a NaN x bound in partial mode does not suppress finite z correction,
and a finite tested bound can still trigger arithmetic that propagates NaN
from an untested center/bound. Full mode tests centers; partial mode tests
opposite bounds as in §9.2. No generic NaN rejection or repair occurs.

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
| Same partially overlapping rectangle, type 0/1/4. | Post-open cleanup. | Object is removed; cleanup requires full containment. |
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

## 12. Family and texture selectors: traced contracts *[compat]*

The four selectors share the following native workflow. Each creates the
scripted window, queries its grid as IID `0xDAA6B9BE`, initialises the grid
with `(2,32,0)`, clears rows, populates, then calls grid `+0x20C` with
`(0,0,1,0)` (textures use their third argument for the last word). The meaning
of the grid's last two sort options is not assigned here. Each temporarily
installs editor callback subobject `+0x128`, registers/runs/unregisters the modal
window, restores editor focus and removes the callback. Only result
`0x42B7C356` accepts. Missing window or grid returns false; a missing grid
prevents the modal attempt and still shuts down/Releases the created window.

| Method | Factory selector | Script key (type,group,instance) | Grid ID |
|---|---|---|---|
| Building `0x004827D0` | `0x89264BF9` | `(0,0x96A006B0,0xE9263D4F)` | `0xE9264E71` |
| Flora `0x00482DC0` | `0x89264BFA` | `(0,0x96A006B0,0x89404462)` | `0x69433083` |
| Prop `0x004832E0` | `0x097CEFB0` | `(0,0x96A006B0,0x697CEF4C)` | `0x097CEFB1` |
| Texture `0x0047CC30` | `0x6987B3F8` | `(0,0x96A006B0,0x6987B3F7)` | `0x6987B3F9` |

Column 0 contains `0x%08x` ID text. Building rows come from manager
`0x00B43D68`, family enumeration `+0x34` followed by individual enumeration
`+0x2C`; column 1 is `Family: ` plus the family-name helper's result, or
`Bldg: ` plus the building-name helper's result. Prop rows use manager
`0x00B43D10`: argument 3 enables family enumeration `+0x18`, argument 4 enables
individual enumeration `+0x14`. Column 1 prefixes are ` (F)` and ` (P)`
respectively, followed by the corresponding name-helper result. Flora rows use
manager `0x00B43D80`, families `+0x28` then individuals `+0x20`; family names
are unprefixed helper results, while **every individual name is literal
`<Flora>`**. Texture rows use parallel editor ID/image arrays `+0x1C4` and
`+0x1D0`; column 1 receives the image pointer at the same index through
grid `+0x1C8`. The latter array is indexed without a separate length check.

A non-null second argument supplies preselected IDs. The native search helper
at `0x004696C0` is misleadingly labelled `push_back` in the export: these
call sites pass begin/end/value and compare the returned iterator to end.
Matching IDs select column 0 of the row. Family and individual rows are not
distinguished by a tag in this lookup, so equal IDs can preselect both rows.

Acceptance requests at most 32 selection records, reads the selected row's
**current column-0 text**, parses it with `RZAtoUint32HexA`, and appends each
nonzero result to argument 1's vector. There is no private row-to-ID map, no
output-vector clear, no deduplication and no family/individual tag in output.
Zero is skipped. Empty acceptance returns true with the output unchanged;
cancellation returns false and appends nothing. A null output pointer is not
guarded when a nonzero selected ID is appended. Grid interfaces and temporary
row/enumeration storage are released on normal exit; the window is then shut
down and Released. The selectors themselves do not mark editor data dirty.

Evidence: building `0x0048281C..0x00482DBD`, prop
`0x0048332E..0x004838BC`, flora `0x00482E0F..0x004832D5`, texture
`0x0047CC7B..0x0047CF86`. Name-helper fallbacks and remaining sort-option
semantics remain open; this is not completion of LE4's other dialogs/popups.

### 12.1 Dialog callback and keyboard search

Callback `0x0047DD60` subtracts `0x128` from its callback-this before invoking
editor helpers. Category 3 with command `0x287259F6` dispatches the following
sort controls; it reports handled even if the helper cannot find a grid.

| Controls (column 0 / column 1) | Grid |
|---|---|
| `0x097CEFB2` / `0x097CEFB3` | `0x097CEFB1` |
| `0x69433011` / `0x69433012` | `0x69433083` |
| `0xE9432DD7` / `0xE9432DD8` | `0xE9264E71` |
| `0x6987B3FA` / none | `0x6987B3F9` |

`DoDialogBoxGridSort` (`0x0047ABA0`) locates the clicked control and its parent,
queries that parent's appropriate grid and calls `+0x20C` with
`(column,modifier,1,0)`. Modifier is true if key `0x11` or `0x10` is down.
Missing control/parent/grid ends without sorting. The acquired grid is Released.
These raw option values are established without assuming sort direction or
numeric-vs-text collation.

Otherwise categories 3 and 5 consume message words `0x41..0x5A` as letters;
all other cases return false. `DoDialogBoxScrollToLetter` (`0x0047CA60`)
queries **only prop grid `0x097CEFB1`**, including when called from another
dialog. It scans the inclusive row interval returned by grid `+0xC8`, reading
column 1, and scrolls through `+0x90` to the first row whose first string byte
exactly equals the supplied letter. There is no case folding, substring search
or wraparound in this body. Missing grid/no match causes no scroll; acquired
grid and temporary string are released (`0x0047CA60..0x0047CB65`).

## 13. Rendering: newly established subset *[compat]*

`DrawCurrentLotConfiguration` (`0x0047E060`) obtains surface IID `0x20732180`
from editor reference `+0x138`; acquisition failure skips drawing. Its clear
phase requests colour `(192,192,192)`, locks with `0x8001`, clears through
surface `+0x48`, and unlocks only after a successful lock
(`0x0047E066..0x0047E0D2`). Objects are drawn in **reverse vector order**:
last record first, first record last. Fence drawing follows the object pass
(`0x0047E32D..0x0047E394`). This makes insertion order observable.

`DrawLotConfigurationObject` (`0x0047CF90`) gives type 3 a separate marker
path: colour `(0,72,72)`, lock `0x8400`, two calls to marker helper
`0x008DA370`, then unlock and return. Failed lock skips the calls and unlock.
It bypasses the ordinary object/height-label path (`0x0047CF98..0x0047D01B`).

For the ordinary path, the height-label gate at `0x0047D527..0x0047D53B`
skips height equal to zero, including negative zero. An unordered height passes
the gate (x87 invalid exceptions masked). It promotes the binary32 height to
double and formats literal `+%.1f`. For ordered negative heights it replaces
the first character with `-`; thus −1.0 produces **`--1.0`**, preserving the
minus already formatted by `%f`. Unordered height retains the literal leading
plus; the CRT's NaN spelling is not specified here
(`0x0047D56C..0x0047D5B0`, parity masks `0x44` then `0x05`). Text layout,
normal-object colour/alpha/outline, coordinate rounding and other draw phases
remain LE5.

## 14. Discriminating vectors for the newly traced contracts

These are primary-evidence expectations, not outputs of an implementation.

| Initial state / action | Expected distinction |
|---|---|
| Compatibility vector has only bit 0 and bit 20; accept no rows. | Both bits clear, not merely the displayed bit range. |
| Select zone rows 0 and 14, wealth row 2, purpose row 7. | Resulting set bits are zone 1/15, wealth 3, purpose 8. |
| Change only control `0x294053DB`, then accept otherwise unchanged dialog. | Its model source is unchanged; dirty/title still update. |
| Missing second compatibility grid; accept after selecting a first-grid row. | First vector can change before the null second-grid dereference. Cancel avoids write-back. |
| Two providers share a lowercased directory; first fails key derivation, second succeeds. | Second provider can supply the saved identity; no unconditional false return. |
| Key lookup and QI succeed, path query fails. | Key helper still returns true and retains identity references. |
| Output IDs initially `[0x10]`; selector returns two rows with ID `0x20` and one zero-ID row. | Output becomes `[0x10,0x20,0x20]`; no clear/dedup and zero is suppressed. |
| Prop grid with both enumeration flags false; accept empty selection. | True with no appended IDs. |
| Lot 16×16, type-0 bounds `[0,16]` on both axes; duplicate. | First three offsets fail. Final unchecked bounds are x `[−8,8]`, z `[4,20]`. |
| Type 0, finite in-range bounds except quiet-NaN xmax; masked x87 exceptions. | NaN alone does not reject containment or cause cleanup removal. A finite out-of-range z bound still can. |
| Lot dimensions 1×1; fence coordinates `(0xFF,1)`, `(1,1)`, `(2,1)`. | First two survive, third is removed; combined removal count/dirty reflect the one removal. |
| Nonempty object vector `[A,B]`; add base texture T. | Vector `[A,B,T]`, selected A, draw order T/B/A. |
| Height 0, −0, +1, −1 in separate ordinary draw calls. | No label, no label, `+1.0`, `--1.0` respectively. |

## 15. Final-round discriminating vectors

These expectations derive from native branches and stores, independently of
any implementation or test output.

| Initial state / action | Expected distinction |
|---|---|
| Fence vector `[A,B,A]`; toggle exact A by record overload. | `[B,A]`; one deletion, draw/invalidate, no dirty/title write. |
| Vector contains `(0,0,0,1)`; toggle `(0,1,0,0)`. | Reversed record appends; endpoints are not normalized. |
| Scale 1, nonempty lot; pixel toggle x=1,y=1. | Vertical priority creates `(0,0,0,1)`, not a horizontal segment. |
| Scale 4, width≥1; pixel toggle x=3,y=8. | Both vertical lines 0 and 1 are within tolerance; line 0 wins although line 1 is nearer. |
| Scale 1; pixel toggle x=0,y=−16. | Bytes `(0,255,0,0)`; truncation then byte wrapping, no lower-bound clamp. |
| Type-3 records A(ID 0), B(ID 7); query presence, then enable ID 0. | Query returns 0; enable returns without mutation and B remains. |
| Objects `[A(type3,ID1),X,B(type3,ID2)]`; enable ID3. | `[X,B,new(type3,ID3)]`, with no dirty/title/draw by this helper. Disable once then removes B only. |
| Partial bounds mover; both x bounds quiet NaN, finite z bounds wholly below zero; masked exceptions. | Both x translations skip; z is corrected. No whole-record rejection. |
| Pixel conversion x=16777217, scale 1, rectangle x-origin=16777216. | x output 0 after product spill, rather than 1 from a combined expression (round-to-nearest). |

## 16. Other dialogs: final-round contracts *[compat]*

The name, slope and network-path dialogs use script resource group
`0x96A006B0`, instance `0xE9263D4C`, creation argument `0xC9264BE2`.
Their text control is `0x89264D48`, queried as interface `0x231A1C57`.
They show modally, unregister the dialog, restore editor focus, and write back
only for modal result `0x42B7C356` and a successful second control query.
Missing initial text control merely skips initialization; it does not cancel
the modal dialog. Failed creation, cancellation and missing accepted control
do not mutate the model. Created dialogs are destroyed/released on exit.
This paragraph describes successful ordinary resource/control lifetime; it
does not promise recovery from allocation faults in string construction.

### 16.1 Lot name, size and network path

DoSetLotNameUI (`0x00480770`) initializes the text from configuration `+0x0C`.
Accepted empty text becomes literal `<unnamed>`; other text, including whitespace,
is copied as read. It assigns the name even when unchanged, sets dirty, and
calls SetTitle **twice**, without drawing. It returns true on accepted modal
result even if the second text-control query fails, and false on creation
failure/cancel (`0x004807D1..0x004808F2`).

DoSetLotSizeUI (`0x00486BA0`) uses group `0x96A006B0`, instance
`0xE9263DE5`, creation argument `0x8926EEBE`. Width control `0x0926ECD9`
and depth control `0xE926ED2E` use interface `0x412CE496`. Each successful
initial query sets its selection to unsigned stored dimension minus one.
Missing initial controls do not prevent showing the dialog. On acceptance,
both output dimensions start at 1. A successful accepted query replaces that
dimension with control selection plus one, using 32-bit arithmetic; there is
no local range validation. It invokes SetLotSize (`0x00486080`) with these
width/depth values and final argument true. Accepted result returns true even
when either/both controls are missing; cancellation/creation failure returns
false (`0x00486BFD..0x00486D2D`). SetLotSize's remaining scale/rectangle
arithmetic is still LE4 and is not inferred from these UI selections.

DoSetNetworkNodePathUI (`0x0047E650`) takes an object record and titles its
dialog `Set path ID for connection`. Initial text comes from payload dword 3
when logical size is at least four, otherwise from zero; it does not read a
physical fourth word merely because spare capacity exists. Acceptance reads
the text through control slot `+0xB4`, parses with RZAtoUint32HexA
(`0x0090FCEF`), grows a short payload to four zero-filled dwords, and writes
only word 3. Payloads longer than four keep their length and other words.
It does not set dirty, update title, recompute bounds, draw or invalidate
(`0x0047E6DB..0x0047E711`, `0x0047E7D7..0x0047E856`). Hex-parser lexical
edge cases remain that helper's contract, not a decimal-only validation rule.

### 16.2 Slope and fence options

DoSet3DViewSlopeUI (`0x0047DDF0`) titles its dialog `Set Slope of Displayed Lot`
and initializes text with signed `%d` from the bits at editor `+0x1E0` (§24.3).
Acceptance parses using RZConvertToUint32A (`0x0090FE93`). For comparison it
loads the signed 32-bit result into x87 and, if the sign bit was set, adds
binary32 2^32; no binary32 store intervenes before the lot-limit tests.
In order, a value greater than configuration float `+0x30`, or less than
configuration float `+0x34`, produces `Slope is outside min/max values for this
lot`. Equality passes. With masked invalid exceptions an unordered upper or
lower comparison passes that individual limit (status tests `0x41` and
`0x05` respectively, `0x0047DF8B..0x0047DFAF`).

Only after those tests does unsigned integer value greater than 80 produce
`Slope limited to 80 degrees`. Otherwise it stores the integer at `+0x1E0`,
calls Prepare3DView, then editor virtual `+0x27C` (`0x0047DFB1..0x0047DFEF`).
There is no dirty/title write or implicit clamp. The lot-limit message wins
when a value violates both a lot limit and the 80-degree cap.

DoFenceMenu (`0x004805B0`) has rows 0 `Set Fence Type...`, 1 separator,
2 `Help... (F1)`. After the popup it restores editor focus. Row 0 displays the
native message that fence options are unimplemented until fence functionality
is in place; it does not open a selector or change fence type. Row 2 invokes
help page 0. Separator/cancel/other results do nothing. Menu temporaries are
released (`0x004806DF..0x0048076E`). This is a native placeholder, not an
untraced fence-type dialog that an implementer must invent.

### 16.3 Discriminating vectors

| Action / state | Native expectation |
|---|---|
| Accept empty lot-name text. | Name `<unnamed>`, dirty set, title called twice. |
| Accept name dialog but accepted control is missing. | True return, no name/dirty/title write. |
| Accept size dialog with missing width control and depth selected index 3. | SetLotSize receives `(1,4,true)`; true UI return. |
| Two-word network payload `[7,8]`; accept path `AB`. | Logical payload `[7,8,0,0xAB]`; no dirty/title/redraw. |
| Five-word payload `[1,2,3,4,5]`; accept path `AB`. | `[1,2,3,0xAB,5]`, still five words. |
| Lot upper slope 75, lower 0; accept 90. | Lot-limit message, not 80-degree message; slope unchanged. |
| Lot limits both quiet NaN, masked exceptions; accept 80 then 81. | 80 stores/prepares/refreshes; 81 shows 80-degree message. |
| Fence popup row 0. | Unimplemented-options message; no fence mutation. |

## 17. Additional input/resource helpers: established subset *[compat]*

DoAddNewNetworkNode (`0x0047B0A0`) switches editor mode `+0x19C` to 4 and
installs stored cursor `+0x150` through virtual `+0x154` with true. If already
4 it returns without even resetting the cursor. DoAddEditFences (`0x0047B0D0`)
does the same for mode 8 and cursor `+0x158`. Neither creates a record or sets
dirty. Switching modes therefore does not itself require a placement click.

LoadCursors (`0x0047AE20`) constructs seven cursor temporaries from group
`0x46A006B0`. The stored-field mapping is:

| Editor field | Resource instance |
|---|---|
| `+0x140` | `0x4A5CDC3F` |
| `+0x144` | `0x4A5CDC3A` |
| `+0x148` | `0x4A5CDC3B` |
| `+0x14C` | `0x4A5CDC3C` |
| `+0x150` | `0x4A5CDC40` |
| `+0x154` | `0x4A5CDC3D` |
| `+0x158` | `0x4A5CDC3E` |

Each replacement is skipped on pointer equality; otherwise it AddRefs the
new non-null pointer, stores it, and Releases the previous non-null pointer,
in field order `140,144,148,14C,158,154,150`. Null construction results can
clear previously stored cursors. All seven temporaries are then destroyed in
reverse construction order. Resource construction failure is not reported as
a false return or rollback by this void method.

The condition-cycle helper (`0x0047A8C0`) increments only byte `+0x1DC`, wraps
to zero **only when the incremented byte equals 7**, then tail-calls editor
virtual `+0x27C`. This is not general modulo seven for corrupt initial values:
7 advances to 8, 255 advances to 0. It has no dirty/title/Prepare3DView call.

The drawing-style helper `0x0047A930` always writes binary32 0.5 to its final
output, then three output bytes according to the record's first byte. In
argument order the byte triples are type 0 `(0,0,128)`, type 1 `(128,0,128)`,
type 2 `(0,128,128)`, type 3 `(0,72,72)`, type 4 `(0,128,0)`, and all other
types `(128,128,128)` (`0x0047A938..0x0047A9DC`). These are output-channel
values, without inferring an undocumented pixel-packing convention.

### 17.1 Hit-test priority and popup: established subset

DoHitTest (`0x00483DC0`) first nulls the caller's output record pointer. Only
input flag bit `0x02` affects the selected-object bypass and overlap popup.
With it clear, a non-null selected object whose inclusive integer rectangle
contains the point wins immediately, yielding selected pointer and stored
selected index plus 100 (`0x00483DD6..0x00483E2A`). It need not be the first
object in the configuration vector. This selected rectangle comes from
`0x0047B310`; its remaining conversion details are not supplied here.

Otherwise the method scans all object records in **forward vector order**,
collecting indices whose integer pixel rectangles inclusively contain x/y.
It does not filter by type or height. If flag `0x02` is clear, or only one
candidate exists, the first candidate returns pointer and index plus 100.
Thus drawing's reverse order (§13) must not be substituted for hit-test order.
With that flag set and multiple candidates, it builds a popup in candidate
order, labelled by index/type. An unsigned returned row below candidate count
selects that candidate and restores focus; any other result returns zero with
null output, without continuing to grid/lot fallback tests
(`0x004840B1..0x00484181`, `0x004842F1..0x00484327`).

Object pixel bounds are obtained through `_ftol2` integer conversions, **not**
direct floating-point comparisons (`0x00483E60..0x00483EC3`). Consequently the
unordered-skip rules of §9.6 cannot be copied into this hit test. §27–§28 complete
reciprocal precision differences, invalid/overflow conversions, grid/resize/inside
fallback rectangles and helper `0x0047B310`.

Discriminators: with overlapping records `[A,B]`, selected B and flag zero,
B wins; with no selection and flag zero, A wins. With flag `0x02`, the selected
shortcut is bypassed and the multi-object popup appears; cancelling it returns
zero/null rather than the enclosing lot hit. Cycling condition byte 7 yields
8, not 1; switching twice to fence mode installs its cursor only on the first
switch. These expectations come from native branches, not test outputs.

## 18. Final-round stopping boundary

Round 3/3 ends **PARKED**, without complete-owner credit. The exact inventory
companion remains the exhaustive address/byte list: 111 historical attribution
rows, 109 current exported bodies, 52,646 bytes. Against `de5f2185`, every
entry, attributed byte count, current byte count and method label is unchanged.
Only contract status/references changed.

| Focused contract status | Current bodies | Body bytes |
|---|---:|---:|
| C: focused body contract | 44 | 20,550 |
| P: established subset, explicit residual | 20 | 20,334 |
| O: body remains open | 40 | 11,335 |
| R: attribution/shared-helper treatment unresolved | 5 | 427 |
| Total | 109 | 52,646 |

The 65 non-C bodies/32,096 bytes are **not all untraced bytes**: P includes
substantial proved behavior. Conversely C is a focused-body label, not a claim
that its open dependencies are complete. Fifteen entries/4,652 bytes moved
to C this round; `0x00483DC0` and `0x00484700` moved O to P.

The following table records the round-3 stopping boundary. The later §25–§26
addendum resolves further subsets; the current residual questions are LE9–LE18
in [the question file](questions/lot-configuration-editor.md). The historical
body counts below have not been reclassified by that addendum, and no
complete-owner claim follows from retiring the broad LE3/LE4/LE5/LE7 entries.

| Question | Exact unresolved work; use companion rows for the complete method list |
|---|---|
| LE3 | `0x00484850` physical type-7 payload/copy/allocation consequences; remaining coordinate/movement conversions in `0x00485040`, `0x004851B0`, `0x00483DC0`, `0x0047B310`, `0x0047B390`; invalid/overflow `_ftol2` outcomes in pixel fence/hit paths; water/terrain creation helpers `0x0047C8D0`, `0x0047C930`; related helper/dependency contracts. Fence filtering, ordinary toggles, first-foundation operations and bounds-mover unordered tests are resolved. |
| LE4 | Retaining-wall `0x00480910` and foundation `0x00485D60` dialog row/enumeration/filter/preselection/empty/accept/cancel/failure contracts; SetLotSize `0x00486080` scale selection, rectangle sizing, byte truncation and dirty ordering; foundation/outside menus `0x00486930`, `0x00485B00`; selected-object menu `0x00487B30` types 3/5, exact detail markers and edge table `0x00A8E6C8`; selector sort-option meaning/collation, name fallbacks and untraced add/select wrappers. The name, size-dialog wrapper, slope, network-path and placeholder fence-option contracts in §16 are resolved. |
| LE5 | Init/static family resources and PostInit view/alpha setup; destruction/container cleanup; remaining mouse/key/capture, hit-test numeric/fallback and cycling behavior; complete ordinary drawing, title/name helpers, 3D preparation/refresh; shared-helper ownership. §17 resolves cursor mapping/replacement, mode 4/8 switches, condition cycling, style outputs and hit priority only. |

Attribution remains separate from reachability: the export has six direct calls
to record constructor `0x00478360`, all from `0x00484850`, and one direct call
to decoder `0x00604920`, from texture enumeration at `0x0047FDCD`. These call
sets prove editor dependencies, not exclusive source ownership. The R current
bodies are exactly `0x00478360` (64 bytes), `0x00604920` (169),
`0x006049BF` (7), `0x006049C6` (7), `0x006049CD` (180). Historical
`0x006049D4`/`0x00604A67` are interior fragments, not two extra bodies.
No ownership reassignment or byte restoration follows from the callers alone.
No fourth lane round is requested by this stopping inventory.

## 27. Hit-test conversion and fallback geometry (LE9) *[compat]*

This section completes §17.1; its selected/forward/popup priority remains in
force. All pixel coordinates and rectangle members below are signed 32-bit
integers. Let `s` be binary32 editor field `+0x1AC`, and let T denote the
low-32-bit integer conversion in §28. Do not replace these integer containment
tests with floating-point comparisons or reject NaNs before conversion.

The selected-object shortcut obtains its rectangle from `0x0047B310` (§28).
The forward object scan has a different arithmetic path: left is T of min-x
times the x87 reciprocal of s; top is T of min-z times that reciprocal **after
a binary32 store**; right and bottom use a newly computed x87 reciprocal
without that store. The four object fields are binary32 at `+0x18`, `+0x1C`,
`+0x10`, `+0x14` respectively. The products are not stored as binary32 before
T (`0x00483E60..0x00483EC3`). Every comparison is signed and inclusive. There
is no sorting of reversed bounds and no type/height/visibility filter.

When there are no object candidates, obtain lot rectangle `(L,Tp,R,B)` from
`0x0047B390`. The following order applies; every fallback leaves the output
record pointer null (`0x00484181..0x004842A9`):

1. For i from zero through unsigned lot-width byte inclusive, convert
   `(16*i) * binary32(1/s)` with T. Return **3** if x is inclusively within
   converted-coordinate minus 3 through plus 3 and y is at most B.
2. Otherwise scan j from zero through unsigned depth byte inclusive in the
   same manner. Return **3** if y is within inclusive ±3 and x is at most R.
   Neither scan checks the other coordinate's lower bound.
3. Recompute the lot rectangle. Return **2** in either inclusive rectangle
   `(R,Tp,R+6,B+6)` or `(L,B,R+6,B+6)`: right and bottom resize strips,
   extending outwards rather than centred on the edge.
4. Return **1** for inclusive containment in `(L,Tp,R,B)`; otherwise **0**.

The ±3 and +6 operations wrap in 32 bits before signed comparisons. They do
not saturate. Object hits take precedence over all these strips; grid hits
take precedence over resize hits, including on the right/bottom boundary.
Popup cancellation still returns zero immediately, as in §17.1.

Worked cases (no objects, width=2, depth=1, s=0.5, hence rectangle
`(0,0,64,32)`): `(16,16)` returns 1; `(3,-100)` returns 3; `(4,-100)` returns
0; `(-100,3)` returns 3. `(65,20)` returns 3 because it is within three pixels
of the last vertical line; `(68,20)` and `(70,38)` return 2; `(71,38)` returns
0. `(64,32)` returns 3. With zero dimensions and s=0.5, the line at zero
still exists; `(0,-1)` returns 3 and `(4,4)` returns 2.

With masked x87 invalid and all four object bounds NaN, each converted member
is zero (§28): that object can hit `(0,0)`. It does not disappear under the
unordered rules of the bounds mover. With s=NaN and no objects, the converted
lot/grid coordinates are zero; `(0,-100)` returns 3. These conditional cases
require masked exceptions, not an assumption about finite inputs.

Closure mapping: DoHitTest `0x00483DC0` (the LE3/LE5 hit geometry row), together
with the rectangle-helper dependencies in §28. No engine source was consulted.

## 28. Rectangle widths and integer conversion (LE10) *[compat]*

`0x0047B310` writes exactly four signed 32-bit members, in left/top/right/bottom
order, to its caller's 16-byte rectangle. It divides each binary32 object bound
at offsets `+0x18`, `+0x1C`, `+0x10`, `+0x14` by binary32 s separately in x87,
then immediately applies T. No float-result spill, reciprocal substitution,
origin addition, min/max repair or clipping occurs (`0x0047B317..0x0047B377`).

`0x0047B390` also writes 16 bytes. It zero-extends the two lot-dimension bytes
at `+0x20/+0x21`, shifts each left four bits, and converts those integers to
x87. Depth is stored as binary32 (exact for all byte dimensions), while width
stays in x87. It converts, in order, `0/s`, `0/s`, `(16*width)/s`,
`(16*depth)/s`, with no result spills before T. Thus a normal positive scale
gives origin zero; it is not a client-window rectangle and adds no border.
Zero/negative/NaN scales have no validation (`0x0047B396..0x0047B407`).

### 28.1 Shared conversion T and exceptional inputs

The actual helper `_ftol2` at `0x009EEF04` uses an x87 **signed 64-bit** integer
conversion and returns its result in two 32-bit words; these geometry callers
use only the low word, interpreted as signed when compared. For ordinary
in-range inputs it truncates towards zero, including negative fractions.
A result outside signed 32-bit range therefore wraps rather than saturates:
`2147483648` becomes `-2147483648`; `4294967296` becomes zero.

For reproducing the extreme boundary, the helper first rounds a duplicate
of its input to signed 64 bits under the current x87 rounding control. It
then corrects a rounded result by one towards zero if required. There is a
special path when the low word is zero and the high word has no bits except
possibly its sign bit: it returns those words without correction. This
includes zero and the signed-64 indefinite value. It does not load a new
control word (`0x009EEF0D..0x009EEF78`).

With x87 invalid masked, NaN, either infinity, or a conversion outside signed
64-bit range produces indefinite `0x8000000000000000`; this special path
leaves the **low word zero**, not `0x80000000`. Invalid status is not cleared.
Consequently both rectangle helpers write zero for each such result. With
invalid unmasked there is no promised normal return; observing the application's
exception handler/control word is only needed to select between this masked
case and a trap, not to determine either helper's instruction-level behavior.
Divide-by-zero and invalid in the preceding floating operations are likewise
subject to the caller's x87 masks.

Pixel ToggleFence uses the same T for grid lines, then uses its low byte for
the other endpoint (§9.5). Under masked invalid, a NaN scale converts every
grid line to zero; a click at x=0 chooses vertical line i=0, its segment index
also converts to zero, and the toggled bytes are `(0,0,0,1)`. There is no
implicit invalid-input no-op. Endpoint increments still wrap within a byte.

Worked rectangle cases: s=0.5 and bounds `(-0.75,-0.25,1.25,2.75)` yield
`(-1,0,2,5)`. With s=1 and all four bounds equal to `2147483648`, the rectangle
has four `-2147483648` members; with all four infinite and exceptions masked,
it has four zero members. Width=2, depth=3 and s=0.5 yield `(0,0,64,96)`;
negative s gives negative right/bottom without normalization. Zero dimensions
and nonzero finite s give `(0,0,0,0)`.

Closure mapping: `0x0047B310`, `0x0047B390`, and the invalid-conversion
dependency of pixel ToggleFence `0x00484700` (LE3/LE5 geometry rows).

## 29. Retaining-wall and foundation selectors (LE11) *[compat]*

Both bodies create script resource `(0,0x96A006B0,0xE9263D4E)` with creation
argument `0xA9264C0E`, then request child `0xE9264E71`, IID `0xDAA6B9BE`.
A missing window returns false. A failed child-interface request shuts down
and releases the window and returns false, without showing a modal dialog or
changing the lot. The successful path clears/configures the grid, registers
the window with the modal service, runs it, unregisters it and restores editor
focus **before** interpreting the result. Only result `0x42B7C356` accepts.
Cancel/other results return false without lot writes. The acquired grid is
released, enumeration storage freed, and the window shut down/released on
completion (`0x00480916..0x004809B9`, `0x00480B41..0x00480D0C`,
`0x00485D63..0x00485E01`, `0x00485F5D..0x0048607E`).

| Contract | Retaining wall `0x00480910` | Foundation `0x00485D60` |
|---|---|---|
| Grid configuration | 2 columns, selection capacity 32, final option 0 | 2 columns, selection capacity 1, final option 0 |
| Enumeration source | Global foundation/wall manager `+0x6C` into an ID vector | Same manager `+0x54` into an ID vector |
| Row order/filter | Every enumerated ID, in returned order; no local filter, deduplication, synthetic None row, or sort | Same; implementation-type output from the name helper is not a filter |
| Column 0 | ID formatted `0x%08x`, eight lower-case hex digits | Same |
| Column 1 | Retaining-wall name helper output | Foundation name helper output |
| Preselection | Select every row whose ID occurs in configuration ID vector `+0x9C` | Select the row matching the supplied current ID (first argument); no auto-selection when absent |
| OK result | True, even with no rows selected or no effective change | Same |

Wall OK constructs a fresh ID vector. When the grid reports selections, it
requests at most 32 selected-cell pairs and reads column zero at each returned
row, in the grid's returned order. Parse each as `RZAtoUint32HexA`; append only
nonzero values. There is no second deduplication/sort. Assign this vector to
configuration `+0x9C` even if it is empty, then dirty=1, SetTitle, DrawCurrent,
invalidate, in order (`0x00480B81..0x00480CBD`). It is a replacement, not an
incremental toggle of the preselection.

Foundation OK begins with result ID zero. If a cell is selected, obtain the
selected cell coordinates and parse column zero of that row. Unlike walls,
zero is retained. With a non-null second argument, write the result directly
to that record's `+0x20` (no type check, bounds update, removal, or selection
repair). With a null second argument, call AddLotFoundation(result ID, true).
Then dirty=1, SetTitle, DrawCurrent, invalidate, even if the ID did not change
or the helper's same-ID path was a no-op (`0x00485F9D..0x00486043`).

Worked cases: enumerated wall IDs `[0x10,0x20,0x30]` and stored IDs
`[0x30,0x10]` preselect rows 0 and 2. OK with returned selected rows `[2,0]`
stores `[0x30,0x10]`; OK with no selection stores `[]` and still dirties and
refreshes. Cancel preserves the original list. For a building record with
foundation ID `0x20`, empty-selection foundation OK writes zero and refreshes;
the same operation with null record asks the first-foundation helper to
install ID zero rather than invoking its disable/remove operation.

Closure mapping: DoSetRetainingWallUI `0x00480910` and DoSetFoundationUI
`0x00485D60`, the two LE4 selector-body rows. Name resolution is §35; the
foundation popup's dispatch/result-ID distinction remains §26.2.

## 30. SetLotSize arithmetic and refresh (LE12) *[compat]*

SetLotSize `0x00486080` accepts two **unsigned 32-bit** dimensions and a byte
option. With option nonzero, return immediately if both full arguments equal
the current zero-extended dimension bytes. Otherwise remember whether either
full argument is unsigned-less than its old byte and whether either differs.
Store the **low byte** of each argument in the lot. If the less-than condition
was true, invoke RemoveOutOfBoundsLotConfigurationObjects **after** those
stores. No range validation or minimum dimension is imposed
(`0x00486080..0x00486101`). A wrapped byte becoming smaller does not itself
cause cleanup if the full argument was larger.

Obtain the edit control's underlying window width and height, subtract 7 from
each using 32-bit integer arithmetic, convert each signed result to binary32:
call these W and H. Separately shift each full unsigned dimension left four
bits, with 32-bit wrap, convert the resulting unsigned integer to binary32:
call these X and Y. The unsigned conversion is signed conversion plus binary32
`4294967296` when the shifted sign bit is set (`0x0048611C..0x00486183`).
It uses the full arguments, not the newly truncated lot bytes.

Compute viewport aspect A=W/H and lot aspect B=X/Y in x87, without a binary32
store of either ratio. Choose the new scale as follows, then store the chosen
quotient to binary32 editor `+0x1AC` (`0x00486187..0x004861CC`):

| First comparison | Choose X/W only when | Otherwise |
|---|---|---|
| A is ordered and greater than 1 | B is ordered and greater than A | Y/H |
| A is at most 1, or unordered | B is ordered and less than A | Y/H |

This is the native aspect-dependent branch, including equality; it is not
a universal maximum or minimum of the two scales. Zero/negative viewport
space, zero dimensions, infinities and unordered ratios are not rejected.
An unordered B-versus-A comparison always chooses Y/H. Floating exceptions
follow the current masks as in §28.

Next request the containing window by ID `0x6946DA25`, IID `0xC2AFA76F`.
If acquired, read the edit and containing-window integer rectangles. Preserve
the containing rectangle's left/top; let D be twice (edit-left minus
containing-left), with 32-bit wrap. Set its right to containing-left + D + 7
+ T(X/s), and bottom to containing-top + D + 7 + T(Y/s). T is §28, and the
reciprocal stays in x87 across these two products. **The horizontal margin
is reused for height**; the edit-top offset is not used. Apply the rectangle
and release the acquired window. A failed request skips only this resizing
(`0x004861D2..0x004862AA`).

Finally, if the option is nonzero and the full dimensions differed, dirty=1
then SetTitle. Always DrawCurrent then invalidate on the non-early-return
path. Option zero therefore forces recalculation/redraw even with equal
dimensions, but suppresses this final dirty/title pair (cleanup's own effects
still apply). No selection reset is added (`0x004862AE..0x004862E5`).

Worked cases: edit window 135×71 gives W=128,H=64. Request 2×1: X=32,Y=16,
A=B=2, so s=0.25. With edit-left=4 and containing rectangle initially at
`(0,0,...)`, D=8 and the resulting rectangle is `(0,0,143,79)`.
A 71×135 edit window instead gives A=0.5; the same request chooses s=0.125
and a **256×128** lot pixel extent, despite the 64-pixel available width.
Changing old 4×1 to full arguments 258×1 stores 2×1 but does not run shrink
cleanup; X is 4128. With positive H and a 0×0 request, B is unordered,
the selected scale is zero, and later masked-invalid extent conversions
yield zero (§28), rather than silently forcing a one-tile lot.

Closure mapping: SetLotSize `0x00486080` and the arithmetic dependency of
DoSetLotSizeUI `0x00486BA0` (LE4); the wrapper itself remains §16.1.

## 31. Selected-object popup remainder and add wrappers (LE13) *[compat]*

This completes §3's remainder, separating dispatch from §29's dialogs.
Type 3 offers row 0 `---`, row 1 `Help... (F1)`; only row 1 invokes help with
page 3 (`0x00488AC9..0x00488B39`). It does not invoke the foundation selector.
Type 5 offers row 0 `---`, row 1 `Docking constraint off` if its payload vector
is empty or word zero is zero, otherwise `Docking constraint on`. Row 1 ensures
one zero word exists when empty, then replaces word zero by 1 when zero, 0
otherwise. No other payload word changes. This branch has no dirty, title,
draw, invalidate or bounds-update call. Cancel, separator and unknown choices
do nothing. Both branches restore focus after the popup before dispatch.

For props, base textures and flora, the three labels are exactly `Enable in
Low City Detail`, `Enable in Medium City Detail`, `Enable in High City Detail`,
with suffix ` <-- Current` on the following rows only:

| Existing flag byte AND `0x30` | Current suffix |
|---|---|
| `0x00` | Low only |
| `0x10` | Medium only |
| `0x20` | High only |
| `0x30` | None |

The jumps into shared label-construction blocks do not produce two markers
(`0x00488380..0x00488439`; corresponding flora/base-texture blocks in the same
body). §3's write rules still apply; labels do not broaden their side effects.

The network-edge table is 20 consecutive 8-byte key/name pairs beginning at
`0x00A8E6C8`. Each submenu row is formatted `%08lx: %s`; indices outside 0–19
do not write. The selected key replaces payload word 1 for minimal edges,
word 2 for maximal edges. Exact native labels and keys are:

| Row | Key | Label |
|---:|---|---|
| 0 | `00000000` | parking lot |
| 1 | `00000200` | south |
| 2 | `02000000` | north |
| 3 | `00000002` | east |
| 4 | `00020000` | west |
| 5 | `00020200` | south-west 90 |
| 6 | `02020000` | north-west 90 |
| 7 | `02000002` | north-east 90 |
| 8 | `00000202` | south-east 90 |
| 9 | `02000200` | north-south (vertical) |
| 10 | `00020002` | east-west (horizontal) |
| 11 | `00020202` | SEW tee |
| 12 | `02000202` | NSE tee |
| 13 | `02020002` | NEW tee |
| 14 | `02020200` | NSW tee |
| 15 | `02020202` | 4-way intersection |
| 16 | `02040200` | NS west side (ave/hwy) |
| 17 | `02000204` | NS east side (ave/hwy) |
| 18 | `04020002` | EW north side (ave/hwy) |
| 19 | `00020402` | EW south side (ave/hwy) |

### 31.1 Add-mode and texture-selection wrappers

The four add wrappers do nothing when already in their requested mode.
Otherwise they **clear** the remembered ID vector at editor `+0x1A0` by moving
its end to its begin, then invoke their selector with that vector as output
and a null preselection. They are not copying a saved selection from another
field. Only accepted, nonempty output changes mode and installs the indicated
cursor with option true (`0x00483920..0x00483B11`):

| Wrapper | Selector | Mode / cursor field |
|---|---|---|
| DoAddNewBuilding `0x00483920` | Building families | 1 / `+0x144` |
| DoAddNewFlora `0x004839A0` | Flora families | 2 / `+0x148` |
| DoAddNewProp `0x00483A20` | Prop families; forwards its two inclusion arguments | 3 / `+0x14C` |
| DoAddNewLotBaseTexture `0x00483AA0` | Lot base textures | 5 / `+0x158` |

Cancellation/empty OK leaves mode/cursor unchanged, but the clear has already
happened. No wrapper creates a lot object, dirties or redraws the lot.
DoSelectLotBaseTexturesUI (`0x004838C0`) calls texture enumeration, invokes
DoSelectTexturesUI with its output/preselection arguments and comparator
`0x0047B050`, frees the local temporary vector and returns the selector's
Boolean. Its comparator is specified in §32.

Worked cases: mode 0 with remembered IDs `[0x10]`, cancelling Add Building
leaves mode 0 and remembered IDs empty; accepting `[0x20]` enters mode 1 with
the building cursor. Calling Add Building again in mode 1 shows no dialog and
does not clear `[0x20]`. A type-5 empty payload, row 1, becomes `[1]` without
changing dirty; selecting row 1 again gives `[0]`. Minimal-edge row 18 writes
`0x04020002` to word 1, preserving words 0 and 2 and dirty.

Closure mapping: remaining LE4 portion of DoLotConfigurationObjectMenu
`0x00487B30`, DoSelectLotBaseTexturesUI `0x004838C0`, all four add wrappers
in the table, and comparator `0x0047B050` with §32.

## 32. Grid sort/search and selector names (LE14) *[compat]*

Resolve slots against the **grid IID's primary interface**, not its base-window
subobject: primary table `0x00ADD578` has `+0x20C` = SortColumn `0x009ACED6`
and `+0x214` = text search `0x009A9892`. The exported window-subobject table
`0x00ADD7B0` has unrelated methods at those offsets. Grid QueryInterface
`0x009A5CB5` returns the primary pointer for IID `0xDAA6B9BE`.

### 32.1 Sorting

SortColumn arguments are column, direction, move-whole-rows, comparator.
Direction zero is ascending; nonzero reverses comparator argument order for
descending. A nonzero third argument permutes all columns together, preserving
row association and remapping the selected cells, rather than sorting just
the indicated column's text (`0x009ACED6..0x009AD1E7`). Thus initial selector
`(0,0,1,0)` sorts ascending IDs as text, moving names/images with them; holding
Control or Shift at a selector sort button requests descending order. It is
not a repeated-click direction toggle.

With comparator null, only two text cells (type 2) with non-null text storage
are compared; the result is CRT `__stricmp` on their NUL-terminated byte
strings. Other pairs compare equal. This is case-insensitive text collation,
without numeric parsing/natural-number ordering (`0x009A6E59..0x009A6E8F`).
The direction adapter tests the callback result as signed-less-than zero
(`0x009A8888..0x009A88C3`). Equal text has no ID/row-number secondary key.
For the ASCII ID format of §12, lexicographic and unsigned numerical order
coincide because all IDs have the same prefix and eight-digit width.

Base textures use `0x0047B050`: parse both cell texts as hex, compare
`ID AND 0xF000` first by signed subtraction (range −61440..61440), then compare
the full IDs unsigned. First ID less than second returns −1, greater returns
+1. **Equal IDs also return −1**, not zero (`0x0047B050..0x0047B094`). Do not
invent stable duplicate ordering for this inconsistent equality case. Calling
the ordinary selector sort button with a null comparator subsequently uses
the default textual comparator instead.

### 32.2 Search ranges and equality

Grid `+0x214` takes text pointer, text length, first column, column count,
first row, row count, two output-coordinate pointers, and exact-length byte.
Zero or `0xFFFFFFFF` length means strlen. Lower bounds are inclusive; finite
upper bounds are **exclusive** sums of first+count. Count `0x7FFFFFFF` removes
the corresponding upper bound. The sums wrap in 32 bits, and coordinate tests
are signed. Only populated text cells with storage are candidates. Last byte
nonzero requires equal byte length; all searches require the given bytes to
occur in the cell text through case-sensitive string find. Without the byte,
a substring is sufficient; with it, equal length makes the match exact.
On success write both coordinates and return true; on failure return false
without writing them (`0x009A98EF..0x009A999C`).

Consequently chooser arguments `(text,length,0,1,0,0x7FFFFFFF,...,1)` search
**column zero including row zero**. The 1 is a column count, not a row-start
exclusion. It searches all populated rows, not just after the selection. The
prop letter-scroll helper in §12.1 is a different, case-sensitive first-byte
scan and does not inherit sort's case folding.

### 32.3 Selector-name fallbacks

Family helpers first consult their static FamilyNames.ini map (§33). A hit
supplies its stored string verbatim, including empty strings. If absent:

| Helper | Fallback |
|---|---|
| Building family `0x0047F750` | Ask manager for family members in its returned hash-set traversal order; join at most eight GetBuildingName results with `, `; append `, ...` upon encountering a ninth. No members gives `<Family>`. |
| Prop family `0x0047FA80` | Same eight-name limit and suffix, in manager member-vector order, using GetPropName. No members gives `<Family>`. |
| Flora family `0x0047F950` | Literal `<Flora>`; it does not enumerate members. |

GetBuildingName (`0x0047F6C0`) and GetPropName (`0x0047FA00`) use their manager's
recognized-object/name route; an unrecognized ID gives literal `<Building>`
or `<Prop>` respectively. No resource-name invention or hexadecimal fallback
is inserted. Selector prefixes in §12 are added **outside** these helpers.
Texture selectors have ID text/image cells and no missing-name substitution.
Evidence: `0x0047F6C0..0x0047FC48`; comma and truncation strings at
`0x00A86F20` and `0x00A8DD30`.

Worked cases: default ascending names `["b","A","c"]` become `["A","b","c"]`,
with all row columns attached; direction 1 gives `["c","b","A"]`. Texture IDs
`0x00010000` and `0x00001000` sort in that order because masked groups 0 and
0x1000 precede full-ID comparison. A chooser whose only row is row zero with
`0x0000002a` finds ID 0x2A there; text `0x0000002A` does not match that cell.
Missing family 0x20 with no members displays `Family: <Family>` in a building
selector, ` (F)<Family>` in a prop selector, and `<Flora>` in a flora selector.

Closure mapping: shared semantic dependencies of DoDialogBoxGridSort
`0x0047ABA0`, selectors `0x004827D0/0x004832E0/0x00482DC0/0x0047CC30`, and
chooser SelectLotConfigurationType/PopulateGrid; comparator `0x0047B050` and
the five family/individual name-helper rows above. For non-ASCII collation,
the contract is the linked CRT's current-locale `__stricmp`; an installation-
specific observation would record its active LC_CTYPE and compare the desired
byte strings. No locale-independent Unicode ordering is asserted.

## 33. Family resources and PostInit (LE15) *[compat]*

StaticInit `0x00486F20` sets the process-once byte **before** doing any file
work. It asks application service `0x00B43C94`, slot `+0xD4`, for its path with
argument `0xFFFFFFFF`, appends literal `Sys`, a backslash, and `FamilyNames.ini`.
It passes this path to cRZIniFile, then enumerates `PropFamilies`,
`BuildingFamilies`, `FloraFamilies` in that order. It does not retry on a later
editor Init if the file or a section was absent (`0x00486F20..0x004870F9`;
literal `Sys` at `0x00A8718C`). Path/file/section lexing belongs to cRZIniFile;
the callbacks receive the entry key and value, not a second ad-hoc INI grammar.

Each callback converts the key using RZConvertToUint32A and assigns the value
string to that ID in its process-static map. Prop/building/flora callbacks are
`0x00486440/0x00486490/0x004864E0`, targeting maps
`0x00B2167C/0x00B21690/0x00B216A4`. There is no rejection of zero IDs or empty
values, no family-membership validation, and a later assignment to the same
ID replaces the previous string. These are names, not IDs to append to a
lot object (`0x00486440..0x0048652C`).

RZConvertToUint32A (`0x0090FE93..0x0090FEF3`) selects radix 16 for an initial
`0x`/`0X`; otherwise it scans from the first byte, selecting radix 16 on the
first A–F/a–f encountered, continuing over digits, stopping at NUL or another
character. If no such hex letter was encountered it selects radix 10. It then
delegates conversion of the original string to the linked unsigned conversion
routine with null end-pointer. It is not unconditionally hexadecimal: keys
`10`, `0x10`, `FF`, `0010` select IDs 10, 16, 255, 10 respectively. Leading
non-digit text can stop radix detection; no trimming pass is added here.

PostInit `0x0047B890` performs the following in order:

1. Only while editor view field `+0x138` is null, request child `0x692352B5`,
   IID `0xC12CEA13`. The result is subsequently dereferenced without a success
   guard. On its underlying window enable flag `0x200000`; on the view set
   property `(2,true)`.
2. Still inside that first-view block, ask buffer service `0x00B43C9C`, slot
   `+0x0C`, for a buffer. If successful, initialize it to the edit window's
   current width/height with format pair `(3,16)`, install it on view `+0x10`,
   pass buffer `+0x3C`'s result to InitAlphaSystem, then release the buffer.
   Failure skips this buffer/alpha setup and does not undo the acquired view
   (`0x0047B8D6..0x0047B989`).
3. Request containing child `0x6946DA25`, IID `0xC2AFA76F`; on success enable
   the same underlying-window flag and release it. Failure skips this step.
4. If status field `+0x13C` is null, request child `0x29235446`,
   IID `0x212CDC1F`. If model fields `+0x194/+0x198` are null, obtain them
   through NetworkManager `+0x14` with `(3,1)` and `(0,1)` respectively;
   store each result, AddRef it, and call its Init slot `+0x0C`. Those results
   have no null guard (`0x0047B98C..0x0047BA77`).

Repeated PostInit retains existing fields; it does not rebuild the alpha
buffer merely because the view size changed. A failed first buffer acquisition
also does not retry through that guarded block while the view remains non-null.
Shutdown ownership/field mapping remains §23, not inferred from these AddRefs.

Worked case: a missing FamilyNames.ini on the first editor leaves all three
maps without entries. Installing the file before a second Init does not load
it in that process. In a fresh process, prop entries `10=Decimal` then
`0x0A=Replacement` leave ID 10 named `Replacement`; `0x10=Hex` names ID 16.
A successful first PostInit with buffer allocation failure still stores its
view; the next PostInit skips that buffer acquisition rather than repairing it.

Closure mapping: StaticInit, PostInit and the three callback rows above;
the remaining resource/view dependency of Init `0x004892B0`. This does not
assign editor ownership to the shared image decoder (§36).

## 34. Input, capture and cycling (LE16) *[compat]*

§11/§17/§21 establish the wrapper boundaries; §27–§28 complete hit geometry.
The remaining edit-body behavior is as follows (`0x00486770..0x004869D1`):

| Left-down hit / mode | Effect, before returning handled |
|---|---|
| 0 / any | If selected, clear selection then draw/invalidate; otherwise no action. |
| 1 / mode 0 | Same deselection behavior. |
| 1 / nonzero mode | AddNewLotConfigurationObject with original coordinates/modifiers. |
| 2 / mode 0 | Invoke foundation selector with first-foundation ID and null record, then clear selection even if cancelled. |
| 2 / other mode | No action. |
| 3 / mode 8 | Pixel ToggleFence; other nonzero modes do nothing. |
| 3 / mode 0 | Clear any selection and trigger redraw; no fence edit. |
| Object / mode 0 | Select hit record; eligible drag setup below, then trigger redraw. |
| Object / nonzero mode | AddNewLotConfigurationObject. |

For an object in mode zero, drag setup requires modifier bit `0x02` clear and
type in `{0,1,2,4,7}`. Set dragging `+0x1B8` to 1; request capture only if not
already capturing; call PixelUnitsToObjectUnits to store offsets at
`+0x1BC/+0x1C0`. Other types are selected but not made draggable. Edit left-up
clears dragging and releases capture when held (§11). The outer up/down/move
wrappers convert coordinates and use strict edit-client upper bounds; outside
events go to the base window after converting back. Right-down outside the
client opens the outside menu. Every ordinary routed branch returns handled.
The `+0xE0` host-move state takes precedence in the move wrapper and delegates
directly to the base (`0x0047AD40`, `0x00486690`, `0x00487100`, `0x004895A0`).

Edit right-down acts only in mode zero: hits 0/1 open the edit-area menu, 2 the
foundation popup, 3 the fence popup, and ≥100 the object popup. Other hits or
modes do nothing, still returning handled (`0x004893E0..0x00489485`).

### 34.1 Drag arithmetic and status

DoEditMouseMove translates only while dragging and actually capturing. Read
signed pixel x/y, scale s, selected x/z and the saved offsets. The x pipeline
first stores binary32 of `(pixel-x*s − old-x)`, then subtracts saved x offset,
adds old x, and stores a binary32 target. The z pipeline performs
`pixel-y*s − old-z − saved-z-offset + old-z` in x87 before its binary32 target
store. Pass both to the position MoveObject overload (§25.2). The unequal
intermediate spill must not be replaced by a single algebraic simplification
(`0x004859FC..0x00485A47`).

It then updates status even when no translation happened, using selected
creation word `+0x24` or zero and format
`%08x: Pixel: (%d,%d) World: (%.1f,%.1f)`; world values are pixel coordinates
times s. This is pointer position, not the selected centre.

Keyboard handling is capture-gated: while capturing it performs none of its
commands and returns handled. Otherwise it clears dragging before dispatch.
The already specified Escape/C/B/T/P/F/S/G/R/F1 paths and selected-object
arrow/PageUp/PageDown/M/Control-D/Delete/Backspace/Home/N paths all return
handled, including unsupported keys. No generic key forwarding is inserted.
Home's zero test, including unordered behavior, must retain the native x87
comparison rather than a finite-only assumption (GZOnKeyDown `0x00485710`).

### 34.2 Cycling is a live occupant operation

CycleSelectedObject `0x0047C660` returns for null selection, null active city,
or an empty ID vector returned by editor manager `+0x160`, slot `+0x6C` for
the selected record. Only selected **building type 0 and flora type 4** have
cycling branches; props and textures do not cycle. The ID vector is not
advanced or rewritten in the record.

For a building, get the live city's current building, obtain its current type
ID, find it in the returned ID list, choose the following ID or wrap from the
last to the first, and call the building service's replacement operation with
the city and that ID; store the chosen ID in editor `+0x1E4`.
For flora, locate the selected record's index in the lot vector; return if
absent. Resolve its live occupant through building service `+0x100` and query
IID `0xC83DD057`. Obtain current flora ID, choose next/wrap in the same list,
save occupant placement, remove the old occupant, request the replacement
through flora service `+0x44`, and if non-null apply the saved placement through
its `+0x70`. Release the acquired interfaces. These operations do not call
dirty/title/draw/invalidate (`0x0047C772..0x0047C8C3`).

For both branches an unsuccessful list scan is followed by **one more read
at list-end** before the first-ID fallback. Flora proof:
`0x0047C7CD..0x0047C7ED`. Empty list was rejected earlier, but a live current
ID absent from a nonempty list can reach this read. If the out-of-range word
equals the current ID, the following-ID read can go farther. No deterministic
heap value, safe wrap or crash outcome is promised for that malformed state.

Worked cases: left-down on a type-1 object, mode zero, flags zero, starts
capture and preserves its pointer-to-centre offsets; with flag 2 the same
record is selected without starting a drag. A foundation-strip click followed
by Cancel still clears selection. With returned IDs `[10,20]`, a live building
of type 10 requests replacement 20, then one of type 20 requests 10; neither
operation dirties the lot. A selected prop makes no replacement request.

Closure mapping: DoEditMouseDownL/R, DoEditMouseMove, GZOnMouseDownL/R,
GZOnMouseUpL, GZOnMouseMove and GZOnKeyDown control flow; CycleSelectedObject
for valid live occupants. **Narrow runtime observation LE16-R:** for the absent
current-ID case, break before the list-end read and record begin/end/capacity,
current ID and the two words at/after end, then single-step to the chosen ID
or fault. Repeat with a guarded end page. This settles a particular native
failure outcome; it cannot create a heap-independent portable rule.

## 35. Drawing, help and 3D: contracts and runtime boundary (LE17) *[compat]*

Object-name helper `0x0047F480` assigns by type, not by family/instance name:
0 `building`, 1 `prop`, 2 `lot texture`, 3 `foundation`, 4 `flora`, 7 `network`;
all other values, including 5/6, give `unknown`. Its capitalization option
changes only the first byte to upper case. Family/instance and foundation/wall
name routes are separate (§32 and below).

DoShowHelpUI `0x0047CB70` ignores the requested help page: modal title is
`Help System`, message is `The answer to all your questions is:` followed by
a newline and `Ask Paul Pedriana`; host options are `(0,1,0)` and it returns
handled. It neither opens a page-specific document nor changes editor data.
MessageBoxA `0x0047D610` gets the current configuration name as its caption
when a configuration exists (otherwise empty), forwards caller message and
options to host `+0xB0` with trailing `(1,0)`, and returns the host result.
TriggerLotConfigurationRedraw `0x0047E870` draws first, then invalidates;
it does not set dirty or title.

GetRetainingWallName `0x00480080` starts with literal `unknown`, resolves the
wall key via manager `+0x70`, opens its exemplar and reads property `0x20` on
successful acquisition; failure retains the fallback. Foundation name helper
`0x0047FF30` resolves the key via manager `+0x58`, opens/queries the exemplar,
reads name property `0x20`, and appends implementation description. Type 1 is
`Custom`, 2 `Generic`, 3 `Algorithmic`, otherwise `Unknown implementation type`.
A nonempty name is separated from the description by ` (` and closed by `)`;
with no name it is just the description. The implementation classification
uses the exemplar's property-presence test; it is not inferred from a row's ID.
No selector rejects an ID based on this output (§29).

### 35.1 Rasterization ordering

Preserve §13's buffer acquisition/release, reverse object order, marker locks,
alpha style and signed height-label behavior. DrawCurrent `0x0047E060` starts
by requesting buffer IID `0x20732180` from the edit view. A failed request
does not execute the draw body. Successful drawing proceeds through background,
grid, resize decoration, objects in reverse vector order, then fences in
their stored order, followed by view submission/invalidation and release
(`0x0047E060..0x0047E508`). It does not sort objects by type or elevation.

Background uses RGB `(192,192,192)`, grid `(128,128,128)`, fences `(220,0,0)`;
the grid includes both zero and the last width/depth boundary. Drawing uses
integer rectangle conversions (§28), not hit-test's mixed reciprocal path.
Object drawing first alpha-fills with its style, then draws its outline,
foundation indicator, centre marker, orientation marker and height label in
that order, under the separate buffer-operation locks already recorded in
§13 (`0x0047CF90..0x0047D603`). A failed lock skips its protected operation;
it does not authorize drawing on unlocked storage.

Worked name/help case: type 5 with capitalization true gives `Unknown`;
help requests for pages 1 and 7 produce the same Paul Pedriana dialog.
Worked draw-order case: vector `[A,B]` with overlapping equal-size bounds
submits B's object operations then A's; fences follow both. Rejecting an
outline lock does not suppress an independently successful centre-marker lock.

### 35.2 Exactly what still requires an observation

The following are **bounded observation requests, not complete rasterizer/3D
closure claims**. The host buffer/renderer and live-city services are acquired
interfaces. Their concrete execution and resulting pixels/terrain have not
been measured in this lane. An observation must include its input fixture and
the call arguments/return values, not a screenshot alone.

| ID / remaining row | Required original-program observation |
|---|---|
| LE17-R1: DrawLotConfigurationObject, DrawCurrentLotConfiguration | At s=0.5, lot 2×2, buffer 96×96, place overlapping records A/B with bounds `(4,4,12,12)` and `(8,8,16,16)`, select A, then repeat with foundation ID nonzero, orientation 0–3, heights −1/0/+1, and representative style/type flags. Record each buffer lock result, each fill/line/outline/text call and rectangle, packed color/alpha, and the final buffer bytes. Force one lock failure per operation. This discriminates exact marker endpoints, clipping, outline inclusivity, blend rounding and failure pairing; §13's reverse order and height-label quirks remain mandatory. |
| LE17-R2: Prepare3DView `0x0047BBA0` | On a disposable flat city, record service identities, occupant-filter enumeration/removals, initial water level, every terrain read/write/update rectangle, first with slope 0 then 1 and 45. Break around `0x0047BC48..0x0047BE1D`; capture the x87 control word and intermediate stored float bits. The body scans a 28×28 area and writes a 26×26 area, but exact unsigned-slope conversion, tangent precision, store ordering and host effects must be obtained together; no numeric terrain golden value is asserted here. |
| LE17-R3: GoTo3DView `0x0047B700` | Record the view-service `+0x40` call, terrain `+0x38` result and final view transition for lot sizes 1×1, 2×3, 255×255 and null configuration. The centre arguments are byte `(dimension >> 1)+2`, or `(3,3)` without a configuration; record the resulting world/view target rather than assuming these bytes are pixel coordinates. |
| LE17-R4: GetAllLotBaseTextureTypes `0x0047FC50` | Open the texture selector with a controlled set of texture resources: two accepted IDs with images, a missing image, an unsupported source format and a duplicate ID. At `0x0047FDCD` record resource key, decoder Boolean/output pointer, and the final parallel ID/image arrays `+0x1C4/+0x1D0` before grid population. This settles resource-instance eligibility, duplicate handling and failed-image alignment for that fixture without inventing a row/image correspondence on failure. |

Closure mapping: GetLotConfigurationObjectName, DoShowHelpUI, MessageBoxA and
TriggerLotConfigurationRedraw bodies close; retaining-wall/foundation name
helpers close their fallback/formatting portions. DrawLotConfigurationObject,
DrawCurrentLotConfiguration, Prepare3DView, GoTo3DView and
GetAllLotBaseTextureTypes remain **partial** with the exact observations above.
The LE5 README group must not be marked fully closed from this section.

## 36. Shared constructor/decoder attribution (LE18) *[compat]*

Keep three distinct facts: an editor call reaches a body; the body's behavior
is a dependency; the original source file/class exclusively owned that body.
The first two do not establish the third. The five shared bodies/427 current
export bytes in §18 remain the same five units:

| Entry | Current bytes | Attribution that the available evidence supports |
|---|---:|---|
| `0x00478360` | 64 | Generic 0x34-byte lot-object record initialization (§9.4); six exported direct calls from AddNewLotConfigurationObject, no editor-this/state access proving exclusive ownership. |
| `0x00604920` | 169 | Resource-to-buffer conversion entry, invoked by texture enumeration; opens resource IID `0x1AB50C3F`, obtains source data, dispatches on its format and obtains buffer IID `0x20732180`. Resource/buffer services are general dependencies. |
| `0x006049BF` | 7 | Decoder format dispatch fragment. |
| `0x006049C6` | 7 | Decoder format dispatch fragment. |
| `0x006049CD` | 180 | Exported shared switch case, including its discontiguous ranges from §18; not an editor member established by a source symbol. |

The decoder nulls the caller's output pointer before resource acquisition;
failed acquisition returns false (`0x00604938..0x00604951`, `0x00604B6B`).
Its buffer factory and resource format dispatch are independent of editor
selection/mode. Count neither the two historical interior addresses
`0x006049D4/0x00604A67` nor decoder reachability as additional editor bodies.
No bytes are restored to the editor's exclusive-ownership total by this rule.

Worked attribution case: opening the texture selector and reaching
`0x00604920` once adds one observed dependency call, **zero new function
entries and zero exclusively attributed editor bytes**. The inventory still
has five shared entries totalling 427, not seven entries after adding the
historical fragments. Constructing a new type-1 record exercises the record
initializer without making the generic initializer an editor method.

Closure mapping: the five R/shared-helper ownership rows are classified as
shared dependencies; they are not C editor-body credit. **LE18-R observation:**
break at all five actual entries while opening a texture selector and while
opening another image-resource consumer, recording call stacks, resource keys,
buffer interface targets and returns. This can demonstrate additional sharing
and selects the concrete decoder path for LE17-R4. It cannot prove exclusive
source ownership by the absence of other hits. Exclusive source provenance
would require an original link/debug/source map, not a finite runtime trace;
that evidence is unavailable here. Keep that attribution boundary explicit
rather than reopening broad LE5 or claiming a runtime trace proves a source file.

### 36.1 LE9–LE18 disposition

§27–§36 supersede the earlier broad LE3/LE4/LE5/LE7 stopping labels only for
the rows explicitly mapped in each section. LE9–LE15 have static contracts;
LE16 has an explicit malformed-state observation; LE17 is narrowed to four
specified observations; LE18 resolves shared-versus-exclusive accounting and
states the provenance limit. No game observation or complete-owner closure
was performed or inferred. All ten entries are archived with these limits.

## 19. Window-owner integration and slot inventory *[compat]*

Added 2026-10-23, re-lotcfg-editor5. Sections 1–18 and the
[exact editor body inventory](lot-configuration-editor-inventory.md) remain
the detailed editor contracts and the historical stopping assessment. This
addition specifies the companion chooser and further window integration;
it does not turn that earlier partial coverage into complete-owner credit.
The data model, exemplar properties and configuration interface belong to
[lot-configuration](lot-configuration.md); family/prop identity dependencies
belong to [prop-lotconfig](prop-lotconfig.md). Do not implement a second
configuration format for these windows.

The following are **zero-based primary-window slots** in the 1.1.641 image.
They were checked against the executable's table words, rather than inferred
from export names: editor table `0x00A8D888`, chooser `0x00A8D4E0`.
Folded function names in the export do not establish an unrelated interface.
Unlisted slots through 151 are inherited window behavior; there is no chooser
extension after slot 151. Editor extension slots are 152–159.

| Slot / byte offset | Editor target and contract | Chooser target and contract |
|---|---|---|
| 0 / `+0x00` | `0x0047A830`, QueryInterface, §7 | `0x0049A1E0`, base-window QueryInterface thunk |
| 1 / `+0x04`; 2 / `+0x08` | Shared AddRef `0x008AE960`, Release `0x008AE970` | Same shared entries |
| 3 / `+0x0C` | `0x0047BA80`, main DoMessage, §7 | `0x00489490`, §20.2 |
| 4 / `+0x10`; 5 / `+0x14` | Init `0x004892B0`, Shutdown `0x0047C510`, §7 | Init `0x0047A7B0`, Shutdown `0x0047C2D0`, §20.1 |
| 130 / `+0x208` | Key down `0x00485710`, §11 | Inherited |
| 134 / `+0x218`; 135 / `+0x21C` | Left/right down `0x00487100` / `0x004895A0`, §21 | Inherited |
| 136 / `+0x220`; 138 / `+0x228` | Left up `0x0047AD40` / move `0x00486690`, §21 | Inherited |
| 148 / `+0x250` | Deleting destructor `0x0047E5D0`, §21.1 | Deleting destructor `0x0047CA40`, §20.1 |
| 152 / `+0x260` | Save As entry `0x00485390`, §8.2 | — |
| 153 / `+0x264` | Close `0x004865D0`, §8.3 | — |
| 154 / `+0x268` | Persistence helper `0x0047AB70`, §8.2 | — |
| 155 / `+0x26C`; 156 / `+0x270` | Create new `0x00483B20` / read exemplar `0x00483C80`, §8.1 | — |
| 157 / `+0x274` | Default path `0x0047EFF0`, §8.2 | — |
| 158 / `+0x278` | Edit-area menu `0x004871B0`, §2 | — |
| 159 / `+0x27C` | 3D refresh `0x0047BE20`, unresolved internals LE5 | — |

The editor's dialog callback at object `+0x128` has its own adjusted-this
contract (§12.1); `0x0047DD60` is not primary-window slot 3. The auxiliary
window interface is at `+0xD8` for both owners. These offsets describe native
adapter boundaries, not a portable core layout requirement.

### 19.1 Chooser method inventory

These are the chooser-specific bodies, including helpers identified by their
table installation/use. Generic containers and inherited window methods are
dependencies, not additional chooser methods. This list is separate from
the editor-only historical byte inventory.

| Entry | Method / responsibility | Contract |
|---|---|---|
| `0x0047C990` | Constructor (unnamed export) | §20.1 |
| `0x0047A7B0` | Init | §20.1 |
| `0x00480460` | PostInit | §20.1; service-discovery residual LE7 |
| `0x00489490` | DoMessage | §20.2 |
| `0x0047AAF0` | Set sort column/options (unnamed export) | §20.2 |
| `0x0047EBB0` | SetupLotConfigurationList | §20.3 |
| `0x0049A7F0` | GetSelectedLotConfigurationType | §20.3 |
| `0x0047E580` | GetSelectedLotConfigurationKey | §20.3 |
| `0x0047D940` | IsSelectedLotBeingEdited | §20.4 |
| `0x00486D30` | DoRequestEditLotConfiguration | §20.4 |
| `0x0047ED90` | DoRequestDeleteLotConfiguration | §20.5 |
| `0x0047D9C0` | RemoveLotConfigurationTypeFromGrid | §20.5 |
| `0x0047C4B0` | DoLotConfigurationEditorClosed | §20.4 |
| `0x0047C150` | User close request (unnamed export) | §20.1 |
| `0x0047C2D0` | Shutdown | §20.1 |
| `0x0047C1F0`; `0x0047CA40` | Destruction / deleting wrapper | §20.1 |
| `0x0049A1E0` | QueryInterface folded entry | Primary-window base delegation |

## 20. Lot configuration chooser *[compat]*

### 20.1 State, construction and lifecycle

Construction first constructs the generic window, then installs the chooser
tables. Its owner/controller reference (`+0x128`), grid (`+0x12C`) and
configuration manager (`+0x130`) start null. Two native ordered sets start
empty, with separate allocated sentinels: service references at `+0x134`
(count `+0x138`) and open editor pointers at `+0x140` (count `+0x144`).
Sort column `+0x14C` and option `+0x150` both start zero. Allocation of the
sentinels is followed by unchecked stores; no recoverable out-of-memory
return is established (`0x0047C990..0x0047CA3C`).

On the first Init, set primary flag `0x10000` true and auxiliary interface
flags `1` and `0x10` true, `0x100` and `4` false. Invoke base Init and post
`0x0920B687` to this window with three zero payloads. Repeated Init skips
this block and returns true. These are raw flag contracts; semantic names
for the masks are not established. The editor uses the same flag requests
in its own Init (§7).

PostInit resolves child `0x091C91BA` as grid IID `0xDAA6B9BE` if absent,
then calls its `+0x78` with `(2,1,0)`. The returned grid is used without
checking lookup success. If the chooser manager is absent, acquire the
global lot configuration manager if available, retaining it. A separate
service enumeration queries IID `0x856C8CF7` and retains directory-file
segment interfaces in the first set; §26.1 gives the instruction-verified
acquisition and release contract. PostInit always
rebuilds the list (§20.3). A missing manager yields an empty list; a missing
grid is not a supported empty-state fallback.

User close (`0x0047C150`) asks the first tracked editor to Close with
`(true,true)` and repeats while editors remain. An editor returning false
stops the operation immediately: remaining editors and the chooser stay
open. Already closed editors are not restored. After the set is empty,
invoke controller `+0x14` with false if a controller is present. This relies
on each successful editor close removing its registration; it is not a
snapshot iteration that ignores close callbacks.

Shutdown instead asks tracked editors to Close with `(false,false)`. It
rechecks membership after each close and explicitly Releases/removes an
editor if the callback did not already do so. It then Releases service-set
members, clears that set, clears/releases controller, grid and manager in
that order, and invokes base Shutdown (`0x0047C2D0..0x0047C4AE`).
This is not the cancelable user-close operation. The body destructor
`0x0047C1F0` disposes the two set trees/sentinels, Releases remaining manager,
grid and controller references and invokes the base destructor. It does not
replace Shutdown's editor-close/service-release protocol. The deleting
wrapper `0x0047CA40` additionally frees this object when argument bit 0 is set.

### 20.2 Messages and sorting

All category-3 messages return true, even unknown commands or controls.
For category 3, only command `0x287259F6` dispatches the controls below.
Message `0x0920B687` invokes PostInit and returns true; other categories
delegate to the base window (`0x00489490..0x00489599`).

| Control ID | Action |
|---|---|
| `0x091C91BB` | Request edit with new-configuration option true |
| `0x091C91BC` | Request edit with new-configuration option false |
| `0x091C91BD` | Request deletion |
| `0x0942D1CB` | Set sort column 0 |
| `0x0942D1CC` | Set sort column 1 |
| `0x42B7C353` | Cancelable chooser close (§20.1) |

For either sort control the option is true if key `0x11` or key `0x10`
is down, false otherwise. If grid is null, the sort helper does nothing,
including no state update. Otherwise it stores column and option and calls
grid `+0x20C` with `(column,option,1,0)` (`0x0047AAF0..0x0047AB22`).
Rebuilding the list reapplies the stored pair. Direction/collation meaning
is a grid dependency, not established by the option's boolean value.

### 20.3 Rows, selected type and exemplar key

Rebuild clears the grid with `+0x188` before checking manager availability.
With a manager it enumerates through manager `+0x24` with option false,
placing each returned configuration type's ID (`+0x0C`) in column 0 as
`0x%08x`, and its name (`+0x10`) in column 1. Rows start at zero and are
appended in enumeration order, followed by the stored sort request.
The enumeration container is a hash set; no numeric or alphabetical
pre-sort is promised (`0x0047EBB0`, row writes `0x0047EC7E..0x0047ECF6`).

Selected-type lookup asks grid `+0xA8` for its current cell. Failure gives
type zero. Success reads **column 0 of the selected row**, regardless of
the selected column, and converts its text with `RZAtoUint32HexA`
(`0x0049A7F0`). Thus visible row text, not the displayed name or row index,
is the selection identity. Malformed text behavior belongs to that parser.

Selected-key lookup fails for type zero, absent global manager or an
unresolvable type. On success it writes key words `(0x6534284A,
0xA8FBD372,resolvedIdentityID)`, where the last word is the manager-resolved
type object's `+0x0C` result. Failure does not initialize the caller's key
(`0x0047E580..0x0047E5C1`). The selected type and resolved identity ID must
not silently be assumed identical by a service adapter.

### 20.4 Opening and editor registration

A request first requires a nonzero selected type, including the true/new
option. With no selection, even this option creates nothing. An existing
tracked editor whose stored type at editor `+0x164` equals the selected type
blocks the request. Show title `No can do` and text `The lot you are trying
to edit is already open for editing.`; do not focus/reuse that editor.

The false option requires successful selected-key resolution; failure ends
silently. The true option bypasses key lookup and supplies an all-zero key
to editor Open (the new-configuration route in §8.1). Create the editor
from script key `(0,0x96A006B0,0x09234182)` with window ID `0xC9235387`.
Creation failure is silent. On success retain the chooser in the editor,
copy its controller reference and perform window-manager `+0x94` before
calling Open. Successful Open retains/registers the editor in the pointer
set and relinquishes the temporary creation reference. A failed Open shows
`Unable to open the lot for editing.` with title `No can do`, removes the
window through creator `+0x3C`, calls editor Shutdown and Releases it
(`0x00486D30`). No failed editor is registered.

Registration is by editor pointer, not selected ID. IsSelectedLotBeingEdited
compares the current selected type against each editor's stored type
(`0x0047D940`); a newly created editor may therefore cease to block the
original selected type. The close notification `0x0047C4B0` Releases and
erases only a registered pointer, decrementing the count. An unknown pointer
does nothing, so repeated notifications do not Release twice.

### 20.5 Deleting and failure ordering

No selected type means no operation. A type open for editing shows title
`No can do` and text `The lot you are trying to delete is currently open
for editing.` before identity or filesystem work. Otherwise resolve its
identity (§8.4); failure ends silently. Ask `Are you sure you want to delete
the selected item?` with title `Are you sure`. Only result `0x5301814A`
continues.

Deletion requires the identity's backing file to exist and be writable.
Otherwise show exactly `The exemplar you are trying to delete is either non-existant or write-protected.\n\n`,
where each `\n` denotes an LF byte (two trailing line feeds), with title
`No can do`. Preserve `exemplar`, the spelling `non-existant` and the trailing
blank line (`0x00A8DB98`, used by `0x0047ED90`). The chooser invokes its
message service with raw options `(0x10000,1,0)` for this error. A failed backing-store
delete (`+0x6C`) shows `Unknown error trying to delete item.` with that title.
Neither failure removes a row or unregisters a manager type. On success,
remove the row identified by the identity ID, then ask the global manager
for the selected type and unregister it (`+0x1C`) only if still present
(`0x0047ED90`). The backing-store reference is the identity's retained
persistent segment, not its separate directory-file interface. The delete
request contains exactly the identity's three-word exemplar key (§26.1);
it does not request deletion of the whole containing database file.

Row removal formats the supplied ID as `0x%08x` and searches through grid
`+0x214` with raw arguments
`(text,length,0,1,0,0x7FFFFFFF,&row,&column,1)`. If found, remove one row with
`+0x194(row,1,0)`. Whether found or absent, this helper returns false in AL;
callers must not treat that return as a successful-deletion indicator
(`0x0047DA1D..0x0047DA7D`). The grid's search-range interpretation is a
dependency; the raw `1` is not proof that visible row zero is excluded.

## 21. Editor host drawing and input boundary *[compat]*

The host supplies the generic window, child view, buffer/font/cursor services,
modal UI, configuration manager and persistence services. Portable core
state may use its own layout *[choice]*, but host requests, cancellation,
selection, dirty-state and reference effects must follow these contracts.
There is no separately proved editor-owned disk format: §8 uses the
configuration/exemplar services described in [lot-configuration](lot-configuration.md).

Left/right button down convert incoming window coordinates into the edit
child's coordinates and use the half-open region `0 <= x < width`,
`0 <= y < height`. Inside invokes the corresponding edit handler. Outside
left down converts back and delegates to generic-window left down; outside
right down opens the outside-area menu. Left up and mouse move use the
same region test and generic fallback. For those two handlers, nonzero
base field `+0xE0` bypasses edit dispatch and returns the base handler's
result. Otherwise the wrapper returns true after either route
(`0x0047AD40`, `0x00486690`, `0x00487100`, `0x004895A0`). Do not infer from
drag capture alone that every outside mouse-up reaches DoEditMouseUpL.
The edit-level keyboard/drag/capture rules remain §11; geometry/hit testing
remain §9 and §17.1 with their explicit numeric residuals.

Drawing obtains an edit-view buffer through view `+0x14` with request
`0x20732180`. Only successful acquisition runs the drawing phases.
Background uses RGB `(192,192,192)`, native conversion through window
`+0x1B0`, buffer access `+0x18(0x8001)`, fill `+0x48(color,0)`, and unlock
`+0x1C(0x8001)`. A failed lock skips the fill. Grid/outline use a separate
access `0x8004`; failure skips that phase, not the object pass. Objects are
drawn in reverse record order (§13). Fence overlays follow the objects and
have their own `0x8004` access. Buffer ownership is relinquished at the end
of the draw routine. The output begins null and the final Release is
**unconditional even when acquisition returns false**
(`0x0047E514..0x0047E51A`). Failure leaving a null output therefore has an
unchecked null dereference; it is not a safe no-op result. An adapter must
honor the service's output/reference preconditions. Remaining raster geometry
is LE5. The fence phase calls unlock even after its access request fails
(`0x0047E3A6..0x0047E3A8`, `0x0047E4B0..0x0047E4BB`).
Evidence: `0x0047E060..0x0047E0E5`, object pass
`0x0047E33D..0x0047E36D`, fence access `0x0047E39A..0x0047E3A8`.

`TriggerLotConfigurationRedraw` calls DrawCurrentLotConfiguration and then
window virtual `+0x170` after the draw routine returns
(`0x0047E870..0x0047E882`). Raster generation and presentation invalidation
are separate host effects. Do not replace the trigger with only a dirty
flag. If drawing faults on invalid host output, no subsequent invalidation
is guaranteed. Full 3D preparation/refresh and name fallbacks remain LE5/LE4.

### 21.1 Editor lifetime and static resources: further facts

StaticInit sets its once flag **before** reading `FamilyNames.ini`. It
enumerates `PropFamilies`, `BuildingFamilies` and `FloraFamilies` through
three callbacks (`0x00486F20`), so a missing/unreadable file is not retried
by the next Init merely because the resources are empty. Exact path
composition, callback name collision/fallback rules and error reporting
remain LE5; absence of a returned error here is not a promise of loaded names.

The editor body destructor is `0x0047DC20`, called by deleting wrapper
`0x0047E5D0`. It frees the storage blocks based at `+0x1D0`, `+0x1C4` and
`+0x1A0`, Releases current configuration `+0x190`, frees the string storage
at `+0x178`, Releases manager `+0x160`, chooser `+0x15C` and cursor/service
references at `+0x158,+0x154,+0x150,+0x14C,+0x148,+0x144,+0x140,+0x13C,
+0x138,+0x134`, then invokes the base destructor. Null references/storage
are skipped. It does not perform Shutdown's per-element Release loop on
the owned reference array: callers must execute the lifecycle protocol,
not substitute raw destruction for Shutdown. The deleting wrapper frees
the editor object only when argument bit 0 is set. These effects complement,
rather than contradict, §7's observation that Shutdown itself does not
release the current configuration (`0x0047DC20..0x0047DD5D`,
`0x0047E5D0..0x0047E5ED`).

## 22. Window integration conformance vectors

Use recording host/service doubles; these expected outputs follow native
evidence, not a current core implementation. Numeric residuals are not given
invented NaN/overflow expectations.

| ID | Initial state and action | Expected effects |
|---|---|---|
| LEW01 | Construct chooser, then Init twice | Null controller/grid/manager; empty sets; sort `(0,0)`. First Init requests the five flags and posts one `0x0920B687`; second posts none. |
| LEW02 | Chooser receives unknown category-3 command/control | True; no base dispatch. Other category, except PostInit message, delegates. |
| LEW03 | Sort column 1 with Control up, Shift down | Stored `(1,true)`; grid call `(1,true,1,0)`. With both up, false. With grid absent, stored values unchanged. |
| LEW04 | Rebuild after sort `(1,true)`, manager absent | Grid cleared; no rows or sort request. With manager present, append ID/name rows then reapply sort. |
| LEW05 | Select column 1, row whose column 0 is `0x000000ab` | Selected type `0xAB`. Failed current-cell query returns zero. |
| LEW06 | Selected type 5 resolves to identity ID 9 | Selected key `(0x6534284A,0xA8FBD372,9)`. Missing manager returns false and leaves output sentinel unchanged. |
| LEW07 | No selection; request either edit option | No key lookup, window creation or error UI. |
| LEW08 | Tracked editor stores selected type; request edit/delete | Correct already-open error; no editor creation or delete confirmation. |
| LEW09 | Selected type exists; true/new option | Skip key lookup; create editor and Open with zero key. False option with key-resolution failure creates nothing. |
| LEW10 | Editor creation succeeds, Open fails | Error, creator removal, Shutdown, Release; tracked-editor set unchanged. |
| LEW11 | Registered editor notifies close twice | First notification Releases/removes once; second has no effect. |
| LEW12 | Two tracked editors; first closes, second refuses user close | First remains closed; second and chooser remain open; controller close not invoked. |
| LEW13 | Delete confirmation canceled, or file absent/read-only, or delete service fails | No row removal or manager unregister; failure UI only for the latter two conditions. |
| LEW14 | Row removal search succeeds / fails | Success requests one-row removal; absent makes no removal request; AL false in both cases. |
| LEW15 | Child size 100 by 50; button down at `(99,49)` vs `(100,49)` | First reaches edit handler; second takes left base fallback or right outside menu. Negative coordinates are outside too. |
| LEW16 | Draw buffer fill lock fails; later phases available | No fill/unlock for failed fill lock; later drawing proceeds. Trigger invokes window `+0x170` after draw returns. False acquisition with null output is an unsafe native call, not a passing null-buffer vector. |
| LEW17 | Shutdown then destroy editor holding current configuration | Shutdown's array element releases precede storage free; current configuration released by destructor, not by Shutdown. |
| LEW18 | First StaticInit cannot read family file; Init again | Once flag remains set; no second static-resource attempt. |

Unresolved questions are in [the question file](questions/lot-configuration-editor.md).
This addition does not claim complete numeric/raster contracts, game verdicts,
ABI exports or ownership of shared decoding helpers.

## 23. Portable window/core composition (LE8) *[compat]*

Chooser edit acquisition and registration may be staged around the **core
editor's** Open result (§20.4). The acquired editor executes §8.1, including
PostInit and the key-dependent new/read path; only its successful completion
allows registration. A failure takes the existing error/removal/Shutdown/Release
path, even if Read has already installed a configuration. Similarly, chooser
user-close dispatch invokes the corresponding core editor Close with `(true,true)`
and consumes that result while reexamining its live tracked set (§20.1).
The notification can remove the editor from that set before dispatch returns.
These are composition boundaries, not alternative policy providers: an adapter
must not supply an arbitrary Open/Close verdict to bypass unresolved core
behavior in LE4/LE5, nor decide prompts, verification, reconciliation or save
policy on the editor's behalf. An unsupported core continuation stays
unsupported rather than becoming success. No extra question is required about
staging itself; the native calls already impose the sequence
(`0x00486D30`, `0x0047C150`, `0x00486530`, `0x004865D0`).

| Vector | Setup/action | Required observation |
|---|---|---|
| LEI01 | Confirm deletion; identity resolves; file absent or not writable | Error text above including both trailing LF bytes, title `No can do`, raw options `(0x10000,1,0)`; no backing-store delete, row removal or unregister. |
| LEI02 | Initialized editor with all owned slots non-null; Shutdown then destroy | Notification first; exact §7.1 Release/clear order; no cursor/current-configuration Release until destruction; array elements released before storage is freed. |
| LEI03 | Uninitialized editor with non-null cursor and current configuration; Shutdown | Base Shutdown only; references remain for destruction. |
| LEI04 | Chooser obtains an editor whose Open fails after Read installed its configuration | No tracked registration; error, removal, Shutdown, Release; do not turn the partial Read effects into a successful Open. |
| LEI05 | First tracked editor closes and its notification removes it; next refuses | Reexamine live set; first remains removed, refusing editor and chooser stay open. No forced success or snapshot traversal. |

## 24. Core integration contracts (LE6) *[compat]*

### 24.1 Raw record roles and construction

The bounds names in §9 are **editor-local roles of raw fields**. They do not
rename the [shared record decoder](lot-configuration.md#35-the-object-array),
which assigns serialized v6/v7 to `+0x18/+0x1C` (its minima), and v8/v9 to
`+0x10/+0x14` (its maxima). The editor predicate actually reads `+0x10/+0x14`
for its lower-edge tests and `+0x18/+0x1C` for its upper-edge tests in full
mode, with the opposite edges tested in partial mode
(`0x0047B677..0x0047B6E4`). This is an inconsistent native interpretation,
not a second disk format. Preserve raw fields across the editor/manager
boundary; do not normalize, reorder or swap bounds to make the names agree.
The manager's reconciliation uses its own published roles. Inventory reach
still does not establish exclusive ownership of the five shared bodies named
in §18/the inventory companion; their 427 bytes remain foreign.

The configuration constructor, before editor-specific new-lot overrides, has
the defaults in [lot-configuration §2](lot-configuration.md#2-state).
In particular, compatibility vectors at `+0x44/+0x58/+0x6C` have logical
lengths **16/4/9**, all bits false, including bit 0; empty placeholders are
incorrect. Growth-stage byte `+0x3C` is **1**, water byte `+0x42` is **0**,
and required-roads byte `+0x8E` is **8**. Slope fields `+0x30/+0x34/+0x38`
are binary32 **90/0/90**. Capacities `+0x3E/+0x40` are **65535/0**.
The ignored accepted control `0x294053DB` is seeded from the **water getter**,
not a capacity or another editable scalar: only AL is defined by that getter
and the caller forwards EAX without extension. A portable Boolean control
uses the returned low byte; no contract is assigned to the upper 24 bits
(`0x00477880`, `0x00480DE2..0x00480DEA`). The editor's new-lot path changes
the name, size and slope-before-foundation to `New Lot`, 1×1 and 5 as §8.1
already specifies; it leaves the logical vector lengths and growth stage intact.
Constructor stores and resize arguments are instruction-verified at
`0x004797DC..0x004798C6`.

For the three floating initial compatibility values, `_ftol2` truncates toward
zero to a signed 64-bit result and the editor passes its **low 32 bits** to
the child. Thus ordinary in-range finite fractions are not rounded to nearest
or clamped. Acceptance still uses signed dword-to-float stores (§4), independent
of that initial conversion. Invalid/out-of-64-bit-range conversions belong to
the runtime primitive, not a saturating editor rule
(`0x00480E1C`, `0x00480E67`, `0x00480EB2`, `0x009EEF04..0x009EEF78`).

### 24.2 Identity splitting and case conversion

Use [paths §1.5](paths.md#15-splitpathp-drive-dir-name-ext-0x00917bcb-on-crzstrings-0x009194a0)
literally: drive is the first two bytes if byte 1 is `:`, directory includes
the last separator, and the last dot after that separator starts the extension
**including the dot**. Both slash forms are separators. Each output part is
limited to 255 bytes. No canonicalization of separator spelling, dot segments,
relative paths or UNC prefixes is added. GetPathDirectory concatenates the
split drive and directory (`0x0091A847`); the identity helper tests that result's
existence and compares its lowercased bytes to the lowercased provider path.
The provider key query receives the original-case split name and extension,
not the lowercased directory and not a dotless extension
(`0x0047F248`, `0x0047F2B6`, `0x0047F33E..0x0047F3AC`).

Lowercasing is the native [framework-string transform](rz-strings.md#5-native-transforms-and-formatting),
not filesystem collation: bytes below `0x7F` use CRT classification/lowercase;
the first byte at or above `0x7F` triggers conversion of the **whole current
string** from encoding 8 (framework UTF-8) to encoding `0x10` (UTF-16),
`towlower` on each 16-bit unit, then conversion back. Already transformed
ASCII prefix bytes participate in that whole-string conversion. The converter's
reported output count controls the wide loop, conversion return codes are not
tested, and the result still enters exact length-and-byte equality. Malformed
input/partial conversions follow [string-encoding §§3–4](string-encoding.md),
not a replacement host UTF-8 decoder. There is no normalization or full Unicode
case folding. Under the CRT C locale, wide A–Z gain `0x20`, every other unit
is unchanged, including `0xFFFF` (`0x009F4C4E..0x009F4C75`). Non-C locale
tables remain the **existing foreign RZS2** boundary in
[the string questions](questions/rz-strings.md); resolving editor LE6 does not
authorize host Unicode folding or claim closure of RZS2. Exact native wrapper
dispatch/count behavior is specified at `0x0090EAC5..0x0090EB8D` and in those
dependencies; no additional editor policy is missing.

### 24.3 Literal text contracts and manager bridge

The verification failure question is exactly `One or more lot objects don't fit within the lot bounds. Do you want to continue or cancel?`
(`0x00A8D798`). It uses the editor MessageBox wrapper with option **1**;
the wrapper obtains the current lot name for the title (empty without a
current configuration), then calls the message service with `(text,title,1,1,0)`.
Only `0x5301814A` permits continuation; every other result rejects. No repair
or dirty write accompanies either choice (`0x0047D610`, `0x0047D6B0`).

Slope initial text uses signed decimal `%d` on the stored 32-bit slope bits,
despite unsigned acceptance: high-bit-set values display negatively
(`0x0047DE72..0x0047DE90`, format `0x00A8281C`). Network path initial text
uses **`%08x`**, eight lowercase hexadecimal digits with zero padding and
**no `0x` prefix** (`0x0047E701..0x0047E711`, `0x00A8DB8C`). Fence menu row 0
shows exactly `Sorry, the fence options menu isn't implemented yet and won't be implemented until fence functionality is in place.`
(`0x00A8DDB0`), using the editor wrapper with option 0 and current-lot-name
title; it performs no fence edit. Literal strings above have no implicit
newline; contrast the two explicit trailing LF bytes in §20.5.

Successful Read must call manager `FixLotConfiguration(currentConfiguration)`
at interface `+0x5C`, returning a 32-bit result, **on the same mutable
configuration just installed**, before the warning/dirty handling in §8.1
(`0x00483C80..0x00483D98`). The typed bridge is the already-published
[manager slot 23](prop-lotconfig.md#b5-behaviour-of-the-queries): mutable ordered records and
their ID lists, mutable retaining-wall ID list, dimensions, and that manager's
held building/prop/flora validity and footprint dependencies. It must execute
that manager contract, preserve its mutations and return its first-problem
code. It cannot be replaced by a host-supplied reconciliation outcome, an
editor-bounds conversion, or a Boolean success callback. Code zero returns
true without clearing existing dirty; nonzero marks dirty, updates title and
warns but still returns true. The dependency algorithm, including removal,
footprint/center mutation and code precedence, is already specified in
prop-lotconfig; implementing the bridge is engine work, not another RE question.

### 24.4 Conformance vectors

| Vector | Setup/action | Required observation |
|---|---|---|
| LEI06 | Construct configuration; then create editor new lot | Logical compatibility lengths stay 16/4/9, all false; growth 1, water 0, roads 8. Raw constructor slopes 90/0/90 become 90/0/5 for new lot. |
| LEI07 | Initial compatibility slope values 5.75, -2.75, 0.75 | Child signed integers 5, -2, 0; accepting unchanged controls writes floats 5, -2, 0 and marks dirty. Water-control edits have no accepted model write. |
| LEI08 | Shared decoder receives finite bounds v6/v7=4/5, v8/v9=12/13 | Raw `+18/+1C=4/5`, `+10/+14=12/13`; editor-local bounds read `(xmin,zmin,xmax,zmax)=(12,13,4,5)` without repair or swapping. |
| LEI09 | Identity path `C:\Lots\a.b.SC4Lot`; matching provider directory | Directory `C:\Lots\`; provider gets name `a.b`, extension `.SC4Lot`. An otherwise identical directory using `/` does not compare equal after lowercasing alone. |
| LEI10 | Split leaf `.SC4Lot`, `a.`, or `a` | Name/extension respectively empty/`.SC4Lot`, `a`/`.`, `a`/empty. Directory dots do not create an extension. |
| LEI11 | C-locale directory contains UTF-8 `AÉ` or `aé` | Lowercase results are UTF-8 `aÉ` and `aé`; exact comparison differs. Do not impose Unicode case-insensitive equality. |
| LEI12 | Slope bits `FFFFFFFF`; path payload word 3 `AB`; short payload | Initial texts `-1`, `000000ab`, and `00000000`, respectively. Initial formatting does not alter stored values. |
| LEI13 | Verification fails; current name `New Lot`; choose continue/cancel | Exact §24.3 text and title `New Lot`, service options `(1,1,0)`; only `0x5301814A` returns true, neither result mutates records. |
| LEI14 | Successful Read installs a configuration whose only prop has only an invalid ID | Actual manager Fix removes that ID and record, returns first-problem code 1 (not 2); editor retains the mutated configuration, marks dirty and shows the code-1 warning. No supplied external result can replace these effects. |
| LEI15 | Fix returns zero on an already-dirty editor | Read returns true and leaves dirty true; no reconciliation warning. |

## 25. Residual geometry and title contracts *[compat]*

### 25.1 Type-7 payload request, copying and placement-mode commands

The type-7 temporary record starts with an empty payload vector. Growing it
to three words requests **12 payload bytes**, sets both logical end and
capacity end to start+12, and then performs the four stores described in §9.4.
Thus the fourth word is beyond capacity as well as beyond logical size. This
is a native memory-write quirk, not an implicit fourth payload element.
Evidence: resize `0x006BDF10..0x006BDF66`, insertion growth
`0x0042340A..0x0042341A`, and actual growth helper
`0x0051CA64..0x0051CA95`, `0x0051CB1D..0x0051CB30`. The latter's exported
hashtable name is misleading: its instructions grow a dword vector by
old-size + max(old-size, added-size). For an empty vector that is exactly 3.

The allocation wrapper routes this request through the shared small-block
allocator (`0x0090CF54`, `0x005E55B0`). That allocator adds four header bytes
and chooses the first power-of-two class starting at 8 which accommodates
the total (`0x005B9FC2..0x005BA005`): this request uses the 16-byte class,
with the returned pointer four bytes after the block header. It does not
reserve a fourth payload word. Null underlying allocation is followed by an
unchecked header store in this allocator; neither it nor the editor supplies
a recoverable empty-object result. No fixed subsequent heap corruption or
crash location is promised: neighboring block contents and allocator failure
policy are external state.

On successful record insertion, only the **three logical words** are copied
into the configuration record's separately owned vector. Copy construction
uses source end minus start, requests exactly that many dword bytes and copies
that range (`0x00478934..0x00478938`, `0x00462F00`; assignment on record
shifts uses `0x00462C20..0x00462C93`). The resulting type-7 payload is
`[0,0,0x02000000]`; the stray fourth store is not serialized as a fourth word
by virtue of this insertion. The editor then frees the temporary payload and
record before marking dirty and drawing (`0x00484CE2..0x00484D14`). A safe
replacement should represent the three-word result and expose any requested
native-overrun diagnostic separately, rather than perform an unsafe write.

`DoAddNewWaterConstraint` (`0x0047C8D0`) and
`DoAddNewTerrainConstraint` (`0x0047C930`) are mode commands, not immediate
object creation. If already in the requested mode (6 or 7), they do nothing,
even if pending payload remains. Otherwise they clear the pending payload's
logical size (`+0x1A4 = +0x1A0`) without freeing its capacity, install that
mode, and request cursor `+0x14C` with option true through window `+0x154`.
They do not change configuration records, selection, dirty or title, and do
not draw/invalidate. The apparent memory-move branches are unreachable
self-comparisons (`0x0047C8E2`, `0x0047C942`). Actual placement remains the
separate AddNewLotConfigurationObject operation.

### 25.2 Movement arithmetic and unconditional refresh

Keyboard movement (`0x00485040`) uses only modifier bits 0, 1 and 2.
Start with step 0.5 if bit 1 is set, otherwise 0.25; multiply by 4 for bit 0
and then by 8 for bit 2. The resulting steps for masks 0 through 7 are
respectively **0.25, 1, 0.5, 2, 2, 8, 4, 16**. All are exact binary32 values.
Left/right subtract/add the step from x and both x bounds. Up/down normally
subtract/add it from z and both z bounds. With Space down, Up/Down instead
add/subtract it from y **only for type 1**; other types do not translate.
Other keys do not translate. Every individual coordinate update stores to
binary32; no integer conversion or snapping occurs in this method
(`0x00485059..0x0048516B`).

Position movement (`0x004851B0`) computes target x minus old x and target z
minus old z in x87 registers. The deltas are **not spilled to binary32**:
each is added to the old center and the two corresponding bounds, and each
result is stored independently to binary32. It does not directly assign
the target coordinates or adjust y (`0x004851B4..0x004851F0`). With extreme
finite inputs, preserving the deltas' x87 precision can differ from rounding
them to binary32 before addition. No NaN rejection occurs before these
arithmetic operations; later boundary comparisons follow §9.6.

Both overloads always call the §9.2 boundary mover with option false for
type 4, true for all other types, and ignore its return. Then they set dirty
to 1, update title, draw and invalidate, in that order. This also happens
for an unrecognized key, for Space+Up/Down on an ineligible type, and when
the requested position equals the old position. These are observable
refresh/dirty quirks, not successful-translation tests
(`0x0048516E..0x00485196`, `0x004851F3..0x0048521D`).

### 25.3 Title composition

`SetTitle` (`0x0047F0B0`) obtains the current configuration's name through
its `+0x10` getter and sets the window text to `Lot Editor -- ` followed by
that name, followed by ` *` exactly when the dirty byte is nonzero. It then
invalidates the window, even when the new title equals the old title. It
does not use the identity path, file stem or selected object's name; an empty
configuration name leaves the prefix, plus the dirty suffix if applicable.
The configuration pointer is dereferenced without a null guard. Evidence:
name read/append `0x0047F164..0x0047F187`, dirty test/suffix
`0x0047F18C..0x0047F1A7`, text/invalidation `0x0047F1AC..0x0047F1BF`;
literal bytes at `0x00A8DCC0..0x00A8DCD2` are space, asterisk, NUL, NUL,
then `Lot Editor -- ` and NUL.

### 25.4 Conformance vectors

| Vector | Setup/action | Required observation |
|---|---|---|
| LER01 | Add a type-7 object with successful allocations; observe temporary payload requests/stores and inserted record | Request 12 bytes, vector size/capacity 3, fourth zero store at start+12 outside capacity; inserted record has exactly `[0,0,0x02000000]`. No mandated adjacent-memory consequence. |
| LER02 | Mode 2, pending words `[10,20]`, nonzero capacity; request water | Mode 6, empty pending payload with capacity retained, cursor `+14C` requested with true; no object or dirty/title/draw change. Request water again after supplying pending words: words remain. Terrain from mode 6 clears them and installs 7. |
| LER03 | Interior type 1, y=10, Space+Up, modifier mask 5; then Space+Down mask 7 | y becomes 18 then 2; x/z/bounds unchanged by translation. Each call still boundary-moves, marks dirty, updates title, draws and invalidates. |
| LER04 | Interior type 0, Space+Up, or unrecognized key | No initial translation; boundary mover and dirty/title/draw/invalidation still occur. |
| LER05 | Position move interior object x/z=8/8, bounds 7/7/9/9, target 12/14 | Before boundary adjustment: center 12/14, bounds 11/13/13/15, y unchanged. All result coordinates are binary32 stores; no integer snapping. |
| LER06 | Configuration named `Park`, unrelated identity filename; dirty byte 0 then 2 | Titles `Lot Editor -- Park` and `Lot Editor -- Park *`; each SetTitle invalidates. Empty dirty name yields `Lot Editor --  *` (two spaces before asterisk). |

## 26. Chooser dependencies and auxiliary popups *[compat]*

### 26.1 Directory-file retention and deletion scope

IID `0x856C8CF7` identifies the directory-file database-segment interface:
`cGZDBSegmentDirectoryFiles::QueryInterface` accepts precisely that IID on
its primary interface and AddRefs the returned pointer
(`0x009792AF..0x009792CE`). Other requests go through its persistent-segment
base at object+4; that base recognizes IID `0x656B8F0C` and unknown IID 1
(`0x009756AA..0x009756CE`). This is a file-backed resource provider, not
another editor or a notification sink. Its directory/key/path services are
the dependencies already used by §8.4 identity resolution. The chooser's
retained set pins these segment interfaces through its lifetime; it does not
enumerate configuration types from that set. List rows still come from the
configuration manager.

When the retained set is empty and global provider manager `0x00B43CA8`
exists, PostInit asks manager `+0x90` for an array size with a null array,
allocates count*4 bytes, and calls it again with that array and the count's
address. It processes the resulting count, querying each provider for
`0x856C8CF7`. A successful QueryInterface acquisition is inserted into the
pointer-ordered set **without another AddRef**; each original enumerated
provider is Released whether the query succeeds or fails. A nonempty set
suppresses future acquisition, while an empty result allows a later retry.
Shutdown Releases the stored queried references (§20.1). The insertion helper
copies the pointer value, not an owning smart reference
(`0x004804DE..0x00480580`, `0x0047C0C9..0x0047C0CB`).

Two native lifetime quirks follow from the instructions: this PostInit body
does not free the enumeration array, and it neither examines the set's
inserted/duplicate result nor Releases a successful query rejected as a
duplicate pointer. Such duplicate acquisitions are not balanced by a second
set entry at Shutdown. An implementation may diagnose these leaks without
deliberately leaking host memory. There is no null-allocation recovery in
this acquisition block. Missing provider manager skips it, but does not skip
the final list rebuild (`0x00480581..0x00480583`).

Deletion uses identity `+0x24`, the segment returned by manager `+0x80`,
retained separately from the queried directory-file reference at identity
`+0x28` (`0x0047B25D..0x0047B26E`). It passes the address of identity `+4`
to segment `+0x6C` (`0x0047EECF..0x0047EEDA`). Thus the exact requested
item is key **(type `0x6534284A`, group `0xA8FBD372`, selected configuration
ID)**, with the instance obtained through the configuration's ID getter,
not parsed afresh from a filename (`0x0047BAFE..0x0047BB23`). There is no
wildcard, group deletion, iteration over related exemplars or whole-archive
delete request in the chooser. In the directory-file backend, the base
`+0x6C` entry (`0x009758BD`, misleadingly exported as ReadRecord) dispatches
to `+0x98`, which is `DoDeleteRecord` (`0x0097AC93`). That backend requires
an enabled segment, a matching key and no open records for that entry;
otherwise it returns false. On success it erases that key's mapping, calls
file removal on that entry's path, and returns true **without testing the
file-removal result** (`0x0097AC9D..0x0097AD20`). A resource mapped to an
individual file therefore removes that file, not its containing directory
or another resource. Other segment backends retain their own record-storage
policy; the chooser still requests only the one key.

Grid sort/collation and text-search argument semantics remain the shared
grid dependency LE14. The raw §20.2/§20.5 calls establish neither an
unsearchable row zero nor a particular collation.

### 26.2 Foundation and outside-area popups

Both popups obtain the current pointer position, convert it for the editor,
show the string-list menu with final options zero/zero, and restore focus
to the editor after it returns, before dispatching the selected row. Rows
are zero based; cancellation or an unknown index performs no command.
The literal separator is `---`, with no action.

| Popup | Row | Display | Action |
|---|---:|---|---|
| Foundation (`0x00486930`) | 0 | `Set Lot Foundation...` | Query current first-foundation ID and open the foundation dialog with that ID and option zero. |
| Foundation | 1 | `Delete Lot Foundation` | If a first foundation exists, ask for removal as below; otherwise do nothing. |
| Foundation | 2 | `---` | No action. |
| Foundation | 3 | `Help... (F1)` | Invoke help with argument zero. |
| Outside (`0x00485B00`) | 0 | `Save (Control-S)` | Save with `(true,true)`. |
| Outside | 1 | `Save As... (Control-Shift-S)` | Save As with `(false,true,true)`. |
| Outside | 2 | `Save a Copy As... (Control-Alt-S)` | Save As with `(true,true,true)`. |
| Outside | 3 | `---` | No action. |
| Outside | 4 | `Help... (F1)` | Invoke help with argument zero. |

Foundation removal asks exactly `Are you sure you want to remove the lot foundation?`
through the editor message-box helper with option 3. Only result
**`0x5301814C`** accepts; do not substitute the `0x5301814A` used by chooser
deletion. Acceptance calls AddLotFoundation with `(0,0)` (§9.5), then sets
dirty, updates title, draws and invalidates. Cancellation does none of these.
The popup itself supplies no such refresh for the Set or Help rows: their
own method contracts apply. Evidence: `0x00486AC4..0x00486B57`,
`0x00485CDA..0x00485D23`.

### 26.3 Conformance vectors and remaining boundaries

| Vector | Setup/action | Required observation |
|---|---|---|
| LER07 | Empty service set; enumerate A supporting directory-file IID and B rejecting it; then PostInit again | Query both; Release both enumerated references; retain A's query reference without another AddRef. Second call skips enumeration and still rebuilds list. Shutdown Releases A once. |
| LER08 | Enumeration returns no supporting interface, then PostInit again | Empty set permits another enumeration; final list rebuild occurs on both calls. A null global provider manager also permits list rebuild. |
| LER09 | Two enumerated interfaces query to the same pointer P | Set stores P once; two query acquisitions, two enumerated Releases, no duplicate-query Release in PostInit, one set-member Release at Shutdown. Native diagnostic reports one unbalanced query acquisition. |
| LER10 | Chooser selects configuration ID `0x1234`; backing identity/file passes prechecks | Delete receives key `(0x6534284A,0xA8FBD372,0x1234)` on retained segment, not a request to remove a directory or the containing packed database. Only true removes the chooser row and unregisters the type. |
| LER11 | Foundation exists; choose Delete, message result `0x5301814A`, then repeat with `0x5301814C` | First attempt leaves foundation and dirty alone. Second removes through AddLotFoundation(0,0), then dirty/title/draw/invalidate. No foundation means no confirmation or mutation. |
| LER12 | Outside popup rows 1/2/3/4, or cancel | Respectively Save As(false,true,true), Save As(true,true,true), no command, Help(0), no command; focus restoration precedes dispatch. |

LE3/LE4/LE5/LE7 were broad stopping inventories, not four independently
complete algorithms. Their answered entries record the resolved contracts;
LE9–LE18 replace the remaining scope with numbered, narrower questions.
In particular this addendum does not establish the retaining-wall or
foundation selector bodies, SetLotSize arithmetic, selected-object popup
remainder, ordinary rasterization, PostInit view setup, remaining input/hit
conversion or exclusive ownership of shared helpers. §18's historical
attribution numbers remain unchanged.
