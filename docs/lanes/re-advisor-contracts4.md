| re-advisor-contracts4: answer advisors AD1-AD3 (window procs, private global consumers, map consumers) | re-advisor-contracts4 / .claude/worktrees/re-advisor-contracts4 | in progress | answer AD1-AD3 in advisors.md 12.1/12.2/14.2 |

## Handoff
- goal: answer AD1-AD3 (docs/specs/questions/advisors.md) from the exe; write rules into advisors.md 4.1/4.6/12.1/12.2/14.2.
- done: lane file 98aa2d43; evidence gathered (build-runs/scan207.txt, use207.txt, track2_all.txt): 66 abs refs to 0x00B207C0, all in family (advice/advisor classes, Lua ext, system ctor 0x41A9B5 / dtor 0x41A1D1 / base dtor 0x418A1E); slots 23-28 bodies read; map consumers: MySimAgentSimulator::Init 0x42EC30 iterates GetAdvisors; winproc GetMessageList 0x41E000 / get_advisor_mood 0x41DC20 / MySimWinProc::GetMessageList 0x76F260 copy GetCurrentAdvices (operator= 0x40EB00, copy 0x40F0C0).
- next: write answers into advisors.md 4.1 step 10, 4.6, 12.1, 12.2, 14.2; move AD1-AD3 to answered; questions.py index.
- facts: REA unavailable (no app open); used re_query + decomp + byte scan of bin/SimCity4.exe. Export xrefs miss 0x00B207C0 (0 rows); byte scan is the evidence.
