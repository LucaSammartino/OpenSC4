| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation10: nation owner contracts round 2 | Codex / core-nation10 (detached HEAD) | in progress | Implement nation §§6–8 and run final gate. |

## Handoff
- Goal: implement NQ1–NQ3 answers in engine/core/src/nation; READY queue.
- Done: merged answer branch re-nation-nq3; resolved open-question add/add and additive core module conflict.
- Prior implementation: 0920be534; prior full gate recorded in 6d399b153.
- Next: replace process tuning cache/lifecycle and filesystem Unsupported methods using nation §§6–8.
- Source: docs/specs/nation.md §§2.1, 5.1, 6–8; answered/nation.md NQ1–NQ3.
- No game request outstanding; portable core has no plugin wiring/shadow.
- Do not run tools/questions.py: clean-room prohibition; merged index already reflects answered entries, verify counts later.
