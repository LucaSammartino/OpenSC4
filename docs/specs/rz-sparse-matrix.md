# RZ sparse matrix and column iterator (partial contract)

Written by RE lane `re-rz-sparsematrix4`, 2026-10-23, original final round 3/3.
Outline: §1 identity and limits; §2 representation and coordinates; §3 lookup and counts;
§4 mutation; §5 range output and extrema; §6 column iteration; §7 lifetime and missing operations;
§8 proposed literal vectors; §9 exact-body ledger and remaining attribution.

## 1. Identity and limits

Primary evidence is the Windows export and instruction bytes queried with `tools/re_query.py`,
supported by existing matching decompilation. The current `bin/SimCity4.exe` SHA-256 equals
`ghidra/export/windows/export-summary.txt`'s executable hash:
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
The binary version resource is `1.1.641.0`, original filename `SimCity 4.exe`; export image base
is `00400000`. Addresses below are virtual addresses in this binary.

Round 1's live REA request failed with **No app is open** (`281dd777`). Round 2's isolated
coordinator preflight failed against its inaccessible global npm context (`b1304240`);
neither round published behavior. Round 3 consumes copied coordinator host admission, full
framework services (RZ) guard exit 0 and the actual own round-3 manifest. The supplied successful
Catalog context fixture establishes the admission workflow only. Live target availability is
UNKNOWN; no live retry or target-control operation was performed or required for these primary queries.

This is a bounded contract for observed sparse coordinate/payload operations. It is not complete
matrix-owner coverage, an implementation prescription, a matrix resizing contract, or permission to
substitute a library container's behavior. All success descriptions involving foreign tree insertion
or erasure assume those operations return normally with a valid tree. Allocation failure, exceptions,
invalid pointers and corruption are unresolved, not modeled as callbacks that manufacture success.
See [open questions](questions/rz-sparse-matrix.md).

## 2. Representation and coordinates

Coordinates are **signed 32-bit column, row**, in that argument order for two-coordinate methods.
An outer ordered tree indexes rows; each row contains an ordered tree indexing columns. A row's
matrix-record address is its outer node plus `0x14`: the row key is at record `+0`, its inner tree
at `+4`, and its cell count at `+8`. A cell record is its inner node plus `0x14`: the column key is
at `+0`, its payload at `+4`. The observed pointer specialization carries one 32-bit payload.
The `CellFormatExtra` insertion carries ten 32-bit words (§9 partial entry).

This ordering is grounded in the shared lookup's signed comparisons at `0094c908–0094c925`,
left/right links `+8/+0xC`, and forward traversal at `0040d4c0–0040d4f5` (right subtree's
leftmost node, otherwise parent ascent). Its export name resembles a destructor, but the instructions
are a node-successor operation. `0040d500–0040d53f` performs predecessor traversal and recognizes
the tree header. These are foreign/shared primitives, not matrix bodies credited to this owner.

Matrix offset `+0` is the outer header pointer and `+0xC` is the total stored-cell count in the
observed mutation paths. Insertion checks signed inclusive limits: row minimum `+0x10`, row maximum
`+0x14`, column minimum `+0x18`, column maximum `+0x1C` (`009ac164–009ac186`). These are insertion
bounds, not proof of allocated dimensions, a default extent, or coordinate renumbering behavior.
Sparse holes are absent entries, not implicitly stored zero/default payloads. Occupancy queries count
entries without testing their payload, including entries whose pointer payload is zero.

## 3. Lookup and counts

### 3.1 Cell address lookup

The actual lookup body named `CellFormatExtra::GetCellPtr` is `009a9bd7` (63 bytes).
It looks up the row, then the column; on either miss it returns false in **AL** and leaves the
optional output cell unchanged. On success it returns true in AL, and, if output is nonnull,
writes the address of the stored payload, not a copy of its value (`009a9bfd–009a9c0a`). A null
output therefore permits an existence-only query. No insertion or default construction occurs.
The returned address aliases matrix storage. Its validity after mutation is not established.

Its row helper `009a94a6` and column helper `009a947f` return record addresses after exact signed-key
lookup; their nonnull outputs are mandatory on successful helper calls. Both suppress the output
store on miss. The enclosing lookup supplies its own local output, so this does not require a
nonnull public lookup output. The helpers use the same `0094c8f5` exact-match body.
The exported pointer-specialization label `GetCellPtr` at `009a5bf2` is **not** a lookup (§9).
Do not claim a separately located pointer-specialization lookup from that label.

### 3.2 Occupancy counts

- `GetColCountForRow(row)`, `009a9b7d`: EAX is zero on missing row, otherwise the row record's
  `+8` count. This is occupied columns, not the column span.
- `GetRowCountForCol(column)`, `009a9b9e`: walks rows in order, exact-looks up that column in each,
  and increments a 32-bit EAX result once per hit. Payloads are never inspected.
- `GetCellCountForRange(cMin,cMax,rMin,rMax)`, `009a9280`: walks rows from smallest to largest,
  skips rows below rMin and stops at the first row above rMax. Within eligible rows it skips columns
  below cMin, counts inclusively through cMax, then stops that row at the first column above cMax.
  Empty or inverted bounds produce zero; no bounds normalization occurs. EAX is the 32-bit count.

All comparisons are signed (`009a9297–009a92b4`). The increment is 32-bit, without overflow
detection (`009a92b6`). The `DeleteColumns` caller passes `[start,start+count−1]` and
`[0,0x7FFFFFFF]` for rows (`009ae5ef–009ae603`), corroborating column-first, inclusive endpoints.
This caller restriction does not make the matrix's own comparisons unsigned.

## 4. Mutation

### 4.1 Pointer-payload insertion or replacement

`InsertCell(column,row,inputPayloadAddress,optionalOldPayloadAddress)`, `009ac157`:

1. Check row lower, column lower, row upper, column upper in that order, using signed comparisons.
   An out-of-bounds coordinate returns AL=false before lookup or dereferencing either payload
   address; no count or output write occurs on that path.
2. Find the row, then its column. For an existing cell, if the old-payload output is nonnull,
   copy the old 32-bit payload there **before** loading the 32-bit input payload. Store that input
   into the existing cell. The total count does not change. Return AL=true.
3. For an existing row with no such column, load the 32-bit input, form a cell record with that
   column, invoke unique insertion, and increment matrix count once. The optional old output is
   untouched. Return AL=true on normal helper return.
4. For a missing row, construct a temporary empty row, copy/insert it into the outer tree, then
   load input, insert its cell and increment matrix count once; clean up the temporary row.
   The optional old output is untouched. Return AL=true on normal helper return.

The replacement order is explicit at `009ac1ab–009ac1bc`. Thus, when the input and old-output
addresses are the same valid independent word, that word first receives the old value, and the
matrix is assigned that old value too. Do not pre-read the input before the optional output store.
The missing-cell/new-row branches are at `009ac1c1–009ac23b`. Their helpers' result flags are
not checked before the count increment. This does not establish allocation-failure success or
rollback. No payload AddRef/Release is called directly by this wrapper.

### 4.2 Removal

`RemoveCell(column,row,optionalOldPayloadAddress)`, `009ab6b4`, finds row then column. A miss
returns AL=false with count and output untouched. A hit first copies the 32-bit payload to a
nonnull output, erases the inner node, decrements total count, and erases the row if its remaining
cell count is zero; then returns AL=true (`009ab6dc–009ab70a`). No shifting of other keys occurs
in this wrapper. Optional output can be null. Ownership of pointed-to payload objects is unresolved.

`RemoveRow(start,count)`, `009ab716`, forms a 32-bit wrapping endpoint `start+count` and visits
successive signed row keys from start while key < endpoint. For each existing row it subtracts
that row's cell count from total, then erases the row. Missing keys are skipped. It returns AL=true
even when nothing is removed. Zero count is a no-op; negative counts normally skip the loop, but
the exact rule is the signed comparison against the **wrapped** endpoint, not a count<=0 check.
Remaining row keys are not renumbered (`009ab723–009ab757`).

`RemoveCol(start,count)`, `009ab75e`, forms the same wrapping endpoint and visits each row in
ascending order. Within a row it exact-looks up every successive column key < endpoint, erases
each hit and decrements total once per hit. It then removes an empty row. Before erasing such a
row it computes the next outer node; it does not ask for the successor of an already erased node
(`009ab7b4–009ab7d9`). Returns AL=true even on an empty matrix or empty interval.
No remaining column is renumbered. These are removal wrappers, not grid-level DeleteColumns/Rows.

Cell destruction, deallocation and exceptional effects inside the shared erasers are partial.
Do not infer that a pointer payload is freed or reference-counted merely because its node is erased.

### 4.3 CellFormatExtra insertion: owned copy sequence only

The complete owned wrapper at `009ac02f` applies the same signed insertion bounds and row-then-column
lookup. Its payload is **40 bytes**, copied as ten 32-bit words. Existing-cell replacement first
copies the old payload to a nonnull optional output, then copies input into the cell
(`009ac089–009ac0a1`). Under the normal x86 calling convention's clear direction flag these are
forward word copies, not overlap-safe moves. In particular, if input and old-output are the same
independent 40-byte buffer, replacement restores the old value to the cell, just as §4.1 does
for a single word. Count is unchanged on replacement.

For a new cell in an existing row, it builds a local column-plus-ten-word record, copies that
eleven-word record into the insertion argument, calls `009a94ca`, then increments total
(`009ac0ab–009ac0d7`). For a missing row it first creates/copies/inserts a temporary row through
`009a9c16`, `009ab2b6`, `009abb8d`, then builds/inserts the same cell record and increments total;
temporary-row cleanup calls `009a9427` (`009ac0dc–009ac145`). Old output is not written on either
new-cell branch. Normal completion returns AL=true; bounds rejection returns AL=false before input
access (`009ac14a–009ac154`). These owned sequences are supported; foreign helper allocation,
copy-construction, cleanup and failure guarantees remain partial. No full specialization equivalence
or guarantee for overlapping live matrix storage is established.

## 5. Range output and extrema

### 5.1 Range records

`GetCellRange(cMin,cMax,rMin,rMax,optionalOutput)`, `009a92db`, uses the same signed inclusive
selection as the range count. For each hit it appends one **12-byte** record of three 32-bit words:
column, row, stored pointer payload. Order is ascending row, then ascending column. EAX is the
number of hits. A null output counts without writing. A nonnull output must have room for every
hit: there is no capacity argument/check (`009a9309–009a9335`). It copies payload values,
not their addresses. The caller sizes storage from count ×12 (`009ae608–009ae60e`). Empty/inverted
ranges write no records. Behavior for an output overlapping live matrix storage is unestablished.

### 5.2 Used-coordinate extrema

`GetMinMaxUsedColForRow(row,minOut,maxOut)`, `009a9a89`, first stores INT_MAX (`0x7FFFFFFF`)
and INT_MIN (`0x80000000`) respectively. Missing row returns AL=false with these sentinels.
For an existing nonempty row it writes first and last occupied column, in that order, and returns
AL=true (`009a9ab5–009a9ad0`). Both outputs must be writable. Existing empty-row behavior is not
a portable guarantee because the wrapper dereferences header-derived data without checking emptiness.

`GetMinMaxUsedRowForCol(column,minOut,maxOut)`, `009a9ada`, initializes the same sentinels,
scans rows forward for the first hit, and on success writes that row to minOut and maxOut.
It then scans backward from the largest row to find the last hit and updates maxOut. Returns
AL=true if any row contained the column; otherwise false and sentinels remain
(`009a9b00–009a9b71`). No payload test occurs. These two routines store min before max, so
with the same valid output word the final value is the max (INT_MIN on absence).

`GetMaxUsedCol(rowOut)`, `009a9225`, first sets the required row output and running result to
INT_MIN. For each row in ascending order, read that row's largest column. Update result and rowOut
if the column is greater than the running result **or the running result is still INT_MIN**
(`009a9253–009a9264`). Return column in EAX. Empty matrix leaves both INT_MIN. Equal maxima
normally keep the earliest row; equal INT_MIN maxima keep replacing rowOut until a larger
column occurs. Nonempty-row invariant is required for the header-predecessor read.

## 6. Column iteration

Observed iterator storage is three 32-bit fields: matrix address, current outer row node,
current inner cell-record address. A current record contains the fixed column key; there is no
separately stored column argument. The payload is at that record +4. These addresses alias the matrix.

`col_begin(outputIterator,column)`, `009a9122`, scans rows in ascending order and scans each row's
columns in ascending order until exact column equality. It does not stop early at a larger column.
On a hit it writes all three fields, with the record address at node+0x14. On exhaustion it writes
matrix, outer header, null record. EAX is the supplied output iterator address; the two stack
arguments are consumed (`009a915c–009a917b`). `SortColumn` supplies its column argument and builds
that same end tuple directly (`009acf36–009acf52`).

Prefix increment, `009a8fec` (82 bytes), starts by advancing the outer row once, then skips rows
without the current record's column. Within each candidate nonempty row it reads the fixed column
from the previous record and scans for equality. On a hit it replaces the record address; on outer
header exhaustion it clears the record field. It returns the original iterator address in EAX.
It never produces another cell from the same row. It skips holes, not null payloads
(`009a8ff3–009a903c`). `SortColumn` calls it at `009acf7f`.

The body labeled `col_end` at `009a7577` actually **compares two iterator records**. It returns
true if both current-record fields are null, regardless of matrix or row fields; otherwise true
only if both row-node and record addresses match. The matrix fields are not compared.
False is EAX=0; callers should consume the boolean in AL rather than depend on unpromised upper
bits on the short true path (`009a7577–009a75a0`). The constructor-like `009a5c36` is unrelated.

Incrementing end, retaining an iterator across erasure, or comparing unrelated live matrices is
not promoted to a safe portable operation. There is no null-record validation in increment; it may
read through the previous record. Mutation invalidation and error guarantees remain questions.

## 7. Lifetime and missing operations

The 26-byte entry named matrix destructor at `0095a0d9` checks the byte at object +0x20. If nonzero
it calls `009599a9`, zeros the count at +0xC and clears that byte. It returns AL=true whether the
byte was already clear or not. This conditional wrapper does not itself prove deallocation, payload
destruction, constructor defaults, idempotence of arbitrary underlying state, or whole-object lifetime.
Its helper and its relation to the WinGrid pointer specialization remain partial.

No matrix constructor, default bounds, resize, clear-all, deep copy or assignment contract has been
established here. The pointer insertion's temporary row copy is not matrix copying. The record
initializers misnamed as matrix methods (§9) cannot supply matrix defaults. In particular, absence
does not warrant an invented default-valued successful lookup. Foreign AutoPtr lifetime and
cGZWinGrid resizing behavior are outside this owner's publication.

## 8. Proposed literal distinguishing vectors (UNEXECUTED)

These are deductions from the cited branches/constants, not game/native measurements, validated
test vectors, default promotions, or coverage credit. Each fixture assumes a valid matrix satisfying
the tree invariants and insertion bounds permitting the listed coordinates. A, B, C, D below are
opaque payload words `0x11`, `0x22`, `0x33`, `0x44`; no dereference is needed.

| ID | Literal setup and operation | Proposed result and distinction |
|---|---|---|
| SM-V1 | Entries (column,row,payload): (2,-1,A), (-3,0,B), (2,0,0), (5,3,D). Count range [-3,2] × [-1,0]. | EAX=3; includes negative keys, both endpoints and zero payload. Invert column bounds to [2,-3]: 0. |
| SM-V2 | Same entries, GetCellRange [-3,5] × [-1,3]. | Four records in order (2,-1,0x11), (-3,0,0x22), (2,0,0), (5,3,0x44); null output still returns 4. Distinguishes row-first from column-first ordering and stored value from address. |
| SM-V3 | Same entries, col_begin(column=2), then increment twice. | Rows -1, 0, then end; zero payload at row 0 is visited. A begin for column 4 is end. |
| SM-V4 | Existing cell (2,0) has payload 0x11; input and old-output are the same independent word initially 0x22. InsertCell(2,0,&word,&word). | AL=true, word=0x11, stored payload=0x11, count unchanged. Distinguishes store-old-before-read-input. |
| SM-V5 | Bounds columns [0,2], rows [0,2]; old-output=0x55. InsertCell(3,1,valid input 0x22,&old). | AL=false; count and old-output unchanged. Boundary (2,2) instead accepted on normal insertion-helper success. |
| SM-V6 | Entries (0,1,A), (0,2,B), (0,3,C). RemoveRow(1,2). | Rows 1 and 2 removed; row 3 retains key 3; count=1; AL=true. Zero count removes nothing. |
| SM-V7 | RemoveRow(start=0x7FFFFFFF,count=1) on a valid matrix. | Endpoint wraps to INT_MIN, signed start<end is false: no removals, AL=true. No saturating endpoint. |
| SM-V8 | Entries (1,0,A), (2,0,B), (1,3,C). RemoveCol(1,1). | Leaves (2,0,B); row 3 erased; count=1; column 2 unchanged. |
| SM-V9 | Empty matrix: row-extrema for column 7; column-extrema for row 7; GetMaxUsedCol. | First two AL=false with (INT_MAX,INT_MIN); last EAX=INT_MIN and rowOut=INT_MIN. Required outputs are valid writable words. |
| SM-V10 | Entries (INT_MIN,1,A), (INT_MIN,2,B), assuming bounds admit them. GetMaxUsedCol. | EAX=INT_MIN, rowOut=2, owing to the sentinel override. With maximum 5 at both rows instead, rowOut=1. |
| SM-V11 | Two valid iterator records with record=null and different matrix/row fields. Compare via 009a7577. | AL=true. For nonnull record fields, both row and record addresses must match; matrix fields still ignored. |
| SM-V12 | RemoveCell missing (9,9), old-output initially 0x55. | AL=false, output remains 0x55. Removing the only existing cell returns true, copies its payload, decrements total and erases its row on normal eraser return. |
| SM-V13 | Existing E payload words are [1,2,3,4,5,6,7,8,9,10]. Input and old-output are the same independent buffer initially [11,12,13,14,15,16,17,18,19,20]. Replace that coordinate. | AL=true; buffer and stored payload both [1,2,3,4,5,6,7,8,9,10]; count unchanged. Distinguishes old-copy-first from input snapshot. |

## 9. Exact-body ledger and remaining attribution

The public attribution is matrix **38 / 3276 bytes**, iterator **1 / 82**, combined **39 / 3358**.
This is not a demonstrated unique-Windows-body denominator. Canonical name search, including
`--all`, returns **18 matrix-labeled entries**, one iterator and two foreign AutoPtrs. No matching
`cRZSparseMatrix` row occurs in `re/symport/folded.tsv`; this is absence of a recorded folded row,
not proof that no code sharing exists. The twenty-entry difference between 38 and 18 remains
unreconciled; it cannot be assigned algorithms or bytes by subtraction alone.

S = supported bounded normal-path behavior; P = partial, not completed-body credit;
F = foreign/shared or misattributed to this owner. Template names below abbreviate pointer payload
as P and CellFormatExtra as E. Each row is an exact Windows entry, not an extra template instance.

| Address | Bytes | Export label | Classification and scope |
|---|---:|---|---|
| 0095a0d9 | 26 | WinGridCellData destructor | P: conditional cleanup wrapper only, §7; helper unresolved |
| 009a5bf2 | 46 | P GetCellPtr | F: record initializer, not coordinate lookup; 0 at +0, 0xFF/zero bytes at +0x1C…1F, 0x40000000 at +4…+0x18 and +0x20; callers construct temporary cell data |
| 009a5c36 | 20 | E col_end | F: record initializer calls 009a5bf2, sets two 16-bit fields +0x24,+0x26 to 0xFFFF; returns record address |
| 009a7577 | 44 | P col_end | S, shared/misnamed iterator equality, §6; no end construction |
| 009a9122 | 91 | P col_begin | S: fixed-column scan/output tuple, §6 |
| 009a9225 | 91 | P GetMaxUsedCol | S: extrema and sentinel override, §5.2; nonempty rows |
| 009a9280 | 91 | P GetCellCountForRange | S: signed inclusive selection/count, §3.2 |
| 009a92db | 130 | P GetCellRange | S: selection and ordered 12-byte records, §5.1 |
| 009a9a89 | 81 | P GetMinMaxUsedColForRow | S: sentinels, existing nonempty rows, §5.2 |
| 009a9ada | 163 | P GetMinMaxUsedRowForCol | S: forward first/backward last match, §5.2 |
| 009a9b7d | 33 | P GetColCountForRow | S: exact row/count, §3.2; helper sharing |
| 009a9b9e | 57 | P GetRowCountForCol | S: per-row occupancy, §3.2 |
| 009a9bd7 | 63 | E GetCellPtr | S: optional address output/existence, §3.1; pointer-type alias not separately proven |
| 009ab6b4 | 98 | P RemoveCell | P: owned branch/output/count ordering §4.2; foreign destruction/error semantics open |
| 009ab716 | 72 | P RemoveRow | P: exact wrapped endpoint/erase scheduling §4.2; foreign eraser open |
| 009ab75e | 136 | P RemoveCol | P: exact wrapped endpoint/successor-before-erase §4.2; foreign eraser open |
| 009ac02f | 296 | E InsertCell | P: instruction-verified ten-word copy/replacement and owned helper sequence, §4.3; foreign allocation/copy/error guarantees open |
| 009ac157 | 246 | P InsertCell | P: bounds/replacement/owned helper sequence §4.1; foreign allocation/copy/error open |
| 009a8fec | 82 | P iterator operator++ | S: advance-to-next-row with same column, §6; mutation/end misuse excluded |
| 0095a087 | 35 | AutoPtr Release | F: foreign smart-pointer owner; not matrix credit |
| 0095a0aa | 47 | AutoPtr destructor | F: foreign smart-pointer owner; not matrix credit |

Supporting entries `009a94a6` (36 bytes), `009a947f` (39), `0094c8f5` (56),
`0040d4c0` (54), `0040d500` (64) establish helper lookup/order/traversal only. They are not added
to 39/3358, nor promoted to independently audited whole owners. Insertion's `009a954e`, `009abc11`,
`009ab299`, `009a9c2f`, `009a943e`, erasers `00927cf8`, `009ab2d3` and cleanup `009599a9` remain
foreign/shared or uncertain. Their displayed template names are not proof of exclusive ownership.
No ABI row/version, implementation, game run, native-exact comparison or complete-owner credit results.
