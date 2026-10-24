//! Plain-stream transaction state machine, HTTP wire supplement section 1.
//! The host supplies socket/clock primitives; no TLS is inferred from the URL.
use super::{
    ftp::numeric_host,
    http::{self, Client, Job},
    http_wire::{self as wire, Body, Headers, ReadHost, ReadStep},
    Contract, Unsupported,
};
use crate::rzurl::RzUrl;

pub trait Host: ReadHost + http::Host {
    fn resolve(&mut self, server: &[u8]) -> Option<[u8; 4]>;
    fn connect(&mut self, address: [u8; 4], port: u16) -> bool;
    fn close(&mut self);
    fn send(&mut self, bytes: &[u8]) -> Option<usize>;
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Peer {
    pub initialized: bool,
    pub server: Vec<u8>,
    pub port: u16,
    pub reusable: bool,
}
impl Peer {
    fn close(&mut self, host: &mut impl Host) {
        host.close();
        self.initialized = false;
        self.server.clear();
        self.port = 0;
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub status: u32,
    pub major: u32,
    pub minor: u32,
    pub headers: Headers,
    pub initial: Vec<u8>,
}
fn interrupted(host: &mut impl ReadHost, dequeued: u32, timeout: u32, result: &mut u32) -> bool {
    if host.cancel() {
        host.clear_cancel();
        *result = 7;
        true
    } else if wire::expired(dequeued, timeout, host.now()) {
        *result = 5;
        true
    } else {
        false
    }
}
/// No ceiling; zero successful reads and no-readiness polls keep waiting.
pub fn read_header(
    host: &mut impl ReadHost,
    dequeued: u32,
    timeout: u32,
    skip_informational: bool,
    result: &mut u32,
) -> Option<Response> {
    let mut buffered = Vec::new();
    loop {
        if interrupted(host, dequeued, timeout, result) {
            return None;
        }
        if let Some(end) = buffered.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = &buffered[..end + 2];
            let line_end = head.windows(2).position(|w| w == b"\r\n")?;
            // Literal HTTP/version + decimal-status reading; malformed status
            // spelling is unqualified by the published reader (WWW Q19).
            let mut words = head[..line_end].split(|b| *b == b' ');
            let version = words.next()?.strip_prefix(b"HTTP/")?;
            let dot = version.iter().position(|b| *b == b'.')?;
            let major = super::ftp::atol(&version[..dot]) as u32;
            let minor = super::ftp::atol(&version[dot + 1..]) as u32;
            let status = super::ftp::atol(words.next()?) as u32;
            if status == 0 {
                return None;
            }
            let headers = http::parse_records(&head[line_end + 2..]);
            let initial = buffered[end + 4..].to_vec();
            if skip_informational && (100..200).contains(&status) {
                buffered = initial;
                continue;
            }
            return Some(Response {
                status,
                major,
                minor,
                headers,
                initial,
            });
        }
        if !host.connected() {
            return None;
        }
        match host.receive(1024, 1000) {
            ReadStep::Data(bytes) => buffered.extend(bytes),
            ReadStep::Pending => {}
            ReadStep::Failed | ReadStep::Disconnected => return None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Requeued,
    Complete {
        response: Option<Response>,
        body: Body,
        overflow: Vec<u8>,
    },
}
fn failure(client: &mut Client, host: &mut impl Host, job: &mut Job, peer: &mut Peer) -> Outcome {
    let requeued = wire::retry(client, host, job, peer.reusable);
    peer.close(host);
    if requeued {
        Outcome::Requeued
    } else {
        Outcome::Complete {
            response: None,
            body: Body::default(),
            overflow: Vec::new(),
        }
    }
}
/// One dequeued job, including retry/redirect admission. Requeued jobs receive
/// no completion here. Completed jobs go through Client::final_boundary, which
/// owns the existing cancellation/stop and resource-release contract.
#[allow(clippy::too_many_arguments)]
pub fn execute(
    client: &mut Client,
    host: &mut impl Host,
    job: &mut Job,
    peer: &mut Peer,
    dequeued: u32,
    proxy_header14: Option<&[u8]>,
) -> Contract<Outcome> {
    let timeout = client.timeout;
    let request = client
        .requests
        .get_mut(&job.0[5])
        .ok_or(Unsupported::NativeInputDomain("HTTP request identity"))?;
    let Some(bytes) = wire::request_bytes(request, proxy_header14) else {
        job.0[1] = 2;
        return Ok(Outcome::Complete {
            response: None,
            body: Body::default(),
            overflow: Vec::new(),
        });
    };
    let method = http::method_id(&request.method, u32::MAX);
    let mut url = RzUrl::from_text(&request.url);
    let server = url.server().to_vec();
    let port = url.port() as u16;
    if interrupted(host, dequeued, timeout, &mut job.0[1]) {
        return Ok(failure(client, host, job, peer));
    }
    if !(peer.initialized && host.connected() && peer.port == port && peer.server.eq_ignore_ascii_case(&server)) {
        peer.close(host);
        let Some(address) = numeric_host(&server).or_else(|| host.resolve(&server)) else {
            job.0[1] = 3;
            return Ok(Outcome::Complete {
                response: None,
                body: Body::default(),
                overflow: Vec::new(),
            });
        };
        if !host.connect(address, port) {
            job.0[1] = 4;
            return Ok(Outcome::Complete {
                response: None,
                body: Body::default(),
                overflow: Vec::new(),
            });
        }
        peer.initialized = true;
        peer.server = server;
        peer.port = port;
    }
    let mut sent = 0;
    while sent < bytes.len() {
        if interrupted(host, dequeued, timeout, &mut job.0[1]) {
            return Ok(failure(client, host, job, peer));
        }
        match host.send(&bytes[sent..]) {
            Some(n) if n <= bytes.len() - sent => sent += n,
            Some(_) => return Err(Unsupported::PrimitiveContract("HTTP send exceeds remaining bytes")),
            None => return Ok(failure(client, host, job, peer)),
        }
    }
    let Some(response) = read_header(host, dequeued, timeout, false, &mut job.0[1]) else {
        return Ok(failure(client, host, job, peer));
    };
    peer.reusable = wire::reusable(response.major, response.minor);
    let mut body = Body::default();
    body.result = job.0[1];
    let mut overflow = Vec::new();
    let count = body.read_message(
        host,
        wire::framing(method, response.status, &response.headers),
        &response.initial,
        &client.suffix,
        dequeued,
        timeout,
        &mut overflow,
    );
    job.0[1] = body.result;
    if count == 0 {
        return Ok(failure(client, host, job, peer));
    }
    if wire::finalize(client, host, job, response.status, &response.headers)? {
        return Ok(Outcome::Requeued);
    }
    Ok(Outcome::Complete {
        response: Some(response),
        body,
        overflow,
    })
}
