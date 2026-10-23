| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-shadow5: record-only advisor shadow | Codex / glue-advisors-shadow5 | in progress | Read prior handoffs and establish the specified saved-record interface. |

## Handoff
- Goal: default-off advisor same-process record shadow; real-city exact through 2032.
- Done: implementation committed a8c61b7f; focused advisors 63/63; qbuild all Rust 6229/0 and clippy diagnostics=0.
- Native result for qbuild-20261023-195720-build-ctest: build PASS 220/220, CTest 68/69, new fixture 3 failed assertions. First difference advisor byte 6: shared ShadowOStream put_str omits the GZStr length while COut supplies payload only. Fixed locally in the owned shadow output callback; test diagnostics improved.
- Corrected source committed d2c2ea93. Second qbuild all: Rust 6229/0, clippy 0; native job qbuild-20261023-200955-build-ctest queued. Collect SAME session 65817 through terminal result; do not launch duplicate verify. Previous session 57613 terminated FAIL.
- Next: collect native result, then request real-city through 2032. No game outstanding. Game waiter build-runs/wait-verdict.ps1 uses filesystem events, not qwait.
- Initial native failure full log preserved as build-runs/fc4verify-ctest-initial-failed.log.
- No game request outstanding. qbuild logs build-runs/qbuild-{test,clippy}.log. Initial q.py failed because Bash CreateFileMapping is denied; direct cargo focused check passed.
- Graph compares system/advisor/advice/subject records; MySim records excluded by spec section 2.1, retained as identities in system map.
- Read sets advice TEXT_DIRTY by spec; comparison deliberately strict, no normalization. Event ID (not saved by advice) is seeded from original getter before Read.
- Do not attempt replacement factories or infer native signatures.
