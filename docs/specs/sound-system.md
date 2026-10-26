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
The exported named-vtable index does not enumerate these tables; their full
slot inventory is SS3, not an assumed copy of the track table.

| Named operation | Address | Confirmed boundary |
|---|---|---|
| CreateSnd | `0x0093F304` | obtains a backend sound and associated source interface |
| UpdatePitch | `0x0093F07E` | source pitch update; numeric law remains SS3 |
| UpdateVolPan | `0x0093F136` | source volume/pan update; numeric law remains SS3 |
| UpdatePatch | `0x0093F687` | patch/source refresh |
| NoteOn | `0x0093F806` | refresh patch, load then play source |
| Load | `0x0093F84D` | refresh patch and forward source Load |
| HandleNotifySetPropertyValue | `0x0093FA5B` | property notification entry; full dispatch remains SS3 |

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

## 5. `cHitIniFile` and `cHitPropertySetTrackResource`

### 5.1 INI adapter

`cHitIniFile` is CLSID `0x3D802D67`. Constructors `0x0093638E` and
`0x009363F5`, Create `0x00936480`, destructors `0x00936458`/`0x009364DE`.
It combines the existing INI-file implementation with the HIT initialization
interface at `+0x4C`, table `0x00AD2394`. That interface's slots 0–7 are
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
in [audio.md §10](audio.md#10-sound-data). Full grammar, initialization
ordering, duplicate keys and partial-failure aggregation remain SS4.
No drawing or asynchronous message-consumer contract is exposed by this adapter.

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
notifies registered audio renderers using the stored master volume
(`0x009675DF`); no visual drawing operation is involved.

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
