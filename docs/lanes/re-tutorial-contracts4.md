| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-tutorial-contracts4 (TU6-TU9 native contracts) | `re-tutorial-contracts4` / (RE lane, detached worktree) | in progress; docs only; no ABI export changes | Prove registry and tutorial window contracts, then cheat and filter payload |

## Handoff

- Goal: answer TU6/TU9 first, then TU7/TU8 using native export and instruction proofs; spec text only.
- Worktree reused at reserved detached HEAD; initial status clean.
- Read tutorial questions, glue-tutorial4 report and lane/merge formats.
- Next: query registry 004B5D10/004B5D80, tutorial start and window constructor/close/event; inspect exact instructions for ownership and stack arguments.
- Publish proposed native ABI rows only in build-runs/abi-rows-re-tutorial-contracts4.tsv.
- q.py unavailable: Windows Bash service returns E_ACCESSDENIED; use bounded PowerShell commands.
- No engine reads/edits, tools edits, ABI table edits, subagents, merge, push or ref updates.
