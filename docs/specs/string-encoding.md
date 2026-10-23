# Spec: standalone string encoding conversion

## 1. Scope, evidence and ownership

Windows 1.1.641 instruction evidence; no native execution or implementation output.
This is the first dedicated whole-converter research round, **not** the first research
on every instruction in it. The earlier `re-strings4` round-2 report explicitly left
ConvertStringEncoding unstarted after Utility/Manager. The inherited final storage
owner excludes this separate converter. FS16 already specifies a **partial** UTF-8
to UTF-16 path used by MakeLower: [graphics-formats §3.11](graphics-formats.md#311-regp-12-and-later-compat),
specifically its FS16 discussion. Its low-16-bit, no-surrogate, invalid-input and
truncation facts remain canonical prior findings. String storage, language startup,
UTF-8 validation, Thai, font and TextEdit owners are outside this round.

Addresses below cite original instructions, queried from the matching Windows export
with `re_query.py asm --bytes`; decompilation is navigation only. The six candidate
metadata sizes total 2,159 bytes; that total is not proof of six independent bodies
or complete coverage. The per-entry ledger below determines the actual scope.

## 2. Raw counted interface and dispatch

The core at `00910E18` has six stack arguments in this order: source address,
32-bit source count `N`, source selector, destination address, address of a 32-bit
capacity/result count `C`, destination selector. Its return is a full 32-bit EAX
count, not an AL boolean. Counts are source/destination encoding units: bytes except
for selector `10` hexadecimal, whose units are little-endian 16-bit words.

For `N = 0`, store zero in `C`, return zero, and do not touch destination or resolve
selectors (`00910E1F–00910E2D`). Otherwise resolve source first, then destination:
`FFFFFFFE` becomes `8`; `FFFFFFFF` calls `GetCurrentSystemCodePage` (`00910014`)
separately for that occurrence. Other values pass through. Selector `8` means this
converter's UTF-8, selector `10` hexadecimal means its UTF-16 word representation.
Treat system-codepage resolution as an existing raw primitive, not a fixed 1252
constant or a request to reimplement locale startup.

Before testing equality, scan **all N source bytes** if the resolved source is not
`10`. If every byte is at most `7F`, replace the source selector with `0`
(`00910E6A–00910E7F`). Embedded zero is an ordinary counted byte. There is no
sentinel-length scan in this core; `FFFFFFFF` in the count slot is not a supported
request for strlen.

Dispatch order (`00910E81–00910EAF`, `00911258–00911323`):

1. Equal selectors, source `0` to any non-`10` target, or any non-`10` source to
   target `0`: raw copy path. Copy `min(N,C)` units; double the byte size only for
   target `10`. Write that unit count to `C` and return it. No terminator is added.
2. Source `10` to target `0`: copy the low byte of each of `min(N,C)` source words;
   write/return that count (`00910EB3–00910EDA`). No range validation.
3. Remaining distinct source/destination addresses dispatch to the conversion paths.
   A same-address conversion reaching `00910EA4` returns zero **without changing C**.
   Alias, overlap and in-place operation are otherwise outside the supported domain.
4. Source `0` to target `10`: widen each counted byte to one word, at most `min(N,C)`;
   write/return that count (`00911260–00911289`).

The raw copy path uses memcpy and does not validate equal-selector byte forms.
There is no uniform size-query mode: the owned loops write directly and the copy
path calls memcpy. Only the direct OS branches pass capacity zero through to an
OS sizing call (§3.3). Do not use a null destination to query the owned loops.

## 3. Conversion branches

### 3.1 UTF-8 bytes to UTF-16 words

This composes the **prior FS16 branch**, not new independent whole-converter
coverage. Start with zero source bytes consumed and zero destination words written.
While input remains and a destination word fits, inspect the next sequence. Write
one word only after the whole sequence passes the checks below. On invalid or
incomplete input, stop before consuming its first byte. On capacity exhaustion,
stop before examining another sequence. Return the source bytes consumed and store
the words written in `C` (`00910EF0–00911147`). There is no separate error flag,
replacement character, terminator, normalization or rollback of earlier words.

Every continuation must be `80..BF`. All byte values below are hexadecimal;
the second-byte minimum is additional to that continuation check.

| First byte | Total bytes | Additional check | Word produced |
|---|---:|---|---|
| `00..7F` | 1 | none | zero-extended byte |
| `C2..DF` | 2 | none | usual 5+6 payload bits |
| `E0..EF` | 3 | `E0` requires second byte ≥ `A0` | low 16 bits of combined payload |
| `F0..F7` | 4 | `F0` requires second byte ≥ `90` | low 16 bits of combined payload |
| `F8..FB` | 5 | `F8` requires second byte ≥ `88` | low 16 bits from last three bytes |
| `FC..FD` | 6 | `FC` requires second byte ≥ `84` | low 16 bits from last three bytes |
| `80..C1`, `FE..FF` | — | invalid | no word |

For 3–6-byte forms, the retained word can be calculated from the low four bits of
the third-last byte, then the six payload bits of each of the last two bytes. The
actual word store is at `0091112C`; byte availability and signed remaining-length
checks precede sequence reads (`00910F3D`, `00910F73`, `00910FD8`, `00911048`,
`009110C4`). Keep supported counts below signed/pointer overflow boundaries.
Surrogate-valued 3-byte forms and values above U+10FFFF are accepted. A 4-byte
sequence makes one word, never a surrogate pair. The prior FS16 U+1F600 → `F600`
and invalid Latin-1 Café → `caf` MakeLower witnesses remain **PRIOR**, not new
literal full-converter tests. This is a converter contract, not an assertion of
equivalence to [UTF8IsStringValid](utf8-validation.md).

### 3.2 UTF-16 words to UTF-8 bytes

Each source item is a **zero-extended 16-bit load** (`00911177`), independently
encoded; surrogate pairs are not combined or validated. A word below `0080` uses
one byte, below `0800` two, otherwise three. For two bytes, take bits 6–10 with
prefix `C0` and bits 0–5 with prefix `80`. For three bytes, take bits 12–15 with
prefix `E0`, bits 6–11 with prefix `80`, then bits 0–5 with prefix `80`.
Although the body contains 4–6-byte output arms, their thresholds cannot be reached
by that word load (`0091117F–009111C0`); do not credit them as wider input support.

**Consumption precedes capacity testing.** Read a word and advance the source by
two bytes; determine its output length; if that entire encoded sequence does not
fit the remaining byte capacity, stop without emitting any of that sequence
(`0091117D`, `009111C1–009111C7`). EAX includes that last read word. Thus a
nonempty input with capacity zero returns **1**, with `C=0`. When an earlier word
was emitted, its bytes remain. There is no general `EAX=N` ⇒ complete output rule:
if the last source word does not fit, EAX still equals N.

On each successful item the continuation bytes are stored from the last byte
backwards, then the first byte (`00911214–0091123A` for reachable 3-byte forms).
Only after those stores does the destination advance. At exit return source words
read and store bytes written in C (`00911247–00911251`). Embedded zero emits byte
`00` and conversion continues while the explicit count has remaining words.

### 3.3 Direct OS branches and unsupported pairs

After the copy/narrow/owned UTF paths, source `0` to `10` uses the widening path
already specified. A remaining non-`8`, non-`10` source to `10` calls
**MultiByteToWideChar** with that source selector as CodePage, flags `0`, the exact
source address and positive counted byte length N, destination address and C as
word capacity (`009112CA–009112DF`). A remaining source `10` to a non-`8`, non-`0`
target calls **WideCharToMultiByte** with target CodePage, flags `0`, source N
words, destination C bytes, default-character pointer to the single byte `20`
(space), and a local used-default BOOL initially zero (`009112E5–00911304`).
The used-default reply is never tested.

Each individual OS call is a narrowly typed **raw primitive**: its own numeric
reply, output bytes/words and count are inputs to this orchestration contract.
It must not return a host-computed whole ConvertStringEncoding outcome. Codepage
validity, OS-specific mapping and flags-0 substitution remain at that boundary;
the label “UTF-8” must not silently redirect numeric Windows code page `65001`
to selector `8`.

Both direct branches test the OS integer reply: nonzero means write it into C
and return original N; zero means write C=0 and return 0 (`0091130A–00911316`).
They do not restore destination bytes after failure and do not inspect last-error.
For C=0 these exact API calls are sizing requests under the public Win32 API
contract: a positive required-size reply is stored in C, EAX is N, and the OS
writes no output. This behavior does not extend to the owned UTF or copy paths.
For nonzero insufficient capacity, only the individual OS primitive's specified
mutations may be assumed; no local atomicity guarantee is added.

Remaining unequal legacy-codepage pairs (neither side `8` nor `10`, after the
ASCII/selector-0 shortcuts) do not perform an OS conversion. They write C=0,
return 0, and leave destination untouched (`00911318–0091131E`). Unknown selector
values are not all rejected at entry: equal selectors copy, ASCII can copy, and
an otherwise unsupported selector on an OS branch is passed as the raw CodePage.

### 3.4 Two-stage UTF-16 bridge (`00910B3C`)

Distinct nontrivial conversions with exactly one UTF-8 side and neither side
UTF-16 use this helper (`00911292–009112C0`); source selector 0 shortcuts have
already run. The helper has five stack arguments: source address, resolved source
selector, final destination address, address of final C, resolved target selector.
It receives original source N separately in **EDI**, which it preserves. Do not
infer a normal six-argument public signature from its auto name or decompilation.

Initialize a temporary capacity/count T to `2*N+4`. Use a 1,024-byte local scratch
buffer when T ≤ 1,024; otherwise call the allocation holder primitive `0045C590`
with T **bytes** and use the holder's first pointer field. Its other holder words
start zero. T is nevertheless passed to the core as a **word** capacity. This
does not license a larger accessible buffer: supported input/primitive outcomes
must fit the actual allocation (`00910B48–00910B80`). Overflow and allocation
failure are outside the supported domain.

First call core(source, N, source-selector, scratch, &T, `10`). Only if EAX equals
original N call core(scratch, T, `10`, final-destination, &C, target-selector).
Only if this second EAX equals T is the helper considered successful. Success
returns original N. Either comparison failing writes **final C=0**, then returns
0. Already-written final destination bytes are **not undone**. A first-stage
failure never calls the second stage, so leaves the final destination unchanged.
Both success and failure release a nonnull holder pointer via raw cleanup
primitive `0090CF63` (`00910B88–00910BCD`). Its export name resembles a string
destructor; that name does not turn scratch storage into a newly researched
string owner or imply AddRef/Release semantics.

In particular, §3.2's read-before-capacity behavior can satisfy the bridge's second
comparison even when the last word emitted no UTF-8 bytes. The helper tests
consumption counts, not a separate completeness status. An OS-stage failure's
zero return is tested through the same comparisons; used-default flags are not.

## 4. String-interface wrappers

All four are distinct complete instruction ranges, not aliases. The interface
methods they call remain raw individual existing string primitives: vtable byte
offset `+0C` counted assignment, `+14` data access, `+18` byte-string access, `+1C`
length. Their internal allocation/storage/reference behavior is not reopened.
No wrapper explicitly calls AddRef or Release. Do not infer a NUL scan from the
byte-string access method: these wrappers still pass the explicit length.

| Entry | Stack arguments in order | Normal return channel |
|---|---|---|
| `00911358` | raw source, N, source-selector, output string | 32-bit source count |
| `009113EF` | input string, raw destination, &C, target-selector | 32-bit count, branch-dependent as below |
| `00911466` | input string, source-selector, output string | boolean **AL** only |
| `0091152F` | input string, output string, target-selector | boolean **AL** only |

### 4.1 Raw source to framework string (`00911358`)

Source `FFFFFFFE` or `8` directly calls output counted assignment(raw, N), then
returns N; no validation. Source `FFFFFFFF` first resolves system codepage and
does the same if it resolves to 8. Otherwise form scratch of `6*N` **bytes**:
strictly below 512 uses the local 512-byte buffer and null heap pointer;
512 or more calls GlobalHeapAlloc (`005E55E0`). In the supported successful
allocation domain use its pointer. Set scratch byte 0 to zero; call core(raw,N,
resolved-source,scratch,&T,8), with T initially `6*N`. Preserve core's EAX;
**unconditionally** assign output(scratch,T), call GlobalHeapFree (`005E5620`)
on the saved heap pointer (including null on the stack path), and return the
saved core EAX (`0091138D–009113EE`). A failed or partial conversion still replaces
the output with the reported output prefix. Assignment's own return is ignored.

### 4.2 Framework string to raw destination (`009113EF`)

Target `FFFFFFFE`, `8`, or system resolving to 8: call length; if C exceeds it,
call length again and store that reply in C. Call byte-string access, memcpy
exactly C bytes, return C (`0091140A–0091143D`). This path reports bytes copied,
not full source consumed. It adds no terminator.

Other targets: call length, then byte-string access, then return the core result
for (access-result, length, 8, raw-destination, &C, resolved-target)
(`0091143F–00911465`). The core's output count and partial-write behavior apply.
For example, insufficient UTF-16 capacity returns source bytes consumed, not
the copied-prefix byte count of the target-8 branch.

### 4.3 String interpreted in a source encoding (`00911466`)

Source `FFFFFFFE`, `8`, or system resolving to 8: if input/output objects differ,
call input length, then input data access, then output counted assignment;
if identical, make none of those calls. Return AL=1 in either case
(`0091148E–009114B1`). Resolved source `10` returns **AL=0 without getters or
output mutation** (`009114B3–009114B7`). Other source selectors use the input's
length N, scratch T=`6*N` bytes and the same 512-byte threshold as §4.1; call
byte-string access and core(source,N,selector,scratch,&T,8).

Before calling output counted assignment(scratch,T), compute boolean
`core EAX == N`. Assignment occurs regardless of that boolean, followed by
GlobalHeapFree(saved-pointer), and the saved boolean is returned in AL
(`009114B9–0091152E`). Thus false need not mean output unchanged. Whole EAX
upper bits are not a specified boolean return value. The special resolved-16
rejection precedes getters; do not generalize it to raw-source wrapper §4.1.

### 4.4 Framework string to target-encoded string (`0091152F`)

Target `FFFFFFFE`, `8`, or system resolving to 8 has the identical-object /
length→data→assignment path and returns AL=1. Otherwise call input length N,
use scratch T=`6*N` bytes with the same 512-byte threshold, set scratch[0]=0,
call byte-string access and core(source,N,8,scratch,&T,resolved-target). Compute
`core EAX == N`, unconditionally assign output(scratch,T), free saved heap
pointer, then return the boolean in AL (`0091157A–009115EF`).

**T is passed unchanged to counted byte assignment even for target `10`.**
Core reports words for that target, so this wrapper stores only that many
**bytes** from the scratch word representation; it does not double T
(`009115D2–009115DD`). This observation is local wrapper orchestration, not a
new string-storage algorithm. The AL success check does not detect that mismatch.

### 4.5 Supported domain and state

Valid readable counted input, valid writable count/output storage, distinct raw
buffers, ordinary counts without arithmetic/pointer overflow, normally returning
primitives, and scratch allocations that succeed and fit all writes are supported.
Allocation failure, invalid/null pointers outside a documented OS sizing request,
raw alias/in-place/overlap, overflow and nonreturning behavior are **Unsupported**.
Large-allocation wrappers have an observed null-allocation fallback to their local
buffer with the original large T; this is not a safe-failure or rollback contract.
No converter-owned persistent text state or reference acquisition is present;
system-selector resolution may use the existing cached system-codepage primitive.

## 5. Proposed literal evidence ledger

All vectors here are **PROPOSED / UNEXECUTED**, derived from instructions rather
than a standard decoder or implementation result. Distinct source/destination
buffers are assumed. Destination bytes not explicitly changed keep their sentinel.
Encoding selectors `8`, `10`, `0`, `FFFFFFFE` and `FFFFFFFF` are hexadecimal;
named Windows code pages such as 1252, 1251 and 65001 are decimal. All raw bytes
and words are hexadecimal. “No OS” excludes resolution of the system selector.
Unused source bytes/words shown after the explicit N prove that no sentinel scan
is intended. OS reply rows are **stipulated individual primitive replies**, not
claims that an OS has been run or that any host decoder produced the answer.

| ID | Raw input; N; selectors source → target | Initial destination; C | Expected destination; C; EAX; calls |
|---|---|---|---|
| SE01 | `41 00 42`; 3; `1252 → 8` | `CC CC CC CC`; 2 | `41 00 CC CC`; 2; 2; memcpy(2), no OS conversion (ASCII shortcut) |
| SE02 | `E9 41`; 2; `1252 → 1252` | `CC CC CC CC`; 1 | `E9 CC CC CC`; 1; 1; memcpy(1), no OS conversion |
| SE03 | words `12E9 0042`; 2; `10 → 0` | `CC CC CC`; 1 | `E9 CC CC`; 1; 1; no OS conversion |
| SE04 | `7F 00 41`; 3; `8 → 10` | words `CCCC CCCC CCCC CCCC`; 3 | words `007F 0000 0041 CCCC`; 3; 3; no OS conversion (widen path) |
| SE05 | empty; 0; `FFFFFFFF → FFFFFFFF` | `CC CC`; 7 | `CC CC`; 0; 0; no codepage-resolution or conversion calls |
| SE06 | `41 00 42`; 1; `FFFFFFFE → 10` | words `CCCC CCCC`; 2 | words `0041 CCCC`; 1; 1; no OS, byte after N is not read |
| SE07 | words `D800 0042`; 1; `10 → 8` | `CC CC CC CC`; 3 | `ED A0 80 CC`; 3; 1; no OS; no following-low-surrogate requirement |
| SE08 | words `D83D DE00`; 2; `10 → 8` | `CC CC CC CC CC CC CC`; 6 | `ED A0 BD ED B8 80 CC`; 6; 2; two independent 3-byte items, no OS |
| SE09 | words `0041 0000 0800 0042`; 4; `10 → 8` | `CC CC CC CC CC CC CC`; 6 | `41 00 E0 A0 80 42 CC`; 6; 4; exact capacity, embedded zero continues |
| SE10 | words `0800 0042`; 1; `10 → 8` | `CC CC CC CC`; 2 | `CC CC CC CC`; 0; **1**; word read, sequence does not fit |
| SE11 | words `0041 0800 0042`; 3; `10 → 8` | `CC CC CC CC`; 2 | `41 CC CC CC`; 1; **2**; second word read but not emitted |
| SE12 | words `0041 0042`; 1; `10 → 8` | `CC CC`; 0 | `CC CC`; 0; **1**; zero capacity still reads first word |
| SE13 | `41 C2 A2 42`; 4; `8 → 10` | words `CCCC CCCC CCCC`; 1 | words `0041 CCCC CCCC`; 1; 1; no read/validation of C2 item after output fills |
| SE14 | `41 E0 A0`; 3; `8 → 10` | words `CCCC CCCC CCCC`; 3 | words `0041 CCCC CCCC`; 1; 1; truncated 3-byte item unconsumed |
| SE15 | `C0 AF 42`; 3; `8 → 10` | words `CCCC CCCC`; 2 | words `CCCC CCCC`; 0; 0; invalid lead, no replacement |
| SE16 | `ED A0 81 42`; 4; `8 → 10` | words `CCCC CCCC CCCC`; 2 | words `D801 0042 CCCC`; 2; 4; encoded surrogate accepted |
| SE17 | `F5 80 80 81`; 4; `8 → 10` | words `CCCC CCCC`; 1 | words `0001 CCCC`; 1; 4; value beyond scalar range accepted |
| SE18 | `F8 88 80 80 81`; 5; `8 → 10` | words `CCCC CCCC`; 1 | words `0001 CCCC`; 1; 5; old 5-byte minimum accepted |
| SE19 | `FC 84 80 80 80 82`; 6; `8 → 10` | words `CCCC CCCC`; 1 | words `0002 CCCC`; 1; 6; old 6-byte minimum accepted |
| SE20 | `F8 87 BF BF BF`; 5; `8 → 10` | words `CCCC CCCC`; 1 | words `CCCC CCCC`; 0; 0; 5-byte overlong rejected |
| SE21 | `FC 83 BF BF BF BF`; 6; `8 → 10` | words `CCCC CCCC`; 1 | words `CCCC CCCC`; 0; 0; 6-byte overlong rejected |
| SE22 | `C1 80`; 2; `8 → 8` | `CC CC CC`; 2 | `C1 80 CC`; 2; 2; same-selector memcpy, no validation |
| SE23 | `C1 80`; 2; `0 → 10` | words `CCCC CCCC CCCC`; 2 | words `00C1 0080 CCCC`; 2; 2; selector-0 widening, not UTF8 decode |
| SE24 | words `12E9 0000`; 2; `10 → 10` | words `CCCC CCCC CCCC`; 1 | words `12E9 CCCC CCCC`; 1; 1; memcpy **2 bytes** |
| SE25 | `E9 41`; 2; `1252 → 1251` | `CC CC CC`; 3 | `CC CC CC`; 0; 0; unsupported unequal legacy pair, no OS |
| SE26 | `41 42`; 2; `1252 → 1251` | `CC CC CC`; 1 | `41 CC CC`; 1; 1; ASCII shortcut supersedes unsupported-pair path |
| SE27 | `E9 41`; 2; `FFFFFFFF → FFFFFFFF`, each resolver reply 1252 | `CC CC CC`; 1 | `E9 CC CC`; 1; 1; **two ordered resolver calls**, then equal-selector memcpy(1) |
| SE28 | `E9`; 1; `FFFFFFFF → FFFFFFFE`, resolver reply 8 | words `CCCC CCCC` as four raw sentinel bytes; 4 | raw bytes `E9 CC CC CC`; 1; 1; one resolver call then memcpy(1) |

SE28's destination is byte-oriented despite being displayed initially as paired
sentinels; its final raw bytes are **`E9 CC CC CC`**.
The equality path does not inspect the invalid single-byte UTF8 form. This is an
explicit asymmetry between selectors and supplied memory, not a UTF16 conversion.

### 5.1 Individual OS and bridge reply ledger

Each call below includes its exact raw count and stipulated reply. No reply is a
proxy for an entire converter result. `MB` denotes MultiByteToWideChar and `WC`
WideCharToMultiByte; both use flags 0. All final destinations start with the shown
sentinel independently of scratch initialization.

| ID | Core input and primitive trace | Expected final destination; C; EAX |
|---|---|---|
| SE29 | `E9 41`, N=2, `1252→10`, destination words `CCCC CCCC CCCC`, C=3; MB(1252, bytes `E9 41`, 2, cap=3) replies 2, writes words `00E9 0041` | words `00E9 0041 CCCC`; 2; 2 |
| SE30 | Same input, C=0, valid destination words `CCCC CCCC`; MB(1252, N=2, cap=0) sizing reply 2, no writes | words `CCCC CCCC`; 2; 2 |
| SE31 | `E9 41`, N=2, `1252→10`, words `CCCC CCCC`, C=1; MB replies 0 with a separately stipulated unchanged destination | words `CCCC CCCC`; 0; 0; only C is locally reset, not output bytes restored |
| SE32 | words `4E00`, N=1, `10→1252`, raw `CC CC CC`, C=3; WC(1252, N=1, cap=3, default byte `20`, used-default starts 0) replies 1, writes `20`, sets used-default=1 | `20 CC CC`; 1; 1; used-default ignored |
| SE33 | words `00E9`, N=1, `10→65001`, raw `CC CC`, C=2; WC receives numeric 65001 with default pointer `20`, replies 0, no writes stipulated | `CC CC`; 0; 0; numeric 65001 was not converted to selector 8 |
| SE34 | `E9`, N=1, `1252→8`, raw `CC CC CC`, C=1; bridge T=6, stack scratch; MB(1252,N=1,cap=6 words) replies 1, writes word `00E9`; owned second stage `10→8` cap=1 reads word, emits nothing, EAX=1,C=0 | `CC CC CC`; 0; **1**; bridge declares consumption success despite empty final output |
| SE35 | `E9 41`, N=2, `1252→8`, raw `CC CC CC CC`, C=1; bridge T=8; MB replies 2 with words `00E9 0041`; owned stage cap=1 consumes first word only, returns 1,C=0; bridge comparison 1≠2 | `CC CC CC CC`; 0; 0 |
| SE36 | `41 E9 42`, N=3, `1252→8`, raw `CC CC CC CC`, C=1; MB with initial T=10 replies 3, writes words `0041 00E9 0042`; owned stage emits `41`, reads E9 but cannot emit it, returns 2,C=1; bridge 2≠3 | `41 CC CC CC`; **0**; 0; previously emitted byte survives failure and C reset |
| SE37 | `41 C0 AF`, N=3, `8→1252`, raw `CC CC CC`, C=3; first owned stage produces word `0041`, returns 1,T=1; bridge 1≠3; **no WC call** | `CC CC CC`; 0; 0; prefix scratch never reaches destination |
| SE38 | `C3 A9`, N=2, `8→1252`, raw `CC CC CC`, C=3; first stage T=8→1, EAX=2, word `00E9`; WC(1252,N=1,cap=3,default=`20`) replies 1, writes `E9`, used-default=0 | `E9 CC CC`; 1; 2; bridge success returns original byte count |
| SE39 | Same as SE38 but WC reply 0, with unchanged destination stipulated; second core writes C=0,EAX=0; bridge comparison 0≠1 | `CC CC CC`; 0; 0 |

The observed local branches alone derive SE34–SE39's outer counts and writes;
the ledger does not calculate OS mapping with a modern UTF decoder. A primitive
failure with stipulated partial writes would retain those writes as well.

### 5.2 Wrapper call/order and count ledger

`Assign(bytes,count)`, `Length`, `Data`, `Bytes`, `Alloc`, `Free` below are raw
individual replies/calls; output string assignment itself owns its storage and
reference semantics. Byte prefixes passed to Assign are literal, not C strings.

| ID | Input / wrapper / stipulated primitive results | Expected trace and return |
|---|---|---|
| SE40 | `00911358`, raw `C1 80 00`, N=2, source `8`, output initially bytes `5A 5A 5A` | Assign(`C1 80`,2), return EAX=2; no validation, scratch or Free |
| SE41 | `00911358`, raw words `D800`, N=1, source `10`, output initially `5A 5A`; scratch cap=6 | set scratch[0]=0; owned core emits `ED A0 80`, EAX=1,T=3; Assign(`ED A0 80`,3); Free(null); return EAX=1 |
| SE42 | `00911358`, raw `E9`, N=1, source 1252, output initially `5A`; MB first bridge stage reply 0, no scratch writes stipulated | bridge C reset 0; Assign(empty,0); Free(null); return EAX=0; output is replaced even on failure |
| SE43 | `009113EF`, input bytes `41 00 42`, Length replies 3 twice, Bytes returns those bytes; target `8`; destination `CC CC CC CC`, C=5 | Length→Length→Bytes→memcpy(3); bytes `41 00 42 CC`, C=3,EAX=3 |
| SE44 | Same wrapper/input, Length=3, target `8`, destination `CC CC CC`, C=1 | Length once→Bytes→memcpy(1); `41 CC CC`, C=1,EAX=1; return differs from full source length |
| SE45 | `00911466`, distinct input/output; source `8`, Length=3, Data=`41 00 42`; old output `5A` | Length→Data→Assign(`41 00 42`,3); AL=1; no scratch/free |
| SE46 | `00911466`, source `10`, any valid input/output objects, output initially `5A` | no getters or assignment; output stays `5A`; AL=0 |
| SE47 | `00911466`, input/output same object, source `FFFFFFFE` | no getters, assignment, allocation or free; AL=1 |
| SE48 | `0091152F`, input bytes `C3 A9`, Length=2, Bytes reply; target `10`; old output `5A 5A`; scratch T=12 | Length→Bytes→core emits little-endian `E9 00`, T=1,EAX=2→Assign(**`E9`,1**)→Free(null); AL=1 despite only one byte assigned |
| SE49 | `0091152F`, input `41 C0 AF`, Length=3, target 1252; Bytes reply, old output `5A`; bridge first stage fails as SE37 | Length→Bytes→core EAX=0,T=0→Assign(empty,0)→Free(null); AL=0; output still replaced |
| SE50 | `00911358`, N=85 words `0041` repeated 85 times, selector `10`, distinct output; actual allocated scratch not needed | initial T=510<512, stack path; core EAX=85,T=85, Assign(`41`×85,85), Free(null), EAX=85 |
| SE51 | `00911358`, N=86 words `0041` repeated 86 times, selector `10`; Alloc(516) supplies valid scratch | Alloc(516)→core EAX=86,T=86→Assign(`41`×86,86)→Free(exact allocated pointer); EAX=86 |

SE50–SE51 describe repeated **literal word 0041**, not generated Unicode oracle
output. For the private bridge, N=510 makes initial T=1024 and uses stack;
N=511 makes T=1026 and calls holder allocation for **1026 bytes**, not 2052.
These arithmetic boundary observations are proposed and unexecuted too.

## 6. Attribution ledger

“Supported” below means the normal owned control flow specified here, subject to
§4.5. It does not claim OS/CRT/allocator internals, all possible native executions,
or a blanket 2,159-byte verification total.

| Candidate entry / bytes | Actual range and role | Disposition |
|---|---|---|
| `00910B3C` / 151 | `00910B3C–00910BD2`, private EDI-count UTF-16 bridge | **Supported** orchestration §3.4; allocation/cleanup are raw foreign primitives |
| `00910E18` / 1344 | `00910E18–00911357`, counted core | **Supported** normal dispatch, local loops and return/count mutation §§2–3; UTF8→UTF16 includes **prior Partial FS16** evidence; unreachable wider-output arms are not supported input paths |
| `00911358` / 151 | `00911358–009113EE`, raw-to-string wrapper | **Supported** §4.1 |
| `009113EF` / 119 | `009113EF–00911465`, string-to-raw wrapper | **Supported** §4.2 |
| `00911466` / 201 | `00911466–0091152E`, source-encoded string wrapper | **Supported** §4.3 |
| `0091152F` / 193 | `0091152F–009115EF`, target-encoded string wrapper | **Supported** §4.4 |
| gap `00910BD3–00910E17` | RZParseDelimitedText, 581 bytes | **Foreign**, not part of these entries |
| boundary `009115F0` onward | GetUTF16Character and other UTF16 helpers | **Foreign**, not expanded |
| calls outside the six ranges | system-codepage lookup; Win32 conversions; memcpy; scratch allocation/free; string interface methods | **Foreign** raw primitives, not whole-owner conversion or new capped-family coverage |
| failure/alias/overflow domains in §4.5 | not safely defined by this normal-path contract | **Unknown / Unsupported**, no invented recovery |

The six ranges are disjoint; five are adjacent after the foreign parse gap.
The helper has a single shared call site at `009112A2`, reached from both
UTF8→legacy and legacy→UTF8 dispatch; do not count those as two helper bodies.

## 7. Open questions and verification limits

No new public question is needed to implement the supported local algorithm.
Raw OS outcomes are deliberately individual primitive inputs, not missing
whole-converter branches. Existing strings-language SL1 and other owner questions
are unchanged. Unsupported domains above have no inferred replacement semantics.
The literal ledger is proposed, not an executable conformance suite, native run,
portable decoder output, shadow match or implementation result.
