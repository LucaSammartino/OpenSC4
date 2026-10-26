# nSCRes command classes: closure of the resource-script parser (behaviour spec)

Scope: the command and helper classes of the `nSCRes` resource-script parser that tools/coverage.py assigns as
separate owners. Their commands' behaviour is sc-resources.md §3 (dispatch, variables, macros, the built-ins); this
spec adds the bodies that spec does not cite — the parse-error constructors, `define` / `enddef`, the macro's
`AddCommand`, the block-command plumbing, the file-parser input record and the class-id function — and maps each
owner to its section, so the owners are complete.

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). The listed bodies were read in the decompile and their string literals in the PE.
Written 2026-10-25 (re-scrcmd8).

## 1. Owners and where they are specified

| Owner | Bodies | Specified in |
|---|---|---|
| `cSetCommand` | `0x00677510` | sc-resources.md §3.3 (`set`, `seti`, `setf`, `setc`, `setv3`) |
| `cArrayCreateCommand`, `cEvalCommand`, `cNamespaceCommand`, `cEndBlockCommand`, `cListCommandsCommand` | cited there | sc-resources.md §3.3 (`arrayCreate`, `eval`, `namespace`, `end`, `list`) |
| `cCreateCommand` | `0x00679650`, `0x006767E0` | sc-resources.md §3.2; the name lookup §3 below |
| `cCommandParser::cMacroDefinition` | `0x00678000` (cited), `0x00679080`, `0x00678460`, `0x006774E0` | sc-resources.md §3.2; §3 below |
| `cDefineCommand`, `cEndDefCommand` | `0x00678780`, `0x00678860`, `0x00676BE0` | §3 below |
| `cParseError` | `0x0067A180`, `0x0067A250` | §2 below |
| `cBlockCommand`, `cBlockCommandT` | `0x00676B30`, `0x00676B80`; `0x006773D0`, `0x007E3B10` | §4 below |
| `cFileParser::cInputFileSpec`, `fn:GZCOM_nSCRes_cFileParserCLSID` | `0x00680BE0`; `0x00680890` | §5 below |

## 2. Parse errors (`cParseError`) *[compat]*

The parser throws a `cParseError` (vtable `0x00AA0328`): `+0x04` the code, `+0x08` a string object (buffer
`+0x0C…+0x14`), `+0x18 := 0`. The two constructors format their message with `vsnprintf` into a process-wide
2,048-byte buffer and copy the result into the string:

- `cParseError(code, format, …)` (`0x0067A180`): the given code; buffer `0x00B469B0`;
- `cParseError(format, …)` (`0x0067A250`): code 1; buffer `0x00B471B0`.

Messages longer than the buffer are truncated by `vsnprintf`.

## 3. Macros: `define`, `enddef`, `create` *[compat]*

- **`define`** (`cDefineCommand::Parse`, `0x00678780`): with a definition already active (parser `+0x68`), throw
  `Can't nest definitions!`. Otherwise take the positional arguments (FindNonSwitchIndices with 2, effects-parser §9),
  create a macro (`0x2C` bytes, `0x00678860`: an empty name string, an empty formal-parameter list at `+0x14` and an
  empty command list at `+0x24`), make it the active definition, copy positional 0 as its name and split positional 1
  into its formal parameters (`SplitIntoArguments`).
- **While a definition is active** every command except `enddef` is recorded by `AddCommand` (`0x00679080`): a command
  whose first token is exactly `define` throws `Nested definitions not allowed`; any other is appended (its 12-byte
  argument-vector record copied) to the command list. `0x00678460` and `0x006774E0` are the list's grow and copy
  helpers.
- **`enddef`** (`cEndDefCommand::Parse`, `0x00676BE0`): without an active definition, throw
  `Found enddef without matching define`. Otherwise the macro name maps to the index the macro takes in the parser's
  macro list (`+0x80`, its current count), inserting the name or replacing an existing entry (a later definition wins;
  the earlier macro stays in the list); the macro is appended to the list and the active definition cleared.
- **`create`**: `0x006767E0` is the same insert-or-find on the macro-name map; the expansion is sc-resources.md §3.2.

## 4. Block commands *[compat]*

- **`cBlockCommand::StartBlock(parser)`** (`0x00676B30`): push this command onto the owning parser's block stack (the
  parser's slot 10 gives the owner; stack `+0x34…+0x38`).
- **`cBlockCommand::RegisterCommand(name, command)`** (`0x00676B80`): enter `name` in this block's own command map,
  inserting it or replacing the existing entry, and retain the command (its slot 2). Nested names are searched before
  the global registry (sc-resources.md §3.2).
- **`cBlockCommandT<T>`** (one folded body for the effects parser, the rule parser and the command parser):
  `Parse(args, parser)` (`0x007E3B10`) calls its own slot 5 with `(args, parser's owner)`; `EndBlock(parser)`
  (`0x006773D0`) calls its own slot 6 with the owner. The owner is the parser's slot 10.

## 5. File parser pieces *[compat]*

- `cFileParser::cInputFileSpec` (`0x00680BE0`): two empty strings (at `+0x00` and `+0x14`) and the words `+0x10`,
  `+0x24` := 0 — the record of one registered input file (graphics-rules.md §3).
- `GZCOM_nSCRes_cFileParserCLSID` (`0x00680890`) returns `0x5CF9459A`.

## 6. Conformance tests

1. `define m (a b)` … `enddef` … `define m (c)` … `enddef`: `create m (1)` expands the second definition.
2. `define m (a)` then `define n (b)`: the second `define` is being recorded, so `AddCommand` throws
   `Nested definitions not allowed` (the `Can't nest definitions!` check of `define` itself is only reached when a
   `define` is dispatched while a definition is active).
3. `enddef` with no definition throws `Found enddef without matching define`.
