# The clipboard manager (`cGZClipboardManager`) (behaviour spec)

Scope: the owners `cGZClipboardManager` (32 functions, 2,282 bytes), `cGZClipboardManagerClipboard` (36, 3,856) and
`cGZClipboardManagerClipboardData` (5, 141) as tools/coverage.py assigns them on `bb641878` (inventory in §6). They
are the framework's clipboard service: a manager holding clipboards by ID, each clipboard a map of typed data
objects, and clipboard 0 mirrored to the Windows clipboard as text. The text edit uses it for cut, copy and paste
(text-edit.md, its clipboard reference `+0x28C`).

Related:
- system-service-pointer.md: the accessor `0x004D7B30` (service `0x27EBFFFD`, IID `0x07EC0010`);
- modal-dialogs.md §2.5: the legacy message server (`RZ::MS()`), whose slot 3 sends a message at once;
- text-edit.md: the caller.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the Windows write path's branches and the stub
slots were checked on the instructions. The vtables `0x00ACFEE0` (`cIGZClipboardManager`, 21 slots), `0x00ACFEAC`
(system service), `0x00ACFE94`, `0x00ACFE40` (`cIGZClipboard`, 21 slots) and `0x00ACFDCC` (clipboard data, 7 slots)
were read. Written 2026-10-24 (re-gzclip7).

Conventions:
- "Text" is data type `0x67EC001D` whose value is a string (`cIGZString`, IID `0x089B7DC8`). Clipboard IDs: 0 is
  the system clipboard; −1 in a write means "every clipboard", −2 in a read "the first that succeeds"; −3
  (`0xFFFFFFFD`) is "none".
- Every manager and clipboard slot takes the object's lock (`cRZCriticalSection`) for its whole body.

## 1. The manager *[compat]*

Fields: `+0x04` the system service (`cRZSystemService`, ID `0x27EBFFFD`, priority `0x131334`; its slots 8 and 9 are
thunks to `Init`/`Shutdown`), `+0x08` a COM base table (`0x00ACFE94`, slot 3 the deleting destructor), `+0x20`
initialised (byte), `+0x24` reference count, `+0x28`/`+0x2C` the clipboards (map ID → clipboard) and their count,
`+0x34` the lock. Constructor `0x00923272`; deleting destructor `0x009232C0`.

| Slot | Address | Behaviour |
|---|---|---|
| 0 `QueryInterface` | `0x0092254E` | `0x07EC0010` → itself, AddRef'd, true; else the system service's |
| 1 / 2 | `0x0092259C` / `0x0092270F` | AddRef (`+0x24`). Release at a count below 2: `Shutdown` and delete (returns 0); else decrement |
| 3 `Init()` | `0x00923367` | once: `new cGZClipboardManagerClipboard` (`0x30` bytes), AddRef, `SetID(0)` (its slot 6), its `Init`, added as ID 0 |
| 4 `Shutdown()` | `0x0092251A` | if initialised: `RemoveClipboard(−1)`, initialised := 0 |
| 5 `GetClipboardCount()` | `0x009227F9` | `+0x2C` |
| 6 `GetClipboard(id)` | `0x009230A8` | the clipboard or null (not AddRef'd) |
| 7 `GetClipboards(out array, in/out n)` | `0x00922974` | null array → `n :=` the count; else up to `n` clipboards, each AddRef'd, `n :=` the number copied; returns `n` |
| 8 `AddClipboard(c)` | `0x009233DC` | `RemoveClipboard(c.GetID())`, AddRef, `c.Init()`, insert under its ID |
| 9 `RemoveClipboard(id)` | `0x0092314F` | −1: every clipboard gets `Shutdown` and Release, the map is emptied, true. Else that one, true; not found → false |
| 10 `RemoveClipboard(c)` | `0x009230E4` | by pointer, as slot 9; not found → false |
| 11 `IsClipboardDataTypeAvailable(type, id)` | `0x009229F3` | clipboard slot 10; `id` −2 → any clipboard |
| 12 `GetAvailableClipboardDataTypes(out, in/out n, id)` | `0x00922A66` | clipboard slot 11; `id` −1 → all clipboards appended in ID order, `n :=` the total; returns `n` |
| 13 `GetClipboardWithDataTypeAvailable(type)` | `0x00922AFA` | the first ID whose clipboard has the type, else −3 |
| 14 `GetClipboardData(type, iid, out, id)` | `0x00922B4B` | clipboard slot 12; `id` −2 → the first true |
| 15 `SetClipboardData(data, clear, id)` | `0x00922BCA` | clipboard slot 14; `id` −1 → every clipboard, and the result is **false** |
| 16 `ClearClipboard(type, id)` | `0x00922C3B` | clipboard slot 15; `id` −1 → every clipboard, result false; one clipboard → true |
| 17 (buffer form) | `0x00922D8A` | clipboard slot 16 `(a, b, c)`; `id` −2 → the first true |
| 18 `GetClipboardText(out string, id)` | `0x00922CA6` | clipboard slot 17; `id` −2 → the first true |
| 19 (buffer form) | `0x00922E09` | clipboard slot 18 `(a, b, c, 1)`; `id` −1 → all, result false |
| 20 `SetClipboardText(text, clear, id)` | `0x00922D19` | clipboard slot 19; `id` −1 → all, result false |

The buffer forms (slots 17 and 19) reach clipboard stubs that return false (§2), so with the shipped clipboard they
always fail. Thunks for the other two tables: `0x00923068`, `0x00923070`, `0x00923078`, `0x00923080`, `0x00923088`,
`0x00923090`, `0x00923098`, `0x009230A0`, `0x0092343C`.

## 2. A clipboard (`cGZClipboardManagerClipboard`, IID `0x87EC000B`) *[compat]*

Fields: `+0x04` initialised (byte), `+0x05` notify (byte), `+0x08` reference count, `+0x0C` ID (constructor −3),
`+0x10` a nesting count that silences change notices, `+0x14` change counter, `+0x18` the Windows bridge (§3, only
for ID 0), `+0x1C`/`+0x20` the data (map type → data object) and its count, `+0x28` the lock. Constructor
`0x0092321E`; destructor body `0x00922ECE`, deleting form `0x00923256` (slot 20).

| Slot | Address | Behaviour |
|---|---|---|
| 0 | `0x009223F2` | IIDs 1 and `0x87EC000B` → itself, AddRef'd |
| 1 / 2 | `0x00922436` / `0x0092264B` | AddRef; Release at a count below 2: `Shutdown`, then slot 20 `(1)` (delete), 0 |
| 3 `Init()` | `0x00923D16` | once: initialised := 1, the bridge (ID 0), then the notice `send(1)` |
| 4 `Shutdown()` | `0x009228BD` | if initialised: `send(0)`, `Clear(−1)`, destroy the bridge, initialised := 0 |
| 5 `GetID()` / 6 `SetID(id)` | `0x009CEDE1` (shared) / `0x00923CF6` | `SetID` stores; when initialised, ID 0 creates the bridge and any other ID destroys it |
| 7 `IsNotifying()` / 8 `SetNotify(on)` | `0x0093998F` (shared) / `0x0092245D` | `+0x05` |
| 9 `GetChangeCount()` | `0x009D4714` (shared) | `+0x14` |
| 10 `IsDataTypeAvailable(type)` | `0x00922EE8` | |
| 11 `GetAvailableDataTypes(out, in/out n)` | `0x00922908` | as manager slot 7, with the types |
| 12 `GetData(type, iid, out)` | `0x00922F5C` | the data object's `QueryInterface(iid, out)`; no data → false |
| 13 `GetDataObject(type)` | `0x00922F20` | raw pointer or null |
| 14 `SetData(data, clear)` | `0x00923AE5` | below |
| 15 `Clear(type)` | `0x00922FA6` | nothing when empty. −1: release every object, empty the map; else that type (not present → nothing). Then the change counter + 1 and `post(type)` |
| 16, 18 | `0x009224DC`, `0x009224FB` | false (buffer forms not implemented) |
| 17 `GetText(out string)` | `0x0092246B` | the text object's value (slot 5) as a string, copied into `out` (its slot 14); true when found |
| 19 `SetText(text, clear)` | `0x009232DC` | a new `cRZString(text)` wrapped in a new data object of type text, then `SetData(that, clear)` |

**`SetData(data, clear)`**: with `clear` and a non-empty map, `Clear(−1)` with the notices silenced. With `data`:
`type := data.GetType()`; an existing object of that type is cleared (silenced); `data` is stored (AddRef'd). For
clipboard 0, type text and a bridge, the value as a string is written to Windows (§3) with the lock released around
the call. Then the change counter + 1 and `post(type)` (or `post(−1)` when only `clear` acted); with neither `data`
nor `clear` nothing happens.

**Notices** (message `0x87EC0025`, only while notify is on): `send(flag)` (`0x0092268E`) delivers `{0x87EC0025,
clipboard, ID, flag}` at once through the legacy message server's slot 3; `post(type)` (`0x009226C9`) queues
`{0x87EC0025, clipboard, ID, type}` through its slot 4 *[inferred: queued]*, and only when nothing silences it and
`type` is not 0.

## 3. The Windows bridge (clipboard 0) *[compat]*

Created by `0x00923CC8` (`0x1C` bytes, constructor `0x00923BFD`) and destroyed by `0x009227D7` (`ChangeClipboardChain`,
`DestroyWindow`). Fields: `+0x00` enabled (1), `+0x04` reading and `+0x08` writing counts, `+0x0C` the clipboard,
`+0x10` its window, `+0x14` the next clipboard viewer, `+0x18` the bytes `09 04`. The constructor registers the class
`"Win32ClipboardSupportClass"` (window procedure `0x00923A0F`, 4 extra bytes) and creates the hidden window
`"Win32ClipboardSupport"`, stores the bridge in its user data and joins the viewer chain (`SetClipboardViewer`).

- **Window procedure**: without a bridge or when disabled, `DefWindowProc`. `WM_CHANGECBCHAIN` (`0x30D`): the next
  viewer is replaced when it is the one removed, otherwise the message is forwarded. `WM_DRAWCLIPBOARD` (`0x308`):
  read the Windows text (below); on success, with the reading count raised, `SetText(text, 1)` on the clipboard; then
  forward to the next viewer. Both return 0; other messages go to `DefWindowProc`.
- **Read** (`0x00923444`), only when enabled and not writing: `OpenClipboard`. With `CF_UNICODETEXT` the UTF-16 text
  up to its terminator is taken (converted from code page 16 to the framework's code page unless that is 16); else
  with `CF_TEXT` the bytes up to the terminator, converted from the system code page. True when the result is not
  empty. `CloseClipboard`.
- **Write** (`0x00923760`), only when enabled and not reading: `OpenClipboard`, `EmptyClipboard`, then `CF_TEXT`
  (the text in the system code page) and `CF_UNICODETEXT` (UTF-16), each in a moveable global block with a
  terminator (freed when `SetClipboardData` refuses it). When the framework code page is 16 a first pass sets the two
  formats before the general pass sets them again *[quirk]*. `CloseClipboard`; true.

Changes the game makes to clipboard 0 are written straight through; the bridge's own `SetText` on
`WM_DRAWCLIPBOARD` runs while the reading count blocks a write back.

## 4. The data object (`cGZClipboardManagerClipboardData`, IID `0x07EC0006`) *[compat]*

`0x10` bytes: vtable `0x00ACFDCC`, reference count, type, value (counted). Constructor `(type, value)` `0x009227AD`.
Slots: 0 `QueryInterface` (`0x009223A7`: IIDs 1 and `0x07EC0006`), 1 / 2 the shared reference counting, 3 `GetType`
(shared), 4 the value's `QueryInterface` (`0x009223E0`; false without a value), 5 `GetValue` (shared), 6 the
deleting destructor (`0x0092262F`; the body `0x00922394` releases the value).

## 5. Quirks and cases

Quirks:
1. The manager's "every clipboard" writes (`id` −1 in slots 15, 16, 19, 20) return false even when they succeed.
2. The buffer-form text slots always fail (§1).
3. With framework code page 16 the Windows write sets each format twice (§3).
4. `GetClipboard` and `GetDataObject` return pointers without AddRef.

Cases:
1. Copy in a text edit: `SetClipboardText("abc", 1, 0)` → clipboard 0 `SetText` → `SetData` clears the old data,
   stores the text, writes `CF_TEXT` and `CF_UNICODETEXT`, and (only with notify on, which the constructor leaves
   off) posts `{0x87EC0025, clip, 0, 0x67EC001D}`.
2. Another program copies "x": `WM_DRAWCLIPBOARD` → read "x" → `SetText("x", 1)` (no write back while reading).
3. Paste: `GetClipboardText(s, −2)` → clipboard 0's text into `s`; true.

## 6. Function inventory

Owner `cGZClipboardManager` (32): `0x0092251A`, `0x0092254E`, `0x0092259C`, `0x0092270F`, `0x009227F9`,
`0x00922974`, `0x009229F3`, `0x00922A66`, `0x00922AFA`, `0x00922B4B`, `0x00922BCA`, `0x00922C3B`, `0x00922CA6`,
`0x00922D19`, `0x00922D8A`, `0x00922E09` (slots, §1), thunks `0x00923068`, `0x00923070`, `0x00923078`, `0x00923080`,
`0x00923088`, `0x00923090`, `0x00923098`, `0x009230A0`, `0x0092343C`, `0x009230A8` slot 6, `0x009230E4` slot 10,
`0x0092314F` slot 9, `0x00923272` constructor, `0x009232C0` deleting destructor, `0x00923367` `Init`, `0x009233DC`
slot 8.

Owner `cGZClipboardManagerClipboard` (36): `0x009223F2`, `0x00922436`, `0x0092245D`, `0x0092246B`, `0x009224DC`,
`0x009224FB` (slots), `0x009225C3` bridge teardown, `0x009225E2` (`"basic_string"` length error), `0x009225EE` UTF-16
append helper, `0x0092264B` Release, `0x0092268E` `send`, `0x009226C9` `post`, `0x00922753`, `0x0092286B`,
`0x00922E84` (map destruction), `0x009227D7` destroy the bridge, `0x009228BD` `Shutdown`, `0x00922908` slot 11,
`0x00922EB2`, `0x009231EA` (string buffer helpers), `0x00922ECE` destructor body, `0x00922EE8` slot 10, `0x00922F20`
slot 13, `0x00922F5C` slot 12, `0x00922FA6` slot 15, `0x0092321E` constructor, `0x00923256` slot 20, `0x009232DC`
slot 19, `0x00923444` read, `0x00923760` write, `0x00923A0F` window procedure, `0x00923AE5` slot 14, `0x00923BFD`
bridge constructor, `0x00923CC8` create the bridge, `0x00923CF6` slot 6, `0x00923D16` `Init`.

Owner `cGZClipboardManagerClipboardData` (5): `0x00922394`, `0x009223A7`, `0x009223E0`, `0x0092262F`, `0x009227AD`.

Not covered: `ConvertStringEncoding` and the code-page queries (framework string services); the text edit's own use.
