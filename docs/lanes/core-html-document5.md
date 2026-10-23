| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-html-document5: cHTMLDocument owner closure | Codex; core-html-document5 | in progress | Audit existing module against all owner spec sections, implement gaps and conformance tests. |

## Handoff
- Goal: close cHTMLDocument in the existing Rust module; foreign fonts/surfaces/windows; no ABI or plugin edits.
- Worktree reused at detached HEAD as directed; no branch changes.
- Done: audited §§1–10; corrected cursor resets, SELECT result, numeric conversion/FONT wraparound, one-time colours, slot-15 byte, explicit retention hooks and character-boundary scans.
- Done: direct focused Cargo run passed 21 tests; q.py failed before Cargo due to Git Bash CreateFileMapping error (build-runs/q/20261023-152144-5108.log).
- Done: owner closure README and all additional named conformance tests implemented. Open HD7 (existing) and filed HD8 (BODY loop contradicts example).
- Done: expanded focused run 25/27; corrected both fixtures after literal-spec diagnoses (build-runs/focused-evidence.md). Formatting suggestions applied with apply_patch.
- Done: corrected fixtures and extended DBCS delimiter tests; focused Cargo 27 passed, 0 failed. Explicit-path fmt passed, 3 files, 0 diff lines.
- Commits: 4acae585 compatibility fixes; 82b4c094 section conformance tests; next commit finalizes character-boundary coverage.
- Full gate attempt 1: workspace tests passed; clippy skip_while_next failed; native configure stalled at MSVC ABI detection for >10 minutes. Cancellation collected terminal exit 1 (session 33428).
- Original full logs preserved as build-runs/qbuild-test-attempt1.log, qbuild-clippy-attempt1.log and fc4verify-configure-attempt1.log.
- Next: lint correction uses find; run required corrected-source all gate. Native subprocess inspection/taskkill denied; session cancellation succeeded. Do not claim CMake/CTest pass.
- No game requests outstanding; core-only change, no plugin/capi/ABI wiring.
