| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dispatch-complete4 (dispatch replacement completion) | `Codex` / `glue-dispatch-complete4` (engine lane) | in progress | Complete published dispatch host contracts and measure native/replacement exercise |

## Handoff
- Goal: finish supported dispatch replacement, including vehicle views and independent source/pylon comparisons; no completion credit with missing contracts.
- Reused reserved detached worktree; initial HEAD equals main, clean status.
- Baseline qbuild all: cargo workspace and clippy logs clean; full ctest 67/67.
- q.py cannot start Bash (E_ACCESSDENIED); use bounded native PowerShell commands.
- Implemented independent ground/air source registration and slot 15/21 comparisons; real station callback releases; automaton despawn/go-home/group callbacks; active-record shadow identity/ref fixes.
- Added script dispatch type: real station, BeginTool/Dispatch/live record/shadow/Revoke/EndTool; not success unless a vehicle actually exists.
- Milestone 1a2fb6bb: independent sources/stations and scripted live dispatch; original a1 created fire vehicle (ID2, call1, pylon1, 79-byte exact record); unsaved handle differs under DP10.
- Original a1 corrupt/ref0 on air list release. PR8 ownership applies to ground only; conservative air no-release choice and missing contract filed DP13. No replacement credit.
- Fixed core destruction releasing captured references (including Read-before-Init pylons); shadow drops each sample before native cleanup. State getters now mutex-backed (no RefCell), token atomic; added drop ownership regression test.
- Interim all: cargo4238/0, clippy clean, ctest65/67; return-channel citations and script error-string expectation repaired. Quick cargo2372/0 PASS, build7/7 PASS; tree changed during this interim run. Final all next on committed tree.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-dispatch-complete4-r1-manifest.json.
- No game outstanding. Next run original gdc4-original-20261022-a2 to 2032 after final build, then replacement gdc4-replace-20261022-a1; never concurrently.
- a2 also corrupt/ref0; actual bug: SDK SC4List destructor releases elements, so explicit cleanup doubled ground releases. Nodes now cleared after audited cleanup; prior replacement already balanced ground references. DP13 has appended correction (failure does not establish air ownership).
- Gate at 8a99762a: cargo4239/0, clippy clean, ctest67/67. Revalidate after node cleanup; next original label gdc4-original-20261022-a3, replacement gdc4-replace-20261022-a1.
- DP10-DP13 in docs/specs/questions/dispatch.md: missing view, pylon, BeginDispatch and air-list ownership contracts. Completion blocked; no ABI export changes.
