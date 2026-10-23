| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widgets5: widget control owners | Codex / core-widgets5 | ready (partial work) | Review gated portable stages; remaining owner rows are open in module README |

## Delivered work

Final round 3, 2026-10-23. Ready means the delivered partial work is gated and
reviewable; **no whole-owner closure is claimed for any of the five owners**.
The task explicitly permits a READY entry for partial closure in this final round.

- Existing milestones: 0de6813c ListBox/TreeView models and family vectors;
  d5e44377/a88ea3be FileBrowser state, population and resource branches;
  b568439c Grid sparse state, sizing and ownership.
- 7793547e2 adds TextEdit byte-encoding helpers, byte/state access and selection,
  filtering/composition, SetText/raw mutation/high-level replacement stages,
  literal insertion/deletion history limits, raw undo/redo replay, pending
  character accumulation, line-coordinate adapters and byte word navigation.
  Foreign UTF16/Thai/composed-length primitives stay foreign.
- 68d5aa6eb adds Grid text/extra/window queries, reference-safe text-interface
  access, owner-draw/drop-down setters, row/column room/delete/copy helpers,
  overlap/hole handling and default/per-cell format state. ClearCell now returns
  true even when no record exists.
- The module [Owner closure table](../../engine/core/src/widget_controls/README.md)
  maps every widget-controls owner section and incorporated Grid/Browser/TextEdit
  section to code/tests or explicitly open work.
- TextEdit `repairs` records unexecuted owner layout/view work. These are partial
  byte/state algorithms, not delegated foreign layout or native slot completion.
  Grid data helpers likewise do not execute the native recalculation hook.

## Measured verification

Final stable implementation: 68d5aa6eb, engine tree
`3d2954c5a1c53aa04d090807dfc0eb7f810a56c8`.

- Focused widget suite: **45 passed, 0 failed**, 4138 filtered. This includes all
  ListBox/TreeView family vectors, six Grid vectors, existing Browser cases,
  fourteen TextEdit tests and four new Grid tests.
- Explicit owned-file `qbuild fmt`: **10 files, zero differences**.
- Final `qbuild all` collected to terminal, **PASS**: Rust workspace totals
  **6414 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**; native build
  **50/50 edges**; full CTest **68/68**.
- Native label: `qbuild-20261023-233517-build-ctest`. Verdict:
  `build-runs/build-results/qbuild-20261023-233517-build-ctest.verdict`.
  Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- Native build took 82.1 seconds (33 C/C++ cache hits, zero misses); CTest took
  188.5 seconds; native verify total 274.9 seconds.
- Verify reported “tree changed during the run” because documentation-only
  checkpoint 465883b70 was committed while it waited in the shared build queue.
  `git diff 68d5aa6eb HEAD -- engine` was empty after completion; both engine
  tree hashes were exactly the value above. No source/input change justified a
  duplicate full gate. The final report commit is documentation only.
- Old pending native verdict was collected before any new verify: label
  `qbuild-20261023-225614-build-ctest`, PASS 159/159 and 68/68. It was not reused
  as the final gate. No operation remains outstanding.
- Corrected focused compile/clippy and invalid relative fmt-input evidence is
  preserved in `build-runs/textedit-focused-initial-failure.md`; the failed
  Git Bash wrapper log is named there. These are distinct from final PASS.
- No game requests or game/shadow/pixel comparisons. No plugin/C ABI wiring,
  ABI exports/version changes, generated ABI-entry edits or coverage-map edits.

## Remaining owner sections (open)

These are implementation omissions unless a question ID is explicitly named.
They are not disguised as missing specifications.

| Owner / widget-controls rows | Remaining work |
|---|---|
| Grid / 2.1–2.3 | Native lifecycle and slot adapters; recalculation hooks, scrollbars, input/focus/hit testing and painting; editable-cell text, resource/format/extra wrappers. Incorporated win-grid 5.1–5.4 and 5.6–5.14 retain open work as listed per row. WG5 dependencies and WG6 initializer/alignment/size-copy gaps remain provisional. |
| TextEdit / 3.1–3.3 | Native QueryInterface/lifecycle/default integration; execution of pending layout/view repairs; text-edit 5,7–10 (caret/timers, keyboard/clipboard, font/bar/pixel layout, painting, mouse/messages/resources) and remaining adapters/layout extension. Section 11's timer/clipboard/layout/bar/paint/focus/resource vectors are unimplemented. TE5/TE8 foreign boundaries retained; TE9 defaults require explicit resolved inputs. |
| FileBrowser / 4.1–4.2 | Automatic directory/file refresh orchestration, AddControls, Init/paint/property/lifetime adapters and remaining resource branches; incorporated 8.2,8.5–8.7 incomplete. FBQ1/2/4/5/6 remain open. C05/C14 work recording does not prove refresh execution. |
| ListBox / 5.1–5.5 | Remaining adapter/lifecycle/default coverage and tests; drop sizing/page/removal details, capture and pixel/text preprocessing remain WC1/WC4/WC5. All WC-L01–L15/WC-F01 family vectors are represented, not whole-owner proof. |
| TreeView / 6.1–6.5 | Remaining property/node ABI, native insertion/restart/image lifecycle, drag/modifier/page and layout/drawing details remain WC2/WC3/WC6. All WC-T01–T12 vectors are represented, not whole-owner proof. |

## Questions and next steps

New this round: **TE9**, appended to
[questions/text-edit.md](../specs/questions/text-edit.md): constructor
maximum-undo and preferred-X defaults are missing. The byte/state stage requires
both values explicitly; it does not guess constructor behavior. This does not
request a fourth research round or reopen parked TE5/TE8.

Prior questions retained: WC4/WC5/WC6 in questions/widget-controls.md and WG5/WG6
in questions/win-grid.md, alongside the pre-existing WC/FBQ/TE residuals.
The coordinator must regenerate the question index; this engine lane may not
run scripts from tools/ or edit the generated index.

Review and land the gated partial implementation if useful. The remaining rows
above require further authorized implementation/spec work; no continuation or
whole-owner coverage claim accompanies this READY entry. Detached HEAD is
intentional; the supervisor publishes commits to the reserved core-widgets5 branch.
