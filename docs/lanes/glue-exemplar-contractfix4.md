| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplar-contractfix4 (fix exemplar replacement to answered EX46-EX49, data-formats §8.2/§10.8; EX50 caller trace) | `glue-exemplar-contractfix4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: FC4ExemplarFactory.cpp/.h match data-formats §8.2 (EX46-49 answers): slot14 DebugInstanceTag, slot15 rebuild(vector*), holder 22-24, stubs, removal reveals parent, Get borrowed/Set counted, messages, descending list. EX50: log caller return addresses.
- Done: worktree at main 2b91de8c. Baseline gate: cargo 3980/0, clippy 0, ctest 65/65. df33ed00: code per §8.2 + EX50 caller log + ContractTest (native vs ours at install, logs "exemplars: contract ... MATCH/DIFF"); filed EX51.
- Next: wait game run gecf4-r1-turbo2032 (qwait without --fresh), grep log for "exemplars:"; fix DIFFs; full gate.
- Facts: original candidate (glue-exemplars4) parked, do not touch its ledger.
