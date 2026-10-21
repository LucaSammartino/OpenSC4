# Answered questions: lot-configuration.md

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**GLUE21-1** (`lot-configuration.md` §1): `cSC4LotConfiguration` is not registered with GZCOM for creation
  (0xC0 bytes, ctor `0x004797A0`). Is there a named route to the game's allocate-and-construct pair, or must the
  configuration class itself be reimplemented? This blocks `enable = lotcfg`.~~ **ANSWERED 2026-10-16
  (re-answers31): PARTLY.** There **is** a named route and it is not GZCOM: `FUN_00489910` (`0x00489910`)
  allocates `0xC0` bytes (`FUN_005E55E0`), constructs at `0x004797A0`, AddRefs, `Load(key, detail)` at slot 31,
  and inserts (`FUN_00648F00`). The public routes are manager slots 5, 6 and 22, and the **manager is
  GZCOM-creatable** (CLSID `0x299062C0`). Replace or drive the manager; do not probe GZCOM for the
  configuration class. Written into [lot-configuration.md](../../lot-configuration.md) **§1.1**.
