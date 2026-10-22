| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dispatch-completion4: DP10–DP13 native contracts | `re-dispatch-completion4` / (RE lane, detached worktree) | WIP: DP13 answered; substantive DP10–DP12 boundary/geometry contracts published; diff check passed; no runtime/ABI changes | 1. Review remaining DP10–DP12 contracts listed below before implementing the native providers. No replacement promotion. |

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
