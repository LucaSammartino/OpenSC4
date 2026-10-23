| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-missing-plugin-reporter4 (portable missing-plugin reporter: DoMessage + DoPostCityInit) | `core-missing-plugin-reporter4` / (engine lane) | ready for the train (gate: cargo 5743/0, clippy clean, ctest 68/68); `fc4-core::missing_plugin_reporter`, not wired into plugin/capi | Next: answer MPR1-MPR4; wire into a host only once an owner/subscription contract exists. Question MPR4 filed |

## Report
- `engine/core/src/missing_plugin_reporter.rs`: portable implementation of the reporter's two bodies from
  `docs/specs/missing-plugin-reporter.md`, over a `PrimitiveHost` trait for every foreign call.
- `do_message`: full-word selector compare with `0xEA8AE29A`, post-init on match, always AL 1.
- `post_init`: fresh u32 set (hint 100), context-to-filter word copy (both loads before stores), null-manager
  set-only cleanup with AL 0, weak allocator cache with lookup/store/temporary Release/reload at every
  acquisition site, sentinel list + manager query (AL ignored), per-occupant property adapter, AL-gated insert,
  one Release per occupant, count and popup-AL gates, 8-byte dialog init/AddRef/DoDialog/AL save/Release,
  then node frees (next read before free), sentinel free and set cleanup.
- Tests (`missing_plugin_reporter/tests.rs`, 17): spec traces MPR-T01..T15 plus two cache/view cases.
- Gate: cargo 5743 passed / 0 failed (sum of all `test result:` lines), clippy -D warnings clean,
  CMake build 217/217, ctest 68/68. fmt clean on the lane's .rs files.
- Not done on purpose: T14 (null allocator) and T15 (null dialog allocation) stop with an explicit
  `Unsupported` value instead of guessing a native result; the same goes for invalid holder/variant views,
  success without an output write and count overflow (MPR4). No owner construction, subscription, QI
  layout or plugin/capi wiring. No ABI change.
