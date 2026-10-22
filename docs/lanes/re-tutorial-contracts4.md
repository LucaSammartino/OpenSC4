| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tutorial-contracts4 (TU6-TU9 native contracts) | `re-tutorial-contracts4` / (RE lane, detached worktree) | in progress; docs only; no ABI export changes | Prove registry and tutorial window contracts, then cheat and filter payload |

## Handoff

- Goal: answer TU6/TU9 first, then TU7/TU8 using native export and instruction proofs; spec text only.
- Worktree reused at reserved detached HEAD; initial status clean.
- Read tutorial questions, glue-tutorial4 report and lane/merge formats.
- Done 8bcb132e: TU6 registry public proof, question moved/index regenerated, initial native ABI proposal committed (plan 8e6b5033).
- TU9/TU8 proofs added in tutorial.md4.2.2/4.4.1/4.4.2: allocator C(100), ctor/base/refcount0, no-arg Init/close, event pop4/ALtrue; vtable00A8638C slots6-11; raw FilterMessage dwords.
- Critical binding: window reads borrowed global00B20E88 as cISC4TutorialSystem base; Init calls7/12/11, task step12/11/20/11. Replacement must bind live interface; no guessed Init arguments.
- Next: finish TU7 cheat dispatch, reconcile ABI proposal, then docs validation and report/queue. No game runs requested; no runtime claim.
- Publish proposed native ABI rows only in build-runs/abi-rows-re-tutorial-contracts4.tsv.
- q.py unavailable: Windows Bash service returns E_ACCESSDENIED; use bounded PowerShell commands.
- No engine reads/edits, tools edits, ABI table edits, subagents, merge, push or ref updates.
