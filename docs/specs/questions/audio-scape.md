# audio-scape: open questions
<!-- Questions about [audio-scape.md](../audio-scape.md). Append new entries at the END; when one is answered, move it to answered/audio-scape.md. Rules: README.md. -->

<!-- from: re-scape7 (2026-10-23) -->
- **SCP1 - section 6.2:** nothing in the owner adds to the pending-cluster list `+0x58`, so the countdown branch of
  `OnTick` never runs. Is there a caller outside the audio owners that fills it, or is it dead? [dead]

<!-- from: re-scape7 (2026-10-23) -->
- **SCP2 - sections 3.2, 4.3, 5.1:** no caller found sets flag bits `0x4` (untracked members) or `0x10` (keep
  clusters on clear); every scape seen has flags 0 or 1. Do any data-driven scapes use them? [not used]

<!-- from: re-scape7 (2026-10-23) -->
- **SCP3 - section 1:** what is the third interface at `+0x08` (vtable `0x00A942F8`, slot 3 `0x004F5200` returning
  whether its argument is non-zero), which `QueryInterface` never returns? [an unused callback interface]

<!-- from: re-scape7 (2026-10-23) -->
- **SCP4 - section 3.4:** the sound definition's `extra`, `flag` and `f` words are stored and copied but never read
  by this owner; does the sound manager read them through the instance pointer it receives? [no: the instance is
  only an id]
