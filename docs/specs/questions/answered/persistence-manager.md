# persistence-manager: answered questions

<!-- from: core-persist-manager4 (2026-10-22) -->
- ~~**PR5 - section 8.1 cache slot 12:** after a resource's current key changes independently of its cache entry, are the stored and supplied enumeration predicates given the entry's stored key or resource.GetKey()? The spec explicitly distinguishes lookup's stored key and the enumerated output's current key but does not identify the predicate input. [literal implementation: predicate on the stored entry key; append the current resource key]~~

  **ANSWERED 2026-10-22 (re-persist-feature-contracts4):** DIFFERS — both predicates and the append use the same once-copied resource.GetKey(), before either predicate. Lookup/traversal still use the stored entry. See persistence-manager.md §8.1 and independent vectors §10.1a; native `0x0097C306–0x0097C34B`.
