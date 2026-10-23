# My Street Sim native owner contract

## 1. Scope, evidence, and accounting

This is the separate contract for `cSC4MyStreetSim`, class `0x0BF606A5`,
advisor 30. [mysims.md §1.1](mysims.md#11-classes) explicitly excludes this
owner. The ordinary MySim record, commute algorithms, pool, pathfinder, sixth
agent slot, and historical broad shadow comparisons do not prove this contract.
The inherited MySim and Advisor methods below are typed boundaries, not newly
recovered Street algorithms. A future portable owner must make the Street
decisions described here itself; a host callback returning a whole Street next
state is not this contract.

Evidence is the canonical Windows GOG 1.1.641 export (`functions.tsv`,
`instructions.asm`, queried with `re_query.py`) and executable vtable words.
Addresses below identify instructions, not portable bindings. Decompilation is
secondary and has misleading overlapping fields; all offsets here are taken
from instructions. No native execution or portable implementation was used.

Prior dedicated-owner audit (2026-10-23): the current spec tree and lane reports,
the history of this path, and all-ref commit subjects matching MyStreet were
searched. No earlier dedicated Street owner contract was found. The matches
were consumer/shadow attribution in `mysims1`, `cores26`, and commits `a9490b4c`
and `b2412f01`; they do not establish an original owner family. Historical
`mysims1` results remain historical. This is the first dedicated contract found
by that bounded audit, not a reset of any implementation attempt counter.
Metadata's 33 entries / 5373 bytes is an attribution target to audit, not
completed owner coverage or a count of independent algorithms.

## 2. Identity and dispatch

Let **B** denote a valid complete object. Offsets are evidence notation only;
portable owners use typed state and distinct interface identities. Installed
tables at construction (`0043D605–0043D62A`) are:

| Identity | Adjustment from B | Table | Extent used here |
|---|---:|---|---:|
| Complete object / class hooks | 0 | `00A85ED0` | 17 slots |
| Advisor | +8 | `00A85E78` | 21 slots |
| Serializable | +12 | `00A85E5C` | 6 slots |
| Message target | +16 | `00A85E4C` | 4 slots |
| MySim, IID `0x0A1DA42F` | +184 (`0xB8`) | `00A85D38` | 69 slots |
| Street interface, IID `0x0BF61276` | +696 (`0x2B8`) | `00A85D20` | 6 slots |

QI `0043D0F0` clears the caller's output first. IID `0x0BF6069F`
returns B; IID `0x0BF61276` returns B+696. Each successful own branch
writes the pointer, invokes complete-object AddRef, then returns **AL=1**, pop
8. Other IDs tail-delegate to MySim QI (`00424B00`): `0x0A1DA42F`
returns B+184 and `0x8BC01FFE` returns B, with the same store/AddRef/AL order;
other IDs continue to Advisor QI. No new unsupported-ID policy is inferred
beyond that boundary. Null output and invalid/null self are unsupported; the
apparent null-self branch still dereferences self and is not a safe failure.

The five interface QI/AddRef/Release triples `0043D240–0043D320`
subtract respectively 8, 12, 16, 184, 696 and tail-call the complete-object
entry. The root AddRef/Release (`0043D150`, `0043D160`) are redirects through
shared refcount primitives, ultimately `00968B60` and `0090D997`.
They do not maintain independent counts per interface. Their return channel
belongs to those primitives; no own boolean or independent Street count is
introduced.

### 2.1 Factory and lifecycle

`0043DFA0` requests 712 (`0x2C8`) bytes, calls the constructor when allocation
is nonnull, and returns B+696 in EAX. A null allocation returns EAX=0.
There is no AddRef in this factory body. Registration provenance is separately
tracked below; factory name alone is not registration evidence.

Construction (`0043D5F0`) first invokes inherited MySim construction. It
temporarily installs `00A85D08` on the Street subobject, then the six final
tables above. Its own explicit data stores are:

| Field (relative to B) | Owned value / role |
|---|---|
| `+0x2C0` | null initially, then owned reference to a fresh 20-byte helper if allocation succeeds |
| `+0x12C` | literal `0x0BFA331E`; semantic meaning not established here |
| `+0x150` | raw 32-bit `0xFFFFFFFF` |

The helper has a ref interface at `00A80810`; construction zeros its +4/+8/+12
before allocating an 8-byte storage area, sets +4 and +8 to that area, +12 to
area+8, the first byte of the area to zero, and +16 to zero. The misleading
export name of the storage allocator does not make this a message target.
If helper allocation returns null, `+0x2C0` stays null. Failure of its nested
storage allocation is unchecked and unsupported. Installing a distinct new
helper performs new.AddRef, stores it, then releases an old nonnull helper;
the old field was explicitly zeroed, so ordinary construction has no old
release. Constructor returns B in EAX. Other defaults belong to inherited
construction and are not asserted to be zero or ordinary-MySim defaults.
No copy constructor or copy contract has been established.

Advisor `Init(arg32)` (`0043D170`) receives B+8 and pop 4. It calls inherited
MySim Init with the same interface self and argument. If **AL=0**, it returns
AL=0 without the Street poststep. If AL is nonzero, it tests the 32-bit field
at `(B+8)+0x15C` = B+`0x164`. If zero, it invokes inherited
`CreateLuaTable(B)` (`00429040`); that result is not tested. It then returns
AL=1. A nonzero field skips that call. There is no Street rollback of inherited
failure effects. Upper EAX bits are not a boolean result.

Advisor Shutdown `0043D1A0` is solely a tail jump to inherited MySim Shutdown
(`00428C20`), preserving its return/cleanup contract. There is no distinct
Street detach or helper release in this wrapper.

Root slot 3 (`0043D590`) is the deleting destructor: it reinstalls the six
Street tables, releases the nonnull helper at B+`0x2C0`, calls inherited
destruction (`00429D90`) on B, then frees B iff bit 0 of its 32-bit argument is
set. It returns the old B value in EAX, pop 4; that value is not a live object
after freeing. It does not clear the helper field before Release. Exceptions,
reentrancy, destruction twice, and nonreturning callbacks are not normalized
into success/failure behavior.

### 2.2 Serialization boundary

Serializable slot 5 (`0043D220`) returns **EAX=`0x0BF606A5`**, pop 0.
Slots 3/4 (`0043D1B0`, `0043D1D0`) receive one stream pointer, pop 4:
call inherited Write/Read respectively, ignore that return, call stream
vtable +`0x48` with no explicit argument, then return **full EAX 1 if its AL
is zero, otherwise EAX 0** (`NEG AL; SBB EAX,EAX; INC EAX`).
This proves a Street wrapper, not a separate Street save-record schema, and
does not establish that inherited data contains a complete Street snapshot.

## 3. Owned behavior

The remaining native scheduling, balloon, attachment, message and trip-update
contracts are being audited. They must not be supplied by ordinary MySim
expectations. See the final ledger and open questions as those sections land.

## 4. Proposed literal traces (UNEXECUTED)

These are independent analytical expectations, not measured native results,
portable tests, coverage, or a runtime oracle. Inputs include raw results of
individual foreign calls. A valid initialized object/interface is a premise.

| Trace | Literal setup and operation | Expected ordered observations |
|---|---|---|
| L1 | Valid B; QI `0x0BF61276`; output initially `0x11223344` | output zero; output B+696; root AddRef; AL=1, pop 8 |
| L2 | Same B through MySim identity B+184; QI `0x0BF6069F` | subtract 184; output zero; output B; root AddRef; AL=1 |
| L3 | Init inherited return EAX=`0x100`; Lua field zero | inherited call only; AL=0, no CreateLuaTable |
| L4 | Init inherited AL=2; Lua field zero; CreateLuaTable returns raw failure | inherited call; CreateLuaTable(B); AL=1 |
| L5 | Init inherited AL=1; Lua field `0x12345678` | inherited call; no CreateLuaTable; AL=1 |
| L6 | Write inherited AL=0; stream +0x48 returns EAX=`0x100` | inherited Write; stream query; full EAX=1 |
| L7 | Read inherited AL=1; stream +0x48 returns EAX=`0x102` | inherited Read; stream query; full EAX=0 |
| L8 | Destructor arg=2, helper H nonnull | reinstall tables; H.Release; inherited destruction; no free; EAX=B |
| L9 | Destructor arg=3, helper null | reinstall tables; inherited destruction; free B; EAX=old B |
| L10 | Two fresh constructions B1/B2; helper allocations H1/H2 distinct | each sets its own constants and holds its own helper; releasing H1 does not release H2 |
| L11 | Copy, exact initial refcount, initial active/position state, or nested storage allocation failure | **Unsupported**; inherited defaults/copy/domain unproved; no golden snapshot |
