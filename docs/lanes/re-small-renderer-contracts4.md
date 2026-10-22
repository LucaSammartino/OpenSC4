| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-small-renderer-contracts4: native renderer and path owner contracts | Codex / `re-small-renderer-contracts4` (RE lane, detached worktree) | wip: round 2 proofs committed; PA4/FF4 answered (12 cumulative); questions check and diff check passed; no ABI/game changes | 1. Reconcile TB4/GT4/FF2 inventory scopes. 2. Finish FF3 numeric stores; retain exact residuals. |

## Report

This round published the preserved flame evidence first, then added primary native instruction/data proof. All deliverables are spec, question and lane documentation; no engine, ABI-table, map or tool changes are committed.

### Commits

- `bea42a5f`: preserved flame defaults, visibility, replacement and Init/stage evidence; Init updates existing flames and does not create them.
- `a2e1f920`: material lifetime/copy/assignment and byte-string/locale path boundaries; PA4 answered.
- `b0bdd07a`: glyph-manager lifetime and inherited slots; flame index/selection, rotation, persistence, direct destruction and measured inventory; FF4 answered.

### Native results

- texture-bindings §3.1: unsigned 16-bit material release frees at **0 or 1**, otherwise decrements; releases stage 1 before stage 0. Assignment acquires before releasing; material value copy copies the count word. Manager slots 6/7 and teardown were already published in round 1. Named inventory is 55 bodies/4758B; four class/create helpers add 111B, yielding 59/4869 before unnamed/value/shared bodies. This does not reproduce 58/4883.
- glyph-textures §1.1.1: constructor is **005D8CA0**, not destructor 005D8900; initial references and sweep time are zero. Inherited slots and message thunks are specified. Direct destruction frees container/key storage without Shutdown or texture-info releases; absent glyph-pointer removal does nothing. Named inventory is 38 bodies/4047B (manager 25, glyph-info 8, string-info 5), not 43/4475; shared/unnamed slots are separately identified.
- fire-flames §§2.1/4.1–4.3: first primitive supplies count but its index offset is **ignored**; partial triples read physical trailing indices. Unordered area comparison selects a triangle; all ordered-less comparisons continue one past the vector. Local generator is cRZRandom(-1); unchecked zero-length normalization and some float stores are proved.
- Flame transform uses unsigned side × **90 radians**, x87 trig, stated float stores and rows (c,0,s),(0,1,0),(−s,0,c). Temporary counter/kind are **2/5**; embedded transform is unchanged. Box wall draw order is side, free horizontal coordinate, height. ViewUpdate sends old-position then new-position transforms.
- Fire persistence version is 3; read accepts exactly 3. Inline flame writing omits made/visible/effect and transform counters. Reads append, retain earlier additions on failure, and do not reclaim the failed fresh flame. Direct destruction frees vector storage without walking flame pointees; initialized Shutdown performs stop/release/free work. Owner inventory enumerates 21 bodies/3740B including the mislabeled flame constructor and unnamed fire destructor; triangle/point helpers are separate.

### Questions and limits

PA4 and FF4 moved, with DIFFERS rules, to their exact answered files. Ten prior answers were retained without re-answering them. No new question IDs were filed.

TB4, GT4 and FF2 remain open **only for the documented inventory-scope/inherited-wrapper reconciliation**, not for the newly proved lifetime rules. FF3 remains open for exact area accumulation and the full barycentric/placement intermediate-store sequence; its index/generator/error boundaries are published. No family-wide completion claim is made. FF1 and the other pre-existing questions were outside this round's scope.

Next evidence work: identify the original selection of bodies behind 58/4883, 43/4475 and 16/3513 rather than counting unrelated shared slots to force totals. For FF3 continue native 00470EC0's area call/accumulation and GetPointsOnModel 00472175–00472321, including extended-register values retained across float stores.

### Validation and artifacts

`python tools/questions.py index`, `python tools/questions.py check` and `git diff --check` passed. Cargo, clippy, ctest, game runs and shadow comparison: n/a, docs only. No C ABI exports changed; no ABI proposal file.

Read-only sources were the main checkout's export, selected decompilation ranges and executable data. REA was not needed or called. No network, external communication, subagents, branch/ref updates, merge or push. Commits succeeded despite Git's nonfatal packed-refs.lock permission diagnostic. Ignored evidence helpers remain in this worktree's build-runs/: renderer-inventory.py, inventory-*.tsv and native-data.py.
