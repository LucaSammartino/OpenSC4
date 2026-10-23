| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| tools-coverage-owner-audit4: public owner closure audit | Codex; tools-coverage-owner-audit4 (detached reserved worktree) | in progress | Reproduce coverage accounting and audit owner completion obligations for trains100-108. |

## Handoff
- Goal: explain nine flat trains using public coverage policy, implementation, specs and landed reports; identify actionable owner closures.
- Scope: own report and ignored evidence only; no engine changes, coverage input changes, promotion or original RE material.
- Done: initial handoff 7bde56f5; reproduced all headline sums independently from generated per-function accounting; pinned107 full report matches normalized published file.
- Evidence: build-runs/audit_accounting.py, accounting-reproduction.json (full input hashes), owner-accounting.json, owner-audit-{report,pinned107}.md; all ignored, original analysis contents never displayed.
- Next: fix train report discovery to compare whole train ranges; read landed reports and spec sections for owner obligations; write compact closure table.
- Facts: denominator 29,536 functions / 5,584,842 bytes. Specified 3,226,906/14,602; implemented 2,018,463/8,982; verified 1,099,943/4,325; replaced 1,012,661/3,899 (bytes/functions).
- Correction: whole-train comparisons show train100 +8,437 implemented bytes /109 functions (PersistResourceManager); train101-108 all four byte deltas zero. Comparing only a final merge-resolution commit had missed train100's earlier mapping change.
- Facts: default now selects train108, whereas published report pins train107; changed raw shadow counts do not change headline statuses. No arithmetic defect demonstrated.
- Supported mapping-lag candidates: cRZDirectory 4,385 B /20 functions and cHTMLDocument 38,218 B /162 functions have complete-published-owner reports but map core=-. Native glue is NOT an implemented prerequisite in policy.
- Real gaps: BudgetWindow same-class callbacks, audio event/manager incomplete dispatch, watercraft WC3-WC7, snapshots SN1-SN9 and six partial persistence runtime owners. Do not promote these by path existence.
- Caps: GZ messages public report logical2/3 => one completion round remains; persistence runtime/player-drive/LegacyControl/WWW public final3/3 => zero. Others UNKNOWN unless their public report says otherwise.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-tools-coverage-owner-audit4-r1-manifest.json.
- No game requests, external communications, subagents or questions.

## Findings checkpoint

The current tool exactly sums the published owner map. Its `implemented` flag checks only the listed paths; it does not discover new modules or require a game/shadow. Thus unchanged rows make newly landed portable work invisible. The substantive distinction is whether a whole published owner is complete, not whether a DLL adapter exists.

The whole-train correction above is retained: train100 really moved, through the `cGZPersistResourceManager` row; train101-108 did not. The published full report uses train107 evidence, and reproduces exactly with that run pinned. A default train108 reproduction changes evidence counts but not the four headline totals. Ignored full input SHA-256s and independent owner rollups are in `build-runs/accounting-reproduction.json` and `build-runs/owner-accounting.json`.

For selection, highest-byte gaps include BudgetWindow (73,268), Watercraft (38,073), Helicopter (35,775), Airplane (34,889), MapView (31,396), ConstructionOccupant (26,539), SnapshotAlbum (23,300), AmbienceManager (11,298), Lua collector (9,045), AudioEventHandler (8,877), LuaThread (7,303), AudioSoundManager (6,909), DBSerialRecord (6,120), and MessageServer (5,796). These are owner sums, not promises of incremental credit. The closure table still needs exact missing operations and source/spec citations; existing capped families must not be restarted or renamed.
