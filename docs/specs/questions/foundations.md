# foundations: open questions
<!-- Questions about [foundations.md](../foundations.md). Append new entries at the END; when one is answered, move it to answered/foundations.md. Rules: README.md. -->

<!-- from: From the return-channel audit (2026-10-16) -->
- **HG2:** cISC4App (`vt:cSC4App::vftable_3`) return channels are in no spec: slot 10 RequestQuit (the autotest falls
  back to AutotestFinish when it reads false), slot 13 LoadCity, slots 15/16 SaveCity, slot 21
  GetDebugFunctionalityEnabled (it gates the cheat-code registration in `cFC4Budget.cpp` and `cFC4Simulator.cpp`).
  Does each set AL/EAX, and with what meaning? [SDK bool trusted; LoadCity and SaveCity only logged; a garbage
  RequestQuit true is covered by the 90 s quit grace]

<!-- from: From the return-channel audit (2026-10-16) -->
- **HG3:** framework slots read as bool with no spec: `vt:cGZCOM` slot 3 GetClassObject (every site also tests the out
  pointer), cIGZPersistResourceManager slots 4 GetPrivateResource and 14, cIGZDate slots 4 SetDayNumber and 6
  SetMonthDayYear (passed through by cFC4Date / cFC4Simulator), cIGZVariant slot 54 CreateValString, cIGZWin slot 36
  GetChildAs. Is each a real bool? [SDK bool trusted]

<!-- from: glue-rng4 native RNG contract audit (2026-10-22) -->
- **RN4 - sections 6.1, 6.4 and 8.6, native-original fixture contract:** Can either exact owner,
  `cRZRandom` or `cRZRandomSuper`, be exercised independently inside the supported GOG 1.1.641 process through
  a published SDK/service/factory route or a fully typed supported-image native entrypoint? The SDK declares
  only an opaque `intptr_t cISC4AdvisorSystem::GetRandom()`; it declares neither generator's operations, and
  the reconciled ABI tables/direct-call contracts contain neither exact owner. Section 6 publishes behavior
  and some addresses, but not a complete calling, object acquisition/construction, storage, lifetime or
  disposal contract. Please publish the permitted route, complete argument/return/calling conventions and
  supported-image validation contract, and how an isolated native instance can be given known seed/clock
  inputs without disturbing the game's streams. Include raw/bounded/signed/double/Gaussian/weighted methods
  where present, zero-bound draw behavior, seed sentinel/zero fallback and cache/reseed behavior. If no such
  route exists, explicitly state that native sequence verification is blocked. [Do not call behavior-spec
  addresses, dereference the opaque advisor handle, invent object layouts, or count Rust/spec/SDK-generated
  sequences as an independent game-native reference. Native-original versus Rust remains unverified.]

<!-- from: glue-rng4 native RNG contract audit (2026-10-22) -->
- **RN5 - sections 6.2-6.4, real native insertion contract:** Is there a supported published way to install
  an opt-in Rust-backed replacement for either exact owner `cRZRandom` or `cRZRandomSuper` so actual game RNG
  calls reach it? Please identify the SDK service/factory registration and acquisition path, or publish a
  complete typed native insertion contract, including caller coverage, ownership/lifetime, installation and
  removal timing, and how independent member/temporary and process-wide streams retain their seed ownership
  and draw order across city loads. The current SDK, reconciled ABI and foundations random/seeding sections
  expose no such route; a list of native static addresses and the opaque advisor `GetRandom()` return are not
  an insertion contract. If neither owner is replaceable through a supported route, please say so explicitly.
  [No `rng` enable key, invented registration, vtable/constructor hook, private-field patch or native-operation
  forwarding replacement is installed. Existing defaults, maps/tools and RNG-owning subsystem behavior stay
  unchanged; same-name simulation/renderer generators are outside this request.]

<!-- from: verify-variants4 bounded native variant verification (2026-10-22) -->
- **HG4 - sections 9.2, 10.1, 17.1, independent native serialization fixture:** What published SDK acquisition,
  ownership and disposal route supplies an isolated ORIGINAL segment input/output stream capable of executing
  its own ReadVariant/WriteVariant against caller-owned bytes? A property Write into a plugin segment stream
  only calls that plugin's WriteVariant, so comparing its Rust serialization with Rust is not native
  verification. Include error-state access, reset/position/lifetime and null-object read-into behavior.
  [Native serialization/read-into not exercised; Rust serialization is used only to expose the independently
  populated Rust Variant's typed storage. No native serialization equivalence or save-payload claim.]

<!-- from: verify-variants4 bounded native variant verification (2026-10-22) -->
- **HG5 - sections 9.1 and 17.1, variant behavior beyond typed storage:** Please publish the behavioral
  contract for cross-type GetVal/As/CanGetVal, string create/get/set, reference-alias operations, and object/
  pointer acquisition/retention/release, including empty/invalid values and failure output preservation.
  The slot table proves call ABI but not these conversions or lifetimes. Confirm CreateCopy hands back one
  owned reference and that numeric SetRefX copies input storage. [Literal published SDK reading for owned
  numeric clones and copying setters only; two scalar values and 0/1/3-element arrays per numeric/character
  type, clone isolation, CopyFrom and Erase are bounded experiments. No whole 127-method owner equivalence;
  unsupported conversions, string operations, pointers, objects and reference aliasing are excluded.]
  **Partial publication 2026-10-23 (re-variant-storage4):** [variant-storage](../variant-storage.md)
  now specifies copying Ref setters, mutable numeric/character aliases (including the AsVoidPtr
  exception), replacement, CopyFrom/self-copy, bounded string operations and object/pointer ref order.
  CreateCopy owns one reference; HG6's clone identity/AL channel is answered separately. This is static
  evidence, not a verifier promotion. HG5 remains open for general RZString growth/overlap/failure,
  foreign interface behavior, allocation-failure domains and broader invalid-value qualification;
  existing data-formats §5.1 conversion rules are inputs, not new research.

<!-- from: verify-variants4 independent owned native variants (2026-10-22) -->
- **HG7 - section 9.1 scalar GetCount:** What does GetCount return on a scalar, including after typed
  setters and native CreateCopy/CopyFrom? Rust Variant::count returns 1 for scalars. Original run
  vv4-original2-7c5ca4-1810 executed 70 literal cases/420 checks, 392 exact and 28 scalar-shape differences,
  with 71 successful native owned clones, no refusals and clean2032/ref_warnings=0. Every typed getter,
  array count/data, copy isolation, CopyFrom state and Erase state passed. Please specify the native scalar
  metadata contract separately from the one serialized scalar element. [Keep literal Rust scalar count=1;
  preserve the 28 differences as failures rather than adjusting the oracle to native results. A normal-speed
  third original run with explicit scalar metadata and a current40 control are planned to isolate pacing.]

  **HG7 measurement update:** vv4-normal3-e78179-1815, normal speed through 2032-01-01, clean/ref0,
  reproduces 70 cases/420 checks/392 exact/28 HG7 differences/0 unexplained/0 refusals/71 clones.
  Every tested native scalar type 0x01 through 0x0E reports its matching type and GetCount=0 (Rust count=1).
  This is not a turbo-only difference. The full scripted save succeeds. No core semantics were changed.

  **HG7 published-contract correction (round3, 2026-10-22):** This question is already answered by
  data-formats §5.2 (scalar GetCount=0, array element count), its text writer discussion
  (fresh SetValUint32 count0), §10.5 item42 (scalar7 count0, array-one count1), and foundations §4.1
  (scalar variant count0). The previous instruction to retain logical count1 as the native metadata
  oracle was incorrect. Keep the old 28 failures as historical measurements; preserve logical storage
  count1 for existing consumers, and compare the actual production native GetCount path instead.
  HG4-HG6 remain unresolved; this correction establishes no conversion/serialization/return-channel contract.

<!-- from: glue-city-shadow9 (2026-10-24) -->
- **HG8 - section 3.6 city owner and persistence:** Please publish the complete cSC4City scalar payload
  (versions 8..12), field update rules and public getter/setter contracts, including a read-only capture
  route for the city serializable interface that cannot write other records. Section 3.6 explicitly defers
  the other city fields; save-records section 2 says the city-written auxiliary record is unspecified.
  The brief names core/src/save.rs as a city implementation but it implements the simulator v4 record.
  [No city persistence oracle or whole-owner credit. Measure only PositionToCell on eight fixed caller inputs
  per load/month (budget section 17 item 6: floor, 16 m origin) and CellWidthX/Z (terrain section 2.1).
  All other cISC4City methods, serialized fields, 53 component references and auxiliary records remain
  uncompared. No native output is used to populate independent expected values; no Write or mutators called.]
