# Property setter helpers

## 1. Scope and evidence

This specifies the helper-owned normal returned-call paths of eleven Windows 1.1.641
`cSCPropertyHelp::SetPropertyValue` entries. It is the first dedicated family study,
round 1 of at most 3; it does not reopen exemplar converters or Variant storage research.
Original instruction evidence, rather than generated overload names or decompiler prototypes,
determines the contract. No game/native execution or comparison is claimed.

Prior overlap audit (2026-10-23): residential-education-health §2 / RG16 already establishes
bool/u32/f32 scalar lookup/create/install; protection's PR9 setter paragraph establishes scalar
u32 versus copied u32 array and inherited-property mutation; plumbing names typed setters;
occupant-manager §AddOccupantGroup names a u32-array consumer; lot-records and lot-developer
also describe public consumers, not complete helper algorithms. These remain valid partial
inputs. An address/family search of public specs and lane reports found no earlier dedicated
complete contract for these eleven entries. Owner-level legacy coverage is not exact-entry
algorithm qualification. EX52f holder slot-14/16 wrappers, including `0x005FE05B`, are different
functions: their Release return channel does not apply here.

Source: the main checkout's `bin/SimCity4.exe`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, and the matching
`ghidra/export/windows/export-summary.txt` (image base `0x00400000`). `re_query.py func`,
`callers` and `asm --bytes` were queried first; decompilation was navigation only and
omits actual value arguments in some prototypes. Addresses below are original Windows VAs.
Read-only PE mapping confirmed machine `0x014C` (x86), that image base, and every
exported instruction byte in the eleven contiguous bodies (1,585 bytes). The normalized
first 115 bytes of each body have identical instruction text and relative branch
destinations. This verifies the common branch attribution, not eleven independent algorithms.
No live provider was needed; no REA query or provider failure is claimed.

## 2. Common acquisition, installation and update

All eleven helpers take a holder pointer and 32-bit property ID as their first two stack
arguments, followed by the value argument described in §3. There is no helper `this`
receiver. The holder becomes the ECX receiver for its virtual calls. Each helper ends
with plain `RET`; the caller removes the helper arguments.

The common sequence is supported directly at `0x005FE210–0x005FE299` and corresponding
offsets in the other entries:

1. Call holder slot 9 (`+0x24`) with the ID. Test the full returned pointer for zero
   (`0x005FE220–0x005FE227`), not AL, a separate status, or an output parameter.
2. If nonzero, use that exact property identity. Do not create, install, clone, AddRef,
   or localize it. An inherited lookup result is updated in place. This preserves the
   shared-exemplar behavior already documented by protection PR9.
3. If zero, read the pool descriptor through `0x00B0A9EC`, then its free-head at `+8`.
   If that head is zero, call `0x0090CF9A` with receiver `0x00B0A9E8` and use its
   returned pointer. There is no null check after that call. Pool allocator internals
   are foreign to this contract.
4. Read the first word of the chosen block, store it as the descriptor's new free-head,
   and increment the descriptor's 32-bit word at `+0xC` by one. These stores precede
   construction and installation (`0x005FE242–0x005FE259`). This is pool bookkeeping,
   not evidence of a property reference-count increment.
5. Set the block's first word to `0x00AA6C54`; call the Variant constructor
   `0x009166D4` with receiver block `+4`; then set the block's first word to
   `0x00AA6988`, the word at block `+4` to `0x00AA6788`, and the ID word at
   block `+0x18` to the caller's ID, in that order (`0x005FE25C–0x005FE277`).
   Constructor initial value/refcount internals remain foreign; these helpers do not
   explicitly initialize another reference-count field. Protection's existing empty-new-value
   description remains a constructor-level input, not a newly proved default here.
6. Call holder slot 15 (`+0x3C`) with `(newProperty, 0)` before setting the requested
   value (`0x005FE27A–0x005FE27F`). Ignore its return bits and continue with the same
   new property pointer, without relookup. The zero is an argument, not a tested result.
7. Call property slot 5 (`+0x14`) on the selected property, and use the returned Variant
   pointer directly as the receiver for the typed setter. There is no null check.
   The newly selected original property table's slot 5 targets `0x005FD7E0`, whose
   non-null receiver returns receiver `+4` (`0x005FD7E4`); its misleading vector
   `operator[]` auto-name does not describe this call. A foreign existing property may
   return a different valid Variant identity: the helper uses that result without adjustment.
   Dispatch the value call in §3. Its result is not tested.
8. Restore saved registers and return. There is **no helper-owned temporary property or
   Variant AddRef/Release**, before or after install/value dispatch; no rollback or
   failure cleanup. The extra saved EDI on the new-property path is just a register save.

The only helper-owned conditional branches are the lookup-pointer and free-head tests.
No Boolean installation failure prevents the value call. Any holder/Variant effects already
performed survive a normally returning primitive failure; the helper makes no compensating
call. Which objects the holder retains, reference-count deltas inside foreign calls, and
typed-setter storage effects are the primitives' contracts, not additional helper algorithms.

There is no public output pointer or helper success flag. EAX/AL after normal return is
exactly what the final typed call leaves there: the epilogue does not alter it. This is
raw register passthrough, **not** a Boolean API or a final Release result. A caller must
not interpret unspecified upper bits as a full-width Boolean. The public operational
contract is void-like (no success result); the machine evidence cannot recover a source
declaration merely from `RET`. For example `SetAutoBudget` ignores the result and removes
12 bytes (`0x006F7DB4–0x006F7DBE`); the s64 caller removes 16 bytes without checking a
result (`0x006DBDB4–0x006DBDBC`). Do not substitute EX52f's cleanup return channel.

## 3. Overload inventory and argument dispatch

The third argument begins at entry stack `+0xC`. Each row has its own complete body;
none is a thunk. Counts below are the helper's computation, not a host vector abstraction.

| Entry | Bytes | Input and helper stack footprint | Final Variant slot / byte offset | Dispatch evidence |
|---|---:|---|---|---|
| `0x005FE210` | 138 | bool, forwarded 32-bit stack word; 12 bytes total | 11 / `+0x2C` | `0x005FE28A–0x005FE293` |
| `0x005FE2A0` | 138 | u8, forwarded 32-bit stack word; 12 | 14 / `+0x38` | `0x005FE31A–0x005FE323` |
| `0x005FE330` | 138 | s32 word; 12 | 29 / `+0x74` | `0x005FE3AA–0x005FE3B3` |
| `0x005FE3C0` | 138 | u32 word; 12 | 26 / `+0x68` | `0x005FE43A–0x005FE443` |
| `0x005FE450` | 146 | s64 low then high word; 16 | 35 / `+0x8C` | `0x005FE4CA–0x005FE4D8` |
| `0x005FE4F0` | 141 | f32 raw word; 12 | 38 / `+0x98` | `0x005FE56A–0x005FE573` |
| `0x005FE580` | 141 | string interface pointer; 12 | 56 / `+0xE0` | `0x005FE5FA–0x005FE603` |
| `0x005FE610` | 149 | u8 vector descriptor pointer; 12; byte count = end − begin | 94 / `+0x178` | `0x005FE68A–0x005FE69B` |
| `0x005FE6B0` | 152 | u32 vector descriptor pointer; 12; count = (end − begin) arithmetic-right-shift 2 | 102 / `+0x198` | `0x005FE72A–0x005FE73E` |
| `0x005FE750` | 152 | f32 vector descriptor pointer; 12; count = (end − begin) arithmetic-right-shift 2 | 110 / `+0x1B8` | `0x005FE7CA–0x005FE7DE` |
| `0x005FE7F0` | 152 | s64 vector descriptor pointer; 12; count = (end − begin) arithmetic-right-shift 3 | 108 / `+0x1B0` | `0x005FE86A–0x005FE87E` |

Vector dispatch loads begin from descriptor `+0`, then end from `+4`, **after** acquiring
the Variant pointer. It reads no capacity word. It subtracts in 32-bit registers and
passes `(begin, computedCount)` without changing begin, including count zero. It does not
iterate, copy elements, clamp a count, or test emptiness itself. A valid domain here means
an independent readable buffer and aligned element-sized nonnegative extent representable
in the signed count calculation; malformed/wrapping/overlapping descriptors are not promised.

Scalar forwarding performs no helper conversion, saturation, Boolean normalization, or
floating arithmetic. s64 pushes high then low so the callee sees the original low/high
pair. Typed Variant primitive behavior remains separately owned. In particular a one-element
array selects an array slot, whereas a scalar selects a scalar slot.

### 3.1 Raw foreign primitive boundary

The following identifies only the call contract needed by these helpers. It is not a new
Variant storage/conversion or allocation algorithm specification. Targets are the words in
the original embedded Variant table `0x00AA6788`, verified against the PE bytes and the
public interface table. Existing properties dispatch through their own Variant table.

| Helper entry suffix | Original final target (including adapter) | Primitive inputs / width | Normal residual return evidence |
|---|---|---|---|
| `E210` | `0x00916657` → `0x009157B1` | bool low byte in a 4-byte slot; callee pops 4 | AL = input low byte; remaining EAX bits unspecified by this contract (`0x009157CE–0x009157D6`) |
| `E2A0` | `0x0091665C` → `0x009157D9` | u8 low byte in a 4-byte slot; pops 4 | AL = input low byte, not a success flag (`0x009157F6–0x009157FE`) |
| `E330` | `0x00916675` → `0x009158A5` | s32 raw word; pops 4 | EAX = supplied word (`0x009158C2–0x009158CA`) |
| `E3C0` | `0x00916670` → `0x0091587D` | u32 raw word; pops 4 | EAX = supplied word (`0x0091589A–0x009158A2`) |
| `E450` | `0x0091668A` → `0x009158FC` | s64 low/high; adapter and primitive each pop their own 8 bytes | EAX = high word, not EDX:EAX s64 result (`0x00915919–0x00915928`) |
| `E4F0` | `0x0091669A` → `0x0091592B` | f32 adapter loads/stores a 32-bit float through x87; pops 4 | EAX = word delivered by adapter (`0x00915948–0x00915950`); no helper float return |
| `E580` | `0x009166CF` → `0x00915A1D` | string interface pointer, unchanged; pops 4 | untyped residual EAX of delegated string-setting call, no status promise (`0x00915A3A–0x00915A43`) |
| `E610` | `0x00914F45` | `(u8 data, elementCount)`; pops 8; byte extent count | untyped residual EAX; no Boolean result |
| `E6B0` | `0x00915014` | `(u32 data, elementCount)`; pops 8; byte extent count × 4 | same |
| `E750` | `0x009152AB` | `(f32 data, elementCount)`; pops 8; byte extent count × 4 | same |
| `E7F0` | `0x00915239` | `(s64 data, elementCount)`; pops 8; byte extent count × 8 | same |

The four vector targets are **copying** inputs, despite some metadata calling them `Ref*`.
Their direct copy calls are `0x00914F6D/0x00914F8F`, `0x00915042/0x0091506E`,
`0x009152D9/0x00915305`, and `0x00915267/0x00915293`, respectively. These establish
byte extents and copying invocation; destination management and success/failure domains stay
foreign. The matching normal array tags are `0x8002`, `0x8006`, `0x800A`, `0x8009`.
An empty descriptor still invokes the primitive once, including a null begin when begin=end=0;
this does not promise any particular zero-allocation pointer or residual EAX. Nonempty copies
require valid readable independent storage. No overlap, self-copy or interior-alias guarantee follows.

The f32 helper itself forwards raw bits unchanged. The original scalar adapter performs
`FLD`/`FSTP` (`0x0091669A–0x009166A7`), so do not generalize that observation into
preservation of every NaN payload or exceptional x87 input. Normal `+0`, `-0`, and exactly
representable finite scalar examples below need no arithmetic conversion. The f32 array
primitive copies bytes and does not run that scalar adapter per element. The string helper
does not dereference or adjust its third argument; the raw adapter's string operations remain
foreign. Neither a null string nor aliasing it with the destination is a supported case here.

## 4. Proposed trace ledger

**PROPOSED / UNEXECUTED.** These are independent expectations derived from original branches,
dispatches and literal arithmetic, not portable implementation output, an executable test,
or a native comparison. Foreign primitives below supply only individual call results and
stated side effects; none supplies a precomputed helper acquisition/update/refcount outcome.
No trace is a new supported primitive failure mode merely because arbitrary register bits
can be observed after a void call.

### 4.1 Identities, sentinels and exact sequence vocabulary

Use asymmetric IDs `IA=0x13579BDF`, `IB=0xA2468CEF`; holder H; existing own property P;
inherited property Q; fresh pool block N; distinct variants VP, VQ, VN. For an original
new object, VN=N+4. P and Q start with deliberately different old types/values and property
reference-count sentinels 17 and 29. These are chosen fixture counts, **not native defaults**.
For N, call its post-constructor count C (unknown natively); C=13 may be chosen by a trace
fixture. The pool descriptor starts with free-head N, N's free-link R distinct from N,
and live-block word 41. Caller sentinel O=`0xCC33AA55` is unrelated readable/writable memory,
not a hidden helper output. A vector's ignored capacity word can be `0xD00DFEED`.

| Sequence | Exact helper-owned order, with each foreign call's result supplied independently |
|---|---|
| E(P, IA, S) | holder lookup slot 9 `(IA)` → P; property slot 5 on P → VP; typed slot S on VP with the original third input; restore/RET. No pool access/install/ref calls. |
| E(Q, IB, S) | lookup `(IB)` → Q; property slot 5 on Q → VQ; typed slot S on VQ; restore/RET. H receives no own replacement property from the helper. |
| M(IA, S) | lookup `(IA)` → 0; read descriptor head N; read N's first word R; head:=R; live-block word:=42; N's first word:=`0x00AA6C54`; constructor on N+4; N's first word:=`0x00AA6988`; N+4 first word:=`0x00AA6788`; N+0x18:=IA; holder slot 15 `(N,0)`; property slot 5 on **N** → VN; typed slot S on VN; restore/RET. No relookup or release. |
| A(IB, S) | as M, but head initially 0: call allocator `0x0090CF9A` on `0x00B0A9E8` → N, then read N's link R and perform the same stores. ID store/install/value dispatch use IB. The allocator's internal pool effects are supplied separately, not inferred; for a count-after-allocator of 41 the helper stores 42. |

On all four sequences the helper's own property/Variant AddRef count and Release count are
zero; it makes no direct refcount field store. If lookup/GetVariant/setter are stipulated
ref-neutral, P stays 17 and Q stays 29. If raw holder install is stipulated to retain N once,
N ends at C+1 (14 for a fixture C=13); if stipulated ref-neutral it stays C. Those are
conditional fixture calculations, not an answer about the holder or constructor defaults.
O and the input descriptor/buffer are untouched by helper stores (foreign effects remain
as explicitly stipulated). All input addresses below are symbolic independent buffers,
not claims about mapped addresses in the game.

### 4.2 Scalar, pointer and branch cases

| Case | Input and independent primitive facts | Expected trace / result at helper boundary |
|---|---|---|
| S1 bool false / existing | IA; word `0x00000000`; lookup P, GetVariant VP, old VP a u32 array | E(P,IA,11); bool primitive receives low byte 0. Final AL=0, upper bits not a Boolean result. No install; ref sentinel 17 under ref-neutral primitives. A zero AL is not a failed lookup/update. |
| S2 bool true / missing | IB; word `0x00000001`; lookup 0, free-head N; install retains N once, leaves AL=1 | M with ID IB, slot 11. Install observes the constructed property and ID **before** the requested bool value call. AL=1 after the original scalar primitive; N count C+1 only from the stipulated install. O stays its sentinel. |
| S3 u32 / inherited | IA; `0xFEDCBA98`; lookup Q rather than a local P, old VQ f32 | E(Q,IA,26); original primitive receives and leaves EAX=`0xFEDCBA98`. Q is changed by that primitive; no copy, install or local override. Ref sentinel 29 under neutral primitives. |
| S4 u32 zero / missing, install false bits | IB; 0; lookup 0; free-head N; slot 15 normally returns with AL=0, ref-neutral and no holder retention | M(IB,26) continues on N and sets its Variant; EAX=0 from scalar u32 input. No rollback, relookup, destruction or Release. N is still selected even though H did not retain it. This does not assert that the built-in holder actually rejects this ordinary input. |
| S5 f32 / existing | IA; word `0x80000000` (−0); lookup P; old VP bool | E(P,IA,38); helper forwards `0x80000000`; original scalar adapter preserves this normal −0 example; final EAX=`0x80000000`, no x87 helper result. No status test. |
| S6 f32 / allocator route | IB; word `0x3FC00000` (1.5); lookup 0, free-head 0; allocator returns valid N | A(IB,38); one allocator call before construction/install; value call last, EAX=`0x3FC00000` with the normal original adapter. No helper-added ref or cleanup. |
| S7 u8 / existing | IB; low byte `0xA5` in a stack word with distinct upper bits `0x7B6C00A5` | E(P,IB,14); helper forwards the **whole word**, original primitive consumes its low byte; final AL=`0xA5`, upper EAX unspecified. This is not conversion to bool or u32. |
| S8 s32 / missing | IA; word `0x81234567` (−2,128,394,905) | M(IA,29); no helper extension/clamp; EAX=`0x81234567` after the original primitive. Distinguishes the typed slot from u32 despite identical bits. |
| S9 s64 / existing | IB; low=`0x89ABCDEF`, high=`0xFEDCBA98` | E(P,IB,35); callee stack pair is low then high, no truncation. EAX=`0xFEDCBA98` after original adapter/primitive; EDX is not a specified result half. Caller removes 16 helper-argument bytes. |
| S10 string / missing | IA; valid independent string interface T, distinct from N and VN | M(IA,56); exactly T passed without pointer adjustment. Foreign string primitive supplies its own effects/residual EAX; helper neither interprets them nor releases T. No invented string-copy/storage contract. |
| S11 pointer versus low-byte status | nonzero P chosen with low byte zero; property slot 5 returns valid VP also with low byte zero | E(P,IA,26) still occurs. Full lookup pointer is tested, never AL; GetVariant has no result test at all. There is no `(bool, outProperty)` output sentinel to prefill. |
| S12 raw false bits after value call | any existing E case; a valid foreign Variant setter returns normally leaving EAX=`0x13572400` | Helper returns that full residual EAX; it does not turn AL=0 into failure or make a cleanup call. This is register passthrough for a void-like call, not a claimed Boolean setter result. Repeat with `0x24681301` to distinguish AL and full-EAX interpretations. |

S4 deliberately separates raw install return bits from independently supplied retention effects:
AL=0 does not prove the holder did nothing. With the same return bits and a stipulated retain,
the helper's sequence is unchanged and N's fixture count is C+1. Conversely AL=1 with a
ref-neutral raw install still gives the same sequence and C. No fabricated holder implementation
or null-property/true-status combination is necessary: lookup returns only one pointer.

### 4.3 Arrays: literal counts 0, 1 and 3

Let D hold `(begin=B, end=B+extent, ignoredCapacity)`. Each row is one typed primitive
call `(B,count)` **after** E or M's lookup/install/GetVariant prefix. For a nonempty
independent buffer, the raw copying primitive copies the stated byte extent in element
order; the helper performs no copy itself. Counts/extents below are literal arithmetic,
not results of executing either the original or a reimplementation.

| Entry / slot | Element words or bytes | Empty | One | Three |
|---|---|---|---|---|
| `E610` / 94 u8 | `A5 00 7F` | end=B, difference 0, count 0, byte extent 0 | end=B+1, count 1, copy 1 byte `A5` | end=B+3, count 3, copy 3 bytes `A5 00 7F` |
| `E6B0` / 102 u32 | `01234567`, `89ABCDEF`, `FEDCBA98` | 0 → 0, extent 0 | difference 4 >> 2 = 1, copy 4 bytes | difference 12 >> 2 = 3, copy 12 bytes |
| `E750` / 110 f32 | raw `80000000`, `3FC00000`, `C0200000` (−0, 1.5, −2.5) | 0 → 0, extent 0 | 4 >> 2 = 1, copy 4 bytes | 12 >> 2 = 3, copy 12 bytes, no per-element scalar x87 adapter |
| `E7F0` / 108 s64 | low/high pairs `(89ABCDEF,FEDCBA98)`, `(01234567,00000000)`, `(76543210,FFFFFFFF)` | 0 → 0, extent 0 | 8 >> 3 = 1, copy 8 bytes | 24 >> 3 = 3, copy 24 bytes in the given pair order |

For each of these twelve cases propose E(P,IA,S) with a mismatched old scalar and
M(IB,S) with a fresh property: 24 expanded call traces, using the independent primitive
facts of §4.1. This exposes replacement versus creation without using a primitive that
computes an entire helper outcome. One-element u32 still selects slot 102 and array tag
`0x8006`, never scalar slot 26/type 6. Normal bool S1/S2 and high-bit u32 S3 remain distinct.
Final return is untyped residual EAX for every vector case, not count, copied bytes or
success. Refs/O are as in §4.1, with no helper cleanup regardless of that residual word.

Two additional empty-vector distinguishers per array entry are proposed: `(B,B)` with a
valid independent nonzero B, and `(0,0)`. Both dispatch count zero once, with the exact
different begin pointer. The helper does not dereference B; it does read D+0 then D+4.
No assertion about allocator-zero identity, copy of inaccessible memory, or native empty-array
return bits is attached to those cases. Descriptor read order follows the GetVariant call;
this is an ordering observation, not support for reentrant mutation of D.

### 4.4 Failure prefixes and non-guarantees

| Primitive event | Proven prefix / continuation | Boundary of the claim |
|---|---|---|
| lookup returns 0 normally | enters M/A; no caller output exists | This is absence, not a separate false-status/out-pointer pair. |
| install returns normally with any EAX/AL bits | still calls GetVariant on N, then typed setter | Normal-call trace only; actual holder retention/replacement effects are separate inputs. |
| setter returns normally with zero/nonzero residual bits | returns without compensation; no additional property identity is acquired | Does not infer primitive failure from void-like residual bits. Effects already supplied by primitives remain. |
| allocator returns 0 | next block-link read would address zero; no helper guard or normal recovery path | Actual fault/allocation-exception behavior Unsupported; no completion trace or safe Boolean failure invented. |
| property slot 5 returns 0 | next Variant vtable load would address zero | Unsupported; do not invent a null-output success/failure branch. |
| constructor/install/setter does not return | only the already executed stores/calls are established | Pool head/count stores precede constructor; install precedes value. No helper catch/rollback appears. No promise about unwinding or foreign exception cleanup. |

Reentrant holder callbacks, aliasing sources, malformed vectors, count overflow, allocation
failure, and concurrent pool mutation are not normal-domain trace cases. No atomic
transaction, rollback, constructor-default refcount, extra AddRef, or lifetime guarantee for
an unretained new object can be inferred from these prefixes.

## 5. Qualification and limits

The denominator is **eleven named helper entries**, not all PropertyHelp methods: seven
scalar/string dispatches and four vector dispatches. All eleven are distinct contiguous
non-thunk bodies in the matching export and PE, total 1,585 bytes. Of that total, 1,265
bytes are eleven copies of the 115-byte acquisition prefix; 320 bytes are their remaining
dispatch/epilogue suffixes. This is repeated inline logic, not a shared external thunk and
not 1,585 bytes of newly independent algorithm coverage. The Variant adapters, including
five-byte scalar jumps, are separately owned and excluded from that denominator.

Supported means helper-owned normal returned-call contract; Partial means the whole
entry's wider domains are not completely specified. Foreign and Unknown below are explicit
dependencies/domains, not extra helpers counted as solved.

| Function | Supported helper-owned contract | Partial full-entry boundary | Foreign | Unknown / unsupported |
|---|---|---|---|---|
| `0x005FE210` | common E/M/A branches, bool word dispatch, AL residue | whole-entry lifetime/failure domains | holder, pool, constructor, bool setter | allocation/null/reentrant/exception cases |
| `0x005FE2A0` | common branches, u8 word dispatch, AL residue | same | same, u8 setter | same |
| `0x005FE330` | common branches, s32 word dispatch, EAX residue | same | same, s32 setter | same |
| `0x005FE3C0` | common branches, u32 word dispatch, EAX residue; retains PR9/RG16 | same | same, u32 setter | same |
| `0x005FE450` | common branches, low/high forwarding, high-word EAX residue | same | same, s64 adapter/setter | same |
| `0x005FE4F0` | common branches, raw f32 forwarding, adapter boundary; retains RG16 | same plus exceptional x87 domain | same, scalar f32 adapter/setter | exceptional float inputs in addition to common limits |
| `0x005FE580` | common branches, unchanged string-interface pointer | string storage/lifetime outside scope | same, string adapter/setter | null/overlap/lifetime in addition to common limits |
| `0x005FE610` | common branches, byte difference and slot-94 input | vector storage/allocation outside scope | same, u8 copying primitive | invalid/overlapping descriptor/buffer plus common limits |
| `0x005FE6B0` | common branches, shift-2 count, slot-102 input; retains PR9 | same | same, u32 copying primitive | same |
| `0x005FE750` | common branches, shift-2 count, slot-110 input | same | same, f32 copying primitive | same |
| `0x005FE7F0` | common branches, shift-3 count, slot-108 input | same | same, s64 copying primitive | same |

No whole-class, all-method, default-profile, native-test or blanket 1,585-byte algorithm
credit follows from this bounded contract. The old converter/Variant research and native
profiles/caps are unchanged. No new ABI row or C ABI export is proposed. The unsupported
domains above are explicit limits, not an automatic next research round or claimed answers
to existing questions. No question entry is moved or closed by this document.
