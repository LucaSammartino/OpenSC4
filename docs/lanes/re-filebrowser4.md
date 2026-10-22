| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-filebrowser4: partial cGZWinFileBrowser spec and exact export inventory | `re-filebrowser4` / (RE lane; detached worktree) | ready for docs review; 174 inventory entries / 28,907 B; 27 conformance cases; FBQ1-FBQ6 open; docs checks pass; no ABI or game changes | 1. Resolve FBQ1-FBQ6 before complete-owner credit. |

## Report

Published [win-filebrowser.md](../specs/win-filebrowser.md). This is a partial
browser contract supported by export/decomp evidence and instruction checks.
It does not claim complete-owner coverage, implementation or runtime verification.
No engine, tools, ABI, unreconciled rows, renderer, Grid or Legacy files changed.
No game files or decompiled bodies were committed. No subagents, merge, push or
branch switch was used.

### Audit and reconciliation

- Preserved tip `6e85917e` contained no draft spec or implementation, only the
  lane report and ignored candidate inventory. Nothing was accepted as verified.
- Preserved set: 100 candidates / 25,579 export-defined bytes, including reach,
  neighbour, container and possible export-boundary artifacts.
- Primary interface table `00ADD190`: 107 slots / 106 distinct targets. It ends
  before `00ADD33C`, a separate comparator pointer, not browser slot 107.
- Found 74 primary targets missing from that set. Published union: 174 distinct
  entries / 28,907 export-defined bytes. Every row has a contract section or
  open-question destination. Inventory is not behavior credit.
- Secondary window table `00A8D000`: 151 exported slots. Primary table is absent
  from the ABI table; no speculative signatures or ABI rows were submitted.
- Named widget/HTML/Legacy/UI specs and named question files had no differences
  between the starting tip and local main reference. Their reviewed contracts
  remain untouched. The new spec section 10 narrowly reconciles the older
  ui-windows section 14 exclusion for this assigned owner.
- WT1/WT2/WT3/WT5/WT6 and HD7 concern other owners; none was answered from browser
  evidence. No answer archive was read or changed.

### Contracts established

Interface identity/pointer adjustment; directory existence and parent navigation;
path notification order; ambiguous zero from row lookup and its signed/wrapping
scan bound; browser flag replacement and the add-flags intersection quirk;
selection tags/state and distinct change/double-click notifications; programmatic
selection without a file-existence check; new-directory suffixes through 256,
including attempted creation of an occupied final candidate.

Byte checks corrected a misleading payload interpretation: primary slot 90
fills a 20-byte descriptor containing type, ID and three borrowed string-object
pointers. Selection notification payload points to that temporary descriptor,
not to a string. Descriptor lifetime and string ownership are explicit.

Also specified file-list flag/filter gates, accept-any-pattern behavior,
filter-index validation and deferred dirty/invalidate behavior; icon/button
retaining setters with unsigned limits, same-pointer no-op and AddRef-before-
Release order; icon loader dimensions, strip arithmetic, failed-lock behavior,
release and return channels; default-button initialization-failure release/null.

Large bodies have explicitly partial contracts: AddControls (3,149 B),
UpdateFileList (2,241 B), default-icon drawing (3,423 B), button drawing (1,938 B).
Exact pixels, full layout/dispatch, comparator, constructor defaults and
remaining primary slots are still unproved.

### Validation and evidence

- `python build-runs/fb-validate.py`: PASS. Cross-checks all 174 published
  addresses/lengths/slots against captured export metadata; totals, 107 slot
  positions, 106 primary targets, 27 consecutive vector IDs, six question IDs,
  local links and prose-only output.
- `python tools/questions.py index` and `python tools/questions.py check`: PASS.
- `git diff --check`: PASS. Changes limited to the new spec, its question file,
  generated question index and this lane report.
- FB-C01-FB-C27 are proposed discriminating contract cases, not native runtime
  results. Cargo, clippy, ctest, game runs and shadow comparison: n/a.
- Queried static export before cached decomp reads. Numeric branches, widths and
  order were checked with `asm --bytes`; exact anchors are beside the rules.
  No REA request was needed for the cached evidence.
- `q.py` could not start Bash (E_ACCESSDENIED); used capped native PowerShell.
- Ignored evidence/helpers stay in this worktree's build-runs: fb-audited.tsv,
  fb-vtable.txt, original fb.tsv, fb-read.py, fb-inventory.py, fb-validate.py.
  None is committed.
- Commits succeeded despite a denied optional packed-refs.lock maintenance
  attempt. Detached HEAD was preserved; actual HEAD is the queue tip.

### Questions and remaining work

[questions/win-filebrowser.md](../specs/questions/win-filebrowser.md): FBQ1 pixel
recipes/return channels; FBQ2 full control/layout/dispatch; FBQ3 constructor and
complete lifecycle; FBQ4 remaining primary methods/filter ownership/deferred
selection; FBQ5 sorting/drive lists/width arithmetic; FBQ6 attribution artifacts.
All six remain open. Index regenerated. No existing proved ID was moved.
Resolve these before claiming a fully specified owner or fitting pixel/order/
selection tests to an implementation.

### Commits

- `6876d42d`: interfaces/navigation/lookup and initial conformance cases.
- `c02f2186`: selection/flags/mkdir/large-body partial contracts, exact inventory,
  explicit questions and generated index.
- Final commit: resource/filter ownership, 27 conformance cases and this report.
  Queue is ready for this bounded docs publication; native owner remains partial.
