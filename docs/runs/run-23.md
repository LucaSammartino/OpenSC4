tag: run-23
run: 23
date: 2026-10-12
lane: run
dll: unknown
pacing: unknown
verdict: crash
switches: `budget_default=1, power_report=1, observe=1`, DLL F5439E03 (2c77349 + be7bb41 + 6aa6735, merged ef4159c)
result: **crash** `0xC0000409` ~0.3 s after PostCityInit, no `power recompute` line ever written. Last log lines were `IsLocallyFundedObjectInDepartment` / `GetLocalFundingPercent`. Diagnosis: a Rust panic in the power core or capi, prime suspect the "Read keeps the saved words, Init sizes the bitmaps from the city" change. **This run also produced the first x87 control-word measurements** — 24-bit at DLL attach and through PreFrameWorkInit/PreAppInit, 53-bit (`CW=0x027F`) from PostAppInit-enter onward.
