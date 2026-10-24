//! Cookie owner, web-network §3. IO and CRT calendar conversion are foreign.
use super::{Contract, Unsupported};
use crate::rzurl::{url_type, RzUrl};

struct Url {
    server: Vec<u8>,
    path: Vec<u8>,
    port: u32,
    url_type: u32,
}
impl Url {
    fn new(url: &RzUrl) -> Self {
        let mut url = url.clone();
        let url_type = url.url_type();
        Self {
            server: url.server().to_vec(),
            path: url.absolute_path().to_vec(),
            port: url.port(),
            url_type,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookie {
    pub text: Vec<u8>,
    pub pairs: Vec<(Vec<u8>, Vec<u8>)>,
    pub comment: Vec<u8>,
    pub comment_url: Vec<u8>,
    pub domain: Vec<u8>,
    pub path: Vec<u8>,
    pub version: i32,
    pub expires: u32,
    pub ports: [u32; 16],
    pub secure: bool,
}
impl Default for Cookie {
    fn default() -> Self {
        Self {
            text: Vec::new(),
            pairs: Vec::new(),
            comment: Vec::new(),
            comment_url: Vec::new(),
            domain: Vec::new(),
            path: Vec::new(),
            version: 0,
            expires: 2,
            ports: [0; 16],
            secure: false,
        }
    }
}

/// Runtime date and INI IO dependencies.
pub trait Host {
    fn now(&mut self) -> u32;
    fn date_seconds(&mut self, text: &[u8]) -> Contract<u32>;
    fn exists(&mut self, path: &[u8]) -> bool;
    fn clear(&mut self, path: &[u8]);
    fn binary_marker(&mut self, path: &[u8]) -> bool;
    fn write_binary_marker(&mut self, path: &[u8]);
    fn read_text(&mut self, path: &[u8], key: &[u8]) -> Option<Vec<u8>>;
    fn write_text(&mut self, path: &[u8], key: &[u8], text: &[u8]);
    fn read_binary(&mut self, path: &[u8], key: &[u8], limit: usize) -> Contract<Vec<u8>> {
        let encoded = self.read_text(path, key).unwrap_or_default();
        let encoded = &encoded[..encoded.len().min(limit * 2)];
        if encoded.len() % 2 != 0 {
            return Err(Unsupported::NativeInputDomain("odd cookie hex payload"));
        }
        let mut bytes = Vec::new();
        for pair in encoded.chunks_exact(2) {
            let hex = |b: u8| (b as char).to_digit(16);
            let (Some(a), Some(b)) = (hex(pair[0]), hex(pair[1])) else {
                return Ok(Vec::new());
            };
            bytes.push((a * 16 + b) as u8);
        }
        Ok(bytes)
    }
    fn write_binary(&mut self, path: &[u8], key: &[u8], text: &[u8]) -> Contract<()> {
        const HEX: &[u8] = b"0123456789ABCDEF";
        let encoded: Vec<_> = text
            .iter()
            .flat_map(|b| [HEX[(b >> 4) as usize], HEX[(b & 15) as usize]])
            .collect();
        self.write_text(path, key, &encoded);
        Ok(())
    }
}

fn trim(text: &[u8]) -> &[u8] {
    let start = text
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t'))
        .unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t'))
        .map_or(start, |i| i + 1);
    &text[start..end]
}
/// Shared native splitter: only nonempty pre-trim tails count; quotes are consumed.
pub fn split(text: &[u8], delimiter: u8, capacity: usize, quotes: bool) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut item = Vec::new();
    let mut quoted = false;
    for &byte in text {
        if quotes && byte == b'"' {
            quoted = !quoted;
        } else if byte == delimiter && !quoted {
            out.push(trim(&item).to_vec());
            item.clear();
            if out.len() == capacity {
                return out;
            }
        } else {
            item.push(byte);
        }
    }
    if !item.is_empty() && out.len() < capacity {
        out.push(trim(&item).to_vec());
    }
    out
}
/// Decimal prefix conversion (32-bit atol). Overflow is represented by low bits.
pub fn decimal(text: &[u8]) -> i32 {
    super::ftp::atol(text)
}

impl Cookie {
    pub fn parse(text: &[u8], url: Option<&RzUrl>, host: &mut impl Host) -> Contract<Self> {
        let parsed = url.map(Url::new);
        let url = parsed.as_ref();
        let now = host.now();
        let mut cookie = Self {
            text: text.to_vec(),
            ..Self::default()
        };
        if !text.is_empty() {
            cookie.pairs = split(text, b';', 40, true)
                .iter()
                .map(|item| {
                    let item = trim(item);
                    let at = item.iter().position(|b| *b == b'=');
                    // Literal item-only trim (§3.2); inner blanks are preserved (Q15).
                    at.map_or_else(
                        || (item.to_vec(), Vec::new()),
                        |i| (item[..i].to_vec(), item[i + 1..].to_vec()),
                    )
                })
                .collect();
        }
        let mut domain_set = false;
        let mut path_set = false;
        for (name, value) in &cookie.pairs {
            match name.to_ascii_lowercase().as_slice() {
                b"comment" => cookie.comment = value.clone(),
                b"commenturl" => cookie.comment_url = value.clone(),
                b"domain" => {
                    domain_set = true;
                    cookie.domain = value.clone();
                    if !cookie.domain.starts_with(b".") {
                        cookie.domain.insert(0, b'.');
                    }
                }
                b"path" => {
                    path_set = true;
                    cookie.path = value.clone();
                }
                b"version" => cookie.version = decimal(value),
                b"discard" => cookie.expires = 1,
                b"expires" => {
                    let parsed = host.date_seconds(value)?;
                    cookie.expires = if parsed == 0 { now.wrapping_add(86400) } else { parsed };
                }
                b"max-age" => cookie.expires = now.wrapping_add(decimal(value) as u32),
                b"secure" => cookie.secure = true,
                b"port" => {
                    cookie.ports = [0; 16];
                    if value.is_empty() {
                        cookie.ports[0] = url.map_or(80, port);
                    } else {
                        let values = split(value, b',', 16, false);
                        for (slot, item) in cookie
                            .ports
                            .iter_mut()
                            .zip(values.iter().take(values.len().saturating_sub(1)))
                        {
                            *slot = decimal(item) as u32;
                        }
                    }
                }
                _ => {}
            }
        }
        if !domain_set {
            if let Some(url) = url {
                cookie.domain = url
                    .server
                    .iter()
                    .position(|b| *b == b'.')
                    .map_or_else(Vec::new, |i| url.server[i..].to_vec());
            }
            if !cookie.domain.is_empty() {
                cookie.text.extend_from_slice(b";  domain=");
                cookie.text.extend_from_slice(&cookie.domain);
            }
        }
        if !path_set {
            cookie.path = url.map_or_else(|| b"/".to_vec(), |u| u.path.clone());
            if !cookie.path.is_empty() {
                cookie.text.extend_from_slice(b"; path=");
                cookie.text.extend_from_slice(&cookie.path);
            }
        }
        Ok(cookie)
    }
    fn alive(&self, now: u32) -> bool {
        self.expires == 2 || now < self.expires
    }
    fn port_matches(&self, url: &Url) -> bool {
        self.ports[0] == 0 || self.ports.iter().take_while(|p| **p != 0).any(|p| *p == port(url))
    }
    fn matches(&self, url: &Url, now: u32) -> bool {
        self.alive(now)
            && (!self.secure || url.url_type == url_type::HTTPS)
            && suffix(&url.server, &self.domain)
            && !self.path.is_empty()
            && url.path.starts_with(&self.path)
            && self.port_matches(url)
    }
}
fn port(url: &Url) -> u32 {
    if url.port == 0 {
        80
    } else {
        url.port
    }
}
fn suffix(server: &[u8], domain: &[u8]) -> bool {
    !domain.is_empty()
        && domain.len() <= server.len()
        && server.windows(domain.len()).position(|w| w == domain) == server.len().checked_sub(domain.len())
}

#[derive(Clone, Debug)]
pub struct Manager {
    pub initialized: bool,
    pub binary: bool,
    pub capacity: usize,
    pub path: Vec<u8>,
    pub records: Vec<Cookie>,
}
impl Default for Manager {
    fn default() -> Self {
        Self::new(b"CookieData.ini".to_vec())
    }
}
impl Manager {
    pub fn new(path: Vec<u8>) -> Self {
        Self {
            initialized: false,
            binary: false,
            capacity: 16384,
            path,
            records: Vec::new(),
        }
    }
    pub fn set_binary(&mut self, binary: bool) {
        self.binary = binary;
    }
    pub fn file_path(&self) -> &[u8] {
        &self.path
    }
    pub fn set_file_path(&mut self, host: &mut impl Host, path: &[u8]) -> bool {
        if self.path != path {
            self.path = path.to_vec();
            let _ = self.read(host);
        }
        true
    }
    pub fn init(&mut self, host: &mut impl Host) -> bool {
        if !self.initialized {
            if self.records.is_empty() {
                let _ = self.read(host);
            }
            self.initialized = true;
        }
        true
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        if self.initialized {
            let _ = self.write(host);
            self.records.clear();
            self.initialized = false;
        }
        true
    }
    pub fn enumerate(&self, count: &mut usize) -> Vec<Cookie> {
        *count = (*count).min(self.records.len());
        self.records[..*count].to_vec()
    }
    pub fn remove_all(&mut self) {
        self.records.clear();
    }
    pub fn add_text(&mut self, text: &[u8], url: Option<&RzUrl>, host: &mut impl Host) -> Contract<bool> {
        self.add(
            Cookie {
                text: text.to_vec(),
                ..Cookie::default()
            },
            url,
            host,
        )
    }
    pub fn add(&mut self, mut cookie: Cookie, url: Option<&RzUrl>, host: &mut impl Host) -> Contract<bool> {
        if cookie.pairs.is_empty() {
            if cookie.text.is_empty() {
                return Ok(false);
            }
            cookie = Cookie::parse(&cookie.text, url, host)?;
        }
        let parsed = url.map(Url::new);
        let url = parsed.as_ref();
        if let Some(url) = url {
            let domain = &cookie.domain;
            if cookie.path.is_empty()
                || !url.path.starts_with(&cookie.path)
                || domain.len() <= 2
                || domain.ends_with(b".")
                || !domain.contains(&b'.')
                || (domain.starts_with(b".") && !domain[2..].contains(&b'.'))
                || !suffix(&url.server, domain)
                || !cookie.port_matches(url)
            {
                return Ok(false);
            }
            if url.server[..url.server.len() - domain.len()].contains(&b'.') {
                return Ok(false);
            }
        }
        self.records.retain(|old| {
            old.pairs.first().map(|p| &p.0) != cookie.pairs.first().map(|p| &p.0)
                || old.domain != cookie.domain
                || old.path != cookie.path
        });
        let now = host.now();
        if !cookie.alive(now) {
            return Ok(false);
        }
        if self.records.len() >= self.capacity {
            self.evict(now);
        }
        self.records.push(cookie);
        Ok(true)
    }
    pub fn evict(&mut self, now: u32) -> usize {
        let before = self.records.len();
        self.records.retain(|c| c.alive(now));
        while before - self.records.len() < 4 && !self.records.is_empty() {
            let i = self
                .records
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| if c.expires == 2 { now } else { c.expires })
                .unwrap()
                .0;
            self.records.remove(i);
        }
        before - self.records.len()
    }
    pub fn lookup(&self, url: &RzUrl, k: usize, now: u32) -> Option<Cookie> {
        let url = Url::new(url);
        self.records.iter().filter(|c| c.matches(&url, now)).nth(k).cloned()
    }
    pub fn lookup_text(&self, url: &[u8], k: usize, now: u32) -> Option<Cookie> {
        self.lookup(&RzUrl::from_text(url), k, now)
    }
    /// §3.4 copying overload. Literal success-only assignment on a miss (Q16).
    pub fn lookup_into(&self, url: &RzUrl, out: &mut Cookie, k: usize, now: u32) -> bool {
        if let Some(cookie) = self.lookup(url, k, now) {
            *out = cookie;
            true
        } else {
            false
        }
    }
    pub fn header(&self, url: &RzUrl, now: u32, out: &mut Vec<u8>) -> bool {
        let url = Url::new(url);
        out.clear();
        let mut version = false;
        for (k, cookie) in self
            .records
            .iter()
            .filter(|c| c.matches(&url, now))
            .take(1000)
            .enumerate()
        {
            for (i, (name, value)) in cookie.pairs.iter().enumerate() {
                let lower = name.to_ascii_lowercase();
                if matches!(
                    lower.as_slice(),
                    b"comment" | b"commenturl" | b"discard" | b"expires" | b"max-age" | b"secure"
                ) || (lower == b"version" && version)
                {
                    continue;
                }
                if k != 0 || i != 0 {
                    out.extend_from_slice(b"; ");
                }
                if lower == b"version" && !out.is_empty() {
                    let mut prefix = vec![b'$'];
                    prefix.extend_from_slice(name);
                    prefix.push(b'=');
                    prefix.extend_from_slice(value);
                    prefix.extend_from_slice(b"; ");
                    out.truncate(out.len() - 2);
                    prefix.extend_from_slice(out);
                    *out = prefix;
                    continue;
                }
                if matches!(lower.as_slice(), b"domain" | b"path" | b"port" | b"version") {
                    out.push(b'$');
                }
                if lower == b"version" {
                    version = true;
                }
                out.extend_from_slice(name);
                out.push(b'=');
                out.extend_from_slice(value);
            }
        }
        !out.is_empty()
    }
    pub fn header_text(&self, url: &[u8], now: u32, out: &mut Vec<u8>) -> bool {
        self.header(&RzUrl::from_text(url), now, out)
    }
    pub fn clear_file(&self, host: &mut impl Host) {
        host.clear(&self.path);
    }
    pub fn write(&self, host: &mut impl Host) -> Contract<bool> {
        host.clear(&self.path);
        let now = host.now();
        if self.binary {
            host.write_binary_marker(&self.path);
        }
        for (n, cookie) in self.records.iter().filter(|c| c.expires > now).enumerate() {
            let key = format!("CookieData{n:05}");
            if self.binary {
                host.write_binary(&self.path, key.as_bytes(), &cookie.text)?;
            } else {
                host.write_text(&self.path, key.as_bytes(), &cookie.text);
            }
        }
        Ok(true)
    }
    pub fn read(&mut self, host: &mut impl Host) -> Contract<bool> {
        self.records.clear();
        if !host.exists(&self.path) {
            return Ok(true);
        }
        let binary = host.binary_marker(&self.path);
        for n in 0..10000 {
            let key = format!("CookieData{n:05}");
            let text = if binary {
                let text = host.read_binary(&self.path, key.as_bytes(), 16384)?;
                if text.is_empty() {
                    break;
                }
                text
            } else {
                let Some(text) = host.read_text(&self.path, key.as_bytes()) else {
                    break;
                };
                text
            };
            let _ = self.add_text(&text, None, host)?;
        }
        Ok(true)
    }
}
