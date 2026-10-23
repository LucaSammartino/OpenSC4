# Native scoped game pause

## 1. Scope, identity and status

This is the first dedicated contract for `cSC4AutoGamePause` and, separately,
`fn:SC4GamePause`. It covers their own normal-return field, branch, call and
temporary-reference decisions. It does not specify the participant implementations
or a complete application owner. Admission is for both application and other free
functions; the helper remains separately owned even where its typed target is unknown.

Evidence is the Windows x86 GOG 1.1.641.0 PE, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
The working PE hash matches `ghidra/export/windows/export-summary.txt`; its machine
is `0x014C`. Byte-bearing export instructions were consulted first and the unusual
first lookup constants and both complete class bodies were checked against PE bytes.
Names are navigation metadata, not proof of parameter types or service identity.
No native execution or equivalence result is claimed.

| Owner/body | Export extent (inclusive), size | Status and boundary |
|---|---|---|
| Class constructor `0x005F9320` | `005F9320–005F9353`, 52 bytes | Supported normal field/call/return contract, §2 |
| Class destructor `0x005F9360` | `005F9360–005F9380`, 33 bytes | Supported normal read/call contract, §2 |
| Separate `fn:SC4GamePause`, `0x005F91C0` | `005F91C0–005F931E`, 351 bytes | Supported own control flow, §3; Partial typed first-participant identity (AGP1) |
| Shared cleanup `0x004495C0`, `0x005F8F20` | Only null test and slot `+0x08` boundary used | Foreign utility; normal Release behavior verified, no exception contract credited |
| Framework, animation, simulator, app, city, clock, first participant | Individual replies/calls only | Foreign; no participant body credit |
| Allocation, factory/COM identity, copies/moves, exceptional lifetime | Not established | Unknown/Unsupported |

History audit: [application §12](application.md#12-open-not-specified-here) and
the final [re-application4 report](../lanes/re-application4.md) expressly exclude
this class. [Advisors §6.8.1](advisors.md#681-popup-display-and-the-simulation-pause-compat),
[UI windows §6](ui-windows.md), and [modal dialogs §4](modal-dialogs.md#4-modal-dialogs-in-unattended-play)
give selected popup/camera simulator-and-clock projections, not this storage or
four-participant coordinator contract. [Clock24 §9](clock24.md#9-who-reaches-in)
qualifies its clock caller boundary. Current spec-path/name searches, all-ref
pause-path history and scoped content-history searches found no earlier complete
same-class/helper contract. Existing caller projections remain canonical for those
callers. Portable UI options and hidden-pause nesting models are not evidence for
the native class. The capped application/clock/simulator/advice/view/UI/audio and
region families are unchanged.

## 2. Class storage and lifetime

The constructor receives `this` in ECX and four 32-bit stack argument slots in
order F1, F2, F3, F4. Their selected meanings are §3; the familiar caller labels
are sound, animation, simulator, clock, but the first label is unconfirmed here.
No default arguments are established. At least the four addressed bytes must be
writable; the bodies do not establish allocation size, alignment or a vtable.

Before invoking any helper callback, it stores low8(F1), low8(F2), low8(F3),
low8(F4) at `this+0`, `+1`, `+2`, `+3`, in that order
(`0x005F933A–0x005F9342`). These are raw bytes, not normalized Booleans.
It invokes the separate helper with five stack slots `(1,F1,F2,F3,F4)`, retaining
the original full argument words for this entry call (`0x005F9334–0x005F9345`).
After a normal return it returns `this` in full EAX and removes its four caller
argument slots (`0x005F934D–0x005F9351`). It does not test the helper result.

The destructor reads offsets `+3`, `+2`, `+1`, `+0` in that order, zero-extending
each byte, then invokes the helper with `(0,F1,F2,F3,F4)` using those current
stored bytes (`0x005F9360–0x005F9378`). It has no explicit stack arguments and
does not modify the four bytes, clear an active bit, query prior state or check a
reply. Its return is void; the instructions define no success Boolean or full-EAX
result (`0x005F937D–0x005F9380`). It does not itself deallocate storage.

Thus an ordinary pair requests entry and exit operations using the stored
selections, without storing service identities or previous pause states. The
helper reacquires participants on exit. Equal flags do not suppress a repeated
call. There is no class nesting counter, LIFO check or previous-state restoration.
The service's own hidden-pause counters, where publicly specified, are foreign
behavior. Multiple intact scope objects may be paired in an interrupted order;
this is not evidence for callback reentry. Arbitrary field mutation is outside the
ordinary lifetime protocol, although the destructor plainly reads storage anew.
Invalid storage, duplicate destruction, copy/move protocols, exceptions, constructor
failure unwind and callback reentry are Unsupported, not promised safe no-ops.

## 3. Separate helper algorithm

### 3.1 Inputs, gating and service order

`SC4GamePause(D,F1,F2,F3,F4)` is a caller-cleaned five-slot function. Only the
low byte of each slot is tested. D low byte zero selects exit operations; any
nonzero low byte selects entry operations. Each Fi independently selects its
stage by low-byte nonzero, in the fixed order F1, F2, F3, F4. There is no test
for equality to 1, no aggregate success test and no previous-state query.
For any argument, `2`, `0x80`, `0xFFFFFFFF` select true, whereas `0x100` selects
false. The constructor stores these flags respectively as `02`, `80`, `FF`, `00`.
Its entry uses full original slots, but the helper's byte tests make entry and
exit selections agree for intact storage.

Each selected stage independently zeroes the same 32-bit temporary slot (the
now-consumed F1 argument slot), obtains the framework with `RZGetFramework`,
and, if nonnull, calls framework slot `+0x14` with `(service ID, IID, &temporary)`.
The output pointer, not the lookup's return value, controls the next step.
An absent framework skips lookup, leaving the initialized null output. Every
selected stage reaches its temporary cleanup before the next selected stage.
Unselected stages perform no framework, lookup, operation or cleanup calls.

| Stage | Lookup `(service ID, IID)` | Nonnull participant operation when D≠0 | When D=0 | Positive instruction evidence |
|---|---|---|---|---|
| F1 (typed identity unresolved) | `(0xC416025C,0x0073283C)` | slot `+0x48`, no arguments | slot `+0x4C`, no arguments | Gate `005F91C0–91C9`; lookup `91CB–91EB`; pointer/direction/calls `91EE–9205`; cleanup `9208–920C` |
| F2 animation | `(0x86AD10EC,0x86AD10ED)` | `PauseAnimationSystem(1)`, slot `+0x30` | same slot with 0 | Gate `005F9211–9215`; lookup `9217–9237`; pointer/direction/calls `923A–9254`; cleanup `9257–925B` |
| F3 simulator | `(0x46956659,0x8695664E)` | `HiddenPause()`, slot `+0x28` | `HiddenResume()`, slot `+0x34` | Gate `005F9260–9264`; lookup `9266–9286`; pointer/direction/calls `9289–92A0`; cleanup `92A3–92A7` |
| F4 application → current city → clock | `(0x66,0x26CE01C0)` | clock `HiddenPause(1)`, slot `+0x68` | same slot with 0 | Gate `005F92AC–92B0`; lookup `92B2–92CF`; chain `92D2–92F4`; direction/calls `92F6–9300`, `930D–9310`; cleanup `9303–9308`, `9313–9318` |

The F1 constants are literal PE instructions, including the unusual IID whose
numeric value lies inside another exported function. The public ID table calls
`0xC416025C` the graphic-system service. This evidence does not justify calling
the returned interface a sound interface or mapping its operations to another
service. AGP1 remains open; the boundary is a raw lookup reply and individual
virtual operations, not a direct native-address fallback. The animation method
name/slot comes from the public ABI table; simulator/clock names match the
existing advisor and clock projections. Participant internals are not reopened.

### 3.2 Pointer guards, references and replies

For F1/F2/F3, a null lookup output skips the participant operation and proceeds
through null cleanup. A nonnull output invokes the selected operation regardless
of the lookup return register, then calls Release once through slot `+0x08`.
Normal cleanup null tests and Release calls are at `0x004495DF–0x004495EF`
(F1) and `0x005F8F3F–0x005F8F4F` (other stages). Template names attached to
these shared bodies do not change which temporary is released.

F4 additionally checks the application pointer, the application slot `+0x94`
current-city reply, and city slot `+0x158` clock reply, in sequence. Any null
reply skips the remaining chain but still cleans up the application temporary.
A nonnull clock receives exactly one slot `+0x68` call with a full 32-bit stack
argument 1 or 0. Neither city nor clock is AddRef'd or Released here. There is
no explicit AddRef on the framework or any participant in this helper; acquiring
a reference through lookup is a foreign service obligation. The helper releases
each nonnull lookup output, with no independent refcount validation.

Both F4 directions clean up the application temporary. In particular the
pause path computes its address at `0x005F9303` before popping EBX and tail-jumping
to cleanup. Pseudocode's apparent cleanup of the direction slot on that path is
incorrect: the instruction stack offsets identify the F1 temporary in both paths.
No helper output parameter is exposed; its temporary is internal stack storage.

Lookup return bits, participant operation return bits and Release return bits are
not tested. Pointer getters use full EAX only as a null/non-null pointer. No
overall success result is constructed: the helper is void, and residual AL/EAX
is not a contractual status channel. The destructor also leaves no defined
success result. There is no conditional rollback: an earlier participant call
and its Release remain in the trace when a later stage is unavailable or replies
with false. A false-valued operation reply does not stop later stages. A null
framework/output is locally guarded; an invalid nonnull pointer is not.

Different selected stages may see different framework replies even in one
helper invocation. Destructor lookup may return a different simulator, app,
current city or clock than entry, or entry may have skipped a participant that
exit finds present. No binding is remembered to compensate for this. Effects on
service state are exactly the foreign individual calls, not a promised final
whole-game pause state.

## 4. Independent proposed traces

PROPOSED / UNEXECUTED. These literal fixtures qualify the spec before any future
production output; they are not native runs or a symbolic equivalence claim.
The ledger below records own field/branch/acquisition/reference decisions. Foreign
operations are individual callbacks; their internal state is not inferred.
