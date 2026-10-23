| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-legacy-control5: legacy control owner | Codex / core-legacy-control5 | in progress | Inspect existing owner modules and implement spec closure |

## Handoff
- Goal: implement the complete cGZLegacyControl core owner, foreign window/script/renderer traits, conformance tests and owner closure table; no ABI or plugin wiring.
- Done: reused existing module; implemented newly published §§9.10–9.20 bodies, foreign SC3 procedure allocation, layout facts, and recording conformance fixtures. Validation pending.
- Next: collect focused cargo test session 23766, resolve failures, audit remaining spec sections, add README closure matrix; final qbuild all.
- Evidence: q.py focused wrapper failed before cargo (Git Bash CreateFileMapping error); direct cargo test running. No game requests (core-only, no plugin wiring).
- Constraints: clean-room permitted sources only; final qbuild all required; manifest C:/Users/tlori/FreeCity4/build-runs/run4-core-legacy-control5-r1-manifest.json.
