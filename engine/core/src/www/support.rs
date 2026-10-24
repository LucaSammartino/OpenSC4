//! Qualified small WWW support bodies, web-network section 4.15.
use std::collections::BTreeSet;

pub trait Host {
    fn effect(&mut self, effect: Effect);
    fn progress(&mut self, recipient: u32, record: &[u32; 3]);
    fn remove_lookup(&mut self, id: u32) -> bool;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    LockNames,
    UnlockNames,
    FreeNameSentinel,
    FreeName,
    CreateMessage,
    AddMessage,
    ReleaseMessage,
}
/// Synchronous borrowed progress; never retain the stack record.
pub fn forward_progress(host: &mut impl Host, listener: bool, current_job: bool, recipient: Option<u32>, words: [u32; 2]) -> bool {
    if listener && current_job {
        if let Some(recipient) = recipient.filter(|r| *r != 0) {
            let record = [words[0], words[1], if words[1] == 0 { 50 } else { 1 }];
            host.effect(Effect::CreateMessage);
            host.effect(Effect::AddMessage);
            host.progress(recipient, &record);
            host.effect(Effect::ReleaseMessage);
        }
    }
    true
}
/// The CRT formatter result and its bounded temporary are primitive inputs.
pub fn formatted(destination: &mut Vec<u8>, temporary: &[u8; 1024], result: i32) -> i32 {
    destination.clear();
    if (0..1024).contains(&result) { destination.extend_from_slice(&temporary[..result as usize]); }
    result
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub initialized: bool,
    pub references: u32,
    pub state: u32,
    pub settings: [u32; 2],
    pub ids: BTreeSet<u32>,
}
impl Default for Name {
    fn default() -> Self {
        Self { initialized: false, references: 0, state: 0, settings: [10, 1800], ids: BTreeSet::new() }
    }
}
impl Name {
    pub fn remove_ids(host: &mut impl Host, ids: &BTreeSet<u32>) -> u32 {
        host.effect(Effect::LockNames);
        let count = ids.iter().filter(|id| host.remove_lookup(**id)).count() as u32;
        host.effect(Effect::UnlockNames);
        count
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> bool {
        if self.initialized {
            Self::remove_ids(host, &self.ids);
            self.ids.clear();
            self.initialized = false;
        }
        true
    }
    pub fn destroy(&mut self, host: &mut impl Host, flags: u32) {
        self.shutdown(host);
        self.ids.clear();
        host.effect(Effect::FreeNameSentinel);
        if flags & 1 != 0 { host.effect(Effect::FreeName); }
    }
    pub fn release(&mut self, host: &mut impl Host) -> u32 {
        if self.references > 1 { self.references -= 1; return self.references; }
        self.destroy(host, 1);
        0
    }
}
pub fn address_byte(address: u32, index: usize) -> u8 { address.to_le_bytes()[index] }
pub fn port_byte(port: u16, index: usize) -> u8 { (port as u32).to_le_bytes()[index] }
pub fn raw_port(port: u16) -> u16 { port }
pub fn false_stub() -> bool { false }

pub trait DrainHost {
    fn open(&mut self) -> bool;
    fn blocking(&mut self, value: bool);
    fn connected(&mut self) -> bool;
    fn readable(&mut self, timeout: u32) -> bool;
    /// Signed native count retained separately for the -1 logging quirk.
    fn receive(&mut self, capacity: usize) -> Option<(i32, Vec<u8>)>;
    fn log(&mut self, bytes: &[u8], count: usize);
    fn close(&mut self);
}
pub fn drain_close(host: &mut impl DrainHost, counter: &mut u32, logging: bool) {
    if !host.open() { return; }
    host.blocking(true);
    while host.connected() && host.readable(0) {
        let Some((count, bytes)) = host.receive(1024) else { break; };
        if count == 0 { break; }
        *counter = counter.wrapping_add(count as u32);
        if logging {
            let length = if count == -1 { bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len()) } else { count as usize };
            host.log(&bytes, length);
        }
    }
    host.close();
}
