| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-video-preferences4 (bounded portable panel owner) | Codex / core-video-preferences4 | in progress | Implement published readback, qualified actions and lifetime; verify portable fixtures and full gate |

## Handoff
- Goal: independent Rust video-preferences owner, spec sections 2-7 only; no ABI/plugin/native integration.
- Base HEAD: 8e95854da8454d89131af5b7f1735f02c7b0cae2; publication commit 48a401f2 is present.
- Read rules, lane format, video-preferences sections 1-8, VP1-5 and question filing rules.
- Native VP-T1..24 remain PROPOSED / UNEXECUTED; portable fixtures will be independently transcribed.
- Implemented independent literal fixture data before production comparisons; raw P bytes/dwords, independent u16 baselines, C/guard/first-redraw state, ordered readback and raw primitive calls.
- Implemented qualified actions, messages, EndDialog, modal chronology, shared slots, explicit redraw/show slices. Full redraw, valid-P Auto/redraw and unknown event branches reject before unqualified work.
- Refcounts: EndDialog/QI emit the published virtual AddRef/Release effects; separate direct Release accepts explicit raw count snapshot because AddRef arithmetic/initial count are VP3 unknowns.
- First-redraw slice transfers retained interface at 0079020d; no fabricated final Release/whole-method success.
- Code/tests milestone: 1244c283 (all Rust sources formatted in place with absolute filenames).
- Focused quick Rust PASS: 3429 + 21 + 13 + 19 + 0 = 3482 passed, zero failed; dependent compile Finished. Evidence: build-runs/fc4verify-quick-test.log and fc4verify-quick-deps.log.
- Outstanding SAME quick operation: exec session 82127, launched qbuild.py quick; native CMake has 217 steps, observed progressing. Do not cancel/restart/duplicate.
- Next: collect terminal quick result from session 82127; fmt-check absolute owned .rs paths; run ONE canonical qbuild all with engine source tree hash before/after. No all run yet.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded PowerShell output and approved qbuild wrapper.
- Task explicitly says no pre-edit baseline; run one canonical all after focused checks.
- No game requests; no native owner launch or complete redraw/default claims.
