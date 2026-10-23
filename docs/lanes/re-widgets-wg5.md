| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widgets-wg5 (widget control family specs) | Codex / `re-widgets-wg5` | ready: family spec complete within documented coverage; doccheck passed | Coordinator lands queued tip; implementers use linked owner contracts and preserve WC1–WC3 residuals |

## Report

- Added [widget-controls.md](../specs/widget-controls.md), a family contract for Grid, TextEdit, ListBox, TreeView and FileBrowser. Incorporated existing owner specs by link instead of duplicating their inventories and detailed algorithms.
- Added ListBox's 66-slot list interface and window-extension inventory, construction/state, selection/index/ID behavior, row geometry, input routing, notifications, reference ownership and drawing/service boundary.
- Added TreeView's 60-slot primary interface and window overrides, construction/root/state, selection/expansion/visibility, key/wheel/bar dispatch, and interface-level drawing/lifetime boundary.
- Native checks corrected misleading ListBox export labels: W slots 158/159 remove/insert storage; public selection is L slots 10/11. Native thunk displacement is 0xD8, despite decimal-228 names. Tree primary table ends before non-pointer 0x40490FDB.
- Numeric instructions checked for ListBox binary32 constructor default, signed gutter/division geometry, equal-only color invalidation; TreeView integer/byte defaults, one-step wheel behavior (including delta zero), Init return and view offsets. Evidence is address-cited in the spec.
- Added 28 proposed discriminating conformance scenarios. They are documentation vectors, not executed engine tests or game measurements.
- Filed WC1, WC2 and WC3 in [questions/widget-controls.md](../specs/questions/widget-controls.md); regenerated QUESTIONS.md. These cover unresolved adapters, descriptor/removal/order/resource failure contracts, exact rendering and drag/modifier behavior. No existing question was moved or claimed answered.
- Validation: `python tools/doccheck.py check` passed again on 2026-10-23 in round 2; `git diff --check` passed. The previous round also passed doccheck links. Cargo/clippy/ctest and game/shadow checks are n/a for this documentation-only lane. No ABI exports or ABI files changed; no ABI row proposal.
- Scope limit: inventories do not establish whole-owner completeness. Existing Grid/TextEdit/FileBrowser parked residuals remain with their own specs. ListBox/TreeView residuals are explicit; this round is ready as a bounded spec deliverable, not a full 107 KB behavior proof.
- Tools: used re_query.py first, local decomp and native PE table words. REA was not called (brief says no binary open). q.py failed with Windows CreateFileMapping error 5; native PowerShell succeeded. Git printed a packed-refs.lock permission warning during commit, but created the detached-HEAD commit normally; shared refs were not changed.

## Completion and next steps

- Initial family map: `8b792da7`; completed spec/questions/index: `7d1d1b7b3514e35a3f25ec6a5b627053eca072f4`. Previous guard-limit handoff: `8f5eafe872c6019ec6b059b543aa185028f06c1e`.
- Round 2 found no unfinished spec sections listed in that handoff. It validated the existing deliverable and finalized this report and the merge-queue entry.
- Queue: `build-runs/merge-queue/re-widgets-wg5.json`, kind `re`, state `ready`, ABI false, no game runs. Its tip is the full final detached-HEAD commit; the supervisor publishes it to the reserved lane branch.
- Implementers should use widget-controls and its linked owner specs. Future RE work should resolve WC1–WC3 and the linked owner residuals before claiming full coverage or pixel-exact behavior.
- No engine reads/edits, ABI edits, game requests or new RE queries in round 2.
