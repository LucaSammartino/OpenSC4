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
the complete mode table and bytecode interpretation remain SS1.

### 3.3 Properties, errors and commands

*[compat]* Sound priority, volume and pan are register/property-backed, at
IDs `0x13`, `0x15`, `0x16`; original frequency is read at `0x49`
(`0x0093E453–0x0093E4D0`). These getters are not independent cached fields.

*[compat]* CheckedRegId compares its signed 32-bit argument to `0x4A`.
Values greater than 74 report `Illegal Register Id` through the HIT manager
and substitute register 1; negative values pass this particular check.
This is not a general guarantee that every negative index is valid.
Evidence: `CMP`/signed `JLE` at `0x0093A7EF–0x0093A7F5`.

DoCommand consumes an opcode from track data and supports register loads,
notes, control and timer-driven continuation. It is not a text-command API.
The inspected initial cases include register-based NoteOn with a missing-patch
error (`Could not set patch - file or resource not found`), immediate NoteOn,
and byte/long register loads. Complete operand widths, opcode semantics,
branching and error recovery are **unsettled** (SS1); an engine must not assume
ordinary MIDI semantics or treat unknown commands as successful no-ops.

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
routine is `0x00938419`. The folded slot 7 must not be identified as Unpause
without checking its calling contract (SS2). Primary/service tables inherit
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
file grammar, duplicate registration behavior, semaphore/ducking arbitration,
pause behavior and source callback payloads remain SS2. Do not infer them from
the names in the inventory. These are direct calls/callbacks, not demonstrated
SC4 message-server subscriptions.

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

The constructor `0x00939750` builds a property set and copies a supplied
three-word resource key; it is not an empty anonymous track. Key setters are
SetKey `0x009377AF`, SetType `0x0094052A`, SetInstance `0x00940536`;
GetKey is `0x00940512`. QI/Release are `0x00939796`/`0x009397CA`,
GetRefCount `0x00939792`, destructor `0x00939727`, assignment `0x00939FFC`,
and CreatePrivateCopy `0x00939D62`.

Primary property table `0x00AD2898`: slots 0–2 QI/AddRef/Release;
3–9 SetValue, ClearValue, ClearAll, FirstKeyAndValue, NextKeyAndValue,
Value, HasValue; 10 folded count getter; 11 destructor; 12–13 folded
Read entries; 14–15 unnamed entries; 16–18 folded debug entries.
Resource table `0x00AD285C`: slots 0–2 QI/AddRef/Release thunks;
3 GetKey, 4 SetKey, 5–7 folded key-component accessors,
8 SetType, 9 folded component setter, 10 SetInstance, 11 GetRefCount,
12–14 unnamed resource operations. The export also lists tables
`0x00AD28E4`/`0x00AD2920`; these must not be conflated without checking
which resource constructor installs them (SS5).

*[compat]* Write `0x0093FF4A` selects an output path by resource type:
`0x0B8D821A` selects the old path, `0x0B9EB87E` the other path;
other types report false. Read `0x00940DFF` installs the resource instance
as property `0xFF6F9546`, then selects legacy input for `0x0B8D821A`
and the property serializer for other types. This asymmetry is intentional
to the observed boundary; do not reject every nonlegacy read solely because
Write would reject its type. Write_OldFormat `0x00940E5C` starts with
`TKDT,2,`; ReadLegacyFormat is `0x00941930`. Complete field order, escaping,
version handling, malformed-input behavior and copy ownership remain SS5.
See [audio.md §10](audio.md#10-sound-data) for resource-factory selection.

## 6. `cFreshPlayer` and `cFreshCellPlayer`

### 6.1 Score player

Create `0x009456A4`, constructor `0x009489AE`, QI `0x00946FE6`, Release
`0x00947004`, destructors `0x00948C54`/`0x00948C98`. CLSID `0x3C1CF59E`.
Constructor state includes no score/timer, clear initialized/playing flags,
volume `0x400`, empty cell-player storage, seeded private randomness and
spatial/division configuration. The 4096 cell-pointer entries use a row
stride of 64; this is storage capacity, **not** permission to read arbitrary
64×64 scores. Accepted score dimensions and bounds checks remain SS6.

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
to the owner and imports score configuration (`0x0094701E`). Full overload
argument contracts remain SS6. LoadScore is `0x00948C11`.

*[compat]* Play() returns true even if not initialized. When initialized
and previously stopped, it derives player settings from the score, starts
a timer, sets score starting coordinates, marks playing, and registers the
update interface with sound-system period `0xFA` (250) if that service exists
(`0x00947DB8`). Consequently, success alone cannot verify audible music.
SetZoom and SetPlayPercent also return true when uninitialized, but only
store the requested value when initialized (`0x0094776D`, `0x0094777F`).
Update(volume) forwards the volume then invokes Play() (`0x00947150`);
this method is not the periodic cell-selection algorithm.

State changes use a shared freshness critical section. Volume fading uses
a callback timer (`0x0094832A`, `0x0094761C`); callbacks can skip work when
that lock is already held. Exact fade divisions/clamping, cell-selection
probabilities, tempo timing, coordinate rounding, reseeding and stop/shutdown
ordering remain SS6. In particular, the routine named ProbOfCellStarting
at `0x009477A6` visibly counts busy cell players; its name is not sufficient
evidence for a probability law. There is no demonstrated SC4 message-switch
for this owner: scheduling enters through its update interface and callbacks.

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
protocol and special loop/countdown branch remain SS7.

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
`0x00946C2C`, UpdateScaledVol `0x00945820`; their remaining policies are SS7.

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
| 18–27 | folded entry, SetDataDirectory, directory/extension mapping operations (SS8) |
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
conversion and the unlisted tail/folded entries remain SS8.

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
their queues; UnregisterTarget removes the matching target from both active
lists, or defers removal during enumeration (`0x00967DD4`, `0x00967E4F`).
The exact period classifier and timer boundary remain SS8. SetMasterVolume
notifies registered audio renderers using the stored master volume
(`0x009675DF`); no visual drawing operation is involved.

### 7.2 Framework manager

The method/address inventory and sound-id transformation remain in
[audio.md §8.6](audio.md#86-cgzsndmgr--the-framework-sound-manager).
The export lacks usable bodies for most `cGZSndMgr` methods; this section
adds instruction-confirmed facts, not guessed implementations of those bodies.
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
GetSound/SndAutoPlay resource/cache/creator selection and shutdown reference
ordering remain SS9. This service is invoked through interfaces; no additional
SC4 message IDs are established by these methods.

## 8. Directors

These are class-factory/lifecycle objects, not sound sources. They inherit
the COM director's framework and class-registration contract. The named
owner-specific operations below augment that common table; in the director
interface OnStart is slot 4 (`+0x10`), as in [audio.md §1](audio.md#1-identity).
Other inherited lifecycle slot numbers are not reconstructed from names here.

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
PostAppInit returns false when the sound system is absent; when present it
uses the property-cache interface and value `0xCC3014EE`. The meaning of that
cache operation and its missing-cache behavior remain SS10.

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
Complete inherited vtable slots and factory-failure/aggregation conventions
remain SS10; their names must not be used to invent such behavior.

## 9. `cGZPlayList`

This is a text parser, not an audio player or GZCOM service. Named operations
are construction/destruction `0x008D569F`/`0x008D56C8`, Open `0x008D6B22`,
_GrabSegment `0x008D6056`, _StripEndSegment `0x008D60C4`, _IsSpec
`0x008D6123`, _AddNextPathSegment `0x008D6196`, _GetLine `0x008D6258`,
_MakeAbsolutePath `0x008D6352`, _ParseExtM3UFormat `0x008D64E3`,
_ParseM3UFormat `0x008D66C9`, _ParsePLSFormat `0x008D6824`.
No sound-playback, window-drawing or message-handler vtable is demonstrated
for this owner. Any proposed COM slots require evidence (SS11).

*[compat]* Open selects extended M3U for a first header `#EXTM3U`, PLS for
`[playlist]`, otherwise plain M3U. In the plain case it restores the content
position so that the first line is parsed as input, not discarded as an
unrecognized header. The format is content-selected; the filename extension
alone is not the selector (`0x008D6B22`).

*[compat]* _GetLine trims the obtained line and applies the framework string
encoding conversion (`0x008D6258`). Path resolution is based on the playlist
directory; _MakeAbsolutePath processes path segments and allows an absolute
specification to replace the current base (`0x008D6352`). Drive-letter syntax
is recognized by a colon in the second character (`0x008D6123`). Exact UNC,
separator, dot-segment and encoding corner cases remain SS11.

*[compat]* Extended M3U requires each record header to begin `#EXTINF:` and
then consumes a following path line, resolves it, and appends a record.
Its numeric metadata uses the C-library decimal-long conversion. EOF before
the next record is success; EOF where a record's path is required, or a
record header without the required prefix, returns false. Previously appended
entries are not rolled back by this parser (`0x008D64E3`). Full title storage,
plain-M3U comment behavior, PLS ordering/version rules, Open's reset/append
semantics and file IO failure behavior remain SS11. Do not replace this with
a standards-compliant general-purpose playlist parser without compatibility
tests: the observed extended parser has no general comment-skipping loop.

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
| SS-V21 | Valid extended playlist header, then EOF between records | extended parser returns true |
| SS-V22 | Extended record header with no following path, or non-EXTINF record header | false; earlier records retained by parser |
| SS-V23 | Playlist with no recognized first header, containing one path | plain-M3U selection retains that first path as parser input |
| SS-V24 | Track resource of neither supported write type | Write returns false; do not use that result to infer the Read selector |

## 11. Open items

See [the question ledger](questions/sound-system.md). This specification separates
confirmed forwarding/state contracts from unresolved decoder and scheduling
details; unresolved items are not implementation latitude.
