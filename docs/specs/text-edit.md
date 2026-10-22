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
existing text length is not a safe general insertion contract (TE3).

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
avoid that case. Its return is true, not a removed-byte count.

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
The insertion helper's count-limit eviction and the deletion helper's
absence of that test remain TE4; do not substitute a conventional bounded
undo stack. SetMaxUndoLevelCount (`0x00994ED7`) merely stores T+0xAC;
it does not immediately prune either stack.

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
| Up/Down | In editable multiline mode move one line, preserve a preferred pixel X and use font hit mapping. Without shift collapse selection; with shift keep anchor. When editor is noneditable or single-line, bit 0x800 instead forwards message 5 with key in the secondary payload. Boundary-without-selection invokes reason 2. |
| Page Up/Down | Editable multiline only; move by visible-line count, clamp to first/last line, map the pixel X on destination and apply shift-selection rules. Boundary-without-selection invokes reason 2. Exact preferred-X reuse differs from Up/Down; see TE5. |
| Escape / Return | With editor bit 0x800, forward message 5 to notification target with key and original modifiers; Escape first collapses a nonempty selection and invalidates. No LF is inserted by the key-down path itself. |

Clipboard evidence: native gates/copy call/cut redispatch at
`0x009C11E3–12A0`; paste branch `0x009C1504` onward. No clipboard manager
implementation, OS format, newline conversion, persistence, error UI or
ownership beyond the local string lifetime is inferred. No real user
clipboard was accessed for this specification. The clipboard service may
be absent; paste then does nothing, while a permitted cut still dispatches
Delete. This is a distinguishing failure behavior.

The LF correction helper (`0x009BD0AF`) decrements insertion index and
column by one only when both are positive and the byte immediately before
insertion is LF. It does not change insertion line or selection itself.

## 8. Layout, font primitives and view

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
three-quarter-width target; its helper contracts remain TE5.

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
that exact interaction is TE6.

With background bit 0x20 it fills when no image exists, or draws the image.
The boolean stored with the image chooses direct blit versus edge drawing
using image dimensions divided by three, excluding existing scrollbar
strips. With nonzero visible width it draws each visible line, stepping y
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

Shutdown (`0x009BD003`) stops running caret/scroll timers, releases and
nulls both scrollbar interfaces, both scrollbar images and clipboard
service, then calls base Shutdown. The deleting destructor wrapper
`0x009BFF52` calls helper `0x009BFAE3` then deallocates storage; destructor
helper and base/child/resource destruction ordering remain TE7.

The primary +0x26C hook is natively a sound-service adapter at
`0x009BD9F2`, despite the folded label `cGZWinIMEComposition::SetFont`.
It indexes sound IDs at W+0x27C+4*reason. Zero ID or no RZ::SS returns
false. Otherwise it calls service +0x34 with (5,0x5B523BA4,out,ID,0,0,0),
then +0x20 on successful acquisition, releases a nonnull temporary handle
on either path, and returns the play result or false. Native evidence
`0x009BD9FC–DA62`. No user-facing error dialog or text mutation is implied;
foreign sound service interpretation is outside this owner.

## 11. Independent distinguishing vectors

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
| Permitted Ctrl+X, clipboard service null, selection (1,3) | Delete still runs; selected bytes removed if editable |
| Bit 0x2000 set, 0x1000 set, no capture; Ctrl+C versus Ctrl+V | Copy suppressed; paste still calls manager if present |
| Wheel +99 followed by +99, not capturing | No line-scroll calls on either event; no accumulation |
| Positive visible height 7; font height 16 | One visible row, rather than zero |
| Width sufficient; text `a` versus `a` followed by LF; font present, no wrap | One line start versus two; final LF creates empty trailing line |
| Font glyph advance 10; GetColumnFromPosition adjusted x 9 | First character's starting column 0; no nearest-edge rounding |
| Focused visible caret; line heights 23 then 24 | Drawing widths float 1.0 then 2.0; each reset to float 1.0 |
| Change only vertical gutter on initialized widget | Invalidates without the horizontal setter's full reflow |
| Two far-apart left downs within timer interval | Second takes double-click word-selection path; no spatial test in owner |

## 12. Whole-owner checklist and current limits

The name search contains 146 matching functions including adjusted thunks,
STL helpers, a factory and a nested action constructor. Direct, non-nested
`cGZWinTextEdit::` labels account for **134 bodies / 19,415 bytes**.
That is a label inventory, not yet reconciliation to the task's 19,999-byte
owner attribution. Folded aliases and unnamed native helpers must be
counted separately; no whole-owner credit is claimed.

| Area | Current evidence disposition |
|---|---|
| Identity, state, whole-text replacement, selection, overwrite, character input, high-level insertion, caret timer | Rules above; external helper contracts remain bounded |
| Raw insertion/deletion, undo/redo, key navigation/clipboard, mouse, focus | Sections 6-10; TE3/TE4 and exact word/composed motion remain bounded |
| Line starts, metrics, painting, hit mapping, scrollbars | Sections 8-9; TE5/TE6 prevent full owner closure |
| Init/shutdown, images/font ownership, flags/messages | Section 10; destructor ordering and other resource edges TE7 |
| Every label, folded alias, thin method and helper | Companion inventory and questions required; no credit beyond documented contracts |

Open contracts live in [questions/text-edit.md](questions/text-edit.md).
