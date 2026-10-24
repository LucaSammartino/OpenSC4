tag: split11-r2-split-616
run: 616
date: 2026-10-24
lane: tool-regress-split11
dll: coordinator integration snapshot SHA256 4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1
pacing: turbo to 2032-01-01, ShadowSplit 3, queue
verdict: FAIL (stalled, incomplete)
switches: unchanged house shadows partitioned traffic / resources / all remaining shadows, plus standard replacements
result: split11-r2-s3 reports regress wall 276 s (Python invocation 496.77 s, retained separately). Traffic stalled 2028-11-02, real_seconds 195.484, watchdog_stalls=1; resources stalled 2032-05-11, real_seconds 193.953, watchdog_stalls=2; remaining shadows stalled 2030-02-17, real_seconds 190.984, watchdog_stalls=2. 51/52 labels exact over unequal truncated intervals: traffic 34/34, flammability 36/36, propmgr 150/151 (mismatch total 1), dbpf 74/74, weather 77/77. Replacement profile reached 2032 with native verdict clean but watchdog_stalls=1. Neither parity nor performance sign-off. All counts retained in lane build-runs/split11-r2-measurements.json and regress-split11-r2-s3.out. One fresh split retry requested.
