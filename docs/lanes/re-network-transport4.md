| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-network-transport4: socket, stream and lookup contracts | Codex / re-network-transport4 (detached worktree) | ready: bounded partial spec; docs checks pass; no native runs | Coordinator review/land; retain Q11–Q14 and all unsupported domains. |

## Report

Published [network-transport.md](../specs/network-transport.md), linked from
[web-network.md](../specs/web-network.md). This is the first dedicated bounded behavior
contract for the selected socket/stream/lookup owners, not a complete-owner claim.

The contract covers default invalid handle and explicitly unwritten address bytes;
unconditional create overwrite; bind/connect/listen/accept state and error transitions;
close invalidation after the raw close even on failure; distinct shutdown/stop-listen
policies; single-call partial I/O, zero-request asymmetry, EOF-close and nonfatal raw
errors; readiness error asymmetry, exact timeout encoding, output-write boundaries;
raw address/port representation; full shared error translation; immediate/cached/queued
lookup dispatch, strict unsigned cache expiry, queue cancellation and reference-call order.

Primary executable/export SHA identities are in spec §1. Instructions and literal table
bytes are the primary evidence; pseudocode served only as navigation. No live REA calls
were needed or made. Excluded capped owners were not reopened. Generic string/runtime/
clock/container facilities remain typed foreign dependencies, not owner algorithm oracles.

Independently counted ledger: 75 named candidate addresses / 7569 B. Of these, 42/4917 B
have bounded contracts (35 S/3072 B, 7 P/1845 B), and 33/2652 B remain U. Ten separately
qualified helper bodies total 684 B; another 128 B text adapter remains U. None of these
counts grants blanket native/coverage credit or exclusive attribution. They differ from
the provisional 4049+1700+2799=8548 B ownership inventory, which is explicitly not proof.

Forty independently derived **PROPOSED / UNEXECUTED** vector groups: lifecycle 11,
transfer 7, address 3, readiness 7, lookup 12. No game/native/DNS/network/socket/loopback
execution occurred; no implementation, engine, tests, ABI, SDK, maps or coverage edits.
ABI=false; game_runs=[].

Filed [Q11–Q14](../specs/questions/network-transport.md), indexed as four open entries:
lookup startup/identity; notification/timer/auxiliary-resource boundary; residual
adapters/public address construction; worker completion scalar. The audit withdrew an
unproved queued-ID payload claim: the worker reads a reused argument stack slot for
message +30. Full completion-payload equivalence is expressly unsupported. Existing
web-network Q1/Q3/Q7/Q9/Q10 remain untouched. No questions were marked answered.

Checks: `questions.py index` and `check` PASS; `doccheck.py check` and `links` PASS;
`git diff --check` PASS; ignored `build-runs/transport-doc-audit.py` PASS (all 75 primary
body lengths, status totals, 40 unique vector groups, asymmetric byte/timeout/wrap literals,
four indexable question IDs). `build-runs/transport-evidence.py` independently decoded
the primary error jump tables and counted named candidates; it does not execute native code.
Cargo/clippy/ctest: n/a for this documentation-only role.

Retained environment diagnostics: q.py failed to start Bash (E_ACCESSDENIED), so bounded
PowerShell reads were used. Legacy `lanes.py check --resume --kind re --no-rea-handshake`
reported REA-DOWN (obsolete npm path absent), missing lane branch and attached-worktree
expectation. The current reservation explicitly mandates detached HEAD and allows
export-only research without REA success; no branch/config/runtime reset was attempted.
The configured current manifest passed codexguard accounting; runtime STOP was not set.
Each successful local commit also printed a packed-refs.lock permission diagnostic while
creating the detached commit successfully; no shared-ref repair or Git maintenance was run.

Milestones: initial record `64966474`; first lifecycle/partial-I/O contract `d8f728aa`;
lookup/readiness/literals `2fa799bf`; full body ledger and payload boundary `b071a620`.
This report and final index-format correction are the final documentation milestone.
Next: coordinator review/land this bounded publication. Any future owner expansion must
respect Q11–Q14 and the excluded owner caps; no native production equivalence is claimed.
