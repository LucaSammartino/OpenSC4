tag: traffic-drift10-r2-dbpf-turbo-2032-a
run: 493
date: 2026-10-24
lane: glue-traffic-drift10
dll: source merge dff0e55d6 (main 655671626 plus deferred DBPF queue); SHA256 6e89375a905e324663b2eabce86d734c29c0ba574505cce6cb00c3ef68683451
pacing: turbo to 2032-01-27; seconds=1800, timeout=2100
verdict: clean; foreign_exceptions=0, ref_warnings=0, watchdog_stalls=0, rtc=0, overruns=0
switches: main build-runs/regress-train192-shadows.ini copied verbatim except autoload_city omitted; dbpf_shadow=1; traffic_shadow=1, traffic_shadow_hypotheses=7; propmgr_shadow=1, propmgr_shadow_cells=32
result: PASS: traffic 72/72 exact, between-delivery resyncs=0; propmgr 220/220; dbpf 73/73; all 52 verdict shadow categories exact (10344/10344 lines). Landvalue 73/73, automata and automataowners each 72/72. No positive shadow mismatches or traffic resyncs in full log search. Reached 2032-01-27 in 808.329 real seconds, wall 829 seconds, slot 1. Final-source qbuild all PASS: Rust 7094/0, clippy 0 diagnostics, native 162/162, CTest 81/81. Unique label submitted once and collected to terminal result without qwait; logs/verdict build-runs/game-results/traffic-drift10-r2-dbpf-turbo-2032-a.*. DBPF comparison scope remains public resource-manager packed files, as on main.
