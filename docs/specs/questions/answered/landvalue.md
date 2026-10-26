# landvalue: answered questions (archive)
<!-- Archive for [landvalue.md](../../landvalue.md). Lanes do not read this file. Open questions are in ../landvalue.md. -->

<!-- from: glue-landvalue-capture7 (2026-10-24) -->
- ~~**LV9 - host provision needed after LV8 native closure:** Section 6.1.4 explicitly requires an identified host admission/exclusion mechanism and supplies no existing game installation API. The plugin currently has lifecycle notifications, QI and post-pass observers, which LV8 rules out as exclusion proof. Please identify or coordinate a host-side provision that covers pre-fetch admission on every affected thread, both city startup paths and input lifetime through Shutdown, or confirm that native capture remains ineligible until such a host exists. This is an integration dependency, not a request to re-infer native fields/signatures or reinterpret lifecycle ordering. Vectors 26-31 now have a deterministic reference-host fixture, but it does not install that host in the game. Native capture remains disabled; LV2/LV7 remain open and sampled output is measured separately. [No supported host mechanism identified; input_capture=pass-end-sample; native_step_capture=0.]~~
  **ANSWERED 2026-10-26 (re-lv28):** MATCHES — §6.1.5 confirms native capture
  remains ineligible without an independently supplied host satisfying §6.1.4. Actual
  SimIdle/SimNewDay deliveries are the observation boundary; safe route publication and
  retirement additionally require pre-fetch admission/exclusion on every affected route
  and thread, both startup paths, and owner-input validity through the drained Shutdown
  boundary. Calendar/idle destination fetches and message-server retained dispatch are
  identified; they expose no native exclusion service. Reference-host fixtures do not
  install one. Keep pass-end-sample/native_step_capture=0; LV2/LV7 remain open.

<!-- from: glue-landvalue-step6 (2026-10-24); copied from that lane before train178 -->
- ~~**LV8 - adapter installation/exclusion contract after LV6 layout closure:** Sections 6.1.2 and 6.1.3 explicitly retain unproved observer-registration, executable-mutation and callback-exclusion obligations. Which supported host lifecycle boundary guarantees no OnTick/notification/in-flight entry while a global native M slot or entry interceptor is installed/restored, covers both loaded and newly constructed owners before the first initialized delivery, and retires the forwarder safely? The published field/forwarding ABI does not establish that boundary, and a QI proxy/re-registration is explicitly rejected. Please reconcile the host acceptance checklist or publish the missing dispatch contract; no detour bytes, native signatures, mutation barrier or installation API are inferred. This lane implements/tests owned progress interval decoding and retains `input_capture=pass-end-sample; native_step_capture=0` until installation can satisfy that checklist. LV2/LV7 remain open; exact sampled comparisons will not be credited as per-Step provenance.~~
  **ANSWERED 2026-10-24 (re-landvalue-lv8):** DIFFERS — no native city lifecycle
  notification supplies the proposed exclusion/drain guarantee. Pre-init/pre-shutdown
  order Init/Shutdown; ordinary pause permits idle, HiddenPause does not wait for existing
  calls, and message-server targets copied before unsubscription remain callable. Section
  6.1.4 reconciles the checklist with an explicit host dispatch contract covering admission,
  drain, publication, both startup paths, owner-read lifetime, restoration ownership and
  retirement of reachable code/context. A persistent global route and per-city capture
  have different lifetimes. Conformance vectors 26–31 cover these distinctions. This
  closes the native-contract question, not host implementation or runtime acceptance;
  keep native_step_capture=0 until demonstrated. LV2/LV7 remain open. No additional native
  field, signature, installation API, executable patch or C ABI export is established.

<!-- from: verify-landvalue-step4 (2026-10-22) -->
- ~~**LV6 - section 6.1 native Step capture ABI record:** please record the already specified native progress layouts in `docs/abi/layouts.md`: the land-value interface-relative pipeline state, both iterator pointers, ignoreZoning, developer cursor/header, iterator fields and developer map node fields, including their widths and the canonical interface base. The message-target slot 3 / `DoMessage` ABI is present in `interfaces.tsv` and section 11.3, but these field layouts are absent from `docs/abi`. CONTRIBUTING requires game layout dependencies to be recorded there before engine glue uses them. Is a version-gated, QueryInterface-obtained message-target slot-3 forwarder, with progress read through the separately obtained land-value interface before/after each native SimIdle/SimNewDay delivery, the supported boundary? Please document any additional ABI needed for that adapter. [Stop the native-layout/hook path until recorded; retain and explicitly label the existing pass-end sample. No input-boundary fidelity credit, even for zero output differences. No algorithm, math, golden or sampling adjustment.]~~
  **ANSWERED 2026-10-23 (re-landvalue-lv6):** DIFFERS — the layouts are already published in
  `docs/abi/layouts.md` (Land-value native progress, including the ScanningIterator, developer-node
  and message-target tables). No additional field row or C ABI export is needed for these reads.
  Landvalue §6.1.3 records the exact-width lookup and acceptance rule; §16.19–25 specify conformance
  vectors. Before/forward/after at the actual native M delivery with separately held L is the
  observation boundary, but QI alone installs no interceptor. It is not an approved installation
  mechanism. Host exclusion, native target identity/order, first-delivery coverage and safe
  restoration/lifetime remain mandatory adapter acceptance evidence under §6.1.2, not facts
  established by a layout record. Existing pass-end samples retain native_step_capture=0 and
  no input-fidelity credit; LV2/LV7 remain unresolved runtime verification. No algorithm,
  math, golden or sampling changes.

<!-- from: re-landvalue-step-abi4 (2026-10-22), partial static LV6 evidence -->
LV6 static follow-up (2026-10-22, re-landvalue-step-abi4): §6.1.1 and §11.3 now establish
canonical L/M bases, synchronous slot-3 before/forward/after ordering, reference ownership,
iterator semantics and per-developer capture. Exact-width layout proposal:
`build-runs/pending-abi/re-landvalue-step-abi4-layouts.md` (coordinator must review/apply to
`docs/abi/layouts.md` and run the full train gate; no direct ABI edit or gate verdict here).
LV6 remains open: no installation path has been proved that intercepts existing native agent
deliveries while preserving identity/order, shutdown removal, notification lifetime and early
loaded-map capture. QueryInterface alone installs nothing. Until ABI application and that proof,
retain `input_capture=pass-end-sample; native_step_capture=0`; no input-fidelity credit.

<!-- from: re-landvalue-step-abi4 (2026-10-22), round-2 bounded installation proof -->
LV6 installation/startup follow-up: §6.1.2 now proves synchronous pre-init `0x26D31EC0`
before city Init on both NewCity (`0x00454937` / `0x00454941`) and LoadCity
(`0x00455898` / `0x004558A2`), and synchronous pre-shutdown `0x26D31EC2`
(`0x00454ABF`) before city Shutdown (`0x00454AC9`). NewCity's pre-init occurs
**before** creation of the land-value owner; a QI-only object forwarder installed there
does not cover that branch. Native DoMessage needs a six-byte whole-instruction prefix
for a five-byte entry branch; continuation `0x006BF0D6`, with M as this and callee pop 4.
No replacement bytes or installed hook are supplied. The precise remaining question is:
which reconciled observer-registration/host body executes before both pre-init sends,
and what excludes OnTick, notification callbacks and in-flight entries while installing
and restoring (including at pre-shutdown)? For an object-only route, identify the callback
after CreateCityComponents and before land-value's first AddAgent; none is established.
Post-init is queued and its simulator handler HiddenResumes, so it cannot substitute for
that proof merely by name. Layout proposal now includes initialized/refcount widths and
entry-span evidence. LV6 stays open; ABI application/full train gate, real adapter capture,
and LV2/LV7 runtime verification remain prerequisites. No sampling/math/output adjustment.

The two historical follow-ups above are preserved verbatim. Their pending-publication wording
is superseded by this answer; their installation warnings remain acceptance requirements.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV1:** with several growth developers, is the developers' address order the tuning's listing order? [not
  compared]~~ **ANSWERED 2026-10-14** from the exe (tuning reader `0x006BE110`; developer creation `0x006991A0` from
  the resource enumeration callback `0x00699E20`, load factory `0x006B7870`) and written into landvalue.md §7.4.
  **No.** The list is a map keyed by the developer pointer as an unsigned 32-bit value, walked from the smallest:
  ascending pointer order, fixed by the allocator. That matches neither the tuning order nor, in general, the
  creation order. A shadow reproduces it exactly by sorting the pointers `GetDeveloper(ID)` returns.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV2:** a desirability map can change between the original's pass reading it and the comparison. [rerun with the
  previous map and keep the better result]~~ **ANSWERED 2026-10-14** from the exe (growth developer slot 13
  `0x007C9FB0` returns the published grid; state 17 copies the working grid into it in one call; land value state 7
  reads and applies it in one call) and written into landvalue.md §7.4. **The bracket is sound.** The map that state
  7 sees is never half-built and changes at most once before the comparison, so there are exactly two candidates:
  the map before the developer's last state-17 copy, and the live one. The developer's slot 10 tells a shadow when to
  snapshot the first.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV3:** is `GetAreaSlope` on the lot developer free of side effects? [assumed read-only]~~ **ANSWERED 2026-10-14**
  from the exe (`0x006C5DC0`, its vertex reader `0x006C4310`, terrain `0x007413C0`/`0x00744980`) and written into
  landvalue.md §6. **Yes, read-only**: terrain altitude and water level reads, locals only.

<!-- from: From the land value and flammability shadows (2026-10-14) -->
- ~~**LV4:** `GetLandValueType` is sampled at each tract's first cell. Is the type uniform over a tract, or which
  cell does the original use? [first cell]~~ **ANSWERED 2026-10-14** from the exe (`0x006BD700` calls the total grid's
  `GetCellValue`, which shifts the cell to its tract) and written into landvalue.md §7.6. **Uniform**: every cell of
  a tract gives the same value and type, so the first cell is exact.

<!-- from: From applying the residential, budget, demand, land value and flammability answers (2026-10-14) -->
- ~~**LV5:** LV2: a developer first seen while its slot 10 is false gets no snapshot until slot 10 turns true, so there
  is no second candidate in that window. [no snapshot]~~
  **ANSWERED 2026-10-14** from the growth developer's state machine (growth-developer §2, §3.2: slot 10 is `state < 17`,
  slot 28 is `state == 0`) and written into landvalue.md §7.4. Slot 10 is false only in states 17–20, and D changes in
  that window's first call, so a developer first seen there has already published unless it is still in state 17, which
  no slot distinguishes. The second candidate cannot be recovered. Since the state is 0 after construction, `Init` and a
  load, only a shadow that starts mid-update meets this. No original behaviour for the shadow; the implementer's choice
  (no snapshot) stands.

<!-- from: verify-landvalue49 coordinator review (2026-10-21) -->
- ~~**LV1 - input capture across a native pass:** train46 was72/72, train49 was70/72 and train51 was72/72 with no landvalue algorithm change. Diagnostic turbo/normal runs vlv49-a1 and vlv49-n1 were71/74 and71/73, zeroresync. Some mismatched outputs are reproduced bitwise by an earlier SimNewDay input snapshot; one pass is not reproduced by any captured day, and the first pass lacks an initial desirability snapshot. This is evidence consistent with input timing, not proof of a one-day shift. Exactly when does the original read each input (including mean/mode altitude and developer desirability) over its multi-frame pass, and which boundary permits an independent shadow to capture the same inputs? Specify startup snapshot timing and the unexplained pass before any scheduling/algorithm correction. Never choose a snapshot merely because its output matches. [open: independent input provenance]~~ **ANSWERED 2026-10-21 (re-landvalue-inputs4):** DIFFERS — the original has no input snapshot: every state reads the world in its own `Step` call, per tract, when its iterator reaches the tract (states 1, 3, 4, 5, 8 budget-sliced, budget test before `GetNext` at `0x006BE865`/`0x0060C8D0`), developer maps in each developer's state-7 call; land/water is read at states 1, 3, 8, altitude at 3 and 4, zoning at 3 and 8 (`DoIdle` `0x006BE7B0`). The only exact capture boundary is each SimIdle/SimNewDay delivery to land value (`DoMessage` `0x006BF0D0`), with pipelineState `+0x38` and the iterators `+0x3C`/`+0x40` read before and after; a day/pass-end sample is evidence, not input. Startup: pass 1 reads the developer map restored by `Read` unless the developer's first state 17 (started by the BDS SimBegin) precedes land value's state 7; the previous-map candidate is any D taken after the load while slot 10 has stayed true. Written into landvalue.md §6.1 (typed input record and examples). vlv49 a1/n1 p1 (intrinsic exact, total 391/383 off with the live map) fit this: the developer published after pass 1's state 7. a1 p2 ("no SimNewDay reproduces", closest day 1 off in 3 intrinsic tracts) is consistent with states 1/3/4 of one pass reading different worlds, which no single-instant sample can reproduce, but the log has no per-call capture, so it is not proved; the a1 p2/p3 flips ((4,14) 8/7 then 7/8, meanAlt 270.05667/270.05804 swapped) show the inputs going A to B and back to A between pass ends.
