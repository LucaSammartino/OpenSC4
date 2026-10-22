# Persistence runtime audit (before implementation, 2026-10-22)

Source: persistence.md sections 2-7, foundations.md sections 2.3 and 9.2.
The published attribution gives owner totals, not individual method byte sizes.
No per-method byte credit can be inferred from these groups or Rust methods.

| New owner | Published functions / bytes | Published body groups to implement | Initial status |
|---|---|---|---|
| DBRecord | 9 / 207 | construction, QI, AddRef, Release/destruction, key get/set, flags get/set, close | unimplemented; initial reference count unspecified |
| DBSerialRecord | 49 / 6,172 | inherited record; serial/input/output QI; scalar/array signed/unsigned 8/16/32/64; float32/64; strings; keys; variants; raw stream forwarding | unimplemented; raw stream I/O remains a primitive, not typed encoding |
| PersistResource | 6 / 115 | construction, interlocked AddRef/Release, QI, key get/set, derived type | unimplemented |
| PersistResourceFactory | 14 / 680 | locked lifetime; record/type construction; convenience; allocation hook; Read/Write QI and serial hooks; temporary release | unimplemented; initial count and failed-allocation output unspecified |
| BufferResource | 32 / 5,030 | buffer construction/destruction; buffer-first QI; get/get-as/set; format dispatch; v0/v1/v2; BMP load; QFS/BMP save | unimplemented; foreign pixel conversion is a primitive |
| BufferResourceFactory | 21 / 1,517 | target and version stack; type/QI creation; serial dispatch; temporary load/save; file open/access/close; module-resource entrypoints | unimplemented; OS file/module allocation domains need explicit hosts |
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
