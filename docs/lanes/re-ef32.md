| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ef32 — EF28 startup lifecycle (kind docs) | Codex; worktree re-ef32; detached HEAD, reserved branch re-ef32; commits 470bb27b5, 99026bbdb | ready; doccheck check + links 2/2 PASS; diff whitespace PASS; cargo/clippy/ctest n/a; no new game runs; shadow n/a; EF28 answered/narrowed; ABI/seams unchanged | 1. Land docs. 2. Collect the paired runtime trace in effects §18.11.3 before attributing the startup failure. |

- Imported only the EF28 question file from replace-effectsres30; main's existing entries survived. No engine files were read or changed.
- effects.md §18.11 specifies separate city/region manager ownership, resource registration and EF28-C1–C3 worked cases.
- audio-ambience.md §2.5 specifies the live-view prerequisite: PostCityInit enrolls ticks; synchronous PreCityShutdown disables/removes them before screen destruction.
- view3d-window.md §2.1 specifies application screen ownership and publication/withdrawal of the primary view interface.
- The region screen requests its manager at 007AD129, calls Init at 007AD130 and publishes 00B43D1C at 007AD13D; this does not populate city-view global 00B43DC4.
- Resource advertisement records a creator and enumerates (CLSID, 0, context); it supplies no hidden audio/view setup before GetClassObject.
- EF28 moved to docs/specs/questions/answered/effects.md with the ANSWERED marker; tools/questions.py index ran successfully (generated index unchanged).
- Remaining execution attribution needs manager caller stacks/lifetimes, every write to 00B43DC4, nested city-message delivery, tick-list calls and screen/city identities in failing and passing runs.
- Existing evidence stays distinct: house-on1 corrupt at 004E2C06; unchanged-DLL house-on2 clean. No new game run, replacement acceptance or shadow credit is claimed.
- Worked cases are static predictions, not executed engine tests. doccheck check and links passed; git diff --check passed.
- Sources: original export/decomp only. Failed cSC4App/cWinSC4App pseudocode bodies were resolved from exported instruction bytes; REA was not used.
- No C ABI exports or docs/abi rows changed; no ABI proposal file is needed.
