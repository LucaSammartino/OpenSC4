| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-answers5 (network owner questions) | Codex / detached `re-netocc-answers5` | blocked: coordinator STOP for reboot | Resume owner traces from the round-2 Handoff; no merge queue written |

## Report

Updated `docs/specs/network-occupants.md` with three complete question answers, corrected live layouts,
record tables, failure/ownership rules, placement transitions and expanded prose conformance vectors
16, 17, 19 and 20. No implementation, ABI exports or ABI tables changed; no proposed ABI rows.

- **NO34** (`c2d33055`): demolition quote reads signed-int64 property `0x0A2ECED8` directly from the
  control exemplar during one-time StaticInit. The float property `0xE9A18D3F` does not supply the quote.
  Test 17 now covers zero/missing/failed lookup, exact signed words and one-time loading.
- **NO37** (`ddbbc96d`): pylon serialization receives primary+0xC. The first alleged transform is a bool;
  there is one transform at live +7C, a u16 call at +14C, and a GZ string at +1AC rather than an int64.
  Source IID is 0x26BB767E; owner IID is 1. v3/v4 preserve absent tail; reached releases are not rolled back.
  Placement bits/results, renderer +70 insertion/+74 removal, repeat Init, packing tuples and shutdown
  resources are explicit. Renderer +28's return is discarded.
- **NO35** (`015ea80f`): pole record/live offsets reconcile with the same +C adjustment. Spec now covers
  counted line-reference list, two collections of counted xyz lists, exact three wall-helper records,
  four vectors through the selected shared +E4 table, distinct destination/line IIDs and delegate cleanup.
  Corrected quirk 11 and test 16; live +B0 vector end is not the released delegate at live +BC.

All three entries moved to `docs/specs/questions/answered/network-occupants.md`, marked DIFFERS with
date/lane, and the questions index regenerated. `questions.py find` confirms all three are archived.
No new questions filed.

**Validation:** `python tools/questions.py index` and `check` passed after every milestone;
`git diff --check` passed. Numeric widths, signed-byte extension, ref IIDs, placement masks/call slots,
and record operation order were checked against exported instruction bytes. PE data confirms initial
quote words are zero. Sources were the main checkout's export and scoped decomp functions; REA was
not required. Cargo, clippy, ctest and in-game/shadow runs: n/a (spec-only lane). These are specified
conformance vectors, not executed engine tests or a claim of owner closure.

**Remaining:** NO33 stays open for pole connection algorithms, tables, geometry, drawing/picking and
resource closure; only layout/record facts were corrected here. NO36 stays open for pylon texture/model,
draw/pick/message and other owner algorithms; lifecycle and packing inputs alone do not close it.
UG5 remains open and the underground spec is unchanged: connection geometry/eight-neighbor decisions,
callback, view/model/rendering and record/reference closure still need a dedicated trace. NO12, NO13,
NO14 and NO29–NO32 were not reached. Work was bounded to finish and land verified record/placement
answers within the measured lane budget rather than declaring these wider questions answered.

## Handoff
- STOP received 2026-10-23: coordinator reboot (logon-session leak). No merge-queue entry written; resume only when released.
- Goal: harvest PR10, then answer NO33/NO36/UG5 with prose conformance vectors; those three broad questions remain OPEN, with no round-2 vectors yet added.
- Done: `3393a1a2` harvests a8b30439; `36a3a300` harvests a33fa02e; amarret authorship retained. `c2b2f5ce` reconciles the harvested tree. The second harvest briefly contained conflict text after failed UTF-8 patch generation; use c2b2f5ce or later, not that intermediate tree.
- Reconciliation: keep existing NO34/35/37 answers and vectors; retain new PR material; PR question IDs NO32-37 become NO38-43. Re/names_windows.tsv unions addresses with pre-round rows winning duplicates; contributor symbol files added. Coverage additions set spec only, with core/shadow/replace '-'. No engine/plugin/capi or ABI edits.
- Validation at c2b2f5ce tree: questions index/check and doccheck check PASS; no game/build tests (spec-only). Index was regenerated, not hand-merged. Harvest prose was inspected as state/record/ordering rules, not copied control flow.
- REA procedure_pseudo_code returned target_unavailable (no app open); fall back to main export and scoped decomp. No session-opening tools authorized/used.
- Conflict decisions: accept PR occupied-cell rounding (0061B010 bytes; PE AA7D78=0.0625, AA7D7C=12582912); accept PR path-type/entry-byte table (00619790 bytes). Retain local distinct pole-vs-line reference IID rule and detailed pole record/vectors, rather than PR's shorter versions. Keep local demolition quote contract and existing pylon record/lifecycle answer.
- Next exact step: correct/expand network section 3.6 from the instruction evidence below, then underground sections 1/4-9, then continue pole/pylon owner geometry and add vectors. Do not mark broad closure from the partial facts below.
- UG5 verified: actual Read 00657590 compares u16 to exactly 2 at 006575A7 and rejects other versions BEFORE base. Receives serializable primary+4; displacement C4 maps live C8 (attached), C8 maps live CC (linked). Both ref reads request IID 26BB767E; attached result is adjusted -4 before storing (00657693-006576D4). Seven lists use 006574A0; live starts 6C/78/84/90/A0/AC/BC, u32 count and 24-byte vertices (same wall helper as section 3.4).
- UG5 callback 00653DA0 only releases static resource manager B46830, nulls it, returns true; DOES NOT clear latch B46838 (verified all 28 instruction bytes).
- UG5 packed states at B46854/B4685C/B46864, verified 00654260 bytes: (1,1,1,0,0,1,0,3,-1,-1,-1,-1.0,1,-1); (1,1,1,0,1,1,0,3,4,5,-1,-1.0,1,-1); (1,1,1,0,1,0,0,3,4,5,-1,-1.0,1,-1). Null renderer leaves states untouched; renderer +28 return ignored.
- IMPORTANT harvested address-label corrections still needed: 00648360 is PIPE QueryOccupantInterface, not GetTransform; 006540F0 copies position, not SetTransform. 00654620 is SetLinkedOccupant (old release, new store/AddRef; null removes owner via manager +54(this,1,0)), despite decomp label HasViewDependentModel. 00655520 reloads static texture bindings for zoom, despite decomp label GetModelInstances; true model enumeration at 00659D70 supplies one adjusted instance and AddRef (verify its vtable use). Do not trust inferred names as signatures.
- UG5 creation decomp at class lines 364-572 is misanalysed (entry 65471D); use asm of 654710 before publishing numeric geometry/eight-neighbour offsets. It searches own cell first; neighbour hits are released, fresh objects request manager links, then own-cell occupant is reacquired and refreshed. Full geometry/descriptor rules remain unverified.
- Pole facts read but NOT yet specified: ctor 0064DB30 zeroes position/cell/kind/table/model/effect/flags/vectors, rotation+zoom=-1, dirty D0=1, identity transform. 00649B30 confirms subtype (instance>>8)&15: 1/2 table A, 3/4 B, otherwise retain pointer (initially null). PE tables match harvested signs/values; exact float bits: 31.7=41FD999A,38.7=421ACCCD,4.8=4099999A,6.5=40D00000,3.3=40533333,4.5=40900000 (negatives sign bit).
- Pole geometry read but needs further asm confirmation: floor is 10x10 square, optional 0.7853900194168091 Y rotation for model instance high word 0503/0504, UV arrays AA9B18=(0,0,1,1), AA9B28=(0,1,1,0). Walls use topA,bottomA,bottomB,topB; bottom y=10C, v=(topY-bottomY)*0.0625; u=0/1, RGB white with retained alpha. 0064C470 byte-checked edge table: rotations 0..3 use [(2,3),(1,2)], [(1,2),(0,1)], [(0,1),(3,0)], [(3,0),(2,3)].
- Pole bounding/pick: 00649A00 is occupant-view +8 (add 8 to displacements); integer bounds expand only destinations with nonempty wire list, comparisons UNSIGNED despite signed cell conversion; x/z min = cell*static size, max=(cell+1)*size, minY floor10C, maxY positionY+60. Pick 0064A870 builds model bounds translated by pole position, expands over TWO wall lists, ray-box test with strictly closer distance; verify remaining transforms/slots16-17/line walk before closing NO33.
- Pylon verified 005F0C50: SHARED u32 timer B23B80 advances only dispatch state 1/4; unsigned total>=500 resets to 0 (drops excess), toggles blink1A4 and dirties68. UI/dispatch-type/state setters dirty and clear blink only on changed value. Bitmap setter always stores 1A8/string1AC and dirties. Source setter AddRefs new before releasing old, no dirtying.
- Pylon 005F20A0 first page byte-checked: W=f32(ViewportUnitsToWorldUnits(44))*0.5 rounded f32; H=ViewportUnitsToWorldUnits(150)/cos(view pitch) rounded f32. Atlas stride type4=0.203125 otherwise0.171875; blink selects frame3, else UI0 maps frame1. Quad at E4 is stride20 xyzuv, left=-W/right=W, y=0/H,z=0. Remaining yaw, two-corner bounds, call-number quad and placement require asm verification before publishing.
- Pylon texture tables PE AA4F20 state0..6 = EA3EE100/110/120/130/140/150/120; AA4F3C = 14315E00/10/20/30/40/50/20; AA4F58 = 2BB075B4,2BB06F3F, then five 2BB075B4. RebuildCurrentTexture type0/6 selects first,1/5 second,4 builds MySim; other types key0. Needs failure/handle cleanup check and BuildMySim/BuildCallNumber asm; decomp is unreliable there.
- Evidence: main decomp cSC4PowerPoleOccupant.c, cSC4PylonOccupant.c, cSC4UndergroundConnectionOccupant.c; connection actual Read in _unnamed/region_00650000.c around line1339. Ignored helpers build-runs/read-pe.py, read-evidence.py, harvest-patch.py; invoke Python -X utf8 for generated patch stdout. No RE output committed.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-netocc-answers5-r2-manifest.json. Remaining open besides targets: NO18/19/24/27-32 and harvested NO38-43; consult actual question files for full list. No new ABI rows or new questions beyond harvested entries.
