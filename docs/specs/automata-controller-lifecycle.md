# Automata controller: inherited base lifecycle

## 1. Scope and evidence

This is a bounded Windows 1.1.641 contract for behavior owned by
`cSC4AutomataController`, supplementing [automata.md](automata.md) §3.5 and
[the controller-manager contract](automata-controller-manager.md). It is **partial**,
not a complete automata owner or an admission for a native wrapper. Generator,
attractor, Automaton and manager algorithms remain separate. Addresses below cite
the matching Windows export and instruction bytes; decompilation was navigation,
not an authority for numeric types or signatures. Proposed traces are unexecuted.

All ordinary call paths below assume valid storage, normal returned calls and no
reentrant mutation of controller fields or list links. No invalid-pointer guard,
exception recovery, allocation-failure unwind or foreign retention policy is
implied. A primitive whose return is ignored can return a failure value and the
controller still performs its next stated operation. Nonreturning calls are a
different domain.

## 2. Pointer views and state

Let **C** be the primary controller interface/object pointer. The serializable
interface is **S = C + 4**: constructor `00517D4E–00517D54` installs the two
vtables; QI `00514F80–00514F87` returns this adjusted pointer; secondary QI,
AddRef and Release thunks `00517F30`, `005183B0`, `005183C0` subtract four.
Write `00517FDC` uses its incoming S directly. Its saved position address S+0x10
(`00518060`) is C+0x14, matching GetPosition `00515A5E–00515A70`.
Saved flags S+0x3C (`0051808E`) are C+0x40; saved list head S+0x40
(`005180A4`) is C+0x44. These are two pointer views, not overlapping live fields.

| C offset | Width / role | Constructor writes (`00517D40–00517DE4`) |
|---|---|---|
| +0 / +4 | primary / serializable vtable pointer | `00A95D90` / `00A95D74` |
| +8 | initialized byte | 0 |
| +0x0C | retained occupant pointer | null |
| +0x10 | borrowed template pointer | null |
| +0x14, +0x18, +0x1C | position, three binary32 words | copies global `00B227CC`; value qualification below |
| +0x20 | unsigned 32-bit reference count | 0 |
| +0x24…+0x27 | four grid-bound bytes | each 0 |
| +0x28…+0x3C | embedded string/storage state | vtable `00A80810`; eight-byte allocation, empty terminator, begin=end and capacity=begin+8; +0x38/+0x3C zero |
| +0x40 | unsigned 32-bit flags | 2 |
| +0x44 | controlled-item list sentinel pointer | twelve-byte allocation; next and previous point to sentinel |
| +0x48 | unsigned 32-bit controlled-item count | 0 |
| +0x4C | retained clock/service pointer | null |
| +0x50 / +0x54 / +0x58 | binary32 elapsed lifetime / lifetime / activation delay elapsed | all positive-zero bits |

No constructor write establishes padding bytes +9…+0x0B or the sentinel payload.
The constructor is an internal ECX-receiver body returning C in EAX; this does
not establish a public constructor ABI or a callable standalone base factory.

## 3. References and interface outputs

Base QI (`00514F60–00514FB4`) recognizes IDs 1 and `6A1857F8`, storing C;
ID `E4FDA3D4` stores S. Each hit stores the output **before** calling virtual
AddRef and returns true in AL. A miss returns false in AL, leaves the output
untouched and does not acquire a reference. Both arguments are four-byte stack
slots, popped by `ret 8`. Null output storage is unchecked. The apparent
null-receiver arm for the adjusted hit still dereferences the receiver for
AddRef: it is not a supported null-object guard.

AddRef (`008AE800–008AE807`, shared body despite its export name) increments
C+0x20 modulo 2^32 and returns the stored count in EAX. Release
(`00517F10–00517F27`) unsigned-compares the count with 1: above 1 it decrements,
stores and returns the count; at 0 **or** 1 it calls virtual slot 39 (+0x9C)
without first decrementing/storing the count, then returns zero in EAX.
No atomic operation or underflow guard is present. The slot-39 body at
`00519340` is mislabeled Write in the export: it calls virtual Shutdown, then
the base destructor, then heap free. Thus final Release cleanup is different
from direct entry to the base destructor.

## 4. Template and occupant assignment

SetTemplate (`00517430–0051745D`, slot 15) first stores the raw pointer at
C+0x10. A nonnull template clears the embedded string only if nonempty, copies
template+0x4C to lifetime C+0x54 and template+0x58 to flags C+0x40. A null
template leaves string, lifetime and flags unchanged. Same-pointer assignment
still repeats the nonnull effects. It neither AddRefs the new template nor
Releases the old one, and does not reset either elapsed field or activate.
It pops one pointer argument and defines no boolean result.

SetOccupant (`00517F40–00517F6E`, slot 14) compares pointer identity. For a
different value: AddRef new if nonnull; store new at C+0x0C; Release old if
nonnull. Same-pointer assignment performs none of these. Returned AddRef/Release
values are ignored. Null detaches by storing null before releasing old. No
position copy, grid update or controlled-item removal is part of this body.
It pops one pointer argument and defines no boolean result.

## 5. Shutdown and destruction

Shutdown (`005169F0–00516A90`, slot 4) always calls virtual Activate(false)
first. Only then it tests initialized byte C+8. If clear, it returns true in AL
without clearing template, occupant, clock or list. If set, it clears that byte
**before** walking the controlled-item list from sentinel.next in link order.
For each node it saves payload A=node+8, calls virtual CheckFlag(0x100), and if
true calls A slot 13 (+0x34) with null; it then calls A Release. Both returned
values are ignored. The next link is read after these calls. This specifies
controller-owned ordering, not the transitive automaton detachment policy.

After all payload calls, a separate pass frees each node, restores sentinel
self-links and stores count zero. Then it saves occupant, stores null before
its Release; saves clock, stores null before its Release; and finally stores
template null. It returns true in AL on every normal returned path. No template
Release occurs. Citations: payload pass `00516A13–00516A3C`; node pass and count
`00516A3F–00516A67`; references/template `00516A6A–00516A89`.

A second Shutdown still calls Activate(false), but the initialized-byte test
skips the cleanup pass. Under inherited base Activate and unchanged state this
does no further cleanup. A reference attached after Shutdown can therefore
survive repeated Shutdown until destruction or reassignment. This is conditional
idempotence, not a general guarantee for overridden/reentrant calls.

The direct base destructor (`00517F80–00517FC8`) reinstalls base vtables,
Releases a nonnull clock, destroys list **storage**, destroys string storage,
then Releases nonnull occupant. The list helper `005650A0–005650DD` frees nodes
and sentinel without calling payload Release or detach. The base destructor
does not call Shutdown, clear initialized, release template or release each
controlled payload. It does not null the clock/occupant fields before calling
their Releases. Do not substitute a modern automatic ownership cleanup model.

## 6. Remaining bounded work

Init, activation, tick/expiry, bounds and controlled-item removal are being
reconciled against instruction bytes. Until those sections and the per-body
ledger below are finalized, AT22 remains open. Foreign template, occupant,
automaton, clock, geometry and registration behavior is never a primitive that
computes the entire controller outcome.
