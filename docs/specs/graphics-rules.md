# Spec: Graphics Rules (`cSC4ConfigRuleManager`, `cSC4RuleParser`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [render-properties.md](render-properties.md) (the properties the rules set),
[command-line.md](command-line.md) (the `-writeLog` switch and the callers of §6), [INVENTORY-M6.md](INVENTORY-M6.md)
(F1, "Graphics rules").

## 0. Summary

At start-up the config rule manager measures the machine (CPU, memory, screen, video card and driver, sound), reads
`Graphics Rules.sgr` (which includes `Video Cards.sgr`) and an optional user `Graphics Options.sgr`, and evaluates
the rules in them: matching rules set render properties and recommend an option for each **option group** (the
graphics settings of the options dialog, such as Shadows or Texture Quality). Choosing an option in the dialog
applies that option's property set. Everything is logged to `<computer name>-config-log.txt`.

## 1. The manager *[compat]*

| Item | Value |
|---|---|
| COM class | `0x0A1D8285` (`0x007CE130`); `GZCOM_CreatecSC4ConfigRuleManager` (`0x007D1030`); constructor `0x007D0CC0` |
| `QueryInterface` (`0x007CE140`) | `0x0A1D8284`, else the `cRZUnknown` base |
| Vtable `0x00ABB34C` | 3 `Init`, 4 `SetOptionsFromConfiguration`, 5 `LogConfigurationToFile`, 6 `ApplyOptionGroupSetting`, 7 `GetOptionGroupAutoSetting`; the `+4` interface's slot 3 (`0x007D0DB0`) is the deleting destructor |
| `+0x0C` | the rules (0x7C bytes each, in file order) |
| `+0x18` | the option groups (16 bytes each: the recommended option, then the options, each a property set of 0x24 bytes) |
| `+0x24`, `+0x28`, `+0x34` | the flags, the string variables (0x14 bytes each), the numeric variables (floats) |
| `+0x40` | the render properties (counted) |
| `+0x6C` | the log file |

A **property set** is three lists: (boolean index, value), (integer index, value), (float index, value). Applying it
(`0x007CE1F0`) calls the render properties' `SetBool`, `SetInt` and `SetFloat` for each, in that order.
Within each list, insertion order and duplicate assignments are preserved. The float list stores binary32;
the integer list stores signed 32-bit values. Writes are not coalesced and setter failure does not stop the
remaining writes. Construction creates empty rules/groups, sixteen empty strings and eleven +0.0 numbers.
Replacing the render-property object in `Init` retains the new object and releases the old one; the same
object is left alone (`0x007D189C–0x007D18B9`). `Init` has no success return and does not evaluate rules.
Destruction releases the held properties and owned lists/file resources; secondary-interface thunks
adjust `this` by four bytes (`0x007D0DA0`, `0x007D0DB0`).

## 2. Variables (`GatherInformation`, `0x007CF780`) *[compat]*

**Flags** (bit = index), named for `has` / `hasNo`:

| Bit | Name | Source |
|---|---|---|
| 0 | `hardwareDriver` | the driver is a hardware driver and `kForceSoftwareDriver` is false |
| 1 | `DXT` | current driver information record, byte `+0x2C`, nonzero |
| 2 | `fullscreen` | current driver information record, byte `+0x20`, nonzero |
| 3 | `MMX` | CPU feature bit 23 |
| 4 | `FPU` | CPU feature bit 0 |
| 5 | `backingStore` | a backing store exists; interface acquisition is specified in §2.4 (native test `0x007C2790`) |
| 6 | `forcedFSAA` | boolean result of the rendering probe `0x008D2320(currentDriver)` |

**Numbers** (`atLeast` / `atMost`), as floats: 0 `cpuSpeed` (MHz, `0x00891552`), 1 `memory`, 2 `freeMemory` (MiB),
3 `screenWidth`, 4 `screenHeight`, 5 `screenBPP`, 6 `videoMemory` and 7 `textureMemory` (MB: the graphic system's
memory figures shifted right by 20), 8 `textureStages`, 9 `bogoFPS`, 10 `bogoMPPS`.
The constructor initializes all eleven binary32 numbers to positive zero (`0x007D0D79–0x007D0D86`).
`GatherInformation` writes only indices 0–8: neither bogo number is measured by this manager, including on
later gathers. The forced-FSAA probe is a separate rendering test, not a source for either bogo number.

**Strings** (`stringMatch` / `stringNotMatch`): 0 `userName`, 1 `appName`, 2 `version`, 3 `osVersion`,
4 `computerName`, 5 `CPU`, 6 `sglDriverName`, 7 `sglDriverVersion`, 8 `driverName`, 9 `driverVersion`,
10 `cardName`, 11 `cardVersion`, 12 `cardIdentity`, 13 `soundDriverName`, 14 `soundCardName`, 15 `buildType`.

- 6–11 are successive newline-delimited fields of the driver's description string **after discarding its
  first field** (`0x007CFAF7`, then `0x007CFB26–0x007CFCF5`; delimiter at `0x00A916BC` is byte `0x0A`).
- 13 and 14 come from the sound system (service `0xE9D27715`), or `Unknown` without one.
- 15 is `Release`.

### 2.1 Exact provider boundary

The graphics system comes from `RZ::GS()`. Its query for interface `0xC47B747C`, followed by that interface's
slot 3, supplies the current driver (`0x007CF9AE–0x007CF9E2`). Driver slot 64 (`+0x100`) fills a **0x34-byte
record**. The DX7 implementation at `0x00886E10` copies thirteen dwords from driver-interface-relative
`+0x1D4`; the alternative implementation at `0x008DC320` copies thirteen from `+0x3A8`.
Treat the returned record, not either concrete driver's object layout, as the boundary:

| Record field | Manager consumption | Native evidence |
|---|---|---|
| byte `+0x20` | nonzero sets `fullscreen` | `0x007CFA77–0x007CFA8B` |
| unsigned dword `+0x24` | convert to binary32 `textureStages` | `0x007CF9F6–0x007CFA12` |
| byte `+0x2C` | nonzero sets `DXT` | `0x007CFA60–0x007CFA74` |
| byte `+0x30` | nonzero AND render bool index 0 (`kForceSoftwareDriver`) clear sets `hardwareDriver` | `0x007CFA2F–0x007CFA5D` |

These bytes are provider reports, not values recomputed from screen size, card identity, texture formats or
command-line switches by the rule manager. In particular `forcedFSAA` is not a fourth capability byte or an
FSAA preference. Its helper fetches this record itself and immediately returns false when byte `+0x30` is
zero (`0x008D2338–0x008D2358`). On hardware it runs twenty render iterations, obtains a readback surface,
and samples the 8×8 grid at x,y = 4,12,…,60. Count samples for which R+G+B is nonzero, ignoring alpha;
the result is true iff that count exceeds one (`0x008D25C1–0x008D25E8`, `0x008D2709–0x008D2756`).
Without a readback surface the result remains false. This measures a rendering symptom of forced FSAA;
it is independent of `kForceSoftwareDriver`, which is checked later for flag 0. The manager invokes the
probe on each gather, even when that render property forces software.

The OS/version provider supplies CPU features, CPU MHz, memory, and the first six strings (§2.5).
Screen dimensions/BPP instead come from GS slot 10 (`+0x28`), which fills three unsigned dwords in
width, height, BPP order (`0x007CF8BF–0x007CF928`). For memory the native version helper first truncates each unsigned byte count to MiB, then rounds
that integer **up to a multiple of four** with 32-bit wrapping arithmetic: subtract one, OR with 3, add one
(`0x0089159C–0x008915B9`). Zero maps to zero. Video and texture memory instead use GS slot 23 (`+0x5C`),
with arguments 2 and 0 respectively; each unsigned byte count is shifted right 20 with **no** four-MiB
rounding (`0x007CF965–0x007CF99F`). CPU MHz, screen values, memory values and stage count are converted
from unsigned 32-bit quantities to binary32; the native signed x87 loads add 2^32 for negative signed
representations before the float store. Do not reinterpret these as signed hardware quantities.

A portable implementation may inject the OS results, capability record, description, sound names,
backing-store presence and probe result through a provider. It owns the flag assembly, units/conversions,
fixed field mapping and all parser/evaluator algorithms below. It must keep provider values explicit in
fixtures; a constant false FSAA result or guessed DXT support is not evidence of native acquisition parity.
The GPU rendering/readback operation belongs to that provider/renderer boundary; neither bogo value is
an output of it.

### 2.2 Regather and unavailable services (GR6)

Regather clears/rebuilds the flags and refreshes the measured fields, but does **not** assign string
index 12 (`cardIdentity`). Parsing assigns that identity separately (§3). Thus an explicit log after a
driver change can contain a new `cardName` and the old parsed `cardIdentity`. The description splitter
`NextToken` (`0x0067A960–0x0067AAEC`) returns one field before the first LF and removes that field plus
one delimiter from the remaining description. Without another delimiter it returns the whole remainder
and empties the remainder. Repeated reads of an exhausted remainder return empty strings. Empty fields
are retained; CR bytes are not removed by this LF-only splitter. Only six fields after the discarded
first field are consumed (`0x007CFAF7–0x007CFD07`); additional fields are ignored.

`Init` is void: it retains the supplied properties reference, calls gather, then parse, and invokes
fallback only when **parse** returns false (`0x007D18B9–0x007D18D2`). Gather has no checked success result.
It calls the GS/current-driver interfaces without a general unavailable-service recovery branch; its
hardware path reads properties bool index 0 without checking the retained properties pointer. Null
properties and failed mandatory provider acquisition therefore have no native recoverable-error
contract. They must not be described as a successful zero-filled gather or as a trigger for fallback.
The optional sound-service absence still has the explicit `Unknown` behavior in §2.

### 2.3 Native forced-FSAA acquisition (GR9)

All slots below are **zero-based**, relative to the returned interface, with x86 byte offsets in
parentheses. These are renderer calls the provider performs itself. The provider must not invoke
`0x008D2320`, the native rule manager, or its logger to obtain the result.

Use the current driver from §2.1. Fetch its mode record through slot 64 (`+0x100`); if byte `+0x30` is
zero, return false immediately, with no rendering or readback. Otherwise consume the driver's pending
error through slot 50 (`+0xC8`, `GetError`), ignoring the result. In DX7 this getter clears the error
(`0x00882000–0x00882010`). Acquire graphics service `0xC416025C` from framework slot 5 (`+0x14`,
service ID, IID, output-interface pointer), for IIDs `0x0073283C` (`cIGZGraphicSystem`) and `0xC47B747C`
(the 3D graphics-system interface). The first is retained for the probe's lifetime; the second supplies
slot 12 (`+0x30`, `SetUIRenderingMode`). The two IIDs can also be queried from the existing GS interface
(`0x0098DEF0–0x0098DF2E`). Do not treat the small IID `0x0073283C` as a function address.

Call `SetUIRenderingMode` once before drawing. The target is the **current driver's color/render
buffer**, with the existing video mode and viewport. There is no offscreen-target creation, 64×64
viewport change, or target switch in the probe. The 64×64 size below is the region copied for readback.
The GS method establishes the normal UI matrices/states (`0x0098EA30`); invoking that exposed method
also preserves its native viewport-dependent behavior. Set the driver's clear color to four binary32
zeros using slot 12 (`+0x30`, `ClearColor`, R, G, B, A). Evidence for setup and the otherwise unused
base-GS reference: `0x008D2359–0x008D23E0`.

Build 64 white quads, row-major with row and column each 0 through 7, in one **4096-byte**, initially
zeroed vertex array. Each vertex occupies 16 bytes: binary32 x, y, z at offsets 0, 4, 8 and four color
bytes at offset 12. z remains positive zero; all four color bytes are `0xFF`. There are no RHW,
normal, or texture-coordinate fields. For column c and row r, compute x = (c × 8.125 + 4) − 0.0625 and
y = (r × 8.125 + 4) − 0.0625, then emit vertices in this order:

| Vertex | x | y | z | Color bytes |
|---|---|---|---|---|
| 0 | x | y | 0 | FF FF FF FF |
| 1 | x | y + 0.125 | 0 | FF FF FF FF |
| 2 | x + 0.125 | y + 0.125 | 0 | FF FF FF FF |
| 3 | x + 0.125 | y | 0 | FF FF FF FF |

The instructions multiply, add, then subtract in that order, with binary32 constants `8.125`, `4`,
`0.0625`, `0.125`; coordinates are stored as binary32 (`0x008D2400–0x008D249E`; constants at
`0x00ACC150`, `0x00A8E17C`, `0x00A8E174`, `0x00A867A8`). All coordinates in this bounded grid are
exactly representable, so these operations introduce no rounding ambiguity here.

Perform **twenty** iterations, numbered 0 through 19. Each iteration rebuilds the same vertex contents
and makes these driver calls in order (`0x008D24A4–0x008D256A`):

| Order | Driver slot (offset) | Arguments, in interface order |
|---|---|---|
| 1 | 11 (`+0x2C`), `Clear` | mask `0x4000` (color buffer) |
| 2–6 | 45 (`+0xB4`), `Disable` | separate calls for capability IDs **4, 3, 0, 1, 5**, in that order |
| 7 | 5 (`+0x14`), `InterleavedArrays` | standard vertex-format ID **1**, stride **16**, vertex-array pointer |
| 8 | 3 (`+0x0C`), `DrawArrays` | primitive ID **6** (quads), first vertex **0**, vertex count **256** |
| 9, iteration 19 only | readback operation below | copy (0, 0, 64, 64) before this iteration's Flush |
| 10 | 43 (`+0xAC`), `Flush` | no arguments |

On iteration 19 query the **driver** for IID `0xE69BFE2A`. On that returned interface call slot 3
(`+0x0C`, `CopyColorBuffer`) with **x=0, y=0, width=64, height=64, destination=null**. The fifth argument
is an optional destination buffer, **not a pixel-format enum**. Retain its returned `cIGZBuffer` and
release the queried readback interface immediately. The DX7 implementation is `0x00883950`, reached
through the four-entry table at `0x00AC47DC`; the main driver's QI is the adjusting thunk at
`0x00886440` (`0x00882A8E–0x00882AA5`). Call sites: `0x008D2518–0x008D2551`.

With a null destination the DX7 method allocates through GS slot 3 (`+0x0C`, output buffer pointer) and
initializes the result to the requested dimensions with **pixel-format ID 9, 32 bits per pixel**
(`0x00883AE9–0x00883B58`; pixel-format construction `0x0044A490–0x0044A4A5`). It copies/converts the
driver's color surface into that buffer. The provider need not lock a DirectDraw surface, interpret its
pitch, or assume a 16-bit screen is a 16-bit readback. Read via the returned buffer's slot 21
(`+0x54`, `GetPixel(x,y)`): the manager consumes its packed 32-bit color, summing the bytes at bits
16–23, 8–15, and 0–7, and ignoring the high byte. Unsupported/failed color-buffer acquisition can
return null; the probe's result then remains false. The original does not check for failure of the
mandatory readback-interface QI; a provider that cannot acquire that interface must report unavailable,
not fabricate a measured false result.

After all twenty iterations, clear once more with mask `0x4000`, Flush once more, and call the 3D GS
interface's `SetUIRenderingMode` once more. Only then inspect the saved readback. Sample y=4,12,…,60,
and, within each row, x=4,12,…,60. Return true iff **more than one** of the 64 RGB sums is nonzero.
There is no intensity threshold beyond nonzero. Release the readback buffer, then the two acquired GS
references (`0x008D256C–0x008D25E8`, `0x008D2705–0x008D2782`).

**State after the probe.** Native cleanup establishes **UI defaults**, not the caller's saved state.
There is no push/pop, target restoration, previous-state capture, or image restoration. Clear color
remains zero and the current color buffer has been cleared. The last bound vertex-array pointer is not
explicitly unbound. The final UI setup, rather than a reversal of the five Disable calls, establishes
the final render state. A provider must run this synchronously at the same graphics/UI boundary; using
a different target or merely consulting an antialiasing preference is a different measurement.

**Worked probe case.** Quad (0,0) spans 3.9375 through 4.0625 in both axes; quad (7,7) spans 60.8125
through 60.9375. An interface fixture must observe 20 DrawArrays calls with format 1/stride 16, 21
color clears and 21 Flush calls, one copy before the twentieth Flush, and two UI-setup calls. A saved
readback containing `0xFF000000` at every sample except `0x00000001` at (4,4) yields one lit sample and
**false**. Changing (12,4) to `0x00010000` yields two and **true**. Changing only alpha at (12,4) to
`0xFF000000` leaves false. This is a discriminator for counting, not a claim about which pixels a
particular GPU's forced-FSAA mode will illuminate. A software mode record returns false without any
of these calls; a hardware run with a null returned buffer performs setup/draw/cleanup and returns false.

### 2.4 Backing-store presence through renderer interfaces (GR9)

Presence and enabledness are different. The native presence helper `0x007C2790` has **no vtable slot**;
the exposed `cISC43DRender` getter, slot 56 (`+0xE0`, `GetBackingStoreEnabled`, `0x007C2750`), reads
enabledness only. A disabled existing store must still set graphics-rule bit 5.

An independent provider can measure presence using the paired public setter: save the getter's byte,
call slot 57 (`+0xE4`, `SetBackingStoreEnabled`) with byte **1**, read slot 56, then restore the saved
byte with slot 57. The intermediate getter is the presence result. The setter forces its argument to
zero when no store exists, otherwise stores the supplied byte, and has no other side effects
(`0x007C2760–0x007C2780`). Perform these calls synchronously without rendering, event dispatch, or a
driver change between them. This is a reversible acquisition operation, not permission to leave
backing-store use enabled.

Obtain the actual view's renderer through the **window manager**, not a `cISC4App` getter. Acquire
service `0xA417445E`, IID `0x000005A4` (`cIGZWinMgr`), through framework slot 5 (`+0x14`, service ID,
IID, output pointer; `0x004177D2–0x004177E1`). Its slot 3 (`+0x0C`, `GetMainWindow`, shared body
`0x009DB37A`) returns a borrowed main window. On that window call slot 34 (`+0x88`,
`GetChildWindowFromID`) with child-window ID `0x6104489A`; the result is also borrowed. On that child
call slot 36 (`+0x90`, `GetChildAs`) with `(0x9A47B417, 0xFA47B3F9, output view)`, then view slot 4
(`+0x10`, `GetRenderer`, `0x0043FD00`). The view output is counted; this last renderer pointer is
borrowed. Query IID **`0x681BD529`** on the renderer to retain a
`cISC43DRender` reference. See [view3d-window.md](view3d-window.md) for the view's interface boundary.
The renderer QI returns its main interface for this IID (`0x007C1894–0x007C18DE`). Release each
acquired reference once after use; do not release either borrowed window or the borrowed renderer.
See §2.7 for the acquisition lifetime and independent-Init contract.

There is also a route before a view exists: GZCOM slot 3 (`+0x0C`; class ID, IID, output pointer)
with renderer class ID **`0xE9C622D8`**, IID **`0x681BD529`**, and output-interface pointer, as used at
`0x007F4322–0x007F4336`. Use the resulting renderer **without Init**, solely for the getter/setter
measurement, then Release it. Both methods access the shared renderer state and do not depend on that
instance being initialized. Construction does not initialize or destroy the shared backing store;
the factory only constructs the object (`0x007CB070–0x007CB08A`, `0x007C9B10–0x007C9F87`). Its
destructor's Shutdown is inert while the instance remains uninitialized (`0x007CA038`,
`0x007C7075–0x007C707C`). Consequently a missing view need not be guessed to mean absent backing
store, and creating this uninitialized interface object does not allocate a backing store. Failure to
obtain either renderer interface is unavailable acquisition, not a false presence measurement.

**Worked backing-store cases.** With an existing store disabled, the getter sequence is 0, then 1 after
the request, then 0 after restoration: set bit 5. Without a store it is 0, 0, 0: clear bit 5. With an
existing enabled store it is 1, 1, 1: set bit 5 and preserve enabledness. A freshly constructed,
uninitialized renderer facade observes these same shared-state cases. None of these tests reads a
private global, calls the private presence helper, or changes the stored presence itself.

### 2.5 Version/OS acquisition and string bytes (GR9)

**There is no version/OS service IID or getter slot used here.** Gather constructs a local
`cRZVersion`, invokes its methods directly, and destroys it; its one-entry virtual table is a
destructor, not a measurement interface (`0x007CF804–0x007CF877`, `0x008915C1–0x008915EB`,
table `0x00AC51FC`). A native provider must independently acquire these inputs through the Windows
APIs/CPU operations below. It must not resolve and call the original private helper addresses. These
OS APIs are the acquisition boundary; inventing a game service ID for them would not be compatible.
Use the **game executable's** path and resources, never the replacement DLL's version resources.

The first six string fields are byte strings. Keep returned ANSI bytes, case, punctuation and spaces;
do not introduce labels, trim, localize, or substitute a host-formatted version number. The actual
gather order is indices **0, 1, 2, 3, 5, 4**, followed by CPU features and numeric inputs
(`0x007CF812–0x007CF877`).

| Index / name | Independent acquisition and formatting | Evidence |
|---|---|---|
| 0 `userName` | `GetUserNameA`, 512-byte buffer and input size 512. On success copy the returned NUL-terminated bytes. With failure and the null fallback argument used by gather, assign the single byte **`?`**. | `0x008919B1–0x00891A44`; fallback literal `0x00AA1B6C` |
| 1 `appName` | Current executable's version-resource **`FileDescription`** string, selected as below; no application-title getter or added text. | `0x0089174C–0x008917F6` |
| 2 `version` | Same resource selection, **`FileVersion`** string verbatim. This is not the fixed numeric file version reformatted with dots. | `0x008917F7–0x008918A1` |
| 3 `osVersion` | Call **`GetVersion`**. Major is the low byte, minor is the next byte. If bit 31 is clear, format `Windows NT %u.%u`; otherwise format `Windows %u.%u`. Both values are unsigned decimal without padding. Ignore build/service-pack/high-word data. Preserve any OS compatibility version returned to the game process; do not replace it with an unvirtualized OS version. | `0x0089151D–0x00891551` |
| 4 `computerName` | `GetComputerNameA`, 512-byte buffer and input size 512. On success copy the returned NUL-terminated bytes. On failure with gather's null fallback, assign **`localhost`**. | `0x00891A45–0x00891AD8` |
| 5 `CPU` | CPUID leaf 0 vendor bytes, in register order **EBX, EDX, ECX**, little-endian bytes from each, followed by a NUL. This is the 12-byte vendor identifier, not the processor brand/model string. If CPUID is unavailable, the outer version method clears this field to empty even though its lower-level helper writes `Unknown` before returning failure. | `0x0089194D–0x008919B0`, `0x00892BC0–0x00892C44`, particularly `0x00892C03–0x00892C0B` |

**Resource selection.** Obtain the current executable module path. The original path helper tries
`GetModuleFileNameW` for module null with capacity 260, falling back to `GetModuleFileNameA`; it
converts the path to the version helper's string encoding and then to ANSI for version APIs
(`0x00912467–0x00912512`). Load with `GetFileVersionInfoSizeA` and `GetFileVersionInfoA`. Query
`\VarFileInfo\Translation` with `VerQueryValueA`; use its **first** pair of 16-bit language and code-page
values, in that order. Form the lookup path `\StringFileInfo\%04lx%04lx\FileDescription` or
`\StringFileInfo\%04lx%04lx\FileVersion`: lowercase hexadecimal, minimum four digits per value,
no separators between them. Query with `VerQueryValueA` and copy the returned NUL-terminated string,
without using the returned length to keep an extra terminal NUL. There is no language search, preferred
locale, fixed-info formatting fallback, or fallback to another translation pair
(`0x00891610–0x008916AA`, `0x00891771–0x008917D5`, `0x0089181C–0x00891880`).

If the resource is unavailable because the initial size query returned zero, these two methods return
false **without overwriting** their destination fields. If the loaded resource has the translation
data but an individual string query fails, that destination is cleared to empty
(`0x00891759–0x00891770`, `0x008917E0–0x008917F1`; corresponding FileVersion branches).
Gather ignores these return values. On a new manager unchanged fields are empty; on regather they may
retain their previous strings. This describes the ordinary missing-resource/string cases, not the
unchecked allocation/partial-load failure paths still bounded by GR6.

**Other OS measurements.** CPU flag bits are CPUID leaf 1 **EDX**, not ECX: bit 23 for MMX and bit 0
for FPU (`0x00892D06–0x00892D17`). Without CPUID the feature mask is zero. Physical memory comes from
**`GlobalMemoryStatus`**, with a 32-byte `MEMORYSTATUS` whose length is set to 32: use the unsigned
32-bit total-physical and available-physical fields (`0x0089157F–0x008915A7`). Apply §2.1's MiB truncation
and four-MiB rounding independently. Do not silently replace these API results with 64-bit
`GlobalMemoryStatusEx` values. Screen values use the GS getter in §2.1; no desktop-resolution query is
part of `cRZVersion` acquisition.

**CPU MHz on the game's CPUID/RDTSC host profile.** There is no interface CPU-speed getter either.
The native gather asks its local helper for a measurement with parameter zero, which selects the TSC
path when CPUID EDX bit 4 is set (`0x00891552–0x00891570`, `0x00892F5E–0x00892F72`). An independent
provider can make that same measurement with `QueryPerformanceFrequency`, `QueryPerformanceCounter`
and RDTSC. For each of three trials, save the current thread priority, compute the signed-64 integer
quotient frequency/20, set priority to 15, and `Sleep(0)`. Read counters Q0 and Q1, then TSC C0. Repeatedly
read the performance counter until its value Q2 is at least Q1 + frequency/20, or the counter call fails;
then read TSC C1 and restore the saved priority. Subtract half of (Q1−Q0), arithmetic-shifted right by
one, from (Q2−Q1), clamping a negative corrected duration to zero. With nonzero duration T, calculate
**((C1−C0) × frequency) / (T × 1,000,000)** and truncate toward zero to the result dword. Zero duration
leaves the trial's initially zero result unchanged. Sort the three unsigned trial results and select
the **middle** one. Failure of `QueryPerformanceFrequency` leaves zero. The native x87 calculation
loads signed 64-bit differences/frequency, multiplies the numerator, multiplies the denominator by the
binary64 constant 1,000,000, then divides and converts with `_ftol2`; do not round to the nearest MHz
(`0x00893060–0x00893274`, especially `0x00893184–0x0089322B`, `0x00893763–0x00893770`). Finally apply
the unsigned-dword-to-binary32 conversion from §2.1. This timing measurement is subject to scheduling;
a nominal CPU-frequency registry value is not the same observation.
The worked timing path assumes successful counter reads; failure of an individual counter read is
unchecked native failure behavior within GR6, not a guaranteed zero-MHz result.

**Legacy non-TSC acquisition.** When feature bit 4 is clear, gather's zero measurement parameter
selects the instruction-loop estimator instead (`0x00892FB3–0x0089300A`). Its calibration family is
CPUID leaf 1 EAX bits 8–11, ignoring extended-family bits, clamped to 11. Without CPUID, the original
tests whether EFLAGS bit 18 (AC) can change and restores the saved flags: unchanged selects family 3,
changed selects family 4 (`0x00892B50–0x00892BB0`, `0x00892D60–0x00892DA5`,
`0x00892E6A–0x00892E86`). The table at `0x00AC53CC` supplies a calibration multiplier: family 3 =
115, family 4 = 47, family 5 = 43, families 6–11 = 38 (entries 0–2 are zero). Multiply it by 4000
to obtain U.

With a successful `QueryPerformanceFrequency`, run ten trials. Between two performance-counter
reads, perform **4000 repetitions of a 32-bit bit-scan-forward of `0x80000000`**, with the loop counter
decremented as a 16-bit quantity; the scan result is 31 and the input remains unchanged. This measures
that instruction loop, not a host-language search or a nominal-frequency API. Keep the smallest
**unsigned low-dword counter difference** D. All subsequent arithmetic is unsigned 32-bit: multiply D
by 100,000 with wrapping, divide by the integer quotient (low dword of frequency)/10, and call that
quotient M. Increment M if M modulo the low frequency dword is **strictly greater** than that frequency
shifted right one. Divide U by M and increment the result if U modulo M is **strictly greater** than
M shifted right one. That final integer is the CPU MHz result. A failed frequency query leaves zero;
zero divisors and the obsolete family-0–2 failure paths are not native recoverable-success contracts
(`0x00893280–0x008933B3`, especially `0x008932DE–0x008932EC`, `0x00893317–0x0089338F`).
GR6's unchecked-failure boundary and GR7's floating-point profile remain unchanged.

**Worked OS/string case.** Supply user bytes `Player`, computer bytes `SC4BOX`, CPUID vendor registers
EBX=`0x68747541`, EDX=`0x69746E65`, ECX=`0x444D4163`, and GetVersion result `0x00000206`.
Give the executable a first translation pair (language `0x0409`, code page `0x04E4`), with
`FileDescription` = `SimCity 4` and `FileVersion` = `1.1.641.0 ` (one trailing space). The lookup prefix
is `\StringFileInfo\040904e4\`; even if a second pair has different strings, fields 0–5 must be
**`Player`, `SimCity 4`, `1.1.641.0 `, `Windows NT 6.2`, `SC4BOX`, `AuthenticAMD`**. In particular keep
the version's trailing space. Changing GetVersion to `0x80000A04` yields `Windows 4.10`. Failed user/
computer queries produce `?`/`localhost`; unavailable CPUID makes `CPU` empty, not `Unknown`.

For a numeric fixture, total/available physical byte counts `0x065FFFFF`/`0x06100000` become 101/97
whole MiB and then **104/100** MiB. With CPUID EDX=`0x00800011`, MMX and FPU are both set. With a
performance-counter frequency of 1,000,000, Q1−Q0=2 and Q2−Q1=50,001, the corrected duration is
50,000. Three TSC differences of 120,000,000, 125,000,000 and 130,000,000 produce 2400, 2500 and 2600
MHz; the median provider output is **2500**, stored as binary32 2500. These are supplied OS/CPU
measurements for conformance, not a measured performance claim for any actual machine.
For a legacy family-6 fixture with frequency 1,000,000 and minimum trial difference 100, U=152,000,
M=100, and the final result is **1520 MHz**. Both remainder comparisons leave that value unchanged.

### 2.6 Optional sound acquisition (GR9)

Ask framework slot 5 (`+0x14`) for **service ID `0xE9D27715`, IID `0x49D28666`**, with an initially null
output pointer. With an interface, call **slot 25 (`+0x64`) once**, passing writable `cIGZString`
references for **soundDriverName first, soundCardName second**. These are the game's foreign string
interfaces, not two raw character buffers or two getters returning `char*`. Ignore the method's return
value and retain the bytes it assigned; the manager adds no prefix, separator, newline, version number
or case conversion. Release the acquired sound interface. If acquisition yields no interface, assign
the exact bytes **`Unknown`** to both fields (`0x007CFD0F–0x007CFDF7`). Do not replace successful empty
strings with that absence fallback.

The service's method is `0x00874F7C`, table `0x00AC2920` slot 25, and its QI accepts IID `0x49D28666`
(`0x008749C4`). That method performs the device choice itself and delegates to its slot 24 (`+0x60`,
the two output strings followed by a device index); providers must use **slot 25**, not guess a device
index. No sound-driver enumeration, OS default-device guess, or manager callback is needed.

**Worked sound case.** If the service method assigns `driver-x` to the first reference and `Card X`
to the second, fields 13 and 14 become exactly `driver-x` and `Card X`. If it instead assigns two empty
strings, both remain empty. An absent service produces `Unknown`, `Unknown` and no slot-25 call.
An interface fixture can distinguish the two output references and verify exactly one call and its
matching Release independently of the sound hardware installed on the test machine.

### 2.7 Independent Init, ownership and the GR11 fault boundary

**Native caller contract.** Startup creates class `0x0A1D8285` for IID `0x0A1D8284` through GZCOM
slot 3, calls manager slot 3 with the application's render properties, and immediately calls manager
slot 4 with no arguments (`0x0044C487–0x0044C4AB`). The first call is **void Init**, the second is
`SetOptionsFromConfiguration`. The caller does not test an Init status. An adapter's status 0 is
adapter evidence, not a native return contract. The application has already selected its current
graphics driver before these calls (`0x0044C420–0x0044C45D`); Init measures that live driver.

Init retains/replaces its properties reference, gathers, then parses or installs fallback (§§1, 2.2).
It does not apply rules until slot 4 is called. A second Init with the same properties still gathers
and parses, without a second properties AddRef/Release pair (`0x007D189C–0x007D18D2`). The other
examined application users apply selected options through slot 6 and obtain recommendations through
slot 7 (`0x0044A1CC–0x0044A29D`, `0x0044A333–0x0044A403`). These calls consume the manager interface;
they require no manager storage layout or renderer/view sub-object. Standard unknown IID `1` is
also supported through the base QI (`0x007CE140–0x007CE166`, `0x009457C6–0x009457DE`); no additional
application-specific manager IID or post-Init notification is required by these paths.

**State and lifetime.** There is no additional saved-state restoration after §§2.3–2.4. On hardware
the final operations are the extra Clear, Flush and `SetUIRenderingMode`, followed by readback
inspection and reference cleanup (`0x008D256C–0x008D2588`, `0x008D2771–0x008D2782`). Software
returns before that setup. A failed/null color-buffer result still takes the hardware cleanup path.
This establishes UI defaults; it does not restore a previous target, image, clear color or vertex
binding. Keep acquisition synchronous at the graphics/UI boundary, with the same current driver.
No extra renderer Init/Shutdown, driver selection, window creation/destruction, event pump or view
registration is part of gathering. The backing-store enable byte alone is saved and restored by the
independent presence measurement in §2.4.

The following ownership distinctions are part of the interface-only acquisition contract. A borrowed
result contributes **no** reference for the receiver to release. If it must outlive the synchronous
use, obtain an explicit AddRef/QI reference and release that reference only. A successful QI or
framework/GZCOM output contributes one acquired reference to balance, even when another interface
pointer denotes the same underlying object.

| Acquisition | Receiver's ownership / completion rule | Evidence |
|---|---|---|
| `RZ::GS()` or `RZ::WM()` convenience getter | Borrowed cached service pointer; do not Release that getter result | `0x00913C1A–0x00913C45`, `0x00913C46–0x00913C71` |
| Framework service output, or GS QI for either probe IID | Acquired reference; Release once per successful acquisition | `0x004177D2–0x004177E1`, `0x0098DEFB–0x0098DF2D`, §2.3 |
| 3D GS slot 3, current driver | Borrowed driver; keep the GS/current-driver lifetime stable and do not Release this getter result | table `0x00ADA9F8`, slot 3 body `0x00407BA0–0x00407BA3` |
| Window-manager slot 3, then window slot 34 | Borrowed main window, then borrowed child; neither lookup grants an owned reference | `0x009DB37A–0x009DB37D`, `0x0099DE74–0x0099DEC3` |
| Window slot 36, `GetChildAs` | On success, one acquired view reference from the child's QI; Release once, without calling Shutdown | `0x0099B8AB–0x0099B8CF`, `0x007E567C–0x007E5695` |
| View slot 4, `GetRenderer` | Borrowed renderer; its subsequent QI grants a separate acquired renderer reference | `0x0043FD00–0x0043FD06`, §2.4 |
| GZCOM renderer output, before a view exists | Own only this uninitialized renderer facade; measure and Release it without Init; do not manufacture a view window | §2.4 |

**Worked interface cases (GR11).** These are source-derived fixtures, not reported game results.

1. Let an existing view have an owner-held lifetime and a disabled, existing backing store. Record
   window, view, renderer, GS and driver identities before an independent Init. The hardware path
   performs §2.3's 20 draws, 21 clears, 21 Flush calls and two UI-mode calls. Backing-store getters
   return 0, 1, 0 around the temporary enable/restore. Balance the acquired view and renderer
   references; do not release the borrowed root/child/renderer/driver. The same live view and driver
   remain obtainable afterwards, backing-store use is still disabled, and no view/window Shutdown
   occurs. Do not expect the previous color buffer or arbitrary prior 3D state to survive.
2. With a software driver and no view yet, use only the uninitialized renderer facade for presence.
   There are no probe draws, clears, Flush or UI-mode calls. Releasing the facade does not create,
   publish or shut down a view, and no view is required merely to complete Init. Its uninitialized
   Shutdown exits immediately (`0x007C7075–0x007C707C`). Absence of a view is not by itself evidence
   that no backing store exists.
3. Use valid rules containing a property assignment and initialize a fresh manager twice with the
   same properties object. There are two gathers/parses and only one retained properties reference;
   no rule-property setter is called until slot 4. On hardware the probe runs twice. Releasing the
   independent manager balances its retained properties reference without a view lifecycle call.

**What the reported fault establishes.** At executable `+0xE2C06` (`0x004E2C06`), the native
predicate `0x004E2C00` dereferences the shared **view-window reference**, before calling its slot 22
(`+0x58`, `GetCurrentViewLevel`) and comparing the result with **1**. The getter itself is
`0x007E57F0`. The reported null read therefore occurs before this view-level call, not in a driver
draw, FSAA readback or rules evaluation. The stack's `0x004E51AD` follows an audio-ambience condition
call in `cSC4AudioAmbienceLayer::CanHandle` (`0x004E519B–0x004E51AD`). The key-off
`+0x3529C0` fault instead belongs to `cSTETerrainView3D::DrawTexturedTerrainQuad` (`0x007529C0`);
it is a distinct dereference and supplies no state-restoration rule for this manager.

Native publication occurs at view construction (`0x007F0FC4` → `0x00602120`) and is refreshed at
pre-city initialization (`0x007F45FE` → `0x00602590`). View Shutdown clears the shared view reference
without checking which view previously published it (`0x007F1529` → `0x00602130`). Thus even a
temporary view's lifecycle can disturb this shared reference; a temporary **renderer facade** in
§2.4 is a different object. None of these view publication/clearing operations belongs to manager
Init, the FSAA probe, or the backing-store getter/setter. This is a reason to check acquisition
lifetimes, not evidence that a borrowed reference was actually released in the reported runs.

**Remaining runtime observation.** Static inspection cannot identify the first loss of the view in
these particular runs. To settle GR11's residual crash attribution, an RE-side observer must replay
the same source/DLL, save, script and graphics mode for both switch settings and record:

- Entry/exit of each independent Init/gather, including probe count, current-driver identity and
  the public window-tree view identity before acquisition, after acquisition and after cleanup.
- Every view construction, pre-city publication and Shutdown above, with view identity, thread and
  caller stack, including any temporary view. Record the first write that makes the reference
  consumed by `0x004E2C00` null, or establish that no view was published before its first use. A
  read-only RE diagnostic watchpoint may identify that writer; its storage is not an implementation
  interface. Also record the matching ownership acquisitions/releases that led to a Shutdown.
- At the first failed predicate, whether a live view is still obtainable through §2.4 and whether
  the native lifecycle cleared its publication, or an unexpected writer changed it. An acquisition
  cleanup that shuts down a view is an ownership/lifecycle deviation; a first loss elsewhere needs
  that writer's cause investigated. A completed provider call with a live view does not explain a
  later null reference by itself.

The supplied 77/77 properties, 12/12 recommendations, 13/13 shadow snapshots and zero reference
warnings do not observe these lifetimes. They cannot settle that first writer or the s0 acceptance
condition. No new runtime result, crash fix, additional restore, native-layout dependency or
replacement acceptance is claimed by this answer.

## 3. Reading the rules (`ParseConfigFile`, `0x007CFE10`) *[compat]*

1. A rule parser (`cSC4RuleParser`) is made. When `cardVersion` is longer than 26 characters, its 4 characters at
   offset 8 and at offset 22, read as hexadecimal, are the card's vendor and device IDs.
2. The files `Graphics Rules.sgr` and `Graphics Options.sgr` (the distinct native bases in §3.1) are
   given to the parser, with the symbol `prefsFirstTime` set to true or false from an application flag
   (`+0xF0C` of application slot 38) *[inferred: the first run]*. `cardIdentity` starts as a copy of `cardName`.
3. On success, the rules and option groups move into the manager, and when the card was found in a `vendor` block
   (§4.4), `cardIdentity` := `"<vendor>, <card>"`. The result is the parse's success.

`Init(properties)` (slot 3, `0x007D1890`) holds the render properties, gathers the variables and parses; when the
parse fails it installs the fallback rules (§7).

The parser registers both paths before running its parse entry point. The rule path is registered with
argument 1 and the options path with argument 0 (`0x007CFF83`, `0x007CFFC1`); file-loading semantics are
those of the shared resource parser. Successful parsing swaps both completed lists into the manager;
failure does not publish partly parsed lists. `cardIdentity` is set to `cardName` before checking the parse
result, so failure leaves that default identity. Vendor/device IDs start at zero when the length check
fails; the check is strictly greater than 26 and both substrings have length four at zero-based offsets
8 and 22. There is no PCI-identity query in this manager. `prefsFirstTime` is the literal `true` or `false`
selected directly by the nonzero/zero application byte (`0x007CFFE2–0x007D0009`).

Fixed-offset ID conversion is permissive, not a four-hex-digit validation step. The shared
`RZAtoUint32HexA` skips leading CRT whitespace, accepts optional `0x`/`0X`, then accumulates a prefix
of at most eight hex digits, stopping on the first other byte; no digits gives zero
(`0x0090FCEF–0x0090FD8F`). Here its input is the NUL-terminated four-byte substring, so `12G4` gives
18 and `GGGG` gives zero without making parsing fail. A short `cardVersion` leaves both IDs zero.

The file-registration argument is a bit field, not a priority: bit 0 makes errors in that
top-level file fatal to the aggregate result. Bit 1 requests a directory watcher; neither graphics
registration sets it (`0x00681E10–0x00681E65`). Both paths are visited in registration order in one
parse session (`0x00680990`, `0x0068135C–0x00681387`, `0x006813ED`). The per-file catch records an
error except for code 2 (open failure) on an optional file, and marks failure only for a required
file (`0x006813A2–0x006813E9`). Thus a missing options file is ignored; even a non-open parse error
in that optional file does not set the aggregate failure flag. There is no per-file rollback of
already executed callbacks. The no-partial-publication rule above applies when the aggregate result
is false; optional-file errors can leave a partial options contribution in a successful publication.
After a required-file error, later registered files are still visited. The parser runs its begin
hook once, its end hook only if the aggregate succeeded, and its finish hook in either case.
`DoParseFile` treats a false `ReadLine` result as end of input and does not separately classify a
read failure (`0x00680F64–0x00680F78`). This is not an all-files transaction or an I/O-integrity check.

### 3.1 Native path acquisition and file selection (GR10)

The directory provider is the existing **`cISC4App`, IID `0x26CE01C0`**. Its slots below are
zero based on the public application face, not on another application interface. Application QI
returns that face at whole-object `+0x0C` and AddRefs it (`0x004492A7–0x004492C2`). The native
`SL` helpers use the application pointer cached by `SL::OnAppCreate` (`0x00601C00`); they do not
acquire a separate directory or filesystem service. See [application.md §9](application.md#9-directories)
for the getters' stored-path contracts. Treat their returned strings as opaque bases rather than
deducing a folder from an exported method name.

Let **P** be the rules base, **U** the options base, and **A** the automatic-log base:

| Use | Native helper | `cISC4App` slot / byte offset | Arguments after `this` | Complete path |
|---|---|---|---|---|
| Required rules | `SL::DataDirectory`, `0x00602230` | **53 / `+0xD4`** | output string reference, signed language index **−1** | P followed by literal `Graphics Rules.sgr` |
| Optional options | `SL::UserDataDirectory`, `0x00602270` | **58 / `+0xE8`** | output string reference | U followed by literal `Graphics Options.sgr` |
| Automatic configuration log (§6.2) | `SL::AppDirectory`, `0x00602210` | **51 / `+0xCC`** | output string reference | A followed by stored `computerName`, then literal `-config-log.txt` |

The dispatch and arguments are instruction-checked at `0x00602230–0x00602249`,
`0x00602270–0x00602287` and `0x00602210–0x00602227`. Slot 53 is exported as
`GetPluginDirectory`; its negative index copies the base without adding a language subfolder
(`0x0044BF90–0x0044BFAC`). Slot 58 is exported as `GetRegionsDirectory`, but is the plain
stored user-base getter (`0x00449E80`). **Neither slot 51 nor slot 56 obtains a rule-file base.**
The `.../Apps/` and `.../Plugins/` values reported for those two slots in GR10 therefore do not
justify appending either rule filename to them.

Construction is literal string append, not a path-join/search API: no separator, `Apps/`,
`Plugins/`, `Data/`, language folder or parent traversal is inserted. The supplied bases normally
include a trailing separator. If P were `C:/R` without one, the registered rules path would be
`C:/RGraphics Rules.sgr`, not `C:/R/Graphics Rules.sgr`. The manager appends the filename byte
ranges at `0x007CFF38–0x007CFF53` and `0x007CFF9B–0x007CFFB6`; the shared append body
preserves the existing prefix (`0x0040EBC0`). A relative or empty base is not repaired: the
resulting relative filename uses ordinary operating-system path resolution, not a game search.

**Parser acquisition and open order.** The COM call at `0x007CFE36–0x007CFE49` uses
`GZCOM` slot 3 (`+0x0C`) with **class ID `0x5CF9459A`, IID `0x1CF94583`, output interface
pointer**. This creates `nSCRes::cFileParser`; it is not a filesystem search service. On that
interface the manager calls slot 3 (`+0x0C`, Init, no arguments), slot 5 (`+0x14`, SetParser,
the constructed rule-command parser), then slot 6 (`+0x18`, AddInputFilePath) twice:

1. Register **P + `Graphics Rules.sgr`**, flags **1**, at `0x007CFF83–0x007CFF93`.
2. Register **U + `Graphics Options.sgr`**, flags **0**, at `0x007CFFC1–0x007CFFD1`.

Slot 8 (`+0x20`, ParseFiles, no arguments) then parses those registrations in that order
(`0x007D003B–0x007D0041`, `0x0068135C–0x00681387`). Registration copies the supplied filename
and records its parent for include handling; it does not locate an existing copy
(`0x00681DA0–0x00681E23`). Each top-level attempt constructs a `cRZTextFile` with that full
filename and opens it with arguments **(1, 2, 7)** (`0x00680EC7–0x00680EE2`): read access,
open existing. The underlying `cRZFile` opens the supplied path through the Windows file API
(`0x0093066E–0x00930682`, `0x00919B00–0x00919C77`). There is no game-filesystem mount,
resource-key lookup, archive lookup, directory enumeration or alternate-directory retry here.
In particular, neither the parser nor the manager tries the other base after open failure.
Explicit `include`/`sinclude` directives can name additional files; their parent-relative joining
is the separate contract in [sc-resources.md §4.2](sc-resources.md#42-input-and-failure-contract).

**Duplicate and missing-file outcomes.** In this table P and U are distinct directories;
“present” means an accessible, otherwise valid file with the indicated basename. These are
top-level selection rules, before any explicit includes in the selected contents.

| Copies of the indicated basename | `Graphics Rules.sgr` | `Graphics Options.sgr` |
|---|---|---|
| In both P and U | Read P's copy only; U's copy is ignored. | Read U's copy only, after the rules attempt; P's copy is ignored. |
| In P only | Read P's copy. | Attempt U, ignore its optional open failure; P's copy is ignored. |
| In U only | Attempt P, record required open failure; U's copy is ignored. | Read U's copy after the rules attempt. |
| In neither | Attempt P and record required open failure. | Attempt U and ignore its optional open failure. |

A valid required rules file with no options file still succeeds. A missing required rules file
makes the aggregate parse fail even if the options file exists and parses; the options attempt
still occurs. Neither file present also fails. Failed aggregate parsing does not publish either
temporary list, and `Init` installs fallback rules (`0x007D18C2–0x007D18CD`). As in §1, native
`Init` has no success/status return; GR10's reported status is a replacement-side observation.
Optional-file syntax errors retain the partial-contribution behavior already specified above.

Source-derived path fixture: with P=`C:/R/`, U=`C:/U/`, A=`C:/A/` and stored computer name
`HOST`, the two registered paths are exactly `C:/R/Graphics Rules.sgr` and
`C:/U/Graphics Options.sgr`, in that order. A conflicting `C:/U/Graphics Rules.sgr` or
`C:/R/Graphics Options.sgr` contributes nothing unless explicitly included. The automatic log
path is independently `C:/A/HOST-config-log.txt`. These are expected examples, not executed
native/replacement parity evidence.

## 4. The language *[compat]*

The files use the game's command parser (`nSCRes`): one command per line with its arguments, `#` comments, `set
name value` symbols used as `$name`, blocks closed by `end`, and `sinclude "file"` (the shipped rules end with
`sinclude "Video Cards.sgr"`) *[inferred: these generic commands belong to the shared parser]*. Arguments are
words, numbers or quoted strings; a switch is a `-word` argument. Names of flags and variables are matched against
the tables of §2 (`nSCRes::ParseEnum`); a property name is resolved with the render properties' find, booleans first,
then integers, then floats (`AddOptionToSet`, `0x007E4050`); an unknown one is the parse error
`Unknown render property`.

### 4.1 Option groups

```
optionGroup <group number>
   option <option number>
      property <name> <value>
      ...
   ...
end
```

Group and option numbers are unsigned 32-bit numbers and grow dense tables through index+1 as needed.
New intervening groups have recommendation 0 and no options; new options are empty property sets.
The 16-byte default group is zeroed at all four dword offsets (`+0`, `+4`, `+8`, `+0xC`) by
`0x007D0410–0x007D041F`: recommendation 0 followed by an empty options list. The option-group parser
constructs this fill value at `0x007E5549` and passes it with the requested group count (index+1) to
the resize helper at `0x007E5552`. The export's `cSC4ResponseCurve` name for the shared zeroing body
does not describe its use here; the caller establishes the option-group contract.
Reopening a group/option selects its existing set and appends properties (`0x007E54C0`, `0x007E4910`).
Group `end` clears the parser's current-group selection to -1 (`0x007E3B50`). A property line requires a
preceding option selection; `DoProperty` constructs no implicit option (`0x007E4490`). This is a structural
precondition, not a promise of a recoverable error for malformed blocks that violate it.

### 4.2 Rules

```
rule ["name"] [-partial] [-any]        or        partialRule ["name"] [-any]
   has <flag>              hasNo <flag>
   atLeast <number> <v>    atMost <number> <v>
   stringMatch <string> <pattern>      stringNotMatch <string> <pattern>
   property <name> <value>
   option <group> <option>
   <nested rules>
end
```

- `partialRule` is `rule -partial`. An unnamed rule takes the previous emitted rule's name followed by `>`;
  with no previous rule, the constructor name `unknown` remains (`0x007E45F4–0x007E464A`).
- Each rule records its **depth** (how many rules enclose it).
- `has f` requires flag f set, `hasNo f` requires it clear (both set f in the rule's mask); `atLeast n v` requires
  number n ≥ v, `atMost n v` requires n ≤ v; `stringMatch s p` requires string s to match the wildcard pattern p
  (`WildcardMatch`, case-insensitive), `stringNotMatch` the opposite.
- Without `-any` **all** the conditions must hold (`0x007CE900`); with `-any` **one** is enough (`0x007CEA80`).
  A rule without conditions always matches, but a `-any` rule without conditions never does *[quirk]*.

Conditions are stored by kind, rather than interpreted as a Boolean expression in source order. Flags
share one expected-value bitset and one mask: `has` sets an expected bit, `hasNo` clears it, and both set
its mask bit (`0x007E3ED0`, `0x007E3F90`). Repeating a flag overwrites that bit's expectation, so `has MMX`
followed by `hasNo MMX` means only the latter, even in all mode. All mode requires masked flags equal to
the expected bitset; any mode requires at least one masked bit to agree. Numeric and string conditions
append separate entries, including duplicates. Short-circuit order is flags, lower bounds, upper bounds,
positive string matches, negative string matches (`0x007CE900`, `0x007CEA80`). Properties and option
recommendations are actions, never conditions.

**Threshold conversion is significant.** Both bounds call the shared real-number parser and `_ftol2`, then
store only EAX (`0x007E435B–0x007E4368`, `0x007E440F–0x007E4417`). Conversion truncates toward zero and keeps
the low 32 bits; comparison loads those bits as a **signed integer**, not a float. Thus 2.9 becomes 2 and
-2.9 becomes -2. The helper converts to signed 64-bit first (`0x009EEF13`), so representable larger values
wrap at 2^32: 2147483648 compares as -2147483648. An invalid signed-64 conversion under masked x87
exceptions produces integer-indefinite with low word zero. Real-number expression parsing is the internal
shared `nSCRes::ParseFloat` algorithm, not an OS-provider callback; unconsumed trailing input is rejected
as `Bad real number` (`0x0067BBC8–0x0067BBEB`). Do not round thresholds to binary32 before integer conversion.
Environment numbers, unlike thresholds, are already binary32.

Both bounds are inclusive for ordered operands. On a supplied NaN environment value, all-mode numeric
tests pass, while any-mode numeric tests do not provide a match: the native x87 status tests differ
(`0x007CE93B–0x007CE943`, `0x007CE975–0x007CE97D`, `0x007CEABD–0x007CEAC5`, `0x007CEAF7–0x007CEAFF`).
Gathering supplies integer-derived finite values; NaN is an evaluator-fixture/embedding edge case.
For unordered comparison the x87 status has C0=C2=C3=1. All-mode lower-bound rejection tests
`AH & 0x41` for zero; upper-bound rejection tests odd parity of `AH & 0x05`. Neither rejects
unordered. Any-mode lower-bound acceptance tests odd parity of `AH & 0x41`, and upper-bound
acceptance tests `AH & 0x01` for zero; neither accepts unordered. This is the normative answer
to GR5, including when both numeric condition lists contain entries. A failing flag or string
condition can still reject the all-mode rule.

The matcher receives the variable as its first argument, the pattern as its second, and zero as its third
(`0x007CE9C0–0x007CE9CA`, `0x007CEA2D–0x007CEA37`; the any-condition path uses the same convention).
Zero makes temporary copies of **both** strings lowercase before matching (`0x00918F08–0x00918F18`);
the stored environment and pattern are unchanged. Matching covers the entire string. `?` consumes exactly
one byte, `*` consumes zero or more bytes, including separators; all other bytes are literal. There are no
character classes or escape syntax in `MatchPattern` (`0x00918103–0x0091814D`). In particular, `*` matches
the empty string, while `?` does not. ASCII case behavior is portable; non-ASCII lowercase behavior belongs
to the shared string service, rather than a Unicode case-folding rule invented by this manager.

### 4.3 Values

Booleans are parsed by the parser's boolean reader (`true` / `false`, as the shipped file writes them), integers
and floats by its number readers.
Flag, numeric-variable and string-variable names require exact, case-sensitive enum spelling: the shared
enum reader compares equal-length byte sequences (`0x0067AB23–0x0067AB2F`) and throws on unknown names.
Case-insensitive wildcard matching therefore does not make `has mmx` an alias for `has MMX`.

### 4.4 Video cards

```
vendor "name" <id> [<id> ...]
   card <device id> "card name"
end
```

A `vendor` block whose IDs include the detected vendor ID enables its `card` lines; a `card` whose device ID equals
the detected one marks the card found and records its name (`0x007E4730`, `0x007E4800`).
Before a new vendor is tested, the active vendor name is cleared; a matching ID records that block's name.
After the first matching card, the found latch suppresses later vendor/card changes. A matching vendor
without a matching card does not change `cardIdentity` (`0x007E484A–0x007E48F8`, `0x007E4730`).

### 4.5 Shared-parser facts and remaining grammar boundary (GR4)

These are source-established requirements in addition to the graphics callbacks. They do not yet
close the complete shared lexical/command grammar requested in GR4.

- Registration calls the base `cCommandParser::RegisterCommands` first, then registers exactly the
  four graphics commands `rule`, `partialRule`, `optionGroup`, `vendor` (`0x007E3DF0–0x007E3EC8`).
  Shared registration includes `define`, `enddef`, `create`, `arrayCreate`, `namespace`
  (`0x00678B00–0x00678D7D`). These names are not additional graphics-owner callbacks.
- Dispatch substitutes variables in **every** token, including the command name, before searching
  block command tables from innermost to outermost and then the root table (`0x00679170–0x006792C1`).
  An unrecognized command throws `Unknown command`; a recognized block callback can reject an
  unexpected switch after running. This is not a fixed parse of only the shipped command subset.
- Variable substitution scans within tokens, accepts `$name` and `${name}`, and reports
  `Unknown variable` when lookup fails (`0x00677B90–0x00677F84`). Name scanning accepts letters,
  digits, underscore and colon. The scan advances through the original input while building a
  replacement output; it does not recursively rescan inserted replacement text. Full quoting and
  boundary-dependent parenthesization remain part of open GR4.
- The shared `eval` command requires exactly one argument, splits that argument into command
  tokens and calls `ParseCommandArguments` (`0x00679A20–0x00679AA2`). Its body invokes no Lua API.
  The numeric reader likewise uses the native expression routines below, not a Lua parser.

The verified numeric precedence, from weakest to strongest, is left-associative `+ -`,
left-associative `* / %`, right-associative `^`, recursive unary `+ -`, and primaries/parentheses.
Consequently `2^3^2` is 512 and `-2^2` is 4. `EvalFactor` handles signs before `EvalTerm` handles
power (`0x0067B360–0x0067B3AB`, `0x0067B920–0x0067B987`). Additive and multiplicative accumulators
are stored as **binary64** after operations (`0x0067B990–0x0067BA3C`,
`0x0067BA4E–0x0067BAC6`); do not evaluate the whole expression in binary32. `%` computes
left minus `floor(left/right)` times right, with binary64 spill of right and quotient before
the floor helper (`0x0067BA06–0x0067BA2F`). The expression wrapper rejects empty expressions and
trailing garbage (`0x0067BB20–0x0067BB9B`). Decimal conversion uses `StrToDEnglish`; lowercase
`0x` selects unsigned hexadecimal conversion (`0x00679DA8–0x00679DF4`). Function names are parsed
as a letter followed by alphanumeric/underscore bytes. Function/domain/overflow details not
established here remain open; the existence of functions is not permission to substitute an
unrelated language evaluator.

`ParseUint` truncates by `_ftol2` and retains low 32 bits (`0x0067BC82–0x0067BC8F`), as do the
threshold callbacks in §4.2. The signed property reader `ParseInt` is **different**: add binary32
12582912 to the expression result, store binary32, reinterpret those bits as an integer, and
subtract `0x4B400000` modulo 2^32 (`0x0067BC26–0x0067BC61`). It is not truncation or a checked
integer parse. Under round-to-nearest/even, 2.5 gives 2 and 3.5 gives 4. Outside the small exact
range this bit trick must not be replaced by a mathematical integer conversion. Native behavior
depends on the active x87 rounding control.

Dense group and option creation have no checked `index+1` overflow. `UINT32_MAX` wraps the resize
target to zero (`0x007E5546`, `0x007E4996`); the selected index remains `UINT32_MAX`. Group resize
can shrink/clear the vector (`0x007D0FD0–0x007D1015`). Later selected-element access is unchecked.
There is no evidence for a recoverable parser error for this input or allocator failure. A host
adapter may reject unsafe input, but cannot claim that rejection is native parser parity.

## 5. Evaluation (`ApplyRules`, `0x007D10B0`) *[compat]*

The rules are walked in file order with a stack of depths:

1. When the stack is empty or the rule is deeper than the top, it is tested. If it fails, it and every following
   deeper rule (its children) are skipped. If it matches:
   - log `applying <name>`;
   - apply its property set;
   - for each `option g o`, the recommended option of group g := o (the table grows as needed), logging
     `  option g = o`;
   - if the render property `kForceSoftwareDriver` is now true, clear the `hardwareDriver` flag;
   - unless the rule is partial, push its depth.
2. Otherwise (the rule is not deeper than the top), every following rule at least as deep as the top is skipped and
   the top is popped.

So among siblings the first matching non-partial rule wins and stops the rest (an if / else-if chain); partial
rules never stop their siblings; children are tested only under a matching parent.

Rule `option g o` actions only write recommendations, in source order; duplicate recommendations last-win.
They may grow the group list, but do not create option property sets, validate o against existing options,
or call `ApplyOptionGroupSetting`. No preference option is automatically applied by this evaluator.
Recommendations and render properties persist between evaluations. `ApplyRules` resets only its local
depth stack; it does not gather information, reparse, or reset recommendations. Once a matching rule sees
`kForceSoftwareDriver` true it clears flag 0, but a later false assignment does not restore that flag.
`ApplyOptionGroupSetting` itself does not update flags or recommendations: that flag check belongs only
to matching-rule evaluation. Another gather recomputes flags from the provider and current properties.

## 6. Applying and logging

- `SetOptionsFromConfiguration` (slot 4, `0x007D18E0`): logging is on unless the command line has
  `-writeLog:off` (case-insensitive). With logging, the log file `<app directory><computerName>-config-log.txt` is
  opened and the variables are written (`LogConfigInfo` `0x007CEF40`: `name = value` with `%s`, `%d` or `%g`); then
  `ApplyRules` (log headings `Rules`, `-----`); then, with logging, the option summary (`LogOptions` `0x007CE260`,
  lines such as `Texture Quality      current: %3d recommended: %3d`) and the file is closed.
- `ApplyOptionGroupSetting(g, o)` (slot 6, `0x007CE870`): applies option o's property set of group g; out-of-range
  numbers do nothing.
- `GetOptionGroupAutoSetting(g)` (slot 7, `0x007CE8D0`): the recommended option of group g, 0 when g is out of
  range.

`LogConfigurationToFile(path)` (slot 5, `0x007D0420`) is a distinct operation: gather information again,
set/open the supplied path, then if open write configuration information, close, and return true; if not
open return false. It never parses, installs fallback, applies rules or logs option recommendations. It
does not consult `writeLog`. A failed open still leaves the newly gathered environment in the manager.
Conversely `SetOptionsFromConfiguration` does not gather again; it evaluates the stored environment even
with logging disabled or file opening unsuccessful. Logging failure does not suppress rule evaluation.

`LogConfigInfo` starts with `Log generated on <date>, HH:MM` (date/time from the shared clock services),
then emits `Flags`/`-----`, `Strings`/`-------`, `Numbers`/`-------`, with blank lines between sections.
Flags use `%s` values `true`/`false` in §2 order, strings use `%s` in index order, then numbers use `%g`
in index order, as `name = value` (`0x007CEF40`). `LogOptions` also dumps
all render bools, ints and floats by ascending index, using their property names and `%s` (`true`/`false`),
`%d`, `%g` respectively. Its preference summary uses unsigned application bytes as current values and
`GetOptionGroupAutoSetting` as recommended values, not current renderer property values:

| Order/label | Group | Application-byte offset |
|---|---|---|
| Visual Effects | 0 | `+0xF0D` |
| Cars/Sims | 1 | `+0xF0E` |
| Shadows | 2 | `+0xF0F` |
| Atmospheric effects | 4 | `+0xF11` |
| Water effects | 5 | `+0xF12` |
| Software/Hardware | 6 | `+0xF13` |
| Cursor | 7 | `+0xF14` |
| VariableSpeedAutomata | 10 | `+0xF18` |
| Translucent UI | 8 | `+0xF15` |
| Graphic Detail | 9 | `+0xF16` |
| Building Load Speed | 15 | `+0xF28` |
| Texture Quality | 3 | `+0xF10` |

If the application-preference object is absent these twelve summary lines are omitted; property dumping
still occurs (`0x007CE260`). Each summary uses `current: %3d recommended: %3d` after its padded label.

### 6.1 Byte and write boundary (GR6)

`LogOptions` emits `Options`, `-------`, the twelve preference lines when available, one blank
line, `Properties`, `-------`, bool properties, a blank line, int properties, a blank line,
float properties, and a final blank line. Property lists have no separate type headings
(`0x007CE293`, `0x007CE2C3`, `0x007CE623–0x007CE6A2`, `0x007CE6A2–0x007CE85C`).
Each preference label occupies 20 bytes, followed by one space and `current:`; thus `current:`
starts at zero-based byte 21, including for the 20-byte `VariableSpeedAutomata`. Values use
signed decimal `%3d`, a minimum width, never clipping a larger number. Current values are
zero-extended application bytes (`0x007CE30C–0x007CE31D`, `0x007CE4E0–0x007CE4EB`).

The log text file is initialized in byte mode. `WriteLine` writes the formatted payload with
its explicit byte length, then writes bytes `0D 0A` separately. It writes no trailing NUL.
If the payload write returns false, it skips the terminator; otherwise its return is the
terminator write result (`0x009306DE–0x0093072A`). The alternate wide mode writes `0D 00 0A 00`,
but is not selected by these manager entry points. This corrects the provisional LF rule.

The manager tests whether the file is open before formatting each line, but ignores
`WriteLine` results. `LogConfigurationToFile` checks open state after attempting open; after
that check succeeds it logs, closes, and returns true regardless of write/close return values
(`0x007D0446–0x007D0463`). Consequently successful return does not certify a complete file.
`SetOptionsFromConfiguration` likewise does not turn I/O failure into failed rule evaluation.
The external formatter remains the byte-oriented string service; §12 distinguishes its
foreign formatting primitives from manager-owned line construction.

`%s` for a property name is NUL-terminated **bytes**, with no UTF-8 validation, normalization or
replacement. `Sprintf` takes its byte length and appends that exact range
(`0x0090F915–0x0090F97A`, `0x0090FB46`); its glyph-count calculation does not transcode the
appended bytes. In these unpadded, unqualified `%s` fields it does not truncate to a character
count. An embedded NUL ends the name; `%` in a name is data, not a second format string. A
non-UTF-8 name is not grounds for the native logger to reject the profile. Numeric `%g` is
delegated by `Sprintf` to native CRT `__snprintf` (`0x0090F83F–0x0090F8DB`), with a promoted
binary64 argument and default precision. The exact exceptional-value spelling is still an
explicit GR6 evidence gap, not permission to adopt a host formatter silently.

### 6.2 Automatic log directory (GR10)

When automatic logging is enabled, `SetOptionsFromConfiguration` obtains **A** through
`SL::AppDirectory`: `cISC4App` IID `0x26CE01C0`, **slot 51 / `+0xCC`**, with an output string
reference (§3.1). It then appends the stored `computerName` byte range followed by
`-config-log.txt` (`0x007D19E5–0x007D1A27`). No separator, subfolder or filename sanitization is
inserted. This uses neither the rules base P nor the options base U; the measured `.../Apps/`
return for slot 51 applies to this log path.

The manager supplies that complete path to the log file's slot 28 (`+0x70`, SetPath), then
calls slot 3 (`+0x0C`, Open) with **(2, 1, 1)** (`0x007D1A2C–0x007D1A45`): write access,
create/replace. It does not create missing parent directories or retry under another base.
Open failure leaves logging ineffective while rule evaluation still proceeds (§6).
`LogConfigurationToFile(path)` remains the separate explicit-path operation: the caller's
path is used directly, without acquiring or prepending any of these three bases.

## 7. Fallback rules (`SetFallbackRules`, `0x007D13C0`)

When the files cannot be parsed, `Using fallback rules.` is logged and three rules are built in:

| Rule | Depth; condition | Properties, in write order |
|---|---|---|
| `standard software` | 0; `hasNo hardwareDriver` | `RenderCloudsAndFog`, `RenderShadows`, `RenderDecals`, `UseSecondStage` false |
| `standard hardware` | 0; **no conditions** | `RenderCloudsAndFog`, `RenderShadows`, `RenderDecals` true, `UseSecondStage` false |
| `two stages` | 1; `atLeast textureStages 2` | `UseSecondStage` true |

All three use all-condition mode and are non-partial. The second rule is the unconditional alternative to
the first, not a positive hardware-flag test. The software match suppresses both the hardware alternative
and its child. Thus a software environment never enables the second stage, even if its reported stage
count exceeds two. Hardware with fewer than two stages receives the second row only; at exactly two it
also receives the third row. Installing fallback clears/replaces the rule list, without constructing any
option groups or applying any properties until evaluation.

Evidence: default rule depth, partial/any bytes and flag masks are zero (`0x007D0523–0x007D052F`);
the software rule clears expected flag bit 0 and sets mask bit 0 (`0x007D143B–0x007D144A`). The hardware
rule is freshly constructed at `0x007D15A5`, with no condition additions before its insertion at
`0x007D171A–0x007D174D`. The child stores depth 1 and the integer pair (numeric index 8, threshold 2)
at `0x007D1782–0x007D17AA`; `RuleMatchesAll` loads that threshold as a signed integer and compares it
to the environment float inclusively (`0x007CE936–0x007CE943`).

## 8. Users

`cSC4App::PostFrameWorkInit` and `DoPreferencesChanged` (command-line.md) call `SetOptionsFromConfiguration`; the
graphics options dialog uses slots 6 and 7.
The startup factory/Init/evaluate sequence and independent acquisition lifetimes are specified in §2.7 (GR11).

## 9. Quirks

1. Partial rules apply and never stop their siblings; a plain rule stops its later siblings even if they would match
   better (§5).
2. Card IDs are read at fixed offsets of `cardVersion` (§3).
3. An unnamed rule's name is its predecessor's name with `>` appended, whatever the nesting (§4.2).
4. A `-any` rule with no conditions never matches (§4.2).

## 10. Open questions

See [questions/graphics-rules.md](questions/graphics-rules.md) for remaining evidence gaps.
GR1–GR3 are resolved in §§2, 4.2 and 7.

## 11. Conformance tests

1. **First match:** siblings `rule "a"` (`atLeast memory 512`) and `rule "b"` (no conditions) with memory 1024:
   only "a" is applied; with memory 256 only "b".
2. **Partial:** two sibling `partialRule`s without conditions are both applied.
3. **Any:** `rule -any` with `atLeast memory 4096` and `has MMX` matches on an MMX machine with 512 MB.
4. **Children:** a child of a failing rule is never tested; `rule -any` with only a `property` line is never
   applied.
5. **Options:** `option 2 3` in a matching rule makes `GetOptionGroupAutoSetting(2)` = 3; group 99 gives 0.
6. **Unknown property:** `property noSuchThing 1` is a parse error.
7. **Software:** a matching rule that sets `forceSoftwareDriver true` clears `hardwareDriver` for the rules after it.
8. **Log name:** on a machine named `ALPHA` the log is `ALPHA-config-log.txt` in the application directory.
9. **Independent Init (GR11):** §2.7's worked cases distinguish counted/borrowed acquisition, preserved
   view lifetime and backing-store enabledness, software early exit, and evaluation after Init.

Concrete scripts, injected inputs and independent expected results are in
[graphics-rules-vectors.md](graphics-rules-vectors.md). They are proof-derived fixtures, not claims of
executed native/shadow runs.

## 12. Owner-method inventory and implementation boundary

This inventory covers every named manager entry returned by the export, including all public slots;
supporting command methods are listed separately so an implementer does not mistake a parser callback
for a missing manager operation. Allocation/vector relocation helpers implement the value-list ownership
described in §1; they add no rule syntax. The inventory is not a new coverage-byte measurement.

| Manager entry/address | Contract |
|---|---|
| CLSID `007CE130`; QI `007CE140`; QI thunk `007D0DA0`; factory `007D1030` | §1 class/interface identity and normal unknown-interface delegation |
| constructor `007D0CC0`; destructor `007D0DC0`; deleting destructor `007D1050`; destructor thunk `007D0DB0` | §1 defaults and ownership |
| ApplyPropertySet `007CE1F0` | §1 typed ordered writes, duplicates retained |
| LogOptions `007CE260`; LogConfigInfo `007CEF40` | §6 preference summary, renderer dump, environment dump |
| ApplyOptionGroupSetting `007CE870`; GetOptionGroupAutoSetting `007CE8D0` | §6 unsigned bounds checks; getters return 0 outside range; setters have no effect outside range |
| RuleMatchesAll `007CE900`; RuleMatchesAny `007CEA80` | §4.2 masks, integer thresholds, wildcard polarity and short circuit |
| GatherInformation `007CF780` | §§2–2.1 provider acquisition and conversions |
| ParseConfigFile `007CFE10` | §3 fixed IDs, paths, symbols and successful-list transfer |
| LogConfigurationToFile `007D0420` | §6 regather, explicit path, success boolean, no evaluation |
| ApplyRules `007D10B0` | §5 depth traversal and persistent side effects |
| SetFallbackRules `007D13C0` | §7 exact three-rule construction |
| Init `007D1890`; SetOptionsFromConfiguration `007D18E0` | §§3/6 initialization versus evaluation/logging |

| Supporting entries | Contract |
|---|---|
| default group value `007D0410`, used at `007E5549`; group resize `007D0FD0` | §4.1 zero recommendation and empty options list used to fill newly added groups |
| cSC4ConfigRule constructor `007D0500` | name `unknown`, depth 0, non-partial/all mode, empty conditions/actions |
| cSC4RuleParser constructor `007E4DE0` with export continuation `007E4DE9`; destructor `007E5030` | own parser lists/provider reference; vendor/device IDs initially 0; dispose parser-owned lists after swaps |
| parser RegisterCommands `007E3DF0`; ParseBegin `007E5580`, thunk `007E5620` | register rule/partialRule, optionGroup, vendor; parse-begin clears depth/rules/groups and acquires render-property service |
| rule command constructor `00677290`, RegisterCommands `007E3C60`, Parse `007E4550`, EndBlock `007E3DD0` | §4.2; append flat preorder rule with current depth, push its index, increment depth; end decrements depth and pops current-rule index |
| DoHas `007E3ED0`, DoHasNo `007E3F90`, DoAtLeast `007E4300`, DoAtMost `007E43B0`, DoStringMatch `007E5160`, DoStringNotMatch `007E5400` | §4.2; target current enclosing rule, not merely the most recently emitted child |
| rule DoOption `007E4180`, DoProperty `007E4250` | append recommendation/property action to current rule |
| option-group RegisterCommands `007E3B70`, Parse `007E54C0`, EndBlock `007E3B50`, DoOption `007E4910`, DoProperty `007E4490` | §4.1 selection, table growth, accumulation and structural preconditions |
| vendor RegisterCommands `007E3C00`, Parse `007E4800`, DoCard `007E4730` | §4.4 active-vendor filtering and first-card latch |
| AddOptionToSet `007E4050` | §4 typed property lookup/parse; bool before int before float; unknown-property failure |

The command parser, tokenization, symbols/includes and real-number expression reader are internal shared
`nSCRes` dependencies. A complete implementation must supply/reuse those algorithms as well as these
owner callbacks; they are not environmental facts to obtain from a hardware/OS provider. The scripts here
use the specified literal command subset. This spec does not infer extra expression operators or lexical
escapes from the shipped files. The externally injectable operations are only those listed in §2.1,
filesystem/path acquisition and the application preference/command-line data used in §§3/6. The rule
manager and its callbacks own deterministic parsing into lists, wildcard matching, numeric conversion,
tree traversal, recommendation updates and typed property dispatch.

### 12.1 Concrete shared bodies still required by complete-owner credit

These are actual native dependency entries, not proposed new graphics methods. Existing callback
inventory above remains the graphics-specific owner inventory. A literal-only parser does not
cover these dependencies. Conversely their foreign primitives do not imply ownership of Lua.

| Native entry | Responsibility and contract status |
|---|---|
| `0067B3B0` SplitIntoArguments | token/bracket/quote/comment splitting; full lexical tables and escapes remain GR4 |
| `00677B90` SubstituteVariables | token interpolation and unknown-name errors; boundary parenthesization remains GR4 |
| `00678B00` RegisterCommands; `00679120` ParseCommandArguments; `00679450` ParseCommand | base command registry, substitution, enclosing-block/root dispatch; full registration/arity remains GR4 |
| `00677510` cSetCommand::Parse | typed symbol construction; complete aliases, extra-argument and conversion policy remain GR4 |
| `00679A20` cEvalCommand::Parse | exactly one argument, retokenize and dispatch; no Lua call in this body |
| `00676B30` StartBlock; `00676110` cEndBlockCommand::Parse | push block; end calls its end hook and pops; end outside a block throws; EOF validation remains GR4 |
| `00681DA0` AddInputFilePath; `00680990`/`006812F0` ParseFiles; catch `006813A2`; continuation `006813ED` | registration bits, ordered parse and optional-file failure policy, §3 |
| `00680EA0` DoParseFile; continuation `00681176`; `00680B20` relative-path test; `00680C40` path-prefix insertion; `00680E30` FindParentDir | file reading/includes/path construction; full sinclude distinction and cycle behavior remain GR4 |
| `00679DA0` EvalPositiveConstant; `0067AC30` parenthesized-expression helper; `0067AC60` EvalSignedNumber | primary/function selection; complete function/domain and literal rules remain GR4 |
| `0067B360` EvalFactor; `0067B920` EvalTerm; `0067B990` multiplicative layer; `0067BA40` additive layer; `0067BB20` ParseExpression | signs, precedence, operation order and expression termination, §4.5; general Lua parsing is not required |
| `0067BBA0` ParseFloat; `0067BC00` ParseInt; `0067BC70` ParseUint; `0067BCD0` ParseBool | native conversion wrappers; bool aliases/exceptional conversions remain GR4 |
| `0090F574` cRZString::Sprintf; `0090E312` byte append; `009306DE` WriteLine | shared string/text algorithms used by §6.1; append ends in byte memcpy (`0090E355`) |

Standard foreign primitives called by these algorithms include CRT character classification,
decimal/hex numeric conversion, memcpy, floor/pow and the math library, `_ftol2`, and printf
numeric formatting. Their observable profiles must be matched or explicitly bounded; they are
not a license to replace the owned dispatcher, expression precedence or symbol/include algorithms
with a provider callback. The decompiler mislabels the call at `0067BC8A` as a Lua operation in
some pseudocode: the instructions call `_ftol2` at `009EEF04`. Do not infer a Lua dependency from
that label. This round does not assert exhaustive absence of all scripting paths in base commands
whose registration/body coverage is still open.

Fallback construction stores each bool-name lookup result directly in its action record without
checking for an unknown-name result (`007D1441–007D146D` and following analogous property additions).
It does not turn missing fallback properties into a second parser failure. Applying those records
uses the ordinary renderer setter boundary; no replacement name or property is invented.
