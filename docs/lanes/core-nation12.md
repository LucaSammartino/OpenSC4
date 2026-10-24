| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation12 — nation NQ4 closure | Codex / core-nation12 | in progress | Implement locale mapping and ten conformance vectors; all/verify gate |

## Handoff
- Goal: implement nation §6.4 and all ten §5.2 vectors; close portable cSC4Nation owner.
- Merged re-nation-nq4; resolved question-index conflicts retaining unrelated current mysims rows and adjusting totals for answered NQ4.
- Implemented host-supplied narrow/wide CRT mapping, legacy UTF-8 conversion, registry boundary comparisons and locale-aware Downloads exclusion.
- Added ten §5.2 vectors plus two boundary regressions; module README closure table covers all sections, no open owner rows.
- Formatting check passed for owner.rs and nation_owner.rs. Focused cargo tests still running (session 96473); collect this operation before starting all.
- The q.py Bash wrapper failed before cargo (CreateFileMapping Win32 error 5); preserved build-runs/q/20261024-132652-34868.log; direct cargo invocation is running.
- Next: collect focused tests, then qbuild all once on stable source and queued qbuild verify.
- No ABI, capi or plugin changes; no game request outstanding.
- Required final checks: qbuild all and queued qbuild verify; manifest run4-core-nation12-r1-manifest.json.
