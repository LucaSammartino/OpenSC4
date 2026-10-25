//! Plain-stream transaction state machine, HTTP wire supplement section 1.
//! The host supplies socket/clock primitives; no TLS is inferred from the URL.
use super::{
    ftp::numeric_host,
    http::{self, Client, Job, Resource},
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
/// Foreign proxy settings. Endpoint fields are deliberately not connect inputs:
/// Q20 specifies absolute-form formatting sent to the request URL's origin.
#[derive(Clone, Copy, Debug)]
pub struct Proxy<'a> {
    pub server: &'a [u8],
    pub port: u16,
    pub header14: &'a [u8],
}
pub fn execute_with_proxy(
    client: &mut Client,
    host: &mut impl Host,
    job: &mut Job,
    peer: &mut Peer,
    dequeued: u32,
    proxy: Option<Proxy<'_>>,
) -> Contract<Outcome> {
    execute(client, host, job, peer, dequeued, proxy.map(|p| p.header14))
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Peer {
    pub initialized: bool,
    pub server: Vec<u8>,
    pub port: u16,
    pub reusable: bool,
    pub received: Vec<u8>,
}
impl Peer {
    fn close(&mut self, host: &mut impl Host) {
        host.close();
        self.initialized = false;
        self.server.clear();
        self.port = 0;
        self.received.clear();
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub status: i32,
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
    read_header_into(
        host,
        dequeued,
        timeout,
        skip_informational,
        result,
        &mut Resource::default(),
    )
}
fn read_header_into(
    host: &mut impl ReadHost,
    dequeued: u32,
    timeout: u32,
    skip_informational: bool,
    result: &mut u32,
    resource: &mut Resource,
) -> Option<Response> {
    loop {
        if interrupted(host, dequeued, timeout, result) {
            return None;
        }
        if let Some(end) = resource.raw_header.windows(4).position(|w| w == b"\r\n\r\n") {
            resource.body = resource.raw_header[end + 4..].to_vec();
            resource.raw_header.truncate(end + 4);
            resource.headers = http::parse_records(&resource.raw_header);
            let status = resource.status_in_locale(|b| host.status_whitespace(b));
            if status == 0 {
                return None;
            }
            if skip_informational && (100..200).contains(&status) {
                resource.raw_header = std::mem::take(&mut resource.body);
                continue;
            }
            let (major, minor) = resource.version();
            return Some(Response {
                status,
                major: major as u32,
                minor: minor as u32,
                headers: resource.headers.iter().skip(1).cloned().collect(),
                initial: resource.body.clone(),
            });
        }
        if !host.connected() {
            return None;
        }
        match host.receive(1024, 1000) {
            ReadStep::Data(bytes) => resource.raw_header.extend(bytes),
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
        if !host.connect(address, if port == 0 { 80 } else { port }) {
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
    if interrupted(host, dequeued, timeout, &mut job.0[1]) {
        return Ok(failure(client, host, job, peer));
    }
    // A distinct held response is allocated after send, not cast from the request.
    // Failed acquisition deliberately leaves this unpublished hold alive (Q20).
    let response_id = client.create_response(host);
    let resource = client.resources.get_mut(&response_id).expect("new response");
    resource.raw_header = std::mem::take(&mut peer.received);
    let Some(response) = read_header_into(host, dequeued, timeout, false, &mut job.0[1], resource) else {
        return Ok(failure(client, host, job, peer));
    };
    peer.reusable = wire::reusable(response.major, response.minor);
    let mut body = Body::default();
    body.result = job.0[1];
    let mut overflow = Vec::new();
    let mode = wire::framing(method, response.status as u32, &response.headers);
    let read_body = mode.chunked || mode.length != 0;
    let count = if read_body {
        body.read_message(
            host,
            mode,
            &response.initial,
            &client.suffix,
            dequeued,
            timeout,
            &mut overflow,
        )
    } else {
        // Zero/missing length skips ReadMessage, not finalization (wire §2).
        body.bytes = response.initial.clone();
        0
    };
    job.0[1] = body.result;
    let resource = client.resources.get_mut(&response_id).expect("local response");
    resource.body = body.bytes.clone();
    resource.file = body.file;
    if read_body && count == 0 {
        return Ok(failure(client, host, job, peer));
    }
    // Publish only at finalization. Overwriting a previous response does not Release it.
    job.0[6] = response_id;
    host.effect(http::Effect::PublishResponse(response_id));
    peer.received = overflow.clone();
    if wire::finalize(client, host, job, response.status as u32, &response.headers)? {
        return Ok(Outcome::Requeued);
    }
    Ok(Outcome::Complete {
        response: Some(response),
        body,
        overflow,
    })
}

/// Execute the published current record and connect transport to completion/lifetime.
/// The caller uses Client::dequeue for each attempt, including retries and redirects.
pub fn run_current(
    client: &mut Client,
    host: &mut impl Host,
    worker: u32,
    peer: &mut Peer,
    proxy_header14: Option<&[u8]>,
) -> Contract<Outcome> {
    let mut job = client
        .workers
        .get(&worker)
        .and_then(|r| r.current)
        .ok_or(Unsupported::NativeInputDomain("HTTP current job"))?;
    let dequeued = job.0[2];
    let outcome = execute(client, host, &mut job, peer, dequeued, proxy_header14)?;
    client.workers.get_mut(&worker).expect("current worker").current = Some(job);
    if matches!(outcome, Outcome::Requeued) {
        client.workers.get_mut(&worker).expect("current worker").current = None;
    } else {
        client.final_boundary(host, worker)?;
    }
    Ok(outcome)
}
