# Lane providers: Claude and Codex

Provider transport reference. [coordinating.md](coordinating.md) owns launch/landing procedures;
the [prompt](../../.claude/workflows/fc4-lane.js) owns lane rules. Shared launch/refill scripts enforce worktree,
subsystem, worker, three-glue and round caps. Changing providers never resets limits or parking.

## Run commands

From the clean main checkout, after the mandatory HANDOFF section0 read and `lanes.py guard`:

```powershell
python tools/lane_launch.py                                      # true live workers and rounds
python tools/lane_launch.py build-runs/<reviewed-spec>.json --provider codex
python tools/lane_launch.py build-runs/<reviewed-spec>.json --provider claude
python tools/lane_refill.py --provider codex                      # reviewed queue, ongoing refills
```

The refiller is a foreground process; a coordinator starting it in the background must use a hidden window
and redirect output to build-runs. Its existing owner lock prevents a second dispatcher. `--once` performs
one iteration. `--queue <spec> ...` adds reviewed briefs. Every launch runs fresh slots and check with touches,
dependencies and resume when applicable. It refuses STOP, parking, exhausted rounds, overlapping live work,
disk above85%, and a pause for the selected provider. The default ledger is
`build-runs/run4-coordinator-state.json`; the queue is `run4-refill-queue.json`. Use the existing coordinator
lock for manual state edits.

An explicit `provider` field in a queued spec selects that backend. Otherwise the refiller's `--provider`
is used; without either, the state provider defaults to Claude. Individual launch `--provider` wins over
the spec. No automatic provider failover occurs.

## Authentication and models

Claude continues to invoke the saved `fc4-lane` Workflow, once per reserved round, with its original allowed
tools. It uses the existing Claude Code login and its model setting/Workflow override.

Codex uses the installed CLI and its original credential store. `codex login status` must say
`Logged in using ChatGPT`; otherwise launch fails. Run `codex login` to select subscription authentication.
The worker removes API-key environment overrides and forces the OpenAI provider with ChatGPT login.
It never copies credentials, uses private HTTP endpoints, or falls back to API billing.

Codex inherits only `model`, `model_reasoning_effort`, and the configured Windows sandbox implementation
from the user's config. The normal user MCP/plugin configuration is excluded to keep engine lanes' context
small and preserve their clean-room boundary. RE lanes receive only the existing REA configuration and its
two approved decompiler tools. A spec can explicitly select Codex options:

```json
{
  "provider": "codex",
  "codex": {"model": "<available Codex model>", "model_reasoning_effort": "high"},
  "resume": true,
  "touches": ["<exact subsystem key>"],
  "args": {"name": "<lane>", "kind": "engine", "task": "<reviewed task>", "maxRounds": 1}
}
```

Use provider-specific model identifiers; Claude aliases are not Codex model names. The actual model and
provider are recorded with the completion and copied into the shared round history. Changing a model
does not reset the context/progress/round limits. Subscription limits still apply to all account usage.

## Guard and token accounting

`lane_prompt.mjs` renders the existing Workflow's canonical instructions and result schema without calling
a model. The Codex adapter changes only transport instructions (PowerShell, apply_patch, commit attribution,
context command) and adds a small final lane-specific section. Static prefixes remain identical across
lanes of the same kind/provider. Both use focused briefs, quiet wrappers, disk logs, fresh conversations
per round, and short structured results.

`laneguard.py` owns the shared policy. Claude keeps its existing hooks and transcript adapter. Codex uses
`codexguard.py` SessionStart/PreToolUse hooks with an explicit registered lane/round, persistent transcript,
and the same lane counter and parking ledger. The hook accepts Codex patch syntax for the permitted handoff
files. It rejects nested agent creation. Exceptions emit a denial and stop the supervisor instead of allowing
work without telemetry.

Codex writes initial usage after its first tool call. Before measured usage exists, only the exact read-only
`git rev-parse --show-toplevel` identity handshake is allowed. It counts toward the watchdog. Further calls
require the latest input-token measurement; cached input is already included and is not added twice.
Cumulative billed tokens are not the context size. Missing later telemetry fails closed.

The worker uses an isolated CLI process, a scoped workspace permission profile, and the user's Windows sandbox setup.
It supplies the reviewed guard inline and uses Codex's one-invocation hook-trust option so it executes in
noninteractive mode. That option enables the guard; it does not bypass guard decisions or sandbox policy.
General hook limitations remain those of the installed CLI. Changes to its hook/transcript formats require
another real denial test before use. Persistent rollout files are required; do not use ephemeral sessions.

Git commits use a detached HEAD in the existing lane worktree. Before launch the supervisor verifies the
registered lane branch and repository and persists its base commit. The profile grants writes only to that
worktree's Git metadata and the shared object store; shared refs, config and hooks remain read-only. After the
round the supervisor checks ancestry, compare-and-swaps only the reserved lane ref, and reattaches HEAD. A
concurrent ref change or rewritten history fails closed and preserves the detached commits for review. Do not
reset or switch a failed detached worktree blindly. This is needed because workspace-write protects resolved
Git metadata even with --add-dir. The original lane guard still governs every model tool call.

Completion is written atomically to the existing `run4-<lane>-r<n>-result.json` format. Codex's raw event
stream, final JSON, schema, manifest and guard status stay alongside it in build-runs. The supervisor PID
remains live until the model process finishes. Confirmed completions retire PIDs to avoid Windows PID reuse.

## Limits and recovery

Provider errors or usage/rate limits stop the refiller. Diagnostics are kept in
`build-runs/provider-pauses/<provider>.json`; the earlier untagged `run4-refill-paused.json` belongs to Claude.
Do not clear a pause merely because time passed. Verify the selected provider's availability first. An
explicitly selected other provider retains the original provider's pause. STOP always blocks both.

Resume the original worktree using its next remaining round and a reviewed resume brief. Never replay a
Claude transcript into Codex or vice versa: committed state, the lane report and preserved WIP are the handoff.
Merge only reviewed ready work, regenerate coverage after landing, and remove only ancestor-proved landed
nonparked worktrees. A guard denial still ends that lane's work; do not route around it with another provider.

## Validation

Run `python -m pytest tools/tests -q` for provider/guard changes. Hook/transcript changes also require a real
watchdog-denial probe and a detached-commit/publication probe before use. Scratch probes consume provider usage
but do not count as implementation lane rounds. Keep evidence under `build-runs/codex-adapter-canary/`.

References: [Codex noninteractive mode](https://learn.chatgpt.com/docs/non-interactive-mode),
[subscription authentication](https://learn.chatgpt.com/docs/auth), and
[hook protocol and limitations](https://learn.chatgpt.com/docs/hooks).

For transport failures, preserve edits and original results. Coordinator recovery commits contain only reviewed
files and do not consume another lane round. Verify publication from actual HEAD and ancestry; shared refs stay
protected. Keep Git auto-maintenance disabled in child shells.
