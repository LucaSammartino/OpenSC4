| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-streams4: clean-room Rust core for the output/input stream owners (output-streams spec) | Claude lane / .claude/worktrees/core-streams4 | in progress | implement engine/core/src/streams, tests, gate |

## Handoff
- goal: portable Rust core for cRZOStream, cRZTextOStream, cRZNullOStream, cRZOFStream sink, cRZIStream, cRZNullIStream per docs/specs/output-streams.md; tests; no plugin/CAPI.
- done: lane file.
- next: read spec sections 0-9, write engine/core/src/streams/mod.rs.
