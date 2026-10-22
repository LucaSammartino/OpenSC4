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

/// Primitive thread-CRT classifier. The argument is the sign-extended input byte (PA4).
/// An adapter must report unsupported locale/input domains instead of substituting Unicode.
pub type CrtIsAlpha = fn(i32) -> Result<bool, UnsupportedLocaleByte>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedLocaleByte(pub i32);

/// Only the invariant basic-character/EOF domain is supplied portably. Other signed bytes need
/// an actual CRT adapter; even a chosen single-byte encoding does not establish their behavior.
pub fn invariant_crt_is_alpha(byte: i32) -> Result<bool, UnsupportedLocaleByte> {
    match byte {
        -1 | 0..=127 => Ok((b'A' as i32..=b'Z' as i32).contains(&byte)
            || (b'a' as i32..=b'z' as i32).contains(&byte)),
        _ => Err(UnsupportedLocaleByte(byte)),
    }
}

pub fn is_full_path(text: &[u8]) -> Result<bool, UnsupportedLocaleByte> {
    is_full_path_with_classifier(text, invariant_crt_is_alpha)
}

pub fn is_full_path_with_classifier(text: &[u8], classifier: CrtIsAlpha) -> Result<bool, UnsupportedLocaleByte> {
    let text = c_text(text);
    let alpha = classifier(i32::from(text.first().copied().unwrap_or(0) as i8))?;
    let rooted = text.starts_with(b"\\\\") || (text.len() >= 3 && alpha && text[1..3] == *b":\\");
    Ok(rooted && !text.windows(2).any(|p| is_path_separator(p[0]) && p[1] == b'.'))
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

pub fn fullpath_current(rel: Option<&[u8]>, max: usize, provider: &mut impl CurrentDirectory) -> Option<Vec<u8>> {
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

/// The non-null-output form, with both the caller's `max` and the physical slice bound enforced.
pub fn fullpath_into(out: &mut [u8], rel: Option<&[u8]>, max: usize, base: &[u8]) -> bool {
    let text = fullpath_with_base(rel, max, base);
    copy_c_result(out, text.as_deref())
}

/// The null-output form: a fresh allocation with the terminating NUL, subject to `max` (§1.2).
pub fn fullpath_alloc(rel: Option<&[u8]>, max: usize, base: &[u8]) -> Option<Vec<u8>> {
    let mut text = fullpath_with_base(rel, max, base)?;
    text.push(0);
    Some(text)
}

pub fn resolve_path(
    out: &mut Vec<u8>,
    input: &[u8],
    base: Option<&[u8]>,
    provider: &mut impl CurrentDirectory,
) -> Result<bool, UnsupportedLocaleByte> {
    resolve_path_with_classifier(out, input, base, provider, invariant_crt_is_alpha)
}

pub fn resolve_path_with_classifier(
    out: &mut Vec<u8>,
    input: &[u8],
    base: Option<&[u8]>,
    provider: &mut impl CurrentDirectory,
    classifier: CrtIsAlpha,
) -> Result<bool, UnsupportedLocaleByte> {
    let result = if is_full_path_with_classifier(input, classifier)? {
        // PA4: classification sees the C-string prefix, assignment copies the counted range.
        Some(input.to_vec())
    } else if let Some(base) = base {
        fullpath_with_base(Some(input), FULLPATH_MAX, base)
    } else {
        fullpath_current(Some(input), FULLPATH_MAX, provider)
    };
    if let Some(result) = result {
        *out = result;
        Ok(true)
    } else {
        Ok(false)
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

/// Optional native output buffers (§1.5). The two-byte drive needs three bytes including NUL;
/// the cRZString overload's four-byte argument slot can supply its first three bytes here.
pub fn splitpath_into(
    path: &[u8],
    drive: Option<&mut [u8; 3]>,
    directory: Option<&mut [u8; 256]>,
    name: Option<&mut [u8; 256]>,
    extension: Option<&mut [u8; 256]>,
) {
    let parts = splitpath(path);
    if let Some(out) = drive {
        copy_c_result(out, Some(&parts.drive));
    }
    if let Some(out) = directory {
        copy_c_result(out, Some(&parts.directory));
    }
    if let Some(out) = name {
        copy_c_result(out, Some(&parts.name));
    }
    if let Some(out) = extension {
        copy_c_result(out, Some(&parts.extension));
    }
}

pub fn fix_drive_string(drive: &mut Vec<u8>) -> Result<(), UnsupportedLocaleByte> {
    fix_drive_string_with_classifier(drive, invariant_crt_is_alpha)
}

pub fn fix_drive_string_with_classifier(
    drive: &mut Vec<u8>,
    classifier: CrtIsAlpha,
) -> Result<(), UnsupportedLocaleByte> {
    if !drive.is_empty() {
        let alpha = classifier(i32::from(drive[0] as i8))?;
        drive.resize(2, 0);
        if !alpha {
            drive[0] = b'C';
        }
        drive[1] = b':';
    }
    Ok(())
}

pub fn fix_directory_string(directory: &mut Vec<u8>) {
    if !directory.is_empty() {
        if !is_path_separator(directory[0]) {
            directory.insert(0, b'\\');
        }
        if !is_path_separator(*directory.last().unwrap()) {
            directory.push(b'\\');
        }
    }
}

pub fn fix_extension_string(extension: &mut Vec<u8>) {
    if !extension.is_empty() && extension[0] != b'.' {
        extension.insert(0, b'.');
    }
}

/// Signed C-style capacity/count. Rejection resets count but keeps earlier writes (PA6).
/// The caller must provide storage for the positive requested capacity.
pub fn split_directory_into(path: &[u8], parts: &mut [[u8; 260]], count: &mut i32) -> bool {
    if *count > 0 && *count as usize > parts.len() {
        *count = 0;
        return false;
    }
    let capacity = *count;
    *count = 0;
    split_directory_emit(path, capacity, |text| {
        copy_c_result(&mut parts[*count as usize], Some(text));
        *count += 1;
    })
    .then_some(())
    .map_or_else(
        || {
            *count = 0;
            false
        },
        |()| true,
    )
}

/// Owned convenience form; only the buffer/count form exposes partial writes on failure.
pub fn split_directory(path: &[u8], capacity: i32) -> Option<Vec<Vec<u8>>> {
    let mut parts = Vec::new();
    split_directory_emit(path, capacity, |text| parts.push(text.to_vec())).then_some(parts)
}

fn split_directory_emit(path: &[u8], capacity: i32, mut emit: impl FnMut(&[u8])) -> bool {
    let path = c_text(path);
    if path.is_empty() || path.len() > i32::MAX as usize {
        return false;
    }
    // PA6: indices are signed and the final prefix copies end, not end+1, bytes.
    // A negative capacity follows the literal signed loop test, like zero: no writes.
    let mut end = path.len() as i32 - 1;
    let mut written = 0;
    while written < capacity && end > 0 {
        let scan_end = if is_path_separator(path[end as usize]) { end - 1 } else { end };
        let separator = if written + 1 < capacity {
            path[..=scan_end as usize].iter().rposition(|&c| is_path_separator(c))
        } else {
            None
        };
        let (start, span) = match separator {
            Some(index) => (index + 1, end as usize - index),
            None => (0, end as usize),
        };
        if span >= 255 {
            return false;
        }
        let mut text = path[start..start + span].to_vec();
        if !text.last().copied().is_some_and(is_path_separator) {
            text.push(b'\\');
        }
        emit(&text);
        written += 1;
        match separator {
            Some(index) => end = index as i32,
            None => break,
        }
    }
    true
}

pub fn split_directory_path(path: &[u8], last: &mut Vec<u8>, parent: &mut Vec<u8>) -> bool {
    let Some(parts) = split_directory(path, 2) else {
        return false;
    };
    // §3.9 [choice]: unwritten output slots are empty instead of uninitialised stack text.
    *last = parts.first().cloned().unwrap_or_default();
    *parent = parts.get(1).cloned().unwrap_or_default();
    true
}
