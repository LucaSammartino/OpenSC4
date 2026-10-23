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
to a general-purpose constructor default. PE section inspection independently places the entire array in the
zero-filled virtual tail of `.data`, beyond its raw file bytes. This establishes zero before static construction,
including the untouched placement scalars; it does not establish reset-to-zero on reload. *[compat]*

| Constructor action | Evidence |
|---|---|
| Loaded byte `R+4 := 0` | `0x00641F76` |
| Evaluator and bridge vector triples are null/empty | `0x00641F79–0x00641F8B` |
| Support container setup, with initial size argument 100 to `0x00829BC0` | `0x00641F8E–0x00641F9C`; setup explicitly zeros entry count at `0x00829C1E`; allocation/bucket policy is a primitive |
| Null vector triples at `P+0x28`, `P+0x54`, `P+0x60`, `P+0x6C`, `P+0xA0` | `0x00641FA1–0x00641FEF` |
| `R+0x100 = R+0x108 = 10.0` (bits `0x41200000`) | `0x00641FFA–0x00642011` |
| `R+0x104 = R+0x10C = 0x40E24630` (binary32) | `0x00641FF5–0x00642005` |
| `R+0x110 = 20.0` (bits `0x41A00000`) | `0x00642017` |

The five derived fields therefore initially contain nonzero values even though the three source angles in
static storage are zero. They are recomputed only when the placement/derivation wrapper runs (§5).

The loaded flag is a byte at `R+4`; the separate dword `R+0` is fixed type metadata, not the loaded flag.
Untouched metadata and widths initially inherit static zero, then reload overwrites them. Constructor-time
support-container allocation failure is not proven to complete construction safely.

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

The following are the fixed dword metadata writes, in record-index order, rather than an inferred semantic
name for each field. Other metadata bytes are not reset. Values are hexadecimal except `R+0`.
Evidence: stores throughout `0x00623331–0x00623AFC`; widths are as above.

| Type | R+0 | R+8 | R+0x0C | R+0x10 | R+0x14 | R+0x18 |
|---|---|---|---|---|---|---|
| road | 1 | A766C101 | 2821ED93 | 0 | E17BB266 | 49E13260 |
| rail | 2 | C8433ECB | E8347989 | 0 | 217BB308 | 0 |
| highway | 0 | 08433F47 | A8434037 | 0 | 8BF30A8B | 0 |
| street | 1 | 092A047D | A92A02EA | 0 | 2A920BF9 | 0 |
| pipe | 1 | 09511847 | A951171C | F0 | 82C2FF49 | 0 |
| power line | 0 | A88E1592 | 088E1962 | 0 | A16F1463 | 69E13770 |
| avenue | 0 | 0B730A26 | CB730FAC | 0 | 6B730B83 | 0 |
| subway | 1 | 8A15EE13 | 8A15F3F2 | F1 | C17BB329 | 0 |
| light rail | 2 | 0B79E2E1 | 2B79DFFB | 0 | AB79E761 | 0 |
| monorail | 2 | ABE08433 | EBE084C2 | 0 | EBE091B6 | 0 |
| one-way road | 1 | 2BE08438 | CBE084CB | 0 | ABE091A8 | 0 |
| dirt road | 1 | 4BE08AFB | 6BE08658 | 0 | 4BE091B2 | 0 |
| ground highway | 0 | 6BE0843B | EBE084D1 | 0 | 6BE091B4 | 0 |

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
multiplied by binary32 `0x3C8EFA35` (0.01745329238474369) at `0x00AA8C60` and stored to binary32, even if their getter returned false
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

## 6. Independent literal vectors — UNEXECUTED

Every vector below is a proposed static conformance vector, derived from original instructions before any
implementation output was consulted. **UNEXECUTED** means no original-program run, engine test, differential
result or native proof. Named pointers denote distinct primitive-created objects, not literal process addresses.
All primitives return normally unless a row explicitly stops at an unproved failure boundary. *[compat]*

### 6.1 Construction, clear and repeated reload

| Vector | Given / operation | Expected owned trace and state |
|---|---|---|
| TI-V01 | Fresh static initialization | Construct 0…12; loaded bytes all 0; evaluator/bridge/placement vectors empty; support map empty after setup; angles and untouched P+0x20 are zero; derived words in offset order +100,+104,+108,+10C,+110 are `41200000,40E24630,41200000,40E24630,41A00000` |
| TI-V02 | Loaded=1; evaluators `[E1,null,E2]`; bridges `[B1,B2]`; support nodes in buckets 0:`S1→S2`, 2:`S3`; Clear | Delete E2 then E1; destroy B1 then B2; free S1's +0x14 allocation, +8 allocation, node, then S2 similarly, then S3; skip null allocations; all three containers empty, backing storage retained; loaded=1, all P fields/vectors and derived words unchanged |
| TI-V03 | Repeat Clear after V02 | No evaluator/bridge/node deletion; support count still zero; loaded still 1; same P/derived state |
| TI-V04 | Exit after a completed reload | Destroy records 12…0; within each record Clear as V02, placement vector backing buffers in order A0,6C,60,54,28, support bucket storage, bridge backing storage, evaluator backing storage; normally no second bridge-element destruction |
| TI-V05 | First road reload: foreign evaluator creations E1,E2; property resource acquisition false | Old containers cleared; append E1,E2; bridge/support primitive outputs retained; P stays zero; wrapper replaces all five nonzero constructor-derived words with +0; loaded changes 0→1 only at road LoadComplete |
| TI-V06 | Later road reload after E1,E2 were loaded; new creations E3,E4; property acquisition false | Delete E2 then E1 before new loading; append E3,E4; previous bridge/support contents destroyed; P scalars and placement vectors retained; derived fields recomputed from retained angles; loaded stays 1 throughout |
| TI-V07 | Power-line reload after fresh static construction | Empty containers cleared; metadata/widths overwritten to §4 values; loaded becomes 1; P remains zero/empty, five constructor-derived words remain V01's nonzero words |
| TI-V08 | Two direct LoadEvaluator calls with identical resource arguments, creations E1,E2, foreign AddRules returns failure each time | Append `[E1,E2]`; no deduplication or owner-side deletion on parser result; loaded unchanged; subsequent Clear deletes E2,E1 |
| TI-V09 | Reload road then one-way; one's placement acquisition fails but other primitives return | Road is already published when one-way is cleared; one-way still derives and publishes; remaining types proceed in §4 order; final verifier-store clear/enumeration occurs after power-line publication |
| TI-V10 | Direct LoadPlacementParams with whole-resource failure, loaded=0 or 1 | P unchanged; AL=false; no property calls or success-path release; loaded and derived fields unchanged because wrapper was not invoked |

In V05 the parser/resource failures of the two evaluator calls may still yield two empty evaluators: their
contents are primitive outputs. A resource lookup's false return alone is not evidence that no evaluator was
created or appended. In V06 no old evaluator/bridge/support references survive successful Clear; placement
vector allocations and their contents can survive failed whole-exemplar acquisition.

### 6.2 Property absence, empty vectors, defaults and angles

These vectors call the admitted placement loader with missing properties unless otherwise stated. `A4`, `A5`,
`A6`, `A7` abbreviate `0xE974EAA4`…`0xE974EAA7`. A present valid vector is supplied through the primitive's
four-byte storage/count; no scalar-to-vector type coercion is assumed.

| Vector | Given / primitive results | Expected owned output |
|---|---|---|
| TI-V11 | Fresh P, all property objects absent | Early scalars stay zero, angle stores remain +0; P+25=0; P+50=1000; all five vectors empty; P+78=1.5, +7C=32.0, +80=0, +84=0, +88=1, +8C=1, +90=10, +94=20, +98=`3DCCCCCD`, +9C remains zero; return true and release once |
| TI-V12 | Prior P+28=`[7,8]`, A4/A5/A6/A7 nonempty; all their property objects now absent | P+28 retains `[7,8]`; A4,A5,A6,A7 destination lengths become zero; companion fallback loops have zero iterations |
| TI-V13 | A4 valid count 2, raw words `[00000011,00000022]`; A5,A6 absent | P+54 has two words; P+60=`[00000000,00000000]`; P+6C=`[41200000,41200000]` |
| TI-V14 | Same A4 as V13, A5 present valid count 0, A6 present valid count 1 with word `40E00000` | P+60 is empty, P+6C length is 1; no padding, fallback or length-equality rejection |
| TI-V15 | A4 present valid count 0; A5 present count 3 with words `[1,2,3]`; A6 absent | Lengths are 0,3,0; explicit A5 survives even with no A4 entries |
| TI-V16 | Scalar float getter for P+0 returns true, writes 30.0 (`41F00000`); plane getter writes 35.0 (`420C0000`) | P+0=`3F060A92`; P+20=`420C0000` without radians conversion; under round-to-nearest float stores |
| TI-V17 | Later admitted resource lacks the P+0 property after V16 | Absent scalar wrapper leaves radians `3F060A92`, then owner converts again: P+0=`3C15B9BE`; if whole resource was absent instead, P+0 would remain `3F060A92` |
| TI-V18 | Scalar angle getter writes 90.0 (`42B40000`) | Stored angle `3FC90FDB` under round-to-nearest float stores; no literal tangent/derived bits asserted near π/2 |
| TI-V19 | 0x4A2D49B9 succeeds with 9; 0xEA86E3B5 absent; previous P+9C=77 | Final P+8C=1 (overrides 9), P+9C=77; missing-property fallback targets a different field |
| TI-V20 | 0x4A2D49B9 succeeds with 9; 0xEA86E3B5 succeeds with 77 | Final P+8C=9, P+9C=77; no cross-field fallback |
| TI-V21 | Wrong-type scalar primitive returns false and leaves P+84 untouched | Since owner preinitialized P+84 to zero, final value is zero; there is no claimed rule that every wrong type returns false or preserves output |
| TI-V22 | A5 primitive returns false after writing one word `99`; A4 count 2 | Owner appends two zero words to that primitive result, producing `[99,0,0]`; false result does not itself prove rollback of partial getter writes |

Angle vectors use the measured binary32 conversion constant and a binary32 final store; integer defaults are
not float encodings. The `30*constant` product is exactly representable within binary64, so the listed angle
words are independent static arithmetic, not tan-library approximations. Derived all-zero V05 is exact in the
normal finite zero domain. Nonzero derived bit patterns await precision question TI3.

### 6.3 Primitive-boundary failure vectors

| Vector | Given / operation | Supported owned conclusion / unresolved boundary |
|---|---|---|
| TI-V23 | Direct LoadBridgeTypes twice; INI primitive calls callback twice each time, in order K1,K1; callback appends one record per call | Four records in callback order; wrapper neither clears nor deduplicates; specialization runs after each enumeration. Ordinary outer reload first erases the old range. This is conditional on callback outputs, not a claim about every duplicate parser key |
| TI-V24 | Bridge enumeration returns failure after callback produced B1; specialization returns normally | No owner-side removal of B1 on enumeration result; specialization still called; outer reload can publish. Parser-owned removals remain primitive outcomes |
| TI-V25 | Support enumeration returns normally with only S1 inserted, rather than all requested nodes | S1 remains; owner performs no result-based rollback. A later outer reload first frees S1. Direct LoadSupports repeats into existing map; duplicate-key replace/merge/reject is unknown |
| TI-V26 | Global verifier enumeration returns normally with zero callbacks | Previous store was cleared before enumeration; no owner-side restoration. Record loaded flags remain as published |
| TI-V27 | Evaluator allocation returns null | Owner passes null to AddRulesFromResource; no proved normal return, crash-free completion or loaded outcome beyond the already reached point. Earlier records remain published, prior cleared objects stay destroyed; exact nonreturn/exception behavior is TI2 |

## 7. Body boundary and publication inventory

Status **specified** means the owned normal-return state/call trace is described above with primitive outputs
as inputs. **Partial** means an explicit precision/failure or foreign-content boundary remains. **Unknown**
means no algorithm claim is made. Sizes below are individual canonical export function-body sizes, not coverage
credit, and are not summed. Reach aliases are not additional native bodies. In particular historical attribution
of **20 rows / 4,836 bytes** does not measure a complete native manager implementation.

| Native body | Export bytes | Status / evidence and boundary |
|---|---|---|
| 0x00641F70 constructor | 182 | specified, §2; support setup primitive, no OOM claim |
| 0x00A73D10 static initializer | 48 | specified, §2; 13 forward constructions and exit registration |
| 0x00A7C140 reverse disposal | 35 | specified, §3; 13 reverse destructions |
| 0x006421D0 Clear | 72 | specified, §3; contents, retained capacities/state, ordered calls |
| 0x00643510 destructor | 107 | specified, §3; foreign element destruction treated as primitive |
| 0x0063BF90 LoadComplete | 5 | specified, §4; byte store only |
| 0x00623310 ReloadNetworkTypeInfo (NetworkTool owner) | 2058 | specified owned outer body, §4; no branches on loader results; excluded primitive failures TI2 |
| 0x0063CBE0 LoadPlacementParams | 1171 | partial, §5; owned writes/defaults/order specified, variant conversion domain TI1 |
| 0x0063D460 placement + derivation wrapper | 103 | partial, §5; stores/operation order specified, x87 environment TI3 |
| 0x0063D360 LoadEvaluator | 113 | partial, §4.1; append/ownership specified, null allocation TI2; foreign evaluator algorithm excluded |
| 0x0063F780 LoadSupports | 20 | partial, §4.2; forwarding specified, support-map duplicate/parser outcomes TI4 |
| 0x006437A0 LoadBridgeTypes | 144 | partial, §4.2; enumeration/specialization order specified; foreign contents excluded |
| 0x0063D3E0 LoadTileConflictRules | 123 | partial, §4.2; global-store forwarding/no preliminary clear specified; callback contents excluded |
| 0x0063FAF0 LoadTilePatternVerifiers | 129 | partial, §4.2; preliminary store-clear/callback order specified; callback contents excluded |
| 0x00624F70 placement-vector destruction helper | 88 | specified lifecycle use, §3; five backing-buffer frees in reverse field order |
| 0x00641EB0 bridge-range erase helper | 79 | specified full-range use, §3; no general nonempty suffix-move contract claimed |
| 0x006412B0 bridge range copy/move helper | 83 | partial: full Clear supplies empty suffix; general assignment algorithm unknown and uncredited |
| 0x0063D760 support-container clear helper | 117 | specified lifecycle use, §3; bucket/chain/allocation free order |
| 0x00622A90 evaluator constructor/factory | not counted | foreign evaluator body, **unknown here**, despite reach attribution to TypeInfo; not owned manager algorithm |
| 0x00640230 bridge element destruction | not counted | foreign content primitive, **unknown here** beyond owned call order |
| 0x00642220, 0x006436D0, 0x0063F700, 0x0063D1E0, 0x0063F7A0 | not counted | foreign bridge/support/conflict/verifier callback bodies, **unknown here**; prior specs retain their established claims |

This is an address-qualified inventory of the bodies actually needed for this publication, not a claim to
exhaust or complete an old reach-based twenty-row family. Allocation/container/property utilities, callback
content algorithms and their reachable callees are not counted as newly specified TypeInfo algorithm bytes.
No ABI rows, maps or generated coverage are changed.

## 8. Unresolved domain

[TI1–TI4](questions/network-type-info.md) bound present-but-wrong-type and scalar/vector variant behavior,
nonreturning failure paths and allocation safety, x87 environment/derived exact bits, and duplicate support-key
outcomes. In particular no strong rollback, safe OOM, resource-type rejection policy or exact tangent golden
bits are inferred from a successful decompilation or from these UNEXECUTED vectors. A replacement's internal
container representation is a *[choice]*; the ordered state/ownership effects above are *[compat]*.
