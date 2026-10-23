| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widgets5: widget control owners | Codex / core-widgets5 | in progress | Inspect existing modules and implement widget-controls owner contracts |

## Handoff
- Goal: close Grid, TextEdit, ListBox, TreeView and FileBrowser against widget-controls.
- Done: inspected existing layers; new widget_controls module models ListBox/TreeView state and all family vectors (14 tests pass).
- Next: close remaining specified ListBox/TreeView operations/tests; implement incorporated Grid/TextEdit/FileBrowser contracts. README is an honest progress closure table.
- Focused test: cargo test --manifest-path engine/Cargo.toml -p fc4-core widget_controls --lib: 14 passed, 4138 filtered.
- q.py Bash failed with CreateFileMapping access denied; use direct PowerShell cargo/qbuild. Failed evidence: build-runs/q/20261023-210713-7296.log.
- Formatting applied through apply_patch from rustfmt --check diffs, never emit-stdout. qbuild fmt needs absolute filenames.
- Filed WC4/WC5/WC6 in docs/specs/questions/widget-controls.md. Existing WC1–WC3 remain open.
- Tree native insert ordering intentionally open WC2; insert_at is semantic fixture construction, not native mapping. Root included literally (WC6).
- Full gate not yet run; plugin/game not used (core-only task).
- No game requests; no ABI or plugin changes authorized.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-widgets5-r1-manifest.json.
