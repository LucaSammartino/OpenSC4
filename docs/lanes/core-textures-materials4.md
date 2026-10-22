| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-materials4: portable textures and materials | Codex / `core-textures-materials4` (engine lane) | in progress; reserved detached worktree reused; main baseline already gated | 1. Audit published owners and existing primitives. 2. Implement and test. 3. Full gate and report. |

## Handoff
- Goal: implement published portable behavior of cS3DTextureBindingFactory, cS3DTextureBinding, cS3DMaterialManager, cS3DMaterial, core only.
- Done: 4336efc6 implements factory/binding algorithms, published material state and manager methods; S3D view adapter reuses existing parser decisions and one flag-normalization primitive.
- Audit: factory primary slots 0-18 and published helpers/lifecycle covered; binding constructor/AddRef/Release covered; material constructor state covered; manager slots 0-5/deleting destruction covered, slots 6/7 unpublished. No whole-family completion credit.
- Tests: 25 focused cases added; first qbuild quick is still running (exec session 75417); no pre-edit full baseline per task override.
- Next: collect quick result, resolve meaningful failures, add format-to-owner integration coverage, run fmt check and stable final full gate.
- Questions filed: TB4 (missing method inventory), TB5 (numeric domain), TB6 (instance zero), TB7 (failure/upload boundaries). Question index regeneration belongs to coordinator (tools/ prohibited).
- No game requests (portable core only); no shadow/replacement claim.
- No CAPI/plugin/ABI/map edits; no complete-owner credit until body inventory is reconciled.
