// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded portable owner algorithms from `docs/specs/html-ui-hooks.md` §§2–8.
//!
//! Inputs are GetScheme/GetFullURL results, not URLs to parse. Primitive providers
//! must implement only the named foreign operations; they never decide whether a
//! document loads or a link dispatches. No filesystem, network, OS, or game calls.
//! HU1–4 remain open: non-ASCII/embedded NUL, unspecified hex conversions, actual
//! foreign service semantics, invalid memory, allocation failures, registration
//! retention and reentrant/mutating callbacks have no native conformance claim.
//! Callbacks receive values/snapshots, never a mutable hook. Logical allocation
//! and view tokens establish identity only, not pointer arithmetic or ABI layout.

/// Portable allocation bound (policy, not a native failure threshold; HU3).
pub const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    EncodingHu1,
    HexConversionHu1,
    NullServiceHu3,
    NullSegmentHu3,
    AllocationHu3,
    DestroyedHu3,
    ReentryOrRegistrationHu4,
}

/// Full primitive return register; only AL is a boolean/dispatch result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Register(pub u32);
impl Register {
    pub fn al(self) -> u8 {
        self.0 as u8
    }
}

fn bytes_domain(bytes: &[u8]) -> Result<(), Unsupported> {
    if bytes.len() > MAX_BYTES {
        return Err(Unsupported::AllocationHu3);
    }
    if !bytes.is_ascii() || bytes.contains(&0) {
        return Err(Unsupported::EncodingHu1);
    }
    Ok(())
}

/// §3: ASCII case-insensitive *equality* on the raw scheme primitive.
pub fn test_url(raw_scheme: &[u8]) -> Result<bool, Unsupported> {
    bytes_domain(raw_scheme)?;
    Ok(raw_scheme.eq_ignore_ascii_case(b"sc4"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub resource_type: u32,
    pub group: u32,
    pub instance: u32,
}

pub trait HexPrimitive {
    /// Exact raw buffer passed to RZAtoUint32HexA. Malformed/overflow results
    /// require an explicit supplied primitive outcome; do not infer them.
    fn convert(&mut self, input: &[u8]) -> Result<u32, Unsupported>;
}

/// Optional supported ordinary hex primitive. No prefix, whitespace, suffix,
/// overflow or malformed-input semantics are asserted (HU1).
pub struct ValidHex;
impl HexPrimitive for ValidHex {
    fn convert(&mut self, input: &[u8]) -> Result<u32, Unsupported> {
        if input.is_empty() || input.len() > 8 || !input.iter().all(u8::is_ascii_hexdigit) {
            return Err(Unsupported::HexConversionHu1);
        }
        Ok(input.iter().fold(0u32, |value, &c| {
            let digit = if c.is_ascii_digit() {
                c - b'0'
            } else {
                c.to_ascii_lowercase() - b'a' + 10
            };
            (value << 4) | u32::from(digit)
        }))
    }
}

/// §4 token primitive: skip leading separators; remove token plus following
/// separator run only when a separator is present. In particular no-separator
/// input is copied as a token without clearing the source.
fn split_token(source: &mut Vec<u8>, separator: u8) -> Option<Vec<u8>> {
    let start = source.iter().position(|&c| c != separator)?;
    let end = source[start..]
        .iter()
        .position(|&c| c == separator)
        .map(|n| n + start);
    let token_end = end.unwrap_or(source.len());
    let token = source[start..token_end].to_vec();
    if let Some(end) = end {
        let following = source[end..]
            .iter()
            .position(|&c| c != separator)
            .map_or(source.len(), |n| n + end);
        source.drain(..following);
    } else if start != 0 {
        // A leading separator is still a separator-present input (§4).
        source.clear();
    }
    Some(token)
}

/// §4: own key stores/selection; foreign conversion receives the exact token or
/// whole remaining buffer. No scheme parsing or query/fragment normalization.
pub fn select_key(
    raw_full_url: &[u8],
    hex: &mut impl HexPrimitive,
) -> Result<Option<ResourceKey>, Unsupported> {
    bytes_domain(raw_full_url)?;
    if !raw_full_url.starts_with(b"sc4://") {
        return Ok(None);
    }
    let mut remaining = raw_full_url[6..].to_vec();
    let Some(label) = split_token(&mut remaining, b'/') else {
        return Ok(None);
    };
    let mut key = ResourceKey {
        resource_type: 0x856DDBAC,
        group: 0,
        instance: 0,
    };
    if remaining.contains(&b'/') {
        if let Some(token) = split_token(&mut remaining, b'/') {
            key.group = hex.convert(&token)?;
        }
    } else if label.eq_ignore_ascii_case(b"html") {
        key.group = 0x0A133FC0;
    } else if label.eq_ignore_ascii_case(b"image") {
        key.group = 0xCA133ECB;
    }
    if remaining.len() >= 8 {
        key.instance = hex.convert(&remaining)?;
    }
    Ok(Some(key))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Service(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment(pub u64);
pub struct Acquisition {
    pub result: Register,
    pub segment: Option<Segment>,
}

pub trait ResourcePrimitives {
    fn acquire(&mut self, service: Service, key: ResourceKey) -> Acquisition;
    fn read_size(&mut self, segment: Segment, key: ResourceKey, size: &mut u32) -> u32;
    fn read_bytes(
        &mut self,
        segment: Segment,
        key: ResourceKey,
        output: &mut [u8],
        size: &mut u32,
    ) -> u32;
    fn release_segment(&mut self, segment: Segment);
}

/// §5: two reads, EAX overwrites first size pointer, exact sentinel, resize with
/// prefix preservation and fill only on growth, ignore second result/size.
/// Unsupported terminates a partial trace; it is neither native false nor a
/// promise of native cleanup/rollback after an unsupported allocation (HU3).
pub fn read_resource(
    service: Option<Service>,
    key: ResourceKey,
    output: &mut Vec<u8>,
    foreign: &mut impl ResourcePrimitives,
) -> Result<bool, Unsupported> {
    let service = service.ok_or(Unsupported::NullServiceHu3)?;
    let acquisition = foreign.acquire(service, key);
    if acquisition.result.al() == 0 {
        return Ok(false);
    }
    let segment = acquisition.segment.ok_or(Unsupported::NullSegmentHu3)?;
    let mut size = 0;
    size = foreign.read_size(segment, key, &mut size);
    if size == u32::MAX {
        foreign.release_segment(segment);
        return Ok(false);
    }
    let length = usize::try_from(size).map_err(|_| Unsupported::AllocationHu3)?;
    if length > MAX_BYTES || output.len() > MAX_BYTES {
        return Err(Unsupported::AllocationHu3);
    }
    output
        .try_reserve(length.saturating_sub(output.len()))
        .map_err(|_| Unsupported::AllocationHu3)?;
    output.resize(length, 0);
    let _ = foreign.read_bytes(segment, key, output, &mut size);
    foreign.release_segment(segment);
    Ok(true)
}

pub trait LinkPrimitives {
    /// Observation of owned action-local processing, never command execution.
    fn action_temporary(&mut self, erased: &[u8], token: Option<&[u8]>, remaining: &[u8]);
    fn destroy_action_temporaries(&mut self);
    /// Includes raw advisor retrieval/DoLink; return full EAX for AL extraction.
    fn advisor_do_link(&mut self, payload: &[u8]) -> Register;
}

fn link_gate(input: &[u8], word: &[u8]) -> bool {
    input.len() >= 6
        && input.windows(word.len()).position(|part| part == word) == Some(6)
        && input.len() >= 12
        && input.get(12) == Some(&b'/')
}

/// §6: first exact action/advice occurrence at 6; arbitrary first six bytes.
/// Action locals are destroyed before false. Advice forwards the whole remainder.
pub fn do_url(raw_full_url: &[u8], foreign: &mut impl LinkPrimitives) -> Result<u8, Unsupported> {
    bytes_domain(raw_full_url)?;
    if link_gate(raw_full_url, b"action") {
        let erased = &raw_full_url[13..];
        let mut remaining = erased.to_vec();
        let token = split_token(&mut remaining, b'?');
        foreign.action_temporary(erased, token.as_deref(), &remaining);
        drop(token);
        drop(remaining);
        foreign.destroy_action_temporaries();
        return Ok(0);
    }
    if link_gate(raw_full_url, b"advice") {
        return Ok(foreign.advisor_do_link(&raw_full_url[13..]).al());
    }
    Ok(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    Base,
    Hook,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    pub allocation: Allocation,
    pub kind: ViewKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Framework(pub u64);
pub struct Lookup {
    pub result: Register,
    pub service: Option<Service>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifeEvent {
    AddRef(Service),
    Store(Option<Service>),
    Release {
        service: Service,
        retained: Option<Service>,
    },
    RestoreTables,
    BaseDestroy(Allocation),
    Free(Allocation),
}
pub trait LifecyclePrimitives {
    fn lookup(&mut self, framework: Framework, class: u32, iid: u32) -> Lookup;
    /// Nonmutating primitive callback; retained is a snapshot at this exact call.
    fn event(&mut self, event: LifeEvent);
}

/// Logical live object. Deliberately not Clone and no automatic Drop service
/// calls: only the specified explicit lifecycle owns those events. Host object
/// ownership/registration and reentry are unsupported (HU4).
pub struct Hook {
    allocation: Allocation,
    count: u32,
    retained: Option<Service>,
    destroyed: bool,
}
impl Hook {
    pub fn new(allocation: Allocation) -> Self {
        Self {
            allocation,
            count: 0,
            retained: None,
            destroyed: false,
        }
    }
    fn live(&self) -> Result<(), Unsupported> {
        if self.destroyed {
            Err(Unsupported::DestroyedHu3)
        } else {
            Ok(())
        }
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn retained(&self) -> Option<Service> {
        self.retained
    }
    pub fn add_ref(&mut self) -> Result<u32, Unsupported> {
        self.live()?;
        self.count = self.count.wrapping_add(1);
        Ok(self.count)
    }
    pub fn remove_ref(&mut self) -> Result<u32, Unsupported> {
        self.live()?;
        if self.count != 0 {
            self.count -= 1;
        }
        Ok(self.count)
    }
    pub fn query_interface(&mut self, iid: u32, output: &mut View) -> Result<bool, Unsupported> {
        self.live()?;
        if iid != 1 && iid != 0xC49B192C {
            return Ok(false);
        }
        *output = View {
            allocation: self.allocation,
            kind: ViewKind::Hook,
        };
        self.add_ref()?;
        Ok(true)
    }
    fn release_service(&self, service: Service, foreign: &mut impl LifecyclePrimitives) {
        foreign.event(LifeEvent::Release {
            service,
            retained: self.retained,
        });
    }
    pub fn init(
        &mut self,
        framework: Option<Framework>,
        foreign: &mut impl LifecyclePrimitives,
    ) -> Result<bool, Unsupported> {
        self.live()?;
        let temporary = framework
            .and_then(|framework| foreign.lookup(framework, 0x056B906E, 0x656B8EFC).service);
        if temporary != self.retained {
            if let Some(service) = temporary {
                foreign.event(LifeEvent::AddRef(service));
            }
            let old = self.retained;
            self.retained = temporary;
            foreign.event(LifeEvent::Store(temporary));
            if let Some(service) = old {
                self.release_service(service, foreign);
            }
        }
        if let Some(service) = temporary {
            self.release_service(service, foreign);
        }
        Ok(true)
    }
    pub fn shutdown(
        &mut self,
        foreign: &mut impl LifecyclePrimitives,
    ) -> Result<bool, Unsupported> {
        self.live()?;
        if let Some(service) = self.retained.take() {
            foreign.event(LifeEvent::Store(None));
            self.release_service(service, foreign);
        }
        Ok(true)
    }
    pub fn destroy(
        &mut self,
        flag: u32,
        foreign: &mut impl LifecyclePrimitives,
    ) -> Result<View, Unsupported> {
        self.live()?;
        foreign.event(LifeEvent::RestoreTables);
        if let Some(service) = self.retained {
            self.release_service(service, foreign);
        }
        foreign.event(LifeEvent::BaseDestroy(self.allocation));
        self.destroyed = true;
        if flag & 1 != 0 {
            foreign.event(LifeEvent::Free(self.allocation));
        }
        Ok(View {
            allocation: self.allocation,
            kind: ViewKind::Base,
        })
    }
    pub fn release(&mut self, foreign: &mut impl LifecyclePrimitives) -> Result<u32, Unsupported> {
        self.live()?;
        if self.count > 1 {
            self.count -= 1;
            return Ok(self.count);
        }
        self.destroy(1, foreign)?;
        Ok(0)
    }
    pub fn load_document(
        &self,
        raw_full_url: &[u8],
        output: &mut Vec<u8>,
        hex: &mut impl HexPrimitive,
        foreign: &mut impl ResourcePrimitives,
    ) -> Result<bool, Unsupported> {
        self.live()?;
        let Some(key) = select_key(raw_full_url, hex)? else {
            return Ok(false);
        };
        read_resource(self.retained, key, output, foreign)
    }
}

#[cfg(test)]
#[path = "html_ui_hooks_tests.rs"]
mod tests;
