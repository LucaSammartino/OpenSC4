| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwin-spec9: widget owner contracts | Codex; `re-gzwin-spec9` (reserved lane, detached HEAD) | in progress: round 2 | Finish partial owners, then continue ranked owners; validate and requeue. |

## Report

### Delivered contracts

Added three prose/table specifications, linked from [widget-toolkit.md](../specs/widget-toolkit.md):

- [widget-generator.md](../specs/widget-generator.md): primary overrides and helper inventory, construction defaults,
  procedure lifetime and notification order, the mouse/nonmouse dispatch distinction, client rectangle calculation,
  layout negotiation and final-placement boundary, capture and drawing responsibilities. A handled procedure mouse
  message still reaches base dispatch. A failed procedure Init follows partial initialization rather than rollback.
- [widget-html-window.md](../specs/widget-html-window.md): primary overrides and method families, document/service
  state, parse-before-install replacement, status notifications, local cancellation, signed scroll clamping, viewport
  updates and the synchronous layout/paint boundary. Cancelling a loading request returns false; cancelling again
  returns true. HTML completion-target DoMessage is distinct from primary window dispatch.
- [widget-custom.md](../specs/widget-custom.md): parent forwarding mutates the input record only for types 7, 10 and
  13; direct key callbacks instead send fresh records to the notification target. Also specifies QI null-output and
  the fill-drawing boundary.

There are 25 conformance vectors across the new files. Existing child/parent, z-order, base message/input and
buffer/presentation contracts remain in widget-toolkit.md; links avoid duplicating those rules. HTML parsing is
linked to html-document.md and modal/generated window integration to modal-dialogs.md and ui-windows.md.

**Scope is deliberately partial.** The three addressed owners total **15,433 bytes** (7,049 + 8,139 + 245), not
15,433 bytes newly fully specified. The requested 100+ KB aspiration was not reached. No newly complete owner or
coverage-percentage increase is claimed. Stop broadening this round at the two-thirds analysis allocation and
preserve remaining contracts as questions. The largest unworked owners remain listed below for the next round.

### Catch-row ranking and coverage proposals

Ran main-checkout `tools/coverage.py --owners` against the Windows export and working executable, with outputs
under this worktree's `build-runs/`. Filter: subsystem `widget toolkit`, owner starts `cGZWin`, excluding the exact
owners cGZWinGrid/TextEdit/ListBox/TreeView/FileBrowser. This yields **54 owners, 1,548 function records,
164,976 bytes**, largest first. Nested/folded owners matching the wildcard remain listed, rather than being
silently rolled into the excluded exact rows. These are attribution-weighted function bytes, not source-file size.

The destination column proposes the eventual **spec column per owner**. `hold` means retain `-` in the coverage
map now: a partial destination is not whole-owner closure. `defer` means no destination proposed in this round.
The map itself was not edited. In particular, do not put widget-toolkit.md on the broad `cGZWin*` row.

| Owner | Bytes | Functions | Proposed spec column / landing action |
|---|---:|---:|---|
| cGZWin | 12,876 | 160 | widget-toolkit.md; hold for its existing open base questions |
| cGZWinCOMDirector | 9,760 | 115 | -; defer |
| cGZWinCommandSystemSupport | 8,755 | 16 | -; defer |
| cGZWinLuaScriptView | 8,515 | 46 | -; defer |
| cGZWinHTML | 8,139 | 61 | widget-html-window.md; hold, WTH1 |
| cGZWinBtn | 7,818 | 87 | -; defer |
| cGZWinGen | 7,049 | 56 | widget-generator.md; hold, WTG1 |
| cGZWinCtrlMgr | 6,805 | 48 | -; defer |
| cGZWinScrollbar2 | 6,320 | 37 | -; defer |
| cGZWinLineInput | 5,859 | 59 | -; defer |
| cGZWinMgrW32 | 5,106 | 65 | -; defer; related modal-dialogs.md is not an owner-closure claim |
| cGZWinOptGrp | 4,946 | 49 | -; defer |
| cGZWinCombo | 4,820 | 55 | -; defer |
| cGZWinSpinner | 4,707 | 51 | -; defer |
| cGZWinColorPicker2 | 4,061 | 11 | -; defer |
| cGZWinMsgBox | 3,756 | 27 | -; defer |
| cGZWinLuaScriptConsole | 3,545 | 22 | -; defer; unreliable sampled pseudocode |
| cGZWinDragAndDropManager | 3,518 | 48 | -; defer |
| cGZWinIMEComposition | 3,407 | 67 | -; defer |
| cGZWinSlider | 3,398 | 35 | -; defer |
| cGZWinGenDlg | 3,231 | 30 | -; defer |
| cGZWinGenTitleBar | 3,123 | 23 | -; defer |
| cGZWinOutline | 2,803 | 34 | -; defer |
| cGZWinPieMenu | 2,651 | 15 | -; defer |
| cGZWinText | 2,554 | 23 | -; defer |
| cGZWinToolTip | 2,269 | 13 | -; defer |
| cGZWinIMECandidateList | 2,188 | 13 | -; defer |
| cGZWinIMEProxy | 2,129 | 34 | -; defer |
| cGZWinTextTicker | 2,054 | 16 | -; defer |
| cGZWin32COMDirector | 2,043 | 37 | -; defer |
| cGZWinScrollbar | 1,824 | 15 | -; defer |
| cGZWinFlatRect | 1,538 | 18 | -; defer |
| cGZWinMgrBase | 1,469 | 9 | -; defer |
| cGZWinBMP | 1,407 | 26 | -; defer; audit suspect early-address labels |
| cGZWinLuaScriptViewVariables | 1,184 | 2 | -; defer |
| cGZWinKnob | 1,176 | 13 | -; defer |
| cGZWinFolderRow | 1,029 | 9 | -; defer |
| cGZWinIMEServer | 883 | 35 | -; defer |
| cGZWinLuaScriptViewCallStack | 876 | 5 | -; defer |
| cGZWinToolTipMgr | 859 | 12 | -; defer |
| cGZWinIMEServer::cGZIMECandidateListDataEx | 804 | 7 | -; defer |
| cGZWinLuaScriptViewBreakpoints | 744 | 2 | -; defer |
| cGZWinFolders | 637 | 9 | -; defer |
| cGZWinLuaScriptViewGeneral | 560 | 7 | -; defer |
| cGZWinTreeViewDrag | 544 | 7 | -; defer |
| cGZWinTestService | 325 | 1 | -; defer |
| cGZWinCustom | 245 | 6 | widget-custom.md; complete in round 2, WTC1 answered |
| cGZWinDlg | 195 | 5 | -; defer |
| cGZWinTreeView::GZWinTreeView_TreeNode | 151 | 1 | -; defer; coordinate with widget-controls.md |
| cGZWinIMEServer::cGZIMECompositionStringDataEx | 122 | 2 | -; defer |
| cGZWinLuaScriptView::ScriptViewInfo | 85 | 1 | -; defer |
| cGZWinKeyAcceleratorResFactory | 61 | 1 | -; defer |
| cGZWinTextEdit::UndoRedoAction | 31 | 1 | -; defer; coordinate with text-edit.md |
| cGZWin::ChildList | 22 | 1 | -; defer; related widget-toolkit.md child-list rules |

### Evidence and limitations

Used main-checkout `tools/re_query.py` first, then owner pseudocode under main-checkout decomp/. No REA call was
made: the task states no binary is open. Numeric contracts were checked against paged `asm --bytes` queries;
specs cite the relevant instruction addresses. Existing ABI metadata supplied primary slot numbers.

Important evidence checkpoints include generator construction `0099B6A5`, client calculation `0099922F`,
procedure dispatch `0099A276`, procedure notifications `009998C8`, HTML signed clamping `008FAD0E`, ready-state
installation `008FBF73`, cancellation `008FA9A7`, ready-only painting `008FB3F5`, custom coordinate mutation
`0095B9F8`, QI `0095B9C1`, and direct key callbacks `009CD75C/009CD789`. Evidence helpers, ranking TSV and
coverage outputs remain ignored under build-runs/. No decompiler output or assembly was committed.

The q.py Bash wrapper failed to start (CreateFileMapping access denied), so direct capped PowerShell commands
were used. A qgrep invocation failed on cp1252 output; subsequent use selected UTF-8. Neither failure is REA
evidence. Git reported inability to create shared packed-refs.lock after commits, but each detached-HEAD commit
completed and its actual HEAD was verified. Shared refs/config were not changed.

Some auto-matched cGZWinBMP methods at 007B/007C appear to describe a different owner. Lua console sampling has
broken stack recovery. Those samples did not become behavior rules. Complete secondary vtables, all drawing
helper metrics, and many owner constructors still need analysis. The new specs label those limits explicitly.

### Validation, questions and next work

- `python tools/doccheck.py check`: **PASS** on the delivered docs.
- `python tools/questions.py index`: regenerated after filing WTG1, WTH1 and WTC1 in
  [questions/widget-toolkit.md](../specs/questions/widget-toolkit.md). No existing question answered or archived.
- Cargo, clippy and ctest: n/a; documentation-only RE lane. No game runs or shadow comparison requested.
- No engine/, plugin, capi, tools/coverage-map.tsv, docs/abi/, README or HANDOFF edits. No C ABI change or ABI rows.
- Continue WTG1 with title-bar/secondary-interface work, WTH1 with completion protocol and scrollbar/draw helpers,
  and WTC1 with construction/teardown attribution. Then tackle COM director, command support and scripting view
  in rank order, checking existing related specs before defining new contracts.

## Handoff

- Round 2 active: first finish WTG1/WTH1/WTC1, then largest catch-row owners; original report above is round 1.
- Round 2 milestone: WTC1 resolved from constructor 0095BAB3, raw secondary table and adjustment instructions; custom owner complete.
- Round 2 milestone: generator section 8 covers all 32 secondary slots, ownership, decorations, geometry, input and drawing calls; WTG1 narrowed to two helper pixel contracts. Doccheck passes before final report.
- Next exact step: query generator/custom secondary tables and constructor/teardown evidence from main export.
- Ready to land this bounded documentation increment; not a full-owner closure or 100 KB completion claim.
- Milestones: 59e23d78b initial record; 59f82984a generator; dbcc2edb2 HTML/custom.
- Final review added folded custom key callback behavior; report records all 54 owner proposals.
- Next exact step for successor: query cGZWinGen secondary table and cGZWinGenTitleBar; resolve WTG1.
- Evidence: build-runs/owners.tsv, ranked-owners.tsv, coverage.json, verify-constants.py in this worktree.
- No outstanding game requests; no ABI rows; preserve all coverage-map values until full owner closure.
