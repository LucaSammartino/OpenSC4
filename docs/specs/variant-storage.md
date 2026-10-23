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
interface at owner+4. No standalone-constructor route follows from the SDK name.

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
(`0x009166F0`–`0x00916711`). No post-release alias is valid.

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

A null input takes the replacement branch even for matching type/count and skips the byte
copy. The owner does not initialize the newly allocated bytes. Allocation size zero is
passed through, with no special Empty normalization; the resulting pointer and successful
allocation behavior are primitive-dependent. Null and zero length are not evidence for an
arbitrary null-buffer getter or an allocation-failure success guarantee.

The no-argument `RefX()` loads the first payload word (`0x0060CE80`). The mutable
`AsX()` entries return the address of inline payload (`0x0060FB50`). These are different
alias channels: on an array, RefX points to elements, while mutable AsX aliases the storage
holding the data pointer itself. Neither accessor checks the type, retains a reference,
allocates or converts. Use element aliases only for the stored matching array type and
inline aliases only for the stored matching scalar type. Writing through a valid matching
alias changes the variant; modifying the external input after a copying setter does not.

A reuse branch preserves the array allocation and its element aliases. A replacement
branch frees the old allocation before obtaining new storage; prior element aliases then
expire even if an allocator later returns the same address. An inline alias remains an
address inside a living variant but may refer to a different representation after a type
change. It must not be used as an old typed value after replacement.

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

## 6. Publication boundary

This publication establishes the native identities, destruction and bounded array/alias
rules above. Detailed CopyFrom/self-copy dispatch, the remaining typed family widths,
and bounded string/object operations require the additional owner paths, not the SDK
inventory alone. No whole-owner equivalence, byte denominator, default-enable decision,
native verification promotion or fourth verifier round follows from this document.
