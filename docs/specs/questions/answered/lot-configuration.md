# Answered questions: lot-configuration.md

<!-- from: glue-lotconf-sim-shadow6 (2026-10-24) -->
- **GLUE22 — §4 native capacity and cost comparison:** provide a named interface/slot contract for the building-capacity lookup used by FillBuildingInformation, including parameters, return channel and ownership. The configuration's documented slots expose min/max capacity but the independent core manager's building_capacity host cannot currently obtain its input (the existing plugin host returns 0). Also identify any lot-configuration cost getter if one exists; none is specified among the 34 slots. [Compare independent size/compatibility/object derivation now; leave occupancy and costs unmeasured rather than copying native outputs as expected values.]
  **ANSWERED 2026-10-24 (re-lotconf-glue22):** MATCHES — `cISC4BuildingDevelopmentSimulator` IID
  `0x07813EAE`, slot 39 (`+0x9C`, `0x0069CA60`), takes uint32 building type (thiscall, pop 4) and returns
  a borrowed `BuildingProfile&` address in EAX; capacity is uint16 at +8. Unknown types insert a zero profile;
  slot 40 is the non-inserting nullable alternative. Expand families first; a direct valid type is included
  even when expansion returns 0. Native min/max getters return AX. There is no configuration cost getter:
  profile plop/budget costs are per building type and define no family aggregate. Capacity inputs from the
  native profile measure configuration reduction only; independently expected profile capacities must come
  from resource properties. Written into [lot-configuration.md](../../lot-configuration.md) §§4.1–4.2,
  conformance cases 15–19; occupancy and costs remain unmeasured by the existing shadow.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**GLUE21-1** (`lot-configuration.md` §1): `cSC4LotConfiguration` is not registered with GZCOM for creation
  (0xC0 bytes, ctor `0x004797A0`). Is there a named route to the game's allocate-and-construct pair, or must the
  configuration class itself be reimplemented? This blocks `enable = lotcfg`.~~ **ANSWERED 2026-10-16
  (re-answers31): PARTLY.** There **is** a named route and it is not GZCOM: `FUN_00489910` (`0x00489910`)
  allocates `0xC0` bytes (`FUN_005E55E0`), constructs at `0x004797A0`, AddRefs, `Load(key, detail)` at slot 31,
  and inserts (`FUN_00648F00`). The public routes are manager slots 5, 6 and 22, and the **manager is
  GZCOM-creatable** (CLSID `0x299062C0`). Replace or drive the manager; do not probe GZCOM for the
  configuration class. Written into [lot-configuration.md](../../lot-configuration.md) **§1.1**.
