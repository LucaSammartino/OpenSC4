| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-landvalue-step-abi4: LV6 native capture ABI proof | Codex / `re-landvalue-step-abi4` (RE spec lane; detached HEAD) | in progress | 1. Verify native bytes/callers. 2. Publish exact pending ABI layouts and forwarding boundary. 3. Retain runtime gaps; coordinator review/apply/full gate. |

## Handoff
- Goal: resolve the static LV6 prerequisite for native Step capture without code changes or direct docs/abi edits.
- Done: verified export bytes/callers for native bases, Step, DoMessage, iterator, map, float32 weight and both synchronous dispatch paths; publication committed with this handoff.
- Done: added landvalue §6.1.1/11.3 ownership/order proof and narrowed LV6; exact pending ABI proposal is included in this commit.
- Next: validate Markdown/diff/question index and exact ABI citations; finalize report and wip queue for coordinator application/full gate.
- Read narrowly: landvalue sections 6.1/11.3, current relevant interfaces/layouts records.
- Proposal destination: build-runs/pending-abi/re-landvalue-step-abi4-layouts.md; coordinator must apply and run full train gate.
- LV2/LV7 remain runtime evidence gaps; no sampling/math/golden/engine/tools edits.
- Installation remains unproved: QI does not intercept existing registrations; target identity/order, shutdown and pre-publication startup require proof. LV6 remains open.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-landvalue-step-abi4-r1-manifest.json.
- Commit native proof/layout proposal by call 30; update this handoff at each milestone.
