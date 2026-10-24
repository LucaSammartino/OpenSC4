| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-viewutil-vu16 (native view utility ABI) | Codex / `re-viewutil-vu16` | in progress | Resolve VU16; doccheck; publish spec and queue |

## Handoff
- Goal: answer glue-viewutil-shadow11 VU16 with native call contracts and isolation rules for the 50-body family.
- Worktree reused at detached HEAD; initially clean, no main-relative commits.
- Read rules and lane format. q.py Bash wrapper fails with CreateFileMapping access denied; use local PowerShell commands with capped output.
- Imported VU16 verbatim (no answered copy existed); source report has remaining-work scope rather than an Owner closure table: rotation 1/50 compared, other 49 blocked.
- Inspected all 50 entry/return boundaries. Min/max are caller-pop four-f32; angles and ground-plane predicate are receiver-independent callee-pop helpers. Block-rounding inputs are f32.
- Next: finish argument/return and provider-write verification, extend spec and conformance vectors, archive VU16, run documentation gates.
- Only VU16 is assigned; existing VU12/VU13/VU15 remain outside scope.
- No engine, plugin, capi, shared ABI tables, or coverage-map edits.
