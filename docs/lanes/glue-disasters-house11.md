| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-disasters-house11 (countable house disaster shadow) | Codex / `glue-disasters-house11` (engine; reserved detached HEAD) | READY measured partial closure: run494 disasters73/73, episodes1071/1071, resync0; clean2032-01-02 in1019.469s, ref1 and propmgr219/220 remain; Rust7119/0 clippy0 CTest81/81; ABI unchanged; DS28/DS47 open | (1) Land default-off measured work; (2) resolve reference warning and prop query difference before house-profile inclusion; (3) resolve DS28/DS47 before other disaster types |

## Final result

**READY for measured partial landing, not qualified for the house profile.** Exact countable result: `disasters 73/73` in [run494](../runs/house11-fire-riot-final-03-523.md), label `house11-fire-riot-final-03`. All dated summaries through2032-01-01 have mismatches0, resyncs0, unfollowable0, failures0. Episode comparisons1071/1071 exact: fire65/65, riot941/941, quiet65/65; original/ours steps55/55; no numbered or unexplained differences. Wait session57705 was collected to terminal; no request remains outstanding.

Reached2032-01-02 and PostSystemServiceShutdown in **1019.469s**, queue wall1042s, below1800s. Verdict `clean`; foreign exceptions, watchdog stalls, rtc, overruns, Rust panics and structured exceptions zero. **ref_warnings=1 remains, and propmgr is219/220.** These are outstanding qualification failures; monthly exactness does not waive them. Both switches remain default-off. No tools/ file or coverage TSV was read or edited.

### Change and gate

Commit81b5d3024 removes the legacy coverage>50 unfollowable increment: section5.4 specifies ordinary coverage refusals, so orig0/ours0 is fully observed. Actual result disagreements, cumulative differences, failures, resyncs and missing leader inputs remain counted. Ownership is unchanged; no acquired reference was relabeled CREATED. Earlier commits implement the bounded fire/riot schedule and monthly summaries, described in the historical report below. No new owners or questions this final round; C ABI unchanged.

One final-source `qbuild all` passed: Rust7119/0, clippy0 diagnostics; queued native request `qbuild-20261024-151051-build-ctest`, build47/47 edges in3.4s (cache30 hit/1 miss), CTest81/81 in98.7s, native total106.1s. Gate session14601 collected to terminal. Source unchanged since that gate; no redundant full check or direct MSVC invocation. Full capture `build-runs/gate-house11-final.log`; native logs `fc4verify-build.log`, `fc4verify-ctest.log`.

### Measured limits

Fire(9,70), riot(40,33), nine leaders removed after200 samples, schedule completed at fire58/riot202 with resync0. DLL SHA prefix0311f5eb. Request retained in game-queue/done; terminal log/verdict in `build-runs/game-results/house11-fire-riot-final-03.*`. Earlier [run493](../runs/house11-fire-riot-final-02-522.md), final-02, completed both episodes exactly but stalled outside the plugin at2026-02-28 after283.047s, ref0. Its separate verdict is preserved. Final-03 is one unchanged-source retry, not a replacement verdict.

Remaining warning: `PruneGone:275`, class0x2977AA47, object187DDAE8, at15:37:22.930. It releases an AddRef-held observed occupant after manager removal; investigate lifetime rather than suppressing the warning. Previous run492's additional warnings remain documented below.

**Another house shadow is disturbed versus train193:** propmgr219/220 versus220/220. Its only nonzero query is2031-05-01, lot(23,53)4/3, listsDiffering1, resultsDiffering0, captured21, filterChecks973, inputErrors0. Run492 failed a different query, so causality remains unestablished. Landvalue72/72 versus train19371/72; all other51 non-disaster groups exact. Candidate52/53 groups exact; baseline51/52. All per-shadow counts are committed in run494. Ordered payload comparison and nonzero lines: `build-runs/house11-final-03-comparison.txt`, against authorized main `build-runs/regress-train193-shadows.log`. This does not claim identical city outputs.

### Profile, questions and next steps

Do **not** apply the addition until ref0 and other house shadows qualify. Exact proposed single profile line, reconstructed from the measured train193 ini (harness-managed autoload excluded):

```powershell
shadows = "disable = budget,ordinance`nbudget_shadow = 1`ndemand_shadow = 1`npollution_shadow = 1`nprotection_shadow = 1`ncrime_shadow = 1`naura_shadow = 1`nflora_shadow = 1`nlandfill_shadow = 1`npower_shadow = 1`nplumbing_shadow = 1`nlandvalue_shadow = 1`nflammability_shadow = 1`nresidential_shadow = 1`nneighbours_shadow = 1`nzones_shadow = 1`ndevelopment_shadow = 1`ntract_shadow = 1`nzonedev_shadow = 1`nlotdev_shadow = 1`nhistory_shadow = 1`nclock24_shadow = 1`ncommercial_shadow = 1`ncivic_shadow = 1`nparks_shadow = 1`ntraffic_shadow = 1`ntraffic_shadow_hypotheses = 7`ntraffic_shadow_lines = 1`nterrain_shadow = 1`nautomata_shadow = 1`nnetmgr_shadow = 1`nmysims_shadow = 1`noccmgr_shadow = 1`noccmgr_shadow_follow = 1`nadvisors_shadow = 1`ntutorial_shadow = 1`ncitydetail_shadow = 1`nvariant_shadow = 1`nfeature_shadow = 1`nlotcfg_shadow = 1`npropmgr_shadow = 1`npropmgr_shadow_cells = 32`nnetocc_shadow = 1`nnetrules_shadow = 1`nstrings_shadow = 1`nlua_shadow = 1`noverlays_shadow = 1`noverlays_fixture = 1`nscres_constructed = 1`nsimgrid_shadow = 1`nlotrec_shadow = 1`nordinance_shadow = 1`nlotconf_shadow = 1`nautomata_owners_shadow = 1`nweather_shadow = 1`nscres_file_shadow = 1`ndbpf_shadow = 1`nautotest_unpause = 2`nautotest_end = quit`npopup_dialogs = 0`ndisasters_shadow = 1`ndisasters_scripted = 1"
```

The coordinator must preserve newer main additions and append only the two disaster assignments to the existing single string, never add a second shadows entry. This snapshot is a proposal, not authorization to replace newer settings.

No new questions. Existing DS28/DS47 in `docs/specs/questions/disasters.md` keep other disaster types off. Next: land this gated measured default-off work; resolve reference lifetime and prop-list failures before profile inclusion; resolve existing contracts before extending the schedule. Detached HEAD is intentional; supervisor publishes the reserved branch. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-disasters-house11-r3-manifest.json`.

Everything below is historical run492 evidence, superseded by the current result above.

## Previous round result (run 492; retained evidence)

**WIP, not qualified for house inclusion.** Exact countable result: `disasters 3/73` in [run 492](../runs/house11-fire-riot-01-521.md), label `house11-fire-riot-01`. The request was submitted once and its existing terminal verdict was collected from wait session 98875. No game request remains outstanding. The user's resume instruction calls for WIP when the disaster lines are non-exact; this is a measured failure, not a stop on the short-form spec questions.

The game reached 2032-01-03 and PostSystemServiceShutdown in 1200.343 real seconds (queue wall 1220s), within 1800s. Verdict word `clean`; foreign exceptions, stalls, rtc, overruns, Rust panics and structured exceptions all zero. **Reference warnings were 3**, so the run does not meet the required ref0 qualification despite its verdict word. DLL SHA prefix `afc115f5`.

### Changes

Implementation `6345f0625` adds default-off `disasters_scripted`, requiring the existing default-off `disasters_shadow`. It starts a fire then riot through the original manager, holds turbo off during their animation-time updates and restores configured turbo after both layers end. Commit `21f74d387` bounds the riot by removing its leaders through occupant-manager slot 21 after 200 samples, retaining owned holds through both ending ticks. Commit `eec9904dc` waits for ongoing=false on both layers rather than dirty=false, because an ended riot remains subscribed (§6.7). Monthly `disasters shadow YYYY-MM-DD: ... mismatches=N` lines include cumulative differences and diagnostic failures. C ABI exports unchanged.

The measured schedule started fire at (9,70), riot at (40,33), removed nine leader occupants at 200 riot samples and completed at fire=58/riot=202, resyncs=0. Through city shutdown every episode comparison was exact: **1259/1259: fire 69/69, riot 1111/1111, quiet 79/79; original steps 55, ours 55; no numbered or unexplained differences**. Resyncs, failures and missing/unfollowable native inputs apart from the legacy coverage diagnostic were zero.

### Why the monthly shadow failed in run492

The first three monthly lines (load, February and March 2026) are exact. April 2026 onward retains cumulative `unfollowable`, ending at 11 on 2032-01-01, with resyncs=0 and failures=0. The existing line `if (in.coverage > 50) ++g.unfollowable` in FC4DisasterShadow.cpp counts ordinary high-coverage refused ignitions even when **orig 0 / ours 0 and mismatches=0**. Examples at coverage 75 and 97 are preserved in the game log. Section 5.4 explicitly specifies these refusals; this accounting needs an implementation correction, not a new spec question. No source was changed to erase this failed measurement or relabel it exact.

Two per-object last-reference warnings occur at `PruneGone:275`, class `0x2977AA47` (objects `186C2768`, `18A3CF88`); one at `DropAll:259`, class `0x896E75AF` (object `14FBEC84`). These warnings remain unresolved. Both helpers currently release references acquired by Keep; investigate lifetimes and game removal ordering rather than masking warnings or changing acquired references to CREATED releases. See docs/ops/lessons.md §5.1b.

### Other house shadows versus train193 in run492

**The scripted run disturbs another house shadow:** propmgr falls from 220/220 to **219/220**. Its only nonzero line is the 2026-07-01 query: `mismatches=1 calls=160 cells=32 lotCells=24 props=20 resultsDiffering=0 listsDiffering=1 first: lot(32,53) 1/0 @0 0A014D68/00000000 lotInput=slot40-before-prune captured=24 filterChecks=814 inputErrors=0`. This is an observed run difference; causality among destruction, altered city development and capture timing is not established.

Landvalue improves from 71/72 to 72/72; train193's August 2028 nonzero line is absent. All remaining 51 non-disaster groups are exact, including flora, landfill and traffic (whose logs use `mismatched` rather than `mismatches=`). Many payloads, dates and sample totals change, so this is not a claim of identical city outputs. Full ordered-payload comparison and nonzero lines: `build-runs/house11-shadow-comparison.txt`, generated by `build-runs/compare-house11.py` against main's explicitly authorized `build-runs/regress-train193-shadows.log`. The parser checks all numeric mismatch fields, all 52 baseline dated groups and the new disaster group: candidate 51/53 groups exact, baseline 51/52.

Per-group dated clean/total, with train193 in parentheses:

```text
advisors 73/73 (73/73)
aura 72/72 (71/71)
automata 72/72 (72/72)
automataowners 72/72 (72/72)
budget 72/72 (72/72)
citydetail 1/1 (1/1)
civic 72/72 (72/72)
clock 73/73 (73/73)
commercial 18/18 (22/22)
crime 71/71 (71/71)
dbpf 73/73 (73/73)
demand 2240/2240 (2241/2241)
development 925/925 (913/913)
disasters 3/73 (absent)
feature 75/75 (75/75)
flammability 54/54 (54/54)
flora 72/72 (72/72)
history 79/79 (79/79)
landfill 72/72 (72/72)
landvalue 72/72 (71/72)
lotcfg 293/293 (293/293)
lotconf 73/73 (73/73)
lotdev 73/73 (73/73)
lotrec 73/73 (73/73)
lua 73/73 (73/73)
mysims 73/73 (73/73)
neighbours 72/72 (72/72)
netmgr 73/73 (73/73)
netocc 72/72 (72/72)
netrules 72/72 (72/72)
occmgr 72/72 (72/72)
ordinance 72/72 (72/72)
overlayrecords 292/292 (292/292)
overlayreplay 292/292 (292/292)
overlays 73/73 (73/73)
parks 72/72 (72/72)
plumbing 144/144 (144/144)
pollution 72/72 (72/72)
power 393/393 (390/390)
propmgr 219/220 (220/220)
protection 288/288 (288/288)
residential 1136/1136 (1136/1136)
scres 149/149 (149/149)
simgrid 73/73 (73/73)
strings 72/72 (72/72)
terrain 594/594 (593/593)
tract 145/145 (145/145)
traffic 72/72 (72/72)
tutorial 146/146 (146/146)
variant 1/1 (1/1)
weather 73/73 (73/73)
zonedev 365/365 (365/365)
zones 292/292 (292/292)
```

Terrain shutdown additionally reports all seven query kinds exact, 17425 other checks with mismatched=0, unfollowed vertices=0 and pending=0; followed-late=1 and one monthly ahead>0 diagnostic.

### Previous gates and evidence

Final-source `qbuild all` already passed on engine source `eec9904dc`: Rust 7119 passed/0 failed, clippy 0 diagnostics, native build 52/52 in 4.3s, CTest 81/81 in 344.5s, native total 358.0s. Request `qbuild-20261024-130822-build-ctest`; no changed-tree warning. The preceding intermediate pass had a changed-tree warning and remains distinct under `build-runs/intermediate-house11/` (build 260/260, 349.7s; CTest81/81, 876.8s); it is not the final gate.

This resume collected exactly one new explicit `qbuild verify` to terminal: `qbuild-20261024-135021-build-ctest`, session 62645, PASS, build 46/46 edges in 2.0s (30 cache hits/0 misses), CTest 81/81 in 75.9s, native total 79.5s. Captured tail/full output: `build-runs/verify-house11-resume.log`; native logs `fc4verify-build.log` and `fc4verify-ctest.log`. Initial q.py failed before launching verify because Git Bash mapping was denied; the direct Python capture helper launched the only replacement request. Source was unchanged after the full gate; only reports and ignored comparison helpers changed during this resume.

Game evidence: `build-runs/game-results/house11-fire-riot-01.verdict` and `.log`; original request retained by the harness under game-queue/done. Existing verdict must not be overwritten. No qwait was used, no game was started directly, and no tools/ file was read or edited.

### Previous questions and profile proposal

DS47 was filed by this lane in `docs/specs/questions/disasters.md` for typed observation/ownership contracts for pipe burst, toxic spill and terrain-deforming disasters; DS28 remains open. Those types stay off with no verification claim. No new spec question was needed for the measured high-coverage accounting error; its refusal contract is already explicit. The coordinator must regenerate the question index (engine lanes cannot run tools/).

**Keep the current single `shadows = ...` profile string unchanged for this WIP result.** After qualification, the proposed exact additional ini assignments are `disasters_shadow = 1` and `disasters_scripted = 1`, appended inside that existing shadows string, never as a second shadows assignment. No profile addition or coverage credit is warranted by 3/73/ref3/propmgr219/220. Both switches remain default-off; tools/regress.ps1 and both coverage TSVs are untouched.

Next: correct cumulative accounting for matching coverage refusals; investigate all three acquired-reference lifetime warnings and the prop-manager list mismatch; full-gate changed source, then rerun with a new unique label and the same house switches. Publish only an all-month exact/ref0 result without added house shadow failures. Resolve DS28/DS47 independently before extending the schedule to other types. Detached commits are intentional; the supervisor publishes the reserved branch. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-disasters-house11-r2-manifest.json`.
