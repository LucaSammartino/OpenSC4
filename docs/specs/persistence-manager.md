# Persistent resource manager

Companion to [persistence.md](persistence.md), covering only `cGZPersistResourceManager`.
Evidence is the Windows 1.1.641 export and native instructions. Method names below describe behaviour;
unnamed entries are identified by their vtable slot and address. No external ABI names are implied.

## 1. State and ownership

The manager has an ordered registry of DB segments, an ordered registry of resource caches, a factory-ID
registry and a resource-type-to-factory-ID registry. These are distinct: removing a segment does not itself
evict loaded resources. The resource-cache registry contains a default cache created by the constructor
(`0x009744B2`, `0x0097C613`). The audit inventory is 109 attributed functions, 8,437 native bytes, including
container helpers (selection file `run4-selection-owners.tsv`, 2026-10-22).

Object and output pointers must be valid unless a nullable case is stated explicitly. The native bodies
have no general null-input/heap-exhaustion-to-false policy. Cache registration can retain a null pointer
without AddRef, but later callback loops dereference cache pointers unchecked; this is not a usable null
cache implementation.

Returned segment interfaces and returned resources carry references. Successful factory/resource lookups
must release their temporary references after forwarding calls. In particular, a resource can remain in a
cache after the caller's requested QueryInterface fails: a successful underlying load is cached **before**
that final QueryInterface (`0x009732EB`).

## 2. Load and create dispatch

Slot 3 (`0x009732EB`) takes a key, requested IID, output and two forwarded creation arguments. Under the
cache-registry lock it queries caches in registry order with cache slot 7 `(key, outResource)`, stopping at
the first cache reporting success. On a miss it invokes the manager's internal load entry at byte offset
`0xB8`, asking for `cIGZPersistResource` (`0x456B8F1D`). If this fails, it returns false. After a successful
load it calls the internal cache-insertion entry at offset `0xB0`, then QueryInterfaces the resource for the
caller's IID. Finally it releases its resource reference and the lock. A cached hit also performs this
QueryInterface; it does not call the factory again when that QueryInterface fails.

Slot 6 (`0x00972DA2`) explicitly creates from a supplied key: call internal offset `0xB4` for a resource,
set its key, QueryInterface to the requested IID, and insert it into the cache only if that QueryInterface
succeeds. Release the temporary resource in either case. This differs from slot 3's load path.

Slot 5 (`0x009730CC`) follows the same create/set-key/QueryInterface/cache sequence using a generated key;
the key-generation contract is specified with the remaining mutation entries below.

## 3. Segment queries and enumeration

Slot 32 (`0x0097377F`) searches the segment sequence from its beginning. It clears the output first,
asks segment slot 19 whether the key is present, and returns the first matching segment with AddRef.
False leaves the output null. Slot 31 (`0x009737FA`) instead compares segment slot 14's identifier and
returns the first equal identifier, also AddRef'ing it.

Slot 36 (`0x009738A2`) returns the total segment count. With a non-null output array and nonzero input
capacity, it writes the first min(capacity, total) segment pointers in registry order, AddRef'ing each,
and replaces the capacity with the number written. With null array or zero capacity, it leaves the
capacity unchanged. This entry itself takes no registry lock.

Slot 15 (`0x009731C2`) allocates and returns a new referenced key list even for an empty result. Under
the cache-registry lock it asks every cache's slot 12 to append keys, forwarding the filter argument,
and returns the sum of their reported counts. Slot 17 (`0x009739CB`) similarly aggregates every segment's
slot 18. Neither manager loop deduplicates keys or checks append success; key-list duplicates are allowed
(persistence §5). The segment aggregation takes the cache-registry lock, not the segment-registry lock.

Slot 4 (`0x00972D60`) bypasses all cache lookup and insertion: invoke internal load offset `0xB8` for
the resource interface, QueryInterface for the requested IID, and release the temporary reference.
It can therefore load a different object for a key already cached. Neither load entry negatively caches
failures. A later request starts again with the then-current cache, factory and segment registries.

## 4. Factory registry

Slots below refer to the manager vtable at `0x00AD89D0`; the native table has 48 entries. The final four
entries are internal dispatch hooks rather than additional claims about the public interface table.

| Slot / address | Inputs and result |
|---|---|
| 19 / `0x00974407` | Register `(factoryID, resourceType, factoryOrNull)`. A supplied factory is AddRef'd. Otherwise ask GZCOM to create `factoryID` with factory IID `0xA56B8F17`. A non-null result is sufficient, irrespective of the COM boolean. If still null return false without registry mutation. Otherwise retain that reference in a new factory-ID entry, append a type-to-ID entry, return true. |
| 20 / `0x00973FD6` | Remove **all** factory-ID entries with the given ID; Release each stored factory reference. Return whether any existed. Leave all type-to-ID associations in place. |
| 21 / `0x00974042` | Given `(resourceType, outFactory)`, find the first matching type association, then the first matching factory-ID entry. On success write the factory pointer, AddRef and return true. On either miss return false and leave the output untouched. Do not continue to another type association if its selected factory ID is missing. |
| 22 / `0x00972D4E` | Given `(key, outFactory)`, forward key.type to slot 21. |
| 23 / `0x00972D30` | Given `(resource, outFactory)`, forward resource.GetType() to slot 21. This is the resource's own type, which need not equal its stored key.type. |
| 24 / `0x009732C7` | Return the number of factory-ID entries, including duplicates. |
| 25 / `0x0097409E` | Return the factory pointer at a zero-based hash-iteration index, or null if out of range. The pointer is **borrowed**, with no AddRef. |

Registration is not replacement and has no uniqueness test. Re-registering the same factory ID and/or type
adds entries to both maps and retains another factory reference. Registration, removal, lookup, count and
indexed access take the factory lock. The COM creation/AddRef preceding registration occurs before taking it.
No manager operation removes individual type associations or clears them during Shutdown.

### 4.1 Exact duplicate and iteration order

Each map is a separate chained hash table. Hash is the unsigned 32-bit ID itself; bucket is ID modulo the
current bucket count. A new entry with an existing equal ID is linked **immediately after the first equal
entry**. If no equal entry exists it is linked at the bucket head. Lookup returns the first equal entry.
Thus without growth, registrations A, B, C for one ID give chain A, C, B, and select A. This rule applies
independently to factory IDs and resource types (`0x00973AC4`, `0x00973B37`).

Both tables start with 193 buckets: constructor requests 100 and the prime table at `0x00A829B0` selects
193. Before each insertion, grow if entry count + 1 exceeds bucket count, to the first listed prime at
least that large (193, 389, 769, 1543, …; saturate at the last prime `0xFFFFFFFB`). Growth visits old buckets
in ascending order and each old chain from head to tail, relinking each node at the new bucket's **head**.
Equal-ID chain order therefore reverses on growth: do not implement an unconditional first- or last-wins
dictionary. A lookup can change after registering an unrelated entry. The two maps can reach different
growth thresholds after factory-ID removals, since type associations are retained.

For an independent implementation, the complete growth sequence from the initial bucket count is
`0xC1, 0x185, 0x301, 0x607, 0xC07, 0x1807, 0x3001, 0x6011, 0xC005, 0x1800D, 0x30005, 0x60019,
0xC0001, 0x180005, 0x30000B, 0x60000D, 0xC00005, 0x1800013, 0x3000005, 0x6000017, 0xC000013,
0x18000005, 0x30000059, 0x60000005, 0xC0000001, 0xFFFFFFFB` (native data `0x00A829B8–0x00A82A1C`).

Indexed factory access walks ascending bucket numbers, then chain order, and counts duplicate entries.
Evidence: insert wrappers `0x0097420D`/`0x00974231`; resize bodies `0x009978AF` and `0x004F0940` (the latter's
export label “begin” is misleading); native unsigned division and equal-node splice `0x00973AD0–0x00973B35`;
native resize relinking `0x0099790A–0x00997936`; lookup `0x0097405B–0x00974089`.

## 5. Segment registry and direct records

| Slot / address | Behaviour |
|---|---|
| 26 / `0x00972B73` | Tail-dispatch to slot 27, preserving its argument. |
| 27 / `0x0097437B` | AddRef segment, remove its existing pointer occurrence through slot 29, prepend it, return true. |
| 28 / `0x009743C1` | Same, but append it. |
| 29 / `0x009742B7` | Find first pointer-identical segment, Release its registry reference, erase it and return true; false if absent. |
| 30 / `0x0097371F` | Test pointer identity membership; boolean. |
| 31 / `0x009737FA` | Find first segment with equal segment ID; referenced output, cleared on miss (§3). |
| 32 / `0x0097377F` | Find first segment reporting key presence; referenced output, cleared on miss (§3). |
| 33 / `0x00973873` | Return segment count under segment lock. |
| 34 / `0x00973D88` | Return indexed segment pointer, or null if out of range, under segment lock. **Borrowed**, no AddRef. |
| 35 / `0x0097391B` | With non-null callback, call `(borrowedSegment, context)` in segment order, stopping at the first false return. Null callback does nothing. No lock or per-callback AddRef. |
| 36 / `0x009738A2` | Referenced array prefix and total count (§3), no lock. |
| 37 / `0x00972B78` | Given `(key, outRecord, writable)`, find the first containing segment and call its slot 21 `(key, outRecord, flags)`, using flags 1 for false and 3 for nonzero writable. Return its boolean, Release segment. No fallback to later segments. On no containing segment return false without clearing outRecord. |
| 38 / `0x00972BBF` | First argument is unused. For the record at `*recordPointer`, call record slot 5 (Close), preserve its boolean, then Release the record and return that boolean. Leave the pointer value unchanged; it is no longer an owned reference. |

Prepend/append move an existing segment rather than duplicate it. Their preliminary AddRef protects a
segment whose only live reference was in the registry while the old reference is released. Segment IDs
need not be unique; lookup order decides. Mutation does not call segment Open/Close/Shutdown, change a
segment ID or invalidate resource caches. All segment mutation, membership, first-match lookups and indexed
access use the segment lock. See §3 for aggregation's different lock. These rules determine manager
precedence; directory scanning and package ordering before registration remain data-formats.md's subject.

The deque uses 32 pointer elements per allocation block. Reallocation and either-direction shifting preserve
logical order; block boundaries do not create a search-order boundary. Single erase shifts the shorter
side and removes one end element. Helpers are catalogued in §11; the 32-element layout follows the native
`0x80` byte block steps at `0x0097316E`, `0x009730A9`, `0x0097304D`.

## 6. Underlying creation/loading and failure

The final two arguments on slots 3–6 are passed unchanged into the factory's creation call. This owner
does not inspect them; this contract does not invent names or meanings for those factory-specific arguments.
All booleans in these paths are the returned low byte; upper EAX bits are not a second status channel.

| Internal slot / address | Behaviour |
|---|---|
| 44 / `0x009733C6` | Iterate caches, asking slot 11 whether each accepts the key. At the first true, call its slot 8 `(key, resource)` and return true **regardless of that insertion's result**. False only if no cache accepts. No lock inside this hook. |
| 45 / `0x00972FAE` | Resolve factory by key.type (manager slot 22), call factory slot 4 `(key.type, iid, out, a, b)`, Release factory and return creation result. On missing factory return false without writing out. No segment required. |
| 46 / `0x00972EAE` | Resolve factory by key.type first, then find first segment containing key, then call internal slot 47. Release both temporary interfaces. Missing factory avoids segment lookup; missing segment or failed slot 47 returns false. No cache access here. |
| 47 / `0x00972F1D` | Open selected segment's record with flags 1. Ask selected factory slot 3 `(record, resourceIID, outResource, a, b)` to create/load it. On success set resource key, QueryInterface for requested iid/out, then call segment slot 24 `(record)` and Release record and resource. Return the QueryInterface boolean, ignoring segment-close result. On open failure with a non-null record, or factory-creation failure, call segment slot 25 `(&record)` and return false. If open fails and record remains null, no cleanup callback. |

There is no search for another segment or factory after a selected one fails. A failure on this call is
not remembered on the next call. A successful resource creation must return a valid resource interface;
there is no success-with-null defense. Temporary resources returned on a factory's **false** creation result
are not released by these paths; factories must obey their own failure/output contract. Similarly, caller
outputs are not globally pre-cleared: preserve the exact forwarding/untouched behaviour rather than assuming
all false paths clear an output. Segment lookup itself does clear its own temporary output (§3).

Native cross-check: factory-before-segment dispatch `0x00972EC4–0x00972EF9`; record open, create, set key,
QueryInterface, segment close and Releases `0x00972F28–0x00972F91`; abort selection `0x00972F96–0x00972FA6`.

### 6.1 Keys and explicit registration

Slot 5 constructs a key `(requestedType, currentGroup, nextInstance)` with all components 32-bit. The group
starts at 0. The instance counter starts at `0x0000FFFF`; allocation returns the old counter and increments
modulo `2^32`, **before** any factory lookup. A failed creation or QueryInterface consumes the instance ID;
no collision search, rollback, or retry exists. The group read and counter increment are separate operations
under the key lock (`0x00972FF2`, `0x00973023`); there is no group setter in this owner's table. The counter
and group are not reset by Shutdown. Generated keys are set on the resource after creation succeeds.
Native key order and widths: `0x009730EA–0x00973118`, counter increment `0x00973009–0x00973014`, constructor
initial values `0x0097453D–0x00974546`.

Slot 7 (`0x00972E4B`) registers an existing resource: null returns false. Otherwise under cache lock, fetch
its key; if manager slot 10 already reports that key present in any cache, return true without replacing
anything or changing ownership. Otherwise return internal slot 44's acceptance result. A false insertion
inside the chosen cache still yields true. Slot 8 (`0x00972E2B`) first sets the supplied resource's key to
the given key and then invokes slot 7; the key mutation happens even if registration fails or is a duplicate.

Slot 10 (`0x00973485`) checks only the caches, in order, with cache slot 10 and stops on first true.
Slot 9 (`0x0097341B`) similarly finds a cache reporting presence, then calls cache slot 9 to erase the key;
if that erase returns false it continues to subsequent caches. It returns the first successful erase,
or false; it never removes a DB record. Both hold the cache-registry lock.

Slot 14 (`0x00973964`) tests presence in the **segments** only, in order, returning true on first containing
segment. Thus a resource can be cached but absent from segments, or present on disk but absent from caches.

## 7. Write paths

Slot 12 (`0x00972BDC`) takes `(key, segment)`: obtain the resource with slot 3, requesting resource IID and
zero for both forwarded arguments; if successful call slot 13 `(resource, key, segment)`, Release resource,
and return slot 13's result. Slot 11 (`0x00972CDC`) takes `(keyList, segment)`, snapshots its initial Size(),
and invokes slot 12 for each key in enumeration order. It continues after failures and returns the AND
of those results; an empty list returns true. It is not transactional.

Slot 13 (`0x00972C24`) takes `(resource, creationKey, segment)`:

1. Query resource's current key. If the segment contains it, open that key with flags 3. Otherwise create
   a record using **creationKey**, which may differ from the resource key.
2. Initialize the result from that open/create boolean. If false, skip writing. If a non-null record was
   produced, still abort it via segment slot 25 `(&record)` before returning false.
3. If opened/created, resolve factory by **resource.GetType()**. If found, call factory slot 6
   `(resource, record)`. Only if this returns true, commit via segment slot 23 `(&record)`.
4. On successful commit set the local record pointer to null. Release the temporary factory if obtained.
   If the local record pointer is still non-null, call segment slot 25 `(&record)`.
5. Return the original open/create boolean. A failed factory lookup, failed factory Write or failed commit
   **does not turn it false**. This can make a bulk write report true although records were aborted.

Record ownership for this write path is delegated to commit/abort's pointer argument: the manager itself
does not call record.Release. There is no cache eviction, resource-key correction or manager retry after
write failure. This return-value quirk is confirmed by preservation of BL at `0x00972C7E`, the only later
assignment `BL=1` at `0x00972CB8`, and return at `0x00972CD5`. Record flag widths and branch order were checked
at `0x00972C51–0x00972C7B`; cleanup and commit tests at `0x00972CA1–0x00972CD0`.

## 8. Cache registry and built-in cache

Slot 39 (`0x009740F1`) registers a cache at the list's end, taking one reference; pointer-identical duplicate
registration returns false and takes none. Slot 40 (`0x00973A57`) removes the first pointer-identical cache,
releases its registry reference, returns true, or false when absent. Removal does **not** call cache
Shutdown or flush it. Both use the cache-registry lock.

Slots 41/42/43 broadcast cache slots 15/16/17 respectively: query automatic-purge flag (bitwise OR of
all low-byte results, zero on empty), set that flag for every cache, and purge (bitwise OR of all returned
low bytes, zero on empty). They visit every cache without short-circuiting. Slots 41 and 42 take the **key
lock**, whereas slot 43 takes the cache-registry lock (`0x009734DC`, `0x00973526`, `0x009735B8`).

### 8.1 Built-in cache (`0x00AD9428`)

These cache bodies are included in the manager's attributed owner, even though they implement a second
vtable. Construction (`0x0097C613`) sets reference count, maintenance counter and automatic-purge flag to
zero and the key filter to null; it constructs a resource hash table and two critical sections.

| Slot / address | Contract |
|---|---|
| 0 / `0x0097BE6C` | QI accepts unknown IID 1 and cache IID `0x6BB923E4`, writes this and AddRefs. Other IID: false, output untouched. |
| 1 / `0x0097BE8E` | Under reference lock increment count, but return the **previous** 32-bit count. |
| 2 / `0x0097BEBA` | Under reference lock: unsigned entry count <2 destroys after unlocking and returns zero; otherwise decrement and return new count. |
| 3 / `0x00A6D837` | No-op (no returned boolean contract). |
| 4 / `0x0097C26D` | Under entries lock call virtual slot 18 to clear; then sweep/release any remaining entries and clear the table. Returns no specified value. |
| 5 / `0x0097BFA4` | Replace filter: if pointer differs, AddRef new non-null filter before assignment and Release old non-null filter. Same pointer is a no-op. |
| 6 / `0x009DB663` | Return borrowed filter pointer; no AddRef. |
| 7 / `0x0097C36E` | Exact-key lookup under entries lock. On hit write resource, AddRef, true; on miss false, output untouched. Does not apply the filter. |
| 8 / `0x0097C595` | Under entries lock, if exact key absent: AddRef resource, SetKey(key), insert and return true. If key present, false with neither reference nor key changes. Does not apply the filter. |
| 9 / `0x0097C3BB` | Erase exact key under entries lock, Release stored reference; return whether found. |
| 10 / `0x0097C41A` | Exact-key membership under entries lock. |
| 11 / `0x0097BFAC` | Accept key if no filter or filter slot 3 `(key)` returns true; no lock. |
| 12 / `0x0097C2D3` | Enumerate entries under entries lock. Read resource.GetKey() once per visited entry; apply stored filter then supplied filter to that copied current key (short-circuit AND; null means accept). Count accepted entries, optionally append that same key to supplied key list via its slot 4, ignoring insertion result. Return accepted count. |
| 13 / `0x0097BFCA` | Wrap supplied type in a referenced type-equality filter, delegate to slot 12, release filter and return count. |
| 14 / `0x0097BF43` | Maintenance tick with ignored argument. If flag nonzero, increment 32-bit counter; if the **signed** result exceeds 128, reset counter to 0 and invoke slot 20 once. Always true. Disabled ticks leave counter unchanged. |
| 15 / `0x0097BEF6` | Return automatic-purge byte under reference lock. |
| 16 / `0x0097BF1C` | Store automatic-purge byte under reference lock; does not reset counter. |
| 17 / `0x0097BF63` | Invoke slot 20 repeatedly until false or eight successful calls. Return whether at least one returned true. Even an empty cache receives one call. |
| 18 / `0x0097C452` | Release each non-null resource and clear all entries. This entry has no lock of its own. |
| 19 / `0x0097C53E` | Deleting destructor: destroy members, free object if flag bit 0 is set. |
| 20 / `0x0097C48F` | Under entries lock, purge each non-null resource whose resource slot 11 reports an **unsigned** reference count <=1; Release it and erase its entry. Also erase null entries but do not count these as purged resources. Return whether any non-null resource was purged. |

The default cache accepts all keys initially, so it normally consumes manager insertion before later
caches. Manager hook 44 does not try a second accepting cache after duplicate rejection. Resource cache
entries use full `(type, group, instance)` equality; the hash is **instance alone**, unsigned modulo bucket
count (`0x0097C049`, equality `0x00431180`). Unique insertion links new keys at bucket head. Initial buckets
and growth rule match §4.1, using key.instance for rehashing (`0x0096F614`, `0x0097C0C5`). Enumeration walks
ascending buckets then each chain (`0x0097C081`, `0x0096F5B7`, `0x0097C1EA`), not insertion order. Distinct
keys with equal instance remain distinct. Changing a resource's key externally does not reindex its cache
entry: membership/lookup uses the stored key, but enumeration uses the resource's **current** GetKey()
for **both predicates and output**, not just output (PR5).

**Enumeration key snapshot (PR5).** Before either predicate, slot 12 calls resource slot 3 with a
local 12-byte key destination (`0x0097C306–0x0097C312`). It passes that destination to the stored
filter (`0x0097C315–0x0097C327`), then, only if accepted, the supplied filter
(`0x0097C329–0x0097C33A`), then, only if both accept and a list was supplied, list slot 4
(`0x0097C33C–0x0097C348`). There is no second GetKey and no load of the entry's stored key in
this path. Changing a resource from current key K to L between calls makes **both** filters see L
on the next enumeration, while lookup and bucket/chain position remain those of K. A predicate
that changes only the resource's current key after the copy cannot change the copied key used
later in that same iteration. This does not specify behavior for callbacks that erase/reindex the
table, invalidate objects, or write through a predicate's key argument.

The list is appended to, never cleared here. Rejected entries produce neither append nor count;
accepted entries increment the count even with a null list or a false list-slot-4 result
(`0x0097C33C–0x0097C34B`). Duplicate current keys remain separate accepted entries. Traversal
order is the stored table's ascending buckets and chains, even if the current keys would hash
elsewhere. Enumeration acquires no temporary resource or filter references: its only per-resource
virtual call is GetKey. The lock is released before returning the accepted count
(`0x0097C35E–0x0097C36B`).

Changing the filter does not remove existing entries or inhibit direct lookup, erase or direct cache
insertion. Enumeration reapplies the current filter; purge is independent of it. Resource references held
by callers keep reported count above 1 and normally prevent purge. The cache owns one reference per entry.
The count threshold is verified at `0x0097C4C5–0x0097C4D4`, old-count AddRef at `0x0097BEA3–0x0097BEB4`,
tick signed comparison at `0x0097BF49–0x0097BF57`, and eight-pass bound at `0x0097BF69–0x0097BF78`.

The destructor (`0x0097C23F`) releases the filter and frees table/lock storage; it does **not** call Clear
or release resources still stored in nodes. Normal manager Shutdown explicitly calls cache slot 4 first.

### 8.2 Type-only filter

Filter vtable `0x00AD8970` wraps a single 32-bit type. Its predicate (`0x00973194`) compares key.type only,
ignoring group and instance; deletion (`0x009731A5`) restores the base filter vtable and conditionally frees
storage. Manager slots 16/18 (`0x00973239`, `0x00973280`) create and reference this filter, call slots 15/17
respectively, then release it. Result lists still contain full keys and preserve duplicate occurrences.

## 9. Lifecycle, interface identity and concurrency

Manager QI accepts manager IID `0x656B8EFC` and returns the main pointer with AddRef. Other requests delegate
to service QI at main+4, which accepts service IID `0x287FB697`; unknown IID 1 returns main+8. Unrecognized
IID leaves output untouched. All three interface pointers share the plain inherited reference count,
initially 0: AddRef returns the incremented count; Release decrements if unsigned count >1, otherwise calls
the virtual deleting destructor and returns zero. Manager wrappers at `0x0093056F`/`0x00930577` adjust this
before forwarding. This count is not interlocked. Base evidence: `0x00972B51`, `0x0091DD67`, `0x009457C6`,
`0x00968B60`, `0x0090D997`, initial zero `0x0090DA1E`.

The constructor initializes the service with ID `0x056B906E` and both priority fields 1,000,000, running
flag false (`0x0091DE0F`, constructor instructions `0x009744BB–0x009744EB`). It creates empty factory/type maps
and segment sequence, the cache list with its one default cache, and the key state of §6.1. The default
cache has one net registry reference after the temporary auto-reference is released. No packages or
factories are automatically registered. A null default-cache allocation is inserted as a null list element;
later loops do not defend against it. Do not treat heap exhaustion as a normal false-return contract.

Service vtable `0x00AD899C` slot 8 is an inherited true-returning no-op initialization. Slot 9 is manager
Shutdown (`0x00973EEB`), entered with the service pointer (main+4), with this exact order:

1. Acquire cache-registry lock, factory lock, then segment lock (offsets main+`0x80`, `0x20`, `0x50`).
2. Call each cache's slot 4 in order; ignore any returned value. Clear the cache list, releasing its
   references, then release the cache-registry lock.
3. Release every factory reference in factory-map iteration order, clear the **factory-ID map only**,
   and release the factory lock. Retain type-to-ID entries.
4. Release each segment reference in registry order, clear the segment sequence and release segment lock.
   Return true, including when already empty. Do not change running flag, priority, key group or counter.

After Shutdown, normal load may still succeed if callers register new factories/segments, but no cache
is recreated automatically. Initializing again with the no-op entry does not restore one. Stale type
associations can affect subsequent factory registrations (§4). The ordinary/deleting manager destructor
(`0x00973E67`/`0x0097429B`) destroys containers and locks and conditionally frees storage. It does not call
Shutdown, Close segments or explicitly Release raw factory/segment pointers still in their containers.
The cache list's auto-reference destructors do Release cache references. Shutdown is the explicit resource
cleanup operation; final Release is not an equivalent flush.

Service slot 11 (`0x0097356D`, main+4) takes the cache-registry lock and calls every cache slot 14 with the
supplied argument. Its native return is stack-state-dependent; §9.1 gives the exact byte rule and shipped
consumer, and §9.2 specifies the deliberately deterministic clean-room return (PR4).

Locks are recursive critical-section objects. Factory/segment/cache callbacks generally run while the
corresponding lock remains held, with the exceptions explicitly listed above. They are not copied to
snapshots. A callback that removes its current registry element can invalidate the native iterator.
There is no owner-local concurrent/reentrant-mutation safety or rollback guarantee for these iterations;
use stable registries while enumerating. This documents the observed mechanism, not a newly promised
thread-safe public API. In particular, wrong/different locks on slots 17/41/42 and unlocked slots 35/36
must not be read as protection against arbitrary concurrent registry mutation.

### 9.1 Service Update: native return and shipped consumer (PR4)

With a stable cache registry, Update visits every cache once in registry order, passes the unchanged
32-bit argument to cache slot 14, and releases the cache-registry lock after the last callback. A true
callback result does not stop traversal. The empty registry still acquires/releases the lock. Evidence:
`0x00973577–0x009735AC`; the callback is at `0x00973597`, and the loop back-edge at `0x009735A5` depends
only on reaching the list sentinel, not on a callback result.

**Native undefined initial value:** the byte at `[EBP-1]` is never initialized by this body. The stack
reservation at `0x00973570` does not clear it. The lock helper at `0x0048C9F0–0x0048CA08`, called with
the object at `[EBP-0xC]`, writes only that object's first two 32-bit fields, not `[EBP-1]`. Each callback's
entire AL byte is ORed into `[EBP-1]` at `0x0097359C`; `0x009735AF` loads that byte into AL for return.
There is no boolean normalization. Upper EAX bits do not constitute a specified return value.

Let **S** denote the pre-existing accumulator byte, and **R1…Rn** the callback return bytes for a particular
execution. The returned byte is **S OR R1 OR … OR Rn**, using 8-bit bitwise OR. With no callbacks, or with
all callbacks returning zero, it is S. With canonical boolean callbacks, any true result forces bit 0
but leaves the other seven bits dependent on S. Thus even the ordinary built-in cache's true result
does not establish that the returned byte equals 1. Neither initial false nor initial true is a native
contract; S is prior stack content, not a caller input or a fresh random value on each call.

For illustration only, latent S=`0x80` and callback bytes `[0,1,0]` yield `0x81`; latent S=`0x00` and
those same bytes yield `0x01`. An empty registry would yield `0x80` and `0x00` respectively. These are
conditional byte calculations, not measured runs or controllable API inputs.

The shipped registration/dispatch trace is:

| Evidence | Consequence for this return |
|---|---|
| Constructor `0x009744BB–0x009744E4`: service ID `0x056B906E`, service pointer main+4, vtable `0x00AD899C`; the executable word at `0x00AD89C8` is `0x0097356D`. | This is service slot 11, not manager slot 11. Export has zero direct calls to this target and one data reference, this vtable word. |
| `cGZResManCOMDirector::OnStart`, `0x0097531C–0x0097534E`: create class `0x656B8F27` as service IID `0x287FB697`, then pass that service pointer to framework slots 3 and 11. Class creation uses `0x009750E4`, which enters the manager constructor at `0x009750F5`. | The manager's service is registered for framework dispatch. Framework slot 11 here is registration, not a call to the service's slot 11. |
| Framework vtable `0x00AC3090`: slot 11 targets `0x00879D21`; that body appends the pointer to the list at framework+`0xA8` (`0x00879D45–0x00879D9B`). Slot 16 targets `0x0087A812` (word `0x00AC30D0`). | This list supplies the service Update dispatcher. |
| Dispatcher `0x0087A812` copies that list to a temporary vector, then calls each service slot 11 at `0x0087A8C1`, passing its update counter from framework+`0xC4` (`0x0087A85C–0x0087A8C1`). Immediately afterward it advances EDI and compares EDI with EBX (`0x0087A8C4–0x0087A8C9`). | The callback's AL/EAX is neither tested, stored, accumulated nor forwarded. Dispatch continues independently of this return, including false or noncanonical bytes. |

The other two indirect `+0x2C` calls in the framework code interval `0x008793BD–0x0087B0B8` are at
`0x0087AF73` and `0x0087AFC3`. Their receivers are respectively the embedded COM interface (obtained at
`0x0087AF5D`, getter `0x008793F9`) and the application global `0x00B540B4`, not a registered service.
The executable's 19 immediate references to service ID `0x056B906E` comprise its constructor and 18
framework lookups that all request manager IID `0x656B8EFC`, not service IID `0x287FB697`. Representative
lookup: `0x004496D5–0x004496E1`; the lookup sites are recorded in the lane report.

**Observed consumer verdict:** the identified shipped service Update dispatch ignores the return byte;
no return-observing persistence service caller was found along these registration, dispatch and explicit
ID-lookup paths. This is static evidence for these shipped paths, not a claim about arbitrary dynamically
obtained interfaces, external plugins or modified executables. Ghidra pseudocode omits the dispatch loop
as allegedly unreachable; the executable/export instructions above establish that loop and its behavior.

### 9.2 Clean-room return policy (PR4 choice)

**Chosen deviation from the undefined native baseline:** expose canonical false for an empty registry or
all-false callbacks, and canonical true if any cache Update returns true. Start the logical OR reduction
at false, but execute every callback in registry order even once its result is true. Preserve the native
argument, lock scope and callback effects independently of the chosen return. This rule covers callbacks
obeying their boolean interface; it does not promise to emulate arbitrary noncanonical injected AL bytes.

The choice is justified by the native OR operation's aggregation intent and the shipped dispatch's
discarded return: it gives a useful, deterministic result without changing that dispatch's decisions or
callback effects. It is an explicit compatibility policy, not evidence that native starts at zero, returns
constant true, or matches these chosen bytes in empty/all-false cases. Exact native byte comparison is
inapplicable where S is unknown. The independent vectors in §10.1 separate those native constraints from
the chosen outputs.

## 10. Independent input/output/failure vectors

Use instrumented factories, resources, segments, caches and key filters with explicit return values and
reference ledgers; these are proposed conformance inputs, **not executed native/game results**. A factory
success supplies a referenced valid resource, and Close/commit/abort callbacks follow the pointer ownership
stated above. Except where specified, the registry has one default cache and all counts below are net
changes after manager temporaries are released. A “call trace” lists only semantically relevant callbacks.

| ID | Input/setup | Required output and externally visible effects |
|---|---|---|
| PM01 | Empty manager; slot 3 key K, out prefilled sentinel | false; caller output untouched; no segment open or factory creation; cache remains empty. |
| PM02 | Factory F maps K.type; segments A then B both contain K; A open fails, B would succeed | false; F lookup/AddRef then A presence/open; no B open, no cache entry. Repeat after A recovers: success via A, not a remembered failure. |
| PM03 | F and A load R; R rejects requested IID X | Slot 3 false but cache holds R (+1 cache reference). Repeat with X: cache hit, no factory/segment call, false. Repeat with supported IID: true, caller gets reference. |
| PM04 | Same IID rejection through slot 6 explicit creation | false; no cache insertion; temporary R released. Repeat creates again. |
| PM05 | R already cached for K; slot 4 loads same key | factory/segment called, separate resource returned; cached R remains; no new cache ownership. |
| PM06 | First generated create fails, second succeeds for type T | First consumes `(T,0,0xFFFF)`; second gets `(T,0,0x10000)`. No lookup for a collision. Explicit-key create never advances the counter. |
| PM07 | K absent; cache C1 accepts but insert=false; C2 accepts and would succeed | Hook 44 true; only C1 insert called; C2 untouched. Slot 6 can return true without caching. |
| PM08 | Cache contains K->R; register different S with same key | Slot 7 true; S not retained; R unchanged. Slot 8 with key K changes S's key even though R remains cached. |
| PM09 | Cache C1 contains K but erase=false; C2 contains K and erase=true | Slot 9 true; tries C1 then C2; no segment record deletion. |
| PM10 | Register segments append A, append B, prepend B, append B | Orders `[A]`, `[A,B]`, `[B,A]`, `[A,B]`; one net reference per present segment; no duplicate B; same-ID lookup returns whichever is first. |
| PM11 | Segments `[A,B]`; indexed lookup 0 and 2, then array capacity 1 | Index 0 gives borrowed A; index 2 null; array returns total=2, written=1, A AddRef'd. Null array with capacity 7 returns 2 and leaves 7 unchanged. |
| PM12 | Register F1 at (ID=7,type=T), then F2 at (7,T), no growth | Count=2, lookup T->F1 with AddRef. Remove 7 releases F1 and F2, count=0; lookup T false with output sentinel unchanged; type associations still exist. |
| PM13 | Register (ID=7,type=T,F1), then (ID=8,type=T,F2), no growth; remove 7 | T still selects association to missing 7: lookup false, no fallback to F2. Shutdown also retains these type associations. |
| PM14 | F1 then F2 for (7,T), followed by 191 registrations with distinct other IDs/types, then one more | At 193 entries lookup T selects F1. The 194th registration grows both maps 193->389 and reverses their equal-key chains: lookup T selects F2. Counts include duplicates. |
| PM15 | Direct built-in cache inserts K=(1,2,5)->R and L=(9,2,5)->S; then K->Q | Both first inserts true, +1 reference each; same instance is not equal key. Third false, Q ref/key unchanged. Lookup K returns R with caller reference. |
| PM16 | Empty built-in cache: insert instance 194 then instance 1 (other components equal) | Both hash to bucket 1 at 193 buckets; enumeration lists instance 1 then 194. Changing R's current key afterward changes enumerated key, but lookup remains at original stored key. |
| PM17 | Existing R's key rejected by newly set stored filter; direct Get and enumeration | Get still succeeds/AddRefs. Enumeration excludes it. Direct cache Insert of another rejected key still succeeds, but manager hook 44 skips this cache for rejected key. |
| PM18 | Two caches append the same key K and report counts 1 and 1 | Slot 15 returns 2, newly referenced list `[K,K]`; type-only variant filters by type and preserves duplicates. Empty registry still returns a referenced empty list and zero. |
| PM19 | Built-in cache has R(count=1), S(count=2), each non-null | One purge returns true, releases/erases R, retains S. Drop caller's S reference and purge: S erased, true. Third purge false. Null-only entry is erased but returns false. |
| PM20 | New cache; enable purge, 128 ticks, disable, 20 ticks, re-enable, one tick | No purge in first 128 or disabled interval. Last tick is counter 129: reset to 0, invoke one purge. Setter does not reset accumulated ticks. Each tick returns true. |
| PM21 | Derived/instrumented purge slot 20 returns true nine times | Cache slot 17 invokes it exactly 8 times, returns true; next call continues with remaining response. If first result false, exactly one call and false. |
| PM22 | Write: open=true; missing factory OR Write=false OR commit=false | Slot 13 returns true in all three cases, aborts non-null record; no extra record.Release by manager. With open=false and null record: false, no abort. With open=false and non-null record: false plus abort. |
| PM23 | Resource current key K, explicit creationKey L; segment lacks K | CreateRecord receives L. If K present instead, OpenRecord receives K and flags 3. Factory chosen by resource.GetType(), not K.type or L.type. |
| PM24 | Bulk write keys `[K,L]`, first load=false, second write open=true but commit=false | Both attempted, aggregate false; no rollback of either. Empty list returns true without writes. If first instead succeeds, aggregate true despite second commit failure. |
| PM25 | Direct record open requested writable=0 then nonzero; CloseRecord pointer P | Flags 1 then 3. Slot 38 calls `P.Close` then `P.Release`, returns Close boolean, leaves pointer value unchanged. Missing segment leaves supplied output sentinel untouched. |
| PM26 | Add cache C twice, remove once, retain external C reference | true then false; one net registry ref; removal true releases it, no C.Shutdown/Clear. External reference keeps C alive with contents intact. |
| PM27 | Shutdown with caches C1,C2, factories F1,F2, segments A,B | C1/C2 Shutdown before list Releases; then factory Releases/clear; then A/B Releases/clear. True; second Shutdown true with no callbacks. No cache restored by initialization, no type-map/counter reset. |
| PM28 | Query unknown IID on manager or built-in cache, sentinel output | false, sentinel unchanged. Successful cache AddRef from count 0 returns 0 but stores 1; manager AddRef from count 0 returns/stores 1. |
| PM29 | Segment callback returns true for A, false for B with C later | Callback trace A,B only; null callback has no trace. No AddRefs around callbacks. Registration mutation inside callback has no safe-iteration result vector. |
| PM30 | Service update with caches C1,C2 returning 0/1 | Both receive argument in order; native return has bit 0 set, other bits depend on S. Chosen clean-room return true. See §9.1–9.2 and the independent PR4 cases below. |

### 10.1a Independent cache key-mutation vectors (PR5)

These are source-derived expectations, not executed measurements. Use the built-in cache directly,
with a stable table and valid owned resources/filters. Let K=(1,2,194), J=(1,2,1), L=(9,7,8),
M=(10,7,9). Insert K->R then J->S into the initial 193-bucket table: both stored entries are in
bucket 1, and traversal is S then R. Insertion gives each resource one cache reference. Each
predicate logs its full input key; equality below means equality of all three words. Reset the
output to `[sentinel]` for each call. Except where stated, predicates do not mutate anything.

| Vector | Operation / predicates | Required result |
|---|---|---|
| PR5-A | Before mutation, stored accepts K, supplied accepts K | Stored trace `[J,K]`, supplied trace `[K]`; count 1, output `[sentinel,K]`. |
| PR5-B | Change only R.GetKey() to L; repeat A | Stored trace `[J,L]`, supplied not called; count 0, output `[sentinel]`. Lookup K still finds R; lookup L misses. |
| PR5-C | After mutation, stored accepts L, supplied accepts L | Stored trace `[J,L]`, supplied trace `[L]`; count 1, output `[sentinel,L]`. |
| PR5-D | After mutation, stored accepts L, supplied accepts K | Stored trace `[J,L]`, supplied trace `[L]`; count 0, output `[sentinel]`. Distinguishes the supplied predicate independently. |
| PR5-E | Set S.GetKey() to L too; no predicates | Count 2; output `[sentinel,L,L]`, with append calls for S then R. With null output, still count 2. With a recording list whose append always returns false, still count 2 and two append attempts. |
| PR5-F | Restore S to J and R to L. Stored rejects J; when given L it sets R's current key to M and accepts. Supplied accepts L | This call supplies/appends L, count 1; the next enumeration sees M. No second GetKey in the first call. |

For A–F enumeration itself performs no resource/filter AddRef or Release. Lookup in B separately
adds its usual caller reference on the K hit; release that reference before checking the cache's
ownership balance. Dropping a cache entry releases its one stored reference under its stored key.

### 10.1 Independent service Update vectors (PR4)

These are derived specification vectors, not executed tests or game captures. Remove the built-in cache
before installing the exact registry shown; keep the registry stable during callbacks. Each named cache
logs `(name, argument)` when its slot 14 is entered and returns the listed canonical boolean. Native S
has the meaning in §9.1 and must not be silently set to zero by an oracle. All cases acquire the
cache-registry lock before the trace and release it afterward.

| ID | Registry and callback returns; supplied argument | Exact callback trace | Native returned-byte constraint | Chosen clean-room return |
|---|---|---|---|---|
| PMU01 empty | `[]`; `0x12345678` | none | S; no determinate boolean | false |
| PMU02 false | `[A:false]`; `0` | `(A,0)` | S; no determinate boolean | false |
| PMU03 all false | `[A:false,B:false,C:false]`; `0xFFFFFFFF` | `(A,0xFFFFFFFF)`, `(B,0xFFFFFFFF)`, `(C,0xFFFFFFFF)` | S; no determinate boolean | false |
| PMU04 all true | `[A:true,B:true]`; `7` | `(A,7)`, `(B,7)` | S OR `0x01`; nonzero, not necessarily `0x01` | true; B still called |
| PMU05 mixed | `[A:false,B:true,C:false]`; `0x12345678` | `(A,0x12345678)`, `(B,0x12345678)`, `(C,0x12345678)` | S OR `0x01`; nonzero | true; C still called |
| PMU06 order/early true | `[C:true,A:false,B:true]`; `0x80000000` | `(C,0x80000000)`, `(A,0x80000000)`, `(B,0x80000000)` | S OR `0x01`; nonzero | true; all three called in registry order |

## 11. Complete attributed-body audit

All 109 selected bodies (8,437 B) were read in descending-size order, with truncated displays reread. Folded
functions outside this selection were inspected where needed to resolve calls; their bytes are not counted
again. The tables above cover every manager/cache entry. Remaining selection bodies are container/filter
mechanisms below, not additional owners. Addresses list audited function **starts**, not inferred spans.

| Group | Audited starts and resulting contract |
|---|---|
| Manager public entries | `00972B51`, `00972B78`, `00972BBF`, `00972BDC`, `00972C24`, `00972CDC`, `00972D30`, `00972D4E`, `00972D60`, `00972DA2`, `00972E2B`, `00972E4B`, `009730CC`, `009731C2`, `00973239`, `00973280`, `009732C7`, `009732EB`, `0097341B`, `00973485`, `009734DC`, `00973526`, `009735B8`, `0097371F`, `0097377F`, `009737FA`, `00973873`, `009738A2`, `0097391B`, `00973964`, `009739CB`, `00973A57`, `00973D88`, `00973FD6`, `00974042`, `0097409E`, `009740F1`, `009742B7`, `0097437B`, `009743C1`, `00974407`: §§2–9. |
| Internal dispatch/key state | `00972EAE`, `00972F1D`, `00972FAE`, `00972FF2`, `00973023`, `009733C6`: §§2/6. |
| Manager lifecycle/thunks | `00973E67`, `00973ED3`, `00973EDB`, `00973EE3`, `00973EEB`, `0097356D`, `0097429B`, `009744B2`: constructor, member destruction/deletion, interface-adjusting QI/deletion thunks, Shutdown and update (§9). |
| Factory/type hash support | `0050B9F0`, `00973AC4`, `00973B37`, `0097420D`, `00974231`, `00974496`: allocate/copy scalar pair node, duplicate insertion/resize dispatch, initialize type table (§4.1). No hidden factory creation/removal policy in these helpers. |
| Deque movement/index/erase | `0097304D`, `009730A9`, `0097316E`, `00973602`, `00973630`, `00973669`, `00973697`, `009736DB`, `00973AAD`, `00973BAA`, `00973BE8`, `00973C0E`, `00973D06`, `00973D47`, `00973DCC`, `00973E0B`, `00973E4A`, `00974159`: pointer iterator advance/retreat/index; bidirectional range copy and forwarding wrappers; pop/clear/free block storage; remap/grow pointer-block map; erase preserving survivor order (§5). |
| Deque end insertion | `00892252`, `00974255`, `00974337`, `0097435C`: append/prepend in current block or allocate a new block after map capacity check (§5). |
| Type filter | `00973194`, `009731A5`: type-only predicate and deletion (§8.2). |
| Built-in cache entries | `0097BE6C`, `0097BE8E`, `0097BEBA`, `0097BEF6`, `0097BF1C`, `0097BF43`, `0097BF63`, `0097BFA4`, `0097BFAC`, `0097BFCA`, `0097C23F`, `0097C26D`, `0097C2D3`, `0097C36E`, `0097C3BB`, `0097C41A`, `0097C452`, `0097C48F`, `0097C53E`, `0097C595`, `0097C613`: §8.1. |
| Value/vector support | `0097BF85`, `0097C55A`, `00997959`: four-word value copy; initialize empty hash bucket vector with prime count and zeros; insert repeated pointer value, moving/reallocating storage when needed. This is storage management, not extra cache/factory acceptance logic. |

Numeric decisions were checked in native instructions, in particular unsigned full-key comparison, hash
division, counter order/width, tick's signed threshold, purge's unsigned count test, duplicate link order,
record flags, output writes and write-status preservation. Misleading folded names were not treated as
method semantics. All required pseudocode was present in local decomp files; no REA query was necessary.
