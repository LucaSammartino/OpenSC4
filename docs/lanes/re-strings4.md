| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-strings4: strings and language spec (cRZLanguageUtility first) | Claude, .claude/worktrees/re-strings4 | in progress | write docs/specs/strings-language.md |

## Handoff
- Goal: docs/specs/strings-language.md, largest exact owner family first: cRZLanguageUtility (31 fns, 11952 uncovered bytes),
  then cRZLanguageManager (41, 6312) / cRZString (50, 6063) if budget allows. Feature total uncovered 43131 bytes.
- Preflight: GO (2026-10-21). No existing questions file for this spec.
- Done: spec docs/specs/strings-language.md §1-§7 (language IDs, encoding, utility state, 4 calendar tables, currency,
  separators, flags, case/compare) from instruction walks (build-runs/emu.py).
- Next: §8 Make*String functions (00928699 date, 009284ba time, 00929379 number, 0092901a/00928c55 money).
- Facts: per-function owner list in build-runs/slnames.txt (from coverage.py --owners). coverage-map.tsv rows for the
  feature exist (lines 430-435) with spec column "-"; the coordinator sets the spec column (tools/ not edited by lane).
