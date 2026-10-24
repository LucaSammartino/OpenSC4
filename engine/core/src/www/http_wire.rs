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
    if job.0[2] < 3 && reuse && !matches!(job.0[1], 5 | 7) {
        job.0[2] = job.0[2].wrapping_add(1);
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
