| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-query-tool5: query tool owner | Codex / core-query-tool5 (detached HEAD) | in progress | Implement section 8 and closure tests |

## Handoff
- Goal: portable query owner, city-tools-input section 8; no plugin/C ABI changes.
- Existing modules: view_input_controls supplies ControlHost, Pixel/Vec3, cursor helpers; city_input_tools contains flora/torch/situations only. No full shared base implementation exists.
- Implemented query_tool lifecycle, hover, tips, dialog walks/fillers, notifications, and published building-value helpers; closure table in module README.
- Read all section 8 subsections and query conformance requirements. Filed CT18-CT20 for conflicts and omitted contracts.
- Initial focused result: 13 passed (before extended fixtures/money/network readout changes). Expanded focused check is session 37275; collect that operation.
- Next: collect focused check, audit open prefixes, format with paths relative to engine (core/src/query_tool/*.rs), run final all gate once.
- q.py wrapper failed before cargo due Git Bash CreateFileMapping error; evidence build-runs/q/20261023-154334-6224.log. Direct PowerShell cargo works.
- qbuild fmt initially failed from engine/ prefixed paths (tool resolves paths under engine); rerun correct explicit paths. Formatter fixes applied via apply_patch from --check hunks, never stdout source rewrites.
- No game requests outstanding; core-only task.
