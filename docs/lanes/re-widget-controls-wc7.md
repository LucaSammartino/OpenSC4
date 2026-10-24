| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widget-controls-wc7: control behavior questions | Codex; detached reserved worktree re-widget-controls-wc7 | ready: partial closure WC4/WC5/WC6; doccheck passed | Implement settled rules; continue RE on WC1-WC3. |

## Report

Partial closure, as permitted by the round cutoff: answered WC4, WC5 and WC6
in [widget-controls](../specs/widget-controls.md), moved them to the
[answer archive](../specs/questions/answered/widget-controls.md), and regenerated
the question index. WC1-WC3 remain in the open question file. WC2 and WC3
were not already archived at this base. No duplicate open questions were added.

### Changes and evidence

- `2cc7327cc`: WC6, sections 6.3/6.5 and vectors T13-T18. Root flag 0x2,
  image flag 0x4, connector flag 0x8, preorder membership/depth, last-selected
  navigation anchor, reserved image space, paint rectangles and inclusive hit
  edges. Viewport height derives floor whole-row and ceil paint/page counts.
  Export instructions verify integer widths, shifts and x87 division/ceil.
- `819363eb7`: WC4/WC5, sections 5.3/5.5 and vectors L19-L26. Removing the
  selected non-last row retains its index, next-page uses requested origin
  plus wholeRows, and drop policies specify parent clipping and raw-string
  width measurement. Literal ASCII # introduces color syntax; ## removes one
  byte for a literal prefix. Constants were read from the main executable;
  behavior was checked in the local export/decomp and instructions.
- Fourteen added vectors are specification scenarios, not executed game
  comparisons. No translated pseudocode or instruction listings were added.

### Gate and limits

`python tools/doccheck.py check`: PASS, exit 0, `doccheck check: ok`.
Log: `build-runs/doccheck-re-widget-controls-wc7.log`. `git diff --check`
also passed. Cargo, clippy and ctest: n/a for this spec-only lane.

The first attempt through q.py failed before starting doccheck because Git
Bash CreateFileMapping was denied (wrapper exit 256). The successful check
was then run directly in PowerShell, foreground, with output saved locally.
Git printed a packed-refs.lock permission warning during commits, but both
commits completed successfully; no refs/config/maintenance changes were made.

No game runs, shadow comparison, REA calls, C ABI changes, proposed ABI rows,
engine/plugin/capi changes or coverage-map changes. The export supplied the
needed evidence; no REA error or fallback occurred.

### Owner closure guidance

The requested owner table still has open WC2/WC3 rows. None of its TreeView
rows 6.1-6.5 or 8 can be marked fully closed by this partial change. An engine
lane can remove the WC6 flatten-flags boundary from 6.5 and the literal
root-inclusion proviso below the table once implemented/tested, but WC3 paint
and gradient requirements remain open. Row 8 must retain WC2/WC3 while dropping
WC6. ListBox removal/drop-sizing and color-prefix predicates are now available
for owner tests. Grid, TextEdit and FileBrowser rows receive no new closure.

## Handoff

- Goal remaining: close WC1-WC3; no question was left open because of odd original behavior.
- Completed: WC6 2cc7327cc; WC4/WC5 819363eb7; doccheck passed.
- Investigation stopped near the prescribed two-thirds context cutoff; ready applies to committed partial work.
- Next RE step: WC1 residual ListBox adapters, comparison helpers, malformed rendering, allocation/destruction and capture behavior.
- Then WC2 TreeView descriptor/property ABI, insertion sentinels/list helpers, ownership/restart; WC3 input/drag/pixel/resource failures.
- WC6 anchors: FindTreeInfo 009D2FD6, RecalculateAll 009D354F, DrawItem 009D0CB1, DrawNode 009D17A8, key anchor 009D15C1-009D15E3.
- WC4 anchors: removal 009C93D7, NextPage 009C8EF3, height 009C900F, width 009C9D3B, Init 009C8415.
- WC5 anchors: paint 009CA19A; constants 00A9F354 (#), 00AA6DDC (##).
- No game-request labels outstanding; no ABI row file required.
