# Native text-edit widget

Scope: only the native `cGZWinTextEdit` owner requested as
`cGZWinTextEdit19999B`; the export's class name has no `19999B` suffix.
This is a partial behavior contract, not whole-widget-family coverage.
Evidence is the Windows 1.1.641 export and original owner pseudocode in
`decomp/cGZWinTextEdit.c`, checked against native instructions where stated.
No implementation, constructed runtime output, or live clipboard observation
is evidence for this document. Base window behavior belongs to
[widget-toolkit.md](widget-toolkit.md).

## 1. Identity, coordinates and evidence boundaries

The primary window vtable is `0x00ADFBD0`; the text interface vtable is
`0x00ADFA68`. QueryInterface recognizes IID `0x231A1C57`, returns the
subobject at window base **+0xD8**, AddRefs through the primary interface,
and returns true; other IIDs go to cGZWin (`0x009BC5E1`). The exported
`__ZThn228` spelling is not a reliable displacement: native bodies subtract
**0xD8**, not decimal 228 (`0x009C053E`, `0x009BD9DE`).

In this document **W** means window base and **T = W+0xD8** means text
interface. The decompiler often prints the same class type for both. Never
combine its field offsets without accounting for the entry point.

| Native state | Location relative to W | Established meaning |
|---|---|---|
| Text string | +0xDC | Byte string; begin/end pointers at +0xE0/+0xE4 |
| Line starts | +0x108 vector | Offsets into that byte string |
| Insertion line, column, index | +0x114/+0x118/+0x11C | Separate cached coordinates and byte index |
| Selection endpoints | +0x120/+0x124 | Ordered anchor and active end; may be reversed |
| Editor flags | +0x128 | Separate from cGZWin flags +0xC8 |
| Code-page selector | +0x12C | Captured from GetCurrentFrameworkCodePage |
| Horizontal/vertical gutters | +0x158/+0x15C | Initially 5 and 0 |
| First visible line | +0x16C | Distinct from insertion line |
| Maximum text bytes | +0x1D0 = T+0xF8 | Initially 0xFFFFFFFF |
| Caret visibility | +0x228 byte | Initially false |
| Caret and scroll callback timers | +0x22C/+0x254 | Separate timer objects |
| Clipboard service reference | +0x28C | Initially null; no OS clipboard call implied by construction |

Default initialization (`0x009BFF6E`) sets insertion and both selection
ends to zero, editor flags to zero before applying the constructor's flags
through text slot 77, mask character to byte `0x2A` (`*`), the internal
string at +0x144 from one byte `0x2D`, first visible line to zero, caret
period +0x224 to 1000, and the separate +0x220 interval to 500.
The font reference and background/scrollbar references start null.
It sets white base fill and clears six explicit font-color overrides.
These are constructor defaults, not a claim that every subsequent Init
repeats the constructor.

## 2. Byte text and whole-text replacement

Indices and selection endpoints are **byte offsets**, not UTF-16 units or
grapheme indices. Character helpers sometimes move by more than one byte;
that does not change storage or endpoint units.

`SetText` at `0x009C04FE` takes T, source bytes, a byte count and a notify
boolean. A null source is replaced by an empty string. Count zero means
`strlen(source)`, not an explicit zero-length slice (`0x009C0508–0523`).
It erases the existing text before filtering the candidate. It removes
disallowed characters and calls the composition fixer against the now-empty
destination. A nonzero combined correction count calls primary slot
`+0x26C` with reason 0. That hook's downstream effect is not established here.

If filtered length exceeds the maximum, ordinary code pages keep exactly
the maximum byte count. For selectors **8 or 100**, the truncation loop
advances by GetCharLength until the accumulated count is **at least** the
maximum. It can exceed that maximum by ending a multibyte character
(`0x009C0585–05C5`). This loop uses the original source pointer, whereas
the final assignment uses the filtered string; preserve that distinction.
Maximum zero keeps zero bytes. Truncation invokes the same hook with reason 1.

For a nonempty filtered candidate it assigns the retained prefix, rebuilds
line lengths, updates scrollbar values, zeroes insertion line/column/index
and both selection ends, checks automatic scrollbars, and invalidates.
The empty-candidate branch uses RecalculateAll and invalidates instead.
Both branches clear **both undo and redo histories**, even when replacing
the text by the same bytes. With notify true it sends message
`0x9B1552D8` with the text-interface pointer and two zero payload words to
the window's notification target. It returns true (`0x009C062C–068C`).

The maximum setter `0x00994E71` only stores a 32-bit value at T+0xF8.
Lowering it does not immediately truncate existing text, clear history,
reflow, or notify. No safer post-limit insertion behavior is inferred when
existing length already exceeds a newly lowered limit.

## 3. Selection and insert/overwrite state

`SetSelection(anchor, end)` (`0x009BD997`) clamps each endpoint separately
to the current byte length using **unsigned** comparisons. Thus
`0xFFFFFFFF` means end-of-text in this operation. It stores the two
endpoints without sorting, moves insertion index to the clamped second
endpoint, recomputes insertion line/column and invalidates. It returns true.
It does not itself put the insertion point in view, record undo, or notify.
Evidence: native stores and calls `0x009BD99A–09EF`.

`SetInsertMode(true)` clears editor bit `0x4000`; false sets it
(`0x009BCAFE`). The bit therefore means overwrite. The setter returns true
and does not edit the text. This differs from treating a set bit as insert.

The word-separator predicate `0x009BC5CC` recognizes exactly byte space
`0x20` and LF `0x0A`; punctuation and tab are not separators in that
predicate. Higher-level word movement and selection require their own
boundary rules, rather than a platform Unicode word-break API.

## 4. Character input and high-level replacement

`GZOnCharacter` (`0x009C0996`) first queries manager key states 0x11 and
0x12. Editing proceeds only if editor bit 1 is set. CR is converted to LF;
bytes below 0x20 other than LF return true immediately. Thus character
input does not insert tab. Programmatic SetText is a separate path.

For code-page selector 8 or greater than 99, bytes are accumulated in the
pending input string. A leading-byte helper may cause an immediate true
return after saving the first byte; insufficient accumulated length also
returns true. Other selectors use a one-byte pending string. Once ready,
input calls InsertTextHighLevel only when the window is not capturing;
the normal exit clears pending bytes and returns false. A true return
therefore does not prove text changed. Exact foreign encoding-helper
semantics remain an explicit boundary.

`InsertTextHighLevel` (`0x009C07E1`) handles replacement, independently of
the raw insert primitive. If editor bit `0x200` is set and the candidate's
first byte is LF, it collapses selection to insertion and invalidates,
without calling insertion. Otherwise it snapshots the five insertion/
selection words. A nonempty selection is removed in increasing endpoint
order, insertion moves to the lower endpoint, and the removed bytes are
saved. With overwrite bit `0x4000`, no saved deletion, and insertion before
end-of-text, it removes one character at insertion before inserting.

It calls the raw insert slot with candidate bytes and current byte index.
Zero bytes inserted invokes hook +0x26C reason 1. A nonzero result records
an insertion undo action including any removed text. It resets preferred
horizontal position to 0xFFFFFFFF, collapses selection to insertion, puts
insertion in view, and conditionally sends `0x9B1552D8` when editor bit
`0x800` is set. It returns true even for rejected LF or zero insertion.
Failures after the earlier deletion are not shown to roll that deletion back.

## 5. Caret and timer boundary

ToggleCaretVisibility (`0x009BC60E`) changes the visibility byte and
invalidates only when this window currently has focus. MakeCaretVisible
(`0x009BC635`) likewise requires focus and does nothing if already visible.
StartCaretTimer (`0x009BC65C`) starts only if stopped and stops only if
running; returns false when already in the requested running state.
Start passes the unsigned caret-period value shifted right by one and T as
callback data. Zero/odd periods are not rounded up. Timer scheduling and
delivery belong to the external callback-timer primitive.

## 6. Independent distinguishing vectors

These are specification examples derived from bodies, not game-run results.

| Setup and action | Required observation |
|---|---|
| Text `abcde`; SetSelection(4,1) | Endpoints remain (4,1); insertion is 1, not 4 |
| Same text; SetSelection(0xFFFFFFFF,2) | Endpoints (5,2); insertion 2; first argument does not clamp to zero |
| Existing `abcde`; lower maximum to 2 | Text remains `abcde` immediately after setter |
| Ordinary code page; SetText(`abc`,0,false), maximum 2 | Stores `ab`, reason-1 hook; history cleared |
| Encoding helper reports a two-byte character; maximum 1; SetText of that character | Selectors 8/100 can retain 2 bytes; ordinary path retains 1 |
| SetInsertMode(true), then false | Overwrite bit becomes clear, then set; no text mutation |
| Focus elsewhere; ToggleCaretVisibility | No visibility transition and no invalidation by this helper |
| Caret period 1001; start stopped timer twice | First request passes 500 and T; second returns false without another start |
| Editable character input byte 0x09 | Returns true before high-level insertion |

## 7. Whole-owner checklist and current limits

The name search contains 146 matching functions including adjusted thunks,
STL helpers, a factory and a nested action constructor. Direct, non-nested
`cGZWinTextEdit::` labels account for **134 bodies / 19,415 bytes**.
That is a label inventory, not yet reconciliation to the task's 19,999-byte
owner attribution. Folded aliases and unnamed native helpers must be
counted separately; no whole-owner credit is claimed.

| Area | Current evidence disposition |
|---|---|
| Identity, state, whole-text replacement, selection, overwrite, character input, high-level insertion, caret timer | Rules above; external helper contracts remain bounded |
| Raw insertion/deletion, undo/redo, key navigation/clipboard, mouse, focus | Native bodies present; detailed contracts pending |
| Line starts, metrics, painting, hit mapping, scrollbars | Native bodies present; detailed contracts pending |
| Init/shutdown/destructors, images/font ownership, flags/messages, thin methods | Native bodies present; complete checklist pending |

Open contracts live in [questions/text-edit.md](questions/text-edit.md).
