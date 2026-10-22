# Portable lot editor coverage audit

Inventory: **109 current bodies / 52,646 bytes**; published C44/20,550 B, P20/20,334 B, O40/11,335 B, R5/427 B.

Complete below means a focused portable algorithm within its stated domain, never a whole-owner claim. Partial entries retain dependency barriers. Unsupported entries have no implementation credit. Foreign entries are shared/dependency bodies and receive no exclusive editor credit. Supplemental Save As prefix is accounted with its corrected workflow, not a new body.

Effects are pending primitive UI/render/title requirements. This source does not supply native window lifecycle, title/name resolution, 3D preparation or rendering. Borrowed widget controls do not manufacture COM references. Identity and cursor acquired references have explicit transfer/release ordering.

Read/Open are explicitly incomplete: successful foreign read installs the data and stops at reconciliation; public Open stops before missing PostInit. SetLotSize and its UI wrapper are unsupported. No host computes save, compatibility, duplication, selection, geometry, reconciliation or fallback hit outcomes.

| Entry | Bytes | Published | Portable | Method and exact scope |
|---|---:|---|---|---|
| 00480D10 | 6818 | C | partial | DoSetLotCompatibilitiesUI: compatibility_dialog; complete ordered grid kernel; unknown fractional initial conversion/constructor defaults LE6; native grid fault surfaced explicitly |
| 00487B30 | 5971 | P | partial | DoLotConfigurationObjectMenu: object_action/network_type; supported direct mutations; selector bounds, help, edge table, types3/5 remain blocked |
| 004871B0 | 2425 | C | partial | DoEditAreaMenu: edit_action/edit_area_menu/execute; all 25 dispatch rows; dependent commands explicitly unsupported |
| 0047CF90 | 1652 | P | unsupported | DrawLotConfigurationObject: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004827D0 | 1514 | P | partial | DoSelectBuildingFamiliesUI: selector_rows/selector_dialog kernel only; names/fallback/catalogue adapter/collation remain LE4/LE5 |
| 004832E0 | 1503 | P | partial | DoSelectPropFamiliesUI: selector_rows/selector_dialog kernel only; option gates/preselection; name/collation remain LE4/LE5 |
| 00483DC0 | 1373 | P | partial | DoHitTest: hit_candidates priority kernel on supplied integer rectangles; conversion, popup labels and fallback unsupported |
| 00482DC0 | 1304 | P | partial | DoSelectFloraFamiliesUI: selector_rows/selector_dialog kernel only; literal individual Flora label; family-name fallback remains LE4/LE5 |
| 00484850 | 1236 | P | unsupported | AddNewLotConfigurationObject: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047E060 | 1193 | P | unsupported | DrawCurrentLotConfiguration: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00480910 | 1021 | O | unsupported | DoSetRetainingWallUI: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047CC30 | 850 | P | partial | DoSelectTexturesUI: selector rows/dialog kernel only; raw image inputs; image catalogue/decoder remain foreign/unsupported |
| 00485D60 | 799 | O | unsupported | DoSetFoundationUI: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047FC50 | 726 | O | unsupported | GetAllLotBaseTextureTypes: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00485710 | 717 | P | partial | GZOnKeyDown: key_down capture guard and defined dispatch; unknown selected cycling/commands explicit errors |
| 00485445 | 700 | C | partial | SaveAs (false entry): save_as; complete ordinary transfer/rollback sequencing; provider path split/text details LE6; allocation fault explicit |
| 0047F200 | 640 | C | partial | GetIdentityInfoForLotConfigurationExemplarPath: identity_from_path; provider enumeration/ref cleanup; ASCII backslash paths only, splitter details LE6 |
| 0047BBA0 | 638 | O | unsupported | Prepare3DView: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047DDF0 | 618 | C | partial | DoSet3DViewSlopeUI: slope_dialog; exact lot-limit-before-cap arithmetic; foreign parser, LE5 view and initial text format remain dependencies |
| 00486080 | 614 | O | unsupported | SetLotSize: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047C660 | 612 | O | unsupported | CycleSelectedObject: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00486930 | 610 | O | unsupported | DoLotFoundationMenu: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00485B00 | 605 | O | unsupported | DoOutsideAreaMenu: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00484E10 | 557 | C | complete | DuplicateObject: geometry::duplicate; sequential stores and final untested candidate |
| 0047AE20 | 547 | C | partial | LoadCursors: load_cursors primitive resource handles and replacement order; foreign cursor construction |
| 0047F480 | 541 | O | unsupported | GetLotConfigurationObjectName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047E650 | 531 | C | partial | DoSetNetworkNodePathUI: network_path_dialog; logical size and zero-filled growth; foreign hex parser and exact initial text format dependency |
| 0047F750 | 510 | O | unsupported | GetBuildingFamilyName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00484330 | 494 | C | complete | RemoveOutOfBoundsLotConfigurationObjects: geometry::cleanup; unordered-per-check, signed fences, no selection repair |
| 0047B890 | 488 | P | unsupported | PostInit: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00486F20 | 473 | O | unsupported | StaticInit: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00484530 | 462 | C | complete | AddLotFoundation: geometry::add_foundation; unknown creation word explicitly marked |
| 0047FA80 | 457 | O | unsupported | GetPropFamilyName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004805B0 | 447 | C | partial | DoFenceMenu: fence_menu row dispatch; placeholder wording LE6, help dependency LE5 |
| 0047B480 | 443 | C | complete | MoveObjectToWithinLotBounds: geometry::move_within; four tests and final x-only clamp |
| 00486770 | 434 | P | unsupported | DoEditMouseDownL: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00480170 | 411 | C | unsupported | DeleteLotConfigurationObject: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00480770 | 409 | C | partial | DoSetLotNameUI: name_dialog; empty name/title twice/missing control; unknown native name/title plumbing retained |
| 0047ABA0 | 406 | C | partial | DoDialogBoxGridSort: dialog_callback raw Sort event; no invented collation or host implementation |
| 00486BA0 | 398 | C | unsupported | DoSetLotSizeUI: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047E890 | 377 | C | complete | ToggleFence: geometry::toggle_fence; ordinary allocation domain, no dirty write |
| 0047DA80 | 375 | C | partial | cSC4WinLotConfigurationEditor: Editor::default known state only; no native vtables/window lifecycle or constructor completeness |
| 00485040 | 353 | P | partial | MoveObject (key): move_key established finite subset only; LE3 remaining numeric/movement semantics unsupported |
| 00483B20 | 342 | C | partial | CreateNewLotConfiguration: create_new exact published fields; unknown remaining constructor fields LE6, no native allocator fault emulation |
| 0047FF30 | 335 | O | unsupported | GetFoundationNameAndImplementationType: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047F0B0 | 327 | P | unsupported | SetTitle: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047C510 | 322 | C | unsupported | Shutdown: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00484700 | 322 | P | partial | ToggleFence: toggle_fence_pixels finite/in-range conversion subset; invalid/overflow unsupported |
| 004892B0 | 295 | P | unsupported | Init: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004859E0 | 286 | P | unsupported | DoEditMouseMove: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00483C80 | 281 | C | partial | ReadLotConfigurationExemplar: read failure/pointer replacement; positive read stops at LotConfigManager::fix adapter barrier |
| 0047CA60 | 262 | C | partial | DoDialogBoxScrollToLetter: dialog_callback letter kernel; prop-only grid inclusive interval/first byte; borrowed widget adapter |
| 0047B100 | 238 | C | complete | SetSelectedLotConfigurationObject: Editor::set_selection; opaque identity retained when absent |
| 004852A0 | 238 | C | partial | Save: save; finite verification, persistence and notification order; unknown prompt wording/options LE6 |
| 00480080 | 228 | O | unsupported | GetRetainingWallName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047D6B0 | 212 | P | partial | VerifyObjectsAreWithinLotBounds: verify_bounds finite subset; LE3 unordered/invalid numeric cases unsupported |
| 0047AD40 | 211 | O | unsupported | GZOnMouseUpL: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00486690 | 211 | O | unsupported | GZOnMouseMove: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004865D0 | 191 | C | partial | Close: close full ask/save decision tree; underlying Save/SaveAs dependency barriers retained |
| 0047B640 | 185 | P | partial | FUN_0047B640 (bounds predicate): verify_bounds finite predicate; LE3 unordered cases unsupported |
| 0047CB70 | 181 | O | unsupported | DoShowHelpUI: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047F950 | 174 | O | unsupported | GetFloraFamilyName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00487100 | 174 | O | unsupported | GZOnMouseDownL: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047A930 | 173 | C | complete | FUN_0047A930: drawing_style; byte channels and 0.5 output |
| 004895A0 | 169 | O | unsupported | GZOnMouseDownR: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004893E0 | 166 | O | unsupported | DoEditMouseDownR: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047B1F0 | 165 | C | partial | GetIdentityInfoForLotConfigurationExemplarKey: identity_from_key; query/path/ref order; foreign provider acquisition required |
| 00604920 | 169 | R | foreign | FUN_00604920 (shared image decoder): Shared image decoder/switch/record constructor; no exclusive implementation credit |
| 0047D610 | 146 | O | unsupported | MessageBoxA: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00486530 | 145 | C | partial | Open: open is blocked at PostInit; separately named open_configuration_stage is explicitly not Open |
| 0047DD60 | 136 | C | partial | DoMessage (dialog callback): dialog_callback categories/control mapping; sort semantics foreign |
| 0047B700 | 133 | O | unsupported | GoTo3DView: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047B390 | 130 | O | unsupported | FUN_0047B390: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047F6C0 | 130 | O | unsupported | GetBuildingName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00484D40 | 130 | C | partial | RotateObject: rotate finite arithmetic subset; nonfinite inputs explicitly unsupported |
| 0047FA00 | 127 | O | unsupported | GetPropName: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00483A20 | 126 | O | unsupported | DoAddNewProp: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004851B0 | 119 | P | partial | MoveObject (position): move_position established finite subset only; LE3 remaining numeric/movement semantics unsupported |
| 0047B310 | 114 | O | unsupported | FUN_0047B310: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00483920 | 114 | O | unsupported | DoAddNewBuilding: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004839A0 | 114 | O | unsupported | DoAddNewFlora: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00483AA0 | 114 | O | unsupported | DoAddNewLotBaseTexture: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047B2A0 | 102 | C | complete | FUN_0047B2A0: geometry::contained; unordered comparisons individually pass |
| 0047BAD0 | 101 | C | partial | GetIdentityInfoForLotConfigurationType: identity_from_type; exact published type/group/ID; foreign catalogue required |
| 004838C0 | 92 | O | unsupported | DoSelectLotBaseTexturesUI: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047C8D0 | 88 | O | unsupported | DoAddNewWaterConstraint: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047C930 | 88 | O | unsupported | DoAddNewTerrainConstraint: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047EFF0 | 83 | C | partial | GetDefaultPath: default_path; backslash Exemplar/Lot Templates append; exact path utility edge cases LE6 |
| 0047B420 | 81 | C | complete | LotFoundationIsPresent: geometry::foundation; first type-3 ID, including zero |
| 0047BB40 | 81 | C | complete | FUN_0047BB40: geometry::contained; unsigned dimensions times 16 |
| 00486440 | 77 | O | unsupported | FUN_00486440: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00486490 | 77 | O | unsupported | FUN_00486490: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 004864E0 | 77 | O | unsupported | FUN_004864E0: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047BA80 | 73 | C | unsupported | DoMessage (main window): Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047B050 | 69 | O | unsupported | FUN_0047B050: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00478360 | 64 | R | foreign | FUN_00478360 (record constructor): Shared image decoder/switch/record constructor; no exclusive implementation credit |
| 0047A830 | 64 | C | unsupported | QueryInterface: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00484DD0 | 58 | C | complete | MirrorObject: geometry::mirror; type 2 only |
| 0047A8F0 | 53 | C | complete | PixelUnitsToObjectUnits: geometry::pixel_units; two binary32 stores per coordinate |
| 0047AA00 | 45 | O | unsupported | FUN_0047AA00: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 0047A890 | 42 | C | complete | DoEditMouseUpL: Editor::mouse_up; drag gates capture release |
| 0047B0A0 | 39 | C | complete | DoAddNewNetworkNode: Editor::placement_mode(4); no record creation |
| 0047B0D0 | 39 | C | complete | DoAddEditFences: Editor::placement_mode(8); same-mode no-op |
| 0047A8C0 | 35 | C | complete | FUN_0047A8C0: Editor::cycle_condition; equality reset at 7 |
| 0047E870 | 19 | O | unsupported | TriggerLotConfigurationRedraw: Explicit LE3/LE4/LE5 or owner/interface dependency; no generic window stub |
| 00480590 | 18 | C | partial | SetDataHasChanged: set_dirty stores arbitrary byte and emits pending title requirement; full title LE5 |
| 006049BF | 7 | R | foreign | FUN_006049BF: Shared image decoder/switch/record constructor; no exclusive implementation credit |
| 006049C6 | 7 | R | foreign | FUN_006049C6: Shared image decoder/switch/record constructor; no exclusive implementation credit |
| 006049CD | 180 | R | foreign | library switchD_006049B8::caseD_0: Shared image decoder/switch/record constructor; no exclusive implementation credit |

Coverage classifications (body bytes are attribution, not implemented byte equivalents): complete 16/3274 B; partial 35/30431 B; unsupported 53/18514 B; foreign 5/427 B.

No game run/shadow or drop-in-window completion is claimed. Physical type-7 three-allocation/four-store behavior is unsupported, as are SetLotSize arithmetic, foundation/wall dialogs, constraint branches, detail marker logic, edge table, selector name fallback, remaining lifecycle/input/capture/hit fallback/render/title/name/3D semantics.
