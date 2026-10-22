| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-textedit4 (cGZWinTextEdit final round 3/3) | `re-textedit4` / (RE lane, detached worktree) | in progress; 150 bodies / 19,999B reconciled; 44 proposed vectors; no owner promotion | Prove residual TE1/TE5/TE8, then park exact gaps; no fourth round. |

## Handoff

- Goal: final round 3/3: prove residual TE1/TE5/TE8 using native bodies; park exact gaps, preserve worktree, no fourth round.
- Baseline: b613685e (134 direct labels/19,415B; 20 vectors; TE2 answered).
- Done: actual coverage reconciled: 150 bodies/19,999B, all 134 old rows plus 16 exact measured additions, no size mismatches; public attribution evidence added.
- Done: eeeaf4ac reconciles attribution and proves TE4 history eviction; original 20 vectors preserved.
- Done: TE3 numeric failure domains, TE6 mixed password/hit coordinates and TE7 cleanup/image/color/callback failures specified; ten further vectors (34 total).
- Done: 1f74cd36 publishes TE3/TE6/TE7 answers and explicit unsafe-call/resource boundaries.
- Done: new layout extension proves large metrics (334B), bar sizing (454B), pixel-hit (387B), word/composed motion (250/224B), bar ranges (241B), drag routing (236B), scroll and index contracts; 44 total vectors.
- Done: final inventory audit edits prepared; questions.py check and git diff --check PASS; address join still exact; 48 P, 36 P/B, 50 U direct entries and 44 vectors measured.
- Round 3 milestone: TE1 helper byte tables/malformed and pending outcomes proved; caret-in-view, AddScrollBar and SelectCurrentWord owner paths published; eight new proposed vectors (52 total).
- Done: a8095773 publishes byte helper/view contracts and TE1 answer (52 proposed vectors).
- Round 3 milestone: page/arrow arguments and cache retention proved; individual accessors, period/input/lifecycle adapters, LF/composition owner loops published; 60 proposed vectors.
- Next: count exact final per-body dispositions, park TE5 foreign helpers and TE8 remaining addresses; check docs, final report and queue. No fourth round.
- Round 2 stopped at its guard; this is the explicitly authorized final third round, with no restart/fourth round.
- Current substantive commits: eeeaf4ac attribution/history; 1f74cd36 TE3/TE6/TE7; 733c7744 layout/body extension.
- Latest audit removes superseded TE7 uncertainty and updates ~36 newly proved inventory dispositions; no whole-owner promotion.
- Open TE1 encoding; TE5 Page-key preferred-X audit, single-line caret-in-view, AddScrollBar resource negotiation, word-selection/composition repair; TE8 remaining U/P-B/folded/generated contracts.
- Queue not written. No ABI changes/proposals. No native/game/conformance execution; vectors are proposed cases.
- Git commits succeed but print packed-refs.lock denial; do not change shared refs/config or run maintenance.
- Main: C:/Users/tlori/FreeCity4; WT: C:/Users/tlori/FreeCity4/.claude/worktrees/re-textedit4.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-textedit4-r3-manifest.json.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded PowerShell shell calls.
- Keep original 20 vectors and answered TE2; newly traced bodies only; no whole-owner credit before inventory reconciliation.
- No game/clipboard/network actions; no REA used. No ABI proposals yet.

## Prior report (b613685e; retained during round 2)

Published [text-edit.md](../specs/text-edit.md) and the
[address-by-address inventory](../specs/text-edit-inventory.md). The request's
`cGZWinTextEdit19999B` is represented by native export class
`cGZWinTextEdit`; no export class with that suffix was found. This is a
substantive partial contract, ready to merge as documentation, with **no
complete-owner or whole-widget-family coverage claim**.

Changes and evidence:
- f6875240: native byte text/state, whole-text replacement limits, ordered
  selection, overwrite/input behavior and caret timers. First substantive
  behavior was committed in the initial inventory/reading phase.
- 24a4278a: raw mutations, undo/redo, key/clipboard gates, font/layout/view
  boundaries, password/selection/caret rendering, mouse/messages/flags and
  lifetime behavior. TE2 resolved using the actual sound-adapter instructions.
- b7db002c: exact direct-label inventory, exclusions/unnamed helpers, key
  routing instruction audit, destructor sequence and partial-owner checklist.
- Final audit: Shutdown clears the font before stopping timers; background
  image ownership endpoint remains explicitly open rather than inferred.

Measured inventory: 146 name matches total **19,908B**, including nested
action constructor, five adjusted thunks, folded maximum setter, foreign
factory and four STL/allocator bodies. The direct nonnested class-label
subset is **134 bodies / 19,415B**. Sixteen additional native helpers are
listed separately. The brief's **19,999B** attribution is not reconciled;
TE8 records this exact discrepancy. The inventory is not coverage credit.

The largest actual owner methods examined were GZOnKeyDown 2938B, GZPaint
1850B, InitializeMemberVariables 471B, SetScrollBarSizes 454B, raw InsertText
447B, InsertTextHighLevel 437B, AddScrollBar 413B and SetText 399B. Unclosed
scrollbar-size/resource rules remain marked U/B despite examining the body.

Distinguishing native facts:
- Text subobject is W+0xD8 despite exported thunk names saying n228.
- Selection retains endpoint order and clamps unsigned values to byte length.
- Maximum setter does not truncate existing bytes; selectors 8/100 can cross
  the limit to finish a character in SetText.
- Cut dispatches Delete even with no clipboard manager; password bit blocks
  copy/cut but does not occur in the paste gate.
- Editable Escape sends message 5; noneditable Ctrl+Escape can copy. Return
  uses bit 0x400 and message 0x9B1552D5; arrow forwarding uses bit 0x800.
- Wheel divides each event by 100 without remainder accumulation.
- Painting uses a separate password mask, measured selection intervals and a
  1/2-pixel caret threshold at line height 24, restoring float width to 1.0.
- Horizontal/vertical gutter setters differ in reflow behavior.

Foreign contracts: font measurement/wrapping/drawing slots, callback timers,
clipboard service, WCM/window/capture/drawing primitives, encoding/composition
helpers and sound-service adapter remain explicit boundaries. No customary
editor API, Unicode behavior, OS clipboard format or resource failure
recovery was assumed. No real clipboard was read or written.

Validation: questions index regenerated; `tools/questions.py check` PASS;
`git diff --check` PASS. The inventory has 134 direct rows and 16 separately
listed helpers, and the spec contains **20 independent native-derived
vectors**. These are proposed conformance cases, not executed game/shadow
results. Cargo/clippy/ctest and game runs: n/a for this docs-only lane.
REA was not needed; no REA error/fallback occurred.

Questions: **TE2 answered** in
[answered/text-edit.md](../specs/questions/answered/text-edit.md).
**TE1, TE3-TE8 open** in
[questions/text-edit.md](../specs/questions/text-edit.md): encoding,
raw-mutation invalid domains, history eviction, exact navigation/scroll/
metrics/resource paths, multibyte mask mapping, remaining ownership and
attribution/thin-method closure.

Scope: only docs/specs, its questions/index and this lane report changed.
No engine reads/edits, tools/maps/ABI table edits, ABI proposal or C ABI
export changes. No game requests, network, external communication, subagents,
branch/ref changes, merges or pushes. Work stayed on reserved detached HEAD;
supervisor publishes commits to the reserved lane branch.

Next steps belong to the coordinator: accept the bounded document without
owner promotion; use the exact U/B checklist and TE1/TE3-TE8 if separately
authorizing future RE. This guarded round is finished; no self-restart or
additional lane round requested.
