# resource-catalog: open questions

<!-- from: re-resource-catalog4 (2026-10-23); native research boundaries, not portable implementation claims -->
- **RC1 — §2 enumeration/row pointer lifetime:** resource service slot 48 supplies a type enumeration; updateKeyList stores raw key pointers and has no enumeration Release or per-key reference calls. What are service ownership and validity guarantees across refresh, list clearing and resource database mutation? Do not infer owned/leaked enumeration from absence of Release.

<!-- from: re-resource-catalog4 (2026-10-23) -->
- **RC2 — §2 displayed order and names:** identify the exact list interface/slot 20C comparator, direction, equal-label stability and malformed name behavior, plus helper 006695A0's resource-name resolution/failure contract. Owned filtering and post-order selection are specified independently; no numeric key ordering or stable tie order is claimed.

<!-- from: re-resource-catalog4 (2026-10-23) -->
- **RC3 — §2/§8 descriptor registration and empty filters:** settle 006866E0/00686B90 Ini entry grammar, incomplete/malformed fields, duplicate descriptor handling, descriptor preview creation failures, and Init's empty-admitted-set/map entry zero outcome. Catalog setters and Init's literal zero-wildcard predicate are proven; this is not permission to invent registration defaults or repair undefined null dereferences.

<!-- from: re-resource-catalog4 (2026-10-23) -->
- **RC4 — §4/§8 picker and add-resource boundaries:** settle 00685D70's owned resource-creation path, variant type/storage validity for picker slot 18 input and slot 1C output, modal initialization failures, and undecoded adjusting thunks. Known output permutation and zero-on-no-selection do not prove acceptance success, resource creation or an invalid-variant rejection rule. No table/property/value/curve question is reopened.

<!-- from: re-resource-catalog4 (2026-10-23) -->
- **RC5 — §5 text parser, notification ownership and split export:** settle foreign RZConvertToUint32A malformed/prefix/overflow behavior only as needed for the key-input consumer; confirm QI ownership for 612CE0C3 in the text notification path (no Release there, unlike spinner change), and recover correct instruction boundaries across 0068262E–0068263C. Current contract forwards parser result unchanged and gives no clean failure rule for invalid controls/storage. No occupied-set termination bound exists in getNextID.
