// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct Ini {
    section: Option<Vec<String>>,
    current: Option<String>,
    loaded: Vec<String>,
    ids: Vec<u32>,
    paths: Mutex<Vec<String>>,
    exists: bool,
}
impl DriverSettings for Ini {
    fn exists(&self, _: &str) -> bool {
        self.exists
    }
    fn entries(&self, path: &str, section: &str) -> Option<Vec<String>> {
        assert_eq!(section, "Graphics Drivers");
        self.paths.lock().unwrap().push(path.to_owned());
        self.section.clone()
    }
    fn value(&self, _: &str, section: &str, key: &str) -> Option<String> {
        assert_eq!((section, key), ("Current Graphics Driver", "Driver"));
        self.current.clone()
    }
    fn load(&mut self, entry: &str) -> Option<Arc<dyn Driver>> {
        self.loaded.push(entry.to_owned());
        self.ids.pop().map(|id| {
            Arc::new(TestDriver {
                id,
                modes: vec![],
                enumerations: AtomicU32::new(0),
            }) as Arc<dyn Driver>
        })
    }
}

#[test]
fn gzg3_driver_preference_worked_cases_and_empty_section() {
    for (explicit, ini_value, ids, expected) in [
        (
            0xc4554841,
            Some("7ACA35C6"),
            vec![0x7aca35c6, 0xc4554841],
            Some(0xc4554841),
        ),
        (1, Some("7ACA35C6"), vec![0x7aca35c6, 0xc4554841], Some(0x7aca35c6)),
        (0, None, vec![0x7aca35c6], Some(0x7aca35c6)),
        (0, None, vec![0xc4554841], None),
    ] {
        let mut g = graphics(&log());
        g.set_default_driver_class_id(explicit);
        g.set_driver_settings_ini_file_path("custom.ini");
        let mut ini = Ini {
            section: None,
            current: ini_value.map(str::to_owned),
            ids,
            loaded: vec![],
            paths: Mutex::new(vec![]),
            exists: true,
        };
        assert_eq!(g.load_all_drivers(&mut ini, "app"), expected.is_some());
        assert_eq!(g.driver().map(|d| d.id()), expected);
        assert_eq!(
            ini.loaded,
            [
                "SimGLRef.dll, C4554841",
                "SimGLSoft.dll, 7ACA35C6",
                "SimGLDx7.dll, BADB6906"
            ]
        );
        assert_eq!(*ini.paths.lock().unwrap(), ["custom.ini"]);
    }
    let mut g = graphics(&log());
    let mut ini = Ini {
        section: Some(vec![]),
        current: None,
        ids: vec![],
        loaded: vec![],
        paths: Mutex::new(vec![]),
        exists: false,
    };
    g.set_driver_settings_ini_file_path("missing.ini");
    assert!(!g.load_all_drivers(&mut ini, "app/"));
    assert!(ini.loaded.is_empty());
    assert_eq!(*ini.paths.lock().unwrap(), ["app/GZGraphic2.ini"]);
    g.control_float_f0 = 3.25;
    g.control_slot17(0xffffffff);
    assert_eq!(g.control_slot20().to_bits(), 3.25_f32.to_bits());
}

struct UiDriver {
    enabled: bool,
    calls: Mutex<Vec<(u32, UiArgument)>>,
    viewport: Mutex<Rect>,
}
impl Driver for UiDriver {
    fn id(&self) -> u32 {
        1
    }
    fn modes(&self) -> Vec<ScreenMode> {
        vec![]
    }
    fn enumerate_setup(&self) {}
    fn ui_available(&self) -> bool {
        self.enabled
    }
    fn ui_call(&self, slot: u32, args: UiArgument) -> Result<()> {
        self.calls.lock().unwrap().push((slot, args));
        Ok(())
    }
    fn ui_viewport(&self) -> Result<Rect> {
        self.ui_call(0x130, UiArgument::None)?;
        Ok(*self.viewport.lock().unwrap())
    }
}
#[test]
fn gzg3_ui_order_and_cached_projection_worked_case() {
    let d = Arc::new(UiDriver {
        enabled: true,
        calls: Mutex::new(vec![]),
        viewport: Mutex::new(Rect(10, 20, 650, 500)),
    });
    let driver: Arc<dyn Driver> = d.clone();
    let mut g = graphics(&log());
    g.set_driver(Some(&driver));
    use UiArgument::*;
    let mut p = [0.; 16];
    p[0] = 0.003125;
    p[5] = -1. / 240.;
    p[12..].copy_from_slice(&[-1., 1., 0.5, 1.]);
    let expected = vec![
        (0xcc, Word(1)),
        (0xb4, Word(5)),
        (0xcc, Word(0)),
        (0xb0, Word(5)),
        (0xd4, Words(vec![0, 3, 2, 1])),
        (0xd0, Word(0)),
        (0xb4, Word(2)),
        (0xb0, Word(1)),
        (0xb4, Word(1)),
        (0xb4, Word(0)),
        (0xb4, Word(4)),
        (0xb4, Word(3)),
        (0xb4, Word(6)),
        (0x84, Floats(vec![1.])),
        (0x80, Floats(vec![1., 1., 1.])),
        (0x44, Word(0)),
        (0x3c, Word(1)),
        (0x88, Words(vec![1, 1])),
        (0x10c, Word(0)),
        (0x12c, None),
        (0x130, None),
        (0xa0, Word(1)),
        (0xa4, Projection(p)),
        (0xa0, Word(0)),
        (0xa8, None),
    ];
    for viewport in [Rect(10, 20, 650, 500), Rect(0, 0, 800, 600)] {
        *d.viewport.lock().unwrap() = viewport;
        g.set_ui_rendering_mode().unwrap();
        assert_eq!(std::mem::take(&mut *d.calls.lock().unwrap()), expected);
    }
    let disabled = Arc::new(UiDriver {
        enabled: false,
        calls: Mutex::new(vec![]),
        viewport: Mutex::new(Rect::default()),
    });
    g.set_driver(Some(&(disabled.clone() as Arc<dyn Driver>)));
    g.set_ui_rendering_mode().unwrap();
    assert!(disabled.calls.lock().unwrap().is_empty());
}

struct AlphaBuffer {
    id: u32,
    trace: Arc<Mutex<Vec<(u32, bool)>>>,
    inner: TestBuffer,
}
impl Buffer for AlphaBuffer {
    fn lock(&self, flags: u32) -> bool {
        assert_eq!(flags, 0x8040);
        self.trace.lock().unwrap().push((self.id, true));
        self.inner.lock_ok
    }
    fn unlock(&self, flags: u32) {
        assert_eq!(flags, 0x8040);
        self.trace.lock().unwrap().push((self.id, false));
    }
    fn record(&self) -> BufferRecord {
        self.inner.record
    }
    fn native_rgb(&self, c: u32) -> u32 {
        c
    }
    fn pixels(&self, r: BufferRecord, op: &mut dyn FnMut(&mut [u8], usize) -> Result<()>) -> Result<()> {
        self.inner.pixels(r, op)
    }
}
fn alpha_buffer(id: u32, trace: &Arc<Mutex<Vec<(u32, bool)>>>, ok: bool, format: u32) -> AlphaBuffer {
    AlphaBuffer {
        id,
        trace: trace.clone(),
        inner: TestBuffer {
            log: log(),
            lock_ok: ok,
            pixels: Mutex::new(vec![0; 24]),
            record: BufferRecord {
                pixels: 0,
                stride: 12,
                width: 3,
                height: 2,
                colour_type: format,
                bytes_per_pixel: 2,
            },
        },
    }
}
#[test]
fn gzg5_ordinary_alpha_worked_pixels_and_lock_failures() {
    let trace = Arc::new(Mutex::new(vec![]));
    for (format, word, expected) in [(4, 0x7fff_u16, 0x7bde_u16), (5, 0xffff, 0xf79e)] {
        let s = alpha_buffer(1, &trace, true, format);
        let d = alpha_buffer(2, &trace, true, format);
        let a = alpha_buffer(3, &trace, true, 2);
        s.inner.pixels.lock().unwrap()[14..18].copy_from_slice(&[word.to_le_bytes(), word.to_le_bytes()].concat());
        a.inner.pixels.lock().unwrap()[13..15].copy_from_slice(&[31, 0]);
        let src = AlphaSource {
            buffer: &s,
            depth: 16,
            colour_key: None,
        };
        assert_eq!(alpha_blt_no_clip(src, &d, &a, Rect(1, 1, 3, 2), [0, 0]), Ok(1));
        assert_eq!(
            &d.inner.pixels.lock().unwrap()[..4],
            &[expected.to_le_bytes(), [0, 0]].concat()
        );
        assert!(d.inner.pixels.lock().unwrap()[4..].iter().all(|&b| b == 0));
        assert_eq!(
            std::mem::take(&mut *trace.lock().unwrap()),
            [(1, true), (2, true), (3, true), (3, false), (2, false), (1, false)]
        );
    }
    for (failure, expected) in [
        (1, vec![(1, true)]),
        (2, vec![(1, true), (2, true), (1, false)]),
        (3, vec![(1, true), (2, true), (3, true), (2, false), (1, false)]),
        (
            0,
            vec![(1, true), (2, true), (3, true), (3, false), (2, false), (1, false)],
        ),
    ] {
        let s = alpha_buffer(1, &trace, failure != 1, 4);
        let d = alpha_buffer(2, &trace, failure != 2, 7); // unsupported destination still succeeds after locks
        let a = alpha_buffer(3, &trace, failure != 3, 2);
        d.inner.pixels.lock().unwrap().fill(0xa5);
        assert_eq!(
            alpha_blt_no_clip(
                AlphaSource {
                    buffer: &s,
                    depth: 16,
                    colour_key: None
                },
                &d,
                &a,
                Rect(0, 0, 1, 1),
                [0, 0]
            ),
            Ok(u8::from(failure == 0))
        );
        assert!(d.inner.pixels.lock().unwrap().iter().all(|&b| b == 0xa5));
        assert_eq!(std::mem::take(&mut *trace.lock().unwrap()), expected);
    }
}

struct Texture;

#[test]
fn gzg5_ordinary_keyed_type6_and_unsupported_source_depth() {
    let trace = Arc::new(Mutex::new(vec![]));
    let s = alpha_buffer(1, &trace, true, 4);
    let d = alpha_buffer(2, &trace, true, 6);
    let a = alpha_buffer(3, &trace, true, 2);
    s.inner.pixels.lock().unwrap()[..4].copy_from_slice(&[0xff, 0x7f, 0, 0]);
    a.inner.pixels.lock().unwrap()[..2].fill(31);
    d.inner.pixels.lock().unwrap()[..4].copy_from_slice(&[0x34, 0x12, 0xff, 0x7f]);
    assert_eq!(
        alpha_blt_no_clip(
            AlphaSource {
                buffer: &s,
                depth: 16,
                colour_key: Some(0x7fff)
            },
            &d,
            &a,
            Rect(0, 0, 2, 1),
            [0, 0]
        ),
        Ok(1)
    );
    assert_eq!(&d.inner.pixels.lock().unwrap()[..4], &[0x34, 0x12, 0, 0]);
    d.inner.pixels.lock().unwrap().fill(0xa5);
    assert_eq!(
        alpha_blt_no_clip(
            AlphaSource {
                buffer: &s,
                depth: 8,
                colour_key: None
            },
            &d,
            &a,
            Rect(0, 0, 2, 1),
            [0, 0]
        ),
        Ok(1)
    );
    assert!(d.inner.pixels.lock().unwrap().iter().all(|&b| b == 0xa5));
    assert_eq!(
        alpha_blt_no_clip(
            AlphaSource {
                buffer: &s,
                depth: 16,
                colour_key: None
            },
            &s,
            &a,
            Rect(0, 0, 1, 1),
            [0, 0]
        ),
        Err(Error::Unsupported("GZG5"))
    );
}

impl ImageTexture for Texture {}
struct Cache(bool);
impl ImageCache for Cache {
    fn query_texture(&self, iid: u32) -> Option<Arc<dyn ImageTexture>> {
        assert_eq!(iid, 0xeb4d101f);
        self.0.then(|| Arc::new(Texture) as Arc<dyn ImageTexture>)
    }
}
struct Source {
    cache: Option<Arc<dyn ImageCache>>,
    attach: bool,
}
impl ImageSource for Source {
    fn cache(&self) -> Option<&Arc<dyn ImageCache>> {
        self.cache.as_ref()
    }
    fn allocate_cache(&mut self, bytes: usize) -> Option<Arc<dyn ImageCache>> {
        assert_eq!(bytes, 0x68);
        Some(Arc::new(Cache(true)))
    }
    fn attach_cache(&mut self, cache: &Arc<dyn ImageCache>) -> bool {
        if self.attach {
            self.cache = Some(cache.clone());
        }
        self.attach
    }
}
#[test]
fn gzg2_image_cache_failure_and_null_source_worked_cases() {
    let mut binding = ImageBinding::default();
    let mut source = Source {
        cache: Some(Arc::new(Cache(true))),
        attach: true,
    };
    assert_eq!(binding.set_image(Some(&mut source)), 1);
    let old_cache = Arc::downgrade(binding.cache.as_ref().unwrap());
    let old_texture = Arc::downgrade(binding.texture.as_ref().unwrap());
    assert_eq!(binding.set_image(None), 0);
    assert!(binding.cache.is_none());
    assert_eq!(old_cache.strong_count(), 1); // source still owns its cache
    assert_eq!(old_texture.strong_count(), 1); // null source preserves the texture
    let rejected: Arc<dyn ImageCache> = Arc::new(Cache(false));
    let mut bad = Source {
        cache: Some(rejected.clone()),
        attach: true,
    };
    assert_eq!(binding.set_image(Some(&mut bad)), 0);
    assert!(binding.cache.is_none() && binding.texture.is_none());
    assert_eq!(Arc::strong_count(&rejected), 2);
    assert_eq!(old_texture.strong_count(), 0);
    assert_eq!(binding.set_image(None), 0);
    let mut fresh = Source {
        cache: None,
        attach: true,
    };
    assert_eq!(binding.set_image(Some(&mut fresh)), 1);
    assert_eq!(Arc::strong_count(fresh.cache.as_ref().unwrap()), 2);
    let mut failed = Source {
        cache: None,
        attach: false,
    };
    assert_eq!(binding.set_image(Some(&mut failed)), 0);
    assert!(binding.cache.is_none() && binding.texture.is_none());
}

#[test]
fn gzg2_image_allocation_failure_releases_old_binding_in_order() {
    // Section 4.4: release the cache before source callbacks, then release the
    // old texture even when allocation fails. No old binding is restored.
    struct MissingCache {
        cache: std::sync::Weak<dyn ImageCache>,
        texture: std::sync::Weak<dyn ImageTexture>,
        allocated: bool,
    }
    impl ImageSource for MissingCache {
        fn cache(&self) -> Option<&Arc<dyn ImageCache>> {
            assert_eq!(self.cache.strong_count(), 0);
            assert_eq!(self.texture.strong_count(), 1);
            None
        }
        fn allocate_cache(&mut self, bytes: usize) -> Option<Arc<dyn ImageCache>> {
            assert_eq!(bytes, 0x68);
            assert_eq!(self.cache.strong_count(), 0);
            assert_eq!(self.texture.strong_count(), 1);
            self.allocated = true;
            None
        }
        fn attach_cache(&mut self, _: &Arc<dyn ImageCache>) -> bool {
            panic!("allocation failure must not attach a cache")
        }
    }
    let mut binding = ImageBinding {
        cache: Some(Arc::new(Cache(true))),
        texture: Some(Arc::new(Texture)),
    };
    let mut source = MissingCache {
        cache: Arc::downgrade(binding.cache.as_ref().unwrap()),
        texture: Arc::downgrade(binding.texture.as_ref().unwrap()),
        allocated: false,
    };
    assert_eq!(binding.set_image(Some(&mut source)), 0);
    assert!(source.allocated);
    assert!(binding.cache.is_none() && binding.texture.is_none());
    assert_eq!(source.cache.strong_count(), 0);
    assert_eq!(source.texture.strong_count(), 0);
}

struct ExtensionDevice(Arc<Mutex<Vec<u32>>>);
impl HardwareDevice for ExtensionDevice {
    fn init(&mut self, _: &Arc<dyn Buffer>) -> bool {
        true
    }
    fn apply_state(&mut self, _: &EngineState) -> Result<()> {
        Ok(())
    }
    fn update_transform(&mut self, _: &EngineState) -> Result<()> {
        Ok(())
    }
    fn submit(&mut self, _: u32, _: &[Point], _: &EngineState) -> Result<()> {
        Ok(())
    }
    fn suspend(&mut self) -> Result<()> {
        Ok(())
    }
    fn resume(&mut self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&mut self) {
        self.0.lock().unwrap().push(25);
    }
    fn viewport(&mut self, bounds: Rect) -> Result<()> {
        assert_eq!(bounds, Rect(1, -12, 4, 12));
        self.0.lock().unwrap().push(27);
        Ok(())
    }
    fn matrix_mode(&mut self, mode: u32) -> Result<()> {
        assert_eq!(mode, 1);
        self.0.lock().unwrap().push(28);
        Ok(())
    }
    fn projection(&mut self, matrix: &[f32; 16]) -> Result<()> {
        assert_eq!(*matrix, [0.25; 16]);
        self.0.lock().unwrap().push(128);
        Ok(())
    }
    fn vertices(&mut self, primitive: u32, records: &[[u32; 6]]) -> Result<()> {
        assert_eq!(records, &[[1, 2, 3, 4, 5, 6]]);
        self.0.lock().unwrap().push(primitive);
        Ok(())
    }
}

#[test]
fn gzg2_extension_forwarding_shutdown_and_base_fallback() {
    let trace = Arc::new(Mutex::new(vec![]));
    let t = target(Rect::default(), Some(buffer(&log(), 5, true)));
    let mut e = SimGlEngine::new(Box::new(ExtensionDevice(trace.clone())), None);
    assert!(!e.base.state.dirty && !e.base.state.transform_dirty);
    assert!(e.init(&t));
    // The test buffer is 8x2. §4.4 uses height-bottom, then width/height.
    e.hw_viewport(Rect(1, 2, 5, 14)).unwrap();
    e.hw_projection(&[0.25; 16]).unwrap();
    e.hw_vertices(false, &[[1, 2, 3, 4, 5, 6]]).unwrap();
    e.hw_vertices(true, &[[1, 2, 3, 4, 5, 6]]).unwrap();
    assert_eq!(e.draw(6, &[]), Ok(0));
    assert_eq!(e.draw(3, &[]), Ok(0));
    e.shutdown();
    assert_eq!(*trace.lock().unwrap(), [27, 28, 128, 0, 6, 25]);
}
