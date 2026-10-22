| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-final-gaps4: advisor activation contracts | `C:\Users\tlori\FreeCity4\.claude\worktrees\re-advisor-final-gaps4` (branch `re-advisor-final-gaps4`, RE/docs) | blocked: AD12–AD15 specified; uncommitted because Git index-lock creation is denied; docs checks pass; no game runs or ABI changes | coordinator must commit the five changed docs, then enqueue; AD11/AD16 remain open |

## Report (2026-10-22, resumed round 2)

Audited the preserved `2314170e` WIP against the original export and decompilation, using byte verification for
constants, offsets, widths, conversions and call arguments. The current-main comparison of advisors.md contained
only this lane's draft changes; no reviewed advisor-spec edits needed copying. No merge or main-checkout edits.

AD12–AD15 are now answered **DIFFERS**, relative to the replacements described in the public questions. No core
implementation was read. Stable spec anchors are `advisors.md#ad12`, `#ad13`, `#ad14`, `#ad15`.

- AD12: title/message GUIDs are the text resource's u32 instance. Refresh re-reads and URL-decodes Lua strings,
  parses the resource key, loads localized text (or a diagnostic on lookup failure), then invokes the global
  detokenizer in place with HTML-clean true and restores its previous mode. String getters borrow embedded
  objects and do not refresh. Empty field list versus invalid table have distinct dirty/context behavior.
  Proof: `0x00407D70`, `0x00407DA0`, `0x00800640`, `0x00407BB0`, `0x004178F0`, `0x00409300`, `0x00602FB0`;
  fallback concatenation also checked at `0x00408AE0`.
- AD13: inactive or expire-requested advice skips link execution and returns true. Otherwise the borrowed link
  bytes are executed as `DoBuffer(p,strlen(p),p)` under an owning advice context; the status is ignored, the
  previous context is restored, true is returned. Proof: `0x00407ED0`, push `0x00419840`, pop `0x00418CE0`.
- AD14: base QI answers the three interfaces and unknown 1, AddRefs once on success, and preserves failure output.
  CityPlanning adds its CLSID; MySim clears output first and adds its two IIDs. Proof: `0x0040D430`,
  `0x009457C6`, `0x00411810`, `0x00424B00`; interface thunk adjustments verified.
- AD15: Init rebuilds two index lists from `advisor_heads`, filtering advisor_type and numeric gender 1. Count,
  Set and ID contracts include defaults, invalidation, retained selection and fresh Lua lookup using the actual
  supplied key. Corrected WIP's non-number claim: numeric strings convert too. Proof: `0x0040FA10`,
  `0x0040D870`, `0x0040D3E0`, `0x0040F3A0`, `0x0040D330`; numeric conversion chain `0x00880BC0` →
  `0x008802B0` → `0x008B7640` → `lua_tonumber` / `_ftol2`. No save-format extension is specified.

Added discriminating **prose requirements** in §17 items 28–31, not executable tests or claimed passing vectors.
Moved the original four entries to `docs/specs/questions/answered/advisors.md`; regenerated QUESTIONS.md.
Filed **AD16** in `docs/specs/questions/advisors.md` for the unproved missing/non-string text-provider edge;
removed the WIP's assumed absent-field default and program-wide `news` string claim. AD11 remains open.

Validation: `tools/questions.py index` succeeded; `git diff --check` passed. Five modified tracked files are all
docs. No engine/tools/ABI/unreconciled changes, C ABI change, cargo/clippy/ctest, game run or shadow claim.
Export/decomp contained all needed functions; REA was not invoked and no REA error occurred.

## Commit blocker and exact next step

The attempted milestone `git add` and `git commit -F build-runs/msg-re-advisor-final-gaps4-1.txt` both failed:
`Unable to create .../.git/worktrees/re-advisor-final-gaps4/index.lock: Permission denied`.
The directory ACL has explicit sandbox-user write-deny entries. The prescribed q.py wrapper also fails before
Git starts (`Bash/Service/CreateInstance/E_ACCESSDENIED`). No ACLs or repository metadata were changed to work
around the restriction. No new commit or ready merge-queue record could be produced; the five-doc changes are
preserved unstaged in this worktree. Existing tip remains `2314170e`.

Coordinator next: in a context with repository metadata write access, stage these five docs, commit using the
prepared UTF-8 message file, update this lane's state, and write a `kind: docs`, `abi: false`, `game_runs: []`
merge-queue record naming that new committed tip. Do not enqueue the old tip as the completed work.


### Coordinator delivery recovery (2026-10-22)
Worker stopped after sandbox-denied Git metadata writes; guard result is preserved. Coordinator reviewed the bounded docs diff and committed these exact five files outside the worker sandbox. Static proofs and remaining questions above are unchanged. This is docs-only delivery, not a new lane round or runtime validation. Queue records the committed tip.
