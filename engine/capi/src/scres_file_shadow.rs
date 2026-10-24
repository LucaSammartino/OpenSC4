// SPDX-License-Identifier: GPL-3.0-or-later
//! Private synchronous replay of the production file loop on native physical reads.
//! Command execution is an observed foreign boundary, not graphics-state parity.
use crate::scres_shadow::ParserBytes;
use fc4_core::sc_resources::{files, lexical, Result};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[repr(C)]
pub struct FileEvent {
    pub kind: u32, // 0=open, 1=true read, 2=false read, 3=command, 4=return, 5=throw
    pub path: ParserBytes,
    pub line: ParserBytes,
    pub physical: u32,
    pub value: u32,    // open bool or inclusive return count
    pub position: i32, // logical file position; never a buffered OS handle offset
}

#[derive(Clone)]
struct Event {
    kind: u32,
    path: String,
    line: Vec<u8>,
    physical: u32,
    value: u32,
    position: i32,
}
#[derive(Default)]
struct Replay {
    events: VecDeque<Event>,
    bad: u32,
}
impl Replay {
    fn take(&mut self, kind: u32, path: &str) -> Option<Event> {
        let e = self.events.pop_front()?;
        if e.kind != kind || e.path != path {
            self.bad += 1;
            None // No seeking/resynchronization: a missing event remains a failure.
        } else {
            Some(e)
        }
    }
}
struct Source(Rc<RefCell<Replay>>);
struct Lines {
    replay: Rc<RefCell<Replay>>,
    path: String,
    physical: u32,
    position: i32,
}
impl files::Lines for Lines {
    fn read_line(&mut self) -> Option<Vec<u8>> {
        let mut r = self.replay.borrow_mut();
        let Some(e) = r.events.pop_front() else {
            r.bad += 1;
            return None;
        };
        let success = e.kind == 1;
        if success {
            self.physical += 1;
        }
        // The captured ReadLine bytes/bool/position are the foreign input.
        // Check association and physical numbering independently of routing.
        if !matches!(e.kind, 1 | 2)
            || e.path != self.path
            || e.physical != self.physical
            || e.position < self.position
            || (!success && !e.line.is_empty())
        {
            r.bad += 1;
        }
        self.position = e.position;
        success.then_some(e.line)
    }
}
impl files::Files for Source {
    fn open(&mut self, path: &str) -> Option<Box<dyn files::Lines>> {
        let mut r = self.0.borrow_mut();
        let Some(e) = r.take(0, path) else {
            r.bad += 1;
            return None;
        };
        (e.value != 0).then(|| {
            Box::new(Lines {
                replay: self.0.clone(),
                path: path.into(),
                physical: 0,
                position: e.position,
            }) as Box<dyn files::Lines>
        })
    }
    fn watch(&mut self, _: &str, _: u32, _: u32) -> Option<Box<dyn files::Watcher>> {
        None
    }
    fn finished(&mut self, path: &str, physical: u32, result: &Result<u32>) {
        let mut r = self.0.borrow_mut();
        let Some(e) = r.take(if result.is_ok() { 4 } else { 5 }, path) else {
            r.bad += 1;
            return;
        };
        if e.physical != physical || result.as_ref().is_ok_and(|count| *count != e.value) {
            r.bad += 1;
        }
    }
}
fn compare(events: VecDeque<Event>) -> u32 {
    let Some(root) = events.front().map(|e| e.path.clone()) else {
        return 1;
    };
    let replay = Rc::new(RefCell::new(Replay {
        events,
        ..Replay::default()
    }));
    let mut source = Source(replay.clone());
    let _result = files::parse_file_with(&mut source, &root, &mut |path, physical, line, _| {
        let mut r = replay.borrow_mut();
        let Some(e) = r.take(3, path) else {
            r.bad += 1;
            return Ok(());
        };
        // The native parser's input is a C string after comment removal.
        let line = &line[..line.iter().position(|b| matches!(b, 0 | b'#')).unwrap_or(line.len())];
        if e.physical != physical || lexical::split(line) != lexical::split(&e.line) {
            r.bad += 1;
        }
        Ok(())
    });
    let r = replay.borrow();
    r.bad + u32::from(!r.events.is_empty())
}

/// Private static-link transport; not a public C ABI export.
/// # Safety
/// All event arrays and spans must remain readable until this call returns.
#[no_mangle]
pub unsafe extern "C" fn fc4_scres_file_compare(events: *const FileEvent, count: usize) -> u32 {
    unsafe fn bytes<'a>(b: &ParserBytes) -> Option<&'a [u8]> {
        if b.length > 1024 * 1024 || (b.length != 0 && b.bytes.is_null()) {
            None
        } else if b.length == 0 {
            Some(&[])
        } else {
            Some(std::slice::from_raw_parts(b.bytes, b.length))
        }
    }
    if events.is_null() || count == 0 || count > 100_000 {
        return 1;
    }
    let mut copies = VecDeque::new();
    for e in std::slice::from_raw_parts(events, count) {
        let (Some(path), Some(line)) = (bytes(&e.path), bytes(&e.line)) else {
            return 1;
        };
        let Ok(path) = std::str::from_utf8(path) else { return 1 };
        copies.push_back(Event {
            kind: e.kind,
            path: path.into(),
            line: line.into(),
            physical: e.physical,
            value: e.value,
            position: e.position,
        });
    }
    compare(copies)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(kind: u32, path: &str, line: &[u8], physical: u32, value: u32) -> Event {
        Event {
            kind,
            path: path.into(),
            line: line.into(),
            physical,
            value,
            position: physical as i32,
        }
    }
    fn tree() -> VecDeque<Event> {
        [
            event(0, "d/a", b"", 0, 1),
            event(1, "d/a", b"# comment", 1, 0),
            event(3, "d/a", b"", 1, 0),
            event(1, "d/a", b"include b", 2, 0),
            event(0, "d/b", b"", 0, 1),
            event(1, "d/b", b"", 1, 0),
            event(3, "d/b", b"", 1, 0),
            event(2, "d/b", b"", 1, 0),
            event(4, "d/b", b"", 1, 1),
            event(1, "d/a", b"sinclude missing", 3, 0),
            event(0, "d/missing", b"", 0, 0),
            event(5, "d/missing", b"", 0, 0),
            event(2, "d/a", b"", 3, 0),
            event(4, "d/a", b"", 3, 4),
        ]
        .into()
    }
    #[test]
    fn physical_blank_nested_optional_failure_and_totals() {
        assert_eq!(compare(tree()), 0);
    }
    #[test]
    fn wrong_physical_position_total_and_missing_read_never_resync() {
        for index in [1, 4, 8, 13] {
            let mut t = tree();
            if index == 8 || index == 13 {
                t[index].value += 1;
            } else {
                t[index].physical += 1;
            }
            // Open physical is not used; corrupt its path instead.
            if index == 4 {
                t[index].path.push('x');
            }
            assert_ne!(compare(t), 0, "index {index}");
        }
        let mut t = tree();
        t.remove(7);
        assert_ne!(compare(t), 0);
    }

    #[test]
    fn raw_preprocessing_suppresses_without_using_native_dispatch_decisions() {
        let mut t = tree();
        t[1].line = b"#< include must-not-be-opened".to_vec();
        t.remove(2); // Native supplied no command or include boundary here.
        t[2].line = b"#>include b".to_vec();
        assert_eq!(compare(t.clone()), 0);
        t.back_mut().unwrap().value += 1;
        assert_ne!(compare(t), 0); // Inclusive totals still compared.
    }
}
