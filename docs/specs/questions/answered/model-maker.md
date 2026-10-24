# model-maker: answered questions (archive)
<!-- Archive for [model-maker.md](../../model-maker.md). Lanes do not read this file. Open questions are in ../model-maker.md. -->

<!-- from: re-modelmaker (2026-10-22) -->
- ~~**MM2 - §14, §20:** the model-loading bodies outlined in §14 or listed as remainder: `AddModels` (`0x00494390`),
  `AddModel` (`0x004902E0`), `LoadModelsCommon` (`0x00490C70`), `LoadFoundationModels` (`0x00491040`),
  `FindModelTransform` (`0x0048FA40`), `CreatePlaceHolderModelInstance` (`0x0048F920`), `CreateOccupantShadow`
  (`0x00491230`), `DoTerrainHidingForCells` (`0x00491CD0`) and `RebuildDynamicTexture` (`0x00492CC0`), and the
  out-of-class helpers of §20. [outline]~~
  **ANSWERED 2026-10-23 (re-modelmaker7):** MATCHES the outline, with details it lacked. Every body is now
  specified from the instruction bytes:
  - `AddModels` §14: "every zoom" sets the rotation mask to all as well; the building and prop results are the
    state effect's;
  - `AddModel` §12.11; `LoadModelsCommon` §12.7: only the first model sets the result and gets the building flag
    `0x40` and the development call;
  - `LoadFoundationModels` §12.8; the transform record and `FindModelTransform` §12.10;
    `CreatePlaceHolderModelInstance` §12.9; `CreateOccupantShadow` §12.12; `DoTerrainHidingForCells` §13.2;
  - `RebuildDynamicTexture` §11.2, with the tint kernels `0x008D08E0`..`0x008D0E10` in §11.3 and §11.3.1.

  The helpers `0x00445F10`..`0x00447B00` are owned by `cSC4AnimationCoordinator` (animation-coordinator.md). Both
  owners are claimed in tools/coverage-map.tsv.
