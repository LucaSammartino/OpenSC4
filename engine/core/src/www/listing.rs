//! Destructive FTP listing parser: 00812de0 and its individual helpers.
use super::{Contract, Unsupported};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: Vec<u8>,
    pub kind: u32,
}

fn len(line: &[u8]) -> Contract<usize> {
    line.iter()
        .position(|b| *b == 0)
        .ok_or(Unsupported::NativeInputDomain(
            "listing requires NUL backing",
        ))
}
fn contains(s: &[u8], sub: &[u8]) -> bool {
    s.windows(sub.len()).any(|w| w == sub)
}
fn first_word(line: &mut [u8]) -> Contract<()> {
    let n = len(line)?;
    if let Some(i) = line[..n].iter().position(|b| *b == b' ') {
        line[i] = 0;
    }
    Ok(())
}

/// 00811fc0; returns the borrowed suffix offset, with spaces taking priority over tabs.
pub fn last_field(line: &mut [u8]) -> Contract<usize> {
    let mut n = len(line)?;
    while n > 2 && matches!(line[n - 1], b'\n' | b'\r' | b' ' | b'\t') {
        n -= 1;
        line[n] = 0;
    }
    let delimiter = line[..n]
        .iter()
        .rposition(|b| *b == b' ')
        .or_else(|| line[..n].iter().rposition(|b| *b == b'\t'));
    let mut start = delimiter.map_or(0, |i| i + 1);
    while start < n && matches!(line[start], b' ' | b'\t') {
        start += 1;
    }
    Ok(start)
}
fn fixed(line: &mut [u8], index: usize) -> Contract<()> {
    let b = line
        .get_mut(index)
        .ok_or(Unsupported::NativeInputDomain("fixed-column backing"))?;
    *b = 0;
    Ok(())
}
fn trim_spaces(line: &mut [u8], start: usize, minimum: usize) -> Contract<()> {
    let mut n = len(&line[start..])?;
    while n >= minimum && n > 0 && line[start + n - 1] == b' ' {
        n -= 1;
        line[start + n] = 0;
    }
    Ok(())
}
fn emit(line: &[u8], start: usize, kind: u32, dots: u8, output: &mut Entry) -> Contract<bool> {
    let name = &line[start..start + len(&line[start..])?];
    if kind == 4 && (dots >= 1 && name == b"." || dots >= 2 && name == b"..") {
        return Ok(false);
    }
    *output = Entry {
        name: name.to_vec(),
        kind,
    };
    Ok(true)
}

/// Unsupported input domains are NOT a parser rejection and have no native-output claim.
/// An ordinary false leaves `output` untouched, even after destructive input changes.
pub fn parse(line: &mut [u8], selector: u32, output: &mut Entry) -> Contract<bool> {
    let n = len(line)?;
    match selector {
        301 | 306 | 308 | 317 => {
            let dir = contains(&line[..n], b"<DIR>") || contains(&line[..n], b"<dir>");
            if selector != 317 || dir {
                first_word(line)?;
                if !dir && len(line)? == 0 {
                    return Ok(false);
                }
                return emit(line, 0, if dir { 4 } else { 8 }, 2, output);
            }
            if line.len() < 12 {
                return Err(Unsupported::NativeInputDomain("317 fixed columns"));
            }
            // Never fill native uninitialized holes or invent a full-extension terminator.
            let end = line[..8]
                .iter()
                .rposition(|b| *b != b' ')
                .map_or(0, |i| i + 1);
            let name_end = line[..end].iter().position(|b| *b == 0).unwrap_or(end);
            if line[..name_end].contains(&b' ')
                || !line[9..12].iter().any(|b| matches!(b, 0 | b' '))
            {
                return Err(Unsupported::NativeInputDomain(
                    "317 uninitialized name/extension",
                ));
            }
            let mut name = line[..name_end].to_vec();
            if line[9..12].iter().any(|b| *b != b' ') {
                name.push(b'.');
                let extension_end = line[9..12]
                    .iter()
                    .position(|b| matches!(b, 0 | b' '))
                    .unwrap_or(3);
                name.extend_from_slice(&line[9..9 + extension_end]);
            }
            *output = Entry { name, kind: 8 };
            Ok(true)
        }
        302 => {
            fixed(line, 30)?;
            let start = last_field(line)?;
            let dir = line
                .get(..5)
                .is_some_and(|s| s.eq_ignore_ascii_case(b"<dir>"));
            emit(line, start, if dir { 4 } else { 8 }, 2, output)
        }
        303 => {
            let start = last_field(line)?;
            let n = len(line)?;
            let dir = contains(&line[..n], b" DIR ") || contains(&line[..n], b"<dir>");
            emit(line, start, if dir { 4 } else { 8 }, 2, output)
        }
        304 => {
            fixed(line, 18)?;
            trim_spaces(line, 0, 2)?;
            let n = len(line)?;
            let space =
                line[..n]
                    .iter()
                    .position(|b| *b == b' ')
                    .ok_or(Unsupported::NativeInputDomain(
                        "304 missing space dereference",
                    ))?;
            line[space] = b'.';
            let mut from = space + 1;
            while from < n && line[from] == b' ' {
                from += 1;
            }
            if from == n || matches!(line[from], 0 | b' ' | b'\t') {
                return Ok(false);
            }
            line.copy_within(from..=n, space + 1);
            emit(line, 0, 8, 0, output)
        }
        305 => {
            if contains(&line[..n], b"Disk size") {
                return Ok(false);
            }
            fixed(line, 13)?;
            for (start, dots) in [(0, 1), (41, 2)] {
                if start == 41 {
                    fixed(line, 54)?;
                }
                trim_spaces(line, start, 1)?;
                let n = len(&line[start..])?;
                if n == 0 {
                    continue;
                }
                let dir = matches!(line[start + n - 1], b'/' | b'\\');
                if dir {
                    line[start + n - 1] = 0;
                }
                if emit(line, start, if dir { 4 } else { 8 }, dots, output)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        311 | 312 => {
            if line[0] == b' ' {
                return Ok(false);
            }
            let Some(semi) = line[..n].iter().position(|b| *b == b';') else {
                return Ok(false);
            };
            line[semi] = 0;
            if semi > 4 && line[..semi].ends_with(b".DIR") {
                line[semi - 4] = 0;
                return emit(line, 0, 4, 0, output);
            }
            line[semi] = b';';
            first_word(line)?;
            let n = len(line)?;
            if let Some(tab) = line[..n].iter().position(|b| *b == b'\t') {
                line[tab] = 0;
            }
            emit(line, 0, 8, 0, output)
        }
        313 => {
            first_word(line)?;
            let n = len(line)?;
            if n == 0 {
                return Err(Unsupported::NativeInputDomain("313 empty last-byte read"));
            }
            let dir = matches!(line[n - 1], b'/' | b'\\');
            if dir {
                line[n - 1] = 0;
            }
            emit(line, 0, if dir { 4 } else { 8 }, 2, output)
        }
        314 => {
            let b = line
                .get(8)
                .ok_or(Unsupported::NativeInputDomain("314 byte 8 read"))?;
            if *b == b'-' {
                return parse(line, 310, output);
            }
            let marker = line[..n].windows(5).position(|w| w == b"<DIR>");
            let start = marker.unwrap_or(0);
            let Some(space) = line[start..n].iter().rposition(|b| *b == b' ') else {
                return Ok(false);
            };
            emit(
                line,
                start + space + 1,
                if marker.is_some() { 4 } else { 8 },
                2,
                output,
            )
        }
        315 => {
            let dir = line[..n].windows(5).position(|w| w == b"d    ");
            let found = dir
                .or_else(|| line[..n].windows(5).position(|w| w == b"     "))
                .ok_or(Unsupported::NativeInputDomain(
                    "315 missing fixed-column prefix",
                ))?;
            let start = found + 51;
            if start >= line.len() {
                return Err(Unsupported::NativeInputDomain("315 name backing"));
            }
            emit(line, start, if dir.is_some() { 4 } else { 8 }, 0, output)
        }
        _ => {
            let start = last_field(line)?;
            match line[0] {
                b'd' | b'l' => {
                    let n = len(&line[start..])?;
                    if n > 0 && matches!(line[start + n - 1], b'/' | b'\\') {
                        line[start + n - 1] = 0;
                    }
                    if line[0] == b'l' {
                        let n = len(line)?;
                        let Some(arrow) = line[..n].windows(4).position(|w| w == b" -> ") else {
                            return Ok(false);
                        };
                        line[arrow] = 0;
                        let start = last_field(line)?;
                        emit(line, start, 4, 0, output)
                    } else {
                        emit(line, start, 4, 2, output)
                    }
                }
                b'-' | b'f' => emit(line, start, 8, 0, output),
                _ if selector == 307 => emit(line, start, 8, 0, output),
                _ => Ok(false),
            }
        }
    }
}

/// 00812d60: copies LF too, then appends an explicit NUL in the length-bearing value.
pub fn next_line(source: &[u8], cursor: &mut u32, output: &mut Vec<u8>) -> bool {
    let start = *cursor as usize;
    if start >= source.len() {
        return false;
    }
    let end = source[start..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(source.len(), |n| start + n + 1);
    *output = source[start..end].to_vec();
    output.push(0);
    *cursor = end as u32;
    true
}

/// 00816230 and 00812970 list effects. An O allocation helper is not implemented here;
/// Vec storage represents the published node sequence, not native allocator equivalence.
pub fn replace(source: &[u8], selector: u32, list: &mut Vec<Entry>) -> Contract<bool> {
    list.clear();
    let mut cursor = 0;
    let mut line = Vec::new();
    let mut entry = Entry {
        name: Vec::new(),
        kind: 0,
    };
    while next_line(source, &mut cursor, &mut line) {
        if parse(&mut line, selector, &mut entry)? {
            list.push(entry.clone());
        }
    }
    Ok(true)
}

/// 008126d0: ordered, guarded, case-sensitive substring detector.
pub fn detect(reply: &[u8], selector: &mut u32) -> bool {
    for (text, minimum, value) in [
        (b"Windows_NT".as_slice(), 10, 314),
        (b"WINDOWS NT Type: L8", 19, 315),
        (b"UNIX", 4, 310),
        (b"MultiNet", 8, 311),
        (b"VMS", 0, 312),
        (b"PC Type: L8", 0, 317),
    ] {
        if reply.len() >= minimum && contains(reply, text) {
            *selector = value;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn old() -> Entry {
        Entry {
            name: b"old".to_vec(),
            kind: 99,
        }
    }
    #[test]
    fn constructed_selector_vectors() {
        for (selector, input, kind, name) in [
            (301, b"alpha beta".as_slice(), 8, b"alpha".as_slice()),
            (301, b".. no-marker", 8, b".."),
            (303, b"meta DIR alpha beta", 4, b"beta"),
            (310, b"-rw metadata long name.txt", 8, b"name.txt"),
            (307, b"? metadata name", 8, b"name"),
            (310, b"lrwx metadata link -> dest", 4, b"link"),
            (311, b"ABC.DIR;1 other", 4, b"ABC"),
            (312, b"ABC.dir;1 other", 8, b"ABC.dir;1"),
            (314, b"size last two words.txt", 8, b"words.txt"),
        ] {
            let mut line = input.to_vec();
            line.push(0);
            let mut out = old();
            assert_eq!(parse(&mut line, selector, &mut out), Ok(true), "{selector}");
            assert_eq!(
                out,
                Entry {
                    name: name.to_vec(),
                    kind
                }
            );
        }
    }
    #[test]
    fn rejected_output_and_destructive_input() {
        for (selector, text) in [
            (301, b". <DIR>".as_slice()),
            (310, b"? metadata name"),
            (310, b"lrwx metadata link"),
            (313, b"../ metadata"),
        ] {
            let mut line = text.to_vec();
            line.push(0);
            let mut output = old();
            assert_eq!(parse(&mut line, selector, &mut output), Ok(false));
            assert_eq!(output, old());
            if selector == 301 {
                assert_eq!(line[1], 0);
            }
        }
        let mut line = b"a b\tc\0".to_vec();
        assert_eq!(last_field(&mut line), Ok(2));
        let mut line = b"a \0".to_vec();
        assert_eq!(last_field(&mut line), Ok(2));
        assert_eq!(line[1], b' ');
    }
    #[test]
    fn fixed_column_distinctions_and_unsafe_domains() {
        let mut output = old();
        let mut line = vec![b' '; 55];
        line[..3].copy_from_slice(b"../");
        line[54] = 0;
        assert_eq!(parse(&mut line, 305, &mut output), Ok(true));
        assert_eq!(output.name, b"..");
        let mut line = b"NAME     X  \0".to_vec();
        assert_eq!(parse(&mut line, 317, &mut output), Ok(true));
        assert_eq!(output.name, b"NAME.X");
        for text in [b"N AME    X  \0".as_slice(), b"NAME     XYZ\0"] {
            assert!(parse(&mut text.to_vec(), 317, &mut output).is_err());
        }
        assert!(parse(&mut b"\0".to_vec(), 313, &mut output).is_err());
        assert!(parse(&mut vec![0; 19], 304, &mut output).is_err());
        assert!(parse(&mut b"no prefix\0".to_vec(), 315, &mut output).is_err());
        let mut line = vec![b' '; 60];
        line[0] = b'd';
        line[51..54].copy_from_slice(b"..\0");
        assert_eq!(parse(&mut line, 315, &mut output), Ok(true));
        assert_eq!(output.name, b"..");
    }
    #[test]
    fn format_order_and_line_list_replacement() {
        let mut selector = 312;
        assert!(!detect(b"unix", &mut selector));
        assert_eq!(selector, 312);
        assert!(detect(b"UNIX Windows_NT", &mut selector));
        assert_eq!(selector, 314);
        let mut list = vec![old()];
        assert_eq!(replace(b"? rejected\n", 310, &mut list), Ok(true));
        assert!(list.is_empty());
        assert_eq!(replace(b"- meta a\nd meta b", 310, &mut list), Ok(true));
        assert_eq!(
            list.iter().map(|e| e.name.as_slice()).collect::<Vec<_>>(),
            [b"a", b"b"]
        );
        let mut cursor = 50;
        let mut output = b"unchanged".to_vec();
        assert!(!next_line(b"a", &mut cursor, &mut output));
        assert_eq!(cursor, 50);
        assert_eq!(output, b"unchanged");
        cursor = 0;
        assert!(next_line(b"a\nb", &mut cursor, &mut output));
        assert_eq!(output, b"a\n\0");
    }
}
