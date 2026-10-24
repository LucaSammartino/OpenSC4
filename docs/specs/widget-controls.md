# Standard window controls

Target: SimCity 4 Deluxe, Windows GOG 1.1.641. This family comprises
`cGZWinGrid`, `cGZWinTextEdit`, `cGZWinListBox`, `cGZWinTreeView` and
`cGZWinFileBrowser`. Requirements marked *[compat]* describe the native
interface behavior. Internal representations may differ *[choice]*.
Addresses identify evidence, not a required implementation address.

## 1. Shared boundary and reading map

*[compat]* These controls use the window hierarchy, lifetime, notification
target, coordinate conversion, focus/capture, dispatch and drawing services
specified by [widget-toolkit.md](widget-toolkit.md). A control interface and
a window interface can designate different addresses of the same object.
Successful interface queries retain the object; callers release the returned
interface. Do not apply an offset from one view to another.

The following owner specifications are incorporated by reference. Their
explicit unknowns remain unknown; this family document does not turn partial
coverage into whole-owner coverage. The Grid listbox mode and the standalone
ListBox are separate interfaces with separate notification protocols.

| Owner | Authoritative detail | Inventory and vectors |
|---|---|---|
| Grid | [win-grid.md](win-grid.md), sections 1–5 | Section 3: 142 control slots; appendix A: 198 attributed functions; section 6 vectors |
| TextEdit | [text-edit.md](text-edit.md), sections 1–10; [layout extension](text-edit-layout.md), [byte helpers](text-edit-encoding.md), [adapters](text-edit-adapters.md) | [Inventory](text-edit-inventory.md), [attribution](text-edit-attribution.md); text-edit section 11 vectors |
| ListBox | Section 5 below | Control slots and window overrides in section 5; section 7 vectors |
| TreeView | Section 6 below | Control slots and window overrides in section 6; section 7 vectors |
| FileBrowser | [win-filebrowser.md](win-filebrowser.md), sections 1–8 | Appendix A inventory; sections 4 and 9 vectors |

No original rendering algorithm, OS filesystem implementation, timer
implementation or font rasterizer is imported by these contracts. An engine
must provide the observed interface calls, state changes and failure outcomes;
equivalent internal services are a design choice. Missing contracts are in
[questions/widget-controls.md](questions/widget-controls.md).

## 2. Grid

### 2.1 Interface and inventory *[compat]*

IID `0xDAA6B9BE` returns the control at object offset zero; the window view is
at `+4`. Constructor `009ABC95` installs primary table `00ADD578` (142 slots)
and window table `00ADD7B0` (151 slots). See win-grid sections 1–3 for the full
slot table and layout; appendix A distinguishes owner methods from helpers.

| Control slots | Method family and contract location in win-grid |
|---|---|
| 0–14 | Query/refcount, Init/Shutdown, window conversion, Refresh, area, flags; sections 3 and 5.1 |
| 15–29 | Default font/color/wrapping, gutters, outline, recalculation pause; sections 5.2 and 5.4 |
| 30–57 | Listbox mode, scrolling, selection, bounds and row/column dimensions; sections 5.3, 5.9 and 5.10 |
| 58–83 | Dimensions and heading data/format; sections 5.5, 5.10 and 5.13 |
| 84–108 | Cell formatting, clearing, deletion/copy and visibility; sections 5.11–5.14 |
| 109–141 | Visible range/hit test, text/bitmap/window values, extra data and service adapters; sections 3, 5.5–5.8 and 5.11 |

### 2.2 Construction, state and dispatch *[compat]*

Construction takes the grid flags; cell data, headings and selection start
empty. Gutters start at 5 per edge, cell gutters at 1, default cell size at
100 by 20, scroll positions at zero, cell limits at −1 (unlimited), and the
visible row/column counts at binary32 1.0. Init returns true and performs base
initialization/default formatting/recalculation only on first initialization.
Shutdown disposes owned cell data and releases/nulls retained interfaces
before base shutdown. The release-to-zero path invokes shutdown and deletion.
The full defaults and ownership order are win-grid sections 2 and 5.1.

DoMessage recognizes type 3, ID `0x887113A3`: sender sub-ID `0x1A7C4511`
updates vertical position and `0x1A7C4510` horizontal position from an existing
bar. A changed value requests layout recalculation. It always subsequently
delegates to base DoMessage (`009A7665`). Wheel input scrolls vertically by
signed `delta / -100`, truncated toward zero (`009A5F7D`). Character input
returns false. Mouse selection, modifier handling, key navigation and focus
redrawing are specified in win-grid sections 5.1 and 5.9, not ListBox section 5.

### 2.3 Drawing, data and failures *[compat]*

The cell value is typed: text, bitmap and child-window ownership cannot be
treated as interchangeable integers. Disposal, copying, moving and reparenting
are win-grid section 5.11. Font, buffer, sound and notification services cross
the interfaces in sections 5.2 and 5.5–5.8. GZPaint (`009AF168`) has the layer,
clipping, format inheritance and selection contract in section 5.12. Cell
rectangles and visible ranges use the grid/header/bar layout; they are not
simple multiples of default cell size when row/column overrides exist.

Preserve the documented no-op/false/null outcomes for bounds, missing data,
paused recalculation and unavailable service interfaces. Do not invent a
successful allocation or default resource where win-grid leaves a foreign
failure unresolved. Its questions remain in [questions/win-grid.md](questions/win-grid.md).

## 3. TextEdit

### 3.1 Interface and inventory *[compat]*

The window is W; the editor interface is T = W+`0xD8`, IID `0x231A1C57`.
Tables are `00ADFBD0` (window) and `00ADFA68` (editor). The native displacement
is `0xD8` even where the export thunk name says `228`. See text-edit section 1.
The [inventory](text-edit-inventory.md) records every direct-label body and
separately records thunks, folded aliases and native unnamed bodies. Its
134 direct bodies / 19,415 bytes are not a complete behavior count; the
[attribution](text-edit-attribution.md) reconciles the 150-body owner set.

Editor slots 0–7 cover interface/lifetime/window/area conversion (slot 3 Init,
4 Shutdown, 5 AsIGZWin, 6 SetArea, 7 GetTextArea). Slot 77 applies editor flags.
Remaining method and adapter meanings are defined in the linked inventory,
text-edit sections 2–10 and the adapter tables; uncertain export names must
not be used to infer argument types. Window message/paint hooks retain base
slot numbering from widget-toolkit. The inventory explicitly identifies
GZOnCharacter `009C0996`, GZOnKeyDown `009C0A95`, GZPaint `009BEA28`,
SetText `009C04FE`, Undo `009C027E` and Redo `009C0396`.

### 3.2 State, input and output *[compat]*

Text and selection indices are byte offsets, including when a character spans
several bytes. Defaults are empty text, insertion and selection endpoints zero,
unlimited maximum (`0xFFFFFFFF`), gutters 5/0, first visible line zero, mask byte
`*`, caret period 1000 and separate interval 500. Flags are initially zero and
then set from the constructor argument. Font, background, bars and clipboard
start absent; construction does not request the OS clipboard. See text-edit
section 1 for offsets and constructor ordering.

Whole-text replacement, insertion, overwrite, filtering, notification/sound
ordering, byte-length limits and code-page-specific truncation are text-edit
sections 2–4 and the byte-helper spec. Reversed selections are valid and
must not be silently reordered as stored state. Raw mutations, history and
undo/redo are sections 6–7. Keyboard shortcuts use the clipboard service
boundary there, not an assumed host shortcut policy. Layout, caret-in-view,
mouse mapping, focus and timer callbacks are sections 5, 8 and 10 plus the
layout extension. Paint emits through the buffer/font interfaces in section 9;
password display and byte hit coordinates remain distinct.

### 3.3 Failure and coverage boundary *[compat]*

Apply the linked raw-call domains, missing-resource, selection, history and
layout failure rules. In particular, null SetText input becomes an empty string,
and a zero input count requests strlen, not an explicit empty slice. Native
multibyte truncation can exceed the byte maximum to finish a character for
selectors 8 and 100. These are compatibility rules, not reasons to substitute
Unicode grapheme semantics. The unresolved foreign composition/helper cases
TE5 and TE8 remain in [questions/text-edit.md](questions/text-edit.md). This
family spec does not reopen that parked owner's research rounds.

## 4. FileBrowser

### 4.1 Interface, inventory and defaults *[compat]*

IID `0x3AA52E64` returns the primary view; the window is at +4. The actual
constructor `009A57C4` installs `00ADD190` (107 primary slots) and `00ADCF30`
(151 window slots). The adjacent comparator at `00ADD33C` is not slot 107.
win-filebrowser section 1.1 gives confirmed slots, section 8.7 the forwarding
and property methods, appendix A the address inventory and its limitations.

Defaults include empty entry/filter/custom-group collections, 13 absent
retained child controls with all 13 enable bytes true, flags `0xD`, filter
index zero, default comparator `009A3FA7`, empty current/fallback/pending paths,
parent label `..`, and new-directory label `New Folder`. The constructor
creates no child controls or graphics. Complete dimensions/style blocks are
in win-filebrowser section 8.6; Init/AddControls own resource creation.

### 4.2 Dispatch, drawing, IO and failures *[compat]*

The browser orchestrates child controls and retains their references; its
file grid uses section 2's Grid contract. Navigation, path acceptance and
entry lookup are win-filebrowser sections 2–3. Selection, notifications and
double-click are section 6; directory creation is section 7. Population,
comparison, truncation-to-width, child enables, resource creation, icon/buffer
setters, lifecycle, DoMessage and paint are section 8. Use these interface
contracts for file enumeration and font/buffer operations; host filesystem
behavior alone does not define browser filtering, ordering or notifications.

Existence failure, failed child queries, absent controls and partial resource
construction retain their explicitly specified outcomes. Construction is
separate from control/resource creation and can succeed before these later
operations fail. Do not claim atomic rollback or whole-owner completeness:
the residuals listed in section 10/appendix A.1 and
[questions/win-filebrowser.md](questions/win-filebrowser.md) still apply.

## 5. ListBox

### 5.1 Identity and method inventory *[compat]*

Let W be the window/object start and L = W+`0xD8` the list interface.
QueryInterface `009C83DE` recognizes IID `0x4132242B`, writes L and AddRefs
through W. For this IID a null output pointer returns false without AddRef;
other IIDs delegate to cGZWin. Constructor `009CA883` installs window table
`00AE1780` and list table `00AE1678`. Native thunk `009CAA0A` subtracts
`0xD8`; the exported decimal-228 name is misleading.

The list table contains **66 slots**, ending immediately before window table
`00AE1780`. This inventory gives native targets, not inferred C++ signatures.
“Unresolved” means interface presence only; see WC1. A slash separates adjacent
slots, not alternate targets for one slot. The table was checked against the
native executable words at `00AE1678`–`00AE177C`.

| L slot(s) | Target(s) | Established role / boundary |
|---|---|---|
| 0–3 | `009CAA0A` / `0095BA81` / `0095BA8C` / `0095BA6F` | QueryInterface / AddRef / Release / AsIGZWin adapters |
| 4 | `009C93D7` | Remove entry by index |
| 5–6 | `009C9E74` / `009CA084` | Removal wrappers; string-to-index search boundary unresolved |
| 7–8 | `009C9F12` / `009C8B24` | Insert text at index with ID / replace text at index |
| 9 | `009C8A7E` | Resize height to whole lines |
| 10–13 | `009C8CEE` / `009C9C82` / `009D8564` / `009C9E41` | Select index / select ID / get selected index / get selected ID |
| 14–18 | `009C8F3C` / `009CEC4A` / `009CB16A` / `009C88D4` / `009C88DE` | Update suppression and associated adapters; WC1 |
| 19 | `009C9AEB` | Text-area output rectangle |
| 20–21 | `009C8E72` / `009C8E9D` | Previous/next selection without notification |
| 22–23 | `009C8E29` / `009C8E46` | First/last navigation adapters; WC1 |
| 24–25 | `009C8EC4` / `009C8EF3` | Page navigation; previous page in section 5.3 |
| 26–29 | `009C8D71` / `009C8D8F` / `009C9CFA` / `009C8DD1` | Is row visible / make row visible / select first prefix match / set first visible row |
| 30–33 | `009C8F69` / `009CDC27` / `009D858F` / `009CDC23` | Count and state accessors used by owner; full alias/signature audit WC1 |
| 34 | `009C8F99` | Row hit test; x does not affect row calculation |
| 35–36 | `009C912B` / `009C913E` | Service/notification-target adapters; WC1 |
| 37–38 | `009C88E2` / `009C8934` | Background buffer / font reference setters |
| 39–40 | `009C8969` / `009C89A4` | Indexed font-color set/get |
| 41 | `009C89CB` | Font-color adapter; WC1 |
| 42–43 | `009C89DF` / `009C8A09` | Selection color conversion/set and companion adapter |
| 44–46 | `009C8A31` / `009C8A3B` / `009C8A3F` | Selection-highlight enable set/get / intensity setter (numeric edge cases WC1) |
| 47–50 | `009C900F` / `009C9D3B` / `009C9EBB` / `009C9BD7` | Drop sizing/scrollbar adapters; WC1 |
| 51–54 | `009C8783` / `009C879F` / `009C87A3` / `009C87BF` | Flat-outline set/get / 3D-outline set/get |
| 55–60 | `009C87C3` / `009C87FE` / `009C8848` / `009C8877` / `009C888E` / `009C88BD` | Style/gutter adapters; WC1 |
| 61–63 | `009C8F70` / `009C98BD` / `0093877E` | Minimum row-height byte setter / adapter / shared stub; WC1 |
| 64–65 | `009C919E` / `009C91AD` | Notification delivery mode adapters; WC1 |

W uses the common 0–150 window slot numbering. Owner overrides are
0 QueryInterface, 4 Init, 5 Shutdown, 68 SetFlag, 88 GZPaint, 130/131 key
down/up, 134 mouse-down-left, 136 mouse-up-left, 138 mouse-move and 148 deletion.
DoMessage (3) is inherited, as is the wheel hook (139).

The W extension is particularly vulnerable to misleading export names:

| W slot | Target | Actual role established from body/callers |
|---:|---|---|
| 151 | `009C8AF2` | Find an entry, then remove through L slot 4 |
| 152 | `009C8BB0` | Character/prefix navigation when not capturing; returns false |
| 153 | `009C8909` | Replace retained border buffer |
| 154 | `009CA6D0` | Copy indexed entry's text into an output string; false if lookup fails |
| 155 | `009C9C20` | Find matching text and output the first matching index; comparison helper boundary WC1 |
| 156 | `009C8666` | Recompute row height and whole visible-line count |
| 157 | `009C9B9B` | Return indexed entry's string interface through output; false if absent |
| 158 | `009CA0DF` | Remove a stored entry |
| 159 | `009CAC56` | Insert a stored entry |
| 160–161 | `009C9A19` / `009C9A90` | Update vertical scrollbar range/value / place its window |
| 162–163 | `009C993A` / `009C9155` | Add/remove scrollbar |
| 164–165 | `009C91EE` / `009C924E` | Emit selection-change / activation notification |
| 166 | `009C91B4` | Drop completion, returning whether this is a drop list |
| 167–168 | `009CA9FA` / `009CAA02` | Forward to selection-change / activation emitters respectively |

Do not implement W slots 158/159 as “SetSelection/SetSelectionByID”, or 160/161
as selection getters, despite those export labels. L slots 10–13 provide the
selection operations. This distinction is observable through ABI calls.

### 5.2 Construction, state and resource ownership *[compat]*

Constructor `009CA883` takes a drop-list byte. List storage starts empty;
selection and previous-click row start −1. Scroll origin and visible-line
count start zero. Stored minimum height starts zero, cached row height 10,
and selection intensity binary32 1.0. Font, border/background buffers and
two optional service references start null. Flat/3D outline and suppression
bytes start false; drop resizing bytes start true/false. Selection highlighting
starts enabled. Selection color and two outline colors start `0xDEADBEEF`
(unresolved resource sentinel). Default bar gap and text x gutter are 3, y
gutter 1. A timer is allocated for double-click tracking. Native stores and
binary32 width are verified at `009CA8B8`–`009CA97B`.

Relevant W state: entries `+0xDC`, intensity `+0xE0`, selection color `+0xE4`,
font `+0xE8`, three font-color words `+0xEC..+0xF4`, selection `+0xF8`, whole
visible rows `+0xFC`, first row `+0x100`, row height `+0x104`, minimum height
`+0x108`, flat/3D/drop bytes `+0x10D/+0x10E/+0x10F`, border/background
`+0x114/+0x118`, previous-click row `+0x120`, text gutters `+0x12E/+0x12F`,
highlight-enable `+0x132`, delivery-mode byte `+0x160`, bar-capture tracking
`+0x161`, entry count `+0x164`. Entries contain text and a separate caller ID;
IDs need not equal indices or be unique.

Init `009C8415` returns true immediately if base initialization is already set.
Otherwise it requests missing defaults from WCM when available: font
`0x68963C4C`; outline colors `0xA8963FAD/0x08963FAF`; font colors
`0x08963FB9/0x28963FBC`; selection color `0x88963FB4`. Explicit non-sentinel
colors and a preinstalled font survive. Drop mode additionally sizes the list,
sets origin zero, reapplies selection, takes capture and, if possible, obtains
border resource `0xE8963EE5`. It then updates bar placement/range and row
metrics before returning base Init's result. Missing WCM is a skipped default
lookup, not an owner-level Init failure.

Replacing font, background or border AddRefs the incoming nonnull interface
before releasing the old one; null clears it. The font setter recomputes row
metrics. Flat-outline setter clears 3D-outline; 3D-outline setter clears flat;
both invalidate even when assigned the existing byte (`009C8783/009C87A3`).
Shutdown hides the window, then releases/nulls border, background, font and
the references at W+`0x134/0x138`, in that order, then delegates to base
(`009C85EE`). Timer/container destruction belongs to destruction, not these
five releases. Allocation failure recovery for construction remains WC1.

### 5.3 Entries, selection and geometry *[compat]*

Public insertion (`009C9F12`) places text with a caller ID. Index −2 or greater
than count appends; other negative indices become zero. Inserting before or
at a selected entry increments the selected index to keep the same logical
entry selected. On the first insertion, selection becomes zero only if it was
not −1. It updates drop sizing, bar presence/range/placement and invalidates.
Text replacement (`009C8B24`) fails on an empty list or failed entry lookup;
otherwise −2 or an index at/above count means last, other negatives mean first.
It changes the entry's text, updates drop sizing when enabled, invalidates and
returns true. Public removal (`009C93D7`) rejects indices above the last;
it adjusts scroll origin and selection, then updates/removes the vertical bar
and invalidates. Negative removal indices and allocation failure are WC1.

Selecting an index (`009C8CEE`) preserves −1 as no selection; other values clamp
to [0,count−1], with empty count yielding −1. A changed nonnegative selection
outside the visible range is made visible. With notify=true and outside drop
mode it emits selection-changed **even if the index did not change**. It always
invalidates and returns true. Selecting ID (`009C9C82`) searches from the first
entry, applies the same visibility/notify/invalidation rules to the first
matching ID, and returns false without those effects if absent. Getting the
selected ID (`009C9E41`) returns zero when selection is invalid: ID zero is
therefore not an unambiguous no-selection marker.

Row height (`009C8666`) is the maximum of minimum height, background-buffer
height (when present) and font metric at font byte offset `+0x8C` (when present).
Whole visible rows are zero for height zero, otherwise signed division of
`windowHeight − 2*yGutter` by rowHeight, truncating toward zero. The native
operation is IDIV, with a sign-extended gutter byte (`009C86D4`–`009C8702`).
Whole-line resize (`009C8A7E`) returns false for zero row height; otherwise it
shrinks height to the whole-line multiple plus both y gutters, updates the
bars and invalidates, returning true.

IsVisible(row) means `first <= row < first+wholeRows`. MakeVisible moves the
origin to row when above, or `row−wholeRows+1` when below, then refreshes the
bar and invalidates. SetFirstVisible (`009C8DD1`) clamps the origin to at least
zero; it caps at count−wholeRows only when count exceeds wholeRows. Previous
selection fails at the first row, on no selection or an empty list; otherwise
it selects index−1 without notification. Previous page chooses
`max(0,first−wholeRows+1)`, sets that origin and selects it without notification.
First-prefix-match (`009C9CFA`) searches in list order and selects/notifies the
first BeginsWith match with helper argument 1; missing match returns false.
Case/encoding details of that helper remain WC1.

HitTest (`009C8F99`) returns −1 for no entries. With clamp=true it clamps y
into [0,height−1]. The row is signed
`(y−yGutter)/rowHeight + first`, truncated toward zero. A row at/above count
becomes last when clamping and −1 otherwise. There is no separate negative-row
clamp after division; x does not affect this calculation. Nonzero row height
is a caller precondition; see native `009C8FE4`–`009C9005`.
TextArea (`009C9AEB`) normalizes the window rectangle to local origin, insets
by x/y gutters, and subtracts vertical bar width plus bar gap from the right
edge when that bar exists.

#### Removal, page forward and drop sizing (WC4)

For a valid removal index `r`, old count `n`, selection `s` and origin `f`,
storage is removed first. The requested origin is `f−1` only when `r<f`,
otherwise `f`; SetFirstVisible applies its usual clamp. Selection decrements
only when `r<s`, or when `r=s=n−1`. When the selected row has a successor,
the index is retained and now designates that successor, with no select call
or selection notification from this adjustment. The decrement path calls
Select with notify=true; its ordinary/drop notification rules still apply.
Removing the sole selected row yields −1. Subsequent bar removal when all
remaining entries fit resets origin to zero. Drop-height/width refreshes run
when their respective bytes are enabled (`009C9408`–`009C949F`).

NextPage always returns true. It acts only when `n>0` and `f<n−v`, where `v`
is whole visible rows. It requests origin `p=f+v−1`, then selects `p+v`
with notify=false. The latter is based on the requested origin, not the
clamped origin returned by SetFirstVisible; Select can clamp the row and move
the origin again to make it visible. The apparent upper clamp of `p` tests
the old `f` against `n−v`, so it is ineffective inside this branch
(`009C8EF9`–`009C8F33`). For example `n=20,v=4,f=0` requests origin 3,
selects row 7, and ends with origin 4. For `n=10,v=4,f=5` it requests 8,
origin clamps to 6, and selection clamps to row 9. Do not replace this with
a conventional one-page advance.

Drop sizing is persistent policy: height/width sizing calls set the
corresponding enabled byte in drop mode. Insertion, text replacement and
removal run enabled height sizing then enabled width sizing. Init first
recomputes row metrics, runs those same enabled policies, sets origin zero,
and makes the selection visible before taking capture. Height policy is
enabled by default; width policy is disabled by default. Outside drop mode
both sizing calls return false without setting policy (`009C8415`,
`009C8B24`, `009C9F12`, `009C900F`, `009C9D3B`).

Height sizing marks its policy enabled even when it returns false because
the list is empty or `n<v`. Otherwise desired height is `n*rowHeight+2*yGutter`.
Available height is `parentHeight−windowTop−2`; absent parent uses
`INT_MAX` for parentHeight. It preserves width and requests the lesser of
desired/available height. When desired fits (including equality), it sets
wholeRows to `n`, resets origin zero and removes the bar. When clipped it
sets wholeRows to signed `(available−2*yGutter)/rowHeight` truncated toward
zero, adds the bar and calls whole-line resize, so actual height can be
smaller than available. Both branches refresh bar placement/range and
invalidate. Gutters are sign-extended bytes, products/subtractions are
32-bit integer operations, and the clipped division is IDIV
(`009C903F`–`009C911A`). There is no minimum-height or zero-divisor guard in
that branch, nor an automatic empty-list shrink.

Width sizing measures every stored string through font slot `+0xC4`, takes
the unsigned maximum starting at 1, and sets width to that maximum plus
twice the signed x-gutter byte. It does not add scrollbar/border widths or
strip color prefixes for measurement. Empty storage therefore requests
`1+2*xGutter`. A nonempty list without a font requests the default font
through WCM; this path assumes usable WCM/font services rather than returning
a clean allocation failure (`009C9D3B`–`009C9DCC`).

### 5.4 Messages and capture *[compat]*


There is no ListBox-specific DoMessage override. Base dispatch invokes the
window hooks. Key-down `009C8BD1` consumes PageUp/PageDown/Up/Down, invokes the
corresponding navigation method, and issues a notifying selection call if the
index changed. Enter reapplies selection with notify=true, then emits activation
in ordinary mode or completes the drop with the selected index. Escape in drop
mode completes with −1. Other keys in drop mode are consumed. Ordinary mode
forwards unhandled key-down and all key-up to the notification target as base
message types 5 and 6 with key/modifier arguments. Drop key-up returns true.

Mouse-down `009C94AD` computes a clamped row. On first capture it takes capture
and selects/notifies a changed in-range row. Repeated click on the stored row
sets the pending-double-click byte only when timer elapsed is **strictly less**
than WCM's double-click interval, then restarts the timer. Mouse-up without
capture succeeds without effects. Ordinary mouse-up releases capture and emits
pending activation, resets previous-click row to −1 and clears the pending
byte (`009C9623`). Drop mode retains capture while forwarding applicable input
to a scrollbar. Drop mouse-move otherwise changes the hovered row with
notify=false; ordinary mouse-move does not change selection (`009C9769`).
Scrollbar forwarding converts coordinates into the bar window, uses base
message 10 for left-up and 13 for move, and reacquires list capture after
dispatch. The exact asymmetric bar-capture tracking is WC1.

| Event | Delivery (`009C91EE/009C924E/009C91B4`) |
|---|---|
| Selection | Type 3, ID `0x08710E0F`, notification target, selected row converted through base slot 63 |
| Activation/double-click | Same envelope, ID `0x08710E10` |
| Send vs post | W+0x160 zero calls W slot 144 SendMsg; nonzero calls slot 146 PostMsg |
| Drop completion | Only in drop mode: converts the result through L slot 3 and calls the window manager's slot 9 with type `0xF`; returns whether drop mode is active |

Emitter methods return true independently of the target's return. Selection
and activation payloads are row indices, not caller IDs. Do not silently
substitute Grid notifications.

### 5.5 Drawing and failure boundary *[compat]*

GZPaint `009CA19A` uses the current draw-buffer/font interfaces. It excludes
the bar width and gap, reads bar value as first visible row, fills with base
fill when no background buffer exists or blits that buffer otherwise. A drop
border buffer is rendered with the edge helper. A visible selected row is
highlighted only when enabled and intensity is nonzero; the solid fill branch
requires intensity exactly 1.0. Do not assume arbitrary fractional intensity
produces a blend: its pixel result is unresolved WC1.

Text drawing starts at first visible row, advances by cached row height,
clips the last row to the drawing bottom and emits through the font interface.
Normal/selected/alternate font colors are independent of selection fill.
Strings beginning with the color escape are interpreted for drawing rather
than changing stored text; malformed-prefix treatment,
ellipsis fitting and fractional intensity remain WC1. Flat or 3D outline is
drawn after rows when a font exists. With no font, the row text/outline branch
is skipped; background work still occurs. No filesystem/clipboard IO belongs
to this control. Missing bar/resource pointers skip the guarded owner work;
font drawing, draw buffer validity and nonzero divisor are service/caller
preconditions. No blanket allocation rollback or exception contract is proven.

Indexed font-color setter accepts indices 0–2, returns false otherwise. It
stores the supplied word and invalidates **only when old equals new**, not when
changed (`009C8972`–`009C899A`, instruction-verified). Getter returns false for
out-of-range or sentinel `0xDEADBEEF` without writing output; otherwise writes
the word and returns true. Boolean returns are low-byte values, not a promise
about all EAX bits (`009C89A4`).

The color-prefix predicate (WC5) is a leading ASCII `#` byte (`0x23`),
not an ESC control byte or markup tag. The literal `##` escapes it: exactly
one leading byte is removed, leaving the rest as ordinary displayed text;
there is no recursive parsing. A single leading `#` takes the color-prefix
branch, which requires at least seven bytes, removes those seven bytes,
and, in the window-flag-2-enabled branch, scans `#%02x%02x%02x` for RGB.
With that flag disabled it still strips the prefix but keeps the alternate
font-color policy. Prefix recognition and slicing modify only the temporary
paint string. Constants at `00A9F354` and `00AA6DDC` are respectively `#`
and `##`; calls/length checks are at `009CA457`–`009CA508`, with the
alternate-color counterpart starting at `009CA534`. Malformed scan outcomes
remain within WC1; a '#' prefix is recognized even when its suffix is not hex.

## 6. TreeView

### 6.1 Identity and inventory *[compat]*

Let O be the primary control and W=O+4 the base window. IID `0x3AE8BAD2`
returns O with AddRef (`009CFA42`); other IIDs delegate through W. The own-IID
path requires a valid output pointer: it does not guard null. Constructor
`009D2BF0` installs primary `00AE3240` and window `00AE2FE0`. Native primary
table has **60 slots**, followed at `00AE3330` by non-pointer data `0x40490FDB`;
do not extend it into the next pure-call table.

| O slots | Target(s), in slot order | Role / boundary |
|---|---|---|
| 0–2 | `009CFA42` / `0094D34F` / `009D006C` | QueryInterface / AddRef / Release |
| 3–7 | `009CFA64` / `009CFAA6` / `009CFAB9` / `009CFAF2` / `009CFB0D` | Property/flag adapters, WC2 |
| 8–11 | `009B002D` / `009CFB32` / `009CFB49` / `009CFB64` | Property/service adapters, WC2 |
| 12–16 | `00879480` / `009CFB89` / `009CFBBA` / `009CFBC1` / `009CFBF8` | Property/service adapters, WC2 |
| 17–18 | `009D0081` / `009CFC0C` | Property/service adapters, WC2 |
| 19–22 | `009D3F7B` / `009D34A7` / `009D0C63` / `009D407E` | Image/resource adapters (22 releases image-list entries after first four); WC2 |
| 23–25 | `009D3D60` / `009D3CBA` / `009D3A7A` | Insert wrappers / descriptor-based InsertNode |
| 26–28 | `009D1FAB` / `009D2E07` / `009CFC13` | Move node / removal-family boundary / adapter; WC2 |
| 29–30 | `009D33CD` / `009D2092` | Get/set node descriptor |
| 31–38 | `009D219B` / `009D21BA` / `009D21F2` / `009D2209` / `009D2226` / `009D225F` / `009CFC1B` / `009D22BD` | Node text/style/data adapters; exact per-slot contracts WC2 |
| 39 | `009D22D4` | Set node font (null uses default) |
| 40–44 | `009CFC50` / `009CFC67` / `009D2343` / `009D237B` / `009D23D9` | Node properties/expanded getter; WC2 |
| 45–47 | `009D23F0` / `009D3E03` / `009D3E50` | Set expanded / get visible / make visible |
| 48–50 | `009D246B` / `009D2485` / `009D24C4` | Node properties/selected state; WC2 |
| 51–52 | `009D24DB` / `009D24FC` | Parent handle / stored insertion-reference word; −1 if unavailable |
| 53–55 | `009D00B3` / `009D00FA` / `009D00C5` | Selection accessors; WC2 |
| 56–59 | `009D2EE6` / `009D081F` / `009D2514` / `009CFA36` | Select / deselect / clear selection / AsIGZWin |

The 151-slot W table overrides 0/2 interface/lifetime thunks, 3 DoMessage,
4 Init, 5 Shutdown, 88 GZPaint, 130/131 key-down/up, 134/135 left/right down,
138 mouse-move, 139 wheel, and 148 deletion. Other entries use base/shared
contracts. Important non-slot methods: InternalInit `009D3956`, root creation
`009D3312`, GetNodeByHandle `009D1EE1`, RecalculateAll `009D354F`, FindTreeInfo
`009D2FD6`, DrawNode `009D17A8`, DrawItem `009D0CB1`, image-list initialization
`009D0888`, scrollbar creation/check/update `009D0112/009D01C0/009D0331/009D049B`,
selection-cache repair `009D0FD1`, cursor lookup `009D0626`, drag notification
`009D067F`. Drag helper hooks are Init `009CFDCE`, Shutdown `009CFE60`, paint
`009CFE70`, mouse-up `009CFEF0` and move `009D141A`; detailed drag behavior WC3.
This is an interface inventory, not a complete proof of all these bodies.

### 6.2 State, defaults and lifetime *[compat]*

The model has a handle map, ordered child lists, a flattened visible-node
sequence, and a separate selection sequence. A node's handle is not its row
number. O offsets: next handle `+0xDC`, map `+0xE0`, visible sequence `+0xF8`,
selection sequence `+0x104`, root `+0x110`, default font `+0x114`, flags
`+0x118`, internal-init/layout-dirty/selection-dirty bytes `+0x11C..+0x11E`,
row height `+0x128`, working buffer `+0x14C`, horizontal/vertical bars
`+0x150/+0x154`, first visible row `+0x158`, image-list
sequence `+0x1B8`. Node state includes handle +0, insertion-reference +4,
text +8, user data +0x1C, font +0x20, four colors +0x24..+0x30, four image
indices +0x34..+0x40, expanded +0x44, force-branch +0x45, selected +0x46,
flattened-visible +0x47, hit rectangles, visible index +0x7C, parent +0x80
and child list/count +0x84/+0x88.

Constructor defaults: empty map/sequences, next handle zero, absent root/font/
working buffer/bars, flags `0xB`, row height 16, `+0x130=9`, margins
`+0x134/+0x138/+0x13C=3`, `+0x148=44`, `+0x170=1`, color words at
`+0x124/+0x174..+0x180=0xDEADBEEF`, false initialization/dirty/scroll-state
bytes. These are raw stored defaults, not inferred names for every style word.
Native integer and byte stores: `009D2C4E`–`009D2D3D`.

InternalInit runs once, marks itself initialized, resolves sentinel base color
to −1 and default four colors to black/white/white/black through window color
conversion. It requests font `0x68963C4C` if absent, AddRefs a returned font,
initializes the image list, creates a root if absent and requests a missing
working buffer from the graphics service. Failed buffer initialization releases
and nulls that buffer. Init calls base Init, invokes InternalInit
if needed, recalculates and returns true (`009D3F56`–`009D3F7A`, instructions
checked); it does not propagate base Init's return value.

Root creation `009D3312` makes handle zero, expanded and force-branch true,
unselected, no parent, data zero, image indices 0/1/2/3 and default font/colors.
It increments the next handle and installs the root in the map. Repeated root
creation when present does nothing. Node lookup returns null for an absent
handle. Shutdown clears visible/selection caches, destroys mapped nodes and
image entries, releases/nulls default font, working buffer and both bars, then
base shutdown (`009D320F`, W-relative entry). Root and resource allocation
failures are not safely guarded throughout; do not promise successful partial
initialization or atomic rollback. Exact restart-after-shutdown behavior WC2.

### 6.3 Node operations and selection *[compat]*

InsertNode `009D3A7A` lazily initializes. Its descriptor begins with parent
handle, insertion reference, text interface, user data, four image indices,
font, four color words and an expanded byte at descriptor+`0x34`. It returns
−1 for absent parent/map, or an insertion reference that cannot be found.
On success it consumes the next handle, copies text/style/data, assigns parent,
starts unselected and not force-branch, inserts in the parent's ordered list,
updates child count/map, marks layout dirty and invalidates. Null font uses
the default. Row height grows to accommodate the font metric, but is not
reduced by this operation. Ordering sentinels −2, −3, −4 exist; the exact
prepend/append/sorted helper contracts and invalid-reference mutation/leak
behavior are WC2. An allocation returning null is subsequently dereferenced;
no graceful false/−1 result is established for that case.

SetExpanded `009D23F0` fails for an absent handle. On a changed byte it marks
layout dirty, invalidates, then sends expand `0x1AF5A13C` or collapse
`0x1AF5A13D` with the node handle converted through W slot 63. Same value
succeeds without notification. GetVisible `009D3E03` refreshes dirty layout,
then requires the node's flattened-visible byte and row within the current
viewport; it is not simply the expanded byte. MakeVisible `009D3E50` fails for
an absent handle; otherwise it refreshes dirty layout, expands closed ancestors
while sending expand notifications, recalculates and fails if still absent
from the flattened sequence. It adjusts the vertical bar when above/below
the viewport, without changing selection. Node-font setter `009D22D4` uses
default font for null, marks layout dirty/invalidate only on a changed pointer,
and grows row height when necessary. Per-node font AddRef is not observed.

Select `009D2EE6` fails for an absent handle. Its second argument forces
replacement when true; false permits accumulation only when control flag
`0x10` is enabled. A newly selected node is made visible, marked selected and
appended to the selection sequence. A replacement removes other selection
entries and clears their selected bytes. On a changed selection it invalidates
and sends `0x1AF5A13A` with converted zero payload; unchanged selection succeeds
without notification. Deselect `009D081F` removes the matching handle from
the selection sequence, clears its selected byte, invalidates and sends the
same message; absent selection returns false. Clear `009D2514` does this once
for all selected nodes, sends once if nonempty, and returns true even if empty.
Descriptor get/set at `009D33CD/009D2092` transfers text/data/style and state;
the setter invokes expanded, make-visible and select/deselect operations as
needed, preserving their notifications, then dirties layout and invalidates.
The complete output descriptor ABI and child-handle copy are WC2.

#### Root inclusion and navigation anchor (WC6)

Flattening is preorder in child-list order. Control flag `0x2` includes the
special root (node byte `+0x45`); clearing it omits that node and visits its
children at the same depth, even if its expanded byte is false. With the flag
set, the root occupies row zero and its children require expansion. Ordinary
nodes occupy a row regardless of their expanded byte; only their descendants
are gated by expansion. A special root does not increment indentation depth
for its children. A hidden root has its flattened-visible byte cleared with
the other nodes and is not added back. These are owner rules, not an implied
native-treeview style (`009D2FEB`–`009D30EF`). Keyboard navigation uses the
last entry in the selection sequence when there are several, the sole entry
when there is one, and row zero when none. It does not use the first selected
node (`009D15C1`–`009D15E3`).

### 6.4 Input, dispatch and notification *[compat]*

DoMessage `009CFD0B` recognizes type 3 / ID `0x887113A3`. Sub-IDs
`0x1756BAC7` and `0x1756BAC6` read horizontal/vertical bar values and update
cached positions. It subsequently calls base DoMessage for every message.
These are TreeView IDs, not Grid's scrollbar IDs.
The matching bar must exist: these message branches dereference it without
a null guard (`009CFD2C`, `009CFD4C`).

Wheel `009CFF09` consumes the event even without a vertical bar. With a bar,
negative delta moves down **one row** when first+visible count is below total;
zero or positive delta moves up one when first>0. Magnitude is ignored; zero
is not a no-op. Integer width and unsigned boundary comparisons are checked
at `009CFF09`–`009CFF43`; these offsets in that entry use W, not O.

Key-down `009D14BF` returns true. Escape clears selection. Enter sends
activation `0x1AF5A13B` for each selected node in sequence order and toggles
expanded state for nodes having children or force-branch, emitting corresponding
expand/collapse messages. Home/End/Up/Down/PageUp/PageDown navigate the visible
sequence and select one node with replacement=true. Left collapses an expanded
node or navigates to its parent when already closed; Right expands a closed
node, otherwise advances one visible row. Navigation clamps to the visible
range; zero-node indexing and exact page arithmetic remain WC3. Key-up returns
false (`009CFF04`).

Mouse-down `009D1031` operates on the node under the cursor; expander hits
toggle expansion and emit expand/collapse, while text/image hits select or
activate according to modifier/double-click state. Activation sends
`0x1AF5A13B` and can toggle a branch as Enter does. Hit rectangles come from
painting, including the horizontal scroll transform. Right-down, drag start,
drag-over, cancellation and exact modifier/double-click behavior remain WC3;
do not replace them with an assumed host-treeview policy. Tree notifications
are synchronous W slot 144 sends to the notification target, with handles
converted by W slot 63; selection-changed has zero payload rather than a
selected-handle list. Tree and ListBox notifications are distinct protocols.

### 6.5 Drawing and failure boundary *[compat]*

The owner draws through font, buffer, image-list and scrollbar interfaces.
RecalculateAll rebuilds flattened layout from the hierarchy, clears prior
flattened-visible flags, measures content and updates buffers/bar requirements
and viewport. It is demanded by dirty painting/visibility queries; the engine
need not expose its native containers. DrawNode recursively draws hierarchy;
DrawItem chooses closed/expanded image indices with selected overrides, draws
an image when enabled/available, measures text through that node's font, draws
selection background and emits text through font slot `+0xCC`. A missing image
index −1 skips that image; it does not suppress text. The paint pass records
text/icon/expander rectangles used by subsequent input. Selected and unselected
font/background colors are separate. See `009D0CB1`, `009D17A8`, `009D37C6`.

GZPaint first recalculates dirty layout, optionally clears/fills when flag
`0x20` is set, repairs dirty selection cache, renders the tree into its working
buffer and blits the viewport to the window drawing buffer. Buffer access
flags are acquired/tested and released around guarded owner operations;
failed access skips that guarded operation. IO consists of graphics/font/
resource-service calls; no directory enumeration or clipboard IO is proven
for this owner. Exact line/expander pixels, selected gradient arithmetic,
image-list load failure and drag-helper drawing remain WC3. In particular,
this section is an interface drawing contract, not pixel-exact conformance.

#### Layout and hit geometry (WC6)

Use unsigned 32-bit stored dimensions and wraparound arithmetic. Let `h` be
row height (`+0x128`), `b` expander size (`+0x130`), `g` spacing (`+0x134`),
`i` reserved image width (`+0x12C`), and `mx,my` the margins (`+0x138/+0x13C`).
Flag `0x4` enables images; `0x8` enables connector/expander drawing. Neither
changes membership in the flattened sequence. Layout measurement uses
`c=(i-b)/2` rounded down only when images are enabled and `i>b`, otherwise
zero. Ordinary depth `d` contributes `(b+g+c)*(d+1)` before the reserved image
and text space. Root contribution is zero. Maximum content width is the
maximum contribution plus `g+i+3+font-measured text width` (`009D3017`–
`009D30D9`). Image/text space is reserved even when image drawing is disabled.

Painting deliberately differs at the numeric edge: its `c` is the logical
right shift of the wrapping difference `i-b` whenever images are enabled,
without the measurement pass's `i>b` guard (`009D17DA`–`009D17E8`). For an
ordinary node its expander origin is `x=(b+g+c)*d+c`,
`y=(row-first)*h`; the root uses `(0,0)`. Let `a=x+b+g` (root: `a=0`).
DrawItem's text rectangle is `[a+g+i,y,a+g+i+3+textWidth,y+h]`; text is
emitted three pixels right and one pixel down from that rectangle's top-left.
The image, if enabled with a usable index, is centered in the `i` by `h`
reserved area at `(a,y)`, with each centering difference shifted logically
right by one. Selected image indices override the ordinary index only when
not −1; expanded images apply only to expanded nodes having children.
There is no image-index bounds check (`009D0CB1`–`009D0DFD`).

Painting resets the visited node's three rectangle-valid bytes. Text geometry
becomes valid when DrawItem targets the working buffer; image geometry becomes
valid only after successful `0x8010` buffer access/blit. A branch's expander
rectangle is `[x,y+((h-b) arithmetic-shift-right 1),x+b,
y+((h-b) arithmetic-shift-right 1)+b]`, recorded only after successful
`0x8008` access. A leaf has connector lines but no expander hit rectangle.
These rectangles remain in working-buffer coordinates and are not clipped
to the viewport (`009D17C0`–`009D17C8`, `009D1A00` vicinity,
`009D0DAC`–`009D0DB9`, `009D0F64` vicinity).

The drawable viewport starts at window left/top plus `mx,my`. Available
width/height subtract twice the respective margin and the opposite-axis bar's
thickness when present, then are capped by working-buffer dimensions.
Whole rows are unsigned `viewportHeight/h`; the paint/page count is
`ceil(viewportHeight/h)`, including a partial row. The quotient is computed
with x87 unsigned-integer conversion and division, passed as a double to
ceil, then converted to integer; no intervening float store is present
(`009D3713`–`009D3772`; ceil entry `009F0A20`). Thus `h=0` is outside the
safe domain. Last paint row is `first+paintCount−1`; it is not clamped to
total here. The unsigned correction comparison at `009D3782`–`009D37B7`
compares `last-first` against `paintCount−1`, so ordinary arithmetic does
not trigger its apparent end-of-content adjustment.

Cursor-to-row lookup first applies the inclusive rectangle test to
`(mouseX-mx,mouseY-my)` against `[0,0,viewportWidth,viewportHeight]`, then
unsigned-divides adjusted Y by `h` and accepts offsets up to `last-first`
inclusive (`009D0626`–`009D0678`). It does not independently check the
flattened-sequence length. Exact bottom-edge hits can therefore reach an
unbacked row. Mouse handlers test the recorded subrectangles inclusively
after transforming X by horizontal scroll minus `mx`, and Y by `−my`.
Stale/unpainted geometry and out-of-range edge lookup are native quirks;
no fabricated valid hit is specified for them.

## 7. Family conformance vectors

The linked owner vectors are part of this specification. These additional
vectors are implementer-facing scenarios, not executed game comparisons.
Instrument notification, font/buffer and scrollbar interfaces where indicated.
Use ordinary valid initialized services unless a failure is the test input.

| ID | Setup and action | Required observation |
|---|---|---|
| WC-T13 | Root R has children A,B; A has child C and is expanded; clear flag 0x2 and collapse R | Flattened sequence A,C,B; root not flattened-visible; A/B depth zero |
| WC-T14 | Same hierarchy, set flag 0x2 | Collapsed R alone; expand R to obtain R,A,C,B; A/B still depth zero |
| WC-T15 | Select A then B with accumulation enabled; navigate Up | Replacement selection is the row before B, using the last selected entry |
| WC-T16 | Viewport height 33, row height 16, first row 0, enough content | Whole-row count 2, paint/page count 3, last paint row 2 |
| WC-T17 | Images disabled, b=9,g=3,i=12,h=16,d=1,row=2,first=1,textWidth=20 | Expander origin (12,16); DrawItem origin (24,16); text rectangle [39,16,62,32], text origin (42,17); image space remains reserved |
| WC-T18 | Successful paint of a branch, then fail its next expander/image access | Visited node's expander/image validity is cleared; failed guarded operations do not restore it; text rectangle still records when DrawItem targets working buffer |
| WC-L19 | Entries A,B,C; select index 1, then remove index 1 | Selection stays 1 and now designates C; no selection message from the index adjustment |
| WC-L20 | Entries A,B,C; select index 2, then remove index 2 | Selection becomes 1 (B); ordinary mode emits selection-changed through Select(notify=true) |
| WC-L21 | Twenty entries, wholeRows=4, origin=0; NextPage | Selection 7, final origin 4, no selection notification |
| WC-L22 | Ten entries, wholeRows=4, origin=5; NextPage | Selection 9, final origin 6; repeated NextPage at origin 6 is a successful no-op |
| WC-L23 | Drop list: five entries, rowHeight=10, yGutter=1, wholeRows=2; available height 100; height-size | Height 52, wholeRows 5, origin 0, no bar |
| WC-L24 | Same inputs, available height 37 | Initial request 37; wholeRows 3; whole-line resize requests height 32; bar added |
| WC-L25 | Empty drop list, xGutter=3; width-size, then height-size | Width request 7; height-size returns false without shrinking height; both policies are now enabled |
| WC-L26 | Paint `abc`, `##abc`, `###abc`, `#112233abc`, keeping stored strings | Ordinary `abc`; literal `#abc`; literal `##abc`; prefix-stripped `abc`; all stored strings unchanged |
| WC-L01 | Query ListBox own IID through W, then through L; release results | Both designate L=W+0xD8, with matching retention; null own-IID output fails without AddRef |
| WC-L02 | Three entries, selection 1; select index 99 with notify=true twice in ordinary mode | Selection 2; two selection messages, each type 3 / 0x08710E0F, payload row 2 |
| WC-L03 | Same test in drop mode | Selection 2 and invalidation, no selection message from the public select operation |
| WC-L04 | IDs [7,7,9], selection −1; select ID 7 then ID 99 | First selects row 0; second returns false and preserves row 0 |
| WC-L05 | Insert before selected row 1; then replace text at index −2 | Selection follows same entry to row 2; replacement affects last entry |
| WC-L06 | Empty list, then select −1 and query selected ID | Selection remains −1; ID result zero; no fabricated entry |
| WC-L07 | Height 42, y gutter 1, font metric 10, minimum 0, no background | Row height 10, four whole rows; whole-line resize preserves 42 |
| WC-L08 | Same list, first row 2, y=10 versus y=11, arbitrary x | Hit rows 2 and 3 respectively; x has no effect |
| WC-L09 | Row height zero | Metrics report zero visible rows; whole-line resize returns false; do not call division-based hit test |
| WC-L10 | Write valid font color 1 first with a different word, then the same word | Different write stores without invalidate; equal write invalidates; index 3 fails |
| WC-L11 | Getter with sentinel color and prefilled output word | False; output word unchanged |
| WC-L12 | Replace font/background/border by same pointer, then null | Incoming AddRef precedes outgoing Release; null clears/relinquishes reference |
| WC-L13 | Enable flat outline then 3D outline | Flat byte becomes false; 3D byte true; invalidates on both setters |
| WC-L14 | Emitter with delivery byte 0 then nonzero | Send then Post respectively; IDs/payload identical, emitters return true |
| WC-L15 | Ordinary key-up with recording target; drop key-up | Ordinary forwards message type 6 with key/modifier; drop consumes without forwarding |
| WC-T01 | Construct, then Init twice with available services | Primary/window views differ by 4; one internal default/root initialization; Init returns true each time |
| WC-T02 | Query absent node; set expanded on absent handle | Lookup null; setter false, no invalidation/notification |
| WC-T03 | Set a node expanded true twice, then false | One expand and one collapse message only; same-value call succeeds silently |
| WC-T04 | Select A, select B with force replacement; deselect absent C | Only B selected; changed calls notify 0x1AF5A13A with zero payload; C returns false |
| WC-T05 | Flag 0x10 enabled, select A then B with replacement=false; clear twice | A/B retained in selection order; first clear notifies once; second returns true without notification |
| WC-T06 | Child under collapsed ancestor; MakeVisible(child) | Ancestor expands and notifies before refreshed visibility; selection unchanged |
| WC-T07 | Vertical bar first=2, visible=4, total=10; wheel delta −1 then −1000 | Each requests first+1; magnitude does not scale movement |
| WC-T08 | Vertical bar first=2; wheel delta 0 | Requests first=1; consumed |
| WC-T09 | No vertical bar; wheel either sign | Consumed, no bar call |
| WC-T10 | Selected branch A and leaf B; Enter | Activation in selection order for both; A toggles expansion, B does not |
| WC-T11 | Deliver tree bar IDs 0x1756BAC6/7, then unrelated message | Matching bar caches update; all three messages still reach base dispatch |
| WC-T12 | Node image index −1 with nonempty text and valid font | No image draw for that index; font text draw still occurs |
| WC-F01 | Call native W extension slot 158 versus L slot 10 on ListBox | W slot removes storage; L slot selects; the operations must not be aliased by export names |

## 8. Coverage and unresolved contracts

The linked Grid, TextEdit and FileBrowser inventories remain authoritative;
their pre-existing parked questions are not duplicated here. ListBox's 66
list slots plus window extensions and TreeView's 60 primary slots plus window
overrides are inventoried above. **An inventory is not a complete owner spec.**
WC1–WC3 identify exact unproved adapters, entry ordering/destruction, rendering
and drag boundaries. No engine implementation, binary hook, ABI-table update,
in-game run or shadow equality is claimed. These contracts are sufficient for
separately testable state/message portions; the unresolved areas must remain
explicit implementation questions rather than guessed compatibility rules.
