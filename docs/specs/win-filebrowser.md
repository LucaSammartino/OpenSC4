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

### 2.1 Set current directory (`009A44A8`, 113 bytes)

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

## 5. Coverage limits

Lifecycle, control construction, resource drawing, list population, sorting,
selection, filters and layout are being audited in this lane. Until their
contracts are written here, their presence in a vtable or coverage attribution
is not a specification. Public widget-toolkit questions WT1/WT2/WT3/WT5/WT6
and HTML-document HD7 concern other owners and are not answered by this work.
