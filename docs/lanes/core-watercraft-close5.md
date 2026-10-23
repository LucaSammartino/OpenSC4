| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close5 (watercraft owner closure) | Codex / `core-watercraft-close5` (engine lane) | in progress | Audit watercraft spec and existing core; implement gaps and owner closure tests. |

## Handoff
- Goal: close cSC4Watercraft against all watercraft.md sections, core only; no ABI changes.
- Done: 432e5530 audited all watercraft sections; added literal §6.2 commands and geometry-aware dock/undock state dispatch; owner README maps all sections and five public vectors.
- Focused cargo test: 35 passed, 0 failed; log build-runs/q/20261023-115009-3052.log.
- New question WC8 records summary arithmetic choices; existing WC3–WC7 still block full closure.
- Added gear storage, slowing-within-gear braking rate, and four regressions for towing/following, stillness/backwards time and inherited AEMS commands.
- Formatting passed for five owned Rust files. Stable-source qbuild all is running as exec session 69040; collect that same session to termination, do not duplicate it.
- Next: collect full gate, record verdict/log paths, finalize report and WIP queue (owner remains open).
- No game requests outstanding. No plugin/capi/ABI changes; full owner closure cannot be claimed.
