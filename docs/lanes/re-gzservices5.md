| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzservices5 (GZ framework core specification) | Codex / detached `re-gzservices5` | ready: bounded nine-owner spec and 38 declarative vectors; doccheck passed; no engine or ABI edits | Resolve GZF1–GZF9 before claiming complete framework replacement coverage |

## Report

Added [gz-framework-core](../specs/gz-framework-core.md), covering the nine requested
owners: framework, COM, command server, misc/tool command supports, buffer, logging,
filesystem and legacy script interpreter. It records inventories and table addresses,
construction/state, reference ownership, dispatch and IO contracts, observed failure
paths, and 38 declarative conformance vectors. Existing message-server, application,
command, drawing, stream, input-control and Lua specifications remain linked authorities.

This is partial behavioral coverage across all nine owners, not a fully recovered
replacement contract for the approximately 70 KB target. Important remaining gaps are
framework callback signatures/lifecycle details, library/ID edge cases, command adapter
parameter schemas, the remaining buffer slots, logging configuration, CD validation and
script numeric/default-value behavior. These are GZF1–GZF9 in
[the question file](../specs/questions/gz-framework-core.md); the question index was regenerated.
No existing question was moved to answered. The documentation is ready to land with
those limitations explicit; a full engine replacement still needs follow-up RE.

Evidence: read-only export queries first, then bounded per-function decomp reads.
The constructor-installed framework and buffer tables were checked directly against
the executable because named export lookup did not expose the framework table.
Instructions confirmed framework Release's unsigned boundary, signed count clamping,
logging's dword recursion guard and deferred-list selection, script uint32 variant
width, and COM batch-ID dword writes. No decompiled text or assembly was copied into
the specification. REA was intentionally unused: the brief says no binary is open.

Validation: `python tools/doccheck.py check` passed; `git diff --check` passed.
Cargo, clippy and ctest are n/a for this documentation-only lane. No game runs were
requested or executed; no shadow comparison or dynamic behavior verdict is claimed.
No engine/, plugin, capi, ABI table, coverage map, README or coordinator handoff edits.
No ABI proposal file; no C ABI exports changed.

Commits: `e48b8326` (lane plan), `e884b58e` (specification and question milestone),
followed by this report and evidence corrections. Git emitted a shared packed-refs
lock permission diagnostic during commits but completed them successfully on detached
HEAD; no shared-ref or configuration changes were attempted.

## Handoff

- Done: spec/questions committed `e884b58e`; documentation gate passed.
- Queue kind is `re` as explicitly requested by this lane brief.
- Next RE pass: start with GZF1's constructor table and lifecycle helper targets in §2.1, then GZF4/GZF5 parameter schemas.
- Local ignored evidence readers are in `build-runs/read-evidence.py` and `table-evidence.py`; neither is committed.
- No outstanding game request; no implementation or native replacement claims.
- Coordinator publishes detached HEAD; lane did not move refs.
