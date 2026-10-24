//! web-network section 6: cache vectors with deterministic clock/file replies.
use fc4_core::www::cache::*;
#[derive(Default)]
struct Io {
    now: u32,
    exists: bool,
    create: bool,
    write: bool,
    read: Option<Vec<u8>>,
    deleted: Vec<Vec<u8>>,
    reads: usize,
    reserves: usize,
    writes: usize,
    attempts: Vec<&'static str>,
    ini: Vec<(Vec<u8>, Vec<u8>)>,
    clears: usize,
    collisions: usize,
    values: Vec<Vec<u8>>,
    files: Vec<Vec<u8>>,
    buckets: Option<Vec<Vec<Vec<u8>>>>,
    length: u32,
}
impl Host for Io {
    fn now(&mut self) -> u32 {
        self.now
    }
    fn temporary_directory(&mut self) -> Vec<u8> {
        b"T:\\".to_vec()
    }
    fn exists(&mut self, _: &[u8]) -> bool {
        self.exists
    }
    fn create_directory(&mut self, _: &[u8]) -> bool {
        self.create
    }
    fn remove_file(&mut self, path: &[u8]) -> bool {
        self.deleted.push(path.to_vec());
        false
    }
    fn read_file(&mut self, _: &[u8]) -> Option<Vec<u8>> {
        self.reads += 1;
        self.read.clone()
    }
    fn write_file(&mut self, _: &[u8], _: &[u8]) -> bool {
        self.writes += 1;
        self.write
    }
    fn create_new(&mut self, _: &[u8]) -> bool {
        self.reserves += 1;
        self.reserves > self.collisions
    }
    fn copy_file(&mut self, _: &[u8], _: &[u8], overwrite: bool) -> bool {
        assert!(overwrite);
        self.write
    }
    fn file_length(&mut self, _: &[u8]) -> u32 {
        self.length
    }
    fn read_ini_values(&mut self, _: &[u8], _: &[u8]) -> Option<Vec<Vec<u8>>> {
        Some(self.values.clone())
    }
    fn cache_files(&mut self, _: &[u8]) -> Option<Vec<Vec<u8>>> {
        Some(self.files.clone())
    }
    fn map_buckets(&mut self) -> Vec<Vec<Vec<u8>>> {
        self.buckets.clone().unwrap_or_else(|| vec![vec![b"K".to_vec()]])
    }
    fn clear_ini_section(&mut self, _: &[u8], section: &[u8]) {
        assert_eq!(section, b"Cache Entries");
        self.clears += 1;
    }
    fn write_ini(&mut self, _: &[u8], _: &[u8], key: &[u8], value: &[u8]) -> bool {
        self.ini.push((key.to_vec(), value.to_vec()));
        false
    }
    fn persistence_attempt(&mut self, op: &'static str) {
        self.attempts.push(op);
    }
}
fn fixture(io: &mut Io) -> Cache {
    io.now = 100;
    let mut cache = Cache::new(b"fixture.ini".to_vec());
    cache.add(io, b"K", Some(vec![1, 2, 3]), false, 255, 1);
    cache
}
#[test]
fn clear_map_releases_only_map_holds_and_preserves_files_settings_and_counters() {
    let mut io = Io::default();
    let mut cache = fixture(&mut io);
    cache.directory = b"dir\\".to_vec();
    cache.entries.get_mut(b"K".as_slice()).unwrap().filename = b"kept.tmp".to_vec();
    let mut caller = None;
    assert!(cache.data(&mut io, b"K", Some(&mut caller)));
    assert_eq!(caller.as_ref().unwrap().references(), 2);
    let counters = (cache.operations, cache.since_maintenance);
    cache.clear_map();
    assert!(cache.entries.is_empty());
    assert_eq!(caller.as_ref().unwrap().references(), 1);
    assert_eq!(caller.unwrap().bytes(), [1, 2, 3]);
    assert!(io.deleted.is_empty());
    assert_eq!(cache.directory(), b"dir\\");
    assert_eq!(cache.ini_name(), b"fixture.ini");
    assert_eq!((cache.operations, cache.since_maintenance), counters);
    assert!(!cache.initialized);
}

#[test]
fn nfc01_through_nfc06() {
    let mut io = Io::default();
    let mut c = fixture(&mut io);
    assert!(c.data(&mut io, b"K", None));
    assert_eq!(io.reads, 0);
    assert_eq!(c.info(b"K").unwrap().access, 100);
    c.entries.get_mut(b"K".as_slice()).unwrap().filename = b"old".to_vec();
    io.now = 101;
    assert!(!c.data(&mut io, b"K", None));
    assert!(c.entries.is_empty());
    assert_eq!(io.deleted, [b"old".to_vec()]);
    c = fixture(&mut io);
    assert_eq!(c.add_sized(&mut io, b"K", None, 10, false, 0, 10), Ok(false));
    assert!(c.entries.is_empty());
    c = fixture(&mut io);
    assert_eq!(c.add_sized(&mut io, b"K", Some(vec![1]), 0, true, 0, 10), Ok(false));
    assert!(c.entries.is_empty());
    c = fixture(&mut io);
    let counters = (c.operations, c.since_maintenance);
    assert!(!c.add(&mut io, b"", Some(vec![9]), false, 0, 10));
    assert_eq!((c.operations, c.since_maintenance), counters);
    assert!(c.entries.contains_key(b"K".as_slice()));
    assert!(c.add(&mut io, b"K", Some(vec![9]), true, 1, -1));
    let e = c.info(b"K").unwrap();
    assert_eq!((e.creation, e.access, e.expiration), (100, 100, 432100));
    c.add(&mut io, b"K", Some(vec![9]), false, 1, -2);
    assert_eq!(c.info(b"K").unwrap().expiration, 2147483647);
}
#[test]
fn nfc07_through_nfc12() {
    let mut io = Io::default();
    let mut c = fixture(&mut io);
    assert_eq!(c.file(&mut io, b"K", None), Ok(false));
    assert_eq!(io.reserves, 0);
    c.directory = b"dir\\".to_vec();
    let mut out = vec![9];
    assert_eq!(c.file(&mut io, b"K", Some(&mut out)), Ok(false));
    assert_eq!(out, b"dir\\");
    assert!(c.info(b"K").unwrap().filename.is_empty());
    c.operations = 9;
    c.since_maintenance = 40;
    assert!(!c.data(&mut io, b"missing", None));
    assert_eq!((c.operations, c.since_maintenance), (10, 0));
    c.entries.get_mut(b"K".as_slice()).unwrap().expiration = 0x7fffffff;
    io.now = 0x80000000;
    assert!(c.present(&mut io, b"K"));
    c.maintain(&mut io, 0x80000000);
    assert!(c.entries.is_empty());
    assert!(!c.set_directory(&mut io, b""));
    assert!(c.directory.is_empty());
    assert!(c.init(&mut io));
    assert!(c.initialized);
    assert_eq!(c.directory, b"T:\\Maxis Network Cache\\");
    assert_eq!(io.attempts, ["read", "remove-unused"]);
    c.init(&mut io);
    assert_eq!(io.attempts.len(), 2);
}
#[test]
fn holders_loading_failures_paths_settings_and_persistence() {
    let mut io = Io::default();
    let mut c = fixture(&mut io);
    let mut out = None;
    assert!(c.data(&mut io, b"K", Some(&mut out)));
    assert_eq!(out.as_ref().unwrap().references(), 2);
    assert_eq!(out.as_ref().unwrap().bytes(), [1, 2, 3]);
    c.remove(&mut io, b"K");
    assert_eq!(out.as_ref().unwrap().references(), 1);
    c = fixture(&mut io);
    io.write = true;
    c.force_file(&mut io, b"K").unwrap();
    assert!(c.present_file(&mut io, b"K"));
    c.entries.get_mut(b"K".as_slice()).unwrap().ram = None;
    io.read = Some(vec![4, 5]);
    assert!(c.data(&mut io, b"K", Some(&mut out)));
    assert_eq!(out.as_ref().unwrap().bytes(), [4, 5]);
    assert_eq!(c.size(b"K"), 3); // Loading does not rewrite the info length.
    c.write_ini(&mut io).unwrap();
    assert_eq!(
        io.ini,
        [(b"CacheEntry00000".to_vec(), b"K,100.cache,3,255,1,100,100,101".to_vec())]
    );
    c.entries.get_mut(b"K".as_slice()).unwrap().ram = None;
    io.read = None;
    let saved = out.clone();
    assert!(!c.data(&mut io, b"K", Some(&mut out)));
    assert_eq!(out, saved);
    assert!(c.entries.is_empty());
    assert_eq!(io.deleted.last().unwrap(), b"100.cache");
    assert!(c.set_ram_size(u32::MAX));
    assert!(c.set_file_size(0));
    assert!(c.set_default_lifetime(-2));
    assert_eq!((c.ram_size, c.file_size, c.default_lifetime), (u32::MAX, 0, -2));
    io.exists = true;
    assert!(c.set_ini_name(&mut io, b"new.ini"));
    assert_eq!(io.deleted.last().unwrap(), b"fixture.ini");
    assert!(!c.set_directory(&mut Io::default(), b"bad"));
    assert_eq!(c.directory, b"bad\\");
    c.initialized = true;
    assert!(c.shutdown(&mut io));
    assert!(!c.initialized);
}

#[test]
fn nfc13_through_nfc19() {
    let mut io = Io {
        now: 100,
        collisions: 1,
        exists: true,
        ..Io::default()
    };
    let mut c = Cache::default();
    assert_eq!(c.ini_name(), b"FileCache.ini");
    assert_eq!(
        c.reserve_filename(&mut io, b"a.html", None, false),
        (true, b"99.html.cache".to_vec())
    );
    assert_eq!(io.reserves, 2); // CREATE_NEW refuses the occupied candidate.
    c.add(&mut io, b"K", Some(vec![1]), false, 0, 1);
    assert!(!c.add_source_file(&mut io, b"K", b"outside\\source", false, 1, -1));
    assert!(c.info(b"K").is_none());
    assert!(!io.deleted.iter().any(|p| p == b"outside\\source"));
    io.values = vec![
        b"a,b,f.cache,3,1,-2,200,300,500".to_vec(),
        b"bad,x.cache,4000001,0,0,0,0,500".to_vec(),
        b"bad,y.cache,1,0,0,0,0,100".to_vec(),
    ];
    c.read_ini(&mut io).unwrap();
    let e = c.info(b"a,b").unwrap();
    assert_eq!((e.length, e.creation, e.access, e.expiration), (3, 100, 100, 500));
    assert!(e.ram.is_none());
    assert!(io.deleted.contains(&b"x.cache".to_vec()));
    assert!(io.deleted.contains(&b"y.cache".to_vec()));
    c.entries.clear();
    let entry = Entry {
        filename: b"a.cache".to_vec(),
        ram: None,
        length: 3,
        attribute: 0,
        lifetime: 1,
        creation: 0,
        access: 50,
        expiration: 1,
    };
    c.entries.insert(b"Z".to_vec(), entry.clone());
    c.entries.insert(b"A".to_vec(), entry);
    io.files = vec![b"a.cache".to_vec(), b"A.cache".to_vec(), b"keep.txt".to_vec()];
    io.deleted.clear();
    c.remove_unused_files(&mut io).unwrap();
    assert_eq!(io.deleted, [b"A.cache".to_vec()]);
    c.operations = 9;
    c.since_maintenance = 10;
    io.buckets = Some(vec![vec![b"Z".to_vec()], vec![b"A".to_vec()]]);
    c.write_ini(&mut io).unwrap();
    assert_eq!(
        (c.operations, c.since_maintenance, c.info(b"Z").unwrap().access),
        (9, 10, 50)
    );
    assert_eq!(
        io.ini[0],
        (b"CacheEntry00000".to_vec(), b"Z,a.cache,3,0,1,0,50,1".to_vec())
    );
    assert!(io.ini[1].1.starts_with(b"A,"));
    assert_eq!(io.deleted.last().unwrap(), b"FileCache.ini");
    assert_eq!(hash(&[0xff]), u32::MAX);
}

#[test]
fn borrowed_info_bypasses_expiry_and_no_copy_keeps_allocation() {
    let mut io = Io::default();
    let mut c = fixture(&mut io);
    let bytes = vec![7, 8, 9];
    let address = bytes.as_ptr();
    assert!(c.add(&mut io, b"N", Some(bytes), false, 0, 10));
    assert_eq!(c.info(b"N").unwrap().ram.as_ref().unwrap().bytes().as_ptr(), address);
    assert!(c.present_ram(&mut io, b"N"));
    let mut info = c.info(b"K").unwrap().clone();
    io.now = 101;
    assert!(c.copy_info(b"K", &mut info));
    assert_eq!(c.created(b"K"), 100);
    assert_eq!(c.size(b"K"), 3);
    assert!(!c.present_ram(&mut io, b"K"));
    assert_eq!(c.created(b"K"), 0);
    assert_eq!(info.ram.unwrap().bytes(), [1, 2, 3]);
    assert!(!c.copy_info(b"missing", &mut c.info(b"N").unwrap().clone()));
}
