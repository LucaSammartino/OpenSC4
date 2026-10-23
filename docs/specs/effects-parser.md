# Spec: native effects text description parser (`cSC4EffectsParser`)

## 1. Evidence, domain and boundaries

This is a **partial owner contract**, derived statically on 2026-10-23 from the Windows
1.1.641.0 original, not an executed parser oracle. The independently hashed
`bin/SimCity4.exe` and `ghidra/export/windows/export-summary.txt` agree on SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
Addresses below are Windows virtual addresses at image base `00400000`.
Primary evidence is export instructions, with raw executable data for literals;
local pseudocode was navigation only. The ordinary full lane check reported
REA-DOWN (configured launcher absent); no live procedure or native execution was used.

[effects.md §3](effects.md#3-the-effect-directory-resource-compat-data-format)
supplies the prior public in-memory component record layout. Its binary reader and
runtime algorithms do not establish text grammar or text defaults. This document
uses `D+offset` for the component record supplied to `ParseDescRecOptions`.
The generic command tokenizer, switch finder, numeric/vector conversions,
allocators, factories, directory implementation and runtime effects remain foreign.
No claim is made about the separate `EffectsParser::ParseQueuedFiles` queue wrapper.

Normal domains require valid live objects and successful returning foreign calls.
Allocation failure, invalid objects, exceptions, callback reentrancy and rollback
are unproved. All examples below are **PROPOSED / UNEXECUTED** static predictions.

## 2. Primitive input boundary and finite option selection

`005989D0` takes `this` in ECX, a token-container pointer and `D` as two stack
arguments; the normal epilogue is `RET 8` (`0059973C–00599744`). It has no explicit
success-result construction. Treat its source-level result as void; in particular
the last switch finder's AL is not a parser success status.

Each lookup calls `nSCRes::FindSwitchArgumentIndices` (`0067A7D0`) with shared
index scratch `00B22D2C`, the token container, a literal key, and a request count.
AL zero selects the absent branch; nonzero selects the present branch. Returned
indices address 20-byte token records from the container's first pointer.
This contract exposes **individual lookups and individual conversions**, never a
host-produced completed description or parser verdict. The scratch is reread after
calls; neither reentrancy nor concurrent use is established.

The finder owns dash recognition, case, delimiters, aliases, duplicate occurrence
selection, insufficient-value handling, terminators, and any token marking/removal.
These are not inferred from the literal keys. There is no owner-side loop scanning
all tokens, unknown-option rejection, explicit cursor increment or explicit erase
in this body. Unknown or empty input for which all lookups return absent follows
the absent branches below. That does not prove the foreign finder accepts arbitrary
unknown, malformed or empty input. Actual whole-input consumption remains open.

The complete owner lookup order is finite:

| Order | Literal key | Request count | Owner branch |
|---|---|---:|---|
| 1 | `offset` | 1 | vector copy and transform marks |
| 2–6 | `rotateX`, `rotateY`, `rotateZ`, `rotateXYZ`, `rotateZXY` | 1, 1, 1, 3, 3 | rotation branches, numeric details pending |
| 7 | `scale` | 1 | scalar copy, scale flag decision |
| 8 | `lodRange` | 2 | if present, skips lookup of `lod` |
| 9 | `lod` | 1 | only when `lodRange` absent |
| 10–11 | `emitScale`, `sizeScale` | 1 each | scalar pair writes |
| 12 | `prob` | 1 | select-only weight write |
| 13–14 | `ignoreLength`, `respectLength` | 0 each | set then clear flag |

These are distinct literal keys, not established aliases. Lookup order overrides
textual order for distinct keys. `lodRange` has priority over `lod`; both length
keys are queried, so `respectLength` wins when both lookups report present.
The no-rotation subdomain (all five rotation lookups absent) is the finite normal
option family fully described in §3. Rotation calculation/store order is explicitly
outside that supported subdomain, rather than delegated as a host parser outcome.

## 3. Supported non-rotation option state transitions

Fields not mentioned by a taken branch retain their input bits. There is no record
initialization at entry. The LOD absent branch is an exception: it actively writes
defaults on every call. Count increments below are byte increments modulo 256;
bit operations preserve all unrelated bits.

| Option | Present branch | Absent branch | Instruction anchors |
|---|---|---|---|
| `offset` | Convert first returned token with foreign `ParseVector3` (`0067C060`); copy three returned 32-bit words to `D+3C,+40,+44` in that order. OR byte `D+15` with `01`; increment byte `D+14` once. No zero-vector special case. | retain translation, flags and count | `005989DA–00598A35` |
| `scale` | Foreign `ParseFloat` (`0067BBA0`) yields x87 ST0. Store binary32 at `D+48` **before** comparing the still-live ST0 with binary32 1.0. Ordered equality clears bit `02` of byte `D+15`; inequality or unordered sets it. Increment byte `D+14` once even for 1.0. | retain scale, flags and count | `00599483–005994D6` |
| `lodRange` | Require `this+4A0 == 0`; convert first then second indexed tokens with foreign `ParseUint` (`0067BC70`). Write low byte of first result at `D+4C`; write low byte of second result plus one modulo 256 at `D+4D`. No order/range clamp in owner. Skip `lod` lookup. | query `lod` next | `005994D9–00599555` |
| `lod` | Require `this+4A0 == 0`; one `ParseUint`; write low byte at `D+4C`, its increment modulo 256 at `D+4D`. | if also no `lodRange`, write bytes 1 and 6 to `D+4C,+4D`, including inside select | `00599557–005995C2` |
| `emitScale` | Convert first token; store binary32 at `D+54`. Compute scratch index count from `(end-begin) >> 2`; unsigned count below 2 stores the same still-live first conversion at `D+58`. Otherwise discard first ST0 and convert second indexed token, storing that at `D+58`. Further indices are not converted here. | retain both fields | `005995C6–00599626` |
| `sizeScale` | Same two-value rule, writing `D+5C,+60`. | retain both fields | `00599629–00599689` |
| `prob` | Require `this+4A0 != 0`; convert first token. Multiply ST0 by binary32 65535.0 (`00AA0174`), add binary32 12582912.0 (`00A9F378`), store binary32 temporary, copy **its low 16 raw bits** to `D+66`. No clamp and no integer-conversion instruction. | retain weight | `0059968C–005996FE` |
| `ignoreLength` | OR 32-bit `D+10` with 1 | retain flags | `00599702–0059971B` |
| `respectLength` | AND 32-bit `D+10` with `FFFFFFFE` | retain flags | `0059971F–00599738` |

The scale comparison uses the x87 value before store rounding, not a reread of
`D+48`; a value that rounds to 1.0 in that store can still set the flag. Probability
arithmetic uses x87 operations and the active precision/rounding controls, then a
binary32 store. Do not replace it with a cast, saturation, decimal rounding or a
host's float-to-integer routine. Numeric spelling/locale, overflow, NaN production
and vector token grammar belong to the typed conversion primitives, not this spec.

Forbidden LOD-in-select takes an error-construction call with literal
`Can't specify LOD in a select`, then a call at `009EF84F` with metadata
`00B024F4` (`00599500–00599518`, `0059957E–00599596`). Forbidden `prob` outside
select similarly uses `-prob only valid inside a select group`
(`005996AF–005996CB`). The latter target's export name is misleading; its unwind
semantics have not been established here. These are error exits, not a proved
false return, rollback or recoverable continuation. Earlier writes precede them;
this publication's supported normal domain excludes these paths.

## 4. Independent literal option ledgers

For these vectors, token strings are exact input record payloads, not a claim that
the generic lexer accepts their spelling. `F(key)=[indices]` stipulates that one
foreign lookup returns nonzero and that exact scratch list; all unlisted lookups
return zero and leave input records intact. This explicitly chosen primitive
fixture has **zero removed tokens**, hence the remaining stream is the input
stream and the next token is token 0 (or EOF for empty). The owner's explicit
consumption is zero; consumption of the real foreign finder is unproved.
`PF(i)=value`, `PU(i)=u32` and `PV(i)=three raw binary32 words` stipulate individual
conversion results. They are not native-golden numeric conversions.

Common asymmetric input: byte count `D+14=FE`, transform flags `D+15=A7`,
32-bit flags `D+10=A5A50001`; translation `(9,8,7)`, scale 3, LOD bytes `(7,9)`,
emit pair `(11,12)`, size pair `(13,14)`, weight `1234` hex. Rotation lookups
are absent throughout. Except in P8, select state `this+4A0=0`.

| ID | Exact token payload stream and primitive fixture | Predicted owner output / calls |
|---|---|---|
| P1 | `[]`; all absent | LOD `(1,6)`; every other field retained; no conversion |
| P2 | `["-scale","1"]`; `F(scale)=[1]`, `PF(1)=1` exactly | `D+48=1`, flags `A5`, count `FF`; LOD `(1,6)` |
| P3 | `["-offset","0,0,0","-scale","2"]`; `F(offset)=[1]`, `PV(1)=(0,0,0)`; `F(scale)=[3]`, `PF(3)=2` | vector conversion first, float second; translation all zero, scale 2, transform flags `A7`, count `00` after two increments |
| P4 | `["-lod","255","-lodRange","258","511"]`; `F(lodRange)=[3,4]`, `PU(3)=258`, `PU(4)=511` | LOD `(2,0)`; exactly two uint calls, no `lod` lookup or conversion; count/transform flags retained |
| P5 | `["-respectLength","-ignoreLength"]`; both zero-value lookups present | `D+10=A5A50000`; owner sets then clears despite text order |
| P6 | `["-emitScale","2","-sizeScale","3","4"]`; scratch lists `[1]`, `[3,4]`; `PF(1)=2`, `PF(3)=3`, `PF(4)=4` | emit `(2,2)`, size `(3,4)`; float calls at indices 1,3,4 in order; count retained |
| P7 | `["-scale","2","-scale","1","-unknown",""]`; stipulated `F(scale)=[3]`, `PF(3)=1` | one conversion of index 3, one increment, scale 1, flags `A5`; no owner dispatch of unknown or empty payload. Alternate finder fixture `[1]` instead yields scale 2, flags `A7`; duplicate selection is a foreign question |
| P8 | `["-prob","1"]`; select state 7; `F(prob)=[1]`, `PF(1)=1` exactly; round-to-nearest, sufficient x87 precision | binary32 temporary bits `4B40FFFF`, weight `FFFF`; LOD `(1,6)`; no count increment |

No portable implementation outputs were used to choose these literals or results.
P7's alternatives are conditional predictions, not two native outcomes for the same
fully specified original reader. Explicit empty **values**, terminators and missing
values have no claimed acceptance rule; see [open questions](questions/effects-parser.md).

## 5. Registration, commands and lifetime

Pending bounded expansion. No named/anonymous collision or reference policy is
inferred from the binary directory or from pointer storage alone.

## 6. Coverage statement

`ParseDescRecOptions` (`005989D0`, exported size 3445 bytes) is **Partial**:
supported no-rotation normal branches above; five rotation bodies and foreign
reader language prevent whole-body/whole-parser support. The size is a metadata
anchor, not a newly certified byte-coverage count. No complete inventory, default
grammar, native validation or runtime proof is claimed. Further own-body statuses
will be listed individually as examined.
