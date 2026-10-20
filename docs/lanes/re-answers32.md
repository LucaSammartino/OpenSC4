# Lane: re-answers32 (RE answers: prop-occupant PO1b/PO2/PO3/PO5/PO6)

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| re-answers32 (RE answers: prop-occupant PO1b the stride-0x88 allocator group, PO2 the sixth sub-object, PO3 the placeable seam, PO5 terrain re-solve, PO6 slots 5/6 SDK names) | `C:\Users\tlori\FreeCity4\.claude\worktrees\re-answers32` / branch `re-answers32` | **ready, not merged** (docs only: `prop-occupant.md`, `questions/answered/prop-occupant.md`, `questions/unsorted.md`, `QUESTIONS.md`; **no `engine/` touched**, no game runs, no ABI rows proposed — the 25 rows from re-answers27 stay in `build-runs/pending-abi/re-answers27-rows.tsv` and are untouched, but two of their `method` names should be corrected when applied: slot 5 `GetPropPlacementProperties`, slot 6 `SetPropPlacementProperties`). All five questions ANSWERED 2026-10-20 in `docs/specs/prop-occupant.md` §10, rules written into §1.2, §2.4, §4.1/§4.1b, §4.2, §5.2, §6.1, §8, quirk 9, tests 26–30. | Merge into main with `git merge --no-ff` (docs only; nothing to gate or regress). Note `prop-occupant.md` arrived on main only via re-answers27; if re-answers27 has not merged first this lane's diff will conflict on that file — merge re-answers27 first. |

## The five answers, one line each

1. **PO1b — DIFFERS from its premise.** The stride-`0x88` allocator group serves **`cSC4PropOccupantAutomataController`**
   (CLSID `0x2A4017A0`): `operator new` `0x005EDD80`, GZCOM creator `0x005F0490`, destructor `0x005F04D0` all pop/push
   pool `0x00B0A914`; the constructor `0x005EFDB0` writes up to `+0x84`, matching the stride. The "pool/factory method
   table" at `0x00AA4960` is actually **five pool-object vtables** (`0x00AA4960`…`0x00AA49AC`, one 4-dword
   `[AllocNode, FreeNode, CreatePool, FreePool]` group each at 16-byte spacing), each installed by its own
   `___cxa_atexit`-managed init constructor into its own global: group 0 `0x00B0A8E4` (init `0x00A736E0`), group 1
   `0x00B0A8F0` (`0x00A73740`), group 2 `0x00B0A8FC` (`0x00A73780`), group 3 `0x00B0A908` (`0x00A737C0`), group 4
   `0x00B0A914` (`0x00A73800`). Node counts (measured from each `CreatePool`'s `MOV ESI, n`): 78 / 102 / 110 / 75 / 120.
2. **PO2 — MATCHES the [inferred] guess.** The sixth sub-object is **`cISCExemplarPropertyHolder`** (IID `0x0AC2B5F7`).
   Proof: the QI chain ends at `cSCExemplarPropertyHolder::QueryInterface` `0x005FD310`, which answers `0x0AC2B5F7`
   with `LEA EAX, [ECX + 0x14]` from `ECX = obj + 0x10` — exactly `obj + 0x24`; and the `+0x24` vtable's
   QI/AddRef/Release are `SUB ECX, 0x24` thunks to the primary trio (`0x005EE140`/`0x0064DE00`/`0x0064DE10`). Its two
   real slots are `SetDefaultExemplar` `0x00458E60` (AddRef-new, store at `+0x28`, Release-old) and `GetDefaultExemplar`
   `0x0060CE80` (`MOV EAX, [ECX+4]`). The plain prop never sets `+0x28`; `LoadPropProperties` QIs `0x0AC2B5F7` first
   and falls back to the property holder — the fallback building-development.md §8 (LG27) needed for occupants without
   this sub-object.
3. **PO3 — seam specified; `CreateGeometry` DIFFERS.** Occupant `+0x30` holds one `cSC43DPlaceableObject`
   (vtable `0x00AA4468`, 17 slots, `0x30` bytes, QI answers only `1` and `0xC6BB5EFA`). Occupant slots 18/19/20 are
   `GetPlaceableObject` (folded `MOV EAX, [ECX+0x30]` getter), `GetOrCreatePlaceableObject` `0x005E91C0` (lazily creates
   through pool `0x00B0A8CC`, constructor `0x005EC370`, `0x30`-byte node) and `SetPlaceableObject` `0x0078DAF0`
   (AddRef-new/Release-old). Consumers measured: `cSC4ModelMaker::AddModels` `0x00494390` calls slot 19 at `0x0049440E`;
   `cSC4View3DWin::AddAutomataToItemList` (`0x007ECBD1`) and `AddBuildingsToItemList` (`0x007F0588`) call it and then
   `SetPlaceableObject` the result. `CreateGeometry` is a **network-occupant** slot (network-geometry.md §86), not part
   of this seam — `cSC43DPlaceableObject` has no such slot.
4. **PO5 — MATCHES the [inferred] guess: a prop's y is never re-solved.** The box is written exactly twice, both at
   creation: `SetBoundingBox` (slot 4) only from `LoadPropProperties`'s tail (`0x004A2BBE`, `call [edx+0x10]`, with the
   two float[3] arrays built at `0x004A2B80`–`0x004A2BB8`), and `SetPosition` (occupant slot 9) only from
   `AddPropWithUsage` (`0x004A2046`, `call [edx+0x24]`) with a caller-supplied y (the lot altitude at creation). The
   terraforming brush (`ChangeTerrainLevel` `0x00746440`) ends in `RedisplayTerrain` `0x00749850`, which posts only view
   messages (`0x278E6003`/`0x678E6E3A`/`0x870BC918`); the model maker's terrain-hiding pass (`0x00492440` /
   `0x00491CD0`) writes per-cell view flags through city slots `0x144`/`0x148`, never an occupant box. The
   re-solve/destroy rule stays network-occupant-only.
5. **PO6 — MATCHES the behaviour, DIFFERS from the ABI names.** The SDK header
   (`engine/third_party/gzcom-dll/gzcom-dll/include/cISC4PropOccupant.h`) names slot 5
   **`GetPropPlacementProperties`** and slot 6 **`SetPropPlacementProperties(cSC4PropPlacementProperties*)`**. The
   measured channels stand (`33 C0 C3` in EAX; `ret 4`). The folded ABI names are decompiler guesses. The real
   `cSC4PropPlacementProperties` is a `0xB8`-byte `cGZPersistResourceFactory` subclass (vtable `0x00AA44C4`) created by
   its factory `cSC4PropPlacementPropertiesFactory::CreateInstance` `0x005EDE90` (registered at `0x00AA44E8`); the plain
   prop stores none — §4.1b records why the stub pair is correct for this class.

## Notes for the next agent

- `cSC43DPlaceableObject`'s full slot table and layout are now in §6.1 (vtable `0x00AA4468`; `AddModel` grows the
  instance array through the global heap; `RemoveAll` frees the quad-record vector of `0x54`-byte `cQuadRec`s holding
  `cS3DTextureBinding*`s; flags at `+0x2C`: bit 0 overlays visible, bit 1 in renderer, bits 2–6 zoom levels, bits 7–10
  rotations). If a rendering lane needs `AddModelsToRenderer`/`RemoveModelsFromRenderer` behaviour, the renderer global
  is `0x00B43DD0` (vtable `+0x70` add, `+0x74` remove) — same global dispatch.md and INVENTORY-M6.md already name.
- The automata controller's vtable set is now known (`0x00AA4D28` primary at `+0x00` plus five sub-object vtables
  listed in §1.2); its own spec (lot-records.md §16 Q6) still does not exist.
- PO4's leftover (the 0x28-byte sub-record at `+0x54` driven by slots 15/16 through manager `0x00B43D5C`) remains the
  only open prop-occupant question (next free ID PO7).
