| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-materials4: portable textures and materials | Codex / `core-textures-materials4` (engine lane) | in progress; reserved detached worktree reused; main baseline already gated | 1. Audit published owners and existing primitives. 2. Implement and test. 3. Full gate and report. |

## Handoff
- Goal: implement published portable behavior of cS3DTextureBindingFactory, cS3DTextureBinding, cS3DMaterialManager, cS3DMaterial, core only.
- Done: 4336efc6 implements factory/binding algorithms, published material state and manager methods; S3D view adapter reuses existing parser decisions and one flag-normalization primitive.
- Audit: factory primary slots 0-18 and published helpers/lifecycle covered; binding constructor/AddRef/Release covered; material constructor state covered; manager slots 0-5/deleting destruction covered, slots 6/7 unpublished. No whole-family completion credit.
- Gate: first qbuild quick PASS, 2603 tests / 0 failed; capi/conformance compile-only and native build 206/206 (303.1s). Wrapper noted tree changed because formatting/docs were committed during run; final gate will use stable sources.
- Tests: first 24 core cases passed; added six lifecycle/upload/failure cases and two parsed-S3D integration cases after quick completed.
- Formatting: six implementation files pass qbuild fmt with absolute paths. Shared lib.rs has pre-existing ordering/layout drift (125 diff lines); do not mass-reformat it.
- Next: format new tests, commit this checkpoint, then stable final full qbuild all and report.
- Questions filed: TB4 (missing method inventory), TB5 (numeric domain), TB6 (instance zero), TB7 (failure/upload boundaries). Question index regeneration belongs to coordinator (tools/ prohibited).
- No game requests (portable core only); no shadow/replacement claim.
- No CAPI/plugin/ABI/map edits; no complete-owner credit until body inventory is reconciled.
