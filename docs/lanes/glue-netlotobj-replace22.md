| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netlotobj-replace22: network object ownership | Codex / `glue-netlotobj-replace22` (engine) | Ready measured partial: cargo 7642 passed; clippy 0 diagnostics; CTest 91/91; netlotobj22-r5-crew-shadow-20261025 73/73; netlotobj22-r5-crew-house52-20261025 clean through 2032, ref_warnings=0 | Specify crew creation/Init and configuration IID/container ABI; complete three-class replacement |

## Report

- Prior commits `52a788ed5`, `31aa93259`, `7b220ba6a` fix borrowed lot/cache ownership and pin lot views across replacing iterator callbacks. `3e16477d6`, `52177b0c0` add configuration getters, atomic COM lifetime, and exact-token forced choices; DLL-only tests avoid game allocator use.
- This round replaces the logging-only crew shutdown callback with the specified slot-5 call (no stack arguments; return ignored). It never dereferences a crew after shutdown can unregister and destroy it.
- Expanded `plugin_netlot_ownership` checks message-driven reentrant crew unregistration, balanced references, duplicate registrations, absent removal, repeated shutdown, and the specified bounded loop for a crew that does not unregister. Registration and lot/configuration ownership tests remain covered.
- `qbuild all`: cargo 7642 passed, 0 failed/ignored; clippy 0 diagnostics. Initial native CTest 90/91 exposed the ABI scanner treating typed compatibility-interface probe calls as SDK calls; existing `abi: via cFC4I` markers correct that classification.
- Final `qbuild verify`: build PASS 49/49 edges; CTest PASS 91/91. Verdict: `build-runs/build-results/qbuild-20261025-084443-build-ctest.verdict`. Initial failed evidence remains in `build-runs/r5-initial-failed-*` and the `qbuild-20261025-082054-build-ctest.verdict`.
- Exact shadow line: `netlotobj22-r5-crew-shadow-20261025 73/73`; every line mismatches=0, 0 configuration differences, 0 resyncs; clean through 2032-01-17, ref_warnings=0.
- Full recorded 52-key house profile: `netlotobj22-r5-crew-house52-20261025`, clean through 2032-01-01 in 107.859 seconds, ref_warnings=0, disabled=-. Profile preserved from the preceding measured request; no tools files read or edited.
- Run evidence: `build-runs/game-results/netlotobj22-r5-crew-shadow-20261025.verdict` and `build-runs/game-results/netlotobj22-r5-crew-house52-20261025.verdict`, with matching logs. Both runs used DLL SHA256 prefix `ff339a63`; no requests outstanding.
- Still missing: crew creation/six-argument Init contract and Rust-backed crew construction; configuration IID and output-set layout/allocator. Explicit `// SPEC-GAP:` comments remain; the set bridge stays inactive. No complete three-class replacement credit is claimed.
- Pending branch adds C export `fc4_netlot_set_next_configuration_token`; this round adds no exports. ABI version unchanged; replacement keys and shadows remain default-off. No questions, spec edits, tools edits, README closure tables, or docs/runs records.
