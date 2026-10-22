# Persistence manager attributed-body mapping

Source: `docs/specs/persistence-manager.md` sections 1–11, including PR4's
initialized logical-OR policy and PR5's current-key enumeration contract. The
109 starts below are exactly the section 11
checklist (8,437 native bytes); they identify contracts, not code translations.
All algorithms execute in this Rust module. `Primitives` exposes foreign
factory/resource/record/segment/filter/cache calls only, never manager outputs.
The built-in cache always uses this module, never foreign cache delegation.

This is a portable owner with symbolic identities and exclusive Rust borrows.
Lock events preserve native recursive scopes and order for an embedding; there
is no native ABI or live-game integration. Rust storage replaces native node
layouts. Segment element blocks still contain 32 slots, with owner-local
movement and erase; VecDeque handles the block-pointer map's allocation.
Type-filter temporaries use owned `Filter::Type` values and Rust destruction.
Final Release/destruction and explicit Shutdown remain different operations.

PR5 (§8.1 slot 12, §10.1a): enumeration copies each visited resource's current
key once, before stored then supplied predicates, and appends that same copy.
External SetKey does not reindex the stored lookup key. Resource-key mutation
from a predicate affects subsequent enumerations, not this entry's copied key.
This implements the published contract; it is not measured native equivalence.

PR4 (§9.2) deliberately uses initialized canonical logical OR and visits every
cache. The native initial byte is uninitialized: empty/all-false native returns
are not determinate, and even a true callback does not fix every returned bit.
The Rust policy does not reproduce that byte or claim exact native returns.
Stable tables/registries and valid objects remain the callback preconditions;
native reentrancy and unspecified domains have no equivalence claim.

The three `cache.rs` PR5 tests exercise the published A-D predicates, E's
duplicate/current-key and null-output cases, and F's mutation from either
predicate. Exact primitive traces check GetKey once before filtering, stored
then supplied order, short-circuit rejection, and no enumeration AddRef/Release.
They also check old-key lookup, type filtering, sentinel preservation, later
observation of SetKey, and balanced cache/filter ownership after cleanup.
The concrete portable KeyList always succeeds on append; PR5-E's foreign list
returning false is not executable through this API. The owner ignores insert's
return, but no native list-failure or ABI comparison is claimed by these tests.
The 109-row mapping below is a complete source-contract inventory, not a
measurement of native executions or runtime owner coverage.

| Body start | Rust implementation |
|---|---|
| 00972B51 | PersistManager::query_interface |
| 00972B78 | open_record |
| 00972BBF | close_record |
| 00972BDC | write_key |
| 00972C24 | write_resource |
| 00972CDC | write_keys |
| 00972D30 | factory_for_resource |
| 00972D4E | factory_for_key |
| 00972D60 | load_uncached |
| 00972DA2 | create_as |
| 00972E2B | register_resource_as |
| 00972E4B | register_resource |
| 009730CC | create |
| 009731C2 | cached_keys |
| 00973239 | cached_keys_of_type |
| 00973280 | segment_keys_of_type |
| 009732C7 | factory_count |
| 009732EB | load |
| 0097341B | unregister_resource |
| 00973485 | is_cached |
| 009734DC | automatic_purge |
| 00973526 | set_automatic_purge |
| 009735B8 | purge |
| 0097371F | has_segment |
| 0097377F | segment_for_key |
| 009737FA | segment_by_id |
| 00973873 | segment_count |
| 009738A2 | segment_array |
| 0097391B | visit_segments |
| 00973964 | is_in_segments |
| 009739CB | segment_keys |
| 00973A57 | remove_cache |
| 00973D88 | segment_at |
| 00973FD6 | remove_factory |
| 00974042 | factory_for_type |
| 0097409E | factory_at |
| 009740F1 | register_cache |
| 009742B7 | remove_segment |
| 0097437B | prepend_segment / move_segment |
| 009743C1 | append_segment / move_segment |
| 00974407 | register_factory |
| 00972EAE | load_underlying |
| 00972F1D | load_selected |
| 00972FAE | create_underlying |
| 00972FF2 | current_group |
| 00973023 | allocate_instance |
| 009733C6 | insert_cache |
| 00973E67 | destroy / Drop (not Shutdown) |
| 00973ED3 | symbolic InterfaceView QI forwarding |
| 00973EDB | symbolic InterfaceView destruction forwarding |
| 00973EE3 | symbolic InterfaceView destruction forwarding |
| 00973EEB | shutdown |
| 0097356D | update: explicit PR4 choice, not native byte oracle |
| 0097429B | destroy(flags) |
| 009744B2 | new / ServiceState / CacheStore::create |
| 0050B9F0 | Table scalar-key/value node ownership and copy |
| 00973AC4 | Table::insert_duplicate / grow (factories) |
| 00973B37 | Table::insert_duplicate / grow (types) |
| 0097420D | register_factory / factory-table insertion |
| 00974231 | register_factory / type-table insertion |
| 00974496 | Table::new (types) |
| 0097304D | PointerDeque::iter / indexed block advance |
| 009730A9 | PointerDeque::erase / reverse block movement |
| 0097316E | PointerDeque::get block indexing |
| 00973602 | PointerDeque::erase forward range movement |
| 00973630 | PointerDeque::erase shorter-side selection |
| 00973669 | PointerDeque::erase bidirectional forwarding |
| 00973697 | PointerDeque::erase reverse range movement |
| 009736DB | PointerDeque::get / iterator index |
| 00973AAD | PointerDeque::iter / logical iteration |
| 00973BAA | PointerDeque::erase front pop/block reclamation |
| 00973BE8 | PointerDeque::erase back pop/block reclamation |
| 00973C0E | PointerDeque::clear |
| 00973D06 | PointerDeque::clear block ownership/drop |
| 00973D47 | PointerDeque::get |
| 00973DCC | VecDeque-owned block-map remap/growth |
| 00973E0B | PointerDeque::clear / block-map destruction |
| 00973E4A | PointerDeque::erase / block end cleanup |
| 00974159 | PointerDeque::erase survivor-order preservation |
| 00892252 | PointerDeque::push_back current-block insertion |
| 00974255 | PointerDeque::push_back new-block allocation |
| 00974337 | PointerDeque::push_front current-block insertion |
| 0097435C | PointerDeque::push_front new-block allocation |
| 00973194 | Filter::Type predicate |
| 009731A5 | owned Filter::Type scope/drop |
| 0097BE6C | BuiltinCache::query_interface |
| 0097BE8E | BuiltinCache::add_ref: return OLD count |
| 0097BEBA | BuiltinCache::release |
| 0097BEF6 | BuiltinCache::automatic_purge |
| 0097BF1C | BuiltinCache::set_automatic_purge |
| 0097BF43 | BuiltinCache::update / update_with |
| 0097BF63 | BuiltinCache::purge / purge_with |
| 0097BFA4 | BuiltinCache::set_filter |
| 0097BFAC | BuiltinCache::accepts |
| 0097BFCA | BuiltinCache::enumerate_type |
| 0097C23F | BuiltinCache::destroy: no resource Clear |
| 0097C26D | BuiltinCache::shutdown / shutdown_with |
| 0097C2D3 | BuiltinCache::enumerate (PR5 current-key copy) |
| 0097C36E | BuiltinCache::get |
| 0097C3BB | BuiltinCache::erase |
| 0097C41A | BuiltinCache::contains |
| 0097C452 | BuiltinCache::clear |
| 0097C48F | BuiltinCache::purge_one |
| 0097C53E | BuiltinCache::destroy(flags) |
| 0097C595 | BuiltinCache::insert / Table::insert_unique |
| 0097C613 | BuiltinCache::new / CacheStore::create |
| 0097BF85 | Rust Key/Object tuple value copies |
| 0097C55A | Table::new / zeroed bucket vector |
| 00997959 | Vec-owned bucket/map allocation and relocation |

Inherited manager AddRef/Release, slot 26 tail-forwarding, service Init and
cache Init/get-filter are implemented as well; their folded addresses are not
extra selected bodies. No DBPF parsing or data-format code receives manager
credit from this mapping.
