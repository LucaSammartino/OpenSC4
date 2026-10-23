| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close5 (watercraft owner closure) | Codex / `core-watercraft-close5` (engine lane) | in progress | Audit watercraft spec and existing core; implement gaps and owner closure tests. |

## Handoff
- Goal: close cSC4Watercraft against all watercraft.md sections, core only; no ABI changes.
- Done: audited all watercraft sections; added literal §6.2 commands and geometry-aware dock/undock state dispatch; owner README maps all sections and five public vectors.
- Focused cargo test: 35 passed, 0 failed; log build-runs/q/20261023-115009-3052.log.
- New question WC8 records summary arithmetic choices; existing WC3–WC7 still block full closure.
- Next: add missing regression assertions named in README; fmt check explicit owned files, then stable-source qbuild all once.
- No game requests outstanding; no full gate run. No plugin/capi/ABI changes; full owner closure cannot be claimed.
