| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-proplot-pl23 (raw profile and foundation fields) | Codex / `re-proplot-pl23` | in progress | Resolve PL23, document conformance cases, run doccheck. |

## Handoff

- Goal: close PL23 with behaviour and raw-comparison limits; no implementation edits.
- Done: verified byte writes and full copies in export assembly; added PL23 rules to both specs and four prose conformance vectors; moved PL23 to answered archive.
- Next: regenerate questions index, commit this milestone, run doccheck and questions check, finalize report and queue.
- Facts: native run vpc4-r2-native-20261022a reports 1544/1544 profile and 1908/1909 configuration raw differences. These are supplied evidence, not a new measurement.
- Read glue-dormant-shadows5 worktree lane report: no heading Owner closure or open: rows exists in its current file; owner mapping and retained PL23 section supply the closure requirement (propmgr 219/220, lotcfg 292/293). No new game run needed to answer the literal memory-write contract.
- Sources: profile byte write 004A3E52, full copy 004A3FDE; foundation fields 0047A59E–0047A609 skip +0x24, copy 0047892B–00478931 preserves it. No REA used; export sufficed.
