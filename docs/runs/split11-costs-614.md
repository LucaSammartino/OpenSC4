tag: split11-costs-614
run: 614
date: 2026-10-24
lane: tool-regress-split11
dll: coordinator integration snapshot SHA256 4853374130ffde73a29adf15e197802a7b4f79d307141fdb23983bc4eb3bdfd1
pacing: turbo, requested 2026-07-01, four independent queue slots after train199 cleared
verdict: clean (timing probes only)
switches: independent traffic (including hypotheses/lines), demand, pollution, resource/weather families; disable budget,ordinance
result: split11-r2-cost-traffic 27.781 s, demand 3.625 s, pollution 4.937 s, resources 167.359 s; all verdicts clean. Traffic ended 2026-07-01, demand/pollution 2026-07-04. Resources reached 2026-07-01 after 15.625 s but continued during a 151.734 s shutdown tail to 2032-09-28; final days/s is unsuitable for resource projection. Full-horizon estimates: traffic 383.5 s, demand 42.5 s, pollution 62.6 s, resources 367.4 s (simulation projection plus fixed shutdown tail). No full-profile parity or coverage credit claimed. Logs and verdicts copied to lane build-runs/split11-r2-cost-*.
