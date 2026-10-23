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
selection, insufficient-value handling, terminators, and token marking; §9 now
specifies that service at the token-record boundary.
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

These are distinct literal keys. Lookup order overrides
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
semantics are specified in §10. These are exception exits, with no owner rollback
or recoverable continuation. Earlier writes precede them;
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

### 5.1 Named references: `AddDescription` (`005A2810`)

This body is a **component-reference append**, not a named-description factory.
Its four stack arguments occupy 16 bytes (normal `RET 10` hex). Argument 1 is
the token container, argument 4 supplies the initial low flags bit (zero clears,
nonzero sets). Arguments 2 and 3 are not read by this body: the name actually
comes from the **current first shared scratch index**, and type is hardcoded 6.
ECX is the main parser pointer `P`. The directly proved caller `005A3890` supplies
the redundant name and 6 in arguments 2 and 3 (`005A390B–005A391C`).

Normal sequence, with successful foreign calls and sufficient local append space:

1. If `P+90` is null, return without constructing a record, looking up a name,
   parsing options or appending. There is no false-result construction.
2. Construct temporary record `D` using shared constructor `005997B0`.
3. Read the indexed token's string pointer and call the object at `P+90`, vslot
   `+48`, with `(namePointer, 6, null)`. Inspect only returned AL. Zero constructs
   `No such description: '%s'` and enters the foreign error transport; it is
   outside the successful normal domain (`005A2831–005A2889`).
4. Reread the shared first index and copy that token string into `D+00` through
   foreign string assignment `00408A70`; this is not an independently generated
   identity. The code does not lowercase or otherwise normalize this string.
5. Copy low 16 bits of `P+4A0` to `D+64`; set `D+0C=6`; set/clear bit 0 of
   32-bit `D+10` according to argument 4, preserving its other bits. Call
   `ParseDescRecOptions(P,tokens,D)` **after** those writes
   (`005A28B9–005A28F0`), so its `respectLength`/`ignoreLength` can override that bit.
6. Compare end pointer `P+450` with capacity pointer `P+454`. When unequal and end
   is nonnull, invoke record copy construction `005888A0` with destination=end
   and source=D, then increase `P+450` by `6C` hex. No name search or replacement
   decision occurs on this local append branch. The observed unequal/null-end
   branch skips construction and still advances end; it is not included in the
   valid-container normal domain. When end equals capacity, call foreign vector
   insertion `0059EDC0` on `P+44C` with `(end,&D,tagPointer,1,1)` instead; its
   allocation and copy internals are outside this publication
   (`005A28F5–005A2930`).
7. After either returning append branch, if D's string-buffer pointer is nonnull,
   pass it to the foreign cleanup at `0090CF63`. Then return
   (`005A2935–005A2950`). No explicit object AddRef/Release call occurs here.

The append is performed again for every successful invocation, even for a repeated
name; the local owner does not replace or deduplicate components. Whether repeated
names pass the foreign `+48` validation, or identify an existing directory object,
is a separate contract. `D+68` retains the constructor sentinel `FFFFFFFF` on
this named path; no returned directory index is stored here.

### 5.2 Anonymous registration: `AddAnonDescription` (`005A2960`)

This entry takes `(tokens,descriptionObject,initialBit)` as three stack arguments,
ECX=P; it ends in `RET C` hex. With `P+90` null it does nothing. Otherwise:

1. Construct the same temporary record. Ensure its name is empty by comparing
   string begin/end and, if unequal, writing a terminator at begin and resetting
   end to begin (`005A2974–005A298A`).
2. Call `P+90` vslot `+44` with **null name and the supplied object**. Store full
   returned EAX at `D+68`, without a sentinel/failure-result test.
3. Call supplied object's vslot `+00` with no stack arguments; store full returned
   EAX at `D+0C` (`005A298E–005A29AB`). This is a type/property result, not a
   parser-side constructor/factory selection. No particular concrete class is
   selected in this body.
4. Copy low word `P+4A0` to `D+64`, initialize flags bit 0 from argument 3,
   then parse options (`005A29AF–005A29E3`).
5. Append to `P+44C` with the same end/capacity decision and `6C` stride; perform
   the same temporary string cleanup (`005A29E8–005A2A42`).

There is **no anonymous counter, synthesized name, collision test, reference-count
adjustment on the supplied object, or check of the registration result** in this
body. The foreign registration primitive may implement identity and reference
transfer; those effects must not be invented here. It receives one raw object and
returns one raw index, never a completed component or final parser verdict.
Registration precedes option parsing and append; no rollback policy is proved
for a later conversion, error or allocation failure.

### 5.3 Shared component construction, distinguished from option defaults

`005997B0` is a shared description-record constructor, not a new parser-exclusive
body. On its successful normal return it has created an empty string with an
8-byte buffer; `D+0C=0`, `D+10=0`, transform count/flags bytes at `+14,+15=0`,
identity 3×3 at `+18..+38`, zero translation at `+3C..+44`, scale 1 at `+48`,
emit and size pairs all 1 at `+54,+58,+5C,+60`, LOD bytes zero at `+4C,+4D`,
16-bit fields `+4E=1`, `+50=16`, `+64=0`, `+66=0`, and index `+68=FFFFFFFF`
(`005997B6–005998B7`). Earlier global-backed transform writes are overwritten
by the final identity/zero stores. This is a list of measured field writes, not
a guarantee that all 108 bytes, padding included, are initialized.

Thus a newly appended record with no options has LOD `(1,6)`, not the constructor's
`(0,0)`. Passing an existing record directly to the option routine has the
retention behavior of §3, not these constructor defaults.

### 5.4 Manager acquisition and same-manager behavior (`005988D0`)

Use **entry-relative coordinates** here: let S be the ECX value received by this
body, M the stack argument, H the stored pointer at `S-460` hex and K the stored
pointer at `S-45C`. Do not pass the main-pointer offsets of `005A2810` to this
adjusted entry. The sole exported direct caller is virtual-adjustment thunk
`0059F490`, which subtracts the dword at incoming ECX−4 before jumping here.

The body performs this ordered sequence (`005988DA–00598980`):

- When M differs from H: if M nonnull, call M's `+04` AddRef slot; store M to H;
  if old H nonnull, call its `+08` Release slot. Thus acquisition precedes store,
  and store precedes release. When M equals H, all three operations are skipped.
- For **any nonnull M, including the same-manager case**, initialize a local
  output pointer to null, call M's QI slot with ID `C97CF5CE` and its address,
  and use the resulting pointer T. The returned boolean is not inspected.
- If T nonnull, AddRef T, then reread and Release the nonnull local QI result.
  For returning, nonreentrant callbacks this cancels one temporary reference pair.
- If T differs from K, AddRef nonnull T, store T to K, then Release nonnull old K.
  If equal, skip this assignment sequence. Finally Release nonnull T once.
- Unconditionally clear byte `S-448` to zero and return (`RET 4`). There is no
  constructed boolean/status result.

The stack-local/object-field address equality guard at `0059893F–00598945`
is false for the normal nonoverlapping object/stack domain above. When M is null,
the QI and **all K updates are skipped**: clearing H does not clear K. When a
nonnull M's QI leaves its output null, the normal path replaces K with null and
releases old K. Nothing here proves that a missing interface returns false or
that null/non-null result pairs obey a particular QI failure convention.

### 5.5 QueryInterface, factory identity and reference boundaries

`00595870` has two stack arguments `(iid,out)`, a boolean in AL, and `RET 8`.
Null out returns AL=0 without any query/ref call (`00595870–0059587A`). For IID
`6956BABC`, it stores an adjusted interface pointer, invokes the canonical
unknown's `+04` slot, then returns AL=1 (`0059589B–005958D5`). To retain the native
multiple-inheritance adjustment precisely, let Q be this entry's ECX and V the
table pointer at `Q-4D4`: for the ordinary live-object domain `Q-4D8 != 0`,
the stored pointer is `Q-4D4 + signed32(V+0C)`, and the AddRef receiver is
`Q-4D4 + signed32(V+04)`. These table reads precede the virtual call; do not
replace them with an assumed main-object pointer. Other IIDs tail-delegate to
foreign `nSCRes::cCommandParser::QueryInterface` (`00675F50`) with ECX=`Q-448`.
No owner rule for unknown-IID output clearing is established.

Factory `0059F860` allocates `4F4` hex bytes, invokes `0059EEC0` with argument 1,
and returns the constructor result adjusted by the table at result+4 (table+4
offset plus result+4). It makes no explicit AddRef call. The factory and constructor
table stores (`0059EECF–0059EFA6`) positively connect the constructor labelled
`EffectsParser::EffectsParser` with this native class; that label alone does not
pull in the queue algorithm at `0058E4E0`. Construction zeros pointers at main
`+90,+94` (`0059EFAF–0059EFB5`) and component begin/end/capacity at
`+44C,+450,+454` (`0059F2A4–0059F2B0`). Whole-constructor support is not claimed.

The constructor-selected unknown table `00AA04D4` routes AddRef/Release through
`0059F3C0` / `0059F3D0` to shared bodies `00675FC0` / `00675FD0`. The earlier
identification of `0059F3E0` / `0059F3F0` as those reference operations was wrong;
the latter entries belong to the separate parser-service table `00AA04A0`.
The callback via `0059F4D0` reaches `0059EBE0`; it is not the destructor.
The actual deleting destructor is `005A27F0`, with member cleanup at `0059F4E0`.
See §11 for reference counts and held-interface release order.

## 6. Command registration and one concrete command path

### 6.1 Root dictionary selected by the owner

`RegisterCommands` (`00596200`, ECX=P, no stack arguments, no constructed result)
first invokes foreign base registration `00678B00`, then issues the following
**21 ordered registrations** to foreign `00676DB0`. Names are literal bytes reached
through initialized name-pointer slots `00B0A288..00B0A2C0`, `00B0A310`,
`00B0A380`, `00B0A394`, `00B0A3C4`, `00B0A3DC`, `00B0A3F8`. The table records
raw descriptor/dispatch associations; symbolic handler names are not treated as
proof of command meaning.

| Order | Literal | Descriptor vptr | Direct hook `+0C`, or block hooks `+14 / +18` |
|---:|---|---|---|
| 1 | `renderprop` | `00A9F85C` | `00596630` |
| 2 | `textureID` | `00A9F86C` | `0059F890` |
| 3 | `modelID` | `00A9F87C` | `0059FE50` |
| 4 | `soundID` | `00A9F88C` | `0059FF40` |
| 5 | `brushID` | `00A9F89C` | `005A0030` |
| 6 | `effectID` | `00A9F8AC` | `0059A640` |
| 7 | `effectGroup` | `00A9F8BC` | `00596750 / 00648F10` |
| 8 | `instance` | `00A9FA08` | `0059A730` |
| 9 | `testEffect` | `00A9F8D8` | `0059D1D0` |
| 10 | `messageTrigger` | `00A9F8E8` | `0059E820` |
| 11 | `camera` | `00A9F8F8` | `0059A2A0` |
| 12 | `setPriority` | `00A9F908` | `005A24C0` |
| 13 | `effectsResource` | `00A9F918` | `00596810 / 00596970` |
| 14 | `loadResource` | `00A9F934` | `00596AB0` |
| 15 | `effect` | `00A9F944` | `005A0D40 / 0059EB50` |
| 16 | `particles` | `00A9F960` | `005A1A50 / 0059D9B0` |
| 17 | `dynamicParticle` | `00A9F97C` | `0059DB00 / 0059DCF0` |
| 18 | `decal` | `00A9F998` | `0059DD80 / 0059DF50` |
| 19 | `shake` | `00A9F9B4` | `0059DFE0 / 0059E260` |
| 20 | `light` | `00A9F9D0` | `0059E2F0 / 0059E4D0` |
| 21 | `sequence` | `00A9F9EC` | `0059E600 / 0059E780` |

Instruction groups are `00596209–00596342` (1–8), `00596347–00596433` (9–14),
`00596438–00596598` (15–21). Simple commands request a 4-byte allocation and write
their vptr. Block commands request `18` hex bytes, invoke shared constructor
`00677290`, then overwrite the vptr before registration. The ordinary normal domain
here requires successful allocation and registration. The null-allocation branches
pass null to the foreign registrar; they do not prove a recoverable policy.
Registration results are not used for owner branching. Ordering is proved;
case folding, duplicate lookup/re-registration policy and descriptor lifetime are
foreign registrar decisions, not specified anew here. No unrelated graphics or
runtime command algorithms are included.

### 6.2 `sequenceEffect` within an effect block

The nested command is positively associated: `00596D9E` selects descriptor
`00A9FDFC`; `00596DA8–00596DB2` registers it using pointer slot `00B0A2FC`,
whose literal is `sequenceEffect`; descriptor `+0C` is `005A3890`.
This is distinct from the root `sequence` block above. Other nested dictionary
entries and block execution algorithms are outside this bounded publication.

`005A3890` takes a token container and parser pointer as two stack arguments;
its own command-object ECX is unused. Its normal exit is `RET 8` without a
constructed success value. It checks the parser's current-name string begin/end
at `P+434,+438`; the empty-name branch constructs `Not in effects block` and
enters foreign error transport (`005A3898–005A38EA`). For a nonempty name:

1. Call foreign `FindNonSwitchIndices` (`0067A660`) with shared scratch,
   the input container and request count 1 (`005A38EF–005A38FB`).
2. Do not inspect returned AL. Use scratch index 0 to read the first selected
   token string, then invoke `AddDescription(P,tokens,string,6,0)`
   (`005A3900–005A391C`). Only that first nonswitch index is read by this command.
3. Return after the append routine returns. There is no explicit token erase,
   append-result check, count override, or second name conversion here.

Therefore a valid primitive selection of one name builds a **type-6 reference**
with initial ignore-length bit clear, then permits the option routine to alter
that bit. This handler does not instantiate a concrete sequence object or choose
a description factory; anonymous concrete-object registration is the separate
boundary in §5.2. Behavior when the foreign finder fails to provide a usable first
index is unsupported; do not turn the unchecked index into a guessed success rule.

## 7. Registration and reference trace ledgers

All rows remain **PROPOSED / UNEXECUTED**. Token fixtures have the same foreign
no-removal stipulation as §4, so consumed=0 and remaining stream=input; a concrete
native finder may differ. Initial component end E is nonnull with at least two
records of free capacity unless stated. Foreign copies preserve the supplied
record's semantic fields; their allocation mechanics are not golden outputs.

| ID | Exact inputs / primitive results | Predicted owner trace and state |
|---|---|---|
| R1 | `sequenceEffect` input `["sequenceEffect","loopA"]`; current block name `"parent"`; nonswitch scratch `[1]`; `+48(name="loopA",type=6,out=null)` AL=1; no option hits; select state `12345678` hex | record ctor → validation → string copy → select word `5678` → type 6 → clear initial bit → options (LOD 1,6) → copy to E → end E+6C → temporary cleanup. Record name `loopA`, flags 0, index `FFFFFFFF`. Repeat same call with successful primitive validation: second copy to E+6C, end E+D8; no owner duplicate suppression |
| R2 | `["sequenceEffect","loopA","-ignoreLength"]`; same selection and validation as R1; ignore-length lookup present | initial clear is overridden; appended flags bit 0=1; no other flags changed |
| R3 | direct anonymous input `["soundEffect","bell"]`; object O; argument 3 nonzero; select state 0; foreign `+44(null,O)` EAX=`FFFFFFFF`; O `+00` EAX=9; all option lookups absent | ctor → ensure empty name → register → read type → initialize bit → parse → append → cleanup; record empty name, type 9, index `FFFFFFFF`, flags bit 0=1, LOD 1,6. No sentinel-result branch and no direct AddRef/Release of O |
| R4 | same as R3 but foreign raw index `00000000`, then separate invocation raw index `FFFFFFFF`; same object and other fixtures | both raw values stored verbatim and each invocation appends; there is no observed local counter to wrap, nor any proved foreign counter boundary |
| R5 | direct named or anonymous input as above; `P+90=null` | immediate return, no ctor/conversion/registration/append/cleanup; end unchanged; no explicit status |
| R6 | successful named validation; end=capacity C | option mutations precede exactly one insertion call on `P+44C`, args `(C,&D,tagPointer,1,1)`; temporary cleanup after it returns; do not predict allocation size or final pointers without the foreign insertion contract |
| M1 | S-entry manager call: H=A, K=X, input M=B; QI output T=Y; all four distinct nonnull | `B.AddRef → H=B → A.Release → B.QI(C97CF5CE) → Y.AddRef → Y.Release → Y.AddRef → K=Y → X.Release → Y.Release → resetByte=0` |
| M2 | H=A, K=X, M=A; QI output T=X | no H acquire/store/release; `A.QI → X.AddRef → X.Release → X.Release → resetByte=0`; no K assignment. The final release balances the QI-held reference under that foreign convention |
| M3 | H=A, K=X, M=null | `H=null → A.Release → resetByte=0`; K remains X; no QI |
| M4 | H=A, K=X, M=A; QI output remains null | `A.QI → K=null → X.Release → resetByte=0`; no H change; QI's boolean is immaterial |
| Q1 | `QueryInterface(Q,6956BABC,out)` with valid nonnull out and ordinary live Q | store table-adjusted pointer → canonical AddRef → AL=1; null out instead gives AL=0 without AddRef |

M1–M4 describe returning nonreentrant callbacks; global reference totals also depend
on the foreign QI's actual reference convention. No destruction trace is claimed.

## 8. Individually bounded coverage statement

`ParseDescRecOptions` (`005989D0`, exported size 3445 bytes) is **Partial**:
supported no-rotation normal branches above; five rotation bodies and foreign
reader language prevent whole-body/whole-parser support. The size is a metadata
anchor, not a newly certified byte-coverage count. No complete inventory, default
grammar or runtime proof is claimed. §§9–11 add reader, validation and lifetime
contracts without completing the command family. Further own-body statuses
are measured individually below; **Supported** always means the stated normal
domain with primitive results, not the entire language, errors or runtime.

| Body / exported size | Status | Signature / result channel and boundary |
|---|---|---|
| `005A2810` / 321 | Supported normal owner control | ECX=P, 4 stack args, `RET 10` hex; no explicit status; named validation/append §5.1, foreign error/insertion excluded |
| `005A2960` / 227 | Supported normal owner control | ECX=P, 3 stack args, `RET C` hex; no explicit status; raw anonymous registration result → field, append §5.2 |
| `005988D0` / 177 | Supported normal owner control | adjusted ECX=S, one pointer arg, `RET 4`; no explicit status; nonreentrant acquisition/assignment §5.4 |
| `00595870` / 102 | Supported owned branches; foreign fallback | adjusted ECX=Q, iid/out, `RET 8`, AL bool; null out / IID `6956BABC`; other IDs delegated §5.5 |
| `00596200` / 921 | Supported normal registration sequence | ECX=P, no args, no explicit result; 21 descriptor registrations §6.1; foreign dictionary policy |
| `005A3890` / 153 | Supported nonempty-block / valid-selection branch | ECX unused, tokens/P stack args, `RET 8`; no explicit result; exact append decision §6.2 |
| `0059F860` / 44 | Partial factory identity | no stack args, EAX adjusted pointer; successful construction only, §5.5 |
| `0059EEC0` / 1253 | Partial constructor anchors | ECX storage, one construction-mode arg, EAX storage; selected pointer/vector zero stores and factory relationship only |
| `005997B0` / 264 | Foreign shared record constructor, measured field writes | ECX=D, EAX=D; §5.3 defaults are supporting input facts, not parser-exclusive coverage |
| `0059F3B0`, `0059F490` / 8 each | Supported adjustment only | subtract incoming `[ECX-4]`, tail-jump to owned QI/SetManager; no separate return channel |
| `0059F3C0`, `0059F3D0` / 14 each | Shared reference operation | adjustment to `00675FC0` / `00675FD0`; §11 |
| `0059F3E0`, `0059F3F0` / 14 each | Parser service delegation | not AddRef/Release |
| `0059F4D0` / 8; `0059EBE0` / 195 | Reset callback | distinct from destructor; internal reset algorithms excluded |

Generic readers, conversions, base QI/registration/reference operations,
string/allocator operations and vector copy/insertion remain **Foreign** to the
parser owner; §§9–11 specify several of those dependencies independently.
Unexamined command hooks, rotation calculations and complete tokenizer grammar
remain **Unknown** here. Exported sizes are function extents, never a sum of supported
bytes or a complete parser inventory. The concrete factory, caller and descriptor
links above establish ownership more narrowly than name metadata alone.

## 9. Switch and positional reader contract *[compat]*

This section answers EFP1 for already tokenized, mutable token records. A token
has a byte string, a recorded length and an index; command token 0 is included.
Lexing quotes, bracketed vectors and whitespace happens before this boundary.
There is no additional delimiter splitting inside these readers: `-scale=2`
is not a lookup for `scale`, and `--scale` is not `-scale`.

The initialized switch-matching prefix set is the one byte `-`; the value-list
terminator prefix set is the two bytes `-+`. Initializers `00A742F0` and
`00A74320` copy ranges `00A9F34C..4D` and `00A9F350..352` respectively.
Recognition is byte-exact and case-sensitive, with no alias expansion.
`00679F20` selects the first token, starting at index 0, whose length is exactly
one plus the key length, whose first byte is `-`, and whose remaining bytes equal
the whole key (`00679F77–0067A02C`, equality search `004426C0`).

`FindSwitchArgumentIndices` (`0067A7D0`) clears the scratch **index list** on
every call. An absent match returns false with an empty list and no token changes.
A match changes only its first byte from `-` to `+`, before any arity check.
It collects every subsequent token index until EOF or a token of length greater
than one whose first byte is `-` or `+` and whose second byte satisfies CRT
`isalpha`. The switch's own index is not returned. The request count is a
**minimum**, not a maximum: zero still collects values; a request for one may
return three. Fewer than the requested number throws `Expecting at least %d
arguments for -%s`; the marking and partial scratch list already exist.
Otherwise it returns true (`0067A81E–0067A94F`).

Neither tokens nor values are erased, and token indices and string lengths do
not change. A second lookup of the same key can find the next duplicate because
the first now starts with `+`. It does not select the last occurrence. Negative
numbers such as `-2`, positive numbers such as `+3`, lone `-`/`+`, and empty
records are values. An unknown `-word` or `+word` terminates the preceding value
list even if no command recognizes that word. `--` is not an end-of-options
escape: its second byte is not alphabetic, so it is a value. Alphabetic testing
uses the native CRT locale; the examples below use ASCII.

`FindNonSwitchIndices` (`0067A660`) clears its scratch list, starts at token 1,
and collects the initial positional prefix up to the same `-`/`+` followed by
alphabetic test. It does not resume after switches, skip marked switches, mark
positional tokens, or enforce an exact maximum. Insufficient initial positional
values throw `Expecting at least %d arguments` (`0067A685–0067A7C5`).

Unknown switches are checked separately after a dispatched command returns by
the generic parser (`00679120`, helpers `00679E50` and `0067A040`): an unmarked
`-` followed by alphabetic text can cause `Unexpected switch: %s`. A marked
`+word` does not count as an unhandled switch. There is no reader-level error for
unused ordinary values. These rules do not make whole-command parsing atomic.

### 9.1 Reader conformance vectors (static predictions, unexecuted)

Every row starts with new records and a nonempty old scratch list. Indices are
zero-based and every record, including command token 0, remains after the call.
The explicit empty string below is an existing empty token, not a lexer claim.

| ID | Token records / lookup | Reply and scratch | Mutation / failure |
|---|---|---|---|
| SW1 | `cmd -scale 2 -scale 3`; key `scale`, minimum 1, twice | first true `[2]`; second true `[4]` | first token 1 becomes `+scale`; then token 3 becomes `+scale` |
| SW2 | `cmd -Scale 2 --scale 3 -scale=4`; key `scale`, minimum 1 | false `[]` | unchanged |
| SW3 | `cmd -k -2 +3 -- - + "" -next 9`; key `k`, minimum 1 | true `[2,3,4,5,6,7]` | token 1 becomes `+k`; `-next` terminates collection |
| SW4 | `cmd -k +next`; key `k`, minimum 1 | exception, scratch `[]` | token 1 is already `+k`; `+next` remains |
| SW5 | `cmd -k ""`; key `k`, minimum 1 | true `[2]` | token 1 becomes `+k`; numeric conversion of empty string is a separate operation |
| SW6 | `cmd -k 1 2 3`; key `k`, minimum 0 | true `[2,3,4]` | token 1 becomes `+k`; no truncation to minimum |
| SW7 | `cmd first -unknown value later`; positional minimum 1 | scratch `[1]`, normal return | no mutation; later handler completion can reject unhandled `-unknown` |
| SW8 | `cmd -k 1`; key `missing`, minimum 4 | false `[]` | absence does not trigger minimum-count error |

Earlier §4 vectors intentionally inject foreign primitive replies and leave
records unchanged. In particular P7's `[3]` alternative is not the native reader
outcome on its first lookup. SW1 supplies the native duplicate policy.

## 10. Error transport and owner failure prefix *[compat]*

`009EF84F` is a C++ throw helper, despite its exported vector-constructor name.
It calls imported `KERNEL32!RaiseException` at IAT `00A8026C`, using exception
code `E06D7363`, flags 1 (noncontinuable), and three information words:
`19930520`, the supplied exception-object pointer, and supplied throw metadata.
The template is at `00AE62EC`; instructions are `009EF85A–009EF87D`.
Effects parse errors supply metadata `00B024F4`, whose destructor entry is
`005998C0`. No boolean error status is returned by the command bodies.

The LOD-in-select errors and prob-outside-select error in §3 occur after their
switch is marked by §9 and after all earlier successful option writes. They
occur before any conversion or write of the rejected LOD/prob value. No local
catch, compensation or saved-description restoration is installed in
`005989D0`. In `AddDescription`, a failed name validation occurs before name-copy,
option processing and append; an option error occurs before append. Normal
temporary cleanup after append is not an exception handler. The append and later
option lookups are not executed on these throws. Likewise a command rejected
outside an effects block throws before positional selection.

This defines the observable failure prefix, not permission to resume execution
after the throw instruction. A caller may handle the C++ exception at a higher
scope; the complete file-parser catch/report/recovery policy is not established
by these owner bodies. Destruction of the transported exception is distinct
from rollback of parser fields and token marking.

| ID | Initial state and input | Predicted failure prefix (unexecuted) |
|---|---|---|
| ER1 | select active; fresh D; `cmd -scale 2 -lod 3`; exact finite conversions | scale stored and count incremented; both switches marked; LOD error throws before its uint conversion; no append |
| ER2 | select inactive; `cmd -offset vector -prob 1`; vector primitive returns `(1,2,3)` | translation stored and count incremented; default LOD `(1,6)` written; prob marked; throw before probability conversion; no append |
| ER3 | absent referenced description with nonnull collection | collection validation only, then `No such description: '%s'` throw; no options or append |

## 11. Directory identity and actual final lifetime *[compat]*

### 11.1 Component directory service used by commands

The collection tables `00A9F040` and `00A9F198` bind slot `+44` to
`AddComponentDescription` (`0058AC10`) and `+48` to `GetComponentDescription`
(`00589070`). The type selects a separate component bucket: the native bucket
stride is `20` hex, selected by `type - 3`. Valid component objects and supported
types are required; these methods do not provide a checked invalid-type boundary.
Keys are copies of the supplied name, transformed bytewise with CRT tolower
(`0058909C–005890C6`, `0058ACAE` onward, helper `00586AC0`, callback `009F0543`).
ASCII case differences identify the same entry within a type. Names of different
types have separate namespaces. Empty nonnull name and null name are distinct.

Named lookup returns false on missing key. It does not clear a nonnull output
pointer on failure. With an existing key it returns true; a null output is a
validation-only lookup and acquires no object reference. A nonnull output gets
the stored pointer followed by its `+10` retain call (`005890CB–005890F8`).

Registration determines the bucket by calling the supplied object's type getter
at slot 0. A null name **always appends** to that bucket, returning the previous
element count; it neither invents a name nor adds a key to the named dictionary.
A new nonnull key appends and records that index. An existing nonnull key keeps
its index and replaces the stored pointer; supplying the identical pointer makes
no retain/release calls. Replacement retains the new pointer (`+10`) before
publishing it, then releases the old pointer (`+14`). An append retains one net
collection-owned reference; the caller's reference remains the caller's. In the
direct spare-capacity anonymous path the exact calls are retain temporary,
retain stored pointer, release temporary (`0058AC36–0058AC75`). Full-capacity
insertion uses the same reference-holding container service; it is not a raw
ownership transfer. The return is the per-type vector index, not a global
anonymous counter or a text name (`0058AC10–0058AD4C`).

| ID | Initial directory and operation | Predicted result (unexecuted) |
|---|---|---|
| DI1 | type 6 has `Mixed`→index 2/object A; register `mIXED`, type-6 B | return 2; B retained, entry becomes B, A released; bucket length unchanged |
| DI2 | same entry; lookup `MIXED`, type 6, null out | true, no retain; type 7 lookup with no corresponding key returns false |
| DI3 | type-6 length 4; register null name, object B twice | returns 4 then 5, each appends and holds one reference, no generated names |
| DI4 | same named pointer registered again | same index, no retain/release |
| DI5 | missing key with output sentinel A | false, output still A, no retain |

### 11.2 Native parser final release

The unknown table `00AA04D4` has QI/AddRef/Release entries
`0059F3B0/0059F3C0/0059F3D0`. After the adjustment used by these stubs, let the
shared receiver be S. The count lives at `S-88` hex. AddRef increments the u32
count and returns it (`00675FC0`); Release with count greater than 1 decrements
and returns it. With count at most 1, Release dispatches deleting destructor
slot 0 on `S-90` with argument 1, then returns zero (`00675FD0–00675FF3`). It
does not first store zero. This is a shared base operation, not a new parser-only
reference-count algorithm.

The parser's slot-0 deleting destructor `005A27F0` invokes `0059F4E0`; it then
frees the allocation through `005E5620` when argument bit 0 is set. This positive
vtable/call-chain identification separates destruction from callback `0059EBE0`.
Member cleanup first disposes priority/name structures and the temporary
descriptions. A held component at `P+490` is released through `+14` before the
group-record arrays and other description storage are destroyed
(`0059F5A0–0059F720`). After those structures and the four ID-map trees are
disposed, it releases nonnull interfaces in this exact order:
renderer-properties `P+A8`, current resource `P+94`, active collection `P+90`,
manager `P+8C`, each through slot `+08`, then tail-calls base destruction
`00679AB0` (`0059F818–0059F85D`). Aliased pointers still receive their separate
held-reference releases. These are destructor reads/calls, not SetManager's
acquire-before-release assignment protocol; the slots are not cleared first.

| ID | Initial live parser references | Predicted calls (unexecuted) |
|---|---|---|
| LT1 | count 3 then Release | count 2 returned; no destructor |
| LT2 | count 1; P+490=C, A8=R, 94=U, 90=K, 8C=M; successful returning cleanup | C.`+14`, description/container cleanup, R.Release, U.Release, K.Release, M.Release, base destruction, allocation free, Release returns 0 |
| LT3 | final-release state as LT2 but 94 and 90 both K | K receives two releases in those positions; no alias suppression |
