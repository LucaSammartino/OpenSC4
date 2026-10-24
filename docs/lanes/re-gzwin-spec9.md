| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwin-spec9: widget owner contracts | Codex; `re-gzwin-spec9` (reserved lane, detached HEAD) | ready: round 2 docs; custom complete, generator expanded; doccheck passed | Land increment; finish WTG1 helper pixels and WTH1, then largest unworked owners. |

## Report

### Delivered contracts

Round 2 extends the specifications linked from [widget-toolkit.md](../specs/widget-toolkit.md).
The HTML contract is retained from round 1; this round changes Custom and Generator:

- [widget-generator.md](../specs/widget-generator.md): primary overrides and helper inventory, construction defaults,
  procedure lifetime and notification order, the mouse/nonmouse dispatch distinction, client rectangle calculation,
  layout negotiation and final-placement boundary, capture and drawing responsibilities. A handled procedure mouse
  message still reaches base dispatch. A failed procedure Init follows partial initialization rather than rollback.
  **Round 2 adds section 8:** all 32 secondary slots with corrected getter/setter attribution; raw/indexed/keyed
  data and bitmap reference ownership; gutter truncation and ignored arguments; title/side/color-bar creation,
  theme lookups, failure paths and placement; generator flag side effects; asymmetric inverse geometry,
  bounds precedence and packed picking; edge/drag/cursor behavior; exact close records; and bitmap,
  fill, outline and procedure drawing calls. Class-creation failure in AddColorBar returns true without a child;
  dragging commits through base SetArea and bypasses generator constraints/layout; nonmodal close posts type 1
  to the parent and can be intercepted, rather than directly deleting the window.
- [widget-html-window.md](../specs/widget-html-window.md): primary overrides and method families, document/service
  state, parse-before-install replacement, status notifications, local cancellation, signed scroll clamping, viewport
  updates and the synchronous layout/paint boundary. Cancelling a loading request returns false; cancelling again
  returns true. HTML completion-target DoMessage is distinct from primary window dispatch.
- [widget-custom.md](../specs/widget-custom.md): parent forwarding mutates the input record only for types 7, 10 and
  13; direct key callbacks instead send fresh records to the notification target. Also specifies QI null-output and
  the fill-drawing boundary. **Round 2 completes WTC1:** constructor 0095BAB3 adds only tables to base state;
  the four-slot secondary interface uses +0xD8 and shares primary lifetime; nondeleting teardown calls base
  Shutdown then base destruction. AsIGZWin borrows the complete pointer. Export thunk names suggesting 228
  bytes are contradicted by the measured 216-byte adjustment.

There are **50 conformance vectors** across the three files: 31 generator, 9 HTML and 10 custom.
Round 2 adds **25** (21 generator and 4 custom). Existing child/parent, z-order, base message/input and
buffer/presentation contracts remain in widget-toolkit.md; links avoid duplicating those rules. HTML parsing is
linked to html-document.md and modal/generated window integration to modal-dialogs.md and ui-windows.md.

**The overall assignment remains incomplete.** Round 2 finishes **one owner, cGZWinCustom (245 weighted
bytes)**, and expands cGZWinGen (7,049 weighted bytes); it does not claim that those 7,049 bytes are newly fully
specified. The requested additional 60+ KB was **not reached**. Generator remains partial for two external
drawing helpers' pixel contracts; HTML remains partial under WTH1. No new large owner was opened. The spec
analysis checkpoint was 99,291 measured context tokens; the generator milestone/review checkpoint was 106,399
of 150,000 after writing and checking the result. The remaining work is preserved below rather than assigning
whole-owner credit to sampled methods. READY means this documentation increment is reviewable,
not that the ranked-owner program is complete.

### Catch-row ranking and coverage proposals

Ran main-checkout `tools/coverage.py --owners` against the Windows export and working executable, with outputs
under this worktree's `build-runs/`. **Re-ran in round 2**, producing `owners-r2.tsv` and `coverage-r2.json`;
`round2-summary.py` independently reaggregates the ranking and confirms the unchanged totals below.
Filter: subsystem `widget toolkit`, owner starts `cGZWin`, excluding the exact
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
| cGZWinGen | 7,049 | 56 | widget-generator.md; hold, WTG1 narrowed to two helper pixel contracts; section 8 expanded in round 2 |
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

Round 1 evidence checkpoints include generator construction `0099B6A5`, client calculation `0099922F`,
procedure dispatch `0099A276`, procedure notifications `009998C8`, HTML signed clamping `008FAD0E`, ready-state
installation `008FBF73`, cancellation `008FA9A7`, ready-only painting `008FB3F5`, custom coordinate mutation
`0095B9F8`, QI `0095B9C1`, and direct key callbacks `009CD75C/009CD789`. Evidence helpers, ranking TSV and
coverage outputs remain ignored under build-runs/. No decompiler output or assembly was committed.

Round 2 adds raw secondary-table verification from the read-only working executable, plus instruction checks
for custom construction/teardown `0095BAB3/0095B9A1`, shared-reference thunks `0095BA76/81/8C`, generator
secondary table `00ADC5F8`, gutter overloads `0099919B..009991D6`, keyed-data replacement `0099A90B`,
inverse title expansion `00999301..00999317`, resize-mode selection `00999C15`, direct base drag placement
`0099A509`, decoration creation `009999BA/0099A96E`, bitmap draw calls `009995E7`, and nonmodal close
posting `009993F7/0099CA60`. `read-image.py` and `source-page.py` are ignored evidence helpers in build-runs/.

In round 1 the q.py Bash wrapper failed to start (CreateFileMapping access denied), so direct capped PowerShell commands
were used. A qgrep invocation failed on cp1252 output; subsequent use selected UTF-8. Neither failure is REA
evidence. Git reported inability to create shared packed-refs.lock after commits, but each detached-HEAD commit
completed and its actual HEAD was verified. Shared refs/config were not changed.

Some auto-matched cGZWinBMP methods at 007B/007C appear to describe a different owner. Lua console sampling has
broken stack recovery. Those round-1 samples did not become behavior rules. The HTML secondary tables,
generator drawing-helper pixels and many untouched owner constructors still need analysis. Generator and
Custom secondary tables are now specified; no Lua/BMP sample became a new rule.

### Validation, questions and next work

- `python tools/doccheck.py check`: **PASS** on the delivered docs.
- `python tools/questions.py index` and `python tools/questions.py check`: **PASS**. WTC1 answered in
  [questions/answered/widget-toolkit.md](../specs/questions/answered/widget-toolkit.md); WTG1 narrowed to
  helper pixels in [questions/widget-toolkit.md](../specs/questions/widget-toolkit.md). WTH1 unchanged.
- `git diff --check`: **PASS**. No new question IDs filed. The round-2 coverage rerun verifies inventory,
  not an increase in coverage; tools/coverage-map.tsv was not changed.
- Cargo, clippy and ctest: n/a; documentation-only RE lane. No game runs or shadow comparison requested.
- No engine/, plugin, capi, tools/coverage-map.tsv, docs/abi/, README or HANDOFF edits. No C ABI change or ABI rows.
- Continue WTG1 with RZDrawRaisedRect 008D9900 and RZDrawBltEdge 008D8800; link their pixel contracts to
  generator section 8.5, then decide owner closure. Complete WTH1's HTML completion protocol, constructor,
  secondary tables and scrollbar/draw helpers. Then tackle COM director, command support and scripting view
  in rank order, checking existing specs before defining contracts. Base cGZWin remains held for its open questions.

## Handoff

- Ready to land a bounded round-2 documentation increment; 60+ KB target remains unmet.
- Milestones: c2c72fa48 completes Custom/WTC1; 7720666c7 expands Generator section 8 and passes doccheck.
- Next exact step: query RZDrawRaisedRect 008D9900 and RZDrawBltEdge 008D8800; specify helper pixels and close WTG1.
- Then complete WTH1 before opening COMDirector/command support/Lua view in ranked order.
- Ranking unchanged: 54 owners, 1,548 records, 164,976 bytes; Custom alone newly complete (245 bytes).
- Evidence: build-runs/owners-r2.tsv, coverage-r2.json, round2-summary.py, read-image.py, source-page.py.
- WTC1 archived; WTG1 narrowed; WTH1 unchanged. No ABI rows or game requests.
- Propose widget-custom.md only for cGZWinCustom; hold Generator/HTML/base; never credit the broad catch row.
