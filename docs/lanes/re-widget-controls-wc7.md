| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widget-controls-wc7: control behavior questions | `Codex` / detached reserved worktree `re-widget-controls-wc7` (RE lane) | in progress: final round; WC1/WC4–WC6 archived, WC2/WC3 under review; no ABI/game changes | Finish existing TreeView contracts, run doccheck once, commit report and enqueue ready. |

## Report

Round 2 stops at the prescribed two-thirds context cutoff with passing,
committed partial work. It does not claim complete widget_controls closure.

### Changes and measured evidence

- `14590ec17`: answered WC1 in widget-controls section 5.6, corrected the
  ListBox slot inventory, added vectors L27-L36, moved WC1 into the answer
  archive and regenerated the question index. Sorting adapters had been
  mistaken for update suppression. Folded getters, byte/output signatures,
  clear/remove/copy/search wrappers, colors/gutters/drop policies, scrollbar
  skin and notification delivery are specified. Native negative removals can
  decrement count without deleting storage. Fractional highlight intensity
  produces no fill; NaN survives the setter. Malformed scans can reuse prior
  RGB stack words; the owner appends no ellipsis. Move and mouse-up use
  opposite capture-owner comparisons. Shutdown/destruction domains differ.
- `391e9d561`: partial WC2 section 6.6 and vectors T19-T26. Insertion and
  get/set use different descriptor layouts; getter replaces a native child
  list with immediate handles. Sentinels -2/-3/-4 mean prepend/append/sorted;
  equal sorted labels precede older equals. Globally absent references fail
  after consuming a handle and leaking the constructed node; valid references
  outside the chosen parent append. Node fonts are borrowed. Shutdown leaves
  internal-init and the now-dangling root pointer intact, so later Init is not
  a supported fresh restart. O3-O18/O37/O40/O41 property quirks are specified,
  including unsigned clamp to the flags word and absent negative index check.
- Evidence used only the main checkout's re_query export and bounded decomp
  reads. Numeric widths, x87 conditions, index bounds and insertion list
  helpers were verified against instructions with address citations. REA
  was not invoked because the brief says this session has no binary open.
- Eighteen added vectors are implementer scenarios, not executed comparisons.
  No engine, plugin, capi, tools/coverage-map.tsv or ABI-table changes.

### Gate and limits

`python tools/doccheck.py check`: PASS, exit 0, `doccheck check: ok`, after
both milestones. Final log: `build-runs/doccheck-re-widget-controls-wc7-r2-final.log`.
`git diff --check` passed. Cargo, clippy and ctest: n/a for spec-only work.
Git printed a packed-refs.lock permission warning during commits, but
commits completed successfully. Shared refs/config/hooks were not changed.
No game run, shadow exactness result, game request or new C ABI export.
No proposed ABI rows file was needed.

### Questions and owner closure

WC1 is now archived in `docs/specs/questions/answered/widget-controls.md`.
WC2 and WC3 remain open in `docs/specs/questions/widget-controls.md`; WC2
has an explicit list of its remaining work. Prior WC4/WC5/WC6 remain archived;
no duplicate questions were added. Round 1 commits were `2cc7327cc` (WC6)
and `819363eb7` (WC4/WC5).

The expressly requested engine README table was read only. TreeView rows
6.1-6.5 and 8 cannot yet be marked fully closed: WC2/WC3 remain. An engine
lane can remove the WC6 boundary from 6.5/8 and the root-inclusion proviso
below the table after implementing/testing the already-settled rules; its
WC3 rendering boundary remains. WC2 property/descriptor/insertion/restart
boundaries can now be narrowed to the remaining items below. ListBox WC1
boundaries now have behavior and vectors for owner tests; no full owner
implementation claim is made. Grid, TextEdit and FileBrowser receive no new
closure from this round. No engine file was edited.

## Handoff

- Final round 3 started 2026-10-24 at 4f94638c6; detached HEAD retained. Bash q.py fails CreateFileMapping; use bounded PowerShell reads and re_query directly.
- Round 3 milestone: WC2 adapters/wrappers, move/removal, image ownership/failure and T27–T36 specified; WC2 archived. Gate deferred to final once. Next: settle WC3 input/drag/drawing from named owners; then refresh inventory/coverage and index.
- Goal remaining: finish WC2 and WC3; WC1/WC4/WC5/WC6 answered.
- Done: 14590ec17 WC1; 391e9d561 partial WC2; doccheck and whitespace passed.
- Next exact step: inspect O19-24 image/insertion wrapper bodies at 009D3F7B/009D34A7/009D0C63/009D407E/009D3D60/009D3CBA.
- Then O26-28 move/removal/wrapper at 009D1FAB/009D2E07/009CFC13; instruction reads began but their full contracts were not published.
- Remaining WC2 node/selection adapters: O31-36/38/42-44/48-55; see question progress paragraph.
- WC3 remains unchanged: 009D1031/009D135A/009D14BF input; 009D2577 paint; 009D17A8/009D0CB1 drawing; 009D0888 images; drag helper 009CFD89-009D14BE.
- Main decomp files: decomp/cGZWinTreeView.c and decomp/cGZWinTreeViewDrag.c; use absolute main paths.
- Node insertion 009D3A7A starts at decomp line 2215; InternalInit 2138; Shutdown 1800. Export is the authority for widths/offsets.
- WC2 descriptors differ: insert text +8 and expanded +34; get/set text +0, four state bytes +2C..2F, native child list +30.
- No game-request labels outstanding; no ABI rows file.
- Detached HEAD is intentional; supervisor publishes commits to reserved branch.
