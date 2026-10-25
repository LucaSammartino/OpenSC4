| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tokens7: the default detokenizer's built-in tokens and the advice token methods, advice-tokens.md; the UTF-16 helpers claimed for drawing-fonts.md (RE spec lane) | `re-batch8` branch (outside contributor checkout, fork amarret/OpenSC4; rebased on main `4aa46040`, after re-autorec7) | **ready** (docs only). `City`/`Mayor`/`Year`/`Population`/`Region`, the advice method's fixed names (links, mission values, advisor name, building costs), the neighbour-deal tokens; drawing-fonts.md gains `UTF16GetThaiCharLevel`; specified +7,334 bytes (+17 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All functions of the three token owners were read in the decompile and in the disassembly, with the static token
strings (their start-up constructors and literals) and the format literals. `UTF16GetThaiCharLevel` (`0x00911A40`)
was read in the disassembly with its two jump tables; it matches text-edit-encoding.md's table (which states it is
not owner coverage). Every other `fn:UTF16*` function was already described in drawing-fonts.md. Written on
2026-10-24.

### What changed

- New docs/specs/advice-tokens.md.
- docs/specs/drawing-fonts.md: the Thai level rule after the class table.
- docs/specs/property-sets.md §5.2: the bool comparison is a case-sensitive `CompareTo`.

### Quirks found

- `mission_mayor_rating` passes both aura outputs at one float; `mission_money` prints only the low 32 bits.
- The `Region` token does not check for a missing active region.

### Coverage

- tools/coverage-map.tsv: `cSCStringDetokenizerDefault` (before `cSCStringDetokenizer*`),
  `fn:gAdviceDetokenizerFunction`, `fn:gNeighborDealAdviceDetokenizerFunction` → advice-tokens.md; `fn:UTF16*` →
  drawing-fonts.md.
- Specified bytes 4,880,227 → 4,887,561 (+7,334, 87.5%, `cov_local.py`).
