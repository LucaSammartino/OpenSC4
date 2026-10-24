tag: ordshadow6-20261024-turbo-a-374
run: 374
date: 2026-10-24
lane: glue-ordinance-shadow6
dll: FreeCity4-glue-ordinance-shadow6, SHA256 87f628e2ec1faa93c51744e6e00b0307d8da5fe6af08df80d6a73fa89199ea25
pacing: turbo
verdict: clean
switches: enable = simulator; disable = ordinance; ordinance_shadow = 1; Konradshohe; end_date = 2032-01-01; seconds = 600
result: ordshadow6-20261024-turbo-a 72/72; ordinance 72/72 exact monthly lines (2026-02-01 through 2032-01-01), every line captured=1 mismatches=0; 20 original records, 43 effect properties and 956 comparisons each month (68832 total). AUTOTEST VERDICT clean at 2032-01-01, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0; 83.437 real seconds, 96 s harness wall. No shadow incomplete lines or other warnings. Evidence: build-runs/game-results/ordshadow6-20261024-turbo-a.verdict and .log in the lane worktree.
summary: ordinance shadow exact 72/72 to 2032, run 374

Final dated line: `ordinance shadow 2032-01-01: records=20 checks=956 effects=43 captured=1 mismatches=0 months=72 clean=72`.

Each month starts from original serialized records, including pending charges and raw saved flags. Private core copies run with read-only city inputs. Comparisons cover original availability, on/enabled flags, adjusted and current monthly amounts, conditions, simulator state queries, both total money values and cumulative effects (float bits and 32-bit offsets).

Effect queries cover indices 0..8 of section 6.2/6.3 properties plus properties enumerated from the live misc holders. Longer mod arrays and custom foreign ordinance implementations are outside this measurement. No normal-speed rerun was required because every turbo line was exact.
