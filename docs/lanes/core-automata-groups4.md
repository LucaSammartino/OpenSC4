| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-automata-groups4 (portable group owners) | Codex / `core-automata-groups4` (engine lane, detached HEAD) | in progress | Audit published inventory, implement owners, validate and report. |

## Handoff
- Goal: published portable cSC4OccupantGroupTemplate / tKeyEffect owners only; no ABI or native glue.
- Done: audited §§1–4,7,10; separate portable owner implementation and 11 boundary/lifetime/thread tests written; focused quick check running.
- Next: collect quick result, correct any failures, add independent conformance vectors and exact per-owner checklist.
- Questions: OG3 inventory mismatch (13 published distinct bodies versus brief's 15); OG4 inherited base initial count/QI side effects.
- Primitive audit: reuse rng/seeding/ResKey; automata/script.rs is a provisional name/ID/template facade and remains untouched.
- Current quick session 92551; q.py fails with WSL CreateInstance E_ACCESSDENIED, so use native PowerShell for non-build formatting.
- Follow task override: no redundant pre-edit full baseline; focused quick checks then one frozen full gate.
- Main: C:/Users/tlori/FreeCity4; manifest: build-runs/run4-core-automata-groups4-r1-manifest.json in main.
- No game request or shadow/replacement claim; no subagents or branch/ref changes.
