| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-portdev-complete4 (PD12 native prop switching) | `Codex` / `glue-portdev-complete4` (engine lane) | **ready**: implementation d7ebfcde, diagnostic cb4fdf09; ABI unchanged; cargo 4289/0, clippy clean, ctest 68/68; final shadow 144/144 exact, final replacement clean2032/ref0; actual saved-port reload clean2033/ref0, original-save reload shadow26/26 exact; no new questions | Coordinator: union ports with current replacements before default promotion. |

## Report

Implemented the PD12 answer for `enable = ports`. No default promotion, ABI-table edits, CAPI export changes or ABI-version change.

### Changes

- `d7ebfcde`: replace the unwired `SetAccessPropsState` host with native spatial enumeration. Inclusive lot cells go through city slot96 to float world bounds; reconciled occupant-manager slot40 receives a custom filter and allocator-B list. The filter tests prop type, inclusive x/z (NaN rejected), then occupant-holder `HasProperty(0xC98204B8)`. Property values and owning-lot labels are never read.
- Enumeration Boolean is ignored. Release the filter before switching the finished list. Prop IID `0xE9793A65` output nullness governs switching, independently of QueryInterface's Boolean. Preserve provider order and duplicates, take the specified transient AddRef/Release pair, call the acquired prop's void slot11, then release it. Manager slot48 releases occupant references and nodes; the local destructor frees the empty sentinel. Borrowed lot/city/manager/holder pointers are not released.
- `FC4PortProps.h` contains the shared filter predicate and snapshot walk. Native diagnostics separate unavailable-provider fallbacks from valid empty queries and report before/after prop states using reconciled getter slot12. The diagnostic count never controls the advisor event.
- `cb4fdf09`: explicit `portprops x z state` script action, state0 or1, to test the native boundary on an existing lot. It calls no airport/seaport developer. Added grammar checks and additive README documentation.
- `port_props_test.cpp`: independent expectations for all twelve published PD12 vectors, plus bbox/zero-property checks. Covers false/non-null and true/null query outputs, duplicate references, list cleanup, already-set state, and a setter removing a later occupant from manager membership. Native grid dedup/tag policy is emulated by the provider test; it is not claimed as a new native-manager implementation or native mutation experiment.

### Stable gate

Final `qbuild all` on runtime commit `cb4fdf09`: **cargo 4289 passed / 0 failed**, **clippy 0 diagnostics**, **ctest 68/68**, build PASS; no tree-changed warning. Subsequent commits change documentation only. Focused quick and native `port_props`/`harness_logic` checks passed. No Rust source touched, so rustfmt is n/a. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.

Final DLL SHA256: `d949fc5d25576df33f36bbf01308c9307300da6fc42db8f9dfc9d727ed0dd2e6`; unchanged across final shadow/replacement and both captured-city reloads. Initial full gate also passed, but its tree changed during ctest; the stable repeat above is the reported gate.

### Isolated game runs

All seven requests completed **clean**, with **ref_warnings=0**, no foreign exception or stall. All runs used unique labels; no request remains outstanding.

| Label | Actual end date | Port evidence |
|---|---|---|
| pdc4-original-20261022-a | 2032-01-06 | Original developers; airport and seaport built, full save succeeded. |
| pdc4-shadow-20261022-a | 2032-01-06 | 144/144 exact: airport72/72, seaport72/72; both ports present. |
| pdc4-replace-20261022-a | 2032-01-03 | Both ports adopted; automatic PD12 selected3/called3 once; each Write ports=1; full save captured. |
| pdc4-reload-20261022-a | 2033-01-01 | Actually loaded captured replacement save hash38f234...; airport and seaport Read ports=1 before Init; native zero/multiple/transition probes. |
| pdc4-origreload-shadow-20261022-a | 2033-01-02 | Actually loaded captured original save hash559425...; 26/26 exact (13 per developer), same native prop probes. |
| pdc4-final-shadow-20261022-a | 2032-01-01 | Final DLL, immutable source, 144/144 exact; early zero and later three native matches. |
| pdc4-final-replace-20261022-a | 2032-01-09 | Final DLL, both ports; automatic no-access switch3 calls, one advisor event, held references return to0. |

Scripts: `build-runs/ports-complete4-build.txt`, `ports-complete4-reload.txt`, `ports-complete4-transition.txt`. Verdicts/logs are under `build-runs/game-results/<label>.*`.

Native selection in the automatic no-access callback: seaport rectangle `(0,90)-(10,94)`, enum1, tested19, positions13, holders13, matched3, listed3, calls3, unavailable0. Thus ten otherwise spatially eligible props lacked the property. One automatic callback occurs after adoption; later months do not repeat it. The explicit March probe is logged separately. Both developers report ports1, history100, and required budget700/500; the seaport records one advisor event. Native props initially have state1, so that automatic call is1->1, as PD12 permits.

On each actual saved-city reload, the airport query tests37 candidates, examines9 holders and returns0 matches/calls; seaport queries test20, examine14 holders and return3 matches/calls. Each of the three real native props follows **1->0, 0->1, 1->1** across the diagnostic actions. Every query has unavailable0 and the completed runs have ref0. Early construction probes in the final runs return0 on the seaport as well (11 candidates,4 holders); they cannot establish an automatic0->1 transition because the matching props do not exist yet. Snapshot-mutation and unusual QI-result cases are independently exercised by the C++ vectors.

### Captured saves and actual record evidence

Immutable input: `C:/Users/tlori/FreeCity4/build-runs/fixtures/ports/Konradshohe-864e0a8817e2.sc4`, 1520556 bytes, SHA256 `864e0a8817e2b450f76bd0bc9fdb5210d7b0f0be0d34feb35d5f78ea9e4f484a`. Rechecked unchanged after all runs.

Both full saves succeeded at game date2031-12-14. Before slot reuse, copied each stable slot1 city into `C:/Users/tlori/FreeCity4/build-runs/fixtures/glue-portdev-complete4/`, refusing overwrites; adjacent JSON records hashes, source and save metadata:

- `pdc4-original-20261022-a.sc4`: 999140 bytes, SHA256 `55942555271226a66c7d939b40e6a9283c2b337fd64b6ceffbc172c74217a77e`.
- `pdc4-replace-20261022-a.sc4`: 1001692 bytes, SHA256 `38f234433b0c387393a7e25a971953c37b7127ea93c7d5d0a4cb2c789e78b0be`.

Harness verdicts on the two reloads identify those exact fixture hashes and `LoadCity -> true`; replacement Read logs show one port in each developer before Init. These are persisted port-city loads, with no build action in the reload script.

`build-runs/inspect-port-records.py` reads DBPF/QFS using the public engine format definitions. Its JSON results `pdc4-original-records.json` and `pdc4-replace-records.json` show identical22-byte payloads for both class records, excluding object-ID framing:

- Airport `aa2049a1,299b2d1b,00000000`: `020001009d0e27ca0b00010000000000000007000200`, one port, stage0/type0, saved coordinate `(7,2)`.
- Seaport `ea230a28,299b2d1b,00000000`: `0200010092ad29ea0c0001000000000000000a005a00`, one port, stage0/type0, saved coordinate `(10,90)`.

Build top-lefts are `(2,2)` for the6x18 airport and `(0,90)` for the11x5 seaport; saved coordinates above agree exactly between native and ours. Game data and helper artifacts are untracked. No Documents directory was accessed.

### Questions and remaining coordination

No new questions; PD12 was implemented from the published answer, reconciled interfaces and SDK. No shadow differences, so no normal-speed discrepancy run is required. Live growth/advice upgrade was not triggered: this fixture has0 trips and the airport later skips for lack of road access; the shipped seaport has no growth advice. Existing core/conformance growth and zero-prop advisor/once-only recheck tests passed in the full gate; no live upgrade claim is made.

Coordinator must run `ports` with the current replacement union before any default promotion. Keep this implementation opt-in until then. Reserved detached worktree was reused; commits do not switch branches or alter shared refs. Git printed a denied packed-refs lock attempt during commits, but each detached HEAD commit succeeded. Lane guard: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-portdev-complete4-r1-manifest.json`; no subagents or restarts.

## Coordinator same-process receipt

Independently checked pdc4-final-shadow-20261022-a.verdict: **ports 144/144**, airport72 plus seaport72, resynced lines0, clean2032/ref0. This is the final same-DLL real-city shadow, separate from saved-city reload and the combined replacement-union check.

## Coordinator same-process receipt

Independently checked pdc4-final-shadow-20261022-a.verdict: **ports 144/144**, airport72 plus seaport72, resynced lines0, clean2032/ref0. This is the final same-DLL real-city shadow, separate from saved-city reload and the combined replacement-union check.
