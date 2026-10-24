| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-renderprops-shadow9: render properties round 2 | Codex / `glue-renderprops-shadow9` (engine lane, detached HEAD) | ready for measured partial scope: cargo 6999/0, clippy clean, CTest 80/80; house renderprops 73/73 exact to 2032, clean, ref_warnings=0; RP8 filed; no public ABI change | (1) Land measured registry and owned-fixture coverage. (2) Refresh questions index. (3) Implement RP7 trace/view bridges before whole-owner credit. |

## Result and coverage

Merged `re-renderprops-rp7` in `88601f276`, preserving both sides of additive plugin lists and removing this lane's open RP7 copy. Its answered pointer remains. Added isolated native-owner comparisons in `5739fb9f1`; diagnostic fixes are in final engine source `1d55acf7c`. Later commits change documentation only. `renderprops_shadow` remains default-off, with no public C ABI export changes or ABI-version edit.

The existing **542 application registry fields per month** remain: three counts, 77 full names, and six Find probes per entry (462 results), using independent production-core C-locale ASCII expectations. The acquired application stays live throughout borrowed properties use. No borrowed application/view values are changed or used to bootstrap expected state.

Each monthly call also constructs two private original owners in aligned caller-owned 0x30-byte storage, after checking the original application vtable and three documented getter slots against the supported image binding. Each has a distinct Rust `RenderPropertiesOwner`, initialized independently from registered defaults. Fixture plan `finite-v1` compares:

- All 77 default values in both instances.
- All valid indices of all three native setters. Three passes each issue every write twice, preserving duplicates: Boolean false/true/false; integer -17, INT_MIN+index, INT_MAX-index; float binary32 negative zero, 1.5, -17.25. After every completed native void setter, replay only the independently generated input and compare all 77 getters. No setter status is inferred.
- The second instance stays at defaults through every pass and after destruction of the first.
- P/U QueryInterface acceptance of IID 1, U pointer adjustment, rejected IDs with unchanged output/null output, shared initial/reference counts, AddRef and non-final Release through both views.
- U deleting destructor flags 2 and 0, checking its P return against the core model. Only private fixture tables are destroyed. Outer storage remains caller-owned; the audit accounts for the acquired reference consumed by explicit non-freeing destruction.

Each fixture sample has **36,520 checks**: 36,058 native comparisons and 462 Rust replay-acceptance checks. With the unchanged registry, that is **37,062 checks/sample**. No hardware debug register or exception-dispatch state is changed. Getters use immutable Rust state without RefCell.

## Measured game result

**`renderprops-shadow9-r2-fixture2032-20261024a`: clean, renderprops 73/73 exact**, including all 73 available native-fixture records, **2026-01-27 through 2032-01-01**. Zero duplicate months, zero resyncs. Total **2,705,526 checks** (2,671,800 native comparisons plus 33,726 replay-acceptance checks); **33,726 native setter invocations** completed on isolated fixtures. See [run 492](../runs/renderprops-shadow9-r2-fixture2032-20261024a-492.md).

Konradshohe, the previous train187 house switches plus renderprops_shadow=1, turbo, end_date=2032-01-01, seconds=1800, timeout=2100. Clean shutdown at target after **712.203 seconds**, 725 seconds queue wall, slot 3. foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0, Rust panics=0, structured exceptions=0, other WARNING lines=0. Two expected startup x87 warning lines occurred, as recognized by the harness. No turbo-only difference occurred, so no normal-speed repeat was required.

DLL SHA256: `FD4B31E3C96B9D591E829A27E1AD32EE87420F9B3BF202F35991BE7B92DD506E`. Evidence: `build-runs/game-results/renderprops-shadow9-r2-fixture2032-20261024a.{verdict,log}`; bounded collector `build-runs/renderprops-r2-evidence.py`. The unique request was submitted once; its event watcher was collected to terminal success without qwait. No request remains outstanding, and no completed verdict was discarded.

## Checks and preserved failure

Final `qbuild.py all` on `1d55acf7c`: **PASS**, workspace **6999 passed / 0 failed / 0 ignored**, clippy `-D warnings` zero diagnostics, native build **49/49 edges (2.1 s)**, **CTest 80/80 (79.9 s)**. Queue `qbuild-20261024-095420-build-ctest` reached terminal pass. Logs remain in `build-runs/qbuild-{test,clippy}.log` and `build-runs/fc4verify-{build,ctest}.log`. Focused transport tests **2/2** passed; explicit owned-file formatting passed.

The first all operation on `5739fb9f1` passed Rust 6999/6999, clippy, and native build 221/221, but CTest was **78/80**: the ret-channel checker associated the fixture helper named Destroy with unrelated Lua Destroy, and capi_smoke reused a cached train-integration object with compiled header ABI97 while current Rust/header are ABI98. Renamed the helper to TerminateFixtureStorage and added compiled-source provenance to the smoke mismatch diagnostic, forcing a current-source rebuild. No version number changed. Failed logs are preserved separately as `build-runs/renderprops-r2-gate1-{test,clippy,build,ctest}.log`; failed queue `qbuild-20261024-094717-build-ctest` was collected to its terminal result before corrections and rerun.

## Still uncompared — whole-owner target remains unmet

- **Live configured values and genuine game setter inputs:** RP7 explicitly provides no public ordered-history service and no existing continuous observer entitlement. This lane attaches after startup configuration. Its fixture tests exercise assignments/getters but do not predict the live application from incomplete history. Live values remain unavailable under RP7-7/8.
- **Private view S and copying:** no complete-V identity/lifetime bridge is installed. Embedded S is never acquired, independently released, or deleted. Scroll save/restore, setter-bypassing table restoration, P+8 reference-field copies, flag-change behavior, view lifetime and text-helper integration remain uncompared. No pure-model test is counted as native view coverage.
- **Freeing lifetime paths:** native Release at count 0/1, deleting destructor flags 1/3, overflow/wrap boundaries and heap-free effects remain unmeasured. Caller-owned fixture storage must not be passed to a freeing destructor.
- **Other domains:** exceptional x87/NaN inputs, unchecked invalid indices, noncanonical Boolean bytes, non-C/high-byte name behavior (RP6), and extra metadata without public getters (RP1/RP2) remain outside this measured profile.

This is ready to land as a measured extension of partial coverage. **Do not award whole-owner verified credit.** The whole-owner target requires coordinated construction-to-destruction input/copy observation, a complete-view bridge and a specified native heap fixture, with additional domain measurements.

## Questions and follow-up

**RP8** appended to `docs/specs/questions/render-properties.md` asks confirmation of the literal receiver-only private constructor/caller-owned-storage ABI used by these successful fixtures; its interpretation is marked in code. RP7 was removed from the open file in favor of its answered copy. The requested `tools/questions.py index` was not executed because the task's non-bending clean-room rule prohibits scripts under tools/. Coordinator must regenerate/check the questions index before landing. Generated coverage tables and shared regress profiles were not manually edited.

Historical registry-only results remain unchanged in [run 490](../runs/renderprops-shadow9-20261024-r1-490.md) and [run 491](../runs/renderprops-shadow9-20261024-r2-491.md).
