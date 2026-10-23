# 2D drawing and fonts: the font family (cGZFont, cGZFontSys, cFontCOMDirector)

Spec for the framework's text system: the font catalogue (`cGZFontSys`), the font object (`cGZFont`), its glyph
cache and its COM director. Written by re-drawing4 (2026-10-21) from the Ghidra export (`tools/re_query.py`),
`decomp/` and constants read from `bin/SimCity4.exe`. REA was unavailable (no app open); every numeric rule below
was checked against the instructions at the cited address unless the text says otherwise.

Out of scope: the DX7 driver (re-driver4), base widgets (re-widgets4), the five `nGZDrawSoftRender::cSpanRender*`
blitters (view3d.md §"soft span renderer" records their identity), and the full `cGZDrawFontRenderer` cache contract
(§15 is a summary). FreeType is linked statically (`FT_Get_Char_Index`, `ft_new_input_stream`); its rasteriser is a library
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
| 14 | MXF font files |
| 15 | Glyph-cache holder (summary; FN23) |

## 1. Owners, identities, coverage claim

| Owner | Identity | Bytes (coverage.py --owners) | This spec |
|---|---|---|---|
| `cFontCOMDirector` | registers 3 classes (below); director ID `0x5AA40BD3` | 220 | complete |
| `cGZFontSys` | class `0x20673960`, object 0x70 bytes, interface asked as `0x287FB697` at OnStart | 8 358 | complete: all 25 slots of vtable `0x00AC5488` (§12.1 inventory) |
| `cGZFontSys::FontFixedInfo` | catalogue record (§12) | 105 | fields complete (§12: +0 font list, +4 family, +0x18 style, +0x1C kind, +0x20 source type) |
| `cGZFont` | class `0x1AA404AC`, object 0x15C bytes, interface `0x10673960` (QI also answers 1), vtable `0x00AC55D8`, 64 slots | 16 187 | complete: all 64 slots (§4.1 inventory) |
| `cGZFont::CachedBitmapData` | two glyph caches inside the font (§7) | 56 | complete with §7 |
| default font renderer (inherited label `cGZDrawFontRenderer`) | class `0x08CDA30B`, object 8 bytes from director factory `0x00895F65`; corrected vtable **`0x00AC55B4`** | inherited 4 444, unaudited attribution | this spec remains identity only; bounded behavior in [font-renderer.md](font-renderer.md) |

Identity correction (re-font-renderer4, 2026-10-23): constructor `0x00897722` writes `0x00AC55B4`;
QI `0x0089620D` accepts `0xA8BC2A75` and 1. The larger `0x00AC14BC` holder's constructor
`0x00829F70` requests that class/interface into member +`0x43C`; it is not the 8-byte object.
See [font-renderer.md §1](font-renderer.md#1-identity-ownership-and-exact-body-inventory) for the
factory, QI, pointer-view and byte-attribution audit. Other font-owner claims and exclusions here stand.

`cFontCOMDirector` (ctor and factories at `0x00895F4C`/`0x00895F65`, `CreateFontSys`) adds the classes
`0x20673960 → new cGZFontSys (0x70 B)`, `0x1AA404AC → new cGZFont (0x15C B)`, `0x08CDA30B → new 8-byte font
renderer`. `OnStart` creates class `0x20673960` with interface `0x287FB697` and, if that succeeds, registers it with
the framework (framework vtable +0x0C) and releases its own reference.

Coverage claim (precise): exact owners are `cFontCOMDirector`, `cGZFont::CachedBitmapData`,
`cGZFontSys::FontFixedInfo`, `cGZFont` and `cGZFontSys`. FN1-FN6 are answered (re-fontanswers4); the owner audit
(re-fontowners4, 2026-10-21) read every vtable slot from `bin/SimCity4.exe`: §4.1 lists all 64 `cGZFont` slots and
§12.1 all 25 `cGZFontSys` slots, each with the section that specifies it. Remainder for these five owners: empty.
Non-virtual helpers reached only from those slots (catalogue/map node helpers, the deleting-destructor bodies, the
string converters of §6, the palette builder of §9) are specified through their callers. Out of scope and
uncredited: `cGZDrawFontRenderer` (its 4 444 bytes stay a separate owner). Map proposal: the `cGZFont*`,
`cGZFontSys*` and `cFontCOMDirector` rows of `tools/coverage-map.tsv` (subsystem "2D drawing and fonts") may name
spec `drawing-fonts.md`; do not credit `cGZDrawFontRenderer` or the rest of the subsystem.

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

**Constructor defaults** (FN11; the class factory `0x00895F4C` allocates the object and runs the constructor
`0x00899AAB`): initialised false, refCount 0, both strings empty, faceKind 0, **pointSize 16, charHeight 16**,
styleFlags 0, foreground and background colour 0, **aaMode 1** (monochrome), **+0x50 = 2**, **aaGamma 1.0,
aaSharpness 0.0**, aaTable empty, scaleX = scaleY = 1.0, advanceScale 1.0, charSpacing 0, lineSpacing 0,
codePage 0, surfaceLockCount 0, ascent 0, descentRaw 0, renderer null, the six scratch vectors and the map vector
empty, **defaultChar 0x20** (space) with glyph index 0, draw records empty, current palette null, MRU list empty,
both caches empty. A FreeType holder (0x34 bytes, `0x00897FED`: face null, source type 0 at +0x24, u16 load flags
0x000A at +0x20, an empty name map at +0x28) is created and stored at +0xEC (null if the allocation fails).
Note pointSize 16 is non-zero, so Init always calls SetSize(16) unless a size was set before.

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

### 4.1 `cGZFont` slot inventory (vtable `0x00AC55D8`, 64 slots)

Every slot, read from the vtable in `bin/SimCity4.exe`. "Shared" marks a one-instruction accessor whose body the
linker shares with other classes (the address is not a font-specific function). Families: **A** plain field
getters, **B** setters/getters with rules in §4, **M** metrics (§5, §8), **D** drawing (§9), **C** glyph cache (§7).

| Slot | Address | Method | Family | Rule / spec |
|---|---|---|---|---|
| 0 | `0x0089626E` | QueryInterface(id, &out) | — | id `0x10673960` or 1 → this, AddRef, true; else false, out untouched |
| 1 | `0x0094C4B1` | AddRef (shared) | — | refCount (+0x08) += 1, returns it |
| 2 | `0x008972A8` | Release | — | §3 |
| 3 | `0x0089955C` | Init | — | §3 |
| 4 | `0x008982E1` | Shutdown | — | §3 |
| 5 | `0x008989E5` | GetName(&out) | A | out ← familyName (string assign) |
| 6 | `0x00898A00` | GetStyle(mask) | B | §4 |
| 7 | `0x009DB37A` | GetStyles (shared) | A | styleFlags (+0x40) |
| 8 | `0x008962A9` | SetStyle(mask, on) | B | §4 |
| 9 | `0x009DB800` | GetFaceKind (shared) | A | faceKind (+0x34): 1 scalable, 2 fixed-size, set by Init |
| 10 | `0x00999165` | GetSize (shared) | A | pointSize (+0x38) |
| 11 | `0x00897AB1` | SetSize | M | §5 |
| 12 | `0x0097CE34` | GetFontColor (shared) | A | foreground (+0x44) |
| 13 | `0x0099B7EE` | GetFontBackgroundColor (shared) | A | background (+0x48) |
| 14 | `0x00898A13` | SetFontColor(fg, bg) | B | §4 |
| 15 / 16 | `0x0099BE4C` / `0x00896654` | Get/SetAntialiasing | B | §4 |
| 17 / 18 | `0x00896662` / `0x00899987` | Get/SetAntialiasingGammaCorrection | B | §4, §10 |
| 19 / 20 | `0x00896666` / `0x008999CE` | Get/SetAntialiasingSharpness | B | §4, §10 |
| 21 / 22 | `0x00898A24` / `0x00898A3F` | Get/SetFilePath | B | §4 |
| 23 / 24 | `0x0089666A` / `0x00896678` | Get/SetCodePage(&cp / cp) | B | §4 (both return true) |
| 25 | `0x00898A5A` | IsCodePageAvailable | B | §6 |
| 26 | `0x00896684` | IsCharEncodingAvailable | B | §4 |
| 27 | `0x00899CA7` | IsCharAvailable(s, n) | C | below |
| 28 | `0x00899D2A` | PreloadChars(s, n) | C | below |
| 29 / 30 | `0x00896692` / `0x0089854E` | Get/SetScaling | B | §4 |
| 31 / 32 | `0x008966A7` / `0x0089865F` | Get/SetAdvanceScaling | B | §4 |
| 33 / 34 | `0x008966BB` / `0x008966D0` | Get/SetSpacingAdjustment | B | §4 |
| 35 | `0x008966EB` | GetLineHeight | M | §5 |
| 36 | `0x008972BD` | CharWidth(ch) | M | §8 |
| 37 | `0x008966F2` | CharHeight(a, b) | M | §5; both stack arguments are ignored (returns charHeight) |
| 38 / 39 | `0x008966F8` / `0x00896702` | GetSizeAbove/BelowBaseline | M | §5 |
| 40 | `0x00897348` | GetAdvanceWidth(ch) | M | §8 |
| 41 | `0x00899D81` | CalculateWordsToFitInWidth | M | §8.1 |
| 42 | `0x00896808` | CalculateCharsToFitInWidth | M | §8 |
| 43 | `0x00896886` | CalculateNumberOfLines | M | §8 |
| 44 | `0x008968EB` | CalculateWidthOfLines | M | §8.2 |
| 45 | `0x008973D4` | CalculateTextArea (6 args) | M | §8.2 |
| 46 | `0x00896957` | CalculateTextArea(s, n, rect, wrap, fixWidth) | M | §8.2 table |
| 47 | `0x008969F2` | TextArea(s, n, rect) | M | rect = (0, 0, GetStringWidth(s, n), CharHeight) |
| 48 / 49 | `0x00899F71` / `0x00899FF4` | GetStringWidth / GetStringVisibleWidth | M | §8 |
| 50 | `0x00896A2D` | FormatForEllipsis | M | §8.3 |
| 51 / 52 | `0x0089A099` / `0x0089A35B` | DrawTextLine / DrawTextParagraph | D | §9 |
| 53 | `0x009C8F69` | GetRenderer (shared) | D | renderer (+0x8C), not AddRef'd |
| 54 | `0x00896B45` | SetRenderer | D | §9 |
| 55 / 56 | `0x00896B8E` / `0x00896BC2` | Lock/UnlockSurfaceForFontUse | D | §9 |
| 57 | `0x00898FC0` | deleting destructor(flags) | — | destructor §3; flag bit 0 frees the 0x15C bytes |
| 58 | `0x00897304` | GetXOffset(ch) | M | §8 |
| 59 | `0x008962C4` | GetAppropriateCachedBitmapData | C | §7 |
| 60 / 61 | `0x0089834C` / `0x00898FDC` | GetCharData / AddCharData | C | §7 |
| 62 / 63 | `0x00897748` / `0x0089632D` | CopyGlyphToSourceData / ReallocateSourceData | C | §7 |

- **IsCharAvailable(s, n)** (slot 27): n = 0 → true. Otherwise the text is converted (§6, scratch 0, using the
  current codePage), and true iff every converted UTF-16 unit has a glyph in the open face (the same lookup as
  IsCodePageAvailable, §6; the first absent unit returns false). codePage is set to 0x10 during the walk and
  restored. It does not test `initialised`: callers must ask an initialised font (the original dereferences the
  face handle).
- **PreloadChars(s, n)** (slot 28): when initialised and n ≠ 0, converts the text (§6, scratch 0) and calls
  GetCharData (slot 60) on every converted unit, filling the active cache. Returns `initialised`.

## 5. Size and vertical metrics

`SetSize(size)` (`0x00897AB1`) fails for size 0 or while either cache holds glyphs (cache fields +0x12C, +0x158
non-zero). Size is clamped to 2048 and stored. Without an open face it returns true. Otherwise it sets the FreeType
char size to `size` at 72×72 dpi with the scaling transform and, with face size metrics A = face+0x7C,
B = face+0x6C, C = face+0x74 (16.16 fixed):

- descentRaw = A / 65536, ascent = (A + B) / 65536, charHeight = (A − C + B) / 65536 (signed integer division),
  charHeight 0 becomes 1;
- for font source types 2 and 3 (§12) charHeight += 1, and += 1 again if it is then > 28.
  Both tests are unsigned: only a quotient of exactly 0 becomes 1 (a negative quotient is stored as is).

**Source type of a font** (FN10). It is the holder's +0x24, set when the face is opened (Init → `0x00898CD9`),
from the file **contents**, not from the file name and not from the catalogue. The probe (`0x008A1BD8`) tries in
this order: an sfnt header (first 4 bytes 00 01 00 00, 00 02 00 00 or "OTTO") whose table directory has a `DSIG`
table → source type **4**; an sfnt header with a `loca` (or `sloc`) table → **1**; a PostScript file starting
with "%!" (`0x008A1B75`) → **2**; otherwise 0, and the face is not opened (Init fails). Source type 3 maps a probe
result of 0x16 that only arises together with an error, so on an opened font it never occurs. Consequence: a
signed TrueType file (has `DSIG`) gets type 4 here although `GetFontSourceTypeFromFileName` (§12) would call a
`.ttf` type 1; only type 2/3 changes behaviour (this section and §7). On a successful open the holder's u16 load
flags (+0x20, 0x000A from the constructor) gain 0x80 for types 1 and 4 and 0x01 for types 2/3; AddCharData (§7)
passes these flags to every glyph load. Before Init the type is 0.

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
  (A + B) / 65536 − vertical bearing / 64 (each term divided separately, signed truncation).
- **Advance** (FN7, `0x00899176`..`0x00899225`). The unscaled 16.16 advance `adv` is face+0x80 when the face has
  the fixed-size flag (face+0x5C ≠ 0, the same flag that makes faceKind 2), else face+0xD0 (the loaded glyph's
  16.16 advance). With s = advanceScale (float at +0x70):
  - Let t be the float32 constant at `0x00AC56E0`, bits `0x3F7AE148`, exactly
    0.980000019073486328125 (the stored approximation of 0.98).
  - s == 1.0: advance = adv / 65536 (signed integer division, truncation toward 0).
  - t < s < 1.0 and ch == U+0020: v = ftol(adv · s · 2^-16), evaluated as described below; narrow v to s16
    before the comparison. If sign-extended v + 1 > adv / 65536 then store v − 1, else v.
    (A space that would reach the full unscaled width is shortened by one pixel.)
  - t < s < 1.0 and ch != U+0020: **ignore s**, and use adv / 65536, as in the s == 1.0 case
    (`0x008991BF`..`0x008991F2`). This corrects the earlier description of the non-space branch.
  - Other s: advance = ftol(adv · s · 2^-16), evaluated as below.
  - The result is stored as s16. U+200B and U+FEFF then get advance 0 (not in the space branch, which cannot
    apply to them).
  Example: adv = 0x00058000 (5.5 px), s = 0.99, ch = space: v = trunc(5.445) = 5; 5 + 1 > 5 → advance 4.
  Same with ch = 'a': advance 5. s = 1.0: advance 5.
- **Advance evaluation and precision** (FN16). The two scaled paths load signed i32 adv exactly into the x87,
  multiply by the float32 s, then multiply by the float32 constant 2^-16 (bits `0x37800000` at `0x00AC276C`),
  then call `_ftol2` to truncate toward zero. These are two successive floating operations, with no intermediate
  memory store (`0x008991C5`..`0x008991D1`, `0x008991FE`..`0x0089920A`). Multiplication uses the **current
  thread's x87 precision and rounding controls**; `_ftol2`'s integer truncation does not undo the product's
  prior rounding (`0x009EEF04`..`0x009EEF78`). The font does not select its own precision.
  In the ordinary application startup, precision is explicitly set to **24 significand bits**, overriding
  the CRT's 53-bit default: `cSC4App::PreFrameWorkInit` calls `0x005F85E0` at `0x0045236C`; that wrapper passes
  0 to `0x008929E0`. The latter clears CW bits 8..9 and reloads CW (`0x008929E8`..`0x00892A05`), selecting
  24-bit precision while preserving the other control bits. Its arguments 1 and 2 select 53 and 64 bits.
  The earlier CRT default setter is `0x009F6C30` (control value `0x10000`, mask `0x30000`). Thus a fixed f64
  product is **not** the ordinary application's advance rule. With finite, normal results and round-to-nearest,
  round adv·s to 24 significant bits first, then scale by the exact power of two and truncate. x87 keeps its
  extended exponent range; this is not a general instruction to use float32 storage for overflow/underflow.
  Precision is not a process-wide invariant: for example `cGZLua5Thread::Pause` saves the incoming precision,
  selects 53 bits if it was 24, and restores the saved value after its Lua call (`0x008B6C35`..`0x008B6C65`).
  A font call entered under another precision inherits that precision; this evidence does not assert a font
  call occurs inside that particular Lua call. A host using a different precision must emulate the application's
  24-bit default explicitly to reproduce its normal results.

  Discriminating vectors (round-to-nearest, finite inputs; s given as float32 bits; final stored s16):

  | adv | s bits | Character | PC 24 | PC 53 or 64 | Reason |
  |---|---|---|---|---|---|
  | 65535 | `3F800080` | U+0041 | 1 | 0 | Exact scaled product is 1 − 2^-32; PC 24 rounds it to 1 before truncation |
  | −65535 | `3F800080` | U+0041 | −1 | 0 | Negative counterpart: truncation cannot recover a fraction rounded away |
  | 524288 (8 px) | `3F7D70A4` | U+0041 | 8 | 8 | Narrow-band non-space branch bypasses multiplication; a scaled implementation gives 7 |
  | 524288 (8 px) | `3F7D70A4` | U+0020 | 7 | 7 | Space uses the product, truncates to 7, and 7 + 1 is not greater than 8 |
  | 360448 (5.5 px) | `3F7D70A4` | U+0020 | 4 | 4 | Product truncates to 5; the space adjustment decrements it |

  The first two products were also measured on x87 hardware with CW `0x007F`, `0x027F`, `0x037F`, storing
  the result in extended format before integer conversion: respectively ±1 versus ±(1 − 2^-32).
  These are arithmetic measurements of the documented instruction sequence, not game-run observations.
- **Missing or unloadable glyph** (FN7, `0x008992BD`). Reached when the font has no holder or no open face, when
  the glyph load (or the glyph-done call `0x0089E29E`) reports an error, or when the index is 0 and neither the
  name map nor the default glyph supplies one. With h = charHeight (unsigned) and w = h >> 1: the record's source
  rectangle is (penX, 0)-(penX + w, w), i.e. a **w × w square**; xOffset 0, yOffset 0 (the square sits at the
  line top); advance = ftol(w · advanceScale) (integer w times the float scale, no 1/65536 factor). The raster is
  a **hollow outline**: w × w bytes, 0xFF on the first and last row and column, 0x00 inside (`0x008962D7`),
  passed with 256 levels. U+200B/U+FEFF instead get an empty raster (no pixels, 2 levels) and advance 0.
  Example: charHeight 15, advanceScale 1.0 → a 7×7 outline, advance 7. Edge: when the glyph loads but the
  glyph-done call reports an error, the loaded record is already in the map; the box raster is still
  appended to the source bitmap, but the map insert is unique (`0x00897CB2`), so the box record is dropped and the
  returned record is the loaded glyph.
- **Name map** `0x00898EB2` (FN9), used for source types 2/3: for ch ≥ 0x80 it replaces `FT_Get_Char_Index`
  entirely; for 0x20 ≤ ch < 0x80 it is tried when that index is 0. Steps: (1) the holder's own map (+0x28,
  u16 character → glyph index) is consulted first; a hit returns the cached index. (2) On a miss, a process-wide
  table character → PostScript glyph name is built once (`0x008988F1`) from 479 static {u16 code, name} pairs at
  `0x00B150F0` (Adobe-glyph-list style: 0x20 "space", 0x21 "exclam", ...; codes 0x20..0xFB02; insert is unique so
  the first of two duplicates wins: 0xB7 → "periodcentered", 0x3A5 → "Upsilon"). If ch has a name and the face
  resolves that glyph name (`0x0089F4F3`) to a code whose `FT_Get_Char_Index` is non-zero, that index is cached in
  the holder's map and returned. (3) Otherwise the name `uniXXXX` (four upper-case hex digits of ch & 0xFFFF,
  `sprintf "uni%04X"`) is tried the same way and cached on success. (4) Else 0, and nothing is cached (the next
  request repeats the search).
- **Complete name map** (FN15): [font-name-map-fn15.tsv](data/font-name-map-fn15.tsv) is the normative,
  complete character → name table, after keeping the first occurrence of each character. Its 477 data rows
  are sorted by the hexadecimal, four-digit unsigned UTF-16 code; names are case-sensitive ASCII. A missing
  code has no table name; use the `uniXXXX` fallback above, rather than another version of the Adobe glyph list.
  Reproducible provenance: in the Windows executable with SHA-256
  `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, image base `0x00400000`,
  read 479 consecutive 8-byte records at VA `0x00B150F0` (file offset `0x007150F0`, exclusive end
  `0x00715FE8`). Each record has a little-endian u16 character at +0, two padding bytes, and a little-endian
  32-bit VA at +4 pointing to a NUL-terminated ASCII name. Translate VAs through the PE section table.
  Visit records in ascending address order and keep the first name for each code, then sort by code for the
  TSV. The reader at `0x008988FA`/`0x00898905` uses precisely those fields; its stride and bound at
  `0x00898920`/`0x00898923` are 8 and `0xEF8` (479 records). The unique insertion's equal-key path at
  `0x00897ECD`..`0x00897EF7` returns the existing entry without overwriting its name.
  The only discarded records, using zero-based ordinals, are 421 (U+00B7, `middot`, first is ordinal 108,
  `periodcentered`) and 424 (U+03A5, `Upsilon1`, first is ordinal 282, `Upsilon`). The TSV has header
  `code<TAB>name`; its canonical UTF-8/LF content, including the final newline and no BOM, has SHA-256
  `421d2a61eab4f96e50a94aee1980bdcbfc718f7b2e147b34fbc262172345ecae`.
  Discriminating vectors: U+00B7 must request `periodcentered`, never `middot`; U+03A5 must request `Upsilon`,
  never `Upsilon1`; U+2126 requests `Omega`; U+FB01/U+FB02 request `fi`/`fl`. With no holder-cache hit and a
  face accepting only the discarded alias, the alias is **not** tried: try the retained name, then `uni00B7`
  or `uni03A5`, then return 0 if neither resolves to a non-zero character index. U+FFFF has no table entry
  and goes straight to `uniFFFF`.
- **CopyGlyphToSourceData** (slot 62, `0x00897748`) appends the raster at the cache's pen x. With underline or
  strike style it widens the cell to max(raster width, advance) and folds a positive xOffset into the cell. It grows
  the source bitmap (slot 63) to width `cellWidth·32 + x` or to the needed height. 2-level rasters are unpacked MSB
  first into bytes 0x00/0xFF. When aaMode ≠ 1 and the aa table is non-empty every byte is mapped through it.
  Underline: rows from `ascent − yOffset + uOffset` (`0x00896618`), strike: from `ascent − charHeight/4 − yOffset`
  (`0x00896636`), each filled with 0xFF for the line thickness, clipped to charHeight; srcBottom grows to include
  them.
- **Underline and strike-through geometry** (FN8): both depend only on pointSize (+0x38, unsigned), not on face
  data, and use different tables.

  | Thickness | 1 | 2 | 3 | 4 |
  |---|---|---|---|---|
  | underline T_u (`0x00896618`) | pointSize 0..15 | 16..25 | 26..35 | ≥ 36 |
  | strike T_s (`0x00896636`) | pointSize 0..23 | 24..47 | 48..71 | ≥ 72 |

  That is, T_u = 1 below 16, 2 below 26, 3 below 36, else 4; T_s = 1 below 24, 2 below 48, 3 below 72, else 4.
  Underline: the first row is r = ascent − yOffset + T_u (the "offset" **is** the thickness), and T_u rows are
  filled. Strike: r = ascent − (charHeight >> 2) − yOffset, T_s rows filled. For each line: when r < 0 or
  r ≥ charHeight nothing is drawn (and srcBottom is unchanged); otherwise rows r, r+1, ... are filled with 0xFF
  across the cell width while row < charHeight, and srcBottom = max(srcBottom, r + rowsFilled). Underline is
  drawn before strike. Example: pointSize 12, charHeight 15, ascent 12, yOffset 3: underline row 10 (1 row);
  strike row 12 − 3 − 3 = 6 (1 row). pointSize 30: underline T_u 3 rows, strike T_s 2 rows.
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
| 47 | TextArea(s, n, rect) | rect = (0, 0, GetStringWidth(s, n), CharHeight) (3 stack arguments, `RET 0xC`) |
| 44 | CalculateWidthOfLines(s, n, height) | §8.2 |
| 45 | CalculateTextArea(s, n, rect, minW, maxW, maxH) | §8.2 |
| 46 | CalculateTextArea(s, n, rect, wrap, fixWidth) | §8.2 |
| 50 | FormatForEllipsis(text, ellipsis, rect, keepAll) | §8.3 |

**8.1 Words to fit.** n = 0 or w = 0 → returns 0 and width 0. Text is converted (scratch 2, map vector cleared).
Walk words (`UTF16GetWord`, §8.4) from i = 0: if s[i] is CR (0x0D) or LF (0x0A), i += 1 and stop (the line end is
consumed, one unit only); stop when VW(word) + sum > w (unsigned; VW = GetStringVisibleWidth, slot 49) — otherwise
sum += GetStringWidth(word) and i += word length. Trailing spaces of a word therefore never count against w, but
do count in the sum carried to the next word. If nothing fits (i = 0):
- mode 1 takes whole words while the unit at the word start is not NUL, LF, CR or a §8.4 space (0x20,
  U+2002..U+200B), up to n — in practice the rest of the line, overflowing w;
- mode 2 repeats: prev = i; i += (Thai cluster length (§8.4) if s[i] is U+0E00..U+0E7F, else 1) until
  GetStringWidth(s, i) ≥ w; then i = prev unless prev = 0. So at least one cluster is always taken, and otherwise
  the prefix is strictly narrower than w. Then, if i < n, the map vector is non-empty and map[i−1] == map[i],
  i += 1;
- any other mode returns 0.
Afterwards (all paths that took something): if i > 1 (signed), s[i−1] is not a §8.4 space, i < n and s[i] is a
§8.4 space, i += 1. The result is mapped back through the §11 map (i = map[i] when the map is non-empty), converted
to a source-unit length, and `width` = GetStringWidth of that prefix.

**CR LF** counts as **two** line ends: a call starting at CR consumes only the CR, and the next call starts at LF,
consumes it and returns 1 (an empty line). "ab\r\ncd" at a wide width gives the lines "ab\r", "\n", "cd", so
CalculateNumberOfLines returns 3. CalculateNumberOfLines (`0x00896886`) has no progress guard: it trusts each
call to return ≥ 1, which holds for n > 0 and w > 0 before the §11 map-back.

**8.4 Words, clusters and whitespace** (FN13; shared UTF-16 helpers in `0x009100..0x00912700`).

Character classes (each a fixed set built once into a hash set; "in" means set membership):

| Class | Test | Members |
|---|---|---|
| HB hard break | `0x00911F42`, in set | U+000A, U+000D, U+0085, U+2028, U+2029 |
| SB break-after | `0x00911FFE`, in set, or c ≤ 0x20 and c ∉ {0x0A, 0x0D} | 0020 002D 0082 2002-200B 2010 2012-2015 203E 3000 301C FE31-FE32 FE58 (plus every control ≤ 0x20 except LF, CR) |
| NS no-start (closing) | `0x00911D4A` returns "can begin a line" = not in set | 223 distinct codes (u16 tables from `0x00ACE760`): 0021 0025 0029 002C 002E 003A-003B 003F 005D 007D 00A8 00B0-00B1 00B7 00F7 02C7 02C9 02CD 0394 039B 2015-2016 2019 201C-201D 2025-2027 2030 2032-2033 203B 2103 2190-2192 2196-2199 221F-2220 2225 222B 2234-2236 2252 2266-2267 2295 2299 22BF 2502 250C 2514 2518 2524 252C 253C 2551-2554 2558-255D 2561-2563 2565-2566 256A-256D 256F-2570 2595 25CB 25CF 25E2-25E5 2606 2640 2642 3001-3003 3005 3008-300B 300D 300F 3011 3015 3017 301D 3041 3043 3045 3047 3049 3063 3083 3085 3087 308E 309B-309E 30A1 30A3 30A5 30A7 30A9 30C3 30E3 30E5 30E7 30EE 30F5-30F6 30FB-30FE 311D 3121 3124 BCA0 C2A4 C774 D2B8 D3F0 FE31 FE33-FE38 FE3D-FE40 FE4B FE50-FE52 FE54 FE56 FE59-FE5C FE5F-FE60 FE62 FE64-FE65 FE69-FE6B FF01-FF0C FF0E FF10 FF12-FF13 FF15-FF1C FF1E-FF20 FF3D FF3F-FF40 FF5B-FF5E FF61 FF63-FF65 FF67-FF70 FF9E-FF9F FFE0 (the table data as stored, including the odd entries 3008-300B, 301D, BCA0 etc.) |
| NE no-end (opening) | `0x00911E46` returns "can end a line" = not in set | 0024 0028 005B-005C 007B 00A3 00A5 2018 201C 2035 3008 300A 300C 300E 3010 3014 3016 301D FE5B FE5D FF04 FF08 FF3A-FF3B FF5B FF62 FFE1 FFE5 |
| CJK | `0x0091009B` | 2E80-E000, F900-FAFF, FE30-FE4F, FF10-FFEE (inclusive) |
| Thai | `0x009100DE` | 0E00-0E7F |
| Thai decoration | `0x00911B3E` | 0E31, 0E34-0E3A, 0E47-0E4E |
| space (visible width) | `0x00896254` | U+0020, U+2002..U+200B |

**UTF16GetWord(s, n)** (`0x0091227B`) returns the length of the first word: 0 for n = 0, else at least 1. Walking
pairs (p = previous unit, c = next unit), the word ends **before** c when any of: p is U+0E46; p is HB; c is HB;
or (c can begin a line and p can end a line) and (p is CJK, or c is CJK, or p is SB and c is not SB). Otherwise c
joins the word. So a word is a run of non-breaking characters **plus its trailing spaces/hyphens/controls**; a
hard break is always a word of its own; every CJK character is its own word unless a closing mark follows it or an
opening mark precedes the next. Examples: "foo  bar" → 5 ("foo  "); "  bar" → 2; "well-known" → 5 ("well-");
"a\tb" → 2 ("a\t"); "ab\r\n" → 2, then "\r" → 1; "漢字。" → 1 ("漢"), then "字。" → 2.

**Thai cluster length** (`UTF16ThaiCharGroupLength`, `0x00911CDE`) at s with n units: n = 0 → 0; n = 1 → 1;
s[1] = U+0E33 → 2; s[1] not a decoration → 1; else (n < 3 → 2); s[2] = U+0E33 → 3; s[2] not a decoration → 2;
else 3 + (s[3] == U+0E33) — s[3] is read even when n = 3 (no bound check).

**Composed character** (GetStringWidth, `GetComposedCharLength` `0x00912588` with the code page forced to UTF-16):
a Thai unit (U+0E00..U+0E7F) starts a Thai cluster as above; any other unit is 1 (surrogate pairs are not
joined). GetStringWidth adds GetAdvanceWidth of the **first unit only** of each cluster: the decorations of a
cluster add nothing.

**GetStringVisibleWidth** drops trailing units of the space class only (U+0020, U+2002..U+200B); tab, CR, LF and
U+00A0 are kept and measured. **GetAdvanceWidth** returns 0 (no charSpacing) when the font is uninitialised or
GetCharData returns no record; a missing glyph normally still has a record (§7 box), so this is the null case.

**8.2 Text area.** `CalculateTextArea(s, n, rect, wrap, fixWidth)` (slot 46, `0x00896957`, no return value),
rect = {left, top, right, bottom}, lh = GetLineHeight (slot 35):

| Case (tested in this order) | right | bottom |
|---|---|---|
| s null or n = 0 | left | top + lh |
| wrap = false | left + GetStringWidth(s, n) | top + lh |
| wrap, fixWidth = false | left + CalculateWidthOfLines(s, n, bottom − top) | unchanged |
| wrap, fixWidth = true | unchanged | top + CalculateNumberOfLines(s, n, right − left) · lh |

Example: rect (10, 20, 110, 70), lh 16, text of width 250 that wraps into 3 lines at width 100: wrap+fixWidth →
bottom = 20 + 48 = 68; no wrap → right = 260, bottom = 36.

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

**8.3 Ellipsis.** `FormatForEllipsis(text, ellipsis, rect, keepAll)` (slot 50, `0x00896A2D`) edits the string
`text` in place and returns its original length L (string length, before any change). W = rect.right − rect.left.
1. keepAll → nothing changes.
2. fit = CalculateCharsToFitInWidth(text, L, W, null) (slot 42). If fit ≥ L (unsigned) the text fits: unchanged.
3. E = GetStringWidth(ellipsis, its length). If E < W (signed): fit2 = CalculateCharsToFitInWidth(text, L, W − E,
   null); truncate text to fit2; remove trailing ' ' (0x20 only) characters one by one; append the ellipsis.
4. Otherwise (E ≥ W) the text is truncated to length 0: the result is the empty string, not the part that fits.
Example: W 50, "Hello world" (L 11) fits 7 characters, ellipsis "..." 9 pixels: fit2 at width 41 is 6 → "Hello " →
"Hello" → "Hello..."; returns 11. With W 8 the same call empties the text.

## 9. Drawing text

The separate [default renderer contract](font-renderer.md) specifies the 8-byte class `0x08CDA30B`
behind these callbacks, with corrected identity, slot types, clipping and pixel effects. Renderer behavior
remains outside this font-owner spec's coverage claim.

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
one.

**Explicit surface lock slots** (FN17; slot 55 `0x00896B8E`, slot 56 `0x00896BC2`): the counter at +0x80 is
a wrapping 32-bit integer. Lock calls renderer +0x14 only when the **old** count is 0, then increments the
count unconditionally. Unlock calls renderer +0x18 only when the **old** count is 1, then decrements the count
unconditionally. Both callbacks receive the font, the caller's target, and the mode derived from aaMode
(1 → 1, 2 → 2, 3 → 3, other → 1; `0x00896B2C`). Any callback return value is ignored. Both slots return
boolean **true** if control returns normally (only AL is defined as 1; the upper return-register bits are not
a 32-bit success code). There is no counter rollback, renderer-null guard, or underflow guard. A null renderer
on a callback edge is dereferenced **before** changing the count and faults; it does not produce a false
return. A nested lock/unlock which does not need a callback succeeds even with a null renderer. These are
the explicit slots' rules; the automatic lock in DrawTextLine above has its own failure check.

| Old count | Operation and renderer result | Renderer calls | New count | Slot result |
|---|---|---|---|---|
| 0 | Lock, renderer returns false | +0x14 once | 1 | true |
| 1 | Lock, null renderer | none | 2 | true |
| 2 | Unlock, null renderer | none | 1 | true |
| 1 | Unlock, renderer returns false | +0x18 once | 0 | true |
| 0 | Unlock, null renderer | none | `0xFFFFFFFF` | true |
| `0xFFFFFFFF` | Lock, null renderer | none | 0 | true |
| 0 | Lock, null renderer | null dereference | no completed increment | no normal return |
| 1 | Unlock, null renderer | null dereference | no completed decrement | no normal return |

These vectors distinguish counter bookkeeping and return channels independently of actual surface access.
Evidence: callback dispatches `0x00896BB3`/`0x00896BE7`, unconditional counter changes
`0x00896BB6`/`0x00896BEA`, and boolean returns `0x00896BBC`/`0x00896BF0`.

**Palette** (`SetCurrentPalette(fg, bg)`, `0x0089838C`): reuse the current palette when its (fg, bg) matches; else
search the MRU list and move a hit to the front ("matches" = entry 127 == fg and entry 0 == bg, full 32 bits). A
miss builds 128 dword entries: 0..7 = bg (raw), then for k = 0..14 eight entries of a composed colour, then
120..127 = fg (raw) — the fg fill overwrites the k = 14 block, so only k = 0..13 survive (entries 8 + 8k .. 15 + 8k).
Before inserting, a list holding more than 3 palettes drops its last one.

*Channel split and compose (FN12).* `RZ::GS()` (`0x00913C1A`) is the graphic-system service fetched with
iid `0x0073283C`, which `QueryInterface` (`0x0098DEF0`) answers with the subobject at +0x1C (vtable `0x00ADAA58`).
Its +0x90 (`0x0098EA00`) and +0x8C (`0x0098E9B0`) forward to the **screen buffer** (`cGZScreenBuffer`, graphic
system +0x100, see `span-render-4444.md` §1.2): +0x90(c, &r, &g, &b) → buffer +0x7C (`0x00825F80`),
+0x8C(&out, r, g, b) → buffer +0x78 (`0x00825DA0`), *out = result. Both return false and write nothing when there
is no screen buffer. So the format is **the screen buffer's colour type** (buffer +0xC): 7 (A4R4G4B4) when the
display depth is ≤ 16, 9 (A8R8G8B8) otherwise; not a fixed ARGB format. Rules (r, g, b are bytes; the split
returns false and leaves its outputs untouched for any other type; the compose returns 0 for any other type):

| Colour type | split(c) → r, g, b | compose(r, g, b) |
|---|---|---|
| 4, 6 (1555) | r5·33 >> 2, g5·33 >> 2, b5·33 >> 2 (bits 10, 5, 0) | 0x8000 \| (r & 0xF8) << 7 \| (g & 0xF8) << 2 \| b >> 3 |
| 5 (565) | r5·33 >> 2, g6·65 >> 4, b5·33 >> 2 (bits 11, 5, 0) | (r & 0xF8) << 8 \| (g & 0xFC) << 3 \| b >> 3 |
| 7 (4444) | r4·17, g4·17, b4·17 (bits 8, 4, 0; the top nibble is ignored) | 0xF000 \| (r & 0xF0) << 4 \| (g & 0xF0) \| b >> 4 |
| 8, 9 (8888) | bits 16..23, 8..15, 0..7 (bits 24..31 ignored) | 0xFF000000 \| r << 16 \| g << 8 \| b |

The font calls the split with **only the low 16 bits** of fg and of bg (`MOVZX` at `0x00898414`, `0x00898433`).
Per channel, value = bg_ch + (fg_ch − bg_ch)·k / 16 with signed division truncating toward zero (`IDIV` at
`0x00898470` ..`0x008984A2`), always 0..255, passed as (r, g, b) to the compose; the result fills the eight
entries (`memset32`, `0x008984BF`). Consequences: **alpha is not interpolated** — entries 8..119 get the compose's
fixed alpha (0xF in type 7, 0xFF in type 9, none in 565/1555), whatever the alpha of fg and bg; k = 0 gives the
compose of the split bg, not raw bg; with a 32-bit screen (type 9) **red is always 0** in entries 8..119 because
bits 16..23 were cut before the split. Without a screen buffer, or with another type, the split leaves the stack
bytes and the compose leaves the output dword uninitialised (entries 8..119 undefined; type 0..3 → compose 0).

Vectors (`docs/specs/proofs/font_palette_fn12.py` runs `0x00825F80`/`0x00825DA0` under unicorn: 0 mismatches
against the table over 9 363 inputs per type; columns are entries 0, 8 (k 0), 16 (k 1), 24 (k 2), 72 (k 8),
112 (k 13), 120):

| type | fg | bg | entries |
|---|---|---|---|
| 7 | 0000FFFF | 0000F000 | F000 F000 F000 F111 F777 FCCC FFFF |
| 7 | 0000F000 | 0000FFFF | FFFF FFFF FFFF FEEE F888 F333 F000 |
| 7 | 0000F84C | 00000000 | 0000 F000 F000 F101 F426 F63A F84C |
| 9 | FFFFFFFF | FF000000 | FF000000 FF000000 FF000F0F FF001F1F FF007F7F FF00CFCF FFFFFFFF |
| 9 | 00FF0000 | 00000000 | 00000000 FF000000 FF000000 FF000000 FF000000 FF000000 00FF0000 |
| 9 | FF123456 | FF000000 | FF000000 FF000000 FF000305 FF00060A FF001A2B FF002A45 FF123456 |

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
  passed to AddFont(path) (slot 14); returns the count added (files matching `.PFB` count as well).
- `AddFont(path)`: once per path; relative names fall back to `<Windows>\Fonts\`. The file is opened with FreeType;
  a non-empty family name records FontFixedInfo {family, style flags, kind (2 if fixed-size else 1), source
  type} under the path and adds the lower-case family to the family set.
- System antialiasing settings (vtable `0x00AC5488`; all zero after construction `0x0089529A`..`0x008952A6`):
  slot 5 `SetCodePage(cp)` (`0x00893D7B`) stores +0x20 and returns true; slot 7 `SetDefaultAntialiasing(mode)`
  (`0x00893D87`) stores +0x14; slot 6 `SetRenderer(r)` (`0x00893D91`) AddRefs r (if non-null), releases the old
  one (if non-null), stores +0x24 (no same-pointer check: setting the current renderer again is AddRef then Release).
  Slot 8 `SetMonochromeSizeRange(min, max, family)` (`0x008954CB`): with family null it stores the system range
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
  and returns null. With `cache` the font is recorded (FN14): when the catalogue map has an entry
  for the path, the font is appended to that entry's font list; when it has **none**, a **new entry is created**
  for the path: {font list = [font], family = GetName (slot 5, now the face's family), style = slot 7, kind =
  faceKind (slot 9), source type 0}, inserted into the catalogue map, and the lower-cased family is added to the
  family set — so a later family lookup can find a font that was only ever acquired by path. Without `cache` the
  creator's reference is released after the AddRef (net refcount 1). Returns an AddRef'd font.
- `FontAcquire(familyList, kind, size, style, cache)` (`0x00894ADF`, slot 11, 5 stack arguments):
  1. Split a lower-cased copy of familyList at ',' (`RZParseDelimitedText`, at most 32 names; n = count).
     Splitting rules (FN14, `0x00910BD3`): leading blanks (space, tab) of the whole text are skipped and a text
     that is empty or all blanks gives n = 0. Every unquoted ',' ends a field **even when the field is empty**,
     so "Arial,,Tahoma" gives 3 names ("arial", "", "tahoma") and ",Arial" gives 2; the part after the last ','
     becomes a field only when non-empty ("Arial," gives 1). Fields ended by ',' lose trailing blanks; blanks
     after each ',' are skipped; the last field keeps its trailing blanks (the whole-text trailing trim at
     `0x00910C0F` tests the first character instead of the last, so it never trims): "Arial , Tahoma " →
     "arial", "tahoma ". A field that starts and ends with '"' loses both quotes, and ',' inside an odd number of
     '"' does not split. An empty name is used like any other (it matches only a family whose name is empty).
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

### 12.1 `cGZFontSys` state and slot inventory (vtable `0x00AC5488`, 25 slots)

State (offsets ours to name): +0x04 second interface (vtable `0x00AC5454`); +0x08 initialised (bool); +0x0C
refCount; +0x14 default aaMode; +0x18/+0x1C system monochrome range; +0x20 codePage; +0x24 renderer; +0x28 installed
family set (lower-case names); +0x34 font-name map (ID → name, `FontSetName`); +0x40 catalogue (path →
FontFixedInfo, entry count +0x44, iterated in path order); +0x4C substitution map (substitute family → vector of
aliases); +0x64 per-family monochrome ranges. Each catalogue entry's FontFixedInfo +0 is its **cache list**: a list
of {font, recorded style} pairs, one per font made or registered for that path, each holding one reference.

| Slot | Address | Method (names ours where the export has none) | Spec |
|---|---|---|---|
| 0 | `0x00893D2C` | QueryInterface(id, &out) | below |
| 1 | `0x0094C47C` | AddRef: refCount += 1, returns it (shared) | — |
| 2 | `0x00893DD7` | Release | below |
| 3 | `0x00895E68` | Init | §12 |
| 4 | `0x00894F8D` | Shutdown | below |
| 5 | `0x00893D7B` | SetCodePage | §12 |
| 6 | `0x00893D91` | SetRenderer | §12 |
| 7 | `0x00893D87` | SetDefaultAntialiasing | §12 |
| 8 | `0x008954CB` | SetMonochromeSizeRange | §12 |
| 9 | `0x008942E3` | FontAcquireByKind(kind, size, style, colour, cache) | below |
| 10 | `0x00895A1C` | FontAcquire(path, size, style, cache) | §12 |
| 11 | `0x00894ADF` | FontAcquire(familyList, kind, size, style, cache) | §12 |
| 12 | `0x00893F22` | FontRelease(font) | below |
| 13 | `0x008957D6` | AddFont(font) | below |
| 14 | `0x00895564` | AddFont(path) | §12 |
| 15 | `0x00894864` | AddAllFontsFromDirectory(dir) | §12 |
| 16 | `0x0093878E` | one ignored argument, returns false (shared stub) | — |
| 17 | `0x00894A3A` | RemoveFontsByFamily(name) | below |
| 18 | `0x00893E5B` | GetFontInfoList(records, max) | below |
| 19 | `0x00893ECB` | GetCachedFonts(fonts, max) | below |
| 20 | `0x0089595D` | AddSubstitution(family, alias) | below |
| 21 | `0x00894198` | GetSubstitutions(family, strings, max) | below |
| 22 | `0x008944B2` | IsFamilyAvailable(name, unused, style) | below |
| 23 | `0x0089423C` | GetAvailableSubstitution(family, &out) | below |
| 24 | `0x0089538D` | deleting destructor(flags) | below |

- **QueryInterface** (slot 0): id 1 or `0x00673960` → this; id `0x287FB697` → the second interface (this + 4);
  then AddRef and return true. Any other id returns false and leaves `out` untouched.
- **Release** (slot 2): refCount > 1 → decrement and return the new count. Otherwise Shutdown (slot 4), then the
  deleting destructor (slot 24, flag 1), return 0.
- **Shutdown** (slot 4) returns true; it acts only when initialised: for every catalogue entry, every cached font
  is Released once (a font still referenced elsewhere is left alive, nothing else happens), and the cache list is
  emptied; then the catalogue, the font-name map, the family set and the substitution map are cleared, the
  renderer is Released and set to null, and initialised is cleared. Kept: default aaMode, both monochrome range
  stores, codePage, file patterns. A later Init runs again (it tests the flag) and re-registers the four names.
- **FontAcquireByKind** (slot 9): calls slot 11 with an empty family list (so entries are scored by kind, §12
  step 4 n = 0), kind, size, style and cache; on a non-null result it calls the font's SetFontColor(colour,
  `0x08000000`) (font slot 14; bg = the "use default" sentinel of §9) and returns the font. With `cache` this
  recolours the shared cached font.
- **FontRelease(font)** (slot 12): every catalogue entry's cache list is searched for the pointer. On a hit the
  font is Released; when that Release returned 1 (only the cache's reference remains) it is Released again and
  its list node is removed. The search moves on to the next entry. Returns the result of the last Release made
  (0 when the pointer was not found; normally 0 after the cache let go, else the caller-visible count).
- **AddFont(font)** (slot 13): null → false. key = font's GetFilePath (font slot 21). Key not in the catalogue: a
  new entry {family = GetName (slot 5), style = GetStyles (slot 7), kind = faceKind (slot 9), source type 0
  (FontFixedInfo default)}, the font AddRef'd into its cache list with recorded style = GetStyles, and the
  lower-cased family added to the family set; true. Key present: if a font in its cache list has the same size
  (font slot 10) → false, nothing added; else the font is AddRef'd and appended with recorded style = GetStyles;
  true. The family set is not touched on that path.
- **RemoveFontsByFamily(name)** (slot 17): every catalogue entry whose family string IsEqual(name) (string slot
  10, flag false) has each cached font Released once and is erased. The family set is not updated (so slot 22
  and the §12 step 2 test still see the name). Always returns false.
- **GetFontInfoList(records, max)** (slot 18): n = catalogue count; max 0 → return n, nothing written. Otherwise
  writes min(n, max) records of 0x88 bytes in path order and returns that count. Record: bytes 0..0x7F zero-filled,
  then the family copied for at most 127 bytes (always NUL-terminated); +0x80 style (u32); +0x84 kind (u32).
- **GetCachedFonts(fonts, max)** (slot 19): walks entries in path order and each cache list in list order. max 0 →
  returns the total number of cached fonts, nothing written; otherwise writes the first max pointers and returns
  min(total, max). The pointers are not AddRef'd.
- **AddSubstitution(family, alias)** (slot 20): key = family exactly as given (no lower-casing; map keys order
  byte-wise, case-sensitive: compare of the common prefix, then the shorter string first, `0x004B5CA0`). Missing key → insert
  it with an empty alias vector. Append alias (as given; duplicates are kept). Returns true.
- **GetSubstitutions(family, strings, max)** (slot 21): exact key lookup; missing → 0. Else returns
  m = min(aliasCount, max) and, for i = 0 .. max − 1, assigns alias i into strings[i] (string slot 3, chars +
  length) when strings[i] is non-null. The loop bound is max, not m: the original reads past the alias vector
  when max > aliasCount (undefined). A conforming implementation assigns only the first m entries and leaves the
  rest untouched; callers pass max ≤ aliasCount.
- **IsFamilyAvailable(name, unused, style)** (slot 22): lower(name) must be in the family set, else false. Then true
  iff some catalogue entry has style == (style & 3) and lower(entry family) == lower(name) (byte compare after
  lower-casing). The second argument is ignored.
- **GetAvailableSubstitution(family, &out)** (slot 23): exact key lookup; for each alias in order, the first with
  IsFamilyAvailable(alias, 0, 0) (slot 22, so style 0) is assigned to out (string slot 3) and true is returned.
  Missing key or no available alias → false, out untouched.
- **Deleting destructor** (slot 24): the destructor (`0x00894F2C`) restores both vtables and destroys the
  containers (no Release of cached fonts or renderer: Release calls Shutdown first); flag bit 0 frees the 0x70
  bytes.

Worked example (slots 20-23): AddSubstitution("Arial", "Tahoma"), AddSubstitution("Arial", "Verdana"), installed
families {"verdana"} with a regular (style 0) catalogue entry for Verdana. GetSubstitutions("Arial", s, 2) → 2,
s = {"Tahoma", "Verdana"}; GetSubstitutions("arial", s, 2) → 0 (keys are exact); GetAvailableSubstitution("Arial")
→ true, "Verdana" ("tahoma" is not in the family set).

## 13. Open questions

FN1-FN17 are answered (FN7-FN14 by re-font-core-answers4; FN15-FN17 by re-font-final-gaps4)
(archive: `docs/specs/questions/answered/drawing-fonts.md`); open entries, if any, are in
`docs/specs/questions/drawing-fonts.md` (FN18-FN23; FN23 is §15's).

## 14. MXF font files *[compat]*

*Added 2026-10-23 (re-fonts).* The game's own fonts (`Fonts\*.mxf` in the install, 19 files) are ordinary TrueType or
Type 1 font files, lightly obfuscated:

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | magic `MXFN` |
| 4 | 4 | L, unsigned big-endian 32-bit integer: how many payload bytes are obfuscated |
| 8 | 1 | key k |
| 9 | … | payload: the font file, each byte at a payload offset below L XORed with k |

The font stream recognises this wrapper only in an opened file longer than 9 bytes with the initial magic `MXFN`.
For a recognised wrapper, logical font offset zero is physical file offset 9. An unwrapped file is exposed from
physical offset zero without XOR. A nine-byte file containing just the header is therefore not recognised as MXF.
This stream serves `AddFont` and font creation (§12); recognition and big-endian decoding are instruction-checked
at `0x00896D70`–`0x00896DD1` (opener `0x00896D2F`, exported as `_FT_New_Stream`).

A successful read of n bytes at logical offset p returns the corresponding stored bytes, with XOR by k applied
only to bytes whose logical offset is less than L. The transformed prefix has length min(n, L − p) when p < L,
and zero otherwise. Reads beginning at or after L are unchanged. The read reports success or failure; a failed
underlying read receives no XOR pass (`ReadAt`, `0x00896DDD`–`0x00896E47`). `Read` at `0x00896E4A` forwards that
operation and returns its Boolean status. Failed-read buffer contents are not specified here.

Worked boundary example: L = 2, k = `0x9D`, stored payload `9D 9C 02 03`. A four-byte read at logical offset zero
returns `00 01 02 03`; a two-byte read at offset one returns `01 02`. L = 0 leaves every payload byte unchanged
while still hiding the nine-byte wrapper. These examples concern successful reads within the payload.

Shipped files (`SimCity 4 Deluxe\Fonts`, all 19): L = 40,000,000 (so the whole payload is obfuscated) and k = `0x9D`.
Sixteen payloads are TrueType (`00 01 00 00`), three are Type 1 fonts in PFB segments (`80 01`: `Arta`, `Arta
(Italic)`, `Stoclet ITC TT (Bold)`). Every file's length is a multiple of 10,000 bytes, longer than its font: the tail
is filler that is not part of the font *[inferred]*.

A writer may use L = 40,000,000 and k = `0x9D` to match the shipped convention *[choice]*. The stream contract
also permits partial obfuscation: encode only payload offsets below L with XOR and leave the remainder unchanged.
Neither whole-payload obfuscation nor padding to a multiple of 10,000 is a reader requirement.

## 15. The glyph-cache holder (class `0x8B3255F9`, labelled `cGZDrawFontRenderer`) *[summary; FN23]*

*Added 2026-10-23 (re-fonts).* Identity: [font-renderer.md
§1](font-renderer.md#1-identity-ownership-and-exact-body-inventory) (class `0x8B3255F9`, 0x450 bytes from
`0x0082A2B0`, constructor `0x00829F70`, vtable `0x00AC14BC`; it holds the 8-byte default renderer `0x08CDA30B` at
`+0x43C`). Its cache, in outline: the holder keeps a cache surface and a table from glyph to its place in it. A glyph
is added (`AddGlyphToCache`, `0x0082A2F0`) at the current pen of the current row; when it does not fit across, a new
row starts below the tallest glyph of the row; when it does not fit down, the cache is flushed and restarted (or the
add fails when flushing is not allowed). The glyph's coverage bytes are written through a 256-entry colour table into
the cache surface's format (8, 16, 24 or 32 bits per pixel). Target setup also tests hardware-surface interface
`0xAB300B2B`; its complete drawing and fallback contract remains FN23. This summary claims no whole-holder coverage
(font-renderer.md FR3 keeps that owner's accounting open). It does not reopen the separate `cGZFont` owner.
