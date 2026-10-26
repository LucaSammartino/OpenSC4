| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-factories24 (partial safety and serialization tests) | Codex / `glue-netocc-factories24` | ready partial: cargo 7713/7713, clippy 0 diagnostics, CTest 100/100; preserved house/tool game verdicts clean through 2032, netocc_shadow 73/73 each | Implement eleven remaining factories and layout-compatible Rust pole connections |

This round adds executable regression tests, not new factories.
The native pole serialization call through line primary+8 now actually invokes Write, beyond the earlier QI-only test.
Every truncated line-record prefix is rejected while preserving the old pair of counted pole references.
The truncation tests include failure after acquiring the first incoming reference.
A complete read resolves two references to the same pole independently, releases the old pair, and writes both IDs back.
Explicit clearing and final Release return the mock pole to its original reference count.
Full stable-source qbuild all PASS: Rust 7713/7713, clippy zero diagnostics, queued native build 53/53 edges, CTest 100/100.
Evidence: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log.
No plugin runtime source changed this round; the completed, preserved game verdicts below remain applicable.
netocc-f24-house-2032-20261025c 73/73
House list plus netocc: clean 2032-01-01, ref_warnings=0, every shadow measurement mismatches=0.
netocc-f24-tool-2032-20261025c 73/73
Native power-line tool: both scripted type-5 placements ok=1; clean 2032-01-06, ref_warnings=0, mismatches=0 throughout.
Run records: build-runs/game-results/<labels>.verdict and .log; requests in build-runs/game-queue/done/.
Prior fixes retained: native pole registration and native serializable-view offsets (line+8, isolated pole+12).
Missing factories: Network, PreBuilt, Bridge, Tunnel, Underground, Pipe, Subway, UndergroundConnection, PipeConnection, SubwayConnection, Pylon.
Rust poles remain isolated/default-off because native non-virtual connection methods require compatible concrete storage.
The safe live path still uses native poles; no Rust pole connection-path or additional factory credit is claimed.
No C ABI exports changed; shadows remain default-off. No tools, specs, coverage tables, or run-record files edited.
