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
| ParseEnum | `0x0067AAF0` | Search the primary then optional secondary name/value table |
| SplitIntoArguments | `0x0067B3B0` | Produce command and argument tokens |
| ParseFloat / ParseInt / ParseUint / ParseBool | `0x0067BBA0` / `0x0067BC00` / `0x0067BC70` / `0x0067BCD0` | Convert a complete expression or boolean spelling |
| ParseColor / ParseColorAlpha | `0x0067BDE0` / `0x0067BE90` | Three RGB expressions; RGB with optional alpha |
| ParseVector2 / ParseVector3 | `0x0067BF90` / `0x0067C060` | Up to two/three expressions, missing coordinates zero |

*[compat]* Whitespace comprises space, tab, LF and CR. `#` starts a comment.
Both single and double quotes group a token. Brackets, parentheses and braces
pair with `]`, `)` and `}` respectively. Outer quote/group delimiters are
removed from the resulting argument. An unterminated quote reports error 5;
an unterminated group reports error 6. These delimiter sets are established by
initializers `0x00A74350`–`0x00A74410`.

*[compat]* The first `#` limits the entire input, including quoted/grouped
text. There is no backslash escape. A quote closes at the next identical quote;
the other quote character is ordinary content. Adjacent unquoted and quoted
pieces concatenate into one argument (`a"b c"d` gives `ab cd`). A group is
special when it begins an argument: it becomes one argument and its closer
ends that argument even without following whitespace. Only occurrences of its
own opener and closer affect nesting; quotes and other kinds of brackets
inside it are literal. Thus `([)]` gives arguments `[` and `]`, whereas
`(a(b)c)` gives `a(b)c`. Quotes inside a group do not protect a closing bracket.
These are the literal CountArguments/SplitIntoArguments rules, not a balanced
mixed-bracket grammar (`0x0067A320`, `0x0067B3B0`).

### 2.2 Numeric conversion

*[compat]* Numeric conversion requires a complete expression: empty input,
invalid terms and unconsumed trailing text are errors. Product-level operators
`*`, `/`, `%` bind more tightly than `+` and `-`; each tier associates left.
The repeated arithmetic accumulators are stored in binary64 after each
iteration. `%` is **a − floor(a / b) × b**, not integer remainder or fmod:
its quotient is stored as binary64 before floor, and both operands are
binary64 for the multiply/subtract (`0x0067BA0F`–`0x0067BA2F`; floor dispatch
`0x009EFF60`). For example, `-5%3=1` and `5%-3=-1`.

The next tighter tier is right-associative power `^`. Its operands are signed
factors, so unary signs bind *more tightly* than power: `-2^2=4`, `2^3^2=512`,
`2^-2=0.25`. Any sequence of unary `+` and `-`, with whitespace between them,
is accepted. A factor is a number, a parenthesized full expression, `pi`, `e`,
or a named function. Identifiers begin with a letter, continue with letters,
digits or underscore and compare case-sensitively. Unknown identifiers throw;
there is no implicit variable lookup or multiplication in this evaluator
(`0x0067AC30`, `0x0067AC60`, `0x0067B360`, `0x0067B920`).

| Name | Arguments / result |
|---|---|
| `pi`, `e` | Constants, without parentheses; pi is binary32 `0x40490FDB` promoted to double; e is exp(1) |
| `sqrt`, `exp`, `log`, `abs`, `floor`, `ceil`, `sqr` | One parenthesized expression; log is natural logarithm; sqr multiplies its binary64 argument by itself |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan` | One parenthesized expression; radians |
| `sind`, `cosd`, `tand` | One expression in degrees, multiplied by binary32 `0x3C8EFA35` promoted to the evaluation precision |
| `dasin`, `dacos`, `datan` | One expression; inverse-radian result multiplied by binary32 `0x42652EE1` |
| `pow`, `atan2`, `datan2` | Two full expressions separated by comma inside parentheses; power(a,b), atan2(a,b), and degree atan2(a,b) respectively |

Function arguments may contain whitespace and nested arithmetic. No optional
arguments, aliases such as `dsin`, or extra commas are accepted. Ordinary
function results are stored as binary64 before returning; `sqr` reloads its
binary64 operand for the multiplication, and the three forward degree trig
functions return their x87 result directly after loading a binary64 argument.
Constants and widths are checked at `0x0067AD39`–`0x0067AD3F`,
`0x0067AD7D`–`0x0067AD95`, `0x0067B000`–`0x0067B0A8`,
`0x0067B0B3`–`0x0067B2A8`; data `0x00AAEBFC`, `0x00AAEC00`, `0x00B0D09C`.

Decimal operands must start with a digit or `.`. Numeric value conversion is
English-locale atof, but cursor advancement is a separate permissive scan:
consume all digits and dots, then at most one `e`/`E`, its optional sign and
all following digits. It does not require exponent digits or only one dot.
Consequently `.` yields zero, `1.2.3` yields 1.2 and `1e+` yields 1; `1e+x`
fails for trailing `x`. Leading-zero decimal is decimal. Only a literal
lowercase `0x` selects unsigned 32-bit base-autodetect strtoul conversion;
`0X10` does not select that branch and fails for trailing text. The hex result
is converted as unsigned (high-bit values gain 4294967296 after signed load).
No finite/domain check is added: arithmetic overflow, division by zero and
invalid math can return infinities/NaNs under the normal masked floating-point
environment. Bare `inf`/`nan` are unknown function names, not decimal literals
(`0x00679D00`, `0x00679DA0`, `0x0067BB20`).

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

### 2.3 Enums, colors and vectors

*[compat]* Enum names compare byte-for-byte with equal length, without case
folding. Entries are name/value pairs ending in a null name. Search the first
table in order and, if unmatched, the optional second table in order. Return
the first matching entry's value; otherwise throw parse error 4, `Unknown enum`.
This is not a numeric-expression fallback (`0x0067AAF0`–`0x0067ABC9`).

Colors and vectors split their component text into arguments and apply
`ParseFloat` to each present argument. Color requires exactly three components.
ColorAlpha requires three or four; omitted alpha is binary32 1.0. Components
are returned as binary32 values in input order, with no scaling to bytes,
packing or clamping. Instruction anchors for storage and default alpha are
`0x0067BE44`–`0x0067BE78` and `0x0067BEFE`–`0x0067BF38` (the latter loads
`0x3F800000` from `0x00A81228`).

Vector2 accepts zero through two components; Vector3 zero through three.
Too many components throw. Missing coordinates use the zero-initialized
binary32 scalar at `0x00B479B0`; supplied coordinates are individually stored
as binary32. There is no replication of the first component into missing
coordinates. Instruction anchors: `0x0067BFD3`–`0x0067C032`,
`0x0067C0A3`–`0x0067C11E`.

## 3. nSCRes::cCommandParser

### 3.1 State and method inventory

The parser owns a command vector and a name-to-command-index registry. A later
registration of a name selects the latest command. It also maintains nested
block maps, variable namespaces, a macro registry and at most one macro under
construction. These are independent namespaces, not persistence fields.

| Method | Address | Observable purpose |
|---|---|---|
| QueryInterface | `0x00675F50` | IIDs 1 and `0x095035F4`; null output pointer returns false |
| RegisterCommand | `0x00676DB0` | Register a command name and retain its command entry |
| SubstituteVariables | `0x00677B90` | Resolve variable references in argument tokens |
| RegisterCommands | `0x00678B00` | Install built-in commands |
| ParseCommandArguments | `0x00679120` | Prepare command arguments |
| ParseCommand | `0x00679450` | Dispatch or record a command |
| Macro instantiation | `0x00679650` | Expand a stored macro and dispatch its commands |

The native layout has a separate unknown-interface table at `0x00AAEB00`
(QI/AddRef/Release thunks `0x00678A00/10/20`), a primary table at `0x00AAEB0C`
(destructor, RegisterCommands), and the parser method table at `0x00AAEACC`.
The parser table has these zero-based slots; a byte offset such as +0x14 is
slot 5, not slot 14. Unknown semantic names remain SCR2.

| Slot | Native entry / settled operation |
|---|---|
| 0 | `0x00678A30`: Init forwards to the owner's RegisterCommands hook |
| 1 | `0x00679620`: Shutdown clears parser-owned command entries |
| 2 | `0x00679630`: reset parse state (`0x006793B0`) |
| 3 | `0x00678A40`: no-op (`0x00407990`, a single return despite its symbol name) |
| 4 | `0x00678A50`: clear active macro pointer |
| 5 | `0x00679640`: ParseCommand |
| 6 | `0x00679C80`: variable assignment helper (`0x00679BD0`) |
| 7 | `0x00678A90`: helper `0x00677210`, name unsettled |
| 8 | `0x00678A80`: pop variable namespace suffix |
| 9 | `0x00678FE0`: helper `0x00678AB0`, name unsettled |
| 10 | `0x00678A60`: return owning parser pointer |
| 11 | `0x00678A70`: no-op (`0x00648F00`) |
| 12 | `0x00678AA0`: helper `0x006772C0`, name unsettled |

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
| 9–11 | SetModified, ErrorString, GetParser (raw pointer at +0x10) |
| 12–14 | NumInputFiles, InputFilePath, Modified |

### 4.2 Input and failure contract

File flags use bit 0 for required files and bit 1 for watching. Registration
marks the parser modified. A watched file uses watcher CLSID `0xE953DA68`, IID
`0xE953DA64`. File ordering, optional-open failures, and pre/post parser hooks
are specified in graphics-rules section 3. The callback table at `0x00AAF464`
has QI/AddRef/Release at slots 0–2 and message delivery at slot 3
(`0x006808D0`). Delivery reads message type through message slot 4. Type
`0x6AE76406` sets modified=true and returns true; other types return false
without changing modified. This flag is +0x30 relative to the callback
subobject, not the main-interface pointer.

`DoParseFile` (`0x00680EA0`) sends ordinary lines to the command parser's slot
5 (byte offset +0x14). It recognizes `include` and `sinclude`, each requiring
exactly one argument (initializers `0x00A74900`, `0x00A74940`). Both recursively
parse the named file. `sinclude` suppresses an included-file open error
(parse-error code 2); `include` propagates it. Other parse errors are not
silenced by `sinclude` (`0x0068110F`–`0x00681170`). Relative path handling and
recovery after a nested error remain SCR3. A false `ReadLine` result ends
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
serial record. QueryInterface accepts IIDs 1 and `0xA2ACE662`, retains the
returned interface, and leaves the output untouched on an unrecognized IID
(`0x0097EF84`). Construction (`0x0097F410`) starts reference count, record,
cookie and all three flags at zero and creates an empty map. There is no
message callback or drawing method.

### 5.2 Field and wire contract

*[compat]* Automatic read consumes a uint32 cookie. Cookies have byte spellings
`TKT1` (text) and `TKB1` (binary), constants at `0x00B17A74` and `0x00B17A78`.
The token descriptor is eight bytes: uint32 ID, uint16 type, uint16 count.
Descriptors match only if ID, type and count all match, with the latter two
compared as uint16 (`0x0097EF5B`–`0x0097EF83`, instruction verified).
The complete type-number mapping remains SCR4.
Do not substitute the vtable slot index for the wire type number.

A missing field makes a typed getter fail without calling the typed field
reader. Setters find or create the field and delegate the typed values and
count. `SetTokenString` requires an existing field. Fields are emitted in
ascending numeric ID order, with definition before values. Definition, field
creation, and value failures stop reading/writing; `ReadTokens`
(`0x00980AC9`) returns false for those failures.

`CreateField` rejects count zero and unsupported type codes outside 1–12.
Wire type 3 is uint32. An existing matching descriptor is not a new creation;
a different descriptor at the same ID causes the old map entry to be erased
before replacement (`0x009805F8`). This is a descriptor operation, not a
coercion between the requested typed getter and the stored type.

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
The character helper increments its counter before testing counter < total
size, and only then calls the serial byte reader (`0x0097EF35`–`0x0097EF58`,
instruction verified). With initial counter 4 and total N, at most N−5 bytes
can be read through this helper. Thus the last physical byte of a record
beginning with the four-byte cookie is excluded. An attempted read at the
boundary fails without a serial read; the counter still advances. A delimiter
in that excluded byte is not consumable. No message state or lookahead cache
alters this rule.

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
Construction (`0x00983314`) starts reference count zero and initializes the
stack container. Field writes and OutputStream require an active WriteRoot
context; the native stream getter directly uses stack.back() without testing
for an empty stack (`0x00982F1C`).

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
precision is established by this owner. Numeric type spellings are `bool`,
`Sint8`, `Uint8`, `Sint16`, `Uint16`, `Sint32`, `Uint32`, `Float32`, `Float64`;
string overloads use `RZChar*` and `RZString`. Hex words use lowercase
`0x%08x` (`0x009828A5`).
Resource keys are labeled in order `G:<word1> I:<word2> T:<word0>`
(`0x009828EA`); each key component uses that hex-word format.

Array lines have six leading spaces, then `<type> <name>[<count>] = `,
then each element followed by `, `, then LF. The comma-space remains after
the final element. A zero-count array writes its header and LF without an
element or comma. Element conversion is the corresponding scalar stream
conversion; bool elements are `true` / `false`. This punctuation is shared
across the nine array methods (`0x009822DA`–`0x00982818`). It differs from the
token adapter's comma-delimited wire format in section 6.

`WriteRoot` (`0x009835ED`) obtains a surveyor using CLSID `0xBB194C78` / IID
`0x9B194C66`. The surveyed graph determines dependency and object indices.
Output order is resource name/key, indexed dependency list, indexed object
directory, blank line, then object bodies in surveyor order. Completion
flushes the stream, calls each dependency resource's method at +0x28 in
dependency order, then
pops/releases the active surveyor. There is no message handling or drawing.

The following table gives line spelling; `<key>` is the formatted key above,
`<type>` is the serializable type name and indices/IDs use uint32 stream
conversion. All line endings written by this owner are LF.

| Output item | Text before LF |
|---|---|
| Root header | `Resource <name> <key>` |
| Dependency header | `   Dep. Resource #<index> <name> <key>` |
| Object directory and body header | `   Object #<index> <type>` |
| Body open / close | `   {` / `   }` (close followed by another LF) |
| Null referent field | `      cIGZPersistSerializableReferent* <name> = 0` |
| Local referent field | `      cIGZPersistSerializableReferent* <name> = Object #<index> <type> GZCLSID: <hexclass>` |
| External referent field | `      cIGZPersistSerializableReferent* <name> = Object ID <id> <type> in dep. resource #<index> <key>` |
| Type info | `      TypeName: <type> GZCLSID: <hexclass> Version: <version>` |

The object directory is followed by one blank line. Each object body is
preceded by its repeated directory-style header. After all bodies another
LF is written before flush. The same-resource vs external-reference decision
uses identity of the referent's resource and the surveyor's root resource,
not equality of their display names or keys (`0x00982F2B`).

Inline Object output (`0x0098298A`) ignores its field-name argument and starts
`      Object GZCLSID: <hexclass>`. A nonnull object then writes
`\n      {\n`, calls the object's serialization method at +0x14, and writes
`      }\n` followed by another LF. A null object writes class zero and LF
without braces. Reference output
(`0x00982F2B`) distinguishes null, local object index/type/class, and external
object ID/type/dependency-resource index/key. Slot 25 first queries IID
`0xBB11A53E` to obtain the referent interface. No checked stream-failure path
was observed; do not infer transactional output or rollback on a partial
write. WriteRoot does not test factory success or the returned surveyor for
null before invoking it (`0x00983613`–`0x00983636`). Thus factory failure and
empty-stack field writes have no defined successful output/result contract.
*[choice]* A portable implementation may reject those invalid contexts
without reproducing a null dereference. It must preserve the successful
output and ordering contracts above.

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
| SCR-V22 | ParseColorAlpha `2 -1 0.5` | Binary32 components (2, −1, 0.5, 1); no clamping |
| SCR-V23 | ParseVector3 `7`; ParseVector2 empty; three components to Vector2 | (7,0,0); (0,0); error |
| SCR-V24 | Same enum name in primary and secondary tables; unknown name | Primary value wins; error 4 |
| SCR-V25 | File callback with type 0x6AE76406; then unrelated type | true and modified set; false with flag preserved |
| SCR-V26 | Text helper counter 4, total 6 | One successful byte read possible; second attempt fails without reading byte 5 |
| SCR-V27 | GetField descriptor matches ID and type but different count | Failure; no implicit resizing/coercion |
| SCR-V28 | ASCII bool array name flags, values true/false | `      bool flags[2] = true, false, ` followed by LF |
| SCR-V29 | ASCII zero-count bool array name flags | `      bool flags[0] = ` followed by LF |
| SCR-V30 | ASCII inline null object with any field name | `      Object GZCLSID: 0x00000000` and LF; no braces/name |

Additional vectors for switch/positional readers and required/optional file
ordering are normative in the linked specs. Inputs whose exact result is an
open question must not be promoted into guessed conformance expectations.
