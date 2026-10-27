# cSC4WinSnapshotAlbum

`../album.rs` owns the album store. `../window.rs` composes that store with the
existing base window, list box, text edit and `AlbumPicture` owners. File IO,
image loading, date/language input and simulator pause are foreign traits.
The UI adapter resolves the script controls, executes the text editor's pending
repairs, then reclaims/shuts down its widgets after album shutdown.

| snapshots.md section | Code | Spec fixture / test | Explicit open boundary |
|---|---|---|---|
| 1.1 Folders | `album_directory_name`, `create_album`, `refresh_albums` | `folder_creation_filter_sort_and_companion_lifecycle`: Big Town--2/album.ini | SN1 number meaning, SN3 sanitizer/encoding, SN4 native ordering |
| 1.2 Pictures | `VPath::companion`, `save_description`, `new_picture_path` | `folder_creation_filter_sort_and_companion_lifecycle`: empty X.txt removed | SN3 collision/region names, SN5 encoding |
| 1.3 Information | `GraphicInfo::encode`, `Information::decode`, `caption` | `prescribed_info_wire_bytes_and_read_fields`: S XOR 0x7B = 0x28; caption Unsupported | SN1 ninth field, SN2 caption layout, SN5 malformed text |
| 3.1 Creation | `AlbumWindow::{bind,on_init,show_current_city,on_shutdown}` | `creation_binds_existing_controls_and_modal_shutdown_saves_then_clears` | SN7 native UI script IDs, lifetime and destructor Unsupported |
| 3.2 Browsing | `AlbumWindow::{action,refresh_graphics,set_initial_file,set_current_file}`, `SnapshotAlbum` browse/delete | `browse_saves_widget_text_and_deletes_companions_then_shows_no_image`; `failed_description_save_preserves_edit_selection_and_modal_pause` | SN4 order/default/endpoints use documented portable policies; SN7 raw routing Unsupported; SN9 failure/rollback |
| 3.3 Albums | `go_to_album`, `add_new_graphic`, `delete_album` | `import_move_select_and_delete_refresh_the_bound_controls`; existing copy/same-folder fixture | SN1 number meaning, SN3 naming, SN4 selection, SN9 failure/overwrite/nested removal |
| 4 Quirks | `GraphicInfo::encode`, `album_directory_name` | `names_trim_at_character_boundary_and_do_not_invent_collision_suffixes` | SN3/SN5 native text encoding; portable UTF-8 boundaries only |
| 5 Open questions | `Unsupported::question` | `prescribed_info_wire_bytes_and_read_fields`, raw routing test | SN1-SN9 remain open in `docs/specs/questions/snapshots.md` |
| 6 Conformance | Store/window tests above; taker fixture in parent table | Worked cases 1-3 above; 4-5 belong to taker | No native owner replacement, C ABI or game behavior equivalence claim |

See the parent README for the full spec-section closure table and portable
policy limits. Unsupported cases use `Error::Gap(Unsupported)`; `Gap` is a
compatibility type alias. Caption inputs do not silently resolve SN2.
