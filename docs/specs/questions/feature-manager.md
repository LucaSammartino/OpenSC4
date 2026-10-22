# feature-manager: open questions
<!-- Questions about [feature-manager.md](../feature-manager.md). Append new entries at the END; when one is answered, move it to answered/feature-manager.md. Rules: README.md. -->

<!-- from: re-featuremgr (2026-10-19) -->
- **FM1 - §6:** is slot 5 `IsFeatureEnabled` or slot 6 `IsExpansionPackInstalled` called anywhere in the shipped
  executable? Two whole-section scans (spec §6) find only slot-7 users: `cSC4Nation::Init` (through the cache
  `0x00B43C98` that `SL::OnAppInit1` fills) and `cSC4Nation::ResetTutorialCities` (through `cISC4App` slot 32). A
  manager pointer kept in memory and used later would escape both. An in-game probe that counts calls of slots 5
  and 6, with return addresses, over one session would settle it. [reachability only: the spec's answers stand
  either way]

<!-- from: re-featuremgr (2026-10-19) -->
- **FM2 - §6:** the key `tutorial_registry`'s `add_default_region(n, dir)` writes into `default_regions`, and so
  whether pack 1 reaches `timbuktu` through key 1 of the registry's map (`0x004B79A4`). Belongs with the tutorial
  registry. [reachability only: the spec's answers stand either way]

<!-- from: core-feature-manager4 (2026-10-22) -->
- **FM3 - §4.1 file/INI read failures:** after a resource resolves successfully, what happens if its file cannot
  be read, or fails partway through `[Features]`? Are entries already enumerated retained, is the file always
  released, and does scanning continue through later packs? [The portable raw-byte provider requires readable
  file bytes; resolved malformed/empty files install the pack, unresolved keys do not. Native streaming I/O
  failure behavior is outside this provider contract pending an answer; no read failure is converted to a miss.]

<!-- from: core-feature-manager4 (2026-10-22) -->
- **FM4 - §3 AddRef signed-count overflow:** does incrementing `0x7FFFFFFF` wrap to `0x80000000` and return those
  register bits, so the following signed Release destroys the object without decrementing? [Literal 32-bit
  increment wraps in the portable owner; the signed Release test is applied to that resulting count.]
