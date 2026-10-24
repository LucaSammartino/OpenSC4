| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tutreg7: the tutorial registry `cSC4TutorialRegistry`, new spec tutorial-registry.md (RE spec lane) | `re-batch7` branch (outside contributor checkout, fork amarret/OpenSC4; on main `5c0ff782`, after re-tuttask7) | **ready** (docs only). tutorial-registry.md §1–§7: fields and containers, `SetLua` and the reset, `Init` and the Lua tables `tutorial_registry` / `default_regions` / `tutorials`, the completed-tutorial preferences pass, the shipped script's registrations, `GetTutorialRegionDefaultDirName`, `GetRegionalCity`, `GetNextActive`, quirks, eight conformance cases; FM2 of feature-manager.md answered; tutorial.md corrected; owner claimed: `cSC4TutorialRegistry`: specified +4,860 bytes (+19 functions) | none new |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 19 claimed functions were disassembled. The Lua key names were read from their literals (`0x00A8FFA8`–
`0x00A90014`). The shipped `tutorial_registry` script (`SimCity_1.dat`, `0xCA63E2A3`/`0x4A5E8EF6`/`0xFF7D91E7`) was
read for the registrations; nothing from it is committed beyond the keys and values the spec quotes. Written on
2026-10-23. tutorial.md (TU1, TU6) had the item layout, `GetItem` and `SetActive`; the rest of the owner had no spec.

### What changed

- New docs/specs/tutorial-registry.md.
- Read on the bytes:
  - the two maps (region directory → ordered item list; pack → default directory) and the Lua context;
  - `SetLua`, which ignores a null or unchanged context;
  - `Init`: getting the context from the script server, loading `tutorial_registry.lua` only when the global is
    missing, the `default_regions` and `tutorials` walks, the order of insertion, the completed-tutorial pass over
    the 256 preference cells and its save condition;
  - `GetTutorialRegionDefaultDirName` (pack entry, else pack 0);
  - `GetRegionalCity` (tutorial ID, else the GUID's hex digits in the city's file path);
  - `GetNextActive` (walk from the highest order, stop at a completed tutorial).
- feature-manager.md: FM2 answered (§6 and §9; the entry moved to questions/answered/feature-manager.md).
- tutorial.md: no `regions` table is read, `region_dir` is only lowercased, the script is loaded only when needed,
  the `What's New` tutorial (`0x2C12D0E9`, order 4) added to the script table, the two "still open" notes settled.
- QUESTIONS.md regenerated.

### Quirks found

- `SetLua(null)` is ignored, so the advisor shutdown leaves the registry filled and holding its context.
- The preferences save runs when the last nonzero cell is a known GUID, and is skipped when that cell was cleared.
- Of two tutorials with the same order, the later one read is placed first.
- `GetNextActive` returns null once the highest-ordered tutorial with a city is completed.
- `default_regions` and `tutorials` are walked without a table check.

### Coverage

- tools/coverage-map.tsv: the `cSC4TutorialRegistry` row now cites tutorial-registry.md.
- Specified bytes 4,548,700 → 4,553,560 (+4,860, 81.5%, `cov_local.py`, on `5c0ff782` + the nineteen lanes
  before).

### Not covered

- The shared reset `0x004B72C0` and the `defaultDirs` map internals (other owners; the reset is described).
