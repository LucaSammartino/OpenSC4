| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-render-properties-complete4: render-properties owner contracts | Codex / `core-render-properties-complete4` (engine lane, detached HEAD) | in progress | Complete RP3-RP5, audit all owner operations, run full gate, report RP6 residuals. |

## Handoff
- Goal: implement the entire render-properties owner from the public spec, including RP3-RP5.
- Done: implementation milestone `2cead6e7`; RP3 lifetime owner (symbolic P/U, actual Box retention/free), byte/NUL lookup, foreign byte-ctype injection and byte names; independent RP3-RP5 tests and all77 NUL vectors.
- Baseline all: cargo 4238 passed, clippy clean, native build 206/206; ctest failed cargo_test after concurrent source edits and wrapper marked tree changed. This is not a clean baseline verdict; final all required after fixes.
- Done: quick PASS: core 2383/2383; dependents compiled; native build 4/4. All22 render-properties tests pass. All77 vectors independently audited against spec registry names/acronyms/defaults/extra fields. Four touched Rust files fmt PASS.
- Done: Find now explicitly returns table position; unsupported QI can omit output cell. Fixed synthetic fixture acronym expectation (reeu).
- Done: `67561d7f` completed quick validation; subsequent all passed cargo 4250/4250 and ctest 67/67 but clippy found six needless test borrows. Removed those borrows; rerun all required.
- Done: `rp-complete4-r1-core-smoke-20261022a` clean, ref_warnings=0, 2026-04-10 under turbo; DLL sha256 prefix cbb2b337. No render-properties log/shadow rows; only DLL smoke evidence.
- Next: rerun full qbuild all on unchanged source; record complete checklist and residuals; final report/queue.
- Facts: no game request outstanding; no ABI edits; RP6 live foreign ctype binding remains open. No adapter/shadow exists: smoke cannot grant owner game/coverage credit. Six-borrow fix touches tests only, so existing smoke remains valid.
- API: CLocale is explicit only; ByteCtype supplies primitive signed-int tolower/isupper at construction and query time, never original Find results. UTF-8 name convenience views may return None; byte getters are complete.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-render-properties-complete4-r1-manifest.json`.
