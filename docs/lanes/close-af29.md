| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-af29 aircraft closure; kind engine | Codex; C:/Users/tlori/FreeCity4/.claude/worktrees/close-af29; detached HEAD, reserved branch close-af29; code commits de4360192, bc0cc93d9; ABI/seam reservations none | blocked on shared native builder; cargo 7909/7909, clippy 0 diagnostics, fmt 5/5; CTest pending; game/shadow unmeasured; questions none; whole-owner credit remains partial | 1. Service native build requests. 2. Collect existing operations. 3. Run shared game queue. 4. Enqueue kind engine after validation. |

2026-10-26 12:55:17: quick request pending since 12:18:30; full native request pending since 12:36:45.
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
AC Rust: qbuild all test exit=0, 7909 passed/0 failed/0 ignored; build-runs/qbuild-test.log (includes aircraft_closure cases).
AC focused: aircraft_closure 16/16 passed; new worked-case tests live in engine/conformance/tests/aircraft_closure.rs; captured stdout in build-runs/close-af29-checks.log.
AC clippy: exit=0, diagnostics=0; build-runs/qbuild-clippy.log. AC fmt: 5 files, diff_lines=0; build-runs/close-af29-checks.log. git diff --check passed.
AC native: qbuild all invoked once; build/CTest not started. Pending build-runs/build-queue/qbuild-20261026-121830-quick.req and qbuild-20261026-123645-build-ctest.req; sessions 84418 and 21748 respectively.
AC game: no request submitted, no verified DLL/game/ref_warnings/shadow result. After build use MAIN/build-runs/game-queue, parallel=1 above [ini], turbo/end_date and automata/helicopter shadows; metadata is not flight validation.
AC enqueue: WIP merge-queue entry only; merge-train enqueue deferred until native/game validation. No ready credit claimed; ABI unchanged; no new question filed or coverage/regress edits.
Next: service the shared native builder, collect those SAME operations (do not rerun all absent a fix), run one shared-queue game check, then update evidence and enqueue --kind engine with actual full HEAD.
