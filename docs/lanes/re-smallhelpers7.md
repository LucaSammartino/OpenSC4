| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-smallhelpers7: small free functions (temp names, record export, resource error box, spawning, buffer format conversion, legacy transform, scripted-window overloads) plus rows for free functions other specs already specify, small-helpers.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-picview7) | **ready** (docs only). 16 owners claimed: specified +5,325 bytes (+22 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
The seven new owners were read in the decompile and the disassembly (strings from the data section). For the nine
row-only owners the existing behaviour text was read before claiming them: data-formats.md (`RZFtoaEnglishA`,
`RZAtoUint64HexA`, `RZStrtodEnglishA`), occupant-manager.md (`SC4GetOccupantViewLevel`), crime.md
(`SC4LoadExemplarString`), network-rules.md §1, §8.1 (`NetworkNameToID`, `LoadOverrideCallback`), terrain.md
(`Expand`), auto-game-pause.md §3 (`SC4GamePause`, listed there as supported) and ui-windows.md §5.1
(`SC4CreateWindowFromScript`). Left out: `fn:SendFatalExceptionNotification` (a framework method, gz-framework-core
area), `fn:ParseRateCurveCommands` (particle effects), `fn:D3DXMatrixIsIdentity` (driver). Written on 2026-10-24.

### What changed

- New docs/specs/small-helpers.md.

### Quirks found

- `RZSpawnWait` drops its first argument unless it repeats the program, and passes an empty argv without arguments.
- `GetNewTempFilePath` leaves the reserved empty file on disk; it counts down from the current time.
- `S3DTransformLegacyLoad` overwrites the transform's change counter with a byte read last.
- `ConvertBufferFromFormatToFormat` leaves the buffer shut down when its second `Init` fails.

### Coverage

- tools/coverage-map.tsv: 16 `fn:` rows after `fn:SC4LoadString` (nine pointing at the specs listed above).
- Specified bytes 4,922,628 → 4,927,953 (+5,325, 88.3%, `cov_local.py`).
