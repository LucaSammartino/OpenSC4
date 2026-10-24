| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotconf-glue22: capacity and cost contracts | Codex, `re-lotconf-glue22` (reserved detached worktree) | in progress | Resolve GLUE22 from original export; doccheck |

## Handoff

- Goal: close GLUE22 with named capacity lookup contract and cost getter findings, plus spec conformance cases.
- Done: read task rules, open question and landed shadow report. Existing comparison is 73/73 exact on defined fields; occupancy/cost remain unmeasured.
- Next: inspect FillBuildingInformation and its callers/callees in the original export, then verify stack/return/ownership against instructions.
- Main original sources: `C:/Users/tlori/FreeCity4`; use `tools/re_query.py`; no REA binary available.
- No engine reads/edits, ABI changes or game runs planned.
