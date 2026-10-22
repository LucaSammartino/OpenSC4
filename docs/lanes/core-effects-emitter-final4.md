| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-emitter-final4: EF14 emitter corrections | Codex / `core-effects-emitter-final4` (engine lane, detached HEAD) | ready; cargo 4224 passed, clippy 0 diagnostics, ctest 67/67; 26 focused particle tests; ABI unchanged; game n/a; questions none | Coordinator review/integration; no further family round requested. |

## Changes

Implemented the newly published effects.md §7.1.1 EF14(a-e) contract in the pure-core emitter model.

- Plain emission advances phase without wrapping, then tops up to `ceil(curve[0])`. It applies neither detail/density nor C0 scaling, changes no loop/carry/countdown state, and skips newborn slot-20 aging and birth hooks. Documented safe choices remain: empty curve means zero; target at/below the live count creates nothing rather than underflowing an unsigned loop.
- Particle init draws life from description fields D+0C/D+10 using the supplied global RzRandom, even for degenerate ranges and failed slot-20 inits. Particles store float age and life; slot 20 first accumulates float age and survives only when age is less than life. Aging precedes emission. Plain newborns always enter alive at age zero.
- C0 bits retain lowest-set-bit precedence. SetDescription resets density scale +1E0 and suppression. Only rho sees the local float period floor; burst ages and gaps retain the raw description period.
- Curve interpolation returns exact entries at integral positions without reading/interpolating a neighbor. The documented safe clamp covers native out-of-bounds fractions; empty curves remain zero.
- The conformance adapter now uses the constructor's life upper bound by default, because D+10 is also the burst-age bound. A new plain-target fixture distinguishes top-up from rate emission. Its existing seven-update delayed-stop fixture remains valid.

Rust-only interface changes: EmitterDesc replaces the artificial fixed `life` with `life_min` and the existing D+10 `max_age`; the constructor initializes a degenerate range. `live()` returns a computed vector of diagnostic remaining lives instead of a borrowed slice of stored countdowns. No plugin, CAPI, tools, ABI exports or ABI version changed. No original-owner outputs are accepted as host inputs.

## Measured verification

- Initial `qbuild all`: cargo 4213 passed; clippy 0 diagnostics; CMake 206/206 edges; ctest 67/67. This initial invocation reported a tree change during the run, so it is not the final verification.
- First quick failed on an incorrectly changed fixture expectation (observed 7, expected 6). The expectation was restored and a separate discriminating fixture added.
- Final quick: cargo core/conformance 3547 passed; capi/formats dependent compile passed; CMake 4/4 edges. Commit activity during that run caused its tree-change annotation.
- Final `qbuild all` on clean committed HEAD `0e7abf36`: cargo workspace 4224 passed / 0 failed / 0 ignored; clippy exit 0 / 0 diagnostics; CMake 2/2 edges; full ctest 67/67, 0 failed. fc4verify PASS in 90.3 s, with no tree-change annotation.
- `qbuild fmt` passed for all three touched Rust files; `git diff --check` passed.
- Eleven new deterministic tests bring the focused particle suite to 26 tests. They cover fixed-target replenishment and aging order; skipped birth aging (including zero/NaN life); unchanged plain state and safe underflow; life draws after start-delay and before gap RNG, including rejected particles; oldest-first rate births; stored-age rounding and exact death boundary; C0 precedence and density reset; float period floor/raw zero-period gaps; exact curve endpoints and safe short-period clamping; suppression and delayed stop with no extra life draws.

Logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`, `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`; quick detail in `build-runs/fc4verify-quick-test.log` and `build-runs/fc4verify-quick-deps.log`.

## Scope, questions and next steps

No new questions: the published EF14 answer resolves the implementation choices used here. No game requests or native shadow captures were made for this pure-core change; native ref warnings and shadow exactness are unmeasured. This is not full effects-family coverage or runtime replacement credit.

Remaining outside this model: motion/render integration, extra-physics terrain/water kills, non-life initialization draws and the other emitters' birth hooks. Those require their own model/runtime work and are parked; no fourth family round is requested. Within the requested emitter/life scope, nothing remains open.

Milestones: `c3833c3a` implementation and initial focused tests; `574acda9` conformance reconciliation, remaining focused tests and formatting; `0e7abf36` quick-gate record and clean full-gate input. The final report commit changes documentation only. Coordinator review/integration is next; detached commits are published by the supervisor, not this lane.
