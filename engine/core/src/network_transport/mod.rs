//! Portable socket/stream owner for `docs/specs/network-transport.md` §§1–6.
//!
//! This is a logical Rust API, not a native layout, COM binding, factory, or OS
//! adapter. Hosts perform individual raw operations only. Calls must be normal,
//! nonreentrant, with valid nonaliasing storage and literal well-formed replies.
//! No sockets, threads, clocks, resolver, or timers are started here. Rust Drop
//! has no foreign effects; native-style logical teardown is an explicit method.

pub const INVALID: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    BufferExtent,
    RawReply,
    TimerScheduling,
    NativeAddressAdapter,
    Notifications,
    Options,
    Factory,
    NameLookup,
}
pub type Contract<T> = Result<T, Unsupported>;

/// A raw record, including padding; no byte-order conversion is implied.
/// Default's zero tail is a *portable storage choice*. Only bytes 0..2 are
/// initialized by the published native constructor; tail zeros are not evidence.
/// FTP's public host/port pair cannot represent this record's padding/family.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawAddress(pub [u8; 16]);

/// No native adapter is certified by §5/Q13 (including the adjacent port helper).
pub fn require_native_address_adapter() -> Contract<()> {
    Err(Unsupported::NativeAddressAdapter)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeval {
    pub seconds: u32,
    pub microseconds: u32,
}
fn timeval(ms: u32) -> Option<Timeval> {
    (ms != u32::MAX).then_some(Timeval {
        seconds: ms / 1000,
        microseconds: (ms % 1000) * 1000,
    })
}

/// Logical fd-set storage, not a native fd_set layout or capacity claim.
pub type FdSet = Vec<u32>;

/// Every variant is ONE raw call, never an owner result. Scalars carry their
/// full 32-bit bit pattern (-1 is ffffffff). Resource results are ignored except
/// LastError. Output writes remain visible on failure. Lengths initially are 16.
pub enum RawOp<'a> {
    Create {
        family: u32,
        style: u32,
        protocol: u32,
    },
    Close(u32),
    Bind {
        handle: u32,
        address: &'a RawAddress,
        length: u32,
    },
    Connect {
        handle: u32,
        address: &'a RawAddress,
        length: u32,
    },
    Listen {
        handle: u32,
        backlog: u32,
    },
    Accept {
        handle: u32,
        peer: &'a mut RawAddress,
        length: &'a mut u32,
    },
    Shutdown {
        handle: u32,
        how: u32,
    },
    Recv {
        handle: u32,
        buffer: &'a mut [u8],
        count: i32,
        flags: u32,
    },
    Send {
        handle: u32,
        buffer: &'a [u8],
        count: i32,
        flags: u32,
    },
    Select {
        nfds: u32,
        read: Option<&'a mut FdSet>,
        write: Option<&'a mut FdSet>,
        timeout: Option<Timeval>,
    },
    Membership {
        handle: u32,
        set: &'a FdSet,
    },
    Ioctl {
        handle: u32,
        command: u32,
        output: &'a mut u32,
    },
    GetName {
        peer: bool,
        handle: u32,
        address: &'a mut RawAddress,
        length: &'a mut u32,
    },
    LastError,
    ConstructTimer,
    DestroyTimer,
    /// Q12: opaque individual foreign cleanup, no resource-kind inference.
    CleanupAuxiliary(u32),
    FreeAuxiliary(u32),
    /// Logical already-owned reference only; no COM binding/refcount inference.
    ReleaseTarget(u32),
    FreeObject(u32),
}
pub trait RawHost {
    fn raw(&mut self, operation: RawOp<'_>) -> u32;
}

pub fn translate_error(raw: u32) -> u32 {
    match raw {
        10004 => 2,
        10009 => 3,
        10013 => 4,
        10014 => 5,
        10022 => 6,
        10024 => 7,
        10035..=10071 => raw - 10027,
        10101 => 45,
        10091..=10093 => raw - 10045,
        11001..=11004 => raw - 10952,
        _ => 1,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Auxiliary {
    pub record: u32,
    pub handle: u32,
}

/// Public fields allow explicit portable state snapshots, not native pointer
/// fabrication. Target/auxiliary snapshots must represent owned resources;
/// teardown never acquires/releases a borrowed reference on the caller's behalf.
#[derive(Debug, PartialEq, Eq)]
pub struct Socket {
    pub id: u32,
    pub handle: u32,
    pub bound: u8,
    pub local: RawAddress,
    pub peer: RawAddress,
    pub family: u32,
    pub style: u32,
    pub protocol: u32,
    pub error: u32,
    pub raw_error: u32,
    pub save: u8,
    pub target: Option<u32>,
    pub notification_id: u32,
    pub auxiliary: Option<Auxiliary>,
}
impl Default for Socket {
    fn default() -> Self {
        Self {
            id: 0,
            handle: INVALID,
            bound: 0,
            local: RawAddress::default(),
            peer: RawAddress::default(),
            family: 2,
            style: 1,
            protocol: 0,
            error: 1,
            raw_error: 0,
            save: 0,
            target: None,
            notification_id: 0,
            auxiliary: None,
        }
    }
}
impl Socket {
    fn record(&mut self, raw: u32) {
        self.error = translate_error(raw);
        self.raw_error = raw;
    }
    fn last_error(&mut self, host: &mut impl RawHost) {
        let raw = host.raw(RawOp::LastError);
        self.record(raw);
    }
    pub fn set_id(&mut self, id: u32) -> u32 {
        self.id = id;
        id
    }
    pub fn set_save(&mut self, argument: u32) -> u8 {
        self.save = argument as u8;
        self.save
    }
    pub fn set_style(&mut self, family: u32, style: u32, protocol: u32) -> bool {
        self.family = family;
        self.style = style;
        self.protocol = protocol;
        true
    }
    pub fn is_created(&self) -> bool {
        self.handle != INVALID
    }
    pub fn set_local(&mut self, address: RawAddress) -> bool {
        self.local = address;
        true
    }
    pub fn set_peer(&mut self, address: RawAddress) -> bool {
        self.peer = address;
        true
    }
    pub fn create(&mut self, host: &mut impl RawHost) -> bool {
        self.handle = host.raw(RawOp::Create {
            family: self.family,
            style: self.style,
            protocol: self.protocol,
        });
        if self.handle == INVALID {
            self.last_error(host);
            false
        } else {
            true
        }
    }
    /// Base close; never dispatches through the stream state transition.
    pub fn close(&mut self, host: &mut impl RawHost) -> bool {
        let result = if self.handle == INVALID {
            0
        } else {
            let result = host.raw(RawOp::Close(self.handle));
            self.handle = INVALID;
            result
        };
        let success = result == 0;
        if !success {
            self.last_error(host);
        }
        self.bound = 0;
        success
    }
    pub fn bind(&mut self, host: &mut impl RawHost, address: RawAddress) -> bool {
        self.local = address;
        let result = host.raw(RawOp::Bind {
            handle: self.handle,
            address: &self.local,
            length: 16,
        });
        self.bound = u8::from(result == 0);
        if result != 0 {
            self.last_error(host);
        }
        result == 0
    }
    pub fn bind_local(&mut self, host: &mut impl RawHost) -> bool {
        self.bind(host, self.local)
    }
    pub fn get_address(&mut self, host: &mut impl RawHost, peer: bool, handle: &u32, output: &mut RawAddress) -> bool {
        let mut length = 16;
        let result = host.raw(RawOp::GetName {
            peer,
            handle: *handle,
            address: output,
            length: &mut length,
        });
        if result != 0 {
            self.last_error(host);
        }
        result == 0
    }
    pub fn get_my_address(&mut self, host: &mut impl RawHost, peer: bool, output: &mut RawAddress) -> bool {
        let handle = self.handle;
        self.get_address(host, peer, &handle, output)
    }
    pub fn read_ready(&mut self, host: &mut impl RawHost, timeout: u32) -> bool {
        self.ready(host, timeout, true)
    }
    pub fn write_ready(&mut self, host: &mut impl RawHost, timeout: u32) -> bool {
        self.ready(host, timeout, false)
    }
    fn select_read(&self, host: &mut impl RawHost, timeout: u32) -> u32 {
        let mut read = vec![self.handle];
        host.raw(RawOp::Select {
            nfds: 1,
            read: Some(&mut read),
            write: None,
            timeout: timeval(timeout),
        })
    }
    fn ready(&mut self, host: &mut impl RawHost, timeout: u32, read: bool) -> bool {
        let result = if read {
            self.select_read(host, timeout)
        } else {
            let mut write = vec![self.handle];
            host.raw(RawOp::Select {
                nfds: 1,
                read: None,
                write: Some(&mut write),
                timeout: timeval(timeout),
            })
        };
        if result == INVALID {
            self.last_error(host);
            if read {
                self.handle = INVALID;
            }
            false
        } else {
            result != 0
        }
    }
    pub fn readable_or_writable(&mut self, host: &mut impl RawHost, mask: u32, timeout: u32) -> u32 {
        let mut read = if mask & 1 != 0 { vec![self.handle] } else { vec![] };
        let mut write = if mask & 2 != 0 { vec![self.handle] } else { vec![] };
        let result = host.raw(RawOp::Select {
            nfds: 0,
            read: if mask & 1 != 0 { Some(&mut read) } else { None },
            write: if mask & 2 != 0 { Some(&mut write) } else { None },
            timeout: timeval(timeout),
        });
        if result == INVALID {
            return 0xffff_fffc;
        }
        let r = host.raw(RawOp::Membership {
            handle: self.handle,
            set: &read,
        });
        let w = host.raw(RawOp::Membership {
            handle: self.handle,
            set: &write,
        });
        u32::from(r != 0) | (u32::from(w != 0) << 1)
    }
    pub fn waiting_size(&mut self, host: &mut impl RawHost, output: &mut u32, timeout: u32) -> bool {
        if timeout != 0 {
            match self.select_read(host, timeout) {
                INVALID => {
                    self.last_error(host);
                    return false;
                }
                0 => {
                    *output = 0;
                    return true;
                }
                _ => {}
            }
        }
        if host.raw(RawOp::Ioctl {
            handle: self.handle,
            command: 0x4004_667f,
            output,
        }) == 0
        {
            true
        } else {
            *output = 0;
            self.last_error(host);
            false
        }
    }
    /// Logical destructor normal path, including Q12's opaque auxiliary cleanup.
    /// No allocator algorithm, Release return/refcount, or Rust Drop claim.
    pub fn teardown(&mut self, host: &mut impl RawHost) {
        if self.save == 0 {
            self.close(host);
        }
        if let Some(auxiliary) = self.auxiliary {
            host.raw(RawOp::CleanupAuxiliary(auxiliary.handle));
            host.raw(RawOp::FreeAuxiliary(auxiliary.record));
            self.auxiliary = None;
        }
        if let Some(target) = self.target {
            host.raw(RawOp::ReleaseTarget(target));
            self.target = None;
        }
    }
    /// Explicit barriers; these domains have no success-noop implementation.
    pub fn notifications(&mut self) -> Contract<()> {
        Err(Unsupported::Notifications)
    }
    pub fn options(&mut self) -> Contract<()> {
        Err(Unsupported::Options)
    }
}

/// Portable destination dispatch for the object Accept overload. The core orders
/// H/peer writes, S=2, then style dispatch, and deliberately ignores its bool.
/// Implementations must expose the same socket and not reenter the caller.
pub trait AcceptDestination {
    fn socket_mut(&mut self) -> &mut Socket;
    fn set_connected(&mut self);
    fn set_socket_style(&mut self, family: u32, style: u32, protocol: u32) -> bool;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stream {
    pub socket: Socket,
    pub state: u32,
}
impl AcceptDestination for Stream {
    fn socket_mut(&mut self) -> &mut Socket {
        &mut self.socket
    }
    fn set_connected(&mut self) {
        self.state = 2;
    }
    fn set_socket_style(&mut self, family: u32, style: u32, protocol: u32) -> bool {
        self.socket.set_style(family, style, protocol)
    }
}
impl Stream {
    pub fn close(&mut self, host: &mut impl RawHost) -> bool {
        self.state = 0;
        self.socket.close(host)
    }
    pub fn shutdown(&mut self, host: &mut impl RawHost, how: u32) -> bool {
        self.state = 4;
        if self.socket.handle == INVALID {
            return true;
        }
        let result = host.raw(RawOp::Shutdown {
            handle: self.socket.handle,
            how,
        });
        if result != 0 {
            self.socket.last_error(host);
        }
        result == 0
    }
    pub fn start_listen(&mut self, host: &mut impl RawHost, backlog: u32) -> bool {
        let result = host.raw(RawOp::Listen {
            handle: self.socket.handle,
            backlog,
        });
        if result == 0 {
            self.state = 3;
        } else {
            self.socket.last_error(host);
        }
        result == 0
    }
    pub fn stop_listen(&mut self, host: &mut impl RawHost) -> bool {
        if self.socket.handle == INVALID {
            return true;
        }
        let result = host.raw(RawOp::Close(self.socket.handle));
        self.socket.handle = INVALID;
        if result == 0 {
            self.state = 0;
        } else {
            self.socket.last_error(host);
        }
        result == 0
    }
    pub fn connect(&mut self, host: &mut impl RawHost, completed: &mut u8, timeout: u32) -> Contract<bool> {
        if timeout != 0 {
            return Err(Unsupported::TimerScheduling);
        }
        // Even timeout=0 constructs/destroys the foreign local timer. Binding and
        // starting it are conditional; nonzero scheduling/races remain Q12.
        host.raw(RawOp::ConstructTimer);
        let raw = host.raw(RawOp::Connect {
            handle: self.socket.handle,
            address: &self.socket.peer,
            length: 16,
        });
        let result = if raw == 0 {
            *completed = 1;
            self.state = 2;
            true
        } else {
            *completed = 0;
            self.socket.last_error(host);
            self.socket.raw_error == 10035
        };
        host.raw(RawOp::DestroyTimer);
        Ok(result)
    }
    pub fn connect_address(
        &mut self,
        host: &mut impl RawHost,
        address: RawAddress,
        completed: &mut u8,
        timeout: u32,
    ) -> Contract<bool> {
        // Reject the unsupported domain before changing a snapshot.
        if timeout != 0 {
            return Err(Unsupported::TimerScheduling);
        }
        self.socket.set_peer(address);
        self.connect(host, completed, timeout)
    }
    pub fn accept(
        &mut self,
        host: &mut impl RawHost,
        handle: &mut u32,
        peer: &mut RawAddress,
        completed: &mut u8,
    ) -> bool {
        let mut length = 16;
        *handle = host.raw(RawOp::Accept {
            handle: self.socket.handle,
            peer,
            length: &mut length,
        });
        if *handle == INVALID {
            *completed = 0;
            self.socket.last_error(host);
            self.socket.raw_error == 10035
        } else {
            *completed = 1;
            true
        }
    }
    pub fn accept_into(
        &mut self,
        host: &mut impl RawHost,
        destination: &mut impl AcceptDestination,
        completed: &mut u8,
    ) -> bool {
        let socket = destination.socket_mut();
        let result = self.accept(host, &mut socket.handle, &mut socket.peer, completed);
        if result && *completed != 0 {
            destination.set_connected();
            let _ = destination.set_socket_style(self.socket.family, self.socket.style, self.socket.protocol);
        }
        result
    }
    pub fn is_connected(&mut self, host: &mut impl RawHost) -> bool {
        if self.socket.handle == INVALID {
            return false;
        }
        if self.socket.select_read(host, 0) != INVALID {
            return true;
        }
        self.socket.last_error(host);
        self.socket.handle = INVALID;
        self.state = 0;
        false
    }
    pub fn read(&mut self, host: &mut impl RawHost, buffer: &mut [u8], count: &mut i32) -> Contract<bool> {
        if *count <= 0 {
            return Ok(false);
        }
        self.receive(host, buffer, count, 0)
    }
    pub fn peek(&mut self, host: &mut impl RawHost, buffer: &mut [u8], count: &mut i32) -> Contract<bool> {
        self.receive(host, buffer, count, 2)
    }
    pub fn read_oob(&mut self, host: &mut impl RawHost, buffer: &mut [u8], count: &mut i32) -> Contract<bool> {
        self.receive(host, buffer, count, 1)
    }
    fn receive(&mut self, host: &mut impl RawHost, buffer: &mut [u8], count: &mut i32, flags: u32) -> Contract<bool> {
        if *count == 0 {
            return Ok(true);
        }
        let extent = extent(*count, buffer.len())?;
        let raw = host.raw(RawOp::Recv {
            handle: self.socket.handle,
            buffer: &mut buffer[..extent],
            count: *count,
            flags,
        });
        if raw == INVALID {
            return Ok(self.transfer_error(host, count));
        }
        if raw > *count as u32 {
            return Err(Unsupported::RawReply);
        }
        *count = raw as i32;
        if raw != 0 {
            return Ok(true);
        }
        self.socket.record(10057);
        self.close(host);
        Ok(false)
    }
    pub fn write(&mut self, host: &mut impl RawHost, buffer: &[u8], count: &mut i32, flags: u32) -> Contract<bool> {
        if *count == 0 {
            return Ok(true);
        }
        let extent = extent(*count, buffer.len())?;
        let raw = host.raw(RawOp::Send {
            handle: self.socket.handle,
            buffer: &buffer[..extent],
            count: *count,
            flags,
        });
        if raw == INVALID {
            return Ok(self.transfer_error(host, count));
        }
        if raw > *count as u32 {
            return Err(Unsupported::RawReply);
        }
        *count = raw as i32;
        Ok(true)
    }
    pub fn write_oob(&mut self, host: &mut impl RawHost, buffer: &[u8], count: &mut i32, flags: u32) -> Contract<bool> {
        self.write(host, buffer, count, flags | 1)
    }
    fn transfer_error(&mut self, host: &mut impl RawHost, count: &mut i32) -> bool {
        let raw = host.raw(RawOp::LastError);
        *count = 0;
        self.socket.record(raw);
        matches!(raw, 10035 | 10036)
    }
    pub fn teardown(&mut self, host: &mut impl RawHost, identity: u32, flags: u32) -> u32 {
        self.socket.teardown(host);
        if flags & 1 != 0 {
            host.raw(RawOp::FreeObject(identity));
        }
        identity
    }
}
fn extent(count: i32, capacity: usize) -> Contract<usize> {
    let extent = usize::try_from(count).map_err(|_| Unsupported::BufferExtent)?;
    if extent > capacity {
        Err(Unsupported::BufferExtent)
    } else {
        Ok(extent)
    }
}

/// Published captured-handle callback only, not timer scheduling/owner dispatch.
pub fn captured_close(host: &mut impl RawHost, handle: u32) -> u32 {
    if host.raw(RawOp::Close(handle)) == 0 {
        0
    } else {
        host.raw(RawOp::LastError)
    }
}

#[cfg(test)]
mod tests;
