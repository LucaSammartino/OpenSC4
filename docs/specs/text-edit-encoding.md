# TextEdit byte-encoding boundary

Companion to [text-edit.md section 4](text-edit.md#4-character-input-and-high-level-replacement).
These are the exact native helper contracts used by this editor, not
coverage of a text/encoding owner. All byte comparisons below are unsigned.
Evidence: Windows 1.1.641 instructions, including malformed-input branches;
no clipboard, implementation or runtime observations.

## Length and count primitives (TE1)

GetCharLength (`0x009106A9-73B`) returns zero for byte length zero.
Otherwise selector -2 takes the selector-8 path directly; selector -1 is
first resolved by GetCurrentSystemCodePage (`0x00910014`, foreign service).
Selectors 0 and 16 return one. Selector 8 uses this table:

| First byte | Additional condition | Returned byte length |
|---|---|---:|
| C0-DF | second byte 80-BF | 2 |
| E0-EF | second byte 80-BF | 3 |
| F0-F7 | second byte 80-BF | 4 |
| F8-FB | second byte 80-BF | 5 |
| FC-FD | second byte 80-BF | 6 |
| anything else, or failed second-byte test | none | 1 |

There is **no test that the supplied nonzero length contains the second
byte**, no validation of later continuation bytes and no cap to the supplied
length. A one-byte logical slice beginning E0, with a readable following 80,
returns three. An inaccessible following byte is an unsafe read, not a
defined replacement-character result (`0x009106D5-718`). C0/C1 and the
five/six-byte cases are accepted by this length rule.

Other selectors return two for a lead byte in the table below, otherwise
one (`0x0091071D-738`, classifier `0x0091030F-36C`). No trail-byte or
remaining-length validation is done.

| Numeric selector | Lead-byte range(s), inclusive |
|---:|---|
| 932 | 81-9F or E0-FC |
| 936 | A1-FC |
| 949 | B0-C8 |
| 950 | 81-FE |
| every other selector, including 100 | empty |

GetPreviousCharLength (`0x0091073C-7D7`) takes an end pointer and unsigned
prefix length. Zero length returns zero; -2/-1 resolution and 0/16/8
dispatch agree with the forward helper. For selector 8 it starts at length
one and scans backward through consecutive bytes in 80-BF, adding one for
each; it stops at a different byte or when the supplied prefix is exhausted.
It does not verify a lead byte or its expected count. An all-continuation
prefix of N>0 bytes returns **N+1**. The continuation predicate is
`0x009103C5-3D9`; loop/cap evidence is `0x00910771-790`.

For other selectors it scans the entire prefix from its start. A lead byte
marks the next byte as consumed without reclassifying it. It returns two
only when the scan ends immediately after consuming that following byte;
otherwise one, including a final unmatched lead. This is prefix alignment,
not a test of the single byte before the end (`0x00910794-7D0`).

GetGlyphCountFromString (`0x009107D8-854`) returns zero for zero byte
length. After the same selector resolution, 0/16 return byte length;
selector 8 counts exactly those supplied bytes **outside 80-BF**. It does
not parse or validate the lead's advertised length. Other selectors walk
forward one byte, or two for a lead from the table, and count each step
once; a final unmatched lead still contributes one (`0x00910825-84B`).
These three primitives can disagree on malformed input by design of their
separate bodies. None establishes Unicode scalar or grapheme semantics.

## Pending character input

The editor's accumulation gate is selector **8 or signed >=100**
(`0x009C09D7-9E5`); it does not use the ordinary-motion gate 8/100.
The lead helper `0x009108B4-8EC` accepts C0-FD for selector 8 (or -2),
using `0x009103B0-3C4`; after -1 resolution, selectors signed >=100 use
the lead table above, and all other selectors return false.

On an empty pending string, a recognized lead is saved and returns true
immediately. Otherwise the byte is appended and the editor compares the
pending byte length against **one plus** `0x009108ED-927` evaluated on
the first pending byte. Its selector-8 trailing-count table
(`0x00910128-16E`) is:

| First byte | Expected trailing count |
|---|---:|
| C2-DF | 1 |
| E0-EF | 2 |
| F0-F7 | 3 |
| F8-FB | 4 |
| FC-FD | 5 |
| all others, including C0/C1 | 0 |

Other selectors signed >=100 expect one trailing byte only for the lead
table, otherwise zero (`0x0091036D-383`). The helper supports -2/-1 as
above, but those negative selector values do not enter this editor's
accumulation branch. No continuation/trail validation or timeout is present
in the accumulation path (`0x009C0A42-A93`). Thus C0 is initially saved,
but the next eligible byte completes the pending sequence regardless of
its value; E0 followed by printable 41,42 completes after three bytes.
Control filtering still happens before accumulation, so a rejected control
does not fill or clear a pending sequence. Ready input is offered to the
high-level insertion path unless capturing, and normal completion clears
pending bytes and returns false as specified in section 4.

### UTF16 character adapters (TE5, partial closure)

GetUTF16Character (`0x009115F0`) accepts a source pointer, an in/out byte-count
word and a selector; its result is a 16-bit value. A zero input count returns
zero without reading the source. For selectors other than 16, a first byte
strictly below `0x7F` returns that unsigned byte immediately. **Quirk:** this
fast path leaves the count unchanged, even when its input exceeds one. DEL
(`0x7F`) is not included. Otherwise it requests conversion from the selected
encoding to selector 16 with capacity for one output unit, starting with a
zeroed output word. The count is replaced by ConvertStringEncoding's return
value, and the output word is returned (`0x00911602–38`). This wrapper does
not synthesize a replacement character or check conversion success.

GetPreviousUTF16Character (`0x0091163F`) takes an end pointer and available
byte count. Zero count likewise returns zero without a source read. Unless
the selector is 16, a byte immediately before the end strictly below `0x7F`
returns directly, again leaving the count unchanged. Otherwise ask
GetPreviousCharLength for the span before the end, store that span in the
count, and request conversion of exactly that span to one selector-16 output
unit. Replace the count with the conversion return and return the output word
(`0x00911652–97`). This is a byte-span adapter, not a promise to return a
complete supplementary Unicode scalar. The preconversion count store is
observable when the conversion operation is instrumented.

| Input/action | Required observation |
|---|---|
| Forward count 0 with unreadable source pointer | Result 0; count stays 0; no conversion/source read |
| Forward source starts `41`, selector 8, count 7 | Result `0x0041`; count remains 7; no conversion |
| Backward end preceded by `41`, selector 100, count 9 | Result `0x0041`; count remains 9; no length/conversion call |
| Forward source `7F`, selector 8, count 1 | Conversion is requested; the ASCII shortcut is not used |
| Selector 16, first byte `41` | Conversion is requested despite the byte being below `0x7F` |
| Instrumented backward length result 2, conversion return 0 and untouched output | Count becomes 2 before conversion, then 0; result 0 |

These adapter contracts reduce TE5's foreign boundary but do not specify
ConvertStringEncoding's selector/malformed-input outcomes, Thai character
levels or composed-character lengths. The selector source/service also remains
a foreign contract. No modern Unicode validation is inferred.
