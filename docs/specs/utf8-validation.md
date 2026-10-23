# Spec: standalone counted-byte UTF8IsStringValid

Status: **Supported for the normal counted, readable-buffer domain**. First dedicated publication,
2026-10-23, lane `re-utf8-validation4`, original 1 of 3, round 1. This is clean-room behaviour text;
implementers must not consult the native disassembly or decompilation. Tags follow
[foundations.md](foundations.md). The helper belongs beside, but is separate from, the owners in
[strings-language.md](strings-language.md).

Outline: §1 evidence and boundary; §2 inputs/results; §3 complete finite decision policy;
§4 compatibility consequences; §5 independent proposed byte ledger; §6 coverage and limits.

## 1. Evidence and standalone boundary

Primary evidence is the Windows GOG 1.1.641 executable, file version `1.1.641.0`, original filename
`SimCity 4.exe`, SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
The main-checkout working copy's hash matches `ghidra/export/windows/export-summary.txt`;
the export identifies image base `0x00400000`. Read-only PE header inspection also establishes
x86 machine `0x014C`, PE32 optional-header magic `0x010B` and that same image base.
Instructions and instruction bytes, queried through
`re_query.py`, establish the rules below. No original-program execution was performed.

`UTF8IsStringValid` is a medium-confidence imported name (`auto:B4`, Mac navigation `00014baa`),
not evidence of any Unicode standard. The independently inspected native body is the contiguous
range **`0x0091016F..0x0091030E` inclusive**, 416 bytes and 145 exported instructions,
ending in a return; the next exported
function starts at `0x0091030F`. All branches in the body remain within this range. It is an owned
byte walker with no calls or foreign primitive dependency. Exported direct-call records give one
caller, `IsStringValid` at `0x00910948`; no exported data references target the entry.
These are export observations, not proof that undiscovered indirect references cannot exist.

The caller pushes its count and pointer (`0x00910942..48`) and removes both stack arguments
(`0x00910970..71`). Its UTF-8/framework-code-page branch establishes provenance only; its other
dispatch paths and encoding semantics are outside this spec. This helper is neither a `cRZString`
method nor a `cRZLanguageManager`/`cRZLanguageUtility` method, and is not `ConvertStringEncoding`.

## 2. Input and result contract

| Channel | Contract and evidence |
|---|---|
| Calling convention | x86 caller-cleaned, cdecl-compatible free function; no receiver. Entry reads exactly two 32-bit stack arguments, pointer first and byte count second (`0x0091016F..73`). |
| Pointer | Address of bytes; no decoding, allocation or string object access. Reads are one byte wide. |
| Count | Explicit 32-bit byte count; no sentinel-length mode or NUL-terminated scan. Native forms end = pointer + count in 32-bit arithmetic (`0x00910178`); multibyte length guards are signed comparisons. Source-language signedness is not certified by this alone. |
| Return | Boolean **AL**: 1 exactly when the current pointer equals the calculated end, otherwise 0 (`0x00910304..0B`). Do not interpret the whole EAX register as a normalized integer Boolean: its upper bytes retain pointer-derived bits. |
| Outputs/effects | No output argument, error position, consumed-count channel, input writes, global writes or calls. Only local traversal state changes. |

The supported domain is a readable byte buffer of count **0 through `0x7FFFFFFF`**, with a
nonwrapping 32-bit pointer-plus-count. For positive counts the whole specified span must be
readable. A readable backing buffer with count zero is accepted without reading any byte.
NUL (`00`) inside the span is consumed as an ordinary one-byte value; bytes beyond the count are
irrelevant. A buffer needs no trailing NUL.

There is **no null-pointer validation** or negative-count rejection. Null-pointer use, negative
count interpretations, larger counts, wraparound and inaccessible-buffer behaviour are outside
the supported contract. Do not turn these into fabricated false results or exercise fault cases.
The entry's actual unsigned start/end comparison and final equality are documented, but do not
establish a safe general pointer-validity API.

## 3. Complete normal-domain byte decision policy *[compat]*

Begin at byte offset zero with the entire count remaining. Empty input succeeds. While bytes
remain, inspect the current byte as **unsigned**. Select exactly one row below; ranges are
inclusive. A continuation byte means **`80..BF`**. Native tests this by toggling bit `80` and
requiring the resulting unsigned byte to be below `40`; no signed-char promotion is involved.

| Current byte | Total width | Required continuation bytes | Additional condition |
|---|---:|---|---|
| `00..7F` | 1 | none | none |
| `80..C1` | reject | none read | includes stray continuations and `C0/C1` |
| `C2..DF` | 2 | positions 1 | none |
| `E0..EF` | 3 | positions 1, 2 | for `E0`, position 1 must be `A0..BF` |
| `F0..F7` | 4 | positions 1, 2, 3 | for `F0`, position 1 must be `90..BF` |
| `F8..FB` | 5 | positions 1, 2, 3, 4 | for `F8`, position 1 must be `88..BF` |
| `FC..FD` | 6 | positions 1, 2, 3, 4, 5 | for `FC`, position 1 must be `84..BF` |
| `FE..FF` | reject | none read | none |

Positions are relative to the current lead, with the lead at position 0. For every multibyte row,
the remaining count must first be at least the total width. If shorter, **fail before reading any
continuation**. Otherwise inspect continuations in ascending position order and fail at the first
one outside `80..BF`. Only after all continuations pass, test the row's additional condition.
This order matters: a low first continuation such as `80` under `E0` is structurally a continuation,
so position 2 is inspected before the `A0` lower bound rejects the sequence.

On a successful row, advance the current pointer and reduce remaining count by exactly the row's
width. Continue from the next byte. On any rejection, stop immediately, without advancing past
that lead or inspecting later bytes. Failure therefore does not certify the rest of the input.
Success requires consuming the **entire counted span**. There is no noncharacter, scalar-value,
code-point upper-bound, normalization, locale or text-property test.

| Native evidence | Decision established |
|---|---|
| `0x0091017B..93`, `0x009102FA..0x0091030E` | empty/ASCII paths, unsigned loop bound, common exit and AL equality |
| `0x00910198..C1` | reject below `C2`, two-byte width guard and continuation test |
| `0x009101C6..0x00910206` | three-byte guard, continuation order, `E0/A0` restriction |
| `0x0091020B..59` | four-byte guard, continuation order, `F0/90` restriction; upper lead `F7` |
| `0x0091025E..AA` | five-byte guard, continuation order, `F8/88` restriction |
| `0x009102AC..F7` | reject `FE/FF`, six-byte guard, continuation order, `FC/84` restriction |

In the supported domain each successful iteration consumes at least one byte, never more than
the remaining count, and every rejecting path exits. This proves finite termination and no
out-of-span read: the lead is read only when current < end; each continuation read is preceded
by a guard for its entire sequence width. No byte beyond the first rejected sequence is scanned.

## 4. Compatibility consequences *[quirk]*

This validator accepts obsolete five/six-byte forms and four-byte leads through `F7`. It accepts
surrogate encodings such as `ED A0 80` and values above modern Unicode's maximum such as
`F4 90 80 80`. It rejects short encodings through the explicit lower-bound rules in §3, but adds
no modern scalar-value restriction. An implementation must preserve these outcomes rather than
substitute a current Unicode/RFC 3629 decoder. The function's name and the caller's code-page
labels are not validation oracles.

## 5. Independent proposed byte ledger

**PROPOSED / UNEXECUTED.** Every expectation below is derived from the instruction-established
decision rows in §3, independently of a portable implementation or standard decoder. Hex cells
are literal bytes in readable backing storage, not escaped source-language strings. Unless a
count is given, count is exactly the number of listed bytes. `[]` denotes an empty span with
readable backing storage. Results are the Boolean in AL, not a full-register return value.

### 5.1 Empty, ASCII, lead boundaries and accepted extrema

For accepted rows, the whole count is consumed. The minima/maxima here describe the helper's
byte classes, not a claim about current Unicode. Single invalid leads below have count 1.

| ID | Literal bytes | AL | Derivation |
|---|---|---:|---|
| V01 | `[]` | 1 | empty, start equals end, no read |
| V02 | `00` | 1 | lowest one-byte value; no terminator semantics |
| V03 | `01 41 7F` | 1 | three one-byte iterations including upper boundary |
| V04 | `80` | 0 | lowest stray continuation |
| V05 | `BF` | 0 | highest stray continuation |
| V06 | `C0` | 0 | below first permitted lead |
| V07 | `C1` | 0 | below first permitted lead |
| V08 | `C2 80` | 1 | minimum two-byte form |
| V09 | `DF BF` | 1 | maximum two-byte form |
| V10 | `E0 9F BF` | 0 | continuations pass, extra lower bound fails |
| V11 | `E0 A0 80` | 1 | minimum three-byte form |
| V12 | `E0 BF BF` | 1 | upper continuation bound for special lead |
| V13 | `E1 80 80` | 1 | next lead removes special lower bound |
| V14 | `EF BF BF` | 1 | maximum three-byte form |
| V15 | `F0 8F BF BF` | 0 | continuations pass, extra lower bound fails |
| V16 | `F0 90 80 80` | 1 | minimum four-byte form |
| V17 | `F0 BF BF BF` | 1 | upper continuation bound for special lead |
| V18 | `F1 80 80 80` | 1 | next lead removes special lower bound |
| V19 | `F4 8F BF BF` | 1 | no upper-bound check at modern maximum |
| V20 | `F4 90 80 80` | 1 | still passes above that maximum |
| V21 | `F5 80 80 80` | 1 | F5 belongs to four-byte row |
| V22 | `F7 BF BF BF` | 1 | maximum four-byte form |
| V23 | `F8 87 BF BF BF` | 0 | continuations pass, extra lower bound fails |
| V24 | `F8 88 80 80 80` | 1 | minimum five-byte form |
| V25 | `F8 BF BF BF BF` | 1 | upper continuation bound for special lead |
| V26 | `F9 80 80 80 80` | 1 | next lead removes special lower bound |
| V27 | `FB BF BF BF BF` | 1 | maximum five-byte form |
| V28 | `FC 83 BF BF BF BF` | 0 | continuations pass, extra lower bound fails |
| V29 | `FC 84 80 80 80 80` | 1 | minimum six-byte form |
| V30 | `FC BF BF BF BF BF` | 1 | upper continuation bound for special lead |
| V31 | `FD 80 80 80 80 80` | 1 | next lead removes special lower bound |
| V32 | `FD BF BF BF BF BF` | 1 | maximum six-byte form |
| V33 | `FE` | 0 | first lead above six-byte range |
| V34 | `FF` | 0 | largest byte, invalid lead |
| V35 | `ED 9F BF` | 1 | ordinary three-byte row |
| V36 | `ED A0 80` | 1 | surrogate lower boundary has no special rejection |
| V37 | `ED BF BF` | 1 | surrogate upper boundary has no special rejection |
| V38 | `EE 80 80` | 1 | following lead still ordinary three-byte row |
| V39 | `C0 80` | 0 | rejects at lead, not by decoding an overlong value |
| V40 | `C1 BF` | 0 | rejects at lead even with a continuation available |
| V41 | `E0 80 80` | 0 | three-byte short encoding fails A0 lower bound |
| V42 | `F0 80 80 80` | 0 | four-byte short encoding fails 90 lower bound |
| V43 | `F8 80 80 80 80` | 0 | five-byte short encoding fails 88 lower bound |
| V44 | `FC 80 80 80 80 80` | 0 | six-byte short encoding fails 84 lower bound |

The five shortest accepted multibyte forms V08/V11/V16/V24/V29 have only legal continuation
bytes; their immediately lower restricted first bytes fail (C1 is rejected as a lead, and
V10/V15/V23/V28 fail after continuation checks). V09/V14/V22/V27/V32 prove the upper edges
of each width. This yields rejected contrasts without assuming Unicode scalar semantics.

### 5.2 Truncation under the actual count protocol

Every T row returns **AL=0** with the current pointer still at its lead. Its width guard fails
after reading that lead; **no continuation is read**, even if some are present. The matching
complete literals are V08, V11, V16, V24 and V29. Missing-first/middle/final cases are therefore
count failures, not implied NUL terminators or out-of-span reads.

| ID | Literal bytes | Required width | Count |
|---|---|---:|---:|
| T01 | `C2` | 2 | 1 |
| T02 | `E0` | 3 | 1 |
| T03 | `E0 A0` | 3 | 2 |
| T04 | `F0` | 4 | 1 |
| T05 | `F0 90` | 4 | 2 |
| T06 | `F0 90 80` | 4 | 3 |
| T07 | `F8` | 5 | 1 |
| T08 | `F8 88` | 5 | 2 |
| T09 | `F8 88 80` | 5 | 3 |
| T10 | `F8 88 80 80` | 5 | 4 |
| T11 | `FC` | 6 | 1 |
| T12 | `FC 84` | 6 | 2 |
| T13 | `FC 84 80` | 6 | 3 |
| T14 | `FC 84 80 80` | 6 | 4 |
| T15 | `FC 84 80 80 80` | 6 | 5 |

### 5.3 Every continuation position and both rejected sides

Each B row specifies **two independent literal cases**, both returning **AL=0**. They replace
one continuation in an otherwise accepted sequence with the adjacent rejected byte `7F` or
`C0`. Count equals the full width, so the width guard passes. All earlier continuation checks
pass, the indicated position fails, and no later continuation or lower-bound condition is
examined. Successful use of the accepted bounds `80/BF` is separately shown in §5.1.

| ID | Width | Failing position | Literal with `7F` | Literal with `C0` |
|---|---:|---:|---|---|
| B01 | 2 | 1 | `C2 7F` | `C2 C0` |
| B02 | 3 | 1 | `E1 7F 80` | `E1 C0 80` |
| B03 | 3 | 2 | `E1 80 7F` | `E1 80 C0` |
| B04 | 4 | 1 | `F1 7F 80 80` | `F1 C0 80 80` |
| B05 | 4 | 2 | `F1 80 7F 80` | `F1 80 C0 80` |
| B06 | 4 | 3 | `F1 80 80 7F` | `F1 80 80 C0` |
| B07 | 5 | 1 | `F9 7F 80 80 80` | `F9 C0 80 80 80` |
| B08 | 5 | 2 | `F9 80 7F 80 80` | `F9 80 C0 80 80` |
| B09 | 5 | 3 | `F9 80 80 7F 80` | `F9 80 80 C0 80` |
| B10 | 5 | 4 | `F9 80 80 80 7F` | `F9 80 80 80 C0` |
| B11 | 6 | 1 | `FD 7F 80 80 80 80` | `FD C0 80 80 80 80` |
| B12 | 6 | 2 | `FD 80 7F 80 80 80` | `FD 80 C0 80 80 80` |
| B13 | 6 | 3 | `FD 80 80 7F 80 80` | `FD 80 80 C0 80 80` |
| B14 | 6 | 4 | `FD 80 80 80 7F 80` | `FD 80 80 80 C0 80` |
| B15 | 6 | 5 | `FD 80 80 80 80 7F` | `FD 80 80 80 80 C0` |

### 5.4 Count, embedded NUL, stopping and ordered traces

“Consumed prefix” below is an analytic description of internal pointer advancement, **not an
output channel**. Read offsets include repeated reads of a lead when the native lower-bound
test requires them. All backing buffers here are readable for their full listed size.

| ID | Literal backing bytes | Count | AL | Decision trace |
|---|---|---:|---:|---|
| S01 | `FF` | 0 | 1 | no reads, consumed prefix 0; invalid backing byte excluded by count |
| S02 | `41 FF` | 1 | 1 | read offset 0, consume 1; offset 1 excluded |
| S03 | `00 41` | 2 | 1 | read/consume both; NUL does not stop scan |
| S04 | `00 FF` | 2 | 0 | consume 00, reject FF at offset 1 |
| S05 | `C2 00` | 2 | 0 | zero is not a continuation; consume 0 |
| S06 | `C2 80 00` | 3 | 1 | consume widths 2 then 1 |
| S07 | `C2 80` | 1 | 0 | lead read, width guard fails; backing continuation excluded |
| S08 | `41 80 41` | 3 | 0 | consume prefix 1; reject offset 1; valid suffix offset 2 unread |
| S09 | `41 80 FF` | 3 | 0 | same rejection and read offsets as S08; invalid suffix unread |
| S10 | `E1 7F 41 41` | 4 | 0 | width passes, first continuation fails; offsets 2 and 3 unread |
| S11 | `E1 7F FF FF` | 4 | 0 | same stopping point as S10 despite invalid suffix |
| S12 | `E0 9F 80 41` | 4 | 0 | read offsets 0,1,2,0; then extra bound fails; offset 3 unread |
| S13 | `E0 9F 7F 41` | 4 | 0 | read offsets 0,1,2; position 2 fails before lead re-read/bound check |
| S14 | `F0 8F 80 80 41` | 5 | 0 | read offsets 0,1,2,3,0; then extra bound fails; offset 4 unread |
| S15 | `F8 87 80 80 80 41` | 6 | 0 | read offsets 0,1,2,3,4,0; then extra bound fails; offset 5 unread |
| S16 | `FC 83 80 80 80 80 41` | 7 | 0 | read offsets 0,1,2,3,4,5,0; then extra bound fails; offset 6 unread |
| S17 | `41 C2 80 E0 A0 80 F0 90 80 80 F8 88 80 80 80 FC 84 80 80 80 80 00` | 22 | 1 | successful widths 1,2,3,4,5,6,1 total 22 |
| S18 | `C2 80 BF` | 3 | 0 | consume 2; extra stray continuation fails as the next lead |

These are **107 literal cases**: 44 V cases, 15 T cases, 30 B cases and 18 S cases.
Native executions: **0**. No portable implementation has been run against them. Analytic checks
of finite byte classes and table consistency do not convert these fixtures into native passes.

## 6. Coverage and limits

The lead partition covers all 256 byte values: 128 one-byte values, 68 invalid leads, and
30/16/8/4/2 leads of widths 2/3/4/5/6 respectively. The following checklist separates body
coverage from execution evidence; **Supported** means statically established normal-domain
behaviour, not a native-test pass.

| Branch group | Status | Literal evidence / coverage |
|---|---|---|
| Entry empty vs nonempty | Supported | V01, S01 vs all positive-count cases |
| One-byte lower/upper bounds, embedded zero | Supported | V02–V03, S03–S06 |
| Invalid lower lead range `80..C1` | Supported | V04–V07, V39–V40 |
| Widths 2–6, both lead edges and exceptional/ordinary split | Supported | V08–V32; special lead vs next lead in V11/V13, V16/V18, V24/V26, V29/V31 |
| Five signed remaining-width guards, both outcomes | Supported | T01–T15 fail at every shorter count; V08/V11/V16/V24/V29 pass at exact width |
| All 15 continuation checks, both outcomes | Supported | B01–B15 give 30 rejecting literals; accepted extrema give `80/BF` successes at every position |
| Four extra first-continuation bounds, both outcomes | Supported | V10/V11, V15/V16, V23/V24, V28/V29; special-lead upper bounds V12/V17/V25/V30 |
| Upper invalid leads `FE..FF` | Supported | V33–V34 |
| Advance, repeat, exact-end success, invalid next lead | Supported | S17, S18 |
| Failure exit and ordered read stopping | Supported | S08–S16; no error/output channel exists |
| Full normal-domain body | Supported | §1–§3 cover the entire contiguous body, no callee or omitted normal branch |

| Item | Status | Scope |
|---|---|---|
| Standalone native body, every normal-domain branch | Supported | §2–§3: entry, all eight lead classes, five width guards, all continuation positions, four extra lower bounds, advancement, loop and exit |
| Calling interface and result channel | Supported | two stack words; AL Boolean; no optional outputs |
| Other address/count domains | Unknown/unsupported | no pointer-validity guard; no supported negative/overflow/unreadable domain |
| Foreign dependencies | None in this body | no calls; no host validator/converter substituted for the byte rules |
| Independent byte fixtures | Supported analytically; unexecuted | §5: 107 literal cases, full lead boundaries, every truncation and continuation position, extra bounds and stopping contrasts; no runtime parity claim |
| Dispatcher and other encoding helpers | Not studied | caller admission only; no wider owner credit |

This is one standalone body, not certification of the encoding library or all Unicode semantics.
No ABI rows, engine changes, game runs or source-derived test oracle accompany it.
