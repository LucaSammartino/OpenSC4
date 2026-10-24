# lot-records: answered questions (archive)

<!-- from: glue-simgrid-lots-shadow5 round 2 (2026-10-23); filed in that lane's worktree -->
- ~~**LR12 - sections 1, 3.1, 14: lotrec_shadow observation inventory:** please define the exhaustive
  record-only load/month shadow scope and discovery contract. Is the denominator the lot manager's
  master-list cSC4Lot records (27 fields, including inline trips/properties), or every record class in
  section 1, including business units, construction, signposts, decals, wild flora and the crime-only
  lot mentioned in section 14? The master lot list alone cannot establish the latter inventory;
  section 3.1 explicitly says the lot does not save its occupants. For the intended scope, specify
  public owner/getter or record enumeration, exact native signatures/reference ownership, treatment
  of objects absent from the master list, and required outcomes for unsupported/unspecified prop
  subclasses. [lotrec_shadow deferred; implement and land the independently specified SG3
  simgrid_shadow first, without claiming lot-record coverage]~~
  **ANSWERED 2026-10-23 (re-lotrec-lr12):** MATCHES — §14.1 chooses the complete current master-list
  cSC4Lot payload scope, not every §1 class; §17.6 T58–T65 defines inventory, nested-field, reference
  and failure discriminators. City slot 33 borrows the manager; manager slots 15/16 supply registered
  iterator/count, enumeration borrows lots, QI acquires Serializable and Write returns AL. Iterator
  cleanup is nonvirtual unregister/C-free. Off-list lots, units and occupants are excluded as payloads;
  references retain identity without recursion. Unspecified prop subclasses are outside scope, not
  silently successful. An all-class coverage claim remains unsupported by this named shadow.

<!-- from: glue-lotrecords-bind4 (2026-10-22) -->
- **LR10 - sections 3.2/13.2, concrete lot slot 55:** what is the complete mutation contract of
  `cISC4Lot::SetPopulation(devType, population)` (`0x006C1940`, pop 8)? The save record specifies a vector
  of population maps, not which map(s) this setter selects/creates, whether it clamps to capacity, what
  zero/absent/duplicate developer-type entries do, or which notifications/caches it updates. Native tract
  callers invoke slot 55 (tract-developer.md section 4, population adjustment), and lot-developer.md
  section 6.4 ReoccupyBuilding passes its AL through. The ABI row says `eax-or-void`, not a defined
  boolean contract. Please specify mutations, observable calls/order and return definedness with vectors
  for empty maps, the same developer type in two maps, and setting an existing population to zero.
  [concrete binding withheld; do not infer a BTreeMap insert, successful bool, or call the native method
  on a foreign object whose full native layout/construction contract is not published]
  **ANSWERED 2026-10-22 (re-lotrecords-mutations4):** DIFFERS — §3.2.1/13.2 and T31–T36/T51:
  clamp unsigned u16 to slot-52 AX first; choose the first profile capacity-map containing devType,
  assign/insert only in the corresponding existing population map, preserve zero entries, then update
  wealth (which can insert other zero keys). No map append or message. Explicit AL=0/1 proves the
  boolean result; full EAX is not defined. Unknown native profiles are default-created with empty capacity
  vectors. Invalid population-vector shapes remain unchecked native invariants, not successful cases.

<!-- from: glue-lotrecords-bind4 (2026-10-22) -->
- **LR11 - sections 3.2/13.2, concrete lot slot 34:** please publish the complete `SetState` owner
  behavior needed for a replacement lot, including `UpdatePrimaryOccupantWealth` (`0x006C1430`).
  Lot-developer.md section 3.2 specifies message data4 and expressly leaves that wealth computation
  unspecified; the lot-records slot-34 row specifies pop 12 and no defined return, while the record
  describes saved fields. Required discriminators: no building type, unknown profile, entering/leaving
  constructing/occupied with wealth 0 versus nonzero, population/capacity-container updates and resource
  releases, state-day update, and queued copy lifetime. Which operations occur before the queued copy
  is taken? Native SetState uses a native lot copy constructor (`0x006C2450`) with embedded containers;
  forwarding to it is not a supported operation on a portable record or arbitrary replacement layout.
  [withhold complete-owner activation rather than synthesize successful transitions or manufacture a
  native-layout facade; the LR8/LR9 construction and view answers remain accepted]
  **ANSWERED 2026-10-22 (re-lotrecords-mutations4):** DIFFERS — §3.2.2/13.2, T37–T50/T52–T54,
  and lot-developer.md §3.2 specify state/day, resource release, population rebuild/zeroing and nested
  setter order, occupancy arithmetic/insertion, wealth fallbacks/ties, power cache and snapshot lifetime.
  No building clears wealth; a nonzero unknown profile leaves it unchanged. Message leaving-wealth
  is sampled just before the outer recomputation, after any nested fill. Snapshot follows power update;
  maps copy independently, property objects/configuration are retained, unit/travel references and trips
  are empty. Queue retains message, message retains copy; final destruction balances acquired refs.
  No SetState boolean is defined; no native-layout forwarding contract is inferred. Empty capacity maps
  inside a nonempty vector fault at integer division; invalid missing population maps have no defined
  native container result. Neither is repaired into success. LR8/LR9 remain unchanged.
<!-- Archive for [lot-records.md](../../lot-records.md). Lanes do not read this file. Open questions are in ../lot-records.md. -->

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR1 - section 3.5 Init step 1:** which point does `GetCenter` (lot slot 15) return for a lot rect (cell centre
  of the rect in metres?), and which slope does `SetAltitude` (slot 17) store in fields 7 and 8 ("also sets both
  slopes")? [both come from the host: `LotHost::center_altitude_and_slope(rect)`]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** MATCHES (with exact rules) — section 3.5 Init step 1: GetCenter (0x6BF560) = ((16*x1 + 16*(x2+1)) * 0.5, altitude, (16*z1 + 16*(z2+1)) * 0.5) in metres; Init passes (x, z) to terrain 0x00B43CF4 slot 13 GetAltitude; SetAltitude (0x6BF5C0) acts only when |a - altitude| > double 0.001, then stores a and writes GetLotSlope(lot, false) (BDS slot 36 -> lot developer slot 30 = GetAreaSlope(rect, false)) into both field 7 (+0x2C) and field 8 (+0x28).

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR2 - section 3.5 SetFlags:** in which order are the rect cells locked / unlocked (x outer or z outer), and is
  the lock call the terrain view's per-cell lock with the lot altitude as given? Observable only through the terrain
  view's call order. [x outer, z inner, x1..=x2, z1..=z2]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** MATCHES — section 3.5 SetFlags (0x6BF220): flags stored first; on a bit-0 change with a non-null SL::TerrainView: x outer x1..=x2, z inner z1..=z2, ascending; set -> view slot 16 LevelCell(x, z, altitude as stored, re-read per cell), clear -> view slot 17 RemoveCellLevelling(x, z).

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR3 - section 4 / lot-developer.md section 5.7 Remove:** what does `cISC4BusinessUnit::Remove(lot)` return in
  AL, and does it still call `lot.SetUnit(0)` / `SetTravelDesignate(0)` when the lot is not a member? [true when
  the lot was a member; both setters always]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** DIFFERS — section 4 membership slots: Remove (0x6C08B0) has no defined return (AL = low byte of SetTravelDesignate's EAX: 0 without a designate, else the old designate's Release result); it erases every matching node and always calls lot.SetUnit(0) then lot.SetTravelDesignate(0), member or not. Callers ignore AL.

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR4 - section 3.2 field 2:** a lot that was read but never `Init`ed (the crime-only lot of section 14) holds no
  configuration object, only the ID read. Does its `Write` save 0 (no object) or the ID read? [the ID read]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** DIFFERS — sections 3.2 field 2 and 3.5: Write (0x6C0080) writes only the configuration object's GetID or 0 and never the ID read (+0x30), so a read-but-never-Inited lot saves 0.

<!-- from: core-lotrecords4 (2026-10-21) -->
- **LR5 - section 7 / lot-developer.md section 7.2 AddRetainingWallOccupant:** how does the add path compute the
  wall type, edge mask, the four edge hashes, position and size of the new wall from the lot, and what does
  `UpdateRetainingWalls(lot)` change on an existing wall? Without it a core can read and write walls but not build
  them. [not implemented; record Write/Read and the T7 mask growth only]
  **ANSWERED 2026-10-21 (re-lot-record-answers4):** MATCHES (add path) / DIFFERS (update) — sections 7.1 and 7.2: the add path is AddRetainingWallOccupant 0x005E3070 (edge hashes from GetLotWallEdgeFlags 0x005E2CE0, mask from non-zero hashes, rect-centre position, size (16w, top - a, 16d), AL always 0). UpdateRetainingWalls 0x005E37F0: flags bit 0 clear -> nothing; no wall at the location cell or QI 0x295D1162 fails -> AddRetainingWallOccupant(lot, true); else recompute hashes and, per edge i = 0..3 whose hash changed, remove model i (old != 0) then add model i (new != 0, inserted in the renderer), then SetEdgeHashes(new) once if any changed. Position, size, type, lot flags are never changed and an all-zero wall is kept; the mask changes only in AddRetainingWallModel 0x005E3400 (bit set when the model builds, cleared when it fails, untouched on removal). UpdateSurroundingRetainingWalls 0x005E3960 runs UpdateRetainingWalls on every lot of GetLotsSurroundingLot(lot, 0xF) in ring order, not on the lot itself.

<!-- from: glue-lotrecords4 (2026-10-22) -->
- ~~**LR8 - sections 3.1, 4, 7.1/7.2 and 15, complete-owner runtime interception boundary:** what published
  registration or interception contract lets an independently loaded plugin replace these three owners while
  retaining the native zone developer and lot developer? The class-object store uses GZCOM on load
  (foundations sections 2.7/3.6), but new lots are constructed directly by the zone developer
  (`zones.md` section 12, `0x006BF430` + `0x006C1E50`); `PickUnitToJoin` constructs business units directly,
  without the COM directory (`lot-developer.md` section 5.7, constructor `0x006C0800`); and native
  `AddLotFoundation`/`ClearLot` call the nonvirtual retaining-wall helpers (sections 7.1/7.2/15,
  `0x005E3070`, `0x005E37F0`, `0x005E3960`), including direct wall construction (`0x005E2F40`).
  Please specify the supported redirection boundary for those creation/helper paths, its calling convention,
  installation/restoration lifetime and ownership obligations, or explicitly require replacing the calling
  developers together. Factory registration and method addresses alone do not specify an interception contract.
  [blocked before runtime wiring: no `enable=lotrecords` registration, no partial factory or same-owner forwarding;
  existing Rust owner implementations unchanged; LR6/LR7 remain open]~~
  **ANSWERED 2026-10-22 (re-lotrecords-boundary4):** MATCHES — section 15.1 proves the direct constructor/helper edges, stack ABI, storage and reference transfers. Factory registration covers load only. With no established direct-edge installation/restoration contract, complete-owner replacement must include the zone developer/lot manager for lots and the lot developer for units/wall helpers. No independent native-developer interception contract is invented. Sections 4/7 link the boundary; T28-T30 distinguish factory-only substitution. LR6/LR7 remain open.

<!-- from: glue-lotrecords4 (2026-10-22) -->
- ~~**LR9 - section 3.1 versus published cISC4Lot slot 4, property-holder view:** section 3.1 places the
  plain property-holder view at lot +8 and the traffic-source view at +0x1C, but `docs/abi/interfaces.tsv`
  (`cISC4Lot`, vtable `00ab19b0`, slot 4, target `006c2050`) describes `AsISCPropertyHolder` as returning
  this +0x1C. Which offset and view does that slot actually return, and are the section 3.1 QI view offsets
  correct? [no runtime view/cast implemented; preserve both published sources until reconciled]~~
  **ANSWERED 2026-10-22 (re-lotrecords-boundary4):** DIFFERS — section 3.1 offsets match native bytes; slot 3 is the borrowed property holder (+8), slot 4 the borrowed traffic source (+0x1C). The old ABI method label is wrong. Constructor writes, exact QI/adjustor bytes and native TrafficSourceIterator caller 0x00710841 prove the distinction. QI accepts only IID 1, lot and Serializable. Sections 3.1/13.2 and T26-T27 specify it; four exact ABI row corrections are proposed under build-runs/pending-abi, awaiting coordinator review/application/gate.

<!-- from: glue-automata-ctor7 (2026-10-24); recovered from the ready lane worktree before its merge -->
- ~~**LR13 - section 9 construction references, native writer versus reader:** In the train180
  house-shadow run at 2028-06-01, a construction occupant captured 254 bytes but replay wrote
  250, first difference at byte 142, consuming all input. Immediately afterward destruction
  of the retained reference table released the last reference to a bare cSC4Occupant
  (class 0xA9BC9AB6, serializable vtable 0x00AA41BC). Please specify the full writer-facing
  pointer/interface and QI ownership contracts for fields 10-13 and 15, especially field 13's
  pointer+0x10 view: does QI(cIGZSerializable) always return the derived effect record, or can
  it return an embedded bare occupant which does not answer the reader IID? Does any Write
  create/release temporary occupants or mutate reference identity? What is the required
  same-input comparison outcome when the saved reference does not answer its reader IID?
  Need the documented native identity/ownership contract rather than coercing a reference
  to a guessed effect interface or preserving a rejected non-null token. [keep the existing
  literal IID-checked codec and owners shadow default-off; add bounded replay diagnostics
  and explicit acquisition accounting, then measure the remaining shadows]~~
  **ANSWERED 2026-10-24 (re-lotrec-lr13):** MATCHES — §9.1.1 and T66–T73 specify native views and balanced QI lifetimes. Field 12 uses effect O+4; field 13 uses O+0x10; both query to derived effect Serializable O+0x10, not a separate bare occupant. Read owns successful literal-IID QI results and releases temporary Serializables. Rejected nonnull references become null; construction Read ignores helper failure and can return true with no stream error. This is a comparison mismatch, never an exact round trip. Construction Write does not replace occupants; effect Write's temporary is an empty string. Static facts do not diagnose the historical size loss or require it.

<!-- from: glue-construction-lr13 real-city run construction-lr13-r1-house-2032-20261024b (2026-10-24) -->
- ~~**LR14 - section 9.1.1, intermittent native field-13 bare Serializable and last-reference lifetime:** the final LR13 implementation retains the literal field QI, releases the reader's temporary Serializable on success/rejection, and keeps an independent observer pin. Native T66-T73 pass, including exact helper acquisition counts after RefAudit cache warmup. Yet the house shadows profile in slot 2 captured construction field 13 at offset 158 as ID 3, class `0xA9BC9AB6`, on 2027-01-01: effect IID `0x2977AA49` was rejected, and strict replay consumed all input but reported `254/250` bytes. The capture pin's release then reached zero at `WriteCapture::ClearReferences:79` (bare Serializable vtable reported as game+0x6A41BC). A foreign write access violation followed on 2027-01-09; the run ended corrupt with foreign_exceptions=1/ref_warnings=1. The preceding build's house run reached 2032 clean with 182/182 construction records exact. Please specify how this observed bare callback arises despite the live-effect derived-Serializable contract, its actual ownership before/after writer slot 26, and any distinction between writing a valid live effect and this rejected input. What additional safe observation distinguishes a native temporary from a stale or misidentified field without guessing interfaces? [Literal LR13: reject to null, count mismatch, release only our acquisitions; do not coerce, normalize four bytes, suppress the warning, or leak the pin. Shadow remains default-off. Evidence: build-runs/game-results/construction-lr13-r1-house-2032-20261024b.log lines 8269-8272 and 8652 onward; normal-speed comparison requested separately.]

  LR14 follow-up (2026-10-24): the same final production DLL (SHA256 prefix f7fe29c4) completed the normal-speed house profile to 2032-01-01 cleanly in construction-lr13-r1-house-2032-20261024c-normal (run 471): automataowners 72/72 dated, 73/73 including load, construction 169/169 exact, foreign_exceptions=0/ref_warnings=0. The failed turbo observation remains unresolved; this is not evidence that the bare identity or its release is required behavior.~~
  **ANSWERED 2026-10-24 (re-lotrec-lr14):** MATCHES — §9.1.2 specifies the native teardown route: Shutdown releases field +0x13C without clearing it; effect destruction installs bare Serializable vtable AA41BC at O+0x10. This is stale teardown storage, not a temporary or valid live effect. §9.1.1's live-effect/rejected-input contracts remain literal. Callback QI is borrowed; retained observer pins are separate acquisitions, balanced independently. §9.1.2 defines passive instance/view/Shutdown/destructor tracing needed to attribute run 470; static evidence does not prove that run's timing or crash cause. §9.3 inventories non-Write methods and observation limits; T74–T79 cover ownership, teardown exclusion, rejection and attribution boundaries.
