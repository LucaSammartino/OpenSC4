| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widget-controls-close13 (partial settled widget owners) | `Codex` / `core-widget-controls-close13` (engine lane, detached HEAD) | READY partial milestone 0fede2816: cargo 7405 passed/0 failed; clippy clean; native 47/47; CTest 81/81; focused widgets 99/99; fmt 8 files clean; ABI unchanged; no game runs or new questions | (1) finish remaining ListBox adapter/conformance audit; (2) complete TreeView layout, raw-property aliases and helper integration; (3) audit all literal vectors before claiming owner closure |

## Report

The committed improvements are ready to land under the standing cutoff rule,
which permits passing partial closure. **The requested complete owner closure
is not achieved.** The README still records settled implementation obligations;
they have not been relabeled as spec questions or unsupported open contracts.
New implementation stopped at the two-thirds budget point, then the same full
gate operation was collected to its terminal exit 0.

The requested first merge of `re-widget-controls-wc7` reported Already up to
date; its ancestor check succeeded. Git also reports a denied shared
packed-refs lock, but detached HEAD commits succeed. No shared refs were changed.

### Implemented module paths

The implemented column in `engine/core/src/widget_controls/README.md` names
these paths relative to that directory:

- `list_box.rs`: signed storage insertion/removal preserve native count changes
  after unmatched negative traversal. Public removal delegates to that storage
  primitive. Destruction releases the timer before Shutdown's five retained
  interfaces, then disposes entries and invokes the base-destruction primitive.
  Drop bar move/up dispatch uses half-open list/bar bounds, converts coordinates
  to bar-local space, preserves the opposite capture-tracking comparisons and
  bypasses bounds only when tracking. `mouse_up_at` accepts current coordinates;
  the existing `mouse_up` convenience method uses the last pointer position.
  Flag-2-disabled paint uses the alternate font color.
- `list_box/adapters.rs`: color conversion/getters with untouched failure
  outputs, font-color reset, drop policies, signed gutter/bar-gap bytes,
  minimum height, retained bar skin and empty/full rectangle handling,
  indexed string copying, caller comparison with flag 1, delivery mode and
  false stub. Plain stored fields retain the thin accessor state.
- `tree_view/rendering.rs`: selected image source crops are centered by logical
  halving in the reserved image area. Image hit rectangles reflect the actual
  destination only after successful access. Ancestor continuation coordinates
  include image-dependent indentation. Viewport blitting supplies both source
  and destination rectangles to a primitive in `tree_view.rs`.
- `tree_view/input.rs`: temporary drag drawing emits packed text/image and
  connector/expander rows, restores cached working-buffer hit rectangles and
  restores the working buffer before running the borrowed session. A bounded
  rendering error cleans up the temporary surface/reference.
- `mod.rs`: additive primitive color conversion, skin, buffer-width and base
  destruction hooks. No control algorithms are delegated to these primitives.
- `settled_tests.rs` and `tests.rs`: recording assertions for bar coordinates
  and bounds, untouched output arguments, malformed storage counts, timer-first
  release order, image centering/access failure and preserved drag hit geometry.

Earlier implementation milestones `915505f6a` and `1dac37531` remain included.
This round's implementation commit is `0fede2816`.

### Measured checks

- Focused real-crate cargo test: **99 passed, 0 failed**, 4789 filtered out.
  Evidence: `build-runs/focused-r2-corrected.log`. The initial compile exposed
  a trait-name collision with BrowserHost; the new primitive was renamed to
  `destroy_window`. Failed evidence remains `build-runs/focused-r2.log`.
- Scoped qbuild fmt: **8 explicit owned Rust files, exit 0, diff_lines 0**.
  Evidence: `build-runs/fmt-r2-corrected.log`; original formatting findings
  remain in `fmt-r2.log` and `fmt-diff-r2.log`.
- Required stable-source `qbuild all`: **PASS**, actual terminal exit 0.
  Cargo **7405 passed, 0 failed, 0 ignored**, summed over 50 result blocks;
  clippy **exit 0, diagnostics 0** with the gate's warnings policy.
- The same all operation submitted native build/CTest through the coordinator
  queue as `qbuild-20261024-183548-build-ctest`. **verdict: pass**; build
  **47/47 edges**, 180.5s; CTest **81/81**, 270.3s; verification **PASS**, 453.0s.
  No duplicate standalone verify was launched. Evidence: `build-runs/all-r2.log`,
  `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`,
  `fc4verify-ctest.log`. Source stayed fixed throughout this gate.

### Remaining owner obligations

These are settled implementation work, not unanswered WC questions:

1. ListBox sections **5.1/5.3/5.6/7**: finish the complete slot-by-slot adapter
   and count/geometry audit and assert every literal L vector. The new
   adapters and grouped tests do not establish the full native inventory.
2. TreeView sections **6.1/6.2/6.5/6.6**: complete raw negative default-color
   index aliases and enumeration adapters; connect automatic content
   measurement, working-buffer/bar requirements and viewport recalculation
   to dirty visibility queries/painting. Resource initialization still needs
   the complete owner-side audit.
3. TreeView sections **6.7/7**: complete enclosing helper integration and
   audit all packed rendering/input effects against every literal vector,
   particularly descriptor enumeration T24, exact T17 dimensions and T47
   reselection ordering. The present grouped tests are not an exhaustive
   assertion of all 57 added conformance vectors.
4. The incorporated TextEdit and FileBrowser open boundaries in their own
   specs remain recorded in the README. Their internals were not changed.
   Grid's existing section mapping was retained.

No new questions: WC1-WC6 are settled. No plugin/capi wiring, ABI exports,
ABI version, legacy_control internals or text-edit internals changed. No
game request or shadow comparison was performed; no pixel equality is claimed.
