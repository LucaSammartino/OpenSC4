| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-span8888-final4: final 8888 bridge verification | `glue-span8888-final4` / engine lane, detached HEAD | ready for bounded opt-in fix; code d8da324b; cargo4938/clippy0/ctest68/68; own386/386 and combined-reload133/133 shipped comparisons exact; combined427 Rust-rendered; three clean2032/ref0 runs; historical third refusal unproven; ABI unchanged | Keep opt-in and original alias fallback; SR8888-1/SR8888-6 remain open; no fourth round |

## Changes and contract

Code commit: `d8da324b`. Only `engine/plugin/FC4SpanRender8888.cpp` changed in the engine. No C ABI exports, ABI tables, version, shared context owner, maps, defaults, or core pixel arithmetic changed. The renderer remains opt-in.

The old bridge used `source stride * source height` as the source slice length for every kernel. Published span-render-8888.md sections 7 and 9.1 define width/height reads only for tiled kernels. Direct blits use absolute destination x/y against a pre-offset source base; stretch blits use the specified wrapped accumulators. Neither geometry is bounded by that height.

The bridge now enumerates the specified source reads and captures those texels before rendering. It uses signed byte strides and exact address arithmetic, preserves the existing rejection of pointer wrap, checks accessible data pages, and retains a 64 MiB bridge resource limit. Unused bytes before/among sampled rows are not read from native memory; the private Rust source snapshot supplies a representable base even when the native base lies in an unreadable unused prefix. These checks are safety/resource limits, not invented native bounds.

Alias refusal now checks actual source reads against actual destination writes. An intersection still invokes the original renderer, because native load/store grouping remains open in SR8888-1. Overlapping broad envelopes alone do not establish an alias. Source snapshots are disjoint from Rust's mutable destination slice. Shadow spans/descriptors/gradients and destination bytes are frozen before the original runs; native and Rust see identical input values and texels. Entire destination regions, including untouched bytes, are compared.

Added separate constructed/runtime counters, original fallback and unaccounted counts, source-window diagnostics, and explicit resync=0 reporting. No refusal or differing call is counted as an exact comparison.

## Diagnosis and historical limits

The copied second-run result remains **369/372**, with two status-2 rejections (span counts 22 and 5, mode 0) and one generic memory-window/overlap refusal (count 22). Those three old calls were not compared. This lane does not change their historical numerator or claim their exact captured inputs were recovered.

New same-process traces observed both characteristic status-2 shapes with format 4, unkeyed, mode 0:

- Count 22: stride 588, width 147, height 22; actual texel offsets `[107184,120120)`. The old slice length was only `588*22=12936`. The new native/Rust comparisons include this call and are exact in both shadow runs.
- Count 5: stride 64, width 16, height 16; actual texel offsets `[968,1228)`. The old slice length was `64*16=1024`. The new native/Rust comparisons include this call and are exact in both shadow runs.
- Combined replacement also observed count 46, mode 0, format 4, stride 960, height 46: actual reads `[0,43440)`, old envelope length 44160. The old source/destination envelopes overlapped, but the actual reads/writes were disjoint. This call rendered through Rust; it was **not** a native comparison and receives no exactness credit.

The historical third count-22 refusal has no captured descriptors or distinct refusal reason in the copied evidence. It was not reproduced as that identified call, so its precise cause remains unproven. The new count-46 envelope event is not a substitute for that proof. True aliasing remains parked under SR8888-1; no guessed alias order or fallback suppression was introduced.

## Build gate

- Initial `qbuild all`: PASS, Rust4938/0, clippy0, build208/208, ctest68/68. Implementation edits began after launch, so its wrapper correctly reported a changed tree. Logs preserved in `build-runs/baseline/`.
- Focused `qbuild quick`: changed C++ file compiled and DLL linked without compiler diagnostics. Its session handle expired before final console retrieval; the stable full gate is authoritative.
- Final stable `qbuild all`: **PASS**, Rust4938 passed/0 failed/0 ignored, clippy0 diagnostics, build2/2, ctest68/68 in83.49s; fc4verify84.3s. No source changes followed. Logs preserved in `build-runs/final-gate/` and standard qbuild/fc4verify log paths.
- No Rust files changed; fmt n/a. `git diff --check` passed. q.py failed with Bash/Service/CreateInstance/E_ACCESSDENIED; capped PowerShell and the mandatory qbuild wrappers were used.

## Independent native verification

All three game requests use the same frozen DLL and paired PDB from `build-runs/dll-span8888-final4/`. DLL SHA256: `35e199bf7987e883eac33f38ce72de4f5cb903d2b1b0dccade7a7c715277eea4`.

Each run independently reports:

- Constructed direct: **214/214 exact**, 63344 compared dwords, differing0, rejected0, faults0, untouched corruptions0. These call the preserved native functions and Rust separately on identical initial buffers. The existing 198 cases cover all168 kernels and other methods; 16 added cases exercise ignored zero dimensions for direct/stretch across all seven formats and a reserved/unreadable unused source prefix.
- Constructed native context: type8 **254/254 exact**, type9 **254/254 exact**, 235712 compared arena bytes per type. Original changed118992/type8 and118860/type9 bytes, so these are not zero-work comparisons. Faults, refusal, route mismatch, no-renderer-call and moved texture lock counts are all0.
- The context comes from the real draw-system factory, context vtable `00AC17D0`, Init selects renderer vtable `00AC1874`. Destination lock count0->1/flags0->0x8040; Shutdown clears renderer and restores lock0/flags0; context and destination Release both0. There are508 forwarded constructed-context calls per run, accounted separately from shipped calls. Constructed direct tests bypass the forwarders and are reported separately.
- Separate safety control: one expected true-alias refusal with unchanged destination. It is **not a comparison success** and is excluded from native-comparison numerators/denominators, with its refusal explicitly logged.

## Game runs and complete shipped counts

All requests used unique immutable labels, ran one at a time through the guarded queue, and waited using one local wait script without qwait. Every verdict is retained; none failed or was discarded. All have foreign exceptions0, stalls0, rtc0, overruns0 and **ref_warnings0**.

- `s8888-final4-own-20261022-01`: **clean**, 2032-01-06,120.453s. Span8888 shadow; **386/386 shipped comparisons exact**:118 solid+268 blit, differing calls/pixels0. Original fallbacks0, Rust-rendered shipped0, resync0, unaccounted0. Shipped sites: solid118/plain257/tiled11/stretch0. Full save ok1 in11094ms at2031-01-04; all5 scripted actions ran.
- `s8888-final4-combined-20261022-02`: **clean**, 2032-01-01,117.578s. Literal current40+span8888 replacement; **427 shipped calls, Rust427**,27 solid+400 blit, original fallbacks0, shadow comparisons0, resync0, unaccounted0. This run is replacement/profile evidence, **not 427 exact comparisons**. Shipped sites: solid27/plain389/tiled11/stretch0. Full save ok1 in11047ms at2031-01-01; all5 actions ran. Source-window diagnostics: legacy_short2, legacy_envelope_overlap1.
- `s8888-final4-reload-20261022-03`: **clean**, 2032-01-01,28.110s. Same literal40+span8888, shadow enabled, loads the combined run's saved city; **133/133 shipped comparisons exact**:27 solid+106 blit, differing calls/pixels0. Original fallbacks0, Rust-rendered shipped0, resync0, unaccounted0. Shipped sites: solid27/plain95/tiled11/stretch0. First scripted mark/digest ran2031-01-02, confirming the persisted2031 city was loaded; another full save ok1 in10953ms at2031-06-01; all5 actions ran.

Natural gradient/alpha/alpha-gradient calls were0 in these sessions; only constructed comparisons cover those methods. The existing return-site classifier labels the above non-fixture calls as shipped `DrawPreclippedSpanList` calls; other/unclassified0 in all three runs. The target-producing set-target caller is still unidentified (SR8888-6). No native target-producer RE or unrelated context replacement was attempted. All three shutdowns restore the original renderer entries.

The literal40 keys were landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones, development, tract, history, clock24, commercial, industrial, civic, parks, lotdev, zonedev, traffic, automata, netmgr, netlot, nettool, demolition, occmgr, propmgr, lotcfg, citysit, propdev, simgrids, citydetail, ports. Logs confirm all40 plus span8888 enabled; simulator and budget are the two existing defaults (43 enabled rows total).

## Save/reload evidence and artifacts

The combined run saved `Regions/FC4 Test/City - Konradshohe.sc4` in isolated slot2:1003774 bytes, mtime2026-10-22 16:43:34, matching SaveCity completion. A read-only source copy is preserved at `C:/Users/tlori/FreeCity4/build-runs/fixtures/span8888-final4-combined-saved.sc4`, SHA256 `a13df2abef5bd74cb2a7e80199f17972c1f5121f540d98822bbf84dc886fed3a`. The reload verdict records that same hash staged into isolated slot1 and source preserved. Its hash was rechecked unchanged after reload. No game files were committed or real Documents touched.

Raw `.log`/`.verdict` files: `build-runs/game-results/<label>.*`. Derived complete per-method totals and selected evidence: `build-runs/<label>.summary.json`. Request preparation, single-call wait, evidence summarizer, and action scripts are under this worktree's `build-runs/`. Prior raw evidence remains under `build-runs/evidence-run4/`. The three new results are independent measured sessions; the old369/372 result is retained unchanged.

## Questions and disposition

No new question IDs or C ABI changes. Existing `docs/specs/questions/span-render-8888.md` SR8888-1 (true-alias order), SR8888-5 (pointer-wrap rejection policy) and SR8888-6 (target producer) remain open. The historical third refusal remains explicitly unproven; these new exact observed traces do not establish whole-renderer exactness or justify default promotion.

Ready to merge the bounded opt-in fix and evidence. Keep the original fallback for unproven calls. Preserve the historical third refusal and producer/alias gaps for published-contract review, without a fourth implementation round or new RE work. No game request remains outstanding.
