| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplar-final4 (EX51 a-f applied to FC4ExemplarFactory; 8-scenario native-vs-ours test; EX50 per-module split) | `glue-exemplar-final4` / (engine lane) | in progress (round 3): GetValString fix + SerialWrite fixture + EX50 entry tracing committed UNBUILT; gate and game run not yet done | see Handoff |

## Handoff (round 3, 2026-10-22; replaces the round-2 next list; the round-2 report below stays as history)

Goal (round 3): fix GetValString name conversion; SerialWrite fixture setup (identical SetKey on both sides, SetWriteMode(false) on both factories, EX51 d); EX50 caller addresses at the concrete virtual entries; full gate; one clean union run to 2032 with ref_warnings 0 under a NEW unique label. Do not force slots 14/16 to return 2 or guess EX52 ABI.

Done (this commit, NOT YET COMPILED OR TESTED, the lane guard stopped at 60 calls):
- capi: `fc4_variant_get_string(var_type, data, count, sink, ctx) -> int32` (1 string/true, 0 text written/false, -1 nothing) in engine/capi/src/props.rs on fc4_formats `Property::get_string_into`; `variant_u32` refactored onto a shared `variant_value` (GameValue enum). Unit test `get_val_string_text_and_result`. Declared at the end of the function list in engine/capi/include/fc4.h (additive, no ABI bump).
- plugin: cFC4Variant::GetValString (was the SDK stub, returns false/empty: the cause of the '' names) now goes through the capi; `fc4::VariantValueBytes` exported from FC4PropertyHolder.cpp (wraps the anon ValueBytes), declared in cFC4Variant.h; cFC4Variant.cpp includes fc4.h.
- FC4ExemplarFactory.cpp: `EntryScope` (outermost-entry caller tracing, natural game/other vs ours fc4/fixture counters, nested GetPropertyList counted under the outer entry) on GetPropertyList, SetCohort, SetParent, RebuildEffective, DebugInstanceTag, CreateInstance(record)/CreateFromRecord, factory Read/Write/SerialWrite; `LogEntries()` after LogCallers at shutdown. RunSide2: SetWriteMode(false) via slot 12 on fe and fc, SetKey(kTypeExemplar,0x33,0x44) on e[3] and (…,0x33,0x45) on e[4] before SerialWrite.

Next (exact):
1. File EX53 at the END of docs/specs/questions/data-formats.md (and QUESTIONS.md index per README): "section 5.1 GetValString on a scalar Char (0x0C) / RZUnicodeChar (0x0D): which text, which result? Literal reading taken: unsupported, false, output unchanged (fc4_variant_get_string -1)". Code comments already cite EX53.
2. `qbuild.py --wt <WT> quick` then fix compile errors (watch: fc4_bytes_sink signature in the C++ SinkToString, EntryScope defined before ExHolder uses it, std::atomic arrays zero-init); `qbuild fmt engine/capi/src/props.rs`.
3. `qbuild all` (baseline round 2: cargo 4099/0, clippy 0, ctest 65/65).
4. Game: request with a NEW label (e.g. gef4-r3-union2032), default set + enable=exemplars, sim_turbo=1, end_date 2032; wait with qwait WITHOUT --fresh; grep log for "contract" and "EX50 entry". Expect GetValString DIFFs gone; slots 14/16 return DIFF remains (EX52, do not fix by guessing).
5. Report: natural (game/other) vs ours (fc4/fixture) counters separately; keep old 4/8 and r1 crash trace facts.

Facts: round-2 game runs gef4-r1-union2032 (crash, fixed), gef4-r2-union2032 (clean 2032, ref 0, 4/8 match). No request outstanding. EX52 open (slot 15 allocator, slot 28 args, slots 14/16 returns).

## Report

What changed (engine/plugin/FC4ExemplarFactory.cpp only)
- Slot 15 with a non-null dest fills the native `{begin,end,cap}` vector of 8-byte `{id, prop*}`, one ref per element, ascending ID, own wins; the dest's old elements are released; the buffer grows with the STL node allocator (B, EX52 a). The object's effective list is left alone.
- Holder slots 10-13 touch the own list only: no Mirror, no post, bool ignored. Slots 14/16 build a new cFC4Property and go through slot 15 (effective list updated, posts when true).
- RemoveAllProperties posts only when the own list was non-empty. SetCohort/RemoveAll/slot 15 return values are documented as undefined.
- DebugInstanceTag: " Exemplar Cohort " for cohorts; type test reads raw word (variant slot 73); name through GetValString (slot 55); lookups through holder GetProperty (effective).
- Factory SerialWrite: binary EQZB1###/CQZB1### on record slots 28/49/37/29/50 (the SDK header lacks slot 28, so calls go by the spec's slot numbers); text form refused (EX52 e). Factory Write: QI 0xC56B8F08, then SerialWrite.
- EX50: GetPropertyList calls are split per module (game / fc4 / fc4-fixture / other) and the split is logged at shutdown.
- Contract test: return values that EX51 c calls undefined are no longer compared; added add_slots_b, rebuild_dest_a, serialwrite_d (recording fake serial record), tag_text_ef.

Measured
- Gate: cargo 4099/0, clippy 0, ctest 65/65 (baseline the same).
- gef4-r1-union2032: crash in the contract test (a Uint32 value read as a pointer in SerialWrite kind-1 detection); log kept. Fixed.
- gef4-r2-union2032 (default set + enable=exemplars, turbo): clean to 2032-01-12, ref_warnings 0. EX50: game=0 fc4=3680 fc4_fixture=18 other=0. No SimCity4.exe caller of GetPropertyList in city play without actions. Callers: FreeCity4.dll+0xD0E6AC 3691 (our rebuild), +0xD136CE 5, +0xD0EFE0 2.
- Contract 4/8 MATCH: list_order, remove_reveals_parent, setcohort_refs and rebuild_dest_a (n=2, refs and eff unchanged, matches EX51 a).
- DIFFs:
  - stubs_tag_rebuild and tag_text_ef: our name text is '' where native gives 'Fc4Test', '5' and 'abc'. Our cFC4Variant::GetValString gives empty on these values (bug in ours, still to investigate). Type and cohort text match.
  - add_slots_b: everything matches except the return values of slots 14 and 16 (native 2, ours 1). Native looks like it returns the new property's Release result. Unspecified, so it needs to go in EX52.
  - serialwrite_d: native objects made by CreateInstance(type) seem to have type 0 in their key (exemplar SerialWrite returned 0 with no writes). The native cohort factory wrote the TEXT form (write mode true by default?). The test has to SetKey the exemplars and compare per write mode. Ours writes the spec's binary bytes exactly (sig, key 0 / parent key, count, id, kind 0, variant t6?). Note: the variant type is 6 for a Uint32 from slot 13 (ours); check against native once the native binary path runs.

Next
1. Fix GetValString in our name path, then SetKey on the test exemplars and SetWriteMode(0) on both factories before SerialWrite. Rerun with a new label.
2. Slots 14/16 return value: add it to EX52 or return the Release result.
3. Default promotion: the coordinator decides. Evidence so far: lifecycle/read ok, 4/8 contract.
