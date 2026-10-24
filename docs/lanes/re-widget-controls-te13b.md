| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-widget-controls-te13b: TextEdit and FileBrowser closure | Codex; re-widget-controls-te13b | ready: partial closure; doccheck passed | Implement settled rules; residual TE5/TE8, FBQ2/FBQ5 remain parked |

## Report

The requested owner closure tables were read first. No engine, ABI, plugin,
capi or coverage-map files were changed. All findings use the local export
through re_query.py and selectively read decompilation; REA was not called.

- `5000f7668`: reconciled TE9–TE12. Their rules, vectors and answered records
  already existed in the baseline under re-widgettk-wt13, while duplicate open
  copies remained. Removed those copies without duplicating/re-attributing the
  answered entries. Added the missing explicit history/preferred-X default
  vector. Verified constructor stores and signed shared-counter comparison
  against assembly (`009BFF6E`, `009BF98B`).
- `71ba3ab06`: extended text-edit-encoding with the two UTF16 adapter contracts
  and six vectors. The byte shortcut is strictly below DEL and leaves the
  caller's count untouched; conversion requests use one output unit. Verified
  against instructions at `009115F0–0091169E`.
- The same commit extends win-filebrowser 8.1/8.1.3: false wildcard option
  lowercases both copies; unsigned height/row-height division controls scrollbar
  reservation; enabled widths are subtracted without a lower clamp; pending
  restoration uses the first matching sorted row and otherwise row zero,
  including empty-grid selection requests. Verified arithmetic and branches at
  `009A51E2–530D`, `009A530D–55B2`, `009A56DE–5744`, `00918F08–2F`.
- GZPaint `009A0A17` is fully described in section 8.5: exact background-call
  rectangle, primary/secondary offsets, refresh ordering and final dirty clear.
  FBQ2's residual denominator is reduced from 3 / 4009 B to 2 / 3836 B.
- Six browser vectors FB-C28–33 cover these findings. The inventory continues
  to mark UpdateFileList partial; no whole browser/TextEdit owner credit is claimed.

**Questions:** TE9–TE12 are reconciled with their existing answered records.
TE5, FBQ2 and FBQ5 retain their IDs with narrowed descriptions; TE8 is unchanged.
No new numbered questions were filed, because the unresearched remainder is
not evidence of something impossible to settle. TE5 still needs conversion,
Thai and composed-length results; TE8 still needs allocation/unwind and the
listed foreign outcomes; FBQ2 still needs AddControls and DoMessage; FBQ5 still
needs directory/drive population and remaining file-row configuration. These
are partial closures within this round, not assertions that the code cannot
answer them. No fourth-round restart is requested.

**Gate:** `python tools/doccheck.py check` passed; `git diff --check` passed.
Gate output: `build-runs/doccheck-re-widget-controls-te13b.log`. Questions index
regenerated with `python tools/questions.py index`. Cargo, clippy, ctest and
game runs are n/a for this spec-only lane; no pixel/shadow verdict is claimed.
Git emitted a packed-refs.lock permission warning on commits, but both commits
completed on detached HEAD; no refs/config were changed manually.

## Handoff

- Ready partial specification closure; no active game requests or ABI proposals.
- Implementers can consume the added vectors and reconciled TE9–TE12 immediately.
- Remaining research is exactly the four retained question IDs above, not new
  duplicates. Budget was reserved for verification/reporting rather than
  promoting unreviewed helpers or completing the larger AddControls body.
- Detached HEAD is intentional; the supervisor publishes commits to the lane.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-widget-controls-te13b-r1-manifest.json.
