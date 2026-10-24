//! NetworkFileCache (§6). All paths are length-bearing bytes; no filesystem access.
use super::{directory, Contract, Unsupported};
use std::{collections::BTreeMap, sync::Arc};

/// The adapter transfers allocator-compatible ownership for no-copy insertion.
/// Arc shares the buffer without copying; native 16-bit overflow is outside the
/// qualified domain (§6.3). No RefCell is used by getters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Data(Arc<Vec<u8>>);
impl Data {
    pub fn owned(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn references(&self) -> usize {
        Arc::strong_count(&self.0)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub filename: Vec<u8>,
    pub ram: Option<Data>,
    pub length: u32,
    pub attribute: u32,
    pub lifetime: i32,
    pub creation: u32,
    pub access: u32,
    pub expiration: u32,
}
pub trait Host {
    fn now(&mut self) -> u32;
    fn temporary_directory(&mut self) -> Vec<u8>;
    fn exists(&mut self, path: &[u8]) -> bool;
    fn create_directory(&mut self, path: &[u8]) -> bool;
    fn remove_file(&mut self, path: &[u8]) -> bool;
    /// Open, obtain actual length, allocate and read. None includes zero length
    /// and open/read failure. Partial/failed reads must not escape as success.
    fn read_file(&mut self, path: &[u8]) -> Option<Vec<u8>>;
    fn write_file(&mut self, path: &[u8], data: &[u8]) -> bool;
    /// Atomic CREATE_NEW, write-only/shared-read; close immediately on success.
    fn create_new(&mut self, path: &[u8]) -> bool;
    fn copy_file(&mut self, source: &[u8], destination: &[u8], overwrite: bool) -> bool;
    fn file_length(&mut self, path: &[u8]) -> u32;
    fn read_ini_values(&mut self, path: &[u8], section: &[u8]) -> Option<Vec<Vec<u8>>>;
    fn cache_files(&mut self, directory: &[u8]) -> Option<Vec<Vec<u8>>>;
    /// Native map topology snapshot: ascending buckets, head-to-tail chains.
    /// Bucket count and chain topology are storage inputs, not lexical ordering.
    fn map_buckets(&mut self) -> Vec<Vec<Vec<u8>>>;
    fn clear_ini_section(&mut self, path: &[u8], section: &[u8]);
    fn write_ini(&mut self, path: &[u8], section: &[u8], key: &[u8], value: &[u8]) -> bool;
    fn persistence_attempt(&mut self, operation: &'static str);
}

#[derive(Clone, Debug)]
pub struct Cache {
    pub initialized: bool,
    pub directory: Vec<u8>,
    /// Unqualified global literal is an adapter constructor input.
    pub ini_name: Vec<u8>,
    pub entries: BTreeMap<Vec<u8>, Entry>,
    pub file_size: u32,
    pub ram_size: u32,
    pub default_lifetime: i32,
    pub operations: u32,
    pub since_maintenance: u32,
}
impl Default for Cache {
    fn default() -> Self { Self::new(b"FileCache.ini".to_vec()) }
}
pub fn hash(key: &[u8]) -> u32 {
    key.iter().fold(0u32, |value, byte| value.wrapping_mul(2) ^ (*byte as i8 as i32 as u32))
}
impl Cache {
    pub fn new(ini_name: Vec<u8>) -> Self {
        Self {
            initialized: false,
            directory: Vec::new(),
            ini_name,
            entries: BTreeMap::new(),
            file_size: 3000000,
            ram_size: 500000,
            default_lifetime: 432000,
            operations: 0,
            since_maintenance: 0,
        }
    }
    fn path(&self, name: &[u8]) -> Vec<u8> {
        [self.directory.as_slice(), name].concat()
    }
    pub fn directory(&self) -> &[u8] {
        &self.directory
    }
    pub fn ini_name(&self) -> &[u8] {
        &self.ini_name
    }
    pub fn set_directory(&mut self, host: &mut impl Host, path: &[u8]) -> bool {
        self.directory = directory(path);
        !path.is_empty() && (host.exists(&self.directory) || host.create_directory(&self.directory))
    }
    pub fn set_ini_name(&mut self, host: &mut impl Host, name: &[u8]) -> bool {
        let old = self.path(&self.ini_name);
        if host.exists(&old) {
            host.remove_file(&old);
        }
        self.ini_name = name.to_vec();
        true
    }
    pub fn set_ram_size(&mut self, size: u32) -> bool {
        self.ram_size = size;
        true
    }
    pub fn set_file_size(&mut self, size: u32) -> bool {
        self.file_size = size;
        true
    }
    pub fn set_default_lifetime(&mut self, lifetime: i32) -> bool {
        self.default_lifetime = lifetime;
        true
    }
    pub fn init(&mut self, host: &mut impl Host) -> bool {
        if !self.initialized {
            if self.directory.is_empty() {
                self.directory = directory(&host.temporary_directory());
                self.directory.extend_from_slice(b"Maxis Network Cache\\");
            }
            if !host.exists(&self.directory) {
                host.create_directory(&self.directory);
            }
            let _ = self.read_ini(host);
            let _ = self.remove_unused_files(host);
            self.initialized = true;
        }
        true
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        if self.initialized {
            let _ = self.write_ini(host);
            // Shutdown clears RAM/map storage; it does not remove persisted files.
            self.clear_map();
            self.initialized = false;
        }
        true
    }
    pub fn read_ini(&mut self, host: &mut impl Host) -> Contract<bool> {
        host.persistence_attempt("read");
        let path = self.path(&self.ini_name);
        let Some(values) = host.read_ini_values(&path, b"Cache Entries") else { return Ok(false) };
        let now = host.now();
        for value in values {
            let fields: Vec<_> = value.rsplitn(8, |b| *b == b',').collect();
            if fields.len() != 8 { continue; }
            let number = |i: usize| super::ftp::atol(fields[i]) as u32;
            let (length, expiration) = (number(5), number(0));
            let filename = fields[6].to_vec();
            if length > 4_000_000 || expiration <= now {
                self.remove_file(host, &filename);
                continue;
            }
            if !host.exists(&self.path(&filename)) { continue; }
            self.entries.insert(fields[7].to_vec(), Entry {
                filename, ram: None, length, attribute: number(4), lifetime: number(3) as i32,
                creation: number(2).min(now), access: number(1).min(now), expiration,
            });
        }
        Ok(true)
    }
    /// Release map RAM holds without deleting persisted cache files (§6.2).
    pub fn clear_map(&mut self) {
        self.entries.clear();
    }
    pub fn remove_unused_files(&mut self, host: &mut impl Host) -> Contract<bool> {
        host.persistence_attempt("remove-unused");
        for filename in host.cache_files(&self.directory).unwrap_or_default() {
            if filename.ends_with(b".cache") && !self.entries.values().any(|e| e.filename == filename) {
                self.remove_file(host, &filename);
            }
        }
        Ok(true)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn add_source_file(&mut self, host: &mut impl Host, key: &[u8], source: &[u8], copy: bool, attribute: u8, lifetime: i32) -> bool {
        if key.is_empty() { return false; }
        let now = host.now();
        self.count(host, now);
        self.remove(host, key);
        let split = source.iter().rposition(|b| matches!(b, b'/' | b'\\')).map_or(0, |i| i + 1);
        let mut name = source[split..].to_vec();
        if copy || !source[..split].eq_ignore_ascii_case(&self.directory) {
            let (_, reserved) = self.reserve_filename(host, key, None, false);
            name = reserved;
            if !host.copy_file(source, &self.path(&name), true) { return false; }
        }
        let length = host.file_length(&self.path(&name));
        if matches!(length, 0 | u32::MAX) { return false; }
        let lifetime = if lifetime == -1 { self.default_lifetime } else { lifetime };
        self.entries.insert(key.to_vec(), Entry {
            filename: name, ram: None, length, attribute: attribute as u32, lifetime,
            creation: now, access: now,
            expiration: if lifetime < 0 { i32::MAX as u32 } else { now.wrapping_add(lifetime as u32) },
        });
        true
    }
    /// Return success and the last attempted name/path, including on exhaustion.
    pub fn reserve_filename(&self, host: &mut impl Host, key: &[u8], directory: Option<&[u8]>, full_path: bool) -> (bool, Vec<u8>) {
        let suffix = key.iter().rposition(|b| *b == b'.').map(|i| &key[i..]).filter(|s| (2..=8).contains(&s.len())).unwrap_or_default();
        let suffix: Vec<_> = suffix.iter().copied().filter(|b| b.is_ascii_alphanumeric() || *b == b'.').collect();
        let directory = directory.unwrap_or(&self.directory);
        let mut time = host.now();
        let mut name = Vec::new();
        for _ in 0..10000 {
            name = [(time as i32).to_string().as_bytes(), &suffix, b".cache"].concat();
            let path = [directory, &name].concat();
            if host.create_new(&path) { return (true, if full_path { path } else { name }); }
            time = time.wrapping_sub(1);
        }
        (false, if full_path { [directory, &name].concat() } else { name })
    }
    pub fn info(&self, key: &[u8]) -> Option<&Entry> {
        self.entries.get(key)
    }
    pub fn copy_info(&self, key: &[u8], out: &mut Entry) -> bool {
        if let Some(info) = self.info(key) {
            *out = info.clone();
            true
        } else {
            false
        }
    }
    pub fn size(&self, key: &[u8]) -> u32 {
        self.info(key).map_or(0, |e| e.length)
    }
    pub fn created(&self, key: &[u8]) -> u32 {
        self.info(key).map_or(0, |e| e.creation)
    }
    pub fn time_now(&self, host: &mut impl Host) -> u32 {
        host.now()
    }
    pub fn remove_file(&self, host: &mut impl Host, name: &[u8]) -> bool {
        if name.is_empty() {
            false
        } else {
            host.remove_file(&self.path(name));
            true
        }
    }
    pub fn remove(&mut self, host: &mut impl Host, key: &[u8]) -> bool {
        if let Some(mut entry) = self.entries.remove(key) {
            drop(entry.ram.take());
            self.remove_file(host, &entry.filename);
        }
        true
    }
    pub fn maintain(&mut self, host: &mut impl Host, now: u32) {
        self.since_maintenance = 0;
        let expired: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| now >= e.expiration)
            .map(|(k, _)| k.clone())
            .collect();
        for key in expired {
            self.remove(host, &key);
        }
    }
    fn count(&mut self, host: &mut impl Host, now: u32) {
        self.operations = self.operations.wrapping_add(1);
        self.since_maintenance = self.since_maintenance.wrapping_add(1);
        if self.since_maintenance > 40 {
            self.maintain(host, now);
        }
    }
    fn live(&mut self, host: &mut impl Host, key: &[u8], now: u32) -> bool {
        let Some(entry) = self.entries.get(key) else {
            return false;
        };
        if (now as i32) < entry.expiration as i32 {
            true
        } else {
            self.remove(host, key);
            false
        }
    }
    pub fn present(&mut self, host: &mut impl Host, key: &[u8]) -> bool {
        let now = host.now();
        self.live(host, key, now)
    }
    pub fn present_ram(&mut self, host: &mut impl Host, key: &[u8]) -> bool {
        self.present(host, key) && self.entries[key].ram.is_some()
    }
    pub fn present_file(&mut self, host: &mut impl Host, key: &[u8]) -> bool {
        self.present(host, key) && !self.entries[key].filename.is_empty()
    }
    /// `copy=false` consumes the original allocation; it is never a borrowed slice.
    pub fn add(
        &mut self,
        host: &mut impl Host,
        key: &[u8],
        bytes: Option<Vec<u8>>,
        copy: bool,
        attribute: u8,
        lifetime: i32,
    ) -> bool {
        let size = bytes.as_ref().map_or(0, |b| b.len() as u32);
        // This convenience overload always supplies valid backing for its size.
        self.add_sized(host, key, bytes, size, copy, attribute, lifetime)
            .expect("length derived from supplied buffer")
    }
    #[allow(clippy::too_many_arguments)] // Preserve the specified native input channels.
    pub fn add_sized(
        &mut self,
        host: &mut impl Host,
        key: &[u8],
        bytes: Option<Vec<u8>>,
        size: u32,
        copy: bool,
        attribute: u8,
        lifetime: i32,
    ) -> Contract<bool> {
        if key.is_empty() {
            return Ok(false);
        }
        let now = host.now();
        self.count(host, now);
        self.remove(host, key);
        let Some(mut bytes) = bytes.filter(|_| size != 0) else {
            return Ok(false);
        };
        if size as usize > bytes.len() {
            return Err(Unsupported::NativeInputDomain("cache buffer shorter than size"));
        }
        bytes.truncate(size as usize);
        let bytes = if copy { bytes.clone() } else { bytes };
        let lifetime = if lifetime == -1 {
            self.default_lifetime
        } else {
            lifetime
        };
        self.entries.insert(
            key.to_vec(),
            Entry {
                filename: Vec::new(),
                length: bytes.len() as u32,
                ram: Some(Data::owned(bytes)),
                attribute: attribute as u32,
                lifetime,
                creation: now,
                access: now,
                expiration: if lifetime < 0 {
                    i32::MAX as u32
                } else {
                    now.wrapping_add(lifetime as u32)
                },
            },
        );
        Ok(true)
    }
    pub fn data(&mut self, host: &mut impl Host, key: &[u8], out: Option<&mut Option<Data>>) -> bool {
        let now = host.now();
        self.count(host, now);
        if !self.live(host, key, now) {
            return false;
        }
        if let Some(out) = out {
            if self.entries[key].ram.is_none() {
                let path = self.path(&self.entries[key].filename);
                let Some(bytes) = host.read_file(&path).filter(|b| !b.is_empty()) else {
                    self.remove(host, key);
                    return false;
                };
                self.entries.get_mut(key).unwrap().ram = Some(Data::owned(bytes));
            }
            *out = self.entries[key].ram.clone();
        }
        self.entries.get_mut(key).unwrap().access = now;
        true
    }
    pub fn file(&mut self, host: &mut impl Host, key: &[u8], out: Option<&mut Vec<u8>>) -> Contract<bool> {
        let now = host.now();
        self.count(host, now);
        if !self.live(host, key, now) {
            return Ok(false);
        }
        if let Some(out) = out {
            self.materialize(host, key)?;
            *out = self.path(&self.entries[key].filename);
        }
        let entry = self.entries.get_mut(key).unwrap();
        entry.access = now;
        Ok(!entry.filename.is_empty())
    }
    pub fn force_file(&mut self, host: &mut impl Host, key: &[u8]) -> Contract<bool> {
        self.file(host, key, Some(&mut Vec::new()))
    }
    fn materialize(&mut self, host: &mut impl Host, key: &[u8]) -> Contract<()> {
        if self.entries[key].filename.is_empty() {
            if let Some(data) = self.entries[key].ram.clone() {
                let (_, name) = self.reserve_filename(host, key, None, false);
                let path = self.path(&name);
                self.entries.get_mut(key).unwrap().filename = if host.write_file(&path, data.bytes()) {
                    name
                } else {
                    Vec::new()
                };
            }
        }
        Ok(())
    }
    pub fn write_ini(&mut self, host: &mut impl Host) -> Contract<bool> {
        let path = self.path(&self.ini_name);
        host.remove_file(&path);
        host.clear_ini_section(&path, b"Cache Entries");
        let keys: Vec<_> = host.map_buckets().into_iter().flatten().collect();
        let mut n = 0;
        for key in keys {
            // §6.4 says materialize entries, without a read/expiration sweep.
            if !self.entries.contains_key(&key) { continue; }
            self.materialize(host, &key)?;
            let Some(entry) = self.entries.get(&key) else {
                continue;
            };
            if entry.filename.is_empty() {
                continue;
            }
            let mut value = key.clone();
            value.push(b',');
            value.extend_from_slice(&entry.filename);
            value.extend_from_slice(
                format!(
                    ",{},{},{},{},{},{}",
                    entry.length,
                    entry.attribute,
                    entry.lifetime as u32,
                    entry.creation as i32,
                    entry.access as i32,
                    entry.expiration as i32
                )
                .as_bytes(),
            );
            host.write_ini(&path, b"Cache Entries", format!("CacheEntry{n:05}").as_bytes(), &value);
            n += 1;
        }
        Ok(true)
    }
}
