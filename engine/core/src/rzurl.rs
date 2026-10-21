//! cRZURL: URL parser and resolver (docs/specs/web-network.md section 2).
//!
//! Pure byte-string implementation. No network access. Character classes use the C
//! locale (ASCII); the spec names `isalnum` in the C locale for the scheme test and a
//! one-argument CRT classifier (decimal digits, inferred) for the port.

/// URL type codes (spec table 2.2).
pub mod url_type {
    pub const NONE: u32 = 0;
    pub const OTHER: u32 = 1;
    pub const HTTP: u32 = 2;
    /// Never produced by the scheme step (spec 2.3 step 4 quirk).
    pub const HTTPS: u32 = 3;
    pub const FILE: u32 = 4;
    pub const FTP: u32 = 5;
    pub const MAILTO: u32 = 6;
    pub const NEWS: u32 = 7;
    pub const IRC: u32 = 8;
    pub const RELATIVE: u32 = 9;
}

use url_type::*;

/// Most port digits kept (spec 2.4: a longer run overflows a 16-byte buffer in the original).
const MAX_PORT_DIGITS: usize = 15;

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    find_from(hay, needle, 0)
}

fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from > hay.len() || hay.len() - from < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

fn find_byte(hay: &[u8], b: u8) -> Option<usize> {
    hay.iter().position(|&c| c == b)
}

fn rfind_byte(hay: &[u8], b: u8) -> Option<usize> {
    hay.iter().rposition(|&c| c == b)
}

/// Emulates MSVC `atol` on a run of decimal digits, then the store into an unsigned 32-bit
/// port. Literal reading: digits accumulate with 32-bit wrap-around (spec silent on
/// overflow; question web-network Q5).
fn atol_digits(digits: &[u8]) -> u32 {
    let mut v: u32 = 0;
    for &d in digits {
        v = v.wrapping_mul(10).wrapping_add(u32::from(d - b'0'));
    }
    v
}

/// One cRZURL object (spec 2.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RzUrl {
    allow_relative: bool,
    parsed: bool,
    url_type: u32,
    full: Vec<u8>,
    scheme: Vec<u8>,
    user: Vec<u8>,
    password: Vec<u8>,
    server: Vec<u8>,
    port: u32,
    path: Vec<u8>,
    params: Vec<u8>,
    query: Vec<u8>,
    fragment: Vec<u8>,
}

impl RzUrl {
    /// Default constructor: everything empty, allowRelative false (guess mode).
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct-from-string constructor: full = text, allowRelative true, not parsed.
    pub fn from_text(text: &[u8]) -> Self {
        RzUrl {
            allow_relative: true,
            full: text.to_vec(),
            ..Self::default()
        }
    }

    pub fn allow_relative(&self) -> bool {
        self.allow_relative
    }

    pub fn is_parsed(&self) -> bool {
        self.parsed
    }

    /// GetFullURL: the text as given, unparsed.
    pub fn full_url(&self) -> &[u8] {
        &self.full
    }

    fn ensure_parsed(&mut self) {
        if !self.parsed {
            let text = self.full.clone();
            self.parse_url(&text);
        }
    }

    /// GetURLType (lazy parse).
    pub fn url_type(&mut self) -> u32 {
        self.ensure_parsed();
        self.url_type
    }

    /// GetScheme (lazy parse).
    pub fn scheme(&mut self) -> &[u8] {
        self.ensure_parsed();
        &self.scheme
    }

    /// GetServer (lazy parse).
    pub fn server(&mut self) -> &[u8] {
        self.ensure_parsed();
        &self.server
    }

    /// GetAbsolutePath: the path (lazy parse).
    pub fn absolute_path(&mut self) -> &[u8] {
        self.ensure_parsed();
        &self.path
    }

    /// IsURLRelative: type == 9 (lazy parse).
    pub fn is_url_relative(&mut self) -> bool {
        self.ensure_parsed();
        self.url_type == RELATIVE
    }

    /// Second IsURLRelative overload: a temporary allowRelative URL built from `text`.
    pub fn is_text_relative(text: &[u8]) -> bool {
        RzUrl::from_text(text).is_url_relative()
    }

    // The spec lists the lazy getters above only; these read the stored field as is.
    pub fn user(&self) -> &[u8] {
        &self.user
    }
    pub fn password(&self) -> &[u8] {
        &self.password
    }
    pub fn port(&self) -> u32 {
        self.port
    }
    pub fn params(&self) -> &[u8] {
        &self.params
    }
    pub fn query(&self) -> &[u8] {
        &self.query
    }
    pub fn fragment(&self) -> &[u8] {
        &self.fragment
    }

    /// ParseURL (spec 2.4 last paragraph): clears every component and the port, runs the
    /// scheme step and the type's route, stores and returns the result.
    pub fn parse_url(&mut self, text: &[u8]) -> bool {
        self.scheme.clear();
        self.user.clear();
        self.password.clear();
        self.server.clear();
        self.port = 0;
        self.path.clear();
        self.params.clear();
        self.query.clear();
        self.fragment.clear();
        let mut rest = text.to_vec();
        self.parse_scheme(&mut rest);
        let ok = match self.url_type {
            HTTP | HTTPS | IRC => {
                self.parse_user_info(&mut rest);
                self.parse_server_port(&mut rest);
                self.parse_hier_path(&mut rest);
                true
            }
            FILE => {
                self.parse_server_port(&mut rest);
                self.parse_file_path(&mut rest);
                true
            }
            FTP => {
                self.parse_user_info(&mut rest);
                self.parse_server_port(&mut rest);
                self.parse_ftp_path(&mut rest);
                true
            }
            MAILTO | NEWS => self.parse_opaque_path(&mut rest),
            OTHER => {
                self.path = rest.clone();
                true
            }
            RELATIVE => {
                self.parse_server_port(&mut rest);
                self.parse_hier_path(&mut rest);
                true
            }
            _ => false,
        };
        self.parsed = ok;
        ok
    }

    /// Scheme step (spec 2.3).
    fn parse_scheme(&mut self, rest: &mut Vec<u8>) {
        self.scheme.clear();
        self.url_type = NONE;
        if rest.starts_with(b"URL:") || rest.starts_with(b"url:") {
            rest.drain(..4);
        }
        if rest.starts_with(b"mailto:") {
            // Quirk: stops with type 0, so ParseURL fails for `mailto:` text.
            self.scheme = b"mailto".to_vec();
            rest.drain(..7);
            return;
        }
        if let Some(i) = find(rest, b"://") {
            self.scheme = rest[..i].to_ascii_lowercase();
            rest.drain(..=i);
        }
        if !self.scheme.is_empty() {
            let s = &self.scheme;
            self.url_type = if s.starts_with(b"http") {
                HTTP
            } else if s.starts_with(b"https") {
                HTTPS
            } else if s.starts_with(b"file") {
                FILE
            } else if s.starts_with(b"ftp") {
                FTP
            } else if s.starts_with(b"mailto") {
                MAILTO
            } else if s.starts_with(b"news") || s.starts_with(b"nntp") {
                NEWS
            } else if s.starts_with(b"irc") {
                IRC
            } else {
                NONE
            };
            if self.url_type != NONE {
                return;
            }
            // Literal reading: an unrecognised `xyz://` scheme keeps its scheme text and
            // falls through to steps 6/7 (question web-network Q6).
        }
        if !self.allow_relative {
            self.url_type = if rest.starts_with(b"www.") {
                HTTP
            } else if rest.starts_with(b"ftp.") {
                FTP
            } else if rest.starts_with(b"news.") || rest.starts_with(b"nntp.") {
                NEWS
            } else if rest.starts_with(b"irc.") {
                IRC
            } else {
                NONE
            };
            rest.splice(0..0, b"//".iter().copied());
            return;
        }
        self.url_type = RELATIVE;
        if let Some(j) = find_byte(rest, b':') {
            if j > 0
                && rest[..j]
                    .iter()
                    .all(|&c| c.is_ascii_alphanumeric() || c == b'+' || c == b'.' || c == b'-')
            {
                self.url_type = OTHER;
                self.scheme = rest[..j].to_vec();
                rest.drain(..=j);
            }
        }
    }

    /// User-info step (spec 2.4, 008226d0).
    fn parse_user_info(&mut self, rest: &mut Vec<u8>) {
        self.user.clear();
        self.password.clear();
        if rest.len() >= 2 && rest.starts_with(b"//") {
            if let Some(k) = find_byte(rest, b'@') {
                // Quirk: the user keeps the leading `//`.
                self.user = rest[..k].to_vec();
                rest.drain(..=k);
                if let Some(c) = find_byte(&self.user, b':') {
                    self.password = self.user[c + 1..].to_vec();
                    self.user.truncate(c);
                }
            }
        }
    }

    /// Server and port step (spec 2.4, 008227e0). Gated on a leading `//` (the 2.5
    /// `images/a.png` example has an empty server).
    fn parse_server_port(&mut self, rest: &mut Vec<u8>) {
        self.server.clear();
        self.port = 0;
        if !rest.starts_with(b"//") {
            return;
        }
        rest.drain(..2);
        if self.url_type == FILE {
            return;
        }
        let colon = find_byte(rest, b':');
        let slash = find_byte(rest, b'/');
        match (colon, slash) {
            (Some(c), s) if s.is_none_or(|s| c < s) => {
                let run = rest[c + 1..].iter().take_while(|b| b.is_ascii_digit()).count();
                // Literal reading: at most 15 digits feed atol; the whole run is erased
                // (longer runs are undefined in the original, question web-network Q5).
                let kept = run.min(MAX_PORT_DIGITS);
                if kept > 0 {
                    self.port = atol_digits(&rest[c + 1..c + 1 + kept]);
                }
                self.server = rest[..c].to_vec();
                rest.drain(..c + run + 1);
            }
            (_, Some(s)) => {
                self.server = rest[..s].to_vec();
                rest.drain(..s);
            }
            _ => {
                if self.url_type == HTTP || self.url_type == HTTPS {
                    for d in [b';', b'?', b'#'] {
                        if !self.server.is_empty() {
                            break;
                        }
                        if let Some(p) = find_byte(rest, d) {
                            self.server = rest[..p].to_vec();
                            rest.drain(..p);
                        }
                    }
                }
                if self.server.is_empty() {
                    self.server = std::mem::take(rest);
                }
            }
        }
    }

    /// Hierarchical path step (spec 2.4, 00822a10).
    fn parse_hier_path(&mut self, rest: &mut Vec<u8>) {
        self.path.clear();
        self.params.clear();
        self.query.clear();
        self.fragment.clear();
        if rest.is_empty() {
            if self.url_type != RELATIVE {
                self.path = b"/".to_vec();
            }
            return;
        }
        let q = find_byte(rest, b'?');
        let h = find_byte(rest, b'#');
        if let Some(sc) = find_byte(rest, b';') {
            if q.is_none_or(|q| sc < q) && h.is_none_or(|h| sc < h) {
                let end = q.or(h).unwrap_or(rest.len());
                self.path = rest[..sc].to_vec();
                self.params = rest[sc..end].to_vec();
                rest.drain(..end);
            }
        }
        let q = find_byte(rest, b'?');
        let h = find_byte(rest, b'#');
        if let Some(q) = q {
            if h.is_none_or(|h| q < h) {
                let end = h.unwrap_or(rest.len());
                if self.path.is_empty() {
                    self.path = rest[..q].to_vec();
                }
                self.query = rest[q..end].to_vec();
                rest.drain(..end);
            }
        }
        if let Some(h) = find_byte(rest, b'#') {
            if self.path.is_empty() {
                self.path = rest[..h].to_vec();
            }
            self.fragment = rest[h..].to_vec();
            rest.clear();
        }
        if self.path.is_empty() {
            self.path = std::mem::take(rest);
        }
    }

    /// File path step (spec 2.4, 00822d50).
    fn parse_file_path(&mut self, rest: &mut Vec<u8>) {
        if self.server == b"localhost" {
            self.server.clear();
        }
        self.params.clear();
        self.query.clear();
        self.fragment.clear();
        self.path = std::mem::take(rest);
    }

    /// FTP path step (spec 2.4, 00822df0).
    fn parse_ftp_path(&mut self, rest: &mut Vec<u8>) {
        self.path.clear();
        self.params.clear();
        self.query.clear();
        self.fragment.clear();
        if rest.is_empty() {
            self.path = b"/".to_vec();
        } else if let Some(k) = find_byte(rest, b';') {
            self.path = rest[..k].to_vec();
            self.params = rest[k..].to_vec();
        } else {
            self.path = rest.clone();
        }
        rest.clear();
    }

    /// Opaque path step (spec 2.4, 00822f30, types 6 and 7).
    fn parse_opaque_path(&mut self, rest: &mut Vec<u8>) -> bool {
        self.path.clear();
        self.params.clear();
        self.query.clear();
        self.fragment.clear();
        self.path = std::mem::take(rest);
        !self.path.is_empty()
    }

    /// MakeURLStringFromComponents (spec 2.6): rebuilds and stores `full`, returns it.
    pub fn make_url_string_from_components(&mut self) -> &[u8] {
        self.ensure_parsed();
        let mut f = self.scheme.clone();
        f.push(b':');
        if self.url_type != MAILTO {
            f.extend_from_slice(b"//");
        }
        if !self.user.is_empty() || !self.password.is_empty() {
            f.extend_from_slice(&self.user);
            if !self.password.is_empty() {
                f.push(b':');
                f.extend_from_slice(&self.password);
            }
            f.push(b'@');
        }
        f.extend_from_slice(&self.server);
        if self.port != 0 {
            f.extend_from_slice(format!(":{}", self.port).as_bytes());
        }
        f.extend_from_slice(&self.path);
        f.extend_from_slice(&self.params);
        f.extend_from_slice(&self.query);
        f.extend_from_slice(&self.fragment);
        self.full = f;
        &self.full
    }

    /// ChangeURL (spec 2.7): resolve `new` against `base`.
    pub fn change_url(&mut self, new: &[u8], base: &[u8]) -> bool {
        self.parsed = false;
        self.url_type = NONE;
        self.port = 0;
        self.full.clear();
        self.scheme.clear();
        self.password.clear();
        self.server.clear();
        self.path.clear();
        if base.is_empty() {
            self.full = new.to_vec();
            return self.parse_url(new);
        }
        if new.is_empty() {
            self.full = base.to_vec();
            return true;
        }
        let mut n = RzUrl::from_text(new);
        let mut b = RzUrl::from_text(base);
        // Literal reading: the two parse results are not checked.
        n.ensure_parsed();
        b.ensure_parsed();
        if !n.scheme.is_empty() {
            self.full = new.to_vec();
            return true;
        }
        n.scheme.clone_from(&b.scheme);
        if n.server.is_empty() {
            n.server.clone_from(&b.server);
        }
        if n.user.is_empty() {
            n.user.clone_from(&b.user);
        }
        if n.password.is_empty() {
            n.password.clone_from(&b.password);
        }
        if n.port == 0 {
            n.port = b.port;
        }
        if n.path.is_empty() {
            n.path.clone_from(&b.path);
            // Literal reading of the chain: each later field is taken from B when the
            // previous one is empty, without testing N's own value (question web-network Q4).
            if n.params.is_empty() {
                n.params.clone_from(&b.params);
                if n.params.is_empty() {
                    n.query.clone_from(&b.query);
                    if n.query.is_empty() {
                        n.fragment.clone_from(&b.fragment);
                    }
                }
            }
        } else if !n.path.starts_with(b"/") {
            let mut joined = match rfind_byte(&b.path, b'/') {
                Some(p) => b.path[..=p].to_vec(),
                None => b.path.clone(),
            };
            joined.extend_from_slice(&n.path);
            n.path = normalize_path(&joined);
        }
        n.make_url_string_from_components();
        *self = n;
        true
    }
}

/// Path normalisation (spec 2.8, 00821890).
pub fn normalize_path(path: &[u8]) -> Vec<u8> {
    let mut p = path.to_vec();
    // Step 1.
    let mut from = 0;
    while let Some(h) = find_from(&p, b"./", from) {
        if !(h > 0 && p[h - 1] == b'.') {
            p.drain(h..h + 2);
        }
        from = h + 1;
    }
    // Step 2.
    if p == b"." || p.ends_with(b"/.") {
        p.pop();
    }
    // Step 3. The length test is made once, before the loop (literal reading).
    if p.len() > 3 {
        while let Some(i) = find(&p, b"/../") {
            if i == 0 {
                p.drain(..3);
            } else {
                match rfind_byte(&p[..i], b'/') {
                    Some(s) => p.drain(s + 1..i + 4),
                    None => p.drain(..i + 4),
                };
            }
        }
    }
    // Step 4. With no earlier `/`, erase from the start, as in step 3 (literal reading).
    if p.len() > 3 && p.ends_with(b"/..") {
        let last = p.len() - 3;
        match rfind_byte(&p[..last], b'/') {
            Some(s) => p.truncate(s + 1),
            None => p.clear(),
        }
    }
    p
}

#[cfg(test)]
mod tests;
