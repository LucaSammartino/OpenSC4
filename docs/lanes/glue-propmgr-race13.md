| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-race13: exact native prune-input capture | Codex / glue-propmgr-race13 | in progress; resumed from 0832fe666; prior cargo 7389/0, clippy clean, CTest 81/81; turbo and normal both 219/220, PL24 open | Audit remaining bounds inputs before deciding whether further implementation is authorized by the spec |

## Handoff
Goal: eliminate intermittent lot-list mismatch without weakening comparison; two house runs to 2032.
Done: prior round captured native slot-40 and slot-8 inputs (88730ab68), gated and measured two runs (731/732), filed PL24; final prior report 0832fe666.
Resumed: spec A7.2 still requires half-open bounds; PL24 remains open. Added native city slot-96 observation/replay to close the remaining bounds-input gap. Corner count/order/arguments are strictly checked; mismatch diagnostics now print actual native world bounds. Existing slot-40 rectangle replay is already validated.
Gate: full qbuild all on c5d532838: cargo 7389/0, clippy clean, native 161/161 build edges; CTest 80/81. plugin_autotest_test failed its internal ms<15000 check (end-date quit child 15656 ms). Same DLL/executable passed focused --autotest with that child at 172 ms, proving the timing diagnostic. Failed native logs moved to fc4verify-r2-first-{build,ctest}.log; focused evidence propmgr-r2-autotest-focused.log.
Measured: propmgr-race13-house2032-c-bounds 220/220, clean to 2032-01-04, ref_warnings=0 (run 733, DLL 6da5598c).
Outstanding: propmgr-race13-house2032-d-bounds-normal; wait session 23940 is collecting its verdict via wait-propmgr-verdict.ps1 (filesystem event, no qwait or polling loop).
Next: finish normal-speed run, then rerun queued verify after the demonstrated scheduler-delay failure. Investigate whether exact bounds replay resolves PL24 or merely changes the failure timing.
Facts: prior labels propmgr-race13-house2032-a and propmgr-race13-house2032-b-normal, both 219/220, no resyncs, clean/ref_warnings=0. No request outstanding.

## Prior measured report

The shadow now observes occupant slot 8 at the native pruning call and replays those exact position values to Rust, alongside the ordered pre-prune slot-40 list. It no longer independently samples positions early. Thread-local capture, locked forwarding addresses, reference ownership, and scoped restoration keep other callers forwarded to native methods. Strict call-order/count validation and the full result/list comparison remain enabled; nothing is skipped or tolerated. Source milestone: `88730ab68`.

This isolates a bounds-contract discrepancy rather than fixing the verified-credit failure. With the same slot-40 list and exact native slot-8 inputs, native retained X=144 on the expected upper-X boundary in the turbo run and Z=944 on the expected upper-Z boundary in the normal run. Each failure has inputErrors=0 and all native position reads accounted for. The core follows prop-lotconfig.md A7.2's literal half-open bounds and rejects those entries. PL24 in [questions/prop-manager.md](../specs/questions/prop-manager.md) requests the exact native bounds/NaN contract. No guessed rule or signature was implemented. The 220/220 objective remains open; this is a READY entry for the measured capture/diagnostic work under the task's partial-closure rule.

Final stable-source gate: `qbuild.py --wt <WT> all` PASS with `RUST_TEST_THREADS=1`: **7389 passed, 0 failed**, summed across every test-result line; clippy with warnings denied passed; queued native build **160/160 edges**, full CTest **81/81**. All asynchronous operations were collected to terminal results. No Rust files changed, so no Rust formatting operation was needed. No C ABI exports changed.

The initial full gate failed at a pollution C ABI getter test's 20-second receive timeout/non-unwinding panic (`0xc0000409`); clippy and queued native build/CTest passed in that gate. The same built test passed alone with `--exact --test-threads=1` in 0.00 s. This diagnostic justified changing scheduling input for the full rerun. Initial evidence is preserved in `build-runs/qbuild-initial-{test,clippy}.log`, `fc4verify-initial-{build,ctest}.log`, and `pollution-getters-focused.log`; final logs use the normal qbuild/fc4verify names. The initial q.py focused cargo command failed before execution because Git Bash CreateFileMapping returned Win32 error 5; its separate log remains under `build-runs/q/`.

Exact shadow lines, computed from every propmgr observation in the logs:

```text
propmgr-race13-house2032-a 219/220
propmgr-race13-house2032-b-normal 219/220
```

Both ordinary Konradshohe runs used `propmgr_shadow=1`, cells=32, with the existing default-off setting unchanged. Both reached 2032 clean, ref_warnings=0, input-error lines=0, and propmgr resync lines=0. Turbo reached 2032-01-04 in 31.328 s; normal reached 2032-01-01 in 111.781 s. Each sole list difference is PL24. DLL SHA prefix `513fde7b`, engine source `88730ab68`. Run records: [731, turbo](../runs/propmgr-race13-house2032-a-731.md), [732, normal](../runs/propmgr-race13-house2032-b-normal-732.md). IDs were moved away from the initial 521/522 allocation after those numbers were taken by concurrent integration work. Game logs/verdicts remain in `build-runs/game-results/`; no request remains outstanding.

No coverage tables or regress scripts were edited. The existing house profile already enables propmgr_shadow, but these runs do not qualify for verified credit. Next: resolve PL24 into the behavior spec (including upper X, upper Z and NaN handling), update the core and meaningful boundary tests to that contract, then gate and repeat two house runs at 220/220. The question index regeneration belongs to the coordinator; engine lanes cannot run tools/questions.py.
