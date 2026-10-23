| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widgets5: widget control owners | Codex / core-widgets5 | in progress | Continue Grid owner orchestration, implement TextEdit, finish browser and lifecycle audit |

## Handoff
- Round 3 active (2026-10-23), manifest run4-core-widgets5-r3-manifest.json; detached HEAD is intentional.
- Previous native verdict qbuild-20261023-225614-build-ctest collected again: PASS 159/159, CTest 68/68; no operation outstanding.
- Next: implement TextEdit byte encoding/state/mutations/history with distinguishing vectors, then update closure rows honestly, final stable-source qbuild gate and READY queue (partial work expressly authorized).
- Goal: close all five owners against widget-controls; incomplete, no ready/merge-queue claim.
- Round 1 commits: 0de6813c ListBox/TreeView, d5e44377 Browser state. Round 2 a88ea3be adds Browser population/resource branches; this checkpoint adds Grid state helpers.
- Collected old qbuild-20261023-213040-build-ctest.verdict: native PASS 59/59, CTest 68/68, marked tree changed.
- Current all operation collected to terminal: qbuild-20261023-225614-build-ctest, qbuild PASS; Rust 6396 passed/0 failed/0 ignored, clippy 0 diagnostics, native 159/159, CTest 68/68. No outstanding operation.
- Full logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log; exact native verdict in build-runs/build-results/qbuild-20261023-225614-build-ctest.verdict.
- Full gate ran on unchanged implementation source before final formatting-only width correction; do not call it an exact final-tip gate.
- Focused suite: 27 passed, 4138 filtered. Includes all six Grid vectors; browser adds FB-C19–C21/C25–C27.
- fmt initially failed (1175 diff lines): helper used default width 100 instead of engine/rustfmt.toml width 120. Applied width-120 patches; qbuild fmt PASS five explicit owned files, zero diff.
- Helper build-runs/format-owned.py reads via stdin and emits apply_patch diffs; never writes rustfmt stdout to source. Use python -X utf8 and apply via tool; skip empty patches.
- Failed focused compile extra brace corrected; evidence build-runs/grid-initial-compile-failure.txt. Width failure evidence build-runs/fmt-width-failure.md.
- FIRST next: read module README progress table, finish Grid dispatch/recalculation hooks and bars, input, paint and data/format wrappers. Existing methods are raw state helpers, not complete native slots.
- Grid remaining known work: init/lifecycle adapter; recalc hooks even no-change selection; bar creation/removal/ranges/placement/second layout pass; mouse/hit/key/focus; editable text/find/extras/format/delete-copy wrappers; paint. ClearCell native return is true, helper currently reports deselection.
- Grid provisional dependencies documented and asked as WG5; initialization/text alignment/format-copy discrepancy as WG6 in docs/specs/questions/win-grid.md. Do not treat provisional helpers as proved behavior.
- Grid facts: selection removal matches mode; additions exact ordered pairs. API pages=4, keyboard pages=visible count; empty arrow anchor=(0,1). Pause may go negative; paused recalc clears dirty; depth is context-wide, max 2; fractions only abs(f)>0.05f.
- Grid ownership: bitmap/font copies retain; window copies leave empty; extra not copied. Attach owns one ref. Place calls final SetArea, PlaceAll omits it. Horizontal always-wanted bar test deliberately uses ROW limit/flag. Paint forces pause=0.
- TextEdit absent. Read all incorporated text-edit/encoding/layout/adapters/inventory specs before implementing; read so far: identity and byte replacement/selection/character prefixes, raw history domains and eviction, first layout navigation and encoding/adapters prefixes (not complete).
- TextEdit facts: byte offsets and reversed endpoints; SetText count 0=strlen; selectors 8/100 truncate to finish character (may exceed max); insertion history evicts floor(n/4), limit 0 does not disable; deletion history has no eviction; undo replays raw bytes and 5 state words.
- Browser populate_files executes proved enumeration/filter/sort/cell work with resolved column/icon inputs. refresh_work still records automatic refresh, so automatic directory/file population/lifecycle owner remains incomplete.
- Browser remainder known: AddControls factory/fallback prefix, Init/paint/property dispatch, resource success branches not covered by FBQ1; open FBQ1/2/4/5/6 recipes/sizing/restoration/unproved bodies remain Unsupported. Keep asymmetric shutdown omission +0x280.
- ListBox/TreeView family vectors all represented; audit remaining specified default/resource/lifetime branches and deepen tests. Native insert sentinel mapping WC2, root WC6 and drawing WC1/WC3/WC5 remain explicitly open.
- Existing WC4/WC5/WC6 questions retained; new WG5/WG6 appended only to question file. Coordinator regenerates question index (tools forbidden to engine lane).
- No game requests, plugin/capi wiring, ABI changes, generated ABI entry edits or merge-queue entry. Core-only work remains unfinished.
- Guard last observed 135309/150000 before final checkpoint; context continuation required rather than ready. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-widgets5-r2-manifest.json.
