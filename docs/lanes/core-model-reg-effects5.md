| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-reg-effects5 (model effects owner closure) | Codex / core-model-reg-effects5 | in progress; implementation complete, first gate diagnosed | Repeat full gate after formats adapter correction |

## Handoff
- Goal: close cSC4ModelRegEffects in core only; no ABI/plugin changes.
- Done: initial handoff 4444173e; audited graphics-formats 3.11–3.11.4 and 8.5.
- Done: extended existing owner for lifetime/controls/borrowed records/nulls/failure/flags/motion; added module README closure table.
- Implementation milestone: 6b1e4e59.
- Focused result: fc4-core model_reg_effects 12 passed; conformance model_reg_effects 1 passed; fmt 4 explicit files clean.
- Existing REGP parser reused unchanged, integration covers vector 37 and owner vectors 48–49.
- No new questions; old GF19–GF24 questions are answered by current spec prose (left untouched).
- First full gate: workspace/clippy found 11 stale API errors in formats/tests/model_reg_effects.rs; corrected adapter now passes its focused test (1/1), fmt clean.
- First verify: CMake ABI detection stalled without an object for 8+ minutes; stopped this lane's cmake PID 6220, collected original operation to terminal FAIL (513.3s). No CTest ran.
- Failed full logs preserved in build-runs/failed-gate-1/{qbuild-test.log,qbuild-clippy.log,fc4verify-configure.log}.
- Next: repeat qbuild.py --wt <WT> all on corrected stable source; collect SAME operation to terminal result.
- Failed evidence: first q.py cargo invocation failed before compilation (Bash CreateFileMapping denied); first direct cargo compile found E0597 test fixture lifetime, corrected by declaring borrowed records before owner.
- fmt initially used relative paths (not found); absolute paths work. Formatting applied from rustfmt --check diffs via apply_patch, no source redirection.
- Worktree is reserved and detached; commit normally, do not switch refs.
- No game requests outstanding; core-only task has no shadow hook.
