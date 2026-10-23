tag: vdbpf4-r1-normal-1c53058b-b
run: 274
date: 2026-10-22
lane: verify-dbpf-native4
dll: build-runs/dbpf-native-frozen/FreeCity4.dll (retained SHA256 524cd1243584a4613969b49b28870a0f1cf0d3df242133b431da002304288a5a; actual historical harness prefix524cd124)
pacing: normal; actual2032-01-01
verdict: clean
switches: dbpf_native1; disable=budget; sim_turbo0; native literal fixtures; Konradshohe; script dbpf-native-save.txt; seconds1200/timeout1500
result: Actual clean2032-01-01 with foreign exceptions/watchdog stalls/rtc/overruns/ref warnings0, real121.531s. Full SaveCity succeeded in10875ms, followed by dbpf-native-after-full-save mark and PostAppShutdown/PostSystemServiceShutdown. Native comparison differences remain in the retained lane report/raw log; normal-speed completion does not establish whole-owner or enumeration exactness, replacement, saved-byte equivalence or reload of a newly produced save.
summary: Historical native DBPF normal-speed verification completed full save and shutdown with zero reported reference warnings; raw comparison limits retained.

Actual request queued2026-10-22 19:42:06 BST, started19:42:13 in slot2 after7s, wall146s. Both original request flags turbo0/sim_turbo0 are retained; full save and after-save mark ran before complete shutdown.

Owned immutable request: build-runs/game-queue/done/vdbpf4-r1-normal-1c53058b-b.req. Verdict/raw log: build-runs/game-results/vdbpf4-r1-normal-1c53058b-b.{verdict,log}. Current retained round1 DLL was independently hashed during coordinator bookkeeping and matches the historical harness prefix; do not substitute round2's different3d1eee0c artifact. See the lane report for original transport/enumeration observations and source/gate limits. Numeric metadata was assigned later by the coordinator without changing the original tag, evidence, source or measured outcome.
