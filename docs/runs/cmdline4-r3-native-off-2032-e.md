tag: cmdline4-r3-native-off-2032-e
run: 256
date: 2026-10-22
lane: verify-cmdline-native4
dll: immutable-b source92e01745 SHA256053BD7A077A43A31ACB065C3B985D792CD9FEBFADAA82B0E7A343D742621FABC
pacing: turbo
verdict: clean
switches: enable empty; disable budget; actual enabled simulator only; cmdline_probe=0
result: Konradshohe through2032-01-07; full SaveCity(full) ok1 in10984ms at game2031-12-09, log2026-10-22 21:19:10.807; PostSystemServiceShutdown reached; ref_warnings0/watchdog_stalls0; probe disabled gives no query coverage credit.

The matched supported profile was copied from the successful persist control in
build-runs/evidence-run4. Requested save2031-12-01, mark2031-12-15, end2032-01-01,
seconds420/timeout600, turbo1, original optional owners. The immutable DLL is
build-runs/cmdline4-r2-immutable-b/FreeCity4.dll; source and built output hashes match.
Request/log/verdict remain in this worktree's build-runs game-queue/done and
game-results directories. This clean profile does not establish the cause of the
retained round2 disable-all native stalls. No reload was exercised.
