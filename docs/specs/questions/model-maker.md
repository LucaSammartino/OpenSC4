# model-maker: open questions
<!-- Questions about [model-maker.md](../model-maker.md). Append new entries at the END; when one is answered, move it to answered/model-maker.md. Rules: README.md. -->

<!-- from: re-modelmaker (2026-10-22) -->
- **MM1 - §15.2, §15.4:** the units of the loader's response curves (the time limits look like microseconds, the
  sleeps likewise) and how a curve is read between its points (piecewise linear is inferred). The memory figure
  is settled (MiB, §7.1). [inferred]

<!-- from: re-modelmaker (2026-10-22) -->
- **MM2 - §14, §20:** the model-loading bodies outlined in §14 or listed as remainder: `AddModels` (`0x00494390`),
  `AddModel` (`0x004902E0`), `LoadModelsCommon` (`0x00490C70`), `LoadFoundationModels` (`0x00491040`),
  `FindModelTransform` (`0x0048FA40`), `CreatePlaceHolderModelInstance` (`0x0048F920`), `CreateOccupantShadow`
  (`0x00491230`), `DoTerrainHidingForCells` (`0x00491CD0`) and `RebuildDynamicTexture` (`0x00492CC0`), and the
  out-of-class helpers of §20. [outline]
