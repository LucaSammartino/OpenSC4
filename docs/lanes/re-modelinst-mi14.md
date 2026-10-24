| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-modelinst-mi14 (model-instance observation contracts) | Codex / `re-modelinst-mi14` | ready; MI14 answered; doccheck and questions check PASS; six prose conformance cases; no ABI change or game run | Correct engine fixture constructor binding and rerun; implement synchronous observation with explicit coverage limits |

## Report

MI14 is answered in model-instances §§1.3, 2.2, 3.2, 4.2, 6.3 and 9.2, with prose conformance cases
21–26. Commit `18751dbb0` establishes the getter/fixture/animation/processor bindings; this final milestone
adds sprite layouts, discovery limits, observation lifetime rules, the archive entry and regenerated index.

The constructor assumption in the engine lane is incorrect. Original constructor `0x00802950` takes a
stack construction flag and removes four argument bytes. The standalone factory passes 1. Only destructor
body `0x00802620` is receiver-only; it does not pool-free foreign outer storage. A prior receiver-only
constructor fixture is not validated by these findings and needs correction and a fresh run.

Draw attributes require a caller-owned 20-byte destination and return that destination pointer. Bounds
getters may mutate the local cache. Animation receiver offsets and scalar getters, processor retained
container access, and the distinct inline-animation versus separately allocated table layouts are now
specified with instruction addresses. Numeric layouts/widths were checked against assembly bytes; the
standalone virtual-base displacement and interface vtables were also checked in the original PE data.

GetModelsInVolume is a false/no-output stub. Ordinary network GetModelInstances returns one counted
interface without a capacity check and updates its transform. Derived network capacity is not inferred
from that body. The specified read-only route observes existing game calls synchronously and copies
values during their valid lifetime; its finite cap is an instrumentation choice. It does not promise a
monthly global sweep, discovery of every owner, or whole-owner coverage.

MI14 was not present in this reserved checkout or main. Its complete question was obtained from the
explicitly named glue-modelinst-shadow11 worktree and archived directly, without adding an open copy.
That lane's current report has a per-owner comparison boundary rather than an Owner closure/open: table;
the available boundary was read first. At integration, remove any stale open MI14 brought by that lane.

Validation: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS;
`git diff --check` PASS. `questions.py find MI14` resolves the answered archive. Logs are
`build-runs/mi14-doccheck.log` and `build-runs/mi14-questions-check.log`. These are documentation gates;
the six new cases are specifications, not executed engine tests. Cargo/clippy/CTest and game: n/a.
No engine files were read or edited, no public ABI exports changed, and no ABI-row proposals were needed.
REA was not used; export queries supplied the necessary instructions. q.py's Bash wrapper failed with
Win32 error 5; native PowerShell was used afterward.

## Handoff
- Goal complete: MI14 native observation/binding contract and six conformance cases published.
- Done: binding milestone `18751dbb0`; final archive/index/report milestone is this commit.
- Next: coordinator integrates docs; engine corrects constructor to flag 1 and reruns fixture.
- Keep live shadow scope limited to captured operations with independent baselines.
- No game requests outstanding; no ABI proposals. Remove stale engine-lane open MI14 when merging it.
