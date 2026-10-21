| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzservices4: spec cRZLanguageUtility (locale service: names, currency, separators, driving side) | Claude / .claude/worktrees/re-rzservices4 | in progress | write docs/specs/language-utility.md |

## Handoff
- Goal: public behaviour spec for the RZ locale service `cRZLanguageUtility` (largest uncovered RZ
  class, 106 KB decomp, only an INVENTORY-M6 F6 mention) in docs/specs/language-utility.md.
- Done: worktree, lane file.
- Next: read decomp/cRZLanguageUtility.c by function (ctor 2989, Init 3608, SetLanguage 3951,
  Get* 13..2813), write spec sections.
- Facts: vtable 0x00AD1250. Not owned elsewhere: data-formats §6 covers cRZLanguageManager only.
