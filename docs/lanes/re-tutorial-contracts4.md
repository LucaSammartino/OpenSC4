| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tutorial-contracts4 (TU6-TU9 native contracts) | `re-tutorial-contracts4` / (RE lane, detached worktree) | in progress; docs only; no ABI export changes | Prove registry and tutorial window contracts, then cheat and filter payload |

## Handoff

- Goal: answer TU6/TU9 first, then TU7/TU8 using native export and instruction proofs; spec text only.
- Worktree reused at reserved detached HEAD; initial status clean.
- Read tutorial questions, glue-tutorial4 report and lane/merge formats.
- Proven: TU6 registry ECX is app slot49 result, GetItem pop4/EAX borrowed item, SetActive pop8/AL changed-state; synchronous save only on prefs-cell change. Public proof in tutorial.md 4.2.1.
- Window facts: allocator 005E55E0(100), ctor00442920 no args returns base/refcount0; Init00443E60 no stack args uses global00B20E88; primary vtable00A8638C extracted read-only from PE.
- Next: finish window slots/callbacks and publish ABI rows; then cheat/filter. Initial plan commit 8e6b5033.
- Publish proposed native ABI rows only in build-runs/abi-rows-re-tutorial-contracts4.tsv.
- q.py unavailable: Windows Bash service returns E_ACCESSDENIED; use bounded PowerShell commands.
- No engine reads/edits, tools edits, ABI table edits, subagents, merge, push or ref updates.
