| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-nation12 — nation NQ4 closure | Codex / core-nation12 | in progress | Implement locale mapping and ten conformance vectors; all/verify gate |

## Handoff
- Goal: implement nation §6.4 and all ten §5.2 vectors; close portable cSC4Nation owner.
- Merged re-nation-nq4; resolved question-index conflicts retaining unrelated current mysims rows and adjusting totals for answered NQ4.
- Next: add host-supplied narrow/wide CRT mapping, legacy UTF-8 conversion and registry boundary tests.
- No ABI, capi or plugin changes; no game request outstanding.
- Required final checks: qbuild all and queued qbuild verify; manifest run4-core-nation12-r1-manifest.json.
