# TextEdit navigation and layout body contracts

Extension of [text-edit.md section 8](text-edit.md#8-layout-font-primitives-and-view).
All rules are original Windows 1.1.641 body contracts. Font metrics and
encoding primitives remain explicit foreign inputs; no implementation or
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

GetHorizontalLineMetrics (`0x009BDB83-DC D0`, W; end address
`0x009BDCD0`) consumes a valid line, unsigned pixel scroll and nonnull
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

The vertical **+1** is native (`0x009BCF31-446`, `0x009BCF8E-49F`).
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

## Residuals

TE5 remains open specifically for Page-key versus arrow preferred-X cache
use/reset, single-line caret-in-view three-quarter-width stepping, complete
AddScrollBar construction/resource negotiation and SelectCurrentWord/
composition-repair helpers. The bodies above do not close those distinct
methods or foreign font/encoding/bar implementations. No whole-owner
promotion follows from this extension.
