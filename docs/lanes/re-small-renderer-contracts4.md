| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-small-renderer-contracts4: native renderer and path owner contracts | Codex / `re-small-renderer-contracts4` (RE lane, detached worktree) | in progress: round 2; preserved flame evidence published; ten prior answers retained | 1. Prove TB4, GT4, FF2–FF4, PA4 residuals. 2. Validate questions and report. |

## Handoff
- Round 2 milestone bea42a5f: preserved flame evidence published in fire-flames §4.1; Init corrected in §4. No FF question closed yet.
- Goal: resolve TB4–TB7, GT3–GT6, FF2–FF4, PA4–PA8 with primary native evidence; specs/questions only.
- Worktree reused at detached HEAD; initial status clean and no commits ahead of main.
- Read handoff index, lane row format, merge train instructions, texture/path question entries.
- q.py cannot launch Bash here (CreateInstance E_ACCESSDENIED); use local PowerShell with bounded output.
- First milestone ac0e8a0f: texture §1.11–1.12 answers/archives TB5–TB7; §3 supplies manager slots 6/7 and destruction. Questions index/check passed.
- Native distinctions: signed P hangs on 0x40000001..0x7fffffff; zero levels => one; shifts mask to 31; size/counters/refcounts wrap u32; failure deletion excludes current handle.
- Second milestone 6bd15ef1: glyph/string info slot/lifetime/numeric/restore/output contracts; directory-entry three-body owner (94B); PA6–PA8 corrections. GT3/GT5/GT6, PA5–PA8 answered/archived; index/check passed.
- Round 2 milestone a2e1f920: material release/copy and paths §2.3; PA4 answered, questions index/check passed. Current milestone: glyph manager lifetime/inherited slots, flame transform/FF4 answer, geometry/index and persistence proof. Next: final report and exact residuals. No merge queue yet.
- Remaining requested questions: TB4 inventory scope; GT4 inventory scope; FF2 inherited-wrapper/legacy inventory scope; FF3 complete numeric store sequence. PA4 and FF4 answered this round.
- Pending native proof: 00470BC0..00470C9F (224B), mislabeled vector resize, is flame constructor called at 00472420/00472527 on fresh allocations: made=false,visible=true,wall=false,side=0,effect=null, identity embedded transform; position from shared word B21640 (not read).
- Pending fire proof: constructor 004711C0..0047122A sets stage -1 at complete-object+44, empty vector,on-model=false; SetStage 004728B0..004728F7 equal stage no-op, changed stage stores/creates-if-empty/updates, no error propagation. Update indexes tables directly, no clamp (004710FD..00471110).
- Pending Init proof 00471170..004711BC: initialized => true; otherwise base Init, group5005, set made/update existing flames, true irrespective base return; no CreateFlames. Init's receiver is adjusted (+4), unlike complete-object constructor.
- Pending visibility proof 00470270..00470290 writes byte before effect branch; start/stop argument1. Update 004710D3..00471160 stops0, clears/releases, replaces without rollback. Shutdown loop 00470DE0..00470E28: made effect tests slot5 active, stop0 only if true, release; unmade effect released without stop.
- Pending transform proof 00470CA0..00470DB2 no effect=>no-op; temporary header bytes2,5; x87 FSIN/FCOS and intermediate float stores 00470CD1..00470D1A. Do not claim complete matrix/draw-order proof.
- Inventory helper build-runs/renderer-inventory.py plus inventory-*.tsv (ignored): primary names texture55 bodies4758B (factory43/4271,binding3/89,manager8/333,material1/65), plus4 class/create helpers111B at800BF0(6),800E30(72),809120(6),80A680(27); not reconciled58/4883. Glyph38bodies4047B vs43/4475; flame19bodies3439B before mislabeled ctor vs16/3513. Three texture/three glyph data labels excluded.
- Standalone info/entry contracts published, but no manager/material/fire/family blanket completion. Cargo/clippy/ctest/game/shadow n/a. No ABI proposals. Actual specs only committed through 6bd15ef1; pending flame evidence currently exists only in this handoff.
- Main read-only source: C:/Users/tlori/FreeCity4; export queries via tools/re_query.py.
- No engine reads/edits, tools/maps/ABI edits, external communications, subagents, merge/push or branch changes.
- Guard: C:/Users/tlori/FreeCity4/build-runs/run4-re-small-renderer-contracts4-r2-manifest.json; budget 150000, maxCalls100, one round.
