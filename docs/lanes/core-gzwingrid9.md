| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzwingrid9 (cGZWinGrid round 2) | Codex / core-gzwingrid9 | ready; ABI unchanged; Rust 7039/0, clippy clean, CTest 80/80 | Coordinator publish detached tip and land; portable owner fully implemented |

## Implementation and owner closure

Merged `re-wingrid-wg9` at 825b81f96. Removed duplicate open WG5–WG9 entries,
retaining WG3/WG4, and regenerated the question index with the explicitly
requested local `python tools/questions.py index`. The initial invocation of
the main-checkout script failed on its main-checkout output path; it changed
no main files. The local invocation succeeded.

Implementation commit 44e74bfa6 replaces all provisional WG5–WG9 paths:
- Signed/unlimited bounds independent of fixed flags; format-only used ranges;
  mode-dependent selection membership; viewport-filling relative scrolling.
- Field-by-field default/row/column/cell formatting; sentinel construction;
  host RGB/font initialization; axis-format copies reset both size words.
- Terminal editor-allocation fault after clearing the old cell; acquired
  editor-window interface balanced separately from the created editor.
- Partial cell-format and plain-extra setters; grow-only window-width resize;
  sparse stable SortColumn/SortRow, including the selection old-row quirk.
  Null comparator uses host CRT CTYPE byte comparison; C-locale helper supplied.
- Text draw flags, cross-field vertical placement, corrected color slots,
  header rectangle quirks, and whole-cell-area right-edge extension for text
  and bitmap clipping while retaining bitmap alignment within its cell.

All 19 new spec vectors are individually named tests in `grid/vectors.rs`.
The owner closure table is `engine/core/src/widget_controls/grid/README.md`.
**The cGZWinGrid portable owner is now fully implemented against the current
spec**, including all 198 inventory functions through owner methods or equivalent
Rust container operations. This is a portable-core closure claim; there is no
new C ABI export, native vtable, renderer adapter, or in-game parity claim.
WG3 delayed cell gutters and WG4 extra-object storage quirks remain preserved.

## Validation

- Focused corrected snapshot: 73/73 widget tests, including all 19 new vectors.
  An earlier 69/73 snapshot exposed four assertions of superseded provisional
  behavior; they now assert settled row membership and text-color slots.
- Final eight-file absolute-path `qbuild fmt`: PASS, zero diff lines.
  Formatting fixes were applied using apply_patch from rustfmt check diffs.
- Stable final source `qbuild all`: workspace 7039 passed, zero failed,
  zero ignored; clippy -D warnings zero diagnostics. Initial native build
  216/216 passed, but CTest was 79/80: capi_smoke linked ABI98 against a stale
  object compiled with ABI97. Ninja recorded that object's header dependency
  in train-integration rather than this worktree. Local header and Rust are
  both ABI98. No ABI bump or source correction was made.
- After cleaning generated native inputs, `qbuild verify`: PASS, native build
  255/255 (15.6s), full CTest 80/80 (120.6s), total 138.8s.
  Requests `qbuild-20261024-101502-build-ctest` (failed stale inputs)
  and `qbuild-20261024-102220-build-ctest` (corrected inputs) were separately
  collected to their actual terminal results. Rust source stayed unchanged.
  Failed native logs remain as `build-runs/r2-abi97-failed-fc4verify-*.log`.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`, and the matching build-results
  verdict. Documentation-only commits during the gate did not change Rust.

## Questions, game, and next steps

No new questions. WG5–WG9 answered copies remain in the spec merge; only WG3/WG4
remain open. No game request was needed for this core-only change, so no game
shadow or reference-warning measurement is claimed. A coordinator-owned host
adapter/probe can validate actual rendering/selection later. READY queue uses
the final full 40-character detached HEAD; the supervisor publishes the branch.
