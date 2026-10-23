# Network type info: static records, defaults and reload ownership

## 1. Scope and evidence

This specifies the owned outer lifecycle of the thirteen process-global network type records in Windows
SimCity 4 Deluxe 1.1.641. It complements [network-occupants §8](network-occupants.md#8-csc4networktypeinfo--the-per-network-type-tables),
[network-geometry §2.3–2.4](network-geometry.md#23-network-type-info-snetworktypeinfo-base-0x00b452c8-stride-0x114-13-records)
and [network-bridges §2](network-bridges.md#2-bridge-types). Resource labels, shipped exemplar values and foreign
parser/model algorithms remain in those specs. The new contract is initialization, preservation, destruction,
call order and publication, conditional on the primitive calls described here returning normally. *[compat]*

Primary evidence is the canonical `tools/re_query.py` export, with byte-bearing instruction queries for numeric
and state writes. `ghidra/export/windows/export-summary.txt` identifies image base `0x00400000` and executable
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`; the independently hashed
`bin/SimCity4.exe` matches and its Windows file version is `1.1.641.0` (2026-10-23). Local decompilation was
navigation only. No original execution, native test pass or implementation comparison is claimed.

Notation: `R` is a record; `P = R+0x50` is its placement block. Offsets explicitly prefixed `P` are relative
to that block. A getter's boolean result and writes are primitive inputs to the owner contract; a false return
is not assumed to imply an untouched destination unless absence was established. Allocation failure,
exceptions, invalid property storage and foreign parser internals are outside the supported normal domain.

## 2. Construction and initial state

The static initializer `0x00A73D10–0x00A73D3F` constructs records 0 through 12 at `0x00B452C8 + 0x114*t`,
then registers `0x00A7C140` for exit. These are process globals, not city/region objects. The constructor
`0x00641F70–0x00642025` does **not** initialize every placement scalar. In particular it never writes
`P+0x00`, `P+0x04`, `P+0x08` or `P+0x20`. Initial zeros in untouched static storage must not be attributed
to a general-purpose constructor default. *[compat]*

| Constructor action | Evidence |
|---|---|
| Loaded byte `R+4 := 0` | `0x00641F76` |
| Evaluator and bridge vector triples are null/empty | `0x00641F79–0x00641F8B` |
| Support container setup, with initial size argument 100 to `0x00829BC0` | `0x00641F8E–0x00641F9C`; exact allocation/bucket policy is a primitive |
| Null vector triples at `P+0x28`, `P+0x54`, `P+0x60`, `P+0x6C`, `P+0xA0` | `0x00641FA1–0x00641FEF` |
| `R+0x100 = R+0x108 = 10.0` (bits `0x41200000`) | `0x00641FFA–0x00642011` |
| `R+0x104 = R+0x10C = 0x40E24630` (binary32) | `0x00641FF5–0x00642005` |
| `R+0x110 = 20.0` (bits `0x41A00000`) | `0x00642017` |

The five derived fields therefore initially contain nonzero values even though the three source angles in
static storage are zero. They are recomputed only when the placement/derivation wrapper runs (§5).

## 3. Clear and destruction

`Clear` (`0x006421D0–0x0064221A`) does the following in order. *[compat]*

1. Removes evaluator pointers from the back. For each non-null pointer it calls vtable slot zero with deleting
   argument 1, then reduces the vector's logical end by four bytes. Null entries are skipped but removed.
2. Erases the entire bridge range using `0x00641EB0`. Bridge element destruction runs forward through the
   old range in `0x178`-byte steps (`0x00641EE0–0x00641EEF`, element primitive `0x00640230`).
3. Clears the support hash contents through `0x0063D760`. Buckets are traversed in increasing index, each
   node chain in next-link order; each node's `+0x14` allocation, `+8` allocation, then node are freed when
   non-null. Each bucket head is zeroed and the entry count becomes zero (`0x0063D780–0x0063D7D3`).

Clear retains evaluator/bridge backing storage and the support bucket allocation. It does **not** reset the
loaded byte, widths, other scalar metadata, placement scalars, any placement vector, or derived fields. Repeated
Clear on an already empty record calls no evaluator/bridge element destructor. Thus Clear on a loaded record
can leave loaded equal to 1 with no evaluator/bridge/support contents. It is not a constructor reset.

The destructor `0x00643510–0x0064357A` first calls Clear, then destroys the placement block via `0x00624F70`:
free non-null vector storage in order `P+0xA0`, `P+0x6C`, `P+0x60`, `P+0x54`, `P+0x28`
(`0x00624F73–0x00624FC3`). It then clears supports again, frees support bucket storage, destroys any remaining
bridge range forward (normally empty after Clear), frees bridge vector storage, and finally evaluator vector
storage (`0x00643522–0x00643574`). This is ownership destruction, not COM Release on the evaluator pointer.
Foreign evaluator and bridge element destructor internals remain primitives.

At exit `0x00A7C140–0x00A7C162` calls this destructor for records 12 down to 0. Each record's full destruction
finishes before the next record begins. No loaded-byte reset is introduced by these owned bodies.

## 4. Reload publication and outer loaders

`ReloadNetworkTypeInfo` (`0x00623310–0x00623B19`) reloads records in this order:
**road, one-way road, dirt road, rail, light rail, monorail, street, highway, ground highway, avenue, pipe,
subway, power line**. Each record is processed fully before clearing the next. The function has no all-record
staging, loaded-byte admission check, or rollback branch. On a second invocation it repeats the work even if all
loaded bytes were already 1. *[compat]*

For every type except power line the sequence is: Clear; overwrite fixed record metadata and widths;
LoadEvaluator twice in the call-site order; LoadBridgeTypes; LoadSupports if listed below; placement/derivation
wrapper `0x0063D460`; LoadComplete. Street additionally writes `P+0x25 := 1` after derivation and before
LoadComplete (`0x006237AD`). The placement loader itself writes this flag to zero on admitted resources.

| Types | Widths | Support loader |
|---|---|---|
| road, one-way road, dirt road, rail, light rail, monorail, street | 1, 1 | yes, group `0x8974F80F` |
| highway, ground highway, avenue | 2, 2 | no |
| pipe, subway | 1, 1 | no |
| power line | 1, 2 | no |

Power line is only cleared, given fixed metadata and widths, then marked complete (`0x00623AB3–0x00623B02`);
it receives no evaluator, bridge, support or placement/derivation call. Its constructor-derived numbers survive
an ordinary reload. After all thirteen records, the owner calls LoadTilePatternVerifiers with `TileVerifiers`
(`0x00623B07–0x00623B0D`). It does **not** call LoadTileConflictRules in this body; the earlier loader list in
network-occupants §8 was not an exact execution sequence.

LoadComplete is only the byte store `R+4 := 1` (`0x0063BF90–0x0063BF94`). No loader results are tested by the
outer reload body. In particular, failed placement acquisition still permits derivation and publication. A
previous loaded byte remains 1 throughout a later reload; a first reload remains 0 until its own LoadComplete.
These facts do not promise continuation after a crash or exception in a primitive.

### 4.1 Evaluators and duplicate calls

LoadEvaluator (`0x0063D360–0x0063D3D0`) allocates 16 bytes, calls the foreign evaluator constructor/factory
`0x00622A90`, calls AddRulesFromResource `0x00623020`, then appends the pointer. The resource/parser result is
not tested before append (`0x0063D390–0x0063D3C6`). Identical resource arguments in two returning calls still
produce two vector entries; there is no owner-level deduplication. Direct repeated loader calls accumulate;
outer reload first destroys old entries through Clear. The allocation-null branch still passes null to the
foreign AddRules call (`0x0063D37E–0x0063D390`): no safe OOM result is specified.

### 4.2 Bridges, supports and global stores

LoadBridgeTypes (`0x006437A0–0x0064382F`) asks the supplied INI primitive to enumerate the named section with
ReadBridgeTypeInfo and the existing bridge vector, destroys its temporary section string, then calls
ReadBridgeTypeSpecializations with that vector, instance and width. It does not clear the vector, filter
duplicates, or branch on the enumeration result. Enumeration callback order and all parser outcomes are raw
primitive inputs; [network-bridges §2.2](network-bridges.md#22-one-entry-readbridgetypeinfo-0x00642220)
already documents append-before-parse and failed-resource removal. This publication does not re-prove that
foreign parser. Direct repeated LoadBridgeTypes calls reuse the existing vector; ordinary reload starts empty.

LoadSupports (`0x0063F780–0x0063F793`) passes the record's existing map and the group to
ReadSupportStructureInfos `0x0063F700`. It adds no clear, result check or rollback. Duplicate support-key policy
belongs to that primitive and is not established here. Clear removes all previously owned support nodes before
an outer reload requests new ones.

LoadTilePatternVerifiers clears its separate global store `0x00B466DC` through primitive `0x00584660` **before**
enumerating (`0x0063FAF5–0x0063FAFA`, `0x0063FB37–0x0063FB53`). LoadTileConflictRules uses separate store
`0x00B466D0` and contains no such preliminary clear (`0x0063D421–0x0063D43D`). Neither wrapper checks the
enumeration result. These stores are not owned by any one of the thirteen records.

## 5. Placement admission and numeric derivation

LoadPlacementParams is called with **P**, not R, as its destination (`0x0063D468–0x0063D46D`). It requests the
established exemplar TGI and instance through resource-manager slot `+0x0C`, interface `0xA52160F5`, with two
zero arguments (`0x0063CBE0–0x0063CC15`). A false acquisition returns immediately without changing placement
state or releasing an acquired interface (`0x0063CC18–0x0063CC20`). On success it obtains properties through
slot `+0x1C`, processes them, calls resource-interface slot `+8` once and returns AL=1
(`0x0063D060–0x0063D072`). It does not test individual getter failures except for the explicit defaults below.

The wrapper ignores this result and always recomputes the five derived fields. For slope angle `a=P+0x00`,
smoothness angle `b=P+0x08`, and second slope `c=P+0x04`, it forms x87 `tan(angle)*16.0`; it stores the straight
result to binary32 and continues with the **still resident x87 value**, multiplying by the binary32 constant
at `0x00AA8C50`, then stores the diagonal result to binary32. The second slope has only a straight store.
Evidence: `0x0063D472–0x0063D4BD`. The diagonal is not established as a multiplication of the already rounded
straight field. Current x87 control/precision and non-finite FPTAN behavior remain outside bit-exact claims.

On admitted placement resources the five angle fields `P+0x00`, `+0x04`, `+0x08`, `+0x18`, `+0x1C` are each
multiplied by the binary32 constant at `0x00AA8C60` and stored to binary32, even if their getter returned false
(`0x0063CC36–0x0063CC82`, `0x0063CCB5–0x0063CCE8`). `P+0x20` is not converted. Consequently a missing angle
on a later admitted exemplar can reconvert the previous radians as though they were degrees. A missing whole
exemplar does not reconvert; only the derivation wrapper runs. No rollback/default-reset rule should erase
this distinction.

### 5.1 Explicit defaults and vectors

These are loader-time rules, reached only after successful whole-resource acquisition. The established full
property table remains in network-occupants §8; the following corrects or qualifies its defaults. *[compat]*

| Destination / property | Owner rule and evidence |
|---|---|
| `P+0x50`, `0xC804DD9B` | On false getter result store integer 1000 (`0x0063CD0D–0x0063CD19`) |
| `P+0x54`, `0xE974EAA4` | Set vector length zero before getter (`0x0063CD9A–0x0063CDC9`) |
| `P+0x60`, `0xE974EAA5` | Set length zero before getter; on false result append **one zero word per P+0x54 element**, not one scalar equal to count times 10 (`0x0063CDCE–0x0063CE6A`) |
| `P+0x6C`, `0xE974EAA6` | Set length zero before getter; on false result append one binary32 10.0 (`0x41200000`) per P+0x54 element (`0x0063CE6C–0x0063CF0B`) |
| `P+0xA0`, `0xE974EAA7` | Set length zero before getter (`0x0063CF0D–0x0063CF3E`); earlier `p+0x34` attribution in occupants §8 is incorrect |
| `P+0x78`, `0x4A270DB0` | Preinitialize binary32 1.5, then getter (`0x0063CF43–0x0063CF53`) |
| `P+0x7C`, `0x4A270E94` | Preinitialize binary32 32.0, then getter (`0x0063CF58–0x0063CF68`) |
| `P+0x80`, `0x4A271248` | Preinitialize 32-bit zero, then integer getter (`0x0063CF6D–0x0063CF80`) |
| `P+0x84`, `0x4A27124F` | Preinitialize 32-bit **zero**, not 1 (`0x0063CF85–0x0063CF98`) |
| `P+0x88`, `0x4A275748` | Initialize temporary integer to 1, getter, then copy to destination (`0x0063CF9D–0x0063CFC9`) |
| `P+0x8C`, `0x4A2D49B9` | On false result store integer 1 (`0x0063CFCF–0x0063CFDB`) |
| `P+0x90`, `0x8A2D49EA` | On false result store integer 10 (`0x0063CFEE–0x0063CFFA`) |
| `P+0x94`, `0x8A2D49F0` | On false result store integer 20 (`0x0063D00E–0x0063D01A`) |
| `P+0x98`, `0x6A2D49F4` | On false result store binary32 0.1 (`0x3DCCCCCD`, `0x0063D02E–0x0063D03A`) |
| `P+0x9C`, `0xEA86E3B5` | Getter destination is +0x9C; on false result the owner writes **P+0x8C := 1**, leaving +0x9C as the getter left it (`0x0063D041–0x0063D05A`) |

`P+0x28` (`0xC804DD92`) is a vector getter too, but has **no** preliminary length reset (`0x0063CD1F–0x0063CD29`).
The four later vectors therefore differ from it on absent properties. The owner does not check that successful
vectors have equal lengths, pad a successful short vector, or invoke fallback for a successful empty vector.
The two fallback loops use the `P+0x54` count, not the lengths of either companion vector.

Scalar wrappers `0x005FD4F0`, `0x005FD480`, `0x005FD390` return false without touching the destination when
property lookup yields no object. For present objects they dispatch conversion through variant slots `+0x94`,
`+0x64`, `+0x28` respectively. Those conversions are primitives; wrong-type/empty/vector-to-scalar coercion
is not proven here. Vector wrapper `0x005FF510` likewise returns false without writes on absent property;
on a present property it uses variant raw storage/count, resizes and copies that many four-byte words, then
returns true (`0x005FF521–0x005FF56C`, `0x005FF5B8–0x005FF5C4`). Thus a valid present zero-count vector is
successful and empty. Variant behavior for malformed or mismatched types is unresolved, not a safe rejection
contract. All property getter/allocation internals remain outside the owned algorithm inventory.
