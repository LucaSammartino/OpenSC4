# GZ framework core

Target: SimCity 4 Deluxe, Windows GOG 1.1.641. Static evidence checked 2026-10-23
by lane `re-gzservices5`. Addresses identify this executable only. Slots are zero-based;
on the 32-bit interfaces their byte offsets are four times the slot number.

*[compat]* marks observed requirements. *[choice]* marks implementation freedom.
This is a bounded specification: unresolved contracts are explicitly listed in
[the question file](questions/gz-framework-core.md). In particular, the command
support registration inventory is complete here, but its game-specific command
parameter schemas are not. Do not interpret a registered command name as a schema.

## 1. Boundaries and dependencies

| Owner | Responsibility | Existing authority to use with this document |
|---|---|---|
| `cGZFramework` | application lifetime, service registry, framework callbacks and quit | [application](application.md), especially §§2–5 |
| `cGZCOM` | class-provider lookup, library lifetime, ID allocation | [system service pointers](system-service-pointer.md) for acquired service references |
| `cGZCommandServer` | command registry and delegation | [command system](command-system.md), §§1–5, for parameter sets, generators and dispatcher |
| `cGZMiscCommandSystemSupport` | command/message adapter for framework and application operations | command system §6; [Lua host](lua-host.md) for console Lua execution |
| `cGZToolCommandSystemSupport` | command/message adapter for interactive tools | [city tools and input](city-tools-input.md) for input-control stack and lot placement |
| `cGZBuffer` | pixel storage exposed to drawing interfaces | [8-bit span renderer](span-render-8.md), §§10.4 and 10.7; [UI windows](ui-windows.md) for drawing consumers |
| `nGZLog::cService` | log backend registry and serialized delivery | [output streams](output-streams.md), especially log creation and file-output contracts |
| `cGZFileSystem` | cached application/data/plugin/CD paths | application §4.2 for service installation |
| `cGZScriptTerp` | legacy tagged-text interpreter with vocabulary and target callbacks | Lua host describes a separate interpreter; its Lua syntax does not apply here |

Message queues, message ownership, notification priorities and synchronous versus
queued delivery remain specified by [framework message services](framework-gz-services.md).
These core services do not introduce a second message-queue algorithm. None of the
non-buffer owners is a pixel renderer. Paths and scripts are their IO contracts;
logging writes through backend/stream interfaces.

*[choice]* Core implementations may use native collections, synchronization and file
APIs. Reproduce the externally visible ordering, reference ownership and return values
below; internal STL layout and process-global addresses are not portable requirements.
No C ABI export is introduced by this specification.

## 2. Framework (`cGZFramework`)

### 2.1 Identity, inventory and state

Construction is at `0x0087AD42`; the primary table is `0x00AC3090`, with a
secondary face at `0x00AC3070` installed at object offset `+4`. QI at `0x0087931B`
accepts IID `1` and `0x65` on the primary face, and `0x23EA70A1` on the secondary
face. Success adds a reference. An unsupported IID returns false without writing
the output. The framework owns an embedded COM service and registries of services
and callback participants. Its reference count starts at zero. *[compat]*

The table below deliberately uses descriptive labels where export symbols are
absent. A target address identifies a slot, not a complete recovered signature.
Rows marked open require GZF1 before implementing that interface entry.

| Slots | Entry points / inventory |
|---|---|
| 0–2 | QI `0x0087931B`, AddRef `0x00879355`, Release `0x00879383` |
| 3–6 | add service `0x0087AC3C`, remove service `0x0087A90E`, service QI lookup `0x0087A0AC`, enumerate services `0x0087A106` |
| 7–8 | add/remove callback participant `0x0087A1F4` / `0x008798AA` (callback kind open) |
| 9–10 | ordered participant add/remove `0x00879BB9` / `0x00879C6C` (callback kind open) |
| 11–12 | add/remove tick participant `0x00879D21` / `0x00879DB3` |
| 13–14 | get/set positive scheduling count `0x0087946C` / `0x0087944F` (semantic name open) |
| 15–16 | `0x0087A74B` / `0x0087A812` (open) |
| 17–18 | `0x00472E30` / `0x00472E40` (application uses slot 18 as enable/disable; full semantics open) |
| 19–20 | quit `0x0087963D`, abortive quit `0x0087A610` |
| 21–23 | embedded helper accessor `0x00879479`, global framework accessor `0x008793F2`, embedded COM accessor `0x008793F9` |
| 24–29 | `0x00472E50` (open), get debug level `0x00879480`, two aliases `0x00879487`, set level `0x008794BF`, propagated SetDebugLevel `0x0087948C` |
| 30–35 | `0x00949B2B` (open), stream setters `0x008796BD`, `0x0087970D`, `0x0087975D`, stream operation `0x0087950F`, SetStream `0x008797AD` |
| 36–39 | set application `0x00879427`, get application `0x00879473`, exception report `0x00879685`, `0x00949B32` (open) |
| 40–46 | `0x0087AC20`, lifecycle callbacks `0x00879F15`, `0x00879FBC`, `0x0087A046`, `0x0087A4B6`, `0x0087A527`, `0x0087A59F` (full signatures/order open) |

The inventory was recovered from the constructor-installed table, including unnamed
entries; it is not a claim that the export's named-vtable index covers this class.
Nonvirtual entry points are `sSetFrameWork` `0x008793BD`, `sSetApplication`
`0x008793DD`, `sInit` `0x0087AF13`, `sRun` `0x0087957A`, and `Main` `0x0087B0B9`.

*[compat]* Construction initializes the scheduling count to 4, debug level to zero,
phase to zero, and mutation/iteration flags to false. AddRef returns the incremented
count under the framework lock. Release returns zero when the count is less than
two; otherwise it decrements and returns the result. This entry does not delete
the framework when returning zero (`0x00879355`, `0x00879383`). In particular,
the stored count remains one when Release returns zero at count one; the unsigned
comparison and conditional decrement are at `0x008793A0`–`0x008793AE`.

### 2.2 Service and callback ownership

*[compat]* Service addition rejects null, duplicate service IDs, and attempts while
the service registry is being iterated or mutated (`0x0087AC3C`). Successful
registration keeps owning references and indexes the service by its service ID;
lookup takes service ID, requested IID and output, and asks the registered service
for that IID. A missing service returns false. A lookup is not a class instantiation
request (`0x0087A0AC`). Removal is blocked during protected registry activity and
also removes the service from tick/ordered callback lists (`0x0087A90E`).

*[compat]* Tick registration uses identity to reject a second insertion of the same
participant and retains a reference. Removal returns false for an absent participant.
Mutation while that callback list is protected is rejected (`0x00879D21`,
`0x00879DB3`). The ordered participant list avoids duplicate participant IDs and
inserts according to its supplied ordering value (`0x00879BB9`); tie handling and
the callback interface's semantic name remain GZF1.

### 2.3 Application lifetime and failures

*[compat]* `Main` attempts initialization; it runs the main loop only if initialization
succeeds. It performs framework shutdown after either path and returns the process
result (`0x0087B0B9`). The detailed app hooks and installed service order are in
application §§2–5. `sInit` resets the phase, prepares the embedded COM service,
registers its system-service face, and runs the application's setup hooks before
its Init. Failure of app Init or the later framework initialization stage sets
the process result to `0xFFFFFFFF` and reports false (`0x0087AF13`). Exact partial
unwind obligations and platform loop pacing are GZF1; this does not authorize
inventing a rollback sequence.

*[compat]* Normal quit either posts the OS quit request with the supplied result,
or, in the alternate loop mode, sets a pending-quit flag and stores that result
(`0x0087963D`). Abortive quit invokes the abortive cleanup path and exits the
process with the supplied result (`0x0087A610`). An implementation needs a host
boundary for this behavior. *[choice]* A test host may intercept termination to
record the requested result.

*[compat]* Setting the scheduling count below one stores one (signed test and
store at `0x00879453`–`0x0087945F`).
The basic debug-level setter stores the supplied value; the getter returns it.
Exception reporting uses `??? Unknown exception ???` for a null description,
temporarily disables the framework through slot 18, presents an exception dialog,
and re-enables it (`0x00879685`). *[choice]* The dialog's native presentation is
host-specific; the text and disable/report/enable ordering are the contract.

## 3. COM (`cGZCOM`)

### 3.1 Interfaces and inventory

*[compat]* Primary IID `0x4241330D` returns the primary face with an added reference
(`0x0087B35B`). Other queries delegate to the system-service implementation. The
primary table is `0x00AC3368`; the system-service and lifetime faces are
`0x00AC3334` and `0x00AC331C` respectively. Do not treat folded names in those
tables as the semantic names of their slots.

| Slot | Primary method | Address |
|---|---|---|
| 0–2 | QI, AddRef, Release | `0x0087B35B`, `0x0093056F`, `0x0087B37D` |
| 3 | GetClassObject | `0x0087BE75` |
| 4 | FrameWork | `0x009D41A1` |
| 5–6 | AddLibrary, FreeUnusedLibraries | `0x0087C081`, `0x0087B626` |
| 7–8 | CreateGuid, CreateGuids | `0x0087B4C0`, `0x0087B532` |
| 9–11 | RealInit, RealShutdown, SetServiceRunning | `0x0087BCDD`, `0x0087B8EC`, `0x0087B385` |
| 12–13 | EnumerateLibraries overloads | `0x0087B70F`, `0x0087B6C4` |

Other helpers: constructor `0x0087B9A1`, destructor `0x0087BA12`,
CanUnloadLibrary `0x0087B38D`, FreeAllLibraries `0x0087B67B`, GetLibObject
`0x0087BDA6`. Exact callback signatures and the ID clock epoch are GZF2.

### 3.2 State, lookup and failures

*[compat]* The service keeps a library collection, a class-to-library index, the
framework pointer, the process DLL director, initialization state, a lock, and
ID-generator state. RealInit is idempotent after success. On the first call it
acquires the framework and DLL director and initializes that director with the
COM interface and current application path. Director failure returns false and
does not mark initialization complete (`0x0087BCDD`).

*[compat]* GetClassObject first consults its class index. A mapped class is obtained
through its library; an unmapped class is offered to the process director, with
the framework interface query as a fallback (`0x0087BE75`). This fallback is why
service interfaces and class-provider lookup must remain distinct operations.

*[compat]* Adding an already recorded library is successful without repeating its
initialization. A new library that fails to load reports false. When a loaded
library supplies a director, the director is notified of this COM service
(`0x0087C081`). CanUnloadLibrary permits unloading an unloaded library; a loaded
library's director decides whether it can unload (`0x0087B38D`). Do not assume
FreeUnusedLibraries forcibly removes live providers. Library failure-cache and
shutdown details require GZF2 before replacing the dynamic loader.

*[compat]* The so-called GUIDs are 32-bit values, not 128-bit platform UUIDs.
Single and batch generation are locked and rate-limited by stored time state.
A zero-length batch succeeds without producing values (`0x0087B532`).
The instructions confirm dword writes and unsigned bit shifts (`0x0087B55E`–
`0x0087B5AB`); the epoch, clock wrapper and wraparound policy remain GZF2.
*[choice]* Until GZF2 is answered, inject an ID provider at the core boundary
instead of claiming an unrelated UUID algorithm is compatible.

## 4. Command server

### 4.1 Inventory

Primary table `0x00AD17A0`, IID `0xCB90253A`, service ID `0xEB903A32`.
Secondary system-service table `0x00AD176C` routes Init/Shutdown through
adjusting thunks; the lifetime face is `0x00AD1754`. *[compat]*

| Slots | Methods and addresses |
|---|---|
| 0–5 | QI `0x0092980D`, AddRef `0x0094BC3F`, Release `0x00920BF0`, Init `0x0092A982`, Shutdown `0x0092AE88`, CreateCommandParameterSet `0x0092A9F8` |
| 6–10 | RegisterCommand `0x0092B075`, RegisterCommands `0x00929844`, RegisterCommandParameterInfo `0x0092AD10`, UnregisterCommand `0x0092AD88`, UnregisterCommands `0x00929881` |
| 11–14 | GetCommandInfo `0x0092A1F6`, GetCommandID `0x00929EE0`, GetCommandIDFromCommandString `0x0092AA4A`, EnumerateCommands `0x0092AAD8` |
| 15–18 | RegisterCommandGenerator `0x0092AB0B`, UnregisterCommandGenerator `0x0092A236`, GetCommandDispatcher `0x009DB663`, SetCommandDispatcher `0x00929B92` |
| 19–24 | two ExecuteCommand overloads `0x00929BBA` / `0x00929BA8`, ExecuteCommandFormatted `0x009298AE`, ConvertStringToCommand `0x00929BCC`, ConvertCommandToString `0x00929BDD`, ConvertCommandOutputToString `0x00929BEE` |

### 4.2 State and registration

*[compat]* Construction creates an empty command map and generator set, null
active/default dispatcher and default string-generator pointers, and a false
parameter-description generation flag (`0x0092AF7C`). Init installs the default
dispatcher if none is active and creates/registers a string generator on every
call; it returns true. The exact behavior of the generator and dispatcher is
command system §§3–5, including the repeated-Init quirk.

*[compat]* RegisterCommand rejects ID zero, null name and an existing ID. It does
not replace an existing entry. Success stores the ID/name, processes the supplied
parameter text and notifies generators of registration (`0x0092B075`). Name
lookup and parameter-set creation retain the rules in command system §5.
RegisterCommandParameterInfo returns false for an unknown ID. Nonempty text
replaces the description. With null/empty text, auto-description is generated
only when the generation flag is enabled, using `Format: %s\nOutput: <none>`
(`0x0092AD10`).

*[compat]* Shutdown clears the command registry, clears the active dispatcher,
shuts down/releases its default dispatcher and generator if present, and clears
the generator collection; it returns true (`0x0092AE88`). Execution delegates
through the active dispatcher and string conversion through registered generators;
the server does not directly execute game tools or Lua. Null-dispatcher execution,
bulk partial failure and enumeration cancellation are GZF3.

## 5. Command-system supports

### 5.1 Common lifetime and message contract

Both supports have QI/AddRef/Release in slots 0–2 (`0x005C3E30`, `0x005F47F0`,
`0x005C3E50`), DoMessage in slot 3, deleting destructor in slot 4, FinalRelease
in slot 5 (`0x005C3E70`), Init in slot 6 and Shutdown in slot 7. Concrete tables:
misc `0x00AA1CE4`, tool `0x00AA1AFC`. The tool table `0x00AA1648` has purecall
in slot 3 and is not the concrete dispatch table. *[compat]*

*[compat]* Both start with count zero and initialized=false. Their notification
helpers are populated during construction. Init is guarded by initialized;
it sets the flag, subscribes the notification list and registers names if the
command service is available. It still returns true when that service is absent.
Shutdown removes registrations/subscriptions only when previously initialized,
and returns true. These are message-target adapters: command ID is message type,
input parameter set is data1, output parameter set is data2, as in command system
§4. Their registration alone does not create a separate queued execution path.

### 5.2 Misc support

Constructor `0x005C82E0`, Init `0x005C48E0`, Shutdown `0x005C7F40`, DoMessage
`0x005C8B30`, destructor `0x005C84F0`. Additional state includes retained command
and message-service interfaces, an initially empty pending-wait collection, and
a private random generator constructed with seed argument zero. *[compat]*

The complete name/ID registration inventory from Init follows. All registrations
pass null parameter-description text. The constructor's notification list repeats
`0x0BCE8AA5` and omits `0xABCE8AA7`, although Init registers both names. Preserve
that distinction pending confirmation of notification-helper duplicate handling
(GZF4); do not silently normalize the constructor list. *[compat]*

| ID | Name | ID | Name |
|---|---|---|---|
| `0xABAC4EE4` | ListCommands | `0x2BB40833` | SetDebugLevel |
| `0x6BBC0DAD` | CreateGZLog | `0x8BA12D18` | GZLog |
| `0x4BC4581C` | SetGZLogLevel | `0x8BBD797F` | Assert |
| `0x0BA12D1B` | RZCheckHeap | `0xABAC067C` | ExecuteScript |
| `0x4C149310` | GetDateAndTime | `0xEBAC13FA` | GetTimerTime |
| `0x2BAC2385` | GetFrameCount | `0x0BBEF8A5` | GetFrameRate |
| `0xABAC26B5` | ExecuteCheat | `0xCBAC32F4` | WaitForMessage2 |
| `0xEBAC3957` | CancelWaitForMessage2 | `0x0BBC1580` | GetPathDirectory |
| `0xEBBC158B` | GetPathFileName | `0xCBB14BDC` | GetDirectory |
| `0x8BB14DCB` | GetRandomNumber | `0x8BB14DD6` | GetRandomInteger |
| `0x2BB17574` | TakeSnapshot | `0xCBB17C11` | GameDelay |
| `0xCBB17C12` | GamePause | `0x4BB17C4D` | ReadRegistry |
| `0x2BB17C4E` | WriteRegistry | `0x0BB17C64` | ViewWebBrowser |
| `0xABB17C74` | SendMessage | `0x0BB5C013` | GetAppState |
| `0x6BB9BBE3` | SetScriptAutoYield | `0x0BC24F90` | CreateException |
| `0x0BCE8AA5` | GetPopupModalDialogsSafe | `0xABCE8AA7` | GetPopupModalDialogsEnabled |
| `0x6BCE8AE6` | SetPopupModalDialogsEnabled | `0x6BCECFCE` | GetOccupantCount |
| `0xCC363667` | GetMemInfo | — | — |

IO delegation: CreateGZLog uses the log/file stream creation contract in output
streams; ExecuteScript uses the Lua script server described in Lua host, not
the tagged-text interpreter in §9. Input/output parameter IDs, pending wait
completion and command-specific errors remain GZF4. The status values mentioned
in command system §1.5 are still explicitly inferred there; they are not upgraded
to proven universal status codes by this inventory.

### 5.3 Tool support

Constructor `0x005C5090`, Init `0x005C45A0`, Shutdown `0x005C4680`, DoMessage
`0x005C5380`, destructor `0x005C5110`, drag helper `0x005C4740`.
Init registers and construction subscribes to the following seven IDs. *[compat]*

| ID | Command |
|---|---|
| `0xCBADE58E` | PlaceNetwork |
| `0x2BADF573` | PlaceNetworkIntersection |
| `0x2BB48D26` | PlaceLot |
| `0xEBB48D31` | PlaceZone |
| `0x2BB48D41` | Demolish |
| `0x0BB9853C` | GetViewTarget |
| `0x8BB756A4` | SetViewTarget |

The existing input-control contracts apply after dispatch into a tool; they do
not settle the adapter's parameter schema. Coordinate units, drag sequencing,
view-target conversion and missing-view failures remain GZF5. Implementers can
implement the registration/lifetime contract independently of those open adapters.

## 6. Buffer (`cGZBuffer`)

*[compat]* CLSID `0xC470D325`, primary table `0x00AC1400`. This class already has
substantial coverage in span-render-8 §§10.4 and 10.7 and the format-specific
[1555](span-render-1555.md), [4444](span-render-4444.md),
[565](span-render-565.md), and [8888](span-render-8888.md) drawing specifications.
Use those for actual rendering arithmetic rather than deriving it from the
framework service classes.

Interface inventory required by that bridge: slots 0–3 QI/AddRef/Release/Init;
6–7 Lock/Unlock; 9–11 GetWidth/GetHeight/GetRect; 15 GetColorType;
17 GetBytesPerPixel; 26 GetColorKey; 30 RGB mapping. Exact native signatures
and return channels are tabulated in span-render-8 §10.7. The remaining table
targets, including storage and conversion helpers, need semantic reconciliation
before exposing them (GZF6). There are no ordinary message subscriptions here.

*[compat]* Init rejects a second initialization and either zero dimension.
Storage is row-major, each row padded to a four-byte boundary, and allocated
without clearing. Thus 10×3 pixels at 8 bpp have stride 12 and occupy 36 bytes.
Lock without storage fails; otherwise lock nesting is counted, flags accumulate,
and bit `0x8000` increments the modification counter. Unlock decrements nesting,
clears lock flags and returns true. A type-2 8-bit buffer has no palette; RGB
mapping returns zero for that type. Consequently the linked renderer's solid
fill writes zero even when a nonzero drawing color was selected. These rules
and their instruction evidence are in span-render-8 §10.4.

*[choice]* Core memory need not expose the original object offsets. A native
adapter must still obey the documented lock, stride, ownership and pointer
lifetime contracts. Allocation failure, resize and foreign-storage attachment
outside the covered bridge are GZF6, not implied successful operations.

## 7. Logging (`nGZLog::cService`)

### 7.1 Interfaces, inventory and defaults

Service ID `0x09B69C4E`, logging IID `0x09B69C46`; the logging face table is
`0x00AD20D0`, at object offset `+0x1C`. The system-service primary table is
`0x00AD2130`; its Init/Shutdown slots 8/9 are `0x009342C9` / `0x00935228`.
The secondary lifetime table is `0x00AD2118`. Constructor `0x00934C88` starts
with empty backend/deferred collections, default level `0x7FFFFFFF`, initialized
false, null internally owned output helpers, and recursion depth zero. *[compat]*

| Logging-face slots | Inventory |
|---|---|
| 0–2 | QI/AddRef/Release adjusting thunks |
| 3–5 | AddOutputBackEnd `0x00934D4B`, RemoveOutputBackEnd `0x00934E3B`, RemoveAllOutputBackEnds `0x00933E24` |
| 6 | accessor `0x009CDC23` (semantic name open) |
| 7–9 | AddOutputStream `0x00933C77`, GetOutputBackEnd `0x0093422F`, SetOutputLevel `0x00935042` |
| 10–13 | FilterLevelFromString `0x00933A27`, StringFromFilterLevel `0x00933AAE`, LogLevelFromString `0x00933AE6`, StringFromLogLevel `0x00933B6E` |
| 14–17 | DumpOutputLevels `0x00933FAA`, accessor `0x009D858F`, Output `0x00935148`, OutputLevel `0x009351B8` |

Additional delivery helpers are OutputLowLevel `0x00934A6E`, formatted Output
`0x00934F88` and OnTick `0x00934F05`. Output-stream creation and file flushing
belong to output-streams, not to the log router itself.

### 7.2 Routing and reentrancy

*[compat]* Low-level output visits registered backends and asks each backend's
filter (slot 3) whether it accepts the category/level before invoking its output
slot 7 with category, level and text. The router releases its lock around backend
callbacks. Output arriving from a different thread, or recursively during a
backend callback, is copied into deferred storage instead of immediately invoking
that backend again (`0x00934A6E`). Text must therefore remain valid after the
original caller returns; retaining a borrowed pointer is insufficient.

*[compat]* OnTick drains deferred messages in FIFO order, releases the queue lock
before delivering each item, and returns true (`0x00934F05`). Low-level delivery
also drains its local deferred collection. Its recursion guard drops delivery
when the incremented depth exceeds three (unsigned comparison after dword
increment, `0x00934AF4`–`0x00934B04`). Foreign-thread records use the queue at
`+0x24`; reentrant owner-thread records use `+0x20` (`0x00934A8E`–`0x00934AC1`).

*[compat]* SetOutputLevel with a backend name targets that backend. Without a
backend name it updates all registered backends and remembers either the global
level (null category) or a category-specific setting (`0x00935042`). Filter and
log levels have different sentinel meanings: filter `0` means `all`, filter
`0x7FFFFFFF` means `none`; log `0` means `none`, log `0x7FFFFFFF` means `all`.
The string-to-level parsers compare names without case; null, empty and unknown
names return −1. The full named-level table and numeric conversion are GZF7.
String conversion of an invalid filter/log value returns respectively
`invalid filter level` / `invalid filter level` (the latter wording is also used
by StringFromLogLevel, `0x00933B6E`).

*[compat]* Shutdown is guarded by initialized, removes the framework tick
registration, releases its backend collection and deletes the internal output
helpers, then returns true (`0x00935228`). Backend-name collisions, output
format truncation/allocation failure and initialization configuration remain GZF7.

## 8. File-system service

### 8.1 Inventory and state

Service construction `0x008907AA`; QI `0x0089068A`. File-system face table
`0x00AC5120`; system-service table `0x00AC5190`, lifetime table `0x00AC5178`.
The file-system IID is `0xE0203660`; QI returns its face at `+0x1C` and adds a
reference (`0x0089068A`). Its service ID is `0x2073215D`, supplied to the base
service constructor by `0x008907AA`. Application §4.2 lists the interface ID.

| File-system slot | Method / address |
|---|---|
| 0–2 | QI/AddRef/Release thunks |
| 3–6 | AppDrive `0x008906B4`, AppDirectory `0x008906CE`, AppName `0x008906E8`, AppExtension `0x00890702` |
| 7–8 | DataDirectory `0x00890A32`, PlugInDirectory `0x00890B8D` |
| 9–12 | CDAppDrive `0x0089071C`, CDAppDirectory `0x0089075F`, CDDataDirectory `0x00890778`, CDPlugInDirectory `0x00890791` |
| 13–17 | FindCDDrive `0x00890E98`, UseCDIniFile `0x00890880`, AllowRecursiveCDSearch `0x0089088D`, SetExpectedCDName `0x0089089A`, SetExpectedCDAppPath `0x008908A5` |
| 18–21 | SetDataDirectory `0x008909AA`, SetPluginDirectory `0x008909EE`, GetPluginDirectoryName `0x008908D3`, SetPluginDirectoryName `0x008908B0` |

*[compat]* The constructor creates empty cached path strings, sets plugin directory
name to `Plugins`, and initializes its path-resolution flags to false. Application
components and data/plugin locations are cached separately; changing a plugin
directory name is not itself a filesystem directory creation request. No file
enumeration or file-content read/write API is present on this face.

### 8.2 Resolution and failure

*[compat]* DataDirectory and PlugInDirectory lazily compute and cache their
results. Both consult the application's INI file, section `Directories`;
the plugin key is `PlugIn`. A nonempty configured path is resolved relative to
the application's directory; an empty data setting falls back to that directory,
while an empty plugin setting appends the configured plugin directory name.
Resolution marks the cache valid even when no configured setting was found
(`0x00890A32`, `0x00890B8D`). Exact INI filename construction and the data key's
literal are GZF8.

*[compat]* SetDataDirectory first stores the supplied string and then checks
directory existence. It returns the existence result and sets its cache-valid
flag on success (`0x008909AA`). Failure is therefore not an atomic rejection of
the supplied path. SetPluginDirectory uses the corresponding pattern
(`0x008909EE`). CD discovery/validation has its own expected-volume name,
application-path and recursion/INI options; search ordering, validation rules
and no-disc return values are GZF8. *[choice]* A platform adapter may report
absence of a CD device, but must not fabricate a validated CD path.

## 9. Legacy script interpreter (`cGZScriptTerp`)

### 9.1 Interface, state and ownership

Primary table `0x00AD4718`: 0 QI `0x00944650`, 1 AddRef `0x00968B60`, 2 Release
`0x008D29B9`, 3 SetScriptTarget `0x0094451C`, 4 SetContext `0x00944589`,
5 GetContext `0x009D8564`, 6 ExecuteScriptString `0x00945229`,
7 ExecuteScriptFile `0x00945198`, 8 ExecuteScriptResource `0x00944C72`,
9–10 AddToken `0x0094489C` / `0x00944891`, 11 RemoveToken `0x009448CE`,
12 RemoveAllTokens `0x009448D9`, 13 PushVocabulary `0x00944C4B`,
14 PopVocabulary `0x009448E4`, 15 deleting destructor `0x00944C2F`.
Constructor defaults/IID and token mutation overloads remain GZF9.

*[compat]* State comprises a retained script target, two optional queried target
extensions, retained context, a vocabulary stack, active parse state/input cursor,
and current command/parameter values. SetScriptTarget releases the old target
and both extension references before retaining the replacement. It queries
`0x1A2BDEB9` and `0x1A2BDEBA` on a nonnull replacement and returns true even
when extensions are absent. Null detaches all three (`0x0094451C`). SetContext
releases its old context then stores/AddRefs the replacement; null clears it
(`0x00944589`). Callers must keep an independent reference when passing the same
object back to either setter, because release precedes reacquisition.

*[compat]* PushVocabulary(null) returns false. A nonnull vocabulary is AddRef'd
and pushed. PopVocabulary releases/removes the top vocabulary only when more
than one remains; it returns false at the bottom. Lookup searches newest to
oldest and stops at the first translation (`0x00944C4B`, `0x009448E4`,
`0x0094490C`). Shadowing is scoped, not a global destructive replacement.

### 9.2 Text and callback contract

*[compat]* Tokens are plain text, quoted text, `<`, `>` and `=`. Outside a tag,
text is collected through the next `<` and delivered to the base target's slot 5.
Within tags, space, tab, LF and CR separate words. A double-quoted value loses
its delimiters; spaces inside it remain data. Unquoted words end at whitespace,
quote, `<`, `>` or `=`. There is no escape decoder in this tokenizer
(`0x00944D45`). Do not substitute XML entity or backslash decoding.

*[compat]* A tag starts with `<` and a command identifier, followed by parameter
identifiers and optional `= value` assignments, then `>`. Command and parameter
identifiers resolve through the vocabulary stack or numeric conversion. A
translated identifier must be a variant of type 6 to be accepted as a uint32;
another translated type fails rather than falling through to numeric parsing
(`0x00944940`, instructions `0x00944955`–`0x00944975`). Quoted parameter values
are strings. Unquoted values first use vocabulary translation, then numeric
conversion, then string fallback (`0x009449DD`). Numeric textual acceptance
and unassigned-parameter defaults remain GZF9.

*[compat]* On starting a script, the base target's slot 3 receives the interpreter.
On a recognized command, extension `0x1A2BDEB9` slot 3 receives interpreter and
command ID. Each completed parameter is delivered to its slot 5 with interpreter,
parameter ID and variant. At command completion that extension's slot 4 receives
interpreter and command ID; extension `0x1A2BDEBA` slot 3 then receives interpreter,
command ID and the accumulated parameter collection (`0x009445B2`, `0x00944F3C`,
`0x00944AC8`, `0x009445C8`). This is direct callback dispatch, not GZ message posting.

### 9.3 Execution and failure

*[compat]* Reentrant execution while a parse is active returns false. Token
syntax errors put the interpreter in its error state and make execution return
false; a later execution is allowed after the parser resets its active state.
An unknown command skips to its closing `>` rather than invoking command
callbacks. Unknown parameters are skipped with their optional assigned value.
EOF itself does not test whether every tag has closed: return success depends
on whether the error state was reached (`0x00944FB6`). Do not promise strict XML
well-formedness validation.

*[compat]* ExecuteScriptFile opens and reads file contents, appends a NUL after
the bytes actually read, executes, and frees temporary storage; open/read setup
failure returns false (`0x00945198`). ExecuteScriptResource asks the persist
resource manager for the resource, requires stream IID `0xC56B8F08`, reads the
stream bytes, executes them through ExecuteScriptString, then releases the
resource through the manager. Missing resource or missing stream interface
returns false (`0x00944C72`). Short-read and embedded-NUL details are GZF9.

## 10. Conformance vectors

These are declarative vectors for a future engine lane, not results of running
an implementation. An event recorder should observe host calls and reference
balances. `new` means a fresh fixture; sentinels in output arguments detect writes
on failure. Rows involving existing buffer/command specs inherit their fixtures.

| ID | Setup and action | Required observation |
|---|---|---|
| GZC01 | New framework; QI unsupported IID, output sentinel | false; sentinel unchanged |
| GZC02 | Framework QI `1`, `0x65`, `0x23EA70A1` | primary, primary, secondary face; one added ref each |
| GZC03 | New framework; AddRef twice, Release twice | counts 1, 2, 1, 0; no deletion through Release |
| GZC04 | Set scheduling count 0, then −1, then 7 | getter yields 1, 1, 7 |
| GZC05 | Add null service, then two services with same ID | false, first accepted, duplicate false |
| GZC06 | Lookup absent service with output sentinel | false; no service QI invoked |
| GZC07 | Lookup registered service with IID X | exactly that service receives QI(X) |
| GZC08 | Alternate-loop quit with result 9 | pending flag/result recorded; no OS quit post |
| GZC09 | Native-loop quit with result 9 | host receives quit post carrying 9 |
| GZC10 | App initialization returns false; invoke Main | no run-loop entry; shutdown attempted; result `0xFFFFFFFF` |
| GZC11 | COM RealInit twice; director succeeds | director initialized only once |
| GZC12 | COM RealInit; director fails | false; initialization remains retryable |
| GZC13 | COM batch-ID request with count zero | true; output array unchanged |
| GZC14 | Register command ID 0 or null name | false; registry unchanged |
| GZC15 | Register ID A/name X, then A/name Y | true then false; X remains registered |
| GZC16 | Parameter-description update for absent ID | false |
| GZC17 | New command server | no active dispatcher; empty registry/generator collection |
| GZC18 | Tool Init twice, Shutdown twice | one registration/subscription cycle; all calls true |
| GZC19 | Tool Init without command service | true; later Init while initialized does not retry registration |
| GZC20 | Tool registration recorder | seven ID/name pairs in §5.3, null description on each |
| GZC21 | Misc registration recorder | 35 ID/name pairs in §5.2; popup enabled name is registered |
| GZC22 | Buffer Init(10,3,2,8) | stride 12, bytes 36; no promise of zeroed pixels |
| GZC23 | Buffer Init with zero dimension; second Init after success | false in both cases |
| GZC24 | Buffer Lock without storage | false |
| GZC25 | Type-2 buffer RGB mapping with nonzero components | result 0 |
| GZC26 | Log filter/log string conversions at 0 and `0x7FFFFFFF` | filter all/none; log none/all |
| GZC27 | Backend declines category/level | filter called; output not called |
| GZC28 | Backend callback submits another log record | deferred; no immediate nested backend output |
| GZC29 | Foreign thread submits A then B; owner OnTick | owned text survives caller; A delivered before B |
| GZC30 | File-system plugin-name getter on new service | `Plugins` |
| GZC31 | SetDataDirectory to nonexistent path on fresh service | false after storing path; cache not marked valid by that failure |
| GZC32 | Push null vocabulary; pop bottom vocabulary | false; false; no released bottom entry |
| GZC33 | Lower vocabulary maps T to 1, upper maps T to 2; pop upper | lookup 2 before pop, 1 after |
| GZC34 | Translate command name to non-uint32 variant | no command-start callback for that identifier |
| GZC35 | Tagged command with quoted `a b` value | parameter is string `a b`, one value |
| GZC36 | Target starts another script from a callback | nested execution false; outer parse remains active |
| GZC37 | Unknown command tag followed by plain text | unknown command skipped; later plain text callback still occurs |
| GZC38 | Missing script resource or missing stream IID | false; acquired resource returned to manager |

## 11. Remaining work

GZF1–GZF9 in the [question file](questions/gz-framework-core.md) bound the
unrecovered interfaces and failure paths. Static evidence does not establish a
game verdict, replacement safety, or binary equivalence. No engine conformance
files were written by this RE lane; §10 is the implementation-facing vector set.
