| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widget-controls-wc7: control behavior questions | Codex; re-widget-controls-wc7 | in progress | Verify WC1-WC6 and extend the spec and vectors. |

## Handoff

- Goal: answer widget-controls WC1-WC6 using the local export, with spec rules and vectors.
- Reused the reserved detached worktree; no prior lane commits or local changes.
- Open file contains WC1-WC6; no answered/widget-controls.md exists at this base.
- Owner closure table read from the explicitly requested main-checkout README: TreeView rows 6.1-6.5 and 8 still cite WC2/WC3/WC6.
- WC6 spec rules, vectors T13-T18 and answer archive written: root flag 0x2, images 0x4, connectors 0x8; last selection anchors navigation; floor/ceil counts are derived from viewport height.
- Next: regenerate question index, commit WC6, then investigate ListBox WC4/WC5 and residual WC1.
- No REA, engine changes, ABI changes or game runs planned.
