| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplars4 (exemplar/cohort resource-factory replacement, data-formats §8.1) | `glue-exemplars4` / (engine lane) | in progress | - |

## Handoff
- Goal: plug-in replacement of the exemplar (0x6534284A, class 0x453429B3) and cohort (0x05342861, class 0x053429C8) factories per data-formats §8.1, behind enable=exemplars; formats_shadow keeps comparing ORIGINAL vs Rust.
- Done: worktree created.
- Next: survey SDK headers + FC4FormatsShadow.cpp + capi exstore.
