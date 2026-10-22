| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-key-accelerator4: cRZWinKeyAccelerator core | `core-key-accelerator4` / detached reserved worktree (engine lane) | in progress | 1. Baseline gate. 2. Independent fixtures and owner inventory. 3. Implement and validate. |

## Handoff
- Goal: implement only widget-toolkit section 13 accelerator owner in a new Rust core module.
- Worktree reused at coordinator-reserved detached HEAD; clean at start, no commits beyond main.
- Read section 13, keyboard message section 7.3, all 16 ABI rows and public SDK headers.
- Independent fixtures committed first at 9dfb9b7e: 28 key/modifier cases, 9 asymmetric presence cases, duplicate/deletion/order/capacity, message, dispatch and lifetime traces. Not yet compiled.
- Filed WT7 (owner/lifetime inventory), WT8 (boundary/repeat/failure contract), WT9 (SDK/spec slot conflicts); index regeneration deferred to coordinator because tools/ is forbidden.
- Baseline qbuild all running in exec session 96460: Rust test compilation failed without diagnostics (four rustc exit-1 processes); clippy passed. Preserved baseline-qbuild-test.log / baseline-qbuild-clippy.log. CMake still pending.
- Implemented production BTreeMap owner, all published lookup/registration/enumeration/dispatch decisions and explicit live-count lifecycle; typed host only exposes borrowed service/AddRef/Release/synchronous send.
- Slot 0 is membership only; slot 15 native flags/construction remain WT7 residuals. No full-owner claim.
- Production implementation committed at 7b0b14fa; expanded to 12 tests including explicit WT8 literal boundary choices, no-server initialization, terminal and overflow safety.
- Next: capture baseline completion; focused qbuild quick, formatting check and byte/method inventory; final full gate once code stable. Use absolute file paths for qbuild fmt.
- Scope: pure core, no C ABI changes, game runs, native-exact or replacement claims.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-key-accelerator4-r1-manifest.json.
- All owner gaps must remain explicit; no other widget/HTML owners.

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
