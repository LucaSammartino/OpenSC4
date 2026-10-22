| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view-utilities-contracts4: native view utility contracts | Codex / `re-view-utilities-contracts4` (RE spec lane; detached HEAD) | in progress | 1. Resolve VU2–VU11 against native primary bodies. 2. Publish owner checklist and independent vectors. |

## Handoff
- Goal: resolve VU2–VU11 in view-utilities; reconcile exact 50-function owner family without engine edits.
- Done: native assembly proves signed discrete rotation, failed-ray output writes and independent caches; VU7/VU9/VU11 answered in spec and archived. questions.py check passed.
- Milestone 1 f16fdbb5: §4.1/§4.3/§6 contracts and independent ray/rotation/cache examples; no core/vector fitting.
- Milestone 2 (this commit): all VU2–VU11 contracts, full 50/19748-byte family catalogue, 21 prose conformance examples; question archive/index updated.
- Next: final documentation checks and catalogue arithmetic validation; finish report and queue. Check numeric instruction citations and heading order.
- Measured ownership: 48 SC4ViewUtilities-attributed bodies/19574 bytes plus Min/MaxOfFourFloats 2/174 =50/19748; prior49/19670 omitted rotation78B.
- Native differences: triangle/plane tolerance0.01f; triangle accepts negative t, pick compares Euclidean distance; plane misses zero output; signed remainder and wrapping conversions.
- Sources: main `C:/Users/tlori/FreeCity4`; decomp/SC4ViewUtilities.c; tools/re_query.py.
- Restrictions: specs/questions/lane report only; ignored build-runs helpers allowed; no ABI table edits, engine reads, subagents, network, branch/ref changes.
- Guard: main/build-runs/run4-re-view-utilities-contracts4-r1-manifest.json; run status after each milestone commit.
- No game runs requested; documentation gate only. No ABI exports changed.
