| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| occupants-cities29; kind engine; city shadow timing | Codex; occupants-cities29; detached HEAD (supervisor publishes branch); fixes 6406fa7f7..9fdb9b0e8 plus observer lock | ready; ABI unchanged; cargo 7893, clippy clean, CTest 124/124; keys exact 18/18 scenarios; house clean; questions none | 1. Coordinator reruns census and assigns credit. 2. Track the independently reproduced native shutdown crash below. |

Result: O=occmgr 154/154, P=propmgr 752/752, S=ports 308/308 comparison lines exact across 18 scenarios; 17 clean verdicts, all ref_warnings=0; one native-exit control case below.
Evidence roots under C:/Users/tlori/FreeCity4/build-runs/: B=run7/census3/<scenario>.shadows.txt; F=suite/occupants-cities29-r7-final2/<scenario>.{log,verdict}; R=suite/occupants-cities29-r7-retry1/paradis-aiblinger.{log,verdict}. Manifests pin final DLL SHA256 3ceea2f15dac5c6cbf58124f49f9ea598af61a74a221a8d7bf4c434c5c3be28f.
O: replay Remove before native reentrant Shutdown; retain return/order checks. Native MessagePost/Send hooks distinguish external Move inputs from manager outputs and queued dispatch; final Fulham/Shadis logs expose those inputs. External Destroy adjacency is scoped to actual core shutdown requests: r6-d gen27-konradshohe-s0-default log captured shutdown-requested=0 (line 3860); no check/tolerance exclusions.
S: capture terrain CellHasWater at native seaport Init and terrain notifications; replay inputs, independently compare every access byte. F berlin-konradshohe/london-fulham: native Init water differs=0; later input changes explain the former first-month mismatch. Subscribe at city Init, after app services exist.
P: existing base fix 23e3dbdc4 implements prop-lotconfig.md PL24 closed upper world bounds; all six reported prop-list scenarios now exact (F/R). No new core rule or spec gap was needed. Final observer input writes now share the shadow lock.
Gate: final-source qbuild all PASS, cargo 7893/7893, clippy diagnostics=0, CMake PASS, CTest 124/124; WT/build-runs/qbuild-{test,clippy}.log and build-results/qbuild-20261026-142021-build-ctest.verdict (prior pre-lock gate also passed).
House: game-results/occupants-cities29-r7-house.{log,verdict}, clean, ref_warnings=0; O 5/5 months, P 19/19 lines, S airport 5/5 + seaport 5/5; no resyncs.
Before -> after inexact/lines (O,P,S), B -> F except Aiblinger uses R; changed line totals reflect actual monthly sampling, no dropped comparisons:
berlin-konradshohe: 1/3->0/3,1/15->0/15,1/6->0/6; london-fulham: 3/3->0/3,0/15->0/15,1/6->0/6.
timbuktu-big-city: 2/3->0/3,0/15->0/15,0/6->0/6; timbuktu-making-money: 1/3->0/3,0/15->0/15,1/4->0/6.
timbuktu-rush-hour: 4/4->0/3,0/18->0/15,0/6->0/6; empty-medium: 0/3->0/3,0/15->0/15,1/6->0/6.
empty-large: 0/3->0/3,0/15->0/15,1/6->0/6; gen27-konradshohe-s0-default: 3/24->0/24,1/124->0/124,0/48->0/48.
gen27-konradshohe-s0-churn: 0/24->0/24,2/124->0/124,0/48->0/48; gen27-konradshohe-s0-edge: 0/24->0/25,0/76->0/79,1/48->0/50.
gen27-spandau-s1-default: 2/24->0/24,0/124->0/124,1/48->0/48; gen27-tegel-s0-churn: 1/24->0/24,0/124->0/124,0/48->0/48.
maxisland-tuckerton: 0/2->0/2,1/12->0/12,0/4->0/4; paradis-aiblinger: 1/2->0/2,1/12->0/12,0/4->0/4.
paradis-freudenberg: 0/2->0/2,1/12->0/12,1/4->0/4; paradis-shadis: 2/2->0/2,0/12->0/12,1/4->0/4.
lua-lots-empty: 2/2->0/2,0/14->0/14,1/2->0/4; lua-lots-grown-qualified: 2/2->0/2,0/14->0/10,0/4->0/4.
Preserved failures: F Aiblinger stalled at julian=0 before actions; R clean retry supplies full coverage. F gen27-konradshohe-s0-default reaches end with 199/199 shadow samples exact, then corrupt/clean-exit-crash at native +0x1D0F94; suite/occupants-cities29-r7-control same scenario/DLL, plain profile (all shadows off), reproduces that same native shutdown crash. Pre-lock r6-e also reproduced it; neither failed verdict is called clean.
Limits: suite exits nonzero for script refusals and the preserved failures; the shadow checks remain strict. These are 18 representative scenarios, not a new 150-city census; full-census credit belongs to the coordinator. No questions filed, ABI exports unchanged, no coverage/regress edits.
