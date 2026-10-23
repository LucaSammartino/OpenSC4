| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-reg-effects5 (model effects owner closure) | Codex / core-model-reg-effects5 | in progress; implementation and focused tests complete | Run the stable-source full gate |

## Handoff
- Goal: close cSC4ModelRegEffects in core only; no ABI/plugin changes.
- Done: initial handoff 4444173e; audited graphics-formats 3.11–3.11.4 and 8.5.
- Done: extended existing owner for lifetime/controls/borrowed records/nulls/failure/flags/motion; added module README closure table.
- Focused result: fc4-core model_reg_effects 12 passed; conformance model_reg_effects 1 passed; fmt 4 explicit files clean.
- Existing REGP parser reused unchanged, integration covers vector 37 and owner vectors 48–49.
- No new questions; old GF19–GF24 questions are answered by current spec prose (left untouched).
- Next: qbuild.py --wt <WT> all once on this stable source; collect SAME operation to terminal result.
- Failed evidence: first q.py cargo invocation failed before compilation (Bash CreateFileMapping denied); first direct cargo compile found E0597 test fixture lifetime, corrected by declaring borrowed records before owner.
- fmt initially used relative paths (not found); absolute paths work. Formatting applied from rustfmt --check diffs via apply_patch, no source redirection.
- Worktree is reserved and detached; commit normally, do not switch refs.
- No game requests outstanding; core-only task has no shadow hook.
