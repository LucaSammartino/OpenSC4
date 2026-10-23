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
    /// Filename allocation/collision policy is explicitly Q11.
    fn reserve_filename(&mut self, _directory: &[u8]) -> Contract<Vec<u8>> {
        Err(Unsupported::PrimitiveContract("Q11 cache filename allocation"))
    }
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
            self.entries.clear();
            self.initialized = false;
        }
        true
    }
    pub fn read_ini(&mut self, host: &mut impl Host) -> Contract<bool> {
        host.persistence_attempt("read");
        Err(Unsupported::OwnedBody("Q11 cache INI loader acceptance"))
    }
    pub fn remove_unused_files(&mut self, host: &mut impl Host) -> Contract<bool> {
        host.persistence_attempt("remove-unused");
        Err(Unsupported::OwnedBody("Q11 cache unused-file selection"))
    }
    pub fn add_source_file(&mut self) -> Contract<bool> {
        Err(Unsupported::OwnedBody("Q11 cache source-file admission"))
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
                let name = host.reserve_filename(&self.directory)?;
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
        host.clear_ini_section(&path, b"Cache Entries");
        let keys: Vec<_> = self.entries.keys().cloned().collect();
        let mut n = 0;
        for key in keys {
            // §6.4 says materialize entries, without a read/expiration sweep.
            // Literal direct materialization, pending Q15 on writer edge cases.
            self.materialize(host, &key)?;
            let Some(entry) = self.entries.get(&key) else {
                continue;
            };
            if entry.filename.is_empty() {
                continue;
            }
            // Escaping of delimiters is Q11; refuse that input, rather than invent CSV.
            if key.contains(&b',') || key.contains(&0) || entry.filename.contains(&b',') || entry.filename.contains(&0)
            {
                return Err(Unsupported::NativeInputDomain("Q11 cache INI delimiters"));
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
