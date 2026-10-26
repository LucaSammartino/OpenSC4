// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::snapshots::album_picture::ViewOp;
use crate::snapshots::{Entry, GraphicInfo, MemoryFs, MemoryUi, ViewPosition};
use crate::widget_controls::text_edit::conversion::CodePages;
use crate::widget_controls::text_edit::encoding::Encoding;
use crate::widget_controls::{Delivery, Draw, Handle, Message, Rect, Resource};

#[derive(Default)]
struct Services {
    ui: MemoryUi,
    images: Vec<Option<VPath>>,
    fields: Vec<Option<CaptionFields>>,
    picture_ops: Vec<ViewOp>,
    fail_load: bool,
}

impl Host for Services {}
impl AlbumHost for Services {
    fn load_picture(&mut self, _: Window, path: Option<&VPath>) -> Result<()> {
        if self.fail_load {
            return Err(Error::Image);
        }
        self.images.push(path.cloned());
        Ok(())
    }
    fn information_fields(&mut self, _: Window, fields: Option<&CaptionFields>) {
        self.fields.push(fields.cloned());
    }
}
impl PictureHost for Services {
    fn init_view(&mut self, _: Window) -> Result<()> {
        Ok(())
    }
    fn view_op(&mut self, _: Window, op: ViewOp) {
        self.picture_ops.push(op);
    }
    fn view_left_click(&mut self, _: Window, _: [i32; 2]) {}
    fn view_message(&mut self, _: Window, _: [u32; 4]) -> bool {
        false
    }
    fn scroll_lock(&mut self) -> bool {
        false
    }
    fn elapsed_ms(&mut self, _: Window) -> Option<u32> {
        None
    }
    fn restart_timer(&mut self, _: Window) {}
    fn reset_timer(&mut self, _: Window) {}
    fn send_parent(&mut self, _: Window, _: u32, _: [u32; 3]) {}
}
impl UiPrimitives for Services {
    fn paused(&self) -> bool {
        self.ui.paused()
    }
    fn set_paused(&mut self, paused: bool) {
        self.ui.set_paused(paused);
    }
    fn set_album_visible(&mut self, visible: bool) {
        self.ui.set_album_visible(visible);
    }
    fn set_frame_visible(&mut self, visible: bool) {
        self.ui.set_frame_visible(visible);
    }
    fn post_capture_message(&mut self, message: u32, counter: u8, close: bool) {
        self.ui.post_capture_message(message, counter, close);
    }
    fn raise_lua_event(&mut self, event: u32) {
        self.ui.raise_lua_event(event);
    }
    fn zoom_picture(&mut self, direction: Zoom) {
        self.ui.zoom_picture(direction);
    }
}
impl CodePages for Services {
    fn system_code_page(&mut self) -> i32 {
        0
    }
    fn multi_byte_to_wide(&mut self, _: i32, _: u32, _: &[u8], _: &mut [u16]) -> usize {
        unreachable!("single-byte fixture does not convert")
    }
    fn wide_to_multi_byte(&mut self, _: i32, _: u32, _: &[u16], _: &mut [u8], _: u8) -> usize {
        unreachable!("single-byte fixture does not convert")
    }
    fn composition_tail(&mut self) -> u16 {
        0
    }
}
impl TextForeign for Services {
    fn correction(&mut self, _: u32) {}
    fn invalidate(&mut self, _: Handle) {}
    fn notify(&mut self, _: Handle, _: u32) {}
}
impl Foreign for Services {
    fn add_ref(&mut self, _: Handle) {}
    fn release(&mut self, _: Handle) {}
    fn base_init(&mut self, _: Handle) -> bool {
        true
    }
    fn base_shutdown(&mut self, _: Handle) {}
    fn base_message(&mut self, _: Handle, _: Message) -> bool {
        false
    }
    fn base_query(&mut self, _: Handle, _: u32, _: &mut Option<Handle>) -> bool {
        false
    }
    fn invalidate(&mut self, _: Handle) {}
    fn show(&mut self, _: Handle, _: bool) {}
    fn capture(&mut self, _: Handle, _: bool) {}
    fn convert_payload(&mut self, _: Handle, value: i32) -> i32 {
        value
    }
    fn deliver(&mut self, _: Handle, _: Delivery, _: Message) -> bool {
        true
    }
    fn drop_complete(&mut self, _: Handle, _: u32, _: i32) {}
    fn default_interface(&mut self, _: u32) -> Option<Handle> {
        None
    }
    fn default_color(&mut self, _: u32) -> Option<u32> {
        None
    }
    fn create(&mut self, _: Resource) -> Option<Handle> {
        None
    }
    fn font_height(&mut self, _: Handle) -> i32 {
        10
    }
    fn buffer_height(&mut self, _: Handle) -> i32 {
        1
    }
    fn bar_value(&mut self, _: Handle) -> i32 {
        0
    }
    fn bar_range(&mut self, _: Handle, _: i32, _: i32, _: i32) {}
    fn place_bar(&mut self, _: Handle, _: Rect) {}
    fn create_bar(&mut self, _: Handle) -> Option<Handle> {
        None
    }
    fn destroy_bar(&mut self, _: Handle) {}
    fn timer_elapsed(&mut self, _: Handle) -> u32 {
        0
    }
    fn restart_timer(&mut self, _: Handle) {}
    fn double_click_interval(&mut self) -> u32 {
        500
    }
    fn begins_with(&mut self, text: &[u8], prefix: &[u8], _: u32) -> bool {
        text.starts_with(prefix)
    }
    fn text_equal(&mut self, left: &[u8], right: &[u8]) -> bool {
        left == right
    }
    fn draw(&mut self, _: Draw) {}
}

fn path(value: &str) -> VPath {
    VPath::new(value).unwrap()
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
            view: ViewPosition::default(),
        },
    }
}

fn fixture() -> (AlbumWindow, MemoryFs, Windows, Services) {
    let mut windows = Windows::default();
    let mut host = Services::default();
    let window = windows.create_default(&mut host);
    let handles: Vec<_> = (0..6).map(|_| windows.create_default(&mut host)).collect();
    for &child in &handles {
        assert!(windows.child_add(&mut host, window, child));
    }
    let controls = Controls {
        albums: ListBox::new(handles[0], false, &mut host),
        description: TextEdit::new(
            handles[1],
            handles[1],
            0,
            Encoding {
                selector: 0,
                system_selector: 0,
            },
        ),
        picture: AlbumPicture::new(handles[2], true),
        next: handles[3],
        previous: handles[4],
        count: handles[5],
    };
    let mut fs = MemoryFs::default();
    fs.create_dir(&path("Albums")).unwrap();
    let store = SnapshotAlbum::new(path("Albums"));
    let owner = AlbumWindow::bind(&windows, window, controls, store).unwrap();
    (owner, fs, windows, host)
}

#[test]
fn creation_binds_existing_controls_and_modal_shutdown_saves_then_clears() {
    let (mut owner, mut fs, mut windows, mut host) = fixture();
    owner.on_init(&mut fs, &mut windows, &mut host).unwrap();
    assert_eq!(windows.get(owner.window).id, 0x0a8c_d3ee);
    assert_eq!(
        owner.show_current_city(&mut fs, &mut windows, &mut host, &city(), true),
        Ok(0)
    );
    assert!(host.ui.paused && host.ui.album_visible);
    assert_eq!(owner.controls.albums.entries[0].text, b"Big Town--2");
    assert_eq!(owner.controls.albums.selected, 0);
    assert_eq!(host.images.last(), Some(&None));
    assert!(windows.get(owner.controls.count).caption.is_empty());
    fs.write(&path("Albums/Big Town--2/X.png"), b"picture").unwrap();
    owner
        .go_to_album(&mut fs, &mut windows, &mut host, "Big Town", 2, false)
        .unwrap();
    owner
        .description_control()
        .set_text_bytes(Some(b"saved on shutdown"), 17, false, &mut host)
        .unwrap();
    owner.on_shutdown(&mut fs, &mut windows, &mut host).unwrap();
    assert_eq!(
        fs.read(&path("Albums/Big Town--2/X.txt")).unwrap(),
        Some(b"saved on shutdown".to_vec())
    );
    assert!(!host.ui.paused && !host.ui.album_visible);
    assert!(owner.store.albums().is_empty());
    assert!(owner.controls.albums.entries.is_empty());
    assert!(owner.controls.description.text.is_empty());
    assert!(!windows.get(owner.window).flag(1));
}

#[test]
fn browse_saves_widget_text_and_deletes_companions_then_shows_no_image() {
    let (mut owner, mut fs, mut windows, mut host) = fixture();
    owner.on_init(&mut fs, &mut windows, &mut host).unwrap();
    owner
        .show_current_city(&mut fs, &mut windows, &mut host, &city(), true)
        .unwrap();
    for name in ["X", "Y"] {
        fs.write(&path(&format!("Albums/Big Town--2/{name}.png")), b"picture")
            .unwrap();
    }
    fs.write(&path("Albums/Big Town--2/X.info"), &city().info.encode().unwrap())
        .unwrap();
    fs.write(&path("Albums/Big Town--2/X.txt"), b"old").unwrap();
    owner
        .go_to_album(&mut fs, &mut windows, &mut host, "Big Town", 2, false)
        .unwrap();
    assert_eq!(owner.controls.description.text, b"old");
    assert_eq!(host.fields.last().as_ref().unwrap().as_ref().unwrap().city, b"Big Town");
    owner
        .description_control()
        .set_text_bytes(Some(b"edited"), 6, false, &mut host)
        .unwrap();
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::Next)
        .unwrap();
    assert_eq!(
        fs.read(&path("Albums/Big Town--2/X.txt")).unwrap(),
        Some(b"edited".to_vec())
    );
    assert_eq!(windows.get(owner.controls.count).caption, "(2/2)");
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::Previous)
        .unwrap();
    owner
        .description_control()
        .set_text_bytes(Some(b""), 0, false, &mut host)
        .unwrap();
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::SaveDescription)
        .unwrap();
    assert!(fs.read(&path("Albums/Big Town--2/X.txt")).unwrap().is_none());
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::DeletePicture)
        .unwrap();
    assert!(fs.read(&path("Albums/Big Town--2/X.info")).unwrap().is_none());
    assert!(fs.read(&path("Albums/Big Town--2/X.png")).unwrap().is_none());
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::DeletePicture)
        .unwrap();
    assert_eq!(host.images.last(), Some(&None));
    assert_eq!(host.fields.last(), Some(&None));
}

#[test]
fn zoom_routes_to_picture_size_and_close_restores_already_paused_game() {
    let (mut owner, mut fs, mut windows, mut host) = fixture();
    host.ui.paused = true;
    owner.on_init(&mut fs, &mut windows, &mut host).unwrap();
    owner
        .show_current_city(&mut fs, &mut windows, &mut host, &city(), true)
        .unwrap();
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::Zoom(Zoom::Out))
        .unwrap();
    assert!(!owner.controls.picture.is_large());
    assert_eq!(
        windows.get(owner.controls.picture.window).area,
        crate::ui_windows::base::Rect(0, 0, 32, 32)
    );
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::Zoom(Zoom::In))
        .unwrap();
    assert!(owner.controls.picture.is_large());
    assert!(host.ui.zoom_requests.is_empty());
    owner
        .action(&mut fs, &mut windows, &mut host, AlbumAction::Close)
        .unwrap();
    assert!(host.ui.paused);
    assert!(!host.ui.album_visible);
    assert!(!windows.get(owner.window).flag(1));
    assert_eq!(
        owner.do_win_message([1, 2, 3, 4, 5]),
        Err(Error::Gap(Gap::NativeToolkit))
    );
    assert_eq!(Gap::NativeToolkit.question(), "SN7");
}

#[derive(Default)]
struct FailingFs {
    inner: MemoryFs,
}
impl FileSystem for FailingFs {
    fn entries(&mut self, directory: &VPath) -> Result<Vec<Entry>> {
        self.inner.entries(directory)
    }
    fn create_dir(&mut self, directory: &VPath) -> Result<()> {
        self.inner.create_dir(directory)
    }
    fn remove_dir(&mut self, directory: &VPath) -> Result<()> {
        self.inner.remove_dir(directory)
    }
    fn read(&mut self, file: &VPath) -> Result<Option<Vec<u8>>> {
        self.inner.read(file)
    }
    fn write(&mut self, _: &VPath, _: &[u8]) -> Result<()> {
        Err(Error::Io(5))
    }
    fn remove_file(&mut self, file: &VPath) -> Result<()> {
        self.inner.remove_file(file)
    }
}

#[test]
fn failed_description_save_preserves_edit_selection_and_modal_pause() {
    let (mut owner, mut fs, mut windows, mut host) = fixture();
    owner.on_init(&mut fs, &mut windows, &mut host).unwrap();
    owner
        .show_current_city(&mut fs, &mut windows, &mut host, &city(), true)
        .unwrap();
    fs.write(&path("Albums/Big Town--2/X.png"), b"picture").unwrap();
    owner
        .go_to_album(&mut fs, &mut windows, &mut host, "Big Town", 2, false)
        .unwrap();
    owner
        .description_control()
        .set_text_bytes(Some(b"unsaved"), 7, false, &mut host)
        .unwrap();
    let mut failing = FailingFs { inner: fs };
    assert_eq!(
        owner.action(&mut failing, &mut windows, &mut host, AlbumAction::Close),
        Err(Error::Io(5))
    );
    assert!(owner.store.is_modal());
    assert!(host.ui.paused && host.ui.album_visible);
    assert_eq!(owner.store.current_picture(), Some("X.png"));
    assert_eq!(owner.controls.description.text, b"unsaved");
    assert!(owner.store.description_dirty());
}
