# Low-level sound, HIT tracks and adaptive music

Windows SimCity 4 Deluxe 1.1.641. RE lane `re-sound-hit5`, 2026-10-23.

## 1. Scope and reading guide

This is the contract below the SC4 audio event handler: HIT track/channel playback,
adaptive score players, framework sound services, directors and playlist input.
The SC4 request/stop chain, sound-event payloads, class registration list, sound
resources and game save boundary remain specified in [audio.md](audio.md), especially
§§1, 4–8, 10 and 12. Occupant and layer policy belongs to
[audio-ambience.md](audio-ambience.md). Do not implement those policies a second time here.

*[compat]* Requirements below describe the original's observable interface behavior.
*[choice]* An implementation may use different containers, locks, threads and audio
backends while preserving that behavior. Addresses and object offsets identify the
Windows interface, not a required core memory layout. Slot numbers are zero-based;
byte offsets in a vtable are four times the slot number.

Evidence is the Windows export's named functions and instruction records, with
addresses cited in each section. Names inherited through identical-code folding
are not reliable descriptions of a method's purpose. In particular, an apparent
window-method name in a sound vtable does not establish a drawing contract.
Unsettled behavior is explicitly tracked in
[questions/sound-system.md](questions/sound-system.md); no answer is implied by
the presence of a method in an inventory. Conformance vectors in §10 are prose
fixtures for a future core runner; this document does not add engine files.

## 2. Shared interface and IO boundary

*[compat]* These owners are nonvisual services/resources. Their inspected interfaces
have no drawing surface, clipping rectangle, paint message or render-to-window
operation. A sound **renderer** is an audio backend. Spatial position/orientation
are listener/source audio inputs, not a visual drawing API.

Use GZCOM references at the boundary: successful QueryInterface acquires a reference;
Release reaching zero destroys the object. A secondary interface pointer is not
necessarily the allocation base. In particular, the manager sound interface is at
`+0x1C`, the track's sound interface at `+0x0C`, and the track's register interface
at `+0x04`. Do not call a primary table using a secondary pointer.

Persistent input is separate from live playback. Playlist input consumes text;
HIT initialization consumes INI entries and database/persist records; track resources
read/write property data. None of these observations establishes save support for
live channels, timers or partially played tracks. See [audio.md §12](audio.md#12-save-behaviour).

## 3. `cHitTrackPlayer`

### 3.1 Identity, defaults and inventory

The constructor is `0x0093E8F4`, destructor `0x0093E882`; QueryInterface is
`0x0093A810`. It exposes IUnknown `1` and track IID `0xBB1BE367` at the base,
register IID `0xFD584467` at `+4`, update IID `0x5B34F0AF` at `+0x14`,
IID `0xAA11A64F` at `+0x10`, sound IID `0x206077D2` at `+0x0C`,
IID `0x306077D2` at `+8`, multiple-source IID `0xDD583E2D` at `+0x18`,
and IID `0xE9D00A9A` at `+0x1C`. An unsupported IID returns false without
the success-path AddRef. The constructor obtains sound-system, HIT-manager and
sound-manager services and constructs an embedded channel. Load count, pause,
error-related state, owned-sound lists and track execution state start clear.
Do not interpret constructor initialization as a successful Load or Play.

Primary table `0x00AD3E48`:

| Slots | Operations, in slot order |
|---|---|
| 0–2 | QueryInterface, AddRef, Release |
| 3–6 | interface views (listener-shaped folded entry, another folded view), AsIGZSndMultipleSources, AsIGZSndRegisterSet |
| 7–13 | SetTrackDef, GetTrackDef, SetHitList, GetHitList, CompletionTime, Reset, Update |
| 14–17 | folded entry (unsettled), PlayPause, SetErrorCondition, GetErrorCondition |
| 18–24 | Step, CurrentPlayPosition, destructor, two folded entries, folded interface view, HandleTimerCallback |
| 25–27 | unnamed `0x0093A94C`, unnamed `0x0093AEB1`, folded entry |
| 28–33 | SetRegisterVal, ClearRegisterVal, GetRegisterVal, HasRegisterVal, HandleNotifySetPropertyValue, GetRegisterFloat |

Sound table `0x00AD3D48`, at object `+0x0C`:

| Slots | Operations, in slot order |
|---|---|
| 0–4 | QI/AddRef/Release thunks, Init-shaped folded entry, Shutdown |
| 5–11 | Play(four payloads), Play(one payload), IsPlaying, Stop, Kill, folded entry, GetUseCount |
| 12–19 | folded entry, SetID, Load, Unload, IsLoaded, Pause, Unpause, IsPaused |
| 20–28 | GetVolume, SetVolume, fade-shaped folded entry, IsFading, GetPan, SetPan, GetFrequency, SetFrequency, GetOriginalFrequency |
| 29–36 | two folded entries, GetSoundPriority, SetSoundPriority, SetName, GetName, three folded entries |

Other named operations: CheckedRegId `0x0093A7EB`, CurrentPlayPosition
`0x0093A959`, SetPosition `0x0093AA0E`, SetRegisterVal `0x0093ADBA`,
UpdateVolPan `0x0093AE8C`, UpdatePitch `0x0093B508`, UpdatePatch `0x0093B9AE`,
HandleNotifySetPropertyValue `0x0093BD06`, NoteOn `0x0093BFEF`,
StartSndDependent `0x0093C0EC`, DoCommand `0x0093C229`,
HandleTimerCallback `0x0093DF4C`, NoteOff `0x0093E0FC`, Step `0x0093E2D8`.
Register helper wrappers occupy `0x0093E4B2`, `0x0093E4E3`, `0x0093E4F6`;
the float-named getter is `0x0087B103`. Names alone do not establish operand types.

### 3.2 Loaded, playing and paused are separate states

*[compat]* Load increments a 32-bit count and invokes the current underlying
sound's Load only when the result becomes one. Unload decrements it and invokes
the underlying Unload only when the result becomes zero. Both return true,
including when there is no current sound, and do not forward the child's failure.
IsLoaded tests count nonzero. There is no underflow guard: Unload at zero makes
IsLoaded true. Evidence: instructions `0x0093AF41–0x0093AF72`.

*[compat]* IsPlaying is true if the track execution flag is set, or if any
dependent sound reports playing (`0x0093AFEE`). An exhausted instruction stream
therefore need not mean the whole track is silent. Pause records pause state,
pauses dependents, and unregisters the track update target when playing. The
pause argument determines whether to pause the current source as well.
Unpause clears pause state, unpauses dependents and a paused current source,
and re-registers a playing track with period `0x14` (20). Its return depends on
the resulting error condition (`0x0093B01E`, `0x0093B08B`).

*[compat]* The one-payload Play forwards to four-payload Play with the other
three values zero (`0x0093A8BA`). Repeated Play while IsPlaying is true returns
success without restarting when property `0xFF303B1B` is zero; otherwise it
calls Kill before starting the new request (`0x0093E146`). The four payloads
are written without notification (third argument 1 to register slot 3,
`0x0093B268`, which skips the notification when it is nonzero, `0x0093B346`)
under properties `0xFF141C48`, `0xFF9250B3`,
`0xFF9EC945`, `0xFF1885BE`. Argument-routing mode is property `0xFF4DCD0D`;
the additional mode-dependent writes are:

| Mode | Destinations in write order, for payloads A, B, C, D (IDs hex) |
|---|---|
| 1 | FF90AAC8 = A, FF3EC880 = B |
| 2 | FFB4335C = A |
| 3 | FFB4335C = A, FF90AAC8 = B, FF3EC880 = C |
| 4 | FFCC4D2D = A, FF4A01D6 = B, FF469820 = C |
| 5 | FFB4335C = A, FFCC4D2D = B, FF4A01D6 = C, FF469820 = D |
| other | No additional writes; the four original argument properties are still written. |

All these writes suppress notification. Evidence: `0x0093E1B9–0x0093E296`.
Afterwards call primary slot 15 and save its boolean; independently, nonzero
FF59FDD8 starts a dependent sound. Only if the saved result was true, call
sound-view slot 18 (Unpause) and use its result as Play's result
(`0x0093E299–0x0093E2D0`). Dependent-start failure is not combined with it.
Worked case: mode 3 with `(9,512,-20,77)` writes all four argument properties
and additionally object ID 9, volume 512 and pan -20. D has no extra mapping.

### 3.3 Properties, errors and commands

*[compat]* Sound priority, volume and pan are register/property-backed, at
IDs `0x13`, `0x15`, `0x16`; original frequency is read at `0x49`
(`0x0093E453–0x0093E4D0`). These getters are not independent cached fields.

*[compat]* CheckedRegId compares its signed 32-bit argument to `0x4A`.
Values greater than 74 report `Illegal Register Id` through the HIT manager
and substitute register 1; negative values pass this particular check.
This is not a general guarantee that every negative index is valid.
Evidence: `CMP`/signed `JLE` at `0x0093A7EF–0x0093A7F5`.

#### Register notifications (SS1)

SetRegister offers IDs at most FF to alias helper `0x009377C9`, which indexes
80 words at `0x00B169F8` for IDs below 50 (hex). A nonzero alias replaces
the ID; a zero alias leaves the small ID unchanged (`0x0093B26E–0x0093B28B`).
This API limit differs from bytecode CheckedRegId's limit of 4A. SetRegister
also updates comparison flags against zero **before** property work: equality
at base +9C, signed greater at +9D, signed less at +9E
(`0x0093B291–0x0093B29A`, `0x0093A8CB–0x0093A8F0`). Its third argument
suppresses notification when the low byte is nonzero. Otherwise it invokes
primary slot 32 with the mapped property ID and returns its boolean
(`0x0093B346–0x0093B35C`). Suppressed Play writes still affect these flags.

HandleNotifySetPropertyValue first reads back the current integer value;
its complete property dispatch is below (`0x0093BD06–0x0093BFEE`). IDs and
base offsets in this table are hexadecimal.

| Property ID(s) | Action and result |
|---|---|
| FF099DB2 | UpdatePitch, then true. |
| FFAEC905 | UpdatePatch, then true. |
| FF2D16F2 | Nonzero clears byte at base +100; return true. |
| FF5F15E1 | Primary slot 26 with integer value; return its boolean. |
| FF90AAC8, FF469820, FF4A01D6, FF4C6660, FF53DBF3, FF3EC880, FF40FF96, FF444C9A, FFA6A614, FFAAB623, FFC20061, FFC6B36D, FFCC3B29, FFCC4D2D, FFCE9997 | UpdateVolPan, then true. |
| FF3071B9, FF0150F2, FF734343, FF86FE79, FFD3D39B | If current register interface at +7C exists, get float value with default zero, narrow to single precision and forward `(ID,float)` to its slot 7; return true. |
| FF050A94, FF595FFD, FF7E5922, FFA42951, FFA9B525, FFBDB196, FFF3EDF2, FFF7B8DF, FFFF4E79 | If current register interface exists, forward `(ID,integer,0)` to its slot 3; return true. |
| FFB36659 | With a current register interface, nonzero pauses current sound at +80 with argument false; zero unpauses it. Return true. |
| FFC6E666 | Clear retained target at +84. For a nonzero value find a dependent sound with that sound ID, query IID EBB10CDD and retain it. Absent ID is ignored; failed QI reports `Attempt to set target to object with no register interface`. Return true. |
| other | Return true without a track-specific action. |

#### Confirmed bytecode subset

DoCommand fetches an **unsigned byte**, advances PC at base +6C by one,
subtracts one for the index and dispatches through 97 words at `0x0093DC8B`
(`0x0093C237–0x0093C24C`). Buffer base is +68. There is no length check
before the fetch or confirmed operand reads. Below, r is one unsigned byte
passed through CheckedRegId, u8 is an unsigned byte, and u32 a little-endian
four-byte word. Sizes include opcode and assume valid register operands.

| Opcode (hex) | Size | Confirmed rule / instruction evidence |
|---|---|---|
| 01 | 3 | Read note-value register then countdown register; save second value at +48 and NoteOn(first). False reports `Could not set patch - file or resource not found`; 0093C32D–0093C38C. |
| 02 | 2 | NoteOn(u8), discard return; 0093C391–0093C39F. |
| 03 | 2 | Skip one unused byte, call current sound Stop (slot 8) if present and return its result; absent sound returns true; 0093C3A4–0093C3BA. |
| 04 | 3 | SetRegister(r,u8,0); byte is **zero-extended**, not signed; 0093C3BF–0093C407. |
| 05 | 6 | SetRegister(r,u32,0); 0093C40C–0093C455. |
| 06, 4A | 3 | Destination r, source r; set destination from source with notification argument zero. Both dispatch to 0093C4D8. |
| 07 | 5 | Read u32 file offset, push post-operand PC on return list +78, jump to buffer-base + offset; 0093C839–0093C85B. |
| 08 | 1 | Pop return pointer and resume there; empty list follows 0C completion; 0093C88F–0093C8BE. |
| 09 | 2 | Copy integer register r into wait counter +44; no tempo scale; 0093C8C3–0093C8F1. |
| 0A | 5 | Read u32 entry ID, push post-operand PC, resolve HIT slot 14, set PC to result without null check; 0093C85D–0093C883. |
| 0C | 1 | If execution byte +58 set, clear it and Release playing self-reference; 0093C8A9–0093C8BE. |
| 0D | 5 | Jump to buffer-base + u32 offset, without pushing return pointer; 0093C821–0093C832. |
| 0F, 1D, 1E, 5D, 5E | 1 | Dispatch directly to true-return tail 0093DC84; no operand. |
| 00, 1A, 34–3D, 62–FF | 1 | Report `Illegal Instruction` through HIT slot 29, then normally return true; 0093C240–0093C246, table words, 0093DC72–0093DC84. This is not a silent no-op. |

Most reported-error paths also return true; the boolean alone does not prove
successful execution. Byte register IDs greater than 74 report illegal register
and become register 1 before most per-opcode checks. Setter failure reports
after already consumed operands; there is no PC/register rollback. An error
callback can change track error state and prevent further timer execution.

#### Timer and ramp arithmetic

*[compat]* Timer callback `0x0093DF4C` includes the continuation exported as
NoteOff at `0x0093E0FC–0x0093E143`: that address uses the timer's **existing
stack frame**, not a separately callable note-off function. Actual note-off
at the end calls `0x0093AE78`. The callback takes signed 32-bit elapsed
milliseconds. With clear error state, prune nonplaying dependents. If track
execution is stopped and the dependent list is empty, Kill. Otherwise:

When FF0AD592 is nonzero, ramp target-register ID is FF47F685, current value
comes from that register, endpoint is FF2D16F2, speed is FFD1C82F. Zero speed
clears active. Negate speed with 32-bit wrap if `(current < endpoint)` differs
from `(speed > 0)`. Multiply speed by elapsed with low-32-bit signed wrap,
divide by signed 1000 toward zero, add with wrap. Clamp and clear active only
on a **strict overshoot** in speed's direction, then notify the target register.
Exact arrival leaves active until a later callback crosses the endpoint.
Evidence: signed comparisons, NEG, IMUL/CDQ/IDIV and strict branches at
`0x0093E016–0x0093E078`.

If completion-after-ramp byte +100 is set and active is now zero, clear it
and Kill. Otherwise reduce nonzero wait +44 by signed minimum of (wait,elapsed).
Excess elapsed is discarded (`0x0093E0A5–0x0093E0BC`). While error is clear,
execution byte +58 is set and wait is zero, execute commands in this callback.
The local count starts zero; after each command increment it and, when the
**old** count exceeds 100, report `Too many instructions executed without wait
(infinite loop?)`, then return to the eligibility test (`0x0093E0FC–0x0093E11D`).
First report is after command 102, not a hard loop break if the handler leaves
eligibility unchanged. After the loop decrement nonzero note countdown +48
**once per callback**, not by elapsed; reaching zero invokes actual NoteOff
(`0x0093E11F–0x0093E130`).

Worked cases: `04 0D FF` sets unmapped small register 13 to **255**, advancing
PC by 3. `05 0E 78 56 34 12` sets register 14 to 0x12345678, advancing by 6.
Wait 30 with elapsed 20 leaves 10; another elapsed 20 resumes commands, without
carrying excess 10 into their next wait. Ramp current 0, endpoint 10, speed
100, elapsed 100 arrives at 10 with active still set; next identical callback
clamps back to 10 and clears active after a strict crossing.

#### SS1 residual observation boundary

The opcode table is an instruction-verified **subset**, not a complete
interpreter license. Remaining operand/branch verification is narrowed to
this exact set: **0B, 0E, 10–19, 1B–1C, 1F–33, 3E–49, 4B–5C, 5F–61**.
None may be filled in as an unimplemented-success default. The export treats
byte/word pointer arithmetic inconsistently. The following concrete original
observations would settle the remaining cases without relying on those types:

| Residual area | Required observation |
|---|---|
| Every opcode in the residual set | At DoCommand entry/return record PC +6C, all register API calls, flags +9C…+9E, wait +44, countdown +48 and error +5C. Use a separate padded stream per opcode with distinguishable following bytes and register sentinels; record each consumed byte/destination. Bound execution externally. |
| Each residual branch | Run signed equal/less/greater inputs including 80000000; record next PC relative to whole-file buffer +68, return list +78, and any rewind to the opcode. Make taken/untaken targets observably different. |
| Numeric and tempo-derived setup | Record destination bits/flags for -1, 0, 1, signed extrema, divide-by-zero and nonintegral floats. Vary only the proposed tempo/duration property while keeping bytecode/elapsed callbacks identical; record which setup register or wait/countdown changes. Opcode 09 and callback reduction above do no beat conversion; setup conversion and zero-divisor behavior are not inferred from them. |
| Cross-object/source/HIT-list/global-cache operations | Provide recording interfaces, distinct object IDs and return sentinels, then failed lookup/QI. Record argument words, pointed-to writes, ordering and refcounts. This also settles SS2's external source-handler payload. |
| Error continuation, especially opcode 58 | First use an error handler that records without changing state, then one that sets error. Record PC and actions following the callback. 58 dispatches to 0093C253; its report is followed immediately by the body at 0093C265, so do not assume throw terminates the instruction. |

No runtime trace was performed in this lane. This residual is **narrowed to
the stated observations**, not claimed fully decoded; confirmed rules and
worked cases can be used independently. Conventional MIDI behavior or guessed
decompiler operand widths do not settle the residual.

## 4. `cHitMan` and `cHitChannel`

### 4.1 Manager identity and state

`cHitMan` is the HIT system service, CLSID `0x7B1BD993`. Constructor
`0x00939EA4` initializes the system-service identity `0x1B2CE671` with priority
`0x7A116`, clear lifecycle/pause flags, empty resource/track/channel/group maps,
an error callback, a random generator seeded with `0xFFFFFFFF`, and default 2D
and 3D semantic objects. Init `0x009399CB` installs those default semantic
objects and obtains framework sound/persist services. Shutdown `0x00938643`
and destructor `0x00939FA1` are separate lifetime operations.

Its HIT interface is the secondary table `0x00AD2968`, object `+0x1C`:

| Slots | Operations, in slot order |
|---|---|
| 0–8 | QI/AddRef/Release thunks, Init, Shutdown, UpdateView, Pause, folded entry, IsPaused |
| 9–18 | LoadHitFile, UnloadHitFile, RegisterTrackDataBlock, RemoveTrackDataBlock, GetTrackDataBlockAndOffset, TrackDataEntryPoint, CreateTrackPlayer, GetHitList, AllocChannel, FreeChannel |
| 19–27 | RegisterSem, RemoveSem, RemoveAllSems, GetSem (two overloads), NumSems, GetSemAt, Random, Callback |
| 28–34 | SetTrackFlowErrorHandler, HandleTrackFlowError, SetRegisterVal, GetRegisterVal, GetSourceDataField, RegisterSourceDataRequestHandler, folded getter |
| 35–42 | ControlGroup, AddToControlGroup, SetSequenceGroupSnd, GetSequenceGroupSnd, DuckMapSetSndobPri, DuckMapSndobPri, DuckMapMaxPri, DuckMapRemoveAll |
| 43–46 | AddDBSegment, FindDBSegment, RemoveDBSegment, AddTypeToGroupAssociation |

Additional file operations are ParseHitFile `0x00938D8E`, ParseHitFileFromIGZPersistRecordAsIGZFile
`0x009379EF`, and FilepathHasEmbeddedKey `0x00938EC6`; the Unpause-named
routine is `0x00938419`; it is not the implementation of HIT slot 7 (see below).
Primary/service tables inherit
the framework system-service identity contract; they are not playback interfaces.

*[compat]* Channels are retained by the manager after allocation. AllocChannel
creates a channel and queries sound IID `0x206077D2`; only a successful,
nonnull result is added to its channel list. FreeChannel finds the matching
retained pointer, removes that entry and releases it; an absent pointer has no
such release (`0x00938438`, `0x00937D6C`). The allocation failure path is not
safely recoverable in the inspected original: it can reach a virtual call on
the missing allocation. A replacement may return allocation failure instead
of reproducing a process crash *[choice]*.

HIT file parsing reads a file-sized buffer, registers track data and extracts
entry-point pairs. It searches for track/entry markers (`0x00938D8E`);
it does not establish a validated, bounds-safe interchange format. The exact
file grammar, duplicate registration behavior, semantic/ducking arbitration,
pause behavior and source callback boundary are specified below. These are
direct calls/callbacks, not demonstrated SC4 message-server subscriptions.

#### HIT file and registration rules (SS2)

*[compat]* ParseHitFile (`0x00938D8E`) obtains the file's name and byte size,
reads four bytes into a temporary, seeks back through file slot 9 (`+0x24`),
then reads the **whole file** into a buffer. Neither read result is tested.
It scans for little-endian word `0x58415254` (bytes `TRAX`) in four-byte
steps from file offset zero, but the found pointer is **unused**: the track
data block retains the original buffer and complete byte size, not a slice
after TRAX (`0x00938DDF–0x00938E29`, SetDataBlock `0x00938CD7`). There is
no checked magic/version/length field and no requirement here that TRAX be
found. The scan itself can read at/past the buffer end on malformed input.

The block is named, registered and locally released **before** entry parsing.
Then scan from offset zero in **one-byte steps** for `0x50544E45` (bytes
`ENTP`). Following those four bytes are repeated eight-byte records:

| Relative byte offset from ENTP | Width | Meaning |
|---|---|---|
| 0 | 4 | `ENTP` marker |
| 4 + 8n | 4 | unsigned entry ID; if word equals `0x544E4545` (bytes `EENT`), terminate without reading an offset |
| 8 + 8n | 4 | unsigned byte offset **from the beginning of the retained whole-file buffer** |

The loop tests only the start of each record against file end; it does not
check that eight bytes remain, validate target offsets, require EENT, or
escape an entry ID equal to EENT (`0x00938E48–0x00938E80`). AddEntryPoint
uses map assignment, so a later duplicate ID **within a block** replaces its
offset (`0x00938D49–0x00938D5A`). Missing ENTP simply leaves an empty entry
map; the parser normally returns true even so. Allocation failures are not
gracefully checked. A bounds-safe parser may reject malformed/truncated input
instead of reproducing reads beyond allocation *[choice]*.

RegisterTrackDataBlock rejects null or failed IID `0x9B82EAE7`. Otherwise it
removes a previous registration of that **same object pointer**, then retains
and inserts it at the **front** of the block list (`0x009384EB–0x0093851C`,
insert-before-begin helper `0x009690F4`). Distinct objects sharing filenames
or entry IDs are not deduplicated. GetTrackDataBlockAndOffset searches from
front to back and returns the first block whose slot 5 supplies that ID;
it writes the block and offset only on success and does not AddRef that result
(`0x00937C18–0x00937C47`). TrackDataEntryPoint returns block-buffer + offset,
or null if none; it performs no range validation (`0x009379CB–0x009379DD`).
Re-registering an older object thus moves it ahead of newer competing blocks.

RegisterSem obtains the semantic object's ID from slot 3, replaces the retained
object in the map at that ID and returns whether the ID is nonzero. ID zero
is still **inserted/replaced before returning false** (`0x00938BF1`). These
are semantic objects (default 2D/3D), not semaphore locks with wait queues.
GetSem by ID returns the mapped object or null; index enumeration follows the
map. Registration replacement releases the previous retained object.

#### Group, ducking and pause boundaries

ControlGroup(group-ID) returns a cached control object, or allocates one with
that ID, queries IID `0xFB1C0050`, and inserts it in the group map
(`0x00938AF0`). AddToControlGroup calls this service's group lookup then
group slot 3 with the supplied sound (`0x00937972`). SetSequenceGroupSnd
assigns a retained sound in a separate group-ID map, replacing the previous
one (`0x00938A26`); GetSequenceGroupSnd returns null when absent. These
manager methods impose **no extra group priority contest or automatic stop**.
Any policy implemented by the returned control object is a separate interface.

*[compat]* DuckMapSetSndobPri is gated by the manager's initialized flag
(byte `+4` relative to the HIT view). When enabled, nonzero signed priority
sets/replaces the sound-key's entry; zero removes it. An absent zero removal
and an unchanged nonzero value do nothing further. A real change recomputes
DuckMapMaxPri, writes manager register `0x1B`, then invokes UpdateView
(`0x00938A41`). DuckMapSndobPri returns zero for an absent key. DuckMapMaxPri
starts at zero and takes the **signed** maximum; an all-negative map therefore
has maximum zero (`0x00937DB3–0x00937DBA`). DuckMapRemoveAll clears the map
and writes register `0x1B` = 0, then UpdateView (`0x00938492`). No random tie-breaker or
winner removal is performed in these methods.

HIT table slot 6, Pause `0x00937875`, calls slot 42 (DuckMapRemoveAll) and
returns true; it does not pause allocated channels or set the paused flag.
Slot 7 is the three-byte folded entry `0x00949ADE`: **no arguments**, return
true, no unpause action. Slot 8 reads byte `+5` of the HIT view
(`0x0093998F`); neither of these two slots writes it. In contrast the exported
name Unpause at `0x00938419` consumes one 32-bit payload, passes it to slot
17 (AllocChannel) and appends the result at view `+0x30`; its `RET 4` confirms
it cannot describe slot 7. Do not implement recursive channel pause based on
that name.

#### Random and source-handler boundary

Random's first argument is the upper endpoint U and second is lower L.
Subtract as wrapping 32-bit `B = U − L`; when B is zero return L **without
advancing the generator**. Otherwise, with unsigned 32-bit state s, form the
64-bit unsigned product-plus-add `P = s × 0x41C64E6D + 0x3039`, save its
low 32 bits as the next state, take `R = (P >> 16) mod 2^32`, then return
`L + high32(R × B)` modulo `2^32`. This is the actual scaling law, with no
rejection step and no guarantee of an exactly unbiased bounded distribution
(`0x00939993`, `0x00913696–0x009136C9`). For nonwrapping U > L its result
lies in `[L,U)`. Starting from constructor seed `0xFFFFFFFF`, one draw gives
next state `0xBE39E1CC`, R `0x4E6CBE39`; Random(20,10) returns 13.

RegisterSourceDataRequestHandler retains/replaces the handler at view `+0x40`
and returns true (`0x00937961`). GetSourceDataField loads that pointer and
**tail-calls its slot 3**, preserving the caller's entire argument stack and
return bits (`0x00937959–0x0093795E`); it does not translate an event, copy a
payload or test for null. Slot 34 is a folded getter of that retained pointer,
not a second handler registration. The manager therefore specifies an opaque
pass-through call boundary, not a source-data schema.

Worked cases: block A maps ID 7 to file offset 32. Later block B maps 7 to 64
and contains duplicate ENTP records `(7,48),(7,64)`: lookup returns B,64 and
the entry pointer is B's file-buffer + 64. Re-register A: lookup returns A,32.
Register a semantic object with ID zero: it is retrievable at zero even though
registration returned false. Duck entries 5 and -2 give max 5; removing the
5 with priority zero gives max 0, writes register 27 = 0 and updates the view.

Runtime boundary for SS2: the external source-handler's **semantic payload
types** are not defined by this tail-call, and a group object's own priority
policy is not established by the manager's cache. To determine those contracts,
install a recording slot-3 source handler and a recording control-group object,
exercise a HIT source-field read and two competing group sounds, and record
argument words, pointed-to output writes, return bits, ordering and refcounts.
No claim about a synthesized callback payload or group winner is required by
the statically settled manager rules above.

### 4.2 Channel construction and source IO

CLSID `0x3B1EA5BE`; creation `0x009358AA`, construction `0x0093FE12`,
destruction `0x0093E60A`, QI `0x0093EC35`. A channel starts with zero
reference state, default play value `0x400` at `+0x1C`, no underlying source,
an owned property set, and a retained framework sound-manager service.
Its primary table is installed at `0x00AD3FC0`; secondary tables are
`0x00AD3F78`, `0x00AD3F64`, `0x00AD3F40`, `0x00AD3F30`, `0x00AD3F1C`
at object offsets `+4`, `+8`, `+0x0C`, `+0x10`, `+0x14` respectively.
The exported named-vtable index does not enumerate these tables; the inventory
below is recovered from executable table words, not that index.

| Named operation | Address | Confirmed boundary |
|---|---|---|
| CreateSnd | `0x0093F304` | obtains a backend sound and associated source interface |
| UpdatePitch | `0x0093F07E` | source pitch update; numeric law below |
| UpdateVolPan | `0x0093F136` | source volume/pan update; numeric law below |
| UpdatePatch | `0x0093F687` | patch/source refresh |
| NoteOn | `0x0093F806` | refresh patch, load then play source |
| Load | `0x0093F84D` | refresh patch and forward source Load |
| HandleNotifySetPropertyValue | `0x0093FA5B` | property notification dispatch below |

*[compat]* CreateSnd chooses the manager's default 2D or 3D source-player
class according to property `0xFFCC156F`, creates it through GZCOM using
sound IID `0x206077D2`, initializes it, and queries source IID `0x09D002D2`.
The return tests availability of that source interface, not merely whether
a sound object was allocated. For 3D playback it also queries IID
`0x306077D2` and applies head-relative/distance properties. This routine has
unchecked downstream pointers; backend/QI failure must not be advertised as
a guaranteed graceful original path.

*[compat]* After patch refresh, Load's no-source branch returns true. When a
source exists, it forwards Load and returns that result, then refreshes source state.
NoteOn refreshes the patch and invokes the channel's Load slot (14);
Load failure returns false without Play. With a source and successful Load,
it calls source Play(one payload, loop, true), where loop tests property
`0xFFF7B8DF` for nonzero. With no source it returns true without audible
playback. These branches do not guarantee safe operation with a missing backend:
UpdatePatch itself dereferences the source interface after attempting creation
(`0x0093F687`). A backend creation failure can therefore fail before the
no-source branch is reached. A replacement may reject such missing dependencies
safely *[choice]*.

#### Channel views and complete table extents (SS3)

QI `0x0093EC35` accepts exactly sound `0x206077D2` at base, 3D
`0x306077D2` at `+4`, update `0x5B34F0AF` at `+8`, registers `0xFD584467`
at `+0x0C`, `0xAA11A64F` at `+0x10`, and source `0x09D002D2` at `+0x14`.
Success writes the adjusted pointer and AddRefs the base. Other IDs, including
`1`, return false without touching the output or AddRef. Secondary QI/refcount
entries adjust back to this same base. Constructor stores confirming the six
views are `0x0093FE41–0x0093FE63`.

The following table lists every installed slot. Addresses denote folded/unnamed
entries when assigning an unrelated exported symbol name would be misleading.
They are callable entry identities, not requirements for an implementation's
internal layout. All addresses in the next table have prefix `0x00`.

| Table/view | Slots | Entries in increasing slot order |
|---|---|---|
| AD3FC0, sound | 0–9 | QI EC35, AddRef EC9F, Release ECA6; Init F937; Shutdown FA21; Play F949; Play F93E; IsPlaying F954; Stop F95C; Kill F964 (all `93xxxx`) |
| same | 10–19 | `94A5EA`, `949B47`, `93F45D`, `93F46C`, Load `93F84D`, Unload `93F06F`, IsLoaded `93FA4C`, Pause `93F96C`, Unpause `93F97F`, IsPaused `93F990` |
| same | 20–28 | GetVolume `93ED1E`, SetVolume `93ED08`, `938789`, `94A5EA`, GetPan `93ECC0`, SetPan `93ECCE`, GetFrequency `93ECE4`, SetFrequency `93ECF2`, GetOriginalFrequency `93F44E` |
| same | 29–39 | `93877E`, `93877E`, `949B47`, `949B73`, `93878E`, `93878E`, `9378BC`, `93877E`, destructor `93FDF6`, `94BC5D`, `968303` |
| same | 40–46 | NoteOn `93F806`, NoteOff/Kill `93F964`, `93FA2E` (child Kill, true if absent), `93FA3D` (child IsPlaying, false if absent), UpdatePitch `93F07E`, UpdateVolPan `93F136`, `92B33A` |
| same | 47–53 | `93F47B`, `93F4BA`, `93F4F5`, `93F547`, `93F599`, `93F5E7`, `93F635` (optional child-interface forwards described below) |
| AD3F78, +4 | 0–16 | `93F9A1`, `93F9A9`, `93F9B1` (QI/refcount thunks); `93F933` (base view); `93ED2C`, `93ED8D`, `93EDE1`, `93EE42`, `93EE96`, `93EF4B`, `949B73`, `93EFE0`, `93EFF0`, `93F012`, `93F022`, `93F044`, `93F05C` |
| AD3F64, +8 | 0–3 | `93F9B9`, `93F9C1`, `93F9C9` (QI/refcount thunks), `949B73` (folded update) |
| AD3F40, +0C | 0–8 | `93F9D1`, `93F9D9`, `93F9E1` (QI/refcount thunks); integer SetRegister `93FD3C`, integer GetRegister `93F6BA`, Clear `93F781`, Has `93F7B4`, float SetRegister `93FD99`, float GetRegister `93F71B` |
| AD3F30, +10 | 0–2 | `93F9E9`, `93F9F1`, `93F9F9` (QI/refcount thunks only) |
| AD3F1C, +14 | 0–3 | `93FA01`, `93FA09`, `93FA11` (QI/refcount thunks), `93F2F3` (retain/replace source pointer at base +2C; consumes two arguments, ignores the second, returns true) |

For primary slots 47/48, query the child for IID `0x6B0446EB` and forward
to its `+0x18` with a float / `+0x1C` with an integer respectively. Slots
49–53 query IID `0x4B79C1A9` and forward to its slots 3–7 with payload
types float, float, integer, integer, float. The queried interface is released
after the call. These are optional backend controls, not additional pitch math.

#### Channel transformations and notifications

*[compat]* UpdatePitch runs only with a child that reports loaded (`+0x40`);
otherwise it returns false. The factor is 1.0 unless float property
`0xFF3071B9` exists, in which case its value is rounded to a 32-bit float
local. If integer property `0xFF099DB2` exists, multiply its **signed** 32-bit
value by that factor in x87 arithmetic, truncate toward zero using `_ftol2`,
pass the low 32-bit result to child SetFrequency (`+0x6C`), and return the
inverse of the child's boolean result. If absent, get child GetFrequency
(`+0x68`), apply the same multiply/truncate and store the result as property
`0xFF099DB2` with notification suppressed (third argument 1); return true
without setting the child frequency on that invocation. This peculiar return
and first-use behavior are visible at `0x0093F0C5–0x0093F12B`.

*[compat]* UpdateVolPan returns true, including with no child. With a child,
property `0xFFCC156F` zero sends integer `0xFF3EC880` directly to SetPan
(`+0x64`). Nonzero instead queries the child's 3D IID and applies:
minimum distance from signed integer `0xFFAAB623` as float at `+0x30`;
maximum distance from signed integer `0xFFA6A614` as float at `+0x38`,
substituting **100000.0f only when zero**; position `(0xFFCC4D2D,
0xFF4A01D6,0xFF469820)` at `+0x14`; velocity `(0xFF444C9A,
0xFFC20061,0xFFCE9997)` at `+0x1C`. Coordinates are signed integer-to-float
conversions, not reinterpretations of float bits. Afterwards, for either mode,
multiply integer properties `0xFF90AAC8` and `0xFF53DBF3`, retain the low
32 bits as signed, divide by signed 1024 with truncation toward zero, cache
at `+0x1C`, and send to child SetVolume (`+0x54`). No clamp is applied.
Instruction evidence: `0x0093F1A2–0x0093F293`, constant `0x00AD3F18`,
and signed `IMUL/CDQ/IDIV` at `0x0093F2D5–0x0093F2DE`.

HandleNotifySetPropertyValue reads back the property; it is not passed the
new numeric value. Its complete dispatch (`0x0093FA5B–0x0093FD3B`) is:

| Property ID(s) | Action |
|---|---|
| `FF099DB2`, `FF3071B9` | UpdatePitch; return true regardless of helper result. |
| `FF3EC880`, `FF444C9A`, `FF469820`, `FF4A01D6`, `FF53DBF3`, `FF90AAC8`, `FFC20061`, `FFCC4D2D`, `FFCE9997`, `FF4C97BE` | UpdateVolPan; return its result. For `FF4C97BE`, first rebuild the property's inherited list described below. |
| `FFA6A614`, `FFAAB623` | UpdateVolPan; discard result and return true. |
| `FF0AD592` | Test primary slot 43 (child IsPlaying); if false write zero back to this property with the ordinary notification argument 0; return true. This is not a patch-load retry. |
| `FFB36659` | If child exists, nonzero calls Pause(true), zero Unpause; return true. |
| `FFAEC905` | Read integer property, discard that value, call this channel's source-view slot 3 with `(null,2)`; return true (`0x0093FC71–0x0093FC83`). |
| `FF0150F2`, `FF86FE79`, `FF734343`, `FFD3D39B` | Read float and call primary slot 47, 53, 49, 50 respectively. First returns true regardless; others return the forwarded result. |
| `FFA42951`, `FFBDB196` | Read integer and return primary slot 52, 51 respectively. |
| all others | Return true, no channel-specific action. In particular `FFCC156F` is read during refresh but has no notification case. |

For `FF4C97BE`, clear the retained property-set list, retain the notifying set,
then follow its nonzero `FF4C97BE` resource reference: persist key has type
`0xEBFEE345`, group `0xFC45CAF7`, instance equal to that property, queried
as IID `0xEBB10CDD`. Append each successfully resolved set and repeat until
the property is missing/zero or resolution returns null. There is no visited
set or cycle bound; do not promise cycle recovery (`0x0093FB5B–0x0093FC12`).

*[compat]* UpdatePatch (`0x0093F687–0x0093F6B9`) attempts CreateSnd only
if child at `+0x24` is null, then unconditionally invokes source at `+0x28`,
slot 3 with `(retained-source-at-+0x2C,2)`, UpdateVolPan, then UpdatePitch,
and returns true. There is no rollback or null guard after failed creation/QI;
it does not discard and recreate an already present child. The failure path
can fault before returning. An implementation can return failure safely as
already permitted above; it must not claim the original retries transactionally.

Worked cases: a loaded child, explicit frequency 22051 and factor 0.5f receive
SetFrequency(11025); a child returning true causes UpdatePitch to return false.
Without the explicit frequency, child GetFrequency() = 22051 instead stores
11025 in `FF099DB2`, suppresses notification, and makes no SetFrequency call
on that first refresh. Volume inputs -3 and 512 produce -1 (not -2), while
1024 and 512 produce 512. In 3D mode a zero maximum becomes 100000.0f,
but -1 remains -1.0f. A failed CreateSnd followed by a null source reaches
an unchecked call, not a successful silent channel; use a failure return in
a safe replacement and test that no later volume/pitch calls occur *[choice]*.

## 5. `cHitIniFile` and `cHitPropertySetTrackResource`

### 5.1 INI adapter

`cHitIniFile` is CLSID `0x3D802D67`. Constructors `0x0093638E` and
`0x009363F5`, Create `0x00936480`, destructors `0x00936458`/`0x009364DE`.
It combines the existing INI-file implementation with the HIT initialization
interface at `+0x3C`, table `0x00AD2394` (constructor store `0x00936417`). That interface's slots 0–7 are
QueryInterface, AddRef, Release, EnumDefaultEntries, InitDBSegments,
EnumEntries thunk, SetFilename, ReadEntry thunk. The inherited INI table
`0x00AD23B8` has slots 0–19: destructor, GetPath, SetPath, folded entry,
SetFile, Close, ReadEntry, ReadNumber, two ReadBinaryEntry overloads,
WriteEntry, WriteNumber, WriteBinaryEntry, EnumEntries, EnumSections,
SectionExists, SetLeaveFileOpenBetweenOperations, LoadSectionNames,
GetFileLine, Open. ReadEntry `0x00936249` and EnumEntries `0x0093624E`
forward to the INI implementation; this is not a second INI syntax.

*[compat]* EnumDefaultEntries `0x0093756A` reads `[Config]` values
`DefaultSndDataSourcePlayer2DClassId` and `DefaultSndDataSourcePlayer3DClassId`
and applies each only when the entry and manager service exist. Values are
converted with the framework unsigned-number converter. The section
`SndFileDefaultDataSourceClassIds` supplies extension/class mappings to the
sound system. Missing service skips the corresponding configuration work;
it does not manufacture a backend.

Initialization helpers and callbacks:

| Operation | Address |
|---|---|
| ProcessTrackDataBlockEntry; InitTrackDataBlocks | `0x00936298`; `0x00936D7B` |
| ProcessDBSegmentsEntryCreate; ProcessDBSegmentsEntryInit | `0x009362C9`; `0x0093657D` |
| ProcessDBSegmentDirectoryFilesEntry | `0x00936707` |
| InitHitManTypeGroups | `0x009369F6` |
| InitDataSourceCreators; InitSndCreators | `0x00936B57`; `0x00936BCC` |
| ProcessPackedDBSegmentEntry; InitPackedDBSegments | `0x00936E6D`; `0x009374A4` |
| InitDBSegments; InitResourceFactories | `0x00937159`; `0x009373D6` |
| SetFilename | `0x009364A1` |

*[compat]* `HitManTypeGroups` entries supply multiple numeric type/group
associations; they are data, not a hard-coded audio resource-type switch.
Directory extension binding and the TLO association are already specified
in [audio.md §10](audio.md#10-sound-data).

#### Initialization grammar and order (SS4)

*[compat]* Enumeration uses the inherited INI reader, not a HIT-specific lexer.
Section lookup lowercases the requested section name. Within a section it reads
lines in file order, left-trims them, skips empty lines and lines beginning `;`,
and stops at the next line beginning `[`. It splits at the first `=`, right-trims
the key and trims both ends of the value; only nonempty key/value pairs reach
the callback. It does not strip an inline semicolon from the value. Callbacks
are invoked after collecting the section, in that same order; callback results
do not stop enumeration (`0x0091D91C`).
No quoting or escaping layer is added by the HIT helpers.

Except for the direct creator callbacks below, each section is first collected
in a map from **converted unsigned key** to string value by `0x009368C0`:
later entries with the same converted key replace earlier values. Processing
then follows ascending unsigned key order, not file order. Thus `16`, `0x10`
and `10A?` are not three independent string identifiers (the last converts to
hexadecimal `0x10A`). Distinct malformed keys converting to zero also collide.
This conversion is used for numeric values as well: `RZConvertToUint32A`
(`0x0090FE93`) chooses base 16 for a leading `0x`/`0X`, or when its initial
scan of decimal digits encounters `A–F`/`a–f`; otherwise it chooses base 10.
It then calls unsigned 32-bit `strtoul`, without checking an end pointer or
error status (`0x009F038B`, `0x009F01CC`). A nonnumeric prefix gives zero,
trailing junk is accepted, a leading minus negates the magnitude modulo
`2^32`, and positive overflow gives `0xFFFFFFFF`. For example `12tail` is 12,
`12AB` is `0x12AB`, `-1` is `0xFFFFFFFF`, and `xyz` is zero. The base-selection
scan does not skip a leading space/sign: ` AB` is therefore zero in base 10.
There is no separate per-entry conversion-failure return.

`SplitToken` here means: find the first literal delimiter, return the prefix
and remove prefix plus delimiter from the mutable string. If absent, return
an empty token and leave the string unchanged (`0x0090E98C`). No token trim
is implied. Comma loops stop on an empty returned token, then process the
whole remainder once; leading/repeated commas can therefore terminate a list
early rather than being ignored. A semicolon inside a DB entry is structural.

| Stage, in EnumDefaultEntries order | Section/key and grammar | Action |
|---|---|---|
| 1 | `[Config]`, the two default-player class keys above | Set 2D then 3D class on the sound manager when ReadEntry succeeds. |
| 2 | `[SndFileDefaultDataSourceClassIds]`, numeric ordering key = `class,extension` | Split once at comma; convert the prefix and pass the entire remainder as the extension to sound-system slot 23 (`+0x5C`). Without a comma the class token is empty (zero) and the whole value is the extension. |
| 3 | `[PackedDbSegments]`, numeric group = `class;covered-section;path` | Convert class from the start of the original value; extract the middle token as a section name, trim the final path, expand its leading `%DataDirectory%`, and test file existence. If present, create IID `0x286D4A6A`, query `0x656B8F0C`, initialize with `(group,path,false)`, apply `(true,false)` at `+0x18`, add to HIT and persist managers, then collect numeric keys from covered-section into the adapter's persistent exclusion set. Missing files are skipped successfully; COM creation failure returns false locally. |
| 4a | `[Config] AllowTextAsKey` | When present, compare the value with `true` using the string comparison helper and save the boolean (initial default false). This flag is passed to new directory segments at `+0x40`. |
| 4b | `[DBSegments]`, group = `class;path` | First pass creates all nonexcluded groups, using class converted from the beginning of the value, IID `0x856C8CF7`, calls `+0x3C` with `(true,true)`, applies AllowTextAsKey, and registers the segment with HIT. |
| 4c | `[DBSegmentDirectoryFiles]`, group = `type:extension[,type:extension…]` | For each nonexcluded group, find IID `0x856C8CF7`, get its directory interface at `+0x30`, bind each converted type to its extension at directory `+0x0C`, and associate that type with the group at HIT slot 46. The final comma-free remainder is processed too. |
| 4d | `[DBSegments]`, same entries | Second pass initializes each nonexcluded segment, after discarding `class;` and trimming the remainder; expand a leading `%DataDirectory%`, find IID `0x656B8F0C`, call `(group,path,true)` at `+0x0C`, and on success add it to persist manager `+0x70`. |
| 5 | `[TrackDataBlocks]`, numeric ordering key = relative filename | Prefix the sound-system data-directory string and call HIT LoadHitFile (slot 9); the map key does not become a track ID. |
| 6 | `[HitManTypeGroups]`, type = `group[,group…]` | Register every converted group for that type, including the final remainder, at HIT slot 46. |
| 7a | `[SndDataSourceCreators]`, subsection name = `class,creator-ID` | Direct callback, in file order: create IID `0x9B75709E`, register the creator and converted creator-ID at sound-manager `+0x64`, and save creator-ID → subsection-name in a shared map. Equal creator-IDs replace the saved subsection name. |
| 7b | Each saved creator subsection, numeric sound-ID = definition string | Visit saved creator-IDs in ascending order, query creator IID `0x9B75709E` through sound-manager `+0x70`, and pass `(sound-ID,definition-string)` to creator `+0x10`. Values are not comma-split here. |
| 8 | `[SndCreators]`, class = optional definition-section name | Split at first comma. With a comma, use the prefix as the section and ignore the suffix; without one, use the entire value as the section. Create IID `0x3AF9866F`, register at sound-manager `+0x3C`, then enumerate that section if nonempty. Each entry converts its key to a sound-ID and invokes the creator's IID `0x1B6C8B18`, slot 3, with `(ID,value,0,0)`. Direct definition callbacks preserve duplicate entries. |
| 9 | `[ResourceFactories]`, resource-type = factory-class | Convert the whole value and call persist-manager `+0x4C` with `(factory-class,resource-type,0)`. |

Evidence: helpers `0x00936298–0x00937569`, creator callbacks
`0x00936253`, `0x009364FA`, `0x009368ED`; delimiter bytes at `0x00A99F14`
(comma), `0x00A870D0` (semicolon), `0x00AA0424` (colon). Stage 4 is the
interface's slot 4, not a second call to EnumDefaultEntries. The exclusion set
is not cleared at the beginning of these helpers. The data-directory expansion
removes exactly the first 15 characters `%DataDirectory%` before prefixing.

*[compat]* There is no aggregate rollback or all-stages success conjunction.
The calls at `0x00937762–0x00937791` ignore earlier returns; the final return
is InitResourceFactories' result, which is unconditionally true on ordinary
return (`0x009374A0`), even when the persist service is absent. Individual
segment failures leave earlier registrations installed. Several paths query
an interface and immediately dereference it without testing success (including
the directory/creator callbacks); malformed dependencies can crash before
the nominal successful return. Safe rejection is an implementation choice.
HitManTypeGroups likewise requires the HIT service when entries exist; missing
services are only skipped where an explicit check exists.

Worked case: `[TrackDataBlocks]` contains, in order, `16=a.hit`, `2=b.hit`,
`0x10=c.hit`; stage 5 attempts `data-directory + b.hit`, then
`data-directory + c.hit`, and never attempts `a.hit`. If loading `b.hit`
fails, `c.hit` is still attempted and stage 9 is still reached; an absent
ResourceFactories section still yields true. A directory value
`0x2026960B:wav,0x00000001:bin` performs two bindings and two type/group
associations. These cases test replacement, unsigned ordering and continuation.

Runtime boundary: repeated **section headers**, locale-dependent comparison of
AllowTextAsKey, and missing-backend crash details are inherited dependencies,
not additional HIT grammar. To settle them, enumerate an INI containing two
identically named sections, compare `true`/`TRUE` under the target locale, and
record which callbacks and `+0x40` flag occur. The rules above do not require
that observation for well-formed, uniquely headed configuration.

### 5.2 Track property resource

The constructors `0x009396D9` and `0x00939750` build a property set and copy a
supplied three-word resource key; neither creates an anonymous track. Key setters are
SetKey `0x009377AF`, SetType `0x0094052A`, SetInstance `0x00940536`;
GetKey is `0x00940512`. QI/Release are `0x00939796`/`0x009397CA`,
GetRefCount `0x00939792`, destructor `0x00939727`, assignment `0x00939FFC`,
and CreatePrivateCopy `0x00939D62`.

Primary property table `0x00AD2898`: slots 0–2 QI/AddRef/Release;
3–9 SetValue, ClearValue, ClearAll, FirstKeyAndValue, NextKeyAndValue,
Value, HasValue; 10 folded count getter; 11 destructor; 12–13 folded
Read entries; 14–15 unnamed entries; 16–18 folded debug entries.
Resource interface at object `+0x18`, table `0x00AD285C`: slots 0–2 QI/AddRef/Release thunks;
3 GetKey, 4 SetKey, 5–7 folded key-component accessors,
8 SetType, 9 folded component setter, 10 SetInstance, 11 GetRefCount,
12–14 unnamed resource operations. Constructor `0x009396D9` installs these
two tables (`0x009396EB–0x009396F1`). Constructor `0x00939750`, also used by
CreatePrivateCopy, installs primary `0x00AD2920` and resource `0x00AD28E4`
(`0x0093975F–0x00939765`). Its primary slots 12/13/14 are ReadLegacyFormat,
Write_OldFormat and the XML-write thunk `0x00941794`; slots 15–17 are debug
entries. The earlier primary table's slots 12/13 both enter the generic
serializer reader, 14 writes XML and 15 requests binary serialization
(`0x00941636`, `0x0094164D`). The resource key is at full-object
`+0x1C/+0x20/+0x24` (type/group/instance); resource methods receive the
`+0x18` interface, so their key offsets are `+4/+8/+0xC`.

*[compat]* Write `0x0093FF4A` selects an output path by resource type:
`0x0B8D821A` selects the old path, `0x0B9EB87E` the other path;
other types report false. Read `0x00940DFF` installs the resource instance
as property `0xFF6F9546`, then selects legacy input for `0x0B8D821A`
and the property serializer for other types. This asymmetry is intentional
to the observed boundary; do not reject every nonlegacy read solely because
Write would reject its type. The generic serializer, including the binary
header, XML grammar and partial-read behavior, is specified in
[property-sets.md §§3–5](property-sets.md#3-reading-a-record-cgzpropertysetserializertreadstream-set-0x009408a5-compat).
It requires serial-record IID `0xC56B8F08`, a successful initial u32 read and
rewind to zero. `0x2BEA4862` and `0xCBE750E0` select its binary reader
(the latter subsequently fails that reader's magic check); all other first
words select XML. The normal nonlegacy Write path selects **XML**, through
`0x00941794 → 0x00941636 → 0x00940973`, binary flag zero. It does not promise
binary output merely because the resource type is `0x0B9EB87E`.

#### 5.2.1 Legacy text and record framing (SS5)

*[compat]* The legacy writer (`0x00940E5C`) emits `TKDT,2,,`, the following
27 fields in order, a comma after every field, and `ETKD`. Missing properties
emit empty fields. All values are u32 property values; decimal output treats
the word as signed (`%ld`), and hex output is `0x%08lx`. Fields 1, 2, 9,
13–16 use hex; the others use decimal. Constants/field stores are at
`0x00940F05–0x0094158D`; format strings/trailer at
`0x00AD448C/0x00AD4494/0x00AD44A0`. The writer submits exactly the string
length, without a NUL, through stream slot 17 (`+0x44`), ignores that call's
success and returns true (`0x009415D3–0x00941635`).

| Fields | Property IDs in positional order |
|---|---|
| 1–5 | `0xFFAEC905`, `0xFF5F15E1`, `0xFF4DCD0D`, `0xFF050A94`, `0xFFB0333D` |
| 6–10 | `0xFF706616`, `0xFFCC156F`, `0xFFCC3B29`, `0xFF891EAA`, `0xFFAE9DDB` |
| 11–15 | `0xFFF7B8DF`, `0xFF90AAC8`, `0xFF3C2160`, `0xFFD7EE4D`, `0xFF59FDD8` |
| 16–21 | `0xFF2D7CD1`, `0xFF0246FE`, `0xFFFF4E79`, `0xFF595FFD`, `0xFF8DCFD8`, `0xFF69510B` |
| 22–27 | `0xFF099DB2`, `0xFF53DBF3`, `0xFF3EC880`, `0xFF47F685`, `0xFFCAA344`, `0xFF303B1B` |

*[compat]* ReadLegacyFormat (`0x00941930`) obtains serial-record size S,
consumes a u32 H, and, if H equals S−4, consumes a replacement H and reduces
S by four. It then reads S−4 bytes into its text buffer. If H is
`0x544B4432` (on-disk bytes `32 44 4B 54`, “2DKT”), it skips **eight further
bytes of that buffer** before tokenizing; otherwise it starts at buffer
offset zero (`0x00941963–0x009419C1`). Thus an ordinary raw `TKDT,2,,…`
record has its first four bytes consumed before the comma tokenizer sees
`,2,,…`. The optional length word and special eight-byte skip are part of
the stream contract; do not infer framing from a TKDT prefix search.

The tokenizer splits on literal comma, preserves empty fields and has no
quote, backslash or CSV escaping. A field needs a following comma; an
undelimited remainder produces an empty token (`0x0090E98C–0x0090EA41`).
The first token is discarded without validating the tag; the second is
converted to the version word; the third is an unused name. Version 0 omits
field 6; versions 0/1 omit fields 22–27; every unsigned version greater than
1 uses all 27 fields. There is no maximum-version rejection or ETKD check.
Nonempty numeric fields use RZConvertToUint32A (`0x0090FE93`): initial
`0x`/`0X` selects base 16; otherwise encountering A–F/a–f in the initial
digit/hex-letter run also selects base 16, and the default is decimal.
No end-pointer or conversion-success check is made here.

The reader overlays the set; it does not clear old properties for empty
fields. Nonempty field 1 equal to the object's instance word causes false
before setting that field or any later legacy field
(`0x00941AC4–0x00941B2E`). The enclosing Read has already installed
`0xFF6F9546 = instance`, and does not roll that back. Field 9 is always
written: default `0x7DCDED3D` if the parsed field 7 is nonzero, else
`0x7DCDED2D`; only a nonempty, nonzero field-9 conversion overrides it
(`0x00941DF8–0x00941E6E`). Empty field 7 counts as zero for this decision,
even if a preexisting 3D property remains in the set. Other empty fields
leave their preexisting values untouched. Ordinary completion returns true.

Legacy QI/read/length failures have no checked false exit: the routine goes
on using its buffer and ignores the record calls' results. Tiny/truncated
records can underflow S−4 or leave data unavailable. These are invalid-input
preconditions, not a demonstrated recoverable error. To establish a stable
verdict for such inputs requires calling the original reader with a mock
serial record that denies QI, then with S=0/3 and with short reads, recording
return/crash, stream requests and properties after each call. No engine
decoder is required to emulate an invalid memory access.

#### 5.2.2 Assignment and private ownership (SS5)

*[compat]* Assignment copies the source reference-count word (`+4`), map
storage, iteration-cursor word (`+0x14`) and all three key words; it leaves
the destination interface tables installed (`0x00939FFC–0x0093A034`).
The map has its own nodes/byte blocks, but owning members inside those
blocks are copied byte for byte; see
[property-sets.md §2](property-sets.md#2-the-property-set-cgzpropertysett-iid-0xebb10cdd-compat)
for the original string-aliasing quirk and the chosen safe value-copy rule.
The copied cursor is not rebased onto the new map. Assignment is not a
reference to the original resource and does not AddRef it.

CreatePrivateCopy is called on the resource view. It allocates 40 bytes,
constructs with the source key using `0x00939750`, assigns from the source
full object (interface minus `0x18`), and returns the new object's QI result
for the caller's requested IID/output (`0x00939D62–0x00939DA0`). There is no
local Release after QI and no null-allocation recovery before assignment.
For a supported IID, QI increments the copied count: a source count N gives
the new object count N+1. An unsupported IID returns the usual QI failure;
this wrapper does not destroy the allocated copy on that path.

**Worked cases (SS5).** A version-0 record with field 7 nonzero and field 9
empty sets sem `0x7DCDED3D`, skips field 6 entirely and consumes no fields
22–27. With instance 42, a nonempty field 1 of `42` returns false while
retaining the enclosing Read's `0xFF6F9546=42`. Writing a set containing
only field 12 equal to `0xFFFFFFFF` emits `-1,` at position 12 and empty
tokens at the other positions; a stream refusing that write still gets a
true writer result. Copying a set with scalar field 12=512 yields independent
scalar storage: changing the copy's field 12 to 256 leaves the original at
512, while its key remains the copied type/group/instance.
See [audio.md §10](audio.md#10-sound-data) for resource-factory selection.

## 6. `cFreshPlayer` and `cFreshCellPlayer`

### 6.1 Score player

Create `0x009456A4`, constructor `0x009489AE`, QI `0x00946FE6`, Release
`0x00947004`, destructors `0x00948C54`/`0x00948C98`. CLSID `0x3C1CF59E`.
Constructor state includes no score/timer, clear initialized/playing flags,
volume `0x400`, empty cell-player storage, seeded private randomness and
spatial/division configuration. The 4096 cell-pointer entries use a row
stride of 64; this is storage capacity, **not** permission to read arbitrary
64×64 scores. Init does not check dimensions against that capacity; see
§6.1.1 for valid-input preconditions.

Primary table `0x00AD4B48`:

| Slots | Operations, in slot order |
|---|---|
| 0–10 | QI, AddRef, Release, Init(resource), Init(), LoadScore, GetScore, Shutdown, Play(), IsPlaying, Stop |
| 11–20 | SetVolume, FadeVolume, SetYPos, SetXPos, SetZoom, SetPlayPercent, SetTempo, GetYPos, GetXPos, GetZoom |
| 21–29 | SetManual, GetFirstSelectableRow, GetLastSelectableRow, GetFirstSelectableColumn, GetLastSelectableColumn, GetCellPlayer, Reseed, SetHeadRelative, SetMinMaxDistance |
| 30–37 | SetNumDivisionsX, SetNumDivisionsZ, SetDivisionEnabled, SetDivision3DPosition, SetKey, GetKey, unnamed `0x00948568`, destructor |
| 38–46 | three Init overloads, Update, Play(volume), StopFade, folded SetVolumeNoLock-named entry, folded SetZPos-named entry, GetZPos |
| 47–54 | GetDataDir, SetSelectionAreaMaxDistances, GetSelectionAreaMaxDistances, SetStressLevel, SetPause, UpdateListener, StopNoLock, unnamed `0x00948719` |
| 55–64 | GetCellToAdd, GetCellToSubtract, SubtractCell, ToggleCell, ProbOfCellStarting, NumCellsBusy, NumCellsDying, NumCellsPlaying, ShowUsage, GetMinCellsPlaying |

*[compat]* Init(resource) obtains a persist record, loads a score through a
record-as-file adapter, and initializes playback only after successful score
load (`0x00947C38`). Init() builds cell players for score cells, binds them
to the owner and imports score configuration (`0x0094701E`). The overload
contracts are below. LoadScore is `0x00948C11`.

*[compat]* Play() returns true even if not initialized. When initialized
and previously stopped, it derives player settings from the score, starts
a timer, sets score starting coordinates, marks playing, and registers the
update interface with sound-system period `0xFA` (250) if that service exists
(`0x00947DB8`). Consequently, success alone cannot verify audible music.
SetZoom and SetPlayPercent also return true when uninitialized, but only
store the requested value when initialized (`0x0094776D`, `0x0094777F`).
Update(volume) forwards the volume then invokes Play() (`0x00947150`);
this method is not the periodic cell-selection algorithm.

State changes use a shared freshness critical section. Scheduling enters
through the update interface and callbacks; no additional SC4 message IDs
are established. The rules below settle SS6, supplementing the score/cell
fields, budgets and division placement in [freshness.md](freshness.md).

#### 6.1.1 Initialization and lifetime (SS6)

| Slot/address | Input and behavior |
|---|---|
| 3 / `0x00947C38` | Init(resource key): copies the key, obtains the persist manager, segment and record (open flag 1), queries serial-record IID `0xC56B8F08`, loads a new score through the record-as-file adapter, closes/releases adapter/record/segment, then calls Init() only on load success. Missing services, open/QI failure or failed load return false. |
| 4 / `0x0094701E` | Init(): once only, requires an existing score; imports volume, budget bases and half-extents, sets listener coordinates to 1 and remembered coordinates to −1, creates/binds players for nonempty cells, counts selectable cells, then sets initialized. Repetition returns true without rebuilding. |
| 38 / `0x00948CB4` | Init(full path string): splits directory and filename/extension and calls slot 39. |
| 39 / `0x00948D5F` | Init(directory string, filename string): allocates a score, concatenates the strings, attempts path loading, then calls slot 40 **without testing the load result** (`0x00948DDE–0x00948DEB`). |
| 40 / `0x00948E0B` | Init(score pointer, directory string): stores the pointer directly, copies the directory and calls Init(). No score AddRef or clone. |

The score is owned and deleted by Shutdown. Replacing it through Init does
not dispose the old score or rebuild an already initialized grid. Failed
resource loading can leave an allocated score stored while initialized is
false; Shutdown's score deletion is conditional on initialized.

Neither the parser (`0x00946039`) nor Init checks dimensions against 64.
Storage-safe rows/columns are at most 64; random selection also requires
both positive. Oversized dimensions can overrun storage; zero dimensions
can divide by zero during selection. These are input preconditions, not
demonstrated false-return cases. Some candidate scans also dereference
player entries without a null check; arbitrary sparse grids are unsafe.

Stop locks and calls StopNoLock. When initialized and playing, StopNoLock
(`0x00947E90`) unregisters the player's `+4` update view, ends the volume
fade, stops/deletes the beat timer, calls Stop on each **busy** cell in score
row/column order, then clears playing. Cells may remain fading. Shutdown
(`0x00947D28`) first ends a fade, locks, calls Stop, deletes the score, then
destroys/clears nonnull players across all 4096 slots. It finally clears
playing/initialized and releases the external-controller reference.

#### 6.1.2 Selection and random state (SS6)

*[compat]* Use slot behavior, not the exported labels: slot 54
`0x00948719` selects the best start; slot 55 `0x00947220` selects a cell to
subtract; slot 56 `0x00947519` remembers row/column and stops a busy player.
Slot 58 `0x00948573` returns an integer start score. Slot 59 `0x009477A6`
counts nonnull busy players over the grid (`0x009477D1–0x009477E1`);
it is not a probability.

The subtraction selector uses byte 1 of a private random u32 modulo rows
and byte 0 modulo columns as a scan origin. Rows start there and wrap;
columns start at the same selected column and wrap on each row. It first
takes a playing, unmarked cell outside the selection window. Otherwise it
gathers playing unmarked cells inside the window, excludes class 20 and
chooses `next private draw modulo candidate count`; none returns false.

The best-start selector uses byte 1 modulo `(2×halfHeight+1)` minus
halfHeight to offset the listener row, and byte 0/halfWidth for the column.
It clamps the origin to the grid and scans the window in the same wrap
order. Only a strictly larger score replaces the best: ties keep the first
visited cell (`0x0094876A–0x009489AD`). Success means best score nonzero.

The scorer rejects absent cells, dead/busy players, cells outside the
window, the last subtracted cell, a nonzero group held by another busy
unmarked player, or a linked busy unmarked player. Cell class is clamped
in place to 0–20. **Class 0 returns zero**, confirmed by
`0x009486B9 → 0x00948710`; this supersedes freshness.md's older undefined
return caveat. Class 20 scores 101 for a matching **nonzero** requested
group, otherwise 100. Other classes score 2 for that nonzero group match;
otherwise they score 1 exactly when C `rand()` is less than signed integer
`T[class] / selectableCellCount`, else zero. A zero count is first replaced
with 10 (`0x009486D3–0x0094870C`). T at `0x00B16F30` is:

| Classes | T values, in order |
|---|---|
| 0–6 | 0, 600, 1500, 3000, 7500, 12000, 15000 |
| 7–13 | 18000, 21000, 25500, 32767, 36000, 45000, 58000 |
| 14–20 | 67500, 90000, 120000, 210000, 300000, 40000000, 40000000 |

C rand ranges over 0–32767: a positive threshold H accepts exactly the
draws below min(H,32768). This is separate from the private cRZRandom
used for scan origins/divisions. Reseed(seed) seeds only the private
generator; it resets beat-in-bar/next/origin to 0/1/0, budget bases to 0/4,
remembered coordinates to zero, recounts and Stop/Play restarts a playing
player (`0x009479D4–0x00947A29`). It does not reseed C rand.

For positive dimension D, SetXPos/SetYPos use signed 32-bit
`p = D×v / 1000`, truncating toward zero. Block mode then snaps to
`(p/(2×half+1))×(2×half+1)+half`, again using signed division; apply upper
selection bound D−half−1, lower bound half and grid bounds. Input v is not
first clamped to 0–1000. X changes column; Y changes row; both recount
(`0x009484A0`, `0x00948504`). The group-count scratch array described in
freshness.md §4.4 has 32 entries and no general bound check on group IDs.

#### 6.1.3 Beat clock and fades (SS6)

*[compat]* Beat due time is signed 32-bit
`((beat−origin)×60000)/tempo`, truncating toward zero; zero tempo returns
−1 (`0x009475FB–0x00947619`). Update compares due time as unsigned, so
zero tempo normally suppresses beat work. Each due beat performs subtract/
add decisions on bar beats bpb−1 and bpb−6, updates every nonnull cell, then
advances counters. The manual/pause/start-switch gates, active-cell budget
and division choice follow freshness.md §4.6. Strictly above 86,400,000 ms,
all three counters become bpb and the timer restarts. SetTempo changes tempo
only while playing and when different, restarts the timer and sets next
beat to origin plus current beat-in-bar (`0x00948420`).

FadeVolume(durationMs, target, stopAtEnd) returns true without work unless
initialized and playing. It stores signed step `(target−current)/20`, target
and stop flag. Both that division and timer period `durationMs/20` truncate
toward zero (`0x00948371–0x00948379`, `0x009483EE–0x009483FD`). Zero step
sets target immediately and calls Stop if requested; this path does not
itself cancel an already active fade. Nonzero step creates a timer only
when none is active, so retargeting preserves the old timer period.

Each callback skips work while the freshness lock is held. Otherwise it
adds step, clamps volume to 0–1024 and stores it. A negative step reaching/
passing target ends the fade and calls StopNoLock if requested; a positive
step reaching/passing target ends it without stopping. It does not snap
an overshoot to target (`0x0094761C`). Out-of-range targets can leave a
clamped fade running indefinitely.

**Worked cases (SS6).** D=9, half=1, block mode, v=500 stores coordinate 4.
Two eligible class-20 cells tie at 100 for requested group zero: the first
visited wins. Class 1 with 10 selectable cells accepts C draws 0–59 and
rejects 60. Private reseeding does not replay those C draws. A 400-ms fade
from 100 to 145 has step 2 and period 20 ms: callback 23 stores 146 and ends
it. A fade from 100 to 110 has step zero and sets 110 immediately. Stop
unregisters before cell Stop calls; Shutdown deletes the score before the
cell players.

### 6.2 Cell player and volume arithmetic

Constructor `0x009458D6` binds an owner, initializes no cell/source, state
zero and default local volume `0x400`. QI `0x009457FB`, Release
`0x009457E1`, destructor `0x00945CD2`. Primary table `0x00AD48A0`:

| Slots | Operations, in slot order |
|---|---|
| 0–8 | QI, AddRef, Release, folded owner getter, SetCell, Start, Update, Stop, Kill |
| 9–17 | SetVol, IsPlaying, IsBusy, IsDead, SetDead, AutoPan, SetHeadRelative, Set3DPosition, SetMinMaxDistance |
| 18–22 | unnamed `0x00945C64`, GetHeadRelative, Get3DPosX, Get3DPosY, Get3DPosZ |

*[compat]* IsPlaying tests state exactly 2; IsBusy tests state nonzero.
State 3 is the source-fading/stopping state. Kill drops the owned sound,
returns state to zero, clears the stop flag and returns true
(`0x00945949`, `0x00945953`, `0x00945A70`). SetCell stores the cell pointer
(`0x00945E70`); it is not itself a Start operation.

*[compat]* Stop without an external controller unregisters a source update
interface if available. A playing cell with zero configured fade-out calls
Kill; a playing cell with nonzero fade-out enters state 3 and requests a
fade to zero. Already-stopping state is preserved. Other states become idle.
With an external controller the call uses StopCellExternal instead
(`0x00945D1A`, `0x00945C20`). Update polls the source/controller and can
return a finished cell to idle (`0x009459BF`). The complete external-controller
protocol and special loop/countdown branch are specified in §6.2.2 below.

*[compat]* UpdateScaledVol `0x00945820` uses signed integer arithmetic,
not a single floating-point product. Let `C` be cell word `+0x24`, `A`
cell-player word `+0x30`, `P` owner's volume word `+0x14`, `S` the
score word `+8`, and `L` cell-player local volume `+0x34`.
Let `mul32` mean low 32 bits of multiplication, interpreted signed, and
`trunc` mean signed division toward zero. Then:

- `intermediate = trunc(mul32(P, mul32(C, A)) / 1048576)`.
- `scaled = trunc(mul32(mul32(S, L), intermediate) / 1048576)`.

The intermediate result is stored before the second stage replaces it.
Instructions `0x00945826–0x00945856` establish the multiply order, 32-bit
widths and both signed `IDIV`s. No clamp appears in this routine. This
formula specifies field inputs without guessing their editor-facing names.
SetVol caches the requested local volume and forwards the recalculated
scaled volume to an existing sound; without a sound it succeeds without
forwarding (`0x00945A89`). Other named helpers are AutoPan `0x00945AAF`,
FilenameHasGuid `0x00945F38`, StartCellExternal `0x0094680F`, Start
`0x00946C2C`, UpdateScaledVol `0x00945820`; their policies follow.

#### 6.2.1 Start and spatial policy (SS7)

*[compat]* Start requires the sound-system service even with an external
controller. It returns false if the service is absent, the player is dead
or busy; otherwise it clears stop mark `+0x2D` and takes the external path
when controller `+0x54` is nonnull (`0x00946C2C`). A valid bound cell is a
precondition: there is no null-cell or sound-manager recovery.

FilenameHasGuid (`0x00945F38`) means **any run of eight consecutive ASCII
hex digits anywhere in the name**. It does not validate a suffix or GUID
syntax. On a match, Start splits at the first literal dot and converts the
last eight characters of the preceding token as hex, stores the result in
cell `+8`, and requests that sound ID (manager slot 18, IID `0x206077D2`).
Otherwise it uses cRZPath::GetFileName and manager slot 17. A failed manager
call or successful call yielding a null sound returns false. Because the
comma-independent SplitToken helper returns empty when the delimiter is
absent, an eight-digit name **without a dot** does not reliably yield those
digits as the ID; no special suffix parser repairs it.

With a sound, Start optionally queries `0xFD584467` and sets register
`0x1C` to 1. It queries 3D IID `0x306077D2`, then immediately dereferences
that view without testing QI success. It sets head-relative, min/max
distance and the stored position when position-set `+0x3C` is true. Distances
are unsigned u32 converted to f32; coordinates are signed integers converted
to f32 (`0x00946E20–0x00946E79`). AutoPan is a separate slot-14 operation;
Start does not call it.

Cell fade-in selector `+0x34` zero starts at the cached scaled volume with
the loop flag and final argument 1. Nonzero starts at zero and requests a
fade from zero to that cached volume using cell `+0x38`, final argument 1.
These playback results are ignored. State becomes 2; a successful query of
update IID `0x5B34F0AF` registers it at period 250. The random-volume factor
is then reset to 1024, or reduced by `abs(signed private draw) remainder
256` when random volume is enabled; scaled volume is recomputed and sent.
Looping loads counter `+0x28` from cell `+0x48`. Successful setup returns
true regardless of Play/fade/registration results. Temporary queried views
are released; the sound reference remains owned at `+0x1C`.

Pitch spread uses signed remainder: for nonzero ordered low/high endpoints,
`k = signed(private draw) remainder (high−low+1) + low`, with remainder
truncated toward zero (`0x00946F67–0x00946F69`). The source frequency is
multiplied by double table P[k] for k≥0 and divided by P[−k] for k<0, then
converted to an integer before setting frequency (`0x00946F7B–0x00946FA4`).
P is the semitone table at `0x00B16EB0`; only indices 0–12 are its intended
range. Signed negative draws can put k below low; there is no bounds clamp.

AutoPan does nothing without a cell. It computes x/z from signed
`(pan−512)×(1/512)` in double precision, then multiplies by double 100 and
rounds to f32 for the 3D position call (`0x00945AD5–0x00945C1F`, double
constant `0x00AD4900`). Without random pan there is no coordinate clamp.
With both axes centered and random pan, one private draw's low two bits
choose `(0,1)`, `(0,−1)`, `(−1,0)`, `(1,0)`. With only one centered axis,
that axis becomes −1 or +1 from a draw's low bit. Debug byte `0x00B6349D`
forces random pan on in the cell; `0x00B6349C` enables forced quadrant
`0x00B634A0`. The 3D sound/QI is assumed valid here too.

#### 6.2.2 External controller and beat transitions (SS7)

*[compat]* StartCellExternal (`0x0094680F`) sets state 2 and computes factor,
scaled volume, loop counter and pitch **before** asking the controller to
start. The name-derived ID is used only if nonzero; otherwise it uses
`CRC24(cRZPath::FileName(name), seed 0) OR 0xFF000000`. Controller slot 3
receives, in order: original filename text, sound ID, cell pointer,
head-relative Boolean, loop Boolean and signed fade-in selector converted
to f32 (`0x0094698F–0x009469BE`). False returns false **without reverting
state 2**, volume factor or counter. Success then calls controller slots
5 position (always), 8 distances, 6 volume, 7 pitch, in that order. Volume
is signed scaled volume × f32 `1/1024` (`0x00A931FC`); position/distance
conversions match the internal path. Pitch is P[abs(k)] for **both signs**,
converted from double to f32, so a negative step raises external pitch.
Post-start setter results are ignored. No internal sound is created.

Update's special loop branch decrements the counter first. Without a stop
mark, exactly zero reloads the configured length. With the mark, a
**nonzero** remaining counter calls Kill (slot 8); zero leaves it playing
until a later update (`0x00945A4C–0x00945A6D`). This is not a conventional
“stop when zero” countdown and does not notify the external controller.

For nonlooping state 2 and state 3, an external controller's slot 9 is
queried with the cell. If it returns false, slot 4 is called with the cell
and signed cell `+0x38` converted to f32, then state becomes zero
(`0x009459D0–0x00945A32`). If it returns true, normal internal-sound checks
continue: a null internal sound leaves the state unchanged; a finished
sound invokes Stop, and a finished fade invokes Kill. StopCellExternal
notifies slot 4 only from state 2; nonzero fade-out changes it to state 3,
while zero fade-out leaves state 2 pending a later controller notification.
Already-stopping state stays 3; other states become zero (`0x00945C20`).

**Worked cases (SS7).** `amb_1234ABCD.wav` is recognized and requests ID
`0x1234ABCD`; `1234-ABCD.wav` is not. External Start returning false leaves
IsBusy true, so another Start fails until the state is cleared. A marked
loop with counter 1 remains in state 2 after the next decrement to zero;
the following decrement to −1 calls Kill. Pan values (768,512), without
random pan, produce position (50,0,0); centered random pan with low draw
bits 2 produces (−100,0,0). An external controller reporting false from
slot 9 receives one slot-4 notification before the player becomes idle.

## 7. `cGZSndSys` and `cGZSndMgr`

### 7.1 Framework sound system

Create `0x009663DF`, constructor `0x00968773`, destructor `0x009689CD`;
QI/AddRef/Release `0x00966723`/`0x00966792`/`0x00966799`.
CLSID `0xE41CE9FD`, service ID initially `0x23CECBA0`. Constructor creates
empty target, renderer, handler, pending-operation and release lists,
critical sections, enabled update state and master volume `0x400`.
If the filesystem service is present it supplies the initial data directory.
EnabledFlag and EnableUpdates/DisableUpdates are distinct state controls.

Primary table `0x00AD7180`:

| Slots | Operations, in slot order |
|---|---|
| 0–4 | QI, AddRef, Release, AsIGZSnd3DListener, AsIGZSnd3DListenerEx |
| 5–8 | DoEvent overloads: payload forms, existing event last |
| 9–12 | PostEvent overloads: payload forms, existing event last |
| 13–17 | two EventCreate overloads, RegisterEventHandler, UnregisterEventHandler, Update |
| 18–27 | running-latch getter, SetDataDirectory, DataDirectory, GetEnabledFlag, SetEnabledFlag, SetExtensionDefaultDataSourceClassId, GetExtensionDefaultDataSourceClassId, mapping count, mapping class-ID lookup, mapping extension lookup |
| 28–34 | GetMasterVolume, SetMasterVolume, RegisterTarget, UnregisterTarget, EnumerateTargets, EnableUpdates, DisableUpdates |
| 35–41 | RegisterRenderer, UnregisterRenderer, IsRegisteredRenderer, NumRenderers, FindPlayingGZSnd, IsSndPlaying, GetCriticalSection |
| 42–48 | two GetSndPropertyCache forms, two GetSndInstanceCache forms, AddToMainlineReleaseList, Panic, SetTestMode |

The service table `0x00AD714C` has Init/Shutdown in slots 8/9.
Listener table `0x00AD7110` has CommitDeferredSettings at slot 5,
GetPosition/SetPosition at 6/7, GetOrientation/SetOrientation at 8/9,
and AsIGZSnd3DListenerEx at 10. Init `0x00967FE4`, Shutdown `0x009683EF`,
DataDirectory `0x0096C061`, GetTestMode `0x00968352` and extension helpers
`0x009676D2`, `0x00967ACB`, `0x0096818B`, `0x009686B6`, `0x009689E9`
complete the named lifecycle/configuration inventory. Numeric listener
conversion and the tail/folded entries are specified in §§7.1.1–7.1.2 below.

*[compat]* DoEvent(existing event) offers it to registered handlers in
list order and stops at the first true result; it returns false when none
handles it. Handler mutation during dispatch is deferred until outermost
dispatch exits. Deferred registrations are drained before deferred removals
(`0x009679AD`). This is a consuming chain, not broadcast-to-all.
Payload overloads allocate an event, populate its payload and invoke that
dispatch; failed/null event creation returns false (`0x00966C82`, `0x00966D3B`).
The four-word payload setters are described in [audio.md §4](audio.md#4-the-sound-event-pool).

*[compat]* PostEvent(existing event) posts the event's message interface
through the stored message service and returns true (`0x00966A95`);
it does not synchronously return a handler's verdict. It assumes a valid
initialized message-service pointer. Do not interpret this success as playback.

*[compat]* DisableUpdates suppresses target enumeration work; EnableUpdates
restores it. UpdateTargets maintains separate ordinary and slower target
lists, stops each enumeration when its callback returns false, defers target
changes while enumerating, and drains them when the outermost enumeration
finishes (`0x00967885`). Registration/removal retain references through
their queues. The following list rules are instruction-confirmed (SS12):

- Outside enumeration, registration first unregisters the pointer, then inserts
  it again: targets at the back of the selected active list, handlers at the
  front. Repeated registration therefore moves an existing entry; it does not
  accumulate one entry per call. Target period exactly 20 selects the ordinary
  list; every other value selects the slower list (`0x00967DFF–0x00967E2F`).
- Unregistration removes only the first matching pointer in each applicable
  active list: one handler, or one entry from each of the two target lists.
  It stops searching that list after erasure. Absence still returns true
  (`0x00967E9F–0x00967EEA`, `0x00967FAB–0x00967FD5`).
  Target unregistration is suppressed entirely when the byte at object
  `+0x120` is set: it returns true without locking or changing a list
  (`0x00967E57–0x00967E5F`). The replacement rule assumes this guard is clear.
- During enumeration, requests retain their pointer in separate queues. At the
  outermost target exit, drain removals FIFO, ordinary additions FIFO (period
  20), then slower additions FIFO (period 250), irrespective of the original
  interleaving (`0x0096792E–0x00967995`). Each drained add uses the ordinary
  registration rule, including its preliminary unregister. Thus a queued slow
  add wins over a queued fast add of the same pointer, even if requested first.
- Handler additions enter the **front** of their pending list; removals enter
  its back. At outermost dispatch exit, drain additions from the front, then
  removals from the front. Add requests are replayed in reverse request order;
  removals in request order (`0x00967F1D–0x00967F46`, `0x009690F4–0x00969107`,
  `0x00967F86–0x00967F98`, `0x009679AD`). Replayed registration inserts at the
  active front. There is no single chronological mutation log.

*[compat, worked SS12]* Register handlers A, B, A outside dispatch: the active
order is A, B, with one retained entry for A. While a handler is dispatching,
request add X, add Y, remove X. Replay adds Y then X, then remove X: the next
dispatch starts with Y ahead of the old handlers. For an initially absent
target T, request slow-add T, fast-add T, remove T during one enumeration:
the removal runs first, then fast-add, then slow-add; exactly one T survives
in the slow list. A chronological target queue would incorrectly remove it.

The remaining target timing contracts are SS8. SetMasterVolume
their queues; UnregisterTarget removes the matching target from both active
lists, or defers removal during enumeration (`0x00967DD4`, `0x00967E4F`).
The exact period classifier and timer boundary are below. SetMasterVolume
notifies registered audio renderers using the stored master volume
(`0x009675DF`); no visual drawing operation is involved.

#### 7.1.1 Timing and table completion (SS8)

*[compat]* RegisterTarget(target, period) classifies **period exactly 20**
as ordinary; every other word, including 0, 19, 21 and 250, means slow
(`0x00967DFF–0x00967E2C`). These are two categories, not per-target clocks.
Outside enumeration, registration first unregisters the same pointer and
then appends one retained entry. During enumeration it appends a retained
pending entry to the corresponding category without altering active lists.
Unregister removes the pointer from both categories, or queues removal.

UpdateTargets visits ordinary targets every enabled call. It visits slow
targets only when unsigned elapsed milliseconds are **greater than 250**,
and restarts that timer before visiting them (`0x009678E0–0x009678E9`).
A false callback stops its current list; it does not suppress the separate
slow-list check. At outermost enumeration exit it drains pending removals,
ordinary additions, then slow additions. Replayed additions use periods
20 and 250 respectively and unregister before appending. Thus additions
win over removals queued during the same enumeration, and a target queued
in both addition categories ends in the slow list. EnableUpdates controls
this enumeration gate; EnabledFlag is the separate initialization gate.

Primary slots 40–52, completing the earlier inventory:

| Slots | Operations, in order |
|---|---|
| 40–45 | IsSndPlaying, GetCriticalSection, borrowed property-cache getter, property-cache out getter, borrowed instance-cache getter, instance-cache out getter |
| 46–52 | AddToMainlineReleaseList, Panic, SetTestMode, GetTestMode, GetSoundSystemThreadId, destructor, running-latch setter |

Slot 18 (`0x0096831B`, folded under a cell-player name) reads the byte at
full-object `+0x3D`; slot 52 (`0x0096831F`) stores its Boolean argument there.
It is the callback-running latch, not SetEnabledFlag. Slots 21/22 manage
EnabledFlag. Slots 42/44 (`0x009683D9`, `0x009683E0`) return borrowed cache
pointers. Slots 43/45 (`0x00966F31`, `0x00966F63`) write the pointer, AddRef
when nonnull, and return whether nonnull; null writes null and returns false.
The system owns its own cache references independently of those callers.

#### 7.1.2 Extensions, listener and renderers (SS8)

*[compat]* Extension insertion (`0x009689E9`) copies the text, lowercases,
trims, removes **one** leading dot, and rejects an empty result. Otherwise
it inserts/replaces that key's class ID, including zero. Lookup
(`0x009686B6`) applies the same normalization and returns false without
changing the output on a miss. It does not insert a miss or parse a full
filename into an extension. Count is the ordered map's entry count.
Class-ID-by-index (`0x00967ACB`) walks sorted keys from the first; out of
range returns false and leaves the output unchanged.

The extension-text-by-index entry (`0x0096818B`) has a different observed
contract: it first **finds the current output string as an exact map key**,
then advances index times from that entry, and assigns the found key to
the output string. No normalization is performed. A missing seed key or
walking past the end returns false unchanged
(`0x00968194–0x009681EA`). Do not replace this with ordinary index-from-first
enumeration without a deliberate compatibility choice.

The basic listener view is full object `+0x0C` (table `0x00AD7110`);
the extended view is `+0x10` (`0x0096830F`). Basic slots 5–10 are
CommitDeferredSettings, GetPosition, SetPosition, GetOrientation,
SetOrientation, AsIGZSnd3DListenerEx. Position is three f32 values and
orientation six f32 values: getters use f32 loads/stores
(`0x009667CA–0x00966837`); setters store/forward the argument words without
integer scaling, normalization or double conversion (`0x00967056`,
`0x00967109`). Relative to the basic view, position is `+0xD8/+0xDC/+0xE0`
and orientation `+0xE4` through `+0xF8`. Renderer notifications query
IID `0x406077D2`, invoke listener slots 7 or 9, and release the temporary
view. A failed query skips that renderer; setter return values are ignored.
Extended velocity uses three f32 words (`0x0096683A`, `0x009671EC`) and
IID `0x5B30E7B3`; the combined update forwards velocity, orientation, then
position (`0x0096729F`).

RegisterRenderer (`0x00967CEE`) checks exact interface-pointer identity,
appends/retains only when absent, and returns true even on duplicates.
IsRegisteredRenderer uses the same pointer test; it does not QI for a COM
identity. UnregisterRenderer removes the matching retained entry under the
renderer lock. Cache/renderer success alone does not establish playback.

#### 7.1.3 Initialization and shutdown (SS8)

*[compat]* Init is entered on service view `S = full object +4`.
It locks the service, optionally refreshes the data directory from the
filesystem, then unconditionally registers message handler `0xDB53383B`
through RZ::MS2. That message service is assumed present. When EnabledFlag
is true and the running latch false, it creates a message queue, requests
property cache class/IID `0x9D4C6B95/0x5D4C6B90`, then instance cache
`0xBDF613B4/0x3DF613A1`, creates/starts the elapsed timer and slow timer,
and creates the callback timer. The callback period/configuration are 20 ms;
it installs UpdateTimerCallback, starts the callback timer, requests thread
priority 4, then stores the timer-start result in the running latch
(`0x00967FE4–0x0096818A`). Cache factory results are not tested. Init returns
true even when disabled or timer start fails; allocations and required
services have no general checked recovery. Repeating Init still registers
the message handler before checking the running latch.

Shutdown (`0x009683EF`) sets the shutdown byte `S+0x11C` first, stops the
callback and elapsed timers under the service lock, releases/reacquires
that lock, unregisters message `0xDB53383B`, and stops/deletes the callback
timer. It clears the three event-handler operation lists at
`S+0x9C/+0xA0/+0xA4` before target cleanup. Under the target lock it visits
ordinary `S+0x68` then slow `S+0x6C` targets; each successful QI for
`0x206077D2` gets sound slot 9 (Stop). A temporary sound reference is
released before the next query, with the last released after the traversal.

It clears active and queued target lists in order
`S+0x68, +0x6C, +0x70, +0x74, +0x78`, clears the running latch through
primary slot 52, and clears those lists again. It then clears renderers
`S+0x8C`, the three event-handler lists again, the extension map `S+0xB8`,
and the mainline-release list `S+0xC4`. Last it releases/nulls the property
cache `S+0xD4`, instance cache `S+0xD8`, and message queue `S+0x2C`, in that
order (`0x00968617–0x009686AC`). Retained list entries release their owned
references when cleared. No renderer Shutdown call or rollback transaction
is implied by those reference releases; this method returns true.

The consuming event chain in §7.1 above is unchanged. These lifecycle rules
do not change DoEvent's event ownership or turn PostEvent into synchronous
playback. Runtime scheduling precision and device/cache factory outcomes
remain external observations: to determine an actual backend failure case,
record both factory results and callback-timer Start's result, then the
running latch/cache outputs and whether `0xDB53383B` is still registered.
That observation distinguishes a disabled/failed start from successful Init's
unconditional true return.

**Worked cases (SS8).** Targets registered at 20, 19 and 250 put only the
first in the ordinary list. At slow elapsed 250 ms only ordinary runs; at
251 ms slow also runs and its timer restarts. Queued remove plus ordinary
add plus slow add of the same pointer leaves one slow registration. Setting
`" .WAV "` to 7 then `"wav"` to 9 leaves count 1 and lookup `".WaV"` gives 9.
Extension-text index 0 with an empty output string fails; seeding it with
`"wav"` succeeds. Registering the same renderer twice retains one entry.
SetPosition(1.5,−2.25,0) round-trips as those f32 values. With EnabledFlag
false, Init returns true and registers the message handler but creates no
new cache/timer set. Shutdown releases caches before its message queue.

### 7.2 Framework manager

The method/address inventory and sound-id transformation remain in
[audio.md §8.6](audio.md#86-cgzsndmgr--the-framework-sound-manager).
The export lacks usable pseudocode for most `cGZSndMgr` methods; the rules
below were recovered from the instructions (SS9).
Create `0x009663F8`, constructor `0x00969AF1`, destructors
`0x00969889`/`0x00969AB9`; manager interface `+0x1C`, table `0x00AD7300`:

| Slots | Operations, in slot order |
|---|---|
| 0–7 | QI/AddRef/Release thunks, GetResource, SetSystemSndVol, folded setter, SystemSndID, SetSystemSndID |
| 8–16 | Set/GetDefaultSndDataSourcePlayer2DClassId, Set/GetDefaultSndDataSourcePlayer3DClassId, RegisterSnd, two UnregisterSnd forms, RegisterSndCreator, RemoveSndCreator |
| 17–23 | two GetSound forms, two SndAutoPlay forms, two SystemSndAutoPlay forms, AddSndToAutoPlayList |
| 24–31 | GetSndDataSource, RegisterSndDataSourceCreator, two RemoveSndDataSourceCreator forms, GetSndDataSourceCreator, RegisterSndDataSource, two UnregisterSndDataSource forms |

*[compat]* Init `0x00968BA7` sets its initialization flag before acquiring
the sound system. If that service is absent, the first call returns false
but leaves the flag set; a subsequent call can report true without retrying
that acquisition. When the service exists, it registers the manager update
interface (`+0x20`) with period 250 and retains a persist-resource-manager
reference. Instructions `0x00968BBE–0x00968C08` establish this partial-init
behavior. SystemSndID `0x009692FC` looks up the configured system-sound
mapping and returns zero when no entry exists (`0x00969313–0x00969324`).
This service is invoked through interfaces; no additional SC4 message IDs are
established by these methods.

#### Construction and service acquisition (SS9)

*[compat]* The factory allocates `0xA8` bytes and returns null on allocation
failure (`0x009663F8–0x00966410`). Construction uses system-service ID
`0x1AF85BFB`, priority `0x7A11E`, and these base-object offsets; manager-method
instructions generally use the interface pointer at `+0x1C`, so their apparent
member offsets are smaller by `0x1C`.

| Base offset | Initial state / purpose |
|---|---|
| `+0x24` | initialized byte = 0 |
| `+0x28`, `+0x2C` | null sound-system and persist-resource-manager references |
| `+0x30` | system-sound volume = `0x400` |
| `+0x34`, `+0x4C` | empty registered-sound and system-sound-ID hash tables; each initialized with bucket request 100 |
| `+0x48`, `+0x68`, `+0x70`, `+0x80`, `+0x90` | zero registry/list traversal counters |
| `+0x60` | separately allocated critical-section object; null if that allocation fails |
| `+0x64`, `+0x6C` | empty sound-creator and autoplay lists |
| `+0x74`, `+0x84` | empty unsigned-ID ordered maps of data-source creators and registered data sources |
| `+0x94` | empty unsigned-ID set |
| `+0xA0`, `+0xA4` | default 2D/3D player class IDs = 0 |

Evidence: `0x00969AF7–0x00969BC4`, hash initializer
`0x00969AD5–0x00969AE8`. The class-ID setters simply store the supplied
32-bit value and return true (`0x00968B67`, `0x00968B87`); the manager does not
substitute a built-in renderer class for zero.

*[compat]* Init does not test the result of RegisterTarget and returns true
even if resource-manager acquisition produces null (`0x00968BDF–0x00968C08`).
GetResource takes `(key, iid, output)` and delegates to resource-manager slot
3 (`+0x0C`), forwarding those arguments followed by two zero arguments.
Without the resource-manager reference it returns false without writing the
output (`0x00968C18–0x00968C38`). This is a separate API; GetSound and
GetSndDataSource do not implicitly call it or synthesize a resource key.

#### Registry and creator resolution (SS9)

*[compat]* Both ID lookup forms accept `(id, a, b, iid, output)` and return
a Boolean in the low byte. `a` and `b` are forwarded unchanged to creators.
The manager itself does not zero the output before a lookup.

| Stage | GetSound, slot 18 (`0x009693C0`) | GetSndDataSource, slot 24 (`0x00969055`) |
|---|---|---|
| Gate | Ask stored sound system for NumRenderers, slot 38 (`+0x98`). Zero returns false before cache lookup, even for a registered sound. The service pointer is assumed valid. | No renderer-count or initialized-flag gate. |
| Registered object | Under the manager lock, find the exact ID in the sound hash. | Under the same lock, find the exact ID in the registered-source map. |
| Cache hit | A nonnull object receives QI with only `(iid, output)`; return its result. A null stored object returns false. | Same. |
| Cache miss | End the first lock scope, reacquire the lock and try sound creators in list order, newest registration first. | End the first lock scope, reacquire the lock and try data-source creators in ascending unsigned creator-ID order. The requested source ID is not used to select only a same-ID creator. |
| Creator call | Creator slot 3 (`+0x0C`) receives all five original arguments; stop on the first true result. | Same. |
| Failure | No creators, or every creator returning false, yields false. | Same. |

There is **no creator fallback after a cache hit whose QI fails**, and no
automatic insertion of creator results into either registry. A callback can
register its own result, but that is its action. Output writes and successful
reference transfers on delegated QI/creator calls belong to that callee's
contract. In particular the manager does not restore an output changed by a
failed callback. Evidence: `0x009693C9–0x0096946D`,
`0x0096905F–0x009690ED`. GetSound before a successful service acquisition has
an unchecked service dereference, not an additional clean false result.

*[compat]* RegisterSnd and RegisterSndDataSource retain the first pointer
registered for an ID; later same-ID registrations leave it unchanged. These
are side-effect operations, without an established Boolean status result
(`0x009699BA–0x00969A2E`, `0x009697B7–0x0096982B`). A null pointer can occupy
a previously absent ID and then blocks creator fallback just like any cache
hit. RegisterSndCreator rejects null or failure to QI `0x3AF9866F`; on success
it removes the original pointer's old list entry and pushes that original
pointer to the front. RegisterSndDataSourceCreator rejects null or failure to
QI `0x9B75709E`; on success it replaces the creator at the supplied creator
ID. The QI is an eligibility check; storage retains the supplied pointer,
not the returned interface pointer (`0x00969335–0x009693B8`,
`0x00969A31–0x00969AB1`). GetSndDataSourceCreator does an exact creator-ID
lookup and QI, returning false without an output write on absence; it does
not perform the source-resolution walk (`0x00968FFE–0x00969050`).

*[compat, worked SS9 lookup]* With zero renderers, registered sound S under
ID 7 is not queried and GetSound(7) returns false, preserving an output
sentinel. With one renderer, if S rejects IID X, that rejection is returned
and creator C is not tried. For a missing data-source ID 99, creators at
IDs 20 and 10 are tried in order 10, 20; if 10 fails and 20 succeeds, the
result is true. A second call repeats that walk unless a creator registered
a source in the meantime. A missing resource-manager reference makes
GetResource fail independently of these outcomes.

#### Autoplay and shutdown (SS9)

*[compat]* SndAutoPlay(id, volume), slot 20, first checks the sound registry
under the lock. On a hit it retains that sound directly, without the renderer
gate or a QI. On a miss it calls GetSound's ID form with
`(id, 0, 0, 0x206077D2, output)`; a false result returns false. A successful
lookup is appended to the autoplay list **before** Play is called. The list
retains its own reference; the temporary lookup reference is released after
leaving the lock, before invoking sound slot 6 (`+0x18`) with
`(volume, 0, 1)`. Return the Play result. A false Play result does not undo
the list insertion (`0x009694A2–0x00969534`). Cached null pointers and creators
that report success without a valid pointer are unchecked in this path.

AddSndToAutoPlayList always appends, retaining one reference per entry, and
returns true; it does not deduplicate (`0x00969475–0x0096949F`). Each manager
update checks **only the front sound** using its slot 7 (`+0x1C`). If it is
no longer playing, remove exactly that one front entry. A playing front blocks
cleanup of all later entries, even stopped ones (`0x00968F11–0x00968F4C`).
The volume-less overload uses the stored system volume. SystemSndAutoPlay
resolves the system-sound index and passes ID **zero** to SndAutoPlay when
unmapped; it does not short-circuit that missing mapping
(`0x00969536–0x00969583`).

*[compat]* Shutdown acquires the manager lock and releases retained contents
in this order: autoplay list, system-sound mapping, registered sounds,
sound-creator list, data-source-creator map, then the ID set. Finally it clears
the initialized byte and returns true (`0x0096966C–0x0096970F`). It calls
neither Play/Stop/Kill on the sounds nor UnregisterTarget on the sound system.
The **registered-data-source map at `+0x84` survives Shutdown**, as do the two
service references, system volume and default player class IDs. Thus the
"clears every registry and list" shorthand in sound-framework.md §3.2 is too
broad; this paragraph specifies the observed boundary. Reinitialization can
register the same manager update pointer again using §7.1's replacement rule.

Destruction calls Shutdown first, destroys the critical section, then destroys
the ID set, registered-data-source map, creator map, list/hash containers,
and finally releases the resource-manager reference before the sound-system
reference (`0x00969889–0x00969917`). Those surviving source references are
therefore released at destruction or explicit unregistration, not Shutdown.

*[compat, worked SS9 ownership]* Autoplay A reports false from Play and B
reports true. Both are retained in order A, B. If both subsequently report
stopped, the next update releases only A's list reference; the following
update releases B's. Register data source D at ID 42, then Shutdown: the
autoplay/sound/creator references are released, but GetSndDataSource(42)
still queries D. Destroying the manager releases D's registry reference.

## 8. Directors

These are class-factory/lifecycle objects, not sound sources. They inherit
the COM director's framework and class-registration contract. The named
owner-specific operations below augment that common table; in the director
interface OnStart is slot 4 (`+0x10`), as in [audio.md §1](audio.md#1-identity).
Folded export names are not used as lifecycle names; the complete slot
contracts are listed below (SS10).

| Owner | Construction and named lifecycle operations | Identity/factory effects |
|---|---|---|
| cGZSoundCOMDirector | ctor `0x00966498`; GetDirectorID `0x00966539`; OnStart `0x0096655A`; PostAppInit `0x00966678`; PreAppShutdown `0x0096653F` | director `0xE9D3A40B`; eight classes listed in audio.md §1 |
| GZSndSysMacCOMDirector | ctor `0x0087484B`; GetDirectorID `0x008748B9`; OnStart `0x008748BF` | director `0xE9D3A016`; five classes below |
| cFreshnessCOMDirector | ctor `0x009456EC`; dtor `0x00945636`; GetDirectorID `0x00945630`; GetDirector `0x0094575A` and `0x00945785` | director `0xFD53FF17`; five classes below |

*[compat]* The sound director OnStart registers its framework hook, creates
the sound system and manager using service IID `0x287FB697`, and adds nonnull
created services to the framework. It also creates class `0x7DC495CC`
with handler IID `0xBB523BB2` and registers a nonnull result with the sound
system. It returns true even when individual creations produce null outputs.
PreAppShutdown calls sound-system DisableUpdates (slot 34, `call [edx+0x88]` at
`0x00966551`) if the service exists and returns true; it does not call Panic (slot 47).
The byte-level detail of this layer is in [sound-framework.md](sound-framework.md).
PostAppInit is specified below, including its missing-cache behavior.

*[compat]* Despite its name, GZSndSysMacCOMDirector is present in the Windows
program. Its constructor registers classes `0xFDED7F31`, `0xFDED7F34`,
`0xE9D28415`, `0x89D3D983`, `0x1C5F21FE`. OnStart registers its framework
hook and requests `0xE9D28415` and `0x89D3D983` under service IID
`0x287FB697`, adding each nonnull result to the framework; it returns true.
Backend creation is distinct from choosing the active renderer.

*[compat]* The freshness director registers `0x3C1CF59E` (player),
`0x5C1CF5A3` (score), `0xBDDA72DD`, `0x7DD9C9F2` (cell), `0xBDD9D7A5`.
Both GetDirector entry points return the same lazily constructed process
singleton; construction occurs only once and arranges process-exit cleanup.
No separate owner-specific message switch is established for these directors.

### 8.1 Complete inherited dispatch (SS10)

*[compat]* The primary tables are `0x00AD7070` (sound), `0x00AC2848`
(Windows backend named Mac), and `0x00AD47C0` (freshness). All use the following
17 slots; byte offsets are relative to the selected interface's table.
Shared-body details are also documented in
[renderer-gdrivers-director.md §6](renderer-gdrivers-director.md#6-shared-dispatch-and-destruction-checklist).

| Slot / offset | Contract and target |
|---|---|
| 0 / `+0x00` | QI `0x0090DE02`: IID 1 or `0xA21EE941` returns primary; `0x03FA40BF` returns object `+0x04`; successful queries AddRef once. Unsupported IID returns false without clearing output. Supported IID assumes a writable output pointer. |
| 1 / `+0x04` | `0x0094C4B1`: increment the 32-bit reference count at object `+0x08`, return the new count. |
| 2 / `+0x08` | Release `0x0090E242`: delegate to slot 10; reaching zero does not delete the director. |
| 3 / `+0x0C` | InitializeCOM `0x0090DE57`: null COM handle returns false; otherwise remember it, obtain framework, store library path, initialize children in vector order, return true without aggregating child results. |
| 4 / `+0x10` | OnStart: sound `0x0096655A`, backend `0x008748BF`; freshness uses `0x0091559A`, which ignores its one argument and returns true. |
| 5 / `+0x14` | EnumClassObjects `0x0090E02B`: enumerate children first, then the director's local class registry; no Boolean result contract. |
| 6 / `+0x18` | GetClassObject `0x0090DFAC`, §8.2. |
| 7 / `+0x1C` | CanUnloadNow `0x0090DEA2`: with count zero, return false if any child returns false; otherwise true. A nonzero local count returns true without consulting children. |
| 8 / `+0x20` | `0x00648F00`: return true, no side effects. The folded label `operator.new` does not describe this slot. |
| 9 / `+0x24` | `0x009C32AC`: return current 32-bit reference count. |
| 10 / `+0x28` | RemoveRef `0x0090DE43`: decrement only when nonzero, return resulting count; no destruction. |
| 11 / `+0x2C` | `0x009CDC23`: return stored framework handle, initially null, without retaining it. |
| 12 / `+0x30` | `0x009D858F`: return stored COM handle, initially null, without retaining it. |
| 13 / `+0x34` | GetDirectorID: owner-specific constants in the table above. |
| 14 / `+0x38` | GetLibraryPath `0x0090DECD`: assign stored path to supplied string; return true. |
| 15 / `+0x3C` | `0x0090DE52`: shared size query returns 32-bit zero. |
| 16 / `+0x40` | Deleting destructor; sound/backend share `0x0090E117`; freshness `0x00945636`. Explicit deletion is separate from Release. |

Construction initializes reference count zero, null COM/framework handles,
an empty path, empty child vector and empty class registry
(`0x0090E18D–0x0090E1CC`). Reference increment wraps at 32 bits; decrement
saturates at zero. These are GZ Boolean/interface contracts, not HRESULTs.

*[compat]* The secondary identity at object `+0x04` has exactly slots 0–10.
Tables: sound `0x00AD7040`, backend `0x00AC281C`, freshness `0x00AD4790`.
Slots 0/1/2 (`+0x00/+0x04/+0x08`) adjust to the primary identity and invoke
QI/AddRef/Release (`0x00460F50`, `0x00460F60`, `0x00460F70`).

| Secondary slot / offset | Sound | Backend | Freshness |
|---|---|---|---|
| 3 / `+0x0C` | true, no-op | true, no-op | true, no-op |
| 4 / `+0x10` (PreAppInit) | true, no-op | true, no-op | true, no-op |
| 5 / `+0x14` (PostAppInit) | §8.3 | true, no-op | true, no-op |
| 6 / `+0x18` (PreAppShutdown) | DisableUpdates if service exists; true | same shared body | true, no-op |
| 7 / `+0x1C` | true, no-op | true, no-op | true, no-op |
| 8 / `+0x20` | true, no-op | true, no-op | true, no-op |
| 9 / `+0x24` | true, no-op | true, no-op | true, no-op |
| 10 / `+0x28` | true, no-op | true, no-op | true, no-op |

All listed no-ops point to `0x00949ADE`, whose instructions return true
without an explicit argument or side effect. The unrelated nonempty
PreFrameWorkInit at `0x0087B1FC` is not installed in these tables. Unnamed
callback slots above are fully specified by position and behavior; their
folded symbol names do not establish additional lifecycle actions.

### 8.2 Class factories and failures (SS10)

*[compat]* Constructors register no-argument construction callbacks using
the shared AddCls form at `0x0090E133`, registry kind 1. GetClassObject takes
`(class ID, requested IID, output)`, tries children in vector order until one
returns true, then looks up the local class. For kind 1 it calls the
constructor callback and immediately invokes QI on its returned identity.
Successful QI returns true and supplies the caller's retained interface.
Failed QI causes AddRef followed by Release on the construction identity,
then returns false (`0x0090DFAC–0x0090E023`). The director adds no output
clearing: an unknown class leaves a sentinel alone when no child changed it;
failed delegated QI preserves or changes it according to that QI.

There is no controlling-outer parameter, aggregation negotiation, or
aggregation-specific error in this director API. The stored COM/framework
handles are services, not an outer unknown. Do not infer Windows
IClassFactory/CreateInstance semantics from the method name. The shared
kind-2 registry form delegates directly to `(iid, output)` callbacks, but
these owners install kind-1 callbacks.

*[compat]* Construction callbacks return null when their initial heap
allocation fails. For the backend's five registrations the returned identity
is as follows; successful results may be subobjects rather than allocation
base addresses.

| Class ID | Callback | Allocation bytes | Returned identity relative to allocation |
|---|---|---:|---:|
| `0xFDED7F31` | `0x008747F5` | `0xD8` | `+0x00` |
| `0xFDED7F34` | `0x0087480E` | `0x100` | `+0xD8` |
| `0xE9D28415` | `0x008747B0` | `0xA0` | `+0x08` |
| `0x89D3D983` | `0x008747D4` | `0x54` | `+0x04` |
| `0x1C5F21FE` | `0x00874835` | `0x14` | `+0x00` |

The offset-returning callbacks explicitly keep allocation failure null; they
do not return an offset from null. However GetClassObject has **no null-result
check** before the QI dereference (`0x0090DFFC–0x0090E00B`). Consequently
allocation failure in a kind-1 callback is not a guaranteed false return
through that API. Likewise OnStart assumes a valid framework and COM handle.
Its true return on null service outputs concerns ordinary failed creations
that return control, not an exception from an invalid handle or allocation.
Previously added services are not rolled back when a later creation fails.

*[compat, worked SS10 factory]* On a newly constructed backend director,
Release returns zero and does not destroy it; AddRef then returns one.
GetClassObject for an unknown class with output sentinel P returns false and
preserves P when there are no children. A known class that rejects the IID
receives QI, AddRef, Release in that order on its construction identity.
If OnStart's first service succeeds and the second outputs null, the first
remains registered and OnStart returns true. Injecting allocation failure
into a constructor requires testing the unchecked factory dereference as a
failure boundary, not expecting an ordinary false result.

### 8.3 PostAppInit creates the global property set (SS10)

*[compat]* Sound PostAppInit obtains a retained sound-system service. Absence
returns false. Otherwise it gets the borrowed property cache through primary
slot 42 (`+0xA8`), retains a nonnull cache, invokes **CreateInstanceSet** through
cache slot 3 (`+0x0C`) with instance `0xCC3014EE`, releases the cache and service,
and returns true. It ignores CreateInstanceSet's Boolean result. If the sound
system exists but its cache is null, the conditional AddRef is skipped but the
slot-3 dereference is not: this is an unchecked null-cache failure, not false
or successful no-op (`0x00966678–0x009666C8`).

The cache table installed at `0x0096A38E` is `0x00AD73F8`; its slot 3 points
to `0x0096A3DE`. That body requests a new resource with key
`(type 0xEBFEE345, group 0xAC31543D, instance 0xCC3014EE)` and IID
`0x456B8F1D`, then registers the resource with its resource manager. Only if
both operations return true does it store the retained set in its instance
map and return true; otherwise it releases any temporary resource and returns
false (`0x0096A3DE–0x0096A460`). Thus the director creates the **global property
set**, rather than assigning a property value or warming a sound instance.
The same instance ID is used by SetGlobalProperty
(`0x00969F72–0x00969F77`).

*[compat, worked SS10 cache]* A fake sound system returns cache C. PostAppInit
retains C, calls C slot 3 with `0xCC3014EE`, then releases C. Even when that
call reports false, PostAppInit reports true. A missing sound-system service
reports false; a present service with a null cache reaches an invalid
dereference. These three cases must not share a success/failure policy.

## 9. `cGZPlayList`

This is a text parser, not an audio player or GZCOM service. Named operations
are construction/destruction `0x008D569F`/`0x008D56C8`, Open `0x008D6B22`,
_GrabSegment `0x008D6056`, _StripEndSegment `0x008D60C4`, _IsSpec
`0x008D6123`, _AddNextPathSegment `0x008D6196`, _GetLine `0x008D6258`,
_MakeAbsolutePath `0x008D6352`, _ParseExtM3UFormat `0x008D64E3`,
_ParseM3UFormat `0x008D66C9`, _ParsePLSFormat `0x008D6824`.
No sound-playback, window-drawing, message-handler or COM vtable belongs to
this parser. Its strings and file helper use interfaces internally; that
does not make the playlist itself a COM service (SS11).

*[compat]* Open selects extended M3U for a first header `#EXTM3U`, PLS for
`[playlist]`, otherwise plain M3U. In the plain case it restores the content
position so that the first line is parsed as input, not discarded as an
unrecognized header. The format is content-selected; the filename extension
alone is not the selector (`0x008D6B22`).

*[compat]* Format-header comparisons are exact and case-sensitive after line
trimming/conversion. A lowercase `#extm3u` or `[Playlist]` selects plain M3U.
Filename extension does not override the header (`0x008D6C0F–0x008D6C2C` and
the following comparison). The full IO, grammar and path rules follow (SS11).

*[compat, SS12 simple-path rule]* The path helper appends the next segment
directly to the base string; it neither chooses a slash style nor inserts a
separator (`0x008D6208–0x008D6211`). Its segment delimiter is the literal
backslash byte `0x5C` (`0x008D608D–0x008D60B5`). With the usual directory base
`C:\Music\` and entry `sub\a.mp3`, the result is `C:\Music\sub\a.mp3`.
With a helper base `C:\Music` lacking its trailing separator, `a.mp3` yields
`C:\Musica.mp3`. Joining with one chosen separator only happens to agree when
the supplied directory already ends in that separator; it is not the rule.

*[compat]* Extended M3U requires each record header to begin `#EXTINF:` and
then consumes a following path line, resolves it, and appends a record.
Its numeric metadata uses the C-library decimal-long conversion. EOF before
the next record is success; EOF where a record's path is required, or a
record header without the required prefix, returns false. Previously appended
entries are not rolled back by this parser (`0x008D64E3`). Do not replace this with
a standards-compliant general-purpose playlist parser without compatibility
tests: the observed extended parser has no general comment-skipping loop.

### 9.1 Object state, IO and line conversion (SS11)

*[compat]* Construction starts with an empty entry vector and three empty
strings: stored playlist path (`+0x0C`), playlist directory (`+0x20`), remaining
input (`+0x34`); it installs no playlist vtable
(`0x008D569F–0x008D56C7`). Open does **not** clear the entry vector. All three
parsers append to it, including across repeated Open calls. Failure does not
roll back entries already appended by this or an earlier call.

Open first tries the file/shortcut-resolution helper `0x0091AA3B` for the
stored path; on false it stores the supplied path literally. It derives the
directory from that stored path using GetPathDirectory. It constructs the file
helper from the supplied Open argument and opens with numeric arguments
`(1, 2, 1)` (`0x008D6B31–0x008D6B72`). Failure to open returns false without
parsing. On success it obtains the byte size, resizes the remaining-input
string, allocates a size-byte buffer, reads the entire file and closes it.
A false read returns false without parsing (`0x008D6B7D–0x008D6BBA`). Thus even
a failed Open may change the stored path/directory/input string, while
preserving prior entries.

After a successful read, NUL bytes at offsets 0 through size−2 are replaced
with spaces; the last byte is left alone. The buffer is copied into the
framework string and freed (`0x008D6BBF–0x008D6BE8`). The first logical line
selects a parser. For plain M3U the saved pre-header input is restored, so
the first line becomes a record. A parser's Boolean result becomes Open's
result; Open does not validate existence or playability of any listed path.

*[compat]* _GetLine skips leading space, tab, LF and CR, extracts through the
next LF or CR (or the remaining input at EOF), trims the resulting line, and
converts it **in place** from the current system/locale encoding (`−1`) to
framework encoding ID 8 (`0x008D6258–0x008D631F`). Consequently blank lines
are skipped, and leading/trailing whitespace cannot be preserved in titles
or paths. There is no playlist-specific UTF-8/UTF-16 BOM detector or stripping
step. The conversion helper resolves `−1` through GetCurrentSystemCodePage;
source ID 8 is already in the destination encoding, source ID 16 returns
false, and other code pages invoke the byte converter
(`0x00911473–0x0091151C`). _GetLine ignores the conversion status and still
reports a line. This must not be replaced by automatic Unicode sniffing or
an encoding error that aborts the playlist.

*[quirk]* If the nonblank final text has **no CR or LF**, _GetLine copies it
to the output but does not erase it from the remaining input
(`0x008D62E4–0x008D630F`). The two scanning helpers only find offsets; they
do not consume source text (`0x008D5F5B`, `0x008D5FDC`). Repeated calls thus
return the same final text. Plain M3U with an unterminated final path keeps
appending it and does not reach normal EOF. Other parsers may terminate on
that text as a count/header/error, but must not be assumed to consume it.
Worked helper case: source `a.mp3` yields `a.mp3` on each of two calls and
the source remains unchanged; source `a.mp3` followed by CR/LF yields one
line and then false. Use a bounded helper test for the first case rather
than running an unbounded Open loop.

*[runtime boundary]* To pin down non-ASCII output bytes rather than guess the
framework encoding table or OS locale, run the original under recorded
code-page settings with a one-line title containing bytes `0x80`, `0xE9`, an
invalid multibyte sequence, and UTF-8/UTF-16 BOM variants; record the resulting
title/path bytes and Open result. The static contract fixes the conversion
call, destination ID, ignored failure status and absence of BOM handling,
but does not supply a locale-independent replacement-byte table.

For a zero-byte file, the successful-read path computes an unsigned size−1
bound and scans the buffer with no zero-size guard (`0x008D6BBF–0x008D6BD8`).
The allocation is also unchecked before Read. To settle these IO edge outcomes,
observe whether a zero-byte read returns false before that scan; if it returns
true, record the exception/outcome. Repeat with injected buffer-allocation
failure. Do not claim that empty-file Open is a guaranteed successful empty
playlist or that allocation failure has a clean false return. Nonempty
whitespace-only input reaches plain M3U, appends nothing and returns true.

### 9.2 Entry formats (SS11)

*[compat]* An entry stores title at entry `+0x00`, path at `+0x14`, and a
32-bit duration at `+0x28` (`0x008D6171–0x008D618D`). Numeric metadata is the
signed 32-bit C decimal-long conversion: optional sign and leading whitespace,
decimal prefix, nonnumeric text yielding zero, with no duration scaling or
clamping in the parser. Negative durations, including `−1`, are retained.
Out-of-range text is delegated to the library conversion, not parsed as a
floating-point value.

| Format | One record and its stored fields | Return / ordering |
|---|---|---|
| Plain M3U (`0x008D66C9`) | Every logical nonblank line becomes a path, resolved as §9.3. Title is the last filename component **without its extension**, obtained before resolution by GetPathName with its extension flag zero. Duration is 0. Lines beginning `#` are records, not comments; HTTP URLs are path text, not network requests. | Read to EOF, append in file order, return true. |
| Extended M3U (`0x008D64E3`) | Require the exact prefix `#EXTINF:`. Consume the prefix through `:`, split the next token at the first comma and decimal-convert that token as duration. The remaining header text is the title, including any later commas. Consume one following logical line as path and resolve it. | EOF between records is true; wrong record prefix or missing path is false. Append each completed pair in file order; retain completed pairs on later failure. |
| PLS (`0x008D6824`) | Sequential, case-sensitive prefix parser described below. Numeric suffixes on File/Title/Length are not matched or sorted. | Counts only entries appended in this parse; partial additions survive false. |

The plain parser's extra concatenation branch requires both `:` and `\\` to
be found at index zero of the same line (`0x008D66F7–0x008D671D`), which an
ordinary string cannot satisfy. In particular it is not a URL/comment filter.
GetPathName's false extension flag omits the extension
(`0x0091A9EF–0x0091AA03`). Extended M3U does not separately trim the title
after splitting; spaces immediately after the comma remain. If the comma
is absent, SplitToken produces an empty duration token without consuming the
remaining header text: duration zero and that remaining text as title, not
a format rejection (`0x0090E9DF–0x0090EA16`).

*[compat]* PLS starts by reading a logical line after `[playlist]`. Immediate
EOF succeeds with no new entries. Thereafter:

1. A line beginning `NumberOfEntries` (first 15 bytes) terminates parsing.
   Erase the first **16** bytes, decimal-convert the remainder, and compare
   against this parse's appended count. Return that equality immediately,
   without reading Version or anything else (`0x008D6A9A–0x008D6AB8`). An
   equals sign is not separately validated by this count operation.
2. Other than that terminator, skip lines until one begins `File` (first four
   bytes). Strip through its first `=` and use the remaining text as path.
   A missing `=` erases zero bytes. Names such as `File99` and `Filename`
   pass the same prefix check (`0x008D688F–0x008D68D6`).
3. Read the next logical line, required even at the final record. Use it as
   title. If it begins `Title` (five bytes), strip through its first `=`.
   Otherwise keep the whole line as title **and** as the next outer-loop
   lookahead. In that latter case duration stays zero and no Length line is
   consumed (`0x008D68EB–0x008D6952`).
4. If there is no lookahead yet, read another required line. If it begins
   `Length` (six bytes), strip through its first `=`; otherwise retain that
   line as the outer lookahead. In either case decimal-convert this line for
   duration (`0x008D6958–0x008D69B8`).
5. Obtain a nonempty lookahead before appending, reading another logical line
   if necessary. EOF here returns false **before this record is appended**.
   After each such read the loop also exits if the **current path** begins
   `File`, even if the converted lookahead is empty; the prefix test is on
   the path, not the newly read line (`0x008D69D4–0x008D69E5`).
   Resolve the current path, append its title/path/duration, increment the
   local count, and resume at the lookahead (`0x008D69BD–0x008D6A48`). There
   is no suffix correlation, uniqueness check or index-based reorder.

EOF while skipping unrecognized lines, or while any required title/duration/
lookahead is needed, returns false. A final `NumberOfEntries` is therefore
needed for ordinary nonempty records, and it must follow them; a conventional
PLS that puts a nonzero count immediately after `[playlist]` fails before
reading records. Version has no validation or required value: an unrelated
Version line before the terminator is skipped, and one after it is unread.

### 9.3 Literal path processing (SS11)

*[compat]* The directory helper used by Open splits on either `/` or `\`
and retains the original directory bytes, including its final separator;
it does not choose a canonical slash style (`0x0091A847–0x0091A914`,
`0x00917C28–0x00917C77`, IsPathSeparator `0x005F8FE0`). It copies a two-byte
drive prefix when byte 1 is `:`. This does not make a relative playlist
filename into an OS-canonical absolute path.

_MakeAbsolutePath starts with the supplied base and consumes the entry text
from left to right (`0x008D6352`). The following rules act on **backslash**
segments only, not on forward slashes:

- A remaining string beginning with `\` is a specification: search for the
  next backslash starting at byte offset **2**, take the whole prefix through
  that separator, and replace the accumulated base with it. For a remaining
  string with `:` as byte 1, do the same starting at offset **0**. The drive
  character is not validated (`0x008D6123–0x008D6168`). UNC host prefixes
  therefore replace the base, as do drive prefixes and single-root paths.
- Otherwise take the next prefix through its backslash, or all remaining
  text at end. Append it literally unless it starts with `.`. No separator
  is inserted and existing separators are not collapsed
  (`0x008D6056–0x008D60B5`, `0x008D6208–0x008D6211`).
- For a segment starting with dots, count its leading dots. One dot appends
  nothing; two dots strip one segment from the accumulated base; three dots
  strip two, and so on. The rest of that dot-leading segment is discarded,
  so `.hidden\` is treated like `.\`, and `..hidden\` like `..\`
  (`0x008D61CF–0x008D6204`). This is not general filesystem normalization.
- Stripping first removes one trailing backslash, then truncates through the
  last remaining backslash. It has no drive/UNC-share root clamp. With no
  backslash left, truncation can empty the base (`0x008D60C4`). Forward slash
  segments and URL schemes receive no analogous normalization.

For a one-byte remaining `\`, specification extraction at offset 2 fails;
the ordinary segment path appends it instead. Repeated separators can cause
a later leading-backslash specification to replace the accumulated base;
they are not a request to join with exactly one separator. These routines
return a textual path without resolving symlinks, checking the target file,
or implementing URL retrieval.

*[compat, worked SS11 paths]* With base `C:\Music\Lists\`:

| Entry text | Resolved text |
|---|---|
| `sub\a.mp3` | `C:\Music\Lists\sub\a.mp3` |
| `.\a.mp3` | `C:\Music\Lists\a.mp3` |
| `..\a.mp3` | `C:\Music\a.mp3` |
| `...\a.mp3` | `C:\a.mp3` |
| `.hidden\a.mp3` | `C:\Music\Lists\a.mp3` |
| `sub/../a.mp3` | `C:\Music\Lists\sub/../a.mp3` |
| `http://host/a.mp3` | `C:\Music\Lists\http://host/a.mp3` |
| `D:a.mp3` | `D:a.mp3` |
| `\root\a.mp3` | `\root\a.mp3` |
| `\\host\share\a.mp3` | `\\host\share\a.mp3` |

With base `C:/Music/`, `sub\a.mp3` becomes `C:/Music/sub\a.mp3` (mixed
separators). With base `C:\`, `..\a.mp3` becomes `a.mp3`, losing the drive
prefix. These cases distinguish the parser from an OS path canonicalizer.

### 9.4 Worked IO/format cases (SS11)

*[compat]* In these format fixtures every listed line ends with CR/LF unless
the fixture explicitly states otherwise. Open a plain file at
`C:\Music\list.m3u` containing the three
logical lines `a.mp3`, `#note`, `http://host/b.mp3`. It returns true and appends
three records, all duration zero. Their paths are the directory plus those
literal line texts; the first title is `a`. Open the same file again on the
same object: six records, not three. A following missing-file Open returns
false and leaves all six entries.

An extended file consisting of `#EXTM3U`, `#EXTINF:-1,Two, parts`, `a.mp3`,
then `#comment` appends one entry with duration `−1` and title `Two, parts`,
then returns false at the comment. The appended entry remains.

A PLS file with lines `[playlist]`, `File2=b.mp3`, `Title99=Bee`, `Length7=12`,
`File1=a.mp3`, `Title1=Aye`, `Length1=-1`, `NumberOfEntries=2`, `Version=99`
returns true and appends B then A with their given titles and durations
12 and `−1`. Indices are ignored and Version is unread. Put
`NumberOfEntries=2` immediately after the header instead: false, no new
entries. Omit the count and end immediately after `Length1=-1`: false,
with B retained but A not yet appended. Use only `[playlist]`: true with no
new entries. These results depend on logical-line content, not the filename
extension.

## 10. Conformance vectors

These vectors name observable inputs and results; they are not recorded game
runs. Use fake services and child sounds to observe forwarding and ownership.

| ID | Input/setup | Required result |
|---|---|---|
| SS-V01 | Track count 0, child Load returns false; Load twice, Unload twice | true for all four calls; child Load once, Unload once; IsLoaded false at end |
| SS-V02 | New track, Unload once | true; IsLoaded true (count underflow) |
| SS-V03 | CheckedRegId: 74, 75, -1 | 74, 1, -1; only 75 reports illegal-register error |
| SS-V04 | Track execution ended; one dependent IsPlaying true | Track IsPlaying true |
| SS-V05 | Already-playing track; restart property 0; Play again | true, no Kill/restart |
| SS-V06 | Channel source exists; Load returns true; loop property nonzero | NoteOn reaches source Play with loop=true |
| SS-V07 | Channel source Load false; NoteOn | false; no source Play |
| SS-V08 | FreeChannel with a retained matching pointer, then repeat | first removes/releases one retained entry; second does not release it again |
| SS-V09 | New freshness player; Play, SetZoom(9), SetPlayPercent(25) before initialization | all return true; no playback registration; setters do not store those values |
| SS-V10 | Cell states 0, 2, 3 | IsPlaying false/true/false; IsBusy false/true/true |
| SS-V11 | Cell state 3 with a retained sound; Kill | true; sound reference released; state 0, stop flag clear |
| SS-V12 | Scaled-volume inputs C=A=P=S=L=1024 | intermediate 1024, output 1024 |
| SS-V13 | C=513, A=P=S=L=1024 | intermediate 513, output 513 |
| SS-V14 | C=-1, A=1, P=1024, S=L=1024 | intermediate 0, output 0 (signed division toward zero) |
| SS-V15 | Handlers A(false), B(true), C(true); DoEvent | A then B invoked; C skipped; result true |
| SS-V16 | No handler consumes an event | DoEvent returns false |
| SS-V17 | DisableUpdates, enumerate targets; EnableUpdates, enumerate again | disabled pass invokes no target callback; enabled pass resumes callbacks |
| SS-V18 | Manager Init with missing sound service, then call Init again | first false; second may return true from retained initialization flag; no successful service acquisition is implied |
| SS-V19 | Missing SystemSndID mapping | result zero |
| SS-V20 | Both freshness GetDirector entry points, repeatedly | same singleton identity and one construction |
| SS-V21 | Valid CR/LF-terminated extended playlist header, then EOF between records | extended parser returns true |
| SS-V22 | CR/LF-terminated extended record header with no following path, or non-EXTINF record header | false; earlier records retained by parser |
| SS-V23 | Playlist with no recognized first header, containing one CR/LF-terminated path | plain-M3U selection retains that first path as parser input |
| SS-V24 | Track resource of neither supported write type | Write returns false; do not use that result to infer the Read selector |

## 11. Open items

See [the question ledger](questions/sound-system.md). This specification separates
confirmed forwarding/state contracts from unresolved decoder and scheduling
details; unresolved items are not implementation latitude.
