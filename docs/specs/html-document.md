# cHTMLDocument: HTML help/news document parser

Exact owner `cHTMLDocument` (SimCity 4 Deluxe 1.1.641), 70 functions in 008fe6e2..009081ba, 38220 bytes
(67 named methods, plus two `hashtable<cRZString,cRZString>` helpers at 008ffdc8 / 009001e7 and the
this-adjusting QueryInterface thunk 009006da). It turns the text of an HTML page (help, news, tips) into a tree
of `cHTML*` element objects that `cGZWinHTML` lays out and draws. The only creator is
`cGZWinHTML::InternalSetDocument` (call at 008fbe02). Element classes (`cHTMLBODY`, `cHTMLTextSegment`, ...)
are separate owners and out of scope; this spec names only what the document passes to them.

Outline: §1 lifecycle and interfaces · §2 input, tokens, tag table · §3 attribute list · §4 grammar sets and
dispatch · §5 document entry points · §6 head elements · §7 element parsers (body) · §8 style stack and
text segments · §9 downloads and images · §10 per-method remainder.

Address citations are to the 1.1.641 Windows executable. Helper functions not in this owner keep their
`FUN_` address. "Text" below means the document's source string (offset 0x180, a `cRZString`), "position"
the parse cursor (0x194, a byte index into the text).

## 1. Lifecycle and interfaces

**Object layout used by this spec** (offsets from the primary base):

| Offset | Meaning |
|---|---|
| 0x00 / 0x04 | primary vtable 00acd938 (16 slots) / secondary vtable 00acd928 (message target, 4 slots) |
| 0x14 | root element (a `cHTMLBODY`), owned |
| 0x18 | "initialised" byte (set once by Init) |
| 0x20 | constant 1 from the constructor |
| 0x28 | token push-back flag (§2) |
| 0x2c / 0x30 | owner window (the `cGZWinHTML`, AddRef'd) / optional second interface (AddRef'd if non-null) |
| 0x34 | critical section guarding the download table (§9) |
| 0x3c | `RZ::GS()` graphics system pointer |
| 0x40 / 0x54 | proportional font face name (default "arial") / its size (= size-table entry 3) |
| 0x58 / 0x6c | fixed-pitch font face name (default "courier new") / its size (= size-table entry 3) |
| 0xb8 | base URL string, heap-allocated on first SetBaseURL |
| 0xd4 | raw tag text of the current token (between `<` and `>`), then the upper-cased tag name |
| 0xe8 | current token id (§2) |
| 0xec | attribute text of the current tag |
| 0x100 | pending character data (text run) |
| 0x114 | attribute map: upper-cased name -> value (`hash_map<cRZString,cRZString>`, entry count at 0x124) |
| 0x130 | current text style record (0x28 bytes, see §8) |
| 0x144..0x155 | link/state fields reset by Init (§1.2) |
| 0x158 | current table-cell / container context passed to new text segments |
| 0x15c..0x164 | style stack: vector of 0x28-byte style records |
| 0x180 / 0x194 | source text / parse position |
| 0x198 | end sentinel of the download table (`map<cRZString,record>`) |
| 0x1a4 | lazily created image loader interface |
| 0x1a8 | font-size table, 7 x int32 for HTML sizes 1..7; 0x1b4 is its entry 3 (size 4) |

### 1.1 Constructor (00905ac5)

- Installs both vtables, zero-initialises every pointer, count and flag listed above, constructs the strings and
  containers, takes `RZ::GS()`.
- **One-time statics** (guard byte 00b624a0): on the first construction in the process it asks the graphics
  system (GS vtable +0x88, a colour constructor taking r,g,b) for four colours and stores them globally, then builds
  the tag table and grammar sets (§2.3, §4.1):

  | Global | r,g,b | Used as |
  |---|---|---|
  | 00b62494 | 0,0,0 | default text colour (Init stores it at 0x14c) |
  | 00b62498 | 100,32,164 | body constructor argument 2 (visited-link colour slot of `cHTMLBODY`) |
  | 00b62490 | 0,0,255 | body constructor argument 1 (link colour slot) |
  | 00b6249c | 192,0,255 | body constructor argument 3 (active-link colour slot) |

  The colour-slot names are the `cHTMLBODY` constructor's parameters in argument order (link, vlink, alink);
  they are passed unchanged by every body creation in this owner (00905985, 009080a2).
- Calls `SetFontSizeTable(table 00acd4a0, 7)`: the default HTML size table is
  `{8, 10, 12, 14, 18, 24, 36}` for sizes 1..7.
- Sets the proportional face to "arial" and the fixed face to "courier new", then copies size-table entry 3
  (0x1b4) into both face sizes again (0x54, 0x6c): with the default table that is **14**.

### 1.2 Init (009007ca)

`Init(window, extra, baseURL)` runs only once per object (byte 0x18). It stores and AddRefs `window`, stores and
AddRefs `extra` when non-null, calls `SetBaseURL(baseURL)`, then sets: 0x144 = 1, 0x148 = size-table entry 3,
0x14c = default text colour (black), 0x150 = 0, bytes 0x154 and 0x155 = 0. It always returns 1, also on the
second call (which changes nothing).

### 1.3 SetBaseURL (00900302)

Creates the base-URL string on first use (copy of the argument), later assigns into it. Then forwards the same
URL to the owner window's object at window+0xd8 (its vtable +0x4c). Returns 1.

### 1.4 SetFontSizeTable (008feeb8, vtable slot 7)

Copies `min(count, 7)` 32-bit sizes from the given array to the start of the table at 0x1a8 (entries past
`count` keep their old values), then sets both face sizes (0x54, 0x6c) to entry 3 (0x1b4). Example: the
default table gives face size 14; a 3-entry table `{6,7,8}` leaves entry 3 unchanged.

### 1.5 COM surface

- `QueryInterface` (008fe759): IID 1 (`kGZIID_cIGZUnknown`) and IID 0xe0eca23a return the primary base;
  IID 0x452294aa returns the secondary base (this+4, or null when this is null). Each hit AddRefs through
  primary vtable slot 1 and returns true; any other IID returns false without touching the out pointer.
- The secondary vtable's slot 0 (009006da) adjusts `this` by -4 and jumps to QueryInterface.
- Primary vtable map (00acd938): 0 QueryInterface, 1 AddRef (008fe79a), 2 Release (008fe7a1), 3 destructor
  (009010e1), 4..6 (009006b4, 009006bb, 008fee66), 7 SetFontSizeTable, 8 ParseSGMLElements,
  9 ParseSGMLDocumentType, 10 ParseHTMLDocument, 11 ParseTextDocument, 12..15 (009d4714, 008fe89e, 00995e67,
  009006c0). Slots 3..6 and 12..15 are outside this owner's named inventory (gap functions).
- Secondary vtable (00acd928): slot 3 is `DoMessage` (§9).

## 2. Input, tokens and the tag table

### 2.1 Character helpers (shared, not in this owner)

`HTMLIsSpace(text, i)` (0090a929 / 0090a820) and `HTMLAdvance(text, i)` (0090a947) decide whitespace and step
one character. Every scan below uses them; a reimplementation must reuse the same helpers so DBCS steps match.

### 2.2 Token ids

| Id | Meaning |
|---|---|
| 0 | unknown tag, or `<` with no closing `>` |
| 1 | end of input |
| 2..162 | a tag from the table (§2.3) |
| 3 | a run of character data (text token; never in the tag table) |

### 2.3 Tag table (InitTagList 00902d4f, global hash map 00b624a4)

Built once (§1.1): 160 entries mapping an upper-case tag name to its id. Hash = sum of the name's bytes as
signed chars (008fe87e) modulo the bucket count. The full list, id in parentheses:

!DOCTYPE(2) HTML(4) /HTML(5) HEAD(6) /HEAD(7) BODY(8) /BODY(9) TITLE(10) /TITLE(11) BASE(12) STYLE(13)
/STYLE(14) META(15) LINK(16) ISINDEX(17) TT(18) /TT(19) I(20) /I(21) B(22) /B(23) STRIKE(24) /STRIKE(25)
BIG(26) /BIG(27) SMALL(28) /SMALL(29) U(30) /U(31) FONT(32) /FONT(33) EM(34) /EM(35) STRONG(36)
/STRONG(37) DFN(38) /DFN(39) CODE(40) /CODE(41) SAMP(42) /SAMP(43) KBD(44) /KBD(45) VAR(46) /VAR(47)
CITE(48) /CITE(49) ABBR(50) /ABBR(51) ACRONYM(52) /ACRONYM(53) A(54) /A(55) IMG(56) OBJECT(57) /OBJECT(58)
BR(59) SCRIPT(60) /SCRIPT(61) MAP(62) /MAP(63) Q(64) /Q(65) SUB(66) /SUB(67) SUP(68) /SUP(69) SPAN(70)
/SPAN(71) BDO(72) /BDO(73) BLINK(74) /BLINK(75) INPUT(76) SELECT(77) /SELECT(78) OPTION(79) /OPTION(80)
OPTGROUP(81) /OPTGROUP(82) TEXTAREA(83) /TEXTAREA(84) LABEL(85) /LABEL(86) BUTTON(87) /BUTTON(88)
H1(89) /H1(90) H2(91) /H2(92) H3(93) /H3(94) H4(95) /H4(96) H5(97) /H5(98) H6(99) /H6(100) P(101) /P(102)
PRE(103) /PRE(104) UL(105) /UL(106) OL(107) /OL(108) DL(109) /DL(110) DIV(111) /DIV(112) NOSCRIPT(113)
/NOSCRIPT(114) BLOCKQUOTE(115) /BLOCKQUOTE(116) FORM(117) /FORM(118) HR(119) TABLE(120) /TABLE(121)
FIELDSET(122) /FIELDSET(123) ADDRESS(124) /ADDRESS(125) CENTER(126) /CENTER(127) DIR(128) /DIR(129)
MENU(130) /MENU(131) CAPTION(132) /CAPTION(133) COL(134) COLGROUP(135) /COLGROUP(136) THEAD(137)
/THEAD(138) TBODY(139) /TBODY(140) TFOOT(141) /TFOOT(142) TD(143) /TD(144) TH(145) /TH(146) TR(147)
/TR(148) LI(149) /LI(150) DT(151) /DT(152) DD(153) /DD(154) AREA(155) FRAMESET(156) /FRAMESET(157)
FRAME(158) NOFRAMES(159) /NOFRAMES(160) NOBR(161) /NOBR(162)

Closing tags are separate entries named with the leading `/`; there are no closing entries for void elements
(IMG, BR, INPUT, HR, META, LINK, BASE, ISINDEX, COL, AREA, FRAME). Insertion is insert-unique (00900f4c), so
the table holds exactly these 160 names.

### 2.4 ParseCDATA (008ffac2): collect a text run

1. Clear the pending text (0x100).
2. While position < length: find the next `<` from the position.
   - No `<`: append position..end to the pending text, position = length, stop.
   - `<` later than position: append the characters before it, position = the `<`, continue.
   - `<` at position: if at least 4 characters remain and they are `<!--`, skip the comment: position += 4, then
     position = (index of the next `-->` from there) + 3, or length when there is none; continue.
     Otherwise stop (a tag starts here).
3. Return true when the pending text is non-empty.

Comments are therefore invisible: text on both sides of a comment joins into one run. No entity decoding and
no whitespace collapsing happen here (GetStringForEscape, §10, owns entities; StripWhiteSpace is not applied).

### 2.5 GetToken(skipText) (00905cf0)

1. **Push-back**: when flag 0x28 is set, clear it and return with the current token unchanged, unless
   `skipText` is true and the current token is 3 (text): then fall through and read a fresh token.
2. Token = 0. If position >= length: token = 1, return (00905d52).
3. Loop: ParseCDATA. If it found text and `skipText` is false: token = 3, return (the text stays in 0x100).
   If it found no text and position >= length: token = 1, return. Text found with `skipText` true is dropped.
   A `<!--` at the position is skipped as in §2.4. Repeat until the character at the position is `<`.
4. Position += 1. Scan forward with HTMLAdvance to the first `>`. None before the end: return with token 0
   and the position just after the `<` (00905e09).
5. Tag text = characters between `<` and `>`; position = after the `>`. `SplitTag(tag text, attribute text)`.
6. Look the tag name up in the tag table; on a hit the token becomes its id, on a miss it stays 0.
7. `ParseAttributeList` (§3).

`SkipUntilToken(id)` (009065c5) calls `GetToken(false)` until the token equals `id`; it returns false (and
leaves the token as last read) when the position reaches the end first, true on a match.

`SkipDocumentWhitespace` (008fe7b8) advances the position over whitespace characters.

### 2.6 SplitTag (00900138) and StripWhiteSpace (008feb74)

SplitTag: find the first whitespace character in the tag text. If found, the attribute text = everything after
that character and the tag text is truncated at it; otherwise the attribute text is cleared. Then strip the tag
text, upper-case it, and strip the attribute text.

StripWhiteSpace removes leading whitespace (one erase of the whole prefix), then removes trailing whitespace
one character at a time from the end.

Worked example: `<a  href="x.html" >` gives tag text `A`, attribute text `href="x.html"`, token 54.
`<!-- c --><b>` at position 0 gives no text, skips the comment, token 22. `<foo bar>` gives token 0 with
attribute map {BAR: ""}.

## 3. Attribute list (ParseAttributeList 009056da)

Skipped entirely when the token is 2 (`!DOCTYPE`): the attribute text then stays raw for §5.3. Otherwise:

1. Clear the attribute map; strip the attribute text.
2. Scan the attribute text from index 0. Whitespace is skipped one character at a time.
3. **Name**: starts at the first non-space character (any character), then continues while the character is an
   ASCII letter `A-Z`/`a-z` (008fe5b5), `-` or `/`. Digits, `_`, `.` and `:` end a name.
4. Skip whitespace. If the next character is `=`:
   - next is `"`: the value is everything after it up to (not including) the next `"`, or to the end;
     the scan resumes after the closing quote.
   - next is `'`: the same with `'`.
   - otherwise the value starts at the character right after `=` (no whitespace skipping, so `a= b` yields
     a value starting with the space) and ends before the first whitespace that follows at least one
     character; the scan resumes at that whitespace.
5. No `=`: the value is empty and the scan resumes at the non-space character found.
6. The name is upper-cased; the value keeps its case. Insert-unique into the map: **the first occurrence of a
   repeated attribute wins** (00900f4c returns the existing entry and does not overwrite).
7. The attribute text is cleared at the end.

`FindAttribute(name, out, remove)` (0090072c): look `name` up (exact, so callers pass upper-case names). Found:
copy the value to `out`, erase the entry when `remove` is true (009001c9), return 1. Missing: clear `out`,
return 0. Parsers pass `remove = true` for attributes they consume, so what remains afterwards is "unhandled".

`DumpUnhandledAttributes` (008ff7fd): when the map is non-empty it walks it (008ff7e7 per entry); the walk has
no state change visible to the document (diagnostic in the shipping build; HD1).

Worked example: attribute text `HREF=foo.htm target='_x' href="bar" NOWRAP` gives
{HREF: `foo.htm`, TARGET: `_x`, NOWRAP: ``}; the second `href` is dropped.

## 4. Grammar sets and dispatch

### 4.1 BuildFirstSets (008ff1b9)

Built once (§1.1) as `std::set<long>` globals; `AppendSet(dest, src)` (008ff187) inserts every member of `src`
into `dest`; `IsInSet(id, set)` (008fee4a) is membership.

| Set (global) | Members (token ids) |
|---|---|
| FONTSTYLE 00b624b8 | TT 18, I 20, B 22, STRIKE 24, BIG 26, SMALL 28, U 30, FONT 32 |
| PHRASE 00b624c4 | EM 34, STRONG 36, DFN 38, CODE 40, SAMP 42, KBD 44, VAR 46, CITE 48, ABBR 50, ACRONYM 52 |
| SPECIAL 00b624d0 | A 54, IMG 56, OBJECT 57, BR 59, SCRIPT 60, MAP 62, Q 64, SUB 66, SUP 68, SPAN 70, BDO 72, BLINK 74 |
| FORMCTRL 00b624dc | INPUT 76, SELECT 77, TEXTAREA 83, LABEL 85, BUTTON 87 |
| INLINE 00b624e8 | text 3, plus FONTSTYLE, PHRASE, SPECIAL, FORMCTRL |
| HEADING 00b624f4 | H1..H6 (89, 91, 93, 95, 97, 99) |
| LIST 00b62500 | UL 105, OL 107 |
| BLOCK 00b6250c | P 101, PRE 103, DL 109, DIV 111, NOSCRIPT 113, BLOCKQUOTE 115, FORM 117, HR 119, TABLE 120, FIELDSET 122, ADDRESS 124, CENTER 126, plus HEADING and LIST |
| FLOW 00b62518 | INLINE plus BLOCK |

### 4.2 ParseFLOW (008feefa) and ParseINLINE (00907cca)

ParseFLOW(parent): token in INLINE -> ParseINLINE; else in BLOCK -> ParseBLOCK; else return false.

ParseINLINE(parent): token 3 -> `InsertTextSegment(pending text, parent)`, true; FONTSTYLE -> ParseFONTSTYLE;
PHRASE -> ParsePHRASE; SPECIAL -> ParseSPECIAL; FORMCTRL -> ParseFORMCTRL; else false. The order of the tests
is the order above; the sets are disjoint so the order is not observable.

## 5. Document entry points

### 5.1 ParseSGMLElements(text) (00908122, vtable 8)

Copy `text` into the source, ParseCDATA, `GetToken(false)`:
- token 2 (`!DOCTYPE`) -> vtable 9 (ParseSGMLDocumentType);
- token 4 (`HTML`) -> ParseHTMLElements;
- anything else (text, EOF, other tags) -> clear the pending text and call vtable 11 (ParseTextDocument).

Note the first ParseCDATA leaves the position after leading text, so a document that does not start with a
tag is handed to ParseTextDocument with the position past that text; ParseTextDocument restarts from 0 (§5.4).

### 5.2 ParseHTMLDocument (0090818e, vtable 10)

SkipDocumentWhitespace, ParseCDATA, `GetToken(false)`; token 4 -> ParseHTMLElements (its result), else false.

### 5.3 ParseSGMLDocumentType (008fff91, vtable 9)

Takes the first 4 characters of the raw `!DOCTYPE` attribute text, upper-cased. Not `HTML` -> return false
(the document is not parsed). Otherwise it looks for `DTD HTML`, extracts the version digits/dots after it and
strips a trailing `//` pair (the result is a local and has no lasting effect), then returns vtable 10
(ParseHTMLDocument). Example: `<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN">` parses as HTML;
`<!DOCTYPE svg>` produces no tree.

### 5.4 ParseTextDocument (00905985, vtable 11): plain text

Delete the existing root (vtable slot 0 with flag 1), allocate a new `cHTMLBODY(link, vlink, alink colours)`
as root, push the style (00902d28, §8), set byte 0x154 = 1 (preformatted mode, HD2), insert the whole source
text as one text segment under the root, set position = length, pop the style (00900e5f). Returns 1.

### 5.5 ParseHTMLElements (009080a2)

1. Allocate a new `cHTMLBODY` with the three link colours and store it as root (0x14). The previous root is
   **not** released here (unlike §5.4).
2. SkipDocumentWhitespace; if the current token is `HTML` (4), `GetToken(false)`.
3. ParseHEAD (§6), SkipDocumentWhitespace, `GetToken(false)`, ParseBODY (§7).
4. `GetToken(true)`; when it is not `/HTML` (5), set the push-back flag. Returns true.

## 6. Head (ParseHEAD 00906e94)

If the current token is `HEAD` (6), read the next with `GetToken(true)` (text is skipped in the head). Then
loop on the current token, reading the next with `GetToken(true)` after each handled one:

| Token | Action |
|---|---|
| META 15 | ParseMETA |
| LINK 16 | ParseLINK |
| ISINDEX 17 | ignored |
| TITLE 10 | ParseTITLE |
| BASE 12 | ParseBASE |
| STYLE 13 | SkipUntilToken(/STYLE 14) (style sheets are ignored) |
| OBJECT 57 | SkipUntilToken(/OBJECT 58) |
| SCRIPT 60 | ParseSCRIPT |
| /HEAD 7 | return true (consumed) |
| anything else | set push-back, return true (the token is re-read by the caller) |

So `<html><body>` without a head works: BODY is pushed back and ParseHTMLElements' next GetToken returns it.

## 7. Element parsers

Common pattern: a parser allocates its element, hands it to the parent through the parent's vtable +4
(add-child; ownership passes to the parent) and returns true. "Delete" below means the element's vtable slot 0
with flag 1 (deleting destructor). Parsers are called with their token current and read further tokens
themselves.

### 7.1 Head helpers

- **ParseTITLE** (00906476): `GetToken(false)`; when it is text, the title string (0x98) = the pending text.
  Then `GetToken(true)` (normally `/TITLE`, consumed without a check).
- **ParseBASE** (0090092d): `FindAttribute("HREF", remove)`; when present, `SetBaseURL(value)`. Returns 1.
- **ParseSCRIPT** (00906acd): `SkipUntilToken(/SCRIPT 61)`; script bodies are discarded (also in the body,
  through ParseSPECIAL).

### 7.2 Dispatchers

- **ParseFORMCTRL** (00906e59): INPUT -> ParseINPUT; SELECT -> ParseSELECT; TEXTAREA -> SkipUntilToken(/TEXTAREA);
  LABEL -> SkipUntilToken(/LABEL); BUTTON -> SkipUntilToken(/BUTTON); else false. Text inside TEXTAREA,
  LABEL and BUTTON is dropped.
- **ParseSPECIAL** (00907c3e): A -> ParseA; IMG -> ParseIMG; BR -> ParseBR; SCRIPT -> ParseSCRIPT;
  MAP -> ParseMAP; SPAN -> ParseSPAN; BLINK -> ParseBLINK; OBJECT, Q, SUB, SUP, BDO -> SkipUntilToken of the
  matching close tag (58, 65, 67, 69, 73); else false. Skipped elements lose their content
  (`x<sup>2</sup>` renders `x`).

### 7.3 Void and form elements

- **ParseBR** (008ff725): add a new `cHTMLBR` (0x1c bytes) to the parent; attributes are ignored. Returns 1.
- **ParseINPUT** (008ff82e): DumpUnhandledAttributes (every attribute is "unhandled"), then add a new
  `cHTMLINPUT` (0x1c bytes) to the parent. Returns 1.
- **ParseSELECT** (00906aff): create a `cHTMLSELECT`. Loop `GetToken(true)`: OPTION -> ParseOPTION(select);
  OPTGROUP -> SkipUntilToken(/OPTGROUP) (its options are lost); anything else ends the loop. Then, when the
  token is `/SELECT` or SkipUntilToken(/SELECT) succeeds, **and** the select has at least one child, add it to
  the parent; otherwise delete it.
- **ParseOPTION** (009060df): create a `cHTMLOPTION`; `GetToken(false)`. Text -> insert it as a text segment
  into the option and add the option to the select; otherwise delete the option. Finally, when the current
  token is not `/OPTION`, set push-back. Example: `<select><option>One<option>Two</select>` gives two options
  "One" and "Two"; `<select><option></option></select>` adds nothing at all.

### 7.4 DIV (ParseDIV 00906d32, ParseDIVActual 009065fd)

ParseDIV: alignment = the current alignment (0x158); `ALIGN` present -> MakeOutputStyle(value). Then
ParseDIVActual(parent, closeId = /DIV 112, alignment).

ParseDIVActual(parent, closeId, alignment): add a new `cHTMLDIV` (0x28 bytes) to the parent; push the current
alignment onto the alignment stack (vector 0x168..0x170) and make `alignment` current; loop `GetToken(false)`
while the token is in FLOW, calling ParseFLOW(div); when the stopping token is not `closeId`,
SkipUntilToken(closeId) (everything up to the close tag is discarded); pop the alignment stack into 0x158 when
it is non-empty. Returns true.

### 7.5 ParseBLOCK (00907135): block dispatcher

| Token | Handler |
|---|---|
| P 101 | 0090614f (gap function, not in this owner's inventory) |
| PRE 103 | 00906a5d (gap) |
| DL 109 | 009066a3 (gap) |
| DIV 111 | ParseDIV |
| NOSCRIPT 113 | SkipUntilToken(/NOSCRIPT 114): content dropped |
| BLOCKQUOTE 115 | the method exported as `ParseDL` (00906c6b) |
| FORM 117 | the method exported as `ParseP` (00906d9b), §7.6 |
| HR 119 | ParseHR |
| TABLE 120 | ParseTABLE |
| FIELDSET 122, ADDRESS 124, DIR 128, MENU 130 | SkipUntilToken of the close tag (123, 125, 129, 131): content dropped |
| CENTER 126 | 00906d22 (gap, 16 bytes) |
| H1..H6 | 00906754 (gap) |
| UL, OL | 00906f1d (gap) |
| anything else | false |

Naming note: the export's auto-matched names `ParseP` and `ParseDL` do not match the tokens they serve (the
dispatch above and their own close-tag constants 0x76 / BLOCKQUOTE prove it). Implementers should follow the
token, not the name.

### 7.6 FORM (exported as `ParseP`, 00906d9b)

Create the element with constructor 008fed5c (0x28 bytes; the export calls it `cHTMLP`). Loop
`GetToken(false)`:
- SCRIPT -> ParseSCRIPT;
- a nested FORM -> SkipUntilToken(/FORM 118) (the nested form and everything up to the first `/FORM` vanish,
  and that `/FORM` is consumed);
- INLINE -> ParseINLINE(form); BLOCK -> ParseBLOCK(form);
- anything else ends the loop.

When the stopping token is not `/FORM`, SkipUntilToken(/FORM). Then: the form has children -> add it to the
parent; no children -> delete it. Returns true. Example: `<form>a<p>b</p></form>` adds one form with a text
segment and a paragraph; `<form></form>` adds nothing; `<form>a</div>b</form>` keeps `a` and drops `b`
(the `/DIV` stops the loop, the skip discards `b`).

### 7.7 BODY (ParseBODY 00907e42)

Attributes are read into the root element (0x14, the `cHTMLBODY`), each with `remove = true`:

| Attribute | Effect |
|---|---|
| BACKGROUND | only when a base URL exists (0xb8 non-null): resolved against it (`cRZURL::ChangeURL` + `GetFullURL`) and kept as the background URL. Without a base URL it is ignored. |
| BGCOLOR | parsed by `cHTMLShared::SetColor` into a colour that starts as 0xff000000; 008fe5d1 allocates a 4-byte colour and stores its pointer at body+0x2c (the old pointer is overwritten, not freed) |
| LINK / VLINK / ALINK | parsed the same way and converted by the graphics system (GS vtable +0x88, the r,g,b constructor of §1.1) into body+0x30 / +0x34 / +0x38 (helpers 009081d2, 0090824c, 0090820f) |
| TEXT | same conversion into the document's text colour 0x14c; byte 0x155 = 0 |
| BGPROPERTIES | any value: body byte +0x40 = 1 |

Then DumpUnhandledAttributes("ParseBODY", "BODY"). When the background URL is non-empty, the document calls its own
primary vtable slot 16 (+0x40, 009059fc, a gap function: the image request of §9) with (body+0x28, the body's
image-waiter interface, or null when there is no body; URL).

Content: when the current token is BODY 8, `GetToken(false)` first (so ParseBODY also works when entered on the
first content token of a page without `<body>`). Then loop with a result flag starting true:
- BLOCK -> result = ParseBLOCK(root); SCRIPT 60 -> result = ParseSCRIPT; INLINE -> result = ParseINLINE(root);
- end of input (1) -> leave the loop at once;
- any other token (stray close tags, unknown tags 0, HEAD-only tags) -> skipped, result unchanged;
- after each step `GetToken(false)`; leave the loop when result is false.

After the loop, when the token is not `/BODY` 9, set push-back. Return the result (true unless a handler
failed). Order matters: a handler that returns false still has the following token read, and that token is
then pushed back. `</p>` or `</td>` in body content are just skipped.

Example: `<body bgcolor=white text=#ff0000>Hi</body>` sets the body background colour, text colour red
(0x14c), adds one text segment "Hi", stops on `/BODY` without push-back.

### 7.8 Tables (ParseTABLE 00906f3c, ParseTR 009064b0, ParseTD 009061dc, ParseTH 009063e6)

**ParseTABLE(parent).** Create a `cHTMLTABLE` (0x94 bytes). Attributes: BORDER -> `cHTMLShared::SetPixels`
(table+0x28); WIDTH -> `cHTMLTABLE::SetWIDTH`; BGCOLOR -> `cHTMLTABLE::SetBGCOLOR` (sets the table colour
+0x44 and its "set" byte +0x48); CELLPADDING / CELLSPACING -> `cHTMLShared::SetLength` (+0x34 / +0x3c).
DumpUnhandledAttributes("ParseTABLE", "TABLE"). Then a fixed-order grammar, every read with `GetToken(true)`
(text between table parts is dropped):

1. Optional CAPTION 132: SkipUntilToken(/CAPTION 133); the caption is discarded.
2. Any number of COL 134 (ignored, result = true) and COLGROUP 135. COLGROUP (gap 00905ea7): read tokens while
   they are COL, then set push-back on the stopping token **whatever it is**, returning true.
3. Optional THEAD 137 (gap 00906c26): read; while TR 147: ParseTR(table), read; push-back unless the stop is
   `/THEAD` 138. Returns true.
4. Optional TFOOT 141 (gap 00906be1): the same with `/TFOOT` 142.
5. Any number of TBODY 139 or TR 147 (gap 00906b90): on TBODY read once more; then the TR loop as above;
   push-back unless the stop is `/TBODY` 140. A bare TR therefore starts an implicit body.
6. When the token is not `/TABLE` 121: result = SkipUntilToken(/TABLE) (all remaining content up to the first
   `/TABLE` is discarded; nested tables are not counted).

Result true -> add the table to the parent; false (no `/TABLE` before the end of input) -> delete it. Returns
the result. Every helper returns true, so in practice only a missing `/TABLE` deletes the table. The sections
are not reordered: a THEAD after a TR, or TFOOT after a TBODY, falls through to step 6 and is discarded with
everything up to `/TABLE`.

Edge case (exact): an explicit `</colgroup>` is pushed back by step 2 and then fails every later step, so
`<table><colgroup><col></colgroup><tr><td>x</td></tr></table>` yields a table with **no rows** (step 6 skips the
rest). Without the `</colgroup>` close tag the rows are kept.

**ParseTR(table).** Create a `cHTMLTR` (0x3c bytes). When the table's colour is set (+0x48), copy it to the row
(+0x34) and set the row's flag (+0x38). BGCOLOR overrides it (008fe68f: SetColor, default 0xff000000, GS
+0x88 -> row+0x34, flag +0x38 = 1). No unhandled-attribute dump. Loop `GetToken(true)`: TH 145 -> ParseTH(row);
TD 143 -> ParseTD(row); anything else stops (a cell's result is kept as the row result). Push-back unless the
stop is `/TR` 148. When the result is false or the row has no children (child vector +0x1c..+0x20), delete the
row and return false; otherwise add it to the table and append it to the table's row list (table+0x4c), return
true. Empty rows (`<tr></tr>`) disappear.

**ParseTD(row).** Create a `cHTMLTD` (0x3c bytes). Inherit the row colour (+0x34/+0x38) when set. Add the cell to
the row and append it to the row's cell list (row+0x28) **before** reading attributes. Attributes:

| Attribute | Effect |
|---|---|
| VALIGN | upper-cased, exact match: TOP 1, MIDDLE 2, BOTTOM 3, BASELINE 4 -> cell+0x28; other values leave the constructor default |
| WIDTH | `cHTMLShared::SetLength` (cell+0x2c) |
| BGCOLOR | as for the row (cell+0x34, flag +0x38) |
| NOWRAP | `cHTMLTD::SetNOWRAP(value)` |
| ALIGN | MakeOutputStyle (§8.1); **absent means 0 (LEFT)**, not inherited from the row or table |

Push the current alignment (0x158) on the alignment stack (0x168) and make the cell alignment current;
DumpUnhandledAttributes("ParseTD", "TD"). Loop `GetToken(false)` while the token is in FLOW: ParseFLOW(cell).
Push-back unless the stop is `/TD` 144. Pop the alignment stack into 0x158 when non-empty. Always returns true;
the cell is kept even when empty.

**ParseTH(row).** Create a `cHTMLTH` (0x28 bytes) and add it to the row (not to the row's cell list). No
attributes are read (all are left in the map); no alignment push. `GetToken(false)`; while FLOW: ParseFLOW(th),
`GetToken(false)`. Push-back unless the stop is `/TH` 146. Returns true.

Implicit closes: because a cell's loop stops on any non-FLOW token and pushes it back, `<tr><td>a<td>b</tr>`
gives two cells "a" and "b", and `<td>a<tr>` ends the cell and the row (TR is then seen by the TBODY loop).

Example: `<table border=1><tr><td align=right>1</td><td>2</td></tr></table>` -> one table, one row, two cells;
cell 1 has alignment 2 (RIGHT) for its text segment, cell 2 alignment 0.

### 7.9 Lists items and bullets (ParseLI 00905fed, CreateBullet 008ff8ab)

**ParseLI(list, number).** Create a `cHTMLListItem` (0x28 bytes) and add it to the list. Then the marker:
- `number < 0` (the UL caller 00907d6b passes -1): CreateBullet(0); a non-null result is added to the item.
- otherwise (the OL caller 0090689f counts from 1, +1 per LI): the text `Sprintf("%3d. ", number)` is inserted
  as a text segment into the item (`"  1. "`, `" 12. "`, `"123. "`, `"1000. "`).
Next a `cHTMLListItemHangingIndent` (0x28 bytes) is added to the item (a marker child; the content does not go
inside it). `GetToken(false)`; while FLOW: ParseFLOW(item), `GetToken(false)`. Push-back unless the stop is
`/LI` 150. Returns true. A following `<li>` therefore closes the previous item implicitly.

**CreateBullet(style)** returns a new `cHTMLIMG` showing a bullet, or null:
1. Diameter d = (font size 0x148) / 2, unsigned, at least 1. Colour = the text colour 0x14c.
2. Look in the process-wide bullet cache (vector of 16-byte records {d, style, colour, surface} at
   00b62524..00b62528) for a record with equal d, style and colour. A hit with a non-null surface skips to step 5.
3. Otherwise create a drawing surface through the graphics system (GS vtable +0xc); failure returns null.
   Initialise it to width d+4, height d+2 (surface vtable +0xc, pixel-format code 3). Background: the body's
   BGCOLOR (body+0x2c) when the body exists and has one, else grey (192,192,192); fill the surface with it inside
   lock 0x8001. When `style == 0`, inside lock 0x8100 draw a **filled** circle (`RZDrawDrawCircle`) in the box
   (0,0)-(d-1,d-1) with the text colour. Other styles leave the surface blank.
4. Append the new record to the cache (a hit with a null surface also appends a fresh record).
5. Create a `cHTMLIMG` (0x3c bytes), set its alignment (+0x38) to the current alignment 0x158, give it the
   surface (its +0x20 interface, slot 0) and return it.

The cache is never pruned inside this owner. Example: font size 14, black text, white body -> an 11 x 9 surface,
white, with a black filled circle of diameter 7 at the top-left; a second bullet at the same size and colour
reuses that surface.

### 7.10 BLOCKQUOTE (exported as `ParseDL`, 00906c6b) and HR (ParseHR 0090097a)

**BLOCKQUOTE.** Create a container element (0x28 bytes, ctor 008fed0c, vtable 00acd66c). Attributes are
ignored. Loop `GetToken(false)`: SCRIPT -> ParseSCRIPT; BLOCK -> ParseBLOCK(quote); INLINE -> ParseINLINE(quote);
anything else ends the loop. Handler results are ignored. When the stop is not `/BLOCKQUOTE` 116,
SkipUntilToken(/BLOCKQUOTE) (the rest up to it is discarded). Children -> add to the parent; none -> delete.
Returns true. Same shape as FORM (§7.6) except a nested BLOCKQUOTE is parsed normally through ParseBLOCK.

**HR.** Create a `cHTMLHR` (0x2c bytes) and add it to the parent first. Attributes (no unhandled dump):
- ALIGN -> MakeRelativeToTextAlignmentStyle (§8.1) at +0x1c: LEFT 0, RIGHT 1, CENTER and anything else 4
  (the vertical-placement mapping, not MakeOutputStyle);
- SIZE -> `RZConvertToUint32A(value)` at +0x20;
- WIDTH -> when the value contains `%` (first occurrence), set the percent byte +0x28 = 1 and cut the value at
  the `%`; then `RZConvertToUint32A` at +0x24.
Returns true. Example: `<hr width=50% size=2>` -> width 50, percent 1, size 2; `<hr width=300>` -> width 300,
percent 0.

## 8. Alignment and style state

### 8.1 Alignment keywords

Both upper-case the argument in place first, then compare with exact string equality (00418a40).

| Keyword | MakeOutputStyle (008fe8d6) | MakeRelativeToTextAlignmentStyle (008fe92b) |
|---|---|---|
| LEFT | 0 | 0 |
| RIGHT | 2 | 1 |
| CENTER | 1 | 4 (default) |
| TOP | 0 | 2 |
| MIDDLE | 0 | 3 |
| BOTTOM | 0 | 4 |
| anything else | 0 | 4 |

MakeOutputStyle values are the horizontal output-style codes stored as the current alignment (0x158);
MakeRelativeToTextAlignmentStyle gives the vertical placement code for inline objects (images, §10).

### 8.2 Text segments and the style stack

`InsertTextSegment(text, parent)` (008fe6e2): allocate a `cHTMLTextSegment` (0x58 bytes) constructed with
(document, text, current style record 0x130, current alignment 0x158) and add it to the parent through vtable
+4. The text is passed as-is.

Style stack: push (00902d28) copies the current style record (0x130) onto the vector at 0x15c; pop (00900e5f)
does nothing on an empty stack, otherwise restores the last 0x28-byte record into 0x130 and removes it.

## 9. Downloads and images

- **DoMessage** (008fff4e, secondary vtable slot 3; `this` is the secondary base): when the message type
  (message vtable +0x10) is **0x75758483**, read the message's status (+0x38) and payload (+0x50); a non-zero
  status calls ResourceHasBeenDownloaded(payload) on the primary object. That message type returns 1
  (handled) whatever the status; every other type returns 0.
- **LoadFromImageFile(path, outImage)** (008fe71c): on first use create the image loader through
  `GZCOM()` (vtable +0xc) with class id 0xa2ffb5d3 and interface id 0x02ffbb60, stored at 0x1a4; when that
  fails, return without touching `outImage`. Then call loader vtable +0x10 (path, outImage, 1).
- **ResourceHasBeenDownloaded(record)** (008ffbfc), under the document's critical section (0x34):
  1. Find the record's URL (record +8) in the download table (end sentinel 0x198). Not found -> nothing.
  2. When the record's "is a file" byte (+0x2c) is 0, the downloaded bytes are written to a temporary file
     (`cRZFile::MakeTempPathName`) first; otherwise the record's own path is used (HD4).
  3. LoadFromImageFile(path, &entry.image (+0x24)); remove the temporary file if one was made.
  4. When the image is non-null, call slot 0 of every waiter in the entry's list (+0x28..+0x2c) with the
     image, then clear the list. Entries stay in the table (later requests for the same URL reuse it).
  Unlock.

## 10. Per-method remainder

Specified above (complete): constructor, Init, SetBaseURL, QueryInterface (+ thunk 009006da),
SetFontSizeTable, InitTagList, BuildFirstSets, AppendSet, IsInSet,
ParseCDATA, GetToken, SkipUntilToken, SkipDocumentWhitespace, SplitTag, StripWhiteSpace,
ParseAttributeList, FindAttribute, DumpUnhandledAttributes (HD1), ParseFLOW, ParseINLINE,
ParseSGMLElements, ParseHTMLDocument, ParseSGMLDocumentType, ParseTextDocument, ParseHTMLElements,
ParseHEAD, InsertTextSegment, ParseTITLE, ParseBASE, ParseSCRIPT, ParseFORMCTRL, ParseSPECIAL,
ParseBR, ParseINPUT, ParseSELECT, ParseOPTION, ParseDIV, ParseDIVActual, MakeOutputStyle,
MakeRelativeToTextAlignmentStyle, ParseBLOCK, ParseP (FORM), DoMessage, LoadFromImageFile,
ResourceHasBeenDownloaded (HD4 open on the temp-file source).

Round 2: ParseBODY (§7.7), ParseTABLE, ParseTR, ParseTD, ParseTH (§7.8), ParseLI, CreateBullet (§7.9),
ParseDL (BLOCKQUOTE), ParseHR (§7.10).

Remaining (not yet specified; whole-owner credit withheld): ParseMETA, ParseLINK, ParseFONTSTYLE, ParsePHRASE, ParseA,
ParseIMG, ParseMAP, ParseAREA, ParseSPAN, ParseBLINK, CreateBullet, GetStringForEscape (entity table,
6918 bytes), the two hashtable helpers (008ffdc8
clear, 009001e7 destructor). Gap functions reached from this owner but outside its inventory (P 0090614f, PRE
00906a5d, DL 009066a3, headings 00906754, lists 00906f1d, CENTER 00906d22, style push/pop, tag lookup
008fead9) are not counted toward the owner.
