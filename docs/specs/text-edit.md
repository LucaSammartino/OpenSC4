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

### Runtime defaults (TE10)

Before the constructor's final SetFlags call, the following are explicit
stores, not values to be supplied by layout. All offsets below use **W**.

| State | Offset | Initial value |
|---|---|---|
| Caret color | +0x1C0 | `0xFF000000` |
| Selection highlight fill | +0x1C4 | `0xFF000000` |
| Focus outline color | +0x1C8 | `0xFF000000` |
| Visible width, height, row count | +0x160/+0x164/+0x168 | All zero |
| Horizontal byte offset, pixel offset | +0x174/+0x178 | Both zero |
| Line height | +0x1CC | **1**, before RecalculateAll replaces it |
| Image edge option | +0x180 byte | **true** |
| Preferred X, maximum undo levels | +0x170/+0x184 | `0xFFFFFFFF`, `0x4000` |
| Sound IDs 0 through 3 | +0x27C..+0x288 | **Not initialized by construction** |

Evidence: `0x009BFFB8–0x009C002F`, `0x009C0081–0x009C00A2`;
the line-height source is the integer 1 established at `0x009BFF85–87`.
The sound setter uses **T+0x1A4+4*index**, hence W+0x27C, not W+0x1A4
(`0x009BCC57–71`). Neither constructor `0x009C01AF` nor member initialization
`0x009BFF6E` assigns those four words; SetFlags exits its runtime side effects
before initialization (`0x009BFD3A–4A`). Consequently there is no native zero
sound default: a portable model must represent these as unspecified until
assigned, or explicitly label any deterministic substitute. Do not confuse
them with the zeroed image/scrollbar fields at W+0x1D4..+0x1E0.

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
`+0x26C` with reason 0. Its sound-adapter contract is specified in section 10 (TE2).

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
therefore does not prove text changed. The exact helper byte tables and
malformed-input outcomes are specified in [text-edit-encoding.md](text-edit-encoding.md)
(TE1/TE5), including composed lengths and UTF16 conversion. The system
selector service and Windows code-page tables remain environment inputs.

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

## 6. Raw mutations and history

Raw InsertText (`0x009BF5B7`, T) clamps insertion beyond text length to end,
copies and filters the input, and fixes composition using the destination
and insertion offset. Corrections invoke reason 0. It limits the retained
byte count against existing length plus insertion length, using character
stepping for selectors 8/100 and a byte remainder otherwise; truncation
invokes reason 1. Empty filtered input returns zero without insertion.
It inserts the retained bytes and advances insertion by the glyph-count
helper, then conditionally repairs line starts and scroll state when the
base window is initialized. Return value is the retained byte count.
This raw entry does not itself create an undo action or send text-change
notification. The behavior when a lowered maximum is already below the
existing text length depends on the selector; see the call domains below.

Raw RemoveText (`0x009BF8A8`, T) rejects an unsigned start at or beyond
length by returning zero. Otherwise it caps count to the remaining bytes,
optionally assigns removed bytes to the caller's string, erases them,
moves insertion to the removal start, recomputes line state, adjusts view
and scrollbars and invalidates. It returns bytes removed. It does not
clear or collapse selection on its own; higher-level callers do that.
Raw RemoveChar (`0x009BF776`) selects ordinary-character length for true
mode, composed-character length for false, optionally copies removed
bytes, erases and repairs view/lines. An out-of-range index becomes
length-minus-one, including unsigned underflow for empty text: no benign
empty-text result is promised by this body. Normal guarded key callers
avoid that case. Its normal return is true, not a removed-byte count.

### Raw-call domains and failure paths (TE3)

Use valid byte buffers and a nonoverflowing existing-plus-input length.
For ordinary-selector insertion the supported domain includes existing
length L <= maximum M. If filtered input exceeds the remaining space,
the retained count is M-L, not a saturating subtraction. With L > M,
that subtraction wraps at 32 bits (`0x009BF649-64E`): it must not be
implemented as an evidenced clean rejection or clamped no-op. For L-M >= 2,
the downstream string insertion's unsigned length guard takes its length
failure helper before source copying (`0x0090E8D3-8DE`, `0x0079C6E0`).
For L-M = 1 that guard does not take the same path; the wrapped source
range is invalid and no successful result is promised. No owner recovery
or rollback handler surrounds these calls.

Selectors 8/100 have a different lowered-limit outcome: if L >= M and
filtered input is nonempty, their stepping loop starts with zero retained
bytes and skips all stepping (`0x009BF650-687`). Reason 1 still runs and
the owner continues through zero-byte insertion, zero glyph movement and
its normal initialized-widget view updates, returning zero. An already
empty filtered input returns earlier without that truncation hook. The
input filtering/composition hooks precede all of these limit decisions.

RemoveChar requires nonempty text and a character-valid byte position.
An excessive unsigned index on nonempty text is replaced by L-1, which
need not itself be the start of a multibyte character. On empty text the
owner manufactures index 0xFFFFFFFF, remaining count 1 and address
textBegin-1 (`0x009BF789-7A6`). With ordinary-character mode, selector 0
or 16, and no output string, GetCharLength returns 1 without dereferencing
that address; string erase then invokes its out-of-range failure path
(`0x004425CE-5D9`, `0x009DD0A0`). Other selector/composed/output-copy paths
can use the invalid address earlier. Thus empty removal has neither a
portable no-op nor a universal single exception contract. These are
native failure boundaries, not instructions to dereference invalid memory
in a clean-room implementation.

Undo (`0x009C027E`) and Redo (`0x009C0396`) return false if editor bit
`0x100` is clear or their respective stack is empty. Otherwise they build
an inverse action on the opposite stack, directly erase/reinsert the
recorded bytes, restore the recorded **five** insertion/selection words,
pop a 60-byte action, reflow, put insertion in view, invalidate and return
true. This replay uses stored bytes, rather than re-running the high-level
input filter. No text-change message is sent directly by these two bodies.

Insertion/deletion record helpers (`0x009C068D`, `0x009C075F`) require
`0x100`, save the five-word pre-edit state, push an action, and clear redo.
An insertion action can also contain the replaced bytes; thus replacing a
selection can be undone together. There is no evidenced typing coalescing.
SetMaxUndoLevelCount (`0x00994ED7`) merely stores T+0xAC (W+0x184);
it does not immediately prune either stack. The following limit rule is
specific to **insertion record creation**, not a general history capacity.

### Insertion history eviction and action lifetime (TE4)

For a valid history vector containing n complete 60-byte actions, compare
n with the configured unsigned limit. If n is at least the limit, erase
the earliest **floor(n/4)** actions, then append the new insertion action.
If n is below the limit, append without eviction. The count calculation
uses signed division of the vector byte difference by 60, then an unsigned
comparison against the stored limit; the quarter uses a logical shift by
2 (`0x009C06AA-06D5`). This rule presupposes a valid representable vector,
not fabricated pointers or overflowed allocation sizes.

The callee labelled vector `clear` (`0x009C04CB`) is actually a range erase:
it assigns the surviving suffix forward into the erased range, destroys
the vacated tail actions and updates the end pointer. The chain
`0x009C04DF -> 0x009C04AE -> 0x009C017D -> 0x009BFF03` establishes
forward 60-byte assignment; `0x009C04EA -> 0x009BF518 -> 0x009BEA0D`
destroys each trailing action's owned string, without freeing each action
slot. Scalar action fields and the five saved state words are copied;
the string is assigned through its own operator. No bytewise relocation
or loss of retained actions follows from the misleading `clear` label.

Deletion record creation (`0x009C075F-07DE`) has no count-limit branch:
with history enabled it appends one action and erases the entire redo range.
Insertion creation also erases redo after appending. Both return true on
their normal enabled path and false with bit 0x100 clear; no owner-level
allocation-failure recovery is evidenced. Record creation does not merge
adjacent typing. The maximum setter therefore does **not** impose a hard
stack bound, and zero does **not** disable recording. Starting empty with
limit zero, insertion-only counts are 1,2,3,4,4,...; deletion-only recording
can continue growing. A preexisting larger stack is reduced by a quarter
per insertion, rather than instantly resized to the configured limit.

## 7. Key routing and clipboard boundary

GZOnKeyDown (`0x009C0A95`, 2938 bytes) interprets modifier bits 0/1/2 as
the shift/control/alt paths respectively (native extraction at
`0x009C0A9B–0AB4`). Unknown keys and most suppressed paths return true.
There is no Ctrl+A branch in this body. Character entry is separate.

| Key path | Established behavior |
|---|---|
| Ctrl+C / Ctrl+X | Require control, no shift/alt, editor bit 0x1000, bit 0x2000 clear, and no capture. Copy sorted selection bytes through clipboard manager +0x50 with arguments (string,1,0), when service and selection exist. Return of that call is ignored. Cut then dispatches Delete with modifiers zero, even if clipboard service is null or selection is empty. |
| Ctrl+V | Require control, no shift/alt, no capture and 0x1000. Unlike copy/cut there is no 0x2000 check here. With a service, call +0x48(outputString,0); only a true result calls InsertTextHighLevel. There is no additional editable-bit test in this branch. |
| Ctrl+Z / Ctrl+Y | Require control and no alt; shift does not exclude these paths. Dispatch undo/redo when the corresponding stack is nonempty; otherwise reason 3. Replay itself still tests 0x100. |
| Insert | With editable bit 1 and editor bit 0x80, XOR overwrite bit 0x4000; otherwise consume without change. |
| Delete | Requires editable bit 1. Delete sorted selection, or composed character at insertion if before end; record deletion on success. Failure invokes reason 1. Collapse selection afterward; send text-change only with bit 0x800. |
| Backspace | Requires editable bit 1. Selected range uses the same deletion path; otherwise locate previous character length, remove in ordinary-character mode and record deletion. At start, reason 1 and no removal. |
| Left/Right | Require editable bit 1. Normal movement calls composed-character stepping; control calls word stepping. Alt selects line-edge behavior before the Right step, but Left's alt path goes directly to column zero. Shift retains anchor, otherwise collapse selection to new insertion; put in view, show caret, invalidate and reset preferred X. |
| Home/End | Require editable bit 1. No control chooses current line edge; control chooses document edge. Apply the same shift-selection/view/caret handling. End uses the LF correction helper before final selection handling. |
| Up/Down | In editable multiline mode move one line, preserve a preferred pixel X and use font hit mapping. Without shift collapse selection; with shift keep anchor. When editor is noneditable or single-line, bit 0x800 instead sends 0x9B1552D5 with (T,key,0). Boundary-without-selection invokes reason 2. |
| Page Up/Down | Editable multiline only; move by visible-line count, clamp to first/last line, map the pixel X on destination and apply shift-selection rules. Boundary-without-selection invokes reason 2. Exact preferred-X reuse differs from Up/Down; see TE5. |
| Escape | With editable bit 1, collapse a nonempty selection and invalidate, then send message 5 with (0x1B,originalModifiers,0) and return that send's result. Without editable bit 1 it jumps to the copy gates: Ctrl+Escape can copy selection when the same clipboard gates pass. |
| Return | With editor bit 0x400, send 0x9B1552D5 with (T,0x0D,0), regardless of editable bit 1; otherwise consume. No LF is inserted by this key-down path. |

Clipboard evidence: native gates/copy call/cut redispatch at
`0x009C11E3–12A0`; paste branch `0x009C1504` onward. Escape/Return and
arrow forwarding differ natively at `0x009C0BA3–0C02`, `0x009C14D9–14F9`.
No clipboard manager
implementation, OS format, newline conversion, persistence, error UI or
ownership beyond the local string lifetime is inferred. No real user
clipboard was accessed for this specification. The clipboard service may
be absent; paste then does nothing, while a permitted cut still dispatches
Delete. This is a distinguishing failure behavior.

The LF correction helper (`0x009BD0AF`) decrements insertion index and
column by one only when both are positive and the byte immediately before
insertion is LF. It does not change insertion line or selection itself.

## 8. Layout, font primitives and view

New native byte/word stepping, line/index conversion, horizontal visible
spans, scroll offsets, scrollbar geometry/range arithmetic, page scrolling
and drag-timer routing are specified in [text-edit-layout.md](text-edit-layout.md).
Composition and conversion results are specified in the encoding supplement;
the native owner navigation/view paths have instruction proofs.

CalculateVisibleWidth/Height (`0x009BCBC5`, `0x009BCC0E`) subtract twice
the corresponding gutter plus the orthogonal scrollbar thickness from
window width/height, returning zero when the unsigned dimension is smaller
than that sum. GetTextArea (`0x009BD19B`) returns local rectangle
(gutterX,gutterY,gutterX+cachedVisibleWidth,gutterY+cachedVisibleHeight).
It does not add absolute window position.

RecalculateAll (`0x009BF98B`) does nothing before base initialization.
Afterward line height is font +0x8C, or 16 with no font. Visible rows use
**unsigned integer division** of visible height by line height, promoted
from zero to one only when visible height is positive as a signed value
(`0x009BF99B–F9ED`). A font returning zero height is not guarded before DIV.
Then it recalculates width/line starts, scrollbar values and insertion
validity; first visible line becomes zero for no lines or at most last
line otherwise. Automatic scrollbar checking is guarded by a shared
recursion counter at `0x00BAC098`, not an unlimited recursive reflow.

**Exact automatic-check guard (TE11):** interpret that shared 32-bit word as
signed and perform the automatic check only when it is **less than 1**.
Increment it immediately before TestForPossibleScrollBarAdditionOrRemoval
(`0x009BE7AF`) and decrement it immediately after that call returns normally
(`0x009BFA56–6C`). Thus an ordinary outer call changes 0 to 1 to 0; reentrant
RecalculateAll on this or another editor still performs layout and scrollbar
value updates, but skips its own automatic addition/removal check at 1.
Direct calls of the automatic-check helper do not acquire this guard. This
is not an exception-safe reset: no unwind handler restores the word if the
helper fails to return normally. A negative word also passes the literal
signed comparison; it is not an equality-to-zero test.

CalculateAllLineLengths (`0x009BF3E0`) requires a font. Empty text clears
line starts and zeroes insertion coordinates. Nonempty text ensures an
initial zero line start. With width zero or editor bit 0x200 it consumes
the entire remaining text as a line. Otherwise, without bit 2, it scans
through the next LF (including LF in that span); with bit 2 it asks font
+0xA4 for a span fitting (bytes,remaining,width,0,2). A trailing LF produces
an additional line start at end-of-text; a final non-LF span does not.
No font fallback wrapping or independent word-wrap algorithm is supplied
by this owner. Progress/nonzero spans from the font are a foreign contract.

The layout and draw calls require these external font contracts:

| Font vtable byte offset | Inputs and result consumed by this owner |
|---|---|
| +0x8C | Pixel line height |
| +0xA4 | Source bytes, remaining byte count, available width, 0, 2; returns bytes fitting a wrapped span |
| +0xC0 | Source bytes and byte count; returns measured pixel advance |
| +0xCC | Buffer, x/y, bytes/count, foreground/background values, clip rectangle, 0; draws the specified span |

These offsets define call-site requirements, not a claim to have specified
the foreign font owner or its font loading, shaping, rasterization or
rounding. Window draw origin/buffer/color conversions and drawing context
remain the cGZWin contracts in widget-toolkit.md, with unresolved behavior
there remaining unresolved here.

GetColumnFromPosition (`0x009BE179`) uses x + horizontalScroll - gutterX.
Empty lines or adjusted x <= 0 produce column zero. Otherwise it measures
successive character prefixes and returns the prefix's **starting** byte
column when x is <= that prefix's advance; it does not choose the nearest
half-glyph edge. GetPositionFromLineAndColumn (`0x009BE239`) clamps line
to last and column only against total remaining text, measures that prefix,
adds gutterX and subtracts horizontal scroll. Y is
(line-firstVisible)*lineHeight+gutterY. Empty lines use gutters before the
same horizontal subtraction. Out-of-contract coordinate arithmetic is not
silently promoted to a Unicode or platform hit-test API.

PutInsertionPositionInView (`0x009BE302`) moves firstVisible to insertion
line when above, or insertionLine-visibleRows+1 when below the visible
range. In unwrapped horizontal-pixel mode, it adds a negative caret x to
scroll offset, or adds caretX-visibleWidth when x >= visibleWidth.
Single-line bit 0x200 uses a separate character-offset scroll path and a
three-quarter-width target; the exact strict comparisons and update gate
are specified in the layout companion.

Automatic scrollbar check (`0x009BE7AF`) adds vertical only with bit 8,
line count greater than visible rows and nonzero visible width/height;
otherwise removes an existing vertical bar. With bit 4 and bit 2 clear,
horizontal is added only when the font-measured longest line exceeds
visible width and both dimensions are nonzero; otherwise it is removed.
AddScrollBar (`0x009BE477`) creates through WCM, attaches the bar's window
as child, and reflows/resizes/sets values. Creation failure clears that
orientation's enable bit but returns true after the update path. Null WCM
is tolerated in this creation path. Exact range/size/resource rules are TE5.

## 9. Painting

GZPaint (`0x009BEA28`, 1850 bytes) returns true. It derives visible line
interval from firstVisible and visibleRows, capped at line-start count,
resolves colors through base window conversion, and processes background
before text, caret and final focus outline.

With editor bit 0x2000 it builds a separate mask string of one mask byte
per GetGlyphCountFromString result; selection endpoints are translated to
glyph counts for drawing. The underlying text is not overwritten. This
does not establish correct mapping for every multibyte line arrangement;
the exact mixed-coordinate behavior is specified below.

With background bit 0x20 it fills when no image exists, or draws the image.
The boolean stored with the image is **false for direct blit, true for edge
drawing** (TE12; byte W+0x180, `0x009BEBAC–BC`). False submits the image's
reported source rectangle and the full draw area to context slot +0x9C
with final arguments 0, 0. True copies that source rectangle, divides its
right and bottom words by 3 using signed integer division toward zero,
and calls RZDrawBltEdge with final mode **1**. Its destination is the draw
area with an existing vertical bar's width removed from the right and an
existing horizontal bar's height removed from the bottom. The direct
branch does **not** perform those strip subtractions
(`0x009BEBBE–0x009BEC53`). These are source right/bottom words, not a
general computation of `(right-left)/3` and `(bottom-top)/3`; ordinary
zero-origin images make the distinction invisible. Pixel behavior within
the foreign blit helpers remains a separate contract. With nonzero visible
width it draws each visible line, stepping y
by line height and clipping to draw area minus gutters and scrollbar
strips. Wrapping without horizontal bar uses the line span directly;
otherwise horizontal metrics choose the visible span and x displacement.

A selection is painted when focused or bit 0x10000 permits inactive
selection. Endpoints are sorted only for overlap calculations. A fully
selected span gets its measured-width highlight rectangle then selected
colors. A partially selected span first draws the ordinary span, fills
only the measured selected interval and redraws that interval in selected
colors. Empty selection does not cause a highlight fill. Thus selection
background is not simply a whole-line fill.

Caret drawing requires focus, caret visibility and insertion line within
the visible row range. Its x comes from measured insertion prefix plus
draw-left+gutterX minus horizontal scroll. Native unsigned comparison
against line height 24 selects width **1 below 24, 2 at/above 24**
(`0x009BF07C–F08C`). Wrapped caret x at/beyond the right edge is pulled
back inside; caret y is clipped to draw-top+1 and draw-bottom-1. Drawing
context +0x64 receives this width as a **32-bit float**, a vertical line
is drawn through +0x88, then +0x64 is explicitly reset to float 1.0
(`0x009BF0FA–F132`). This restores width to 1, not its previous value.
Editor bit 0x10 plus focus finally draws a one-pixel outline.

### Password coordinates and pixel selection (TE6)

Password mode changes the draw buffer and translates each selection
endpoint from original-prefix byte length to glyph count. It does **not**
translate the line-start array, line byte counts, horizontal visible-span
indices, insertion line or insertion column. Painting still obtains line
starts from W+0x108 and calls the ordinary GetHorizontalLineMetrics;
caret measurement passes maskBegin+originalLineStart and the **stored
byte column** to font +0xC0 (`0x009BECAE-ED07`, `0x009BEFEB-F071`).
Thus a multibyte prefix can yield a glyph-based selection endpoint while
the caret measures more mask bytes. Native calls can describe a span
beyond the mask's logical length; actual foreign drawing/memory behavior
there is outside the safe buffer domain, not corrected Unicode rendering.

GetLineAndColumnFromPosition (`0x009BF162`, T, 387B) also chooses the
mask in password mode and sets its stepping selector to zero, but retains
the original line starts/counts. Signed negative y first resets **both**
x and y to zero; negative x is then clamped to zero. Its adjusted x is
x-gutterX+horizontalScroll. Its row is firstVisible+unsigned(y/lineHeight),
capped to the last line: there is **no gutterY subtraction**. Empty chosen
buffer yields (0,0). The line-height division precedes this empty check.

Within a nonempty row it measures successive chosen-buffer prefixes until
the signed measured width exceeds adjusted x. Let p be the preceding
prefix width and q the new width. It backs up one character when
(x-p-1) < (q-x), with signed 32-bit comparison (`0x009BF2B9-2CA`). For a
10-pixel character, x=5 chooses its start and x=6 its end; this differs
from the separate GetColumnFromPosition's whole-prefix test. On this
interior-hit branch only, password mode converts the selected mask column
through LengthFromGlyphCount using the **beginning of the original whole
text**, its original selector and that column (`0x009BF2CC-2E3`). If no
prefix exceeds x, it instead returns the original line byte count directly,
without that conversion. No original-line-relative conversion is inserted.
These rules specify the mixed indices, not multibyte shaping or graphemes;
the encoding primitive residual is TE1.

## 10. Mouse, messages, flags and resource lifetime

Left down (`0x009BE862`) with editable bit 1 recognizes a double click
solely from an already-running elapsed timer being **strictly below**
+0x220 (default 500); no distance or previous-button-position check occurs
in this body. A double click resets the timer; otherwise it restarts it.
It captures, maps mouse position to insertion, corrects preceding LF,
updates active selection and, without shift bit 1, anchor. Double click
then calls SelectCurrentWord. It invalidates and resets preferred X.
Without editable bit 1 it instead sends `0x9B1552D6` with T and mouse x/y.

During captured editable mouse movement (`0x009BD1F6`), it starts the scroll
timer, remaps insertion and active selection and invalidates only when the
active end changed. Noncaptured editable movement stops the timer. Left up
(`0x009BD1CD`) releases capture and stops scrolling only when editable and
currently capturing. Right down (`0x009BC6D8`) forwards `0x9B1552D7` with
T and x/y only when not capturing.

Wheel (`0x009BC705`) is ignored while capturing. Otherwise it issues
floor(abs(delta)/100) line-scroll calls, positive up and negative down.
Division is signed truncation by +100/-100 (`0x009BC71A–C73E`); no residual
is accumulated. Delta +99 therefore causes no scroll; +199 causes one.
It returns true even when no movement occurs.

DoMessage (`0x009BD0E1`) handles type 3 with secondary ID `0x887113A3`:
payload orientation 0 samples vertical scrollbar value into firstVisible;
orientation 1 samples horizontal into pixel scroll; missing bar is
consumed. A changed value invalidates. Other orientations delegate to base.
Types `0x631B56CC` / `0x631B56CD` toggle caret / perform cursor scroll;
other messages delegate. Callback wrappers obtain their dispatcher and
post those IDs; the timer primitive owns scheduling and delivery lifetime.
Focus gain/loss invalidate and conditionally send `0x9B1552DA` with bit
0x20000 / `0x9B1552D9` with bit 0x8000. Focus gain then calls base handling.

Text SetFlags (`0x009BFCEF`) compares requested flags with old flags,
computes XOR before normalization, and then normalizes: 0x2000 forces
0x200 and clears bits 2/4; 0x200 clears 2/4; bit 2 clears 4. When initialized,
changes under 0x41 update caret timer from window enabled state and bits
0x40/1; changes under 0xB reflow, invalidate and choose editable cursor 3
or clear cursor. Side-effect masks use the **pre-normalization XOR**
(`0x009BFD04–FD7C`). The text-interface set/clear-bit wrappers call this
whole-word setter. Base-window SetFlag (`0x009BC7DD`) is distinct: only
exact base flag 1 additionally controls the caret timer before delegating.

Init (`0x009BFB9C`) is idempotent by base init flag and returns true. First
init calls base, tries WCM font by requested style or default 0x68963C4C,
falls back to that default if requested lookup fails, reserves line starts,
sets editable cursor 3, optionally starts caret, refreshes code page,
acquires/AddRefs clipboard service if absent, and recalculates. It does
not fail its return merely because font lookup or clipboard acquisition
fails. Later font-dependent operations are not all null-safe.

SetFont (`0x009BFDC6`) changes the held reference only for a different
pointer, then reflows and invalidates if initialized. SetFontStyle
(`0x009BC904`) always stores the requested style and returns true; after
initialization it applies a successfully found font, without Init's second
fallback lookup. SetBackgroundImage (`0x009BC9AC`) AddRefs incoming before
releasing old, compares its mode independently, and invalidates if either
changed. Its getter returns the stored pointer without an AddRef in its body.

Horizontal gutter changes (`0x009BFE03`) recalculate width and full layout;
vertical gutter changes (`0x009BCA09`) only invalidate when initialized.
They are not symmetric reflow setters.

Shutdown (`0x009BD003`) first clears the held font reference, then stops
running caret/scroll timers, releases and
nulls both scrollbar interfaces, both scrollbar images and clipboard
service, then calls base Shutdown. The deleting destructor wrapper
`0x009BFF52` calls helper `0x009BFAE3` then deallocates storage. The helper
rewrites both editor vtables, calls editor Shutdown, destroys scroll then
caret callback timer, invokes cleanup for the +0x208 member, releases the
font, destroys redo then undo storage, destroys the strings at +0x144 and
+0x130, frees nonnull line-start storage, destroys mask and main strings,
and tail-calls cGZWin's destructor. Native order is `0x009BFAE6–FB81`;
foreign allocation/timer implementations remain explicit boundaries. Because
Shutdown has already cleared the font, the later conditional font-release
site normally sees null. The additional resource contract below establishes
that native editor cleanup has no matching background-image release.

The primary +0x26C hook is natively a sound-service adapter at
`0x009BD9F2`, despite the folded label `cGZWinIMEComposition::SetFont`.
It indexes sound IDs at W+0x27C+4*reason. Zero ID or no RZ::SS returns
false. Otherwise it calls service +0x34 with (5,0x5B523BA4,out,ID,0,0,0),
then +0x20 on successful acquisition, releases a nonnull temporary handle
on either path, and returns the play result or false. Native evidence
`0x009BD9FC–DA62`. No user-facing error dialog or text mutation is implied;
foreign sound service interpretation is outside this owner.

### Additional resource and callback contracts (TE7)

SetBackgroundImage (`0x009BC9AC`, T) compares identity first: a changed
nonnull image is AddRef'd before releasing the old nonnull image and
storing it at W+0x17C. Unchanged identity performs neither reference call.
The separate byte option at W+0x180 can still change. Either change
invalidates; neither change does not. **Native editor Shutdown and its
destructor contain no matching release of W+0x17C**. In particular the
W+0x208 destructor helper is just a return (`0x00A6D837`), not hidden
background-image cleanup. Replacing/clearing through the setter releases
the prior image; do not infer balanced teardown of this retained reference.

The two scrollbar-image setters (`0x009BD278`, `0x009BD2F9`, T) differ:
they always AddRef the incoming nonnull image, then Release the old
nonnull image, **even on identity replacement**. They store it at
W+0x1DC / W+0x1E0 and copy four rectangle words to W+0x1E4 / W+0x1F4.
The rectangle pointer must be valid even when the image is null. With a
nonnull image and existing corresponding bar, call bar +0x44 with
(rectangle,image,0). If the supplied rectangle is empty (right <= left or
bottom <= top, signed comparisons in `0x007C1870`), use the image's +0x30
rectangle for that call instead. A null image updates storage without
notifying an existing bar. Both normal paths return true and do not
inspect the bar's result. Shutdown releases and clears both stored images,
after releasing/clearing both bars; the background image is a separate field.

Color storage is unchecked: SetColor/GetColor directly access
W+0x1A8+4*index and return true (`0x009BC967`, `0x009BC97B`). No invalid
index returns false; callers must keep accesses within the intended color
fields. ClearFontColor writes 0xFF000000 for indices 0/2, 0xFFFFFFFF for
4, and 0x40000000 otherwise, at that same unchecked address; it does not
invalidate. Raw color storage differs from SetFontColor, which first calls
the base +0x200 conversion; GetFontColor uses base +0x1FC conversion and
writes the caller's output only when virtual GetColor succeeds. Its own
default GetColor always succeeds. SetOutlineUse toggles bit 0x10 and
converts/stores its color at W+0x1C8; GetOutlineUse writes both the bit and
the reverse-converted color. Neither outline body invalidates or checks
output pointers (`0x009BD4B4-50D`).

Scroll timer start checks running state and requests interval 100 with
argument zero only when stopped; stop calls the timer only when running
(`0x009BCC74-CAE`). Neither wrapper defines a boolean success result or
recovers timer failures. The scroll callback obtains RZ::WM, does nothing
if null, otherwise calls +0x38 with (callback argument,0x631B56CD,0,0,0).
The caret callback similarly sends 0x631B56CC through its temporary service
wrapper, then destroys that wrapper (`0x009BD16C-19A`). Neither callback
changes selection, scrolls or paints synchronously, retries, or branches
on dispatch success. Timer delivery and queued-message cancellation remain
foreign service contracts.

Init returns true on its normal completion even if WCM is absent or both
font lookups return null; it does not test the base Init result. The
editable-cursor path dereferences the cursor-manager result without a null
test (`0x009BFC24-33`). Such a missing manager has no owner-level clean
failure result. Pixel hit mapping divides by stored line height without a
zero guard, and font-dependent metrics dereference the font; these methods
require initialized, coherent layout and valid font/service inputs. No
generic pre-init/null-resource safety is promised. Destruction's history
cleanup destroys every owned action string then frees nonnull vector
storage, using the range helpers specified in section 6.

## 11. Independent distinguishing vectors

These are specification examples derived from bodies, not game-run results.

| Setup and action | Required observation |
|---|---|
| Fresh editor, before Init/layout | Colors at W+0x1C0/+0x1C4/+0x1C8 all `0xFF000000`; visible metrics and horizontal offsets zero; line height 1; image edge option true |
| Fresh editor, before Init; inspect history limit and preferred X | Maximum undo levels `0x4000`; preferred X `0xFFFFFFFF` (unset), rather than zero |
| Preexisting distinct bytes in W+0x27C..+0x288; construct editor with flags zero | Those sound words are untouched; no assertion of zero IDs |
| Initialized A reflows with shared counter 0; automatic check reenters B's RecalculateAll | A checks at counter 1; B updates layout but performs no automatic check; normal return restores counter 0 |
| Shared counter -1 versus 1, otherwise initialized editor | -1 permits a check (temporary 0); 1 skips it; normal return retains the original counter |
| Image source `(0,0,30,18)`, draw area `(0,0,100,60)`, vertical width 7, horizontal height 5 | Option false: direct source `(0,0,30,18)`, destination `(0,0,100,60)`; true: edge source `(0,0,10,6)`, destination `(0,0,93,55)`, mode 1 |
| Text `abcde`; SetSelection(4,1) | Endpoints remain (4,1); insertion is 1, not 4 |
| Same text; SetSelection(0xFFFFFFFF,2) | Endpoints (5,2); insertion 2; first argument does not clamp to zero |
| Existing `abcde`; lower maximum to 2 | Text remains `abcde` immediately after setter |
| Ordinary code page; SetText(`abc`,0,false), maximum 2 | Stores `ab`, reason-1 hook; history cleared |
| Encoding helper reports a two-byte character; maximum 1; SetText of that character | Selectors 8/100 can retain 2 bytes; ordinary path retains 1 |
| SetInsertMode(true), then false | Overwrite bit becomes clear, then set; no text mutation |
| Focus elsewhere; ToggleCaretVisibility | No visibility transition and no invalidation by this helper |
| Caret period 1001; start stopped timer twice | First request passes 500 and T; second returns false without another start |
| Editable character input byte 0x09 | Returns true before high-level insertion |
| Permitted Ctrl+X, clipboard service null, selection (1,3) | Delete still runs; selected bytes removed if editable |
| Bit 0x2000 set, 0x1000 set, no capture; Ctrl+C versus Ctrl+V | Copy suppressed; paste still calls manager if present |
| Wheel +99 followed by +99, not capturing | No line-scroll calls on either event; no accumulation |
| Positive visible height 7; font height 16 | One visible row, rather than zero |
| Width sufficient; text `a` versus `a` followed by LF; font present, no wrap | One line start versus two; final LF creates empty trailing line |
| Font glyph advance 10; GetColumnFromPosition adjusted x 9 | First character's starting column 0; no nearest-edge rounding |
| Focused visible caret; line heights 23 then 24 | Drawing widths float 1.0 then 2.0; each reset to float 1.0 |
| Change only vertical gutter on initialized widget | Invalidates without the horizontal setter's full reflow |
| Two far-apart left downs within timer interval | Second takes double-click word-selection path; no spatial test in owner |
| Noneditable widget, clipboard bits permit copy; Ctrl+Escape | Runs copy gates; editable Escape instead sends message 5 |
| Only editor bit 0x400 set; Return key-down | Sends 0x9B1552D5 with (T,13,0); editable bit 1 is not required |
| History enabled, limit 8, eight distinct undo actions; create an insertion record | Oldest two are destroyed, six survive in order, new action appended: count 7; redo emptied |
| Same eight actions and limit 8; create a deletion record | All eight survive, new deletion appended: count 9; redo emptied |
| Empty enabled history, limit 0; create five insertion records | Counts 1,2,3,4,4; the fifth evicts only the first record |
| Enabled history with three actions; lower limit to 0 | Setter preserves all three; next insertion keeps them and appends a fourth |
| Selector 8 or 100, existing length 5, maximum 2, nonempty input unchanged by filtering/composition; raw insertion | Reason 1, zero retained bytes and zero glyph movement; normal return 0, existing bytes retained |
| Ordinary selector, existing `abcde`, maximum 2; raw insertion of `x` | Retained count wraps to 0xFFFFFFFD; downstream string length-failure helper is called, rather than a clean return 0 |
| Empty text, ordinary-character RemoveChar, selector 0, no output string | Manufactured index 0xFFFFFFFF reaches string erase's out-of-range failure path; no normal true/no-op contract |
| Password text has original bytes C3 A9 7A under selector 8; glyph helper counts two; caret/selection endpoint 2; mask byte `*` | Mask length 2 and translated selection endpoint 1, but caret metric still receives stored column 2; observe call arguments without reading an invalid span |
| Monospace 10-pixel prefixes, gutters/scroll zero, one nonempty line; pixel hit x=5 then x=6 | GetLineAndColumnFromPosition returns columns 0 then 1; separate GetColumnFromPosition returns 0 for both |
| Two rows, firstVisible 0, line height 10, gutterY 7; pixel hit y=10 | Row 1; gutterY is not subtracted |
| Same background pointer and option, then same pointer with changed option | First call no reference calls/invalidation; second invalidates with no reference calls |
| Same nonnull scrollbar image assigned again with valid empty rectangle and existing bar | Incoming AddRef precedes old Release, rectangle copied, image +0x30 rectangle passed to bar +0x44 with image and 0 |
| Null WM/service result in scroll/caret callback | No dispatcher call and no direct widget mutation; no retry |
| SetOutlineUse with valid color and either enable value | Bit/color change but no invalidation by this method |
| Text `ab` + LF + `cde`, coherent starts [0,3]; GetIndexFromLineAndColumn(0,4) | Writes 4 and returns true: column can cross the requested line boundary |
| Same text; GetIndexFromLineAndColumn(99,1) | Uses last start 3, writes 4 and returns true despite out-of-range line |
| Ordinary byte text `one,two  three`; word step +1 from 0 | Lands at byte 9; punctuation is not a boundary |
| Selector 101; ordinary versus composed stepping +1, composed helper reports 2 | Ordinary advances one byte; composed advances two |
| `abcd`, byte steps 1, font advances 10/prefix, visible width 15; horizontal metrics with scroll 9 then 10 | Absolute spans [0,3), displacement -9; then [1,3), displacement 0 |
| Unwrapped pixel mode, horizontal bar present, offset 5, line height 16; ScrollLeft | Offset becomes signed -11; value update/invalidation occurs and return is true |
| Same mode, visible width 100, offset 95, line height 16, longest line 500; ScrollRight | Offset capped at 100, not 111 or 400 |
| Editor 100x50, vertical width 10, horizontal height 8, both bars; resize bars | Horizontal (0,42,90,50) first; vertical (91,0,100,42) second |
| Ten lines, four visible rows, firstVisible 5; ScrollPageDown | FirstVisible becomes 9, with value update/invalidation and true |
| Capturing, cursor x=-3,y=-3; ScrollLineUp succeeds | Vertical route wins; +0x228 gets (original x,lineHeight/2,0), with no horizontal scroll call |

Additional final-round proposed vectors (45-52), derived from native
branches, not executed results. Helpers/services are controlled boundary
inputs; unsafe spans are observed by call arguments, not dereferenced.

| Independent case | Distinguishing expected behavior |
|---|---|
| Selector 8, logical slice E0 of length 1 with readable following byte 80 | Forward length 3; no cap to the supplied length |
| Selector 8, two-byte prefix 80 BF | Previous length 3 and glyph count 0 |
| Selector 100, bytes 81 41 | Forward length 1 and glyph count 2; no generic DBCS inference from selector >=100 |
| Enabled noncapturing selector 8; character C0 then 41 | First returns true and saves C0; second offers both bytes to high-level insertion and clears pending, returning false |
| Same, E0 then 41 then 42 | First two return true; third offers all three despite invalid trail bytes |
| Bit 0x200; byte/prefix advance 10 pixels, visible width 40, column 6, offset 0 | Relative X=60 starts stepping; offset reaches 4 and relative X=20, because X=30 equality does not stop |
| Both bar bits/arguments set; both manager/bar results null | Each direction clears its bit; final recalculate/size/values(true) still occurs; returns true |
| Text `abc def`, insertion 5, preexisting selection [0,5] | Word selection extends upper end to 7 and retains lower 0, rather than first collapsing to the word `def` |

Additional final-round proposed vectors (53-60), with controlled foreign
inputs and no claims of execution:

| Independent case | Distinguishing expected behavior |
|---|---|
| Cache X=70, current measured X=20; compare Page Down and Down arrow from an interior line | Page uses 20 for destination-column query; arrow uses 70; both retain cache 70 |
| Byte text `abc`, destination three sentinel bytes, buffer GetText requested count 2 | Copies `ab`, count remains 2, third sentinel untouched; no appended NUL |
| Selection anchor 1, active 4; both GetSelection output pointers alias one valid external word | Final output 4, return true |
| Byte text `abc`, coherent line starts [0]; insertion and both endpoints 99; repair | Insertion 3, active end 3, anchor still 99; line/column recomputed |
| Not initialized; SetCaretPeriod(0), then SetCaretPeriod(0xFFFFFFFF) | Stored periods 250 then 10000; no timer/flag calls |
| Text `a` + LF + `b`, starts [0,2]; GetCharacterCountOfLine(0), then (99) | Returns byte spans 2 then 1 |
| Bit 0x200, selector 0; filter CR LF `x` through string overload | Removes only LF; resulting string CR `x`, removed count 1 |
| Composition candidate `AB`; consumed length 1 for each; level rejects A and accepts B; initial previous P | A erased, B retained; B's level test still receives previous P; removed count 1 |

Final-round proposed vectors (61-64), also not executed:

| Independent case | Distinguishing expected behavior |
|---|---|
| FirstVisible 0, horizontal byte offset 0, pixel offset 5; ScrollToStart | Returns false, leaves pixel offset 5, no update/invalidation |
| Nonempty starts [0,2], D=3, requested entry count 1; AdjustLinePositionArray | Writes starts[0]=0 but keeps vector length 2; no general shrinking |
| Stored flags include only bit 1; IsFlagSet(mask 3) | True because any requested bit suffices |
| Controlled color service records slots; set caret then highlight fill colors | Caret setter uses primary +0x200; highlight setter uses +0x1FC; neither invalidates |

## 12. Whole-owner checklist and current limits

The name search contains 146 matching functions including adjusted thunks,
STL helpers, a factory and a nested action constructor. Direct, non-nested
`cGZWinTextEdit::` labels account for **134 bodies / 19,415 bytes**.
The actual coverage owner output assigns **150 bodies / 19,999 bytes**:
all 134 direct entries plus five adjustment thunks and eleven unnamed
reach/neighbour helpers. The address join has no missing direct entries
or changed sizes. See the [measured attribution](text-edit-attribution.md)
and [address-by-address checklist](text-edit-inventory.md). This reconciles
the brief's byte total, not its behavior coverage; no whole-owner credit
is claimed while body and foreign-boundary residuals remain.

| Area | Current evidence disposition |
|---|---|
| Identity, state, whole-text replacement, selection, overwrite, character input, high-level insertion, caret timer | Rules above; external helper contracts remain bounded |
| Raw insertion/deletion, undo/redo, key navigation/clipboard, mouse, focus | Sections 6-10 and layout/encoding extensions; TE3/TE4 domains/eviction, page/arrow arguments and TE5 composition resolved |
| Line starts, metrics, painting, hit mapping, scrollbars | Sections 8-9 and layout extension; password coordinates, caret-in-view and AddScrollBar owner ordering specified; individual residual bodies remain TE8 |
| Init/shutdown, images/font ownership, flags/messages | Section 10 specifies owner cleanup, including absent background-image release; foreign resource implementations remain boundaries |
| Every label, folded alias, thin method and helper | Companion inventory and questions required; no credit beyond documented contracts |

Final round 3/3 adds [byte helper contracts](text-edit-encoding.md) and
[individual adapters/repair contracts](text-edit-adapters.md). TE1 is
answered; TE5 conversion/composition was completed on 2026-10-24. TE8's
allocation and C-library dependencies are tracked separately below. The 150-body attribution is not a
150-body behavioral proof, and no whole-widget-family credit is claimed.

Open contracts live in [questions/text-edit.md](questions/text-edit.md).
