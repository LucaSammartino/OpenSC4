| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-pr-harvest5: contributor spec harvest | Codex; re-pr-harvest5 (reserved detached HEAD) | ready: eleven selected contributor commits harvested; clean-room review and doccheck pass | Merge train publishes the actual HEAD to the reserved lane branch and lands it. |

## Report

Completed 2026-10-23 against pre-harvest main `66f935e6`. Ten selected PR10 commits
and PR9's single commit are present in the requested order, with original authors
and `cherry-pick -x` trailers. The round-1 reboot checkpoint is `dc6e9729`; round-2
behavioural rewrites began at `dccfbcec`. This report accompanies the final review commit.

### Commits taken

| Source | Harvest commit | Material |
|---|---|---|
| PR10 `46ecc345` | `a8dc40e6` | occupant filters |
| PR10 `8853d10c` | `5e8196c6` | 3D window commands, panels and displays |
| PR10 `5517422f` | `ec33f456` | quad grid and rendering passes |
| PR10 `128498f4` | `5a106945` | query tool dialogs and values |
| PR10 `4048a59f` | `4ee7425d` | lot placement and facing |
| PR10 `f02a528f` | `c6ac2b76` | budget sheet, taxes and loans |
| PR10 `47f2fde4` | `9b7befdc` | model maker and background loader |
| PR10 `9087568e` | `16713743` | application preferences, resources and plugins |
| PR10 `13cf2b33` | `788f2a53` | signposts and TextDecal interface supplement |
| PR10 `c95be639` | `5cf8acf6` | construction lot division, cranes and cleanup |
| PR9 `e968dbd1` | `229aeb6b` | region/city dialogs and obliteration |

Deferred as requested: PR10 `a8b30439` and `a33fa02e` (network/underground occupant
specs under active extension in another lane); `a12daabf` (drawing-fonts under
implementation in an engine lane). No fetch, push, GitHub action, merge or shared
ref update was performed.

### Conflicts and preservation

- Kept main's existing spec sections and added contributor detail in their owning
  sections. Main's newer construction contract remains. The two renamed original
  headings retain their material: budget-window-bodies §4.2 now has the complete
  display mapping; lot-developer's old §6.7.10 Open is now §6.7.15 Open.
- Main's dedicated `text-decal-occupant.md` and all vectors remain authoritative.
  Omitted the contributor's superseded duplicate TextDecal behaviour, including its
  older colour threshold; retained the new interface supplement in lot-records.
  Main's OP1 and OP2-TD answers remain; the signpost OP2 receiver/stream question
  moves to the archive with the contributor's evidence.
- Regenerated `docs/specs/QUESTIONS.md` with `tools/questions.py index` rather than
  hand-merging. `INVENTORY-M6.md` remains unchanged; the round-1 tools search found
  no inventory generator. Existing answered-question text is preserved.
- Unioned RE symbol/name tables, retaining main's exact row on every duplicate
  key. The audit verifies every pre-harvest key still has its original row.
- Kept coverage row order, keys, classification and every non-spec cell exactly.
  Omitted contributor row additions, key renames and reclassification so first-match
  semantics are unchanged. No engine, plugin, capi or docs/abi edits; no ABI exports
  changed and no ABI rows proposed.

### Clean-room review

Reviewed the complete harvested spec additions (84 review pages) and question
additions (26 pages) against CONTRIBUTING.md Specs. Retained behaviour, equations,
orderings, constants, record layouts and short interface/address facts. Rewrites:

- region-city-dialogs §5.5.1: replaced the conditional/loop/goto block with removal
  scheduling, batch, sparkle and state-transition rules, preserving its examples.
- occupant-manager §13.2.8: replaced temporary-result branch tracing with type/group
  acceptance predicates and explicit exceptions.
- occupant-manager §13.2.3: replaced the inherited demolition-filter branch/mnemonic
  summary with field roles and the authoritative demolition.md §4.2 contract;
  retained bounds, privilege, filters, types, network mask and destructor facts.
- occupant-manager filter evidence: removed instruction-mnemonic snippets.
- city-tools-input §3.10: restated the placement test as ordered requirements and
  output effects, preserving early-failure precedence and cost/message rules.
- Archived OP3: replaced a register-indexed load phrase with the selected-row fact.
  Added missing MATCHES/DIFFERS markers to imported answers, retaining attribution.
- Escaped the filename-rule table's literal pipe for correct Markdown rendering.

### Coverage-map spec cells

Exactly eight existing rows changed, only column 4:

| Row | Previous spec | Final spec |
|---|---|---|
| cSC4ConstructionOccupant | lot-records.md | lot-records.md, lot-developer.md |
| SC4QuadGrid | - | render3d.md |
| cSC4NewRegionDialog | - | region-city-dialogs.md |
| cSC4ImportCityDialog | - | region-city-dialogs.md |
| cSC4EstablishCityWinProc | - | region-city-dialogs.md |
| cSC4ObliterateCity | - | region-city-dialogs.md |
| cSC4View3DWin | - | view3d-window.md, ui-windows.md, view3d.md |
| cSC4SubBudgetWindow | budget-window.md | budget-window.md, budget-window-bodies.md |

### Validation and questions

Final checks: `python tools/doccheck.py check` PASS; `python tools/questions.py
check` PASS after index regeneration; `git diff --check` PASS. The read-only
`build-runs/harvest-audit.py` passes scope, exact coverage preservation, prior symbol
rows, prior answered-question lines and all eleven source-author/-x checks.
No vectors were changed. Cargo, clippy and ctest: n/a for this spec/RE harvest.
Game runs: none; no new runtime/shadow verdict or fresh numeric RE measurement is
claimed. Contributor measurements in their imported lane reports are historical.

Imported open questions: AP10–11 (application); BW18 (budget-window); CT14–17
(city-tools-input); MM1–2 (model-maker); OM9 (occupant-manager); OP18–19
(occupants-props); VW7–8 (view3d-window); CY1–12 (region-city-dialogs); RM13–14
(region-model); QR4 (region-screen), all under `docs/specs/questions/`.
Imported/archived answers: CT3/5/8/10/12, OM10, OP2/3/4/5/8/9/17, TV1 and VW2–5
in the corresponding `docs/specs/questions/answered/` files. Remaining uncertainties
stay open; this lane adds no new RE investigation.

Next: merge train lands this batch; owners can pursue the open questions and the
three deferred commits independently. Queue: `build-runs/merge-queue/re-pr-harvest5.json`,
kind `re`, state `ready`, actual final HEAD. The worktree intentionally remains
detached; the supervisor publishes the commits to the reserved lane branch.
