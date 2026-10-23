| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets-wg5 (widget control family specs) | Codex / `re-widgets-wg5` | in progress: guard context limit after completed spec and validation; queue not written | Fresh turn: finalize ready report/queue using actual HEAD; preserve explicit WC1–WC3 residuals |

## Report

- Added [widget-controls.md](../specs/widget-controls.md), a family contract for Grid, TextEdit, ListBox, TreeView and FileBrowser. Incorporated existing owner specs by link instead of duplicating their inventories and detailed algorithms.
- Added ListBox's 66-slot list interface and window-extension inventory, construction/state, selection/index/ID behavior, row geometry, input routing, notifications, reference ownership and drawing/service boundary.
- Added TreeView's 60-slot primary interface and window overrides, construction/root/state, selection/expansion/visibility, key/wheel/bar dispatch, and interface-level drawing/lifetime boundary.
- Native checks corrected misleading ListBox export labels: W slots 158/159 remove/insert storage; public selection is L slots 10/11. Native thunk displacement is 0xD8, despite decimal-228 names. Tree primary table ends before non-pointer 0x40490FDB.
- Numeric instructions checked for ListBox binary32 constructor default, signed gutter/division geometry, equal-only color invalidation; TreeView integer/byte defaults, one-step wheel behavior (including delta zero), Init return and view offsets. Evidence is address-cited in the spec.
- Added 28 proposed discriminating conformance scenarios. They are documentation vectors, not executed engine tests or game measurements.
- Filed WC1, WC2 and WC3 in [questions/widget-controls.md](../specs/questions/widget-controls.md); regenerated QUESTIONS.md. These cover unresolved adapters, descriptor/removal/order/resource failure contracts, exact rendering and drag/modifier behavior. No existing question was moved or claimed answered.
- Validation: `python tools/doccheck.py check` passed. Cargo/clippy/ctest and game/shadow checks are n/a for this documentation-only lane. No ABI exports or ABI files changed; no ABI row proposal.
- Scope limit: inventories do not establish whole-owner completeness. Existing Grid/TextEdit/FileBrowser parked residuals remain with their own specs. ListBox/TreeView residuals are explicit; this round is ready as a bounded spec deliverable, not a full 107 KB behavior proof.
- Tools: used re_query.py first, local decomp and native PE table words. REA was not called (brief says no binary open). q.py failed with Windows CreateFileMapping error 5; native PowerShell succeeded. Git printed a packed-refs.lock permission warning during commit, but created the detached-HEAD commit normally; shared refs were not changed.

## Handoff

- Goal: specify Grid, TextEdit, ListBox, TreeView and FileBrowser without engine edits.
- Done: initial map `8b792da7`; completed spec/report/questions/index `7d1d1b7b3514e35a3f25ec6a5b627053eca072f4`. Doccheck check, links and git diff --check passed; tree clean at that tip.
- Next: fresh turn should finalize row as ready and write build-runs/merge-queue/re-widgets-wg5.json with actual 40-character HEAD, kind re, abi false, game_runs [], state ready. No further RE is necessary for this bounded deliverable; WC1–WC3 remain explicit.
- STOP: guard denied queue creation at 150k context; no merge-queue file was written. Return continue. Detached HEAD; supervisor publishes reserved branch. Do not switch refs.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; export available via re_query.py. No REA needed or available binary.
- Tool note: q.py Bash wrapper fails with Windows CreateFileMapping error 5; use native PowerShell with capped output.
- No ABI edits, game requests or engine reads.
- Critical: ListBox export labels are shifted/misleading. Native secondary table 00AE1678 at W+0xD8 gives actual SetSelection slot 10=009C8CEE; window slot 158=009CA0DF removes an entry. Tree primary 00AE3240, window 00AE2FE0 at +4; unnamed ctor 009D2BF0.
- Ignored helper build-runs/inspect-widgets.py reads a function by address with 60-line cap or raw PE table words. Anonymous pseudocode is decomp/_unnamed/region_009c0000.c and region_009d0000.c.
