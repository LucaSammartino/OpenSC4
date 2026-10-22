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

## R2 boundary review (validation pending)

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
