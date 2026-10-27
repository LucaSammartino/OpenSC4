| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-cfgrules30 graphics rules; kind engine | Codex / .claude/worktrees/replace-cfgrules30 / detached HEAD; reserved branch replace-cfgrules30; code f84ae8321 + 37219ef48 | wip; (a),(b),(d) pass; (c) held on first-null-writer observation; ABI unchanged; Rust 8205, clippy 0, CTest 142/142; shadow 13/13 exact; GR12 filed | 1. RE observer supplies graphics-rules §2.7 first-null-writer history. 2. Resolve (c), then enqueue engine and flip manager credit. |

- Merged main first (6c5859c2b); cfgrules28 was already an ancestor. Detached HEAD retained; no shared refs changed.
- GR11: retain/replace properties before gathering; same-object Init regathers/reparses without extra AddRef. Added its repeated-Init worked case and hardware double-probe checks.
- GR11: acquire window manager A417445E/000005A4; borrow root/child/renderer, balance counted GetChildAs/QI outputs, use the uninitialized facade only when no view is found. No extra restore or view lifecycle call.
- Added default-off cfgrules_lifecycle snapshots/acquisition-release logs and ABI-only constructor/pre-city/Shutdown entry/exit traces with thread/caller stacks. New native fixture stub fixes the initial quick-build linker failure; that failed/changed-tree run is not the final gate.
- (d) PASS on 37219ef48 / DLL SHA256 7234fd3c4e3347afde74162e8c0b655ee1db17dd2a8921124ae3aad952fd163d: qbuild all Rust 8205/0/0, clippy 0 diagnostics, native build 234/234, CTest 142/142; owned fmt 2 files/0 diffs. WT logs: qbuild-test.log, qbuild-clippy.log, fc4verify-build.log, fc4verify-ctest.log under build-runs/.
- All game logs below are main build-runs/game-results/<label>.log; prefix cfgrules30-r2-. Every run has ref_warnings=0; all 11 requests are terminal, none outstanding.
- Query parity PASS: queries-off1/on1 both clean to 2026-04-10/11, final renderer properties 77/77 and recommendations 12/12 exact; Init status=0.
- (b) owner shadow PASS: queries-off1, 13/13 exact SetOptions/Apply snapshots, each 77 properties plus 12 recommendations; native property writes are not forwarded by the shadow.
- (b) cross-setting saves PASS 2/2: load-off-with-on1 clean to 2026-06-06; load-on-with-off1 clean to 2026-06-14; prepared hashes match query exports (f9cc693d / 348356a4).
- (a) PASS: house-on1, house list plus cfgrules, clean through 2032-01-01, 165.375 s, foreign_exceptions=0.
- (c) current s0 replay: s0-on1/on2/off1 reach 2028-02-02/02-01/01-30; all verdicts corrupt solely at quit-time +0x1D0F94, occupant type 096E763C Shutdown (lines 17681/17756/17792). This shared original fault is the task's exemption, not the former null-view fault.
- (c) s1: s1-on1 clean to 2028-01-28; s1-off1/off2 both stalled at 2026-01-30 (watchdog_stalls=1, foreign_exceptions=0). These failed off controls remain distinct from the passing on run.
- §2.7 history: s0-on1:363–419 has no public view before/after gather cleanup; sole construction at 8000–8005, pre-city publication path at 8032–8037, all after Init. No view Shutdown during acquisition; identity/thread/stacks and counted-reference cleanup are logged.
- (c) HELD: earlier +0xE2C06 did not recur; its first null writer/missing publication and matching native acquisition history remain unidentified. Public lifecycle history does not replace §2.7's required RE-side observer/watchpoint. No such debugger tool is configured here.
- GR12 filed in docs/specs/questions/graphics-rules.md for that residual observation; retired this branch's duplicate open GR11 entry now answered by §2.7. No behavior spec or private renderer/view layout was changed.
- cfgrules is NOT ready for the replace list. Pending credit flip: cSC4ConfigRuleManager (15,096 B) and its rule records. WIP queue only; no ready enqueue, coverage-map edit or replace-list edit.
- Next: obtain GR12 runtime evidence using the original failing source/DLL and matched settings, settle (c), repeat only affected controls after any fix, then enqueue the published actual tip as kind engine.
