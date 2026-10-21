| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-strings4: strings and language spec (cRZLanguageUtility first) | Claude, .claude/worktrees/re-strings4 | in progress | write docs/specs/strings-language.md |

## Handoff
- Goal: docs/specs/strings-language.md, largest exact owner family first: cRZLanguageUtility (31 fns, 11952 uncovered bytes),
  then cRZLanguageManager (41, 6312) / cRZString (50, 6063) if budget allows. Feature total uncovered 43131 bytes.
- Preflight: GO (2026-10-21). No existing questions file for this spec.
- Done: (none yet)
- Next: read decomp/cRZLanguageUtility.c by offset, write the spec sections.
- Facts: per-function owner list in build-runs/slnames.txt (from coverage.py --owners). coverage-map.tsv rows for the
  feature exist (lines 430-435) with spec column "-"; the coordinator sets the spec column (tools/ not edited by lane).
