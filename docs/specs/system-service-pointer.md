# Compiled system-service pointer utility

## 1. Evidence and scope

This is the first dedicated contract for original Windows `cRZSysServPtr`
utility bodies, separate from the capped file, INI, directory, stream, command
line and language owners. It specifies normal, readable, nonreentrant calls
whose foreign methods return normally. **[compat]** marks original behavior;
**[choice]** marks replacement design freedom. No original program was executed.

Primary evidence: canonical Windows export queried with `re_query.py func` and
`asm --bytes`. Export identity: `SimCity 4.exe`, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`
(`export-summary.txt`). All addresses below are original virtual addresses.
The read-only working PE independently has the same SHA-256, machine `0x014C`
(x86), PE32 optional-header magic `0x010B` and image base. Static comparison
of every exported instruction byte in the 43 named entries against that PE
found zero mismatches. This verifies source identity, not execution results.
No SDK or replacement implementation supplied the algorithms.
Template names are discovery labels, not proof of body identity. The provisional
43 named entries / 2,247 metadata bytes are neither coverage nor unique utility
byte credit. Foreign framework/service implementations are outside this owner.
The export splits some normal continuations and applies some incorrect or
overbroad names; the classifications below take precedence over those names.
Live REA availability is UNKNOWN: no live query was needed or attempted.

Outline: section 2 stored construction and identity words; section 3 destruction
and assignment; section 4 temporary lookup wrappers and excluded identity;
section 5 proposed traces; section 6 limits and open questions.

## 2. Ordinary stored-pointer construction

**[compat]** Verified bodies at `0x004302E0` (SndSys), `0x00456A30`
(Registry), `0x00449690` (MessageServer2), `0x004496C0` (Persist),
`0x00449720` (Draw) and `0x008FA948` (WWW) operate on a single pointer-sized
field at object offset zero. There is no wrapper vtable written by these bodies.
The accessed field is exactly 32 bits; this is not proof that every nominal
template instance has the same complete object layout.

Construction first clears that field. It reads the framework pointer through
`0x008793EC`, whose complete body loads the 32-bit word at `0x00B540AC` and
returns it. This is a borrowed framework access: the utility makes no framework
AddRef or Release call. If the framework is null, construction stops with a null
field. Otherwise it calls framework virtual offset `+0x14` with two literal
32-bit identity words and the address of the field as output. The constructor
does not test either AL or EAX after that call. It returns the wrapper address
in EAX. ECX supplies the wrapper on entry; there are no stack arguments.

For example, SndSys clears at `0x004302E3`, tests the framework at
`0x004302EE`, supplies output and literals at `0x004302F4..0x004302FA`,
calls at `0x00430301`, and returns the wrapper at `0x00430304`.
WWW uses a shorter three-byte zeroing instruction at `0x008FA94B`, but the
same normal algorithm. No direct service AddRef, QueryInterface, retry or
failure cleanup occurs in these constructors.

Consequently a foreign lookup that returns false but writes a valid nonnull
pointer still leaves that pointer stored. A lookup returning true with null
output leaves the wrapper null. These are utility branch consequences, not a
claim that the original framework produces both combinations. Output not written
remains the initial null. Foreign reference acquisition is not inferred merely
from the boolean return.

### 2.1 Identity words and argument channels

**[compat]** The logical argument order at framework virtual `+0x14` is
service identity, interface identity, 32-bit output address; ECX is the
framework pointer. The output address is pushed first, the interface word next,
the service word last. They are independent literals, not derived by adding or
subtracting a fixed difference. The wrapper makes no separate manager or
interface-query call. It expects the virtual call to consume the three
four-byte stack arguments; the wrapper has no corresponding stack adjustment.
Any boolean/status channel returned by lookup is separate from the written
pointer. These bodies do not establish whether its producer defines only AL
or all of EAX: neither is read before EAX is overwritten with the wrapper.

The template discovery labels independently spell the interface value as their
second parameter and service value as their third. Their decimal values match
the literal instruction operands below. Registry service `0x25076B9E` is also
named `kSysServiceID` in the original ID-name table (entry `0x00B08878`).
Existing public interface facts corroborate the sound pair in
[audio.md](audio.md) (service/IID `0x23CECBA0`/`0x106077D2`) and MessageServer2
in [framework-gz-services.md](framework-gz-services.md) section 1. No foreign
lookup/QI implementation is incorporated here. For the remaining labels the
table certifies the actual two positional operands; it does not independently
certify each interface's complete type or registry behavior.

### 2.2 Verified stored-construction bodies

**[compat]** All entries in this table have the section 2 algorithm. `C40` is
the 40-byte form: field clear at entry +3, framework load call +9, null test
at +14, output push +20, interface push +21, service push +26, virtual lookup
call +33, wrapper return setup +36. `C37` is the shorter zeroing form:
clear +3, framework call +6, null test +11, output push +17, interface push
at +18, service push +23, virtual call +30, return setup +33. Offsets identify
instructions within each actual body; they are not guessed from the name.

| Discovery interface label | Body / form | Service word (argument 1) | Interface word (argument 2) |
|---|---|---|---|
| cIGZSndSys | `0x004302E0` C40 | `0x23CECBA0` | `0x106077D2` |
| cIGZGraphicSystem | `0x00449560` C40 | `0xC416025C` | `0x0073283C` |
| cIGZMessageServer2 | `0x00449690` C40 | `0x04FA845B` | `0x652294C7` |
| cIGZPersistResourceManager | `0x004496C0` C40 | `0x056B906E` | `0x656B8EFC` |
| cIGZLuaScriptServer | `0x004496F0` C40 | `0x4BD10133` | `0x4BD10129` |
| cIGZDrawSystem | `0x00449720` C40 | `0xEB2EA3CB` | `0x4B2EA3B3` |
| cIGZUIScriptService | `0x00449750` C40 | `0x5A356E15` | `0xFA3562FA` |
| cIGZRegistry | `0x00456A30` C40 | `0x25076B9E` | `0xA237613C` |
| cIGZClipboardManager | `0x004D7B30` C40 | `0x27EBFFFD` | `0x07EC0010` |
| cIGZMessageServer | `0x004D7BB0` C40 | `0x64063387` | `0xC1085722` |
| cIGZFileSystem | `0x004DE430` C40 | `0x2073215D` | `0xE0203660` |
| cIHitMan | `0x004ED100` C40 | `0x1B2CE671` | `0xBB1BD9A8` |
| cIGZSndMgr | `0x004EDFD0` C40 | `0x1AF85BFB` | `0x3AF85BB7` |
| nGZLog::cIService | `0x005C3E80` C40 | `0x09B69C4E` | `0x09B69C46` |
| cIGZCanvas | `0x005C3EB0` C40 | `0xA3460DEB` | `0x66B217A1` |
| cIGZFontSys | `0x005D4920` C40 | `0x20673960` | `0x00673960` |
| cIGZRealtimeDebugDisplay | `0x00602190` C40 | `0x8414A2C2` | `0x6054A2FB` |
| cIGZLanguageManager | `0x00610800` C40 | `0x441E5070` | `0x0054B7D5` |
| cIGZWinCtrlMgr | `0x007B2480` C40 | `0xC2C2EB0F` | `0x22C2EB1F` |
| cIGZWWWSystem | `0x008FA948` C37 | `0x449B1939` | `0x449B1912` |
| cIHitDebugInfo | `0x00936176` C37 | `0xFBC9CD04` | `0x7BBA0CBD` |
| cIGZWinIMEServer | `0x0099394D` C37 | `0x2AC45447` | `0x2AC44AE7` |

## 3. Destruction and reference order

### 3.1 Conditional release without clearing

**[compat]** SndSys `0x00430310` and Registry `0x00456A60` inspect the
32-bit field and do nothing to a null field. Otherwise they call the stored
interface's virtual offset `+0x08` once, with that interface in ECX. There is
no framework acquisition, no AddRef, no field clear and no stored replacement.
The numeric return of the foreign call is ignored. SndSys's decisive addresses
are `0x0043032F..0x0043033F`; Registry's are `0x00456A7F..0x00456A8F`.
WWW `0x008FA96D` has the same normal release rule at
`0x008FA97E..0x008FA98B`, despite its different exception-frame prologue.
These are no-stack-argument member calls. Their return value is not specified
as a usable pointer or reference count. The exception scaffolding does not
establish a public unwind or exception guarantee.

The following entries were instruction-compared, retaining field displacements,
widths, tests and call order. The `D74` form has field test at entry +31,
nonnull pointer load +43 and release call +47. The `D52` form has field test
at +17, nonnull load +26 and release call +30; its prologue uses the intrinsic
exception-frame helper `0x009F18D0`. That helper establishes the frame on its
normal path and preserves the service-call input ECX.

| Form | Discovery labels and actual entry addresses |
|---|---|
| D74 | SndSys `0x00430310`; MessageServer2 `0x004495C0`; Registry `0x00456A60`; MessageServer `0x004D7B60`; FileSystem `0x004DE460`; HitMan `0x004ED130`; SndMgr `0x004EDF00`; Canvas `0x005C3EE0`; FontSys `0x005D4950`; AnimationTickManager `0x005F8F20`; RealtimeDebugDisplay `0x006021C0`; LanguageManager `0x00610830`; WinCtrlMgr `0x007B24B0` |
| D52 | WWWSystem `0x008FA96D`; HitDebugInfo `0x0093619B`; WinIMEServer `0x00993972` |

No type-specific calls occur in those normal release paths. In particular,
`0x006021C0` can release the temporary used by the differently labeled
AEMSEvents wrapper below: its instructions contain no RealtimeDebugDisplay IID
or service test. This is body sharing in the call graph, not proof of a source
template specialization or linker folding history.

### 3.2 Clear before release body

**[compat]** The body labeled Persist destructor at `0x00874DA3` is different:
it captures the pointer, tests null, clears the field **before** invoking that
pointer's virtual offset `+0x08`, then returns the wrapper address in EAX.
Evidence: load/test `0x00874DA6..0x00874DAA`, clear `0x00874DAC`, call
`0x00874DB1`, return-address setup `0x00874DB4`. A null entry makes no
foreign call. This body is a clear-and-release operation on its ordinary path;
its auto-applied destructor name alone does not certify its source-level role.

#### 3.2.1 Clear-and-return-address use in live receivers (SSP2)

**[compat]** This body also serves as a callable **output-address preparation**
operation. Its receiver is the address of a live pointer field, not that
field's interface pointer. It takes no stack arguments; it returns the full
32-bit receiver address in EAX even when the field started null. After any
nonnull Release returns, the field is still null under the nonmutation
assumption. The result is neither a boolean nor the released interface's
reference count. The following native continuations immediately push that
EAX value as a foreign output address, rather than end the owner's lifetime:

| Caller discovery label / call site | W passed in ECX | Subsequent foreign call, receiver and literal arguments |
|---|---|---|
| WinIME `StartComposition`, `0x00995403` | `owner+0x54`, formed `0x009953D7`; ECX set `0x009953FE` | EAX=W pushed `0x0099540B`; virtual `+0x0C` at `0x00995416` on the separately saved `0x0090DDF1` result; logical arguments `0x2AC45170`, `0x2AC44A90`, W |
| WinIME `StartCandidateList`, `0x009954E6` | `owner+0x60`, formed `0x009954BA`; ECX set `0x009954E1` | EAX=W pushed `0x009954EE`; virtual `+0x0C` at `0x009954F9` on the separately saved `0x0090DDF1` result; logical arguments `0x2AC45173`, `0x2AC44A93`, W |
| HitChannel `CreateSnd`, `0x0093F371` | `owner+0x28`, restored/adjusted `0x0093F366..0x0093F36B` | EAX=W pushed `0x0093F379`; virtual `+0x00` at `0x0093F37F` on the separately saved interface from `owner+0x24`; logical arguments `0x09D002D2`, W |

The first two paths check [W] for null before reaching the preparation
(`0x009953DA..0x009953DD`, `0x009954BD..0x009954C0`); a nonnull field
bypasses this sequence. In the specified nonreentrant domain, those paths
therefore make no Release during preparation. The third path imposes no
such test on its output field immediately before preparation: the tested
interface at `owner+0x24` is a separate field. No general owner, allocation
or foreign factory algorithm is specified by these call slices.

These uses positively establish a clear/release-and-return-address role on
live storage. They do not prove a source spelling such as destructor, reset,
address conversion, or a folded specialization. In particular, the Persist
discovery label supplies no Persist IID test to this body. SSP2 stays open
only for that source attribution; the observable contract is established.
The selected caller slices and body were byte-compared to the same PE with
zero mismatches. No source-level object-lifetime reconstruction is required
to use their normal raw-pointer contract.

### 3.3 Assignment (SndSys body)

**[compat]** At `0x004F2020`, the incoming stack argument is a raw interface
pointer, not the address of another wrapper. If nonnull, its virtual `+0x04`
is called first. Then, if the old field is nonnull, its virtual `+0x08` is
called. Only after those calls does the field receive the new pointer. There
is no pointer-equality shortcut: same-pointer assignment performs both calls
in that order. Null replacement releases the old pointer and then stores null.
Old null/new null performs neither call. Foreign numeric reference returns
are ignored. ECX is the destination wrapper; EAX returns that wrapper; the
callee removes one four-byte argument.

Evidence: argument load `0x004F2046`, new-pointer test/call
`0x004F2049..0x004F2051`, old-pointer test/call
`0x004F2054..0x004F2064`, final store `0x004F207C`, wrapper return
`0x004F207E`, `ret 4` at `0x004F2090`. The export splits the continuation
at `0x004F2075` into a separate function label; it is reached by both normal
branches of this body. Counting only the named 73-byte fragment misses the
store and return. During both foreign calls the wrapper still contains its old
pointer. Reentrant mutation and abnormal foreign exits are outside this rule.

For reference accounting, the wrapper consumes a nonnull lookup output as a
future Release obligation; it does not add a reference itself. Assignment
instead explicitly requests a new reference before dropping the old one.
The foreign primitives conventionally called AddRef (`+0x04`) and Release
(`+0x08`) remain opaque here: the utility does not inspect their returned
counts, deletion decisions or service state. A lookup-owned reference is
balanced by one wrapper Release only if that foreign lookup actually supplied
such a reference. A malformed or borrowed-only output is not repaired.

### 3.4 Draw construction followed by shared callable cleanup (SSP1)

**[compat]** The actual callable cleanup for the Draw stack wrappers in the
two paths below is D74 `0x004495C0`, despite that body's MessageServer2 label.
This is a direct native call, not cleanup inlined into the caller, and not
`0x0082AE40`. Both paths pass the same four-byte local's address in ECX to
Draw construction `0x00449720` and then to cleanup. The caller does not pass
the stored interface directly as the cleanup receiver. Inside `0x004495C0`,
the null test is at `0x004495DF`; for nonnull P, ECX changes from W to P at
`0x004495EB`, and P's virtual `+0x08` is called at `0x004495EF`. There is
no further receiver adjustment, lookup, AddRef or wrapper clear. No stack
argument is consumed, and no usable destructor return value is specified.
The wrapper word remains P after normal cleanup; that is an observation of
storage, not permission to reuse an ended object or dereference a dead P.

| Caller path (discovery label) | Exact wrapper storage / construction | Normal use and branch | Cleanup identity |
|---|---|---|---|
| `cGZWin::SetAreaToDrawTo`, `0x0099CF6A` | W=`EBP-8`; address formed `0x0099D03E`, constructor call `0x0099D041` | `0x0099D046` tests the **stored word** through returned EAX=W; `0x0099D04C..0x0099D04F` writes its nonnull predicate as one byte to `0x00BAC05C` | W address re-formed at `0x0099D049`; direct call `0x0099D054` to `0x004495C0`, for either null or nonnull output |
| `cGZWin::create_private_buffer`, `0x0099D0ED`, construction-taking branch | W=`EBP-4`; address formed `0x0099D115`, constructor call `0x0099D118` | Stored word tested `0x0099D120`; nonnull path uses P as ECX for foreign virtual `+0x14` at `0x0099D147`; null path bypasses that call | W address formed `0x0099D150`; direct call `0x0099D153` to `0x004495C0` |

The first path's guard can bypass construction entirely (`0x0099D035` to
`0x0099D059`); that bypass does not destroy an unconstructed local. On the
construction-taking path, lookup status is still ignored: false/output P
sets the predicate to one and releases P; true/output null sets it to zero
and makes no reference call. The byte is written before cleanup. The later
call to arithmetic body `0x0082AE40` at `0x0099D071` instead receives
`owner+0x24` and a stack argument; it is separate from this already completed
wrapper cleanup. No arithmetic behavior is specified here.

In the second path, all the normal successors after construction converge
on `0x0099D150` (including `0x0099D16C`, `0x0099D17A`,
`0x0099D19D`, `0x0099D1A8`). Its earlier return at `0x0099D113`
precedes construction and bypasses cleanup. The byte result of this owner
operation is staged in BL, then copied to AL at `0x0099D158` **after** the
cleanup call; it is not the destructor's EAX result. Its owner field at
`owner+0x64` is separate from W: for example `0x0099D134..0x0099D137`
releases and clears that field before P's virtual use. These owner effects
must not be mistaken for a clear of the Draw wrapper. The foreign operations
and owner algorithm are outside this contract.

This establishes a shared, independently callable cleanup body for actual
Draw wrapper lifetimes, without certifying the source template name, linker
folding history, or exceptional cleanup. It does not assert that these two
paths exhaust all Draw uses. Their new instruction ranges and the two utility
bodies were compared to the matching PE; no mismatches were found.

## 4. Temporary lookup wrappers and identity exclusions

### 4.1 Lookup, release, then return a saved pointer

**[compat]** `0x00602600` and `0x00602640`, despite constructor labels,
do **not** initialize a caller-provided wrapper. Incoming ECX is not used as
an object address. The initial push reserves a local word that is immediately
overwritten with zero; its final pop does not restore incoming ECX. These are
no-stack-argument lookup helpers with a private four-byte stack temporary. They clear that
temporary, obtain the framework through `0x008793EC`, and, when nonnull, pass
the temporary's address to framework `+0x14`. Lookup status is ignored.
They save the temporary's pointer value, run a conditional release destructor
on the temporary, then return the saved pointer in EAX. They make no AddRef
on that saved result and no additional lookup on destruction.

| Actual helper | Service word | Interface word | Local destruction |
|---|---|---|---|
| `0x00602600` (Tick label) | `0x86AD10EC` | `0x86AD10ED` | calls D74 `0x005F8F20` |
| `0x00602640` (AEMSEvents label) | `0xAC1FAE31` | `0x8C1FAEB7` | calls D74 `0x006021C0` |

The two bodies have the same verified shape: local clear at entry +1,
framework call +8, framework null test +13, output address formation +19,
interface push +23, service push +28, lookup call +35, saved pointer load +39,
local destructor call +47 and return-pointer setup +52. In particular,
Tick's `0x00602627` precedes destruction at `0x0060262F` and return setup
at `0x00602634`; AEMSEvents uses `0x00602667`, `0x0060266F`,
`0x00602674`. Lookup false/nonzero-output still triggers that release.
Lookup true/zero-output does not. Null framework returns null after the
null temporary's destructor path. The result carries no surviving reference
acquired by these helper bodies: its continued validity requires a lifetime
guarantee from outside them. No such foreign guarantee is asserted here.

### 4.2 Names that do not establish the supposed utility method

The body labeled Draw destructor at `0x0082AE40` is excluded from this
pointer-lifetime contract. It reads a stack argument (`0x0082AE40`), accesses
four destination words through offsets `+0`, `+4`, `+8`, `+12`, performs
arithmetic and writes them (`0x0082AE54..0x0082AE6B`), and returns with
one four-byte argument consumed (`0x0082AE6F`). It has no pointer null test,
framework acquisition or virtual reference call. The actual Draw stack-wrapper
cleanup is instead directly linked to `0x004495C0` by the lifetimes in section
3.4. No arithmetic-owner algorithm is claimed or researched here.

The Persist-labeled clear-and-release body in section 3.2 has a genuine pointer
algorithm, but not a certified source-level destructor identity. Section 3.2.1
follows three of its eight exported direct calls to establish returned-address
use on live pointer fields. This is stronger evidence than heterogeneous
caller names alone, but still does not identify source-template spelling or
linker-folding history.

Thus the 43 named entries divide into 19 C40, three C37, thirteen D74,
three D52, two temporary lookup helpers, one clear-and-release body, one
assignment fragment and one foreign/mislabeled body. The split assignment
continuation is additional evidence, not a 44th certified named template entry.
These are native body classifications, not whole-owner coverage or a promise
that unnamed/inlined instantiations have been found. No raw thunk was found
among these 43 bodies; the two called intrinsic helpers and assignment
continuation were studied only as needed for this utility.

## 5. Proposed independent trace ledger

Every trace below is **PROPOSED / UNEXECUTED**, derived from the original
instructions, not from replacement output. They are conformance proposals,
not game results or default service availability assertions. Let `W` be a
wrapper at `0x0012F000`, `P=0x01001000`, `Q=0x01002000`. All referenced
objects and tables are readable, methods return normally, and no method mutates
the wrapper. Event `Lookup(S,I,O)` records separate service, interface and
output-address channels, and independently records returned status and output
write. `Ref+`/`Ref-` record virtual `+0x04`/`+0x08` calls, not inferred
counts from foreign implementations. Observe the field during each call and
at normal return without dereferencing a pointer after its lifetime ends.

### 5.1 Output and boolean are independent

Run each of S01-S06 using SndSys C40 `0x004302E0` and D74 `0x00430310`.
The exact lookup identity must be service `0x23CECBA0`, interface
`0x106077D2`, output `0x0012F000`. Initially poison W's field with
`0xDEADBEEF` to distinguish construction from reuse or assignment. The poison
must be overwritten with zero before lookup and must never be released.
For status rows use raw AL `0x00` or `0x01`; vary upper EAX bits arbitrarily.

| ID | Foreign stimulus | Constructor field / EAX | Subsequent destructor events |
|---|---|---|---|
| S01 | Framework null | null / W; no Lookup | none |
| S02 | Lookup AL=0, writes null | null / W | none |
| S03 | Lookup AL=1, writes null | null / W | none |
| S04 | Lookup AL=0, writes P | P / W | `Ref-(P)` once, field still P during call |
| S05 | Lookup AL=1, writes P | P / W | same as S04 |
| S06 | Lookup AL=0, leaves output untouched | null / W | none |

The constructor makes no `Ref+` event in any row. S04 is a discriminating
failure-output case, not evidence that the real framework emits it. Repeat
S05 with Registry C40 `0x00456A30` / D74 `0x00456A60`: the exact words
must become `0x25076B9E`, `0xA237613C`, with the same field/call rule.
Repeat S03/S04 with WWW C37 `0x008FA948` / D52 `0x008FA96D` and
WinIME C37 `0x0099394D` / D52 `0x00993972`, using their distinct section
2.2 words. These distinguish instruction forms without inventing a common
source destructor or treating either word as the other identity.

### 5.2 Assignment and reference ledgers

Run these through `0x004F2020` and its `0x004F2075` continuation. All
return EAX=W and consume the four-byte incoming raw-pointer argument.

| ID | Initial field / raw argument | Ordered events and final field |
|---|---|---|
| A01 | null / P | `Ref+(P)` observing null, then store P |
| A02 | P / Q | `Ref+(Q)` observing P; `Ref-(P)` observing P; store Q |
| A03 | P / P (including argument loaded from this same W) | `Ref+(P)`; `Ref-(P)`; store P; field P throughout both calls |
| A04 | P / null | `Ref-(P)` observing P; store null |
| A05 | null / null | no reference calls; store null |

For an explicit **synthetic reference-counter fixture**, let lookup grant one
reference and let `Ref+` increment and `Ref-` decrement without other effects.
Those fixture rules are inputs, not researched foreign service algorithms.
S05 then has acquired +1, constructor +0, destructor -1, net zero. In A02,
start P with one wrapper reference and Q with one external reference: Q goes
1 to 2 before P goes 1 to 0; final destruction of W releases Q 2 to 1.
In A03 with P initially at 1, counts are 1 to 2 to 1, and final destruction
1 to 0. A04 takes P 1 to 0. Arbitrary values returned by the reference
primitives must not affect any final field or branch above.

### 5.3 Distinguishing destruction and local lifetimes

| ID | Operation and stimulus | Required ordered observations |
|---|---|---|
| L01 | D74 or D52 with field P | one `Ref-(P)` observing field P; no wrapper store or lookup; do not treat destructor EAX as a pointer result |
| L02 | clear-and-release `0x00874DA3` with field P | field becomes null before `Ref-(P)`; EAX=W on return |
| L03 | clear-and-release with null field | no reference calls; field null; EAX=W |
| L04 | Tick helper `0x00602600`, framework present, AL=1/output P | `Lookup(0x86AD10EC,0x86AD10ED,local)` sees local zero; save P; `Ref-(P)` observes local P; return EAX=P, no `Ref+` |
| L05 | Tick helper, AL=0/output P | same release and result as L04 |
| L06 | Tick helper, AL=1/output null, or framework null | no reference calls; EAX=null; framework-null case has no Lookup |
| L07 | AEMSEvents helper `0x00602640`, AL=1/output Q | `Lookup(0xAC1FAE31,0x8C1FAEB7,local)`; `Ref-(Q)` through `0x006021C0`; EAX=Q, no `Ref+` |
| L08 | two successive ordinary W lifetimes, first output P, second Q | framework read and distinct lookup for each construction; release P at first destruction and Q at second; no utility cache or re-acquisition during either destruction |
| L09 | two successive Tick helper calls, outputs P then Q | two framework reads/lookups, local Release P before first return, local Release Q before second return; no retained reference between helper calls |

For L04/L05 with the synthetic counter fixture, grant +1 at lookup and remove
it at local destruction: net zero **before** return. If that was the final
reference, EAX still contains the saved numeric pointer value; the trace must
not dereference it. This separates pointer return from retained ownership.
For two live ordinary wrappers acquired separately from lookup with the same
P, each has its own one-Release obligation. Their destruction order is the
caller's input, not a LIFO rule inferred from the template.

### 5.4 Draw cleanup identity ledger

Each row is independently **PROPOSED / UNEXECUTED**. Use Draw constructor
`0x00449720` with W=`0x0012F000` (initial field `0xDEADBEEF`) and the
actual cleanup target `0x004495C0`. Present-framework lookup must receive
service `0xEB2EA3CB`, interface `0x4B2EA3B3`, output W, and observe field
zero. Constructor EAX is W in every row. There is no `Ref+` event.
Ignore destructor EAX; the cleanup is `ret`, with no stack argument.

| ID | Lookup stimulus / path | Required field and ordered effects |
|---|---|---|
| D01 | AL=0, output P | W becomes P; cleanup receives W, loads P into ECX, calls `Ref-(P)` while W=P; W remains P after cleanup |
| D02 | AL=1, output Q | W becomes Q; same receiver adjustment W to Q; exactly `Ref-(Q)` observing W=Q; W remains Q |
| D03 | AL=1, output null | W=null; cleanup receives W but makes no foreign call; W stays null |
| D04 | AL=0, output untouched, or null framework | W=null; no reference call; null-framework case has no Lookup |
| D05 | `0x0099D041` construction-taking path, AL=0/output P | W=P; byte at `0x00BAC05C` becomes 1 before cleanup at `0x0099D054`; `Ref-(P)` observes W=P and byte=1; byte remains 1 and W remains P |
| D06 | same caller, AL=1/output null | W=null; byte becomes 0 before cleanup; no reference call; byte=0 and W=null afterward |
| D07 | `0x0099D118` nonnull path, lookup AL=0/output P, separate owner field initially null; P's foreign `+0x14` returns AL=0 or AL=1 | foreign use receives P, not W; cleanup at `0x0099D153` then calls `Ref-(P)` with W=P; W remains P; owner returns AL=0 or AL=1 respectively, copied from BL after cleanup, independently of arbitrary Release EAX |

In D07 the foreign call's output is directed to `owner+0x64`, not W;
choose an output Q or null independently of its AL to distinguish the fields.
Its value after that normally returning foreign call remains that supplied
value through the wrapper cleanup. This is a stimulus to a raw primitive,
not a claim about actual service success/failure outputs. D01/D02 distinguish
the shared no-clear destructor from clear-and-release L02, which instead
observes W=null during Release and returns full EAX=W. No row executes the
arithmetic entry as a pointer destructor or infers reference acquisition from
lookup's status. No repeated destruction of one lifetime is proposed.

### 5.5 Clear-and-return-address use ledger

These additional rows are **PROPOSED / UNEXECUTED** and independent of the
Draw rows. Let W be a live four-byte output field. Call `0x00874DA3` with
ECX=W and no stack argument. Only after its return make the specified opaque
foreign call with output address EAX=W. Do not treat that return as an
interface pointer. For the caller rows use the exact field and literal words
in section 3.2.1. Foreign methods do not mutate W except for the explicit
output write in the stimulus.

| ID | Initial field / stimulus | Required ordered observations |
|---|---|---|
| R01 | [W]=P, preparation followed by a foreign write of Q | save P; clear [W]; `Ref-(P)` sees [W]=null; return full EAX=W regardless of Release EAX; foreign output receives W and sees null, then writes Q; final [W]=Q; no `Ref+` |
| R02 | [W]=null, preparation followed by a foreign write of P | no reference calls; EAX=W; foreign output sees null and writes P; final [W]=P |
| R03 | [W]=Q, preparation followed by no output write | clear [W]; `Ref-(Q)` sees null; EAX=W; subsequent foreign call sees null and leaves it null |
| R04 | either WinIME caller's construction-taking sequence, [W]=null, foreign AL=0/output Q or AL=1/output null | preparation emits no reference call; passes the exact W as output; final field immediately after foreign return is Q or null respectively; the call's AL is separate from preparation's full EAX=W |
| R05 | either WinIME caller's initial field P | the nonnull guard bypasses this preparation/output sequence; do not expect a Release from `0x00874DA3` on that path |

R01-R03 specify the callable primitive and output sequence, not a promise
that every caller accepts every starting state. R01 can also be proposed
for the HitChannel slice, whose output field is not guarded for null there.
The callback counts, any ownership acquired through the output, and the
foreign result's meaning remain foreign inputs. Unlike assignment A02,
preparation does not AddRef a replacement and does clear before Release;
unlike Draw cleanup D01, it returns a usable field address and leaves no old
pointer in the field. These distinctions do not depend on resolving SSP2's
source-language name.

## 6. Boundaries and unresolved identity

**[choice]** A replacement may choose its storage and implementation technique,
provided the selected entry's normal pointer, call and ordering contract above
is preserved. Do not implement every label as one uniform smart pointer.

No guarantees are established for malformed output addresses, invalid pointer
vtable contents, reentrant mutation, allocation failure, foreign exceptions,
unwinding, thread safety, cancellation or global-cache lifetime. Normal
constructor clearing is not a supported reset of an already-owned pointer;
normal destructor field contents do not authorize a second destruction of a
dead object. No copy-constructor, wrapper-to-wrapper assignment, conversion,
native factory insertion, service repair or service-manager algorithm has
been established here. Missing nominal destructors are not filled in by analogy.

Open attribution and domain questions are in
[questions/system-service-pointer.md](questions/system-service-pointer.md).
They do not weaken the directly verified normal body rules, and do not reopen
the capped RZ owner or its questions.
