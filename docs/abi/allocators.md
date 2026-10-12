# Memory ownership across the game boundary

These are interface facts (see [README.md](README.md)): which allocator owns the memory of containers that the
game and the plugin both touch. They were read from the target executable (SimCity 4 Deluxe 1.1.641, GOG) and
confirmed in-game (runs 12–16 in `docs/manual-checks.md`). No logic is recorded here.

## Three allocator instances

The game has three instances of the same small-block pooled allocator class. **Each instance owns its own
per-size-class pools.** A block must be freed to the instance that allocated it. A block freed to another instance
is linked into that instance's pools while still belonging to the first one. Later, both instances hand out the
same memory, and the game crashes much later in an unrelated place (seen as an access violation at exe
`0x005E16F5` in the pool allocator).

| Instance | Allocate | Free | Used by |
|---|---|---|---|
| **Global heap** (static object at `0x00B230E0`) | `0x005E55E0` (cdecl `void* (uint32_t size)`) | `0x005E5620` (cdecl `void (void*)`) | the game's `operator new` / `operator delete` |
| **STL node allocator** (static object at `0x00B62698`) | `0x0090CF54` (cdecl `void* (uint32_t size)`) | `0x0090CF63` (cdecl `void (void*)`) | the game's standard containers declared with the default allocator (see the table below) |
| **Framework allocator service** (a heap object created at start-up; service ID `0x3AE4BEA3`, IID `0x3AE4BEAB`) | `cIGZAllocatorService::Allocate`, slot 3 | `cIGZAllocatorService::Deallocate`, slot 4 | containers declared with `cGZAllocatorServiceSTLAllocator`, and the gzcom-dll SDK's `SC4List`, `SC4Vector` and `SC4HashSet` |

The SDK containers always use the framework service. They are therefore only compatible with game containers
whose allocator is also the service.

Every block from these allocators has a 4-byte header in front of it. The byte just before the returned pointer is
the size class. The headers are the same in every instance, so they cannot tell the instances apart.

## Containers filled or freed by the other side

| Method | Container | Game allocator | SDK container compatible? |
|---|---|---|---|
| `cISC4LotManager::GetLotsInCellRect` (slot 7) | list of lots, 12-byte nodes `{next, prev, value}`, one reference per lot | STL node allocator | **no**: free the nodes with `0x0090CF63` |

Other crossings (the remaining lot-manager list queries, neighbour connections, regional cities, the budget
vectors, `EnumerateAgentsByName`, strings) are being classified; until they are listed here, treat them as unknown.

## GetLotsInCellRect

With the last argument `false`, the method:
1. first clears the list it is given, freeing every node with the STL node allocator (`0x0090CF63`) but keeping
   the head node;
2. then appends one node per lot (lots spanning several cells appear once), allocated with `0x0090CF54`;
3. takes one reference on each lot;
4. returns the number of lots appended.

The caller owns the nodes and the references. The caller must therefore:
- release each lot once;
- free each node with `0x0090CF63`;
- free its own head node with whichever allocator allocated it.

Never pass a list that already contains nodes from another allocator: step 1 frees them with the STL node
allocator.
