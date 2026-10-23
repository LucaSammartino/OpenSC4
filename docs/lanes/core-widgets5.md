| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widgets5: widget control owners | Codex / core-widgets5 | in progress | Inspect existing modules and implement widget-controls owner contracts |

## Handoff
- Goal: close Grid, TextEdit, ListBox, TreeView and FileBrowser against widget-controls.
- Done: 0de6813c adds ListBox/TreeView models and all family vectors; FileBrowser partial state/string/ownership layer now adds seven tests (21 total pass).
- Next: collect stable full gate, then complete FileBrowser refresh bodies/resources/lifecycle and vectors; implement Grid/TextEdit. README lists every remaining gap.
- Focused test: cargo test --manifest-path engine/Cargo.toml -p fc4-core widget_controls --lib: 21 passed, 4138 filtered.
- q.py Bash failed with CreateFileMapping access denied; use direct PowerShell cargo/qbuild. Failed evidence: build-runs/q/20261023-210713-7296.log.
- Formatting applied through apply_patch from rustfmt --check diffs, never emit-stdout. qbuild fmt needs absolute filenames.
- Filed WC4/WC5/WC6 in docs/specs/questions/widget-controls.md. Existing WC1–WC3 remain open.
- Tree native insert ordering intentionally open WC2; insert_at is semantic fixture construction, not native mapping. Root included literally (WC6).
- First full gate: 6383 tests, clippy 0 diagnostics, CMake 217/217, CTest 68/68 PASS; verifier flagged tree changed (unlinked FileBrowser file added during native queue). Full logs preserved as build-runs/pre-browser-{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log.
- Stable-source full gate next; plugin/game not used (core-only task).
- FileBrowser refresh_work currently records required calls, does NOT populate. This is incomplete work, never owner closure. Grid/TextEdit absent.
- FileBrowser read: §§1–9 and A.1, known partial §§8.2/8.3; unknown bodies stay FBQ1/2/4/5/6. Raw ctor defaults and shutdown asymmetry +0x280 preserved.
- No game requests; no ABI or plugin changes authorized.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-widgets5-r1-manifest.json.
