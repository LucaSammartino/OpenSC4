tag: lvcapture7-20261024-isolated-t1
run: 410
date: 2026-10-24
lane: glue-landvalue-capture7
dll: FreeCity4 SHA256 ABAD0198BAA9D88E545B6FC76BAD3EF57350671A28F928F2A14C95FB62C18B59 (source9ddac837c)
pacing: turbo
verdict: clean
switches: isolated landvalue_shadow=1, disable=budget, sim_turbo=1, end_date2032-01-01, seconds900, timeout1100; not the canonical house profile
result: lvcapture7-20261024-isolated-t1 71/73; clean/ref_warnings=0/rtc=0/foreign_exceptions=0/watchdog_stalls=0/overruns=0, through2032-01-02, real37.110s, slot4, wall52s. Pass1 at2026-01-28: intrinsic0,total383(max15),slope0,types0,scalars0,mismatches383; live developer map. Pass3 at2026-03-06: intrinsic0,total0,slope1(max0.013541221618652344),types0,scalars0,mismatches1; previous developer map. No incomplete or resynced lines. All input_capture=pass-end-sample/native_step_capture=0; no installed host, fidelity or deterministic-fix credit. LV7 startup and LV2 timing gaps preserved; normal-speed counterpart requested. Logs/verdict/request remain under build-runs/game-results and game-queue/done.
