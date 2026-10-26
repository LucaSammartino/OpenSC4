# File browser window (`cGZWinFileBrowser`)

Target: SimCity 4 Deluxe, Windows GOG 1.1.641. RE lane `re-filebrowser4`,
2026-10-22. This is a **partial owner specification**. Address citations refer
to that executable; they are evidence anchors, not implementation requirements.
Only the contracts explicitly described below are specified. No implementation,
shadow comparison, or in-game verification is claimed.

## 1. Scope, interfaces and evidence inventory

This owner combines a window with directory navigation, a file grid, an optional
filename editor, file filters and selection notifications. Base window behavior
comes from [widget-toolkit.md](widget-toolkit.md); grid behavior comes from
[win-grid.md](win-grid.md). This document does not extend either owner.

The preserved coverage inventory contains **100 candidate functions / 25,579
bytes**, including reachability, neighbour and compiler-helper assignments. That
is an attribution inventory, **not a count of fully specified browser methods**.
The audit also found primary-interface targets missing from that inventory.
The final inventory below distinguishes those sets; do not assign complete
owner coverage from the preserved count.

### 1.1 Interface identity and table boundaries

The browser interface is at object offset zero. Querying IID `0x3AA52E64`
returns that interface and adds a reference. Other IIDs are delegated to the
base window at object offset `+4` (`0099FC88`–`0099FCA9`). The window-interface
QueryInterface and Release thunks subtract four before forwarding (`0047AA30`,
`0047AA40`). A caller must release a successful query result.

The window-interface tables at `00A8D000` and `00ADCF30` have 151 slots (0–150).
The recovered constructor installs **`00ADCF30`** at object `+4`; its slot
148 is `009A3232`, forwarding to deleting destructor `009A35DD`. The older
table's slot 148 is `008AEB00`, forwarding to `008AF100`. Both deleting
wrappers call the same actual destructor `009A31A2`. Native table words and
constructor stores (`009A57DC`–`009A57E2`) prove this distinction; a name-based
vtable query alone does not distinguish these copies.
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

The verified filter-index operation (primary slot 26, `009A0D7B`, 80 bytes)
accepts an unsigned index into a vector of 12-byte filter-group elements.
An out-of-range index returns false without side effects. A valid index stores
the selection, updates the filter combo when present, sets dirty byte `+0x31C`,
invalidates, and returns true. The same valid index repeated still performs
these actions. It does not populate the file list immediately. Evidence:
count division/comparison `009A0D7E`–`009A0D95`, effects `009A0D97`–`009A0DC1`.

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
a local bounds check. The match option at primary `+0x15C` is a byte:
**zero lowercases copies of both candidate and pattern; nonzero preserves
both** before MatchPattern. Original stored names/patterns are not changed
(`009A5104–5116`, `00918F08–2F`). This settles its case-folding polarity;
the directory service's own filter syntax remains an external input.

After enumeration the entry vector is sorted, then grid rows are populated
with optional icons, names and extension text. File extension text has a
leading dot removed when there is at least one following character, and is
lowercased. A column's configured font overrides the common font; otherwise
the common font is used. Display strings are fitted to column width before
being installed in cells; stored entry names remain available separately.
The exact comparator, complete column sizing and pending-selection restore
were originally deferred together. The comparator and string fitting are now
specified below, as are file-column sizing and pending-selection restoration.
Directory-list and drive-popup population are specified in 8.1.4–8.1.5.

#### 8.1.3 File column sizing and selection restoration (FBQ5)

After sorting, set the stored filename-column width at primary `+0x1F0` to
the file grid's window width minus 2. Compute capacity by **unsigned integer
division** of the grid's window height by row height `+0x224`. Subtract
scrollbar width `+0x314` only when the entry count is greater than this
capacity. Then subtract each enabled column's configured width: icon
(`+0x1DC`/`+0x1E4`), auxiliary (`+0x218`/`+0x220`), extension
(`+0x1F4`/`+0x1FC`), in that order. The filename width is calculated even
when its enabled byte `+0x1E8` is false. Each enabled column is installed
individually, including the filename column using this residual width.
There is no lower clamp or zero-row-height fallback: these are 32-bit
subtractions and an unsigned divide (`009A51E2–530D`). Thus narrow layouts
can produce a negative fitting width rather than a minimum usable column.

Restoration searches the **sorted** entry rows, choosing at most one.
Pending type `+0x198` equal to 1 considers directory-tag `0xBAA88502` rows;
type 2 considers rows whose tag is neither parent nor directory. Parent rows
never match. An empty pending string chooses the first eligible row.
Otherwise compare lowercased copies of the entry name and pending string
for equality; the first match wins. This comparison is independent of the
wildcard case option. Other pending types select no match. The candidate row
starts at **zero**, so no eligible row or no matching name leaves row zero
as the fallback (`009A530D–5412`, `009A5508–55B2`).

After installing rows and row heights, set the grid's first visible row to
zero, request selection at column 0 and the candidate row with option true,
then update the grid. Query whether that cell is visible; if false, ask the
grid to bring it into view. These calls occur even for an empty entry vector.
Only when the vector is nonempty invoke the browser's primary slot 102
(selection-change handling) with the candidate row. Return true. There is no
separate missing-name error or owner-side skip of the empty-grid selection
request (`009A56DE–5744`). What the Grid does with an empty-row request belongs
to its own contract; it is not a synthetic file entry.

#### 8.1.1 Default comparison (`009A3FA7`, 547 bytes)

This five-argument callback receives two entry-record pointers, a primary
file-sort mode, a secondary file-sort mode and a directory/type-order mode.
It returns a Boolean precedes predicate, not a signed three-way comparison.
If the first entry's full type is `0xBAA88501` (parent), return true immediately,
even against another parent. Otherwise, if the second is parent, return false.
There is no equality exception before these tests (`009A3FB1`–`009A3FD1`).

For other entries, compare the unsigned low nibbles of their types first.
If different, modes 0 and 1 put the smaller nibble first; all other directory
modes put the larger nibble first (`009A3FD6`–`009A3FDE`,
`009A41AF`–`009A41C3`). Thus directory type 2 precedes file type 3 for modes
0/1, but follows it otherwise. This type grouping takes priority over name
and extension settings.

For equal low nibbles, copy both stored names (record `+0x0C`), derive both
path extensions from those original names, and lowercase all four copies.
Compare unsigned bytes lexicographically, with a shorter equal-prefix string
first; no natural-number or locale collation is added. For a first entry whose
full type is directory `0xBAA88502`, compare names ascending in directory modes
0 or 2, descending in every other mode. For a first entry of full type
`0xBAA88503` or `0xBAA88513`, use the file modes below. Any other first full
type returns false after the type-group check, even if its low nibble matches
a recognized type. Do not reduce that full-type dispatch to a nibble test.

| File mode | Key | Direction |
|---:|---|---|
| 0 | Lowercased full stored name | Ascending |
| 1 | Lowercased full stored name | Descending |
| 2 | Lowercased path extension | Ascending |
| 3 | Lowercased path extension | Descending |
| Any other value | None | Return false |

If the primary file mode is 0/1 and names are equal, substitute the secondary
mode once. If it is 2/3 and extensions are equal, substitute the secondary mode
once. Compare the resulting key without another fallback. Invalid primary
modes do not activate a secondary mode. Equal selected keys return false in
either direction. There is no ID, original-case, or insertion-order tiebreaker.
The extension helper's result is compared as returned, unlike the separate
display extension which strips a leading dot.

Evidence: copy/extension/lowercase `009A3FE4`–`009A4053`, directory mode
`009A4057`–`009A407A`, file full types/ties/mode dispatch
`009A40D4`–`009A4142`. String helpers `009A1AB9` (36 B) and `009A1969`
(25 B) ultimately use unsigned-byte comparison `004973C0`; descending helper
`0066E3A0` reverses the argument order into `004B5CA0`. These are shared
string helpers, not additional browser methods.

Examples: name-ascending puts `file10` before `file2`; case-only differences
compare equal unless the secondary selected key distinguishes them. Extension
mode 2 with secondary mode 0 sorts equal extensions by lowercased name. Two
parent entries compare true in both directions; the callback does not define
a strict ordering for that unsupported duplicate-parent case.

#### 8.1.2 FitStringIntoWidth (`009A3B67`, 346 bytes)

This void helper takes a mutable string, signed 32-bit target width and nullable
font. Null font or an empty string leaves the string unchanged. Ask the font's
fit-count operation (offset `0xA8`) for the original bytes, length, target width
and option zero. A count of zero clears the string; a count equal to its byte
length leaves it unchanged. Only a nonzero partial fit proceeds to a marker.

Request text resource `0x5377BE31` from the resource manager. If unavailable
(including no manager), use `...` from native pointer `00B17B10`. Copy resource
bytes using its pointer/length accessors; this helper adds no retained resource
reference and performs no resource Release. Measure the marker with the font's
rectangle operation (offset `0xBC`), or use a zero rectangle if the marker is
empty. Marker width is signed 32-bit `right-left`. If it is greater than or
equal to target width, clear the original string. The test is **strictly less
than** for the path that appends a marker (`009A3C61`–`009A3C76`).

Otherwise fit the original string a second time at width
`target + left - right`, with ordinary 32-bit integer arithmetic and option
zero. Erase from the returned byte count to the end, right-trim the retained
prefix, then append the marker. There is no extra space or second width check;
even a zero count on this second fit still permits a marker-only result. Font
counts are used without local clamping. This helper does no floating-point
rounding; pixel measurement remains the font's delegated behavior. Evidence:
early exits `009A3B74`–`009A3BBD`, marker acquisition `009A3BC3`–`009A3C34`,
measurement/refit/erase/trim/append `009A3C34`–`009A3CAC`; append helper
`00408C20` forwards an exact byte range to `004081D0`.

For example, when a partial original fit is reported and the marker measures
12 pixels, target width 12 clears the result; width 13 refits the prefix at
1 pixel, right-trims it, then appends the marker. A fontless call never clears
the input merely because width is zero or negative.

#### 8.1.4 Directory display and drive records (`009A4B2E`)

The directory/drive list at primary +0x158 contains records with a string,
type at +0x14, borrowed icon at +0x18 and indentation depth at +0x1C.
UpdateDirectoryList first removes and destroys all records whose depth is
nonzero; depth-zero drive records retain their order. Reset current-row
+0x19C to zero and column span +0x234 to 2. Compare lowercased copies of
the first byte of each surviving record's name and the current directory's
first byte; the first equality identifies the drive. This is a one-byte
comparison, not full root/path matching (`009A4B2E–4C68`).

For that drive, SplitDirectory receives capacity 48 and 260-byte component
slots. Given returned component count N, insert records for components
N-2, N-3, …, 0 immediately after the matched drive, in that order. Each
display name loses exactly one trailing path separator, if present. New
records have type 5, the configured directory icon +0x2D0, and depths
1,2,…,N-1. Increment current-row and column span for each insertion; the
row already includes the number of drives preceding the match. No sorting
of these inserted records occurs (`009A4C68–4D76`). These rules require
successful allocation and a valid SplitDirectory output within capacity;
there is no local allocation-failure recovery.

The collapsed directory grid displays only the last inserted record, or the
matched drive if no component record was inserted. Clear its cells. Install
the record's icon in cell (0,0) if nonnull and apply icon style +0x280 there.
Only with an icon set column 0 to icon width+2. Copy the name, fit it to
grid window width-3-icon-column-width (+0x1E4), using directory style font
+0x2A4 or the common font +0x2C8, then put that fitted name in cell (1,0)
with directory style +0x2A4. Stored names are not shortened. A null icon
does not remove the icon-width subtraction. If no drive matches, return
true without clearing or replacing the collapsed display, although the old
nonzero-depth records were removed and current-row/span were reset/advanced.
The function returns true on every normal completion (`009A4D76–4EB9`).

#### 8.1.5 Drive popup (`009A473F`)

Each open obtains a new grid by the factory/built-in policy of 8.2, retained
at +0xEC with ID `0xBAA55D51`. There is no existing-popup reuse guard in
this body. Let R be the current record count and M the unsigned maximum
visible rows +0x230. Visible capacity is min(R,M); reserve scrollbar width
for text fitting exactly when R>M. Configure **column span+8** columns,
one initial row and option false. The +8 is instruction-proved at
`009A47E9–47FB`; the cached pseudocode's +2 is incorrect.

The grid uses the common font when nonnull, selection color words
`0x40000000,0x40000000`, white background, four unit borders, outline
(+0x308,0), cleared flags 0x800 and 2 and added flags 0x20000 and 0x80000.
Its height is row-height*visible-capacity+2, width equals the collapsed
directory grid's width, and its top is the collapsed grid's bottom minus
one. Arithmetic is 32-bit, without screen-edge fitting or a minimum-height
repair. Configure the first column-span columns to width
unsigned(low32(icon-width*9))>>4; configure the following column to 300
(`009A4895–4963`).

Rows follow record-list order. Apply directory style +0x2A4 to each whole
row. A nonnull icon goes in the record's depth column with icon style
+0x280. Fit a copy of the name using directory/common font into
popup-width-2-(depth+1)*icon-width, additionally subtracting scrollbar width
+0x314 only for R>M; place text in column depth+2 with the fitted explicit
byte count and options (true,false). There is no lower width clamp
(`009A497A–4A81`, arithmetic at `009A49E1–4A05`).

Set all R row heights to +0x224, add the grid window to the browser, set
first visible row zero, select cell (0,current-row) through grid +0xAC,
and update it. If that cell is not visible, first visible row becomes
current-row-visible-capacity+1, without an owner clamp. Request focus through
the manager at primary +8, then grid operation +0x22C, and return true.
The zero-row case still makes these calls (`009A4A81–4B2D`). Grid and
manager response semantics remain their own boundary, rather than an
invented owner-side focus/visibility fallback.

#### 8.1.6 File-row configuration (remainder of `009A4EBA`)

Within directory enumeration, the parent-navigation button, when present,
gets enable flag 2 set according to SplitDirectory reporting at least two
components. The flag-4-disabled path does not refresh that enable state.
File population clears both existing grid cells and column definitions
before clearing the owned entry vector (`009A4EBA` onward).

Every sorted row receives auxiliary style +0x280 at the configured auxiliary
column +0x21C, even if that column is disabled. With the icon column enabled,
parent/directory rows use +0x2CC/+0x2D0 respectively; other rows use their
own icon when nonnull, otherwise +0x2D4. Missing icons omit the image, but
icon-cell style +0x280 is still installed. Parent and directory extension
cells receive extension style when enabled, with no extension text.

Other rows' enabled extension cells contain the lowercased extension, with
one leading dot removed only when another character follows. Fit it into
extension-width-2 using extension style font +0x25C or common font, then
install text with count 0xFFFFFFFF and options (true,false), followed by its
style. All rows also install a fitted name at +0x1EC using filename-width-2,
filename style font +0x238 or common font, count 0xFFFFFFFF and the same
options, then filename style. This name installation is unconditional even
when the filename column's enabled byte is false. These are display copies;
entry names used for sorting/restoration remain unchanged. Set the entire
row range's height to +0x224 before the selection/visibility operations in
8.1.3 (`009A530D–56DE`). No owner checks convert failed grid/style/font
operations into an empty-list or alternate success result.

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
| `+0x11F` | `+0xF4` | `0xBAA55D53` | parent-navigation button |
| `+0x120` | `+0xF8` | `0xBAA55D54` | new-directory button |
| `+0x122` | `+0x100` | `0xBAA55D56` | filename text editor |
| `+0x123` | `+0x104` | `0xBAA55D57` | filter combo |
| `+0x124` | `+0x108` | `0xBAA55D58` | confirmation button |
| `+0x125` | `+0x10C` | `0xBAA55D59` | Cancel button |
| `+0x126` | `+0x110` | `0xBAA55D5A` | Look in label |
| `+0x127` | `+0x114` | `0xBAA55D5B` | File name label |
| `+0x128` | `+0x118` | `0xBAA55D5C` | Files of type label |

Grids request component `0xDAA6B9BF`, interface `0xDAA6B9BE`; buttons request
`0x22ECFC47`/`0x00008810`; the editor requests `0x231A1C58`/`0x231A1C57`.
The arrow first tries resource `0x68963ED8` and uses the procedural arrow
buffer if unavailable. A successfully borrowed resource is AddRef'd while
being assigned and the temporary reference is released afterwards. This table
lists all controls created here, in creation/child-addition order. The popup
grid +0xEC is created separately by 8.1.5, not by enable byte +0x11D here.
Combo requests component/interface `0x59B`/`0x412CE496`; labels request
`0x592`/`0x212CDC1F`. Fallback grids use constructor flags 0x19BFF, editor
0x11FF and combo 1. The fallback's allocation sizes are respectively 744,
436 (button), 656 (editor), 316 (combo) and 276 (label) bytes. Editor,
combo and label retained interfaces are at allocation+0xD8; the others
retain their returned interface directly.

Factory Boolean false triggers fallback even if its output pointer was
written; true proceeds directly with the supplied interface. Each fallback
result is AddRef'd without a null guard. Neither graphics-service acquisition,
GZCOM, fallback allocation nor a missing generated arrow buffer has a graceful
failure path here. Child-addition/configuration return values are ignored;
normal completion returns true. Existing retained controls are not released
or reused first, so this body is not an idempotent rebuild.

The following configuration requests define the child boundary. Offsets are
byte offsets on the retained child interface (window operations are named
explicitly). They preserve numeric options without assuming another control's
unproved implementation. B and F are graphics-service conversions of black
(0,0,0) and white (255,255,255), acquired in that order before any child.

| Child | Configuration beyond ID/name and child addition |
|---|---|
| Collapsed directory grid | Grid dimensions (2,0,true); common font if nonnull; selection words (0x40000000,0x40000000); background F; four border values 1; outline (+0x308,B); color operations +0x44(B,0), then (F,1); clear flags 0x800,2,4; add flag 0x20000; window height row-height+2; default cell style +0x2A4 at (-1,-1). |
| Dropdown arrow | Button +0x14(0x20,0); assign resource/procedural arrow to image state 2 using its bounds; release temporary reference. |
| File grid | Grid dimensions (+0x228,1,false); common font if nonnull; selection words (+0x304,+0x300); background F; colors (B,0),(F,1); four unit borders; outline (+0x308,B); clear flag 2 and add 0x20000. |
| Parent/new-directory buttons | Button +0x14(0x20,0); when respective buffer +0x2F8/+0x2FC is nonnull, +0x44(buffer) and image-state assignment (2,buffer,0). |
| Filename editor | Clear flags 0x200 and 0x800; set common font even when null; color indices 0 and 5 to B, 1 and 4 to F. |
| Filter combo | +0x54(0); set common font; window fill F; color indices 0 to B and 2 to F; +0x40(B). Append every 20-byte filter-label string in vector order with its zero-based index; when labels exist, select stored filter index with notify=false. |
| Confirmation/Cancel | Window captions default string referenced at `00B17B18` / `Cancel`; image-state assignment (2,0,0); +0x14(0x20,0); pass corresponding +0x2F0/+0x2F4 buffer to +0x44 only if nonnull. |
| Three labels | Captions `Look in:`, `File name:`, `Files of type:` respectively; +0x28(B) and +0x1C(4,0). |

Each child also receives its built-in window name from the consecutive name
objects at `00B17B2C` through `00B17BDC`, step 16, in the table's order.
Creation does not assign child rectangles or enforce a minimum browser size.
Init's actual browser dimensions and row height are specified in 8.5;
SetArea delegates geometry to the base window. In particular the file grid
and filename editor remain independently enabled. Pixel recipes of resources
requested here remain FBQ1; primary property mutations remain FBQ4.

#### 8.2.2 Message routing (`009A0ACC`, 687 bytes)

This receives window pointer W, primary P=W-4. Messages are word arrays:
type at +0, sender/subtype at +4 and data/ID at +8. Every normal branch,
including recognized messages, finally calls base DoMessage with W and the
original message and returns its result (`009A0D6C–D7A`). Recognition is not
an unconditional handled=true result. There are no local sender/payload
validity checks or exception recovery. Sender IDs below are obtained through
the sender's window interface; the editor-change row instead tests identity.

| Message and eligibility | Browser response before base handling |
|---|---|
| Type 3, subtype 0x287259F6, ID 0xBAA55D52 | Open popup through primary slot 98. |
| Same subtype, ID 0xBAA55D53 / 0xBAA55D54 | Parent navigation / create directory (primary slots 30 / 32). |
| Same subtype, ID 0xBAA55D58 or 0xBAA55D59 | Forward the original message through window +0x244 to notification target W+0x4C. |
| Type 3, subtype 0xE8711255, ID 0xBAA55D57 | Read selected combo index (+0x78); call filter setter slot 26 only when different from P+0x154. |
| Type 0x9A1552D2, sender ID 0xBAA55D55 | Read payload word at +4; call selection-change slot 102 only when different from P+0x22C. |
| Type 0x9A1552D3, sender ID 0xBAA55D51 | If payload word 0 is 5, pass payload word 2 to directory-selection slot 101; always then close popup through slot 99. |
| Same type, sender ID 0xBAA55D50 | Clear retained collapsed-grid cells, then open popup. |
| Type 0x9A1552D4, sender ID 0xBAA55D55, payload word 0 is 5 | Pass payload word 2 first to selection-double-click slot 103, then file-list-double-click slot 100. Other payload tags do neither. |
| Type 0x9A1552D5, popup ID 0xBAA55D51, data Enter (13) | Get the sender's current cell through +0xA8, pass returned row to directory-selection slot 101, then close popup. |
| Same type/ID, data Escape (27) | Close popup. Other key values have no browser action. |
| Type 0x9A1552D7, popup ID 0xBAA55D51 | Close popup. |
| Type 0x9B1552D8, sender pointer equals retained filename editor P+0x100 | Obtain five-word selection descriptor through primary slot 90. Synchronously forward a temporary notification with type 0x7ABE7203, browser window ID, pointer to that descriptor and final word zero, through window +0x244 to W+0x4C. |

The synthetic descriptor and notification are stack temporaries, not owned
payload transfers; their borrowed members obey section 6. Unlisted IDs,
types and subtypes perform no browser-specific action before base handling.
Evidence: `009A0ACC–0BE3` (identity/key routes), `009A0BE3–0CDA`
(selection payloads), `009A0CDA–0D6C` (commands/filter/forwarding).

#### 8.2.1 EnableChildControl (`0099FDE6`, 615 bytes, primary slot 11)

This void method takes a 32-bit control ID and a byte enable value. Only IDs
`0xBAA55D50..0xBAA55D5C` are accepted (unsigned range check). Store the exact
byte at `+0x11C + (ID-firstID)` only when it differs. Invalid IDs and unchanged
bytes do nothing. A changed nonzero byte only changes storage: no creation,
child enable call, layout, dirty marking or invalidation occurs.

On a changed zero byte, remove the corresponding child with window
ChildRemove (slot 15), Release the retained interface once, then null its field.
The removal result is ignored. IDs ending `51` and `55` skip these operations
even if the associated interface exists; their enable bytes still change.

| ID suffix (prefix `0xBAA55D`) | Primary field removed on disable | Window accessor offset |
|---|---|---|
| `50` | `+0xE8` | `0x14` |
| `51` | None | — |
| `52`, `53`, `54` | `+0xF0`, `+0xF4`, `+0xF8`, respectively | `0x0C` |
| `55` | None (file grid `+0xFC` stays retained) | — |
| `56` | `+0x100` | `0x14` |
| `57`, `58`, `59`, `5A`, `5B`, `5C` | `+0x104`, `+0x108`, `+0x10C`, `+0x110`, `+0x114`, `+0x118`, respectively | `0x0C` |

There is no Boolean success result. Evidence: `0099FDEA`–`0099FE26`, removal
bodies `0099FE2D`–`009A0041`, return `009A004A`; jump table `009A004D` maps
cases 1 and 5 to `009A0047`. For example, disabling filename editor `0xBAA55D56`
removes/releases/nulls it; setting it back to 1 does not recreate it. Disabling
file grid `0xBAA55D55` leaves the retained grid intact.

### 8.3 Resource creation and ownership

LoadDefaultIcon (`009A0EAD`, 321 bytes) creates an 18 by 17 buffer, preferring
component `0xC470D325`/interface `0x20732180` and falling back to the graphic
system's buffer creation when the factory fails. If a matching icon strip
resource `0xA8963EEA` exists, icon N is sourced from horizontal interval
18N..18N+18, vertical interval 0..17. Otherwise call CreateDefaultIcon.
On the strip path, request buffer lock `0x8010`; on success copy the source
rectangle into (0,0,18,17) and unlock. Failed lock skips copying but still
assigns a color key converted from RGB (255,0,255), installs the buffer and
returns true. The procedural fallback's return value is ignored. After either
path, install through the icon setter, release the local reference and return
true. Failure to obtain a buffer or initialize it returns false; an obtained
buffer is released even when initialization fails. The setter's acceptance of
the selector is not propagated. Destination dimensions are byte-verified at
`009A0F14`–`009A0F1C`; source rectangle, lock, copy and key at
`009A0F4F`–`009A0FB4`; installation/release/result at `009A0FCC`–`009A0FEB`.
The source rectangle uses 32-bit multiply/add arithmetic.

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

### 8.4 Icon and button-buffer setters

The icon setter (primary slot 12, `0099EE5A`, 55 bytes) accepts an unsigned
index 0–6 and a nullable buffer interface. Invalid indices have no effect.
For a valid slot containing the same pointer, do nothing. Otherwise AddRef
the incoming non-null buffer before releasing the previous non-null buffer,
then store the incoming pointer. Its array starts at primary offset `+0x2CC`.
There is no explicit invalidation in this body.

The button-buffer setter (primary slot 13, `0099EE91`, 128 bytes) applies the
same replacement/ownership rule to indices 0–3 in the array at `+0x2F0`.
After a changed assignment, pass the new buffer (including null) to the
corresponding button's buffer setter if the button exists. Index-to-button
fields are 0→`+0x108`, 1→`+0x10C`, 2→`+0xF4`, 3→`+0xF8`. An unchanged pointer
does not refresh the child. The unsigned range tests reject `0xFFFFFFFF`.
Evidence: `0099EE5A`–`0099EE8E`, `0099EE91`–`0099EF0E`. These operations retain
references; they do not transfer the caller's reference.

### 8.5 Lifecycle and paint

Init (`009A3CC1`, 742 bytes) receives the secondary/window pointer, checks
the base init flag, and returns Boolean true immediately if already initialized.
On first setup it obtains the resource manager, calls base Init without
testing its result, and sets **actual width 396 and height 236**, in that order,
through window slots 51/52. These are not minimum dimensions: SetW/SetH
construct new area endpoints (`0099BC68`, `0099BC8F`). It then loads default
icons 0–6, invokes primary slots 93 then 94, and copies the fallback directory
to the current directory if the latter is empty (`009A3CC9`–`009A3D76`).

It stores `0xFF7C90FF` at primary `+0x30C`, invokes primary slot 40 with
`(0,5,true)`, and, if the common font field `+0x2C8` is empty, requests resource
`0x68963C4C`. A returned font is retained once. Its `(0,0)` height replaces
`+0x224` only when unsigned-greater than the current value. Primary slot 73
then receives `(5,5,5)`; slots 61,63 receive `+0x2E8`; slot 65 receives
`+0x224`; slots 67,69 receive `(0,0)`. Init sets primary bytes `+0x29E` and
`+0x2C2` to 6, passes the packed color pair derived from RGB `(8,36,107)` and
`(255,255,255)` to slot 71, and enables slot 79 with true. If browser flag 2
is set it clears window flags `0x20000` and `0x40000` and sets `0x10000`;
otherwise it sets `0x20000` and `0x40000`, leaving `0x10000` untouched
(`009A3D7B`–`009A3EDB`). Public property meanings remain in FBQ4 until their
method bodies are reconciled; these ordered calls do not authorize substituting
different setup values.

Next acquire button resources `0x683DB4E5` and `0x683DB4E6` for selectors 2
and 3, respectively. Retain each borrowed resource, or call the procedural
buffer creator when absent, assign it through primary slot 13, then release
the temporary reference. Neither creation success nor the temporary pointer
is checked before Release. The second resource-manager call is also
unconditional: a missing resource manager is not a supported graceful-failure
path (`009A3EE1`–`009A3F63`). After AddControls (slot 91), only on true call
slot 26 with the selected filter index, slot 92, then UpdatePath (slot 95).
Finally clear dirty byte `+0x31C` and return true regardless of AddControls'
result (`009A3F66`–`009A3FA6`). Failed first setup has no local rollback and
does not turn the return into false.

Shutdown (`0099EC98`, 450 bytes) receives the secondary/window pointer and
first invokes primary slots **104,105,106**, in that order: clear file-entry
records, clear drive records, clear custom-entry groups. Their results are
ignored. Then visit the following **29 reference fields** in order, releasing
each non-null interface exactly once and setting that field to null. Offsets
below are all from the **primary** pointer, not the Shutdown argument.

| Order | Primary fields | Number |
|---|---|---:|
| 1 | `+0xE8` through `+0x118`, step four (retained controls) | 13 |
| 2 | `+0x2C8` (common font) | 1 |
| 3 | `+0x2CC` through `+0x2E4`, step four (icons) | 7 |
| 4 | `+0x2F0` through `+0x2FC`, step four (button buffers) | 4 |
| 5 | `+0x310`, then `+0x238`, `+0x25C`, `+0x2A4` | 4 |

Shutdown then tail-calls base Shutdown. It has no own initialized-flag guard;
null fields skip Release, but cleanup slots and base Shutdown are still called
on a repeat. In particular it **does not release `+0x280`**, the third of the
four style-block font fields. Do not silently extend this list to a symmetric
four-field loop. Evidence: `0099EC9D`–`0099ECBE`, scalar sequence
`0099ECBE`–`0099EDCA`, seven/four iteration counts `0099EDCB`–`0099EE01`,
and final fields/tail call `0099EE03`–`0099EE55`.

GZPaint (`009A0A17`, 173 bytes) uses the secondary window pointer W (primary
P = W-4). If byte P+0x309 is nonzero, copy the window area at W+0x24 through
W+0x30. Without background buffer P+0x310, set the draw context's color to
P+0x30C and fill that copied rectangle. With a buffer, ask its slot +0x30 for
a rectangle using that area and options `(0,0)`, then submit buffer and returned
rectangle to context slot +0x9C. The owner does not center, tile, divide into
thirds, or subtract child/scrollbar strips (`009A0A20–98`). The foreign buffer
operation determines its returned rectangle; no pixel sampling rule is implied.
If primary dirty byte `+0x31C` is set, invoke primary slots 92 then 95
(the folded true-return hook and UpdatePath), then clear that byte. Slot 92
does no invalidation itself (`00949ADE–AE0`). These calls follow the
background even when background painting is disabled, and the clear is after
both calls, so dirtiness set during them is also cleared. Return true
(`009A0A98–C3`). List population can therefore happen during paint.
SetArea (`009A0089`, 45 bytes) sets that byte, invalidates, then delegates its
four original arguments to base SetArea (`009A008E`–`009A00AD`). Exact
AddControls and message dispatch are specified in 8.2 and 8.2.2.

### 8.6 Constructor and destruction (native lifecycle contract)

The actual constructor is **`009A57C4` (710 bytes)**, absent from the original
174-entry union. It constructs the base window at `+4`, installs primary
`00ADD190` and secondary `00ADCF30`, and initializes the following defaults
before returning the primary pointer. No controls or graphic resources are
created here; those belong to Init/AddControls.

| State | Constructor value / operation |
|---|---|
| File-entry vector `+0xDC`, custom groups `+0x13C`, filter strings `+0x148` | Empty, zero begin/end/capacity |
| Retained controls `+0xE8..+0x118` | All 13 null |
| Control enables `+0x11C..+0x128` | All 13 bytes true |
| Flags `+0x160`; filter index `+0x154` | `0xD`; zero |
| Sort selector `+0x12C`, option `+0x134`, callback `+0x138` | zero, zero, `009A3FA7` |
| Current directory `+0x170`, fallback `+0x184`, pending name `+0x1A0` | Empty strings |
| Parent label `+0x1B4`, new-directory label `+0x1C8` | `..`, `New Folder` (native pointers `00B17B0C`, `00B17B14`) |
| Pending selection words `+0x198`, `+0x19C` | Both zero |
| Common font `+0x2C8`, icons `+0x2CC..+0x2E4`, buttons `+0x2F0..+0x2FC` | All null |
| `+0x224`, `+0x2E8`, `+0x2EC`, `+0x314`, `+0x318` | 20 each |
| `+0x228`, `+0x22C`, `+0x234`, `+0x300`, `+0x304`, `+0x310` | Zero |
| `+0x230`; `+0x30C` | 8; `0xFFFFFFFF` |
| Bytes `+0x15C`, `+0x308`, `+0x309`, `+0x31C` | False |

Words `+0x164,+0x168,+0x16C` also start at zero.

Five dimension words `+0x1E4,+0x1F0,+0x1FC,+0x208,+0x214` start at 10;
`+0x1F8,+0x204,+0x210` start at `0xFFFFFFFF`. Words `+0x130,+0x1EC,+0x220`
start at 2; `+0x1E0` starts at zero and `+0x21C` at 1. Bytes `+0x1DC,+0x1E8,
+0x218` are true; `+0x1F4,+0x200,+0x20C` are false. These stored defaults
are compatibility facts, not inferred minimum dimensions. Four 36-byte style
blocks at `+0x238,+0x25C,+0x280,+0x2A4` start with null font, six words of
`0x40000000`, packed color `0x000000FF`, and final word `0x40000000`; the
browser then clears each block's byte at relative `+0x1C`. The native helper
`009A5BF2` initializes these blocks despite its misleading exported name.
Three embedded selection strings at `+0x328,+0x33C,+0x350` are constructed
and AddRef'd once. Evidence: constructor `009A57C4`–`009A5A89`, block helper
`009A5BF2`–`009A5C1F`, descriptor constructor `009A1F3C`.

Actual destruction (`009A31A2`, 144 bytes) installs browser tables, destroys
the descriptor's three strings (`009A1F5B`), then strings at `+0x1C8,+0x1B4,
+0x1A0,+0x184,+0x170`, the list at `+0x158`, filter-string vector at `+0x148`,
custom-group vector storage at `+0x13C`, and file-vector storage at `+0xDC`,
then enters the base destructor at `+4`. It **does not call browser Shutdown**
or its three clear slots. The base destructor directly calls base ChildDeleteAll
and base Shutdown (`0099E1A6`–`0099E1B3`); that is not virtual browser cleanup.
Thus browser Shutdown is required to release retained browser references and
destroy pointed-to file/drive/custom records. Normal final Release supplies
that call through FinalRelease; direct destruction alone must not be documented
as equivalent to Shutdown plus destruction.

Deleting wrappers `008AF100` (30 bytes) and `009A35DD` (28 bytes) invoke the
actual destructor, free the primary allocation only if the low bit of the
32-bit argument is set, and return the original primary pointer. Secondary
thunks `008AEB00` and `009A3232` subtract four before forwarding. Release
(`0099FC70`, 24 bytes) unsigned-compares the primary reference count at
`+0x10` with 1: values greater than 1 decrement and return the new value;
0 or 1 invoke primary slot 89 and return zero without a preceding decrement.
An already-zero
count is not a safe no-op release. Slot 89 is **shared FinalRelease `009CFA1D`
(25 bytes)**, not a deleting wrapper: it first calls secondary slot 5 (virtual
browser Shutdown), then secondary slot 148 with deletion flag 1. Thus final
Release does perform browser Shutdown before destruction (`009CFA1E`–`009CFA2E`).

Primary clear methods are Boolean-true operations even on empty collections.
Slot 104 (`009A2EC7`, 72 bytes) destroys each non-null file-entry's three
owned strings and frees that record, then clears the pointer vector without
freeing its capacity. Slot 105 (`009A2DA6`, 57 bytes) destroys each non-null
drive-record string and frees the record, then clears list nodes. Slot 106
(`009A35F9`, 134 bytes) visits every custom-entry group, releases each entry's
non-null interface at record `+4`, destroys its embedded string at `+8`, frees
the entry, clears the group-vector range, and resets selected index `+0x154`
to zero. These methods do not clear grid cells, mark dirty, or invalidate.
Filter-label strings in the separate vector `+0x148` survive these clear
operations and are destroyed with that vector at destruction. Evidence:
`009A2EC7`–`009A2F0E`, `009A1DB8`, `009A2DA6`–`009A2DDE`,
`009A35F9`–`009A367E`, `009A2E07`.

### 8.7 Primary property and forwarding contracts

All offsets here use the primary origin. Output pointers and string interfaces
are required to be valid; none of these methods checks them for null. Boolean
results mean the low return byte; do not infer a 32-bit return from residual
register contents. Dirty setters set `+0x31C` to true, then call window
Invalidate (secondary offset `0x170`); they do this even for repeated values.

| Primary slot / address | Contract |
|---|---|
| 1 / `0094D34F` (7 B, folded) | Increment the 32-bit reference count at `+0x10` and return it; non-atomic, no overflow guard. |
| 3 / `009CFA36` (12 B, folded) | Return the window pointer at primary `+4`, without AddRef; null input maps to null. |
| 7 / `0099FDAB` (29 B) | Given a 32-bit mask, if any masked flag is currently set, call primary SetFlags (slot 6) with those bits cleared. Otherwise no call. No explicit Boolean result. |
| 8 / `0099FDC8` (19 B) | Return 32-bit 1 if any requested mask bit is set in `+0x160`, otherwise zero. This is an any-bit test. |
| 9 / `00993785` (7 B, folded) | Return the entire 32-bit browser flags word `+0x160`; the export's other-owner name is misleading here. |
| 10 / `0099FDDB` (11 B) | Forward unchanged arguments to window GetChildWindowFromID, secondary slot 34, adjusting `this` by four. Retain the base result/ownership contract. |
| 14 and 16 / `009A0DCB` (30 B total, shared target) | Copy the current-directory bytes to the caller's string via its slot 3, using stored begin pointer and exact end-minus-begin length. Return true. No borrowed internal string is returned. |
| 17 / `009A00B6` (36 B) | Call primary slot 15 with the original one-word argument, then dirty/invalidate and return true, irrespective of that callee's result. Slot 15's replacement contract remains FBQ4. |
| 25 / `009A0081` (8 B) | Tail-forward to primary clear-custom-groups slot 106; preserve its true result and reset behavior. |
| 27 / `009A00DA` (34 B) | Write pending-selection word `+0x198` to the first 32-bit output, copy string `+0x1A0` through the second output string's slot 13, ignore the third argument, and return true. |
| 28 / `009A323A` (113 B) | Store the first 32-bit argument at `+0x198`, assign string `+0x1A0` from the second string interface, copy the third string interface into a temporary, append all its bytes to `+0x1A0` with no separator, then dirty/invalidate and return true. Both strings must be valid. There is no immediate selection or validation of the first word. |
| 29 / `009A0131` (20 B) | Copy the input string into new-directory label `+0x1C8` through its slot 13 and return true; no dirty/invalidate. |
| 34 / `009A0170` (29 B) | Write words `+0x12C` and `+0x130` to two 32-bit outputs, in that order; return true. |
| 35 / `009A0145` (43 B) | Store two unvalidated 32-bit sort-setting arguments at `+0x12C` and `+0x130`, dirty/invalidate, return true. Comparator interpretation is in 8.1.1. |
| 36 / `009A01AE` (17 B) | Write the entire wildcard-option word `+0x134` to one 32-bit output and return true. |
| 37 / `009A018D` (33 B) | Store an unvalidated 32-bit option at `+0x134`, dirty/invalidate, return true; no immediate population. |
| 76 / `009A0DE9` (30 B) | Copy parent-label bytes `+0x1B4` to the caller's string via its slot 3, using exact end-minus-begin length; return true. |
| 88 / `009A0AC4` (8 B) | Adjust `this` by four and tail-forward the paint arguments to GZPaint. No additional ownership or dirty behavior. |
| 89 / `009CFA1D` (25 B, folded) | FinalRelease: virtual browser Shutdown followed by deleting destruction, as section 8.6. |
| 92 / `00949ADE` (3 B, folded) | Return Boolean true without other effects. |

The cited method entries and complete native extents prove these widths and
channels. The five folded targets are shared code, not five distinct browser
implementations. A public wrapper's dispatch rule does not resolve the
contract of an unreviewed virtual target. Pending setter slot 28 (`009A323A`)
is distinct from clear-file-entries slot 104 (`009A2EC7`). Slot 28's third
argument is a string interface (`0040AE00` reads its byte pointer and length),
and `00408C20` appends its range. For example strings `report` and `.txt`
store pending name `report.txt`; slot 27 later returns that concatenated name
and leaves its third argument untouched. Restoration on population is in 8.1.3.

#### 8.7.1 Style and property widths (FBQ4, 2026-10-26)

The following are primary slots. A getter returning a pointer borrows that
pointer: it neither AddRefs it nor constructs an owned copy. Unless a row says
otherwise, a setter returns Boolean true in the low byte, sets dirty byte
`+0x31C`, then invalidates. There is no value validation or immediate child
reconfiguration. These rules include repeated scalar assignments.

Three externally configurable style blocks use the same six operations:

| Block origin B | Font get / replace | Word get / set | Byte get / set |
|---|---|---|---|
| `+0x238` | 42 `009A03B8` / 45 `009A031A` | 43 `009A03AA` / 46 `009A0363` | 44 `009A03BF` / 47 `009A0389` |
| `+0x25C` | 48 `009A0465` / 51 `009A03C7` | 49 `009A0457` / 52 `009A0410` | 50 `009A046C` / 53 `009A0436` |
| `+0x2A4` | 54 `009A0512` / 57 `009A0474` | 55 `009A0504` / 58 `009A04BD` | 56 `009A0519` / 59 `009A04E3` |

The font is the pointer at B. Replacement compares pointer identity first;
an identical pointer returns true without reference calls, dirtiness or
invalidation. On change, AddRef the incoming non-null pointer, Release the
old non-null pointer, store the new pointer, dirty and invalidate, in that
order. Null is accepted. Word operations take a 32-bit index I and address
the 32-bit word at B+4+4I. They have **no range check**; the intended six-word
style range is not enforced. The byte operations access B+`0x1D`, not the
constructor-cleared byte B+`0x1C`. A setter copies only the argument's low
byte, without Boolean normalization. A getter **sign-extends that byte to
32 bits**. Thus storing `0x80` reads back as `0xFFFFFF80`.
Evidence: complete bodies `009A031A`–`009A0520`; in particular indexed stores
`009A036B`, `009A0418`, `009A04C5`, byte stores `009A038D`, `009A043A`,
`009A04E7`, and signed loads `009A03BF`, `009A046C`, `009A0519`.

| Get slot / set slot | Stored state and boundary contract |
|---|---|
| 60 `009A0542` / 61 `009A0521` | Entire 32-bit word `+0x1E4`. |
| 62 `009A056A` / 63 `009A0549` | Entire 32-bit word `+0x1FC`. |
| 64 `009A0592` / 65 `009A0571` | Entire 32-bit word `+0x224`. |
| 66 `009A05CD` / 67 `009A0599` | Optional word `+0x300`; setter arguments are an enable byte and a 32-bit value. Nonzero enable stores the value; zero stores sentinel `0x40000000`. Getter writes the raw word to argument 1 and one byte to argument 2: true exactly when that word differs from the sentinel; return true separately. |
| 68 `009A0620` / 69 `009A05EC` | Same optional-word rule at `+0x304`. |
| 70 `009A068E` / 71 `009A063F` | Setter copies argument 1 to `+0x24C,+0x270,+0x294,+0x2B8`, then argument 2 to `+0x250,+0x274,+0x298,+0x2BC`, as 32-bit words. Getter writes only the first block's two words, in argument order; it does not compare all four blocks. |
| 72 `009A06E0` / 73 `009A06AB` | Three 32-bit words `+0x164,+0x168,+0x16C`, copied in argument order in either direction. |
| 74 `009A072A` / 75 `009A0709` | Common-font pointer `+0x2C8`: getter borrows; setter unconditionally overwrites the 32-bit pointer, dirty/invalidate, return true. **No AddRef or Release** occurs, even for replacement. Shutdown still releases the stored non-null pointer (8.5). A caller cannot assume the per-style retaining replacement rule applies here. |
| 80 `009A07A3` / 81 `009A0789` | Word `+0x230`: setter stores the unsigned maximum of the input and 4; return true, with **no dirty/invalidate**. For example `0xFFFFFFFF` stays `0xFFFFFFFF`, while 3 becomes 4. |

These are integer/pointer bit patterns, not recovered floating-point values.
The sentinel compares at `009A05DD` and `009A0630` are exact 32-bit equality;
the optional getter stores at `009A05E5` and `009A0638` are one byte wide.
Common-font store `009A070D` and unsigned clamp `009A0796`–`009A079C` prove
the exceptions to the generic style setter.

Worked cases for an implementation test: replace style font A with B and
observe AddRef(B), Release(A), dirty, invalidate; repeat B and observe none
of those effects. Set style byte to `0x80` and read `0xFFFFFF80`. Disable
optional word `+0x300` while passing 123: its getter writes `0x40000000` and
a single false byte and returns true. Enabling it with `0x40000000` produces
the same getter result. Set common font A then B: neither call changes either
reference count, and the eventual Shutdown releases B only.

The separate `0099FCFF` entry (5 bytes) is only a tail-forwarder to base-window
SetFlag `0099DB6B`. Its exported browser name does not make it an additional
browser flag implementation. It is counted with thunks in the audit below.

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
| FB-C23 | Replace retained icon A with B; repeat assignment of B | First call AddRefs B before releasing A; repeated pointer makes no refcount calls. Invalid index 7 makes no calls. |
| FB-C24 | Assign null button buffer at index 1 while Cancel button exists | Release old retained buffer, store null and forward null to the Cancel button. Reassign null: no child update. Index `0xFFFFFFFF` is ignored. |
| FB-C25 | Icon resource exists but destination lock fails | No resource copy; color key still set, icon setter called, local reference released and loader returns true. |
| FB-C26 | Icon buffer obtained but initialization fails | Loader returns false and releases local buffer; icon setter is not called. |
| FB-C27 | Select valid filter index twice; repeat with index equal to group count | Each valid call updates combo when present, marks dirty and invalidates without immediate file population. Out-of-range call returns false without those actions. |
| FB-C28 | Candidate `Report.TXT`, pattern `*.txt`; instrument matcher arguments | Match option zero submits lowercased copies `report.txt`/`*.txt`; nonzero submits original case; stored strings remain unchanged. |
| FB-C29 | Grid width 200, height 60, row height 20; icon/aux/extension widths 16/20/30 enabled; scrollbar width 12 | Three entries give filename width 132; four give 120. Equality with capacity does not reserve the scrollbar. |
| FB-C30 | Sorted rows parent, directory `Alpha`, file `Beta`; pending type 1/name `ALPHA`, then type 2/name `missing` | Restore row 1 first; missing file name falls back to row 0, including selection-change handling of the parent row. |
| FB-C31 | Empty sorted entry vector | Request grid selection at `(0,0,true)`, update and visibility/bring-into-view operations; do not invoke browser selection-change handler. |
| FB-C32 | Paint with background disabled and dirty set; UpdatePath sets dirty again | No background operations; primary slots 92 then 95; final dirty byte clear; true result. |
| FB-C33 | Background enabled, no buffer, area `(5,7,105,67)` | Set context color to primary +0x30C; fill exactly `(5,7,105,67)` before any dirty refresh. |
| FB-C34 | A retained drive followed by old depth-one record, then another drive; new SplitDirectory count 4 on the first drive | Old depth-one record destroyed; components 2,1,0 inserted in that order with depths 1,2,3; current-row 3 and column span 5; collapsed display uses component 0. |
| FB-C35 | No drive's lowercased first byte matches current directory | Nonzero-depth records removed; old collapsed cells not cleared/replaced; normal true result. |
| FB-C36 | Popup has 5 records, maximum visible 3, row height 20, column span 5 | Height 62, grid column count 13 (not 7), all five row heights installed, scrollbar fitting reservation enabled. |
| FB-C37 | Popup width 160, icon width 16, depth 2, R>M, scrollbar width 12 | Text fitting width 98; indentation-column width 9; text at column 4. |
| FB-C38 | File row has no own icon, default file icon nonnull; filename column disabled | Default icon installed if icon column enabled; fitted filename and style still requested; auxiliary style requested even when auxiliary column disabled. |
| FB-C39 | All setup enable bytes zero | Both graphics colors acquired; no children created; normal true result. Popup enable byte is not an AddControls creation request. |
| FB-C40 | Factory returns true with null child, or fallback allocation returns null | No owner recovery/false result; next required interface use is unsafe. |
| FB-C41 | Popup receives Enter, then Escape in a separate call | Enter gets current row, requests directory selection, closes, then base handling; Escape only closes before base handling. |
| FB-C42 | File-grid type 0x9A1552D4 has payload tag 5 and row 7 | Primary slots 103 then 100 both receive 7; base receives original message afterwards. |
| FB-C43 | Type 0x9B1552D8 from the retained filename editor versus an equal-ID different pointer | Only pointer identity produces the temporary 0x7ABE7203 descriptor notification; both reach base handling. |

These proposed cases specify owner calls and arithmetic; they have not been
executed and do not claim foreign rendering pixels or foreign layout policy.

## 10. Coverage limits and reconciliation

This is partial coverage: resource drawing, control layout, comparator ordering
and remaining primary property/selection methods remain questions. Lifecycle
and defaults have been recovered in sections 8.5–8.6, answering FBQ3.
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
| `008AEB00` | 8 | - | name-low | 8.6 (adjustor thunk) |
| `008AF100` | 30 | - | name-high | 8.6 (deleting wrapper) |
| `00949ADE` | 3 | 92 | new | 8.7 (folded) |
| `0094D34F` | 7 | 1 | new | 8.7 (folded) |
| `00993785` | 7 | 9 | new | 8.7 (folded) |
| `0099EC98` | 450 | - | name-high | 8.5 |
| `0099EE5A` | 55 | 12 | neighbour | 8.4 |
| `0099EE91` | 128 | 13 | neighbour | 8.4 |
| `0099EF11` | 3423 | - | name-medium | 8.3 partial / FBQ1 |
| `0099FC70` | 24 | 2 | name-high | 8.6 |
| `0099FC88` | 34 | 0 | name-high | 1.1 |
| `0099FCAA` | 85 | 4 | neighbour | FBQ4 |
| `0099FCFF` | 5 | - | name-low | 8.7 (base-forwarding thunk) |
| `0099FD04` | 27 | 5 | new | 5 |
| `0099FD1F` | 140 | 6 | new | 5 |
| `0099FDAB` | 29 | 7 | new | 8.7 |
| `0099FDC8` | 19 | 8 | new | 8.7 |
| `0099FDDB` | 11 | 10 | new | 8.7 |
| `0099FDE6` | 615 | 11 | name-high | 8.2.1 |
| `009A0081` | 8 | 25 | neighbour | 8.7 |
| `009A0089` | 45 | - | vtable | 8.5 |
| `009A00B6` | 36 | 17 | new | 8.7 |
| `009A00DA` | 34 | 27 | new | 8.7 |
| `009A00FC` | 53 | 90 | new | 6.1 |
| `009A0131` | 20 | 29 | new | 8.7 |
| `009A0145` | 43 | 35 | new | 8.7 |
| `009A0170` | 29 | 34 | new | 8.7 |
| `009A018D` | 33 | 37 | new | 8.7 |
| `009A01AE` | 17 | 36 | new | 8.7 |
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
| `009A0A17` | 173 | - | name-high | 8.5 |
| `009A0AC4` | 8 | 88 | neighbour | 8.7 (adjustor thunk) |
| `009A0ACC` | 687 | - | name-high | 8.2.2 |
| `009A0D7B` | 80 | 26 | neighbour | 8 |
| `009A0DCB` | 30 | 14,16 | neighbour | 8.7 |
| `009A0DE9` | 30 | 76 | neighbour | 8.7 |
| `009A0E07` | 46 | - | reach | FBQ6 |
| `009A0E35` | 120 | - | reach | FBQ6 |
| `009A0EAD` | 321 | - | name-medium | 8.3 partial / FBQ1 |
| `009A0FEE` | 373 | - | name-medium | FBQ1 |
| `009A1163` | 1938 | - | name-medium | 8.3 partial / FBQ1 |
| `009A18F5` | 116 | 20 | new | FBQ4 |
| `009A1969` | 25 | - | free-neighbour | 8.1.1 (shared string helper) |
| `009A1982` | 90 | - | reach | FBQ6 |
| `009A19DC` | 36 | - | reach | FBQ6 |
| `009A1A00` | 54 | - | reach | FBQ6 |
| `009A1A36` | 88 | - | reach | FBQ6 |
| `009A1A8E` | 43 | - | reach | FBQ6 |
| `009A1AB9` | 36 | - | free-neighbour | 8.1.1 (shared string helper) |
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
| `009A1DB8` | 28 | - | neighbour | 8.6 (record helper) |
| `009A1DD4` | 29 | - | neighbour | FBQ6 |
| `009A1DF1` | 29 | - | neighbour | FBQ6 |
| `009A1E0E` | 83 | - | reach | FBQ6 |
| `009A1E61` | 133 | - | reach | FBQ6 |
| `009A1EE6` | 41 | - | reach | FBQ6 |
| `009A1F0F` | 22 | - | neighbour | FBQ6 |
| `009A1F25` | 23 | - | reach | FBQ6 |
| `009A1F3C` | 31 | - | neighbour | 8.6 (record helper) |
| `009A1F5B` | 28 | - | reach | 8.6 (record helper) |
| `009A1F77` | 3149 | 91 | name-high | 8.2 (child-call boundary) |
| `009A2BC4` | 107 | 30 | name-medium | 2.2 |
| `009A2C2F` | 160 | 33 | neighbour | FBQ4 |
| `009A2CCF` | 215 | - | name-medium | 3.1 |
| `009A2DA6` | 57 | 105 | neighbour | 8.6 |
| `009A2DDF` | 40 | - | reach | FBQ6 |
| `009A2E07` | 31 | - | neighbour | 8.6 (record helper) |
| `009A2E26` | 161 | 23 | neighbour | FBQ4 |
| `009A2EC7` | 72 | 104 | neighbour | 8.6 |
| `009A2F0F` | 24 | - | neighbour | FBQ6 |
| `009A2F27` | 167 | - | reach | FBQ6 |
| `009A2FCE` | 140 | 24 | neighbour | FBQ4 |
| `009A305A` | 51 | - | neighbour | FBQ6 |
| `009A308D` | 90 | - | reach | FBQ6 |
| `009A30E7` | 187 | - | neighbour | FBQ6 |
| `009A31A2` | 144 | - | reach | 8.6 |
| `009A3232` | 8 | - | vtable | 8.6 (adjustor thunk) |
| `009A323A` | 113 | 28 | neighbour | 8.7 |
| `009A32AB` | 97 | 94 | neighbour | FBQ4 |
| `009A330C` | 315 | 93 | neighbour | FBQ4 |
| `009A3447` | 156 | 100 | name-high | 6.2 |
| `009A34E3` | 250 | - | neighbour | FBQ6 |
| `009A35DD` | 28 | - | reach | 8.6 (deleting wrapper) |
| `009A35F9` | 134 | 106 | neighbour | 8.6 |
| `009A367F` | 488 | 102 | name-high | 6.1 |
| `009A3867` | 444 | 103 | name-high | 6.1 |
| `009A3A23` | 208 | 101 | neighbour | FBQ4 |
| `009A3AF3` | 116 | 95 | name-high | 2.3 |
| `009A3B67` | 346 | - | name-medium | 8.1.2 |
| `009A3CC1` | 742 | - | name-high | 8.5 |
| `009A3FA7` | 547 | - | free-neighbour | 8.1.1 |
| `009A41CA` | 116 | 15 | new | FBQ4 |
| `009A423E` | 359 | 19 | name-medium | 6.3 |
| `009A43A5` | 259 | 18 | new | FBQ4 |
| `009A44A8` | 114 | 31 | new | 2.1 |
| `009A451A` | 549 | 32 | name-high | 7 |
| `009A473F` | 1007 | 98 | name-high | 8.1.5 |
| `009A4B2E` | 908 | 97 | name-medium | 8.1.4 |
| `009A4EBA` | 2241 | 96 | name-high | 8.1 / 8.1.3 / 8.1.6 |
| `009A5AAB` | 195 | 21 | new | FBQ4 |
| `009A5B6E` | 132 | 22 | new | FBQ4 |
| `009B689C` | 36 | - | reach | FBQ6 |
| `009B6A49` | 27 | - | reach | FBQ6 |
| `009CFA1D` | 25 | 89 | new | 8.7 (folded) |
| `009CFA36` | 12 | 3 | new | 8.7 (folded) |

### A.1 Final third-round reconciliation and parked residuals (2026-10-22)

Every one of the original 174 addresses occurs exactly once above; sizes sum
to 28,907 B. The 107 primary slots still resolve to 106 distinct targets
(slots 14 and 16 share one). They account for 15,406 B. No neighbouring
comparator pointer has been counted as slot 107.

The following disjoint attribution buckets reconcile all 174 entries. These
are function-body bytes, not distinct behavior counts or repaired boundaries.

| Bucket within the original union | Entries | Bytes | Exact identification |
|---|---:|---:|---|
| Browser method/callback bodies reached directly from the primary table | 100 | 15,344 | All P rows except the five folded targets and primary paint thunk `009A0AC4` |
| Browser method/callback bodies outside the primary table | 13 | 9,404 | `0099EC98`, `0099EF11`, `009A0089`, `009A0A17`, `009A0ACC`, `009A0EAD`, `009A0FEE`, `009A1163`, `009A2CCF`, `009A31A2`, `009A3B67`, `009A3CC1`, `009A3FA7` |
| Shared/folded primary targets | 5 | 54 | `0094D34F`, `009CFA36`, `00993785`, `009CFA1D`, `00949ADE` |
| Adjustor/base-forwarding thunks | 6 | 45 | `0047AA30`, `0047AA40`, `008AEB00`, `009A3232`, `0099FCFF`, `009A0AC4` |
| Deleting wrappers, same underlying destructor | 2 | 58 | `008AF100`, `009A35DD` |
| Owned-record construction/destruction helpers | 4 | 118 | `009A1F3C`, `009A1F5B`, `009A1DB8`, `009A2E07` |
| Shared string-comparison helpers | 2 | 61 | `009A1969`, `009A1AB9` |
| Attribution still unproved | 42 | 3,823 | Every FBQ6 row above, individually enumerated with bytes |
| **Total** | **174** | **28,907** | No row counted twice |

Outside that preserved union, constructor `009A57C4` is one additional owned
method / 710 B, fully specified in 8.6. The union including this newly found
constructor is therefore **175 entries / 29,617 B**. Style initializer
`009A5BF2` (46 B) is cited as a dependency only, not silently inserted into
either inventory or credited as another browser method. One of the body
entries, `009A3FA7`, is an owned global comparator callback rather than a
member method. The 42 unattributed entries prevent an exact total-owned-method
denominator or complete-owner claim.

Disposition audit, updated 2026-10-24: **66 original entries / 15,194 B have
complete owner-side contracts**
in the cited sections, counting shared targets, thunks and helpers separately
as above. This is not 60 unique browser methods. Of the 113 attributed browser
body entries / 24,748 B, **47 / 14,858 B** are specified and **66 / 9,890 B**
retain missing behavior. Adding the constructor gives 48 specified browser
method/callback bodies / 15,568 B; this count still excludes unproved ownership.
The complete-contract inventory count including all specified wrappers/helpers
and the constructor is 67 / 15,904 B. Partial sections receive no full-body
credit. The gain from the earlier 60-entry audit is paint (173 B), FBQ2's
two bodies (3,836 B), and FBQ5's three (4,156 B). Child/service calls are
specified at their boundary; their foreign implementations are not credited.

| Parked question | Exact residual entries | Export bytes | Missing contract |
|---|---:|---:|---|
| FBQ1 | 4 | 6,055 | Procedural pixel recipes, selectors and failure/success channels |
| FBQ4 | 62 | 3,835 | Remaining primary methods, notably style/font ownership and filter/custom-entry mutation |
| FBQ6 | 42 | 3,823 | Unresolved helper ownership / export boundaries |
| **Total unresolved within the original union** | **108** | **13,713** | Every address and size is individually enumerated in its FBQ row above |

FBQ2, FBQ3 and FBQ5 are answered. The largest residual bodies include
`0099EF11` (3,423 B), `009A1163` (1,938 B) and
`005C3A63` (575 B, attribution/boundary unproved). This is the final logical
third-round publication: preserve and park those contracts; no further round
or full-owner coverage is implied.

No full-owner, pixel-match, implementation or replacement credit follows
from this inventory. Questions FBQ1�FBQ6 delimit the missing contracts.
