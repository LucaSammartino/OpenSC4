// SPDX-License-Identifier: GPL-3.0-or-later
//! SC4CreateSavePathForCity and filename sanitation, application.md §11.16.
use super::compatibility::append_path;
use crate::gz_messages::Token;

pub trait SavePathHost {
    /// Selected region directory/name resolved through foreign region interfaces.
    fn save_directory(&mut self, city: Token) -> Option<Vec<u8>>;
    fn city_name(&mut self, city: Token) -> Vec<u8>;
    fn city_id(&mut self, city: Token) -> u32;
    fn filename_system_roundtrip(&mut self, bytes: &[u8]) -> Vec<u8>;
    fn filename_encoding_valid(&mut self, bytes: &[u8]) -> bool;
    fn file_exists(&mut self, path: &[u8]) -> bool;
    /// None means the regional record could not be read, not city ID zero.
    fn existing_city_id(&mut self, path: &[u8]) -> Option<u32>;
}

pub fn sanitize_filename(input: &[u8], direct_string: bool, host: &mut (impl SavePathHost + ?Sized)) -> Vec<u8> {
    let mut filtered = if direct_string {
        input.to_vec()
    } else {
        host.filename_system_roundtrip(input)
    };
    filtered.retain(|b| !b"<>:\\/\"|*?".contains(b));
    if filtered.len() > 256 {
        filtered.truncate(256);
        while !filtered.is_empty() && !host.filename_encoding_valid(&filtered) {
            filtered.pop();
        }
    }
    let mut result = Vec::new();
    for byte in filtered {
        if byte != b' ' || result.last() != Some(&b' ') {
            result.push(byte);
        }
    }
    let start = result.iter().position(|b| *b != b' ').unwrap_or(result.len());
    let end = result.iter().rposition(|b| *b != b' ').map_or(start, |i| i + 1);
    result[start..end].to_vec()
}

pub fn create_save_path(base: &[u8], name: &[u8], id: u32, host: &mut (impl SavePathHost + ?Sized)) -> Vec<u8> {
    let original = sanitize_filename(&[b"City - ".as_slice(), name, b".sc4"].concat(), false, host);
    let mut candidate = original.clone();
    let mut counter = 1u32;
    loop {
        // Literal resolution beneath the supplied region base; the existing
        // AP7 append-separator boundary remains in compatibility::append_path.
        let path = append_path(base, &candidate);
        if !host.file_exists(&path)
            || (id != 0 && host.existing_city_id(&path).is_some_and(|old| old == id || old == 0))
        {
            return path;
        }
        counter = counter.wrapping_add(1);
        let dot = original.iter().rposition(|b| *b == b'.').unwrap_or(original.len());
        candidate = [&original[..dot], format!(" ({counter})").as_bytes(), &original[dot..]].concat();
    }
}
