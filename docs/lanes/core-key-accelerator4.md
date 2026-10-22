| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-key-accelerator4: cRZWinKeyAccelerator core | `core-key-accelerator4` / detached reserved worktree (engine lane) | ready for bounded pure-core merge; source f3aee604; cargo 4999 passed/0 failed; clippy 0 diagnostics; ctest 68/68; WT7-WT9; ABI unchanged; game n/a | 1. Regenerate question index centrally. 2. Resolve WT7-WT9 before native adapter or full-owner claims. |

## Result

Implemented the published section-13 accelerator behavior in `engine/core/src/key_accelerator.rs`, with an additive export at the end of `engine/core/src/lib.rs`. The core owns the ordered table, insertion/duplicate/removal policy, key packing and rejection, asymmetric presence masking, all lookup overload decisions, bounded enumeration with total counting, and synchronous dispatch decisions. The host exposes only borrowed service acquisition, AddRef, Release and MessageSend primitives. Shutdown retains bindings; final live-count Release shuts down before cleaning owned values. Getters use ordinary shared Rust borrows, with no RefCell.

Fixtures were committed before implementation (9dfb9b7e); production followed at 7b0b14fa; final source and 12 tests are at f3aee604. Expected fixture tables contain 28 independent key/modifier cases and 9 presence-mask cases, plus unsigned ordering/capacities 0-6, duplicate/deletion/reinsertion, full four-dword payloads, six input-message types, no-server/uninitialized processing, reference cleanup and separately labeled literal WT8/safe-Rust boundary cases. They do not calculate expected answers by calling a native owner or duplicating the packing formula.

This is a bounded pure-core implementation, not a full native-owner completion. Slot 0 implements membership only; native construction, QueryInterface acquisition/output effects and slot-15 deletion flags remain unpublished (WT7). No C ABI, defaults, maps, shared ABI tables or other widget/HTML owners changed. No game requests, native-exact shadow, replacement or native coverage promotion is claimed.

## Validation and preserved failures

- Final stable `qbuild all` at c40cbb3a3f4c9a18a23df12c47da18207e2c489a: PASS; workspace **4999 passed, 0 failed, 0 ignored**, clippy exit0/0 diagnostics, CMake **2/2 edges** (0.5s), CTest **68/68** (127.6s), fc4verify PASS (128.6s), no tree-changed warning. All 12 new tests passed in this workspace run. Only this report and queue metadata follow that gate; source remains unchanged.
- Focused `qbuild quick`: PASS; fc4-core **2971 passed/0 failed/0 ignored**, dependents fc4-capi/fc4-conformance/fc4-formats compiled with no-run, build **4/4 edges** (488.2s), total491.2s.
- `qbuild fmt` with absolute paths: both new files PASS. The required lib.rs check reports 166 lines of module-order/format differences, including inherited ordering; the lane retains the required additive export and does not reformat unrelated code. Early relative-path fmt invocation failed to locate files; subsequent checks used absolute paths.
- Baseline `qbuild all`: FAIL; initial workspace test build exit101 (four rustc exit-1 subprocesses, no compiler diagnostics), clippy exit0/0 diagnostics; build PASS **212/212** (511.0s), CTest PASS **68/68** (501.0s), fc4verify PASS with tree-changed warning (1016.5s). Initial cargo/clippy launched before implementation; later stages could include new source, so this is not a pristine-base CTest measurement. Failure retained, not replaced by the final pass.
- An unnecessary overlapping quick retry passed the same 2971 tests and compiled dependents, but configure failed with `ninja: error: failed recompaction: Permission denied`; qbuild also rejected `--crates` as an unknown step. This is preserved separately. Do not overlap wrappers or infer skipped tests from buffered summaries.
- Logs under this worktree's `build-runs/`: `baseline-qbuild-{test,clippy}.log`, `baseline-fc4verify-{configure,build,ctest}.log`, `baseline-LastTest.log`; `overlap-fc4verify-{configure,quick-test,quick-deps}.log`; `focused-fc4verify-{quick-test,quick-deps,build}.log`; final logs copied as `final-qbuild-{test,clippy}.log`, `final-fc4verify-{configure,build,ctest}.log` and `final-LastTest.log`. Standard wrapper log names also retain the final run.

## Questions and next steps

Appended **WT7-WT9** to `docs/specs/questions/widget-toolkit.md`: complete lifetime/method inventory and native byte residuals; literal full-width key/repeat/missing-service behavior; public SDK versus spec/ABI overload and return-type conflict. Choices are labeled in code/tests. The generated question index is untouched because this lane may only append its question file and may not run tools/; coordinator should regenerate it centrally. Resolve these contracts before building a native adapter or claiming full-owner coverage. No additional lane round is requested by this report.

## Published owner inventory

Source boundary: widget-toolkit section 13 (lines 420-474), keyboard record delivery section 7.3, the 16 cRZWinKeyAccelerator rows in docs/abi/vtables.tsv, and public cIGZWinKeyAccelerator/cIGZUnknown SDK declarations. No native execution or analysis material used.

| Native slot | Published method | Core coverage / boundary |
|---|---|---|
| 0 | QueryInterface | `supports_interface` implements only the two-IID membership decision; native AddRef/output effects absent from contract (WT7). |
| 1 | AddRef | Ordinary increment; safe Rust rejection at overflow/terminal handle, not native overflow behavior (WT7). |
| 2 | Release | Ordinary decrement; count-one Shutdown before owned map cleanup and terminal state. Native count-zero behavior unspecified (WT7). |
| 3 | Init | Acquire borrowed service identity, take reference, set initialized; once per initialized interval. Missing-server/repeat policy literal and questioned (WT8). |
| 4 | Shutdown | Release precisely acquired server reference, clear initialized, retain map. Repeat success follows literal once-only contract (WT8). |
| 5 | RegisterAccelerator(record) | Verbatim u32 key, copied four-dword value, first insertion wins; live call always true. |
| 6 | RegisterAccelerator(type) | Record initialized as type plus three zero dwords; same duplicate policy. |
| 7 | UnregisterAccelerator | Exact-code erase and present/absent return. |
| 8 | IsAcceleratorPresent | Unsigned key-order scan, optionally mask only stored-side 0x00070000 and 0x10000000. |
| 9 | TestForAccelerator(key, record) | Pure key rejection, sentinel-modifier rejection, printed packing expression, owned-map lookup/copy. Full-width vk ambiguity WT8. |
| 10 | TestForAccelerator(key, type) | Same lookup, type only. |
| 11 | TestForAccelerator(message, record) | Types 5/6 only; data[0]/data[1] supply key/modifiers, data[2] ignored. |
| 12 | TestForAccelerator(message, type) | Same message lookup, type only. |
| 13 | GetKeysForAccelerator | All type matches counted, capacity-bounded writes in unsigned key order; safe slice and separate written/total counts. |
| 14 | ProcessAccelerator | Own lookup and hit/miss decision; synchronous bound record send only when server held; hit true even without server. |
| 15 | Deleting destructor | Rust Drop releases acquired service and drops owned values; native deletion argument/flags/order not implemented or claimed (WT7). |
| nonvirtual | Construction / any helpers | Explicit empty uninitialized Rust state with caller-supplied nonzero live count. Native initial count/complete helper inventory unpublished (WT7). |

## Published byte inventory

Rust does not mirror the native object or tree-node layout; no offsets are dereferenced. Only Message has repr(C).

| Native region | Published meaning | Rust representation / residual |
|---|---|---|
| +0x00..03 | Vtable identity (32-bit ABI) | No native vtable or adapter. |
| +0x04..07 | Reference count | Explicit u32 live count; initial value and edge cases WT7. |
| +0x08 | Initialized byte | bool; no independent enable/disable flag is specified. |
| +0x09..0B | Not described | Not mirrored; padding/layout residual WT7. |
| +0x0C up to next published field +0x18 | Ordered-map storage | Owned BTreeMap<u32, Message>; descriptor bytes and node topology not mirrored. |
| node +0x14..23 | 16-byte message value | Message type_id and data[3], four u32 values; size/alignment checked by fixtures. Node key/header offsets unpublished. |
| +0x18..1B | Held 32-bit message-server reference | Option of opaque host Server identity; core controls AddRef and Release decisions. |
| Beyond +0x1B / total size | Not described | No total-size/full-byte-owner claim; WT7. |

WT9 records SDK versus spec overload ordering and slot-13 return-type conflict. Pure-core signatures follow the behavior spec; raw out-pointer validity, native QI identity and adapter ABI remain embedding work after contract clarification. Section 13.4's cGZWin ownership, fallback dispatch and window Shutdown are a different owner and were not implemented.
