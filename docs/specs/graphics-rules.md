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

## 2. Variables (`GatherInformation`, `0x007CF780`) *[compat]*

**Flags** (bit = index), named for `has` / `hasNo`:

| Bit | Name | Source |
|---|---|---|
| 0 | `hardwareDriver` | the driver is a hardware driver and `kForceSoftwareDriver` is false |
| 1 | `DXT` | current driver information record, byte `+0x2C`, nonzero |
| 2 | `fullscreen` | current driver information record, byte `+0x20`, nonzero |
| 3 | `MMX` | CPU feature bit 23 |
| 4 | `FPU` | CPU feature bit 0 |
| 5 | `backingStore` | a backing store exists (global `0x00B4E1E4` non-null, `0x007C2790`) |
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

The OS/version provider supplies CPU features, CPU MHz, screen dimensions/BPP, memory, and the first six
strings. For memory the native version helper first truncates each unsigned byte count to MiB, then rounds
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

## 3. Reading the rules (`ParseConfigFile`, `0x007CFE10`) *[compat]*

1. A rule parser (`cSC4RuleParser`) is made. When `cardVersion` is longer than 26 characters, its 4 characters at
   offset 8 and at offset 22, read as hexadecimal, are the card's vendor and device IDs.
2. The files `Graphics Rules.sgr` (the data directory) and `Graphics Options.sgr` (the user data directory) are
   given to the parser, with the symbol `prefsFirstTime` set to true or false from an application flag
   (`+0xF0C` of application slot 38) *[inferred: the first run]*. `cardIdentity` starts as a copy of `cardName`.
3. On success, the rules and option groups move into the manager, and when the card was found in a `vendor` block
   (§4.4), `cardIdentity` := `"<vendor>, <card>"`. The result is the parse's success.

`Init(properties)` (slot 3, `0x007D1890`) holds the render properties, gathers the variables and parses; when the
parse fails it installs the fallback rules (§7).

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

Group and option numbers grow the tables as needed; each option collects a property set.

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

- `partialRule` is `rule -partial`. An unnamed rule takes the previous rule's name followed by `>`.
- Each rule records its **depth** (how many rules enclose it).
- `has f` requires flag f set, `hasNo f` requires it clear (both set f in the rule's mask); `atLeast n v` requires
  number n ≥ v, `atMost n v` requires n ≤ v; `stringMatch s p` requires string s to match the wildcard pattern p
  (`WildcardMatch`, case-insensitive), `stringNotMatch` the opposite.
- Without `-any` **all** the conditions must hold (`0x007CE900`); with `-any` **one** is enough (`0x007CEA80`).
  A rule without conditions always matches, but a `-any` rule without conditions never does *[quirk]*.

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

### 4.4 Video cards

```
vendor "name" <id> [<id> ...]
   card <device id> "card name"
end
```

A `vendor` block whose IDs include the detected vendor ID enables its `card` lines; a `card` whose device ID equals
the detected one marks the card found and records its name (`0x007E4730`, `0x007E4800`).

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
