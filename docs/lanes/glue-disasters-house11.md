| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-house11 (countable house disaster shadow) | Codex / `glue-disasters-house11` | in progress | Inspect shadow and harness; implement deterministic schedule; gate and measure house to 2032 |

## Handoff
- Goal: deterministic scripted disaster house shadow, monthly countable exact summaries through 2032.
- Worktree reused at reserved detached HEAD; no branch changes permitted.
- Done: added opt-in disasters_scripted fire/riot schedule and cumulative monthly dated comparison lines; native gate next.
- Implementation commit: 6345f0625. DS47 filed for observation contracts for short-form disasters; DS28 remains open.
- Gate in progress: qbuild all, unified exec session 64395 (collect this operation; do not duplicate). build-runs/qbuild-test.log.
- Prepared request: build-runs/house11.req.pending, label house11-fire-riot-01, not submitted yet. build-runs/wait-house11.py waits without qwait.
- Next: qbuild all and queued verify, then house request with train193 switches plus disaster keys, timeout 1800.
- Monthly failures include numbered differences, refused ignition resyncs, unfollowable inputs, missing leaders, call-result differences and guarded-core failures.
- Schedule uses original manager start APIs and waits for natural lifecycle completion, with turbo held off then restored; no forced resume.
- Constraints: no tools/ reads or edits; propose regress profile change in report only. Native verify uses queued qbuild verify. No qwait for game requests.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-disasters-house11-r1-manifest.json.
