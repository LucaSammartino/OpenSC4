# advisors: open questions
<!-- Questions about [advisors.md](../advisors.md). Append new entries at the END; when one is answered, move it to answered/advisors.md. Rules: README.md. -->

<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD7 - section 8.2:** in which order are the 38 simulation and 9 UI collector entries in the registration vector (which collector first, and the order inside each)? It decides the final `_tf_` global. [simulation collector first, §8.3 table order]
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD8 - section 12.1 slots 9 and 11:** who calls `GetRandom()` and which `cRZRandom` methods/layout must the returned object provide; and what does system slot 11 `DoLink(const cIGZString&)` do (body `0x41B080`)? [slot 9 returns null, slot 11 returns false]
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD9 - section 5.2:** where does a cheat notification (`cIGZMessageTarget::DoMessage(cGZMessage&)`) carry the typed cheat text (`Adv reload` etc.): which `cGZMessage` field and what type? [only the cheat ID is passed]
<!-- from: glue-advisors-final4 (2026-10-21) -->
- **AD10 - section 4.2 / lua-host.md 7.3-7.5:** may a replacement call `UpdateResourcesFromDirectory` `0x005FC510`, `LoadPackageScripts` `0x005FBA20` and register `_dofile_from_resource` `0x005FC020` as they are (calling convention, argument types, who pops), like the AD5/AD6 Lua entries? Also: which directories are `<data directory>` and `<plugin directory>` (which `cISC4App` getters)? [not called yet; reload_scripts left unset]
