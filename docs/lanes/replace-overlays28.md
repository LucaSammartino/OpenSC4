| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-overlays28 (OV17 rectangle consumer boundary) | Codex / `replace-overlays28`, detached HEAD; code `0450aca23`; ABI unchanged | READY TO LAND OV17 ADAPTER ONLY: cargo 7959, clippy 0, CTest 129/129; clean 2032; shadow 73/73; native_interception=0; native replacement INCOMPLETE | Land the consumer adapter; do NOT flip cSTEOverlayManager replacement credit. Finish pre-Init routing/services and full acceptance. |

Merged main first (`2b6c01c66`), including the OV17 answer in overlay-manager section 4.6.1. No speculative call signatures remain at this boundary.
0450aca23 implements all eight version-gated consumers: complete terrain-view base in ECX, rectangle value/context (20-byte cleanup), context/extension/rectangle value (24), rectangle pointer/context (8); water AL ignored.
Real-DLL tests exercise all eight x86 calls against independent word-based receivers, ESP balance, both water results, missing targets, extreme bounds and Rust DrawFrame submissions without a second bounds expansion. Focused CTest 3/3; full CTest also passes these tests.
Native Draw remains disabled: production frame services/device commands and terrain-view pre-Init routing are NOT installed. The portable frame must never be cast from a native context. The overlays key stays default-off.
One final qbuild all PASS: cargo 7959 passed/0 failed/0 ignored; clippy 0 diagnostics; CMake 43/43 edges; CTest 129/129. No Rust files changed; fmt n/a.
Gate logs: build-runs/{qbuild-test,qbuild-clippy,fc4verify-build,fc4verify-ctest}.log; receipt build-runs/build-results/qbuild-20261026-173050-build-ctest.verdict. Earlier quick build PASS 326/326 edges; focused wrapper/PATH launches did not execute tests before the direct CTest pass.
Final gated DLL SHA256 c1113959395fa415c68d7c44a48e7c446f7eccd2409ddb719bbb4ac5ff0429e3; final game runs below both report c1113959.
Game evidence root: C:/Users/tlori/FreeCity4/build-runs/game-results/; each named run has .verdict and .log. All requests use the shared main queue, parallel=1, at most two outstanding.
(a) NOT MET: no full-house replacement run. ro28-r8-final-2032-a enables overlays plus defaults, clean at 2032-01-01, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0 (.log:12008); native_interception=0.
(b) Key-on baseline exact: contract 73/73; records 292/292, incomplete=0; controlled replay 292/292; all mismatches=0 (ro28-r8-final-2032-a.log:11809-11811).
Those are native-manager observations, not replacement/history proof: history=unavailable(OV13), source/image data unavailable. No difference is being waived.
Final baseline probe: targets 8/8 (.log:1574), factory/native-prefix/live-bindings 71/71, mismatches=0 (.log:1581). Targets prove version-gated resolution, not live execution of terrain consumers; native_interception=0.
(c) NOT MET: ro28-r8-final-gen-a, gen27-konradshohe-s0-default with overlays plus defaults, corrupt at 2028-01-28; foreign_exceptions=1, ref_warnings=0, stalls=0 (.log:18773).
All 944 gen actions ran (.log:18552); contract 25/25, records/replay 100/100 exact, mismatches=0, incomplete=0 (.log:18563-18565).
Gen shutdown null-read AV at SimCity 4.exe+0x1D0F94 (.log:18680), matching the previously measured key-off ro28-r7-source-gen-control-a.log:18733 on older DLL 3c1969d2. No unrelated fix or new control claim.
Earlier quick-build DLL b23026a5: ro28-r8-ov17-2032-a clean, ro28-r8-ov17-gen-a corrupt with the same shutdown fault; both shadow-exact. These are separate from final gated-DLL evidence.
Questions: none filed; OV17 was answered on main. No specs, coverage files, replace list, C ABI exports or ABI version changed.
Credit flip requested by the task: cSTEOverlayManager (30,436 B), verified -> replaced. NOT achieved; overlays is NOT ready for the replace list. The ready queue entry lands only the completed OV17 adapter/tests.
Next: install pre-Init terrain-view routing and production services/native drawing, capture original input history before allocation, then measure full-house 2032 and a clean gen action scenario with overlays before requesting replacement credit.
