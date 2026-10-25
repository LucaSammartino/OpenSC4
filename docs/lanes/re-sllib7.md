| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-sllib7: the SL global service cache and the `cSL*` helpers, sl-library.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-persistgraph7) | **ready** (docs only). Every SL global with its source (app, framework service, city or 3D-view getter), `cSLString`, `cSLBuffer`, `cSLCursor`, `cSLWinLocationSaver`, the token helpers; owners claimed: specified +6,268 bytes (+52 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 52 functions were read in the decompile. Read in the disassembly: `SL::OnView3DInit`, the end of
`SL::OnAppInit1` (its temporaries are released, so the globals are borrowed), `cSLBuffer::Load`, `cSLCursor::Load`,
the replacement loop of `cSLString::Load` and the constants it uses (UTF-8 U+2022 and U+200B), the vtable
`0x00AA6E74`. Getter names come from the public gzcom-dll headers; their slot order was checked against slots other
specs confirm (city 50, 83, 84; app 38, 42, 43, 47; buffer 3, 6, 7, 9, 10, 29, 34, 35). Written on 2026-10-24.

### What changed

- New docs/specs/sl-library.md.
- docs/specs/persistence.md §1: the `cSL*`/`SL` item links it.

### Quirks found

- `cSLString::ConvertTokens` always returns false.
- `cSLBuffer`'s colour conversion leaves the converted image without its transparent colour, and a failed
  conversion drops the image.
- `OnCityShutdown` clears `0x00B43CFC`, which nothing in SL sets.

### Coverage

- tools/coverage-map.tsv: `cSL*` and `SL` → sl-library.md.
- Specified bytes 4,869,946 → 4,876,214 (+6,268, 87.3%, `cov_local.py`).
