//! C-string path helpers (paths.md §1). Length limits count bytes, including the output NUL.

pub const FULLPATH_MAX: usize = 520;
const WORK_CAPACITY: usize = 521;

/// C-string entry points stop at the first NUL. Owners retain separately stored string bytes (PA4).
pub fn c_text(text: &[u8]) -> &[u8] {
    &text[..text.iter().position(|&c| c == 0).unwrap_or(text.len())]
}

pub const fn is_path_separator(c: u8) -> bool {
    c == b'\\' || c == b'/'
}

pub const fn is_letter(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

pub fn is_full_path(text: &[u8]) -> bool {
    let text = c_text(text);
    let rooted = text.starts_with(b"\\\\")
        || (text.len() >= 3 && is_letter(text[0]) && text[1..3] == *b":\\");
    rooted && !text.windows(2).any(|p| is_path_separator(p[0]) && p[1] == b'.')
}

/// Owned result text, without a terminator. `None` is the null/failure result (§1.2).
/// A null native output is modelled by returning a fresh Vec; it never aliases the input.
pub fn fullpath_with_base(rel: Option<&[u8]>, max: usize, base: &[u8]) -> Option<Vec<u8>> {
    let rel = c_text(rel?);
    let base = c_text(base);
    if base.len() >= WORK_CAPACITY {
        return None;
    }
    let mut joined = base.to_vec();
    if joined.last().is_some_and(|&c| !is_path_separator(c)) {
        joined.push(b'\\');
    }
    if joined.len().checked_add(rel.len())? >= WORK_CAPACITY {
        return None;
    }
    joined.extend_from_slice(rel);
    let trailing = joined.last().is_some_and(|&c| is_path_separator(c));
    let mut result: Vec<u8> = Vec::with_capacity(WORK_CAPACITY);
    // Boundaries operate on the already copied text, preserving every separator as typed.
    for boundary in joined.iter().copied().map(Some).chain(std::iter::once(None)) {
        if boundary.map_or(true, is_path_separator) {
            let n = result.len();
            if n >= 2 && is_path_separator(result[n - 2]) && result[n - 1] == b'.' {
                result.truncate(n - 2);
            } else if n >= 3 && is_path_separator(result[n - 3]) && result[n - 2..] == *b".." {
                let separator = n - 3;
                if separator > 0 && result[separator - 1] == b':' {
                    return None;
                }
                let parent = result[..separator].iter().rposition(|&c| is_path_separator(c))?;
                // §1.2 [choice]: no previous separator fails instead of touching stack contents.
                result.truncate(parent);
            }
        }
        if let Some(c) = boundary {
            result.push(c);
        }
    }
    if result.last().is_some_and(|&c| is_path_separator(c)) {
        result.pop();
    }
    if result.last() == Some(&b':') {
        result.push(b'\\');
    }
    if trailing {
        result.push(b'\\');
    }
    (result.len().checked_add(1)? <= max).then_some(result)
}

/// Only CRT current-drive/current-directory primitives; never an owner-output callback (§1.3).
pub trait CurrentDirectory {
    /// ASCII drive letter, or none when `_getdrive` reports no current drive.
    fn current_drive(&mut self) -> Option<u8>;
    fn drive_directory(&mut self, drive: u8) -> Option<Vec<u8>>;
}

pub fn fullpath_current(
    rel: Option<&[u8]>,
    max: usize,
    provider: &mut impl CurrentDirectory,
) -> Option<Vec<u8>> {
    let mut rel = c_text(rel?);
    if rel.len() >= 2 && is_path_separator(rel[0]) && is_path_separator(rel[1]) {
        return fullpath_with_base(Some(&rel[2..]), max, b"\\\\");
    }
    let drive = if rel.len() >= 2 && is_letter(rel[0]) && rel[1] == b':' {
        let drive = rel[0].to_ascii_uppercase();
        rel = &rel[2..];
        drive
    } else {
        provider.current_drive().unwrap_or(b'C')
    };
    if rel.first().is_some_and(|&c| is_path_separator(c)) {
        fullpath_with_base(Some(&rel[1..]), max, &[drive, b':', b'\\'])
    } else {
        let base = provider.drive_directory(drive)?;
        fullpath_with_base(Some(rel), max, &base)
    }
}

/// Copy as a C string only on success, preserving the entire destination on failure.
pub fn copy_c_result(out: &mut [u8], text: Option<&[u8]>) -> bool {
    let Some(text) = text else { return false };
    if text.len() >= out.len() {
        return false;
    }
    out[..text.len()].copy_from_slice(text);
    out[text.len()] = 0;
    true
}

pub fn resolve_path(
    out: &mut Vec<u8>,
    input: &[u8],
    base: Option<&[u8]>,
    provider: &mut impl CurrentDirectory,
) -> bool {
    let result = if is_full_path(input) {
        Some(c_text(input).to_vec())
    } else if let Some(base) = base {
        fullpath_with_base(Some(input), FULLPATH_MAX, base)
    } else {
        fullpath_current(Some(input), FULLPATH_MAX, provider)
    };
    if let Some(result) = result {
        *out = result;
        true
    } else {
        false
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathParts {
    pub drive: Vec<u8>,
    pub directory: Vec<u8>,
    pub name: Vec<u8>,
    pub extension: Vec<u8>,
}

pub fn splitpath(path: &[u8]) -> PathParts {
    let mut path = c_text(path);
    let drive = if path.len() >= 2 && path[1] == b':' {
        let drive = path[..2].to_vec();
        path = &path[2..];
        drive
    } else {
        Vec::new()
    };
    let cut = path.iter().rposition(|&c| is_path_separator(c)).map_or(0, |p| p + 1);
    let (directory, file) = path.split_at(cut);
    let dot = file.iter().rposition(|&c| c == b'.').unwrap_or(file.len());
    let bounded = |p: &[u8]| p[..p.len().min(255)].to_vec();
    PathParts {
        drive,
        directory: bounded(directory),
        name: bounded(&file[..dot]),
        extension: bounded(&file[dot..]),
    }
}

/// Reverse component order, with the final available part containing all remaining prefix (§1.6).
/// PA6: a zero-capacity or overlong remainder is rejected without exposing unwritten storage.
pub fn split_directory(path: &[u8], capacity: usize) -> Option<Vec<Vec<u8>>> {
    let path = c_text(path);
    if path.is_empty() || capacity == 0 {
        return None;
    }
    let mut end = path.len();
    let mut parts = Vec::new();
    loop {
        let content_end = if end > 0 && is_path_separator(path[end - 1]) { end - 1 } else { end };
        let separator = path[..content_end].iter().rposition(|&c| is_path_separator(c));
        let start = if parts.len() + 1 == capacity { 0 } else { separator.map_or(0, |p| p + 1) };
        if content_end - start >= 255 {
            return None;
        }
        let mut part = path[start..end].to_vec();
        if separator.is_none() && start == 0 && content_end == end {
            // §1.6's explicitly specified single-component last-byte loss.
            part.pop();
        }
        if !part.last().is_some_and(|&c| is_path_separator(c)) {
            part.push(b'\\');
        }
        parts.push(part);
        if start == 0 {
            return Some(parts);
        }
        end = start;
    }
}

pub fn split_directory_path(path: &[u8], last: &mut Vec<u8>, parent: &mut Vec<u8>) -> bool {
    let Some(mut parts) = split_directory(path, 2) else { return false };
    *last = parts.remove(0);
    // §3.9 [choice]: missing parent is empty instead of uninitialised stack text.
    *parent = parts.pop().unwrap_or_default();
    true
}
