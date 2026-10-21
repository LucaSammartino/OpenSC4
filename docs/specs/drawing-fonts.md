# 2D drawing and fonts: the font family (cGZFont, cGZFontSys, cFontCOMDirector)

Spec for the framework's text system: the font catalogue (`cGZFontSys`), the font object (`cGZFont`), its glyph
cache and its COM director. Written by re-drawing4 (2026-10-21) from the Ghidra export (`tools/re_query.py`),
`decomp/` and constants read from `bin/SimCity4.exe`. REA was unavailable (no app open); every numeric rule below
was checked against the instructions at the cited address unless the text says otherwise.

Out of scope: the DX7 driver (re-driver4), base widgets (re-widgets4), the five `nGZDrawSoftRender::cSpanRender*`
blitters (view3d.md §"soft span renderer" records their identity), and `cGZDrawFontRenderer` beyond its identity
(§9). FreeType is linked statically (`FT_Get_Char_Index`, `ft_new_input_stream`); its rasteriser is a library
contract, not part of this spec.

## Outline

| § | Content |
|---|---|
| 1 | Owners, identities, coverage claim |
| 2 | Font object state |
| 3 | Lifetime: create, Init, Shutdown, Release |
| 4 | Attribute setters and getters (clamps) |
| 5 | Size and vertical metrics |
| 6 | Character encodings |
| 7 | Glyph cache and glyph rasters |
| 8 | Measuring and layout |
| 9 | Drawing text; renderer, palettes, surface lock |
| 10 | Antialiasing lookup table |
| 11 | Thai presentation forms |
| 12 | Font catalogue (`cGZFontSys`) |
| 13 | Open questions |

## 1. Owners, identities, coverage claim

| Owner | Identity | Bytes (coverage.py --owners) | This spec |
|---|---|---|---|
| `cFontCOMDirector` | registers 3 classes (below); director ID `0x5AA40BD3` | 220 | complete |
| `cGZFontSys` | class `0x20673960`, object 0x70 bytes, interface asked as `0x287FB697` at OnStart | 8 358 | partial: FN3/FN4 answered (§12); slots 0, 2, 4, 12, 17-19, 21-24 unaudited |
| `cGZFontSys::FontFixedInfo` | catalogue record (§12) | 105 | fields complete (§12: +0 font list, +4 family, +0x18 style, +0x1C kind, +0x20 source type) |
| `cGZFont` | class `0x1AA404AC`, object 0x15C bytes, interface `0x10673960` (QI also answers 1), vtable `0x00AC55D8`, 64 slots | 16 187 | partial: FN1/FN2/FN5/FN6 answered; 64-slot audit not done |
| `cGZFont::CachedBitmapData` | two glyph caches inside the font (§7) | 56 | complete with §7 |
| `cGZDrawFontRenderer` | class `0x08CDA30B`, object 8 bytes from the director factory `0x00895F65`; vtable `0x00AC14BC` | 4 444 | identity only, uncredited |

`cFontCOMDirector` (ctor and factories at `0x00895F4C`/`0x00895F65`, `CreateFontSys`) adds the classes
`0x20673960 → new cGZFontSys (0x70 B)`, `0x1AA404AC → new cGZFont (0x15C B)`, `0x08CDA30B → new 8-byte font
renderer`. `OnStart` creates class `0x20673960` with interface `0x287FB697` and, if that succeeds, registers it with
the framework (framework vtable +0x0C) and releases its own reference.

Coverage claim (precise): exact owners are `cFontCOMDirector` and `cGZFont::CachedBitmapData`; FN1-FN6 are
answered (re-fontanswers4, 2026-10-21) and no longer block a font core. `cGZFont` and `cGZFontSys` stay partial
until their remaining methods are audited: `cGZFontSys` vtable `0x00AC5488` slots 0 `0x00893D2C`, 2 `0x00893DD7`,
4 `0x00894F8D`, 12 `0x00893F22`, 17 `0x00894A3A`, 18 `0x00893E5B`, 19 `0x00893ECB`, 21 `0x00894198`,
22 `0x008944B2`, 23 `0x0089423C`, 24 `0x0089538D` and AddSubstitution (slot 20 `0x0089595D`) are not specified
here; for `cGZFont` the slot tables of §4 and §8 do not yet list every one of the 64 slots. Map proposal: the
`cGZFont*` and `cFontCOMDirector` rows of `tools/coverage-map.tsv` (subsystem "2D drawing and fonts") may name
spec `drawing-fonts.md`; do not credit the whole subsystem.

## 2. Font object state

Offsets in a `cGZFont`; names are ours.

| Off | Name | Meaning / default |
|---|---|---|
| +0x04 | initialised | bool, set by Init |
| +0x08 | refCount | |
| +0x0C | filePath | string |
| +0x20 | familyName | string, read from the face at Init |
| +0x34 | faceKind | 1 + (face has fixed-size flag at face+0x5C != 0) |
| +0x38 | pointSize | slot 10 returns it |
| +0x3C | charHeight | pixels, ≥ 1 |
| +0x40 | styleFlags | bits 0x3C are user styles (0x04 underline, 0x08 strike-through by §7 use); low bits come from the face |
| +0x44 / +0x48 | foreground / background colour | `SetFontColor(fg, bg)`; slots 12/13 return them |
| +0x4C | aaMode | 1 = monochrome, 2 = palette, other non-zero = grey (§7, §9) |
| +0x50 | (read by DrawTextLine, §9) | |
| +0x54 | aaGamma | float, [0, 10] |
| +0x58 | aaSharpness | float, [-1, 1] |
| +0x5C..+0x60 | aaTable | byte vector, empty or 256 entries (§10) |
| +0x68 / +0x6C | scaleX / scaleY | float, [0.01, 1000] |
| +0x70 | advanceScale | float, [0.1, 10] |
| +0x74 | charSpacing | int, added to every advance |
| +0x78 | lineSpacing | int, added to line height and ascent |
| +0x7C | codePage | 0 = system ANSI, 8 = single-byte, 0x10 = UTF-16, else a Windows code page |
| +0x80 | surfaceLockCount | §9 |
| +0x84 | ascent | pixels |
| +0x88 | descentRaw | pixels (§5) |
| +0x8C | renderer | `cIGZFont` renderer object (§9) |
| +0x90 | 6 scratch UTF-16 vectors (12 bytes each) | slots 0..5 used by §6 callers |
| +0xD8 | glyph→source index map | vector of u32, filled by §11 |
| +0xE4 / +0xE8 | defaultChar / its glyph index | substituted for missing characters |
| +0xEC | FreeType holder; +8 = face | |
| +0xF0 | draw records | vector of 0x24-byte records (§9) |
| +0xFC / +0x100 | current palette / palette MRU list | §9 |
| +0x104 | monoCache | `CachedBitmapData` used when aaMode == 1 |
| +0x130 | greyCache | `CachedBitmapData` for every other aaMode |

## 3. Lifetime

- **Create.** Through the director (class `0x1AA404AC`) or `cGZFontSys::FontAcquire` (§12).
- **Init** (slot 3, `0x0089955C`). Does nothing when already initialised or when filePath is empty. Fails
  (returns false) unless the file exists and FreeType opens it. If pointSize is non-zero, SetSize(pointSize) must
  succeed. Then it reads the family name from the face, sets `styleFlags = faceStyle | (styleFlags & 0x3C)`,
  sets faceKind, looks up the glyph index of defaultChar (0 when the lookup reports an error), and is initialised
  iff the family name is non-empty. On success it creates the default renderer (an 8-byte object, AddRef'd) if
  none is set, and builds the antialiasing table (§10). Returns `initialised`.
- **Shutdown** (slot 4, `0x008982E1`). Only when initialised: SetRenderer(null), clear both caches, drop the
  current palette pointer, free every palette in the MRU list, close the face, clear initialised.
- **Release** (slot 2, `0x008972A8`): when refCount > 1 decrement and return it; otherwise FinalRelease
  (`0x00896293`): Shutdown, then the deleting destructor (slot 57, flag 1).
- **Destructor** (`0x0089893F`): Shutdown, free the FreeType holder, both caches, the palette list, the record
  vector, the map vector, the six scratch vectors, the aa table and both strings.

## 4. Attribute setters and getters

| Slot | Method | Rule |
|---|---|---|
| 6 | GetStyle(mask) | `(styleFlags & mask) == mask` |
| 7 | GetStyles | styleFlags |
| 8 | SetStyle(mask, on) | sets or clears `mask & 0x3C` only |
| 11 | SetSize | §5 |
| 14 | SetFontColor(fg, bg) | stores both |
| 15/16 | Get/SetAntialiasing | Set ignores 0 |
| 17/18 | Get/SetAntialiasingGammaCorrection | clamp [0, 10]; on change rebuild §10 |
| 19/20 | Get/SetAntialiasingSharpness | clamp [-1, 1]; on change rebuild §10 |
| 21/22 | Get/SetFilePath | string copy |
| 23/24 | Get/SetCodePage | Get always returns true |
| 25 | IsCodePageAvailable(cp) | §6 probe table |
| 26 | IsCharEncodingAvailable(e) | `e == 2` |
| 29/30 | Get/SetScaling(x, y) | clamp each to [0.01, 1000]; on change clear both caches and re-apply the char size at 72×72 dpi |
| 31/32 | Get/SetAdvanceScaling | Get returns (advanceScale, 1.0); Set clamps [0.1, 10], on change clears both caches |
| 33/34 | Get/SetSpacingAdjustment(char, line) | plain store |
| 53/54 | Get/SetRenderer | §9 |

## 5. Size and vertical metrics

`SetSize(size)` (`0x00897AB1`) fails for size 0 or while either cache holds glyphs (cache fields +0x12C, +0x158
non-zero). Size is clamped to 2048 and stored. Without an open face it returns true. Otherwise it sets the FreeType
char size to `size` at 72×72 dpi with the scaling transform and, with face size metrics A = face+0x7C,
B = face+0x6C, C = face+0x74 (16.16 fixed):

- descentRaw = A / 65536, ascent = (A + B) / 65536, charHeight = (A − C + B) / 65536 (signed integer division),
  charHeight 0 becomes 1;
- for font source types 2 and 3 (§12) charHeight += 1, and += 1 again if it is then > 28.

Returns true iff the char-size call reported no error.

| Slot | Metric | Value |
|---|---|---|
| 35 | GetLineHeight | charHeight + lineSpacing |
| 37 | CharHeight | charHeight |
| 38 | GetSizeAboveBaseline | ascent + lineSpacing |
| 39 | GetSizeBelowBaseline | GetLineHeight − GetSizeAboveBaseline (virtual calls) |

Worked example: A = 0x000A0000, B = 0x00030000, C = 0xFFFE0000, lineSpacing 2 → charHeight = 15, ascent = 13,
line height 17, above 15, below 2.

## 6. Character encodings

Strings arrive as bytes in the font's codePage. `ConvertCharToUnicode` (`0x0089671F`): length 0 → 0; codePage
0x10 → the UTF-16 unit, length 2; a byte < 0x80 → itself, length 1; codePage 0 and byte ≥ 0x80 → one byte through
the system converter, length 1; any other codePage → the multibyte character length for that code page, converted
to one UTF-16 unit. `GetTextAsUnicode` converts a whole string into scratch vector *k* (codePage 0x10: copy only
when it needs Thai substitution, else the input is used as is), then applies §11 when the text contains Thai
characters that need it. Measuring functions temporarily set codePage to 0x10 while they walk the converted text
and restore it afterwards.

`IsCodePageAvailable(cp)` (slot 25, `0x00898A5A`) builds a list of probe characters, then returns true iff every
probe has a glyph in the open face (`0x0089F538` on the face handle at font +0xEC → +8, non-zero = present; the
first absent probe returns false). The table, read from the switch at `0x00898A7E`..`0x00898C05`:

| cp | probes |
|---|---|
| 8, 0x10 | none probed: true at once |
| 0, 1252 | U+0061 |
| 874 | U+0E01, U+0E2C |
| 932 | U+3061, U+30A4 |
| 936 | U+70A7, U+7535 |
| 949 | U+C2B9, U+C9D1 |
| 950 | U+6F54, U+70B5 |
| 1250 | U+0142 |
| 1251 | U+0401 |
| 1253 | U+0370, U+0393 |
| 1254 | U+011E |
| 1255 | U+05D0, U+05E2 |
| 1256 | U+0621, U+0641 |
| 1257 | U+0160 |
| any other | empty list: true (no probe) |

## 7. Glyph cache and glyph rasters

- `GetAppropriateCachedBitmapData` (slot 59): monoCache when aaMode == 1, else greyCache.
- `GetCharData(ch)` (slot 60): look `ch` up in the active cache's hash map; on a miss call `AddCharData(ch, cache)`
  (slot 61). Returns the glyph record or null.
- Glyph record: srcLeft, srcTop, srcRight, srcBottom (ints, a rectangle in the cache's 8-bit source bitmap), then
  xOffset (s16 at +0x10), yOffset (s16 at +0x12, from the line top), advance (s16 at +0x14).
- `AddCharData` (`0x00898FDC`): for ch < 0x80, or any ch on fonts whose source type is not 2/3, the glyph index is
  `FT_Get_Char_Index` (0 on error). On index 0: source types 2/3 with ch ≥ 0x20 try the font's own map
  (`0x00898EB2`); failing that, the default-char glyph (+0xE8) is used when non-zero. The glyph is loaded at
  2 grey levels (aaMode 1) or 256 levels (otherwise). xOffset = horizontal bearing / 64, yOffset =
  (A + B) / 65536 − vertical bearing / 64; advance = (linear or metric advance) / 65536 when advanceScale == 1.0,
  with U+200B and U+FEFF forced to advance 0. A glyph that cannot be loaded gets an empty box of width
  charHeight/2 drawn as a filled block (U+200B/U+FEFF: an empty box, advance 0).
- **CopyGlyphToSourceData** (slot 62, `0x00897748`) appends the raster at the cache's pen x. With underline or
  strike style it widens the cell to max(raster width, advance) and folds a positive xOffset into the cell. It grows
  the source bitmap (slot 63) to width `cellWidth·32 + x` or to the needed height. 2-level rasters are unpacked MSB
  first into bytes 0x00/0xFF. When aaMode ≠ 1 and the aa table is non-empty every byte is mapped through it.
  Underline: rows from `ascent − yOffset + uOffset` (`0x00896618`), strike: from `ascent − charHeight/4 − yOffset`
  (`0x00896636`), each filled with 0xFF for the line thickness, clipped to charHeight; srcBottom grows to include
  them.
- `ReallocateSourceData(cache, w, h)` (`0x0089632D`) only grows: new block `(h + 4)·w` bytes, zeroed, data starts
  at offset 2·w, old rows copied, pitch = w.
- `CachedBitmapData::Clear` (`0x008982BA`) empties the map and the bitmap.

## 8. Measuring and layout

| Slot | Method | Rule |
|---|---|---|
| 36 | CharWidth(ch) | srcRight − srcLeft of the glyph; 0 if uninitialised or no glyph |
| 58 | GetXOffset(ch) | xOffset |
| 40 | GetAdvanceWidth(ch) | advance + charSpacing |
| 48 | GetStringWidth(s, n) | sum of GetAdvanceWidth over each composed character of the converted text |
| 49 | GetStringVisibleWidth(s, n, &len) | trailing whitespace dropped, then GetStringWidth; `len` = remaining length (in source units for multibyte code pages) |
| 42 | CalculateCharsToFitInWidth(s, n, w, &width) | adds one character at a time; stops before the width exceeds w |
| 41 | CalculateWordsToFitInWidth(s, n, w, &width, mode) | §8.1 |
| 43 | CalculateNumberOfLines(s, n, w) | repeatedly takes CalculateWordsToFitInWidth(…, mode 2) and counts lines (w 0 treated as 1) |
| 47 | TextArea(rect) | rect = (0, 0, GetStringWidth, CharHeight) |
| 44 | CalculateWidthOfLines(s, n, height) | §8.2 |
| 45 | CalculateTextArea(s, n, rect, minW, maxW, maxH) | §8.2 |
| 46 | CalculateTextArea(s, n, rect, wrap, fixWidth) | §8.2 |
| 50 | FormatForEllipsis(text, ellipsis, rect, keepAll) | §8.3 |

**8.1 Words to fit.** Text is converted (scratch 2, map vector cleared). Walk words (`UTF16GetWord`): CR or LF ends
the line and is consumed; stop when the word's visible width plus the width so far exceeds w. If nothing fits:
mode 1 takes whole words up to the first CR, LF, NUL or break character; mode 2 takes Thai clusters or single
characters while the prefix width < w, backs off one, and steps past a duplicated map entry. A count > 1 that ends
on a non-space followed by a space takes the space. The result is mapped back through the §11 map, converted to a
source-unit length, and `width` = GetStringWidth of that prefix.

**8.2 Text area.** Empty text: rect.right = rect.left. With wrap and fixWidth: height = CalculateNumberOfLines ×
line height measured against the rect's width; without wrap: right = left + GetStringWidth; bottom = top + line
height. With wrap and no fixWidth: right = left + CalculateWidthOfLines(rect height).

`CalculateWidthOfLines(s, n, height)` (slot 44, `0x008968EB`) returns the narrowest width, in steps of 10 pixels
from 20 to 4000, at which the text wraps into at most maxLines = height / lineHeight lines (unsigned division;
lineHeight 0 is not guarded). Binary search on lo = 2, hi = 400: mid = (lo + hi) >> 1; lines =
CalculateNumberOfLines(s, n, mid·10); if lines > maxLines, lo = mid + 1; otherwise, if lo == hi return lo·10,
else hi = mid; repeat while lo ≤ hi. Leaving the loop (even 4000 does not fit) returns 0. Height below one line
gives maxLines 0 and so 0 for any text that has a line.

`CalculateTextArea(s, n, rect, minW, maxW, maxH)` (slot 45, `0x008973D4`, 6 stack arguments, returns bool) fits
the text into a box no taller than maxH; rect is {left, top, right, bottom} and only right and bottom are written.
Signed compares except where marked:
1. lh = GetLineHeight; maxH = max(maxH, lh).
2. If minW or maxW is −1: aw = CharWidth('a'); minW −1 → 12·aw; maxW −1 → 32·aw; then if maxW ≤ minW,
   maxW = 2·minW (this repair runs only in this branch).
3. w = 1. If the text contains no U+000A (`Find(s, n, 10)` `0x0089738F` = −1): w = GetStringWidth(s, n), and
   a. if w < minW: one line {w, lh} (right = left + w, bottom = top + lh), true;
   b. if w < 2·minW: W2 = CalculateWidthOfLines(s, n, 2·lh); if W2 < maxW (unsigned) and 2·lh ≤ maxH:
      {W2, 2·lh}, true;
   c. if w < maxW: one line {w, lh}, true.
4. For k = w / maxW + 1 while k < maxH / lh (k = 1 when the text has a newline): H = k·lh;
   W = CalculateWidthOfLines(s, n, H); if W ≠ 0, W < maxW (unsigned) and H ≤ maxH: {W, H}, true.
5. Nothing fits: right = left, bottom = top, false.
Worked example: aw 8, lh 16, minW = maxW = −1 (so 96, 256), maxH 100: width 60 → 60 × 16; width 150 → the
two-line width (say 80) → 80 × 32; k never reaches maxH / lh = 6, so at most 5 lines are tried.

**8.3 Ellipsis.** Unless keepAll: if the characters fitting the rect width are fewer than the text, and the
ellipsis width < rect width, truncate to what fits in (width − ellipsis width), drop trailing spaces, append the
ellipsis; otherwise truncate to what fits.

## 9. Drawing text

`DrawTextLine(target, x, y, text, n, fg, bg, clip, pen)` (slot 51, `0x0089A099`), returns false only when the
surface lock fails, else `initialised`:

1. Convert text (scratch 1). In palette mode (aaMode 2): when bg is the sentinel `0x08000000` and +0x50 == 2,
   bg = 0 if fg ≠ 0 else 0xFFFFFFFF; then SetCurrentPalette(fg, bg). Afterwards fg `0x08000000` means the font's
   foreground colour; bg is masked to 24 bits.
2. If a target is given and surfaceLockCount is 0, lock it through the renderer (renderer +0x14); failure returns
   false; success counts the lock and remembers to unlock.
3. If `pen` is given it overrides (x, y). Every character is first loaded (GetCharData), then for each: missing
   glyph → defaultChar's glyph, else skipped. Box: left = x + xOffset, top = y + yOffset + lineSpacing,
   right = left + width, bottom = top + height. A record is emitted when there is no clip rect, or the box overlaps
   the clip strictly, and width and height are non-zero. Record (0x24 bytes): left, top, right, bottom, source
   pointer, source pitch, width, height, character. x += advance + charSpacing for every drawn or skipped-with-
   glyph character.
4. With a target and ≥ 1 record, call renderer +0x1C with the records and either the current palette (palette
   mode) or the fg colour. Write the pen back; unlock if this call locked.

`DrawTextParagraph(target, rect, text, n, align, fg, bg, clip, pen)` (slot 52): lines are taken with
CalculateWordsToFitInWidth(mode 2) against the rect width; x = left (align 0), left + w/2 − lineWidth/2 (1, integer
halves), right − lineWidth (2); trailing CR/LF are not drawn; y advances by line height while y < rect.bottom.

`SetRenderer(r)`: if different, AddRef and attach r (r +0x0C with the font), detach (r +0x10) and Release the old
one. `Lock/UnlockSurfaceForFontUse` count nested locks; only the 0→1 and 1→0 edges call the renderer (+0x14/+0x18).

**Palette** (`SetCurrentPalette(fg, bg)`, `0x0089838C`): reuse the current palette when its (fg, bg) matches; else
search the MRU list and move a hit to the front. A miss builds 128 ARGB entries: 0..7 = bg, then for k = 0..14
eight entries of bg + (fg − bg)·k / 16 per channel (signed integer division, split and composed by the graphic
system, `RZ::GS()` +0x90/+0x8C), 120..127 = fg. Before inserting, a list holding more than 3 palettes drops its
last one.

## 10. Antialiasing lookup table

`BuildAntialiasingLookupArray` (`0x008993BB`, constants at `0x00A80AC8` 2.0, `0x00A90BBC` 1/255, `0x00A84D2C` 0.5,
`0x00A9D8B0` 255.0): when gamma == 1.0 and sharpness == 0.0 the table is emptied (no remap). Otherwise it is
resized to 256 and the grey cache is cleared. k = pow(sharpness + 1, 1 / log10(2)) in double, stored as float:
the routine at `0x009F2160` is the CRT **log10** (x87 path `FLDLG2; FYL2X` at `0x009F221F`; SSE2 path
`0x009FE548` scales the exponent by log10(2) = 0.30102999566 at `0x00AEAE60`), so 1/log10(2) = 3.321928… and
k = (sharpness + 1)^3.321928 (k = 1 at sharpness 0, k = 10 at sharpness 1, k = 0 at sharpness −1). Call
order (`0x0089942A`..`0x00899456`): L(2.0), 1.0 / L, then pow(float(sharpness) + 1.0, that), FSTP to float.
For i = 0..255: v = float(i) · (1/255); if gamma ≠ 1, v = powf(v, gamma); if sharpness ≠ 0: v = min(v, 1),
then v = powf(2v, k)·0.5 for i ≤ 127, else 1 − powf(2(1 − v), k)·0.5; entry = truncate(v · 255) (`__ftol`).

## 11. Thai presentation forms

`GetSubstitutionChar(ch, form)` (`0x0089645A`) maps to Private Use glyphs, otherwise returns ch:

| ch | form 1 | form 2 | form 3 |
|---|---|---|---|
| U+0E0D | F70F | | |
| U+0E31 | F710 | | |
| U+0E34..0E37 | F701..F704 | | |
| U+0E47 | F712 | | |
| U+0E48 | F713 | F70A | F705 |
| U+0E49 | F714 | F70B | F706 |
| U+0E4A | F715 | F70C | F707 |
| U+0E4B | F716 | F70D | F708 |
| U+0E4C | F717 | F70E | F709 |
| U+0E4D | F711 | | |

`MakeSubstitutedString` (`0x00899665`) first splits SARA AM (U+0E33) into NIKHAHIT (U+0E4D) + SARA AA (U+0E32),
recording the source index of each output unit in the map vector, then chooses form 1/2/3 per character from the
Thai character classes of its neighbours, as follows.

`UTF16GetThaiCharType(ch)` (`0x0091169F`, a lazily built map) returns 0 outside U+0E00..U+0E7F and for any
unlisted code point, else:

| class | code points |
|---|---|
| 1 | U+0E01..0E23, 0E25, 0E27..0E2E (consonants) |
| 2 | U+0E40..0E44 |
| 3 | U+0E30, 0E32, 0E33 |
| 4 | U+0E45 |
| 5 | U+0E24, 0E26 |
| 6 / 7 / 8 | U+0E38 / U+0E39 / U+0E3A (below vowels) |
| 9 | U+0E48..0E4B (tone marks) |
| 10 | U+0E4C, 0E4D |
| 11 | U+0E47 |
| 12 | U+0E4E |
| 13 | U+0E34 |
| 14 | U+0E31, 0E36 |
| 15 | U+0E35, 0E37 |
| 16 | U+0E50..0E59 (digits) |
| 17 | U+0E2F, 0E3F, 0E46, 0E4F, 0E5A, 0E5B |

Second loop (`0x0089980B`..`0x0089997A`), over the split string in place; every decision uses the original
characters (prev, cur, next; prev = 0 with class 0 at the start, next = 0 with class 0 at the end). "Tall" means
prev ∈ {U+0E1B, U+0E1D, U+0E1F, U+0E2C}. A one-step carry holds the form value chosen for the previous character
(0 when it was copied unchanged, except as noted in rule 2). For each character, in this order:
1. cur = U+0E0D and class(next) ∈ 6..8: output GetSubstitutionChar(U+0E0D, 1) = U+F70F; carry := 0.
2. class(cur) ∈ 6..7: copied unchanged, but carry := 1 when prev is tall (else 0). Class 8 is copied, carry 0.
3. class(cur) ∈ 9..11 or 13..15: form = 1 if carry = 1 or prev is tall, else 0; then, if class(cur) = 9 or
   cur = U+0E4C, add 2 when class(prev) < 10 (compiled test at `0x0089990E`: the 13..15 and 10..12 range checks
   reduce to "class(prev) < 10"). Form ≠ 0: output GetSubstitutionChar(cur, form); form 0: copy. carry := form.
4. Any other class (0..5, 12, 16, 17): copied unchanged, carry := 0.
Worked examples: U+0E1B U+0E48 (tall consonant, tone mark) → U+0E1B U+F705 (form 3); U+0E01 U+0E48 → U+0E01
U+F70A (form 2); U+0E1B U+0E34 U+0E48 → U+0E1B U+F701 U+F713 (the vowel gets form 1, the carry gives the tone form
1, class 13 ≥ 10 adds nothing); U+0E1B U+0E38 U+0E48 → U+0E1B U+0E38 U+F705 (carry through the below vowel, then
+2).

## 12. Font catalogue (`cGZFontSys`)

- Construction (`0x00895268`): file patterns `*.tt?`, `*.pfb`, `*.mxf`, `*.otf`; codePage = framework code page.
- `Init` (`0x00895E68`) once: font names by ID 5 "Times New Roman", 9 "Arial", 6 "Courier New",
  10 "Lucida Console" (`FontSetName(id, name)` inserts or replaces).
- `GetFontSourceTypeFromFileName` (`0x0089433C`): extension (lower case) `.ttf` → 1, `.ttc` or `.otf` → 4,
  `.pfb` → 2; otherwise sniff the first 32 bytes: contains "%!PS" → 2, starts 00 01 00 00 → 1, starts "ttcf" → 4,
  else 0.
- `AddAllFontsFromDirectory(dir)`: empty dir means `<Windows>\Fonts\`; for each pattern, every matching file is
  passed to AddFont (slot 14); returns the count added (files matching `.PFB` count as well).
- `AddFont(path)`: once per path; relative names fall back to `<Windows>\Fonts\`. The file is opened with FreeType;
  a non-empty family name records FontFixedInfo {family, style flags, kind (2 if fixed-size else 1), source
  type} under the path and adds the lower-case family to the family set.
- System antialiasing settings (vtable `0x00AC5488`; all zero after construction `0x0089529A`..`0x008952A6`):
  slot 5 `SetCodePage(cp)` (`0x00893D7B`) stores +0x20 and returns true; slot 7 `SetDefaultAntialiasing(mode)`
  (`0x00893D87`) stores +0x14; slot 8 `SetRenderer(r)` (`0x00893D91`) AddRefs r, releases the old one, stores +0x24.
  Slot 9 `SetMonochromeSizeRange(min, max, family)` (`0x008954CB`): with family null it stores the system range
  +0x18 = min, +0x1C = max. With a family name: if min < max (signed) it inserts family → {min, max} into the
  per-family range map +0x64 (insert-unique `0x008950A5`: an existing entry for that name is kept, not
  overwritten); otherwise (min ≥ max) it erases the name from the map (`0x00894E0C`). Names are stored as given
  (no lower-casing).
- `FontAcquire(path, size, style, cache)` (`0x00895A1C`): with `cache`, reuse a font already made for the same path,
  size and style. Otherwise create class `0x1AA404AC` (or `new cGZFont`), set path (slot 22), size (slot 11),
  SetStyle(style, on), codePage (+0x20) and renderer (+0x24), then the aaMode (slot 16), unsigned compares
  (`0x00895C07`..`0x00895C53`):
  1. mode = system default +0x14;
  2. if +0x18 ≤ size ≤ +0x1C, mode = 1 (monochrome);
  3. otherwise look the font's GetName (slot 5, the familyName field) up in the map +0x64; if found and
     min ≤ size ≤ max, mode = 1.
  Step 3 runs before Init, so familyName is still the freshly created font's empty name: only a range registered
  under the empty name can apply there (a conforming implementation keys the lookup by that pre-Init name, not by
  the face's family). With +0x18 = +0x1C = 0 the system range holds only size 0. Then Init; failure releases it
  and returns null. With `cache` the font is recorded in the catalogue entry. Returns an AddRef'd font.
- `FontAcquire(familyList, kind, size, style, cache)` (`0x00894ADF`, slot 11, 5 stack arguments):
  1. Split a lower-cased copy of familyList at ',' (`RZParseDelimitedText`, at most 32 names; n = count).
  2. Substitution walk: for each name i from the last to the first, for each entry of the substitution map
     (+0x4C, in key order; key = substitute family, value = vector of alias strings), for each alias: if
     name i IsEqual alias (string slot 10, flag false) and name i, lower-cased, is not in the installed family
     set (+0x28), name i becomes the entry's key, lower-cased, and that entry's alias loop ends; the walk
     continues with the next map entries using the new name.
  3. masked = style with bits 0x3C cleared (user styles removed). best = none, bestScore = 1000.
  4. For every catalogue entry (+0x40, path → FontFixedInfo, in path order) score it:
     - n > 0: find the first p with name p == lower(entry.family) (case-exact after both are lower-cased);
       p = 0 → 0, p > 0 → 2, no match → 0x20;
     - n = 0: 0 if entry.kind (FontFixedInfo +0x1C) == kind, else 4;
     - then style: entry.style == masked → +0; (entry.style & ~masked) == 0 → +8 (subset); else +0x10.
     If score < bestScore (strict) the entry becomes best: ties keep the earlier path.
  5. No catalogue entries → return null. Otherwise, with `cache`, return the best entry's cached font whose
     size (font slot 10) == size and whose recorded style == style (unmasked), AddRef'd; else return
     `FontAcquire(bestPath, size, style, cache)` (slot 10).
  Because every score is below 1000, a non-empty catalogue always yields a candidate even when no name
  matches. Worked example: list "Arial,Tahoma", request style 0x05 (masked 0x01): an "arial" entry with style 0
  scores 0 + 8 = 8; a "tahoma" entry with style 1 scores 2 + 0 = 2 and wins; an unrelated family scores ≥ 0x20.

## 13. Open questions

FN1-FN6 are answered (archive: `docs/specs/questions/answered/drawing-fonts.md`); open entries, if any, are in
`docs/specs/questions/drawing-fonts.md`.
