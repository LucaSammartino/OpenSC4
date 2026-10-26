| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nm28 — network-manager NM14–NM16; kind docs | Codex; .claude/worktrees/re-nm28; detached HEAD, reserved branch re-nm28 | blocked; required replace-netlot28 merge has one unresolved engine/CMakeLists.txt conflict; spec answers written; no task commit yet; base 8deed0d85; ABI exports unchanged by this lane; whitespace pass, global questions check fails unrelated NT36; game n/a, no new shadow; NM14–NM16 answered | 1. Coordinator resolves/stages the forbidden engine conflict. 2. Commit prepared merge/spec changes. 3. Resolve or waive unrelated NT36 gate, rerun docs checks and publish a ready docs queue entry. |

NM14: §7.12 specifies direct tool allocation, message-target-only QI, all six Init scalars, return/stack contract, ownership, subscriptions and reentrant shutdown; worked lifecycle/countdown case.
NM15: §2.4 confirms IUnknown-only configuration QI and direct allocation; §8.6 specifies the native 8-byte set, 20-byte tree nodes/header, pointer comparator, game allocator and caller-owned references; worked fill/refill case.
NM16: §§3.2/8.5 specify crew/service lifetime, native cache clearing and manager/occupant shutdown order, balanced lot references and a worked native construction case.
At 0x005D0F94 the null object is the owned dust effect, complete construction object +0x13C. Successful SetLot creates it; Init alone does not. No cause is assigned to the separate action crash or the twelve warnings.
Moved the three original entries, including run evidence, to questions/answered/network-manager.md and regenerated QUESTIONS.md.
Evidence: main-checkout re_query export/bytes and decomp; no REA needed; no engine file was read or manually edited.
Merge of replace-netlot28 (537211219) was attempted first and stopped on engine/CMakeLists.txt. Coordinator input requested because lane rules forbid reading/editing engine/; no response received.
Validation: git diff --check -- docs/specs passed after removing the open-question file's trailing blank line. questions.py check reports only NT36 open-and-answered duplication in network-tools; those files were not changed by this merge/lane.
No new game run, cargo, clippy or ctest (spec-only work). Prior 73/73 shadow is reported question evidence, not new validation. No ABI row proposals.
Prepared commit message: build-runs/msg-re-nm28-1.txt. Merge state and all spec edits remain in this worktree pending coordinator resolution; wip queue entry must not be landed.
Next: resolve the merge conflict, commit, rerun the documentation gate and finish the ready docs handoff.
