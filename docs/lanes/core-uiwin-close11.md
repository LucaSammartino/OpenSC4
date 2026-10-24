| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-uiwin-close11: GZ window base owners | Codex; detached core-uiwin-close11 | ready: specified settled core bodies implemented; native contracts remain open | Review owner closure table and resolve WT questions before native adapter closure |

## Report

Implemented the portable base runtime in `engine/core/src/ui_windows/base`,
exported as `ui_windows::base`. It covers constructor state, copy-on-write child
ownership, parenting and z-order, exact-mask flag effects, geometry and draw
rectangles, hit testing, focus/capture, filters and message dispatch, accelerator
wiring, lifecycle, painting/presentation and overlay queues, fade-in and fade-out,
cursor routing, key equivalents, parameters, enumeration, sorting and placement.
Host operations preserve the published native call order, including distinct
drawing and presentation context queries. Plugin and C ABI were not changed.

The [Owner closure table](../../engine/core/src/ui_windows/README.md) maps each
specified section to its code and tests. The old `win::WinStore` remains the
city-window trace facade; the toolkit implementation is `base::Windows`.

| Owner | Fully implemented? |
|---|---|
| cGZWin | All published settled core bodies: yes. Full native owner: no; the spec explicitly leaves vtable details, opaque fields/bits, manager assumptions, graphics slot meanings and concrete buffer creation unsettled. These are explicit boundaries, not invented implementations. |
| cGZFadeTmpWin | All specified constructor/StealBuffer/Plot and fade behavior: yes. Native allocation/lifetime inherits the base owner's open boundary. |

This does not claim the whole `cGZWin*` wildcard for unrelated widget subclasses,
generated/custom/HTML windows or managers. The accelerator owner itself remains
in `key_accelerator`; its window wiring is implemented here.

## Validation

Final engine source is commit `a11227d5e`. The focused suite has 26 passing tests,
covering all 21 numbered widget-toolkit conformance cases and additional failure,
ownership and helper cases. Review corrections have regression tests for deferred
filter cleanup, ineligible fade surface release and distinct context queries.

| Source / operation | Rust tests | Clippy | Native build | CTest | Verdict |
|---|---|---|---|---|---|
| 6e106e666 initial all | 7075/7075 | 0 diagnostics | 256/256 edges | 80/80 | PASS; before review corrections |
| a70980fdf corrected all | 7080/7080 | 0 diagnostics | 47/47 edges | 80/80 | PASS; before context-query correction |
| a11227d5e final all | 7080/7080 | 0 diagnostics | 47/47 edges | 80/80 | PASS; native 352.6 s |
| Explicit final verify, same engine source | n/a | n/a | 210/210 edges | 80/80 | PASS; 127.4 s, no tree-change note |

The final cargo total is independently summed over all 44 `test result:` lines:
7080 passed, 0 failed, 0 ignored. Logs are `build-runs/qbuild-test.log`,
`qbuild-clippy.log`, `fc4verify-build.log` and `fc4verify-ctest.log`.
Final native request labels are `qbuild-20261024-112429-build-ctest` (all) and
`qbuild-20261024-113121-build-ctest` (explicit verify); both completed pass.
The all request notes a tree change caused only by the handoff commit while it
was queued; the engine source stayed unchanged. Explicit verify then ran with
the tree held unchanged and reached its actual terminal PASS.

Formatting: qbuild fmt passes all six owned base files using absolute paths.
Parent-only rustfmt with `skip_children=true` also passes. qbuild fmt including
the parent recursively checks untouched `view3d` files and reports 738 diff
lines there; evidence is preserved in `build-runs/parent-fmt-recursive.diff`.
Those unrelated files were not reformatted. The initial q.py Bash launch failure
is preserved in `build-runs/q/20261024-104909-30944.log`; direct focused Cargo
output passed, and the final qbuild results above are authoritative.

No real-game request was made: this is unwired core code. There is no measured
game shadow or reference-warning count, and no outstanding asynchronous operation.

## Questions and remaining contracts

Appended [WT10–WT12](../specs/questions/widget-toolkit.md): filter mutation timing
(§7.2/§14.3; snapshot passes with outermost deferred removal), type-1 dispatch's
void return (§7.3; delete then return false), and unpublished native reference,
QueryInterface, deleting-destructor and invalid-tree contracts (§§1/2/4/8;
portable creation ownership and explicit manager destruction).

Previously open sections stay open: §1 WT1 native vtable, §2 WT2 unknown bytes,
§3 WT3 default flag meanings, §4.2 WT5 busy-manager detachment (published
synchronous-detach assumption used), §8.1 WT6 registration serial (caller
supplies it), and §12 opaque graphics slots/create-private-buffer internals.
All settled behavior around those boundaries is implemented and tested.

Next: review the closure table, resolve these native contracts before wiring an
adapter, and keep other wildcard owners independently attributed. No coverage
map or ABI version was changed. Implementation milestones: `6e106e666`,
`a70980fdf`, `a11227d5e`; intermediate handoff/gate commits are documentation only.
