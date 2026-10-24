# SC4 audio owner contracts

Supplement to [audio.md](audio.md), sections 17–21. Evidence: Windows 1.1.641 export,
named-function inventory and vtable pointers, audited 2026-10-23 (re-audio9).
This supplement describes behavior; unresolved numerical and resource-loading rules are explicitly
listed in [questions/audio.md](questions/audio.md). It does **not** establish full audio coverage.

## 17. Owner inventory and implementation boundaries

The export's named `cSC4Audio*` functions identify the following **15 owners** (folded `icf::`
prefixes normalized). There is no separate named owner `cSC4Audio`. The counts below count named
entries, including folded names; they are not counts of distinct virtual methods. Unnamed methods
reached through these owners' vtables belong to the same contract. Do not omit scape because only
two of its functions have class-qualified names.

| Owner | Named entries | Existing contract / added contract |
|---|---:|---|
| `cSC4AudioAEMSEventSystem` | 24 | [audio.md](audio.md) §10.7: bank lifetime, service/event IO and errors |
| `cSC4AudioAEMSSoundBank` | 3 | [audio.md](audio.md) §10.7: bank construction, file/resource loading |
| `cSC4AudioAmbienceLayer` | 44 | [audio-ambience.md](audio-ambience.md) §8: construction, update, scores, loops, filtering, failures and slots |
| `cSC4AudioAmbienceManager` | 32 | [audio-ambience.md](audio-ambience.md) §§1–7: construction, services, messages and occupants |
| `cSC4AudioCOMDirector` | 6 | [audio.md](audio.md) §§1.1, 11: class registration and application lifetime |
| `cSC4AudioEmanationModel2D` | 1 | [audio.md](audio.md) §3.4 and §19 below |
| `cSC4AudioEmanationModel3D` | 1 | [audio.md](audio.md) §3.4 and §19 below |
| `cSC4AudioEmanationModel3DEffects` | 1 | [audio.md](audio.md) §3.4 and §19 below |
| `cSC4AudioEventHandler` | 18 | [audio.md](audio.md) §7: full message and sound-event dispatch |
| `cSC4AudioEventHandlerDisasterFire` | 6 | [audio.md](audio.md) §7.4; supplementary lifetime observations in §20 |
| `cSC4AudioListener` | 11 | §18 below |
| `cSC4AudioPreferencesWinProc` | 20 | §21 below; window framework in [ui-windows.md](ui-windows.md) |
| `cSC4AudioScapeManager` | 2 | §20 below; unnamed vtable methods are material |
| `cSC4AudioSemManager` | 4 | §19 below; the export's `Init` name needs qualification |
| `cSC4AudioSoundManager` | 25 | [audio.md](audio.md) §8: instance state, focus and message/event dispatch |

### 17.1 Drawing and IO contract

The managers, listener, bank wrappers and emanation models have no drawing entry point in the
interfaces described here. Their outputs are sound-system calls, resource reads and messages.
Camera data is an **input** to listener positioning. Rendering ownership stays with the view.
The preferences owner implements a window procedure and uses existing child controls and modal
windows; its control messages are not message-server sound events. Use the window framework's
drawing/ownership rules rather than inventing an audio framebuffer. *[compat]*

Keep three dispatch domains distinct: message-server types (e.g. listener-position changed),
sound-system event IDs (start/stop/volume), and UI control notifications. In particular,
`0x6A451908` is a sound-system stop event, not a subscription in the message server.

### 17.2 Manager-message boundary matrix

| Receiver | Input and payload boundary | Observable result / failure rule |
|---|---|---|
| Event handler | Main message-target and modal message-target views differ; see audio §7.2–§7.3 | Handlers turn game messages into sound-system events. Automaton play/stop does not go directly to the sound manager. |
| Sound manager | `DoMessage` `0x004F9430`, secondary message-target at `+0x0C`; audio §8.3 | Listener-position updates refresh running sounds. Sound start/stop/master-volume are handled by `DoEvent`, not by confusing their IDs with message types. |
| Ambience manager | `DoMessage` `0x004EAF20`; requires message payload IID `0x65297976` | Failed payload query returns false; recognized messages dispatch as ambience §7; otherwise true. |
| Listener | `DoMessage` `0x004F1E50`, primary slot 3 | Null returns false; non-null unrecognized message returns true. Camera and tuning inputs are §18. |
| Scape manager | Its service-update and soundscape interfaces are §20 | No message-target interface is established by this audit. Do not synthesize one from the service tick. |
| SEM manager | Registration/lifecycle interfaces are §19 | No message dispatch slot in its four-entry primary table. |

This matrix supplements, rather than replaces, the full subscribed-type lists and payload tables
in [audio.md](audio.md) §7–§8 and [audio-ambience.md](audio-ambience.md) §7.

## 18. Listener: camera inputs and position notifications

### 18.1 Object, methods and lifetime

Factory `0x004EB0E0` allocates 124 bytes. Construction `0x004F1930` initializes refcount
`+0x04` to zero, initialized byte `+0x08` false, three vectors at `+0x0C`, `+0x18`, `+0x24`
to zero, last-notified position at `+0x30` to zero, and four empty vector containers at
`+0x3C`, `+0x48`, `+0x54`, `+0x60`. Sound-system and message-server references at `+0x74`
and `+0x78` start null. Scalar initial bits at `+0x6C` / `+0x70` are `0x3F800000` /
`0x40800000` (1 and 4). These are defaults, not a guarantee of usable tuning arrays before city init.

| Method | Address / slot |
|---|---|
| QueryInterface / AddRef / Release | Table `0x00A9421C`, slots 0/1/2: `0x004F1330`, `0x005F47F0`, `0x004F1A50` |
| DoMessage | Same table, slot 3, `0x004F1E50` |
| Init / Shutdown | `0x004F1360` / `0x004F1400` |
| HandleCheatCodeMessage | `0x004F1470` |
| UpdateListener | `0x004F14B0` |
| Calculate zoom / read tuning | `0x004F18C0` / `0x004F1A80` |
| City init/shutdown / destructor | `0x004F1D60` / `0x004F19E0` |

QueryInterface accepts `1`, message-target IID `0x452294AA`, and `0xCA4A66B9`, returning
the object base and acquiring a reference. Other IDs return false. Init is idempotent;
it subscribes city init `0x26D31EC1`, city shutdown `0x26D31EC2`, and tuning change
`0xC53D10AA`. Shutdown removes those subscriptions and clears its initialized state.
Init stores the acquired message-server reference, then dereferences it without a null gate
(`0x004F13BC`). An absent framework/server therefore faults before setting initialized; it
does not return false. Shutdown, when initialized, unsubscribes the three types in that order,
clears/releases message server, clears/releases sound system, then clears initialized and
returns true. An uninitialized Shutdown returns true without work. Destruction releases server,
sound system and the four vector allocations in reverse member order; it does not call Shutdown.

### 18.2 Message dispatch

| Type | Action |
|---|---|
| `0x26D31EC1` | Acquire the city sound-system reference, subscribe `0xA6B79602` and `0xA6B79612`, read tuning, calculate zoom values, update listener. |
| `0x26D31EC2` | Unsubscribe those two camera notifications and release the city sound-system reference. |
| `0xA6B79602`, `0xA6B79612` | Update listener. |
| `0xC53D10AA` | Read tuning, recalculate zoom, request sound-system event `0xAA36D9B6` if its city sound-system reference exists, then update listener. |
| `0x230E27AC` | Invoke cheat-message helper: QI for `0x65297976`, release the acquired view and return true; null input/missing view returns false. It interprets no command and changes no listener state (`0x004F1470`–`0x004F14A4`). |
| Other non-null input | No action, return true. |

The outer dispatcher returns true after recognized helper calls even if their own result is
false (`0x004F1E94`–`0x004F1EAF`). It returns false only for null input. *[compat]*
Do not subscribe the listener to its own output `0x8A4A69C7`.

### 18.3 Position output and zoom arithmetic

The full geometry and failure rules are in [audio §18.4](audio.md#184-listener-geometry-tuning-and-failure-contract-au40).
Sound-system listener orientation and position are updated before notification gating.

The settled zoom rule is: for each of five indices, multiply the f32 value in the `+0x48`
array by the f32 value in the `+0x3C` array, then by the f32 scalar at `+0x6C`, and store
the result as f32 in the corresponding process-global zoom-height entry. Instructions
`0x004F18C3`–`0x004F1921` use x87 intermediates and round at the final f32 store; they
do not store a rounded intermediate between the two multiplies. *[compat]*

The notification threshold compares **squared camera** displacement from the saved camera position
against the current zoom's f32 threshold. Only strictly greater sends; equality and unordered
comparison do not (`0x004F1826`–`0x004F1845`). The saved camera position is updated
before message allocation. `0x004F1880`–`0x004F18A0` constructs a standard message:

| Field | Value |
|---|---|
| Type | `0x8A4A69C7` |
| Pointer 1 (setter slot 33, `+0x84`) | Listener object |
| Void payload (setter slot 11, `+0x2C`) | Pointer to listener's position vector at `+0x0C` |

The payload is a pointer into listener state, not three copied message integers. Consumers must
use the payload interface described in audio §3.2; persistence of a position snapshot must not be
assumed. Sound-manager and ambience-manager consumers are linked in §17.2.
The message is AddRef'd, filled, delivered through server `+0x10(message, 0)`, then Released;
the server's return is ignored. Allocation failure reaches a null message dereference after
saving the camera position (`0x004F1866`, `0x004F18BB`); it is not a recoverable false return.

## 19. SEM registration and the three emanation owners

`cSC4AudioSemManager` is a 32-byte factory object (`0x004ED700`). Its primary table
`0x00A9403C` has slots 0 QueryInterface `0x004ED0D0`, 1 shared AddRef `0x005F47F0`,
2 Release `0x004ED0B0`, 3 deleting destructor `0x004ED730`. QI accepts `1` and
`0xAA355871`, returns the base, and acquires a reference. A count of one releases through
destruction; the decrement path is not a saturating-zero operation.

The object retains three model pointers (`+0x08`, `+0x10`, `+0x18`), auxiliary references
at `+0x0C` and `+0x14`, and Hit manager at `+0x1C`. Destructor helper `0x004ED6A0`
releases non-null retained references. Model creation/registration is in `0x004EDAA0`:
failure to obtain Hit manager returns false; each model is allocated as 40 bytes and queried
for lifecycle IID `0x89E51472`. Model initialization failure prevents later successful
registration. Full construction, retained-state failure rules and teardown are
[audio.md §19.2](audio.md#192-sem-lifetime-and-model-parameter-contract-au41).

**Name caveat:** the export calls `0x004ED750` `Init`, but its observed effects are
unregistration and teardown: remove model IDs `0x7DCDED2D`, `0x7DCDED3D`, `0x0A63A543`
from Hit, query retained models for the lifecycle interface, call its shutdown slot, and release
retained objects and Hit references. Do not implement this teardown as initialization merely
because of the recovered name. Registration uses Hit slots 19/20 (`+0x4C` / `+0x50`).
The director calls registration after Listener Init and teardown before Listener Shutdown;
§19.2 specifies caller ordering and the null-Hit teardown quirk.

### 19.1 Shared model interface

The constructors and defaults are [audio.md](audio.md) §3.4. The 3D constructor's previously
missing ID is **`0x7DCDED3D`**, verified by `0x004ED9E7`; primary/secondary tables are
`0x00A940A4` / `0x00A94090`. Names and IDs select models, not UI labels.

| Primary slot | 2D / 3D / 3DEffects target | Contract |
|---|---|---|
| 0 | `0x004ED930` (all) | QI: `1` and `0xDDCF3BD7` return base; lifecycle IID `0x89E51472` returns `base+4`; success AddRefs. |
| 1 / 2 | `0x004BDF00` / `0x004ED910` (all) | Shared reference operations; unrelated export class labels are folding artifacts. |
| 3 | `0x004ED900` (all) | Model identifier getter; recovered class label is not semantic evidence. |
| 4 | `0x004EDA60` (all) | Calls supplied object's slot 14 with embedded name view, ignores result, returns true; audio §19.2. |
| 5 | `0x004ED210` / `0x004ED2A0` / `0x004ED3F0` | Parameter transformation; signed first division, logical final shift and projected effects pan: audio §19.2. |
| 6 | `0x004ECDA0` / `0x004ECE70` / `0x004ECF90` | Ordered parameter copies and 3D/effects mode writes: audio §19.2. |

These models do not read PCM files, issue window drawing, or handle game messages through this
interface. They transform sound parameters for Hit. The 3D transformations transfer position
parameters `0x20`–`0x22` and `0x40`–`0x42`; do not treat them as message IDs. Fixed-point
volume/pan arithmetic, parameter flags, rounding and failure behavior are specified in audio §19.2.

## 20. Scape service and fire-handler boundary

### 20.1 Scape construction and slots

Factory `0x004EB120` allocates 152 bytes and returns the **service view at base+4**;
failure returns null. Constructor `0x004F6140` initializes three tables at base / `+4` / `+8`:
`0x00A94340` / `0x00A94308` / `0x00A942F8`. Refcount `+0x18`, sound-system and sound-manager
references `+0x0C` / `+0x10`, initialized byte `+0x14`, elapsed bookkeeping `+0x38`,
enabled byte `+0x84`, and fields `+0x88`–`+0x90` start zero. Service ID `+0x80` starts
`0x49E50BA1`. Timer at `+0x20` is constructed with clock argument 4; three critical sections
at `+0x3C`, `+0x44`, `+0x4C` and three empty lists at `+0x54`, `+0x58`, `+0x5C` are created.
Fields `+0x60`–`+0x74` copy zero from `0x00B21AE8` in the initial image.

Primary QI `0x004F5210` accepts `1`, soundscape IID `0x29E39DB4` (base), and service IID
`0x287FB697` (base+4). Successful queries AddRef. Unrecognized IID returns false.

| Primary slot | Target | Behavior established / remaining boundary |
|---|---|---|
| 0–2 | `0x004F5210`, `0x00524750`, `0x004F5270` | QI / AddRef / Release; shared AddRef name is unrelated. |
| 3–4 | `0x004F53E0`, `0x004F5700` | Initialize / shut down (§20.2). |
| 5 | `0x004F64F0` | Register group using supplied resource, key/flags and lifetime; duplicate key returns false. |
| 6 | `0x004F6A00` | Remove keyed group; missing key returns false. |
| 7 | `0x004F65C0` | Keyed group operation; missing key returns false; inner behavior AU42. |
| 8 | `0x004F6620` | Operate on groups whose saved flags contain every requested mask bit; mode-dependent cleanup AU42. |
| 9–10 | `0x004F7850`, `0x004F7AF0` | Keyed instance operations; missing group clears instance output and returns false. |
| 11 | `0x004F57B0` | Enumerate group positions into caller buffer, subject to capacity. Full capacity/output contract AU42. |
| 12–17 | `0x004F7300`, `0x004F51E0`, `0x004F6B30`, `0x004F7510`, `0x004F5E30`, `0x004F5A20` | Additional instance/group operations; signatures and behavior AU42. |
| 18 | `0x004F69E0` | Deleting destructor wrapping `0x004F62C0`. |

Service table slots 0–2 are adjustment thunks; slot 3 is `0x00407BA0`, slot 4
`0x0070C880`, slot 5 `0x004F6230` returns `0x80000000`, slots 6/7 `0x004F6240` /
`0x004F6250` read/write the enabled byte using the adjusted service view. Slots 8/9
`0x004F62A0` / `0x004F62B0` wrap initialize/shutdown, slot 10 `0x004F67C0` performs
the service tick, and slots 11/12 use `0x005DEB40` / `0x004EA510`. Service getters'
full meanings are AU42; do not infer them from folded names. Table `0x00A942F8` has four
entries: adjustment thunks in slots 0–2 and slot 3 `0x004F5200`.

### 20.2 Lifetime and sound IO

Initialize is idempotent. It requires sound system `(service 0x23CECBA0, IID 0x106077D2)`
and sound manager `(service 0x1AF85BFB, IID 0x3AF85BB7)`, retaining both. It also retains
the sound system in process-global `0x00B21AEC`. With a framework available it registers
the service, starts its timer and marks initialization successful. Missing prerequisites return
false; partial reference acquisition is not documented as transactional rollback. *[compat]*

Shutdown, when initialized, removes groups, unregisters the service from the framework,
releases both local service references and the global sound-system reference, and clears
initialized state. Repeated shutdown succeeds. A stopped scape instance emits sound-system
event **`0x6A451908`** with the instance identifier and three zero arguments when the
global sound system exists (`0x004F5B80`). Missing global sound system suppresses that
external call while local removal continues.

Tick `0x004F67C0` uses elapsed timer difference, reduces expiry countdowns without going below
zero, and stops/removes expired instances. It also advances sound parameters toward targets.
Full timing widths, smoothing, group/resource format, allocation failures and instance ownership
remain AU42. These are material open items, so this is not a complete scape algorithm.

### 20.3 Fire-handler integration

The existing identity section is [audio.md](audio.md) §7.4. Its lifecycle view table
`0x00A941D4` has Init in slot 3 (`0x004F0740`) and Shutdown in slot 4 (`0x004F03A0`).
Init is idempotent and requires the message server; unavailable message server returns false.
It subscribes occupant insert/remove (`0x99EF1142` / `0x99EF1143`) and obtains the scape
service using `(0x49E50BA1, 0x29E39DB4)`. Shutdown removes subscriptions, releases the
scape reference and clears its initialized state. It also removes its city-init subscription.
Fire occupants are handed to the scape interface with sound identifier `0x49E4CE85`
(`IterateFireOccupantCallback`, `0x004F0480`); the callback ignores a null occupant.
Full method inventory, message dispatch, construction/defaults and removal behavior are specified
in [audio §7.4.1](audio.md#741-fire-handler-owner-contract-au44). In particular, Shutdown does
not remove the scape group, and failed initialization does not roll back subscriptions.
Section 7.4 establishes that the separate torch methods belong to the main event handler.

## 21. Audio preferences window

### 21.1 Interface and inventory

For modal/window procedure conventions see [ui-windows.md](ui-windows.md). Primary table
`0x00A94264` has seven entries: slots 0–2 QI/AddRef/Release (`0x004F1F30`, `0x004F1F60`,
`0x004F26D0`); slot 3 DoWinProcMessage `0x004F4280`; slot 4 framework DoWinMsg
`0x0041D4C0`; slot 5 ShowDialog `0x004F5120`; slot 6 Shutdown `0x004F2210`.
Bytes following slot 6 are strings, not further methods.

| Additional method | Address |
|---|---|
| Slider volume / update sound system / read tuning | `0x004F1F90` / `0x004F2230` / `0x004F22C0` |
| End dialog / update preferences / default song selections | `0x004F2480` / `0x004F2520` / `0x004F2580` |
| Modal window / test file existence / static initialization | `0x004F25F0` / `0x004F2A70` / `0x004F2B70` |
| Song enabled / insert music list / exclusion list | `0x004F2D70` / `0x004F31C0` / `0x004F3410` |
| Control notification / initialize dialog | `0x004F35F0` / `0x004F44C0` |

Persistent preferences are separate from the window's state: window references at `+4` / `+8`,
modal-notification flag at `+0x0C`, preferences pointer at `+0x10`. Construction and the
complete song enumeration/exclusion format remain AU43. Do not assume a preference rewrite
merely because a window notification was consumed.

### 21.2 Sound and preference IO

Updating preferences copies the sound-enable byte, master/effects/music volume fields, a further
enable byte, and **two 30-byte song-selection arrays** to the preferences object, then immediately
updates the sound system (`0x004F2520`). Default song-list selection copies at most 30 entries
from each of the two lists (`0x004F2580`); it does not extend the persisted arrays for extra songs.

UpdateSoundSys `0x004F2230` obtains sound-system service `(0x23CECBA0, 0x106077D2)`.
If present, event `0x2A5B69C0` receives the stored master volume when sound is enabled,
otherwise zero, followed by three zero arguments. Independently, it posts message
`0xAA650860` carrying the two other stored volume words. Thus failure to acquire the sound
system suppresses the direct call but does not suppress the message. *[compat]*

EndDialog posts `0xAA650861` when its modal-notification flag is set, detaches the dialog
from its parent and releases the held window reference (`0x004F2480`). Shutdown
`0x004F2210` releases its held window reference. Exact OK/cancel/revert ordering remains AU43.

### 21.3 Slider rule and notification acceptance

GetVolumeFromSliderValue takes a slider integer and a bank selector. Selector zero uses
minimum/maximum words `0x00B21A40` / `0x00B09888`; nonzero uses `0x00B21A3C` /
`0x00B09884`. Image defaults are minimum 0 and maximum 1024. Tuning may replace them.
For slider zero return minimum, for slider 1000 return maximum. For interior slider values:
form the unsigned 32-bit difference maximum minus minimum, convert it and the unsigned slider
to x87 numbers, multiply, multiply by **f32 bits `0xBA83126F`** (approximately −0.001),
truncate toward zero through `_ftol2`, subtract the resulting integer from minimum with 32-bit
wrapping, and clamp with unsigned comparisons to minimum/maximum. Instruction evidence:
`0x004F1F90`–`0x004F2014`; the two unsigned-to-floating conversions correct negative signed
loads by adding 2^32. Do not replace this with a double literal 0.001 and nearest rounding.
Out-of-range slider input is not guaranteed monotonic after integer wrap. *[compat]*

DoWinProcMessage rejects a disabled window or a window other than its own primary/modal view.
For an accepted source, key notification type 5 with Escape (27) invokes the cancel control
`0xEA5509E3` and returns false. Type `0x9A1552D2` routes to control `0x8A550C56` and also
returns false. Type 3 is a control notification envelope: recognized notification kinds filter
which controls are forwarded, and unrecognized kinds return true without dispatch. The complete
filter matrix and individual control actions are AU43. Handling work and returning false are
therefore compatible outcomes; do not equate the return with whether audio settings changed.

### 21.4 Additional conformance vectors

These are spec vectors for implementers; no executable tests or engine files are added by this RE lane.

| ID | Given / action | Expected observable |
|---|---|---|
| AU9-01 | Listener DoMessage(null), then an unrelated non-null message | False, then true with no update. |
| AU9-02 | City init, camera notification, city shutdown | Camera types `0xA6B79602`/`0xA6B79612` registered only for city lifetime; updates occur before shutdown. |
| AU9-03 | Last-notified position P; squared movement exactly threshold T | No `0x8A4A69C7`; a strictly greater squared movement sends it. Orientation/position updates still precede threshold gating. |
| AU9-04 | Listener notification captured before delivery | Type `0x8A4A69C7`, Pointer1 = listener, void payload = listener position pointer, not copied coordinates. |
| AU9-05 | Zoom arrays [2,4,6,8,10] and [3,3,3,3,3], scale 0.5 | Stored zoom heights [3,6,9,12,15]. |
| AU9-06 | Construct all three models | Distinct names and IDs from audio §3.4; 3D ID `0x7DCDED3D`; null dependency field `+0x24`. |
| AU9-07 | Scape factory succeeds; query soundscape and service IIDs | Base and base+4 respectively, with acquired references. |
| AU9-08 | Register an already present scape key | False; no second group is inserted. |
| AU9-09 | Scape instance removal with/without global sound system | With: stop event `0x6A451908`, instance, 0,0,0; without: no external stop call, local cleanup continues. |
| AU9-10 | Preferences update, unavailable sound-system service | No direct master-volume call; `0xAA650860` still posted. |
| AU9-11 | Slider 0/500/1000; min 0, max 1024 | Volumes 0/512/1024 for either selector with image defaults. |
| AU9-12 | Slider 500; min 100, max 1100 | Volume 600; minimum is an offset rather than a multiplier. |
| AU9-13 | 31 entries in each default music list | Only first 30 selections from each list persisted. |
| AU9-14 | Escape from an accepted enabled preferences window | Cancel handler invoked; DoWinProcMessage returns false. |
| AU9-15 | Same Escape from disabled or unrelated window | False with no cancel action. |

AU40 and AU44 have additional vectors in audio §§18.4 and 7.4.1. AU41–AU43 still need
additional vectors when settled, especially numerical edge cases and failure rollback.
