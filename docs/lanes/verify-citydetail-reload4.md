| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-citydetail-reload4: CD7 persistence validation | `verify-citydetail-reload4` / engine lane, detached HEAD | in progress; final logical round 3/3; CD6/CD8 retained; no engine/ABI changes | 1. Locate gated immutable DLL and supported save fixtures. 2. Measure both save/reload directions or precise persistence blocker. 3. Report and park. |

## Handoff

- Goal: narrowed CD7 native-save/replacement-reload and replacement-save/native-reload validation; no CD8 reduction work.
- Start: HEAD 63b0c2bd; previous round stopped at CD8 boundary without runtime checks.
- Done: reused reserved detached worktree; working tree initially clean; no refs/config changed.
- Done: immutable DLL copied from verify-simgrids-shadow4/dll-sgv4-r4, SHA256 ED12FAE2F0A13F6FDE1A03E4171CD29C7D5CB5EDF264A004C5C0294D17A1E1C1.
- Gate reused per explicit task: original qbuild logs re-read, cargo4940/0, clippy clean, ctest68/68; no baseline rebuild.
- Citydetail core/capi/plugin/shadow diff vs gated source7e9f580e is empty. No source files changed this round.
- Input fixture sgv4-r1-native39-20261022c.sc4 rehashed F03DA3420CD8FABE50BD887890310D2D5577E5C5C109F76E3906EF84E255FB7A; exact8-byte payload 0000000001000000.
- Next: wait existing local Python session49390 (build-runs/cd7_run.py); it queues sequential native39/replacement40/native39 runs through2033/2034/2035 and captures each changed DBPF while slot is live.
- Reuse already gated immutable DLL if citydetail code unchanged; task explicitly forbids blind baseline rebuild.
- Preserve actual DBPF SHA256, 8-byte records, original/reference controls, shutdown/ref0; no Documents writes.
- Native control cd7-r2-native-save-20261022a CLEAN2033-01-01/ref0; citydetail factory off, shadow four constructed cases mismatch0. Output captured in live slot2, hash AB2BF8E6D134FCA26167C9250FAF66A2FFF162A42E78758D49D66BEF309D00D1, exact8-byte v0/level1.
- Outstanding: cd7-r2-repl-reload-20261022a in slot1; script then queues cd7-r2-native-reload-20261022a. Never discard an existing verdict. No parallel requests.
- Own build-runs/cd7_record.py audits DBPF index/class framing and exact8-byte records, refuses compressed CD entry. cd7_run.py waits without qwait and preserves input/output hashes and capture metadata.
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
