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

### UTF16 character adapters (TE5)

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

The conversion and composition rules below complete these helper contracts.
The system code-page service and Windows conversion tables are environment
inputs; no modern Unicode validation is inferred.

## Conversion (TE5)

ConvertStringEncoding (`0x00910E18`) takes source, source count, source
selector, destination, an in/out destination-capacity word and destination
selector. Counts are bytes except that **selector 16 counts 16-bit units**.
The return is source units consumed; the output word is destination units
produced. No branch appends a terminator. Zero source count stores output
count zero and returns zero before resolving selectors or reading buffers.
Resolve -2 to 8 and -1 through GetCurrentSystemCodePage independently for
each selector. If a non-16 source consists entirely of bytes <= `0x7F`,
replace its effective selector with zero (`0x00910E32–81`). Embedded NUL
is just another counted byte.

| Effective selectors / condition | Required behavior |
|---|---|
| Equal selectors, or source 0 and destination other than 16, or non-16 source and destination 0 | Raw copy of min(source count, capacity), doubled in bytes only for destination 16. Return/store that count. **Aliased source/destination skips both copy and capacity limiting**, returning/storing the full source count (`00911323–51`). |
| 16 to 0 | Copy the low byte of each of min(count, capacity) source words; return/store that count. This shrink path precedes the alias check (`00910EAF–DA`). |
| Other conversions with identical source/destination pointer | Return zero, **leave the capacity word unchanged**; no conversion (`00910EA1–AD`). |
| 0 to 16 | Zero-extend min(count, capacity) bytes into words; return/store that count (`00911258–89`). |
| 8 to 16 | Decode the valid prefix as described below. |
| 16 to 8 | Encode individual words as described below. |
| Other non-16 selector to 16 | MultiByteToWideChar(source selector, flags 0, explicit source count, destination, capacity). API zero becomes return/output zero; otherwise output is API result and return is full source count (`009112CA–DF`, `0091130A–16`). |
| 16 to other nonzero/non-8 selector | WideCharToMultiByte(destination selector, flags 0, explicit word count, destination, capacity, default byte `20`, used-default output ignored). Same return/output rule (`009112E1–11316`). |
| Neither selector 16, exactly one selector 8 | Two-stage conversion through a temporary selector-16 buffer (`00911292–C0`, `00910B3C`). |
| Remaining distinct nonzero non-16 selectors | Return zero and store output count zero (`00911318–1E`). |

The two-stage buffer reserves `2*sourceCount+4` bytes (32-bit arithmetic),
using its 1024-byte stack buffer when that value is <=1024, heap storage
otherwise. The same numeric value is passed as selector-16 output capacity.
First convert the whole source to 16; require consumed count == source
count. Then convert the resulting word count to the destination; require
consumed count == that intermediate count. On either mismatch return/store
zero; **already written destination bytes are not rolled back**. On success
return the original source count and keep the second output count. Heap
storage is freed on both normal exits (`00910B48–CF`). Allocation failure
is not mapped to a conversion replacement/error byte.

### Selector 8 decoder and encoder quirks

The decoder stops before the first invalid or incomplete sequence or when
the destination is full. It preserves prior output, stores words written
and returns bytes consumed. It emits no replacement. Continuations must
all be `80–BF`; accepted starts/extra constraints are:

| Start | Byte length | Extra constraint |
|---|---:|---|
| 00–7F | 1 | None |
| C2–DF | 2 | None |
| E0–EF | 3 | E0 requires second byte >=A0 |
| F0–F7 | 4 | F0 requires second byte >=90 |
| F8–FB | 5 | F8 requires second byte >=88 |
| FC–FD | 6 | FC requires second byte >=84 |

Others stop decoding. Three-byte forms allow surrogate code values; there
is no F4 upper limit. Every accepted sequence emits **one low 16-bit word**,
including 4–6-byte forms: combine the last three six-bit payloads for those
forms and truncate to 16 bits. In the four-byte case the second byte is
used without stripping `80`, which has the same low-16 result after its
12-bit shift. No surrogate pair is synthesized. Evidence: bounds and
continuation checks `00910F2F–1110C`, word store `0091112C`, counters
`0091113A–47`.

The encoder reads each source word independently: 00–7F uses one byte,
0080–07FF two, 0800–FFFF three, including lone surrogates. Standard leading
and continuation bit patterns are used. It never joins surrogate pairs.
**Quirk:** the source pointer advances before testing destination space;
the first word that does not fit is counted as consumed although no bytes
from it are written. Output count excludes it. With zero capacity and a
nonempty input it therefore returns one and stores zero. With exactly
enough space for a prefix it may similarly count the next unencoded word
(`00911177–C7`, `0091123C–51`).

Windows code-page mapping/default substitutions and API failure are inputs
to the explicit calls above, not a portable promise of a particular locale
or replacement character. These boundaries do not leave the editor's
argument/count/error handling unspecified.

## Thai composition and composed lengths (TE5)

UTF16GetThaiCharType (`0091169F`) returns the following integer class;
everything else is class 0. Values/range endpoints are instruction-verified
at `009116F7–00911A2A` (inclusive ranges below).

| Class | Code values (hex) |
|---:|---|
| 1 | 0E01–0E23, 0E25, 0E27–0E2E |
| 2 | 0E40–0E44 |
| 3 | 0E30, 0E32, 0E33 |
| 4 | 0E45 |
| 5 | 0E24, 0E26 |
| 6 / 7 / 8 | 0E38 / 0E39 / 0E3A |
| 9 | 0E48–0E4B |
| 10 | 0E4C, 0E4D |
| 11 / 12 / 13 | 0E47 / 0E4E / 0E34 |
| 14 / 15 | 0E31, 0E36 / 0E35, 0E37 |
| 16 | 0E50–0E59 |
| 17 | 0E2F, 0E3F, 0E46, 0E4F, 0E5A, 0E5B |

UTF16GetThaiCharLevel(current, previous, following), `00911A40`, uses
these classes. TextEdit's repair caller always supplies following=0.

| Current | Required level |
|---|---|
| Classes 0–5, 16, 17 | 4, except current 0E33 requires previous class 1 or 9; otherwise 0 |
| Classes 6–8 | 5 after previous class 1; otherwise 0 |
| Class 9 | 2 after previous class 13–15, or class 1 when following=0E33; 3 after previous class 1 (other following), 6 or 7; otherwise 0 |
| Classes 10–11 | 3 after previous class 1, 6 or 7; 2 after previous class 13–15; otherwise 0 |
| Classes 12–15 | 3 after previous class 1; otherwise 0 |

The function's unreachable-for-this-classifier default returns 1. Level
arithmetic and context comparisons: `00911A66–B19`. Thus repair erases an
isolated Thai decoration, but accepts ordinary non-Thai code values.

The decoration predicate (`00911B3E`) recognizes exactly 0E31, 0E34–0E3A,
0E47–0E4E; **0E33 is not a decoration**. UTF16ThaiCharGroupLength
(`00911CDE`) returns 0 for count 0, 1 for count 1; otherwise inspect the
second word. If it is 0E33, return 2; if not a decoration, return 1. For
a decorated second word return 2 when count <3 or the third word is neither
0E33 nor a decoration; return 3 if third is 0E33. If third is a decoration,
return 4 when fourth is 0E33, otherwise 3. **Quirk:** this last fourth-word
read is unconditional even when count is only 3 (`00911D09–38`). Callers
must provide readable storage; the function is not bounded by its count.

GetComposedCharLength (`00912588`) returns zero at zero count. Selector 8
returns 1 immediately for a first byte <=7F. Otherwise convert at most
three output words to 16. If conversion consumes anything and its first
word is in 0E00–0E7F, apply the group rule and map its word count back to a
source byte length with LengthFromGlyphCount; otherwise use GetCharLength.
Selector 16 returns 1 for a first word outside that Thai block, otherwise
uses the group rule directly; other selectors use GetCharLength. No -1/-2
resolution occurs before these selector tests (`0091259A–12622`). The
three-word temporary makes the fourth-word quirk above a possible read
outside initialized temporary contents; no deterministic four-character
group is promised from that case.

GetPreviousComposedCharLength (`00912632`) returns zero at zero count.
Selector 8 returns 1 for an immediately preceding byte <=7F; otherwise
walk backward by GetPreviousCharLength, converting each span to one word,
until a non-decoration is included or the prefix count is exhausted.
Return the traversed byte count. Selector 16 performs the analogous word
walk and returns words traversed, with an immediate 1 for preceding word
<=7F. Other selectors delegate to GetPreviousCharLength. **Quirk:** 0E33
stops the backward walk; it is not grouped with a preceding consonant as
it can be in the forward direction. The byte walk does not test conversion
success: its word output aliases the high half of the original count
argument, initially that count's high 16 bits, then retaining the last
converted value if a later conversion writes nothing (`00912664–8D`).
Malformed/nonprogressing spans and exhausted readable storage have no
general recovery guarantee.

### Conversion/composition conformance vectors

| Input/action | Required observation |
|---|---|
| Same buffer, selector 16 to 16, count 5, capacity 1 | Return/output 5; no copy |
| Same buffer, non-ASCII selector 8 to 16 | Return 0; capacity unchanged |
| 8 to 16: `41 C0 AF`, capacity 3 | Output 0041; return 1, output count 1; no replacement |
| 8 to 16: `ED A0 80`, capacity 1 | Output D800; return 3, count 1 |
| 8 to 16: `F0 9F 98 80`, capacity 1 | Output F600; return 4, count 1 (no surrogate pair) |
| 8 to 16: `F8 88 80 80 80`, capacity 1 | Output 0000; return 5, count 1 |
| 16 to 8: [0041, 0800], capacity 1 | Byte 41; return 2, output count 1 |
| 16 to 8: [D800], capacity 3 | Bytes ED A0 80; return 1, count 3 |
| Two-stage conversion whose second stage consumes only a prefix | Return/output count 0; preserve any prefix bytes already written |
| Level(0E38,0041,0) / Level(0E38,0E01,0) | 0 / 5; repair drops/retains the mark respectively |
| Level(0E48,0E01,0) / Level(0E48,0E01,0E33) | 3 / 2 |
| Forward selector-16 [0E01,0E48,0E34,0E33], reported count 3, all four readable | Return 4 despite count 3 |
| Selector-16 [0E01,0E33], forward/backward | Forward length 2; backward from end length 1 |
| Selector 8: base 0E01 followed by decoration 0E48 | Forward/backward composed length 6 bytes |
