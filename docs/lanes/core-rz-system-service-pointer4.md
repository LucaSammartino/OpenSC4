# core-rz-system-service-pointer4 — round 1 interrupted

| Field | Actual result |
|---|---|
| Status | Failed / unstarted implementation; not READY |
| Original family budget | One round consumed of at most three; no refund |
| Reserved source baseline | b5e4b7c30bc5d0b288d93e511de37d6b6b87fd0b |
| Provider / model | Codex / gpt-6.1-sol; unchanged |
| Session | 01a167c9-829d-7b23-90f8-9a1c5ff33b98 |
| Actual completed tool commands | One |
| Production, fixtures, gates, native execution | None |

The actual round started at 09:03:43 BST on 2026-10-23. Its sole completed
command was `git -C "<own worktree>" rev-parse --show-toplevel`, exit 0,
returning this worktree. The actual event stream then contains both an error
and turn.failed with "Selected model is at capacity. Please try a different model."
The final result is an error. This was an upstream provider capacity failure
after the first command, not a compiler, contract, guard or native mismatch.

The status records bootstrap_done=true, tool_calls=1 and last hook 09:03:52.
No implementation, algorithm analysis, test execution or validation was delivered.
The earlier 09:03:15 availability probe predates this failure and does not
qualify its recovery. The later same-model probe actually ran 09:07:45–09:07:53,
returned AVAILABLE with exit 0, and qualified the coordinator's recovery of
this exact pause. It does not erase the interrupted round or promise future
provider availability.

This report is coordinator metadata completion explicitly authorized after
the failure. The existing worktree and source baseline were clean. A finite
09:11:05 host census observed no worktree process references, no own-port4870
service or connection, no inspection errors, and all six known outputs absent.
These observations do not prove a historical exit cause or future lifetime.
The actual missing report is published using the original backend
prepare_codex_git/publish_codex_git compare-and-swap against the reserved
baseline; no source, spec, guard, model, budget or reconstructed reference changes.

The public system-service-pointer contract and SSP2/SSP3 boundaries remain
unchanged. No supported-body, whole-owner, native, shadow, default or coverage
credit follows from this report. A genuine same-family second round may perform
the original owned utility implementation only after normal full-scope admission,
current manifest checks and peer review.

Evidence retained in ignored build-runs: the original r1 manifest, events,
result and status; provider-pauses/probe-20261023-0908.json; coordinator-sysptr-core-r2-census.json;
coordinator-sysptr-core-r1-failure-report-publication.json.

