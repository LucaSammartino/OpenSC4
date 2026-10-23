tag: cmdline4-r3-native-on-2032-f
run: 257
date: 2026-10-22
lane: verify-cmdline-native4
dll: immutable-b source92e01745 SHA256053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC
pacing: turbo
verdict: clean
switches: enable empty; disable budget; actual enabled simulator only; cmdline_probe=1
result: Konradshohe through2032-01-04;123/123 borrowed original versus production Rust exact, differences0/failures0,argc5/raw174; full SaveCity(full) ok1 in11266ms at game2031-12-03, log2026-10-22 21:22:27.908; PostSystemServiceShutdown reached; ref_warnings0/watchdog_stalls0.

Matched OFF-e's DLL, city, script, profile, seconds420/timeout600, end2032-01-01
and turbo1, changing only label and cmdline_probe. The query source independently
captures GetCommandLineA into the real production Rust CmdLine; no native output
seeds expectations. Method inventory and counts are recorded in the lane report
and CMDLINE method log lines. No native mutation, owned construction or release.
CL1-CL4 exclusions remain. No reload fixture or whole-owner/default promotion.
All prior native disable-all failures remain preserved separately.
