| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-regionalcity-replace23 (regional-city shadow checks) | `Codex` / `glue-regionalcity-replace23` (engine) | ready for measured shadow changes only; cargo 7644/0, clippy 0 diagnostics, native build 76/76, ctest 93/93; rc23-r3-poi2032-20261025a 73/73 exact, clean through 2032, ref_warnings=0 | Implement the replacement adapter/key and direct-construction integration; add ownership/registration tests and both requested replacement runs. |

Code commit `5387ddf23` adds seeded point-list getter probes for present, empty and unknown types, comparing IEEE payload bits and reporting the first differing byte.
Each observation now captures the native record again and checks that getter calls preserved its serialized bytes. Shadows remain enabled only by `regionalcity_shadow = 1`.
Rust tests cover empty and absent point lists, signed zero, NaN payloads, mismatches, and sparse population maps without inserting saved entries. The sparse-map fixture explicitly clears the constructor's default commodities.

Focused tests: 4/4. Final `qbuild all`: cargo 7644 passed, 0 failed, 0 ignored; clippy 0 diagnostics; queued native verification `qbuild-20261025-061856-build-ctest` passed, build 76/76 edges and ctest 93/93.
Initial native verification failed only the new fixture's assumption that commodities started empty; full logs remain in `build-runs/rc23-r3-first-fc4verify-{build,ctest}.log`.
Game result: `rc23-r3-poi2032-20261025a 73/73`; clean at 2032-01-01, ref_warnings=0. The first and last observations each decoded 85 records and compared 15731 fields with mismatches=0.
The scripted full save at 2029-01-01 returned ok=1. Request, verdict and log are under `build-runs/game-queue` / `build-runs/game-results`; the runtime code was unchanged by the subsequent fixture correction.

Still missing: the C++ Rust-backed regional-city owner, `regionalcity` replacement registration, direct native construction integration (regional-city §1), ownership/registration tests, and the regionalcity-alone/full-house replacement runs. No native replacement is claimed.
No C ABI exports changed. No spec questions filed or tools files edited.
