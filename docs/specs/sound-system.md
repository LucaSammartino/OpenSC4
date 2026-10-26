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
