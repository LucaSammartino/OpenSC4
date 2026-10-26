| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-effectsres30: effects resource; kind engine | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/replace-effectsres30; detached HEAD; reserved branch replace-effectsres30 | BLOCKED EF27; effectsres not ready; ABI exports unchanged; cargo 7959 PASS, clippy 0, CTest 134/134 | 1. Answer EF27. 2. Implement visual-map insertion. 3. Restore golden baselines, measure (a)-(c), enqueue engine. |

- Both requested branches were already ancestors of the starting main HEAD; restored the prior lane's omitted engine changes in d24ab2245.
- Implemented EF26 typed renderer codecs/version selection and ordered nonempty resource sections; native values retain their allocator/ownership boundary.
- Added nonempty shake/light IO, camera counted curve, bindings/triggers, map writer traversal, in-place shrinking and root-3 dynamic retention.
- Visual-map **Read insertion remains unavailable**: the first nonterminator pair returns false and logs EF27; default-off effectsres is incomplete.
- EF27 is filed in `docs/specs/questions/effects.md`: typed native insertion arguments/ownership or complete compatible string-hash insertion contract.
- Focused checks: **6/6 PASS**, including four registration modes and return-channel review; EF26-C1 uses injected renderer dependencies to test the 86-byte decal, publication, source destruction and normalized write-back; EF26-C6 covers camera normalization. Auxiliary duplicate rows and interrupted shrinking pass.
- Corrected-source `qbuild all`: cargo **7959/7959**, clippy **0 diagnostics**, native build **29/29**, CTest **134/134 PASS**.
- Gate logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; verdict `build-runs/build-results/qbuild-20261026-182918-build-ctest.verdict`.
- Gate's tree-change note is EF27 question text only; engine tree is identical to 4dce5a8a7, the source at gate start. Initial quick build's const-pointer compile error was fixed before the passing checks.
- Native diagnostic **effectsres30-codecs-on1: clean**, 2026-04-01, foreign_exceptions=0, ref_warnings=0; native manager plus effectsres, not the house list. DLL SHA prefix 45ee85db.
- Diagnostic log: `C:/Users/tlori/FreeCity4/build-runs/game-results/effectsres30-codecs-on1.log`; line **166** shows the shipped description sections reached the explicit EF27 map refusal. Its clean verdict is not payload equivalence.
- **(a) NOT MET:** house list plus effectsres to 2032 unmeasured; complete resource Read is blocked by the refusal above. No house-acceptance verdict claimed.
- **(b) UNMEASURED:** restored-golden owner shadow and both save/load directions were not run; no unchanged/exact or cross-save claim while resource Read is incomplete.
- **(c) UNMEASURED:** gen27 Konradshohe s0-grow/s1-grow baseline/candidate pairs were not run; no differential failure claim.
- **effectsres is not ready for the replace list.** No credit/list edits and no ready enqueue; local merge-queue entry is WIP, kind engine.
