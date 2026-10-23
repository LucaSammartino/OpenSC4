// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Result, Unsupported};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};
pub const SERVICE_ID: u32 = 0x09B69C4E;
pub const IID: u32 = 0x09B69C46;
pub const MAX_LEVEL: i32 = i32::MAX;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub category: String,
    pub level: i32,
    pub text: String,
}
pub trait Backend: Send + Sync {
    fn accepts(&self, category: &str, level: i32) -> bool;
    fn output(&self, router: &Log, record: &Record);
    fn set_level(&self, category: Option<&str>, level: i32);
    fn dump(&self, stream: &dyn Stream);
}
pub trait Stream: Send + Sync {
    fn write(&self, text: &str);
}
/// The default backend and stream rendering are foreign output-stream owners.
pub trait BackendFactory {
    fn create(&mut self, name: &str, stream: Option<Arc<dyn Stream>>, timestamp: u8) -> Option<Arc<dyn Backend>>;
}
struct State {
    owner: Option<ThreadId>,
    // Stable borrowed identities must survive moves of the owner/locked State.
    #[allow(clippy::box_collection)]
    backend_names: Option<Box<BTreeSet<String>>>,
    #[allow(clippy::box_collection)]
    observed_categories: Option<Box<BTreeSet<String>>>,
    backends: Vec<(String, Arc<dyn Backend>)>,
    foreign: VecDeque<Record>,
    local: VecDeque<(Record, u32)>,
    depth: u32,
    initialized: bool,
    global_level: i32,
    categories: BTreeMap<String, i32>,
}
pub trait Lifecycle {
    fn remove_tick_registration(&mut self);
    fn delete_output_helpers(&mut self);
}
pub struct Log {
    state: Mutex<State>,
}
impl Default for Log {
    fn default() -> Self {
        Self {
            state: Mutex::new(State {
                owner: None,
                backend_names: Some(Box::default()),
                observed_categories: Some(Box::default()),
                backends: Vec::new(),
                foreign: VecDeque::new(),
                local: VecDeque::new(),
                depth: 0,
                initialized: false,
                global_level: MAX_LEVEL,
                categories: BTreeMap::new(),
            }),
        }
    }
}
impl Log {
    pub fn query_interface(
        &self,
        host: &mut impl super::References,
        this: super::Token,
        base: super::Token,
        iid: u32,
    ) -> Option<super::Token> {
        if iid == IID {
            host.add_ref(this);
            Some(this)
        } else {
            host.query(base, iid)
        }
    }
    pub fn initialized(&self) -> bool {
        self.state.lock().unwrap().initialized
    }
    pub fn default_level(&self) -> i32 {
        self.state.lock().unwrap().global_level
    }
    /// Native Init configuration is GZF7. A host may attach an already
    /// configured tick registration and output helpers, then mark it live.
    pub fn attach_initialized_host(&self) {
        let mut s = self.state.lock().unwrap();
        if !s.initialized {
            // Host fixture supplies the configured helpers on reinitialization;
            // native helper allocation/configuration remains GZF7.
            s.backend_names.get_or_insert_with(Box::default);
            s.observed_categories.get_or_insert_with(Box::default);
            s.owner = Some(thread::current().id());
            s.initialized = true;
        }
    }
    pub fn init(&self) -> Result<()> {
        Err(Unsupported("GZF7"))
    }
    /// Scoped borrowed set access; no cloned set or caller reference is created.
    pub fn backend_names<R>(&self, inspect: impl FnOnce(Option<&BTreeSet<String>>) -> R) -> R {
        inspect(self.state.lock().unwrap().backend_names.as_deref())
    }
    pub fn observed_categories<R>(&self, inspect: impl FnOnce(Option<&BTreeSet<String>>) -> R) -> R {
        inspect(self.state.lock().unwrap().observed_categories.as_deref())
    }
    pub fn add_output_stream(
        &self,
        factory: &mut impl BackendFactory,
        name: &str,
        stream: Option<Arc<dyn Stream>>,
        timestamp: u8,
        output: Option<&mut Option<Arc<dyn Backend>>>,
    ) -> bool {
        let Some(backend) = factory.create(name, stream, timestamp) else {
            return false;
        };
        // A duplicate name does not fail this wrapper or replace its route.
        let _ = self.add_backend(name, backend.clone());
        if let Some(output) = output {
            *output = Some(backend);
        }
        true
    }
    pub fn output_formatted(
        &self,
        formatter: &mut impl super::formatting::Formatter,
        category: Option<&str>,
        format: Option<&[u8]>,
        words: &[u32],
    ) {
        self.output_level_formatted(formatter, category, 0, format, words);
    }
    pub fn output_level_formatted(
        &self,
        formatter: &mut impl super::formatting::Formatter,
        category: Option<&str>,
        level: i32,
        format: Option<&[u8]>,
        words: &[u32],
    ) {
        let (Some(category), Some(format)) = (category, format) else {
            return;
        };
        {
            let mut s = self.state.lock().unwrap();
            if !s.initialized {
                return;
            }
            s.observed_categories
                .as_mut()
                .expect("live category set")
                .insert(category.into());
        }
        let mut buffer = vec![0; 1024];
        let mut status = formatter.format(format, words, &mut buffer);
        if status >= 1024 {
            buffer = vec![0; 1025];
            formatter.format(format, words, &mut buffer);
        } else {
            while status < 0 && buffer.len() < 65536 {
                buffer = vec![0; buffer.len() * 2];
                status = formatter.format(format, words, &mut buffer);
            }
        }
        // Even final overflow submits the last buffer; no invented safe drop.
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        let text = formatter.decode(&buffer[..end]);
        self.output(category, level, &text);
    }
    pub fn dump_output_levels(&self, stream: &dyn Stream, mut describe: impl FnMut(i32) -> String) {
        let s = self.state.lock().unwrap();
        stream.write("All Backends:\n");
        stream.write("-------------\n");
        stream.write(&format!(
            "All unspecified groups showing {}\n",
            describe(s.global_level)
        ));
        for (name, &level) in &s.categories {
            stream.write(&format!("Group '{name}' showing {}\n", describe(level)));
        }
        stream.write("-------------\n");
        for (_, backend) in &s.backends {
            backend.dump(stream);
        }
    }
    pub fn add_backend(&self, name: &str, backend: Arc<dyn Backend>) -> bool {
        let mut s = self.state.lock().unwrap();
        if s.backend_names.as_ref().is_some_and(|names| names.contains(name)) {
            return false;
        }
        s.backend_names.as_mut().expect("live backend set").insert(name.into());
        s.backends.push((name.into(), backend));
        true
    }
    pub fn backend(&self, name: &str) -> Option<Arc<dyn Backend>> {
        self.state
            .lock()
            .unwrap()
            .backends
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.clone())
    }
    pub fn remove_backend(&self, name: &str) -> bool {
        let removed = {
            let mut s = self.state.lock().unwrap();
            let Some(i) = s.backends.iter().position(|(n, _)| n == name) else {
                return false;
            };
            s.backends.remove(i)
        };
        drop(removed);
        true
    }
    pub fn remove_all_backends(&self) {
        let old = std::mem::take(&mut self.state.lock().unwrap().backends);
        drop(old);
    }
    pub fn set_output_level(&self, name: Option<&str>, category: Option<&str>, level: i32) {
        let backends = {
            let mut s = self.state.lock().unwrap();
            if name.is_none() {
                if let Some(c) = category {
                    s.categories.insert(c.into(), level);
                } else {
                    s.global_level = level;
                }
            }
            s.backends
                .iter()
                .filter(|(n, _)| name.map_or(true, |v| n == v))
                .map(|(_, b)| b.clone())
                .collect::<Vec<_>>()
        };
        for b in backends {
            b.set_level(category, level);
        }
    }
    pub fn remembered_category_level(&self, category: &str) -> Option<i32> {
        self.state.lock().unwrap().categories.get(category).copied()
    }
    pub fn output(&self, category: &str, level: i32, text: &str) {
        let record = Record {
            category: category.into(),
            level,
            text: text.into(),
        };
        {
            let mut s = self.state.lock().unwrap();
            if Some(thread::current().id()) != s.owner {
                s.foreign.push_back(record);
                return;
            }
            if s.depth != 0 {
                let depth = s.depth.wrapping_add(1);
                if depth <= 3 {
                    s.local.push_back((record, depth));
                }
                return;
            }
        }
        self.deliver(record, 1);
        loop {
            let next = self.state.lock().unwrap().local.pop_front();
            let Some((record, depth)) = next else { break };
            self.deliver(record, depth);
        }
    }
    fn deliver(&self, record: Record, depth: u32) {
        let backends = {
            let mut s = self.state.lock().unwrap();
            s.depth = depth;
            s.backends.iter().map(|(_, b)| b.clone()).collect::<Vec<_>>()
        };
        // No lock spans either backend call; filters can log, too.
        for b in backends {
            if b.accepts(&record.category, record.level) {
                b.output(self, &record);
            }
        }
        self.state.lock().unwrap().depth = 0;
    }
    pub fn on_tick(&self) -> bool {
        // Only the owner may drain, otherwise records would enqueue forever.
        if Some(thread::current().id()) != self.state.lock().unwrap().owner {
            return true;
        }
        loop {
            let next = self.state.lock().unwrap().foreign.pop_front();
            let Some(r) = next else { break };
            self.output(&r.category, r.level, &r.text);
        }
        true
    }
    pub fn shutdown(&self, host: &mut impl Lifecycle) -> bool {
        {
            let mut s = self.state.lock().unwrap();
            if !s.initialized {
                return true;
            }
            s.initialized = false;
            s.backend_names = None;
            s.observed_categories = None;
        }
        host.remove_tick_registration();
        self.remove_all_backends();
        host.delete_output_helpers();
        true
    }
}
pub fn filter_level_from_string(value: Option<&str>) -> Result<i32> {
    parse(value, false)
}
pub fn log_level_from_string(value: Option<&str>) -> Result<i32> {
    parse(value, true)
}
fn parse(value: Option<&str>, log: bool) -> Result<i32> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Ok(-1);
    };
    if value.eq_ignore_ascii_case("all") {
        Ok(if log { MAX_LEVEL } else { 0 })
    } else if value.eq_ignore_ascii_case("none") {
        Ok(if log { 0 } else { MAX_LEVEL })
    } else {
        // Until the complete table is specified we cannot distinguish an
        // unknown name (native -1) from a valid, unrecovered named level.
        Err(Unsupported("GZF7"))
    }
}
pub fn string_from_filter_level(value: i32) -> Result<&'static str> {
    match value {
        0 => Ok("all"),
        MAX_LEVEL => Ok("none"),
        -1 => Ok("invalid filter level"),
        _ => Err(Unsupported("GZF7")),
    }
}
pub fn string_from_log_level(value: i32) -> Result<&'static str> {
    match value {
        0 => Ok("none"),
        MAX_LEVEL => Ok("all"),
        -1 => Ok("invalid filter level"),
        _ => Err(Unsupported("GZF7")),
    }
}
