# Snapshot owners

Portable Rust owners from `docs/specs/snapshots.md`; no C ABI or plugin binding.
`album` owns the store, `window` composes its controls, `taker` owns capture
scheduling, and `album_picture` owns cWinBMPViewGVExtra size/input behavior.
Foreign IO, image codecs, language/date services and simulator pause are traits.
`Unsupported` (also exported as `Gap` for existing callers) names the boundary;
`Unsupported::question` cites `docs/specs/questions/snapshots.md`. All SN1-SN9
remain open. Supplying a number or ninth field does not assign it a meaning.

## Closure table

The spec has five worked cases in section 6. Other tests below are direct
fixtures of the stated rules, not additional native behavior evidence.

| Spec section | Code item | Spec fixture / test | Open Unsupported boundary |
|---|---|---|---|
| 0 Summary | `SnapshotAlbum`, `SnapshotTaker` | `capture_pipeline_depth_metadata_pause_and_restore` | Native UI binding: `NativeToolkit` (SN7) |
| 1.1 Folders | `album_directory_name`, `create_album`, `refresh_albums` | `folder_creation_filter_sort_and_companion_lifecycle`: Big Town, 2 -> Big Town--2/album.ini (worked case 1) | `AlbumNumberMeaning` (SN1), `FilenameSanitizing` (SN3), `AlbumOrdering` (SN4); portable UTF-8 prefix/byte sort only |
| 1.2 Picture files | `VPath::companion`, `new_picture_path`, `save_description` | `folder_creation_filter_sort_and_companion_lifecycle`: empty X.png description removes X.txt (worked case 3) | `CollisionNaming`, `RegionOnlyNaming` (SN3), `TextEncoding` (SN5) |
| 1.3 Information | `GraphicInfo::encode`, `Information::decode`, `caption_fields`, `caption` | `prescribed_info_wire_bytes_and_read_fields`: first byte 0x28, ten LF fields (worked case 2), caption returns Unsupported | `NinthInformationField` (SN1), `CaptionLayout` (SN2), malformed/encoding rules (SN5) |
| 2 Taker | `SnapshotTaker` | `taker_size_digit_and_four_delivery_fixture`: last Space wraps, 5 -> 60, delivery 4 captures (worked cases 4-5) | Percentage meaning SN1; frame defaults/geometry SN6; toolkit SN7; pixel masks SN8 |
| 3.1 Creation | `AlbumWindow::bind`, `on_init`, `show_current_city`, `on_shutdown` | `creation_binds_existing_controls_and_modal_shutdown_saves_then_clears` | Native window/control script resolution and destruction SN7; explicit borrowed controls |
| 3.2 Browsing | `AlbumWindow::action`, `set_current_file`, store browse/delete methods | `browse_saves_widget_text_and_deletes_companions_then_shows_no_image`, `zoom_routes_to_picture_size_and_close_restores_already_paused_game` | Initial/order/endpoints SN4; raw button/message IDs SN7; failure policy SN9 |
| 3.3 Albums | `AlbumWindow::{go_to_album,add_new_graphic,delete_album}`, store import methods | `import_move_select_and_delete_refresh_the_bound_controls`, `import_copy_move_and_same_folder_retains_original_name` | Number meaning SN1; naming SN3; selection SN4; copy/failure policy SN9 |
| 4 Quirks | `GraphicInfo::encode`, `album_directory_name` | `prescribed_info_wire_bytes_and_read_fields`, `names_trim_at_character_boundary_and_do_not_invent_collision_suffixes` | Native encoding/character boundaries SN3/SN5 |
| 5 Open questions | `Unsupported::question`, `Information::caption` | `prescribed_info_wire_bytes_and_read_fields` asserts SN2 boundary | SN1-SN9 open; no new questions |
| 6 Conformance | `tests` | Five worked cases covered by the tests cited above | No game/native-exact claim |
| picture-view 8 Album picture | `AlbumPicture` | `constructor_init_does_not_resize_or_apply_fit`, `size_switches_use_base_tree_geometry_and_notify_parent`, `double_click_under_500_toggles_and_reset_prevents_a_third_toggle` | Base cWinBMPView drawing/loading is a foreign primitive; initial fit byte supplied |

`OWNER_INVENTORY.md` records the prior portable/native surface distinction.
The bytewise lists, UTF-8 names, checked indices and clamped endpoints in the
existing workflow are provisional portable policies, not resolutions of SN3-SN5.

`AlbumWindow` takes controls already resolved by a UI-script adapter. It uses
`ui_windows::base::Windows` for identity/tree/visibility/geometry and the actual
`widget_controls::{list_box::ListBox,text_edit::TextEdit}` implementations.
The adapter executes TextEdit's pending repairs and reclaims the controls with
`into_controls` after album shutdown; no borrowed window is destroyed here.
Caption inputs are exposed without inventing SN2's localized caption text.
