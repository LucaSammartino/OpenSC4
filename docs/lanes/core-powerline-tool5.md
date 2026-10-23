| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-powerline-tool5: power-line tool owner closure | Codex; core-powerline-tool5 | in progress | Audit published owner contract and existing implementation. |

## Handoff
- Goal: close cSC4PowerLineTool in network_tools, core only; no ABI or plugin edits.
- Worktree reused at detached HEAD; initial status clean.
- Existing implementation is split across power_line.rs and power_line/; no module README exists yet.
- Audited power-line-tool sections 1-9: missing portable type-5 regional Attempt/Place; callbacks currently hide them. Existing NT18/NT22/NT24/NT25/NT26 remain open.
- Next: implement regional algorithm with foreign service primitives and add PL-RA vectors, then finish section-to-test table.
- Coverage-map path is forbidden by clean-room rules; discover owner through allowed specs instead.
- No game requests outstanding; foreign manager/terrain/renderer remain traits.
