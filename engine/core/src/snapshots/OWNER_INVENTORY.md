# Snapshot owner contract inventory

Source: snapshots.md sections 1-4/6; docs/abi/vtables.tsv, layouts.md,
interfaces.tsv, allocators.md, direct-calls.md and unreconciled-rows.tsv.
Audit done before implementation, 2026-10-22. No native binding or full-owner credit.

## Complete named owner surface in the published spec

| Owner / named method (address) | Portable scope / residual |
|---|---|
| Taker CreateWinSnapshotTaker (007B7530) | singleton/show, optional size; native parent/window creation unresolved SN7 |
| Taker Init (007B7A80) | font/help resource identities; help composition unresolved SN7 |
| Taker GoToSizeIndex (007B6D90), AdjustPosition (007B6680) | supplied sizes, index clamp, inside-view placement; defaults/oversize unresolved SN6 |
| Taker GZOnKeyDown (007B6FF0) | Space, Esc, digits, Enter/modifiers/event; slot 70/71 native contracts unresolved SN7 |
| Taker GZOnMouseDownL (007B6610), GZOnMouseMove (007B6AD0), GZOnMouseDownR (007B6630) | portable drag/close; exact drag rules unresolved SN6 |
| Taker DoMessage (007BE0E0), capture helper (007BDE90) | fourth delivery, rectangle selection, image/album pipeline, modal completion |
| Taker TakeSnapshot (007B67A0) | typed capture primitive, opaque-alpha operation; native pixel masks unresolved SN8 |
| Taker Shutdown (007B6570), GZPaint (007B6B30), destructor (007B7910) | ABI names only; contract gap SN7 |
| Album CityNameToAlbumName (005F8970), AlbumExists (007B7990), CreateAlbum (007BB320) | safe-name 48-byte prefix, number, folder, empty ini; sanitizer/number semantics SN1/SN3 |
| Album update_list_of_album_directories (007BAEA0), album_directory_has_album_entries (007B8FB0) | root folder filter and sorted list; collation SN4 |
| Album get_path_for_info_file (007B9140), get_path_for_description_file (007B9280), ObfuscateData (007B7F20) | stem companions; XOR 7B |
| Album GetNewTempFileName (007B7F90) | city/date base PNG; collision suffix/region fallback unresolved SN3 |
| Album GetGraphicInfoForCurrentCity (007B8330) | ten LF-ended fields, decimal signed/unsigned fields; ninth field meaning SN1 |
| Album GetInformationOfStandardFile (007B93C0) | XOR/split/marker and structured caption fields; caption bytes SN2, malformed text SN5 |
| Album CreateSnapshotAlbumWindow (007BCE40), ShowSnapshotAlbum (007BDD60), DoModalWin (007B69E0) | portable instance/current-city selection/pause lifecycle; native UI script binding SN7 |
| Album on_init (007BC350), on_shutdown (007BAD30) | state initialize/save/clear; native controls SN7 |
| Album RefreshGraphicList (007BBD70), SetInitialFile (007BC120), SetCurrentFile (007BA9D0), GoToGraphic (007BA780) | typed lists/selection/load; ordering/initial choice SN4 |
| Album set_image_count_text (007B7230), set_to_no_image_file (007B72D0) | `(i/n)` and explicit no-image state |
| Album GoToNextGraphic (007BC910), GoToPreviousGraphic (007BCA60) | save before navigation; endpoint rule SN4 |
| Album DeleteCurrentGraphic (007BCBB0), DeleteAlbum (007BB5C0) | image/companions and all album files/folder; next selection/nested folder rules SN4/SN9 |
| Album on_button_activated (007BCFC0), DoWinMsg (007BDAE0), end_dialog (007B6510) | typed browse/save/zoom/close/select actions; raw message/control IDs SN7 |
| Album GoToAlbum(name,n,create) (007BD200), convert_album_directory_name_to_album_index (007B6CF0) | naming/create/exact index, -1 if absent |
| Album GoToAlbum(index) (007BC820), GoToAlbumForCurrentCity (007BDC90) | list index/current-city context selection |
| Album AddNewGraphic (007BD3D0), AddNewGraphicForCurrentCity (007BD9A0) | ensure/copy-or-retain/move/metadata/select; failure rollback SN9 |
| Album destructor (007B7970) | ABI name only; native lifetime SN7 |

## ABI method and byte inventory

The taker table has 151 slots (0-150), 604 vtable bytes. Object allocation is
0x10C bytes; the percentage field is at +0xE8 (SN1). The album table has 6 slots
(0-5), 24 vtable bytes; no object byte size/layout is published in the read sources.
Native window IDs are taker 0x85202C0E and album 0x0A8CD3EE.

All owner-target rows (stack bytes exclude this; returns are ABI evidence, not
portable Rust return promises):

| Owner | Slot | Stack bytes | Target | Return evidence |
|---|---:|---:|---|---|
| Taker | 3 | 4 | DoMessage | eax-or-void |
| Taker | 4 | 0 | Init | eax-or-void |
| Taker | 5 | 0 | Shutdown | eax-or-void |
| Taker | 88 | 0 | GZPaint | eax-or-void |
| Taker | 130 | 8 | GZOnKeyDown | eax-or-void |
| Taker | 134 | 12 | GZOnMouseDownL | via-call (low confidence) |
| Taker | 135 | 12 | GZOnMouseDownR | eax-or-void |
| Taker | 138 | 12 | GZOnMouseMove | eax-or-void |
| Taker | 148 | 4 | destructor | eax-or-void |
| Album | 4 | 20 | DoWinMsg | eax-or-void |
| Album | 5 | 4 | destructor | eax-or-void |

The other 142 taker slots target cGZWin (122), icf (15), cGZPersistResource (3),
cSCEditValueDlg (1), cRZString (1). All are residual toolkit/interface surface,
not snapshot owner implementations. The other 4 album slots are QueryInterface,
AddRef, a cRZAutoRefCount target and DoWinProcMessage. They remain unbound.
The exact extracted 157-row inventory is retained in build-runs/snapshot-abi-inventory.tsv.
No owner rows were found in the other six ABI sources checked above.

## File bytes and output limits

- PNG/image bytes: codec host primitive, no codec claim. Companion stems `.info`, `.txt`.
- `.info`: XOR each byte with 0x7B. Ten LF-ended fields: `SanityCheck`, user,
  city, mayor, signed day, date text, signed population, signed 64-bit funds,
  signed ninth value, unsigned `zoom,rotation,x,y`. Unknown field meaning SN1;
  non-ASCII wire encoding, embedded LF and malformed reader rules SN5.
- Description: plain text bytes; empty removes companion. No newline added.
- `album.ini`: empty at creation. Extensions: png/jpg/jpeg/jp2/gif/bmp.
- Count output: `(i/n)`; empty view represented separately.
- Native paint, control binding, captions, slot semantics and all ambiguous
  return channels remain explicit residuals. There is no fullownercredit.
