| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| build-speed31 build/cache tooling; kind tools | Codex / C:/Users/tlori/FreeCity4/.claude/worktrees/build-speed31 / build-speed31 (detached) | WIP; code 462b269f3; ABI unchanged; cargo 8253 passed, clippy 0 diagnostics, corrected ctest 143/143; game/shadow n/a; questions none | 1. Measure matched gates across three worktrees. 2. Tune only measured wins. 3. Resolve tools-suite failures and enqueue ready. |

- Credit NOT flipped: required >=2x median qbuild all, >=5 gates across >=3 worktrees; valid paired sample count 0.
- Historical before (coordinator): 687 builds/40 h, median 155 s, p90 462 s; ctest 21.7 h; Rust cache 6 hits/1149 misses.
- C++ fix: normalized shared disk cache, per-root server configuration, compiler launcher maps cached /showIncludes before Ninja records dependencies; CMake rejects basedirs without the mapper.
- Real train278 proof PASS in shared-worker CTest: A miss, B hit with foreign dependency and stale success; mapped B hit records B, then B header edit rebuilds object AND executable and changes test exit to 1; cache 2 hits/2 misses.
- Rust: qbuild test/clippy, fc4verify and CMake share cache env and build-x86/cargo; registry crates use their stable manifest cwd; workspace/OUT_DIR crates retain path-sensitive keys.
- Real adler2 registry source: A miss then B hit, Rust 1 hit/1 miss (100% hit for B); engine Cargo.lock has four FC4 crates and ZERO third-party crates, so dependency hard-link seeding was not adopted.
- Integration test binaries: 47 -> 3 (4 core + 7 formats + 36 conformance -> one each); module-coverage test guards against omitted suites; all 8253 Rust tests passed, zero ignored.
- Linker: rust-lld not adopted without complete comparative validation; direct cl compile and rust-lld console link both exited 0. No plugin harness/ini changes.
- Queue: --jobs=1 and FC4_QBUILD_MAX=3 unchanged; throughput retuning and <=85% commit validation across passing gates remain outstanding.
- Per-technique gate median/p90, CPU-seconds and disk before/after: N/A for cache, Cargo unification, consolidation, linker and queue; no valid five-gate comparison and no 2x claim.
- Exact required MAIN/qbuild.py --wt WT all ran once: cargo 8253/0/0, clippy exit 0/diagnostics 0; native build PASS 350/350 (175.1 s), initial ctest FAIL 142/143 due to probe comparing absolute versus Ninja-relative paths.
- Corrected MAIN/qbuild.py --wt WT verify: PASS; build 29/29 (2.3 s), ctest 143/143 (96.5 s), native total 98.9 s; C++ 19 hits/0 misses. Warm corrected run is not a before/after performance comparison.
- python -m pytest tools/tests -q --tb=short: 562 passed, 3 failed in 185.56 s; one sandbox Ninja timeout and two unchanged test_ledger byte-weighted-report cases. Full log: build-runs/tool-tests-final.log.
- Targeted real-input/layout tests: 9 passed, 1 deselected in 1.61 s; the deselected Ninja regression subsequently passed through native CTest, not through this sandbox.
- Restricted shell: even Ninja running cmd /c ver hangs; Bash fails CreateFileMapping/Win32 error 5; sccache automatic startup notification pipe is denied. Foreground test server starts successfully.
- Invalid baseline discarded: 1064.67 s wall, 365.69 CPU-s, peak system commit 42.05%, artifacts 630749779 bytes; missing sparse-checkout spec inputs plus 900 s Ninja configure timeout. Sparse setup corrected for subsequent runs.
- Post-validation logical artifacts: engine/target 984430761 bytes + engine/build-x86 3111085108 bytes; includes required MAIN harness's legacy target directory, not a clean optimized worktree disk comparison.
- Benchmark scaffold tools/build_bench.py prepares three isolated local Git checkouts and records verdict/log, wall, descendant Job CPU, sampled commit and logical disk; pin technique revisions and account for pre-existing cache-server compiler CPU before using its numbers as acceptance evidence.
- Artifacts: build-runs/speed-bench/baseline.json (INVALID), build-runs/build-results/qbuild-20261026-235439-build-ctest.verdict (corrected PASS), engine/build-x86/Testing/Temporary/LastTest.log (real stale/fixed proof).
- Code milestones: 91cba8bb2, a1385073b, e54a4f8ce, 462b269f3. No ABI exports, saves, third-party DLLs, coverage-map.tsv or regress.ps1 changed.
- Next: run matched >=5 baseline/after gates on >=3 worktrees outside this shell, capture Rust/C++ rates and compiler-server CPU, measure lld and shared CPU/commit limits, retain only measured improvements, resolve Python-suite failures, then enqueue ready.
