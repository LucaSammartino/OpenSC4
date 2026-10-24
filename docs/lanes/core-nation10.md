| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation10: cSC4Nation core | Codex / core-nation10 (detached HEAD) | in progress | Implement nation.md, tests and owner closure; run gates. |

## Handoff
- Goal: portable cSC4Nation core from nation.md; no capi/plugin/ABI changes.
- Worktree reused; initial status clean, no commits ahead of main.
- Read nation.md sections 1–5 and demand.md sections 8.2/11.2.
- Spec declares no nation save record. Do not invent a nation codec.
- Region filesystem methods have names but no detailed contracts; file questions before choosing behavior.
- Next: add module, owner coverage README, questions and focused tests.
- No game requests outstanding; manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-nation10-r1-manifest.json.
