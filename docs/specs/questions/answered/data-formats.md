# data-formats: answered questions (archive)

## EX57 (2026-10-24)

<!-- from: glue-dbpf-ex56apply11 (2026-10-24) -->
- **EX57 - section 8.3 transient packed-file coverage:** which public interfaces/messages expose temporary load/save, obliteration and regional-city segment open/close lifecycles, with typed signatures, reference ownership and a callback at which indexed reads are stable? Section 8.3 proves these files escape resource-manager traversal but does not specify an observation contract. [shadow covers registered packed roots, multi-packed children and the current city backing path; transient files remain explicitly unobserved, no all-open-files credit]
  **ANSWERED 2026-10-24 (re-dbpf-ex57):** DIFFERS — §8.3.1 specifies ordinary synchronous load/save payloads and typed segment ownership, but no common public callback covers all these lifetimes. Load handlers provide readable intervals; save notifications precede completion, and PostSave does not set void1. Obliteration and regional helpers do not broadcast the ordinary messages; their typed inputs/results and conditional teardown quirks are documented. Existing scope remains valid; all-open-files credit needs additional lifecycle instrumentation. Nine vectors EX57-1–EX57-9 are in §10.9. Static export/instruction evidence only; no runtime coverage claim.

## EX56 (2026-10-24)

- **EX56 - section 8 public held packed-file traversal and reads:** Are resource-manager GetSegmentByIndex and multi-packed GetSegmentByIndex pointers borrowed, and does recursively visiting those roots cover every open packed file (including any city save backing file)? Does public cIGZPersistDBSegment::ReadRecord preserve existing open-record cursors/lifetimes and segment state when called on a game-held readable packed segment, and what is its exact return/in-out-size contract? [Filed by glue-dbpf-close8, 2026-10-24: default-off shadow treats index pointers as borrowed, owns only QI references, compares payload bytes/size without interpreting the return, and claims resource-manager-packed scope only.]
  **ANSWERED 2026-10-24 (re-dbpf-ex56):** MATCHES — Both index pointers are borrowed; only the manager accessor bounds-checks. §8.3 proves the registered city backing segment is a directly reachable packed root after successful load and re-registration after save, but independently opened temporary/regional/save segments prevent an all-open-files claim. ReadRecord returns full-width unsigned count/size or `0xFFFFFFFF`, clears size on rejection, supports plain prefixes but requires complete compressed output capacity, and preserves existing records' references and logical cursors while changing shared file position and possibly scratch allocation. Stable readable sampling needs no record lifecycle or city streaming calls. §10.1.2 adds DFP1–DFP14 and DFT1–DFT3, including explicit city-path coverage and transition invalidation.

## EX52 a/b/f and EX53 (2026-10-22)

- **EX52 c — serial-record slot 28 arguments.** **ANSWERED 2026-10-22 (re-exemplar-final-gaps4):** MATCHES — §8.2 proves thiscall `(bytes, uint32 byteCount)`, 8 for binary signature / 10 for text, and the raw-write tail dispatch through record slot 51. Caller and writable packed-file target agree.
- **EX52 d — kind-1 key getter and foreign property.** **ANSWERED 2026-10-22 (re-exemplar-final-gaps4):** DIFFERS — §8.2 proves property QI `0xE52184AB`, returned interface slot 6 with temporary key-output address, returned pointer sent to record slot 49, then Release. This supplies the previously missing foreign-property contract. A held resource IID alone does not select kind 1.
- **EX52 e (syntax and lookup algorithm only) — text SerialWrite.** **ANSWERED 2026-10-22 (re-exemplar-final-gaps4):** DIFFERS — §8.2 now specifies key/count/property lines, dynamic INI field-5 names, exact supported type formats, unsupported-type malformed lines, no escaping, and ignored text-entry write failures. Discriminating vectors include scalar count 0, signed hex widening, 64-bit array prefix asymmetry and unsupported Float64. Population and contents of the name source remain open under EX52 e.

- **EX52 a/b — non-null rebuild destination allocator and capacity word.** **ANSWERED 2026-10-22 (re-exemplar-final-gaps4):** MATCHES — §8.2 proves B allocation/free `0x0090CF54`/`0x0090CF63`, including insert growth, and three pointers `{begin, end, endOfStorage}` with 8-byte entries. Includes a two-entry layout/free vector. Remaining c–e stay open.
- **EX52 f — holder slots 14/16 native return 2 versus replacement 1.** **ANSWERED 2026-10-22 (re-exemplar-final-gaps4):** DIFFERS — §8.2 traces their last call to the newly constructed property's Release; EAX propagates the surviving reference count (two entries on factory exemplar/cohort, one on plain holder), not a fixed success value. Source property/variant is copied. Null slot-14 input has no established result; public count-return declaration and arbitrary override results are not inferred.
- **EX53 — scalar Char/RZUnicodeChar GetValString text and result.** **ANSWERED 2026-10-22 (re-exemplar-final-gaps4):** DIFFERS — §5.1 gives one raw byte for Char/RZChar, one byte for Unicode units ≤ `0xFF`, otherwise `?`; output is assigned and getter returns false. Explicit-length vectors include `41`, `E9`, `00`, and Unicode `0x0100`/`0xFFE9`. This contradicts the filed replacement behaviour of leaving the output unchanged.
<!-- Archive for [data-formats.md](../../data-formats.md). Lanes do not read this file. Open questions are in ../data-formats.md. -->

## From the exemplar reader (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/data-formats.md` §4, §5 and the §11 exemplar rows (`engine/formats/src/exemplar.rs`). The install
sweep loads 9,347 of 9,347 exemplars. The reader's current answer is in brackets.

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX1:** how are Empty (tag 0) properties stored? [`Values::Empty`; getters return None]~~ **ANSWERED
  2026-10-14** from the exe: **the storage half matches.** Tag 0 has no value bytes; the property is added and holds
  an Empty variant (memory type 0, `GetCount()` 0), so `HasProperty` is true. The getter half does not match: see
  EX5. No shipped record uses tag 0 (data-formats.md §4.2, with an example).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX2:** Uint64 to float, signed or unsigned? [unsigned]~~ **ANSWERED 2026-10-14** from the exe (`0x00915B85`):
  **the bracket matches**, with one addition. The value is unsigned (low 63 bits + 2⁶³), but the add runs at the
  main thread's 24-bit precision, so a Uint64 keeps 24 significant bits even when read as Float64: 16,777,217
  gives 16,777,216.0. Uint32 ≥ 2³¹ goes through the same kind of add (data-formats.md §5.1).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX3:** float to int when out of range or NaN? [saturates; NaN gives 0]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00915AA5` → `_ftol` `0x009EEF04`): **the bracket does not match.** The float is truncated to an int64 and the
  target keeps the low bits, so out-of-range values wrap (300.7 as Uint8 gives 44). NaN, ±∞ and |x| ≥ 2⁶³ give
  `0x8000000000000000`: 0 for targets of 32 bits or fewer, `0x8000000000000000` for 64-bit targets, true for Bool
  (data-formats.md §5.1, with examples).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX4:** is RZChar signed when widened? [zero-extended]~~ **ANSWERED 2026-10-14** from the exe (`0x00915AA5`,
  `0x00915B85`): **the bracket does not match.** RZChar and Char are **sign-extended** for integer and float
  targets: RZChar `0xE9` read as Uint32 gives `0xFFFFFFE9`, as Float32 −23.0. Only RZUnicodeChar is zero-extended
  (data-formats.md §5.1).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX5:** what do the getters return for Empty or object values? [None]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00917350`, `0x00916EAA`, `0x009167C5`): **the bracket does not match.** Both take the text round trip. Empty
  becomes empty text, which parses as 0, so the getters succeed with 0, 0.0 or false. An object becomes its
  `cIGZString` text or its address as `0x%08x`, so `GetProperty(id, uint32&)` returns its address. The string
  getter writes the text but returns false for every non-string type. The object case is a *[choice]*: missing
  (data-formats.md §5.1).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX6:** Char, RZUnicodeChar and void arrays in the text round trip. [Char and RZUnicodeChar arrays as text; a void
  array gives its first byte as `%u`]~~ **ANSWERED 2026-10-14** from the exe (`0x00916EAA`): **the first half
  matches**: Char and RZUnicodeChar arrays give all their characters, RZUnicodeChar units of `0x80` and above as
  `?`. **The void-array half does not**: a void array (and a VoidPtr array) gives the empty text, which parses as
  0 (data-formats.md §5.1, with examples).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX7:** an RZChar scalar as a string. [that one character]~~ **ANSWERED 2026-10-14** from the exe
  (`0x00916EAA`): **the bracket matches.** Char and RZChar scalars give their one character; an RZUnicodeChar
  scalar gives one byte when it is `0xFF` or less, otherwise `?` (data-formats.md §5.1).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX8:** what `%.24g` prints for the exponent and for nan/inf. [2-digit exponent; "nan"/"inf"]~~ **ANSWERED
  2026-10-14** from the exe (`0x0091052A`): **the bracket does not match.** There is no `%.24g`: floats are printed
  fixed-point, never with an exponent, with at most 24 digits after the fraction's leading zeros, cut, not rounded
  (Float32 1e30 gives 31 digits). NaN and infinities print as `1.#INF`, `-1.#IND` and the like *[inferred from the
  MSVC runtime]*. Values do not always survive the round trip: Float32 3.14159 comes back 2 ulps lower
  (data-formats.md §5.1).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX9:** is the round-trip parse's `0x` case-sensitive, and what does "after a digit" mean? [lowercase only;
  anywhere after the first digit]~~ **ANSWERED 2026-10-14** from the exe (`0x009167C5`): **the bracket
  matches**, with additions. `0x` is lowercase only, must start the text and be followed by a hex digit (16
  digits at most). "After a digit" applies only to `e`/`E` and means any earlier digit; `.` makes the text
  floating point wherever it is. `t`/`true`/`f`/`false` are case-sensitive whole-text matches. Char targets
  always get 0 (data-formats.md §5.1, with examples).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX10:** the ParentCohort line: field count and spacing. [exactly 3 fields, leading spaces trimmed]~~
  **ANSWERED 2026-10-14** from the exe (`0x0067DFD0`): **the bracket does not match.** Fields are not counted:
  a fourth and later field is ignored, and a missing field reads 0. Nothing inside the braces is trimmed: a
  space after `{` or `,` makes a `0x` field read 0, while a decimal field survives (data-formats.md §4.3).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX11:** are PropCount, reps and the type name trimmed? [PropCount and reps trimmed; the type name must match
  exactly]~~ **ANSWERED 2026-10-14** from the exe (`0x0067E250`, `0x0067E780`): **the bracket matches for
  PropCount and the type name, not for reps.** PropCount is trimmed of spaces and tabs like every `name=value`
  line. The type name is compared exactly, with no trimming at the `:`. reps is not trimmed: rule N reads it, so
  a leading space is harmless for decimal but turns `0x3` into 0. Rule N saturates at `0xFFFFFFFF`; too few
  property lines fail the read (data-formats.md §4.3).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX12:** is there a cap on reps? [more reps than the record length fails]~~ **ANSWERED 2026-10-14** from the
  exe (`0x0067E780`): **the bracket does not match.** There is no cap and no failure: missing values read 0
  and extra values are ignored. A huge reps crashes the original, because the buffer size (reps × element size,
  32-bit) wraps or cannot be allocated, and the result is not checked. Recommended *[choice]*: fill with 0 and
  fail the read above 16 MiB of values (data-formats.md §4.3).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX13:** are numeric values cut at `}` too? [no, only strings]~~ **ANSWERED 2026-10-14** from the exe
  (`0x0067E780`): **the bracket matches in effect.** Only strings are cut at `}`; a number stops at the first
  character that cannot continue it, so `}` ends it anyway (the last Float32 array value is split at `}`, with the
  same result). Leading spaces are trimmed for array values only, which matters for `0x` in rule N. Uint64 and
  Sint64 are hexadecimal with no sign (data-formats.md §4.3).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX14:** how is Float32 text parsed? [as f64, then rounded to f32; no hex/inf/nan]~~ **ANSWERED 2026-10-14**
  from the exe (`0x00910426`): **the first half of the bracket does not match; "no hex/inf/nan" does.** The game's
  own parser adds the digits one by one in x87 arithmetic, rounded at every step to the main thread's 24-bit
  precision; the exponent's power of ten is a wrapping 32-bit integer. So `0.05` gives `0x3D4CCCCC`, not
  `0x3D4CCCCD`, `1e10` gives 1,410,065,408, and `1e32` gives 0 (data-formats.md §4.3, with examples).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX15:** what counts as a resolvable kind-1 resource key? [an exemplar or cohort that loads, or a key registered
  from the package index]~~ **ANSWERED 2026-10-14** from the exe (`0x009732EB`, `0x00972EAE`): **the first half
  matches, the second does not.** A key resolves if a resource cache already holds it, or if a factory is
  registered for its type, a loaded package holds the record, and the factory's read succeeds. Index presence
  alone is not enough. Any type with a factory counts, not only exemplars and cohorts (data-formats.md §4.6).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX16:** shipped data: `Plugin_002_TheLivingMall.dat` writes `Uint8:0:{}` for two LotConfig properties, now read
  as a scalar 0 (§4.3). Intended? [scalar 0]~~ **ANSWERED 2026-10-14** from the exe (`0x0067E780`, `0x005FFB40`):
  **the bracket matches.** The original stores a scalar Uint8 0. Both readers of these properties use a byte-list
  getter that copies `GetCount()` bytes, 0 for a scalar, so the lot gets no wealth or purpose types. That is
  the same as an empty array, so the difference is not observable (data-formats.md §4.3).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX17:** slot 5 `GetProperty(id, void*, uint32&)`: what type does the caller request? [not implemented]~~
  **ANSWERED 2026-10-14** from the exe (`0x005FD290`, `0x00915714`): **the bracket does not match.** The original
  implements it as a raw copy from a **void array** only (memory type `0x8000`): at most `length` bytes, with
  `length` lowered to the array's size. Any other type returns false, with no conversion. No shipped record has a
  void array, so on shipped data it always returns false, and "not implemented" (always false) is not
  observable there (data-formats.md §5.2).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX18:** are §10.3 tests 18–22 (resource-manager load order) in scope for `engine/formats`? [out of scope]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The tests check the resource
  manager and startup registration (§3, §3.1), not the package reader, so they belong to whichever module
  implements §3.1 (data-formats.md §10.3).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX19:** how is depth counted for `MAX_PARENT_DEPTH`? [at most 32 nested loads below the record; a record that
  fails only on depth is retried when reached shallower]~~ **ANSWERED 2026-10-14** from the exe (`0x009732EB`):
  no original behaviour for the limit, because the original has none (§4.4); the implementer's choice stands.
  The retry half is consistent with the original, which caches only successful loads, so a failed key is loaded
  again on the next request (data-formats.md §4.4).

<!-- from: From the exemplar reader (2026-10-14) -->
- ~~**EX20 — §5.2:** a raw accessor on a type mismatch. [choice: a missing value]~~ **ANSWERED 2026-10-14** from
  the exe (`0x0060CE80`, `0x005FD610`–`0x005FD640`): the original does not check. It returns the stored bytes reinterpreted,
  such as `0x40300000` for a Float32 2.75 read as Uint32, or a heap address for an array. That is garbage
  for the caller, so the *[choice]* of a missing value stands (data-formats.md §5.2).

## From applying the data-format answers (filed by the coordinator from the `apply4a` lane's report, 2026-10-14)

About `docs/specs/data-formats.md` (EX1–EX20 as applied in `engine/formats` and the core variant). The current
answer is in brackets.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX21 — §4.3:** "multiply the scale by 0.1": the double constant 0.1 or the float `0.1f`? [double 0.1: it
  reproduces every worked example, while `0.1f` turns `0.05` into `0x3D4CCCCE`]~~ **ANSWERED 2026-10-14** from the exe
  (`fmul` with the double operand at 0x00A9EFE0 = `0x3FB999999999999A`, at 0x009104A7; 10.0 is the double at
  0x00AA2028) and written into data-formats §4.3 ("Details of the arithmetic").
  - The double constant 0.1, used as a double operand: each scale step rounds the exact product scale × 0.1 once, to
    the caller's precision. The bracket matches. With `0.1f`, `0.05` would give `0x3D4CCCCE` instead of `0x3D4CCCCC`.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX22 — §4.3:** is `value × 10 + digit` rounded once or twice (after the multiply and after the add)? [twice,
  each to 24 bits]~~ **ANSWERED 2026-10-14** from the exe (`fmul` then `faddp` at 0x00910476–0x0091048A, each result
  rounded under the precision control) and written into data-formats §4.3 ("Details of the arithmetic").
  - Twice, each to 24 bits on the main thread. The bracket matches. A fraction digit rounds three times (scale × 0.1,
    digit × scale, + value).
  - Worked example: `33573415` gives 33,573,412.0, where one rounding would give 33,573,416.0.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX23 — §4.3:** does the exponent accept a `+`, and is it read when no mantissa digit came first? [yes to both]~~
  **ANSWERED 2026-10-14** from the exe (exponent test at 0x009104C2 after both mantissa loops; sign at
  0x009104D4–0x009104DE) and written into data-formats §4.3 ("Details of the arithmetic").
  - Yes to both; the bracket matches. `+` and `-` are accepted after the `e`, and the exponent is read whatever the
    mantissa consumed, so the value is 0 × 10ⁿ: `e5` gives 0, `-e5` gives −0.0, `2.5e+3` gives 2500, and `e-32` gives
    the default NaN (`0xFFC00000`), as `0e-32` does.
  - In the §5.1 round trip the classifier stops at an `e` with no digit before it, so `e5` there never reaches the
    parser.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX24 — EX8:** how is −0.0 printed? [`0`]~~ **ANSWERED 2026-10-14** from the exe (the runtime's `$I10_OUTPUT`
  0x00A04942 resets the sign to positive on its zero path at 0x00A04BC1; `_fcvt` 0x009F662C reports sign = (sign is
  '-'); printer 0x0091052A) and written into data-formats §5.1 ("Negative zero").
  - `0`; the bracket matches. The sign of a zero is dropped inside the C runtime's conversion, so no `-` is printed,
    for a Float32 or a Float64.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX25:** signed decimal text read into a Float32: one rounding from the exact value, unlike a Sint64 scalar,
  which goes through double? [one rounding, so `-16777217` gives −16777216.0]~~ **ANSWERED 2026-10-14** from the exe
  (`ConvertFromString` signed branch: `fild` of the int64, then `fstp dword` at 0x00916BF6–0x00916BF9; scalar
  converter 0x00915B85 stores a double first) and written into data-formats §5.1 (Parsing, step 4).
  - One rounding from the exact value; the bracket matches: `-16777217` gives −16,777,216.0.
  - A Sint64 scalar rounds twice (to the stored double, then to Float32). The two differ only above 2⁵³ in magnitude:
    −(2⁵⁴ + 2³⁰ + 1) gives −(2⁵⁴ + 2³¹) from text, but −2⁵⁴ from a scalar.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX26:** a Bool scalar's "leading spaces trimmed": spaces only, or tabs too? [spaces only]~~ **ANSWERED
  2026-10-14** from the exe (the Bool branch of `InterpretEntry` 0x0067E780 calls `cRZString::LTrim` 0x0090E869, whose
  character set at 0x00ACE73C is space and tab) and written into data-formats §4.3 (`Bool`).
  - Spaces **and tabs**, the same trim as array values. The bracket differs: a value starting with a tab before `true`
    reads as true.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX27:** a String value is "trimmed" of what? [spaces and tabs]~~ **ANSWERED 2026-10-14** from the exe (the
  String branch of `InterpretEntry` 0x0067E780 calls `cRZString::Trim` 0x0090E8A6 = `LTrim` + `RTrim`, set " \t" at
  0x00ACE73C) and written into data-formats §4.3 (`String`).
  - Spaces and tabs, at both ends; the bracket matches. The trim runs before one leading and one trailing `"` are
    removed, so white space inside the quotes is kept.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX28 — EX8:** a Float32 signalling NaN as text? [the x87 load quiets it: `1.#QNAN`; a Float64 sNaN gives
  `1.#SNAN`]~~ **ANSWERED 2026-10-14** from the exe (call sites `fld dword` 0x00916F62/0x009170FD and `fld qword`
  0x00916F67/0x00917117, then `fstp qword`; the printer reloads at 0x00910542; runtime `1#SNAN` branch 0x00A049E6) and
  written into data-formats §5.1 ("Signalling NaN").
  - Float32 sNaN: `1.#QNAN` (or `-1.#QNAN`); the bracket matches.
  - Float64 sNaN: **also `1.#QNAN`**; the bracket (`1.#SNAN`) differs. The double is loaded onto the x87 stack before
    printing, and loading an sNaN quiets it under the masked invalid-operation exception. `1#SNAN` is unreachable from
    this printer.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX29 — EX8:** does the 63-character cut also apply to Float32 text (a subnormal reaches it)? [yes]~~ **ANSWERED
  2026-10-14** from the exe (`0x0091052A`, callers `0x00916F77` and `0x0091711E`, `_fltout2` `0x00A0277B`) and written
  into data-formats.md §5.1 "Floats as text". **The bracket matches.**
  - The cut applies to any text longer than 63 characters, whatever the type. The printer returns false and both
    callers ignore it, keeping the cut text.
  - A Float32 reaches it only as one of the seven negative subnormals `0x80000001` to `0x80000007` (below 1e-44 in
    magnitude). `-0.`, 44 zeros and 17 digits make 64 characters, so the last digit is dropped.
  - **Correction to EX8:** the digits are not the exact binary value cut at 24 digits. `_fcvt` gets **17 significant
    digits, rounded half-up**, from the runtime (the count 17 is passed at `0x00A027A0`), padded with zeros. The
    24-digit limit never takes effect. Float32 0.1 prints `0.10000000149011612`, not `0.100000001490116119384765`, and
    3.14159 prints `3.1415901184082031`. Any implementation of the EX8 rule needs the same change. Test 58 and the §11
    row are corrected.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX30 — EX17:** `get_void` with a buffer shorter than `length`? [copies what fits; the original would overrun]~~
  **ANSWERED 2026-10-14**: no original behaviour; the implementer's choice stands. The original (`0x00915714`) copies
  `min(length, n)` bytes with a plain memory copy (`0x009F19B0`) and has no way to know the buffer's size, so the
  bracket's description of it is right: it overruns a buffer shorter than `length` when the array is longer. No
  shipped record has a void array, so this is not observable. Written into data-formats.md §5.2.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX31 — EX15:** when do factories run? [eagerly, inside `load_package`; a factory registered later applies from
  the next `load_package`]~~ **ANSWERED 2026-10-14** from the exe (`GetResource` `0x009732EB`, loader `0x00972EAE`,
  cache offer `0x009733C6`, segment registration `0x0097437B`/`0x009743C1`/`0x009742B7`) and written into
  data-formats.md §3 "When records are read". **The bracket does not match the original's timing.**
  - Registering a segment reads nothing. A factory runs lazily inside `GetResource`, the first time an uncached key is
    requested, and reads the record from the segment that wins at that moment. A success goes to the first registered
    cache that accepts it; a failure is not kept and is read again next time.
  - So a factory registered later applies at once to uncached keys, including keys in packages loaded before it, not
    from the next package.
  - *[choice]* Eager reading stands if (1) later packages replace not-yet-looked-up reads of the same key (or all
    packages load before the first lookup), (2) failed records are retried after later packages, and (3) a late
    factory is applied to earlier packages. Which resource caches the game registers was not traced.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX32:** a Float32 that no text reproduces under the 24-bit parser: what does the writer do? [`to_text` fails
  with `Unsupported`]~~ **ANSWERED 2026-10-14** from the exe (`WriteEntry` `0x0067CDD0`,
  `RemoveUnnecessaryFloatingPointDigits` `0x0067C2B0`, `SerialWrite` `0x00680030`, factory constructor `0x0067E373`)
  and written into data-formats.md §4.3 "Writing text". **The bracket does not match the original**, but it stands as
  a *[choice]*.
  - The original prints each Float32 with `%f` and trims trailing fraction zeros. It never fails and is lossy: 3.14159
    is written `3.14159` and reads back 2 ulps low, 1.0000001 is written `1`, and 1e-7 is written `0`.
  - The factory writes text by default (write-mode byte 1 from the constructor; no caller of slot 12 `0x0067CB10`
    found by pattern search). So any exemplar the game writes into a save is probably text.
  - The reader accepts both forms, so FreeCity4's writer need not copy the loss. Failing with `Unsupported`, writing
    text that reads back exactly, or writing binary are all compatible.

<!-- from: From applying the data-format answers (2026-10-14) -->
- ~~**EX33 — §5.1:** the core property readers apply the scalar conversion steps to every array element (e.g. a
  Float32 array read as `u32`). Right for readers that use raw access plus their own conversion? [every element]~~
  **ANSWERED 2026-10-14** from the exe (`0x005FF000`, `0x005FF510`, `0x005FFB40`, `0x005FFC00`, `0x005FFCC0`,
  `0x005FFD80`, bit copy `0x005FDDC0`) and written into data-formats.md §5.2 "The vector helpers". **The bracket does
  not match the original**, but the *[choice]* stands.
  - The original's vector helpers convert nothing. They copy `GetCount()` elements of their own width from the raw
    data pointer: the same width gives the bits, a narrower stored type is over-read past its buffer, a wider one
    gives its leading bytes, and a scalar gives an empty vector.
  - On shipped data only `0xEA0232A0` (Sint32 read as `vec<u32>`) meets a different type, and there copying and
    converting give the same bits (§7.4). So converting every element is not observable, and it replaces the over-read
    with defined values.
- Not applied: the scalar RZUnicodeChar text rule (one byte when ≤ 0xFF): the reader cannot produce that scalar
  (tag `0x0D` is not read).

## From applying the exemplar, zones draft and traffic-network answers (filed by the coordinator from the `apply8b` lane's report, 2026-10-14)

EX21–EX33, ZQ11–ZQ12 and TN26–TN28 are applied or cited. EX29 replaces the EX8 printer (17 significant digits,
half-up, trimmed, cut at 63 characters, Float32 included); TN27 fixed a real bug (travel type −1 matched every
connection type). The current answer is in brackets.

<!-- from: From applying the exemplar, zones draft and traffic-network answers (2026-10-14) -->
- ~~**EX34:** `_fcvt(x, 64)`: does it round again at the 64th place when the 17 digits reach past it (below 1e-47)?
  [yes, half-up again; 9.5e-65 prints `0.` and 61 zeros after the cut]~~
  **ANSWERED 2026-10-14** from the exe (`_fcvt` `0x009F662C` → `_fpcvt` `0x009F65CF` → `_fptostr` `0x00A0264A`,
  rounding at `0x00A0268E`–`0x00A0269E`) and written into data-formats.md §5.1 ("Below 10⁻⁴⁸ the digits are
  rounded a second time"). Yes: `_fptostr` keeps 64 + decpt digits and rounds half-up again on the next one. It
  happens below 10⁻⁴⁸ (decpt ≤ −48), not below 1e-47: from 1e-48 to 1e-47 all 17 digits still fit. 9.5e-65 prints
  `0.` and 61 zeros (**the bracket matches**). The second rounding can show: 2e-49 prints `0.`, 48 zeros and `2`
  instead of `…1999999999999`, and everything below 5e-65 prints `0`. Two findings on the way, also in §5.1: a tiny
  negative value keeps its sign (−4e-65 prints `-0`, unlike −0.0), and a cut that falls among the fraction digits
  strips trailing zeros (1.0000000000000002e-47 prints 49 characters, not 63).

<!-- from: From applying the exemplar, zones draft and traffic-network answers (2026-10-14) -->
- ~~**EX35:** Negative exponents: §4.3 says "divided", but EX23 explains `e-32` as 1/0, then 0 × ∞ (a reciprocal and
  a multiply, two roundings). Which is it? [one divide, one rounding; the NaN comes out the same]~~
  **ANSWERED 2026-10-14** from the exe (`fld1`/`fdivrp` at `0x00910507`, `fmul` at `0x0091050B`; 10ⁿ from
  `0x0091040D`) and written into data-formats.md §4.3 (Float32). A reciprocal and a multiply: 1/10ⁿ is rounded to
  the caller's precision, then multiplied in, so there are two roundings. **The bracket differs** ("one divide, one
  rounding"): at 24 bits `9e-1` gives `0x3F666667` (one division: `0x3F666666`), `5e-2` gives `0x3D4CCCCC`
  (`0x3D4CCCCD`) and `2.5e-3` gives `0x3B23D70B` (`0x3B23D70A`); `1e-1`, `3e-1` and `1e-3` agree. The NaN of
  `e-32` is the same either way, as the bracket says. A positive exponent is one multiply.

<!-- from: From applying the exemplar, zones draft and traffic-network answers (2026-10-14) -->
- ~~**EX36:** A leading `-` on a NaN result (`-e-32`): does the sign flip the NaN, as `fchs` would? [yes: `0x7FC00000`]~~
  **ANSWERED 2026-10-14** from the exe (`fchs` at `0x00910526`, after the exponent step) and written into
  data-formats.md §4.3 (Exponent). Yes: the sign is applied last and flips the NaN, so `-e-32` gives `0x7FC00000`;
  `-1e-32` gives −∞. **The bracket matches.**

<!-- from: From applying the exemplar, zones draft and traffic-network answers (2026-10-14) -->
- ~~**EX37:** EX31 lookup cache: the original keeps a looked-up record; our store keeps no cache, so a lookup between
  two package loads sees the later package here and the cached record in the original. [no cache; the two agree
  when every package loads before the first lookup]~~
  **ANSWERED 2026-10-14** from the exe (the manager's constructor `0x009744B2` creates and registers one default
  cache, vtable `0x00AD9428`, at `0x00974544`–`0x00974574`; lookup `0x0097C36E`, insert `0x0097C595`, purge
  `0x0097C48F`) and written into data-formats.md §3 ("Cache") and §12 item 2. The original does keep every record
  that loads, with a reference, and returns it before consulting any segment; registering or removing a segment
  never touches the cache. An entry leaves only by key removal, a clear, or a purge of entries no one else holds
  *[unsettled: their callers were not traced]*. **The bracket's condition holds**: the two agree whenever every
  package is registered before the first lookup, which the shipped game does in one pass at startup, the city's
  save going to the back (a lookup during that pass was not ruled out). *[choice]* No cache stands; a store that
  registers packages after lookups must keep the first record of each key looked up.

<!-- from: From applying the exemplar, zones draft and traffic-network answers (2026-10-14) -->
- ~~**EX38:** EX31 late factory: may a factory added after packages are loaded be refused instead of applied to the
  earlier packages? [refused; `add_factory` returns false]~~
  **ANSWERED 2026-10-14** from the exe (the loader, slot 46 `0x00972EAE`, picks the factory by type at every
  uncached request from `GetResource` `0x009732EB`) and written into data-formats.md §3 ("A factory added late").
  The original never refuses a factory: a late one serves every later request for its type, records in earlier
  segments included. **The bracket differs from the original** but stands as *[choice]* on one condition: every
  factory is added before the first package is loaded (condition 3 of §3 is then vacuous), which holds when
  FreeCity4 registers all its factories at startup. A refused factory must be logged as an error.

<!-- from: From applying the exemplar, zones draft and traffic-network answers (2026-10-14) -->
- ~~**EX39:** EX33 scalar: the `props` vector readers give a scalar's one value where the original gives an empty
  vector. This matters for the shipped `Uint8:0:{}` values of `0x88EDC795` and `0x88EDC796`, which no core reader
  reads yet. [kept as one value; the doc says such a reader must treat it as empty]~~
  **ANSWERED 2026-10-14** from the exe (vector helpers `vec<u8>` `0x005FFB40` and `vec<s64>` `0x005FFD80`; readers
  `0x0047A0EC`, `0x0047A16C`, `0x0047A20C`, `0x0060F690`, `0x0060F710`, `0x0069112B`, `0x0069131A`, `0x00693899`)
  and a sweep of the shipped `.dat` files, written into data-formats.md §5.2 ("Shipped scalars that only a vector
  helper reads"). **The bracket differs.** The scalar is not limited to the Living Mall text: 172 binary lot
  configurations store `0x88EDC796` as a Uint8 scalar 0 (161 of them `0x88EDC795` too, 10 also `0x88EDC793`), and
  two landfill tiles store the budget cost `0xEA54D286` as a Sint64 scalar, which the budget simulator reads with
  `vec<s64>`. The original reads every one of them as empty. A vector reader must return an empty vector for a
  scalar; only the converting getters keep the value (the landfill cost reads 1 and 10 that way).

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**EX40:** EX34 (data-formats, `format_float`): when the 63 kept characters hold no significant digit (the first one
  falls at the 64th or 65th character), does the trailing-zero strip also remove the `.`? [only `0` characters are
  stripped, so it leaves `0.` or `-0.`; marked [literal reading]]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (data-formats.md §5.1, the 63-character cut). The strip clears only `0` characters, walking back from the 63rd character and never testing the first (`0x00910686`–`0x00910697`), so it stops at the point and leaves `0.` or `-0.`. Examples: 1.5e-62 prints `0.`, −1.5e-61 prints `-0.`. The case needs more digits after the first significant one; a lone digit at the 64th character is cut without a strip.
<!-- from: glue-formats4 (2026-10-21) -->
- ~~**EX41 - section 8, factory registration:** §8 says the exemplar and cohort factories are "registered with the
  resource manager by type ID" but does not give the registration call. What are the arguments of the resource
  manager's RegisterObjectFactory (which of its two u32 arguments is the resource type and what the other one is),
  and when a second factory is registered for a type that already has one (`0x6534284A`, `0x05342861`), does the
  new one replace the old, is it refused, or is the first kept? Is there an unregister call a plug-in must make at
  shutdown? [not implemented: the plug-in registers no factory; the shadow only reads]~~ **ANSWERED 2026-10-21 (re-exstore-contracts4):** DIFFERS — slot 19 RegisterObjectFactory(class ID, type ID, factory or null); both tables are multimaps, so a second registration is added, neither replaces nor refused, and precedence is undefined after rehash; slot 20 unregisters by class ID only (type entries stay); no unregister needed, the game never calls it; replace by unregistering the game class ID and re-registering under the same class ID (§8.1).
<!-- from: glue-formats4 (2026-10-21) -->
- ~~**EX42 - section 8, CreateInstance contract:** the factory slot `CreateInstance` (`0x0067D820` /
  `0x0067D8D0`) is named by address only. What are its arguments (record or key, requested IID, out pointer and the
  two trailing u32 values), what does it return on an unknown IID, and who owns the reference of the returned object
  (is it returned with one reference the resource manager takes over, or does the manager AddRef it)? [not
  implemented]~~ **ANSWERED 2026-10-21 (re-exstore-contracts4):** DIFFERS — slot 9 CreateInstance(type, IID, out) takes no record/key; the record path is base slot 3 (record, IID, out, u32, u32), trailing values ignored by these factories; unknown type or IID stores null and returns false (object destroyed); each level releases its own reference, the GetResource caller owns one (§8.1).
<!-- from: glue-formats4 (2026-10-21) -->
- ~~**EX43 - section 8, SerialRead record position:** when the resource manager calls a factory's `SerialRead`
  (`0x0067F1E0` / `0x0067F7D0`) with a cIGZPersistDBRecord, is the record positioned at byte 0 of the resource, and
  are its bytes already QFS-decompressed (§2.6) when the index entry is compressed, or must the factory detect and
  decompress the `0x10FB` stream itself? [not implemented; the shadow decompresses itself via engine/formats qfs]~~ **ANSWERED 2026-10-21 (re-exstore-contracts4):** MATCHES — the record is freshly opened at position 0 with bytes already QFS-decompressed by the packed-file segment; the factory must not decompress (§8.1).
<!-- from: glue-formats4 (2026-10-21) -->
- ~~**EX44 - section 8, record close and object identity:** after `SerialRead` returns, who closes the record (the
  factory or the resource manager), and does the factory's result have to be the cSCResExemplar class for the rest
  of the game (callers QueryInterface it for `cISCResExemplar` slots 3-5, 8-15 and for the 25-slot
  `cISCPropertyHolder`); are those slots' contracts (§8 table) the full set a non-game implementation must provide?
  [not implemented: no replacement factory]~~ **ANSWERED 2026-10-21 (re-exstore-contracts4):** MATCHES — the resource manager closes the record (CloseRecord on success, AbortRecord on failure), never the factory; it checks only IID 0x456B8F1D and set-key, not the class; readers use cISCResExemplar/cISCPropertyHolder via QueryInterface, full reader audit stays [inferred] (§8.1).
<!-- from: glue-formats4 (2026-10-21) -->
- ~~**EX45 - section 3.1 step 3, Sku_Data file set:** the multi-package segment of `<install>\Sku_Data\` (recursive)
  is not said to hold only `.dat` files, as step 4b says for Plugins. Which files does it take: every file that
  opens as a DBPF package, only `*.dat`, or every file? And is a file without the DBPF magic in any multi segment
  skipped silently? [*.dat only; files without the magic skipped]~~ **ANSWERED 2026-10-21 (re-exstore-contracts4):** MATCHES — Sku_Data uses the multi-package segment whose own filter is *.dat (OS wildcard search, case-insensitive); files that fail to open as DBPF are skipped silently (§3.1).

<!-- from: glue-exemplars4 (2026-10-21) -->
- ~~**EX46 - section 8 table, cISCResExemplar / cohort slots 3-5 and factory slots 8, 11:** what do the cIGZSerializable slots (3 Write(cIGZOStream&), 4 Read(cIGZIStream&), 5 GetGZCLSID) of `cSCResExemplar` (vtable `0x00AAF0D0`) and `cSCResExemplarCohort` (`0x00AAEFF8`) do (stream format, class ID returned, does the game ever call them), and what do factory slot 8 (`0x0093FF03`, 3 arguments) and slot 11 SerialWrite (`0x00680030` / `0x006804D0`: the binary or text form written, parent key, kind-1 properties) do? Does any game path call them on an exemplar factory or object? [all fail and are logged; the plugin counts calls]~~ **ANSWERED 2026-10-21 (re-exemplar-contracts4):** DIFFERS — slots 3/4 (0x005DEB40) return false and slot 5 (0x004E13E0) returns 0 without touching anything; the object only acts as a cIGZSerializable for message SetExtra. Factory slot 8 (0x0093FF03) forwards (record, iid, out) to slot 3 with trailing 0, 0. SerialWrite writes binary (signature, parent key, u32 count, own properties: id, kind 0 + variant or kind 1 + key) or text (EQZT1###, ParentCohort, PropCount, named entries) by the SetWriteMode byte; reached only through factory slot 6 / manager save. No game caller found for slot 8 or SerialWrite; save callers untraced. data-formats §8.2.

<!-- from: glue-exemplars4 (2026-10-21) -->
- ~~**EX47 - section 8 table and 4.4, slots 14/15 and removal:** what are the arguments and return value of `cISCResExemplar` slot 14 (`0x0067E5B0`) and slot 15 (`0x0067DDC0`), and of the cohort's slots 14 and 15 (`0x0067DAE0`)? Does slot 14 rebuild the effective list too, or do something else? Who calls them besides SetCohort's own `vt+0x3C`? What do the exemplar holder's slots 22-24 (vtable `0x00AAF038` has 25) do and with which arguments? And holder slots 17 RemoveProperty / 18 RemoveAllProperties on an exemplar: are they removed from the own list only, the effective list only, or both, and does a parent's property for that ID become visible again? [slots 14 and 15 take no arguments and rebuild; slots 22-24 absent; removal from the own list, then the effective list rebuilt from the parent]~~ **ANSWERED 2026-10-21 (re-exemplar-contracts4):** DIFFERS — slot 14 is DebugInstanceTag(cIGZString&) (appends key, " Exemplar ", type tag, name; returns true), not a rebuild; slot 15 is rebuild(vector* dest, null = own effective list), called only by SetCohort with null. Holder slot 22 is the deleting-destructor thunk, 23/24 return false (one argument). RemoveProperty/RemoveAllProperties remove from the own list, then in the effective list substitute the parent's property when the parent has the ID (inherited value visible again) or erase. data-formats §8.2.

<!-- from: glue-exemplars4 (2026-10-21) -->
- ~~**EX48 - section 8 table, messages and parent ownership:** for exemplar holder slot 15 AddProperty(property, true) posting `0xC53D10AA` and SetCohort / SetParent(cohort, true) posting `0x853D11F1`: through which service and call is the message sent, with which data fields (the exemplar, the property ID, the cohort)? Are they ever posted with true by the game? Do `GetCohort` (exemplar slot 9) and `GetParent` (cohort slot 6) return the parent with an added reference or borrowed? Does SetCohort AddRef the new parent and release the old? [no message posted, counted; Get* borrowed; Set* AddRefs the new parent and releases the old]~~ **ANSWERED 2026-10-21 (re-exemplar-contracts4):** DIFFERS — the change message (exemplar 0xC53D10AA, cohort 0x853D11F1; also SetCohort and RemoveProperty/RemoveAll with true) is a standard message 0x46DF45E7 with data1/data2 = key words +4/+8 and SetExtra(the object), MessagePost(msg, false) on the message server; no found caller passes true. Get* borrowed; Set* AddRefs the new parent, releases the old, same pointer is a no-op. data-formats §8.2.

<!-- from: glue-exemplars4 (2026-10-21) -->
- ~~**EX49 - section 4.4 GetPropertyList, the list class:** the list `GetPropertyList` creates (`0x0090DC0F`): what is its GZCOM class ID (can a plug-in create it through cIGZCOM::GetClassObject) and its compiled vtable (which slot is "add" (slot 7), with which arguments; the order of the two Push_Front / Push_Back overloads of the SDK's cIGZUnknownList)? Which slots do the readers of the returned list call (occupant holders' EnumProperties, `0x005FEE10`)? [the replacement does not build a list: it asks the game's own factory, kept since the takeover, for an empty object of the same type through CreateInstance(type, IID 0x25216283), adds the effective properties with holder slot 15 (bool false) and returns that object's GetPropertyList]~~ **ANSWERED 2026-10-21 (re-exemplar-contracts4):** DIFFERS — the list is a 12-byte std::list object, vtable 0x00ACE650 (19 slots), no GZCOM class ID (direct construction only), QI 0x12E and 1; slot 7 pushes to the FRONT, so GetPropertyList returns descending ID order; readers use only slots 3 (count), 12 (PopFront with 0x452160EC) and 2. Slot table in data-formats §8.2.
- ~~**EX51 - section 8.2, details the EX46-EX49 answers leave open:** (a) exemplar/cohort slot 15 `rebuild(dest)` with a non-null `dest`: what is the memory layout of the `{id, property}` vector (begin/end/capacity pointers, element size, who owns the references), and what does slot 15 return? (b) holder slots 10-14 and 16 (`AddPropertyData/Object/String/U32/Variant`, `CopyAddProperty`) on an exemplar: do they update the effective list as slot 15 does, and do they post `0xC53D10AA` when their bool is true? (c) return values of `SetCohort` when the parent is unchanged (the no-op) and of slot 18 `RemoveAllProperties`. (d) `SerialWrite` binary form: the 8 bytes at `0x00AAF390` and how a kind-0 variant is written to the serial record (which serial-record slot, header bytes). [non-null dest refused (false); the other adds update both lists and post nothing; no-op SetCohort and RemoveAll return true; SerialWrite still fails and is logged. Native-versus-ours scenario `list_order` in the exemplars contract test reports (b) in game as `add13_has`.]~~ **ANSWERED 2026-10-21 (re-exemplar51-4):** DIFFERS — (a) dest is a 12-byte {begin,end,capacity} vector of 8-byte {u32 id, cISCProperty*} elements, ascending, one reference each, owned by the vector; a non-null dest is rebuilt like the null case (erase, reserve with the game heap, parent then own) and the own effective list is untouched; slot 15 returns nothing (void). (b) slots 10-13 (base bodies) ignore the bool, never post, update an existing own property in place but insert a new ID into the own list only (effective list misses it until a rebuild); slots 14 and 16 call slot 15 virtually, so they update both lists and post when the bool is true. (c) SetCohort and RemoveAllProperties return nothing (void); RemoveAll posts only when the own list was non-empty and the bool is true. (d) "EQZB1###" / "CQZB1###", then SetFieldResKey(parent or zeros), SetFieldUint32 count, per own property SetFieldUint32 id, SetFieldUint8 kind, then SetFieldVariant (kind 0, record slot 50, tag+value, no marker) or SetFieldResKey (kind 1). (e) the cohort appends " Exemplar Cohort ". (f) the name goes through GetValString (variant slot 55, §5.1 converting), type test is a raw 4-byte read. Rules in data-formats §8.2.
  - Evidence (glue-exemplar-contractfix4, game run gecf4-r2-turbo2032, black-box native-versus-ours contract test at PostAppInit; no behaviour taken from it): native returns for `SetCohort`, slot 15 and `RemoveAllProperties` read as 0 / 108 / 208 (no bool: void or garbage `al`); after a slot-13 `AddPropertyU32` the native effective list does not hold the new ID until a rebuild (`add13_has=0`, ours 1). Two more gaps: (e) the native cohort's slot 14 text is ` Exemplar Cohort (Unknown)` where ours (the §8.2 text for both) gives ` Exemplar (Unknown)`: what does the cohort body append? (f) with `ExemplarName` 0x20 held as a uint 5 the native tag shows `'5'`, ours `''`: which string conversion does slot 14 use (§5.1 getter)?
## EX54 / EX55: public packed-file contracts

<!-- from: verify-dbpf-native4 (2026-10-22) -->
- **EX54 - section 2 public packed-file raw-read return contract:** the published SDK declares `uint32_t ReadRecordRaw(key, buffer, uint32_t& recordSize)` but neither section 2 nor the header states what the return value means or the exact failure/buffer-capacity contract. What are its successful return value and in/out size semantics, including zero-length records? [compare raw payload and in/out size on valid independently bounded inputs; log the return value without assigning it success/error meaning; do not call malformed/undersized raw reads]
  **ANSWERED 2026-10-23 (re-dbpf-ex55):** DIFFERS — §2.6.1: non-null reads return and write min(capacity, stored size); zero is ambiguous; null-buffer queries clear size and return logical size for DIR-listed records. Missing/unreadable returns 0xFFFFFFFF with size unchanged; a write-open record clears size; I/O failure returns/writes 0. Vectors DFR1–DFR14 in §10.1.1.

<!-- from: verify-dbpf-native4 (2026-10-22) -->
- **EX55 - section 2 public packed-file key-list/count contract:** run `vdbpf4-r1-native-1c53058b-a` independently opens identical read-locked inputs through the public SDK packed-file factory/QI and the production Rust Package reader. All 60440 natural records (48880 compressed), and all 10 constructed records, match metadata, raw bytes and decompressed payload. The combined check `GetResourceKeyList(list&) == true && sorted(native keys) == unique(raw-file index keys) && GetRecordCount(nullptr) == unique-index count` differs for `00-empty.dat`, `03-multi-qfs.dat` (plain record, one QFS record and the DIR) and shipped `SimCity_1.dat`. What exactly are the empty-list return contract, DIR visibility and native count semantics? The existing detail line groups these three subchecks; it does not identify which subcheck differed. [preserve all three differences; no filtering of DIR, empty-list success reinterpretation, index fitting or native-sourced Rust index; add separate subcheck diagnostics before drawing a narrower conclusion]
  **ANSWERED 2026-10-23 (re-dbpf-ex55):** DIFFERS — §2.6.2: Boolean list means nonzero eligible count; all type E86B1EEF keys are hidden by enumeration; unfiltered count includes them, filtered count excludes them. Lists append without clearing. Vectors DFK1–DFK9 in §10.1.1. Original diagnostics retained below.
  - Normal-speed update: `vdbpf4-r1-normal-1c53058b-b` uses the identical frozen DLL and literal input files, reproduces the same three combined key-list/count differences and all60440 natural/10 constructed exact record transports, and ends clean2032/ref0 with a successful full save and completed shutdown. The differences are not turbo-only.
  - Round2 diagnostic update (verify-dbpf-native4, 2026-10-22): `vdbpf4-r2-own-archive-normal-02d08687` uses replacement profile `clock24` only plus the verifier. The empty fixture reports `listed=0,list_size=0,expected=0,record_count=0`, with equal empty vectors and count. The QFS fixture reports `listed=1,list_size=2,expected=3,record_count=3`; the natural archive reports `listed=1,list_size=60439,expected=60440,record_count=60440`. Both nonempty vector differences are exactly one missing key `E86B1EEF/E86B1EEF/286B1F03` (the DIR), no extra keys. All record counts equal the full unique index. The unchanged combined difference totals remain constructed2/natural1; no filtering or reinterpretation was applied. All60440 natural and10 constructed metadata/raw/payload comparisons still match, failed0/refusals0/fallback0/resync0; clean2032/ref0/fullsave/shutdown. `vdbpf4-r2-own-save-on-02d08687` also reloads a supplied saved city and independently compares its153 records (compressed80) exactly; its only enumeration difference is the missing DIR key (listed152/expected153/count153), clean2035/ref0/fullsave/shutdown. These observations narrow the question; they do not establish the unpublished enumeration or raw-read failure contract, or ownership of the packed segment or cRZFile.
  - Round2 current40 update: `vdbpf4-r2-current40-save-on-02d08687` uses precisely the coordinator's literal40-key replacement profile plus the verifier and reloads the supplied `commands4-r2-house40.sc4` fixture (SHA256 `4ad4e47b96e66110216ce0a2794c7f0cd6cdb5450295b97d320e7e94469efff8`). All153 records (compressed80) match metadata/raw/payload; enumeration remains listed152/expected153/count153, missing only the DIR key. Constructed10/10, combined differences2+1, failed0/refusals0/fallback0/resync0 unchanged. It reaches2035 with fullsave/shutdown, but ref_warnings4; the matching current40 verifier-off control also reaches2035/fullsave/shutdown with the same4 DepartmentBudget last-reference warnings and zero verifier lines. No ref0, whole-owner, or contract-resolution claim.
