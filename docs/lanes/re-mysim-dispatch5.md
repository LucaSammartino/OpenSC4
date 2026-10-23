| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-dispatch5 (dispatch trip contract, MS17 parts) | Codex / detached reserved worktree `re-mysim-dispatch5` | ready: documentation milestone; whole dispatch remains incomplete | Land settled contracts; RE follow-up MS18, then engine dispatch completion |

## Report

Extended mysims sections 1.1, 3.3, 4.6, 4.11, 6.2, 7.2, 8 and 10. The dispatch is now assigned to this
spec and immediate-arrival stubs are no longer permitted. This is a **partial answer** to MS17, not a
claim that the entire dispatch owner can now be completed.

Settled contracts:
- Separate current/home/active/commute locations; queue replacement/front/back insertion; recovery,
  at-location selection and settled path-failure/player-driving transitions in a transition table.
- Synchronous owner notification envelope, changed-mask delivery and flag-before-state order on entering 3.
- Five-minute simulation-clock messages, separate animation progression and strict deadline expiration.
- Wait getter's timestamp quirk: with switch flag clear, timestamp 100 survives at now 115 but clears at 116.
- Five tuning field uses, occupant-only minimum duration, strict cell-radius approach test.
- LeaveForWork/ReturnHome request state gates and hour-dependent durations (hours 7/8/17/18 → 5/540/0/5),
  GoTo redirection, U-Drive-It guard and active/pending entry/exit effects.
- Complete dispatch payload layout (u32 version 1, flag, 18 words, four locations, auxiliary references,
  string, route deque, ordered model map, marker, destination queue), including inline use by MySim v12.
  Serializer receives object+0x0C; first location begins at payload byte 77. The serialized simulation
  service at object+0x120 is distinct from the transient pathfinder at +0x124.
- No residence RNG consumption for specified entry/queue/time/record operations; no dispatch RNG bytes.
- Eight named normative specification vectors in §12, one or more for every extended section. These are
  prose input/output vectors for implementers, not engine files, executed tests or native runtime traces.

Evidence: read-only Windows export queries first, then targeted main-checkout decomp slices. Numeric widths,
record prefix/containers, bool stream byte operation, hour arithmetic, deadline comparisons, approach products
and wait getter checked against `asm --bytes`; citations are in the spec. No REA queries were needed.

Validation: question index regenerated; `git diff --check` passed. No cargo/clippy/ctest or game runs: docs-only
RE work. No ABI export or ABI table changes; no engine/plugin/capi/coverage-map edits. Git emitted a
packed-refs.lock permission diagnostic during commits but created the detached commits successfully.

Questions: moved **answered parts** of MS17 to `docs/specs/questions/answered/mysims.md`; filed **MS18** in
`docs/specs/questions/mysims.md` for route-ready multimodal handoffs, stop/neighbour/recovery choices,
proximity geometry/rounding, remaining notice ordering, full dispatch-tool/mission callbacks and delegated
automata/model RNG. The engine owner must not invent these behaviors.

Commits: `f9379fb6` initial handoff; `58a2a0d2` settled dispatch specification and question disposition;
final report commit also corrects the simulation-service label and records the successful-request marker.

## Handoff
- Goal remaining: close MS18 before declaring the whole dispatch owner specified.
- Next exact RE step: trace `switchTravel` at `0x00435840` with its route-ready caller `0x004365C0`;
  verify the numeric geometry helper `0x00431900` and transport/model selection dependencies.
- Main evidence: `C:/Users/tlori/FreeCity4/decomp/cSC4MySimDispatch.c` and Windows export via re_query.py.
- Preserve timestamp semantics and serializer +0x0C adjustment; several export STL helper names are wrong.
- No game request labels outstanding; no ABI proposal file.
- Queue this documentation milestone as kind re, state ready; subsequent full-owner work is MS18.
