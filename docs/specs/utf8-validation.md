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
the export identifies image base `0x00400000`. Instructions and instruction bytes, queried through
`re_query.py`, establish the rules below. No original-program execution was performed.

`UTF8IsStringValid` is a medium-confidence imported name (`auto:B4`, Mac navigation `00014baa`),
not evidence of any Unicode standard. The independently inspected native body is the contiguous
range **`0x0091016F..0x0091030E` inclusive**, 416 bytes, ending in a return; the next exported
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

The literal cases and analytic traces for this section are added in the next publication milestone.
No fixture or native execution is claimed by this initial rule publication.

## 6. Coverage and limits

| Item | Status | Scope |
|---|---|---|
| Standalone native body, every normal-domain branch | Supported | §2–§3: entry, all eight lead classes, five width guards, all continuation positions, four extra lower bounds, advancement, loop and exit |
| Calling interface and result channel | Supported | two stack words; AL Boolean; no optional outputs |
| Other address/count domains | Unknown/unsupported | no pointer-validity guard; no supported negative/overflow/unreadable domain |
| Foreign dependencies | None in this body | no calls; no host validator/converter substituted for the byte rules |
| Independent byte fixtures | Partial | forthcoming analytic ledger, no execution |
| Dispatcher and other encoding helpers | Not studied | caller admission only; no wider owner credit |

This is one standalone body, not certification of the encoding library or all Unicode semantics.
No ABI rows, engine changes, game runs or source-derived test oracle accompany it.
