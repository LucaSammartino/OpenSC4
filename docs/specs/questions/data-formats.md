# data-formats: open questions
<!-- Questions about [data-formats.md](../data-formats.md). Append new entries at the END; when one is answered, move it to answered/data-formats.md. Rules: README.md. -->

<!-- from: glue-formats4 (2026-10-21) -->
- **EX41 - section 8, factory registration:** §8 says the exemplar and cohort factories are "registered with the
  resource manager by type ID" but does not give the registration call. What are the arguments of the resource
  manager's RegisterObjectFactory (which of its two u32 arguments is the resource type and what the other one is),
  and when a second factory is registered for a type that already has one (`0x6534284A`, `0x05342861`), does the
  new one replace the old, is it refused, or is the first kept? Is there an unregister call a plug-in must make at
  shutdown? [not implemented: the plug-in registers no factory; the shadow only reads]
<!-- from: glue-formats4 (2026-10-21) -->
- **EX42 - section 8, CreateInstance contract:** the factory slot `CreateInstance` (`0x0067D820` /
  `0x0067D8D0`) is named by address only. What are its arguments (record or key, requested IID, out pointer and the
  two trailing u32 values), what does it return on an unknown IID, and who owns the reference of the returned object
  (is it returned with one reference the resource manager takes over, or does the manager AddRef it)? [not
  implemented]
<!-- from: glue-formats4 (2026-10-21) -->
- **EX43 - section 8, SerialRead record position:** when the resource manager calls a factory's `SerialRead`
  (`0x0067F1E0` / `0x0067F7D0`) with a cIGZPersistDBRecord, is the record positioned at byte 0 of the resource, and
  are its bytes already QFS-decompressed (§2.6) when the index entry is compressed, or must the factory detect and
  decompress the `0x10FB` stream itself? [not implemented; the shadow decompresses itself via engine/formats qfs]
<!-- from: glue-formats4 (2026-10-21) -->
- **EX44 - section 8, record close and object identity:** after `SerialRead` returns, who closes the record (the
  factory or the resource manager), and does the factory's result have to be the cSCResExemplar class for the rest
  of the game (callers QueryInterface it for `cISCResExemplar` slots 3-5, 8-15 and for the 25-slot
  `cISCPropertyHolder`); are those slots' contracts (§8 table) the full set a non-game implementation must provide?
  [not implemented: no replacement factory]
