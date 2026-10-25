# Run4 recovery after the specification PRs

The run remains open-ended. The worker target and caps are HANDOFF section0 "User policy" (kept only there); explicit smaller family limits in the coordinator state remain binding. STOP and guard denials always take precedence. Main must stay clean. Both Claude and Codex backends are supported through the shared [provider runner](../ops/lane-providers.md).

## Provider pause

Claude returned a weekly-limit response, reporting a reset at18:00 Europe/London. All workers finished; ten interrupted original worktrees have WIP committed or preserved. Do not treat quota-interrupted work as ready or verified. The usage-interrupted rounds still count. The original `build-runs/run4-refill-paused.json` remains Claude's pause. The user explicitly authorized adding Codex subscription lanes; select that provider explicitly without clearing Claude's pause. Time passing alone is not evidence of provider availability. Never clear a user STOP automatically.

## Prepared work

`build-runs/run4-post-pr-recovery-specs.json` lists ten reviewed existing-worktree resumes and three fresh core briefs. They are queued, with resumes first, while the provider pause remains set. Each resume retains its original worktree and remaining cap, uses the generated `lanes.py brief`, and requires fresh slots/check --resume at launch. The place-lot follow-up has only round2/2 remaining (its family limit is3). The landvalue touch key is corrected to `landvalue`; LV2 and the preserved train65 pass43 trace remain its verification target even if a later run happens to be exact.

The three new pure-core tasks are the largest fresh eligible fully specified groups after the blocked or existing higher-priority work: terrain overlays30,436B, lighting23,453B and render properties23,179B. They use the newly reviewed public specs and must commit code by aboutcall30, identify exact remaining methods, and file contract gaps instead of guessing. No full owner gets credit merely for a module existing.

Previously queued network-model correction and exemplar contract answers remain pending. The effects-emitter follow-up's duplicate-mention cause was removed by train65; its fresh check now says ok, but it stays provider-paused and has just one final family round. The old core-netocc4 resume was denied because the branch is already merged and its original worktree is absent. Its NO15-17 answers do not authorize bypassing that denial or reusing an alias for the same work. The old records branch is also already an ancestor of main.

## Restart procedure

1. Read HANDOFF section0 and run `python tools/lanes.py guard` after any restart/compaction. Check STOP, true CLI-worker count, slots, C: utilization and the current train.
2. Confirm the selected provider's availability, then start `tools/lane_refill.py --provider claude|codex` in a hidden window. Retain its owner lock and the shared coordinator lock; do not create a second dispatcher. Only clear a pause after its actual cause is resolved.
3. Every worker goes through `tools/lane_launch.py` and the selected guarded backend, with fresh slots and check --touches; use --resume for preserved worktrees. Never unpark a lane without its actual cause being fixed, never run a fourth round, and never change guard flags to force a launch.
4. Refill returned slots immediately; keep at most3 glue. Cut a train at2+ ready, run its actual gates, regenerate handoff/coverage/treemap, commit/push, and remove only ancestor-proved landed nonparked worktrees. Keep parked/unlanded worktrees and branches.

The approved live-worker count fix is de1cd3b6 (97 tools tests passed). `slots` still retains activity fallback for unknown worktrees; the native launcher count is the truth for our running agents. PR review/coordinator metadata work did not consume lane-agent rounds.
