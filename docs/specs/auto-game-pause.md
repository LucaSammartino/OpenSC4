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

### 4.1 Fixture vocabulary and explicit reference model

For compact literal traces, `G→g` is one framework acquisition. `L1→p`,
`L2→a`, `L3→s`, `L4→u` mean one lookup on that framework with the exact
respective key pair from §3.1 and a pointer output initialized to null by this
helper. The fixture writes the named output and returns status 1 unless stated
otherwise. Each lookup grants one temporary reference; `R(x)` is one Release
on that output, consuming that temporary reference. These grant/consume rules
are supplied foreign replies for the fixture, not an analysis of lookup bodies.
Every pointer starts with zero temporary references. Framework, city and clock
have separate baseline references that remain unchanged in these traces.

Literal stage expansions, with D truth shown by `+` (nonzero low byte) or `−`
(zero low byte), are:

| Token | Exact ordered callbacks and own decisions | Temporary ref ledger |
|---|---|---|
| `P+` | zero output; `G→g1`; `L1→p`; output nonnull; D true; `p.slot48()`; `R(p)` | `p: 0→1→0` |
| `P−` | zero output; `G→g1`; `L1→p`; output nonnull; D false; `p.slot4C()`; `R(p)` | `p: 0→1→0` |
| `A+` | zero output; `G→g2`; `L2→a`; output nonnull; D true; `a.PauseAnimationSystem(1)`; `R(a)` | `a: 0→1→0` |
| `A−` | zero output; `G→g2`; `L2→a`; output nonnull; D false; `a.PauseAnimationSystem(0)`; `R(a)` | `a: 0→1→0` |
| `S+` | zero output; `G→g3`; `L3→s`; output nonnull; D true; `s.HiddenPause()`; `R(s)` | `s: 0→1→0` |
| `S−` | zero output; `G→g3`; `L3→s`; output nonnull; D false; `s.HiddenResume()`; `R(s)` | `s: 0→1→0` |
| `C+` | zero output; `G→g4`; `L4→u`; output nonnull; `u.GetCity()→c`; `c.Get24HourClock()→k`; both nonnull; D true; `k.HiddenPause(1)`; `R(u)` | `u: 0→1→0`; `c,k: unchanged` |
| `C−` | zero output; `G→g4`; `L4→u`; output nonnull; `u.GetCity()→c`; `c.Get24HourClock()→k`; both nonnull; D false; `k.HiddenPause(0)`; `R(u)` | `u: 0→1→0`; `c,k: unchanged` |

Tokens expand to individual operations, not a foreign whole-helper reply.
There is no Release on `g1–g4`, `c` or `k`. All operation replies in this
table may be arbitrary raw values; they do not decide the next callback.

### 4.2 All selections and direction

The following are independent direct-helper inputs with all selected lookups
present, fresh references per invocation, and D=1. The ordered trace and Release
count are fixed by the selection gates. No field storage is involved in a direct
helper call. For the corresponding D=0 vector replace every `+` by `−`; the
lookup/Release counts and order stay exactly the same.

| F1,F2,F3,F4 | Literal stage trace | Releases |
|---|---|---|
| 0,0,0,0 | empty; no framework acquisition or output initialization | 0 |
| 0,0,0,1 | `C+` | 1 |
| 0,0,1,0 | `S+` | 1 |
| 0,0,1,1 | `S+, C+` | 2 |
| 0,1,0,0 | `A+` | 1 |
| 0,1,0,1 | `A+, C+` | 2 |
| 0,1,1,0 | `A+, S+` | 2 |
| 0,1,1,1 | `A+, S+, C+` | 3 |
| 1,0,0,0 | `P+` | 1 |
| 1,0,0,1 | `P+, C+` | 2 |
| 1,0,1,0 | `P+, S+` | 2 |
| 1,0,1,1 | `P+, S+, C+` | 3 |
| 1,1,0,0 | `P+, A+` | 2 |
| 1,1,0,1 | `P+, A+, C+` | 3 |
| 1,1,1,0 | `P+, A+, S+` | 3 |
| 1,1,1,1 | `P+, A+, S+, C+` | 4 |

`(0,0,1,1)` is the known popup caller selection; `(0,1,1,1)` is the known
application selection. Their labels add no truth-table evidence beyond these
instruction-derived gates. F1's proposed traces specify dispatch, not a claim
that a real framework supplies the unresolved interface.

### 4.3 Raw fields, lifetime pairing and return channels

Use object address token `O` and initial addressed bytes `A5 A5 A5 A5`.
The first framework callback may inspect storage but does not mutate it.

| Input/lifetime | Literal field and callback ledger | Return/reference observations |
|---|---|---|
| Construct O with `(0,0,0,0)`; destroy O | Own stores in order: `00 A5 A5 A5`, `00 00 A5 A5`, `00 00 00 A5`, `00 00 00 00`; both helper traces empty. Destructor reads offsets `3,2,1,0` and makes its helper call even when all bytes are zero | Constructor EAX=`O`; destructor storage remains `00 00 00 00`; zero releases; no destructor status assertion |
| Construct O with `(2,0x80,0xFFFFFFFF,0x100)`; destroy O | Stores `02`, `80`, `FF`, `00` in order. Entry helper receives full slots `(1,2,0x80,0xFFFFFFFF,0x100)` and emits `P+, A+, S+`. Destructor reads `00,FF,80,02` and sends `(0,2,128,255,0)`, emitting `P−, A−, S−` | First callback sees `02 80 FF 00`; constructor EAX=`O` regardless of last callback reply; final fields identical; 3 releases per direction |
| Direct helper `(2,1,0,0,1)`, then `(0x80,1,0,0,1)`, then `(0xFFFFFFFF,1,0,0,1)`, then `(0x100,1,0,0,1)` | Respectively `P+, C+`; `P+, C+`; `P+, C+`; `P−, C−` | Each invocation has 2 releases; no caller-visible output or contractual AL/EAX status |
| Direct helper `(1,1,1,0,0)` twice, without an intervening exit | `P+, A+, P+, A+`; no query or suppression on the second call | Each output independently grants/consumes one reference; no stored helper active flag |
| Construct A `(0,0,1,1)`; construct B `(0,0,1,0)`; destroy A; destroy B | Fields A=`00 00 01 01`, B=`00 00 01 00`; ordered operations `S+, C+, S+, S−, C−, S−` | Releases after each expanded stage: 2,1,2,1; both field sets unchanged at exit. No LIFO guard; no saved state restored |

The interrupted A/B sequence is serial, with every call returning before the
next begins. In the fixture's optional foreign simulator counter starting at 0,
the four simulator callbacks project `0→1→2→1→0`; that is a supplied per-call
counter model, not a new native counter contract. Clock receives 1 then 0 when
A enters/exits, with no operation for B. There is no full-owner final-state
oracle and no real output-sentinel test: these bodies expose no output pointer.

### 4.4 Every guarded absence and ignored reply

These fixtures use D=1 and only the indicated Fi selected unless stated. A
null cleanup produces no Release callback. All normally returning branches are
also valid with D=0; absence skips the operation before direction can matter.

| Fixture | Exact expected prefix, decision and cleanup | Temporary refs |
|---|---|---|
| F1, framework null | zero output; `G→null`; skip lookup; skip operation; null cleanup | 0 |
| F2, framework null | zero output; `G→null`; skip lookup; skip operation; null cleanup | 0 |
| F3, framework null | zero output; `G→null`; skip lookup; skip operation; null cleanup | 0 |
| F4, framework null | zero output; `G→null`; skip lookup and city chain; null cleanup | 0 |
| F1, lookup returns status 1, output remains null | zero output; `G→g1`; `L1→null`; skip operation; null cleanup | 0 |
| F2, lookup returns status 1, output remains null | zero output; `G→g2`; `L2→null`; skip operation; null cleanup | 0 |
| F3, lookup returns status 1, output remains null | zero output; `G→g3`; `L3→null`; skip operation; null cleanup | 0 |
| F4, lookup returns status 1, output remains null | zero output; `G→g4`; `L4→null`; skip city chain; null cleanup | 0 |
| F4, current city absent | zero output; `G→g4`; `L4→u`; `u.GetCity()→null`; skip clock getter and operation; `R(u)` | `u: 0→1→0` |
| F4, clock absent | zero output; `G→g4`; `L4→u`; `u.GetCity()→c`; `c.Get24HourClock()→null`; skip operation; `R(u)` | `u: 0→1→0`; c unchanged |
| All selected; every lookup returns raw status 0 but writes its nonnull output | `P+, A+, S+, C+`, including all four operations | `p,a,s,u` each `0→1→0`; lookup false is not observed |
| All selected; F1 output null and status 0; other outputs present | zero output; `G→g1`; `L1→null`; null cleanup; then `A+, S+, C+` | p stays 0; a,s,u each `0→1→0` |
| All selected; every operation returns raw EAX=0 and every Release returns raw EAX=0 | `P+, A+, S+, C+`; no early stop or compensating operation | each temporary returns to 0 under supplied Release model; reply values do not control the helper |
| All selected; first three outputs present, app city absent | `P+, A+, S+`; zero output; `G→g4`; `L4→u`; `u.GetCity()→null`; `R(u)` | four granted/consumed refs; earlier operations remain, no inverse calls |

There is no observed-false Boolean branch to invent: all tested service replies
in these bodies are pointers. `null` is observed as a pointer; a false raw return
from lookup is ignored even with a nonnull output. No vector supplies an invalid
nonnull object or a throwing callback under the normal-return contract.

### 4.5 Reacquisition and changed identities

These are explicit fixture changes between returning calls, not assumptions
about what a real city transition must do.

| Scoped input and supplied replies | Literal expected trace | Own state/ref result |
|---|---|---|
| O `(0,0,1,1)`; entry simulator s1, app u1, city c1, clock k1; exit s2,u2,c2,k2 | Entry: zero output, `G→g3a`, `L3→s1`, `s1.HiddenPause()`, `R(s1)`; zero output, `G→g4a`, `L4→u1`, `u1.GetCity()→c1`, `c1.Get24HourClock()→k1`, `k1.HiddenPause(1)`, `R(u1)`. Exit: zero output, `G→g3b`, `L3→s2`, `s2.HiddenResume()`, `R(s2)`; zero output, `G→g4b`, `L4→u2`, `u2.GetCity()→c2`, `c2.Get24HourClock()→k2`, `k2.HiddenPause(0)`, `R(u2)` | O bytes remain `00 00 01 01`; s1,u1,s2,u2 each `0→1→0`; no retained old city/clock; baseline refs c1,k1,c2,k2 unchanged |
| O `(0,0,0,1)`; entry app u with no current city; exit same u now returns c2,k2 | Entry: zero output, `G→g4a`, `L4→u`, `u.GetCity()→null`, `R(u)`. Exit: zero output, `G→g4b`, `L4→u`, `u.GetCity()→c2`, `c2.Get24HourClock()→k2`, `k2.HiddenPause(0)`, `R(u)` | O remains `00 00 00 01`; u independently `0→1→0` twice; no matching prior clock entry is required by the helper |
| O `(0,0,1,1)`; entry all present; exit F3 framework null and F4 city null | Entry `S+, C+`. Exit: zero output, `G→null`, null cleanup; zero output, `G→g4`, `L4→u`, `u.GetCity()→null`, `R(u)` | No exit operation on the earlier simulator/clock; no fallback to saved identities; only one exit Release |

## 5. Evidence limits and open work

Own branch, byte-store, return-use and temporary-cleanup claims above are
instruction-backed normal-return contracts. The independent vectors are written
expectations only. No native game, engine, test-source or oracle comparison was
performed. Lookup ownership rules and participant pause/counter semantics remain
the public foreign interfaces' responsibility; the helper does not verify them.

[AGP1](questions/auto-game-pause.md) is the remaining typed first-participant
question. No defaults, native COM/factory identity, allocation protocol, exception
recovery or unsupported lifetime behavior is supplied by analogy. No ABI rows or
native C exports are proposed. This bounded class/helper spec does not change
the capped families or reopen their internal algorithms.
