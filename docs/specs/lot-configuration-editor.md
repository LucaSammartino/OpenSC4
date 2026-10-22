# Native lot configuration editor

Source: SimCity 4 Deluxe GOG 1.1.641, Windows executable; export and
`decomp/cSC4WinLotConfigurationEditor.c` read from the main checkout, 2026-10-22.
This is the native `cSC4WinLotConfigurationEditor`, class ID `0x4918C704`
(registration at `0x00B08A28`). It is separate from the resource/exemplar/value
editors in [resource-exemplar-editor.md](resource-exemplar-editor.md).

**Coverage status: partial, no complete-owner spec credit.** The fresh owner
inventory has 111 entries, including 92 named editor methods. The named bodies
sum to 51,054 bytes; the reach-attributed remainder must also be reconciled.
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
its dirty-dependent suffix remains subject to the string-byte reconciliation.

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

## 4. Lot compatibility dialog: established subset *[compat]*

`DoSetLotCompatibilitiesUI` (`0x00480D10`, 6,818 bytes) creates script resource
`(type=0xE9264B49, group=0x96A006B0, instance=0xE9263D4D)`
(`0x00480D25..0x00480D58`). Creation failure bypasses all model changes.
It registers the created window with the modal service, runs it through the
editor's dialog host, unregisters it and restores focus. Only modal result
`0x42B7C356` enters the write-back branch (`0x004823B1..0x004823F2`).

The following numeric/control rules are instruction-verified; property names
are not assigned without a confirmed interface mapping:

| Child ID / queried interface | Initial source | Accepted write-back |
|---|---|---|
| `0x294053DB` / `0x8810` | Configuration virtual `+0x38`, to child `+0x24`. | Not yet reconciled. |
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

Three multi-selection bit-vector controls and their complete row-to-bit maps
are unresolved: [LE1](questions/lot-configuration-editor.md). The corrupted
stack-based pseudocode is not a sufficient basis for those mappings.

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

Full exact owner inventory, lifecycle, I/O, selectors, mouse/keyboard,
rendering and numeric workflows are being reconciled in this lane. This draft
does not claim those unrecorded contracts. No ABI changes or game-run evidence.
