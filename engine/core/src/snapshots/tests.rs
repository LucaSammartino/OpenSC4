// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent expected fixtures, written before the owner implementation.
use super::*;

fn path(s: &str) -> VPath { VPath::new(s).unwrap() }
fn city() -> CityContext {
    CityContext { name: "Big Town".into(), number: 2, info: GraphicInfo {
        user: "Player".into(), city: "Big Town".into(), mayor: "Mayor".into(),
        day: 17, date: "Jan 17, 2000".into(), population: 123, funds: -9876543210,
        further: -4, view: ViewPosition { zoom: 2, rotation: 3, x: 400, y: 500 },
    }}
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
    assert_eq!(album_directory_name(&name, 2).unwrap(), format!("{}--2", "a".repeat(47)));
    assert_eq!(album_directory_name("Big Town", 2).unwrap(), "Big Town--2");
    assert_eq!(album_directory_name("A/B", 2), Err(Error::Gap(Gap::FilenameSanitizing)));
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    let expected = path("Albums/Big Town--2/Big Town-Jan 17, 2000.png");
    assert_eq!(album.new_picture_path(&mut fs, &city()).unwrap(), expected);
    fs.write(&expected, b"occupied").unwrap();
    assert_eq!(album.new_picture_path(&mut fs, &city()), Err(Error::Gap(Gap::CollisionNaming)));
}

#[test]
fn browse_saves_before_loading_and_delete_removes_only_companions() {
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let mut album = SnapshotAlbum::new(path("Albums"));
    album.go_to_city(&mut fs, &city(), true).unwrap();
    for (name, contents) in [("B.jpg", "B"), ("A.png", "A"), ("A.info", "bad"), ("notes.dat", "keep")] {
        fs.write(&path(&format!("Albums/Big Town--2/{name}")), contents.as_bytes()).unwrap();
    }
    album.refresh_pictures(&mut fs).unwrap();
    album.set_current_file(&mut fs, "A.png").unwrap();
    album.edit_description(b"saved before next".to_vec());
    album.next(&mut fs).unwrap();
    assert_eq!(album.current_picture(), Some("B.jpg"));
    assert_eq!(fs.read(&path("Albums/Big Town--2/A.txt")).unwrap(), Some(b"saved before next".to_vec()));
    album.previous(&mut fs).unwrap();
    album.delete_current(&mut fs).unwrap();
    assert_eq!(album.current_picture(), Some("B.jpg"));
    for file in ["A.png", "A.info", "A.txt"] {
        assert_eq!(fs.read(&path(&format!("Albums/Big Town--2/{file}"))).unwrap(), None);
    }
    assert_eq!(fs.read(&path("Albums/Big Town--2/notes.dat")).unwrap(), Some(b"keep".to_vec()));
    album.delete_album(&mut fs, "Big Town--2").unwrap();
    assert!(fs.entries(&path("Albums/Big Town--2")).is_err());
}

#[test]
fn taker_size_digit_and_four_delivery_fixture() {
    let mut taker = SnapshotTaker::new(vec![Size { width: 40, height: 30 }, Size { width: 80, height: 60 }],
        Size { width: 100, height: 100 }).unwrap();
    taker.go_to_size_index(99).unwrap();
    assert_eq!(taker.size_index(), 1);
    taker.key(Key::Space, Modifiers::default(), true).unwrap();
    assert_eq!(taker.size_index(), 0);
    taker.key(Key::Digit(5), Modifiers::default(), true).unwrap();
    assert_eq!(taker.percentage(), 60);
    assert_eq!(taker.key(Key::Enter, Modifiers::default(), true).unwrap(), KeyResult::PostCapture {
        message: CAPTURE_MESSAGE, counter: 0, close_afterwards: true, lua_event: CAPTURE_LUA_EVENT });
    for counter in [1, 2, 3] {
        assert_eq!(taker.deliver_capture().unwrap(), CaptureDelivery::Repost { message: CAPTURE_MESSAGE, counter });
    }
    assert_eq!(taker.deliver_capture().unwrap(), CaptureDelivery::Ready {
        target: CaptureTarget::Rectangle(Rect { x: 0, y: 0, width: 40, height: 30 }), close_afterwards: true });
    taker.finish_modal();
    assert!(!taker.visible());
}

#[test]
fn modifiers_and_whole_view_boundary_are_discriminating() {
    let mut taker = SnapshotTaker::new(vec![Size { width: 100, height: 80 }], Size { width: 100, height: 80 }).unwrap();
    taker.key(Key::Enter, Modifiers { shift: true, ..Modifiers::default() }, true).unwrap();
    for _ in 0..3 { taker.deliver_capture().unwrap(); }
    assert_eq!(taker.deliver_capture().unwrap(), CaptureDelivery::Ready { target: CaptureTarget::WholeView, close_afterwards: false });
    taker.finish_modal();
    assert!(taker.visible());
    assert_eq!(capture_target(Rect { x: -5, y: 5, width: 100, height: 80 }, Size { width: 100, height: 80 }), CaptureTarget::WholeView);
    assert!(matches!(capture_target(Rect { x: 6, y: 0, width: 100, height: 80 }, Size { width: 100, height: 80 }), CaptureTarget::Rectangle(_)));
}
