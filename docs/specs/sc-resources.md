# SC resource scripts and text persistence

Behaviour specification for SimCity 4 Deluxe 1.1.641. Evidence: exported function
bodies and instruction checks, recorded 2026-10-23 by `re-persist-sl5` and
`re-scres-scr5`.
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

Decimal operands must start with a digit or `.`. Numeric value conversion uses
the game's English decimal helper, but cursor advancement is a separate permissive scan:
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

The English helper is **not** a correctly rounded host `strtod`. It builds the
integer part by multiply-by-ten/add-digit, storing binary64 after each digit.
Fractional digits use successive multiplication of an x87 factor by binary64
0.1, add digit times factor, and store binary64 after each addition. Exponent
digits accumulate modulo 2^32. The power-of-ten helper uses integer repeated
squaring with every product modulo 2^32; interpret the final word as signed
before conversion to floating point. A negative signed exponent passed to
that helper takes integer reciprocal of its positive-magnitude power. An
explicit minus sign after `e` then takes a *floating* reciprocal of the helper
result. Multiply the mantissa by that result and store binary64. Thus `1e10`
is 1410065408, `1e12` is -727379968, `1e32` is zero and `1e-32` is positive
infinity, under masked exceptions. These are compatibility quirks, not range
errors (`0x00910426`–`0x00910529`, `0x009103DA`–`0x00910425`). Integer division
by zero in the helper for a wrapped negative exponent has no successful
numeric-result contract; a portable reader may reject that input.

The lowercase-hex branch saturates unsigned overflow at 0xFFFFFFFF rather
than wrapping (`0x009F01CC`, especially `0x009F0319`–`0x009F0364`). `ParseInt`
retains its bit-conversion rule even outside the small-integer range: float
infinity becomes 0x34400000 after the modular subtraction, negative infinity
0xB4400000; NaN results depend on the stored NaN bits. Under the usual masked
x87 environment, `ParseUint` truncates through signed 64-bit `_ftol2`, retaining
only the low word. Values outside that signed range, infinity and NaN produce
the integer-indefinite low word zero, not unsigned saturation
(`0x0067BC26`–`0x0067BC61`, `0x0067BC8A`, `0x009EEF04`–`0x009EEF78`).

`ParseInt` is not an ordinary truncate-to-integer conversion. It adds binary32
12582912, stores the result as binary32, and subtracts `0x4B400000` from that
stored value's bit representation. Preserve that rule outside the normal
small-integer range too; do not replace it with saturation or a checked cast.
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
slot 5, not slot 14. Names below describe behavior, including unnamed exports.

| Slot | Native entry / settled operation |
|---|---|
| 0 | `0x00678A30`: Init forwards to the owner's RegisterCommands hook |
| 1 | `0x00679620`: Shutdown clears parser-owned command entries |
| 2 | `0x00679630`: reset parse state (`0x006793B0`) |
| 3 | `0x00678A40`: no-op (`0x00407990`, a single return despite its symbol name) |
| 4 | `0x00678A50`: clear active macro pointer |
| 5 | `0x00679640`: ParseCommand |
| 6 | `0x00679C80`: variable assignment helper (`0x00679BD0`) |
| 7 | `0x00678A90`: PushNamespace (`0x00677210`) |
| 8 | `0x00678A80`: pop variable namespace suffix |
| 9 | `0x00678FE0`: RemoveVariable by exact stored key (`0x00678AB0`) |
| 10 | `0x00678A60`: return owning parser pointer |
| 11 | `0x00678A70`: no-op (`0x00648F00`) |
| 12 | `0x00678AA0`: GetVariable (`0x006772C0`), null when absent |

Built-ins are registered in this order: `end`, `version`, `list`, `trace`,
`set`, `seti`, `setf`, `setc`, `setv3`, `eval`, `define`, `enddef`, `create`,
`arrayCreate`, `namespace`. Construction starts version=1, with empty command,
variable and macro registries, empty namespace prefix/stacks and no active
macro (`0x006788B9`–`0x006789F2`). Init invokes RegisterCommands; construction
alone does not install them. Reset sets version=0, empties the namespace
prefix/stack and destroys published macros; it preserves variables and
registered commands (`0x006793B0`, `0x00678FF0`). Slot 4 clears the active
macro pointer; do not conflate it with variable removal or reset.

### 3.2 Dispatch, variables and macros

*[compat]* Ordinary dispatch substitutes variables in every token before
command lookup. Nested block maps are searched newest first, followed by the
global command registry. Unknown commands throw. Completion of a nested
handler checks for unmarked switches; use the switch contract in
effects-parser section 9 rather than silently accepting unused options.

Variable references begin with `$`, with `${name}` also supported, anywhere
inside an argument, including the command token. Names greedily consume
letters, digits, underscore and colon, with no first-character restriction.
Braces merely stop name consumption: a following `}` is skipped if present;
its absence does not itself throw. Unknown names throw. Replacement text is
inserted once, without rescanning it for further references.

Assignment always stores current-prefix plus name. PushNamespace saves the
old prefix length and appends name plus colon. Pop truncates to that saved
length (empty-stack pop has no valid native contract). GetVariable first
tries the entire current prefix plus supplied name, then parent prefixes
newest first, ending with the unqualified name. A colon in a supplied name
does not bypass prefix search. RemoveVariable instead erases the exact
stored key supplied, without prefixing it; it is not ClearVariables
(`0x00677210`, `0x00676380`, `0x006772C0`, `0x00679BD0`, `0x00678AB0`).

Parentheses insertion is unusual: for an **unbraced** reference, wrap the
replacement only when its left edge is the start or follows whitespace, and
its right edge is the end or precedes whitespace, **except** a reference
occupying the entire argument is left unwrapped. The whitespace set is space,
tab, LF, CR. Embedded arithmetic punctuation is not a wrapping boundary.
Braced references remain unwrapped. With x=`1+2`, `$x` becomes `1+2`, `$x*3`
becomes `1+2*3`, and `a $x b` becomes `a (1+2) b`. Macro `%name`/`%{name}`
substitution uses the same name and boundary rules, matching formal names
against the actual-argument list; it is not whole-token substitution
(`0x00677B90`, `0x00678000`; boundary-set initializer `0x00A74200` uses
`0x00A9F358`–`0x00A9F35C`).

During macro definition commands are recorded rather than executed, except
for the literal `enddef` terminator. `define` rejects nested definitions and
requires at least a name and a tokenized formal-parameter list. `enddef`
without an active definition throws; successful completion publishes the
macro. `create` requires exactly three tokens including its command token:
command, macro name, actual-parameter list. The actual and formal counts must
match. Expansion clones the stored commands, substitutes `%` parameters and
dispatches them in stored order. An unknown parameter throws. There is no
direct window-message or drawing callback in this command execution path.

### 3.3 Built-in argument and formatting contracts

Counts below include the command token. Failed counts throw a parse error;
conversion failures propagate. Set commands require at least three tokens,
use token 2 as name and token 3 as value, and do not concatenate extra
positionals (`0x00677510`). All assignments obey section 3.2 namespaces.

| Command | Behavior |
|---|---|
| `end` | Requires a nonempty block stack, calls its top handler's EndBlock hook and pops it; does not enforce an argument count (`0x00676110`) |
| `version` | Exactly two tokens; decimal `atol` prefix conversion of the second, stored as the version. Not expression parsing (`0x00676170`) |
| `list` | Always throws `unimplemented` (`0x00676030`) |
| `trace` | Exactly two tokens; otherwise throws. Successful body has no output or state effect (`0x006771C0`) |
| `set` | Stores the third token literally after ordinary substitution |
| `seti` | ParseInt, then signed decimal formatting |
| `setf` | ParseFloat, then DToStrEnglish |
| `setv3` | ParseVector3; apply optional `-scale` scalar, `-mult` vector, `-add` vector in that order, independent of switch order; emit three space-separated numbers |
| `setc` | ParseColorAlpha, then the component quirks below; accepts `-scale`, `-mult`, `-add`, `-alpha` in that fixed order |
| `eval` | Exactly two tokens; split the second into arguments and dispatch those arguments through this parser, including substitution (`0x00679A20`) |
| `namespace` | Exactly two tokens; enter this block and push its name; EndBlock pops the namespace (`0x00677B10`, `0x00677AF0`) |
| `arrayCreate` | Exactly three tokens: macro name and ParseInt count. Execute `create name [index count]` for indices 0 through count−1. Zero executes none; count is compared unsigned, so a negative parsed count is not treated as zero (`0x006798D0`) |

Vector/color intermediates are stored as binary32 after each component
operation. DToStrEnglish formats promoted binary32 values with `%g` (default
six significant digits) and changes the first decimal comma to a dot
(`0x00679E00`). No clamping is added.

For `setc`, call the four parsed storage components R,G,B,A. `-scale` scales
**G,B,A**, leaving R unchanged. `-mult` and `-add` parse ColorAlpha and act
componentwise on R,G,B,A. Thus a three-component multiplier has A=1 and a
three-component addend adds 1 to A. `-alpha` then overwrites **G**. Emit G,B,A
in that order and append R only if R<1 (unordered/NaN does not pass). This
rotation is literal behavior, not ordinary RGBA semantics. Evidence:
destination base `0x00677644`, scale `0x0067767B`–`0x00677698`, multiply/add
`0x006776D0`–`0x00677764`, override `0x00677795`, output `0x006777CC` and
optional fourth `0x00677801`–`0x00677839`. The separate ParseColorAlpha service
still returns components in input order as specified in section 2.3.

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
5 (byte offset +0x14), after the physical block-comment preprocessing in
section 12. Suppressed block lines reach neither include routing nor that
slot; ordinary empty and line-comment-only lines do reach it. It recognizes
`include` and `sinclude`, each requiring
exactly one argument (initializers `0x00A74900`, `0x00A74940`). Both recursively
parse the named file. `sinclude` suppresses an included-file open error
(parse-error code 2); `include` propagates it. Other parse errors are not
silenced by `sinclude` (`0x0068110F`–`0x00681170`). Relative includes are joined
to the parent directory of the file containing the directive, recursively at
each nesting level. A leading slash/backslash or a colon in character position
2 classifies a path as absolute; it is used unchanged. This is lexical joining,
without dot-segment canonicalization, existence probing or a search-path
fallback (`0x00680B20`, `0x006810DD`).

An included open failure (code 2) is suppressed by `sinclude`; `include`
promotes it to code 3. Other nested errors propagate for both directives.
Unwinding closes each opened file and appends its filename and line context
to the error; parsing does not resume at the next line of the enclosing file
(`0x0068110F`, `0x00681273`). At the top-level registered-file loop, processing
continues with the next registered file. An optional file's open failure is
ignored; other optional errors are recorded without making the aggregate
result false. Required-file errors make that result false. The post hook runs
after the loop regardless; the success hook runs only when the aggregate
result is true (`0x006813A2`–`0x006813ED`). A false `ReadLine` result ends
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
Do not substitute the vtable slot index for the wire type number. The wire
numbers select the following payloads (CreateField `0x009805F8`, constructors
`0x0097F9DF`–`0x00980293`, field Read adapter dispatches):

| Wire type | Payload | Bytes per binary element |
|---|---|---|
| 1 | uint8 | 1 |
| 2 | uint16 | 2 |
| 3 | uint32 | 4 |
| 4 | sint32 | 4 |
| 5 | float32 | 4 |
| 6 | sint64 | 8 |
| 7 | string | uint32 byte length followed by bytes |
| 8 | resource key | 12 |
| 9 | sint8 | 1 |
| 10 | sint16 | 2 |
| 11 | uint64 | 8 |
| 12 | float64 | 8 |

In particular type 6 is signed integer, not double. Field readers for types
5/6/11/12 call adapter byte offsets `0x14/0x18/0x1C/0x10` respectively
(`0x0097FE36`, `0x0097FD60`, `0x0098022B`, `0x009802BF`).

A missing field makes a typed getter fail without calling the typed field
reader. Setters find or create the field and delegate a reference to one typed
value and its zero-based element index (not an array length). `SetTokenString`
requires an existing field. Fields are emitted in
ascending numeric ID order, with definition before values. Definition, field
creation, and value failures stop reading/writing; `ReadTokens`
(`0x00980AC9`) returns false for those failures.

`CreateField` rejects count zero and unsupported type codes outside 1–12.
Wire type 3 is uint32. An existing matching descriptor is not a new creation;
a different descriptor at the same ID causes the old map entry to be erased
before replacement (`0x009805F8`). This is a descriptor operation, not a
coercion between the requested typed getter and the stored type.

`SetTokenString` changes the existing field's definition name, stored at field
+0xC; it does not set a string payload or create a field (`0x00980F3A`). Names
are emitted by the text definition writer but are not part of a binary
descriptor and do not participate in descriptor matching.

The binary adapter table is `0x00AD9978`. It writes descriptor ID/type/count
through the serial record's uint32/uint16/uint16 methods and payload elements
through the corresponding typed methods, in count order, without text names
or comma delimiters. Serial byte order and string/key representation are the
contracts in persistence section 3. Fixed-width reads add the width to their
counter *before* requiring counter < total size, then delegate; equality
fails without a serial read. A descriptor adds eight once before its three
reads; a key adds twelve once. A string instead delegates its length-prefixed
read first, adds returned byte length plus four, and fails only if counter >
total. These distinct boundary rules must not be normalized (`0x0097EBA0`,
`0x0097EBE6`–`0x0097ECD5`, `0x0097ECDE`, `0x0097ED17`, `0x0097ED30`).

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

The uint64 read slot 7 inherits the **binary** base implementation at
`0x0097EC7A`: it reads eight serial bytes with the strict binary counter rule,
not a comma-delimited `%I64u` value. Its text writer still emits decimal text.
The small integer read slots narrow a uint32 text result to 8/16 bits; signed
and unsigned overloads share those low bits. These table-sharing quirks must
not be replaced by a uniform text scanner. No messages or drawing are supported.

### 6.2 Text format and failures

*[compat]* A definition is emitted as `\r\n<name>[ <id>, <type>, <count> ]= `.
Values have trailing comma and space: unsigned integers use `%u, `, signed
integers `%d, `, 64-bit integers `%I64u, ` / `%I64d, `, f32 `%f, ` with the
argument promoted to double, f64 `%g, `, and strings `%s, `. Strings are raw,
unquoted text, so commas are not protected by a quoting layer. Output uses the
serial record's `SetFieldString`; see persistence section 3 for that IO
contract (`0x0097F54D`, `0x0097F5B7`, `0x0097F5E4`–`0x0097F93A`).

There is no fragment-unwrapping layer on text input. With the concrete serial
record, every output fragment has its own four-byte length prefix; the raw
delimiter reader consumes those bytes as ordinary characters too. It ignores
everything through `[` when finding a definition, so a preceding length word
can be skipped incidentally. A length word before a numeric value can prevent
conversion, including when its first byte is a control byte or NUL. Preserve
these independent read/write contracts; this interface does not promise that
its concrete length-prefixed output roundtrips through the raw text reader.

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

The uint32 reader and definition components use decimal `%u`: optional sign,
decimal digits, no expression or hexadecimal-prefix interpretation. Accumulation
wraps modulo 2^32, including negation; sint32 shares those bits. Small reads
then retain 8/16 bits. The signed64 text reader uses `%I64d` and accumulates
modulo 2^64, with optional modular negation. Overflow still counts as successful
conversion (`0x00980B19`, `0x00980B80`, `0x00980D81`–`0x00980DDA`;
runtime accumulation `0x009F83DB`–`0x009F8460`, `0x009F8518`–`0x009F856F`).
This differs from the saturated hex-expression path in section 2.2.

Float text input scans a decimal prefix using `%lf`, then stores as binary64
or narrows to binary32. It accepts decimal exponents rather than the game's
wrapping exponent helper. Overflow can yield infinity without changing the
successful assignment count; narrowing a finite double can likewise overflow
to float infinity. Bare `inf`/`nan` fail and zero the output. The reader scans
only decimal digits, decimal point and exponent, so `1.#INF` reads the numeric
prefix 1, not infinity (`0x009F7D46`–`0x009F7EDA`, `0x00980BEA`,
`0x00980C59`). Output uses the bundled CRT's legacy nonfinite forms, whose
significand strings are `1#INF`, `1#QNAN`, `1#IND`, `1#SNAN`; formatting inserts
the decimal point after the initial 1 and preserves sign. For example `%g`
positive infinity is `1.#INF`, with the text writer's `, ` suffix. `%f` uses
its six fractional positions (positive infinity `1.#INF00, `). Negative
indefinite and other NaN payload classes must not all be respelled modern
`nan` (`0x00A049CA`–`0x00A04A40`, strings `0x00AECFF0`–`0x00AED008`).

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

The resource's base type getter returns its resource key's type member; the
base constructor zeroes that member. Therefore a fresh unkeyed image resource
reports type zero, and assigning a key with type T makes it report T. It has
no independent hardcoded image type, nor does its pixel color type select the
resource-key type (`0x00975633`, getter `0x009CEDE1`, primary table
`0x00AD8F28`).

### 7.2 BMP and TGA input

*[compat]* Record loading (`0x00979171`) first tries BMP at the current cursor.
If that fails it seeks to absolute zero and tries TGA; a failed seek aborts.
The file BMP loader (`0x0097878C`) seeks to `bfOffBits`. The record loader never
seeks to that offset: pixels are read at the cursor following the headers and
any palette. It merely requires unsigned 32-bit `(bfOffBits + 14) >= 54`.
For RLE with zero image-size field it uses `bfSize − bfOffBits` as compressed
length, rejecting length >= bfSize; otherwise it uses the declared image size.
An extra gap before the declared pixel position is therefore decoded as pixel
data on the record path (`0x00978EF5`–`0x00978F39`). Failed loading must not be
treated as a decoded image. Failures before buffer creation retain its prior
state; failure after creation/lock/conversion can leave changed dimensions or
partially written pixels. There is no rollback or buffer replacement on this
path. A subsequent TGA attempt can change the same retained buffer again.

BMP accepts a 14-byte `BM` header and 40-byte information header. Header
validation permits compression values 0 and 1 and depths 4, 8 and 24
(`0x00977372`, checked against instructions). Supported source-to-destination
translations (`0x0097868A`) are 4→16, 8→8/16/32, and 24→16/24/32; other pairs
return false. Record stride uses four-byte alignment of width times
`(bitsPerPixel >> 3)`, including the 4-bpp quirk. The file loader instead uses
ceil(width/2) before alignment for 4-bpp. RLE8 handles runs, absolute spans
with odd-byte padding, EOL, EOB and delta escapes. Malformed compressed data
does not have an established safe bounds contract.

*[choice]* Reject truncated RLE commands and source/destination overruns
without reproducing unsafe native memory access. Such malformed streams have
no deterministic successful-image or rollback requirement. This does not
change the supported RLE escapes, offset behavior or retained-state rules.

TGA loading (`0x00978A69`) seeks to zero, reads an 18-byte header, skips image
ID and color-map bytes and accepts image types 2 and 10. Both raw and packet
RLE paths support 16/24/32-bit conversion. Descriptor bit `0x20` controls
vertical flipping; the horizontal-origin bit is ignored. Source masks are
`0x7C00/0x03E0/0x001F` for 16-bit and `0xFF0000/0x00FF00/0x0000FF` for
24/32-bit. A declared 16-bit alpha channel uses `0x8000`; 32-bit alpha uses
the declared number of most-significant alpha bits. Unsupported pixel depth
can still report success after buffer creation and locking, so success alone
does not validate that unsupported input.

Loaders create the destination using the configured create flags. Both BMP
paths lock/unlock with `0x8040`; TGA uses `0x8080` (`0x0097909D`,
`0x00978A69`). These are lock flags, distinct from creation flags.
The specialized BMP 24-to-4444 path is selected only
when requested color type is 1, destination depth is 16 and the buffer's
reported color type is **7** (`0x0097868A`). The TGA 32-bit path similarly
tests reported color type 7 before calling the ARGB8 dither routine
(`0x00978CCA`–`0x00978CDF`).

### 7.3 Exact 4444 diffusion

*[compat]* Both dither routines start with zero integer error storage and scan
the first output row left-to-right, the next right-to-left, alternating each
row. BMP source starts at its last stored row and moves backwards by aligned
source stride; ARGB8 uses its supplied starting row and signed source pitch.
Per channel, add floor(accumulated weighted error / 16) to the source byte,
clamp only above 255, emit the high nibble, and retain the low nibble as error.
Send that error with weights 7 to the next pixel in the scan direction, 3 to
the next row's pixel opposite the scan direction, 5 directly below and 1 to
the next row's pixel in the scan direction. Off-image contributions go to
padding cells and do not wrap around. There is no half-unit rounding bias,
negative error, floating arithmetic or division by 17.

Errors are packed in uint32 byte lanes B/G/R/(A). Each lane holds a sum of
weighted low nibbles, so extracting bits 4–7 of each lane is the division by
16, with no cross-channel carry for valid byte inputs. The 24-bit version
diffuses B/G/R only and emits alpha nibble F; the ARGB8 version diffuses alpha
too. Destination words are A:R:G:B in descending nibbles. Both use unsigned
byte loads, shifts, unsigned upper clamps and 16-bit output stores, verified
at `0x00978325`–`0x009783DF` and `0x00978571`–`0x00978605`.

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
| 22–24 | GZString (`RZString` label), RZChar string, Object |
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
| SCR-V31 | Split `([)]`, `(a(b)c)`, `a"b c"d` | Respectively `[`, `]`; `a(b)c`; `ab cd` |
| SCR-V32 | Split quoted text containing `#` before its closing quote | Comment truncation occurs first; unterminated-quote error 5 |
| SCR-V33 | ParseFloat `-2^2`, `2^3^2`, `2^-2`, `-5%3`, `5%-3` | 4, 512, 0.25, 1, -1 |
| SCR-V34 | ParseFloat `.`, `1.2.3`, `1e+`, `1e+x`, `0X10` | Zero, approximately 1.2, 1, error, error |
| SCR-V35 | ParseFloat `1e10`, `1e12`, `1e32`, `1e-32` | 1410065408, -727379968, zero, positive infinity |
| SCR-V36 | ParseFloat `0x100000000`, `inf`, `nan`, `sqrt(-1)` | 4294967295, error, error, NaN under masked exceptions |
| SCR-V37 | ParseFloat `pow(2,3)`, `dsin(90)`, `sin(0,1)` | 8, unknown-function error, argument-syntax error |
| SCR-V38 | Fresh command parser, then Init, then set variable and Reset | Initial version 1 and no registrations; Init installs ordered builtins; Reset version 0, variable and registrations retained, prefix/namespace stack and published macros cleared |
| SCR-V39 | Root x=`root`; namespace a x=`parent`; namespace b x=`child`; lookup x before/after each pop | `child`, `parent`, `root`; missing name returns null |
| SCR-V40 | In namespace a, assign x and remove exact key `x`, then `a:x` | First removal leaves `a:x`; second removes it |
| SCR-V41 | Substitute x=`1+2` into `$x`, `$x*3`, `a $x b`, `${x}*3`, `${x` | `1+2`, `1+2*3`, `a (1+2) b`, `1+2*3`, `1+2` |
| SCR-V42 | Macro formal x, actual `1+2`; body arguments `%x*3` and `a %x b` | `1+2*3` and `a (1+2) b`; unknown formal throws |
| SCR-V43 | `set x first ignored`, `seti x [2+3]`, `setf x [1/2]` | x becomes `first`, `5`, `0.5` in sequence |
| SCR-V44 | `version 12suffix`, `trace anything`, `trace`, `list`, `end` outside block | Version 12; trace succeeds silently; last three throw |
| SCR-V45 | `setv3 v [1 2 3] -add [1 1 1] -scale 2 -mult [3 4 5]` | v=`7 17 31`; operations are scale then mult then add |
| SCR-V46 | `setc c [1 2 3 4]`; same with `-alpha 9`; same with `-scale 2` | `2 3 4`; `9 3 4`; `4 6 8` |
| SCR-V47 | `setc c [0.5 2 3]` | `2 3 1 0.5`: fourth component is the stored R, omitted default A was 1 |
| SCR-V48 | `eval [set x hello]`; macro m with two formals, `arrayCreate m 2` | x=`hello`; two expansions with actual lists `0 2` then `1 2` |
| SCR-V49 | File `dir/a.txt` includes `sub/b.txt`, which includes `../c.txt` | Requests `dir/sub/b.txt` then `dir/sub/../c.txt`; no canonicalization at this layer |
| SCR-V50 | `sinclude missing` versus `include missing`; nested syntax error in either | Missing open suppressed versus code 3; syntax errors unwind both files without resuming parent lines |
| SCR-V51 | Optional top-level syntax failure, then valid required file | Error recorded, later file parsed, aggregate success, post and success hooks run |
| SCR-V52 | Required syntax failure, then valid required file | Later file still parsed; aggregate failure; post hook runs, success hook omitted |
| SCR-V53 | Binary fields of types 1–12, count 1 | Payload width/order follows section 5.2 table, including type 6 sint64 and type 12 float64 |
| SCR-V54 | Binary width-4 read, counter 4, total 8 versus 9 | Equality fails without serial IO; total 9 permits read; counter becomes 8 in both cases |
| SCR-V55 | Binary empty string read, initial counter 4, total 8 | Delegates uint32 length read first and succeeds with counter=8; unlike fixed-width equality |
| SCR-V56 | Existing field renamed by SetTokenString | Text definition name changes; ID/type/count and payload unchanged; nonexistent ID fails |
| SCR-V57 | Concrete SetFieldString fragment `42, ` at raw reader cursor | Length bytes `04 00 00 00` are raw input; no fragment unwrap; numeric conversion fails and zeroes output |
| SCR-V58 | Raw uint32 text `4294967296, `, `-1, `, `0x10, ` | Success with 0, 4294967295, 0 (decimal prefix only) |
| SCR-V59 | Raw small-integer text `257, ` into uint8/sint8, `65535, ` into sint16 | Success with 1, 1, -1 |
| SCR-V60 | Raw sint64 text `18446744073709551616, `; text adapter uint64 read | Sint64 succeeds with zero; uint64 instead delegates eight binary bytes, with strict binary boundary |
| SCR-V61 | Raw float text `1e9999, `, `inf, `, `1.#INF, ` | Successful positive infinity; failure/zero; successful 1 |
| SCR-V62 | Write positive infinity as f64 and f32 text | `1.#INF, ` and `1.#INF00, ` respectively; no promised text roundtrip |
| SCR-V63 | Unkeyed image; then assign resource key type T | Base type getter yields zero, then T; color type does not affect this |
| SCR-V64 | Record BMP with valid offset check and gap after palette | Gap bytes are pixel input; file BMP instead seeks declared offset |
| SCR-V65 | Record BMP rejects before destination creation; fails after destination creation | First retains old buffer state; second may leave resized/partially modified retained buffer; no rollback |
| SCR-V66 | 24-bit dither, one row of three pixels B=G=R=15, source rows/padding valid | Output words F000, F111, F111; second pixel receives floor(15×7/16)=6 |
| SCR-V67 | ARGB8 dither, one row of three pixels A=R=G=B=15 | Output words 0000, 1111, 1111; alpha diffuses too |
| SCR-V68 | 24-bit BMP, requested type 1, 16-bit destination, reported color type 7 versus another type | Select 4444 dither only for type 7; ordinary translator otherwise |

Additional vectors for switch/positional readers and required/optional file
ordering are normative in the linked specs. Inputs whose exact result is an
open question must not be promoted into guessed conformance expectations.

## 10. Native observation contracts (SCR8 and SCR9)

### 10.1 x86 arguments, results and interface views

These contracts describe the Windows 1.1.641 entry points, including ones whose
exported name is misleading. They are for observing an existing native call;
they do not prescribe a public FreeCity4 ABI. Addresses are executable virtual
addresses. Member entries use x86 `thiscall`: ECX is the stated object/view,
the stack contains arguments in the order listed below, and the callee removes
their bytes. Each pointer/reference occupies four stack bytes. A `bool` result
is **AL only**; upper EAX bits are not a comparison result. `u32`, `s32` and
pointers return in EAX. `void` has no result to compare, regardless of the
value left in EAX. No method below returns a float in the x87 result register.
For small by-value integers/bools the stack slot is four bytes but only the
declared low byte/word is meaningful. References to 64-bit integers/doubles
are one pointer, never a split by-value argument.

Notation: `S` is a `cIGZString` object/interface, not a character pointer;
`char*` denotes a NUL-terminated byte string. `D` is the eight-byte descriptor
`{u32 id,u16 type,u16 count}`. `K` is three consecutive u32 words in type,
group, instance order. `R` is a serial-record interface. `T&` denotes an
address of one readable/writable T, not a pointer to an array header. Capture
the contents of strings/references while they are alive, not their addresses
as replay values. Native STL containers are not portable C arrays.

#### Command and file parser

| Entry | ECX view | Stack arguments in declaration order | Result; stack bytes removed |
|---|---|---|---|
| ParseCommand `0x00679450` | parser input view; command state base is ECX−0x98 | `const char* line` | void; 4 |
| ParseCommandArguments `0x00679120` | command state base | reference to mutable native vector of S | void; 4 |
| RegisterCommand `0x00676DB0` | command state base | `const char* name`, command object pointer | void; 8 |
| SetParser `0x00680930` | file main interface | parser input interface pointer | void; 4 |
| AddInputFilePath `0x00681DA0` | file main interface | `const char* path`, `u32 flags` | void; 8 |
| ParseFiles `0x006812F0` | file main interface | `s32 start`, `s32 count` | bool; 8 |
| ParseFiles(all) `0x00680990` | file main interface | none | bool from range call; 0 |
| SetModified `0x00680900` | file main interface | bool | void; 4 |
| ErrorString `0x00680910` | file main interface | none | borrowed `const char*`; 0 |
| GetParser `0x0040CE60` (slot 11 of file table) | file main interface | none | raw parser pointer; 0 |
| NumInputFiles `0x006809C0` | file main interface | none | s32; 0 |
| InputFilePath `0x00680A30` | file main interface | `s32 index` | borrowed `const char*`; 4 |
| Modified `0x00680920` | file main interface | none | bool; 0 |
| DoParseFile `0x00680EA0`, continuation `0x00681176` | file main interface | `const char* resolvedPath`, `const char* contextName` | s32 accumulated line count on ordinary return; 8; parse failures throw |

The native vector is three pointers (begin, end, allocation end); each S
element is 20 bytes. It includes the command name at element zero and can be
changed by variable substitution. The third argument of typed token-record
accessors below is unrelated to this vector. ParseCommand produces effects
or a native parse exception, not a bool or an output token array. Observe
tokenization with SplitIntoArguments (section 2) and dispatch with
ParseCommandArguments if token-level comparisons are required; a line call
alone does not expose a returned token list. Borrowed file-parser strings
must be copied before the parser changes its registration/error state.

The −0x98 adjustment is local to `0x00679450` and the base parser layout; do
not subtract it from arbitrary derived parser interface pointers. The game's
derived entry thunks adjust ECX first: `0x007E4FB0` (rule parser),
`0x0059F400` (effects-side parser), `0x00679640` (base thunk). Preserve those
thunks when intercepting a vtable call. This avoids treating an input interface
as the complete derived object. The file main pointer is the object start;
its message-target view is +4 and its `cRZUnknown` view is +8. Evidence:
`0x006794DB`–`0x006794E6`, `0x00681706`–`0x00681729`,
`0x00680A30`–`0x00680A45`, `0x00681F4D`–`0x00681F50`.

#### Token-record object

The `0x00AD9A88` table is at the token object start. It is distinct from both
stack-local IO adapters. Its scalar getters/setters operate on **one indexed
element**, with a complete descriptor reference as their first argument.

| Slots / entry | Arguments | Result; stack bytes removed |
|---|---|---|
| 3 InitRead `0x009810CD` | `R* record` | bool; 4; consumes cookie itself |
| 4 InitRead `0x00981052` | `R* record`, `u32 cookie` | bool; 8; cookie already consumed by caller |
| 5/6 InitWriteText/Binary | `R* record` | bool; 4 |
| 7 GetKey | `K& out` | bool; 4 |
| 8 GetCookie | none | u32; 0 |
| 9 IsText | none | bool; 0 |
| 10 IsTokenAvailable | `const D& definition` | bool; 4 |
| 11 SetTokenString | `const D& definition`, `const char* name` | bool; 8 |
| 12–23 GetFieldT | `const D& definition`, `T& out`, `u32 index` | bool; 12 |
| 24–35 SetFieldT | `const D& definition`, `const T& value`, `u32 index` | bool; 12 |

T follows the twelve-type slot order in section 5.1. Thus a float64 getter
receives a pointer to eight writable bytes; a string getter receives an S
object; a key getter receives twelve writable bytes. None returns its value
in EAX. The descriptor count selects stored field capacity. An element index
equal to or greater than that count fails without copying the element; a
failed getter leaves the output untouched. A setter can create the descriptor
before discovering an out-of-range index, so failure does not imply that the
field map is unchanged. A different descriptor at the same ID is not an
indexed access to the previous field. Shared signed/unsigned machine entries
do not change the declared element width.

Evidence: descriptor matching `0x0097F1D4`–`0x0097F214`, getters
`0x0097F2B4` and setters `0x00980881`–`0x009808BE`; the u32 field's
element accessors `0x0097FC36`–`0x0097FC75` compare the unsigned index with
the stored u16 count and copy through the value pointer. The apparent void
setter in pseudocode is an AL-returning tail delegation, not a void API.
Shutdown `0x00980F87` remains void and can trigger deferred output. It is not
a passive accessor. There is **no token-record raw-byte getter** in this
table. The retained serial pointer at object +8 is an implementation capture
anchor, not another COM interface or a contiguous-data accessor.

#### Token IO adapters, including writes

Both adapters use a 16-byte local object: vptr at +0, borrowed R pointer at
+4, u32 counter at +8, u32 total at +12. The binary initializer
`0x0097EB78` sets table `0x00AD9978`, obtains total through R offset +0xD0,
and sets counter to 4. The text constructor `0x0097EE84` performs that
initialization then installs `0x00AD99E8`. Neither table is a COM interface;
slot zero is a deleting destructor, not QueryInterface. The misleading
export name on `0x0097EB78` does not make it an STL erase operation.

| Slots, either table | Arguments | Result; stack bytes removed |
|---|---|---|
| 1 ReadTokenDef | `D& out` | bool; 4 |
| 2–13 ReadT | `T& out` | bool; 4 |
| 14 WriteTokenDef | `const S& name`, `const D& definition` | bool; 8 |
| 15–26 WriteT | `const T& value` | bool; 4 |

For slots 2–13 and 15–26 the type order is K, S, f64, f32, s64, u64,
s32, u32, s16, u16, s8, u8. String input/output is an S reference, not a
character buffer and length pair. Binary definition writing ignores the name;
text definition writing obtains its character pointer through S slot 5 and
formats the descriptor (section 6). Text numeric writers, **including s64,
u64, f32 and f64**, receive const references. In particular
`0x0097F8F4`–`0x0097F8FB` loads a double from the address supplied in its
single stack word. Numeric reference widths are 1/2/4/8 bytes as declared,
not uniformly four bytes. Key writing `0x0097EEEB` reads three words from
one key reference and dispatches three u32 writes. Names and reference values
must be copied at entry, before invoking the original.

Text helper `0x0097F5B7` takes `const S& fragment`, returns bool in AL and
removes 4 bytes. It calls R raw write at +0xCC with `(const void* bytes,
u32 length)`, rather than R SetFieldString; this is a useful final-byte
observation point for token text. It has no output buffer of its own.
Observe the fragments actually passed to raw write; never call Shutdown or
WriteTokens a second time to discover output. The text read helper
`0x00980A41` takes `(S& out, char delimiter)` in two stack words, returns
bool in AL, and removes 8 bytes. It is a delimiter reader, not a vtable
string getter. The public string slot supplies its own delimiter.

#### Image/buffer resource and ASCII writer

| Entry/view | Arguments | Result; stack bytes removed |
|---|---|---|
| Buffer-resource view +0x18, slot 3, `0x009779C9` | none | EAX buffer pointer, AddRef'd if nonnull; 0 |
| Same view, slot 4, `0x009BC0BD` | `u32 iid`, `void** out` | bool; 8; forwarded buffer QI |
| Same view, slot 5, `0x009779DF` | buffer pointer | void; 4; retained replacement |
| Complete image object, LoadFromRecord `0x00979171` | `R* record`, `u32 option` | bool; 8 |
| Complete image object, LoadBMPFromRecord `0x00978E35` | `R* record`, `u32 option` | bool; 8 |
| Complete image object, LoadTGAFromRecord `0x00978A69` | `R* record`, `u32 option` | bool; 8 |
| Complete image object, LoadFromFile `0x0097878C` | `const S& path`, four u32 option words | bool; 20 |
| Factory LoadFromFile `0x009790ED` | `const char* path`, buffer output pointer address, four u32 option words | bool; 24 |
| Factory LoadFromSerialRecord `0x00977ADC` | buffer output pointer address, `R* record`, u32 option word | bool; 12 |
| Factory SaveToSerialRecord `0x00977E93` | buffer pointer, `R* record`, bool format selector | bool; 12 |

The last three functions do not use ECX as a factory state pointer; treat
them as callee-cleanup stack functions. Forward all option words unchanged
when observing an existing call; they are not output addresses or extra
this pointers. Section 7 defines BMP/TGA decoding behavior. The factory
serial formats are separate image formats, not token descriptors. LoadBMP/TGA take the second stack
word even when a pseudocode listing omits it: the wrapper pushes both and the
BMP routine removes eight bytes (`0x00979177`–`0x009791A2`,
`0x009790EA`). The file resource entry removes 20 bytes
(`0x00978A66`); its factory wrapper converts the char path to S and forwards
the other four words (`0x0097911B`–`0x00979138`).

GetBuffer does not move any serial cursor, serialize, resize or lock pixels;
it increments buffer reference count, which the observer must balance.
Reading the already retained pointer at complete object +0x1C avoids even
that increment for a synchronous observation during the object's lifetime.
It gives a buffer interface, **not a raw pixel pointer or encoded file bytes**.
Use an existing successful buffer lock's pixel pointer, dimensions, pitch and
color type for a passive image comparison. Adding a new lock on the live
buffer is not guaranteed non-mutating. Preserve the different image-entry
and embedded-buffer ECX views.

ASCII writer ECX is the writer itself, table `0x00ADA288`:

| Slots / entry | Arguments | Result; stack bytes removed |
|---|---|---|
| 3 WriteRoot `0x009835ED` | persistent root-resource pointer, output-stream pointer | void; 8 |
| 4–11 Write scalar bool/s8/u8/s16/u16/s32/u32/f32 | `const char* fieldName`, T **by value** | void; 8 |
| 12 WriteFloat64 `0x0098228D` | `const char* fieldName`, f64 **by value** | void; 12 |
| 13–21 WriteArrayT | `const char* fieldName`, `const T* values`, `u32 count` | void; 12 |
| 22 WriteString `0x00982819` | `const char* fieldName`, `const S& value` | void; 8 |
| 23 WriteString `0x0098285F` | `const char* fieldName`, `const char* value` | void; 8 |
| 24 WriteObject `0x0098298A` | `const char* fieldName`, serializable-object pointer | void; 8 |
| 25 WriteObjectRef `0x009811C8` | `const char* fieldName`, unknown-interface pointer | void; 8 |
| 26 WriteObjectRef `0x00982F2B` | `const char* fieldName`, referent-interface pointer | void; 8 |
| 27 WriteTypeInfo `0x00982A1F` | const reference to type-info record `{S* name,u32 classID,u32 version}` | void; 4 |
| 28 OutputStream `0x00982F1C` | none | EAX output-stream pointer; 0 |
| nonvirtual WriteUint32InHex `0x009828A5` / WriteKey `0x009828EA` | u32 by value / `const K&` | void; 4 |

Array T order is the nine-type scalar order in section 8.1. No ASCII writer
slot accepts a 64-bit integer scalar. These void calls cannot establish
durable-write success. Evidence for f64 by-value width is
`0x009822B8`–`0x009822D7`; array pointer/count and 12-byte cleanup are
`0x0098230B`–`0x00982329`, `0x00982378`. Slots 22/23 have distinct
`RZString`/`RZChar*` prefixes and stream operations at
`0x00982823`, `0x0098284C`, `0x00982869`, `0x00982892`; their order in
section 8.1 is corrected accordingly. Slot 25 queries for a referent and
releases that temporary after slot 26 (`0x009811D3`–`0x00981202`).
The type-info argument is a twelve-byte data record, **not** an object with
a vptr: the first word points to an S, and the following two words are
read directly (`0x00982A25`, `0x00982A43`, `0x00982A52`, `0x00982A71`).

OutputStream merely forwards the active surveyor's stream getter; it is not
an output-byte snapshot. Calling it after WriteRoot has popped the surveyor
is invalid. During an active context it does not write or seek, but there is
no universal readable-memory or byte-count accessor on this writer. Capture
the output sink supplied to the existing WriteRoot, or the bytes naturally
written to its underlying sink. Do not re-run WriteRoot: surveying and
serialization invoke live objects again. Record the actual stream identity
and operation order, including nested roots, instead of using one global
"last stream". Ordinary file/stream sinks need not retain readable bytes.

### 10.2 Natural dispatch and passive capture boundaries

An INI file or a DBPF record is not a token record merely because both contain
numbers. The token tables are reached only after the token-record read/write
protocol is selected. Advancing a city date does not itself imply that
protocol ran. Installing these two tables at OnStart cannot expose a parser
which does not call them. Static direct-call counts are also not dynamic
traffic counts: calls through a vtable can have no exported direct callers.

| Natural operation / discriminator | Dispatch actually used | Observation boundary |
|---|---|---|
| Startup graphics-rule files `Graphics Rules.sgr`, optional `Graphics Options.sgr`, recursively `Video Cards.sgr` | file parser → `cRZTextFile::ReadLine` `0x00930833`; ordinary lines call parser input slot 5 (+0x14); includes recurse in the file parser | register file/parser observation before config-rule initialization; capture registered paths, line input, tokens and exceptions; section 4 and graphics-rules section 3 |
| Other users of the shared command parser, including effects | derived parser input thunks → `0x00679450` → direct `0x00679120`; macro/eval expansion also calls the argument entry directly | cover the shared function entry as well as the concrete input vtable; label expanded token dispatch separately from physical file lines |
| Token-compatible resource read through the factory table at `0x00AD9A54`, slot 10 (`0x0097F439`) | query target for IID `0xA2ACE668`; read cookie through factory slot 12; construct token record; call token slot 4 with record plus consumed cookie; call target slot 3 with token record | token InitRead entries, including explicit-cookie variant, before their first invocation; correlate factory target/record identity |
| Token-compatible resource write, same factory slot 11 (`0x0097F4C4`) | query target for IID `0xA2ACE668`; target slot 6 selects text/binary; token slot 5/6 initializes; target slot 4 fills token record; final release serializes | observe init/setters and original Shutdown/raw writes; do not infer bytes from the fill callback's result alone |
| Image resource's factory SerialRead `0x009791B1` | resource type `0x856DDBAC` selects a temporary image and **direct** LoadFromRecord `0x00979171`; that directly tries BMP then TGA | function entry of LoadFromRecord or its factory boundary; capture record bytes and existing buffer writes; token tables never receive this decode |
| Image factory's `LoadFromSerialRecord` / `SaveToSerialRecord` | direct raw/typed serial operations using its own image format | observe `0x00977ADC` / `0x00977E93`, not token IO adapters |
| ASCII graph dump | existing caller invokes writer WriteRoot; graph serialization uses writer field methods and its supplied stream | observe WriteRoot before first dump and follow that stream; normal binary city saving does not imply an ASCII dump |

The token read chain above consumes the cookie **before** explicit-cookie
InitRead. Consequently a trace beginning at slot 4 must include the known
cookie and the record cursor at entry; prepending another cookie to the live
record or reading one there changes the operation. The adapter's initial
counter is still 4. Token adapters are short-lived local objects within the
token record, not process-wide parser singletons. The text constructor briefly
uses the binary table during base initialization, then replaces it.

The specific clean natural file route established here is the startup rule
route, not the shipped pack-1 INI prefix used in the constructed probe.
Graphics rule setup precedes ordinary city load/month observations. Install
before that initialization and capture during startup, not just dated city
callbacks. OnStart installation is useful only if the selected boundary
covers that route: moving token-table installation earlier does not change
the route. These static contracts do **not** assert that a particular city
loads a token-compatible resource or requests an ASCII dump. Those classes
can legitimately remain unobserved. There is no lifecycle event that forces
every owner in sections 3–8 to execute naturally.

For passive byte capture, first prefer copying the buffers of **existing**
raw reads/writes at their original call boundaries. R raw read is offset
+0xC8 `(void* out,u32 length) → bool`; raw write is +0xCC
`(const void* bytes,u32 length) → bool`; both remove eight stack bytes.
Keep input bytes before their lifetime ends; record failures without treating
uninitialized/truncated output as valid input. A high-level operation can
seek backwards (the BMP/TGA fallback does), so retain offsets and ordering,
not just concatenated reads. These are byte operations, not string-field
methods that consume/insert length prefixes.

A read/seek snapshot is a fallback for records supporting it, **not a
non-mutating accessor**: use the SDK's position/size/raw-read/absolute-seek
contract, save the exact cursor at the observation boundary, perform the copy
under a recursion guard, restore that cursor and verify restoration before
resuming. If copying after the native call, restore the native **post-call**
cursor, not its entry cursor. Exclude captures on failed read/seek/restore
and do not attempt snapshots on a write-only stream. Restoring a cursor alone
does not prove that a nonseekable/side-effecting provider was passive. A
native typed getter, Shutdown, serialization callback or ASCII OutputStream
outside an active graph is never a substitute for a raw snapshot API.

Natural coverage requires real nonempty samples associated with these
boundaries, comparing results, consumed positions, output bytes and relevant
state. Report owner/route separately. The reported `scres5-r1-2032-a` and
`scres5-r1-2032-b` (zero calls) establish no equality; the round-2 56/56
constructed adapter cases establish only those adapter cases, including the
16-byte INI prefix treated as scalar binary data. They do not establish INI,
command/file-parser, token-resource, image or graph-writer coverage.

### 10.3 Additional conformance and capture vectors

These are normative spec vectors, not claims of new native game-run results.
Use section 9's wire/format vectors for payload content as well.

| ID | Input/observation | Required result |
|---|---|---|
| SCR-V69 | Typed u32 field descriptor `{42,3,2}`; set index 0 to 11 and index 1 to 22; get index 1 | Both sets succeed; getter writes 22. The last argument is an index, never a request to copy two words |
| SCR-V70 | Same descriptor; get index 2 into sentinel `0xCAFEBABE`; then set index 2 | Both return false in AL; getter preserves sentinel; existing elements remain 11,22 |
| SCR-V71 | Missing field `{43,3,2}`; setter receives index 2 | False element result may coexist with newly created field; do not infer rollback |
| SCR-V72 | Text adapter f64 write with a reference to 1.5; ASCII f64 field `x` with by-value 1.5 | Adapter consumes one pointer word and emits the section-6 fragment; ASCII consumes name plus eight value bytes, emits `      Float64 x = 1.5` and LF using its stream conversion; no void-call EAX comparison |
| SCR-V73 | ASCII slot 22 with S `hi` versus slot 23 with char pointer `hi`, name `x` | Lines `      RZString x = hi` and `      RZChar* x = hi`, each LF; do not interchange string representations |
| SCR-V74 | File parser reads ordinary `set x hello` in a real rule file | Parser input receives the line, argument dispatch sees command/name/value tokens; token IO adapter observation is not required and cannot stand in for these calls |
| SCR-V75 | Factory token read consumes `TKB1`, then explicit-cookie InitRead | InitRead receives cookie by value and record at first descriptor; no second cookie read; local adapter counter starts 4 |
| SCR-V76 | Image LoadFromRecord rejects BMP, seeks zero and tries TGA | One wrapper observation, ordered BMP/seek/TGA suboperations; no token descriptor traffic implied |
| SCR-V77 | Native record call ends at position q, observer snapshots via raw read/seek | Resume at q with native result/output unchanged; refuse comparison if copy or restoration fails |
| SCR-V78 | No token adapter calls during a dated city interval | Zero observations, no natural equality/coverage credit; constructed adapter successes stay a separate verdict |
| SCR-V79 | ASCII WriteRoot has returned and popped its last surveyor | Do not call OutputStream to capture bytes; use the observed sink transcript from the original active root |
| SCR-V80 | ASCII WriteTypeInfo record names S `Thing`, classID 42, version 7 | `      TypeName: Thing GZCLSID: 0x0000002a Version: 7` and LF; read the record fields, do not invoke a type-info vtable |

## 11. Physical file-line observation contract (SCR10)

### 11.1 Native signatures and lifetimes

These are 32-bit Windows `thiscall` boundaries. ECX is the complete
`cRZTextFile` object, also its file-interface view at offset zero; it is
**not** the file-parser interface. Stack arguments below are in entry order
starting at `[ESP+4]`. S is the 20-byte native string from section 10.1.
Bool results are in AL only; upper EAX bits have no result meaning.

| Boundary | Stack arguments | Result; callee stack cleanup |
|---|---|---|
| Path constructor `0x009307B5` | `const char* path` | object pointer in EAX; 4 bytes |
| Text open `0x0093066E` | three 32-bit values: access, creation, sharing | bool; 12 bytes |
| ReadLine `0x00930833` | `S& output` (one writable interface pointer) | bool; 4 bytes |
| Close `0x009188A5` | none | bool; 0 bytes |

The constructor copies the supplied path into the base file's S at object
+4; it does not borrow the caller's character storage. The text object's
table is `0x00AD1E58`. ReadLine clears the caller's output before checking
open success or EOF, and assigns the completed result through the output's
string interface. There is no returned character pointer or length in EAX,
and no caller-provided character-buffer capacity. The caller owns this S;
its bytes can be copied at ReadLine return, before the caller edits or
destroys it. Neither that S nor the stack-local file object may be retained
as an owned COM reference by an observer. In DoParseFile the same output S
is reused for successive reads and is edited for comment processing
immediately after a successful read.

Evidence: constructor and path copy `0x009307B5`–`0x009307F9`,
`0x00919AD3`–`0x00919AE4`; output clearing `0x0093084C`–`0x00930856`;
assignment and bool/cleanup `0x009309FE`–`0x00930A20`; open cleanup
`0x009306DB`; Close `0x009188A5`–`0x009188F0`.

### 11.2 File association, nesting and physical positions

Each DoParseFile invocation constructs its own text file using its first
stack argument, the resolved path, and opens it with `(1,2,7)` in that
order: read access, open existing, and native sharing flags 7. There is no
path argument to ReadLine. Correlate its ECX with the constructor/open ECX
inside the active DoParseFile invocation, copying the constructor path.
The second DoParseFile argument is directory context for relative includes,
not the text file or its output. A child uses the same parser ECX but a
different stack-local text file. Keep a nesting stack and separate file
identities; a previously used stack address can be reused after return.
Remove that association on close/unwind. An open failure throws code 2
before any ReadLine call; it is not a false line read. Section 4.2 governs
include suppression and path joining. Evidence: `0x00680EC7`–`0x00680F0E`,
`0x006810F8`–`0x0068110A`.

The per-file physical line number starts at zero and increments once for
each true ReadLine, including empty, comment-only and include-directive
lines. A false read adds no line. Child lines do not advance the parent's
physical line number. Successful DoParseFile returns its own physical count
plus the returned counts of successful child parses; that inclusive total
is not a parent line number. Ordinary completion closes the file; a parse
exception also closes it and adds the current parent's filename and
physical line number while unwinding. Evidence: `0x00680F73`–`0x00680F7B`,
`0x0068110A`, `0x006811EC`–`0x006811FD`, `0x00681273`–`0x0068129E`.

Capture ReadLine output before comment removal, splitting, substitution or
include dispatch. A successful empty line is an observation even if it
produces no command. Section 12 specifies the physical block filter that
accounts for reads without a command/include boundary. Capture every returned bool, including the terminal
false. For byte offsets use the file's logical position (file slot 6), not
the operating-system handle position: the base file buffers reads. ReadLine
reads ahead and seeks back around terminators. Passive observation must
preserve those positions and must not issue extra reads. If installation
missed construction/open, refuse path-associated replay rather than infer
a filename from the most recent registration. This contract enables input
capture; it does not establish command-state or exception replay equality.

### 11.3 Returned bytes, terminators and failure quirks

For the ordinary single-byte mode used without a two-byte BOM, each CR or
LF terminates a line and is omitted from the returned string. An immediately
following *different* newline byte is consumed as part of that terminator:
both CRLF and LFCR are one terminator. A repeated identical newline byte
is left for the next read, which can therefore return true with an empty
string. Other whitespace and embedded zero bytes are retained in the S's
length-delimited output. A later conversion to a C string is a separate
boundary. A final nonempty unterminated line returns true. A final
terminator does not manufacture another successful empty line at EOF.
There is no 128-byte line-length limit: successful chunks are accumulated.
Evidence: byte/word selection `0x00930913`–`0x0093092E`, terminator and
lookahead/seek-back `0x0093092E`–`0x0093098D`, accumulation
`0x00930990`–`0x009309CE`, byte-string copy `0x0090FBE5`–`0x0090FC12`
and `0x00408BB3`–`0x00408BBA`.

False is deliberately ambiguous. It covers failure to open implicitly,
the signed comparison size <= current position, and failure of the first
chunk read. Output is empty in these cases. A failure after one or more
successful chunks returns **true with the accumulated prefix**; bytes from
the failed chunk are not appended. The subsequent read may return false.
The underlying fixed-size read rejects short transfers as well as errors.
Lookahead read/seek failures do not change a line already marked successful.
Thus ReadLine alone cannot distinguish clean EOF, truncation or I/O failure,
and DoParseFile treats every false as ordinary completion, not a parse
error. An observer may label an independently captured lower-level failure,
but cannot derive one from AL=false alone. Evidence: `0x00930859`–`0x00930886`,
`0x009308E9`–`0x0093090A`, `0x009309B6`–`0x00930A20`,
`0x00918395`–`0x009183A8`, `0x00680F73`–`0x00680F75`.

Text open probes two bytes when available. Either `FF FE` or `FE FF`
selects mode 2 and leaves the position after the marker; any other pair
is rewound to offset zero. No UTF-8 marker handling or endian swap occurs.
Mode 2 scans native little-endian 16-bit units for CR/LF, then converts units
<= `0x7F` to the corresponding byte and all others to `?`. There is an
additional native count quirk: for a scanned payload of b bytes, the wide
append receives b as a **unit count**, appending 2b bytes from the temporary
buffer. Consequently even well-formed wide input can append newline,
following-line or uninitialized temporary-buffer units beyond the scanned
payload. A full 128-byte payload can read beyond that temporary buffer.
This is not a correct Unicode line decoder; no deterministic whole-output
vector is assigned to bytes drawn from uninitialized/out-of-bounds storage.
Capture actual native output, and refuse deterministic replay for such
samples. The quirk is settled behavior, not an unresolved question.
Evidence: BOM probe `0x00930691`–`0x009306CC`; scan step and append count
`0x00930913`–`0x0093093F`, `0x00930997`–`0x009309A4`; doubling the count
`0x0093081B`–`0x0093082A`; range measured in words
`0x00918ACC`–`0x00918AD7`; lossy conversion `0x0090E929`–`0x0090E941`.

### 11.4 Conformance vectors

These are normative vectors derived from the instructions, not new game-run
measurements. Unless stated otherwise, files use single-byte mode, all
underlying reads/seeks succeed, offsets are bytes, and `\r`/`\n` denote
CR/LF. A false read clears a pre-existing output sentinel.

| ID | Input/observation | Required result |
|---|---|---|
| SCR-V81 | Empty file, output initially `old` | First ReadLine false, output empty; physical count 0 |
| SCR-V82 | Bytes `A\r\nB\n\rC\rD\n` | True outputs `A`,`B`,`C`,`D`, next false/empty; positions after true reads 3,6,8,10; physical count 4 |
| SCR-V83 | Bytes `\r\r\n\n` | True empty at position 1; true empty at 3 (CRLF); true empty at 4; next false; count 3 |
| SCR-V84 | Bytes `tail` versus `tail\n` | Each yields one true `tail`, then false/empty; positions 4 versus 5; count 1 in both |
| SCR-V85 | 300 `x` bytes followed by LF | One true output containing all 300 bytes, logical position 301; next false; count 1 |
| SCR-V86 | Bytes `41 00 42 0A` | True S contains three bytes `41 00 42`, position 4; C-string truncation is not a ReadLine result |
| SCR-V87 | File size 260; first 128-byte chunk of `x` succeeds; next chunk fails | True with 128 `x` bytes; failed chunk not appended; no ReadLine error code. A first-chunk failure instead returns false/empty |
| SCR-V88 | Parent lines: empty, `include child.sgr`, `set x y`; child has two physical lines | Parent reads numbered 1,2,3; child reads numbered 1,2 on its own ECX; parent return 5; terminal false in each file adds zero |
| SCR-V89 | Missing included file, or ordinary file reaches false ReadLine | Missing file throws code 2 at open with no line reads (section 4.2 then applies); false ReadLine closes and returns accumulated count without throwing |
| SCR-V90 | Bytes `FF FE 41 00 0A 00 42 00`, successful open and first read | Position starts 2; first read scans one payload word but appends two words, yielding S bytes `41 0A`; logical position 6. This exhibits the count quirk without relying on uninitialized bytes |
| SCR-V91 | Bytes `EF BB BF 41 0A` | Single-byte mode; first true S contains `EF BB BF 41`; UTF-8 BOM is retained |
| SCR-V92 | Same file address reused after a completed DoParseFile; constructor supplies a different path | New file identity/path and physical count starting at zero; never attach the new reads to the old path |

## 12. Physical block-comment preprocessing (SCR11)

### 12.1 Literal markers and discarded lines

*[compat]* File parsing has a block filter **before** section 2.1's `#`
line-comment/token rules and before include routing. Its opener is the exact
two-byte substring `#<`; its closer is `#>`, not `>#`. Neither marker needs
to start a line or token. Leading/trailing whitespace has no special meaning;
whitespace between the two marker bytes prevents a match. Matching is literal
and byte-exact, with no case folding, escaping, quote awareness, or group
awareness. Markers inside quoted strings, bracketed arguments, or text after
an ordinary `#` still participate. Direct command parsing alone does not
perform this file-layer filtering.

The file has a single active/inactive block flag, initially inactive. The
following table defines the transformations of one true ReadLine output,
in order. Searches select the first occurrence, starting at byte position 0.
The retained bytes are not trimmed or joined to any other physical line.

| Condition | Meaning for this physical line |
|---|---|
| An opener occurs anywhere, whether the flag was inactive or active | The flag becomes active. Discard the opener and **everything after it**, leaving only the prefix before it for the closer search. This alone never dispatches that prefix. |
| The flag is active and the remaining line has no closer | Discard the entire remaining line. No include routing, tokenization, or ordinary ParseCommand call occurs. The flag stays active. |
| The flag is active and the remaining line has a closer | The flag becomes inactive. Discard everything from the beginning through that first closer, including both closer bytes. Send only the suffix through the normal include/ordinary-command routing. |
| The flag is inactive and no opener occurred | Send the entire physical line through normal include/ordinary-command routing. A stray closer has no block meaning. |

There is no repeated marker scan after the closer is removed and no nested
block depth. Another opener while active still discards its own line tail;
it does not require another closer later. The opener search takes precedence
over the closer search even when both markers occur on the same line.
Consequently `#< text #>` does **not** close a block: the closer was discarded
with the opener's tail, so this line and subsequent lines without a surviving
closer are suppressed. Conversely, `#>set x y #< tail` closes even an initially
inactive file's block on that same line: the opener activates the flag and
leaves `#>set x y `, whose closer then leaves `set x y ` to route normally.
These are compatibility quirks, not a paired inline-comment grammar.

A closer-only line while active produces an empty suffix and **does** reach
ordinary ParseCommand with an empty string. A blank line while active without
a closer is suppressed. Outside a block, blank lines and ordinary comment
lines also reach ParseCommand, even when SplitIntoArguments finds no tokens.
Thus an empty token list alone cannot identify a suppressed physical read.
After filtering, the surviving suffix/payload uses the usual `#` line-comment
rule, include recognition, argument validation and command parser behavior.

Evidence: `DoParseFile` initializes a stack-local byte flag to zero at
`0x00680F59`; opener search and tail erasure are at
`0x00680F7E`–`0x00680FA5`; active-state closer search, skipping, flag clearing
and prefix erasure are at `0x00680FA7`–`0x00680FE0`. The marker pointers
`0x00B0D10C`/`0x00B0D110` refer to `#<` at `0x00AAF3A0` and `#>` at
`0x00AAF39C`, respectively. The search helper `0x00442850` uses the
length-aware, literal string search `0x004427F0`/`0x004426C0`; erase
`0x004425C0` clamps the requested length to the available tail. Instructions
confirm tail removal at the opener and prefix removal through the closer;
these directions must not be reversed. Tokenization starts at `0x00680FE5`,
and the empty/non-include path calls ordinary ParseCommand at `0x006811BA`–
`0x006811CD`.

### 12.2 Includes, counts and termination

The flag belongs to one DoParseFile invocation, not to the parser object.
Every recursive included file begins inactive and ends its own state on
return. Each registered file also starts inactive. A block's contents do not
open included files or evaluate commands, quotes or groups; malformed syntax
there is ignored. An include in a surviving closing-line suffix is routed
normally and can recurse. A child's unclosed block cannot suppress the next
parent line. There is no additional block state to restore on the parser
object after an include.

Every true ReadLine still increments the current file's physical count once,
including the opener, suppressed content and closer lines. Successful child
totals are still added to the parent's return count under section 11.2.
Neither a suppressed line nor a discarded include contributes any child
count. False ReadLine adds zero and ends the file normally even if the flag
is active: there is no unterminated-block error, synthetic closing line, or
dispatch of saved prefix/content. This also applies to the EOF-or-failure
termination described in section 4.2. Evidence: physical increment
`0x00680F7B` precedes marker filtering; flag initialization `0x00680F59` is
inside each invocation; EOF goes directly from `0x00680F75` to cleanup at
`0x006811D5`, then closes and sums the counts at `0x006811EC`–`0x006811FD`.

The engine lane's supplied Graphics Rules observation (45 physical reads
without a command/include boundary, among 1107 true reads including the
child) is consistent with this distinction. It is prior runtime evidence,
not a new replay measurement of these rules. Full raw-file replay still
needs implementation and a fresh differential before equality is claimed.

### 12.3 Conformance vectors

These are normative instruction-derived vectors, not new game results.
Each listed string is one successful physical ReadLine output, in order;
EOF follows the final string and adds zero. Each file starts inactive.
`empty` means a zero-byte line, and outputs shown below preserve spaces.
Use a command sink accepting ordinary lines so the vectors measure file
routing independently of command execution. Unless a child is named,
the successful file return equals the number of listed physical lines.

| ID | Physical lines | Required routing and state |
|---|---|---|
| SCR-V93 | `#<`; `plain explanation`; `#> set x y` | First two lines suppressed; line 3 ordinary payload ` set x y`; return 3 |
| SCR-V94 | `set x y #< #> set z w`; `set a b`; `#>set c d` | First two lines suppressed, including line 1's command prefix; only line 3 ordinary payload `set c d`; return 3 |
| SCR-V95 | `#>set x y #< tail`; `set a b` | Both ordinary, payloads `set x y ` and `set a b`; line 1 closes despite starting inactive; return 2 |
| SCR-V96 | `#<`; `#<`; `#>set x y`; `set a b` | First two suppressed; lines 3 and 4 ordinary; one closer suffices, no nesting depth; return 4 |
| SCR-V97 | `#<`; `#< #>`; `set x y`; `#>`; `set a b` | Lines 1–3 suppressed: line 2's closer was erased. Line 4 ordinary empty, line 5 ordinary `set a b`; return 5 |
| SCR-V98 | `#<`; `include missing.sgr`; `"(`; empty; EOF | All four lines suppressed, no include/open attempt or quote/group error; no unterminated-block error; return 4 |
| SCR-V99 | `  # <`; `set x y #>`; empty | All three ordinary payloads unchanged; spaced opener does not match and stray closer does not activate a block. Subsequent tokenization still applies section 2's `#` truncation; return 3 |
| SCR-V100 | `set x "#<"`; `#>set a b` | Line 1 suppressed despite quotes; line 2 ordinary `set a b`; return 2 |
| SCR-V101 | `# note #<`; `include missing.sgr`; `#>set a b` | First two suppressed: marker search precedes ordinary line-comment handling. No open attempt; line 3 ordinary `set a b`; return 3 |
| SCR-V102 | Parent: `#<`; `#>include child.sgr`; `set a b`. Child: `set c d`; `#<`; `ignored` | Parent line 1 suppressed; line 2 opens child, whose first line is ordinary and last two suppressed. Child returns 3 without error. Parent line 3 remains ordinary; parent physical count 3, inclusive return 6 |
| SCR-V103 | `#<`; ` #>sinclude missing.sgr`; `set a b` | Line 1 suppressed; line 2 routes the surviving suffix `sinclude missing.sgr`, optional open failure suppressed under section 4.2; line 3 ordinary. No child lines; return 3 |
| SCR-V104 | `#<`; `#>`; empty; `# comment` | Only line 1 suppressed; lines 2–4 all ordinary (empty, empty, `# comment`); return 4 |
| SCR-V105 | `#<`; `prefix #>set x y #>set z w` | Line 1 suppressed; line 2 ordinary payload `set x y #>set z w`; first closer wins, no repeat removal. Later `#` truncates tokenization; return 2 |
| SCR-V106 | `#<`; `#>set x y #< tail`; `set a b` | Line 1 suppressed; line 2 ordinary `set x y ` because its closer survives before the opener; line 3 ordinary, return 3 |
| SCR-V107 | `set x (#<)`; `# >`; `#>set a b` | Lines 1 and 2 suppressed: groups do not protect the opener and spaced closer does not match; line 3 ordinary `set a b`; return 3 |
| SCR-V108 | Registered file A: `#<`; `ignored`. Registered file B: `set x y` | A returns 2 with no ordinary calls or block error; B begins inactive, dispatches its sole line and returns 1 |
