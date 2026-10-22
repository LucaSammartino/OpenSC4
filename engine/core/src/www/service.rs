//! WWW front end. Only foreign cache/cookie/file/handler primitives are supplied by
//! Host; admission, conversion, cache policy, fanout and reference order live here.
use super::{directory, http, native_path, path_preference, Contract, Unsupported};
use crate::rzurl::{url_type, RzUrl};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Job {
    pub id: u32,
    pub kind: u32,
    pub foreign: u32,
    pub url: Vec<u8>,
    pub data: Vec<u8>,
    pub file: bool,
    pub preference: i32,
    pub target: Option<u32>,
    pub custom: u32,
    pub time: u32,
    pub result: u32,
    pub progress: u32,
    pub foreign_pointer: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Lock,
    Unlock,
    AddTarget(u32),
    ReleaseTarget(u32),
    AddService,
    CreateMessage(u32),
    AddMessage,
    ReleaseMessage,
    ReleaseCacheData,
    AddHandler(u32),
    ReleaseHandler(u32),
    Component(&'static str, &'static str),
    CookieFilename(Vec<u8>),
    HttpDirectory(Vec<u8>),
    HttpCacheSetting(Vec<u8>),
    FtpDirectory(Vec<u8>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notification {
    Terminal(bool),
    Progress([u32; 3]),
}
pub enum Incoming<'a> {
    HttpTerminal {
        foreign: Option<u32>,
        success: bool,
        resource: Option<&'a mut http::Resource>,
    },
    FtpTerminal {
        foreign: u32,
        success: bool,
        filename: &'a [u8],
    },
    HttpProgress {
        foreign: Option<u32>,
        words: Option<[u32; 3]>,
    },
    FtpProgress {
        foreign: Option<u32>,
        words: Option<[u32; 3]>,
    },
    Unknown,
}
pub trait Host: http::Host {
    fn effect(&mut self, effect: Effect);
    fn now(&mut self) -> u32;
    fn file_exists(&mut self, path: &[u8]) -> bool;
    /// Foreign GetPathName operation, not an invented second path algorithm.
    fn path_name(&mut self, path: &[u8]) -> Vec<u8>;
    fn file_open(&mut self, path: &[u8], write: bool) -> Option<u32>;
    fn file_size(&mut self, file: u32) -> u32;
    fn file_read(&mut self, file: u32, size: u32) -> Option<Vec<u8>>;
    fn file_write(&mut self, file: u32, bytes: &[u8]) -> bool;
    fn file_close(&mut self, file: u32);
    fn cache_present(&mut self, url: &[u8]) -> bool;
    fn cache_size(&mut self, url: &[u8]) -> u32;
    fn cache_data(&mut self, url: &[u8]) -> Option<Vec<u8>>;
    fn cache_file(&mut self, url: &[u8]) -> Option<Vec<u8>>;
    fn new_cache_file(&mut self) -> Vec<u8>;
    fn insert_cache(
        &mut self,
        url: &[u8],
        data: &[u8],
        file: bool,
        flags: (u32, u32),
        expiry: u32,
    ) -> bool;
    fn cache_directory(&mut self) -> Vec<u8>;
    fn set_cache_directory(&mut self, directory: &[u8]) -> bool;
    fn header(&mut self, foreign: u32, family: u32, index: u32) -> Option<Vec<u8>>;
    fn parse_time(&mut self, text: &[u8]) -> u32;
    fn request_cookies(&mut self, url: &[u8]) -> Option<Vec<u8>>;
    fn response_cookie(&mut self, url: &[u8], text: &[u8]);
    fn handler_init(&mut self, id: u32) -> bool;
    fn handler_shutdown(&mut self, id: u32);
    fn handler_accepts(&mut self, id: u32, url: &mut RzUrl) -> bool;
    fn handler_load(&mut self, id: u32, job: &mut Job) -> bool;
    /// Called outside the WWW lock. Job/progress are borrowed for this call only.
    fn notify(&mut self, service: &mut Www, job: &Job, notification: Notification);
}
fn effect(host: &mut impl Host, value: Effect) {
    Host::effect(host, value);
}

/// Defaults of the published constructor, on supplied existing components. The
/// complete constructor cannot run: its cookie constructor is P:008f9bad/Q9.
#[derive(Clone, Debug)]
pub struct Www {
    pub initialized: bool,
    pub next_job: u32,
    pub cookies: u8,
    pub cache: u8,
    pub default_custom: u32,
    pub default_target: Option<u32>,
    pub handlers: Vec<u32>,
    pub active: Vec<Job>,
    pub waiting: Vec<Job>,
    pub http: http::Client,
}
impl Www {
    pub fn snapshot(http: http::Client) -> Self {
        Self {
            initialized: false,
            next_job: 1,
            cookies: 1,
            cache: 0,
            default_custom: 0,
            default_target: None,
            handlers: Vec::new(),
            active: Vec::new(),
            waiting: Vec::new(),
            http,
        }
    }
    pub fn construct() -> Contract<Self> {
        Err(Unsupported::OwnedBody("008f9bad cookie constructor, P/Q9"))
    }
    /// 008f687d unsupported QI leaves output untouched and emits no hold.
    pub fn query_interface(&mut self, host: &mut impl Host, id: u32, output: &mut u32) -> bool {
        let view = match id {
            1 | 0x449b1912 => 4,
            0x287fb697 => 8,
            0x452294aa => 0,
            _ => return false,
        };
        effect(host, Effect::AddService);
        *output = view;
        true
    }
    pub fn init(&mut self) -> bool {
        self.initialized = true;
        true
    }
    pub fn post_init(&mut self, host: &mut impl Host) -> bool {
        self.init();
        for name in ["cache", "cookie", "http", "ftp"] {
            effect(host, Effect::Component(name, "init"));
        }
        true
    }
    pub fn set_default_target(&mut self, host: &mut impl Host, target: Option<u32>) {
        if let Some(id) = target {
            effect(host, Effect::AddTarget(id));
        }
        if let Some(id) = self.default_target {
            effect(host, Effect::ReleaseTarget(id));
        }
        self.default_target = target;
    }
    pub fn add_handler(&mut self, host: &mut impl Host, id: u32) -> bool {
        effect(host, Effect::AddHandler(id));
        if !host.handler_init(id) {
            effect(host, Effect::ReleaseHandler(id));
            return false;
        }
        self.handlers.push(id);
        true
    }
    pub fn remove_handler(&mut self, host: &mut impl Host, id: u32) {
        if let Some(position) = self.handlers.iter().position(|h| *h == id) {
            self.handlers.remove(position);
            host.handler_shutdown(id);
            effect(host, Effect::ReleaseHandler(id));
        }
    }
    pub fn remove_all_handlers(&mut self, host: &mut impl Host) {
        for id in self.handlers.drain(..) {
            host.handler_shutdown(id);
            effect(host, Effect::ReleaseHandler(id));
        }
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        if self.initialized {
            self.cancel_all(host);
            self.remove_all_handlers(host);
            for (name, action) in [
                ("http", "disconnect-target"),
                ("ftp", "disconnect-target"),
                ("cache", "shutdown"),
                ("cookie", "shutdown"),
            ] {
                effect(host, Effect::Component(name, action));
            }
            self.initialized = false;
        }
        true
    }
    pub fn destroy(&mut self, host: &mut impl Host) {
        self.shutdown(host);
        for name in ["cache", "cookie", "http", "ftp"] {
            effect(host, Effect::Component(name, "release"));
        }
        if let Some(target) = self.default_target.take() {
            effect(host, Effect::ReleaseTarget(target));
        }
    }
    fn allocate(&mut self) -> u32 {
        self.next_job = self.next_job.wrapping_add(1);
        self.next_job
    }
    fn envelope(&mut self, host: &mut impl Host, job: &Job, notification: Notification) {
        if job.target.is_none() {
            return;
        }
        effect(
            host,
            Effect::CreateMessage(match notification {
                Notification::Terminal(_) => 0x75758483,
                Notification::Progress(_) => 0x75758484,
            }),
        );
        effect(host, Effect::AddMessage);
        Host::notify(host, self, job, notification);
        effect(host, Effect::ReleaseMessage);
    }
    fn deliver(&mut self, host: &mut impl Host, job: &mut Job) {
        self.envelope(host, job, Notification::Terminal(job.result != 0));
        if let Some(target) = job.target.take() {
            effect(host, Effect::ReleaseTarget(target));
        }
    }
    /// Public overload 008f72b5 reuses cRZURL, never a second URL parser.
    pub fn load_text(
        &mut self,
        host: &mut impl Host,
        text: &[u8],
        preference: i32,
        target: Option<u32>,
        custom: u32,
        allow_cache: bool,
    ) -> Contract<u32> {
        self.load(
            host,
            &mut RzUrl::from_text(text),
            preference,
            target,
            custom,
            allow_cache,
        )
    }
    /// 008f833d: selected target retained before any handler or built-in dispatch.
    pub fn load(
        &mut self,
        host: &mut impl Host,
        url: &mut RzUrl,
        mut preference: i32,
        target: Option<u32>,
        custom: u32,
        allow_cache: bool,
    ) -> Contract<u32> {
        let Some(target) = target.or(self.default_target) else {
            return Ok(0);
        };
        effect(host, Effect::AddTarget(target));
        let custom = if custom == 0 {
            self.default_custom
        } else {
            custom
        };
        if preference == 4 {
            preference = path_preference(url.absolute_path());
        }
        let mut job = Job {
            url: url.full_url().to_vec(),
            preference,
            target: Some(target),
            custom,
            time: host.now(),
            ..Job::default()
        };
        for handler in self.handlers.clone() {
            if host.handler_accepts(handler, url) {
                job.id = self.allocate();
                job.result = u32::from(host.handler_load(handler, &mut job));
                job.progress = if job.result != 0 { 100 } else { 0 };
                self.deliver(host, &mut job);
                return Ok(job.id);
            }
        }
        match url.url_type() {
            url_type::HTTP => self.admit_http(host, job, allow_cache),
            url_type::FILE => self.local(host, url, job),
            url_type::FTP => Err(Unsupported::OwnedBody("00817600 FTP scheduler admission")),
            url_type::NEWS => Ok(0), // Literal published boundary: selected target hold leaks.
            _ => {
                effect(host, Effect::ReleaseTarget(target));
                Ok(0)
            }
        }
    }
    fn local(&mut self, host: &mut impl Host, url: &mut RzUrl, mut job: Job) -> Contract<u32> {
        job.id = self.allocate();
        let mut path = native_path(url.absolute_path());
        if !host.file_exists(&path) {
            path = host.path_name(&path);
        }
        let exists = host.file_exists(&path);
        let mut returned = 0;
        if exists {
            job.data = path.clone();
            job.file = true;
            job.result = 1;
            job.progress = 100;
            returned = job.id;
            if job.preference <= 1 {
                if let Some(file) = host.file_open(&path, false) {
                    let size = host.file_size(file);
                    if job.preference == 0 || size < 500000 {
                        if let Some(data) = host.file_read(file, size) {
                            job.data = data;
                            job.file = false;
                        } else {
                            job.result = 0;
                            job.progress = 0;
                        }
                    }
                    host.file_close(file);
                }
            }
        }
        self.deliver(host, &mut job);
        Ok(returned)
    }
    fn admit_http(
        &mut self,
        host: &mut impl Host,
        mut job: Job,
        allow_cache: bool,
    ) -> Contract<u32> {
        effect(host, Effect::Lock);
        if self.cache != 0 && allow_cache && host.cache_present(&job.url) {
            let size = host.cache_size(&job.url);
            let ram = wants_ram(job.preference, size);
            let data = if ram { host.cache_data(&job.url) } else { None };
            let cached = if let Some(bytes) = data {
                effect(host, Effect::ReleaseCacheData);
                Some((bytes, false))
            } else {
                host.cache_file(&job.url).map(|bytes| (bytes, true))
            };
            if let Some((bytes, file)) = cached {
                job.id = self.allocate();
                job.kind = 1;
                job.data = bytes;
                job.file = file;
                job.result = 1;
                job.progress = 100;
                effect(host, Effect::Unlock);
                self.deliver(host, &mut job);
                return Ok(job.id);
            }
        }
        if self.active.iter().any(|active| active.url == job.url) {
            job.id = self.allocate();
            let id = job.id;
            self.waiting.push(job);
            effect(host, Effect::Unlock);
            return Ok(id);
        }
        effect(host, Effect::Unlock);
        if self.cookies != 0 && host.request_cookies(&job.url).is_some() {
            // Cookie header construction reaches still-O HTTP record support. No
            // opaque host submission is allowed to stand in for that owned body.
            return Err(Unsupported::OwnedBody("Q9 HTTP header record construction"));
        }
        let foreign = self.http.submit(host, 2, &job.url, None, 0, &[], 0)?;
        if foreign == 0 {
            return Ok(0);
        } // selected target hold remains acquired
        effect(host, Effect::Lock);
        job.id = self.allocate();
        job.kind = 1;
        job.foreign = foreign;
        let id = job.id;
        self.active.push(job);
        effect(host, Effect::Unlock);
        Ok(id)
    }
    pub fn status(&self, id: u32) -> u32 {
        if self
            .active
            .iter()
            .chain(&self.waiting)
            .any(|job| job.id == id)
        {
            2
        } else {
            0
        }
    }
    pub fn get_job(&self, id: u32) -> Option<&Job> {
        self.active.iter().find(|j| j.id == id)
    }
    pub fn http_id(&self, id: u32) -> u32 {
        self.get_job(id)
            .filter(|j| j.kind == 1 || j.kind == 2)
            .map_or(0, |j| j.foreign)
    }
    pub fn ftp_id(&self, id: u32) -> u32 {
        self.get_job(id)
            .filter(|j| j.kind == 3)
            .map_or(0, |j| j.foreign)
    }
    /// 008f7e76 / 008f7451: erasure does NOT release job target or notify.
    pub fn cancel(&mut self, host: &mut impl Host, id: u32) -> bool {
        effect(host, Effect::Lock);
        let found = if let Some(position) = self.active.iter().position(|j| j.id == id) {
            let job = self.active.remove(position);
            if job.kind == 1 || job.kind == 2 {
                self.http.cancel(host, job.foreign);
            }
            true
        } else if let Some(position) = self.waiting.iter().position(|j| j.id == id) {
            self.waiting.remove(position);
            true
        } else {
            false
        };
        effect(host, Effect::Unlock);
        found
    }
    pub fn cancel_all(&mut self, host: &mut impl Host) -> u32 {
        effect(host, Effect::Lock);
        let ids: Vec<_> = self
            .active
            .iter()
            .chain(&self.waiting)
            .map(|j| j.id)
            .collect();
        for id in &ids {
            self.cancel(host, *id);
        }
        self.active.clear();
        self.waiting.clear();
        effect(host, Effect::Unlock);
        ids.len() as u32
    }
    pub fn progress(
        &mut self,
        host: &mut impl Host,
        ftp: bool,
        foreign: Option<u32>,
        words: Option<[u32; 3]>,
    ) -> Contract<bool> {
        if ftp && foreign.is_none() {
            return Ok(true);
        }
        effect(host, Effect::Lock);
        if !ftp && foreign.is_none() && !self.active.is_empty() {
            effect(host, Effect::Unlock);
            return Err(Unsupported::NativeInputDomain(
                "HTTP progress null foreign job",
            ));
        }
        let job = self
            .active
            .iter()
            .find(|j| Some(j.foreign) == foreign)
            .cloned();
        effect(host, Effect::Unlock);
        if let (Some(job), Some(words)) = (job, words) {
            self.envelope(host, &job, Notification::Progress(words));
        }
        Ok(true)
    }
    fn take_foreign(&mut self, host: &mut impl Host, foreign: u32) -> Option<Job> {
        effect(host, Effect::Lock);
        let job = self
            .active
            .iter()
            .position(|j| j.foreign == foreign)
            .map(|p| self.active.remove(p));
        effect(host, Effect::Unlock);
        job
    }
    /// 008f8c5a: remove active before callback; select waiting AFTER callback.
    pub fn complete_http(
        &mut self,
        host: &mut impl Host,
        foreign: Option<u32>,
        success: bool,
        resource: Option<&mut http::Resource>,
    ) -> Contract<bool> {
        let Some(foreign) = foreign else {
            return Ok(true);
        };
        let Some(mut job) = self.take_foreign(host, foreign) else {
            return Ok(true);
        };
        if job.target.is_none() {
            // Q10: no delivery is specified, but this exceptional snapshot's
            // conversion/cache/fanout/cleanup suffix is not distinguished.
            return Err(Unsupported::OwnedBody(
                "Q10 HTTP completion without target suffix",
            ));
        }
        let mut resource = resource;
        job.foreign_pointer = Some(foreign);
        if success && resource.is_some() {
            let r = resource.as_deref_mut().expect("checked resource");
            convert(host, job.preference, &mut r.strings[1], &mut r.file);
            job.result = 1;
            job.progress = 100;
            job.data = r.strings[1].clone();
            job.file = r.file;
            cache_policy(
                host,
                self.cache != 0,
                Some((&job.url, &r.strings[1], r.file)),
                foreign,
            );
            if self.cookies != 0 {
                for family in [0x35, 0x34] {
                    for index in 0..32 {
                        let Some(text) = host.header(foreign, family, index) else {
                            break;
                        };
                        host.response_cookie(&r.strings[0], &text);
                    }
                }
            }
        } else {
            job.result = 0;
            job.progress = 0;
        }
        self.deliver(host, &mut job);
        effect(host, Effect::Lock);
        let mut selected = Vec::new();
        let mut index = 0;
        while index < self.waiting.len() {
            if self.waiting[index].url == job.url {
                selected.push(self.waiting.remove(index));
            } else {
                index += 1;
            }
        }
        effect(host, Effect::Unlock);
        for waiter in selected {
            job.id = waiter.id;
            job.target = waiter.target;
            job.custom = waiter.custom;
            self.deliver(host, &mut job);
        }
        if let Some(resource) = resource {
            resource.file = false;
            resource.strings[1].clear();
        }
        Ok(true)
    }
    /// 008f815f: no cache, cookies or fanout. Pointer validity is a precondition.
    pub fn complete_ftp(
        &mut self,
        host: &mut impl Host,
        foreign: u32,
        success: bool,
        filename: &[u8],
    ) -> bool {
        let Some(mut job) = self.take_foreign(host, foreign) else {
            return true;
        };
        job.result = u32::from(success);
        job.progress = if success { 100 } else { 0 };
        job.file = success;
        job.data = if success {
            filename.to_vec()
        } else {
            Vec::new()
        };
        if success && job.preference != 3 {
            convert(host, job.preference, &mut job.data, &mut job.file);
        }
        self.deliver(host, &mut job);
        true
    }
    pub fn set_directory(&mut self, host: &mut impl Host, path: &[u8]) -> bool {
        let path = directory(path);
        let mut cookie = path.clone();
        cookie.extend(b"CookieData.ini");
        effect(host, Effect::CookieFilename(cookie));
        effect(host, Effect::HttpDirectory(path.clone()));
        effect(host, Effect::HttpCacheSetting(b"_cache".to_vec()));
        effect(host, Effect::FtpDirectory(path.clone()));
        host.set_cache_directory(&path)
    }
    pub fn visited(&self, host: &mut impl Host, url: &[u8]) -> bool {
        host.cache_present(url)
    }
    /// 008f698b: typed decoding of the four published message IDs; unknown is true.
    pub fn do_message(&mut self, host: &mut impl Host, incoming: Incoming<'_>) -> Contract<bool> {
        match incoming {
            Incoming::HttpTerminal {
                foreign,
                success,
                resource,
            } => self.complete_http(host, foreign, success, resource),
            Incoming::FtpTerminal {
                foreign,
                success,
                filename,
            } => Ok(self.complete_ftp(host, foreign, success, filename)),
            Incoming::HttpProgress { foreign, words } => self.progress(host, false, foreign, words),
            Incoming::FtpProgress { foreign, words } => self.progress(host, true, foreign, words),
            Incoming::Unknown => Ok(true),
        }
    }
}

pub fn wants_ram(preference: i32, size: u32) -> bool {
    match preference {
        0 => true,
        1 => size < 2000000,
        2 => size < 500000,
        _ => false,
    }
}
fn convert(host: &mut impl Host, preference: i32, data: &mut Vec<u8>, file: &mut bool) {
    if preference == 3 && !*file {
        let path = host.new_cache_file();
        if let Some(handle) = host.file_open(&path, true) {
            if host.file_write(handle, data) {
                *data = path;
                *file = true;
            }
            host.file_close(handle);
        }
    } else if preference != 3 && *file {
        if let Some(handle) = host.file_open(data, false) {
            let size = host.file_size(handle);
            if wants_ram(preference, size) {
                if let Some(bytes) = host.file_read(handle, size) {
                    *data = bytes;
                    *file = false;
                }
            }
            host.file_close(handle);
        }
    }
}

/// Called splitter from cache policy: at most six slots, quote toggling, no escapes.
pub fn cache_tokens(text: &[u8]) -> Vec<Vec<u8>> {
    let mut tokens = Vec::new();
    let mut token = Vec::new();
    let mut quoted = false;
    let mut tail_present = false;
    for &b in text {
        if b == b'"' {
            quoted = !quoted;
            tail_present = true;
        } else if b == b',' && !quoted {
            tokens.push(trim(&token).to_vec());
            token.clear();
            tail_present = false;
            if tokens.len() == 6 {
                return tokens;
            }
        } else {
            token.push(b);
            tail_present = true;
        }
    }
    if tail_present && tokens.len() < 6 {
        tokens.push(trim(&token).to_vec());
    }
    tokens
}
fn trim(mut value: &[u8]) -> &[u8] {
    while value.first().is_some_and(|b| matches!(b, b' ' | b'\t')) {
        value = &value[1..];
    }
    while value.last().is_some_and(|b| matches!(b, b' ' | b'\t')) {
        value = &value[..value.len() - 1];
    }
    value
}
/// 008f7a50: header occurrences zero only; no repaired max-age argument parsing.
pub fn cache_policy(
    host: &mut impl Host,
    enabled: bool,
    resource: Option<(&[u8], &[u8], bool)>,
    foreign: u32,
) -> bool {
    let Some((url, data, file)) = resource else {
        return false;
    };
    if host
        .header(foreign, 4, 0)
        .is_some_and(|v| v.windows(8).any(|w| w == b"no-cache"))
    {
        return false;
    }
    let date = if let Some(text) = host.header(foreign, 3, 0) {
        host.parse_time(&text)
    } else {
        host.now()
    };
    if let Some(text) = host.header(foreign, 0x2d, 0) {
        host.parse_time(&text);
    }
    let mut expiry = if let Some(text) = host.header(foreign, 0x2c, 0) {
        host.parse_time(&text)
    } else {
        date.wrapping_add(259200)
    };
    let mut recognized: [Vec<u8>; 5] = Default::default();
    if let Some(text) = host.header(foreign, 1, 0) {
        for token in cache_tokens(&text) {
            let c_string = token.split(|b| *b == 0).next().unwrap_or_default();
            for (index, name) in [
                b"private".as_slice(),
                b"no-cache",
                b"must-revalidate",
                b"max-age",
                b"no-store",
            ]
            .iter()
            .enumerate()
            {
                if c_string.eq_ignore_ascii_case(name) {
                    recognized[index] = token.clone();
                }
            }
        }
    }
    if [0, 1, 4].iter().any(|i| !recognized[*i].is_empty()) {
        return false;
    }
    if let Some(equal) = recognized[3].iter().position(|b| *b == b'=') {
        if equal < recognized[3].len() - 1 {
            let bytes = &recognized[3][equal + 1..];
            expiry = date.wrapping_add(super::ftp::atol(bytes) as u32);
        }
    }
    if !enabled || data.is_empty() {
        return false;
    }
    host.insert_cache(url, data, file, if file { (0, 1) } else { (1, 1) }, expiry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Fake {
        effects: Vec<Effect>,
        http_effects: Vec<http::Effect>,
        callbacks: Vec<(Job, Notification)>,
        headers: BTreeMap<(u32, u32), Vec<u8>>,
        cookies: Vec<Vec<u8>>,
        insertions: Vec<(bool, (u32, u32), u32)>,
        size: u32,
        reads: usize,
        exists: bool,
        open: bool,
        read: bool,
        write: bool,
        cached: bool,
        cache_ram: bool,
        cache_file: bool,
        cancel_in_callback: Option<u32>,
        handlers: Vec<u32>,
        accept: bool,
        handler_result: bool,
        time: u32,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                effects: vec![],
                http_effects: vec![],
                callbacks: vec![],
                headers: BTreeMap::new(),
                cookies: vec![],
                insertions: vec![],
                size: 1,
                reads: 0,
                exists: true,
                open: true,
                read: true,
                write: true,
                cached: false,
                cache_ram: true,
                cache_file: true,
                cancel_in_callback: None,
                handlers: vec![],
                accept: false,
                handler_result: false,
                time: 1000,
            }
        }
    }
    impl http::Host for Fake {
        fn effect(&mut self, value: http::Effect) {
            self.http_effects.push(value);
        }
        fn notify(&mut self, _: &mut http::Client, _: http::Job, _: bool) {}
    }
    impl Host for Fake {
        fn effect(&mut self, value: Effect) {
            self.effects.push(value);
        }
        fn now(&mut self) -> u32 {
            self.time
        }
        fn file_exists(&mut self, _: &[u8]) -> bool {
            self.exists
        }
        fn path_name(&mut self, path: &[u8]) -> Vec<u8> {
            path.to_vec()
        }
        fn file_open(&mut self, _: &[u8], _: bool) -> Option<u32> {
            self.open.then_some(1)
        }
        fn file_size(&mut self, _: u32) -> u32 {
            self.size
        }
        fn file_read(&mut self, _: u32, _: u32) -> Option<Vec<u8>> {
            self.reads += 1;
            self.read.then(|| b"RAM".to_vec())
        }
        fn file_write(&mut self, _: u32, _: &[u8]) -> bool {
            self.write
        }
        fn file_close(&mut self, _: u32) {}
        fn cache_present(&mut self, _: &[u8]) -> bool {
            self.cached
        }
        fn cache_size(&mut self, _: &[u8]) -> u32 {
            self.size
        }
        fn cache_data(&mut self, _: &[u8]) -> Option<Vec<u8>> {
            self.reads += 1;
            self.cache_ram.then(|| b"cache-data".to_vec())
        }
        fn cache_file(&mut self, _: &[u8]) -> Option<Vec<u8>> {
            self.cache_file.then(|| b"cache-file".to_vec())
        }
        fn new_cache_file(&mut self) -> Vec<u8> {
            b"new-file".to_vec()
        }
        fn insert_cache(
            &mut self,
            _: &[u8],
            _: &[u8],
            file: bool,
            flags: (u32, u32),
            expiry: u32,
        ) -> bool {
            self.insertions.push((file, flags, expiry));
            true
        }
        fn cache_directory(&mut self) -> Vec<u8> {
            b"directory".to_vec()
        }
        fn set_cache_directory(&mut self, _: &[u8]) -> bool {
            false
        }
        fn header(&mut self, _: u32, family: u32, index: u32) -> Option<Vec<u8>> {
            self.headers.get(&(family, index)).cloned()
        }
        fn parse_time(&mut self, text: &[u8]) -> u32 {
            std::str::from_utf8(text).unwrap().parse().unwrap()
        }
        fn request_cookies(&mut self, _: &[u8]) -> Option<Vec<u8>> {
            None
        }
        fn response_cookie(&mut self, _: &[u8], text: &[u8]) {
            self.cookies.push(text.to_vec());
        }
        fn handler_init(&mut self, _: u32) -> bool {
            true
        }
        fn handler_shutdown(&mut self, id: u32) {
            self.handlers.push(id);
        }
        fn handler_accepts(&mut self, id: u32, _: &mut RzUrl) -> bool {
            self.handlers.push(id);
            self.accept
        }
        fn handler_load(&mut self, id: u32, _: &mut Job) -> bool {
            self.handlers.push(id);
            self.handler_result
        }
        fn notify(&mut self, service: &mut Www, job: &Job, notification: Notification) {
            assert_eq!(service.status(job.id), 0);
            self.callbacks.push((job.clone(), notification));
            if let Some(id) = self.cancel_in_callback.take() {
                assert!(service.cancel(self, id));
            }
        }
    }
    fn service() -> Www {
        Www::snapshot(http::Client::new(None))
    }
    fn resource(file: bool) -> http::Resource {
        http::Resource {
            refs: 1,
            strings: [b"http://host/x".to_vec(), b"payload".to_vec(), vec![]],
            file,
            redirects: 0,
        }
    }
    #[test]
    fn identity_default_hold_order_and_dispatch_target_selection() {
        let mut w = service();
        let mut h = Fake::default();
        let mut out = 123;
        assert!(!w.query_interface(&mut h, 99, &mut out));
        assert_eq!(out, 123);
        assert!(h.effects.is_empty());
        assert!(w.query_interface(&mut h, 1, &mut out));
        assert_eq!(out, 4);
        w.set_default_target(&mut h, Some(9));
        w.set_default_target(&mut h, Some(9));
        w.set_default_target(&mut h, None);
        assert_eq!(
            &h.effects[1..],
            [
                Effect::AddTarget(9),
                Effect::AddTarget(9),
                Effect::ReleaseTarget(9),
                Effect::ReleaseTarget(9)
            ]
        );
        assert_eq!(
            w.load_text(&mut h, b"http://host/x", 0, None, 0, true),
            Ok(0)
        );
        assert!(h.callbacks.is_empty());
    }
    #[test]
    fn handler_failure_owns_attempt_and_releases_after_callback() {
        let mut w = service();
        let mut h = Fake {
            accept: true,
            ..Fake::default()
        };
        w.add_handler(&mut h, 1);
        w.add_handler(&mut h, 2);
        assert_eq!(
            w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, true),
            Ok(2)
        );
        assert_eq!(h.handlers, [1, 1]);
        assert_eq!(h.callbacks[0].0.result, 0);
        assert_eq!(h.effects.last(), Some(&Effect::ReleaseTarget(9)));
        w.remove_handler(&mut h, 1);
        w.remove_all_handlers(&mut h);
        assert!(w.handlers.is_empty());
    }
    #[test]
    fn local_threshold_open_fallback_and_read_failure_nonzero_id() {
        for (size, reads, file) in [(499999, 1, false), (500000, 0, true)] {
            let mut w = service();
            let mut h = Fake {
                size,
                ..Fake::default()
            };
            assert_eq!(
                w.load_text(&mut h, b"file:///C|/x", 1, Some(9), 0, false),
                Ok(2)
            );
            assert_eq!(h.reads, reads);
            assert_eq!(h.callbacks[0].0.file, file);
        }
        let mut w = service();
        let mut h = Fake {
            read: false,
            ..Fake::default()
        };
        assert_eq!(
            w.load_text(&mut h, b"file:///C|/x", 0, Some(9), 0, false),
            Ok(2)
        );
        assert_eq!(h.callbacks[0].0.result, 0);
        let mut w = service();
        let mut h = Fake {
            open: false,
            ..Fake::default()
        };
        assert_eq!(
            w.load_text(&mut h, b"file:///C|/x", 0, Some(9), 0, false),
            Ok(2)
        );
        assert!(h.callbacks[0].0.file);
        let mut w = service();
        let mut h = Fake {
            exists: false,
            ..Fake::default()
        };
        assert_eq!(
            w.load_text(&mut h, b"file:///C|/x", 0, Some(9), 0, false),
            Ok(0)
        );
        assert_eq!(h.callbacks[0].0.progress, 0);
    }
    #[test]
    fn cache_threshold_and_failed_retrieval_continues_to_admission() {
        for (size, reads) in [(1999999, 1), (2000000, 0)] {
            let mut w = service();
            w.cache = 1;
            let mut h = Fake {
                size,
                cached: true,
                ..Fake::default()
            };
            assert_eq!(
                w.load_text(&mut h, b"http://host/x", 1, Some(9), 0, true),
                Ok(2)
            );
            assert_eq!(h.reads, reads);
            assert_eq!(w.status(2), 0);
        }
        let mut w = service();
        w.cache = 1;
        let mut h = Fake {
            cached: true,
            cache_ram: false,
            cache_file: false,
            ..Fake::default()
        };
        assert_eq!(
            w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, true),
            Ok(2)
        );
        assert_eq!(w.status(2), 2);
        assert!(h.callbacks.is_empty());
        assert_eq!(w.http.queue.len(), 1);
    }
    #[test]
    fn duplicate_waiting_foreign_status_cancel_leaks_and_fanout_reentry() {
        let mut w = service();
        let mut h = Fake::default();
        for preference in [3, 0, 1] {
            w.load_text(
                &mut h,
                b"http://host/x",
                preference,
                Some(9),
                preference as u32 + 1,
                false,
            )
            .unwrap();
        }
        assert_eq!(w.http.queue.len(), 1);
        assert_eq!(w.status(3), 2);
        assert_eq!(w.http_id(3), 0);
        assert!(w.get_job(3).is_none());
        h.cancel_in_callback = Some(3);
        let mut r = resource(false);
        w.complete_http(&mut h, Some(1), true, Some(&mut r))
            .unwrap();
        assert_eq!(
            h.callbacks.iter().map(|c| c.0.id).collect::<Vec<_>>(),
            [2, 4]
        );
        assert!(h.callbacks[1].0.file);
        assert_eq!(h.callbacks[1].0.preference, 3);
        assert_eq!(
            h.effects
                .iter()
                .filter(|e| **e == Effect::ReleaseTarget(9))
                .count(),
            2
        );
        assert!(r.strings[1].is_empty());
        assert!(!r.file);
    }
    #[test]
    fn cancellation_late_resource_and_foreign_admission_zero_hold() {
        let mut w = service();
        let mut h = Fake::default();
        w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false)
            .unwrap();
        assert!(w.cancel(&mut h, 2));
        assert!(!w.cancel(&mut h, 2));
        assert!(h.callbacks.is_empty());
        assert!(!h.effects.contains(&Effect::ReleaseTarget(9)));
        let mut r = resource(true);
        let before = r.clone();
        assert_eq!(
            w.complete_http(&mut h, Some(1), true, Some(&mut r)),
            Ok(true)
        );
        assert_eq!(r, before);
        let mut w = service();
        w.http.counter = u32::MAX;
        let mut h = Fake::default();
        assert_eq!(
            w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false),
            Ok(0)
        );
        assert!(!h.effects.contains(&Effect::ReleaseTarget(9)));
        assert_eq!(w.http.queue.len(), 1);
    }
    #[test]
    fn completion_conversion_failure_cookies_and_cache_before_target() {
        let mut w = service();
        w.cache = 1;
        let mut h = Fake {
            write: false,
            ..Fake::default()
        };
        for family in [0x35, 0x34] {
            for index in 0..33 {
                h.headers
                    .insert((family, index), format!("{family}:{index}").into_bytes());
            }
        }
        w.load_text(&mut h, b"http://host/x", 3, Some(9), 0, false)
            .unwrap();
        let mut r = resource(false);
        w.complete_http(&mut h, Some(1), true, Some(&mut r))
            .unwrap();
        assert_eq!(h.callbacks[0].0.result, 1);
        assert!(!h.callbacks[0].0.file);
        assert_eq!(h.cookies.len(), 64);
        assert_eq!(h.cookies[31], b"53:31");
        assert_eq!(h.cookies[32], b"52:0");
        assert_eq!(h.insertions, [(false, (1, 1), 260200)]);
    }
    #[test]
    fn policy_distinguishing_headers_and_wrapping_dates() {
        for (family, text, insert) in [
            (4, b"xno-cachey".as_slice(), false),
            (4, b"NO-CACHE", true),
            (1, b" PRIVATE ", false),
            (1, b"private=field, no-store=1, max-age=60", true),
            (1, b"must-revalidate, max-age", true),
            (1, b"a,b,c,d,e,f,no-store", true),
            (1, b"a,b,c,d,e,no-store,f", false),
            (1, b"\"a,no-store\", public", true),
        ] {
            let mut h = Fake::default();
            h.headers.insert((family, 0), text.to_vec());
            assert_eq!(
                cache_policy(&mut h, true, Some((b"url", b"data", false)), 1),
                insert
            );
            if insert {
                assert_eq!(h.insertions[0].2, 260200);
            }
        }
        let mut h = Fake::default();
        h.headers.insert((3, 0), b"4294901760".to_vec());
        assert!(cache_policy(&mut h, true, Some((b"url", b"file", true)), 1));
        assert_eq!(h.insertions[0], (true, (0, 1), 0x2f480));
        h.headers.insert((0x2c, 0), b"7".to_vec());
        h.headers.insert((0x2d, 0), b"900".to_vec());
        assert!(cache_policy(
            &mut h,
            true,
            Some((b"url", b"data", false)),
            1
        ));
        assert_eq!(h.insertions[1].2, 7);
        h.headers.insert((1, 0), b"max-age\0=60".to_vec());
        h.headers.insert((3, 0), b"1000".to_vec());
        assert!(cache_policy(
            &mut h,
            true,
            Some((b"url", b"data", false)),
            1
        ));
        assert_eq!(h.insertions[2].2, 1060);
        assert!(!cache_policy(
            &mut h,
            false,
            Some((b"url", b"data", false)),
            1
        ));
        assert!(!cache_policy(&mut h, true, None, 1));
    }
    #[test]
    fn splitter_empty_slots_quote_and_directory_side_effects() {
        assert_eq!(
            cache_tokens(b",a,,b, ,"),
            vec![
                b"".to_vec(),
                b"a".to_vec(),
                b"".to_vec(),
                b"b".to_vec(),
                b"".to_vec()
            ]
        );
        assert_eq!(
            cache_tokens(b"\"a,b\",c"),
            vec![b"a,b".to_vec(), b"c".to_vec()]
        );
        let mut w = service();
        let mut h = Fake::default();
        assert!(!w.set_directory(&mut h, b"out"));
        assert_eq!(
            h.effects,
            [
                Effect::CookieFilename(b"out\\CookieData.ini".to_vec()),
                Effect::HttpDirectory(b"out\\".to_vec()),
                Effect::HttpCacheSetting(b"_cache".to_vec()),
                Effect::FtpDirectory(b"out\\".to_vec())
            ]
        );
        h.cached = true;
        assert!(w.visited(&mut h, b"url"));
        assert_eq!(w.cache, 0);
    }
    #[test]
    fn progress_borrowed_no_waiter_update_and_cancel_all() {
        let mut w = service();
        let mut h = Fake::default();
        w.load_text(&mut h, b"http://host/x", 0, Some(9), 0, false)
            .unwrap();
        w.load_text(&mut h, b"http://host/x", 1, Some(9), 0, false)
            .unwrap();
        // Progress callbacks observe live membership (unlike terminal callbacks).
        let before = w.active[0].progress;
        assert_eq!(w.progress(&mut h, true, None, Some([1, 2, 3])), Ok(true));
        assert!(h.callbacks.is_empty());
        assert!(w.progress(&mut h, false, None, Some([1, 2, 3])).is_err());
        assert_eq!(w.active[0].progress, before);
        assert_eq!(w.cancel_all(&mut h), 2);
        assert!(w.active.is_empty() && w.waiting.is_empty());
        assert_eq!(w.do_message(&mut h, Incoming::Unknown), Ok(true));
        w.post_init(&mut h);
        w.shutdown(&mut h);
        assert!(!w.initialized);
    }
}
