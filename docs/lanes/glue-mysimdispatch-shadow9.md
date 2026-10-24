| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysimdispatch-shadow9: dispatch shadow | `Codex` / `glue-mysimdispatch-shadow9` (engine lane) | in progress; contracts under inspection | 1. Implement settled dispatch comparisons. 2. Gate and measure to 2032. |

## Handoff
- Goal: default-off mysimdispatch_shadow; independently compare settled dispatch contracts through 2032 with house switches.
- Worktree reserved at detached HEAD 67a23d0fb; commit normally, supervisor publishes branch.
- Done: inspected settled contracts and public SDK; implemented default-off constructor/header-only shadow, private independent Rust comparator, monthly capture and field counts.
- Next: focused tests/format, stable full gate and queued native verify, house run to 2032. Actual live-trip replay is blocked by missing native callable/input-observation ABI (MS20).
- Compared scope: fresh native constructor state/previous/flags/minutes/travel, class/version; discoverable live serialized class/version only. Native outputs never initialize expected state. This does NOT verify the whole owner.
- Filed MS20 in docs/specs/questions/mysims.md. Coordinator must regenerate QUESTIONS index (tools forbidden here).
- MS18 remains partial: explicitly inventory uncompared methods; do not infer missing native signatures.
- Milestone cdaea5883: scoped shadow implemented. Focused fc4-capi comparator tests pass 2/2. q.py could not start Git Bash (mapping access denied); direct cargo worked. Applied explicit fmt fix and calendar-month deduplication.
- No game requests outstanding. Next operation: stable qbuild all, then queued native verify if all does not already dispatch it.
