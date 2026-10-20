# re-occprops
| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-occprops (spec writer, occupants and props) | `re-occprops` (docs lane) | **ready**: 33 owners / 453 fns / 66,254 bytes specified in 4 commits (e4806019 lot-records 9.1/9.2/10.3/10.4/12, fcae164d occupant-manager 13.1 base class + 13.2 filter family, 5137f5f4 prop-occupant 1.2 cross-ref + OP1-OP10, 951fa9c7 coverage-map spec columns); 10 OP questions filed; no ABI or engine changes | docs/specs/lot-records.md, docs/specs/occupant-manager.md, docs/specs/prop-occupant.md (next steps: -) |


## Report

**Goal.** Lift the coverage-map subsystem "occupants and props" (33 owners, 453 functions, 66,254 bytes —
patterns `cSC4ConstructionOccupant`, `cSC4PropOccupant*`, `cSC4SignpostOccupant`, `cSC4TextDecalOccupant`,
`cSC4Occupant*`, `cSC4*OccupantFilter`, `cSC4*Occupant`) from unspecified to specified by extending the existing
specs. Round 2 of 2: round 1 finished all RE and handed off measurements; this round wrote the spec text.

**What changed (all additive; no existing section deleted).**

1. **`docs/specs/lot-records.md`** (commit e4806019)
   - §1 table: two new class rows — signpost `0xAB72FBB3` (Write `0x005F5080` / Read `0x005F50F0`, version 2/any)
     and text decal `0x8B79C707` (Write `0x005F7DB0` / Read `0x005F7E50`, version 3/any).
   - §9: new §9.1 (saved-field addresses in Write order, incl. the `+0x5C/+0x60` saved refs vs the live
     `+0x68/+0x6C` pair, and record #8 written as `(+0xF8 != 0)`) and §9.2 (messages, `SendConstructionMessage`,
     `SetLot` placement chain, `DoDailySim` state machine roles for `+0x140…+0x184`, `GetPosition`/`GetBoundingBox`
     read the live box at `+0xA0…+0xB4`). The previous "field names not known" note is reduced to `+0x118` only
     (Q3 partially answered).
   - §10.3 `cSC4SignpostOccupant`: record (u16 2, inline prop record §10.1, the `+0x88` stream-slot-0x3C u32, the
     `+0xCC/+0xD0/+0xD4` vec3), state table (`cS3DModelInstanceBasic` at +0x6C/+0x70, text `cRZString` at `+0x98`
     with the shared empty-string vtable `0x00A80810`, edit/blink/tick bytes `+0xA8…+0xB5`, four 3×3 orientation
     matrices `+0xB8…+0x118`, ten texture bindings `+0x540…+0x564` from RID table `0x00AA5214…0x00AA523C`, packed
     render state `+0x574`), the full sub-object map (8-slot class view `0x00AA5478`, 24-slot prop view
     `0x00AA5418`, message target, 34-slot occupant view `0x00AA5380` with its overrides, serializable
     `0x00AA5364`, 26-slot holder view `0x00AA5300`, `cISCExemplarPropertyHolder` `0x00AA52E8`, holder base
     `0x00AA52E4` at +0x6C), and behaviour (`Init 0x005F73A0`, `LoadTextures 0x005F5EF0`, `SetText 0x005F7600`
     with its equal-string-AND-unchanged-capacity skip, `SetDynamic 0x005F4BA0` with the `0x00AA5204` colour rows,
     `SetEditMode 0x005F7320` with the animation-tick register/unregister, `DoMessage 0x005F7510` incl. the
     500 ms blink helper `0x005F4960`, `SetAllFlags 0x005F4A90` with the flag-2 protection, creation sites).
   - §10.4 `cSC4TextDecalOccupant`: record (u16 3 with the v1/v<3 quirks, the duplicated lot-tag write, the
     double-written decal-ID pair, the colour byte = low byte of an f32), state table (`+0x6C` text string, decal
     ids, height/colour/alpha/orientation, the two overlay-manager handles), the same sub-object map (11-slot
     class view `0x00AA5680`, 34-slot occupant view `0x00AA5588`), and behaviour (`SetText/GetText/Height/Color/
     Alpha/Orientation/Dynamic`, `SetPosition 0x005F7AA0` with the measured `+0x88`/`+0x8C` conflict → OP1,
     `UpdateDecal 0x005F7810` incl. the `MoveOccupant` slot-24 refresh, `UpdateOverlayManager 0x005F7C80`,
     `UpdateTexture 0x005F8190` with RID `0xCC1FD62C`, `Init 0x005F7F40`, `Shutdown 0x005F76C0`, `DoMessage
     0x005F8030`, `SetAllFlags 0x005F81F0`, the label tool `0x004BD320`/`0x004BD890` and debug command
     `0x0BEAF1A2`).
   - §12: points to the new base-class section in occupant-manager.md.
   - §13.1: signpost/decal return-channel rows (the `NEG AL; SBB EAX,EAX; INC EAX` epilogue form).
   - §15: signpost/decal embed-the-prop and occupant-manager-coupling replaceability notes.
   - §16: new quirks (duplicated decal writes, colour-byte aliasing, signpost skip rule, `+0xCC` from the live
     crane pointer) and open questions Q7/Q8 (pointing at OP1/OP2).
2. **`docs/specs/occupant-manager.md`** (commit fcae164d)
   - §13.1 The occupant base class `cSC4Occupant 0xA9BC9AB6`: layout table (vptrs `0x00AA41D8`/`0x00AA41BC`,
     holder at +8, type `+0x24`, stored box `+0x28` with the 0xFF default, removal counters `+0x2C`, status
     `+0x2E`, flags `+0x2F`, placeable `+0x30`); the full 34-slot `cISC4Occupant` table with each body (QI
     `0x005E89C0` and the `ADD ECX,8` → holder path, refcount `+0x0C`, removal-flag counting, `PostOccupantMessage
     0x005E9010` incl. the synchronous Move delivery when `+0x30` is set, highlight nibble in `+0x2E`, occupant
     groups through property `0xAA1DD396`, the folded slots by address); the base save record (matches
     lot-records.md §2.3).
   - §13.2 Occupant filters: the `cISC4OccupantFilter` interface (7 slots, IID `0x46A80026`), the base class
     `cSC4OccupantFilter 0xA9C74DC9`, and a table of 18 family members with CLSID, size and the exact accept rule
     of `IsOccupantIncluded`/`IsPropertyHolderIncluded` (Default `0x005E9630`/`0x005E96E0` with its five exclusion
     IDs and property `0xCAA45670`; Realistic `0x005E9990` with the flag bytes, `0x5001`, and the signpost/decal
     exclusions; the demolition filter `0x00466730`; the shared one-type filter (cross-ref §4.4); the types-set
     and hashed variants; ViewLevel; StringSpecification `0x005E9F20` + the 22 tokens of `BuildSpecification
     0x005EA610`; MissingPluginPack with the 1→1 / 0,2→0 epilogue; Subway/Water/FloraBlocking/ActiveFire/
     Queriable/DemolishByNetworks/InsideLot/BlockingLot/SurfaceNetwork; plus the three non-class filters
     (ambience, automata, crime) and `SC4GetOccupantViewLevel 0x005E9CA0`). Untraced bodies are marked and filed
     (OP5, OP6, OP8).
3. **`docs/specs/prop-occupant.md`** (commit 5137f5f4): §1.2 notes that the signpost and text decal embed a
   complete prop occupant at object +4 and point to lot-records.md §10.3/§10.4.
4. **`tools/coverage-map.tsv`** (commit 951fa9c7): the seven occupants-and-props rows got their spec column
   (additive only, order untouched): 209 lot-records.md; 210 prop-occupant.md, lot-records.md; 211/212
   lot-records.md; 213 occupant-manager.md, lot-records.md; 214 occupant-manager.md, flora.md, demolition.md;
   215 occupant-manager.md, lot-records.md, prop-occupant.md, flora.md, landfill.md, disasters.md. Verified by
   pattern-matching the map against the owner names (all 14 spot-checked owners resolve to rows whose spec files
   exist). `coverage.py` was run from <MAIN> (it reads `tools/coverage-map.tsv` relative to the main checkout, not
   the worktree), so the regenerated `docs/coverage-report.md` still shows `-` for this subsystem until the train
   merges the branch — the map edit is in this branch.

**Questions.** `docs/specs/questions/occupants-props.md` (new; prefix OP; index regenerated): OP1 text-decal field
roles (`+0x78/+0x84/+0x88/+0x8C/+0xAC` ctor-vs-setter-vs-Shutdown conflicts), OP2 stream slot 0x3C semantic +
slot 0x30 Float32-vs-Uint32, OP3 signpost colour tables, OP4 Realistic filter flags and property `0x5001`, OP5
BlockingLot slot 3 body, OP6 Immovability/DummyAuto/ZoomRotation bodies, OP7 construction visual flags and the
0x5007 slot call, OP8 the `0xCA0E3852` filter identity and the two set-variant bodies, OP9 automata-filter type
identities, OP10 the `GetOccupantGroups` overload's `0xA52160F5` service. lot-records.md §16 gained Q7/Q8
pointing at OP1/OP2.

**Key RE decision (this round).** The round-1 handoff recorded several offsets from the serializable/occupant
*view* rather than the object base (decomp names like `field26_0x5c`). Re-deriving from `re_query.py asm --bytes`
showed Write/Read run on the view (`ECX` = the +0xC/+0x10 sub-object), so every "conflict" (construction
lot/building refs, decal position fields, signpost `+0xCC` vec3) reconciles as: **saved offsets in the record
tables are view-relative; the state tables use object-base offsets.** All record tables in lot-records.md state
which base they use.

**Not covered here (left to the owners).** `cSC4PipeConnectionOccupant` (9 fns — network side; matches
`cSC4*Occupant` and is pointed at the six-spec list, but its behaviour belongs with network-occupants when that
lane lands). Burnt-building/terrain-decal/automata-controller prop subclass layouts stay with lot-records §16 Q6.
`cSC4OccupantGroupTemplate` (10 fns) is referenced from §13.2's group text; its own behaviour remains with the
automata lane (cross-ref noted).

**Next steps.**
1. Merge-train: kind `docs`, no ABI, no game runs.
2. After merge, regenerate coverage (the map edit lands with the branch; the report will flip this subsystem to
   specified).
3. OP1/OP2 are the two questions that block a byte-exact signpost/decal save implementation; a live-save lane can
   answer them with one debug command each.

---