| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-overlays28 (terrain overlay native interface) | Codex / `replace-overlays28`, engine lane, detached HEAD; code `1fe09a0d7` | Continue: full gate green; partial game clean, shadow 73/73; native_interception=0; replacement incomplete | Implement native construction/Init/history routing, concrete resource/draw adapters, then measure house and gen acceptance. |

Carried forward glue-overlays-replace17 code/tests without its records. Native IID 0x895D6544 now answers a separate, corrected 28-slot interface at the FC4-owned factory (CLSID 0xFC401717).
Added acquired-resource/service transport into the Rust owner: source/binding/key/round/city/shadow/model adds, setters/getters, redisplay/subscription callbacks, independent service/resource lifetimes, deferred model binding acquisition after successful projection.
DLL tests cover native output-first colour/AL/byte getters, canonical IUnknown, lookup routes, retained resources, projection failure and quality-zero invalid-pointer safety. Concrete live GZCOM resource callbacks remain unwired.
Native interface is currently a secondary subobject, not the native instance layout with its draw callback at +4. Terrain view still constructs/initializes/draws native managers; factory registration alone cannot intercept those paths (overlay-manager §1; terrain §15.5.4).
Gate: qbuild all PASS, cargo 7852 passed/0 failed, clippy 0 diagnostics, build 47/47 edges, CTest 118/118; native job qbuild-20261026-034248-build-ctest. Owned Rust fmt: 3 files, diff_lines=0.
Gate logs: build-runs/qbuild-test.log, build-runs/qbuild-clippy.log, build-runs/fc4verify-build.log, build-runs/fc4verify-ctest.log; overlay_factory_{off,on,disabled}_test all passed.
Earlier quick qbuild-20261026-031742-quick failed Rust orphan impls/source argument type; fixed-source qbuild-20261026-032902-quick PASS, 5920 tests and build 57/57. Both verdicts retained in build-runs/build-results/.
Partial game ro28-r1-native-interface-partial-2032: shared main queue, parallel=1, this DLL SHA256 prefix 138419cc; clean to 2032-01-04, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0; live factory/native-interface probe 57/57, mismatches=0.
(a) NOT MET: no full-house replace-list run; native_interception=0 is explicit in that log at line 1563. The partial run enabled overlays plus defaults, not the house list.
(b) Key-on baseline measured exact: overlays contract 73/73, records 292/292 (incomplete=0), controlled replay 292/292, all mismatches=0; log lines 12717-12719. This still observes native terrain managers; history remains unavailable (OV13).
(c) NOT RUN: gen27-konradshohe-s0-default from engine/tests/scenarios/suite.tsv still needs replace-mode acceptance after interception/drawing are implemented.
Game evidence: C:/Users/tlori/FreeCity4/build-runs/game-results/ro28-r1-native-interface-partial-2032.verdict and .log; request is in the shared main queue's done directory.
ABI exports unchanged; no spec questions filed. Overlays is NOT ready for the replace list. No merge entry enqueued for this continuation.
Next: implement the native instance/draw layout and version-guarded pre-Init routing; record the initialization/resource/mutation stream before the first allocation, rather than treating a late snapshot as history.
Next: wire production resource/lighting/terrain services and DrawingProducer commands/rectangle consumers, then rerun qbuild all and acceptance (a)-(c) before enqueueing.
