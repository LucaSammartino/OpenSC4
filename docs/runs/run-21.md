tag: run-21
run: 21
date: unknown
lane: run
dll: unknown
pacing: unknown
verdict: crash
switches: `enable = power` (our simulator and budget)
result: **crash** 0.5 s after load, inside our DLL: a line-item handle of `0x3F800000` (the float 1.0 used as a pointer) reached `SetFullExpenses`. Sent to the plugin lane. Power stays off by default, so manual play is unaffected.
