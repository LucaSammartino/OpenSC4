tag: lvcapture7-20261024-isolated-normal
run: 411
date: 2026-10-24
lane: glue-landvalue-capture7
dll: FreeCity4 SHA256 ABAD0198BAA9D88E545B6FC76BAD3EF57350671A28F928F2A14C95FB62C18B59 (source9ddac837c)
pacing: normal
verdict: clean
switches: isolated landvalue_shadow=1, disable=budget, sim_turbo=0, end_date2032-01-01, seconds900, timeout1100; not the canonical house profile
result: lvcapture7-20261024-isolated-normal 72/73; clean/ref_warnings=0/rtc=0/foreign_exceptions=0/watchdog_stalls=0/overruns=0, through2032-01-01, real110.828s, slot3, wall122s. Pass1 at2026-01-27: intrinsic0,total383(max15),slope0,types0,scalars0,mismatches383; live developer map. No other differences, incomplete or resynced lines. All input_capture=pass-end-sample/native_step_capture=0. LV7 startup discrepancy also occurs at normal speed; the turbo pass3 slope difference was absent here, without establishing a global turbo-only cause. No host installation, input-fidelity, deterministic-fix or coverage-promotion credit. Logs/verdict/request remain under build-runs/game-results and game-queue/done.
