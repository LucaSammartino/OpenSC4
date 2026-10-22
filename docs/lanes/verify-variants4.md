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
- Corrected full gate running session40271; tree changed for diagnostics, so final stable all still required. Next: commit diagnostics, quick, pin DLL, normal3 then current40; stable fullgate. No request outstanding.
- No game requests outstanding; original cap 3, unique labels required, wait without qwait.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-variants4-r1-manifest.json.
