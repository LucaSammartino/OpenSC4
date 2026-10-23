| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widgets5: widget control owners | Codex / core-widgets5 | in progress | Inspect existing modules and implement widget-controls owner contracts |

## Handoff
- Goal: close Grid, TextEdit, ListBox, TreeView and FileBrowser against widget-controls.
- Done: 0de6813c adds ListBox/TreeView models and all family vectors; FileBrowser partial state/string/ownership layer now adds seven tests (21 total pass).
- Next: FIRST collect existing qbuild operation with write_stdin session_id=81518 (60-second waits); do NOT launch another build. Then finish FileBrowser refresh bodies/resources/lifecycle/vectors; implement Grid/TextEdit. README lists remaining gaps.
- Focused test: cargo test --manifest-path engine/Cargo.toml -p fc4-core widget_controls --lib: 21 passed, 4138 filtered.
- q.py Bash failed with CreateFileMapping access denied; use direct PowerShell cargo/qbuild. Failed evidence: build-runs/q/20261023-210713-7296.log.
- Formatting applied through apply_patch from rustfmt --check diffs, never emit-stdout. qbuild fmt needs absolute filenames.
- Filed WC4/WC5/WC6 in docs/specs/questions/widget-controls.md. Existing WC1–WC3 remain open.
- Tree native insert ordering intentionally open WC2; insert_at is semantic fixture construction, not native mapping. Root included literally (WC6).
- First full gate: 6383 tests, clippy 0 diagnostics, CMake 217/217, CTest 68/68 PASS; verifier flagged tree changed (unlinked FileBrowser file added during native queue). Full logs preserved as build-runs/pre-browser-{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log.
- Current full gate on d5e44377: Rust 6390 passed, 0 failed/ignored; clippy 0 diagnostics. Native queued as qbuild-20261023-213040-build-ctest; still running at context stop, terminal result NOT collected. Full logs build-runs/qbuild-test.log and qbuild-clippy.log; source has remained unchanged since starting this gate.
- fmt: PASS all 5 owned Rust files. No game requests, no plugin/capi wiring, no ABI changes. No merge-queue entry: incomplete scope, continue required.
- FileBrowser refresh_work currently records required calls, does NOT populate. This is incomplete work, never owner closure. Grid/TextEdit absent.
- FileBrowser read: §§1–9 and A.1, known partial §§8.2/8.3; unknown bodies stay FBQ1/2/4/5/6. Raw ctor defaults and shutdown asymmetry +0x280 preserved.
- Grid review (not implemented): read §§5.9–5.14 plus much of §§1–2/5.1–5.8. Still read full slot/flag inventory and missing section prefixes before implementing.
- Grid facts: ordered selection; mode controls removal matching, not additions; AddCellToSelection always recalculates. Page API = 4 cells, keyboard page = visible float count. Empty arrow anchor (0,1).
- Grid layout: context-wide depth <=2, pause can go negative, paused recalc clears dirty; partial visible fraction included only abs(f)>0.05f. Horizontal always-wanted bar deliberately tests ROW limit/flag.
- Grid cells: attach owns a reference; copy retains bitmap/font, drops extra data and cannot copy a window; Place adds final SetArea, PlaceAll does not. Paint forces pause=0 and uses separate owner-draw protocol.
- Grid pending paragraphs need a numbered WG5 question before closure: horizontal text alignment (§5.12) and format-copy size words (§5.14). Existing questions file only WG3/WG4; QUESTIONS.md says next WG5. No Grid question filed yet because no Grid code exists.
- Guard at stop: 149531/150000 before this final handoff; preserve pending native operation across continuation. Prepared commit message: build-runs/msg-core-widgets5-3.txt.
- No game requests; no ABI or plugin changes authorized.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-widgets5-r1-manifest.json.
