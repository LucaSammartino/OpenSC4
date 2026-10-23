| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-html-document5: cHTMLDocument owner closure | Codex; core-html-document5 | in progress | Audit existing module against all owner spec sections, implement gaps and conformance tests. |

## Handoff
- Goal: close cHTMLDocument in the existing Rust module; foreign fonts/surfaces/windows; no ABI or plugin edits.
- Worktree reused at detached HEAD as directed; no branch changes.
- Done: audited §§1–10; corrected cursor resets, SELECT result, numeric conversion/FONT wraparound, one-time colours, slot-15 byte, explicit retention hooks and character-boundary scans.
- Done: direct focused Cargo run passed 21 tests; q.py failed before Cargo due to Git Bash CreateFileMapping error (build-runs/q/20261023-152144-5108.log).
- Done: drafted owner closure README; some named additional tests still to implement. Open HD7 (existing) and filed HD8 (BODY loop contradicts example).
- Next: finish additional conformance tests named in README; apply rustfmt suggestions with apply_patch, then final all gate.
- Formatting check currently reports 356 diff lines; first relative-path fmt invocation failed (use absolute file paths).
- No game requests outstanding; core-only change, no plugin/capi/ABI wiring.
