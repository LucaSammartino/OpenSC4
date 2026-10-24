| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-replay6: bounded Lua registration shadow | Codex; glue-lua-replay6 (detached worktree; supervisor publishes reserved branch) | ready: merged-source full gate passed; lua 73/73 exact to 2032, ref_warnings=0 | Land bounded registration changes; retain no full-VM verification credit. |

## Changes and verification scope

Continues glue-lua-shadow5 using the LH6/LH7 answers in lua-host §§1.4.2,
3.5 and 8.6. Full chunk/call replay remains disabled: public interfaces offer
no complete observer, and no replay-input or nondeterminism-exclusion contract
is established. No callbacks, chunks or metamethods are executed by the shadow.

The native sampler now explicitly counts missing C-function bindings (including
`_tf_`) as capture failures, checks stack height after restoration as well as
before it, and records portable preferred-address labels only when VirtualQuery
validates that a callback belongs to the loaded executable image. Zero means no
preferred label. Same-process comparisons always retain exact actual pointers;
foreign-module and heap addresses are never rebased to erase differences.
The production-core registration adapter checks `_tf_` after each local pass
(global then game) while comparing the live app snapshot after both passes.

At load and monthly, the summary now matches the house pattern exactly:
`lua shadow YYYY-MM-DD <event>: scope=registration ... mismatches=N`.
This task explicitly overrides the older §8.6 `lua-registry` summary label.
Per-entry records retain `lua-registry record` for bounded evidence. The startup
line, summaries and README explain the scope; counting `lua` supplies **no
full-VM Lua verification credit**. No tools/ file, regress profile, generated
ABI entry, public C ABI export or version was changed.

## Initial gate

- Implementation commit: `c9a62c9b1`. Six focused capi tests passed, including
  collector order, missing bindings and exact pointer identity without rebasing.
- Owned Rust qbuild fmt and git diff --check passed. The first fmt invocation
  rejected a relative filename; the absolute owned-file invocation found a
  formatting diff, applied with apply_patch, then passed. No compile/test failure.
- One final-source `qbuild all` passed: **6890 passed, 0 failed, 0 ignored**
  across workspace test-result lines; clippy **0 diagnostics**; native fc4verify
  build **233/233** edges (115.1 s), full CTest **72/72** (146.2 s),
  fc4verify **PASS** (265.1 s), qbuild **PASS**, exit 0.
- Native request `qbuild-20261024-030121-build-ctest` collected to terminal
  completion without duplication. Logs preserved as
  `build-runs/initial-qbuild-{test,clippy}.log` and
  `build-runs/initial-fc4verify-{build,ctest}.log`. The tree-change note from
  the verifier was caused by documentation commits/README clarification; runtime
  source stayed at c9a62c9b1 throughout the gate and game run.

## Initial real-city result

[Run 382](../runs/lua-replay6-c9a62c9b1-turbo-2032-20261024a.md), unique label
`lua-replay6-c9a62c9b1-turbo-2032-20261024a`: Konradshohe, turbo,
`lua_shadow=1`, target 2032-01-01, actual verdict **2032-01-21**, **clean**,
`ref_warnings=0`; foreign exceptions, watchdog stalls, rtc and overruns all 0.
City time 29.969 s; queue wait 7 s; run wall time 40 s. DLL SHA256:
`2D4B35923B7B650C9DBE1D3DC3111EAC6FCE645C40759C26FAA74EED48F1A758`.

Exact harness-recognized result: **lua 73/73**. Independently counted 73 dated
samples, load 2026-01-27 through month 2032-01-01, with every collector, binding,
scratch, stack and capture difference count zero. Each has 47 entries: **3431**
per-entry observations. Scratch is 0x0040C890; window_graph actual and preferred
callback labels are both 0x0040C890 in the measured process. A single file-event
wait accepted the unique completed verdict; no qwait, resend or game launch.
No turbo differences occurred, so no normal-speed rerun was required.

## Merged-source final validation

The requested pre-queue main merge introduced traffic-engine changes, so the
full gate and game run were repeated on the changed build inputs. Merged runtime
source **f36681776** passed `qbuild all`: **6891 passed, 0 failed, 0 ignored**,
clippy **0 diagnostics**, native build **203/203** (127.1 s), full CTest
**72/72** (185.3 s), fc4verify **PASS** (314.5 s), qbuild **PASS**, exit 0.
Native request `qbuild-20261024-031454-build-ctest` was collected to completion;
no tree-change note. Logs use ordinary `build-runs/qbuild-{test,clippy}.log`
and `build-runs/fc4verify-{configure,build,ctest}.log` names. The first passing
measurement above is preserved separately; no duplicate progress checks ran.

[Run 383](../runs/lua-replay6-f36681776-turbo-2032-20261024b.md), unique label
`lua-replay6-f36681776-turbo-2032-20261024b`: Konradshohe turbo to 2032,
actual verdict **2032-01-02**, **clean**, **ref_warnings=0**; foreign exceptions,
watchdog stalls, rtc, overruns, panics, structured exceptions, incomplete lines
and other warnings all 0. City time **62.859 s**, queue wait **6 s**, wall **77 s**.
Final DLL SHA256:
`3D64BD56E6A000EA16B2AFDED6DF6CCD5242E4EFAD43F81C5AAD0A54DDC516B4`.

Final exact result: **lua 73/73**, both harness-recognized and independently
counted. Load sample 2026-01-27 through month 2032-01-01, **3431** per-entry
observations, all 47 entries in every sample and all six difference/mismatch
fields zero. Scratch 0x0040C890 throughout. The unique completed verdict was
collected by the same file-event wait, without qwait or resending. No differences
required a normal-speed rerun. Both game labels remain valid separate evidence.
Latest main engine inputs were checked against the merged source before enqueue;
subsequent main changes at that check were documentation only.

## Questions and next steps

No new questions; LH6/LH7 are answered. No nondeterminism fields were excluded.
Closure allocation/upvalues, callback bodies, scripts, historical chunks/calls,
other owners and unsampled VMs remain outside the bounded comparison, never
counted as matches. A complete replay would require a separately justified
capture and side-effect/identity contract. No native/game requests outstanding.
Coordinator: land this bounded change and retain that scope in coverage reports.
