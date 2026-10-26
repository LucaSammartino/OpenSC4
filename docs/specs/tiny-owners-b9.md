# Tiny owners: effect records, parser helpers, audio getters and factories, framework leftovers (behaviour spec)

Scope: owners of one small function each that tools/coverage.py assigns separately and that no spec closes, outside
the areas the backlog leaves out (web, development tools, drivers' blitters, the text-encoding helpers and the
specs that refuse owner credit). Three owners already specified elsewhere get rows here: `_PostEventMessage`
(audio-preferences.md, the notation paragraph), `SendFatalExceptionNotification` (app-shell-window.md §2, the
framework's ordered-participant pass, added in this lane) and `D3DXMatrixIsIdentity` (renderer-driver.md cites it;
§5 gives the body).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every body below was read in the decompile, and those with branches or x87 code in the
disassembly. Every float constant named "0.0" was read in the PE. Written 2026-10-25 (re-tinyown8).

## 1. Effect records *[compat]*

- **`cSC4EffectDescription::operator=(src)`** (`0x0058B4A0`): copy the words `+0x00` and `+0x04`, the vector at
  `+0x08` (`0x0058B050`) and the vector at `+0x14` (`0x0058B180`; both helpers are `cSC4EffectsCollection`
  vector assigns), the words `+0x20`, `+0x24`, `+0x28`, then the name string at `+0x2C` (character-range assign,
  skipped for self-assignment); return `this`. It is the value copy `GetEffectDescription` makes (effects.md,
  slot 6).
- **`cSC4ShakeDescription::operator=(src)`** (`0x00588080`): words `+0x00`, `+0x04`, two `vector<float>` assigns
  (`+0x08`, `+0x14`), words `+0x20`, `+0x24`; return `this` (effects.md, slot 14).
- **`cSC4MBParticle()`** (`0x005ADCB0`): the `cSC4EffectsParticle` base constructor, then the six floats at
  `+0x1C…+0x24` and `+0x38…+0x40` := 0.0 (`0x00B22D4C`) and `+0x44` := 0. The model-based particle record of
  effects-owners.md §1.
- **`cSC4ParticlesDescriptionBase::cTractorPoint()`** (`0x0059B450`): six floats `+0x00…+0x14` := 0.0
  (`0x00B22CC0`), `+0x18` and `+0x1C` := 0. **`cWiggle()`** (`0x0059BAD0`): six floats `+0x04…+0x18` := 0.0; `+0x00`
  is left unset. These are the elements of particles.md §2 (`+0x108` and `+0x1B8`, each two `vec3`; the readers fill
  them).
- **Class ids**: `GZCOM_cSC4EffectsManagerCLSID` (`0x0058D830`) returns `0x49822F75`,
  `GZCOM_cSC4EffectsParserCLSID` (`0x00595840`) `0xA956AD14`, `GZCOM_cSC4EffectsResourceFactoryCLSID`
  (`0x005A3B30`) `0xEA5118B5`.
- **`GZCOM_CreatecSC4EffectsParser()`** (`0x0059F860`): `GlobalHeapAlloc(0x4F4)`, `EffectsParser(this, 1)`
  (effects-parser.md); return the `cIGZUnknown` face through the virtual-base offset (`this + 4 + *(*(this + 4) +
  4)`), or null when the allocation or the constructor gives null.

## 2. Effects-parser switch helpers *[compat]*

Both take the parser's argument vector (20-byte string records) and are in effects-parser.md's anonymous namespace.

- **`ParseFloatRange(out[2], args, indices)`** (`0x005965B0`): `a := ParseFloat(args[indices[0]])`; with two or
  more indices `out := (a, ParseFloat(args[indices[1]]))`, else `out := (a, a)`. A switch with one value gives a
  zero-width range.
- **`ParseRangedFloat(arg, lo, hi)`** (`0x00597BF0`, cdecl, the value returned in ST0): `v := ParseFloat(arg)`;
  throw when `v < lo` or `v > hi`, else return `v`. The two x87 tests (`FCOM` then `test ah, 5` / `test ah, 0x41`)
  let an unordered result through, so a NaN is accepted *[quirk]*. The error is `cParseError` (code 1) with
  "argument out of range: expecting [%g..%g], got %g" (`lo`, `hi`, `v` as doubles; sc-resources-commands.md §2).

## 3. Audio getters and factories *[compat]*

- **Director getters**, each a function-local static built on first call (guard bit 0 of a flag word, then the
  constructor and an `atexit` destructor), returning its address:

  | Getter | Address | Object | Guard |
  |---|---|---|---|
  | `GetSC4AudioCOMDirector` | `0x004ECD60` | `cSC4AudioCOMDirector` at `0x00B21978` | `0x00B219E4` |
  | `GetGZSoundCOMDirector` | `0x009666C9` | `cGZSoundCOMDirector` at `0x00B635A8` | `0x00B635F4` |
  | `GetHitManCOMDirector` | `0x009361D5` | `cHitManCOMDirector` at `0x00B63328` | `0x00B63374` |
  | `GetSCLuaCOMDirector` | `0x00880180` | `cSCLuaCOMDirector` at `0x00B54298` | `0x00B542E4` |

  The result is never null (the dead branch of `StartSFX`, audio.md §6.2).
- **Factories** (`GlobalHeapAlloc`, constructor, null on either failure): `CreatecHitMan` (`0x0093584C`, 0x120
  bytes, returns the constructor's result), `CreatecHitDebugInfo` (`0x00935865`, 0x48 bytes, returns the object
  + 4), `CreatecRZRadio` (`0x004EB150`, 0x90 bytes, object + 0x1C) and `CreatecRZDiskRadioStation` (`0x004EB180`,
  0x100 bytes, the constructor's result) (radio.md, hit-music.md).
- **`cEaseSound()`** (`0x004F81D0`, called from `0x004E7459`): a 0x39-byte record, every field zero: floats
  `+0x00…+0x14` and `+0x28…+0x30` := 0.0 (`0x00B21AF0`), words `+0x18…+0x24` and `+0x34` := 0, byte `+0x38` := 0.
- **`cGroup_SC4RH::Clear()`** (`0x004F0D40`): the per-group step of `cGroupLayer::Clear` (`0x004F0DE0`,
  audio-ambience.md §8.4, "every group: peak := count, count := 0"). For each member node of the group's list
  (`+0x00`): the member record (node `+8`) is removed from the hash map it points to (`+0x18`, found by its key at
  `+0x00` in the bucket `key mod bucket count`), freed (`GlobalHeapFree`), and the node is unlinked and freed. Then
  `+0x08` (peak) := `+0x04` (count) and count := 0.
- **`cIGZSndUpdateTarget()`** (`0x0093F7EB`): an empty list: a 12-byte sentinel node linked to itself, stored at
  `+0x00`. Its three callers are the framework's lifecycle callbacks (gz-framework-core.md §2.1 slots 41–43), so
  the symbol is a folded `std::list` constructor.
- **`EnumFunc_UpdateView(sound)`** (`0x0093790B`): the callback of the Hit manager's `UpdateView`
  (hit-music.md): QI the enumerated object for `0xAA11A64F` (sound-system.md); on success call its slot 3; release
  the holder; return 1 (continue the enumeration).

## 4. `GZCOM()` (`0x0090DDF1`) *[compat]*

`RZGetCOMDllDirector()` (small-helpers2.md) then a tail jump to that director's slot 12 (`+0x30`), `GZCOM()` in
the public SDK's `cIGZCOMDirector` order *SDK*: the process's `cIGZCOM` (system-service-pointer.md uses the
result).

## 5. `D3DXMatrixIsIdentity(m)` (`0x00881970`) *[compat]*

True exactly when the 16 floats are the identity: the four diagonal elements are compared as **bit patterns**
with `0x3F800000` (1.0f), the twelve others as floats with 0.0 (`0x00A81054`), so `-0.0` off the diagonal
passes, a NaN fails, and a diagonal `1.0` only passes as the exact bits. Used by `TexStageMatrix`
(renderer-driver.md).

## 6. Conformance tests

1. `ParseFloatRange` with the switch `-rate 3` gives `(3, 3)`; with `-rate 3 5` gives `(3, 5)`.
2. `ParseRangedFloat("2", 0, 1)` throws "argument out of range: expecting [0..1], got 2"; `ParseRangedFloat("1",
   0, 1)` returns 1.
3. Two calls of `GetSC4AudioCOMDirector` return the same pointer and construct the director once.
4. `D3DXMatrixIsIdentity` of the identity with element 1 set to `-0.0` is true; with element 0 set to the bits of
   `1.0f + 1 ulp` it is false.
