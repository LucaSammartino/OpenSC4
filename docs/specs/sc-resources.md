# SC resource scripts and text persistence

Behaviour specification for SimCity 4 Deluxe 1.1.641. Evidence: exported function
bodies and instruction checks, recorded 2026-10-23 by `re-persist-sl5`.
Requirements marked *[compat]* describe the original; *[choice]* identifies a
portable implementation decision. Addresses are evidence anchors, not an
implementation prescription. Unsettled boundaries are listed in
[sc-resources questions](questions/sc-resources.md).

## 1. Boundaries and shared contracts

This spec covers `nSCRes` resource-script tokenization, its command and file
parsers, `cSLTokenRecord`, `cSLTokenSerialRecordIO_Text`,
`cGZPersistBufferResource`, and `cGZPersistASCIIWriter`.

Reuse [persistence](persistence.md) sections 2–4 for DB records, serial-record
byte order, field IO and persistent-resource ownership. These owners do not
define another DBPF container format. In particular, `cGZPersistBufferResource`
is a BMP/TGA image adapter, distinct from the versioned `cGZBufferResource`
format in persistence section 6.

Reuse [effects-parser](effects-parser.md) section 9 for switch matching,
positional lookup, token marks and their failure vectors. Reuse
[graphics-rules](graphics-rules.md) section 3 for registered-file ordering,
required/optional files and parser hooks. Numeric expressions also have the
floating-point environment side effect in [foundations](foundations.md)
section 18.2. No owner here adds a direct drawing interface: image drawing is
performed through its retained `cIGZBuffer`.

## 2. nSCRes lexical and expression services

### 2.1 Method inventory and token boundaries

These are namespace services, not virtual methods; there are no vtable slots.

| Service | Address | Contract |
|---|---|---|
| CountArguments | `0x0067A320` | Count script arguments under the grouping/quoting rules |
| FindNonSwitchIndices / FindSwitchArgumentIndices | `0x0067A660` / `0x0067A7D0` | See effects-parser section 9 |
| NextToken | `0x0067A960` | Advance through the lexical token stream |
| ParseEnum | `0x0067AAF0` | Resolve an enumeration value; detailed unmatched-name policy remains SCR1 |
| SplitIntoArguments | `0x0067B3B0` | Produce command and argument tokens |
| ParseFloat / ParseInt / ParseUint / ParseBool | `0x0067BBA0` / `0x0067BC00` / `0x0067BC70` / `0x0067BCD0` | Convert a complete expression or boolean spelling |
| ParseColor / ParseColorAlpha | `0x0067BDE0` / `0x0067BE90` | Color conversion; component scaling remains SCR1 |
| ParseVector2 / ParseVector3 | `0x0067BF90` / `0x0067C060` | Vector conversion; exact component grammar remains SCR1 |

*[compat]* Whitespace comprises space, tab, LF and CR. `#` starts a comment.
Both single and double quotes group a token. Brackets, parentheses and braces
pair with `]`, `)` and `}` respectively. Outer quote/group delimiters are
removed from the resulting argument. An unterminated quote reports error 5;
an unterminated group reports error 6. These delimiter sets are established by
initializers `0x00A74350`–`0x00A74410`. Mixed nesting and escape syntax remain
SCR1; applications must not assume shell-style escaping.

### 2.2 Numeric conversion

*[compat]* Numeric conversion requires a complete expression: empty input,
invalid terms and unconsumed trailing text are errors. Product-level operators
`*`, `/`, `%` bind more tightly than `+` and `-`. The repeated arithmetic
accumulators are stored in binary64 after each iteration. Operand grammar and
the exact remainder operation remain SCR1.

`ParseInt` is not an ordinary truncate-to-integer conversion. It adds binary32
12582912, stores the result as binary32, and subtracts `0x4B400000` from that
stored value's bit representation. This rule is relevant within the normal
small-integer conversion range; no saturating overflow rule is established.
`ParseUint` instead uses the runtime floating-to-integer conversion and retains
the low 32 bits. Instruction evidence: `0x0067B990`, `0x0067BA40`,
`0x0067BB20`, `0x0067BC00`, `0x0067BC70`.

`ParseBool` lowercases its input. `true` and `on` produce true; `false`, `off`
and the empty string produce false. Other input uses `ParseUint` and tests
against zero. Numeric parse errors therefore remain errors, not false.

The additive parser can raise x87 precision from 24 to 53 bits without
restoring it. See foundations section 18.2 for the complete compatibility
boundary; this is process state, not per-parser state.

## 3. nSCRes::cCommandParser

### 3.1 State and method inventory

The parser owns a command vector and a name-to-command-index registry. A later
registration of a name selects the latest command. It also maintains nested
block maps, variable namespaces, a macro registry and at most one macro under
construction. These are independent namespaces, not persistence fields.

| Method | Address | Observable purpose |
|---|---|---|
| QueryInterface | `0x00675F50` | Interface identity; full vtable/IID audit remains SCR2 |
| RegisterCommand | `0x00676DB0` | Register a command name and retain its command entry |
| SubstituteVariables | `0x00677B90` | Resolve variable references in argument tokens |
| RegisterCommands | `0x00678B00` | Install built-in commands |
| ParseCommandArguments | `0x00679120` | Prepare command arguments |
| ParseCommand | `0x00679450` | Dispatch or record a command |
| Macro instantiation | `0x00679650` | Expand a stored macro and dispatch its commands |

Built-ins are registered in this order: `end`, `version`, `list`, `trace`,
`set`, `seti`, `setf`, `setc`, `setv3`, `eval`, `define`, `enddef`, `create`,
`arrayCreate`, `namespace`. `list` reports an exception (`0x00676030`). Detailed
argument contracts for the remaining scalar/set/namespace commands are SCR2.

### 3.2 Dispatch, variables and macros

*[compat]* Ordinary dispatch substitutes variables in every token before
command lookup. Nested block maps are searched newest first, followed by the
global command registry. Unknown commands throw. Completion of a nested
handler checks for unmarked switches; use the switch contract in
effects-parser section 9 rather than silently accepting unused options.

Variable references begin with `$`, with `${name}` also supported. Names use
letters, digits, underscore and colon. Substitution resolves against the
parser's variable state. The exact inserted-parenthesis boundary rule remains
SCR2 and must not be guessed from a conventional shell syntax.

During macro definition commands are recorded rather than executed, except
for the literal `enddef` terminator. `define` rejects nested definitions and
requires at least a name and a tokenized formal-parameter list. `enddef`
without an active definition throws; successful completion publishes the
macro. `create` requires exactly three tokens including its command token:
command, macro name, actual-parameter list. The actual and formal counts must
match. Expansion clones the stored commands, substitutes `%` parameters and
dispatches them in stored order. An unknown parameter throws. There is no
direct window-message or drawing callback in this command execution path.

## 4. nSCRes::cFileParser

### 4.1 Construction, interface and ownership

Construction (`0x00681700`) starts with no parser, input files or watchers and
`modified=false`. `SetParser` (`0x00680930`) shuts down the previous parser
before replacing its retained interface. Shutdown (`0x006814C0`) clears the
watchers. Destructor anchors are `0x006817E0` and `0x00681CC0`.

The main table is `0x00AAF474`; slots are zero based. QueryInterface recognizes
`0x1CF94583`; other identities delegate to the `cRZUnknown` subobject at +8.

| Slots | Methods |
|---|---|
| 0–2 | QueryInterface, AddRef, Release |
| 3–5 | Init (no-op at `0x00648F00`), Shutdown, SetParser |
| 6–8 | AddInputFilePath, ParseFiles(start,count), ParseFiles(all) |
| 9–11 | SetModified, ErrorString, getter whose meaning remains SCR3 |
| 12–14 | NumInputFiles, InputFilePath, Modified |

### 4.2 Input and failure contract

File flags use bit 0 for required files and bit 1 for watching. Registration
marks the parser modified. A watched file uses watcher CLSID `0xE953DA68`, IID
`0xE953DA64`. File ordering, optional-open failures, and pre/post parser hooks
are specified in graphics-rules section 3. The callback table at `0x00AAF464`
is separate from the main interface; exact watcher message IDs remain SCR3.

`DoParseFile` (`0x00680EA0`) sends ordinary lines to the command parser's slot
14. It recognizes two recursive include forms, each requiring exactly one
argument. Their exact spelling and required/optional distinction remain SCR3
until the initializer strings are audited. A false `ReadLine` result ends
the file; this layer does not distinguish a read error from ordinary EOF.
Image drawing and object-graph persistence are outside this interface.

## 5. cSLTokenRecord

### 5.1 State and public interface

The record retains a serial-record interface, a cookie, initialized/text/write
flags, and fields keyed by numeric token ID. Native offsets are +8 (serial
record), +0xC (cookie), +0x10/+0x11/+0x12 (flags), +0x14 (field map).
The table at `0x00AD9A88` has the following zero-based slots:

| Slots | Methods |
|---|---|
| 0–2 | QueryInterface, AddRef, Release |
| 3–6 | InitRead(auto-cookie), InitRead(explicit-cookie), InitWriteText, InitWriteBinary |
| 7–11 | GetKey, GetCookie, IsText, IsTokenAvailable, SetTokenString |
| 12–23 | Get u8, s8, u16, s16, u32, s32, u64, s64, f32, f64, string, resource key |
| 24–35 | Set the same twelve types in the same order |
| 36 | Destructor |

Initialization refuses an already initialized record. Successful attachment
retains the serial record; failed read initialization shuts it down and clears
its fields. `GetCookie` is zero when uninitialized. `GetKey` fails without a
serial record. The QueryInterface IID and constructor reference-count default
remain SCR4 (`0x0097EF84`). There is no message callback or drawing method.

### 5.2 Field and wire contract

*[compat]* Automatic read consumes a uint32 cookie. Cookies have byte spellings
`TKT1` (text) and `TKB1` (binary), constants at `0x00B17A74` and `0x00B17A78`.
The token descriptor is eight bytes: uint32 ID, uint16 type, uint16 count.
Exact descriptor matching and the complete type-number mapping remain SCR4.
Do not substitute the vtable slot index for the wire type number.

A missing field makes a typed getter fail without calling the typed field
reader. Setters find or create the field and delegate the typed values and
count. `SetTokenString` requires an existing field. Fields are emitted in
ascending numeric ID order, with definition before values. Definition, field
creation, and value failures stop reading/writing; `ReadTokens`
(`0x00980AC9`) returns false for those failures.

The terminator `TK_TKR_End` has descriptor `{ID=1,type=3,count=1}` and a uint32
zero payload. A reader stops on ID 1 before consuming that payload. In write
mode shutdown (`0x00980F87`) writes the cookie and fields, then releases the
serial record and destroys the field map. It clears initialized state and has
no return channel for write failure. Consumers needing a durable-write result
must not infer one from the void shutdown interface.

## 6. cSLTokenSerialRecordIO_Text

### 6.1 Interface and state

This adapter is not a COM interface. Its actual table is `0x00AD99E8`;
`0x00AD9978` is the binary base table despite misleading symbol attribution.
Construction at `0x0097EE84` attaches the base record, with record pointer +4,
cursor counter +8 initially 4, and total size +0xC.

| Slots | Methods |
|---|---|
| 0–3 | Destructor, ReadTokenDef, Read resource key, Read string |
| 4–13 | Read f64, f32, s64, u64, s32, u32, s16, u16, s8, u8 |
| 14–16 | WriteTokenDef, Write resource key, Write string |
| 17–26 | Write f64, f32, s64, u64, s32, u32, s16, u16, s8, u8 |

Some integer reads share base implementations. This does not change the
caller's width or signedness contract. No messages or drawing are supported.

### 6.2 Text format and failures

*[compat]* A definition is emitted as `\r\n<name>[ <id>, <type>, <count> ]= `.
Values have trailing comma and space: unsigned integers use `%u, `, signed
integers `%d, `, 64-bit integers `%I64u, ` / `%I64d, `, f32 `%f, ` with the
argument promoted to double, f64 `%g, `, and strings `%s, `. Strings are raw,
unquoted text, so commas are not protected by a quoting layer. Output uses the
serial record's `SetFieldString`; see persistence section 3 for that IO
contract (`0x0097F54D`, `0x0097F5B7`, `0x0097F5E4`–`0x0097F93A`).

The delimiter reader (`0x00980A41`) clears its text destination, discards
semicolon comments through CR, drops LF, converts CR and tab to spaces, and
trims the collected text at the delimiter. Exhaustion returns false.
`ReadTokenDef` (`0x00980CCE`) ignores preceding text through `[`, reads comma
delimited ID/type and `]` delimited count, then ignores through `=`. It
publishes the descriptor only when all these reads succeed. Type and count
are narrowed to 16 bits.

Numeric reads accept the numeric prefix recognized by the runtime scanner;
unconsumed suffix text is not itself a conversion failure. A failed numeric
conversion zeroes the output; delimiter exhaustion is a distinct failure and
does not establish that same destination rule. Resource keys are three
successive uint32 values, assigned to the caller only after all three succeed.
The character helper's end-boundary counter excludes the final byte in the
inspected implementation (`0x0097EF35`); exact last-byte vectors remain SCR4.

## 7. cGZPersistBufferResource

### 7.1 Construction, identity and buffer interface

Construction (`0x009780DC`) accepts target color type and create flags. The
base persistent-resource contract is persistence section 4. The retained
buffer at +0x1C and aggregator at +0x20 start null; the scratch vector at +0x2C
starts empty. Construction tries CLSID `0xC470D326` / IID `0x20732180`, then
CLSID `0xC470D325` as fallback. The first provider can supply aggregator IID
`0x12F`, whose outer interface is set to this resource's +0x18 interface.

QueryInterface recognizes IID `0x6F1568E` at +0x18, otherwise tries the base
resource and the aggregator or buffer. Destruction detaches/releases the
aggregator, releases the buffer, and destroys scratch storage. The primary
table at `0x00AD8F28` inherits persistent-resource entries and has destructor
slot 12 (`0x009781D3`). The embedded buffer-resource table at `0x00AD8F10` is:

| Slot | Contract |
|---|---|
| 0–2 | QueryInterface and outer AddRef/Release thunks |
| 3 | GetBuffer (`0x009779C9`), returns an AddRef'd buffer |
| 4 | QueryInterface forwarded to buffer (`0x009BC0BD`) |
| 5 | SetBuffer (`0x009779DF`), retains new buffer before releasing old |

Drawing/locking/pixel access are delegated to `cIGZBuffer`. There is no
resource-level drawing command or message handler in the audited interface.

### 7.2 BMP and TGA input

*[compat]* Record loading (`0x00979171`) first tries BMP at the current cursor.
If that fails it seeks to absolute zero and tries TGA; a failed seek aborts.
The file BMP loader (`0x0097878C`) seeks to `bfOffBits`. Record-side offset
handling remains SCR5. Failed loading must not be treated as a decoded image;
post-failure buffer contents are not established.

BMP accepts a 14-byte `BM` header and 40-byte information header. Header
validation permits compression values 0 and 1 and depths 4, 8 and 24
(`0x00977372`, checked against instructions). Supported source-to-destination
translations (`0x0097868A`) are 4→16, 8→8/16/32, and 24→16/24/32; other pairs
return false. Record stride uses four-byte alignment of width times
`(bitsPerPixel >> 3)`, including the 4-bpp quirk. The file loader instead uses
ceil(width/2) before alignment for 4-bpp. RLE8 handles runs, absolute spans
with odd-byte padding, EOL, EOB and delta escapes. Malformed compressed data
does not have an established safe bounds contract.

TGA loading (`0x00978A69`) seeks to zero, reads an 18-byte header, skips image
ID and color-map bytes and accepts image types 2 and 10. Both raw and packet
RLE paths support 16/24/32-bit conversion. Descriptor bit `0x20` controls
vertical flipping; the horizontal-origin bit is ignored. Source masks are
`0x7C00/0x03E0/0x001F` for 16-bit and `0xFF0000/0x00FF00/0x0000FF` for
24/32-bit. A declared 16-bit alpha channel uses `0x8000`; 32-bit alpha uses
the declared number of most-significant alpha bits. Unsupported pixel depth
can still report success after buffer creation and locking, so success alone
does not validate that unsupported input.

Loaders create then lock/unlock the destination buffer (file flags `0x8040`,
record flags `0x8080`). The specialized 4444 dithering path and exact numeric
diffusion rule require instruction verification; see SCR5. Do not silently
replace it with nondithered truncation for comparison tests.

## 8. cGZPersistASCIIWriter

### 8.1 Interface and graph state

The table at `0x00ADA288` is zero based. QueryInterface (`0x00981A8F`)
recognizes IIDs 1 and `0x1B0AAB0E`. The writer maintains a stack of active
surveyors; OutputStream forwards to the top surveyor's stream (+0x70).
Constructor defaults and empty-stack behavior remain SCR6.

| Slots | Methods |
|---|---|
| 0–3 | QueryInterface, AddRef, Release, WriteRoot |
| 4–12 | Scalar bool, s8, u8, s16, u16, s32, u32, f32, f64 |
| 13–21 | Arrays of the same nine types |
| 22–24 | RZChar string, GZString, Object |
| 25–29 | Unknown-reference adaptor, Referent-reference, TypeInfo, OutputStream, destructor |

### 8.2 Output and ordering

*[compat]* Scalar lines have a six-space prefix followed by
`<type> <name> = <value>\n`. Booleans use `true` / `false`. Strings are written
raw. Floating output uses the stream's typed conversion; no fixed decimal
precision is established. Hex words use lowercase `0x%08x` (`0x009828A5`).
Resource keys are labeled in order `G:<word1> I:<word2> T:<word0>`
(`0x009828EA`). Array punctuation and complete header literals remain SCR6.

`WriteRoot` (`0x009835ED`) obtains a surveyor using CLSID `0xBB194C78` / IID
`0x9B194C66`. The surveyed graph determines dependency and object indices.
Output order is resource name/key, indexed dependency list, indexed object
directory, blank line, then object bodies in surveyor order. Completion
flushes the stream, finalizes dependencies through surveyor slot +0x28, then
pops/releases the active surveyor. There is no message handling or drawing.

Object output (`0x0098298A`) includes GZCLSID and braces and calls the object's
serialization method at +0x14. Null objects have class zero. Reference output
(`0x00982F2B`) distinguishes null, local object index/type/class, and external
object ID/type/dependency-resource index/key. Slot 25 first queries IID
`0xBB11A53E` to obtain the referent interface. No checked stream-failure path
was observed; do not infer transactional output or rollback on a partial
write. Exact failures before a surveyor is installed remain SCR6.

## 9. Conformance vectors

These are independent input/output requirements for a future core suite;
this documentation lane does not assert an existing executable test suite.

| ID | Input / setup | Required result |
|---|---|---|
| SCR-V01 | Split `cmd 'a b' [c d]` | Three tokens: `cmd`, `a b`, `c d` |
| SCR-V02 | Unterminated quoted/grouped argument | Quote error 5 / group error 6 |
| SCR-V03 | ParseFloat `2+3*4`; `2+3junk`; empty input | 14; error; error |
| SCR-V04 | ParseBool `ON`, `false`, empty, `2` | true, false, false, true |
| SCR-V05 | Two registrations for one command name | Later registration receives subsequent dispatch |
| SCR-V06 | Unknown command; enddef without define; nested define | Each reports an exception |
| SCR-V07 | create with mismatched formal/actual count | Error before expanding its command list |
| SCR-V08 | New file parser, then register a path | Modified false, then true |
| SCR-V09 | Init an already initialized token record | Refused; no second attachment |
| SCR-V10 | Missing token getter; SetTokenString on missing token | Both fail |
| SCR-V11 | Write fields IDs 9 then 2, then shutdown | Cookie, field 2, field 9, end descriptor and zero value |
| SCR-V12 | Read terminator ID 1 | Stops before consuming its zero payload |
| SCR-V13 | Text definition `[ 23, 3, 1 ]= ` and uint32 value `42, ` | Descriptor 23/3/1 and value 42 |
| SCR-V14 | Numeric text `12suffix, ` versus `suffix, ` | Prefix conversion 12; failed conversion zeroes output |
| SCR-V15 | Text resource key with only two delimited values | Failure; caller's key remains unchanged |
| SCR-V16 | SetBuffer to its existing buffer | Retain happens before release; no premature destruction |
| SCR-V17 | Record BMP failure followed by failed seek(0) | Failure without TGA attempt |
| SCR-V18 | BMP translation 24→8 versus 8→32 | First unsupported; second supported |
| SCR-V19 | Same TGA pixels with opposite descriptor bit 0x20 | Opposite vertical row order |
| SCR-V20 | ASCII hex word 0xAB and key words [1,2,3] | `0x000000ab`; key labels G=2, I=3, T=1 |
| SCR-V21 | ASCII graph with dependencies and two surveyed objects | Dependencies and directory precede both bodies; body order is surveyor order |

Additional vectors for switch/positional readers and required/optional file
ordering are normative in the linked specs. Inputs whose exact result is an
open question must not be promoted into guessed conformance expectations.
