# TextEdit navigation and layout body contracts

Extension of [text-edit.md section 8](text-edit.md#8-layout-font-primitives-and-view).
All rules are original Windows 1.1.641 body contracts. The
composed/conversion primitives and ordinary byte helpers are specified in
text-edit-encoding.md; font metrics remain external inputs. No implementation or
game output is used. Arithmetic is 32-bit unless a different width is stated.

## Byte coordinates and navigation

GetLineAndColumnFromIndex (`0x009BD593`, T) returns (0,0) for no line
starts, empty text or index zero. On a coherent nonempty line array and a
valid index within text, it locates the half-open interval between adjacent
starts and subtracts its start. If none matches, it uses the last line;
when the preceding original byte is LF, column is zero instead. It does
not clamp arbitrary indices before reading that preceding byte; invalid
indices and incoherent arrays are outside this contract.

GetIndexFromLineAndColumn (`0x009BD610-66B`, T) returns false and writes
zero for empty text/line array. Otherwise it clamps an excessive unsigned
line to the last line, adds the column, and compares the resulting unsigned
index with **whole text length**, not that line's end. It returns true when
the sum is within text; otherwise it writes text length and returns false.
Addition is not saturating, and no character-boundary repair is performed.

Ordinary IncrementInsertionPosition (`0x009BD7FA`) steps through
GetCharLength/GetPreviousCharLength only for selectors **8 or 100**;
other selectors add the signed requested delta directly to the byte index.
Composed stepping (`0x009BD71A`) uses composed forward/backward helpers
for selector 8 or a selector **signed >=100**, a different predicate
(`0x009BD723-737`). Each positive/negative step uses the current index and
remaining/prefix byte length. After all steps, each owner clamps a signed
negative resulting index to zero, otherwise caps it at unsigned text
length, then recomputes insertion line/column. There is no per-iteration
owner boundary check protecting an oversized multi-step helper request.

Word stepping (`0x009BD89D-996`, T) uses bytes, not encoding helpers.
Its whitespace predicate recognizes only space and LF. One positive step
scans until a whitespace byte followed by a nonwhitespace byte, then lands
on the following byte; reaching the last byte lands at text end. One
negative step starts at insertion-1 and searches backward for a
nonwhitespace byte preceded by whitespace; reaching index zero lands at
zero. Repeat for the signed requested count, stopping at the corresponding
end. Nonempty zero-delta and empty-text calls return without recomputation;
other paths clamp the result and recompute line/column. Tabs, punctuation
and multibyte continuation bytes have no special word status.

## Horizontal visible spans

GetHorizontalLineMetrics (`0x009BDB83-0x009BDCD0`, W) consumes a valid line, unsigned pixel scroll and nonnull
start/end/displacement outputs. The measured source is **original text**,
including when the caller paints a password mask. Obtain the original line
start and byte count from the line array/text interface.

At zero pixel scroll, start is the line start and displacement is zero.
For positive scroll, start with the first character's byte length and
walk character boundaries while the accumulated byte position is strictly
less than the line byte count. Before each prefix measurement, save the
preceding measured width (initially zero). Stop when the new unsigned
prefix width is **greater than** scroll; otherwise advance another
character. The output start is lineStart+accumulatedBytes-lastStep, and
displacement is savedPrecedingWidth-scroll. The final line-end prefix is
not measured by this loop, so do not replace it with a general inverse
font metric, especially under excessive scroll.

For the right edge, begin one character beyond that output start. Measure
prefixes from output start while the candidate boundary is strictly before
line end. Stop when unsigned measured width is **at least**
visibleWidth-displacement; otherwise advance another character. Return
the candidate boundary as the absolute exclusive end. This includes the
character crossing the viewport edge. Empty line skips this stage. All
subtractions retain native 32-bit arithmetic; no clamp repairs an excessive
scroll or a foreign zero-length stepping result. Evidence for both edge
tests and output arithmetic is `0x009BDBFB-DC41`, `0x009BDC4A-DCC3`.

GetPixelWidthOfLongestLine (`0x009BDB00`) takes the unsigned maximum of
font +0xC0 measurements over original line spans. True mode visits every
line. False mode visits [firstVisible,firstVisible+visibleRows), without an
extra cap in this body; callers must supply a coherent range.

## Scroll offsets, bar geometry and values

GetHorizontalScrollPixels (`0x009BDDC2`) in bit-0x200 mode measures
the original text prefix of byte length W+0x174 through font +0xC0;
otherwise it returns pixel offset W+0x178. It does not measure the mask.

ScrollLeft/Right (`0x009BDEC9`, `0x009BDF39`) in bit-0x200 mode subtract
the previous character length when the signed byte offset is positive,
or add the next character length when unsigned byte offset is below text
length, respectively. They invalidate even when that branch cannot move,
and return true. Without bit 0x200, a wrapped editor with no horizontal
bar returns true immediately without invalidation. Otherwise left subtracts
lineHeight when the signed pixel offset is positive **without a zero
clamp**, and right adds lineHeight when pixelOffset <= visibleWidth and
then caps to **visibleWidth**, not longestLineWidth-visibleWidth. These
pixel paths update bar values without recomputing ranges, then invalidate.
Thus offset 5 with lineHeight 16 becomes -11 after ScrollLeft.

SetScrollBarSizes (`0x009BCE3D-D002`, 454B, W) obtains each existing
bar's window through +0xC and sets its rectangle through window +0xDC.
Let Ewidth/Eheight be editor window +0xA4/+0xA8, Vwidth the vertical bar
window's +0xA4 and Hheight the horizontal bar window's +0xA8:

| Bars present | Horizontal rectangle | Vertical rectangle |
|---|---|---|
| Neither | No call | No call |
| Horizontal only | (0,Eheight-Hheight,Ewidth,Eheight) | No call |
| Vertical only | No call | (Ewidth-Vwidth+1,0,Ewidth,Eheight) |
| Both | (0,Eheight-Hheight,Ewidth-Vwidth,Eheight), set first | (Ewidth-Vwidth+1,0,Ewidth,Eheight-Hheight), set second |

The vertical **+1** is native (`0x009BCF31-0x009BCF44`, `0x009BCF8E-0x009BCF9F`).
There is no nonnegative-size clamp, null returned-window check, result
check or invalidation in this body. Dimensions are queried through virtual
calls, not cached constants; foreign resize side effects can affect later
queries.

SetScrollBarValues (`0x009BDCD1-DDC1`) always sends an existing bar its
current value through +0x14. With the recompute argument true **and
visibleRows !=0**, it first sends range through +0x1C and a pair through
+0x3C, in that order:

| Bar | Range (+0x1C) | Pair (+0x3C) | Current value (+0x14) |
|---|---|---|---|
| Vertical | (0,maxSigned(lineCount-visibleRows,0)) | (that maximum,unsigned(lineCount/visibleRows)+1) | firstVisible |
| Horizontal | (0,maxSigned(longestWidth-visibleWidth,0)) | (that maximum,signed(longestWidth/visibleWidth)+1) | pixel offset W+0x178 |

Horizontal uses longest-width true mode; **visibleRows**, not width,
guards its recompute branch too. Width zero with nonzero rows can therefore
reach signed division by zero (`0x009BDD94-95`). Current values are sent
even if recomputation was skipped, and owner fields are not clamped to
the sent ranges. Bar callback/result behavior is foreign.

ScrollPageUp (`0x009BE063`, T) returns false only if firstVisible is zero;
otherwise subtracts visibleRows or clamps to zero using the native signed
comparison, updates values, invalidates and returns true. ScrollPageDown
(`0x009BE0A9`) first tests signed(lineCount-visibleRows-firstVisible)>0;
if false, returns false. On the usual coherent range the successful path
adds visibleRows, updates values and invalidates. It does not cap that sum
back to lineCount-visibleRows: 10 lines, 4 visible rows, firstVisible 5
becomes 9. The separate fallback sets lineCount-1 when the unsigned
firstVisible comparison fails. Neither scrolling method moves insertion
or creates a selection; Page key navigation is a different path.

DoCursorScroll (`0x009BCCCE`, W) does nothing unless capturing. It obtains
cursor (x,y) relative to the editor from its manager. Signed y<-2 first
tries ScrollLineUp; y>=editorHeight+2 instead tries ScrollLineDown. On a
true return, it invokes primary +0x228 with (x,lineHeight/2,0) or
(x,editorHeight-lineHeight/2,0), respectively; halving is a logical unsigned
shift. Vertical handling takes priority over horizontal. Within the
vertical band, x<-2 / x>=editorWidth+2 invokes primary +0x260/+0x264,
then on true calls +0x228 at (0,y,0)/(editorWidth,y,0). Otherwise no call.
These are routed window operations, not a proportional drag speed or
distance accumulator. The body has no manager-null guard.

## Caret-in-view arithmetic

PutInsertionPositionInView (`0x009BE302-476`, W) first compares unsigned
insertion line L with firstVisible F and visibleRows R. L<F sets F=L;
otherwise L>=F+R sets F=L-R+1. Additions/subtractions wrap at 32 bits;
there is no separate zero-R clamp. A vertical change marks values dirty.

With bit 0x200 it uses the insertion **column** as a byte offset in the
whole original text. If this is before the horizontal byte offset +0x174,
it sets that offset to the column. Otherwise it measures original-text
prefix width to the column using font +0xC0 and subtracts
GetHorizontalScrollPixels. Only a signed difference **greater than**
visibleWidth starts the stepping loop. While byte offset<text length it
adds GetCharLength at that offset and remeasures the scroll pixels, stopping
when caret-relative X is **strictly below** signed(visibleWidth*3)/4.
Multiplication wraps first and division truncates toward zero
(`0x009BE3ED-3FE`); equality continues stepping. The length helper can
overshoot malformed text or read outside the logical slice as described in
the encoding companion. Font/helper nonprogress is not repaired by this
owner. This branch always calls SetScrollBarValues(false), even without a
change, and does not invalidate.

Without 0x200, horizontal correction runs only if a horizontal bar exists
or wrapping bit 2 is clear. It measures the original line prefix to the
insertion column, subtracts horizontal scroll pixels, then adds that signed
relative X to pixel offset +0x178 if X<0, or adds X-visibleWidth if
X>=visibleWidth. X exactly at visibleWidth enters the change path but adds
zero. Values update only for a vertical or horizontal change; wrapped text
without a horizontal bar skips horizontal correction. This body neither
invalidates nor changes selection/preferred-X (`0x009BE416-476`).

## Scrollbar creation and failure boundary

AddScrollBar (`0x009BE477-613`, W) attempts vertical first, horizontal
second. A direction is attempted only if its Boolean argument is nonzero,
its editor enable bit is set (vertical 8, horizontal 4), and its current bar
pointer is null. For each attempted direction it obtains RZ::WCM and calls
manager +0x44 with (0,2,0,100,0) vertically or (1,1,0,100,0)
horizontally, storing the returned pointer. A null manager or null return
invokes the text-interface ClearFlag slot +0x138 with that direction's bit.
The other direction is still considered. No failure is reported: the body
always returns true (`0x009BE60D`).

For each successful bar with an existing direction image, the signed
rectangle-empty predicate `0x007C1870-887` tests right<=left or
bottom<=top. For an empty saved rectangle it obtains image +0x30 and
passes that returned rectangle, image, zero to bar +0x44; otherwise it
passes the saved rectangle, image, zero. It then obtains bar +0x0C and
passes that result to primary window +0x38 (AddChild). There is no
owner-side AddRef, result check, rollback or retry around these calls.
The window/bar/image primitives retain their own ownership/failure contracts.
If either direction was **attempted**, including both failing, the body
calls RecalculateAll, SetScrollBarSizes, SetScrollBarValues(true), in that
order. No attempt means none of those final calls. ClearFlag can itself
recalculate; a failed attempt need not produce only one recalculation.

## Word selection

SelectCurrentWord (`0x009BE6CE-7AE`, W) returns false without view work
when text is empty. Otherwise it tests the original byte at insertion
using the byte-whitespace predicate in the navigation section. There is
no initial end-index clamp, so insertion at text length requires a readable
byte there; out-of-allocation input is unsafe. A whitespace byte with
insertion zero, or with another whitespace byte immediately before it,
leaves insertion/selection unchanged and merely puts insertion in view,
then returns true. A whitespace byte preceded by a nonspace first decrements
insertion and assigns **both** selection endpoints to that index.

It then extends the existing lower endpoint left byte-by-byte until zero
or preceding whitespace, and the existing upper endpoint right byte-by-byte
until text length or current whitespace; each right step also sets insertion
to the new upper endpoint. The nonspace initial path does **not** first
collapse preexisting selection. After scanning it calls
FixInsertionPositionIfBroken, then PutInsertionPositionInView and returns
true (`0x009BE733-7A9`). It does not invalidate or select punctuation as a
separate class. Character/composition repair is a distinct helper contract.

## Page and arrow preferred-X behavior (TE5)

The four vertical key paths in GZOnKeyDown first measure the current
line/column through text +0xF0, obtaining X and an ignored Y. They initialize
preferred-X W+0x170 to that X only when its current word is 0xFFFFFFFF.
**Page Up and Page Down nevertheless pass the newly measured X** to text
+0xE8 for the destination column; Up/Down arrows pass the **stored**
preferred-X instead. These are distinct native argument loads:
Page Up `0x009C0B7C-0B98`, Page Down `0x009C0DC3-0DE5`, Up
`0x009C0EBB-0ED3`, Down `0x009C146C-1484`. No page/arrow successful
path resets the cache. Horizontal/Home/End paths reach the separate cache
reset at `0x009C10F4`; the vertical success exits bypass it.

With editable bit 1 set and single-line bit 0x200 clear, Page Up subtracts
visibleRows from insertion line, clamping to zero with an unsigned
comparison. Page Down uses line+visibleRows when line<lineCount-visibleRows
(wrapping subtraction), otherwise last line. The normal coherent-range
effect is to move at most to last; zero/oversized row counts are not given
an extra normalization. Both obtain the destination index via text +0xF8
and apply preceding-LF correction. Page Up at line zero and Page Down at
last line still collapse/extend a nonempty selection to insertion without
moving line; with empty selection they issue boundary reason 2. Disabled
or single-line page keys are consumed without the arrow forwarding path.
Up/Down arrows use one-line changes under the same editing gates, with
their section-7 forwarding and boundary rules.

Successful vertical movement updates active selection end to insertion;
without Shift it also sets anchor to insertion. It puts insertion in view
and invalidates; Page Down also makes the caret visible, as Page Up/Up do
through their common path (`0x009C0E24-0E32`, `0x009C14AF-14CE`).
Composition and insertion repair owner loops are specified in
[text-edit-adapters.md](text-edit-adapters.md#filtering-and-composition-repair).

## Residuals

The requested TE5 owner paths are specified, including page/arrow cache
arguments, caret-in-view, AddScrollBar, word selection and composition-repair
call/erase ordering. Exact foreign composed/UTF16 conversion, Thai levels,
font/bar/resource implementations remain call boundaries, explicitly parked
in the public question inventory. No whole-owner promotion follows.
