//! Request wire and redirect policy from web-network-http-wire §§1,3.
use super::{
    http::{method_id, Client, Effect, Host, Job, Request},
    Contract,
};
use crate::rzurl::RzUrl;
pub type Headers = Vec<(Vec<u8>, Vec<u8>)>;
pub fn header<'a>(headers: &'a Headers, name: &[u8]) -> Option<&'a [u8]> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_slice())
}
pub fn erase(headers: &mut Headers, name: &[u8]) {
    if let Some(i) = headers.iter().position(|(n, _)| n.eq_ignore_ascii_case(name)) {
        headers.remove(i);
    }
}
/// Preserves a supplied raw header; body bytes are concatenated without encoding.
pub fn request_bytes(request: &mut Request, proxy: Option<&[u8]>) -> Option<Vec<u8>> {
    if request.raw_header.is_empty() {
        let method = method_id(&request.method, u32::MAX);
        if method == 0 {
            return None;
        }
        let mut url = RzUrl::from_text(&request.url);
        let server = url.server().to_vec();
        let port = url.port();
        let mut target = Vec::new();
        if let Some(header14) = proxy {
            target.extend(b"http://");
            target.extend(&server);
            if port != 0 {
                target.extend(format!(":{port}").as_bytes());
            }
            // Literal ID-14 erasure; the foreign header-name table supplies its bytes.
            erase(&mut request.headers, header14);
        } else if header(&request.headers, b"Host").is_none() {
            request.headers.push((b"Host".to_vec(), server));
        }
        if method == 1 {
            if proxy.is_none() {
                target.push(b'*');
            }
        } else {
            target.extend(url.absolute_path());
            target.extend(url.params());
            target.extend(url.query());
            target.extend(url.fragment());
        }
        let mut bytes = request.method.clone();
        bytes.push(b' ');
        bytes.extend(target);
        bytes.extend(b" HTTP/1.1\r\n");
        for (name, value) in &request.headers {
            bytes.extend(name);
            bytes.extend(b": ");
            bytes.extend(value);
            bytes.extend(b"\r\n");
        }
        bytes.extend(b"\r\n");
        request.raw_header = bytes;
    }
    Some([request.raw_header.as_slice(), request.body.as_slice()].concat())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Framing {
    pub length: u32,
    pub chunked: bool,
}
pub fn framing(method: u32, status: u32, headers: &Headers) -> Framing {
    let chunked =
        header(headers, b"Transfer-Encoding").is_some_and(|v| v.split(|b| *b == 0).next() == Some(b"chunked"));
    let mut length = header(headers, b"Content-Length").map_or(0, |v| super::cookies::decimal(v) as u32);
    if length == 0
        && !chunked
        && matches!(method, 2 | 4)
        && (200..300).contains(&status)
        && header(headers, b"Connection").map_or(true, |v| v.starts_with(b"close"))
    {
        length = 0x7fffffff;
    }
    Framing { length, chunked }
}
pub fn expired(dequeued: u32, timeout: u32, now: u32) -> bool {
    dequeued.wrapping_add(timeout) < now
}
pub fn reusable(major: u32, minor: u32) -> bool {
    !(major < 2 && (major == 0 || minor == 0))
}
/// Mutates the supplied job exactly once; callers do not complete a requeued job.
pub fn retry(client: &mut Client, host: &mut impl Host, job: &mut Job, reuse: bool) -> bool {
    if job.0[3] < 3 && reuse && !matches!(job.0[1], 5 | 7) {
        job.0[3] = job.0[3].wrapping_add(1);
        job.0[1] = 1;
        job.0[4] = 0;
        host.effect(Effect::Lock);
        client.append(host, *job);
        host.effect(Effect::Unlock);
        true
    } else {
        if job.0[1] == 1 {
            job.0[1] = 6;
        }
        false
    }
}
/// Request identity is already validated by the worker; no extra request retain.
pub fn finalize(
    client: &mut Client,
    host: &mut impl Host,
    job: &mut Job,
    status: u32,
    response: &Headers,
) -> Contract<bool> {
    let Some(request) = client.requests.get_mut(&job.0[5]) else {
        job.0[1] = 8;
        return Ok(false);
    };
    let method = method_id(&request.method, u32::MAX);
    if (200..300).contains(&status) {
        job.0[1] = 0;
        return Ok(false);
    }
    if matches!(method, 2..=4) && (300..=303).contains(&status) && request.redirects < 5 {
        let location = if method == 4 {
            header(&request.headers, b"Location")
        } else {
            header(response, b"Location")
        };
        if let Some(location) = location {
            let next = if method == 4 {
                location.to_vec()
            } else {
                let mut url = RzUrl::new();
                url.change_url(location, &request.url);
                url.full_url().to_vec()
            };
            if request.redirects == 0 {
                request.original_url = request.url.clone();
            }
            request.redirects += 1;
            request.url = next;
            if method != 4 {
                request.raw_header.clear();
            }
            job.0[1] = 1;
            job.0[3] = job.0[3].wrapping_add(1);
            job.0[4] = 0;
            host.effect(Effect::Lock);
            client.append(host, *job);
            host.effect(Effect::Unlock);
            return Ok(true);
        }
    }
    job.0[1] = if matches!(method, 2..=5) && status == 401 { 9 } else { 8 };
    Ok(false)
}

/// Socket readiness/receive and temp-file operations are foreign primitives.
/// Pending and zero-byte reads deliberately do not signal EOF.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadStep {
    Data(Vec<u8>),
    Pending,
    Failed,
    Disconnected,
}
pub trait ReadHost {
    fn now(&mut self) -> u32;
    fn cancel(&mut self) -> bool;
    fn clear_cancel(&mut self);
    fn connected(&mut self) -> bool;
    fn receive(&mut self, capacity: usize, poll_timeout: u32) -> ReadStep;
    fn temp_path(&mut self, extension: &[u8]) -> Option<Vec<u8>>;
    fn open_temp(&mut self, path: &[u8], args: (u32, u32, u32)) -> bool;
    fn write_temp(&mut self, bytes: &[u8]) -> bool;
    fn trim_temp(&mut self, bytes: u32);
    fn close_temp(&mut self);
    fn remove_temp(&mut self, path: &[u8]);
    fn progress(&mut self, body: u32, length: u32);
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub bytes: Vec<u8>,
    pub file: bool,
    pub result: u32,
    pub received: u32,
    pub count: u32,
    path: Option<Vec<u8>>,
    open: bool,
}
impl Body {
    fn append(&mut self, host: &mut impl ReadHost, bytes: &[u8], extension: &[u8], checked: bool) -> bool {
        if !self.file && self.bytes.len().wrapping_add(bytes.len()) > 16384 {
            let Some(path) = host.temp_path(extension) else {
                return false;
            };
            self.file = true;
            self.path = Some(path.clone());
            if !host.open_temp(&path, (2, 3, 1)) {
                return false;
            }
            self.open = true;
            if !host.write_temp(&self.bytes) {
                return false;
            }
            self.bytes = path;
        }
        if self.file {
            let written = host.write_temp(bytes);
            if checked && !written {
                return false;
            }
        } else {
            self.bytes.extend(bytes);
        }
        self.count = self.count.wrapping_add(bytes.len() as u32);
        true
    }
    fn cleanup(&mut self, host: &mut impl ReadHost) {
        if self.open {
            host.close_temp();
            self.open = false;
        }
        if !self.file {
            if let Some(path) = self.path.take() {
                host.remove_temp(&path);
            }
        }
    }
    fn io_error(&mut self, host: &mut impl ReadHost) -> u32 {
        self.file = false;
        self.bytes.clear();
        self.count = 0;
        self.cleanup(host);
        0
    }
    fn interrupted(&mut self, host: &mut impl ReadHost, dequeued: u32, timeout: u32) -> bool {
        if host.cancel() {
            host.clear_cancel();
            self.result = 7;
            self.file = false;
            self.bytes.clear();
            return true;
        }
        if expired(dequeued, timeout, host.now()) {
            self.result = 5;
            return true;
        }
        false
    }
    fn receive(&mut self, host: &mut impl ReadHost) -> ReadStep {
        let step = host.receive(1024, 1000);
        if let ReadStep::Data(bytes) = &step {
            self.received = self.received.wrapping_add(bytes.len() as u32);
        }
        step
    }
    /// Returns count, including oversized initial fixed-length buffers; zero is failure.
    #[allow(clippy::too_many_arguments)]
    pub fn read_message(
        &mut self,
        host: &mut impl ReadHost,
        mode: Framing,
        initial: &[u8],
        extension: &[u8],
        dequeued: u32,
        timeout: u32,
        overflow: &mut Vec<u8>,
    ) -> u32 {
        overflow.clear();
        self.bytes.clear();
        self.file = false;
        self.count = 0;
        self.path = None;
        self.open = false;
        if !mode.chunked {
            if mode.length == 0 {
                return 0;
            }
            self.bytes = initial.to_vec();
            self.count = initial.len() as u32;
            if self.count > mode.length {
                overflow.extend(&initial[mode.length as usize..]);
            }
            while self.count < mode.length {
                if self.interrupted(host, dequeued, timeout) {
                    self.cleanup(host);
                    return 0;
                }
                match self.receive(host) {
                    ReadStep::Pending => continue,
                    ReadStep::Disconnected => {
                        self.cleanup(host);
                        return 0;
                    }
                    ReadStep::Failed if mode.length == 0x7fffffff => break,
                    ReadStep::Failed => return self.io_error(host),
                    ReadStep::Data(bytes) => {
                        let take = bytes.len().min(mode.length.wrapping_sub(self.count) as usize);
                        if !self.append(host, &bytes[..take], extension, true) {
                            return self.io_error(host);
                        }
                        overflow.extend(&bytes[take..]);
                        host.progress(self.count, mode.length);
                    }
                }
            }
            self.cleanup(host);
            return self.count;
        }
        let mut wire = initial.to_vec();
        loop {
            if self.interrupted(host, dequeued, timeout) {
                self.cleanup(host);
                return 0;
            }
            if !host.connected() {
                self.cleanup(host);
                return 0;
            }
            let Some(line) = wire.windows(2).position(|w| w == b"\r\n") else {
                match self.receive(host) {
                    ReadStep::Data(bytes) => wire.extend(bytes),
                    ReadStep::Pending => {}
                    _ => return self.io_error(host),
                }
                continue;
            };
            let mut digits = wire[..line].iter().map_while(|b| (*b as char).to_digit(16));
            let Some(first) = digits.next() else {
                self.cleanup(host);
                return 0;
            };
            let size = digits.fold(first, |v, d| v.wrapping_mul(16).wrapping_add(d));
            if size == 0 {
                wire.drain(..line); // Keep size-line CRLF for the CRLFCRLF search.
                loop {
                    if self.interrupted(host, dequeued, timeout) {
                        self.cleanup(host);
                        return 0;
                    }
                    if !host.connected() {
                        self.cleanup(host);
                        return 0;
                    }
                    if wire.windows(4).any(|w| w == b"\r\n\r\n") {
                        self.cleanup(host);
                        return self.count;
                    }
                    match self.receive(host) {
                        ReadStep::Data(bytes) => wire.extend(bytes),
                        ReadStep::Pending => {}
                        _ => return self.io_error(host),
                    }
                }
            }
            wire.drain(..line + 2);
            let mut remaining = size.wrapping_add(2);
            let take = wire.len().min(remaining as usize);
            if !self.append(host, &wire[..take], extension, false) {
                return self.io_error(host);
            }
            wire.drain(..take);
            remaining -= take as u32;
            while remaining != 0 {
                if self.interrupted(host, dequeued, timeout) {
                    self.cleanup(host);
                    return 0;
                }
                match self.receive(host) {
                    ReadStep::Pending => continue,
                    ReadStep::Data(bytes) => {
                        let take = bytes.len().min(remaining as usize);
                        if !self.append(host, &bytes[..take], extension, true) {
                            return self.io_error(host);
                        }
                        remaining -= take as u32;
                        wire.extend(&bytes[take..]);
                    }
                    _ => return self.io_error(host),
                }
            }
            if self.file {
                host.trim_temp(2);
            } else {
                self.bytes.truncate(self.bytes.len().saturating_sub(2));
            }
            self.count = self.count.wrapping_sub(2);
            host.progress(self.count, mode.length);
        }
    }
}
