# TextEdit adapters and repair bodies

Companion to [text-edit.md](text-edit.md). Native Windows 1.1.641
instructions establish these individual bodies. W is the window base;
T=W+0xD8. Names are export labels, not assumed signatures. No complete
owner, font/window service or generated-helper coverage is implied.

## Text and state access

| Export label / address | This / contract |
|---|---|
| GetTextLength, 0x0091880E | T; takes an output string interface and calls its +0x0C with original begin and byte length. Returns that call's result. Despite the label, this is a copy-out adapter, not a numeric length return. No null-output guard. |
| GetText, 0x00994A39 | T; returns original byte length (end-begin), despite the label; no string copy. |
| GetTextGutterX/Y, 0x0099357F/586 | T; return raw words at +0x80/+0x84 (W+0x158/+0x15C). |
| GetMaximumTextToAllow, 0x00994E6A | T; returns +0xF8, no recomputation. Folded setter 0x00994E71 stores its argument there without side effects, as section 2 specifies. |
| GetFirstVisibleLine, 0x00994E7E | T; returns +0x94 (W+0x16C). |
| GetLineHeight, 0x00994E85 | T; returns +0xF4 (W+0x1CC). |
| GetLineCount, 0x00994FF2 | T; returns arithmetic-shift-right-2 of line-vector end minus begin, without a minimum-one correction. |
| GetCaption, 0x00994EB9 | W; returns the address of the embedded original string W+0xDC. No copy/AddRef. |
| SetCaption, 0x00994EC0 | W; delegates to text slot +0xC0 with incoming string interface and notify=false. This is the string SetText adapter already specified in section 2. |
| GetFont, 0x009BD3B9 | T; returns +0xC8 (W+0x1A0), with no AddRef. |
| GetInsertMode, 0x009BCB14 | T; returns the Boolean inverse of flag bit 0x4000. |
| GetInsertionLineAndColumn, 0x009BCB2E | T; writes line +0x3C then column +0x40 to caller outputs and returns true. |
| GetSelection, 0x009BCBAE | T; writes anchor +0x48 then active end +0x4C and returns true; no ordering or clamping. |

The two-output getters reread the second stored word after the first write;
outputs are not checked and aliased outputs receive the second write last.
None of these accessors recalculates layout or invalidates the editor.

The buffer GetText (`0x009BE2DB`, T) takes destination and an in/out byte
count. It caps an excessive unsigned count to original byte length, then
calls `0x009BDDF2` on the internal string with destination, count, start=0.
That helper copies min(requested, length-start) bytes with memcpy only when
nonzero, and returns copied byte count. It does **not** append a NUL.
The general helper's start>length branch calls `0x00407C40`; this adapter
always uses start zero. Output count is required even for empty text;
destination need only be writable when copied count is nonzero. Overlapping
buffers inherit the memcpy boundary; no special overlap repair is present.

## Insertion repair and line counts

GetCharacterCountOfLine (`0x009BD554`, T) returns zero for empty text.
Otherwise, with a coherent nonempty line array, a line below last returns
nextStart-start; a line at or beyond last returns textLength-lastStart.
It is a **byte span**, including an LF before the next start; no glyph
counting or terminator subtraction occurs. Nonempty text with no line starts
is outside its safe-call domain (`0x009BD56E-57F`).

SetInsertionIndex (`0x009BD66E`, T) caps an unsigned excessive argument
or 0xFFFFFFFF to text length, stores the byte index, recomputes cached line
and column through text +0xF4, invalidates primary +0x170, and returns
true. It does not alter selection, preferred-X or put insertion in view.

SetInsertionLineAndColumn (`0x009BD6B4`, T) zeros line/column/index for
an empty line array. Otherwise it clamps unsigned line to last, calls text
+0xFC for that line's byte span, caps unsigned column to that returned span,
stores line/column, and obtains index through +0xF8. The conversion result
is ignored. It returns true with no invalidation, selection or preferred-X
change. Native instructions read the two separate arguments at
`0x009BD6D3` and `0x009BD6F3`; the pseudocode's merged argument is wrong.

FixInsertionPositionIfBroken (`0x009BDA65`, W) zeros all five
line/column/index/selection words if text is empty. Otherwise it caps only
insertion index to length, recomputes line/column, then caps only the active
selection end W+0x124 to length. It leaves anchor W+0x120 untouched,
does not sort endpoints, repair an encoded character or invalidate. This
asymmetry matters to SelectCurrentWord and mutation callers.

## Period, sound and lifecycle adapters

SetCaretPeriod (`0x009BCB45`, T) clamps the **unsigned** request to
[250,10000], stores T+0x14C (W+0x224), and returns true. If the base
window is initialized and editor bit 0x40 is set, it calls text ClearFlag
+0x138(0x40) followed by SetFlag +0x130(0x40); it does not directly
restart a timer. GetCaretPeriod (`0x009BCB9D`) unconditionally writes that
word to its output and returns true. A negative-bit-pattern request thus
caps at 10000, not 250. Timer delivery remains foreign.

SetSoundID (`0x009BCC57`, T) accepts a **signed** index below four,
stores the ID at T+0x1A4+4*index and returns true; signed indices >=4
return false without a store. There is no nonnegative test. Negative
indices can overwrite preceding object state and are not supported benign
inputs; no real resource call is needed to distinguish this rule.

GZOnKeyUp (`0x009BC6B5`, W) returns true for every key except Escape.
Escape dispatches primary +0x240(notificationTarget,6,0x1B,modifiers,0)
and returns that result, independent of edit/selection state.

UpdateScrollBarsAndWindowPosition (`0x009BE945`, W) calls
PutInsertionPositionInView then TestForPossibleScrollBarAdditionOrRemoval
only if initialized. It always tail-calls primary invalidation +0x170.
ShowWindow (`0x009BE99F`, W) performs that update **before** tail-calling
base ShowWindow with the original argument. This is not an editor-local
visibility flag or a conditional update only when showing.

ResizeWindowForExactLineHeights (`0x009BE111`, T) obtains base height
H, subtracts twice gutterY with wrapping 32-bit arithmetic, divides that
unsigned result by unsigned lineHeight, then uses quotient one if its
signed interpretation is <=0. It multiplies by lineHeight, and only when
this differs from H-2*gutterY calls primary +0xD0 with the product plus
2*gutterY. It then always resizes bars, recomputes values(true), invalidates
and returns true. A zero lineHeight reaches native DIV at 0x009BE13C;
there is no safe no-op. Normally H=25,gutterY=2,lineHeight=10 requests
height 24; H=9 with those gutters requests 14 (at least one row).

## Filtering and composition repair

The buffer RemoveBadCharacters (`0x009BF335`, W) returns zero without
buffer/count access unless bit 0x200 is set. When set, it walks the supplied
byte count using GetCharLength and builds a temporary string of retained
chunks. A chunk whose **first byte is LF** is omitted, adding its helper
length to the removed count; all other chunks, including CR, are retained.
If anything was removed it calls lib::_strncpy(destination, temporaryBegin,
the original supplied count), then writes the temporary length back to the
count. The copy writes exactly the original supplied count: bytes before
the first NUL are copied, and that NUL and the rest of the destination range
are zero filled. If no NUL occurs in that count, none is appended. A zero
count writes nothing. Thus filtering `a LF b` with count 3 writes `61 62 00`,
while filtering `a NUL LF b` with count 4 writes `61 00 00 00` even though
the reported retained length is 3. This embedded-NUL truncation is a copy
quirk, not another removed-character test. Source and destination must be
valid nonoverlapping ranges; neither layer validates them. Instructions at
`009F101C–108F` and `009F1095–1113` establish copying and padding. There is
no separate owner terminator store. It returns removed byte count
(`0x009BF39F-3DF`). Malformed helper lengths are not capped to remaining
input; source-range safety remains the caller's obligation.

The string overload (`0x009BF578`) passes its current begin and a local
byte length to that buffer method. If the returned length differs from the
current string length, it resizes the string to that length with zero fill;
it returns the buffer method's removed count. Neither overload updates
editor selection/history/layout by itself.

FixBadCharacterComposition (`0x009BE614`, W) takes a mutable candidate
string, context string and context byte index. Before even testing candidate
emptiness it calls GetPreviousUTF16Character(contextBegin+index, &indexCopy,
selector) for the previous code value. It scans candidate chunks using
GetUTF16Character(candidateBegin+offset, &remaining, selector); the in/out
remaining word becomes the consumed byte count. A zero consumed count
erases **all remaining candidate bytes** and adds their count to the result.
A nonzero consumed count is tested with
UTF16GetThaiCharLevel(low16(current),low16(previous),0). Zero level erases
that chunk and adds its byte count without advancing offset or changing
previous; nonzero level advances offset and replaces previous with current.
The loop compares offset against the candidate's new length after each
erase, returning total removed bytes. No owner validation of helper bounds,
UTF16 values, nonprogress or context index is present (`0x009BE63C`,
`0x009BE66D`, `0x009BE689`). Conversion and Thai-level outcomes are specified
in [text-edit-encoding.md](text-edit-encoding.md#conversion-te5), including
the zero-consumption and invalid-mark cases.

## Flags, images and color adapters

ClearFlag (`0x009BC837`, T) does nothing if no requested bit is currently
set. Otherwise it calls text SetFlags +0x134 with flags AND NOT mask.
There is no explicit Boolean return construction. IsFlagSet
(`0x009BC854`) returns true if **any** mask bit is set, not only if all are.

GetScrollBarImageVertical/Horizontal (`0x009BC864/88E`, T) write the
stored image pointer then copy all four saved rectangle words, even if the
pointer is null. They return whether the pointer output is nonnull **after**
the rectangle copy. Images are borrowed, with no AddRef. Both outputs are
required; ordinary separate outputs avoid aliasing the pointer with the
rectangle copy. Source fields are W+0x1DC/+0x1E4 vertically and
W+0x1E0/+0x1F4 horizontally.

GetBackgroundImage (`0x009BC991`, T) writes the stored image pointer
from T+0xA4 and option **byte** from T+0xA8, in that order, without AddRef
or output guards. It does not construct a Boolean success value.
GetFontStyle (`0x009BC94E`, T) returns false without an output write when
T+0xCC is null; otherwise writes the stored borrowed pointer and returns
true, without AddRef.

SetFillColor (`0x009BC8DB`, W) calls base SetFillColor and retains its
Boolean result. If editor bit 0x20 is then set, it additionally invalidates
through primary +0x170, independent of the base result; it returns the saved
base result. The editor does not add an identity/no-change shortcut.

SetCaretColor (`0x009BD37A`, T) sends the argument to primary +0x200
and stores the returned word at W+0x1C0. GetCaretColor (`0x009BD399`)
sends that stored word through primary +0x1FC, writes the result to its
required output, and returns true. GetHighlightFillColor (`0x009BD47F`)
returns primary +0x1FC applied to W+0x1C4 directly, with no output pointer.
SetHighlightFillColor (`0x009BD495`) also uses **+0x1FC**, applying it to
the incoming word before storing W+0x1C4. Preserve that measured slot,
even though the caret setter uses +0x200. None of these four adapters
invalidates. Conversion primitive semantics remain the base-window boundary.

## Scroll endpoints, geometry and search

The four vertical scroll methods below take T and leave insertion and
selection unchanged. All successful paths call SetScrollBarValues(false),
invalidate and return true; false paths do neither. Arithmetic wraps at
32 bits, including the differences tested as signed.

| Method | Native condition and mutation |
|---|---|
| ScrollLineUp, 0x009BDE57 | Only if signed(firstVisible)>0; decrement by one. High-bit values do not decrement. |
| ScrollLineDown, 0x009BDE87 | Only if signed(lineCount-visibleRows-firstVisible)>0; increment firstVisible by one. |
| ScrollToStart, 0x009BDFC7 | Only if firstVisible or horizontal byte offset is nonzero; zero firstVisible and both byte/pixel horizontal offsets. A nonzero pixel offset **alone** does not trigger the path. |
| ScrollToEnd, 0x009BE009 | Only if signed(lineCount-visibleRows-firstVisible)>0; set firstVisible=lineCount-visibleRows and zero both horizontal offsets. Already at/beyond the endpoint does not reset horizontal state. |

RemoveScrollBar (`0x009BCDBA`, W) handles requested nonnull vertical
then horizontal bars. For each it obtains bar +0x0C, calls primary
+0x40(RemoveChild) with that result, calls the stored bar's Release +0x08,
then clears the stored pointer. Callback results are ignored. If any bar
was handled it calls RecalculateAll once after both directions; no attempt
means no final call. It does not clear the enable bits or explicitly run
SetScrollBarSizes/Values. No rollback for callback failure is present.

SetArea (`0x009BFCA5`, W) calls base SetArea with all four coordinates,
then CalculateVisibleWidth and stores W+0x160, RecalculateAll,
SetScrollBarSizes, SetScrollBarValues(true), and primary invalidation,
in that order. It does not gate those calls on base success, compare areas
for equality or return a saved base result. Underlying geometry/failure
semantics remain in the base-window contract.

GetIndexFromPosition (`0x009BD510`, T) writes zero for empty text.
Otherwise it asks text +0xEC for line/column from the supplied pixel
position, then writes lineStarts[line]+column, with wrapping addition and
no further bound/character clamp. It ignores any status from the pixel
mapping call; the returned line must index a coherent line-start array.
It does not move insertion or selection.

AdjustLinePositionArray (`0x009BF2E5`, W) takes a byte increment D and
requested entry count N. If unsigned N exceeds current count it first
reads starts[N-2], adds D, and passes N and that fill value to vector resize.
Thus the ordinary one-entry extension of a nonempty array is safe; a larger
jump or empty-array extension can read outside the old entries **before**
resize. If N<=current and N>1 it writes starts[N-1]=starts[N-2]+D without
shrinking the vector. If N<=1 it writes starts[0]=0 without checking
emptiness. This method is not a general safe resize API; negative-bit-pattern
N follows the unsigned excessive-count path. All additions wrap.

FindText (`0x009BE96A`, T) takes byte pointer, byte count, starting
position and a Boolean. True calls string find `0x004427F0`; false calls
`0x005C23E0`, both with (pointer,position,count). It returns their result
only if unsigned result<text byte length, otherwise 0xFFFFFFFF. Thus a
helper returning exactly text length is converted to not-found. There is
no owner null-pointer, case-folding, character or selection adjustment.
For valid byte ranges and nonoverflowing positions/counts, true searches
forward from the supplied position for the first exact byte match; false
searches backward for the last match whose start is at most that position.
Overlapping matches count. NUL is an ordinary byte; there is no encoding or
case test. Forward search returns not-found if position+count exceeds text
length; reverse search returns not-found if count exceeds length, otherwise
caps the candidate start to min(position,length-count). An empty pattern's
primitive result is position on the forward path when position<=length,
and min(position,length) on the reverse path. The owner rejects either
result equal to length. Thus an empty pattern in empty text always returns
not-found; an empty reverse search from 0xFFFFFFFF in nonempty text also
returns not-found. These are quirks, not selection/caret operations.
Evidence: `004427F0–2842`, `004426C0–2756`, `005C23E0–2467`,
`005C2380–23D4`, `009BE96A`.
The forward position+count check uses wrapping 32-bit addition without an
independent position guard. Wrapped sums and invalid pointers are outside
the valid-call domain, not promised benign not-found or repaired input.
