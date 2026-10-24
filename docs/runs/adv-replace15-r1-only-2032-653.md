tag: adv-replace15-r1-only-2032-653
run: 653
date: 2026-10-24
lane: glue-advisors-replace15
dll: FreeCity4-glue-advisors-replace15 b592c83e4, sha256 prefix 6270761c
pacing: turbo
verdict: corrupt
switches: Konradshohe; enable=advisors; simulator/budget default-on; turbo=1; end_date=2032-01-01; seconds=600; timeout=900; popup_dialogs=0; autotest_end=quit
result: Failed during Init before a game date: foreign_exceptions=1, ref_warnings=10, Rust panics=0, watchdog_stalls=0. Exception reading null at game module+0x372DE3; our PDB resolves DLL+0x1067440 to Deliver+0x160, called by SendCb and Advisor::base_init. Deliver used SetVoid1 instead of the section 10 IGZUnknown payload, leaving GetIGZUnknown null and overwriting data1. HostOf QI was not tracked before Dispatch Release. Both fixed in fb42fb2e8; tag-GC QI balance fixed too. Wall 10s. Exact evidence: build-runs/game-results/adv-replace15-r1-only-2032.verdict and build-runs/game-results/adv-replace15-r1-only-2032.log. Superseded by corrected run 654; retained as failed evidence.
