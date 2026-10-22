| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-citydetail-reload4: CD7 persistence validation | `verify-citydetail-reload4` / engine lane, detached HEAD | in progress; final logical round 3/3; CD6/CD8 retained; no engine/ABI changes | 1. Locate gated immutable DLL and supported save fixtures. 2. Measure both save/reload directions or precise persistence blocker. 3. Report and park. |

## Handoff

- Goal: narrowed CD7 native-save/replacement-reload and replacement-save/native-reload validation; no CD8 reduction work.
- Start: HEAD 63b0c2bd; previous round stopped at CD8 boundary without runtime checks.
- Done: reused reserved detached worktree; working tree initially clean; no refs/config changed.
- Next: inspect persistence spec, harness fixture/save contract, prior immutable DLL/gate evidence and supported fixtures.
- Reuse already gated immutable DLL if citydetail code unchanged; task explicitly forbids blind baseline rebuild.
- Preserve actual DBPF SHA256, 8-byte records, original/reference controls, shutdown/ref0; no Documents writes.
- No new request outstanding. New game labels must be unique; wait without qwait.
- q.py fails with Bash E_ACCESSDENIED; use capped native PowerShell for read-only shell work.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-citydetail-reload4-r2-manifest.json.
- No subagents, RE, tools scripts, ABI/default/map edits, merge/push/restart, or fourth round.

## Prior round (superseded scope; evidence retained)

Stopped at the task's initial boundary audit, within 15 tool calls. No plugin change was justified by a fully published safe native reduction capture boundary. The lane preserves the existing implementation and makes no new coverage claim.

## Evidence and scope

- `docs/specs/city-detail.md` §3.1 initializes detail last; §3.2 removes through the live flora/prop managers; §3.4 additionally redevelops all network lots. §6 exposes no input-provider injection or city-owned detail-manager accessor. The public `cISC4City.h` interface supplies no detail-manager accessor either.
- `engine/plugin/FC4Observer.cpp` invokes `CityDetailShadowCityLoaded` at `PostCityInit`. This is after the authoritative city-owned reduction, so it cannot capture that pass's original inputs there.
- `engine/plugin/FC4CityDetailShadow.cpp` obtains a fresh original through `GetClassObject` and QI, releases its references, and compares four constructed cases: fresh Init, same-level Read/Init, none-level Read/Init, and version-1 Read refusal. Only the first three compare levels/serialized records. These are serialization/no-pass comparisons, not natural city-owned reduction events.
- The same shadow's `DryRun` creates only Rust with a read-only live host. Its reduction/increase counts are explicitly not compared with native. Repeating a native reduce after city Init would mutate the current city again, including network redevelopment; it would not establish the required comparison over the natural pass's captured inputs.
- `CityDetailContext::pool` retains references for Rust's host call; it does not publish an original-native interception or replay boundary. CD8 asks for the supported capture point, ownership/lifetime across destruction, and complete native event accounting.
- `docs/harness.md` documents read-only, SHA256-recorded `fixture_city` staging from main `build-runs/fixtures`. This resolves CD7's transport limitation; it supplies no reduction capture point. CD7's question text was preserved; no answer or spec rewrite was made by this engine lane.

## Validation and limitations

No builds or game requests were issued: the task explicitly requires stopping if the boundary is absent and forbids a code-free baseline build. No compared, different, refused, fallback or unaccounted runtime events were measured in this round; there is no algorithm-exactness denominator. No new immutable DBPF hashes, 8-byte save records, 2032 clean/ref0 verdicts, reload directions, reduce-owner behavior or shutdown measurements are claimed. No source fixture or game file was touched. No C ABI export changed.

Existing shadow capability is limited to constructed serialization/no-pass comparisons plus unpaired Rust dry runs; it does not establish reduction algorithm exactness or eligible owner/domain coverage. CD6 increase timing remains open and was not bypassed with preference/config writes.

## Question and next step

Appended **CD8** to `docs/specs/questions/city-detail.md` (next free ID in `QUESTIONS.md`). Question index generation was not run because scripts in `tools/` are forbidden and this lane may edit only the per-spec question file under `docs/specs/`. Publish CD8's boundary before implementing a native/Rust reduction comparison. Both requested save/reload directions remain unmeasured in this stopped round.

Operational note: the first q.py read attempt failed with Bash `E_ACCESSDENIED`; subsequent reads used capped PowerShell. Detached HEAD was retained; no shared refs, config or hooks changed.
