| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotconf-glue22: capacity and cost contracts | Codex, `re-lotconf-glue22` (reserved detached worktree) | in progress; GLUE22 resolved; doccheck PASS | Commit spec milestone, finalize report and enqueue |

## Handoff

- Goal: close GLUE22 with named capacity lookup contract and cost getter findings, plus spec conformance cases.
- Done: initialization committed as `206aa3502`; GLUE22 answered in spec §§4.1–4.2, conformance cases 15–19; question moved to answered and index regenerated.
- Verified: BDS IID 0x07813EAE, slot 39 at 0x0069CA60 returns borrowed profile address in EAX, pop 4; capacity uint16 +8. Slot 40 is nullable/non-inserting. Configuration getter results AX only.
- Cost finding: complete 34-slot configuration vtable has no cost getter; profile +0x30/+0x38 are per-type plop/budget costs, no configuration-family aggregate.
- Validation: `python tools/doccheck.py check` PASS; `git diff --check` PASS. Five prose conformance vectors added; no executable tests or game runs.
- Next: commit milestone, replace with final Report + short Handoff, run doccheck and enqueue kind re.
- Existing shadow remains 73/73 exact on defined fields only; this RE result supplies no new dynamic measurements and does not measure occupancy/cost.
- Main original sources: `C:/Users/tlori/FreeCity4`; use `tools/re_query.py`; no REA binary available.
- No engine reads/edits, ABI changes or game runs planned.
