| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dispatch-completion4: DP10–DP13 native contracts | `re-dispatch-completion4` / (RE lane, detached worktree) | in progress: round 2 tracing DP10–DP12 residual contracts; DP13 already answered | Finish native view update, pylon string prefix and station spawn/route ownership contracts. |

## Handoff

- Goal: finish DP10–DP12 exact native contracts in one guarded round; no engine/tools/ABI edits.
- Prior milestones: f1135c91 (DP13); c83804c4 (pylon geometry, caller descriptors, view tracking).
- Done this round: DP11 resolved at 75f733f3; string-interface copy, prefix/return-width audit and independent vectors published.
- Done: native traffic slot18/17 creation/route/cleanup chain and first-road heading; view Update gates, delayed removal, refresh and texture keys published.
- Next: remaining DP12 constants/path-selector inputs and numeric cell conversion; DP10 screen-rectangle/call-display/draw submission.
- Newly verified quirk: non-traffic slot17 rejection leaks the acquired pathfinder; normal routing exits release it.
- Residuals: DP10 renderer update/delayed untrack; DP11 bitmap string input/prefix; DP12 road spawn/route failure.
- Public glue reports two unsaved viewHandle differences; no original runtime trace requested.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-dispatch-completion4-r2-manifest.json.
- Detached HEAD is intentional; commit normally without updating refs. No subagents/restart/network.

## Report

This bounded round is incomplete: DP13 is answered; DP10–DP12 remain open with narrower, explicit gaps.
No complete-owner, supported replacement, exact-shadow, default or runtime promotion is claimed.

Published DP13 in dispatch §4.5.1 and protection §8.4, archived the question and regenerated its index
in milestone f1135c91. Both air slot-6 entries point to 006E0010, which calls 006DFC70 and AddRefs
each copied occupant. Destination node clear 00413E60 does not release old elements. Slot 7 writes
count, masked mask, base with partial writes preserved; count32/33 yields mask0/1. Native instructions
verify widths and reference operations. The public crash correction remains preserved in the archive.

Published substantive further contracts in dispatch §§4.1.1–4.1.3 and §4.2:
- Pylon factory 005F2060 / class49B6D69B / IID C9B6D7D6; occupant adjustment+8, complete primary
  slot numbering, source/owner acquired references, lifecycle and exact call order.
- Placement uses station occupant GetBoundingCityCells and city CellCornerToPosition, not a lot
  interface. BeginTool's single-unit shortcut differs from return. Return wraps call-base to8bits;
  the drop predicate uses half-open X/Z bounds and ignores Y. Numeric x87/store order is cited.
- Vehicle view is a concrete primary object, unknown at+4, message target at+12. Tracking is direct
  TrackOccupant, not an invented SDK virtual. Successful handles increment process-global B215FC;
  renderer registration/unregistration and retain/untrack boundaries are published.
- BeginDispatch source output is modelKey, not spawn position. New slot18 uses seven arguments
  and native tag0/station, tag1/position descriptors; retarget slot17 uses tag0/null and mode2.
  The similarly named 0054F8A0 is a different method, not the creation slot used here. Source AL
  is ignored; manager selection can fall through returning its last rejected candidate.

Remaining DP10: complete Update/refresh argument protocol, visual resources and delayed-untrack
completion. Remaining DP11: SetBitmapInfo's string input representation and final prefix audit.
Remaining DP12: internal station-to-road spawn/heading and full route failure/ownership protocol.
Exact caller layouts do not settle those remaining owner rules; leave unsupported callbacks unwired.
Independent distinguishing vectors for all four question areas are prose requirements in §12,
not claimed executions or fitted native outputs.

Validation: docs-only; git diff --check passed. No cargo/clippy/ctest or game run was warranted or
requested. Public glue report used only to identify gaps (1694/1696 shadow values, two view-handle
differences); no implementation/test outputs read as contract. No engine, tools, maps or ABI table
edits; no ABI proposals. Read-only export/decomp tools sufficed; REA was discovered but not invoked.
q.py failed with Bash/Service/CreateInstance/E_ACCESSDENIED; bounded native PowerShell reads and
configured re_query.py were used. Git emitted a packed-refs.lock permission warning during detached
commit f1135c91, but created the commit successfully; no ref/config/hook changes were attempted.

No game request is outstanding. No subagents, restart, network, merge or push occurred. One guarded
round only; coordinator review is the next step, not an automatically scheduled extra round.
