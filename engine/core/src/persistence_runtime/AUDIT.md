# Persistence runtime audit (before implementation, 2026-10-22)

Source: persistence.md sections 2-7, foundations.md sections 2.3 and 9.2.
The published attribution gives owner totals, not individual method byte sizes.
No per-method byte credit can be inferred from these groups or Rust methods.

| New owner | Published functions / bytes | Published body groups | Bounded owner status |
|---|---|---|---|
| DBRecord | 9 / 207 | construction, QI, AddRef, Release/destruction, key get/set, flags get/set, close | partial; explicit constructor count, logical lifetime; native allocation unsupported (PR9) |
| DBSerialRecord | 49 / 6,172 | inherited record; serial/input/output QI; scalar/array signed/unsigned 8/16/32/64; float32/64; strings; keys; variants; raw stream forwarding | partial; owned typed algorithms; native vtables/error state, allocation thresholds and malformed-memory effects unsupported (PR1/PR10/PR13) |
| PersistResource | 6 / 115 | construction, interlocked AddRef/Release, QI, key get/set, derived type | partial; atomic logical lifetime and key/QI; native object reclamation unsupported |
| PersistResourceFactory | 14 / 680 | locked lifetime; record/type construction; convenience; allocation hook; Read/Write QI and serial hooks; temporary release | partial; sealed BufferResource specialization; other derived owners, reentrancy and native allocation-failure outputs unsupported (PR9) |
| BufferResource | 32 / 5,030 | buffer construction/destruction; buffer-first QI; get/get-as/set; format dispatch; v0/v1/v2; BMP load; QFS/BMP save | partial; foreign colour conversion remains a primitive; native malformed-size/BMP/allocation domains unsupported; padded v1 copy is an explicit choice (PR2/PR3/PR11/PR12) |
| BufferResourceFactory | 21 / 1,517 | target and version stack; type/QI creation; serial dispatch; temporary load/save; file open/access/close; module-resource entrypoints | partial; OS file/module acquisition remains primitive; native hosts/constructors/close-failure precedence unsupported (PR9/PR13) |
| **New-owner denominator** | **131 / 13,721** | | **No blanket owner completion credit** |

Existing authoritative `persistence_manager::KeyList` (22 functions / 1,071 B)
is reused by re-export, unchanged. It implements insertion, size, slice access
and logical reference count; list insertion/erase/erase-all/presence/QI/callback
enumeration and native container internals remain residuals. Manager/cache
(109 / 8,437 B) is outside this lane and receives no additional credit.

PR1: constructor flag explicit, both flag values tested; no DBPF configuration claim.
PR2: all image fixtures constructed, no shipped-form coverage claim.
PR3: packed agreement and padded FreeCity4 row-copy choice tested separately.
Hidden constructors, undefined short-read destinations, arithmetic overflow,
allocation failure details, malformed UTF-16 and foreign RGB conversions are
not implicitly established by implementing a module. Final per-group status
and discriminating test evidence will be recorded here.

## Implemented portable bodies (pre-gate)

- DBRecord: all published state/access/QI/reference/close groups; explicit
  count input, segment AddRef, close-once release even on false, destructor
  release without an extra close. Native constructor allocation unsupported.
- SerialRecord: all integer/signed/float widths and arrays, raw-result and
  swap-after-failure quirks, strings, resource keys, marker-free variants,
  shared-position borrowed stream views and std Cursor byte primitives.
  Native GZCOM vtable adapters, stack/heap allocation thresholds and malformed
  string memory effects are not implemented/claimed.
  Shared native stream error-state mutation is not established by the raw
  boolean primitive contract (PR13); borrowed views expose only the published
  raw/string forwarding subset, not a complete GZ stream vtable.
- Resource: atomic reference count initially zero, QI, locked key and type.
  Native object memory reclamation is represented by logical release plus
  Rust ownership; no plugin/native resource replacement is claimed.
- ResourceFactory: locked record/type construction, forwarded argument route,
  key-before-Read, serial QI acquisition/release, output QI and temporary
  cleanup. Derived hooks are sealed runtime implementations, never foreign
  full-SerialRead providers. Current object representation specializes to
  BufferResource; other derived resource owners and reentrancy unsupported.
- BufferResource: held-buffer references, buffer-first QI, v0/v1/v2 raw-prefix
  dispatch, stored-vs-target initialization, correct packed/padded row copies,
  QFS decoded-size validation, palette/BMP parsing, bottom-up BMP choice,
  version dispatch, packed QFS save, literal BMP headers and ignored raw
  write-result quirk. Pixel colour conversion remains a narrow primitive.
  Owned buffers use only the published colour table, packed stride and an
  explicit caller memory bound. Native thresholds/allocation failures,
  malformed v1 size effects, negative BMP dimensions, inferred translation
  details and buffers supplying a foreign resource interface remain gaps.
- BufferResourceFactory: target/default stack, type allocation/QI/temporary
  cleanup, serial type/QI checks, temporary load/save, file access 1/3 and
  close ordering, module-vs-own-module target selection. OS open/module
  acquisition are explicit primitives; no production native SDF/Win32 host
  implementation or native constructor-allocation failure claim.
  Native file-close-failure result precedence is unresolved (PR13); the
  portable wrapper retains the load/save result and always performs close.

All six owners remain **partial** against the 131-function / 13,721-B
denominator. No exact implemented-byte subtotal is asserted: the spec gives
no per-body byte attribution. Foreign primitives and existing KeyList are
excluded. QFS reuse is via a narrow adapter in formats because formats already
depends on core; core cannot depend back on formats.

## Oracle boundaries

The 17 new core test functions and one formats integration function are test
inventory, not execution results or native function/byte coverage. Field
vectors precede their implementations; v1/v2 literal fixtures were recorded
then too. Other lifetime/BMP/variant/edge tests were added after their bodies.
Expected field/header/pixel bytes are literal; round trips are supplementary.
Synthetic pixel conversion establishes ordering and byte plumbing only.
The real formats codec is tested against a literal RefPack stream and literal
decoded pixels; compressed encoder choices are not asserted native-canonical.
Packed v1 agreement is distinct from the padded v1 PR3 choice. BMP row
translation and padding expectations are portable choices pending PR11,
not a native pixel conversion oracle. File/module fixtures touch no OS files.
There is no native shadow, shipped-image scan, original base-resource
verification or combined manager/runtime coverage credit in this lane.
Full foreign resource subclasses, stream vtables, constructors, allocator
failure effects and undefined malformed input domains remain unsupported.

## R2 boundary review

Rechecked persistence sections 2-7 against the concrete bodies and test
expectations. Typed widths, integer-only swapping (including failed reads),
strings/keys/marker-free variants and borrowed cursor position remain in
`serial.rs`/`variant.rs`. Segment close-once/ref effects remain in `record.rs`.
Atomic resource references, QI and keys remain in `resource.rs`.
`ResourceFactory` owns record/type routing, key-before-Read, serial QI,
requested QI and temporary release. Its production hooks are sealed and
implemented by `BufferResourceFactory`; test-only hook fixtures discriminate
forwarding/failure order, not arbitrary foreign subclass correctness.

Image dispatch, raw prefix versus typed header order, decoded-size checks,
packed/padded copies, BMP parsing/writing and unlock paths remain in
`image.rs`. Factory defaults/stack, type/QI checks, temporary resources,
file access 1/3 and close-after-operation, and module target selection remain
in `factory.rs`. Host boundaries are raw I/O/seek/size/position/close,
buffer allocation/ref/QI/lock/pixels/foreign colour conversion, OS opening
and module bytes, and the existing formats QFS codec. No owner-level oracle
or manager/cache dependency was introduced.

The tests include literal integer/float/header/pixel expectations, a failed
typed read with host-modified prefix, short/UTF-16/legacy strings, key failure
short-circuiting, variant failure publication, segment close-once, QI/ref
cleanup, raw v0 prefix with swapped later fields, v1 stored format, padded
v1 choice versus v2 correct rows, BMP palette/padding/ignored-write results,
explicit memory bound, and in-memory file/module target paths. The formats
integration test consumes literal QFS bytes and separately checks literal
decoded pixels and saved headers; the encoder round trip is supplemental.
None of this establishes native conversion, malformed-memory effects,
native constructor defaults, shipped data or native/shadow equivalence.

## Measured focused test inventory

The coherent quick cargo-test substep ran every test below once successfully
(12 main core, 5 edge core, 1 formats integration). Its complete log contains
12 `test result:` suites / 3,225 passed / 0 failed / 0 ignored, including
successful formats doctests. At this checkpoint dependent compilation and
the complete quick/fmt/all gates are still pending. Evidence:
`build-runs/fc4verify-quick-test.log` and the generated
`build-runs/runtime-execution-summary.json` (individual exact-name counts).

| Test function | File relative to engine/ | Discriminating expectation |
|---|---|---|
| `independent_integer_widths_signed_bits_and_array_order` | core/src/persistence_runtime/tests.rs | Literal signed/unsigned 8/16/32/64 bytes, both orders, unchanged source array |
| `float_bytes_are_never_swapped_and_failed_get_still_swaps` | same | Literal LE IEEE floats with flag 0; swap after false; zero-length write |
| `string_exact_lengths_empty_short_header_and_failure_channels` | same | Literal lengths, empty/short header, failed length short-circuit, valid UTF-16 |
| `record_close_failure_is_once_and_destruction_only_releases` | same | Segment event ordering, false close once, flags/key/QI/logical references |
| `views_share_position_and_resource_buffer_replacement_balances_references` | same | Borrowed shared cursor and buffer reference/QI replacement effects |
| `literal_v0_v1_raw_prefix_and_stored_format_conversion_failure` | same | Raw prefix despite flag 0, swapped later fields, literal packed pixels, stored-format failure cleanup |
| `bmp_headers_palette_padding_row_choice_and_save_ignored_write_failures` | same | Literal BMP headers/rows, ignored offset, palettes/nibbles, padding, three writes despite false |
| `factory_defaults_type_failures_temporary_cleanup_and_settings` | same | Published target/stack, wrong type/IID, temporary release and load |
| `factory_record_type_order_fail_read_qi_and_forwarded_arguments` | same | Key before Read, allocation/Read/output-QI failure cleanup, forwarded arguments |
| `variant_no_marker_unsupported_tags_and_temporary_publication` | same | Literal marker-free tags/values, unsupported write/read tags and preserved output after failure |
| `colour_table_and_explicit_owned_storage_limits` | same | Published bits table, explicit 16-byte bound and checked invalid sizes |
| `independent_get_widths_floats_and_defined_partial_host_bytes` | same | Literal typed reads and known Cursor partial-prefix modification before failed-read swap |
| `padded_v1_choice_v2_correct_rows_and_save_drops_padding` | core/src/persistence_runtime/edge_tests.rs | Separate PR3 padded-v1 choice, correct v2 rows, packed save excluding padding |
| `serial_qi_failure_does_not_call_read_or_release_unacquired_record` | same | Failed serial QI leaves record reference untouched; resource temporary released |
| `delegated_resource_iid_releases_the_buffer_reference_only` | same | Unsupported foreign resource-IID case releases acquired buffer, not resource |
| `legacy_string_total_size_and_key_failure_stop_at_first_raw_failure` | same | Legacy total-size iteration and key partial bytes/short-circuit |
| `file_access_close_failure_and_module_target_selection_are_runtime_owned` | same | In-memory access 1/3, close calls/result choice, null save no open, own/given-module targets |
| `literal_qfs_load_exact_size_failure_and_save_header` | formats/tests/persistence_runtime.rs | Literal RefPack input/pixels, decoded-size/signature failure unlock, saved header and supplemental encoder round trip |
