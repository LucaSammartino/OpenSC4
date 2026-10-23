| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-varfeat-lines5: standard variant and feature shadow lines | Codex / `glue-varfeat-lines5` | in progress | Add dated log lines; native gate and real-city run to 2032 |

## Handoff
- Goal: retain existing variant/feature logs and add standard dated exactness lines.
- Done: added reporting-only dated variant case-group and feature sample/totals lines; original logs retained. Refusals and feature lifetime differences count as mismatches. git diff --check passed.
- Next: run final qbuild all and queued native verify, then request a turbo city run to 2032 with both keys.
- Date is read from the current city simulator through existing interfaces; published date is a fallback.
- Do not read or execute tools/. Native checks via lane qbuild verify; game via queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-varfeat-lines5-r1-manifest.json.
