| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lotconfig-editor4 (portable lot editor, round 2/3) | Codex / `core-lotconfig-editor4` (detached reserved worktree) | ready, bounded portable module | Supervisor may publish this tip; unresolved owner contracts stay parked. |

## Change and stopping boundary

The existing portable module implements focused state/selection and geometry kernels, edit/menu dispatch, compatibility and text-dialog kernels, selector/callback kernels, and save/save-as/copy/close/provider-reference sequencing. It does not implement the native lot-editor window. Round two diagnosed no new published-contract defect and changed no engine source or tests; it recovered validation evidence and completed a stable full gate.

Source milestones: `3bda4ec3` initial kernels and independent fixtures; `1b0e8080` exhaustive audit and failure fixtures; `0161ff69bb8988ff29fa6d4a6dc00ca4eff80381` final source, including separate `RedrawRequired` and fixture nineteen. Recovery/report milestones include `ad72f681`, `c3a09bcd`, `f05ab228`, and `e98e3375`.

Typed foreign decode, persistence, widget and catalogue primitives remain separate from production editor algorithms. The host supplies no reconciliation or editor-workflow outcome oracle. Read installs successfully decoded data and returns explicit `Unsupported` at `LotConfigManager::fix configuration adapter`; Open returns `Unsupported` at PostInit before Read/New. `open_configuration_stage` does not complete Open. UI/render/title effects are pending requirements, not native implementation.

## Exhaustive body audit

The read-only reconciliation compared every address, current byte count and published C/P/O/R label in [coverage.md](../../engine/core/src/lotconfig_editor/coverage.md) against the exact published inventory: **109 bodies / 52,646 B, C44/P20/O40/R5**, all identical. Published bytes are C20,550 / P20,334 / O11,335 / R427.

| Portable scope | Bodies | Attributed body bytes |
|---|---:|---:|
| Focused kernels, within stated domains | 16 | 3,274 |
| Partial, explicit dependency barriers | 36 | 30,842 |
| Unsupported | 52 | 18,103 |
| Foreign/shared | 5 | 427 |

These bytes are attribution only, not equivalent implemented bytes, whole-owner coverage or native credit. No promotion of the owner or additional algorithms occurred this round.

## Stable final-source gate

- Source commit: `0161ff69bb8988ff29fa6d4a6dc00ca4eff80381`; engine tree **`9fcb462783577a4591727cab6a299a407f887b32`**.
- Gate start HEAD `ad72f681024d3c84aec4164aad4871540725fcd7`; end HEAD `c3b143c139cb338abceb84adbec34eb7b51d4707`. Both engine trees and the current report-only tip's engine tree are identical; no dirty engine files before/after. `fc4verify` printed `[the tree changed during the run]`; the start-to-end Git diff contains only this lane report. The engine tree remained identical, so this completed gate is retained without a documentation-only rerun.
- Exact gate command: `"C:\Program Files\Python312\python.exe" C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-lotconfig-editor4 all`.
- Actual UTC start **2026-10-22T23:21:16.920102+00:00**; actual UTC end **2026-10-23T00:10:07.921865+00:00**; qbuild exit **0**. Local start was 2026-10-23 00:21:16 +01:00.
- Cargo workspace: **5159 passed / 0 failed**, summing every one of 22 `test result:` lines; all **19/19** exact editor fixture names matched the final test source and passed.
- Clippy `--workspace --all-targets -- -D warnings`: PASS. Native configure/build and full CTest: PASS. Exact CTest outcome: **fc4verify-ctest.log: 100% tests passed, 0 tests failed out of 68**. Exact wrapper exit/PASS lines follow below.
- Required historical formatting evidence: five new `.rs` files PASS after UTF-8 stdin formatting with max_width=120. `engine/core/src/lib.rs` fmt check remains FAIL with 173 lines of pre-existing ordering drift. Initial relative-path/filename-backslash formatter errors and their fixes remain recorded; no mass formatting or new formatting claim.

```text
fc4verify build: PASS 4/4 edges (650.5 s; sccache Rust 0 hit / 3 miss)  log C:\Users\tlori\FreeCity4\.claude\worktrees\core-lotconfig-editor4\build-runs\fc4verify-build.log
fc4verify ctest: PASS 100% passed, 0 failed of 68 (999.4 s)  log C:\Users\tlori\FreeCity4\.claude\worktrees\core-lotconfig-editor4\build-runs\fc4verify-ctest.log
fc4verify: PASS (1666.1 s)  [the tree changed during the run]
test: exit=0 passed=5159 failed=0 ignored=0  (log C:\Users\tlori\FreeCity4\.claude\worktrees\core-lotconfig-editor4\build-runs\qbuild-test.log)
clippy: exit=0 diagnostics=0  (log C:\Users\tlori\FreeCity4\.claude\worktrees\core-lotconfig-editor4\build-runs\qbuild-clippy.log)

qbuild: PASS

[stderr]
```

Log hashes verified against the completed gate record, with required log modification times after the actual start:

| Log | SHA256 |
|---|---|
| `build-runs/qbuild-clippy.log` | `3544d7d99a8db39b18b5ee9da99dc8eb47e030496a49c0a8599b1bc6c642a249` |
| `build-runs/qbuild-test.log` | `266b23dee70c37a31ae2f6c3534bb8c99f9a8626c9f81b761a59f23926bf8234` |
| `build-runs/fc4verify-build.log` | `51cdfeb20864ada0280dd92e231e5c310877ccdd04f3561d65f4322aa473b214` |
| `build-runs/fc4verify-configure.log` | `4eeb8c8c23974c69dd206ba8bd48d2b2b4725d363718895c57675d6aa8b0123e` |
| `build-runs/fc4verify-ctest.log` | `1aad075b75a9d1765b42a4cb3b3ceca7b8bc428b4ef0dd77a8b629100aec1d9a` |

Full source/command/timing/exit/count/fixture/log proof: local `build-runs/editor-gate-r2-result.json`, SHA256 `0f4cd4ec50afd742ad22f7962d1a67e366e303611f21aaf8f01602c2a1e5b565`. Captured qbuild stdout SHA256 `06916e86b68f64239c4c62e8436087898b0a5e67bc3d20b401f1eb3986a9851d`. Local helper `build-runs/editor-gate-r2.py` SHA256 `72c666c82fab3cbc197d3b91a19cc4fe64d4e6556dc590361f1c5b4386f382ba`. Collection and inventory checkers passed; helpers and evidence remain local/ignored.

## Nineteen discriminating fixtures

Original new-checker command session **5512** was collected after completion: actual process exit **0**, with the same captured qbuild/native/CTest PASS and cargo/clippy exit lines above. Readiness uses those completed process outcomes and matching logs, not merely successful verdict publication.

- `callback_sorts_raw_options_and_scrolls_only_prop_first_byte`: PASS.
- `capture_keyboard_condition_equality_wrap_and_fence_mode`: PASS.
- `compatibility_clears_hidden_bits_and_truncates_before_clamp`: PASS.
- `duplicate_candidates_front_selection_and_cleanup_containment`: PASS.
- `duplicate_third_and_untested_final_candidates`: PASS.
- `fence_order_wrap_and_first_foundation`: PASS.
- `fence_preserves_extended_product_low_byte_and_rejects_invalid_conversion`: PASS.
- `hit_priority_uses_selected_then_vector_then_popup`: PASS.
- `name_missing_control_cancel_and_title_twice`: PASS.
- `path_growth_without_dirty_and_slope_limit_precedence`: PASS.
- `pointer_identity_and_deletion_do_not_normalize_state`: PASS.
- `read_failure_preserves_current_and_success_stops_at_explicit_dependency`: PASS.
- `redraw_helper_remains_distinct_from_direct_draw_and_invalidate`: PASS.
- `save_as_copy_restores_identity_but_not_dirty_or_title_and_releases_slots`: PASS.
- `save_as_failures_rollback_and_cancellation_never_install`: PASS.
- `save_failure_and_close_cancel_preserve_state`: PASS.
- `selector_duplicate_ids_preselection_current_text_append_and_cancel`: PASS.
- `separator_rows_and_direct_flags`: PASS.
- `unordered_containment_and_independent_mover_tests`: PASS.

## Recovery and preserved failures

The old `final-editor-gate.py` was recovered unchanged (SHA256 `5fcf8c897a7dfb07762239ebf53604b1490173ed4f24e8d6a97d1d8bf8766eca`), without a verdict. Its original final command handle/stdout were not exposed in the resumed transport; no old exit, STOP, cancellation cause or compiler death was inferred. The recovered old final test/native logs stopped at compilation, not a completed gate. Old test SHA256 `1338923f688dbb6793afadcaa30f1ef39d9c2192578b2b8e708e675de864991e`; old native build SHA256 `99246cad29ddb08d4f5d12e71fc016f305acacae17644992bdc0710d17bc40a6`. Text archives preserve those exact hashes under `build-runs/evidence-r2-before/`; old clippy failure SHA256 `7d43d86a3267d5e006d89310bc0bfab7457178048106b4b2703502ae7f951812`. Additional original log metadata is in `editor-gate-r2-before.json`.

Before the single new gate, the 2026-10-22 23:18:34Z snapshot queried PID/parent/creation/command identity for 67 relevant processes, zero command-identity uncertainties. Only this round's backend and inspector matched this lane; no old checker/qbuild/cargo/compiler or attributable service output remained. Some protected cwd reads were denied and were not treated as absence; limited-query command identity succeeded. No listener existed at the previously recorded lane-cache port 4871. Initial CIM access denial and missing psutil were inspection failures, recovered through read-only Windows APIs.

The new actual process chain was checker 34380 (created 23:21:16.477718Z, parent 32092), qbuild 35468 (23:21:16.950020Z, parent 34380), cargo 15244/33900 (23:21:16.987453Z / 23:21:17.034922Z), rustc 15060/36868 (23:21:17.123455Z / .127539Z), with compiler output paths in this worktree. At 23:38:40Z it had advanced to clippy cargo 35308/23404 and drivers 27356/37872, still the same qbuild. No duplicate checker was started, and no shared daemon, other lane or train105 was touched. File waits 40618/18571 returned timeout/exit1. Third combined wait/report session96048 printed timeout while its outer PowerShell conditional returned0 with no result file; that was not qbuild success. Subsequent wait shells explicitly propagate the wait exit. No timeout was credited as a gate verdict or cancellation.

The native phase was also attributable: fc4verify1636 (created 23:42:21.560568Z, parent35468), CMake23548 (23:42:24.178137Z), Ninja2364 (.226894Z), release cargo33820 (.408064Z), sccache client35092 (.452246Z). At 23:46:27Z, port4871 listener16500 (created 23:42:24.141834Z, parent19608, cwd cargo/bin) owned rustc25040 (23:42:25.772583Z, parent16500), whose opt-level=3 output was this WT's `engine/build-x86/cargo/i686-pc-windows-msvc/release/deps`. This captured the service-owned compiler despite its generic server cwd; no daemon was stopped. One malformed PowerShell Select-String regex failed during output filtering, then corrected quoting recovered the identity; it did not change any check result. Native build linked the DLL/smoke executable at 23:53:14Z. Full CTest PID19468 (created23:53:28.439922Z, parent1636) used `--test-dir <WT>/engine/build-x86 --output-on-failure -j 4`; its prescribed suite includes `cargo_test` and was observed at 20/68 before the eventual complete result above.

Preserved historical failures: the mixed-source baseline overlapped formatter/source edits and failed doctest/clippy on filename headers/backslashes (`dialogs.rs:1:1`); `baseline-failure.txt` retains the cause and UTF-8 stdin/apply_patch correction. The older full session 58904 was canceled after the redraw correction; partial results receive no final-source credit. Historical quick 3117/0 proves only its earlier seventeen fixtures and no recovered complete native outcome. The existing lib.rs formatting failure remains above. The canonicalize-path warning remains in raw logs. WSL quiet Bash had E_ACCESSDENIED; native capped reads were used. Detached-HEAD commits succeeded while Git also emitted packed-refs.lock permission diagnostics; no shared refs/config/maintenance were changed. No failed check was relabeled PASS by a helper repair.

## Questions, exclusions and next steps

Existing [LE3–LE6](../specs/questions/lot-configuration-editor.md) remain binding; no new question was filed this round. LE6 was filed in round one for raw-offset naming, unpublished constructor/path/text boundaries and the typed manager adapter. LE3 retains physical type-7 allocation/copy and numeric gaps; LE4 retains SetLotSize, foundation/wall/menu/selector gaps; LE5 retains lifecycle, input/render/title/name/3D and attribution gaps. Unknown defaults/text/conversions are not newly synthesized. Explicit unsupported paths remain in production source.

No actual game requests, owner shadow comparison, drop-in/native-window claim, ABI export change, plugin/CAPI/ABI-table/default/map/tool change, user/game-data write, or external communication. Game/shadow/ref-warning verdict: n/a for this pure in-memory module. Native CTest success is gate evidence, not native editor integration credit.

The bounded portable module is ready with the measured full gate above and the disclosed pre-existing formatter failure. The supervisor may publish this detached tip to the reserved lane branch. Remaining owner work stays parked at the published boundaries; no fourth round or family rename is requested.
