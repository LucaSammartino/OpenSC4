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

Research in progress.

## 6. TreeView

Research in progress.

## 7. Family conformance vectors

The linked owner vectors are part of this specification. Additional ListBox
and TreeView cases follow with their contracts.
