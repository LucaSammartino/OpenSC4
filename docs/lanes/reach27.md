| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| reach27 native entry census; engine | Codex / .claude/worktrees/reach27 / reserved reach27 (detached HEAD) | WIP; code 9a00befc0, data ab59fe126; ABI unchanged, no seam reservation, questions none; Rust 7839, clippy 0, CTest 115/115; pilot-off-3, pilot-on-2, pilot-on-3, calibration-1 all clean; shadows 25/25 each | 1. Supply canonical house script/shadow ini. 2. Repeat census/controls, including independent CanCreateLot counts. 3. Resolve concurrency undercount and whole-city noninterference limits before ready. |

Implemented FC4Reach.cpp/.h, fixed-storage x86 VEH, entry snapshots/changed-entry rejection, month caps, thread stepping, TSV, synthetic DLL; reach is off by default. No C ABI exports changed.
Code milestones: 097716c3d, 60da31211, 41a820d2c, 9a00befc0. Final qbuild all: Rust 7839 passed, clippy 0 diagnostics, native build PASS, CTest 115/115; no owned Rust files.
PROVISIONAL DATA: docs/abi/reach.tsv is the exact on-2 census body plus provenance comment, NOT the canonical house result. Canonical house script/profile were unavailable in permitted inputs; no tools/ source was read.
Pilot: Konradshohe, original simulators (disable=all), zones/lotdev shadows, four pricezone actions, turbo; requested 2026-04-30 end, queued quit reaches May 1: five windows including partial Jan/May. Load/shutdown/monthly fixture passes excluded.
(1) Off-3/on-2/on-3: clean, ref_warnings=0; zones+lotdev mismatch lines 25/25 exact per run; four scripted results identical (ok=1, error=0, cost=20, accepted=2/16). Budget/demand shadows inactive with original simulator; full house profile NOT validated.
(1) Whole-city equality NOT established: final funds off/on2/on3=-16/4/13, residential populations=20275/19255/19469. Separate originals are nondeterministic (lessons 5.3); those differences cannot prove tracer noninterference.
(2) Calibration PlaceZone=260 exactly: 256 independently logged monthly shadow calls + 4 scripts; initial load's 64 calls precede tracing. Natural pilots=4/4, across four windows. CanCreateLot=16930 uncapped calibration, 128/96 capped pilots; independent CanCreateLot oracle still missing.
(2) Synthetic DLL: expected capped A=29/29, B=44/44, float/flags/RET/fault=2/2 each; 3 windows, cap32, 5 ignored probes, suspend/resume, hook/unsafe skips, foreign exceptions and fault recovery pass. Final CTest same-entry stress=513/10000 observed (9487 missed): restore/step counts are lower bounds, NOT a natural-reach ceiling.
(3) Loaded wall seconds off3/on2/on3=5.000/8.953/9.000: 1.7906x/1.8000x (<2x pilot target). Harness wall=24/21/24s (includes staging noise); traps=1103442/1102300, rates=123248/122478 per loaded second.
(4) Two full-roster reach-on pilots clean, no crash/hang; both foreign exceptions/ref warnings/aborted steps/slot exhaustions=0. Calibration also clean. These are 2 integration pilots, 0 canonical house runs.
(5) On-2 coverage below; zero-sized ABI rows contribute zero bytes (2 such rows reached).
| On-2 observed metric | Functions | Known bytes |
|---|---:|---:|
| Reached | 7021 | 1453568 |
| At least 20 capped entries | 5138 | 1033584 |
| At least 3 windows hit | 5035 | 1052431 |
| Both thresholds | 4592 | 926211 |
Repeat on-3: reached 7043/1467102 bytes; >=20 entries 5111/1030666; >=3 windows 4990/1037085; both 4576/924248. Differences are separate-run sampling, not bit-exact equivalence evidence.
Roster 35648, armed 35277, skipped 371: exception/loader runtime 43, preexisting jump/hook 258, trap-sensitive prefix 69, unreadable/non-executable 1. Every skipped address/reason is in log and TSV; no late conflicts. Four process-wide monthly probe pauses per pilot omit concurrent activity during those passes too.
Evidence: build-runs/game-results/reach27-{pilot-off-3,pilot-on-2,pilot-on-3,calibration-1}.{log,verdict}; on/calibration .reach.tsv; reach-report.py and verify-reach-result.py; native control stdout in engine/build-x86/Testing/Temporary/LastTest.log; qbuild/fc4verify logs.
Earlier evidence kept distinct: off-1 rejected (script outside [ini]); on-1 clean but tracing unavailable (zero-size ABI row, fixed); off-2 superseded baseline; initial gate 114/115 and changed source snapshot archived under build-runs/reach-initial-gate/. Final build DLL SHA256 starts 477bf162; data header records complete DLL/input hashes and source commit.
