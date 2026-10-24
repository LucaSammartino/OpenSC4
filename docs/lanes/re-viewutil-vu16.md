| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-viewutil-vu16 (native view utility ABI) | Codex / `re-viewutil-vu16` | in progress | Resolve VU16; doccheck; publish spec and queue |

## Handoff
- Goal: answer glue-viewutil-shadow11 VU16 with native call contracts and isolation rules for the 50-body family.
- Worktree reused at detached HEAD; initially clean, no main-relative commits.
- Read rules and lane format. q.py Bash wrapper fails with CreateFileMapping access denied; use local PowerShell commands with capped output.
- Imported VU16 verbatim (no answered copy existed); source report has remaining-work scope rather than an Owner closure table: rotation 1/50 compared, other 49 blocked.
- Inspected all 50 entry/return boundaries. Min/max are caller-pop four-f32; angles and ground-plane predicate are receiver-independent callee-pop helpers. Block-rounding inputs are f32.
- Done: import checkpoint `9e5892ad6`; §13 now declares all 50 call contracts, scalar captures, live-read/private-storage rules and supported image binding; §11 adds four conformance vectors and corrects vector 18.
- Native evidence: 5444 exported instructions across all 50 bodies match examined EXE bytes (0 mismatches). PE32 i386 base 0x400000, SizeOfImage 0x7C8000, timestamp 0x4C12BFB7; hash in §13.4. Helpers/evidence scripts are ignored under build-runs/.
- VU16 and collateral literal-width VU15 moved to answered; VU12/VU13 remain outside scope.
- Gate: question index regenerated; questions check PASS and doccheck check PASS. Logs: build-runs/questions-index-vu16.log, questions-check-vu16.log, doccheck-vu16.log.
- Next: commit this spec/gate milestone, replace lane status with final report and enqueue kind re / ready at actual detached HEAD.
- No game request is outstanding; no native geometry execution or numeric-mode verification was performed.
- No engine, plugin, capi, shared ABI tables, or coverage-map edits.
