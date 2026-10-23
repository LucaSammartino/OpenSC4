// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent expected fixtures, written before the owner implementation.
use super::*;

#[derive(Default)]
struct FaultFs {
    inner: MemoryFs,
    fail_write: Option<VPath>,
    fail_remove: Option<VPath>,
    operations: Vec<String>,
}

impl FileSystem for FaultFs {
    fn entries(&mut self, p: &VPath) -> Result<Vec<Entry>> {
        self.inner.entries(p)
    }
    fn create_dir(&mut self, p: &VPath) -> Result<()> {
        self.inner.create_dir(p)
    }
    fn remove_dir(&mut self, p: &VPath) -> Result<()> {
        self.inner.remove_dir(p)
    }
    fn read(&mut self, p: &VPath) -> Result<Option<Vec<u8>>> {
        self.inner.read(p)
    }
    fn write(&mut self, p: &VPath, bytes: &[u8]) -> Result<()> {
        self.operations.push(format!("write:{}", p.text()));
        if self.fail_write.as_ref() == Some(p) {
            self.inner.write(p, b"partial")?;
            return Err(Error::Io(5));
        }
        self.inner.write(p, bytes)
    }
    fn remove_file(&mut self, p: &VPath) -> Result<()> {
        self.operations.push(format!("remove:{}", p.text()));
        if self.fail_remove.as_ref() == Some(p) {
            return Err(Error::Io(5));
        }
        self.inner.remove_file(p)
    }
}

struct Images {
    depth: ImageDepth,
    calls: Vec<&'static str>,
    targets: Vec<CaptureTarget>,
}

impl ImagePrimitives for Images {
    type Image = Vec<u8>;
    fn capture(&mut self, target: CaptureTarget) -> Result<Self::Image> {
        self.calls.push("capture");
        self.targets.push(target);
        Ok(vec![0, 10, 20, 30])
    }
    fn depth(&self, _: &Self::Image) -> ImageDepth {
        self.depth
    }
    fn force_opaque(&mut self, _: &mut Self::Image) -> Result<()> {
        self.calls.push("opaque");
        Ok(())
    }
    fn encode_png(&mut self, _: &Self::Image) -> Result<Vec<u8>> {
        self.calls.push("png");
        Ok(b"PNG fixture".to_vec())
    }
}

fn ready_taker(modifiers: Modifiers) -> SnapshotTaker {
    let size = Size { width: 100, height: 80 };
    let mut t = SnapshotTaker::new(vec![size], size).unwrap();
    t.key(Key::Enter, modifiers, true).unwrap();
    for _ in 0..4 {
        t.deliver_capture().unwrap();
    }
    t
}

#[test]
fn import_copy_move_and_same_folder_retains_original_name() {
    let mut fs = FaultFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    fs.create_dir(&path("Incoming")).unwrap();
    let source = path("Incoming/source.jpg");
    fs.write(&source, b"source image").unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    let expected = path("Albums/Big Town--2/Big Town-Jan 17, 2000.png");
    fs.operations.clear();
    assert_eq!(
        album
            .add_new_graphic_for_current_city(&mut fs, &source, b"import", &city(), true)
            .unwrap(),
        expected
    );
    assert_eq!(fs.read(&source).unwrap(), None);
    assert_eq!(fs.read(&expected).unwrap(), Some(b"source image".to_vec()));
    assert_eq!(album.description(), b"import");
    assert_eq!(album.current_picture(), Some("Big Town-Jan 17, 2000.png"));
    let copy = fs
        .operations
        .iter()
        .position(|s| s == "write:Albums/Big Town--2/Big Town-Jan 17, 2000.png")
        .unwrap();
    let remove = fs
        .operations
        .iter()
        .position(|s| s == "remove:Incoming/source.jpg")
        .unwrap();
    assert!(copy < remove);
    // Same-directory imports must keep their stem and must not remove the source.
    let inside = path("Albums/Big Town--2/custom-name.jpeg");
    fs.write(&inside, b"keep name").unwrap();
    assert_eq!(
        album
            .add_new_graphic_for_current_city(&mut fs, &inside, b"", &city(), true)
            .unwrap(),
        inside
    );
    assert_eq!(fs.read(&inside).unwrap(), Some(b"keep name".to_vec()));
    assert_eq!(album.current_picture(), Some("custom-name.jpeg"));
}

#[test]
fn failed_copy_does_not_remove_source_and_failed_save_does_not_navigate() {
    let mut fs = FaultFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    fs.create_dir(&path("Incoming")).unwrap();
    let source = path("Incoming/source.png");
    fs.write(&source, b"original").unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    let destination = path("Albums/Big Town--2/Big Town-Jan 17, 2000.png");
    fs.fail_write = Some(destination.clone());
    assert_eq!(
        album.add_new_graphic_for_current_city(&mut fs, &source, b"", &city(), true),
        Err(Error::Io(5))
    );
    assert_eq!(fs.read(&source).unwrap(), Some(b"original".to_vec()));
    fs.fail_write = None;
    fs.remove_file(&destination).unwrap();
    for file in ["A.png", "B.png"] {
        fs.write(&path(&format!("Albums/Big Town--2/{file}")), b"img").unwrap();
    }
    album.refresh_pictures(&mut fs).unwrap();
    album.set_current_file(&mut fs, "A.png").unwrap();
    album.edit_description(b"unsaved".to_vec());
    fs.fail_write = Some(path("Albums/Big Town--2/A.txt"));
    assert_eq!(album.next(&mut fs), Err(Error::Io(5)));
    assert_eq!(album.current_picture(), Some("A.png"));
    assert_eq!(album.description(), b"unsaved");
    assert!(album.description_dirty());
}

#[test]
fn capture_pipeline_depth_metadata_pause_and_restore() {
    for depth in [ImageDepth::Bits16, ImageDepth::Bits32, ImageDepth::Other(24)] {
        let mut fs = MemoryFs::default();
        fs.create_dir(&path("Albums")).unwrap();
        let mut album = SnapshotAlbum::new(path("Albums"));
        let mut ui = MemoryUi::default();
        let mut images = Images {
            depth,
            calls: vec![],
            targets: vec![],
        };
        let mut taker = ready_taker(Modifiers {
            ctrl: true,
            ..Modifiers::default()
        });
        let file = taker
            .capture_into_album(&mut fs, &mut images, &mut ui, &mut album, &city())
            .unwrap();
        assert_eq!(images.targets, vec![CaptureTarget::WholeView]);
        let expected_calls = if depth == ImageDepth::Other(24) {
            vec!["capture", "png"]
        } else {
            vec!["capture", "opaque", "png"]
        };
        assert_eq!(images.calls, expected_calls);
        assert_eq!(fs.read(&file).unwrap(), Some(b"PNG fixture".to_vec()));
        assert_eq!(album.information().unwrap().city(), b"Big Town");
        assert_eq!(album.count_text().as_deref(), Some("(1/1)"));
        assert!(ui.paused && ui.album_visible && !ui.frame_visible);
        assert!(matches!(
            taker.capture_into_album(&mut fs, &mut images, &mut ui, &mut album, &city()),
            Err(Error::InvalidState)
        ));
        taker.complete_modal(&mut fs, &mut ui, &mut album).unwrap();
        assert!(!ui.paused && !ui.album_visible && ui.frame_visible);
    }
}

#[test]
fn failed_snapshot_write_removes_partial_png_and_reports_cleanup_error() {
    for cleanup_fails in [false, true] {
        let mut fs = FaultFs::default();
        fs.create_dir(&path("Albums")).unwrap();
        let expected = path("Albums/Big Town--2/Big Town-Jan 17, 2000.png");
        fs.fail_write = Some(expected.clone());
        if cleanup_fails {
            fs.fail_remove = Some(expected.clone());
        }
        let mut images = Images {
            depth: ImageDepth::Bits32,
            calls: vec![],
            targets: vec![],
        };
        let mut ui = MemoryUi::default();
        let mut album = SnapshotAlbum::new(path("Albums"));
        let mut taker = ready_taker(Modifiers::default());
        assert_eq!(
            taker.capture_into_album(&mut fs, &mut images, &mut ui, &mut album, &city()),
            Err(Error::Io(5))
        );
        assert!(fs
            .operations
            .iter()
            .any(|s| s == "remove:Albums/Big Town--2/Big Town-Jan 17, 2000.png"));
        assert_eq!(fs.read(&expected).unwrap().is_some(), cleanup_fails);
        assert!(!ui.paused && !ui.album_visible);
        assert!(taker.closed());
    }
}

#[test]
fn modal_restores_previous_pause_and_shutdown_saves_then_clears() {
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    fs.write(&path("Albums/Big Town--2/a.gif"), b"image").unwrap();
    album.refresh_pictures(&mut fs).unwrap();
    album.set_initial_file(&mut fs, None).unwrap();
    let mut ui = MemoryUi {
        paused: true,
        ..MemoryUi::default()
    };
    album.begin_modal(&mut ui).unwrap();
    assert_eq!(album.begin_modal(&mut ui), Err(Error::InvalidState));
    album.edit_description(b"persisted".to_vec());
    album.action(&mut fs, &mut ui, AlbumAction::Zoom(Zoom::Out)).unwrap();
    album.on_shutdown(&mut fs, &mut ui).unwrap();
    assert_eq!(
        fs.read(&path("Albums/Big Town--2/a.txt")).unwrap(),
        Some(b"persisted".to_vec())
    );
    assert!(ui.paused && !ui.album_visible);
    assert_eq!(ui.zoom_requests, vec![Zoom::Out]);
    assert!(album.albums().is_empty() && album.pictures().is_empty());
    assert!(album.information().is_none() && album.description().is_empty());
}

#[test]
fn clock_and_date_are_primitives_and_caption_is_structured() {
    struct Clock;
    impl SnapshotClock for Clock {
        fn simulation_day(&mut self) -> i32 {
            -123
        }
    }
    struct Dates;
    impl DateText for Dates {
        fn format_day(&mut self, day: i32) -> String {
            assert_eq!(day, -123);
            "Localized date".into()
        }
    }
    let mut info = city().info;
    info.sample_time(&mut Clock, &mut Dates);
    assert_eq!(info.day, -123);
    assert_eq!(info.date, "Localized date");
    let read = Information::decode(&info.encode().unwrap()).unwrap().unwrap();
    assert_eq!(
        read.caption_fields(),
        CaptionFields {
            city: b"Big Town".to_vec(),
            mayor: b"Mayor".to_vec(),
            date: b"Localized date".to_vec(),
            population: b"123".to_vec(),
            funds: b"-9876543210".to_vec()
        }
    );
}

#[test]
fn path_scope_and_unpublished_contracts_are_explicit() {
    for invalid in [
        "/absolute",
        "../escape",
        "Albums/../escape",
        "C:/Users",
        "a\\b",
        "a//b",
        "a/\0b",
    ] {
        assert_eq!(VPath::new(invalid), Err(Error::InvalidPath));
    }
    let mut bad_info = city().info;
    bad_info.mayor = "one\ntwo".into();
    assert_eq!(bad_info.encode(), Err(Error::Gap(Gap::EmbeddedLineFeed)));
    let marked_short: Vec<_> = b"SanityCheck\nx\n".iter().map(|b| b ^ 0x7b).collect();
    assert_eq!(
        Information::decode(&marked_short),
        Err(Error::Gap(Gap::MalformedInformation))
    );
    let album = SnapshotAlbum::new(path("Albums"));
    assert_eq!(album.new_region_picture_path(), Err(Error::Gap(Gap::RegionOnlyNaming)));
    assert_eq!(album.album_directory_index("not listed"), -1);
    assert!(matches!(
        SnapshotTaker::new(vec![Size { width: 200, height: 80 }], Size { width: 100, height: 80 }),
        Err(Error::Gap(Gap::OversizedFrame))
    ));
}

fn path(s: &str) -> VPath {
    VPath::new(s).unwrap()
}
fn city() -> CityContext {
    CityContext {
        name: "Big Town".into(),
        number: 2,
        info: GraphicInfo {
            user: "Player".into(),
            city: "Big Town".into(),
            mayor: "Mayor".into(),
            day: 17,
            date: "Jan 17, 2000".into(),
            population: 123,
            funds: -9876543210,
            further: -4,
            view: ViewPosition {
                zoom: 2,
                rotation: 3,
                x: 400,
                y: 500,
            },
        },
    }
}

#[test]
fn prescribed_info_wire_bytes_and_read_fields() {
    // Explicit independent plaintext oracle: field order, signed 64-bit funds, final LF.
    let plain = b"SanityCheck\nPlayer\nBig Town\nMayor\n17\nJan 17, 2000\n123\n-9876543210\n-4\n2,3,400,500\n";
    let encoded = city().info.encode().unwrap();
    assert_eq!(encoded[0], 0x28);
    assert_eq!(encoded, plain.iter().map(|b| b ^ 0x7b).collect::<Vec<_>>());
    let info = Information::decode(&encoded).unwrap().unwrap();
    assert_eq!(info.city(), b"Big Town");
    assert_eq!(info.funds(), b"-9876543210");
    assert!(Information::decode(b"not an info file").unwrap().is_none());
}

#[test]
fn folder_creation_filter_sort_and_companion_lifecycle() {
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    for folder in ["Z--1", "Empty--1", "JpegOnly--1", "A--1"] {
        fs.create_dir(&path(&format!("Albums/{folder}"))).unwrap();
    }
    fs.write(&path("Albums/Z--1/album.ini"), b"").unwrap();
    fs.write(&path("Albums/A--1/x.png"), b"image").unwrap();
    fs.write(&path("Albums/JpegOnly--1/x.jpg"), b"image").unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.refresh_albums(&mut fs).unwrap();
    assert_eq!(album.albums(), &["A--1", "Z--1"]);
    album.go_to_city(&mut fs, &city(), true).unwrap();
    assert_eq!(album.albums(), &["A--1", "Big Town--2", "Z--1"]);
    assert_eq!(fs.read(&path("Albums/Big Town--2/album.ini")).unwrap(), Some(vec![]));
    assert!(album.current_picture().is_none());
    fs.write(&path("Albums/Big Town--2/X.png"), b"png").unwrap();
    fs.write(&path("Albums/Big Town--2/X.txt"), b"old").unwrap();
    album.refresh_pictures(&mut fs).unwrap();
    album.set_current_file(&mut fs, "X.png").unwrap();
    assert_eq!(album.count_text().as_deref(), Some("(1/1)"));
    assert_eq!(album.description(), b"old");
    album.edit_description(vec![]);
    album.save_description(&mut fs).unwrap();
    assert_eq!(fs.read(&path("Albums/Big Town--2/X.txt")).unwrap(), None);
}

#[test]
fn names_trim_at_character_boundary_and_do_not_invent_collision_suffixes() {
    let name = format!("{}é", "a".repeat(47));
    assert_eq!(
        album_directory_name(&name, 2).unwrap(),
        format!("{}--2", "a".repeat(47))
    );
    assert_eq!(album_directory_name("Big Town", 2).unwrap(), "Big Town--2");
    assert_eq!(album_directory_name("A/B", 2), Err(Error::Gap(Gap::FilenameSanitizing)));
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    let expected = path("Albums/Big Town--2/Big Town-Jan 17, 2000.png");
    assert_eq!(album.new_picture_path(&mut fs, &city()).unwrap(), expected);
    fs.write(&expected, b"occupied").unwrap();
    assert_eq!(
        album.new_picture_path(&mut fs, &city()),
        Err(Error::Gap(Gap::CollisionNaming))
    );
}

#[test]
fn browse_saves_before_loading_and_delete_removes_only_companions() {
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    for (name, contents) in [("B.jpg", "B"), ("A.png", "A"), ("A.info", "bad"), ("notes.dat", "keep")] {
        fs.write(&path(&format!("Albums/Big Town--2/{name}")), contents.as_bytes())
            .unwrap();
    }
    album.refresh_pictures(&mut fs).unwrap();
    album.set_current_file(&mut fs, "A.png").unwrap();
    album.edit_description(b"saved before next".to_vec());
    album.next(&mut fs).unwrap();
    assert_eq!(album.current_picture(), Some("B.jpg"));
    assert_eq!(
        fs.read(&path("Albums/Big Town--2/A.txt")).unwrap(),
        Some(b"saved before next".to_vec())
    );
    album.previous(&mut fs).unwrap();
    album.delete_current(&mut fs).unwrap();
    assert_eq!(album.current_picture(), Some("B.jpg"));
    for file in ["A.png", "A.info", "A.txt"] {
        assert_eq!(fs.read(&path(&format!("Albums/Big Town--2/{file}"))).unwrap(), None);
    }
    assert_eq!(
        fs.read(&path("Albums/Big Town--2/notes.dat")).unwrap(),
        Some(b"keep".to_vec())
    );
    album.delete_album(&mut fs, "Big Town--2").unwrap();
    assert!(fs.entries(&path("Albums/Big Town--2")).is_err());
}

#[test]
fn taker_size_digit_and_four_delivery_fixture() {
    let mut taker = SnapshotTaker::new(
        vec![Size { width: 40, height: 30 }, Size { width: 80, height: 60 }],
        Size {
            width: 100,
            height: 100,
        },
    )
    .unwrap();
    taker.go_to_size_index(99).unwrap();
    assert_eq!(taker.size_index(), 1);
    taker.key(Key::Space, Modifiers::default(), true).unwrap();
    assert_eq!(taker.size_index(), 0);
    taker.key(Key::Digit(5), Modifiers::default(), true).unwrap();
    assert_eq!(taker.percentage(), Some(60));
    assert_eq!(
        taker.key(Key::Enter, Modifiers::default(), true).unwrap(),
        KeyResult::PostCapture {
            message: CAPTURE_MESSAGE,
            counter: 0,
            close_afterwards: true,
            lua_event: CAPTURE_LUA_EVENT
        }
    );
    for counter in [1, 2, 3] {
        assert_eq!(
            taker.deliver_capture().unwrap(),
            CaptureDelivery::Repost {
                message: CAPTURE_MESSAGE,
                counter
            }
        );
    }
    assert_eq!(
        taker.deliver_capture().unwrap(),
        CaptureDelivery::Ready {
            target: CaptureTarget::Rectangle(Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 30
            }),
            close_afterwards: true
        }
    );
    taker.finish_modal();
    assert!(!taker.visible());
}

#[test]
fn modifiers_and_whole_view_boundary_are_discriminating() {
    let mut taker = SnapshotTaker::new(vec![Size { width: 100, height: 80 }], Size { width: 100, height: 80 }).unwrap();
    taker
        .key(
            Key::Enter,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            true,
        )
        .unwrap();
    for _ in 0..3 {
        taker.deliver_capture().unwrap();
    }
    assert_eq!(
        taker.deliver_capture().unwrap(),
        CaptureDelivery::Ready {
            target: CaptureTarget::WholeView,
            close_afterwards: false
        }
    );
    taker.finish_modal();
    assert!(taker.visible());
    assert_eq!(
        capture_target(
            Rect {
                x: -5,
                y: 5,
                width: 100,
                height: 80
            },
            Size { width: 100, height: 80 }
        ),
        CaptureTarget::WholeView
    );
    assert!(matches!(
        capture_target(
            Rect {
                x: 6,
                y: 0,
                width: 100,
                height: 80
            },
            Size { width: 100, height: 80 }
        ),
        CaptureTarget::Rectangle(_)
    ));
}

#[test]
fn frame_reuse_drag_clamp_and_input_gate() {
    let sizes = vec![Size { width: 40, height: 30 }, Size { width: 80, height: 60 }];
    let view = Size {
        width: 100,
        height: 100,
    };
    let mut existing = None;
    let frame = SnapshotTaker::create_or_show(&mut existing, sizes.clone(), view, None).unwrap();
    assert_eq!(frame.percentage(), None);
    frame.set_position(50, 60).unwrap();
    frame.left_mouse_down(55, 65);
    frame.mouse_move(90, 95).unwrap();
    assert_eq!(
        frame.rectangle(),
        Rect {
            x: 60,
            y: 70,
            width: 40,
            height: 30
        }
    );
    frame.end_drag();
    frame.mouse_move(0, 0).unwrap();
    assert_eq!(frame.rectangle().x, 60);
    frame.go_to_size_index(1).unwrap();
    assert_eq!(
        frame.rectangle(),
        Rect {
            x: 20,
            y: 40,
            width: 80,
            height: 60
        }
    );
    assert_eq!(
        frame.key(Key::Enter, Modifiers::default(), false).unwrap(),
        KeyResult::Ignored
    );
    let frame = SnapshotTaker::create_or_show(&mut existing, sizes.clone(), view, Some(-1)).unwrap();
    assert_eq!(
        frame.rectangle(),
        Rect {
            x: 20,
            y: 40,
            width: 40,
            height: 30
        }
    );
    frame.key(Key::Digit(9), Modifiers::default(), true).unwrap();
    assert_eq!(frame.percentage(), Some(100));
    assert_eq!(
        frame.key(Key::Escape, Modifiers::default(), true).unwrap(),
        KeyResult::Closed
    );
    assert!(frame.closed());
    let frame = SnapshotTaker::create_or_show(&mut existing, sizes, view, None).unwrap();
    assert!(frame.visible() && !frame.closed());
    assert_eq!(frame.percentage(), None);
    frame.right_mouse_down();
    assert!(frame.closed());
}

#[test]
fn selected_same_file_import_replaces_old_dirty_description() {
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    let source = path("Albums/Big Town--2/X.PNG");
    fs.write(&source, b"original").unwrap();
    album.refresh_pictures(&mut fs).unwrap();
    album.set_current_file(&mut fs, "X.PNG").unwrap();
    album.edit_description(b"old edit".to_vec());
    album
        .add_new_graphic_for_current_city(&mut fs, &source, b"new import", &city(), false)
        .unwrap();
    assert_eq!(album.description(), b"new import");
    assert_eq!(
        fs.read(&path("Albums/Big Town--2/X.txt")).unwrap(),
        Some(b"new import".to_vec())
    );
}

#[test]
fn ui_hides_before_post_and_capture_message_reposts_are_owned() {
    let size = Size { width: 100, height: 80 };
    let mut taker = SnapshotTaker::new(vec![size], size).unwrap();
    let mut ui = MemoryUi {
        frame_visible: true,
        ..MemoryUi::default()
    };
    taker
        .on_key(
            Key::Enter,
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
            true,
            &mut ui,
        )
        .unwrap();
    assert!(!ui.frame_visible);
    assert_eq!(
        ui.events,
        vec![
            UiEvent::FrameVisible(false),
            UiEvent::PostCapture {
                message: 0x1a5d_056d,
                counter: 0,
                close_afterwards: false
            },
            UiEvent::LuaEvent(0xaa8b_82a9),
        ]
    );
    for _ in 0..3 {
        taker.on_capture_message(&mut ui).unwrap();
    }
    assert_eq!(
        &ui.events[3..],
        &[
            UiEvent::PostCapture {
                message: 0x1a5d_056d,
                counter: 1,
                close_afterwards: false
            },
            UiEvent::PostCapture {
                message: 0x1a5d_056d,
                counter: 2,
                close_afterwards: false
            },
            UiEvent::PostCapture {
                message: 0x1a5d_056d,
                counter: 3,
                close_afterwards: false
            },
        ]
    );
    assert_eq!(
        taker.on_capture_message(&mut ui).unwrap(),
        CaptureDelivery::Ready {
            target: CaptureTarget::WholeView,
            close_afterwards: false,
        }
    );
    assert_eq!(ui.events.len(), 6);
}
