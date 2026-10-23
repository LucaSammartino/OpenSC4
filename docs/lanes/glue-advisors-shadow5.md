| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-advisors-shadow5: record-only advisor shadow | Codex / glue-advisors-shadow5 | wip: gate passes; clean 2032 game, advisors 0/73 exact; AD17/AD18 filed | Resolve record integration and loading-transition contracts, fix load date, then repeat gate/game. |

## Report (2026-10-23)

Implemented `FC4AdvisorsShadow.{h,cpp}`, enabled only by `advisors_shadow = 1` (default off), with startup/load/month/shutdown wiring. The original system remains installed: no replacement registration, native Read, simulation, Lua execution, or private-state access. The shadow captures the original's section-11 records via its serializable interface, resolves family references to separate core objects, reads them, writes them back and compares bytes strictly. MySim advisors are six identities in the system map; their records are explicitly out of scope under section 2.1. External subject references use audited acquisition/release. Advice event IDs, which advice records omit, are supplied from the original's specified getter before core Read.

The native `advisors_shadow_test` exercises a nested system/advisor/subject graph, opaque external reference ownership, rejected record versions, missing system, and zero native Read calls. The existing test dispatcher gained an operation; **C ABI exports did not change**. Implementation: a8c61b7f; local string-framing correction and fixture diagnostics: d2c2ea93. No core changes were guessed.

Validation on final source: focused advisor tests **63 passed / 0 failed**; `qbuild all` workspace tests **6229 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**. Native `qbuild verify` job `qbuild-20261023-202127-build-ctest` **PASS**, build **220/220** (8.2 s), CTest **69/69** (76.6 s). All asynchronous operations reached terminal results; none is outstanding.

Two earlier failures are preserved. Initial native CTest was 68/69: the new fixture found missing GZStr length framing in the shared output callback. The correction is local to this owned shadow; its output callback prefixes the payload length. Evidence: `build-runs/fc4verify-ctest-initial-failed.log`. The subsequent native configure failed on Ninja recompaction (`Permission denied`), before compilation. Evidence: `build-runs/fc4verify-configure-recompaction-failed.log`. Moving the disposable `.ninja_log` and leftover `.ninja_log.restat` to `build-runs/ninja-*-before-reset.txt` allowed the queued verification above to pass. Rust results are in `build-runs/qbuild-{test,clippy}.log`; final native logs are `build-runs/fc4verify-{build,ctest}.log`.

Game: **adv-shadow5-20261023-r1-2032 0/73**. [Run record](../runs/adv-shadow5-20261023-r1-2032-326.md). Konradshohe turbo reached **2032-01-02**, verdict **clean**, `ref_warnings=0`, foreign exceptions/stalls/rtc/overruns all zero. DLL SHA256 `ef1385a5cc22bf8332077acd445d8a138c45716a3d99ec39b7313855e4910aa8`. There were **73 advisor lines, zero exact**, with 1406 record differences across samples (minimum 1, maximum 28). Load: 766 records, 53969 bytes, one difference. Last comparison, 2032-01-01: 769 records, 54038 bytes, 17 differences. Full verdict/log: `build-runs/game-results/adv-shadow5-20261023-r1-2032.{verdict,log}`. No verification credit is claimed and no regress/coverage tooling was edited.

Questions appended to `docs/specs/questions/advisors.md`:

- **AD17:** clarify CitySituationAdvice (`0x8BE3753B`) base-record integration versus the spec's delegation of that class. The core factory does not register it; the CitySituations advisor (`0x4BE372CD`) differs at byte 61 of a 2369-byte record in every sample.
- **AD18:** advice Read explicitly sets persisted text-dirty (`0x10`), so a strict live-record Read/Write need not be identical. Observed plain-advice differences begin at flags byte 10. Specify whether a record shadow may restore that input bit or must report the prescribed loading transition. No flags were masked or normalized.

Diagnostics retain only the first three differing records per sample; the aggregate counts every failure. The questions cover the observed integration/flags differences; the remaining individual records have not all been classified. A normal-speed run was not used to override these failures: the CitySituations record already differs at load, before turbo drives months, and the Read transition is independent of pacing.

Known additional issue: the load line prints `-4713-11-24` because the observer has not published its Julian date yet. Before the next run, use the city simulator's `GetSimDateNumber()` as `FC4CitySitShadow.cpp` does. This affects the date label, not the byte comparison.

Stop WIP pending AD17/AD18. Next: resolve those contracts, implement and test any specified core/adapter corrections, fix the load date, rerun the full gate and an exact real-city shadow through 2032. Do not install a replacement factory. All work is committed; no game/build request remains outstanding.
