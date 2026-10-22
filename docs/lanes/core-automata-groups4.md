| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-automata-groups4 (portable group owners) | Codex / `core-automata-groups4` (engine lane, detached HEAD) | in progress | Audit published inventory, implement owners, validate and report. |

## Handoff
- Goal: published portable cSC4OccupantGroupTemplate / tKeyEffect owners only; no ABI or native glue.
- Done: owner implementation 1d9cec5a and vectors/checklist 8b00441a; audit §§1–4,7,10; 14 owner unit tests and 3 independent vectors plus negative-runner test written.
- Done: quick PASS core+conformance 3629/0; dependents compiled; CMake 4/4 edges. Earlier turbo-off runner integration failure fixed.
- Done: full gate at 8b00441a: cargo 4306/0 and ctest 67/67 PASS; clippy failed two new Release idiom diagnostics. Preserved logs in build-runs/gate-8b00441a-*.log.
- Done: Rust-1.75-compatible compare/exchange Release fix passes focused clippy with 0 diagnostics; added concurrent-release uniqueness/single-destruction test.
- Next: run final frozen-source gate after this fix, record results, final report/queue WIP pending OG3/OG4.
- Formatting: new sources formatted; required full touched-file check found preexisting root lib.rs ordering/wrapping drift. Shared root edits remain additive per lane rules; no mass reformat.
- Questions: OG3 inventory mismatch (13 published distinct bodies versus brief's 15); OG4 inherited base initial count/QI side effects.
- Primitive audit: reuse rng/seeding/ResKey; automata/script.rs is a provisional name/ID/template facade and remains untouched.
- Checklist: engine/core/src/occupant_groups/README.md (13 explicitly published distinct bodies, 15-body claim withheld pending OG3; inherited QI acquisition withheld pending OG4).
- No build/game session outstanding. q.py fails with WSL CreateInstance E_ACCESSDENIED; native rustfmt used on new source files only.
- Follow task override: no redundant pre-edit full baseline; focused quick checks then one frozen full gate.
- Main: C:/Users/tlori/FreeCity4; manifest: build-runs/run4-core-automata-groups4-r1-manifest.json in main.
- No game request or shadow/replacement claim; no subagents or branch/ref changes.
