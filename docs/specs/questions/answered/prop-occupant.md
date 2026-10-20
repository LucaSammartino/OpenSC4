# prop-occupant: answered questions (archive)
<!-- Archive for [prop-occupant.md](../../prop-occupant.md). Lanes do not read this file. Open questions are in ../prop-occupant.md. -->

<!-- from: re-answers27 (2026-10-16), filed into the spec's own §10; the entries moved here by re-answers32 (2026-10-20) -->
- ~~**PO1b.** Which class the fifth allocator group at `0x00AA4960` serves (stride `0x88`, §4.2). None of the four
  prop classes is `0x88` bytes.~~ **ANSWERED 2026-10-20 (re-answers32): DIFFERS from its premise.** The premise
  said "none of the four prop classes is `0x88` bytes" — true for the four, but the group serves the **fifth**
  prop subclass, `cSC4PropOccupantAutomataController` (size `0x88`; `prop-occupant.md` §1.2/§4.2). Its
  `operator new` `0x005EDD80`, GZCOM creator `0x005F0490` and destructor `0x005F04D0` all use the group-4 pool
  global `0x00B0A914`; `GetGZCLSID` `0x005EFE40` returns `0x2A4017A0`; the constructor `0x005EFDB0` writes up to
  `+0x84`, matching the stride. The "pool/factory method table" at `0x00AA4960` is in fact five pool-object
  vtables (`0x00AA4960`/`0xAA4970`/`0xAA4980`/`0xAA4990`/`0xAA49A0`), each owned by one process-init
  constructor (`0x00A736E0`/`0x00A73740`/`0x00A73780`/`0x00A737C0`/`0x00A73800`) into globals
  `0x00B0A8E4`/`0x00B0A8F0`/`0x00B0A8FC`/`0x00B0A908`/`0x00B0A914`; node counts 78/102/110/75/120 (§4.2).
- ~~**PO2.** The identity of the sixth sub-object at `+0x24` (vtable `0x00AA47D0`, §2.4).~~
  **ANSWERED 2026-10-20 (re-answers32): MATCHES the [inferred] guess.** It is **`cISCExemplarPropertyHolder`**
  (IID `0x0AC2B5F7`, the SDK's `GZIID_cISCExemplarPropertyHolder`). Proof: the QI chain
  (`cSC4PropOccupant::QueryInterface` `0x005EC960` → `cSC4Occupant::QueryInterface` `0x005E89C0` at `+8` →
  `cSCExemplarPropertyHolder::QueryInterface` `0x005FD310` answers `0x0AC2B5F7` with `LEA EAX,[ECX+0x14]` from
  `ECX = obj+0x10`, i.e. exactly `obj+0x24`), and the thunk pair `0x005EE140`/`0x0064DE00`/`0x0064DE10`
  (all `SUB ECX, 0x24` → the primary QI/AddRef/Release). Its two real slots are `SetDefaultExemplar`
  `0x00458E60` (AddRef-new, store at sub+0x04 = obj+0x28, Release-old) and `GetDefaultExemplar` `0x0060CE80`
  (`MOV EAX,[ECX+4]`). The sub-object is the second interface **of the property-holder sub-object at `+0x10`**:
  the holder base constructor `FUN_005FF1C0` installs its vptr at holder+0x14 = obj+0x24 and zeroes the exemplar
  pointer at holder+0x18 = obj+0x28. The plain prop never sets `+0x28` (the exemplar attaches through the
  holder's property data), which is why `LoadPropProperties` (`0x004A2A9E`) QIs the holder for `0x0AC2B5F7`
  first and falls back to the prop's type ID as the catalogue key when the QI fails — the fallback
  building-development.md §8 (LG27) described. Rule written into `prop-occupant.md` §2.4.
- ~~**PO3.** The occupant → placeable → 3D object seam (§6.1): occupant slots 20/21 and `CreateGeometry`.~~
  **ANSWERED 2026-10-20 (re-answers32): the seam is specified; the `CreateGeometry` half of the premise
  DIFFERS.** Occupant `+0x30` holds one `cSC43DPlaceableObject` (vtable `0x00AA4468`, 17 slots, `0x30` bytes,
  QI answers only `1` and `0xC6BB5EFA`); occupant slots 18/19/20 are `GetPlaceableObject` (folded getter
  `MOV EAX,[ECX+0x30]`), `GetOrCreatePlaceableObject` `0x005E91C0` (lazily creates through pool `0x00B0A8CC`,
  constructor `0x005EC370`) and `SetPlaceableObject` `0x0078DAF0` (AddRef-new/Release-old swap at `+0x30`).
  The model maker's `AddModels` `0x00494390` calls slot 19 (`0x0049440E`, `call [eax+0x50]`); the view's item
  lists call it too and then `SetPlaceableObject` the result (`0x007ECBD1`, `0x007F0588`). **`CreateGeometry`
  is not part of this seam** — it is a network-occupant slot (network-geometry.md §86); the placeable object
  has no such slot. Rule written into `prop-occupant.md` §6.1.
- ~~**PO5.** Whether a prop's box y is ever re-solved against terrain after placement (§5.2) *[inferred]*.~~
  **ANSWERED 2026-10-20 (re-answers32): MATCHES the [inferred] guess — it is not.** The box is written exactly
  twice, both at creation: `SetBoundingBox` from `LoadPropProperties`'s tail (`0x004A2BBE`, `call [edx+0x10]`,
  arrays built at `0x004A2B80`–`0x004A2BB8`) and `SetPosition` from `AddPropWithUsage` (`0x004A2046`,
  `call [edx+0x24]`) with a caller-supplied y (from the lot altitude at creation, lot-developer.md §6.5 /
  network-manager.md §8.6). The terraforming brush (`ChangeTerrainLevel` `0x00746440`) ends in
  `RedisplayTerrain` `0x00749850`, which posts only view messages (`0x278E6003`/`0x678E6E3A`/`0x870BC918`);
  the model maker's terrain-hiding pass (`0x00492440`/`0x00491CD0`) writes per-cell view flags through city
  slots `0x144`/`0x148`, never an occupant box. The re-solve/destroy rule stays network-occupant-only
  (network-manager.md). Rule written into `prop-occupant.md` §5.2.
- ~~**PO6.** The SDK names of `cISC4PropOccupant` slots 5 and 6 (§4.1). Their **behaviour is settled** — slot 5
  is a stub returning 0 in EAX (`xor eax,eax; ret`), slot 6 a true no-op (`ret 4`) — but the ABI rows carry
  identical-code-folded names from unrelated classes and nothing indicates the SDK's names.~~
  **ANSWERED 2026-10-20 (re-answers32): MATCHES the behaviour, DIFFERS from the ABI rows' names.** The public
  SDK header (`gzcom-dll/gzcom-dll/include/cISC4PropOccupant.h`) names them **`GetPropPlacementProperties`**
  (slot 5) and **`SetPropPlacementProperties(cSC4PropPlacementProperties*)`** (slot 6). The measured channels
  stand (EAX pointer-or-null; one popped pointer). The ABI rows' folded names
  (`icf::cSC43DRender::GetProjectionMatrixEntries`, `cSC4NetworkTool::DebugDumpEdgeTypes`) are decompiler
  guesses; the pending rows in `build-runs/pending-abi/re-answers27-rows.tsv` should carry the SDK names. The
  real `cSC4PropPlacementProperties` objects are `0xB8`-byte `cGZPersistResourceFactory` subclasses created by
  the factory `0x005EDE90` (vtable `0x00AA44C4`); the plain prop stores none (§4.1b).
