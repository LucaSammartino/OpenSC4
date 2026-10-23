| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-resource-catalog4 (portable catalog/picker/key input) | Codex / core-resource-catalog4 | **ready**; Rust 5400/0, clippy clean, CTest 68/68; 28 new test functions / 24 published rows; ABI unchanged | Coordinator review/publish detached tip; retain RC1–RC5 before widening/native integration |

## Portable catalog/picker/key-input subset

Source commit: 65b9c93a. Independent expected ledger first committed at 9690d238, before production code at c4e57d5a. Reviewed state/reference ordering and completed source at 65b9c93a. Base d35775f9. Reserved detached worktree reused; own manifest lane/engine role/path/round 1 checked. No coordinator spawn check, RE queries, subagents, game requests or branch/ref operations.

Changes: new engine/core/src/resource_catalog/{mod.rs,key_input.rs,tests.rs,ledger.md}, one additive export at the end of core lib.rs. Existing resource_editor, property/value/curve/table owners untouched. No C ABI, plugin, CMake, SDK, maps, coverage, defaults, guards or shared-state edits. No ABI-version change. Family identity figure 16686 is uncredited; no native byte or whole-owner coverage claim.

## Exact portable method inventory

Supported below means the stated portable algorithm/effect subset under valid primitive inputs. It does not promote the native method, its interfaces or its storage to a complete implementation.

| Portable API | Canonical owner/method | Status and exact limits |
|---|---|---|
| Key::new/gti | Three-word TGI and raw GTI representations | Supported word representation/permutation only |
| Catalog::set_type_filter/set_group_filter/set_read_only/set_byte_dd | SetFilterForType/Group, SetReadOnly, folded slot 0C | Supported store-only words/bytes; no immediate rebuild or inferred later use |
| Catalog::new | Catalog constructor | Partial: supplied valid registered descriptors, ordinary Rust storage, 0/0/0/1 setter fields; FFFFFFFF descriptor index is an explicit portable uninitialized marker, not a recovered native default |
| Catalog::init_descriptor_map | Known descriptor loop within Init | Partial: zero-wildcard descriptor admission, original order, visible-to-original map, preview read-only/attach/hide and selector append(-2)/select-zero intents; requires nonempty admitted set and valid previews; no Ini parsing or registration/creation defaults |
| Catalog::update_key_list | updateKeyList | Partial native body, owned loop supported: invalid descriptor/list return before clear; clear before enum failure; count re-queried before each index and after every iteration; exact literal group only, no T recheck/dedup; names/fallback uppercase 0x%08X, borrowed identity stored as row data; explicit foreign post-order permutation; row-zero default and last displayed full-key duplicate; one final preview call |
| Catalog::get_selected_key/refresh | GetSelectedKey/RefreshKeyList | Supported coordinate/row-data primitive consumer: null data false leaves output unchanged, no refresh rebuild on false; no native invalid-control/pointer contract |
| Catalog::set_view_key | setViewKey | Partial: identity switch clear/hide/store/show then key/focus intentions, same-preview key/focus without ref calls; preview/window internals foreign |
| Catalog::selector_command/command | GZOnCommand | Partial: missing child true, successful primitive selector QI map/store before Release, QI failure retains descriptor but rebuilds zero request, focus/false; separate button command routing; checked map-index extension, no native invalid-map repair |
| Catalog::button/kill_self | doButtonPress/KillSelf | Partial: catalog 18 => preview clear + close(1), AL false; 1A => close(0), AL false; 1B => no-op true; other false; nonmodal removes via parent without result; 19 returns explicit Unsupported AddResource, whose published AL=true does not establish a completed creation path |
| Picker::input/output | Picker slots 18/1C adapters | Supported valid-array storage subset: assign/read raw input, read-only/DD zero, T=raw[1], G=raw[0], no rebuild; output starts zero, ignores selected-key false, always writes GTI/assigns output; foreign variant validation/storage/assignment internals withheld |
| Picker::accepted_key, picker_word_output | Existing Add Key consumer; 00664680 | Supported separate modal-zero acceptance conversion and FFFFFFFF output-word store; neither makes variant output availability modal acceptance |
| catalog_qi/picker_qi | Recognized QI branches | Portable effects only: catalog write then AddRef; picker AddRef then write; other IDs Unsupported, not native false. No adjusting thunks, native QI ownership, pointer validation or native counter implementation |
| filename, KeyInput::set_filename | setFileName | Supported byte sanitation (ASCII letters only), lower-case eight-digit I, no prefix for empty name; display string only, no file/path/extension policy |
| Search::new/query/direction/reply | getNextID | Supported steppable raw occupancy consumer: unsigned direction, signed initial normalization, later increment 7FFF -> 0 / decrement 1 -> 7FFF; completion only on a free primitive reply. No native failure, cycle detection or universal/hardcoded bound; all occupied remains noncompletion |
| KeyInput::new | Key-input constructor | Partial: copy key, zero reentrancy, null held references in portable state; no native allocation/table/window construction |
| KeyInput::text_notification/message | DoMessage known consumer decisions | Supported typed-message/raw-source subset: name refresh, raw parser result stored unchanged, spinner find/QI/write with reentrancy transitions and NO text-path QI Release; forward/delegate decisions. Typed message fields replace native message-pointer decoding, not ownership/storage validity |
| KeyInput::spinner_start/spinner_finish | doSpinnerChange + decoded continuation | Partial: control/reentrancy decision and exact search continuation; differing result writes QI spinner and Releases on QI success, then decimal text/select/focus/filename and false. No allocator/export-split invention |
| KeyInput::command | Key-input GZOnCommand | Supported separate button map 18 -> close(0), 19 -> close(1), recognized branch returns foreign AL, other pairs false |
| Catalog::shutdown/destroy, Picker::destroy | Catalog Shutdown/destructors and picker destructor | Scoped effects only: descriptor Releases per reference even aliases, name destruction, preserve vector capacity at Shutdown, current preview cleared without extra Release, list Release once; catalog destruction does not call Shutdown, picker variant destruction precedes catalog; conditional bit-0 free intent |
| KeyInput::shutdown/destroy | Key-input Shutdown/deleting destructor | Scoped effects only: hide/158(0), held 10C then 110 slot10/Release/clear, 114 window14/Release/clear, base Shutdown; deleting destructor explicitly Shutdown then filename/base destruction and conditional bit-0 free; repeat Shutdown still emits foreign window/base calls without repeated held Releases |
| base_init/base_shutdown/base_destroy | Base Init/Shutdown/destructor/deleting wrapper | Partial Init config/create/add/release intents; initialized or creation failure still true; Shutdown solely base; destruction installs base table and destroys base, freeing only for bit 0. No geometry/widget construction |

Unsupported: all Unknown rows of canonical §8, particularly addResource 00685D70; Ini/registration 006866E0/00686B90 and container helpers; catalog 00685610/60/B0/C0, 006858E0, 006859B0, 00686410, 006865B0, 0079B0E0; undecoded picker 006646A0/B0/C0/D0/E0; base 00675C90/DF0; key-input 00682140/F0, 006824A0, 00682730 and alleged split start 00682632. Forwarding/refcount/native ABI bodies are not independent implemented owners. Widget geometry/COM ABI, modal event loop, invalid native pointers, foreign storage validity and Drop-driven foreign cleanup are withheld.

## Primitive and lifetime boundaries

RowPrimitives supplies raw enum success/count/index, borrowed opaque identity plus TGI words, raw name-helper reply, explicit post-order row permutation and selected widget coordinates/row data. InputPrimitives supplies raw parser word, spinner QI result, name bytes and primitive foreign AL/delegation. Search takes one occupancy bool per typed query. Effects are owner-selected intents; no host filter/final selection/search/sanitation/picker-outcome or whole add-resource callback exists.

Borrowed RawKey references must remain valid for the entire catalog lifetime, including after enumeration and refresh. No enumeration or key AddRef/Release is fabricated, and absence of an enumeration Release is not called ownership or a leak. Each descriptor preview reference and held UI reference must already be valid/held as documented; current preview is not an additional owned reference. Safe Rust invalid index/permutation/entry/control refusals are explicit checked extensions, never native false/default or repaired undefined behavior. No RefCell, native pointers, new typed ABI or automatic foreign cleanup.

RC1–RC5 retained unchanged; no new questions or claimed closure. RC1 lifetime; RC2 foreign ordering/naming; RC3 registration/empty admission; RC4 add-resource/variant/native adapters; RC5 parser/QI/export split. Existing questions at docs/specs/questions/resource-catalog.md cover the deliberately unsupported areas.

## Verification

- Independent ledger: all 24 literal §7 rows mapped V01–V24 before production outputs existed. Not the historical 23 count, not executed native goldens.
- 19 test functions cover the 24 rows; 9 additional test functions exercise published-derived edges, 28 total. Parameterized traces include 2 post-order choices, 4 descriptor filters, 4 normalization/direction inputs, 2 wrap traces, 3 spinner continuation/QI traces, explicit destructor/checked-boundary traces and 32769 occupied replies demonstrating continuing noncompletion (not 32769 native cases).
- Baseline all: collected exit 0, Rust 5372/0, clippy 0 diagnostics, native build 217/217, CTest 68/68; source changed during initial run, so baseline only.
- Intermediate quick: collected exit 0; native build passed, no new Rust test log; review changed source, so not final stability evidence.
- Required actual Rust testing: qbuild test collected exit 0, 5400/0, 28 resource_catalog::tests lines ok; before/after engine hashes identical.
- New three Rust files fmt check exit 0. Four-file check exit 1 solely shared lib.rs module ordering; same baseline defect independently reproduced at d35775f9. Shared file change is exactly the appended export, per additive-only rule; no mass reformat.
- Final canonical all: collected exit **0**, **PASS**, Rust **5400 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, native build **4/4 edges PASS**, full CTest **68/68 PASS**. Exactly one final all after actual testing, on committed source **65b9c93a**. Owned qbuild PID 17400, elapsed 922.4 s.
- Before/after tracked-engine SHA-256 are identical: `773e27d22fe31346817972ecf5692a0af23e26cb99ae6efc7d7b7c6b0ccd8f5b` (generated engine/tests/abi_entries.inc excluded; never edited). No engine changes during the gate.
- Durable evidence: build-runs/catalog-all.started.json (exact PID/command/start/source tip), catalog-all.json (terminal exit/before/after), catalog-all.console.log, catalog-evidence.json; qbuild-test.log, qbuild-clippy.log and fc4verify-{build,ctest}.log. No inferred or uncollected PASS.
- No native game/shadow/RE query: prohibited for this lane. No native execution claim, native ownership closure, byte attribution or default enablement.

Next: coordinator reviews and publishes detached commits to reserved lane branch; RC1–RC5 require future published answers before widening/native integration.
