| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-close5: camera control owner closure | Codex; core-camera-close5 | in progress | Audit existing core camera against camera spec, close gaps and run gate. |

## Handoff
- Goal: close cSC4CameraControl in existing engine/core/src/camera; no plugin/C ABI edits.
- Done: audited camera §§2,6,7; added Owner closure/conformance tables; filed CA8–CA10 after confirming next free CA8.
- Done: optional foreign CameraViewUtilities corner/update callbacks; tweak respects optional-view guard; literal AddRef ledger.
- Done: fixed false exhaustion-test claim; discriminating tie/exhaustion/subthreshold tests. Focused final source: 52 camera tests pass.
- Next: explicit-file fmt, commit this milestone, then run qbuild all once on stable source and collect terminal result.
- Evidence: final focused log build-runs/q/20261023-134500-37132.log; initial search fixture log 20261023-133719-39712.log.
- Evidence: fmt relative-file invocation failed path resolution; absolute-file invocation gave fixable formatting, now patched.
- Open: CA7 pre-init fields; CA8 command wrappers; CA9 generic dispatch; CA10 control COM contracts. Do not claim owner closure; final queue state wip.
- Game: none; no plugin/C ABI changes and no request outstanding.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-camera-close5-r1-manifest.json.
