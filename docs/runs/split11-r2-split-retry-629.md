tag: split11-r2-split-retry-629
run: 629
date: 2026-10-24
lane: tool-regress-split11
dll: coordinator integration snapshot SHA256 4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1
pacing: turbo to 2032-01-01, ShadowSplit 3, one retained retry
verdict: FAIL (repeated watchdog stalls, incomplete)
switches: unchanged house shadows partitioned traffic / resources / all remaining shadows, plus standard replacements
result: split11-r2-s3-retry reported regress wall 213 s; complete invocation 603.28 s. All three shadow groups stalled with one watchdog trip each: traffic 2027-04-04 (89.906 s), resources 2028-09-05 (88.547 s), remaining shadows 2027-11-08 (89.515 s). Replacements also stalled at 2029-06-09 (82.907 s), watchdog_stalls=1; a trailing native-profile log line is not an endpoint verdict. coverage.read_regress pass=False; 51/52 labels exact only over truncated unequal intervals. flammability 16/16, traffic 15/15, propmgr 68/70 (mismatch total 2; query-list differences on 2026-03-01 and 2026-12-01), weather 33/33, dbpf 27/27. Full per-label comparison with combined retry run 617 is in docs/lanes/tool-regress-split11.md; machine evidence is build-runs/split11-r2-final.json. No valid full-horizon parity or 35% speed proof; no further retry, watchdog weakening, ABI changes or coverage credit.
