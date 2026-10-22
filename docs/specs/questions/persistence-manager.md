# persistence-manager: open questions
<!-- Questions about persistence-manager.md. Append new entries at the end. -->

<!-- from: core-persist-manager4 (2026-10-22) -->
- **PR5 - section 8.1 cache slot 12:** after a resource's current key changes independently of its cache entry, are the stored and supplied enumeration predicates given the entry's stored key or resource.GetKey()? The spec explicitly distinguishes lookup's stored key and the enumerated output's current key but does not identify the predicate input. [literal implementation: predicate on the stored entry key; append the current resource key]
