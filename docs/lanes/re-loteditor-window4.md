| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-loteditor-window4 (RE: native lot configuration editor) | `re-loteditor-window4` / RE spec lane, detached HEAD | wip: round 2/3 published; LE1/LE2 answered, LE3–LE5 narrowed; exact inventory preserved; no complete-owner credit | Coordinator review native contracts and corrections; remaining fence/dialog/input/render/lifecycle branches require further native tracing. No self-restart. |

## Report

This round changes only the native lot configuration editor specifications,
their question records/index and this lane report. No engine, tools, maps,
ABI tables, other windows, external assets, Git refs/config or main-checkout
files were edited. The exact `ffa85bcf` 109-entry/52646B inventory is preserved;
only its per-entry contract statuses/references changed. Save As still uses
the corrected real entry `0x00485390`.

### Commits and substantive results

- `45b6a09a`: LE2 answered. Native identity resolution can succeed; lowercased
  directory equality precedes provider-derived keys. New name is `New Lot`.
  Save As flags, teardown order and shallow-reference ownership transfers are
  specified, including restoration and copy outcomes.
- `f577d079`: LE1 answered. All 15/3/8 compatibility rows map to bit row+1;
  acceptance clears the entire vectors. Missing grids can fault after partial
  mutation. The first scalar control has no accepted write-back.
- `32513732`: four family/texture selector contracts, callback dispatch and
  prop-only first-byte letter search; duplicate candidates and unchecked final
  fallback; signed fence filtering; unordered containment/cleanup; type-7
  logical-size/four-store discrepancy; record-allocation faults; append vs
  front insertion and resulting selection/draw order; rendering subset and
  height-label zero/unordered/negative-sign behavior. Independent discriminating
  vectors are in spec §14.

### Material corrections and numeric evidence

Cleanup requires full containment for types 0/1/4, not intersection. It returns
the combined object/fence removal count, updates dirty/title on nonzero count,
and does not select, draw or invalidate. The older contrary partial contract
and vector were corrected, not retained as alternative behavior.

Duplication tests offsets (+4,+4), (−4,−4), (0,−4), then inserts at (−8,+4)
without another containment check. Fence coordinate bytes are signed and have
no lower-bound rejection. Quiet NaN comparisons pass individual containment
and cleanup tests when x87 invalid exceptions are masked. These findings use
native instruction status masks, widths, store order and executable constants.
They do not extrapolate movement/hit-test or integer-conversion NaN behavior.

The spec records type-7 resize to three dwords followed by a fourth store,
without inventing safe capacity. It also records reversed common default-bound
stores and the negative-height label's doubled minus. Allocator consequences,
normal drawing details and other unresolved behavior remain explicitly open.

### Questions and remaining work

LE1 and LE2 moved to `docs/specs/questions/answered/lot-configuration-editor.md`
with dated DIFFERS answers and rules in the main spec. The question index was
regenerated. LE3–LE5 remain in the open question file with resolved subsets
identified: fence toggles/foundation, physical payload consequences, other
numeric conversions/unordered cases, other dialogs/popups, sort interpretation
and name fallbacks, lifecycle/input/hit-test/3D/render phases and shared-helper
ownership still block completion. No other question files were read or changed.

### Validation and limits

`git diff --check` passed; question-index regeneration completed. This is
documentation validation only: cargo/clippy/ctest are n/a. No implementation or
test outputs supplied expected behavior, no game runs/shadow verdicts, no ABI
changes or proposed rows, and no runtime or complete-owner coverage claim.

Evidence came from absolute main-checkout export/decomp/executable paths.
Export queries preceded function tracing; instruction bytes were used for
numeric and lost-stack branches. REA was unnecessary and no REA failure occurred.
q.py could not start Bash (`E_ACCESSDENIED`), so bounded PowerShell/Python reads
were used. Git reported a packed-refs.lock permission error during commits,
but detached-HEAD commits succeeded; refs/config/hooks were left untouched.

This authorized round ends here. Coordinator review can use the committed
contracts now; genuine remaining branches retain open status. No merge, push,
subagents, external communication or self-restart was performed.
