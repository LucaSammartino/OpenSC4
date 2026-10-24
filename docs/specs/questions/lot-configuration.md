# lot-configuration: open questions
<!-- Engine implementer questions; answers belong in the behaviour spec. -->

<!-- from: glue-lotconf-sim-shadow6 (2026-10-24) -->
- **GLUE22 — §4 native capacity and cost comparison:** provide a named interface/slot contract for the building-capacity lookup used by FillBuildingInformation, including parameters, return channel and ownership. The configuration's documented slots expose min/max capacity but the independent core manager's building_capacity host cannot currently obtain its input (the existing plugin host returns 0). Also identify any lot-configuration cost getter if one exists; none is specified among the 34 slots. [Compare independent size/compatibility/object derivation now; leave occupancy and costs unmeasured rather than copying native outputs as expected values.]
