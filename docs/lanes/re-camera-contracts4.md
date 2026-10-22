| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-camera-contracts4: CA3–CA6 native contracts | Codex / `re-camera-contracts4` (RE spec lane, detached worktree) | in progress | 1. Verify native owner inventory and CA3–CA6. 2. Publish contracts and independent vectors. |

## Handoff

- Goal: resolve only camera CA3–CA6 using native export/bytes; no engine reads or edits.
- Done: first native proof: constructor box is ±1, remembered target/up both zero, counters 1/0; degenerate look-at has unchecked normalization; Read mutates loaded/fields before failure. Added discriminating prose vectors.
- Milestones: `8cc181db` native defaults and partial reads; `26e3247c` lifecycle, commands, directed scroll and discriminating vectors.
- Next: final documentation checks/report/queue; no further RE round. CA3 perspective-entry residual remains precise and open.
- Done: explicit native owner inventory including all 4/7 vtable slots and folded aliases; CA4–CA6 archived as DIFFERS, CA3 narrowed; generated question index passes; coordinator qualification reconciled.
- Corrected additional primary-byte discrepancy: constructor yaw base 0xBEC90FDB (−pi/8), so rotation 0 yaw is 3pi/8, not pi/4.
- New facts: integer SetZoomAndRotation calls SetCustomMagnification before equal-pair rejection; rotation-only does not call it. Invalid GetRotationCorner writes only y=0, preserving output x/z.
- Main: `C:/Users/tlori/FreeCity4`; worktree: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-camera-contracts4`.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-camera-contracts4-r1-manifest.json`.
- No ABI changes or game requests; CA1 remains outside this round. Complete-owner/runtime credit stays withheld pending proof.
