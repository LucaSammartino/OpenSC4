| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-factories24 (partial safety fix) | Codex / `glue-netocc-factories24` | ready partial: Rust 7713/7713, clippy 0 diagnostics; corrected native verify PASS, CTest 100/100; house and native-tool runs clean through 2032, netocc_shadow 73/73 each | Implement remaining eleven factories and a compatible Rust pole connection path |

Code: retain native pole registration; isolate Rust pole tests behind the seam and check independent lifetime/state.
Fix: move owned adapter storage after the interface bases; native pole Write requires the line serializable view at primary+8 (network-occupants §§4.4, 5.1).
Tests pin line+8 and pole+12 serializable views and exercise native-style line QI; adapter size is checked at compile time.
Enabled netocc logs that poles remain native. No C ABI exports changed; shadows remain default-off.
Gate: qbuild all passed (7713 Rust tests, clippy zero diagnostics, native build and CTest 100/100) before the layout diagnostic.
After the measured layout correction, qbuild verify passed: build 188/188 edges, CTest 100/100. Rust inputs were unchanged.
netocc-f24-house-2032-20261025c 73/73
House list plus netocc: clean 2032-01-01, ref_warnings=0; every shadow measurement mismatches=0.
netocc-f24-tool-2032-20261025c 73/73
Native power-line tool: both scripted type-5 placements ok=1; clean 2032-01-06, ref_warnings=0, every shadow measurement mismatches=0.
First placement increased measured poles/lines from 3/18 to 5/27. This exercises native poles, not Rust pole AddConnection storage.
Evidence: build-runs/game-results/<labels>.verdict and .log; script build-runs/netocc-f24-tool-20261025b.script.
Failed evidence retained: house/tool labels ending 20261025b hit native pole Write null-read at exe+0x1D99E0 before the layout correction.
Missing: Network, PreBuilt, Bridge, Tunnel, Underground, Pipe, Subway, UndergroundConnection, PipeConnection, SubwayConnection, Pylon factories.
Rust poles remain isolated: native non-virtual connection access is not layout-compatible. No new factory credit claimed.
Questions: none filed. No tools, coverage tables, regress profile, or run-record files edited.
