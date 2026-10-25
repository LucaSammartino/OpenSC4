| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-no33: pole/line owner RE | Codex / `re-netocc-no33` (RE, detached HEAD) | ready; NO33/NO43 answered, NO13 supplemented; 13 new spec conformance cases; questions/doccheck passed | Implement published pole/line contracts; no engine changes in this RE lane |

Completed network-occupants.md §§4.1–4.3/4.6–4.7 and §5.1, building on behavior commit `714e50072`.
Native allocation: 0x124-byte blocks, 112 per 32-KiB pool, full marker 0x40000000, rejoin at 84 live.
Published connection copy/removal/refresh ownership, cell walks, output/transform/highlight rules, model replacement/failure, zoom, Pick, Draw, wire geometry and lifetime.
Diagonal floor uses double 0.78539001941680908203125, f32 cosine/sine 3F350554/3F350493, and an x-only intermediate f32 store; translated z fixture yields B4400000.
Line Shutdown virtually clears both pole refs and forwards object+4 to base Shutdown; live offsets and equal-pointer reference behavior are explicit.
Added conformance cases 62–74 (13 cases), retaining earlier constructor/geometry/bounds cases 43–44/54–55.
Moved NO33/NO43 into docs/specs/questions/answered/network-occupants.md; supplemented the existing NO13 entry without duplicating it; regenerated QUESTIONS.md.
Evidence: main export/decomp plus instruction bytes and executable constants; REA was not needed. No decompiled output or game files committed.
Validation: questions index/check and doccheck check passed; git diff whitespace check passed. Cargo/clippy/ctest and game runs n/a (spec-only).
The q.py Bash wrapper failed before checks with Win32 CreateFileMapping error 5; the Python fallback completed the actual gate. Logs are build-runs/{questions-index,questions-check,doccheck}-re-netocc-no33.log.
No C ABI exports or docs/abi rows changed; no ABI proposal file. No engine/, plugin, capi or coverage-map edits.
Remaining: NO42's saved +110 meaning stays open; storage behavior is specified. Implementers must supply the documented valid geometry/texture/property preconditions; no engine closure or game verdict is claimed.
