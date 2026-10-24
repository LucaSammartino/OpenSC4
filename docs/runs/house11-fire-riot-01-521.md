tag: house11-fire-riot-01-521
run: 521
date: 2026-10-24
lane: glue-disasters-house11
dll: FreeCity4-glue-disasters-house11 source eec9904dc sha256-prefix afc115f5
pacing: turbo
verdict: clean
switches: train193 house shadows ini plus disasters_shadow=1 and disasters_scripted=1; disable=budget,ordinance; original disaster layer, default-off deterministic fire then riot schedule; end_date=2032-01-01, seconds=1740, timeout=1800; request house11-fire-riot-01 submitted once
result: WIP qualification. Exact countable line: disasters 3/73. Reached 2032-01-03 and PostSystemServiceShutdown in 1200.343s (queue wall1220s), foreign_exceptions=0 watchdog_stalls=0 rtc=0 overruns=0 ref_warnings=3, no panics or structured exceptions. Episode comparisons 1259/1259 exact: fire69/69 riot1111/1111 quiet79/79, original/ours steps55/55, resyncs0; monthly summaries retain 11 legacy unfollowable increments for matching high-coverage refused ignitions, failures0. Fire(9,70), riot(40,33), nine leaders removed at200 samples, schedule completed. New propmgr query mismatch2026-07-01 lot(32,53) 1/0: propmgr219/220 versus train193220/220; landvalue72/72 versus71/72; every other non-disaster dated group exact. All53 per-group clean/total counts versus52 baseline groups are recorded in docs/lanes/glue-disasters-house11.md and build-runs/house11-shadow-comparison.txt. Candidate51/53 groups exact, baseline51/52. Ref warnings: two PruneGone:275 class0x2977AA47, one DropAll:259 class0x896E75AF. Full all Rust7119/0 clippy clean CTest81/81; resumed explicit verify PASS build46/46 CTest81/81 native79.5s. No house profile/coverage credit; DS47/DS28 open.
