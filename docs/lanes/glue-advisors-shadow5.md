| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-shadow5: record-only advisor shadow | Codex / glue-advisors-shadow5 | in progress | Read prior handoffs and establish the specified saved-record interface. |

## Handoff
- Goal: default-off advisor same-process record shadow; real-city exact through 2032.
- Done: implementation committed a8c61b7f; focused advisors 63/63; qbuild all Rust 6229/0 and clippy diagnostics=0.
- Native result for qbuild-20261023-195720-build-ctest: build PASS 220/220, CTest 68/69, new fixture 3 failed assertions. First difference advisor byte 6: shared ShadowOStream put_str omits the GZStr length while COut supplies payload only. Fixed locally in the owned shadow output callback; test diagnostics improved.
- Corrected source committed d2c2ea93. Second qbuild all: Rust 6229/0, clippy 0; native job qbuild-20261023-200955-build-ctest failed CMake generation: Ninja failed recompaction, Permission denied. Session 65817 terminated FAIL.
- Preserved configure failure in build-runs/fc4verify-configure-recompaction-failed.log; moved disposable .ninja_log and leftover .ninja_log.restat into build-runs/ninja-*-before-reset.txt. Source unchanged.
- Native-only qbuild verify qbuild-20261023-202127-build-ctest PASS: build 220/220 (8.2s), CTest 69/69 (76.6s). Session 29079 terminated PASS.
- Game adv-shadow5-20261023-r1-2032 completed clean to 2032-01-02, ref_warnings=0: advisors 0/73 exact, 1406 total record mismatches, min 1/max 28; final 769 records/54038 bytes, mismatches=17. Run record docs/runs/adv-shadow5-20261023-r1-2032-322.md.
- AD17/AD18 appended to docs/specs/questions/advisors.md: CitySituationAdvice factory/record contract and specified Read TEXT_DIRTY transition versus exact record-shadow acceptance. Strict mismatch retained, no core guess or normalization.
- Next: stop WIP pending contract answers; fix load date (PublishedJulian=0 prints -4713-11-24) using the city simulator date before rerunning. No operation/request outstanding.
- Initial native failure full log preserved as build-runs/fc4verify-ctest-initial-failed.log.
- No game request outstanding. qbuild logs build-runs/qbuild-{test,clippy}.log. Initial q.py failed because Bash CreateFileMapping is denied; direct cargo focused check passed.
- Graph compares system/advisor/advice/subject records; MySim records excluded by spec section 2.1, retained as identities in system map.
- Read sets advice TEXT_DIRTY by spec; comparison deliberately strict, no normalization. Event ID (not saved by advice) is seeded from original getter before Read.
- Do not attempt replacement factories or infer native signatures.
