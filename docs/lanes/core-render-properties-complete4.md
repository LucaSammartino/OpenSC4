| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-render-properties-complete4: render-properties owner contracts | Codex / `core-render-properties-complete4` (engine lane, detached HEAD) | in progress | Complete RP3-RP5, audit all owner operations, run full gate, report RP6 residuals. |

## Handoff
- Goal: implement the entire render-properties owner from the public spec, including RP3-RP5.
- Done: implemented RP3 lifetime owner (symbolic P/U, actual Box retention/free), byte/NUL lookup, foreign byte-ctype injection and byte names; added independent RP3-RP5 tests and all77 NUL vectors.
- Baseline all: cargo 4238 passed, clippy clean, native build 206/206; ctest failed cargo_test after concurrent source edits and wrapper marked tree changed. This is not a clean baseline verdict; final all required after fixes.
- Next: inspect quick result, fix formatting/tests, commit validation milestone; audit all77 registry and run full all on unchanged tree.
- Facts: no game request outstanding; no ABI edits authorized; RP6 foreign ctype binding remains open.
- API: CLocale is explicit only; ByteCtype supplies primitive signed-int tolower/isupper at construction and query time, never original Find results. UTF-8 name convenience views may return None; byte getters are complete.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-render-properties-complete4-r1-manifest.json`.
