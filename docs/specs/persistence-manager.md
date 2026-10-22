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

## 4. Audit status

This intermediate contract is incomplete. Remaining audit: factory/type registries, segment mutation,
key generation, write/failure returns, default-cache methods, lifecycle and every attributed helper.
No coverage-map row is proposed until that audit is complete.
