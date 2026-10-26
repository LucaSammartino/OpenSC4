| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fwmisc7: small framework owners (registry, extension map, default key converter, text file remainder, the C++ file-stream internals), framework-misc.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-recfile7) | **ready** (docs only). Five owners claimed: specified +4,908 bytes (+55 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 55 functions were read in the decompile. Read in the disassembly: the registry's three vtables, its `Remove` and
the initializers of its separator strings (`://`, `/`), the converter's `FileNameToKey` (every path returns false,
nothing is stored), the extension map's slots. `cRZTextFile` reuses sc-resources.md §11 and graphics-rules.md §6.1;
the `fn:GetTrueName` owner is identified as the linked STLport file-stream buffer. Written on 2026-10-24.

### What changed

- New docs/specs/framework-misc.md.

### Quirks found

- `cGZRegistry::Remove` ignores its argument and erases the lookup result for the empty key.
- `cGZRegistry::Get` scans past the requested key to the end of the map.
- The default file-name converter never produces a key.

### Coverage

- tools/coverage-map.tsv: `cGZRegistry`, `cGZRESIDtoExtensionMap`, `cGZFileNameToResKeyConverterDefault`,
  `fn:GetTrueName` → framework-misc.md; `cRZTextFile` → sc-resources.md, graphics-rules.md, framework-misc.md.
- Specified bytes 4,897,614 → 4,902,522 (+4,908, 87.8%, `cov_local.py`).
