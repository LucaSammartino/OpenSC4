tag: vdbpf4-r1-native-1c53058b-a
run: 273
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-native-frozen/FreeCity4.dll (retained SHA256 524cd1243584a4613969b49b28870a0f1cf0d3df242133b431da002304288a5a; actual historical harness prefix524cd124)
pacing: turbo; requested2032-01-01, actual2032-01-04
verdict: clean
switches: dbpf_native1; disable=budget; sim_turbo1; native literal fixtures; Konradshohe; script dbpf-native-save.txt; seconds1200/timeout1500
result: Actual clean2032-01-04 with foreign exceptions/watchdog stalls/rtc/overruns/ref warnings0, real56.000s. Full SaveCity succeeded in11000ms, followed by dbpf-native-after-full-save mark and PostAppShutdown/PostSystemServiceShutdown. Native comparison differences remain in the retained lane report/raw log; this lifecycle result is not whole-owner or enumeration exactness, replacement, saved-byte equivalence or reload of a newly produced save.
summary: Historical native DBPF turbo verification completed full save and shutdown with zero reported reference warnings; raw comparison limits retained.

Actual request queued2026-10-22 19:38:50 BST, started19:38:55 in slot2 after6s, wall79s. The script's2031-12-01 save ran on2031-12-14 and its2031-12-15 mark followed successful save. Actual final date overshot the requested end and is preserved above.

Owned immutable request: build-runs/game-queue/done/vdbpf4-r1-native-1c53058b-a.req. Verdict/raw log: build-runs/game-results/vdbpf4-r1-native-1c53058b-a.{verdict,log}. Current retained round1 DLL was independently hashed during coordinator bookkeeping and matches the historical harness prefix; do not substitute round2's different3d1eee0c artifact. The existing lane report retains original transport/enumeration observations and source/gate limits. Numeric metadata was assigned later by the coordinator without changing the original tag, evidence, source or measured outcome.
