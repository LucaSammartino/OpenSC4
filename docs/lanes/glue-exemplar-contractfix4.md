| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplar-contractfix4 (fix exemplar replacement to answered EX46-EX49, data-formats §8.2/§10.8; EX50 caller trace) | `glue-exemplar-contractfix4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: FC4ExemplarFactory.cpp/.h match data-formats §8.2 (EX46-49 answers): slot14 DebugInstanceTag, slot15 rebuild(vector*), holder 22-24, stubs, removal reveals parent, Get borrowed/Set counted, messages, descending list. EX50: log caller return addresses.
- Done: worktree at main 2b91de8c.
- Next: read FC4ExemplarFactory.cpp, baseline `qbuild all`.
- Facts: original candidate (glue-exemplars4) parked, do not touch its ledger.
