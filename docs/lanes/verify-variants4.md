| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-variants4 (bounded native variants) | `verify-variants4` / detached engine lane | in progress | Confirm SDK ownership/ABI; implement independent comparisons; full gate and guarded game controls. |

## Handoff
- Goal: independent native cRZVariant verification from foundations sections 9, 10, 17.1; no whole-owner claim.
- Worktree reused at reserved detached HEAD; initial status clean and no main-relative commits.
- Baseline qbuild all running (exec session 34312).
- q.py cannot start Bash here (E_ACCESSDENIED); use capped PowerShell and allowed lane wrappers.
- ABI/ownership route confirmed by call 15: GetResource owns exemplar; GetProperty/GetPropertyValue borrowed (foundations 10.2); corrected cFC4IGZVariant slot 7 CreateCopy returns isolated owned temporary, recorded ACQUIRED then RELEASE_CREATED. Never mutate/release borrowed property variant.
- Corrected compiled slots 3-8, getter/setter pairs and copying array setters 92-123 match foundations 17.1; no SDK virtual calls on variants. Validate supported compat image and native vtable/function module before cloning and mutation.
- Implemented bounded literal shadow: 14 numeric/character types, two scalars and 0/1/3 arrays, clone isolation, CopyFrom, Erase; opt-in variant_shadow via existing formats city-load hook.
- qbuild quick PASS: no Rust crates touched, build 5/5 edges. Baseline full gate still running ctest (session 34312); build PASS 208/208.
- HG4/HG5 filed in docs/specs/questions/foundations.md: native segment serialization acquisition and unsupported conversion/string/object/pointer lifetimes. No native serialization claim.
- Baseline PASS cargo 4947, clippy 0, ctest 68/68 (tree changed). Stable gate found return-channel review violations for Erase/CopyFrom/CreateCopy; corrected by discarding nominal bools and judging output/state, HG6 filed.
- First original vv4-original-a567f1-1820 clean2032/ref0/save ok=1; cases=0/refusal=1 because owned clone uses a different native COM vtable. Source exact identity retained; clone now validates all 127 published slots in the identified native image. No clone address hardcoded.
- Immutable first DLL SHA256 52E213A0FC81C1E2E4E6215068B5EB528EE30698B075E0F1B16F47B7B462E175, build-runs/dlls/vv4-a567f1/FreeCity4.dll; FC4_CHECKED OFF (support-image/interface guard, not RTC build).
- Second original vv4-original2-7c5ca4-1810 clean2032/ref0/save ok=1: native cases70 checks420 exact392 failures28 refusals0 clones71. All 28 failures scalar shape; HG7 filed. Typed values/arrays/clone isolation/CopyFrom/Erase pass.
- Added per-type totals, scalar metadata and classified HG7/unexplained counters without changing expected semantics. Final normal-speed original uses last original slot (3/3), then current40 control.
- Stable final qbuild all PASS at e7817958: cargo4947/0, clippy0 diagnostics, ctest68/68; no tree-changed warning (session43582 complete).
- Normal original vv4-normal3-e78179-1815 clean2032-01-01/ref0/save ok=1: 70/420/392exact/28HG7/0unexplained/0refusals/71clones. Native scalar count=0 for all fourteen tested types vs Rust1. Not turbo-only. Original cap3/3 exhausted; no further original runs.
- Final pinned DLL read-only build-runs/dlls/vv4-e78179/FreeCity4.dll SHA256 A7BAE0DBBAF83CA32C22B836704F4D050C1D1C2615E23235E29C0453382E0EFF.
- OUTSTANDING: vv4-current40-e78179-1819 (wait session88753), current40 turbo2032 plus same save script and native shadow; at most this one request outstanding.
- Next: read its existing verdict/log without resubmission; verify hash; report all four run labels, bounded exclusions, HG4-HG7; final report commit and merge queue. Native serialization and edited-save reload unsupported by this ephemeral SDK clone fixture (HG4).
- No game requests outstanding; original cap 3, unique labels required, wait without qwait.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-variants4-r1-manifest.json.
