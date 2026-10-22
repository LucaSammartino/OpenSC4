# Native lot configuration editor: exact body inventory

Companion to [the behavior contract](lot-configuration-editor.md), §6.
Measured 2026-10-22 from main-checkout `build-runs/coverage-owners-base.tsv`,
`ghidra/export/windows/functions.tsv` and `function-details.tsv`, with native
vtable/byte checks where identified below. Only this owner was investigated.

The attribution snapshot has **111 rows / 52,880 bytes**. Mapping its addresses
to current exported function entries gives **109 entries / 52,646 body bytes**,
exactly the brief's owner52646B label. Its 92 named editor entries account for
51,054 bytes; the other 17 current exported entries account for 1,592 bytes.
This arithmetic is an inventory reconciliation, **not complete-owner spec credit**.

The difference is exactly:

- `0x00604920`: attributed 159, current exported body 169 (+10), because the
  latter includes `0x00604B6B..0x00604B74` as well as `0x00604920..0x006049BE`.
- `0x006049CD`: attributed 7, current exported body 180 (+173), with ranges
  `0x006049CD..0x006049D3`, `0x006049D6..0x00604A66`,
  `0x00604B4F..0x00604B6A`. The export labels this a library switch case.
- `0x006049D4` (147 attributed bytes) and `0x00604A67` (270) are labelled
  interior switch/default fragments, **not current exported function entries**.
  Do not count their older spans as two additional current functions.

Thus 52,880 + 10 + 173 − 147 − 270 = 52,646. Current exported body ranges
are the unit of this count; this is not the physical span between entry and
highest addressed instruction. The shared image decoder is reached from
`GetAllLotBaseTextureTypes` at `0x0047FDCD`; ownership of that decoder family
is not promoted merely because this editor calls it.

The second boundary issue is separate from that arithmetic: Save As actually
enters at `0x00485390` (vtable `0x00A8DAE8`). The attributed `0x00485445`
entry starts inside an instruction. Its 700 bytes remain in the table to make
the measured set reproducible, but its contract covers a corrected 881-byte
continuous workflow including supplemental 180-byte prefix and one uncovered
byte. No coverage-map, ABI table or name database was edited.

Status: **C** = focused body contract recorded; **P** = established subset with
explicit unresolved details; **O** = open, no body contract; **R** = attribution/
fragment requiring separate shared-helper treatment. C does not override open
dependencies or imply owner completion. References are sections of the behavior
spec or question IDs in [the question file](questions/lot-configuration-editor.md).
Rows are ordered by attributed size, largest first. Addresses and byte counts
are hexadecimal and decimal respectively; function names omit the class prefix.

| Entry | Attributed B | Current export B | Method / label | Contract |
|---|---:|---:|---|---|
| 00480D10 | 6818 | 6818 | DoSetLotCompatibilitiesUI | C §4 |
| 00487B30 | 5971 | 5971 | DoLotConfigurationObjectMenu | P §3, LE4 |
| 004871B0 | 2425 | 2425 | DoEditAreaMenu | C §2 |
| 0047CF90 | 1652 | 1652 | DrawLotConfigurationObject | P §13, LE5 |
| 004827D0 | 1514 | 1514 | DoSelectBuildingFamiliesUI | P §12, LE4 |
| 004832E0 | 1503 | 1503 | DoSelectPropFamiliesUI | P §12, LE4 |
| 00483DC0 | 1373 | 1373 | DoHitTest | O LE5 |
| 00482DC0 | 1304 | 1304 | DoSelectFloraFamiliesUI | P §12, LE4 |
| 00484850 | 1236 | 1236 | AddNewLotConfigurationObject | P §9.3, LE3 |
| 0047E060 | 1193 | 1193 | DrawCurrentLotConfiguration | P §13, LE5 |
| 00480910 | 1021 | 1021 | DoSetRetainingWallUI | O LE4 |
| 0047CC30 | 850 | 850 | DoSelectTexturesUI | P §12, LE4 |
| 00485D60 | 799 | 799 | DoSetFoundationUI | O LE4 |
| 0047FC50 | 726 | 726 | GetAllLotBaseTextureTypes | O LE5 |
| 00485710 | 717 | 717 | GZOnKeyDown | P §11, LE5 |
| 00485445 | 700 | 700 | SaveAs (false entry) | C §8.2; true entry 00485390 |
| 0047F200 | 640 | 640 | GetIdentityInfoForLotConfigurationExemplarPath | C §8.4 |
| 0047BBA0 | 638 | 638 | Prepare3DView | O LE5 |
| 0047DDF0 | 618 | 618 | DoSet3DViewSlopeUI | O LE4 |
| 00486080 | 614 | 614 | SetLotSize | O LE4 |
| 0047C660 | 612 | 612 | CycleSelectedObject | O LE5 |
| 00486930 | 610 | 610 | DoLotFoundationMenu | O LE4 |
| 00485B00 | 605 | 605 | DoOutsideAreaMenu | O LE4 |
| 00484E10 | 557 | 557 | DuplicateObject | C §9.3 |
| 0047AE20 | 547 | 547 | LoadCursors | O LE5 |
| 0047F480 | 541 | 541 | GetLotConfigurationObjectName | O LE5 |
| 0047E650 | 531 | 531 | DoSetNetworkNodePathUI | O LE4 |
| 0047F750 | 510 | 510 | GetBuildingFamilyName | O LE5 |
| 00484330 | 494 | 494 | RemoveOutOfBoundsLotConfigurationObjects | C §9.2, §9.4 |
| 0047B890 | 488 | 488 | PostInit | P §7, LE5 |
| 00486F20 | 473 | 473 | StaticInit | O LE5 |
| 00484530 | 462 | 462 | AddLotFoundation | C §9.5 |
| 0047FA80 | 457 | 457 | GetPropFamilyName | O LE5 |
| 004805B0 | 447 | 447 | DoFenceMenu | O LE3 |
| 0047B480 | 443 | 443 | MoveObjectToWithinLotBounds | C §9.2, §9.6 |
| 00486770 | 434 | 434 | DoEditMouseDownL | P §11, LE5 |
| 00480170 | 411 | 411 | DeleteLotConfigurationObject | C §9.3 |
| 00480770 | 409 | 409 | DoSetLotNameUI | O LE4 |
| 0047ABA0 | 406 | 406 | DoDialogBoxGridSort | C §12.1; raw sort options specified |
| 00486BA0 | 398 | 398 | DoSetLotSizeUI | O LE4 |
| 0047E890 | 377 | 377 | ToggleFence | C §9.5; allocation failure not normalized |
| 0047DA80 | 375 | 375 | cSC4WinLotConfigurationEditor | C §7 |
| 00485040 | 353 | 353 | MoveObject (key) | P §9.1, LE3 |
| 00483B20 | 342 | 342 | CreateNewLotConfiguration | C §8.1 |
| 0047FF30 | 335 | 335 | GetFoundationNameAndImplementationType | O LE5 |
| 0047F0B0 | 327 | 327 | SetTitle | P §1, LE5 |
| 0047C510 | 322 | 322 | Shutdown | C §7; destructor remains LE5 |
| 00484700 | 322 | 322 | ToggleFence | P §9.5; invalid conversion LE3 |
| 004892B0 | 295 | 295 | Init | P §7, LE5 |
| 004859E0 | 286 | 286 | DoEditMouseMove | P §11, LE5 |
| 00483C80 | 281 | 281 | ReadLotConfigurationExemplar | C §8.1 |
| 00604A67 | 270 | — | interior switch case | R; no exported entry |
| 0047CA60 | 262 | 262 | DoDialogBoxScrollToLetter | C §12.1 |
| 0047B100 | 238 | 238 | SetSelectedLotConfigurationObject | C §1 |
| 004852A0 | 238 | 238 | Save | C §8.2; dependency LE2 |
| 00480080 | 228 | 228 | GetRetainingWallName | O LE5 |
| 0047D6B0 | 212 | 212 | VerifyObjectsAreWithinLotBounds | P §9.2, LE3 |
| 0047AD40 | 211 | 211 | GZOnMouseUpL | O LE5 |
| 00486690 | 211 | 211 | GZOnMouseMove | O LE5 |
| 004865D0 | 191 | 191 | Close | C §8.3 |
| 0047B640 | 185 | 185 | FUN_0047B640 (bounds predicate) | P §9.2, LE3 |
| 0047CB70 | 181 | 181 | DoShowHelpUI | O LE5 |
| 0047F950 | 174 | 174 | GetFloraFamilyName | O LE5 |
| 00487100 | 174 | 174 | GZOnMouseDownL | O LE5 |
| 0047A930 | 173 | 173 | FUN_0047A930 | O LE5 |
| 004895A0 | 169 | 169 | GZOnMouseDownR | O LE5 |
| 004893E0 | 166 | 166 | DoEditMouseDownR | O LE5 |
| 0047B1F0 | 165 | 165 | GetIdentityInfoForLotConfigurationExemplarKey | C §8.4 |
| 00604920 | 159 | 169 | FUN_00604920 (shared image decoder) | R LE5 |
| 006049D4 | 147 | — | interior switch default | R; no exported entry |
| 0047D610 | 146 | 146 | MessageBoxA | O LE5 |
| 00486530 | 145 | 145 | Open | C §8.1; dependency LE2 |
| 0047DD60 | 136 | 136 | DoMessage (dialog callback) | C §12.1 |
| 0047B700 | 133 | 133 | GoTo3DView | O LE5 |
| 0047B390 | 130 | 130 | FUN_0047B390 | O LE5 |
| 0047F6C0 | 130 | 130 | GetBuildingName | O LE5 |
| 00484D40 | 130 | 130 | RotateObject | C §9.1, finite-input rule |
| 0047FA00 | 127 | 127 | GetPropName | O LE5 |
| 00483A20 | 126 | 126 | DoAddNewProp | O LE4 |
| 004851B0 | 119 | 119 | MoveObject (position) | P §9.1, LE3 |
| 0047B310 | 114 | 114 | FUN_0047B310 | O LE5 |
| 00483920 | 114 | 114 | DoAddNewBuilding | O LE4 |
| 004839A0 | 114 | 114 | DoAddNewFlora | O LE4 |
| 00483AA0 | 114 | 114 | DoAddNewLotBaseTexture | O LE4 |
| 0047B2A0 | 102 | 102 | FUN_0047B2A0 | C §9.4 |
| 0047BAD0 | 101 | 101 | GetIdentityInfoForLotConfigurationType | C §8.4 |
| 004838C0 | 92 | 92 | DoSelectLotBaseTexturesUI | O LE4 |
| 0047C8D0 | 88 | 88 | DoAddNewWaterConstraint | O LE3 |
| 0047C930 | 88 | 88 | DoAddNewTerrainConstraint | O LE3 |
| 0047EFF0 | 83 | 83 | GetDefaultPath | C §8.2 |
| 0047B420 | 81 | 81 | LotFoundationIsPresent | C §9.5 |
| 0047BB40 | 81 | 81 | FUN_0047BB40 | C §9.4 |
| 00486440 | 77 | 77 | FUN_00486440 | O LE5 |
| 00486490 | 77 | 77 | FUN_00486490 | O LE5 |
| 004864E0 | 77 | 77 | FUN_004864E0 | O LE5 |
| 0047BA80 | 73 | 73 | DoMessage (main window) | C §7 |
| 0047B050 | 69 | 69 | FUN_0047B050 | O LE3 |
| 00478360 | 64 | 64 | FUN_00478360 (record constructor) | R §9.4; editor dependency proved, exclusive ownership unresolved |
| 0047A830 | 64 | 64 | QueryInterface | C §7 |
| 00484DD0 | 58 | 58 | MirrorObject | C §9.1 |
| 0047A8F0 | 53 | 53 | PixelUnitsToObjectUnits | C §9.6 |
| 0047AA00 | 45 | 45 | FUN_0047AA00 | O LE5 |
| 0047A890 | 42 | 42 | DoEditMouseUpL | C §11 |
| 0047B0A0 | 39 | 39 | DoAddNewNetworkNode | O LE3 |
| 0047B0D0 | 39 | 39 | DoAddEditFences | O LE3 |
| 0047A8C0 | 35 | 35 | FUN_0047A8C0 | O LE5 |
| 0047E870 | 19 | 19 | TriggerLotConfigurationRedraw | O LE5 |
| 00480590 | 18 | 18 | SetDataHasChanged | C §1 |
| 006049BF | 7 | 7 | FUN_006049BF | R LE5; depth-8 switch stub |
| 006049C6 | 7 | 7 | FUN_006049C6 | R LE5; depth-16 switch stub |
| 006049CD | 7 | 180 | library switchD_006049B8::caseD_0 | R LE5 |

## Supplemental direct workflow evidence (excluded from owner total)

| Entry / byte | Export B | Reason |
|---|---:|---|
| 00485390 | 180 | Real Save As entry/prefix, §8.2; vtable proof. |
| 00485444 | 1 uncovered byte | First byte of MOV ECX,EBP crossing the false entry. |
| 0047AB70 | 43 | Persistence virtual target, §8.2. |
| 008AE960 / 008AE970 | 7 each | Folded window AddRef/Release entry addresses, §7; implementations not re-specified. |

No ABI proposals, implementation-derived expected outputs, native game runs,
shadow verdicts or complete-owner coverage are claimed.
