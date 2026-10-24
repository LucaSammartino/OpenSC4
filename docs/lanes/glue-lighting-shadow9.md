| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lighting-shadow9: lighting owner shadow | Codex, `glue-lighting-shadow9` | ready; scoped extension, full gate PASS, house2032 clean/ref_warnings=0,45552/45552 exact | Land scoped extension; ordered owner inputs required before whole-owner credit. |

## Changes and scope

Merged `re-lighting-lt14` in ccf47280a, resolving three shared-file conflicts additively. Removed this lane's open LT14/LT15 duplicates and ran the explicitly requested `python tools/questions.py index`; the answered copies remain. No new questions were filed.

Source d441508d5 retains the original default-off 14-field comparison, and adds 610 independently expected result words per monthly capture: nine results for each of all64 property-presence combinations in `GetDecalShadowParams(nonnull)`, plus seventeen results for each of two empty `CalcShadowProjection` calls (flags0/7, false return and unchanged64-byte matrix). The original null-holder comparison follows these queries, checking that overrides do not mutate shared parameters. Expectations use Rust core calculations and independently supplied properties/canaries; native outputs never seed or resynchronize the model. No native setters, Init/Shutdown calls, synthetic messages, hooks or retained owner references are used.

Diagnostic correction0023e8fee supplies actual scalar u32/f32 properties rather than one-element arrays, retaining arrays for pair/RGB values. Runtime `TypeOf` checks enforce the input record's promised shape. Corrected LT15's shipped texture in the core to `0x69D3A23B`; added the five spec conformance vectors23–27 and the delivery-time visual-hour/minute arithmetic helper. Five core vectors cover texture retention/overrides, visual reads, loaded-clock/month history, view/time ordering and shared-property persistence across owner lifetimes.

**The whole cSC4LightingManager owner is NOT compared.** LT14 §1.6 explicitly says a late live snapshot cannot reconstruct history-sensitive state. This observer lacks a construction-to-query transcript of direct tint/reset calls, consumed settings including LT1's byte, view/time ordering, terrain notifications/heights and earlier shared-property reloads. These scopes are logged unavailable, not initialized from native outputs. Slots0–13,15,18–29 and nonempty slot17 remain uncompared; the cImageARGB8 owner is also outside this comparison. Slots14/16 fallback comparisons remain bounded to the captured exemplar scenario and its unchanged effective shared parameters; no general replay across unobserved reloads is claimed. Full override queries and the empty-projection contract add only their stated query coverage.

No public C ABI exports changed; the added Rust functions are private static-link transports absent from fc4.h and the DLL export list. No ABI version bump.

## Validation

- Focused tests:64/64 (59 core,5 C API); explicit-owned-file formatting PASS for the four changed Rust files. Initial focused compile failure (f64 supplied to the f32 register constructor) is preserved in `build-runs/lighting-focused-r3.log`; corrected results in `lighting-focused-r3-corrected.log`. The quiet Bash wrapper failed startup; a local Python runner collected the actual Cargo operation instead.
- First full gate:7021 Rust PASS,clippy0,native217/217 PASS,CTest79/80. `capi_smoke` linked current Rust ABI98 against an inherited object compiled with headerABI97. Both current source declarations already said98. Preserved evidence: `build-runs/lighting-r3-stale-abi-*.log`; removed only generated `capi_smoke.cpp.obj`, then reran the gate after that diagnosed input invalidation.
- Gate after stale-object correction:7021 Rust PASS,clippy0,native51/51 PASS,CTest80/80; collected job `qbuild-20261024-102901-build-ctest` / session95504 to terminal PASS. Its logs are preserved as `build-runs/lighting-r3-array-fixture-*.log` because the first game run then diagnosed a fixture shape error.
- **Final stable source0023e8fee: full qbuild all PASS**, Rust workspace7021/7021 summed across test-result lines, clippy zero diagnostics, native210/210 edges, CTest80/80 (89.1s). Job `qbuild-20261024-105741-build-ctest` / session31588 collected to actual terminal PASS. Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No source changed after this gate.

## Real-game measurements

Both unique round3 requests used the same house settings as the earlier accepted parameter run, plus lighting_shadow=1, Konradshohe, turbo, target2032-01-01, seconds1500/timeout1800. Each was waited through a Windows directory-notification operation without qwait; completed verdicts were retained. No request or wait remains outstanding.

| Record / unique label | Result |
|---|---|
| [Run495](../runs/lighting9-r3-extended-house2032-20261024a-495.md): `lighting9-r3-extended-house2032-20261024a` | Clean2032,ref_warnings=0,real873.531s/wall889s. Original73/73 exact; new0/73,9344 mismatched words of44530. Four scalar fixtures had array shape, accounting for128 differences per capture; diagnostic source correction0023e8fee. Failed comparison preserved. |
| [Run496](../runs/lighting9-r3-extended-house2032-20261024b-496.md): `lighting9-r3-extended-house2032-20261024b` | **Accepted:** clean2032,foreign_exceptions=0,watchdog_stalls=0,rtc=0,overruns=0,ref_warnings=0; real950.875s/wall980s. Original73/73 captures,1022/1022 words; added73/73 captures,4818cases,44530/44530 words exact. Total45552/45552,624 words per capture,2026-01-27 through2032-01-01,resyncs=0. |

Accepted DLL SHA256: `42A0D20EE5A9FEE4734588430BDF2779C320B1451D4DEC661E90191863ABAB93`. Independent exemplar input presence=2F,texture=69D3A23B. Final log has both shutdown summaries73/73 exact. Evidence remains under `build-runs/game-results/<label>.log` and `.verdict`. Prior differences were input-shape related, not demonstrated only under turbo; corrected turbo comparisons are exact, so no normal-speed retry was needed.

Main's run records were rechecked immediately before writing: highest490. Renumbered this lane's four older colliding records469–472 to491–494, preserving their labels, results and evidence: [initial texture diagnostic](../runs/lighting9-house-2032-r1-20261024-491.md), [reference-warning run](../runs/lighting9-house-2032-r2-20261024-492.md), [stalled run](../runs/lighting9-house-2032-r3-20261024-493.md), and [earlier accepted parameter run](../runs/lighting9-house-2032-r4-20261024-494.md). Round3 uses the next free495/496. Older records describe their historical source and then-open questions, not current coverage.

## Landing

Ready for the scoped query extension. Merge queue uses the full actual detached HEAD; the supervisor publishes that commit to the reserved lane branch. No whole-owner verification or replacement credit is claimed. Next implementation work requires independent instrumentation of the complete ordered input history described in LT14 §1.6, rather than another snapshot approximation.
