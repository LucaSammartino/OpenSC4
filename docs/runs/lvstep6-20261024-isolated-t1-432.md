tag: lvstep6-20261024-isolated-t1-432
run: 432
date: 2026-10-24
lane: glue-landvalue-step6
dll: FreeCity4 SHA256 863F529E12DEC6F3B196B1F1B39DA1EF6CFDEB03ED517360E79FC4D322B7EC55 (source a2457893c)
pacing: turbo
verdict: clean
switches: isolated landvalue diagnostic (not house); disable=budget, landvalue_shadow=1 only, turbo1, end_date2032-01-01, seconds600, timeout750; build-runs/game-queue/lvstep6-20261024-isolated-t1.req
result: lvstep6-20261024-isolated-t1 72/73 (landvalue). Clean/ref_warnings=0/rtc=0/foreign_exceptions=0/watchdog_stalls=0/overruns=0, date2032-01-10, real28.829s, wall40s. Pass2 at2026-02-06 has intrinsic6 (max delta2), total3 (max delta2), slope0, types0, mean270.05667/270.05804, mismatches10; no daily sample among9 reproduces native inputs (closest day1 still3 intrinsic differences). Every other comparison has mismatches=0; no incomplete lines. Difference recorded under LV2, not waived. input_capture=pass-end-sample/native_step_capture=0; no Step capture or replacement credit. Completed log/verdict at build-runs/game-results/lvstep6-20261024-isolated-t1.{log,verdict}.
