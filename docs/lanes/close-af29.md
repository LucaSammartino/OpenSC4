| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-af29 aircraft closure; kind engine | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/close-af29; detached HEAD, reserved branch close-af29; code de4360192/bc0cc93d9, audit 8e7014f37, main merge 828c56b96; ABI/seam reservations none | ready; cargo 7977/7977, clippy 0 diagnostics, fmt 5/5, CTest 130/130; close-af29-r3-aircraft-1/-2 clean, ref_warnings=0; automata 5/5 and helicopter metadata 5/5 exact per run; questions none; whole-owner credit remains partial | 1. Land engine entry at its recorded full HEAD. 2. Obtain aircraft §9 runtime observations before closing remaining rows. |

2026-10-26 resumed after reboot; merged main ef84805f1 without conflicts. This round's evidence supersedes the pre-reboot pending operations.
AC closure: airplane 16 implemented / 12 open (+1 foreign); helicopter 19 implemented / 14 open (+1 foreign), measured in the two owner README tables.
Airplane applied: §3.2.1 AF4 constructor/teardown; §3.3.2–3 AF3 clearance/selection/fire/approach; §3.4.1–2 AF10; §4.3.3 AF5/AF6; §4.4 AF3/AF6 ordered requests.
Helicopter applied: §4.2.6 AF2/AF4 power/frame order/home completion; §4.2.7 AF4 claims; §4.3.3 AF5/AF6 callback requests; §4.4 AF3/AF6 startup/dispatch; §4.2.1 shared AF10 time.
Unchanged implemented rows: both aircraft §0/1/2 bounds/3.1/6/8, automata §6.1–2/6.4/14; airplane §3.3.1; helicopter §4.1/4.1.1/4.2.1/4.2.4/4.3.1 and automata §12; existing tests retained.
Airplane open rows: aircraft §2 tick(AF2/5), §3.2(AF2/4), §3.3(AF3/6), §3.4(AF2/5), §3.4.2(AF2), §3.5(AF2/4), §4.3(AF5), §5(AF6), §7(AF2–6/11); automata §6.5(AF3/6), §6.6(AF3), §15(AF4).
Helicopter open rows: aircraft §2 tick(AF2/5), §4.2 rotor(AF2), §4.2 goals(AF3), §4.2.2(AF3/5), §4.2.3(AF2/4), §4.2.5 and Init(two rows, AF4), §4.3/§4.3.2(AF5), §5(AF6), §7(AF2–6/11); automata §6.5(AF3/6), §6.6(AF3), §15(AF4/11).
AF2 remains: aircraft §9.1 native lift/basis/angular, rotor motion/transform and surface/collision stage observations, including calibration convergence.
AF3 remains: §9.2 continuous airplane controllers/completion, helicopter MoveXZ and 0x36–0x38 claims/crime/rescue service branches.
AF4 remains: §9.3 constructor remainder, native attachment/Init/failure/repeat ordering and phase-request causes. Claim cleanup is specified, not a scale adjustment.
AF5 remains: §9.4 raw control translation, readiness/cadence/projectiles and spotlight output transforms/terrain geometry.
AF6 remains: §9.5 observer/message envelope, external failures/callbacks; ordered goal/repulsor/notification requests are implemented independently.
AF11 remains: §9.6 complete registration/removal enumeration and independent resource/placement/goal input history; archive subset/private metadata fixture is insufficient (vector 35).
AC Rust: qbuild all once this round, exit=0, 7977 passed/0 failed/0 ignored; build-runs/qbuild-test.log includes aircraft_closure 16/16 worked-case tests.
AC quick: PASS 7132 tests, dependent crates compiled, 328/328 native edges; build-runs/build-results/qbuild-20261026-180421-quick.verdict.
AC clippy: exit=0, diagnostics=0; build-runs/qbuild-clippy.log. AC fmt: 5 files, diff_lines=0; git diff --check passed; build-runs/close-af29-r3-checks.log records corrected fmt invocation.
AC native: qbuild all PASS, build 29/29 edges, CTest 130/130; build-runs/fc4verify-build.log and build-runs/fc4verify-ctest.log (103.53 s).
AC game: MAIN/build-runs/game-results/close-af29-r3-aircraft-{1,2}.{verdict,log}: both clean, ref_warnings=0, script 2/2; run 2 verifies final DLL SHA256 fa95dd5a at 38.17 days/s through 2026-05-02.
AC shadows: both runs automata 5/5 exact, 30 components (50564/50999 bytes), mismatches=0; helicopter metadata 5/5 exact, reachable=0/private fixture=1, five fields/sample; no flight/parameter/home/goal/lifecycle/enumeration credit.
AC landing: ready kind engine with actual full HEAD; ABI unchanged, no new questions or coverage/regress edits. Coordinator credit is implemented only for the 16 airplane/19 helicopter bounded rows; §9 rows remain open for the observations above.
