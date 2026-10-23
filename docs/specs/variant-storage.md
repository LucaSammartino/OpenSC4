# Native variant storage and lifetime

## 1. Scope and evidence

This is a bounded original-program contract for `cRZVariant` in Windows SimCity 4 Deluxe
1.1.641. It specifies storage, replacement, aliases and owned copies. It does not qualify a
replacement implementation or claim completion of the 127 interface slots. Converting
getters, parsers and character-to-text rules remain in [data-formats §5.1](data-formats.md#51-converting-getters);
the type inventory remains in [foundations §9.1](foundations.md#91-in-memory-types-compat).
Serialization and acquisition through native segment streams are outside this contract.

Evidence is static: the original export was queried first, then existing decompilation was
used to locate functions and instruction bytes to decide branches, widths and order. The
working original PE and export summary have the same SHA-256,
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, image base
`0x00400000`. No live REA session, native calls, game run or implementation output was used.
Addresses below identify evidence, not an acquisition API for a verifier.

Rules describe valid native objects and readable input buffers, with successful allocation
where allocation is necessary. They do not promise recovery from invalid pointers, allocation
failure, reference-count overflow or reentrant callbacks. A primitive's conditional result
does not establish an owner operation's success: the owner's branch and call order matter.

## 2. Native identities and attribution

The published `cIGZVariant` table at `0x00AA6788` is an embedded interface, not the table
installed by the standalone variant constructor. Its slots 0/1/2 (`0x005FD6F0`,
`0x005FD700`, `0x005FD710`) subtract four bytes from the interface pointer and dispatch
to the containing owner's identity/reference operations. The QueryInterface owner path at
`0x005FD030` handles its own identities and forwards other requests to the embedded variant
interface at owner+4. The original `cSCProperty` constructor explicitly constructs the
variant at this+4, then replaces its table with `0x00AA6788`
(`0x005FDE54`–`0x005FDE68`). This establishes the containing class and embedded layout
without importing property-holder algorithms. No standalone-constructor route follows
from the SDK name.

The default and copy constructors at `0x009166D4` and `0x00916712` instead install
`0x00ACECD0`. Original PE words establish that its first 127 entries agree with
`0x00AA6788` at slots 3–126; only slots 0/1/2 differ. Those three standalone entries are
`0x00914E56`, `0x00914E78`, `0x00914E81`. Some export labels misleadingly attribute these
and the table to `cSCProperty`; the constructor's literal table store, slot bodies and
CreateCopy call chain establish their role here. This is a distinct native implementation
of the same published variant interface, with its own reference state.

Each 127-entry table contains 85 distinct entry addresses. This is ABI inventory, neither
a unique-body denominator nor behavior/byte coverage. Attribution must distinguish:

| Evidence class | Examples and consequence |
|---|---|
| Owner decisions | Destroy `0x00914EA3`, RefUint32 `0x00915014`, CreateCopy `0x00916768`: their branches decide storage, order and publication |
| Shared/folded accessors | `0x0060FB50` returns the address of inline storage; `0x0060CE80` loads its first pointer-sized word; the latter also bears a foreign Lua-thread name. Its use here follows the table and instructions |
| Tail wrappers | Erase calls Destroy; object setter slots 61/124 share `0x0091663F`, a jump to `0x00915A5F`; slot 60/125 share `0x00916644`, a jump to `0x00915A46` |
| Foreign primitives | GlobalHeapAlloc `0x005E55E0`, GlobalHeapFree `0x005E5620`, byte copy `0x009F19B0`, string and held-object virtual calls: credit only their bounded input/output effects, not the whole owner operation |

Standalone QueryInterface accepts IDs `1` and `0x60FD4D2B`, writes the same interface
pointer and calls that object's AddRef; unsupported IDs return false without writing the
output (`0x00914E56`–`0x00914E77`). AddRef increments an unsigned **16-bit** count and
returns its zero-extended value (`0x00914E78`). Release decrements counts above one; for
counts zero or one it dispatches the deleting destructor with argument 1 and returns zero
(`0x00914E81`). These rules do not transfer to the containing owner's reference count.

## 3. Representation, construction and Erase

The standalone constructor requests no other interface. At `0x009166D4` it installs its
table and clears both 32-bit payload words, the 16-bit reference count, the 16-bit type
and the 32-bit element count. The object occupies 20 bytes on the CreateCopy allocation
path. A native caller also constructs a 20-byte stack temporary directly and destroys it
without acquiring a reference (`0x00473CE4`, `0x00473D17`–`0x00473D24`). This proves a
native construction route, not a public factory.

| State | Stored payload and disposal |
|---|---|
| Empty | Cleared payload, type 0, count 0 |
| Scalar numeric/character | Inline value in the eight-byte payload; scalar count is 0 (already published HG7) |
| Array, including RZChar string and raw void bytes | First payload word is allocated data; count is elements, not bytes; signed type bit selects array disposal |
| Scalar VoidPtr | A pointer value; no pointee disposal |
| Scalar IGZUnknown (`0x10`) | A retained object pointer, or null; non-null is released on disposal |

Destroy first tests bit 15 of the 16-bit type. For arrays it passes the data pointer to
GlobalHeapFree, then clears count. Otherwise, exactly type `0x10` with a non-null pointer
calls held-object slot 2 (Release). After disposal it clears the two payload words, type
and count, leaving the variant's own reference count and vtable intact
(`0x00914EA6`–`0x00914EDE`). GlobalHeapFree itself ignores null (`0x005E5625`–`0x005E5627`).
There is no element-by-element object release for an array tag.

Erase calls Destroy and then explicitly returns true in **AL** (`0x00915592`–`0x00915599`).
The upper EAX bytes are not a contractual boolean. The deleting destructor reinstalls the
standalone table, calls Destroy, then frees the object itself if its flag's low bit is set
(`0x009166F0`–`0x00916711`). Aliases into the variant expire on its final Release/deletion.

## 4. Copying array setters and mutable aliases

The input-taking `RefX(pointer, count)` is a copying storage operation, despite its name.
For RefUint32 (`0x00915014`–`0x00915085`) the exact order is:

1. Reuse existing storage only if type is `0x8006`, counts agree, and both stored data and
   input pointers are non-null. Copy `count << 2` bytes into that allocation.
2. Otherwise Destroy the old value first, request `count << 2` bytes, and store the returned
   pointer. If the input pointer is non-null, copy those bytes into the new allocation.
3. Store the count, then the new array type. No allocation-result test precedes the copy.

RefBool (`0x00914EE4`–`0x00914F44`) and RefRZChar (`0x0091545E`–`0x009154BE`) use
one byte per element; RefUint64 (`0x00915086`–`0x009150F7`) uses `count << 3`. These
shifts and the count metadata are 32-bit operations. Counts whose byte size overflows are
outside the qualified valid-buffer domain. No terminator is appended by RefRZChar.

The same reuse/replacement decisions and call order hold for these input-taking slots;
the table gives each independently inspected owner entry and byte multiplier:

| Element | Bytes | Owner entry | Element | Bytes | Owner entry |
|---|---|---|---|---|---|
| Bool | 1 | `0x00914EE4` | Uint8 | 1 | `0x00914F45` |
| Sint8 | 1 | `0x009150F8` | Uint16 | 2 | `0x00914FA6` |
| Sint16 | 2 | `0x00915159` | Uint32 | 4 | `0x00915014` |
| Sint32 | 4 | `0x009151C7` | Uint64 | 8 | `0x00915086` |
| Sint64 | 8 | `0x00915239` | Float32 | 4 | `0x009152AB` |
| Float64 | 8 | `0x0091531D` | Char | 1 | `0x0091538F` |
| RZUnicodeChar | 2 | `0x009153F0` | RZChar | 1 | `0x0091545E` |
| VoidPtr | 4 | `0x009154BF` | Void (raw bytes) | 1 | `0x00915531` |

The two-byte families double the 32-bit count with address arithmetic (e.g.
`0x00914FCA`, `0x00914FE4`); four/eight-byte families shift by two/three. These are byte
copies, with no per-element normalization, reference acquisition or pointee cloning.

A null input takes the replacement branch even for matching type/count and skips the byte
copy. The owner does not initialize the newly allocated bytes. Allocation size zero is
passed through, with no special Empty normalization; the resulting pointer and successful
allocation behavior are primitive-dependent. Null and zero length are not evidence for an
arbitrary null-buffer getter or an allocation-failure success guarantee.

The no-argument `RefX()` loads the first payload word (`0x0060CE80`). The mutable numeric
and character `AsX()` entries (slots 63,65,…,89) return the address of inline payload
(`0x0060FB50`). **Exception:** both AsVoidPtr slots 90/91 load the first payload word;
slot 91 does not return the address of the pointer field. These are different
alias channels: on an array, RefX points to elements, while mutable AsX aliases the storage
holding the data pointer itself. Neither accessor checks the type, retains a reference,
allocates or converts. Use element aliases only for the stored matching array type and
inline aliases only for the stored matching scalar type. Writing through a valid matching
alias changes the variant; modifying the external input after a copying setter does not.

The const scalar accessors load one byte (`0x0040CE50`), two bytes (`0x005FD610`), four
bytes (`0x0060CE80`), two words into EDX:EAX (`0x005FD620`), or an x87 Float32/Float64
value (`0x005FD630`/`0x005FD640`). Returned mutable numeric/character pointers all point
to the same inline start, but the matching type determines the writable width. This is
not permission to overwrite the full eight-byte union through a narrower typed alias.

A reuse branch preserves the array allocation and its element aliases. A replacement
branch frees the old allocation before obtaining new storage; prior element aliases then
expire even if an allocator later returns the same address. An inline alias remains an
address inside a living variant but may refer to a different representation after a type
change. It must not be used as an old typed value after replacement.

Exact self-alias input on the reuse branch copies the allocation onto itself and preserves
its contents. The byte-copy primitive has an overlap-direction test and a backwards-copy
path (`0x009F19C2`–`0x009F19CA`, `0x009F1B4C`–`0x009F1B6A`), rather than relying on a
generic C memcpy assumption. This only helps when the entire input and output spans are
live, valid and non-wrapping. The replacement branch takes no snapshot: passing an old
element alias while changing type/count can free the input before copying it. Such a
dangling-input call has no qualified value result and is not a proposed experiment.

Scalar setters similarly destroy a non-Empty old value when its type differs, then set
the scalar type and write the matching inline width. A same-type write does not dispose
or clear the other payload bytes. Empty skips disposal. Representative decisions are
`0x009157B4`–`0x009157D2`, `0x00915880`–`0x0091589E`; the corresponding value stores are
one byte for Bool/Uint8/Sint8/Char/RZChar, two for Uint16/Sint16/RZUnicodeChar, four for
Uint32/Sint32/Float32/VoidPtr, and eight for Uint64/Sint64/Float64. The integer eight-byte
setters store two dwords (`0x009158EE`/`0x009158F5`, `0x0091591D`/`0x00915924`);
Float32 stores its supplied four bytes (`0x0091594C`), while Float64 loads/stores an x87
double (`0x00915970`–`0x00915974`). This does not establish a new non-finite conversion rule.

## 5. CreateCopy and return channels

CreateCopy allocates 20 bytes, invokes the native copy constructor `0x00916712`, calls
slot 1 (AddRef) on the resulting standalone variant, writes it to the caller's output,
then explicitly sets AL to 1 (`0x00916768`–`0x00916795`). Construction starts the variant
reference count at zero, so successful publication hands the caller **one owned reference**.
The source's own reference count is not incremented by this publication step. The output
is a distinct variant object; clone identity is established by the constructor and table
evidence in §2, not by assuming the borrowed source's vtable.

Allocation failure is not a clean false/null return: the zero-allocation-result path sets
the local object pointer to zero and rejoins an unconditional vtable dereference before
the output store (`0x00916783`–`0x00916790`). Valid non-null output storage is required;
there is no output-pointer check. The normal-return true result therefore says nothing
about allocation failure recovery.

CopyFrom calls its interface-source assignment body at `0x009161DB` and then explicitly
sets AL to 1 (`0x00916649`–`0x00916654`). Judge state and low-byte result independently;
do not treat full EAX or a guessed C++ signature as an additional channel.

### 5.1 Source dispatch, self-copy and isolation

CopyFrom first calls the source's GetType. For supported scalar types 1–15 it calls the
matching const AsX value slot and passes that value to the destination scalar setter.
For type `0x10` it calls the raw RefIGZUnknown getter, then the retaining object setter.
For arrays `0x8000`–`0x800F`, it obtains source count **before** source raw data pointer,
then calls the destination matching RefX copying setter. Empty and unsupported tags
dispose the destination. Evidence: dispatch `0x009161E1`–`0x00916207`, scalar calls
`0x0091620E`–`0x00916382`, void-array path `0x00916387`–`0x0091639E`, typed arrays
`0x009163BE`–`0x009165A8`, default disposal `0x009165B0`–`0x009165B2`.

The clone constructor uses a separate native-layout assignment body (`0x00915EC0`): it
reads the source's 16-bit type and payload/count directly, selecting the same destination
scalar/object/array operations (`0x00915FDF`–`0x00916032`, `0x0091612A`–`0x00916150`).
Thus this CreateCopy path proves copies from the native layouts above, not arbitrary
foreign implementations with a different internal layout. CopyFrom's source dispatch is
through interface methods; effects of foreign implementations remain conditional inputs.

There is no blanket self-pointer guard in either assignment dispatcher. For a normal native
self-CopyFrom, scalar setters rewrite the existing value; an object sees identical pointers
and performs no AddRef/Release; a populated array with non-null storage takes the reuse
branch and copies itself. A zero-count array's pointer identity can depend on the allocator:
if stored data is null it takes the replacement branch again. Do not promise a universal
no-op including allocation traces. Float copying uses the source's x87 value channel and
an explicit Float32/Float64-width spill (`0x009162E3`–`0x0091630A`), not a whole-object
bitwise copy or a promise about signaling-NaN payload preservation.

A distinct destination owns its own copied array allocation, including string bytes, and
therefore permits independent element mutation. Scalars are copied by value. VoidPtr
values and pointer-array elements copy pointer bits, not the pointed-to allocation. Object
copies share the same held object with an additional retained reference; they are not
deep object clones. Neither CopyFrom nor CreateCopy transfers the source variant's own
reference count. Destruction of either variant must leave a distinct copied array alive;
shared objects follow the retaining setter and each variant's later Release (§7).

## 6. Bounded string operations

SetValString's slot wrapper `0x009166CF` jumps to `0x00915A1D`. The owner reads the
source string length through slot 7, then its byte pointer through slot 6, then calls its
own RefRZChar input setter (slot 118). This copies exactly that many bytes into type
`0x800E`, without retaining the input string interface and without a NUL scan or appended
terminator. An input string with length 3 and bytes `41 00 42` therefore stores all three
bytes; later mutation/destruction of the source string does not affect the variant. The
direct native string constructor uses the same length/pointer/RefRZChar route after
initializing Empty (`0x00915CDE`–`0x00915D1C`). A valid string interface is required;
there is no null-interface check at these call sites.

GetValString (slot 55, thunk `0x00917ADB`) has a direct stored-string branch: it passes
the data pointer and count to output-string slot 3, then returns true in AL, regardless
of the output primitive's return register (`0x009172AE`–`0x009172C4`). It does not lend
the variant's allocation to the output. For other stored types, the already published
conversion behavior in data-formats §5.1 applies; no new parser/text algorithm is specified
here. In particular, false from that conversion is not general output preservation.

Conditional primitive facts for the **original cRZString** used by CreateValString:

- Its constructor installs `0x00A80810`, starts its 32-bit reference count at zero and
  creates an empty, NUL-terminated buffer with eight bytes of capacity
  (`0x00578390`–`0x005783C4`). Its Strlen is end-minus-start (`0x0090E4B6`); ToChar
  returns start (`0x009D7E63`). Length is not inferred by scanning for NUL.
- Its length-bearing FromChar slot 3 (`0x0090F546`) assigns the range `[p,p+length)`
  for non-null input; null input clears the string. The existing-length branch copies
  the range and adjusts end (`0x0044D3A0`–`0x0044D3FC`). For a newly constructed empty
  output and lengths 1 or 3, the in-capacity extension branch copies the supplied bytes,
  writes the trailing NUL and advances end (`0x0044D417`–`0x0044D429`,
  `0x0040EC98`–`0x0040ECC8`). Embedded NULs do not terminate this operation. Non-null
  length 0 assigns an empty range; null data clears even if a count was supplied.
- These finite output cases establish independent original string storage for lengths
  0/1/3. This is not a full RZString allocation, alias-overlap or failure contract for
  every length or foreign output implementation.

CreateValString allocates a new 20-byte cRZString, constructs it, calls AddRef once and
then calls the variant's string getter helper. If that helper succeeds, it stores the
string pointer and returns AL=true. Otherwise it releases the temporary string, overwrites
the caller's output with null and returns AL=false (`0x00917A8C`–`0x00917AD8`). The
original string's AddRef is a 32-bit increment (`0x0094D34F`), and Release at count one
invokes its deleting destructor (`0x0090E1F3`). A successful result therefore carries one
owned string reference. An existing caller-owned pointer in the output is not released
by either result path: this is output storage, not an in/out ownership-transfer operation.

The string allocation-null branch, like CreateCopy, rejoins an unconditional vtable load
(`0x00917AA7`–`0x00917AAD`) before publishing an output. No clean allocation-failure
false/null behavior is established. The false/null result above is specifically the
**post-construction getter-false** path. Variant storage and its own reference count do
not change when extracting a separate string.

## 7. Objects and pointers

SetValUnknown (slot 61) and RefIGZUnknown(input) (slot 124) share the jump at
`0x0091663F` to `0x00915A5F`. Their actual owner order is:

1. If the old type is not `0x10`, dispose it unless Empty, then set type `0x10`.
2. Compare input pointer with the current held pointer. Equality causes no reference calls.
3. Otherwise AddRef the non-null new pointer first; Release the non-null old pointer next;
   store the new pointer last (`0x00915A81`–`0x00915A9D`).

Thus object A→B retention occurs while A is still stored; its Release callback occurs
before the slot is updated to B. Array→object frees the array **before** retaining B,
because the type-change disposal precedes this comparison. Object→null releases the old
object and leaves a null pointer with type `0x10`, count 0; null→null is silent. The owner
calls slots 1/2 and ignores their returned counts; it does not infer a particular object's
internal count policy. Observable deltas of +1/−1 below assume an ordinary functioning
reference-counted object. Reentrant side effects and exceptions are outside the contract.

No-argument GetValUnknown (slot 59, `0x009157A3`) returns the pointer only when type is
exactly `0x10`, otherwise null; it does not AddRef. Raw RefIGZUnknown() (slot 126) simply
loads the first payload word without checking type or retaining it. Neither is an owned
acquisition channel. Their borrowed result expires when the variant releases the object
unless the caller acquired a separate reference.

The `(riid, output)` GetValUnknown/RefIGZUnknown slots 60/125 share the helper
`0x00915A46`. On type mismatch or null object they return false in AL **without writing
the output**. On a non-null object they tail-dispatch its QueryInterface, forwarding the
original arguments and that object's result/output policy. The variant does not add a
second reference on top of QueryInterface. A successful owned reference and failed-query
output preservation on this delegated path depend on the held object's actual contract.

The object branch of GetValString instead queries `cIGZString` ID `0x089B7DC8`, gets
length then data from the acquired interface, assigns the output span, releases that
queried interface and returns true (`0x009172CF`–`0x00917308`). This balances one temporary
successful-query reference conditional on a conforming held object. Unlike GetValUnknown,
this branch dereferences the held object without a null check: the null-object string
getter has no qualified success/failure-output rule. Do not propose calling it as a null
probe. Other object-to-text conversion remains the prior published input.

SetValVoidPtr tail-jumps from `0x009166CA` to `0x009159F5`: after any necessary old-type
disposal it stores the four-byte pointer value and type `0x0F`. Null is just a pointer
value. No AddRef, Release or free of that pointee occurs. RefVoidPtr(input,count) owns
only a copied four-byte-per-element pointer array; its disposal frees the array, not its
elements. AsVoidPtr's exception is stated in §4. This does not add converting VoidPtr
getter rules to the existing conversion spec.

## 8. Independent proposed traces — UNEXECUTED

These are literal static expectations written from the original evidence before any new
implementation output. They are not measured passes, game requests or executable fixture
addresses. Use valid objects/buffers and successful allocations; never read an expired alias.
Allocation identities below are symbolic. Each row is independently initialized unless
it explicitly continues another step. No generic SDK reading is the oracle.

| Trace | Inputs/steps | Expected state, ownership or output |
|---|---|---|
| VS-A input copy | External Uint32 buffer `[17,34,51]`; RefUint32(buffer,3); change external middle element to 99 | Variant type `0x8006`, count 3, elements `[17,34,51]`; no input-buffer retention |
| VS-B lengths | RefUint32 with valid independent buffers of 1 element `[17]` and 3 elements `[17,34,51]` | Allocate/copy 4 and 12 bytes respectively; count 1/3; changing either source afterwards leaves variant unchanged |
| VS-C zero/null | Fresh variant; RefUint32(null,0); separately RefUint32(null,3) | Requests 0/12 bytes, stores count 0/3 and type `0x8006`; skips input copy. Zero-size pointer identity and positive-size contents are unspecified; do not read uninitialized elements |
| VS-D mutable scalar | Set Uint32 17; take mutable AsUint32; write 34 through it | Type `0x0006`, count 0, scalar 34; no allocation or reference calls |
| VS-E mutable array/reuse | RefUint32 `[17,34,51]`; obtain p=RefUint32(); write p[1]=68; set same type/count from independent `[1,2,3]` | Initial write gives `[17,68,51]`; reuse preserves p's allocation; p sees `[1,2,3]` afterwards |
| VS-F exact self alias | Non-null populated Uint32 array `[17,34,51]`; RefUint32(its own element pointer,3), then CopyFrom(self) | Both reuse the allocation and leave `[17,34,51]`; no owner allocation/free; no general promise for null-storage zero arrays |
| VS-G replacement sequence | Scalar Uint32 17 → Uint32 array `[17,34,51]` → SetValString length 1 `51` (`Q`) → object B → Erase | Scalar disposal clears; array allocated; array freed before one string-byte allocation/copy; string allocation freed before B.AddRef; Erase calls B.Release before clearing; final Empty/count0 |
| VS-H clone isolation | Clone populated array `[17,34,51]`; write clone middle=99; Erase source | Different variant and element allocations; clone `[17,99,51]` survives; clone own reference count 1; source own reference count unchanged by cloning; release clone once when finished |
| VS-I distinct CopyFrom | Destination scalar 7; source array `[1,2,3]`; CopyFrom(source); mutate source first=9 | Destination owns `[1,2,3]`; source `[9,2,3]`; no variant reference-count transfer |
| VS-J string bytes | Source original string length3 bytes `41 00 42`; SetValString; mutate source to `58 59 5A`; CreateValString with output initially null | Variant still count3 bytes `41 00 42`; extraction true, owned string reference count 1, length3 bytes `41 00 42` plus its own terminator; changing/extracting/erasing variant does not borrow the source string |
| VS-K string finite lengths | Separate RZChar arrays with count0, byte `41`, bytes `41 00 42`; CreateValString on each | Successful owned original output length0/1/3; embedded zero retained; variant unchanged. Count0 data may be null: original FromChar clears |
| VS-L string getter failure | Empty variant; CreateValString with a separate pointer sentinel in output (sentinel's object is held elsewhere) | Prior published Empty string-getter result is false; temporary is created/AddRef'd then released; output becomes null; sentinel is not released |
| VS-M query failure sentinel | Scalar Uint32 17, then separately type `0x10` with null; GetValUnknown(riid,&sentinel) | Both false; sentinel unchanged; no QueryInterface or ref calls |
| VS-N object transitions | A and B each have one external owned reference; set A, set A again, replace with B, clone, Erase source, release clone | A: 1→2→2→1. B: 1→2→3→2→1. On A→B: B.AddRef, A.Release, store B. Clone shares B, while variants are distinct |
| VS-O object self-copy | Variant holding A; CopyFrom(self) | Same A pointer; no extra AddRef/Release; normal AL=true |
| VS-P pointer lifetime | SetValVoidPtr(P), clone; Erase both; separately RefVoidPtr `[P,null,P]` then Erase | P's pointee receives no reference/free calls; only the copied pointer-array allocation is disposed; clone stores the same P bits |

Alias invalidation in VS-G is an event expectation: the element alias dies when its allocation
is freed; no trace dereferences it afterwards. No partial-overlap input of an unproved extent,
null object string getter, invalid output pointer, count overflow or forced allocation failure
is proposed. On delegated QueryInterface failure, only the conditional held-object rule in
§7 applies; VS-M intentionally exercises the owner-handled failure branches.

## 9. Publication boundary

Observed native rules above cover constructor defaults, replacement/Erase, the finite typed
copying family and aliases, native clone identity/return/ownership, interface CopyFrom and
bounded string/object/pointer effects. Conditional inputs include heap results, output-string
assignment and held-object interface/reference behavior. Unknown domains include allocation
failure recovery, zero-size allocator identity, general RZString growth/overlap/failure, foreign
interface implementations, non-finite copy details and callback reentrancy. These are explicit
limits, not claims of false returns or output preservation.

HG6 is answered by original constructor/table/instruction evidence (§2/§5); HG5 remains
open for its broader domains. Existing conversion/parser facts and answered HG7 remain
inputs. The verifier remains used 3/3 and parked: its historical 28 HG7 differences and
70 cases/420 checks are unchanged, and its measured scope excludes the new string/object/
pointer/alias rules. No whole-owner equivalence, byte denominator, default-enable decision,
native verification promotion or fourth verifier round follows from this document.
