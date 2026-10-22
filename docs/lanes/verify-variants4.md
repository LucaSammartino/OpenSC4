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
- Next: finish baseline; full gate; immutable DLL guarded 2032 original/current40 requests and save/reload where supported.
- No game requests outstanding; original cap 3, unique labels required, wait without qwait.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-variants4-r1-manifest.json.
