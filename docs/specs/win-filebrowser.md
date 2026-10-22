# File browser window (`cGZWinFileBrowser`)

Target: SimCity 4 Deluxe, Windows GOG 1.1.641. RE lane `re-filebrowser4`,
2026-10-22. This is a **partial owner specification**. Address citations refer
to that executable; they are evidence anchors, not implementation requirements.
Only the contracts explicitly described below are specified. No implementation,
shadow comparison, or in-game verification is claimed.

## 1. Scope, interfaces and evidence inventory

This owner combines a window with directory navigation, a file grid, an optional
filename editor, file filters and selection notifications. Base window behavior
comes from [widget-toolkit.md](widget-toolkit.md); grid behavior comes from the
separate grid specification. This document does not extend either owner.

The preserved coverage inventory contains **100 candidate functions / 25,579
bytes**, including reachability, neighbour and compiler-helper assignments. That
is an attribution inventory, **not a count of fully specified browser methods**.
The audit also found primary-interface targets missing from that inventory.
The final inventory below must distinguish those sets; do not assign complete
owner coverage from the preserved count.

### 1.1 Interface identity and table boundaries

The browser interface is at object offset zero. Querying IID `0x3AA52E64`
returns that interface and adds a reference. Other IIDs are delegated to the
base window at object offset `+4` (`0099FC88`–`0099FCA9`). The window-interface
QueryInterface and Release thunks subtract four before forwarding (`0047AA30`,
`0047AA40`). A caller must release a successful query result.

The window-interface table at `00A8D000` has 151 exported slots (0–150).
Browser-specific overrides include Init (4), Shutdown (5), DoMessage (3),
SetArea (`009A0089`), GZPaint (`009A0A17`), and the destructor thunk at 148;
inherited entries retain their base-window meanings. The primary table starts
at `00ADD190` and has **107 pointer entries, slots 0–106**. Its next address,
`00ADD33C`, is a separate comparator pointer used by `009A32AB`, not browser
slot 107. The raw table bytes and that reference delimit the table. The primary
table is absent from the current ABI table; this document does not modify ABI
rows or invent signatures for unidentified slots.

Confirmed primary entries used below:

| Slot | Target | Operation |
|---:|---|---|
| 0 | `0099FC88` | QueryInterface |
| 1 | `0094D34F` | shared AddRef |
| 2 | `0099FC70` | Release |
| 19 | `009A423E` | SetSelection |
| 30 | `009A2BC4` | GoToParentDirectory |
| 31 | `009A44A8` | set current directory, existence checked |
| 32 | `009A451A` | CreateNewDirectory |
| 91 | `009A1F77` | AddControls |
| 95 | `009A3AF3` | UpdatePath |
| 96 | `009A4EBA` | UpdateFileList |
| 97 | `009A4B2E` | UpdateDirectoryList |
| 98 | `009A473F` | PullDriveListDown |
| 100 | `009A3447` | HandleFileListDoubleClick |
| 102 | `009A367F` | HandleSelectionChange |
| 103 | `009A3867` | HandleSelectionDoubleClick |

## 2. Navigation and path updates

### 2.1 Set current directory (`009A44A8`, 114 bytes)

Input is a string naming a directory. Copy it to a temporary string and check
directory existence first. A failed check returns false without replacing the
current directory or calling UpdatePath. A successful check copies the string
into the current-directory state, calls UpdatePath, and returns true. This
operation does not require the selected filename to exist.

### 2.2 Parent navigation (`009A2BC4`, 107 bytes)

Split the current directory with capacity for two 260-byte path components.
If the returned component count is at most one, return false without calling
the directory setter. Otherwise pass the second component to the primary
directory setter (slot 31), then return true. The setter's Boolean result is
not propagated. The signed count comparison, capacity and discarded result
are visible at `009A2BCE`–`009A2C2A`.

### 2.3 UpdatePath (`009A3AF3`, 116 bytes)

Check the stored current directory. If it no longer exists, replace it with
the stored fallback-directory string; this method does not recheck the
fallback. Synchronously send notification `0x7ABE7205` to the window's
notification target. Its four 32-bit words are message ID, this window's ID,
a borrowed pointer to the current-directory string object, and zero. Then
call UpdateDirectoryList followed by UpdateFileList and return true.

Ordering matters: the notification precedes both list updates. The string
belongs to the browser; a receiver retaining its value must copy it. Evidence:
the string fields at `+0x170`/`+0x184`, send at `009A3B46`, then calls at
`009A3B50` and `009A3B5A`. These offsets are relative to the primary interface.

## 3. Entry lookup

### 3.1 GetRowOfEntryByName (`009A2CCF`, 215 bytes)

Input is a string. If there is no file grid or the name column is disabled,
return zero. Otherwise obtain the grid's reported row range, add one to its
last-row value with 32-bit wraparound, and use that as the exclusive scan
bound. A bound of zero or `0xFFFFFFFF` is replaced with 1024. Any other
nonpositive signed bound means no iterations. Scan from row zero upward,
reading the configured name-column text. Skip failed cell-text reads; return
the first row whose text equals the input. There is no lowercasing in this
method. Return zero when nothing matches.

Thus zero is ambiguous: it can mean the first row, no match, a disabled column,
or a missing grid. This is not a search API with a negative not-found sentinel.
The range arithmetic and signed loop are at `009A2D14`–`009A2D6A`; the equality
helper call is at `009A2D59`.

## 4. Conformance cases

These are contract vectors for an independent implementation, not results of
running the original game. Use an instrumented directory service/grid/window
notification target so ordering and omitted calls can be observed.

| ID | Setup/input | Required observation |
|---|---|---|
| FB-C01 | Query IID `0x3AA52E64` through either browser or window interface | Same primary interface; one added reference; result is caller-owned. |
| FB-C02 | Set current directory to a nonexistent path | False; current directory unchanged; no UpdatePath call. |
| FB-C03 | Parent splitter returns one component | False; no directory-setter call. |
| FB-C04 | Parent splitter returns two components; overridden setter returns false | Setter receives component 2; parent navigation still returns true. |
| FB-C05 | Current directory has disappeared; fallback also does not exist | Fallback is copied without a second existence check; path notification precedes directory-list update, which precedes file-list update. |
| FB-C06 | Name exists at row 0; repeat with name absent | Both calls return 0. |
| FB-C07 | Name at rows 1 and 3; row 0 text read fails | Returns 1; failed read is skipped. |
| FB-C08 | Range last row is `0xFFFFFFFE` or `0xFFFFFFFF` | Both cases scan with bound 1024; no accidental empty or unbounded scan. |
| FB-C09 | Grid absent, or name column disabled | Returns 0 without attempting cell-text reads. |

## 5. Browser flags

The browser flag word is separate from the base window flags: it is at primary
interface offset `+0x160`. Primary slot 6 (`0099FD1F`, 140 bytes) replaces the
whole 32-bit word. Equal values do nothing. Otherwise store the new value before
performing side effects. If bit 2 changed, use the base window SetFlag operation:
when the new bit is set, clear window bits `0x20000` and `0x40000`, then set
`0x10000`; when cleared, set `0x20000` and `0x40000` (do not additionally clear
`0x10000`). Finally invalidate the window. This path takes precedence over
changes in browser bits 1, 4 and 8. If bit 2 did not change but any of those
three bits changed, call UpdatePath instead. Other bits are stored without
either side effect. Evidence: `0099FD22`–`0099FDA1`.

Primary slot 5 (`0099FD04`, 27 bytes) is an unusual add-flags operation: if
**any** requested bit is already present, it does nothing, even if other
requested bits are absent. Only when the intersection is zero does it call
slot 6 with the bitwise union (`0099FD04`–`0099FD18`). It is not an unconditional
OR setter. For example, existing flags 1 and request 5 leave flags at 1.

## 6. Selection, entries and notifications

### 6.1 Entry types and selection state

The visible file rows refer to owned entry records. The record begins with a
32-bit tag, a 32-bit associated ID, another word, and a string at byte offset
12. Recognized tags in the selection handlers are:

| Tag | Meaning in these handlers | Selection type | Associated ID |
|---|---|---:|---|
| `0xBAA88501` | parent entry | 0 | 0 |
| `0xBAA88502` | directory entry | 1 | 0 |
| `0xBAA88503` | file entry | 2 | entry's second word |
| `0xBAA88513` | additional file-like entry | 2 | entry's second word |

Selection state contains type (`+0x320`), ID (`+0x324`), full path (`+0x328`),
directory (`+0x33C`) and leaf name (`+0x350`), relative to the primary interface.
Directory is copied from the current directory. For parent/directory entries,
append a backslash to the leaf only when it does not already end in a path
separator; files keep their leaf unchanged. Full path is current directory
followed by that leaf. The parent entry is type zero, not type one.

HandleSelectionChange (`009A367F`, 488 bytes) returns false for a null entry or
an unrecognized tag. A recognized entry replaces the selection, records its
row at `+0x22C`, copies a file leaf into the filename editor when one exists,
and sends `0x7ABE7203`, then returns true. HandleSelectionDoubleClick
(`009A3867`, 444 bytes) builds the same selection and sends `0x7ABE7204`;
it does not perform the change handler's row assignment or editor update.
Neither handler checks the row against the entry-vector size before reading
the entry pointer. Callers must supply an in-range row; behavior for invalid
indices is not a safe failure contract.

The notification has four 32-bit words: message ID, browser window ID, pointer
to a temporary selection descriptor, zero. Sending is synchronous. The
temporary's lifetime ends after the handler returns; receivers must copy any
value they retain. The descriptor has five 32-bit words: selection type,
associated ID, pointer to full-path string object, pointer to directory string
object, pointer to leaf-name string object. It is a snapshot of the two
numbers plus borrowed pointers into browser state; it owns no strings.
Primary slot 90 (`009A00FC`, 53 bytes) fills this caller-provided descriptor
and returns true, without AddRef or string copying (`009A00FC`–`009A0130`).
The entry ID is in the descriptor, not the message's fourth word. Tags and
field widths are byte-verified at
`009A3689`–`009A36D7`; change-message construction/send at `009A3817`–`009A3854`.
Later browser mutations can change the strings addressed by an earlier
descriptor. A receiver needing an immutable selection must copy their values.

### 6.2 File-grid double-click (`009A3447`, 156 bytes)

Unlike the selection handlers, this operation first compares the input row as
unsigned against the number of entry pointers. Out-of-range returns false.
For an in-range parent entry, call GoToParentDirectory. For a directory entry,
combine current directory with its name and a trailing backslash, and pass
that to the directory setter. Every in-range row returns true, including file
or unknown tags, and the navigation result is ignored. This method alone does
not send the file-selection-double-click notification; message dispatch can
invoke both operations. Evidence: `009A3447`–`009A34E2`.

### 6.3 Programmatic selection (`009A423E`, 359 bytes)

Split the supplied path into drive, directory, basename and extension. Check
the drive-plus-directory part for existence. Failure returns false before
replacing selection state. Success sets selection directory to that part,
leaf to basename plus extension, full path to their concatenation, type to 2,
and current directory to the selected directory. It marks the primary dirty
byte `+0x31C`, invalidates the window, and returns true. It does not test file
existence, clear or assign the associated ID, refresh the filename editor, or
send the change notification in this body. The ID can therefore remain from
an earlier selection. Do not add a file-existence check to this operation.

## 7. Directory creation (`009A451A`, 549 bytes)

Use the configured new-directory base name, appended to the current directory
and followed by a backslash. If it already exists, try the base name with
` (2)` appended, then ` (3)`, up through ` (256)`, stopping at the first name
whose directory-existence check is false. If all exist, stop the search at
256 anyway. Always attempt CreateDirectory on the final candidate; an
exhausted search does not itself return false. Failed creation returns false
without refreshing either list. Successful creation refreshes the directory
list and then the file list. If a file grid exists, use GetRowOfEntryByName
on the chosen leaf, scroll to that row, and request selection if not already
selected. The ambiguous zero result of that lookup is not specially handled.
Return true after successful creation. The upper bound is a signed compare
with 256 before increment (`009A45ED`–`009A45F9`); creation still occurs at
`009A467D`, and list refresh order is at `009A468F`/`009A4699`.

## 8. Largest owner bodies: population, construction and resources (partial)

### 8.1 File population (`009A4EBA`, 2,241 bytes)

Clear the file-grid contents and old owned file-entry records before rebuilding.
Browser flag 4 enables directory entries and flag 8 enables filesystem files.
Directories are enumerated separately and alphabetized before entry records
are produced. A parent entry is inserted only when SplitDirectory reports at
least two components and browser flag 1 is enabled. Its displayed name comes
from the configured parent-entry string. Directory entries have tag
`0xBAA88502`; parent uses `0xBAA88501`; enumerated files use `0xBAA88503` and
have associated ID zero. File-entry display names retain filename and extension.

When there are no filter groups, accept every enumerated file. With groups,
use the selected group's patterns; an empty pattern vector also accepts every
file. Otherwise accept a file if **any** pattern matches through WildcardMatch,
passing the configured match option. An empty matching result omits that file.
Directories do not pass through these filename-pattern tests. This is not
intersection-of-patterns filtering. The filter group's index is used without
a local bounds check. The exact directory-service filter syntax and wildcard
option semantics are outside the proved portion (FBQ5).

After enumeration the entry vector is sorted, then grid rows are populated
with optional icons, names and extension text. File extension text has a
leading dot removed when there is at least one following character, and is
lowercased. A column's configured font overrides the common font; otherwise
the common font is used. Display strings are fitted to column width before
being installed in cells; stored entry names remain available separately.
The exact comparator, complete column sizing and pending-selection restore
are not specified here (FBQ5). These are substantive missing contracts,
not permission to choose a new ordering or auto-selection policy.

### 8.2 AddControls (`009A1F77`, 3,149 bytes)

Each enabled control is obtained from the component factory first; if that
request fails, construct the built-in control, add a reference to the returned
interface, configure it and add its window to the browser as a child. The
browser retains its interface reference as well as the window hierarchy's
ownership. A grid can be disabled independently of the filename editor.

The observed control IDs and types are below. Enable bytes and retained
interface offsets use the primary browser origin.

| Enable byte | Interface field | ID | Type/use |
|---|---|---|---|
| `+0x11C` | `+0xE8` | `0xBAA55D50` | directory/drive grid |
| `+0x11E` | `+0xF0` | `0xBAA55D52` | dropdown arrow button |
| `+0x121` | `+0xFC` | `0xBAA55D55` | file grid |
| `+0x11F` | `+0xF4` | `0xBAA55D53` | button |
| `+0x120` | `+0xF8` | `0xBAA55D54` | button |
| `+0x122` | `+0x100` | `0xBAA55D56` | filename text editor |
| `+0x125` | `+0x10C` | `0xBAA55D59` | Cancel button |
| `+0x126` | `+0x110` | `0xBAA55D5A` | Look in label |

Grids request component `0xDAA6B9BF`, interface `0xDAA6B9BE`; buttons request
`0x22ECFC47`/`0x00008810`; the editor requests `0x231A1C58`/`0x231A1C57`.
The arrow first tries resource `0x68963ED8` and uses the procedural arrow
buffer if unavailable. A successfully borrowed resource is AddRef'd while
being assigned and the temporary reference is released afterwards. This table
is deliberately incomplete: the remaining controls, all creation-failure
paths and layout belong to FBQ2. The native body is not a contract for
recovering safely from allocation failure.

### 8.3 Resource creation and ownership

LoadDefaultIcon (`009A0EAD`, 321 bytes) creates an 18 by 17 buffer, preferring
component `0xC470D325`/interface `0x20732180` and falling back to the graphic
system's buffer creation when the factory fails. If a matching icon strip
resource `0xA8963EEA` exists, icon N is sourced from horizontal interval
18N..18N+18, vertical interval 0..17. Otherwise call CreateDefaultIcon.
Install the buffer through the browser's icon setter and release the local
buffer reference. The destination dimensions are byte-verified at
`009A0F14`–`009A0F1C`; the sheet rectangle still needs byte revalidation
before pixel conformance is credited (FBQ1).

CreateDefaultIcon (`0099EF11`, 3,423 bytes) accepts an icon selector, a buffer
and drawing area. A null buffer returns true without drawing (argument load,
test and branch at `0099EF18`–`0099EF1F`). Its palette is converted through the
window color-conversion operation rather than treated as raw device pixels.
For example, the first three RGB triples are (255,0,255), (0,0,0), and
(255,248,144), byte-verified at `0099EF2C`–`0099EF63`. The procedural glyph
geometry and lock-failure success behavior remain FBQ1; unreliable stack
recovery in cached pseudocode is not sufficient evidence for those contracts.

CreateDefaultButtonBuffer (`009A1163`, 1,938 bytes) uses the same component
factory/graphic-system fallback and requests an 84 by 22 buffer
(`009A1173`–`009A11BE`). A failed buffer initialization releases the obtained
buffer and clears the caller's output pointer (`009A18DF`–`009A18E6`). The
successful drawing recipe and Boolean result still require instruction-level
recovery (FBQ1). In particular, do not infer success solely from non-null output.

### 8.4 Lifecycle and paint

Init (`009A3CC1`, 742 bytes) checks the base init flag and skips its setup when
already initialized. First setup calls base Init, loads default icons 0–6,
constructs controls and populates lists if control construction reports
success. It returns true even on the already-initialized path. Constructor
defaults, complete setup order and failure preconditions remain FBQ3.

Shutdown (`0099EC98`, 450 bytes) first invokes the three browser cleanup
operations (primary offsets `0x1A0`, `0x1A4`, `0x1A8`), then releases and nulls
retained control/resource interface fields, including each entry of its icon
and button-buffer arrays, before invoking base Shutdown. These are ownership
releases, not borrowed pointers to be silently discarded. Exact counts and
the destructor-versus-Shutdown contract remain FBQ3.

GZPaint (`009A0A17`, 173 bytes) optionally paints a background using the
configured buffer or solid color. If the primary dirty byte `+0x31C` is set,
it invalidates and invokes UpdatePath, then clears that byte. This occurs
after background painting; list population can therefore happen during paint.
SetArea (`009A0089`, 45 bytes) sets that byte, invalidates, then delegates its
four original arguments to base SetArea (`009A008E`–`009A00AD`). Exact
background placement remains FBQ2.

## 9. Additional discriminating conformance cases

| ID | Setup/input | Required observation |
|---|---|---|
| FB-C10 | Existing flags 1, add-flags request 5 | No setter invocation; flags remain 1. Request 4 instead invokes replacement with 5. |
| FB-C11 | Replace flags, simultaneously toggling bit 2 and bit 4 | Window flag changes and invalidate path only; no direct UpdatePath call from the flag setter. |
| FB-C12 | Existing window bit `0x10000` set, clear browser bit 2 | Set `0x20000` and `0x40000`; no clear of `0x10000`. |
| FB-C13 | All new-folder candidates through suffix 255 exist; 256 is free | Attempt creation of suffix 256. |
| FB-C14 | All candidates including 256 exist | Still call CreateDirectory for candidate 256; propagate creation failure; no list refresh. |
| FB-C15 | Programmatic selection names nonexistent file in existing directory | True; selection type 2 and new path stored; no file-existence check or change notification. |
| FB-C16 | Programmatic selection follows selection with associated ID 42 | ID remains 42; do not synthesize zero for the new selection. |
| FB-C17 | In-range file row passed to HandleFileListDoubleClick | True without directory navigation; repeat with out-of-range row for false. |
| FB-C18 | Recognized file-like tag `0xBAA88513`, ID 42 | Both selection handlers retain ID 42; change emits `0x7ABE7203`, double-click emits `0x7ABE7204`; fourth message word is zero. |
| FB-C19 | Two patterns, only second matches filename | File included once; no requirement to match both. Directory names bypass patterns. |
| FB-C20 | Default-icon drawing receives null buffer | True; no color conversion, buffer lock or drawing. |
| FB-C21 | Default-button buffer allocated but initialization fails | Release buffer exactly once and clear output pointer. |
| FB-C22 | Fetch selection descriptor, then change browser selection | Getter returns true, copies type/ID and supplies three borrowed string-object pointers; it does not create owned string copies. Notification payload points to this five-word descriptor, not a string. |

Cases involving unproved pixel geometry, layout, comparator order or pending
selection are intentionally absent. Filling those cases to match an existing
implementation would not settle the native contract.

## 10. Coverage limits and reconciliation

This is partial coverage: especially resource drawing, control layout, default
state, comparator ordering and unclassified primary slots remain questions.
An inventory row does not assert that a method is specified. Public
widget-toolkit WT1/WT2/WT3/WT5/WT6 and HTML-document HD7 concern other owners
and are not answered by this work. Their reviewed spec/question files have no
differences between this worktree's starting tip and the current local main
reference; no renderer, Grid or Legacy contract is rewritten here.

The older exclusion of this owner in ui-windows.md §14 reflects its original
inventory scope. This separately assigned browser spec supersedes that
exclusion for the contracts above only; it does not claim a fully specified
new owner. See [open questions](questions/win-filebrowser.md).

## A. Exact export inventory and coverage disposition

Inventory captured from the Windows export and executable on 2026-10-22.
The preserved attribution set has 100 entries / 25,579 bytes. The primary
interface has 107 slots / 106 distinct targets, 74 absent from that set.
Their union below has **174 distinct entries / 28,907 export-defined bytes**.
Sizes are function-body lengths reported by the export, not ownership proof;
overlapping or bad entry boundaries have not been repaired by this lane.
The secondary interface is inventoried separately in the ABI table and base
window spec; inherited methods are not added to this byte total.

P gives zero-based primary slots; a dash means no primary-table occurrence.
Old is the preserved assignment reason, or `new` when found by primary-table
audit. A question destination explicitly means inventory-only coverage.
Addresses and lengths identify the functions without exporting their bodies.

| Address | Bytes | P | Old | Contract section or open question |
|---|---:|---|---|---|
| `0047AA30` | 8 | - | name-medium | 1.1 |
| `0047AA40` | 8 | - | name-low | 1.1 |
| `0047AA50` | 145 | - | vtable | FBQ6 |
| `005C2D10` | 370 | - | reach | FBQ6 |
| `005C2E82` | 42 | - | funclet | FBQ6 |
| `005C3A63` | 575 | - | reach | FBQ6 |
| `008AEB00` | 8 | - | name-low | FBQ3 |
| `008AF100` | 30 | - | name-high | FBQ3 |
| `00949ADE` | 3 | 92 | new | FBQ4 |
| `0094D34F` | 7 | 1 | new | FBQ4 |
| `00993785` | 7 | 9 | new | FBQ4 |
| `0099EC98` | 450 | - | name-high | 8.4 partial / FBQ3 |
| `0099EE5A` | 55 | 12 | neighbour | FBQ4 |
| `0099EE91` | 128 | 13 | neighbour | FBQ4 |
| `0099EF11` | 3423 | - | name-medium | 8.3 partial / FBQ1 |
| `0099FC70` | 24 | 2 | name-high | FBQ4 |
| `0099FC88` | 34 | 0 | name-high | 1.1 |
| `0099FCAA` | 85 | 4 | neighbour | FBQ4 |
| `0099FCFF` | 5 | - | name-low | FBQ6 |
| `0099FD04` | 27 | 5 | new | 5 |
| `0099FD1F` | 140 | 6 | new | 5 |
| `0099FDAB` | 29 | 7 | new | FBQ4 |
| `0099FDC8` | 19 | 8 | new | FBQ4 |
| `0099FDDB` | 11 | 10 | new | FBQ4 |
| `0099FDE6` | 615 | 11 | name-high | FBQ2 |
| `009A0081` | 8 | 25 | neighbour | FBQ4 |
| `009A0089` | 45 | - | vtable | 8.4 |
| `009A00B6` | 36 | 17 | new | FBQ4 |
| `009A00DA` | 34 | 27 | new | FBQ4 |
| `009A00FC` | 53 | 90 | new | 6.1 |
| `009A0131` | 20 | 29 | new | FBQ4 |
| `009A0145` | 43 | 35 | new | FBQ4 |
| `009A0170` | 29 | 34 | new | FBQ4 |
| `009A018D` | 33 | 37 | new | FBQ4 |
| `009A01AE` | 17 | 36 | new | FBQ4 |
| `009A01BF` | 161 | 41 | new | FBQ4 |
| `009A0260` | 42 | 40 | new | FBQ4 |
| `009A028A` | 73 | 39 | new | FBQ4 |
| `009A02D3` | 71 | 38 | new | FBQ4 |
| `009A031A` | 73 | 45 | new | FBQ4 |
| `009A0363` | 38 | 46 | new | FBQ4 |
| `009A0389` | 33 | 47 | new | FBQ4 |
| `009A03AA` | 14 | 43 | new | FBQ4 |
| `009A03B8` | 7 | 42 | new | FBQ4 |
| `009A03BF` | 8 | 44 | new | FBQ4 |
| `009A03C7` | 73 | 51 | new | FBQ4 |
| `009A0410` | 38 | 52 | new | FBQ4 |
| `009A0436` | 33 | 53 | new | FBQ4 |
| `009A0457` | 14 | 49 | new | FBQ4 |
| `009A0465` | 7 | 48 | new | FBQ4 |
| `009A046C` | 8 | 50 | new | FBQ4 |
| `009A0474` | 73 | 57 | new | FBQ4 |
| `009A04BD` | 38 | 58 | new | FBQ4 |
| `009A04E3` | 33 | 59 | new | FBQ4 |
| `009A0504` | 14 | 55 | new | FBQ4 |
| `009A0512` | 7 | 54 | new | FBQ4 |
| `009A0519` | 8 | 56 | new | FBQ4 |
| `009A0521` | 33 | 61 | new | FBQ4 |
| `009A0542` | 7 | 60 | new | FBQ4 |
| `009A0549` | 33 | 63 | new | FBQ4 |
| `009A056A` | 7 | 62 | new | FBQ4 |
| `009A0571` | 33 | 65 | new | FBQ4 |
| `009A0592` | 7 | 64 | new | FBQ4 |
| `009A0599` | 52 | 67 | new | FBQ4 |
| `009A05CD` | 31 | 66 | new | FBQ4 |
| `009A05EC` | 52 | 69 | new | FBQ4 |
| `009A0620` | 31 | 68 | new | FBQ4 |
| `009A063F` | 79 | 71 | new | FBQ4 |
| `009A068E` | 29 | 70 | new | FBQ4 |
| `009A06AB` | 53 | 73 | new | FBQ4 |
| `009A06E0` | 41 | 72 | new | FBQ4 |
| `009A0709` | 33 | 75 | new | FBQ4 |
| `009A072A` | 7 | 74 | new | FBQ4 |
| `009A0731` | 42 | 77 | new | FBQ4 |
| `009A075B` | 39 | 79 | new | FBQ4 |
| `009A0782` | 7 | 78 | new | FBQ4 |
| `009A0789` | 26 | 81 | new | FBQ4 |
| `009A07A3` | 7 | 80 | new | FBQ4 |
| `009A07AA` | 95 | 83 | new | FBQ4 |
| `009A0809` | 31 | 82 | new | FBQ4 |
| `009A0828` | 73 | 85 | new | FBQ4 |
| `009A0871` | 7 | 84 | new | FBQ4 |
| `009A0878` | 63 | 87 | new | FBQ4 |
| `009A08B7` | 12 | 86 | new | FBQ4 |
| `009A08C3` | 55 | 99 | new | FBQ4 |
| `009A08FA` | 145 | - | reach | FBQ6 |
| `009A098B` | 56 | - | reach | FBQ6 |
| `009A09C3` | 84 | - | reach | FBQ6 |
| `009A0A17` | 173 | - | name-high | 8.4 partial / FBQ2 |
| `009A0AC4` | 8 | 88 | neighbour | FBQ4 |
| `009A0ACC` | 687 | - | name-high | FBQ2 |
| `009A0D7B` | 80 | 26 | neighbour | FBQ4 |
| `009A0DCB` | 30 | 14,16 | neighbour | FBQ4 |
| `009A0DE9` | 30 | 76 | neighbour | FBQ4 |
| `009A0E07` | 46 | - | reach | FBQ6 |
| `009A0E35` | 120 | - | reach | FBQ6 |
| `009A0EAD` | 321 | - | name-medium | 8.3 partial / FBQ1 |
| `009A0FEE` | 373 | - | name-medium | FBQ1 |
| `009A1163` | 1938 | - | name-medium | 8.3 partial / FBQ1 |
| `009A18F5` | 116 | 20 | new | FBQ4 |
| `009A1969` | 25 | - | free-neighbour | FBQ6 |
| `009A1982` | 90 | - | reach | FBQ6 |
| `009A19DC` | 36 | - | reach | FBQ6 |
| `009A1A00` | 54 | - | reach | FBQ6 |
| `009A1A36` | 88 | - | reach | FBQ6 |
| `009A1A8E` | 43 | - | reach | FBQ6 |
| `009A1AB9` | 36 | - | free-neighbour | FBQ6 |
| `009A1AFF` | 40 | - | reach | FBQ6 |
| `009A1B27` | 163 | - | reach | FBQ6 |
| `009A1BCA` | 85 | - | reach | FBQ6 |
| `009A1C1F` | 38 | - | reach | FBQ6 |
| `009A1C45` | 36 | - | reach | FBQ6 |
| `009A1C69` | 41 | - | neighbour | FBQ6 |
| `009A1C92` | 29 | - | neighbour | FBQ6 |
| `009A1CAF` | 27 | - | neighbour | FBQ6 |
| `009A1CCA` | 50 | - | neighbour | FBQ6 |
| `009A1CFC` | 50 | - | neighbour | FBQ6 |
| `009A1D2E` | 64 | - | reach | FBQ6 |
| `009A1D6E` | 74 | - | reach | FBQ6 |
| `009A1DB8` | 28 | - | neighbour | FBQ6 |
| `009A1DD4` | 29 | - | neighbour | FBQ6 |
| `009A1DF1` | 29 | - | neighbour | FBQ6 |
| `009A1E0E` | 83 | - | reach | FBQ6 |
| `009A1E61` | 133 | - | reach | FBQ6 |
| `009A1EE6` | 41 | - | reach | FBQ6 |
| `009A1F0F` | 22 | - | neighbour | FBQ6 |
| `009A1F25` | 23 | - | reach | FBQ6 |
| `009A1F3C` | 31 | - | neighbour | FBQ6 |
| `009A1F5B` | 28 | - | reach | FBQ6 |
| `009A1F77` | 3149 | 91 | name-high | 8.2 partial / FBQ2 |
| `009A2BC4` | 107 | 30 | name-medium | 2.2 |
| `009A2C2F` | 160 | 33 | neighbour | FBQ4 |
| `009A2CCF` | 215 | - | name-medium | 3.1 |
| `009A2DA6` | 57 | 105 | neighbour | FBQ4 |
| `009A2DDF` | 40 | - | reach | FBQ6 |
| `009A2E07` | 31 | - | neighbour | FBQ6 |
| `009A2E26` | 161 | 23 | neighbour | FBQ4 |
| `009A2EC7` | 72 | 104 | neighbour | FBQ4 |
| `009A2F0F` | 24 | - | neighbour | FBQ6 |
| `009A2F27` | 167 | - | reach | FBQ6 |
| `009A2FCE` | 140 | 24 | neighbour | FBQ4 |
| `009A305A` | 51 | - | neighbour | FBQ6 |
| `009A308D` | 90 | - | reach | FBQ6 |
| `009A30E7` | 187 | - | neighbour | FBQ6 |
| `009A31A2` | 144 | - | reach | FBQ3 |
| `009A3232` | 8 | - | vtable | FBQ6 |
| `009A323A` | 113 | 28 | neighbour | FBQ4 |
| `009A32AB` | 97 | 94 | neighbour | FBQ4 |
| `009A330C` | 315 | 93 | neighbour | FBQ4 |
| `009A3447` | 156 | 100 | name-high | 6.2 |
| `009A34E3` | 250 | - | neighbour | FBQ6 |
| `009A35DD` | 28 | - | reach | FBQ6 |
| `009A35F9` | 134 | 106 | neighbour | FBQ4 |
| `009A367F` | 488 | 102 | name-high | 6.1 |
| `009A3867` | 444 | 103 | name-high | 6.1 |
| `009A3A23` | 208 | 101 | neighbour | FBQ4 |
| `009A3AF3` | 116 | 95 | name-high | 2.3 |
| `009A3B67` | 346 | - | name-medium | FBQ5 |
| `009A3CC1` | 742 | - | name-high | 8.4 partial / FBQ3 |
| `009A3FA7` | 547 | - | free-neighbour | FBQ5 |
| `009A41CA` | 116 | 15 | new | FBQ4 |
| `009A423E` | 359 | 19 | name-medium | 6.3 |
| `009A43A5` | 259 | 18 | new | FBQ4 |
| `009A44A8` | 114 | 31 | new | 2.1 |
| `009A451A` | 549 | 32 | name-high | 7 |
| `009A473F` | 1007 | 98 | name-high | FBQ5 |
| `009A4B2E` | 908 | 97 | name-medium | FBQ5 |
| `009A4EBA` | 2241 | 96 | name-high | 8.1 partial / FBQ5 |
| `009A5AAB` | 195 | 21 | new | FBQ4 |
| `009A5B6E` | 132 | 22 | new | FBQ4 |
| `009B689C` | 36 | - | reach | FBQ6 |
| `009B6A49` | 27 | - | reach | FBQ6 |
| `009CFA1D` | 25 | 89 | new | FBQ4 |
| `009CFA36` | 12 | 3 | new | FBQ4 |

No full-owner, pixel-match, implementation or replacement credit follows
from this inventory. Questions FBQ1�FBQ6 delimit the missing contracts.
