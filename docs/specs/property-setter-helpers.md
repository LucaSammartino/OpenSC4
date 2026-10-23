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
occupant-manager §AddOccupantGroup names a u32-array consumer. These remain valid partial
inputs. An address/family search of public specs and lane reports found no earlier dedicated
complete contract for these eleven entries. Owner-level legacy coverage is not exact-entry
algorithm qualification. EX52f holder slot-14/16 wrappers, including `0x005FE05B`, are different
functions: their Release return channel does not apply here.

Source: the main checkout's `bin/SimCity4.exe`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, and the matching
`ghidra/export/windows/export-summary.txt` (image base `0x00400000`). `re_query.py func`,
`callers` and `asm --bytes` were queried first; decompilation was navigation only and
omits actual value arguments in some prototypes. Addresses below are original Windows VAs.

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
   explicitly initialize another reference-count field.
6. Call holder slot 15 (`+0x3C`) with `(newProperty, 0)` before setting the requested
   value (`0x005FE27A–0x005FE27F`). Ignore its return bits and continue with the same
   new property pointer, without relookup. The zero is an argument, not a tested result.
7. Call property slot 5 (`+0x14`) on the selected property, and use the returned Variant
   pointer directly as the receiver for the typed setter. There is no null check.
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
not interpret unspecified upper bits as a full-width Boolean. Source-level void versus
an intentionally consumed result requires caller evidence; the machine contract does not
synthesize either success or failure.

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
| `0x005FE610` | 149 | vector descriptor pointer; 12; byte count = end − begin | 94 / `+0x178` | `0x005FE68A–0x005FE69B` |
| `0x005FE6B0` | 152 | vector descriptor pointer; 12; count = (end − begin) arithmetic-right-shift 2 | 102 / `+0x198` | `0x005FE72A–0x005FE73E` |
| `0x005FE750` | 152 | vector descriptor pointer; 12; count = (end − begin) arithmetic-right-shift 2 | 110 / `+0x1B8` | `0x005FE7CA–0x005FE7DE` |
| `0x005FE7F0` | 152 | vector descriptor pointer; 12; count = (end − begin) arithmetic-right-shift 3 | 108 / `+0x1B0` | `0x005FE86A–0x005FE87E` |

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

## 4. Proposed trace ledger

Pending completion of independent literal cases and primitive dispatch attribution. These
will be proposed, unexecuted original-instruction expectations, not a native comparison.

## 5. Qualification and limits

Helper-owned branches and dispatch in §2–3 are Supported for normal returned calls with
valid pointers. Constructor, pool allocator, holder implementation and Variant storage
algorithms are Foreign. Unchecked-pointer/allocation failure, reentrancy, exceptional or
nonreturning foreign calls and overlapping vector/string inputs are Unknown/unsupported.
No whole-class, all-method, default-profile, native-test or 1,585-byte algorithm credit follows
from this bounded contract. The inventory and ledger will state the final denominator.
