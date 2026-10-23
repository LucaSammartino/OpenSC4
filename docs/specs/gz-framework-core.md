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
| 21–23 | borrowed embedded helper `0x00879479`, byte-flag getter `0x008793F2`, borrowed embedded COM `0x008793F9` |
| 24–29 | `0x00472E50` (open), get debug level `0x00879480`, two aliases `0x00879487`, set level `0x008794BF`, propagated SetDebugLevel `0x0087948C` |
| 30–35 | `0x00949B2B` (open), stream getters `0x008796BD`, `0x0087970D`, `0x0087975D`, stream QI `0x0087950F`, SetStream `0x008797AD` |
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
inserts in **descending signed 32-bit priority**, obtained from participant slot
12 (`0x00879BE3`, signed comparison `0x00879C1F`–`0x00879C22`). Equal priorities
are passed over, so distinct participants tied at insertion retain insertion
order. Registration checks participant IDs along that traversal; it stops at
the insertion point, rather than scanning the rest of the list for duplicates
(`0x00879C02`–`0x00879C44`). The callback interface's semantic name remains GZF1.

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

### 2.4 Accessor ownership (GZF10)

*[compat]* The no-argument helper accessor (slot 21) returns embedded storage
at primary+`0xE8`; COM (slot 23) returns the embedded COM primary face at
primary+8; application (slot 37) returns the current global application pointer,
possibly null. These are **borrowed**, with no AddRef, transfer, or output
argument (`0x00879479`, `0x008793F9`, `0x00879473`). Keep the framework and
application alive while using their borrowed pointers, or explicitly acquire a
reference through the appropriate interface. Slot 22 is **not a framework
pointer accessor**: `0x008793F2` returns only the byte stored at framework+`0xC8`.
No semantic name for that flag is claimed here. SetApplication (slot 36) takes
one pointer, permits null, stores it without retaining/releasing either old or
new value, and has no promised result (`0x00879427`–`0x0087944E`). This differs
from static sSetApplication, which explicitly AddRefs its argument (§2.1).

### 2.5 Streams and propagated debug setting (GZF11)

*[compat]* The following are thiscall on the framework primary. Pops excludes
the return address; AL means a byte boolean, EAX a full pointer/value. Selectors
0 and 1 identify the two output streams, 2 the input stream.

| Slot / target | Arguments after this; Pops | Result |
|---|---|---|
| 31 / `0x008796BD` | none; 0 | EAX borrowed output stream 0 |
| 32 / `0x0087970D` | none; 0 | EAX borrowed output stream 1 |
| 33 / `0x0087975D` | none; 0 | EAX borrowed input stream 2 |
| 34 / `0x0087950F` | selector, iid, out; 12 | AL stream QI result |
| 35 / `0x008797AD` | selector, unknown pointer; 8 | AL replacement QI result, or true on clear |
| 29 / `0x0087948C` | signed 32-bit level; 4 | void, propagated setting |

*[compat]* Each stream getter lazily creates and retains a null-output stream
(0/1) or null-input stream (2) when its slot is empty. Repeated getters return
the same borrowed pointer without adding a caller reference. Allocation failure
returns null, leaving the slot empty and retryable (`0x008796BD`–`0x008797AC`).
Generic stream QI returns false without a write or getter call for an invalid
selector or null out. Otherwise it calls the selected getter and delegates
QI(iid, out), inheriting the stream's output-write and acquired-reference rules.
It does not preclear out. If the getter cannot allocate, this operation
dereferences null instead of returning false (`0x0087952A`–`0x00879567`).

*[compat]* SetStream rejects selectors outside 0–2 without changing streams.
For a valid selector it first releases/clears the previous stream. Null
replacement returns true. Nonnull replacement receives QI for `0xC51B11CB`
(0/1) or `0x051B11BD` (2), with the actual cleared slot as out. Its boolean is
returned and its acquired reference becomes the framework's retained reference.
On ordinary no-write QI failure, the old stream is lost and the slot stays
empty; there is no rollback, second AddRef or cleanup of a failure output write.
The next getter supplies a null stream (`0x008797AD`–`0x00879859`). Keep the
replacement alive across the call, especially when reusing the old borrowed
stream. These inventory entries were previously mislabeled as stream setters
and a stream operation: slots 31–33 are getters and slot 34 is a QI bridge.

*[compat]* Propagated SetDebugLevel stores a separate value at +`0x138` and
requests environment variable `GZFRAMEWORK_DEBUG_LEVEL` with signed decimal
`%d` text. Environment-write failure is ignored; there is no result or rollback
(`0x00879492`–`0x008794BC`). It does not traverse participants or change the
basic level at +`0x128`: slots 25–27 read that basic value, and slot 28 only
stores it (`0x00879480`, `0x00879487`, `0x008794BF`). Thus basic level 7 followed
by propagated level −2 leaves all three basic getters at 7 and requests `-2`.

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

### 3.3 Running state and library enumeration (GZF11)

*[compat]* SetServiceRunning takes one byte in a dword stack slot (thiscall,
Pops 4, void). It stores that byte without normalizing it, initializing or
shutting down libraries, or notifying a director (`0x0087B385` → `0x0091DD91`).

*[compat]* Callback EnumerateLibraries(callback, context), primary slot 12,
is thiscall, Pops 8, void. Null callback is a no-op. Otherwise each call supplies
a borrowed library-entry interface pointer and context to a **cdecl** callback.
False in AL cancels before the next entry; true continues in the internal tree
traversal order. There is no automatic AddRef/Release, snapshot, or guarantee
for mutation during enumeration (`0x0087B70F`–`0x0087B741`). Array enumeration
(slot 13) takes (outArray, inoutCount), Pops 8, returns EAX count. The count
pointer must be writable and nonnull. Null array writes/returns the total,
ignoring incoming capacity. Otherwise incoming capacity is unsigned and clamped
down to the total; exactly that many pointers are written in the same traversal
order and AddRef'd once each for the caller. Zero capacity writes no elements;
excess elements remain untouched. The returned count is the number copied,
not necessarily the total (`0x0087B6C4`–`0x0087B70C`).

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

### 4.3 Identity, formatting and generator replay (GZF11)

*[compat]* Command primary QI(iid, out) accepts `1`, `0xCB90253A` and
`0xCB90253B` at the primary pointer, and `0x287FB697` at primary+4. Querying
IID 1 through that inherited system-service face instead yields the lifetime
face primary+8. Success writes a face/AddRefs/returns true; unsupported queries
return false without writing out; nonnull writable out is required on success
(`0x0092980D`, `0x0091DD67`, `0x009457C6`). All faces share the initially zero
count. AddRef returns its increment modulo 2^32. Release with unsigned count
greater than one decrements/returns it; at zero or one it deletes through the
lifetime face and returns zero (`0x0094BC3F`, `0x00920BF0`). Destruction releases
held generators/dispatchers and destroys the registry, but does not call public
Shutdown or promise its unregistration callbacks (`0x0092AE01`, `0x0092AFD5`).

*[compat]* ExecuteCommandFormatted (`0x009298AE`) is **cdecl varargs**:
stack arguments are (command-primary pointer, format, variadic words), with
caller cleanup and full 32-bit execution status in EAX. The native printf
formatter receives a 511-byte limit in a 512-byte local buffer. Positive result
N dispatches primary slot 19 with (formatted bytes, N, first variadic word),
returning its status; the final local byte is forced to NUL. Zero or negative
formatting result returns status 2 without execution (`0x009298B7`–`0x009298F3`).
**Quirk:** the first variadic word is both the first printf argument and the
third text-execution argument; there is no separately skipped output argument.
Even a format with no conversions needs that word if execution occurs. Null
format, missing words or mismatched printf types have no checked failure
contract. Negative overflow status is not a successful truncated command.
Slot 19 forwards to the default string generator's slot 8, or returns 2 if
absent (`0x00929BBA`); formatting does not bypass the text parser.

*[compat]* RegisterCommandGenerator(pointer) and UnregisterCommandGenerator(pointer)
are thiscall, Pops 4, AL boolean. New registration retains one reference in a
pointer-identity set and replays existing commands in unsigned-ID map order
through generator slot 5 with (ID, borrowed name-string interface, borrowed
parameter-description interface). Insertion precedes replay; callback results
are ignored (`0x0092AB0B`, `0x00929E54`). Duplicate registration returns true
without another retained reference or replay. Unregistration removes/releases
a present pointer, sends no callbacks, and returns true even if absent
(`0x0092A236`); temporary references are balanced (`0x0096E773`). Normal
command-removal notification uses generator slot 6 with those same arguments.
Neither operation writes an output argument or propagates callback failure.
Null is not rejected: with an empty command map it is inserted and returns true,
and unregistering it returns true. With existing commands a newly inserted null
reaches the helper's “notify all” meaning, finds null again and recurses without
progress (`0x0092AB41`–`0x0092AB50`, `0x00929E63`–`0x00929E9C`). There is no
safe false-return contract for that misuse.

## 5. Command-system supports

### 5.1 Common lifetime and message contract

Both supports have QI/AddRef/Release in slots 0–2 (`0x005C3E30`, `0x005F47F0`,
`0x005C3E50`), DoMessage in slot 3, deleting destructor in slot 4, FinalRelease
in slot 5 (`0x005C3E70`), Init in slot 6 and Shutdown in slot 7. Concrete tables:
misc `0x00AA1CE4`, tool `0x00AA1AFC`. The tool table `0x00AA1648` has purecall
in slot 3 and is not the concrete dispatch table. *[compat]*

*[compat, GZF10]* QI(iid, out) accepts **only `0x452294AA`**, returns the
primary pointer, adds one reference and returns true; even IID 1 fails, leaving
out untouched. A successful query requires writable out storage. AddRef returns
the incremented 32-bit count. Release with unsigned count greater than one
decrements and returns it; with count zero or one it calls virtual FinalRelease
and returns zero. FinalRelease calls the deleting destructor with flag 1; it
does **not** call Shutdown (`0x005C3E30`–`0x005C3E7B`). Tool destruction frees
the notification tree and object; misc destruction also destroys its pending
wait collection and releases retained service pointers (`0x005C5110`,
`0x005C7EC0`, `0x005C84F0`). Neither destructor unregisters commands or
subscriptions. Call Shutdown while the object is live when that cleanup is
required; a final Release is not a substitute, including when initialized=true.

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
clears only the supplied flag bits and returns true. A type-2 8-bit buffer has no palette; RGB
mapping returns zero for that type. Consequently the linked renderer's solid
fill writes zero even when a nonzero drawing color was selected. These rules
and their instruction evidence are in span-render-8 §10.4.

*[compat, GZF10]* Buffer QI(iid, out) accepts `1` and `0x20732180` at the
primary face and `0x86D72B57` at primary+4. Each success writes the face, adds
one reference and returns true; unsupported IDs return false without writing
out. Writable out is required on success (`0x00825D00`–`0x00825D52`). The
constructor starts the reference count, stored color key and color-key enable
flags at zero (`0x00827315`–`0x0082731E`). GetColorKey(outKey) returns false
and leaves outKey unchanged when disabled; when enabled it writes the full
32-bit key and returns true (`0x00826620`–`0x00826636`). Thus fresh construction
does not report a key of zero: it reports **no key**, with no output write.
AddRef returns the incremented 32-bit count. Release at count one stores zero,
deletes through slot 40 with flag 1, and returns zero; at every other count it
decrements modulo 2^32, so Release at zero returns `0xFFFFFFFF` without deletion
(`0x00825D60`–`0x00825D90`). References from all faces share that lifetime.

*[compat, GZF10 correction]* Unlock(flags) is mask removal, not an all-flags
reset: retained flags are old flags AND the 32-bit complement of flags. Nesting
is decremented modulo 2^16, even at zero and without storage; no lock ownership
or matching-mask check occurs, and the modification counter is unchanged
(`0x00826490`–`0x008264A4`). For example, locks `0x8040`, `0x0800`, followed
by Unlock(`0x0800`), leave `0x8040` and nesting 1. This resolves the old
ambiguous “clears flags” wording in this section and span-render-8 §10.4 in
favor of span-render-4444's audited bridge. Lock's accumulated mask and change
counter are 32-bit, while its nesting count is 16-bit (`0x00826AAF`–`0x00826ABB`).

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
false, allocated empty internal name sets (null on allocation failure), and
recursion depth zero. *[compat]*

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

*[compat, GZF10]* Delivery ownership is established by the thread executing
the first Init while initialized=false, **not** by construction. That Init
captures the current thread ID before backend setup (`0x009342D2`–`0x009342EA`);
an Init while already initialized does not recapture it. After Shutdown resets
initialization, another Init can bind a different thread. OutputLowLevel compares
the submitting thread with this captured ID (`0x00934A6E`); OnTick drains the
foreign-thread queue but does not establish a new owner (`0x00934F05`).

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
registration, releases its backend collection and deletes the internal name
sets, then returns true (`0x00935228`). Public stream/output/accessor contracts
are in §7.3; initialization configuration and further formatting failure details
remain GZF7.

### 7.3 Stream creation, diagnostics and accessors (GZF11)

*[compat]* AddOutputStream(name, stream, timestampByte, outBackend), slot 7,
is thiscall on the logging face, Pops 16, AL boolean. It creates a default
backend with copied name, retained stream and passed low timestamp byte. Null
name is unchecked (strlen); null stream is stored, not rejected here. The out
pointer may be null. Allocation failure returns false without writing it.
Successful construction calls AddOutputBackEnd with that backend's name and
pointer but **ignores its result** and returns true. Nonnull out receives the
new backend and one caller reference; the temporary reference is released
(`0x00933C77`–`0x00933CE0`, `0x00935361`). AddOutputBackEnd rejects recorded
duplicate names without replacement (`0x00934D82`–`0x00934D89`): therefore this
wrapper can return true and a caller-owned backend that is not routed. Without
out, rejection destroys the new backend when its temporary reference is released.

*[compat]* No-argument slot 6 (`0x009CDC23`) returns the stored backend-name
set pointer at logging-face+`0x28`; slot 15 (`0x009D858F`) returns the observed
category-name set pointer at +`0x24`. Pops 0, EAX pointer, borrowed, no AddRef,
copy or lock. These are internal string sets, not queried interfaces/streams.
Construction allocates empty sets; Shutdown deletes them and nulls their
pointers, invalidating earlier borrows (`0x00934C88`, `0x00935228`). Backend
registration records its name; accepted public output records its category
even when every backend filters the text (`0x00934D8E`–`0x00934DA1`,
`0x00935170`–`0x00935183`). Concrete container ABI is not a portable interface.

*[compat]* Output and OutputLevel (slots 16/17) are **cdecl varargs**, void,
with stack arguments (logging-face pointer, category, format, ...) and
(logging-face pointer, category, signed 32-bit level, format, ...). Uninitialized,
null category or null format means no category insertion, formatting or delivery;
empty nonnull strings are accepted. Output assigns level 0; OutputLevel passes
the supplied level. Each inserts a copied category under the service lock, then
formats and submits to §7.2; these are not unformatted text aliases
(`0x00935148`–`0x00935227`). Deferred category/text storage is owned by the router.

*[compat]* Their shared formatting helper (`0x00934F88`, thiscall on the whole
service; category, level, format, va_list; Pops 16; void) first tries 1024 bytes.
On negative formatter status it retries allocations of 2048, 4096, 8192, 16384,
32768 and 65536 bytes until nonnegative status; the next size, 131072, exceeds
the signed 100000-byte bound. It submits the last buffer even if status stays
negative. The alternative initial-result-at-least-1024 branch allocates only
1025 bytes and formats once again, not result+1 bytes. Allocation success and
termination after final overflow are unchecked (`0x00934FCF`–`0x00935035`).
Thus overflow/invalid format/low memory has no promised false result, safe drop
or clean truncation. GZF7 retains the full printf vocabulary and low-memory
consequences, not the public guards or ordinary bounded formatting settled here.

*[compat]* DumpOutputLevels(stream), slot 14, is thiscall, Pops 4, void. It
borrows a nonnull output stream without AddRef and writes `All Backends:\n`,
`-------------\n`, `All unspecified groups showing ` + default level
description + newline, then each configured category in stored map order as
`Group '` + name + `' showing ` + level description + newline. After another
`-------------\n` it invokes each backend's slot 6 with the same stream in
backend-list order under the service lock. Stream-operation results are ignored;
null stream is unchecked. No initialized guard precedes the global heading;
it changes no levels and drains no pending messages (`0x00933FAA`). Exact
level-description vocabulary remains the GZF7 level table.

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
(`0x008909EE`). *[compat, GZF10]* Failure leaves the previous cache-valid flag
unchanged: it does not invalidate a previously valid cache. Consequently, after
an already valid directory is overwritten with a nonexistent path, the setter
returns false but the next getter returns that newly stored path without lazy
INI resolution. On a fresh invalid cache, the same failure leaves it invalid,
and the getter still resolves lazily. The stores of true occur only on the
success branches (`0x008909D9`–`0x008909E6`, `0x00890A1D`–`0x00890A2A`);
getters test those same flags (`0x00890A40`, `0x00890B9B`).
CD discovery/validation has its own expected-volume name,
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
| GZC39 | Register ordered participants A=−1, B=7, C=7, D=0, with distinct IDs | traversal B, C, D, A; ties retain insertion order |
| GZC40 | New buffer; QI `1`, `0x20732180`, `0x86D72B57`, then unknown IID with sentinel | primary, primary, primary+4; three added refs; final query false/sentinel unchanged |
| GZC41 | Fresh buffer; GetColorKey with sentinel `0x12345678` | false; sentinel unchanged, despite stored key zero |
| GZC42 | Buffer fixture with enabled key `0x89ABCDEF`; GetColorKey | true; writes exactly `0x89ABCDEF` |
| GZC43 | Buffer count zero; AddRef twice and Release twice | 1, 2, 1, 0; exactly one destruction on last call |
| GZC44 | Separate fresh buffer, Release without AddRef | `0xFFFFFFFF`; no destruction; dispose fixture separately |
| GZC45 | Initialized buffer; Lock(`0x8040`), Lock(`0x0800`), Unlock(`0x0800`), Unlock(0) | mask `0x8040`, nesting 0; change count advanced once; both Unlock calls true |
| GZC46 | Fresh buffer without pixels; Unlock(`0xFFFFFFFF`) | true; nesting becomes 65535, mask zero; no pixel access |
| GZC47 | Each support; QI `0x452294AA`, then IID 1 with sentinel | primary/+1 ref, then false/unchanged output |
| GZC48 | Support count 2; Release twice with destruction recorder | 1, then 0; FinalRelease/deleting destructor once; no implicit Shutdown |
| GZC49 | Support at initial count zero; Release | returns zero and deletes; unlike buffer underflow |
| GZC50 | Construct logger on A, Init on B, submit record on A, tick on B | delivery deferred until B drains; repeated Init on A does not rebind |
| GZC51 | Valid cached data and plugin directories; respective setter given nonexistent P | false; each cache stays valid and getter returns P without INI lookup |
| GZC52 | Framework helper/COM/application getters with reference recorders; SetApplication(P), then null | stable embedded addresses; application P then null; no AddRef/Release from these calls |
| GZC53 | Empty framework; each stream getter twice | one retained null stream per selector; stable borrowed pointers; no caller AddRef |
| GZC54 | SetStream(0, P), P answers QI; then stream query | setter queries `0xC51B11CB`; stores acquired reference; later query inherits stream QI result/output writes |
| GZC55 | Existing input; SetStream(2, P), P rejects QI without writing | old reference released before QI(`0x051B11BD`); false; getter next creates null input |
| GZC56 | Stream query invalid selector or null out; SetStream(3, P) | false; no getter/replacement QI/output write; streams untouched |
| GZC57 | SetStream(1, null), getter; allocation failure on a separate empty getter | clear true; lazy null output; failed getter null, later getter retries |
| GZC58 | Basic level 7; propagate −2 with environment recorder rejecting write | basic getters stay 7; environment `GZFRAMEWORK_DEBUG_LEVEL=-2` attempted; no result/rollback |
| GZC59 | COM SetServiceRunning low byte 2 then 0 | bytes stored unchanged; no lifecycle/director calls |
| GZC60 | Three COM libraries; null-array count query, then array capacity 2, element 3 sentinel | count query 3; copy count 2, two AddRefs; sentinel untouched |
| GZC61 | Same COM, capacities 0 then 5 | no elements/refs at 0; second count clamped to 3, three pointers/refs |
| GZC62 | COM null callback, then callback false on first entry | zero then one borrowed visits, unchanged context; no AddRef |
| GZC63 | Command QI primary IDs 1, `0xCB90253A`, `0xCB90253B`, service IID, unknown sentinel | primary for first three; primary+4 for service; +1 ref each; failure leaves sentinel |
| GZC64 | Command initial count zero; Release with destructor/callback recorder | deletes/returns 0; held refs released; no public Shutdown replay |
| GZC65 | Formatted-entry slot-19 spy; format `%u`, first variadic word 12 | text `12`, length 2, third word 12; result exactly spy status |
| GZC66 | Formatted-entry spy; empty format, then negative formatter result | status 2 with no execution in each case |
| GZC67 | Commands registered 9 then 2; register G twice, unregister twice | replay IDs 2,9 once through slot 5; one retained ref; all true; no removal replay |
| GZC68 | Empty command map; register then unregister null | both true; no refs/callbacks; remove before adding commands |
| GZC69 | AddOutputStream with out sentinel; force allocation failure | false; sentinel unchanged; no backend registration |
| GZC70 | Backend name X exists; AddOutputStream(X, stream, 0, out) | true; new caller-owned backend retains stream; original stays routed |
| GZC71 | Uninitialized logger output; initialized output with null category or format | no category insertion, formatting or delivery |
| GZC72 | Initialized logger, accepting backend; Output(C, `x=%d`, 4), OutputLevel(C, 7, `y=%d`, 5) | observed set contains C once; `x=4` level 0, `y=5` level 7 |
| GZC73 | Log set getters before use and after Shutdown with ref recorder | borrowed distinct empty sets initially, no AddRef; null after Shutdown; prior borrows invalid |
| GZC74 | Dump with no category overrides, backend dump recorder | global heading/default description/separators emitted; backend receives same borrowed stream; no routing or tick |

## 11. Remaining work

GZF1–GZF9 in the [question file](questions/gz-framework-core.md) bound the
unrecovered interfaces and failure paths. Static evidence does not establish a
game verdict, replacement safety, or binary equivalence. No engine conformance
files were written by this RE lane; §10 is the implementation-facing vector set.
