| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-exemplar-final4 (EX51 a-f applied to FC4ExemplarFactory; 8-scenario native-vs-ours test; EX50 per-module and per-entry caller split; GetValString through the core) | `glue-exemplar-final4` / (engine lane) | ready (round 3): gate green, gef4-r3-union2032 clean to 2032 ref 0, contract 7/8 match (add_slots_b differs only in the slot 14/16 return values, EX52 f) | answer EX52 a-f and EX53; then a script run with a query-tool click and a save for EX50 |

## Report (round 3, 2026-10-22)

What changed in round 3
- 7c19eb65 (round 2 tail, unbuilt then) validated: `fc4_variant_get_string` (engine/capi/src/props.rs, declared at the end of engine/capi/include/fc4.h, additive, no FC4_ABI_VERSION bump), cFC4Variant::GetValString through the core, SerialWrite fixture SetKey/SetWriteMode(false) on both sides, EX50 EntryScope tracing on the concrete entries.
- 68d4090c: the GetValString body moved to FC4PropertyHolder.cpp behind `fc4::gVariantGetValString` (set at static init), because fc4_plugin_load_test links cFC4Variant.cpp without the core; there it answers false, value unchanged.
- 1ab8f999: main merged (69 commits); fc4.h conflict resolved by keeping main's span565 block, then fc4_variant_get_string at the end.
- Questions: EX53 filed (GetValString on scalar Char / RZUnicodeChar; literal reading: false, output unchanged); EX52 update (f) filed (slot 14/16 return values). QUESTIONS.md data-formats row: open 3, next free EX54.

Measured
- Gate (qbuild all): cargo 4107 passed / 0 failed, clippy 0, ctest 66/66. Focused test props::tests::get_val_string_text_and_result ok; rustfmt clean on props.rs.
- Game gef4-r3-union2032 (default set + enable=exemplars, Konradshohe, turbo, end 2032): verdict clean, date 2032-01-02, ref_warnings 0, foreign_exceptions 0.
- Contract 7/8 MATCH: list_order, remove_reveals_parent, setcohort_refs, stubs_tag_rebuild, rebuild_dest_a, serialwrite_d, tag_text_ef. DIFF add_slots_b: only s14 and s16 returns (native 2, ours 1); all effects match. Not forced (EX52 f).
- EX50 concrete-entry counters, natural (game / other) vs ours (fc4 / fixture):
  - CreateInstance(record): game 9157 (one caller, SimCity 4.exe+0x572F5C, abs 0x00972F5C), other 0 | fc4 0, fixture 0; nested GetPropertyList 3623.
  - GetPropertyList 0/0 | 0/5; SetCohort 0/0 | 0/10 (nested 8); SetParent 0/0 | 0/0; RebuildEffective 0/0 | 0/9 (nested 5); DebugInstanceTag 0/0 | 0/7; Factory.Read 0/0 | 0/0; Factory.Write 0/0 | 0/0; SerialWrite 0/0 | 0/4.
  - GetPropertyList by module: game 0, fc4 3623, fc4_fixture 18, other 0 (the fc4 calls are our rebuild inside the game's CreateInstance(record)).
- Earlier runs: gef4-r1-union2032 crash (fixed 3b0c66ad), gef4-r2-union2032 clean, 4/8.

Remaining gaps
- EX52 a-f open (slot 15 dest allocator and third word, serial slot 28 args, kind-1 key slot, text form, slot 14/16 returns); EX53 open; EX50 open: in city play without actions the game's only entry is CreateInstance(record) from 0x00972F5C, no game caller of GetPropertyList/SetCohort/SetParent/Rebuild/Tag/Read/Write/SerialWrite. A script run with a query-tool click and a save would complete it.
- No default promotion: exemplars stays behind enable=exemplars.

## Report (round 2, history)

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
