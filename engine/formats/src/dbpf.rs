// SPDX-License-Identifier: GPL-3.0-or-later
//! DBPF 1.x packages: the container of `.dat`, `.sc4`, `.sc4lot`, `.sc4desc` and `.sc4model` files.
//!
//! Layout (all fields little-endian `u32` unless noted):
//!
//! ```text
//! 0   char[4] "DBPF"          36  index entry count
//! 4   major version (1)       40  index offset
//! 8   minor version (0 or 1)  44  index size in bytes
//! 12  user major              48  hole entry count
//! 16  user minor              52  hole offset
//! 20  flags                   56  hole size
//! 24  date created            60  index minor version (0 = 7.0; 1 = 7.1 in DBPF 1.1)
//! 28  date modified           64  reserved, 32 bytes (header is 96 bytes)
//! 32  index major version (7)
//! ```
//!
//! An index 7.0 entry is `type, group, instance, offset, size` (20 bytes); 7.1 adds a second instance word
//! after the instance (24 bytes). The DIR record ([`DIR_TGI`]) lists the compressed entries as
//! `type, group, instance, [instance2,] uncompressed size`. Compressed entries are QFS ([`crate::qfs`]).
//!
//! Quirks handled *\[choice\]* (each one adds a note to [`Package::warnings`] instead of failing the open):
//! - The index entry size is taken from `index size / entry count` when that is exactly 20 or 24, and from
//!   the index minor version otherwise (some writers leave the index size or the minor version wrong).
//! - Entries that point outside the file stay in the index; reading them returns [`Error::OutOfBounds`].
//! - A DIR record whose length is not a multiple of its entry size: the trailing bytes are ignored.
//! - An entry is decompressed only if the DIR lists it **and** its data starts with a QFS header. A listed
//!   entry without the header is returned as stored; an unlisted entry is never decompressed (the DIR is the
//!   authority, as in the public documentation). [`Package::read_raw`] always returns the stored bytes.
//! - Duplicate keys: [`Package::find`] returns the **last** index entry with the key (later data in a
//!   package overrides earlier data); [`Package::find_all`] returns every one.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use crate::error::{corrupt, Error, Result};
use crate::qfs;
use crate::reader::{le_u32, Reader};
use crate::tgi::Tgi;
pub use crate::tgi::DIR_TGI;

/// Size of the DBPF 1.x header.
pub const HEADER_LEN: usize = 96;

/// Random-access byte source behind a [`Package`].
pub trait Source {
    /// Total length in bytes.
    fn len(&self) -> u64;
    /// Fills `buf` from `offset`. The caller has already checked that the range is inside `len()`.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()>;
    /// True if the source is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A package held in memory: a `Vec<u8>`, a borrowed slice, or a memory map owned by the caller.
pub struct MemSource<T>(pub T);

impl<T: AsRef<[u8]>> Source for MemSource<T> {
    fn len(&self) -> u64 {
        self.0.as_ref().len() as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let d = self.0.as_ref();
        let start = usize::try_from(offset).map_err(|_| std::io::ErrorKind::UnexpectedEof)?;
        let end = start
            .checked_add(buf.len())
            .filter(|&e| e <= d.len())
            .ok_or(std::io::ErrorKind::UnexpectedEof)?;
        buf.copy_from_slice(&d[start..end]);
        Ok(())
    }
}

/// A package read from an open file (opened read-only; nothing is ever written).
pub struct FileSource {
    file: Mutex<File>,
    len: u64,
}

impl FileSource {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Ok(FileSource {
            file: Mutex::new(file),
            len,
        })
    }
}

impl Source for FileSource {
    fn len(&self) -> u64 {
        self.len
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let mut f = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("file lock poisoned"))?;
        f.seek(SeekFrom::Start(offset))?;
        f.read_exact(buf)
    }
}

/// The DBPF header fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub major: u32,
    pub minor: u32,
    pub user_major: u32,
    pub user_minor: u32,
    pub flags: u32,
    pub created: u32,
    pub modified: u32,
    pub index_major: u32,
    pub index_count: u32,
    pub index_offset: u32,
    pub index_size: u32,
    pub hole_count: u32,
    pub hole_offset: u32,
    pub hole_size: u32,
    pub index_minor: u32,
}

impl Header {
    /// Parses the 96-byte header.
    pub fn parse(d: &[u8]) -> Result<Header> {
        if d.len() < 4 || &d[0..4] != b"DBPF" {
            return Err(Error::BadMagic {
                what: "DBPF header",
                found: d.iter().take(4).copied().collect(),
            });
        }
        if d.len() < HEADER_LEN {
            return Err(Error::Truncated {
                what: "DBPF header",
                needed: HEADER_LEN as u64,
                available: d.len() as u64,
            });
        }
        let f = |i: usize| le_u32(d, 4 + 4 * i);
        Ok(Header {
            major: f(0),
            minor: f(1),
            user_major: f(2),
            user_minor: f(3),
            flags: f(4),
            created: f(5),
            modified: f(6),
            index_major: f(7),
            index_count: f(8),
            index_offset: f(9),
            index_size: f(10),
            hole_count: f(11),
            hole_offset: f(12),
            hole_size: f(13),
            index_minor: f(14),
        })
    }

    fn write(&self, out: &mut [u8]) {
        out[0..4].copy_from_slice(b"DBPF");
        let v = [
            self.major,
            self.minor,
            self.user_major,
            self.user_minor,
            self.flags,
            self.created,
            self.modified,
            self.index_major,
            self.index_count,
            self.index_offset,
            self.index_size,
            self.hole_count,
            self.hole_offset,
            self.hole_size,
            self.index_minor,
        ];
        for (i, x) in v.iter().enumerate() {
            out[4 + 4 * i..8 + 4 * i].copy_from_slice(&x.to_le_bytes());
        }
    }
}

/// One index entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexEntry {
    pub tgi: Tgi,
    /// The second instance word of index 7.1 (`None` for 7.0).
    pub instance2: Option<u32>,
    pub offset: u32,
    pub size: u32,
}

/// One record of the DIR (compressed-entry directory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub tgi: Tgi,
    pub instance2: Option<u32>,
    pub uncompressed_size: u32,
}

/// An open DBPF package.
pub struct Package<S> {
    src: S,
    header: Header,
    entries: Vec<IndexEntry>,
    entry_size: usize,
    dir: HashMap<(Tgi, u32), u32>,
    dir_entries: Vec<DirEntry>,
    warnings: Vec<String>,
}

impl Package<FileSource> {
    /// Opens a package file read-only.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Package::from_source(FileSource::open(path)?)
    }
}

impl<T: AsRef<[u8]>> Package<MemSource<T>> {
    /// Opens a package held in memory (a `Vec<u8>`, a slice or a caller-owned memory map).
    pub fn from_bytes(data: T) -> Result<Self> {
        Package::from_source(MemSource(data))
    }
}

impl<S: Source> Package<S> {
    /// Reads the header, the index and the DIR record from any source.
    pub fn from_source(src: S) -> Result<Self> {
        let len = src.len();
        let mut hb = [0u8; HEADER_LEN];
        let hn = len.min(HEADER_LEN as u64) as usize;
        src.read_at(0, &mut hb[..hn])?;
        let header = Header::parse(&hb[..hn])?;
        if header.major != 1 {
            return Err(Error::Unsupported {
                what: "DBPF",
                detail: format!("version {}.{} (only 1.x is read)", header.major, header.minor),
            });
        }
        if header.index_major != 7 {
            return Err(Error::Unsupported {
                what: "DBPF index",
                detail: format!("index major {}", header.index_major),
            });
        }
        let mut warnings = Vec::new();
        let count = header.index_count as u64;
        let entry_size: usize = if count > 0 && header.index_size as u64 == count * 20 {
            20
        } else if count > 0 && header.index_size as u64 == count * 24 {
            24
        } else {
            let s = if header.index_minor >= 1 { 24 } else { 20 };
            if count > 0 {
                warnings.push(format!(
                    "index size {} does not match {count} entries; using {s}-byte entries (index minor {})",
                    header.index_size, header.index_minor
                ));
            }
            s
        };
        let index_bytes = count * entry_size as u64;
        if index_bytes > 0 && header.index_offset as u64 + index_bytes > len {
            return Err(Error::OutOfBounds {
                what: "DBPF index",
                offset: header.index_offset as u64,
                size: index_bytes,
                limit: len,
            });
        }
        let mut raw = vec![0u8; index_bytes as usize];
        src.read_at(header.index_offset as u64, &mut raw)?;
        let mut r = Reader::new(&raw, "DBPF index");
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let tgi = Tgi::new(r.u32()?, r.u32()?, r.u32()?);
            let instance2 = if entry_size == 24 { Some(r.u32()?) } else { None };
            let e = IndexEntry {
                tgi,
                instance2,
                offset: r.u32()?,
                size: r.u32()?,
            };
            if e.offset as u64 + e.size as u64 > len {
                warnings.push(format!(
                    "entry {} at {}+{} is outside the file ({len} bytes)",
                    e.tgi, e.offset, e.size
                ));
            }
            entries.push(e);
        }
        let mut pkg = Package {
            src,
            header,
            entries,
            entry_size,
            dir: HashMap::new(),
            dir_entries: Vec::new(),
            warnings,
        };
        pkg.load_dir();
        Ok(pkg)
    }

    fn load_dir(&mut self) {
        let Some(e) = self.entries.iter().rev().find(|e| e.tgi == DIR_TGI).copied() else {
            return;
        };
        let data = match self.read_raw(&e) {
            Ok(d) => d,
            Err(err) => {
                self.warnings.push(format!("DIR record unreadable: {err}"));
                return;
            }
        };
        let rec = self.entry_size - 4;
        if data.len() % rec != 0 {
            self.warnings
                .push(format!("DIR record length {} is not a multiple of {rec}", data.len()));
        }
        for chunk in data.chunks_exact(rec) {
            let tgi = Tgi::new(le_u32(chunk, 0), le_u32(chunk, 4), le_u32(chunk, 8));
            let (instance2, size) = if rec == 20 {
                (Some(le_u32(chunk, 12)), le_u32(chunk, 16))
            } else {
                (None, le_u32(chunk, 12))
            };
            self.dir.insert((tgi, instance2.unwrap_or(0)), size);
            self.dir_entries.push(DirEntry {
                tgi,
                instance2,
                uncompressed_size: size,
            });
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Index entries in file order (including the DIR record itself).
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// The DIR records in file order.
    pub fn dir_entries(&self) -> &[DirEntry] {
        &self.dir_entries
    }

    /// 20 for index 7.0, 24 for 7.1.
    pub fn index_entry_size(&self) -> usize {
        self.entry_size
    }

    /// Notes about quirks found while opening (see the module documentation).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// The last index entry with this key.
    pub fn find(&self, tgi: &Tgi) -> Option<&IndexEntry> {
        self.entries.iter().rev().find(|e| e.tgi == *tgi)
    }

    /// Every index entry with this key, in file order.
    pub fn find_all<'a>(&'a self, tgi: &'a Tgi) -> impl Iterator<Item = &'a IndexEntry> + 'a {
        self.entries.iter().filter(move |e| e.tgi == *tgi)
    }

    /// Entries of one type.
    pub fn entries_of_type(&self, type_id: u32) -> impl Iterator<Item = &IndexEntry> + '_ {
        self.entries.iter().filter(move |e| e.tgi.type_id == type_id)
    }

    /// The uncompressed size the DIR records for this entry, if the DIR lists it as compressed.
    pub fn dir_size(&self, e: &IndexEntry) -> Option<u32> {
        self.dir.get(&(e.tgi, e.instance2.unwrap_or(0))).copied()
    }

    /// True if the DIR lists this entry as compressed.
    pub fn is_compressed(&self, e: &IndexEntry) -> bool {
        self.dir_size(e).is_some()
    }

    /// The stored bytes of an entry, exactly as in the file.
    pub fn read_raw(&self, e: &IndexEntry) -> Result<Vec<u8>> {
        let end = e.offset as u64 + e.size as u64;
        if end > self.src.len() {
            return Err(Error::OutOfBounds {
                what: "DBPF entry",
                offset: e.offset as u64,
                size: e.size as u64,
                limit: self.src.len(),
            });
        }
        let mut buf = vec![0u8; e.size as usize];
        self.src.read_at(e.offset as u64, &mut buf)?;
        Ok(buf)
    }

    /// The contents of an entry, decompressed if the DIR lists it and it carries a QFS header.
    pub fn read(&self, e: &IndexEntry) -> Result<Vec<u8>> {
        let raw = self.read_raw(e)?;
        if self.is_compressed(e) && qfs::has_header(&raw) {
            qfs::decompress(&raw)
        } else {
            Ok(raw)
        }
    }

    /// The contents of the last entry with this key.
    pub fn read_tgi(&self, tgi: &Tgi) -> Result<Vec<u8>> {
        let e = self.find(tgi).ok_or(Error::NotFound(*tgi))?;
        self.read(e)
    }
}

/// How [`PackageBuilder`] stores an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    /// Store as given.
    None,
    /// Always QFS-compress.
    Always,
    /// The original writer's rule (foundations §3.1): compress when the entry is 50 ..= 16,000,000 bytes and
    /// the result is below 80 % of its size.
    Auto,
}

/// Writes DBPF 1.0 packages with index 7.0 and a DIR record. Used for test fixtures and, later, for saves.
#[derive(Default)]
pub struct PackageBuilder {
    entries: Vec<(Tgi, Vec<u8>, Option<u32>)>,
    /// Header date fields (seconds; 0 by default).
    pub created: u32,
    pub modified: u32,
}

impl PackageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry. Fails only if compression was requested for data over 16 MiB.
    pub fn add(&mut self, tgi: Tgi, data: &[u8], compression: Compression) -> Result<&mut Self> {
        let (stored, size) = match compression {
            Compression::None => (data.to_vec(), None),
            Compression::Always => (qfs::compress(data)?, Some(data.len() as u32)),
            Compression::Auto => {
                if (50..=16_000_000).contains(&data.len()) {
                    let c = qfs::compress(data)?;
                    if (c.len() as u64) * 5 < (data.len() as u64) * 4 {
                        (c, Some(data.len() as u32))
                    } else {
                        (data.to_vec(), None)
                    }
                } else {
                    (data.to_vec(), None)
                }
            }
        };
        self.entries.push((tgi, stored, size));
        Ok(self)
    }

    /// Adds an entry whose bytes are already in their stored form; `dir_size` lists it in the DIR.
    pub fn add_stored(&mut self, tgi: Tgi, stored: Vec<u8>, dir_size: Option<u32>) -> &mut Self {
        self.entries.push((tgi, stored, dir_size));
        self
    }

    /// Serialises the package: header, entry data, the DIR record (if any entry is compressed), the index.
    pub fn build(&self) -> Result<Vec<u8>> {
        let mut out = vec![0u8; HEADER_LEN];
        let mut index: Vec<(Tgi, u32, u32)> = Vec::new();
        let mut dir = Vec::new();
        let pos = |out: &Vec<u8>| u32::try_from(out.len()).map_err(|_| corrupt("DBPF build", "package exceeds 4 GiB"));
        for (tgi, data, size) in &self.entries {
            let off = pos(&out)?;
            out.extend_from_slice(data);
            index.push((*tgi, off, data.len() as u32));
            if let Some(s) = size {
                for w in [tgi.type_id, tgi.group, tgi.instance, *s] {
                    dir.extend_from_slice(&w.to_le_bytes());
                }
            }
        }
        if !dir.is_empty() {
            let off = pos(&out)?;
            out.extend_from_slice(&dir);
            index.push((DIR_TGI, off, dir.len() as u32));
        }
        let index_offset = pos(&out)?;
        for (tgi, off, size) in &index {
            for w in [tgi.type_id, tgi.group, tgi.instance, *off, *size] {
                out.extend_from_slice(&w.to_le_bytes());
            }
        }
        let header = Header {
            major: 1,
            minor: 0,
            created: self.created,
            modified: self.modified,
            index_major: 7,
            index_count: index.len() as u32,
            index_offset,
            index_size: (index.len() * 20) as u32,
            ..Header::default()
        };
        header.write(&mut out[..HEADER_LEN]);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(i: u32) -> Tgi {
        Tgi::new(0x1111_1111, 0x2222_2222, i)
    }

    #[test]
    fn build_and_read_back() {
        let big: Vec<u8> = b"hello world ".iter().copied().cycle().take(5000).collect();
        let mut b = PackageBuilder::new();
        b.add(t(1), b"short", Compression::Auto).unwrap();
        b.add(t(2), &big, Compression::Auto).unwrap();
        b.add(t(3), &big, Compression::None).unwrap();
        let bytes = b.build().unwrap();
        let p = Package::from_bytes(&bytes[..]).unwrap();
        assert_eq!(p.header().major, 1);
        assert_eq!(p.index_entry_size(), 20);
        assert_eq!(p.entries().len(), 4); // three + DIR
        assert!(p.warnings().is_empty());
        let e1 = *p.find(&t(1)).unwrap();
        let e2 = *p.find(&t(2)).unwrap();
        let e3 = *p.find(&t(3)).unwrap();
        assert!(!p.is_compressed(&e1));
        assert_eq!(p.dir_size(&e2), Some(5000));
        assert!(e2.size < 200);
        assert!(!p.is_compressed(&e3));
        assert_eq!(p.read(&e1).unwrap(), b"short");
        assert_eq!(p.read(&e2).unwrap(), big);
        assert_eq!(p.read(&e3).unwrap(), big);
        assert!(qfs::has_header(&p.read_raw(&e2).unwrap()));
        assert!(matches!(p.read_tgi(&t(9)), Err(Error::NotFound(_))));
    }

    #[test]
    fn duplicates_last_wins() {
        let mut b = PackageBuilder::new();
        b.add(t(1), b"first", Compression::None).unwrap();
        b.add(t(1), b"second", Compression::None).unwrap();
        let bytes = b.build().unwrap();
        let p = Package::from_bytes(bytes).unwrap();
        assert_eq!(p.read_tgi(&t(1)).unwrap(), b"second");
        assert_eq!(p.find_all(&t(1)).count(), 2);
    }

    #[test]
    fn dir_listed_but_not_compressed_is_returned_raw() {
        let mut b = PackageBuilder::new();
        b.add_stored(t(1), b"plain bytes".to_vec(), Some(11));
        // compressed bytes that the DIR does not list stay compressed
        let c = qfs::compress(&[7u8; 300]).unwrap();
        b.add_stored(t(2), c.clone(), None);
        let p = Package::from_bytes(b.build().unwrap()).unwrap();
        assert_eq!(p.read_tgi(&t(1)).unwrap(), b"plain bytes");
        assert_eq!(p.read_tgi(&t(2)).unwrap(), c);
    }

    fn patch(bytes: &mut [u8], off: usize, v: u32) {
        bytes[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn index_7_1() {
        // Hand-build a package with 24-byte index entries and a 20-byte DIR record.
        let data = qfs::compress(&[5u8; 400]).unwrap();
        let mut out = vec![0u8; HEADER_LEN];
        let off = out.len() as u32;
        out.extend_from_slice(&data);
        let dir_off = out.len() as u32;
        for w in [1u32, 2, 3, 77, 400] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        let idx = out.len() as u32;
        for w in [1u32, 2, 3, 77, off, data.len() as u32] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        for w in [DIR_TGI.type_id, DIR_TGI.group, DIR_TGI.instance, 0, dir_off, 20] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        let h = Header {
            major: 1,
            minor: 1,
            index_major: 7,
            index_count: 2,
            index_offset: idx,
            index_size: 48,
            index_minor: 1,
            ..Header::default()
        };
        h.write(&mut out[..HEADER_LEN]);
        let p = Package::from_bytes(out).unwrap();
        assert_eq!(p.index_entry_size(), 24);
        let e = *p.find(&Tgi::new(1, 2, 3)).unwrap();
        assert_eq!(e.instance2, Some(77));
        assert_eq!(p.read(&e).unwrap(), vec![5u8; 400]);
    }

    #[test]
    fn bad_inputs_are_errors() {
        assert!(matches!(Package::from_bytes(&b"DBP"[..]), Err(Error::BadMagic { .. })));
        assert!(matches!(
            Package::from_bytes(&b"DBPF\x01\0\0\0"[..]),
            Err(Error::Truncated { .. })
        ));
        let mut b = PackageBuilder::new();
        b.add(t(1), b"abc", Compression::None).unwrap();
        let good = b.build().unwrap();

        let mut v2 = good.clone();
        patch(&mut v2, 4, 2);
        assert!(matches!(Package::from_bytes(v2), Err(Error::Unsupported { .. })));

        let mut bad_index = good.clone();
        patch(&mut bad_index, 40, 0xFFFF_FF00);
        assert!(matches!(Package::from_bytes(bad_index), Err(Error::OutOfBounds { .. })));

        // entry outside the file: open succeeds with a warning, reading it fails
        let mut bad_entry = good.clone();
        let idx = le_u32(&good, 40) as usize;
        patch(&mut bad_entry, idx + 16, 1 << 20);
        let p = Package::from_bytes(bad_entry).unwrap();
        assert_eq!(p.warnings().len(), 1);
        assert!(matches!(p.read(&p.entries()[0]), Err(Error::OutOfBounds { .. })));

        // wrong index size field: falls back to the minor version, with a warning
        let mut bad_size = good.clone();
        patch(&mut bad_size, 44, 7);
        let p = Package::from_bytes(bad_size).unwrap();
        assert_eq!(p.read_tgi(&t(1)).unwrap(), b"abc");
        assert_eq!(p.warnings().len(), 1);

        // truncating a valid package anywhere never panics
        for n in 0..good.len() {
            if let Ok(p) = Package::from_bytes(&good[..n]) {
                for e in p.entries() {
                    let _ = p.read(e);
                }
            }
        }
    }

    #[test]
    fn file_source() {
        let mut b = PackageBuilder::new();
        b.add(t(4), &[9u8; 1000], Compression::Auto).unwrap();
        let dir = std::env::temp_dir().join(format!("fc4-formats-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fixture.dat");
        std::fs::write(&path, b.build().unwrap()).unwrap();
        let p = Package::open(&path).unwrap();
        assert_eq!(p.read_tgi(&t(4)).unwrap(), vec![9u8; 1000]);
        drop(p);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
