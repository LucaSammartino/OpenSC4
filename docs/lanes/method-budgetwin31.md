| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| method-budgetwin31: SubBudgetWindow methods; kind engine | Codex / method-budgetwin31 / detached HEAD; reserved branch method-budgetwin31 | wip; code 45ec2150b, 764347911, e91f397a1; ABI unchanged; gate green; verified 0/73268 bytes; BW26 | 1. Resolve BW26 panel entry. 2. Resolve reached BW25 helper contracts. 3. Exercise rates/funding and complete output comparisons. 4. Enqueue kind engine after acceptance. |

Gate: qbuild all after the dispatch fix PASS: cargo 8255 passed, clippy 0 diagnostics, CTest 142/142; focused replay 2/2; fmt 2/2. Logs: WT/build-runs/qbuild-test.log, WT/build-runs/qbuild-clippy.log, WT/build-runs/fc4verify-ctest.log.
Games: method-budgetwin31-reach1 clean/ref_warnings=0 but UI dispatch stalled; method-budgetwin31-compare2 clean/ref_warnings=0/end=2026-03-02, 50 actions executed, 16 SKIPPED. Logs: MAIN/build-runs/game-results/<label>.log and <label>.verdict. Corrected census: MAIN/build-runs/suite/method-budgetwin31-compare2/play.reach.tsv.
Credit/missing: 48 mapped bodies / 73268 B; no new verified bytes. Restore group recorder compares ordered funding/ref calls and unchanged owner/saved-pair bytes; live calls=0, complete=0, mismatches=0, resyncs=0 (compare2.log:4954). Individual service recording remains absent. Slot 24 entered 0x00690330 once: budget-dialogs income/expense, not the requested panel. BW26 filed in docs/specs/questions/budget-window.md; reached unclassified helpers remain BW25.
Ledger: owner cSC4SubBudgetWindow; calls/complete/mismatches. Zero calls = no live panel/owner observation (BW26); >=64 = capped entries, unclassified support contract/output missing (BW25). '-' = no comparison, never exact credit. Addresses/bytes: WT/build-runs/coverage-start.tsv; entry counts: corrected census above. Groups remain sorted by bytes.

| Method / address | Bytes | Calls / complete / mismatches |
|---|---|---|
| updateNeighborBudget 0x00781C90<br>initNeighborBudget 0x0077E600<br>on_init 0x007876B0<br>UpdateValues 0x007898A0 | 14988<br>13933<br>8672<br>6265 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| initOrdinanceBudget 0x0077C660<br>updateIndividualBudget 0x00785740<br>on_button_activated 0x0078B120<br>updateOrdinanceBudget 0x0077ADA0 | 4468<br>2913<br>2789<br>2566 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| set_comboBox 0x0077DAB0<br>initIndividualBudget 0x00786690<br>DoWinMsg 0x0078BCA0<br>SetEmptyWindow 0x0077BEC0 | 2387<br>2248<br>1363<br>1205 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| add_image_button2 0x0077B960<br>add_comboBox 0x007798C0<br>constructor 0x007862B0<br>SetupWindow 0x0077A480 | 811<br>692<br>636<br>612 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| SetupWindowBig 0x0077A6F0<br>SetOccupantFundingLevel 0x0077BC90<br>add_text 0x00779660<br>SetupWindowIndiv 0x0077A960 | 612<br>556<br>493<br>468 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| add_image_button1 0x0077B7B0<br>CreateSubBudgetWindow 0x00779E50<br>DoIndividualBudgetWindow 0x00786530<br>add_Bitmap 0x0077A250 | 420<br>395<br>337<br>308 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| destructor cleanup 0x0077C4C0<br>on_shutdown 0x0077E440<br>support? 0x0077D9B0<br>restore append helper 0x0077D8C0 | 286<br>268<br>242<br>237 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| GetBudgetSegment 0x0077A390<br>ApplyDealEdits 0x0077AB40<br>support? 0x0077D7E0<br>DestroySubBudgetWindow 0x0077A1A0 | 234<br>220<br>211<br>166 | 0/0/-<br>0/0/-<br>>=64/0/-<br>0/0/- |
| support? 0x0077A080<br>RestoreStartSettings 0x0077C5E0<br>end_dialog 0x00779850<br>support? 0x0077C3C0 | 154<br>120<br>97<br>95 | 0/0/-<br>0/0/0<br>0/0/-<br>0/0/- |
| set_window_indiv_item 0x007795A0<br>get_window_indiv_item 0x00779600<br>support? 0x0077E5A0<br>support? 0x0077E550 | 92<br>91<br>84<br>72 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| support? 0x00779FE0<br>support? 0x0077A030<br>restore copy helper 0x0077ACA0<br>support? 0x0077ACE0 | 70<br>68<br>64<br>62 | 0/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
| support? 0x0077AC20<br>support? 0x0077AC60<br>support? 0x0077C380<br>deleting destructor 0x0077E420 | 58<br>56<br>54<br>30 | >=64/0/-<br>0/0/-<br>0/0/-<br>0/0/- |
Next: acquire a real initialized panel/sub-budget through a specified entry; record rate/funding edits and each reached method's returns/out-parameters/fields/collaborator calls. Extend observation bindings; keep partial comparisons complete=0. No ready train enqueue; local merge record is wip.
