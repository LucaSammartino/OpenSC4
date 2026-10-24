//! FTP owner state transitions. External operations are primitives, never owner callbacks.
//! O helpers (initialization, control creation, receive options and close helper) remain
//! explicit barriers; callers may exercise supported bodies on existing state snapshots.
use super::{leaf, listing, Contract, Unsupported};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Socket {
    Control,
    Listener,
    Data,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Address {
    pub host: [u8; 4],
    pub port_word: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoFailure;
pub type IoResult<T> = Result<T, IoFailure>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    AddTarget(u32),
    ReleaseTarget(u32),
    CreateMessage(u32),
    AddMessage,
    ReleaseMessage,
    ConstructThread,
    StartDispatcher(u32),
    DestroyThread,
    DestroyFile,
    DestroyBuffer(&'static str),
    DestroySocket(Socket),
    DeleteSession,
    DestroyScheduler,
    FreeScheduler,
    BorrowedPwdReply(Vec<u8>),
    ConstructLookupString,
    DestroyLookupString,
}

/// Slots are the seven arguments of 008120e0, not smart pointers. Borrowed objects
/// remain valid only inside `notify`; no target retain is implied by message delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub words: [u32; 6],
}

pub trait Io {
    fn now(&mut self) -> u32;
    fn lookup(&mut self, host: &[u8]) -> Option<[u8; 4]>;
    fn is_open(&mut self, socket: Socket) -> bool;
    fn create(&mut self, socket: Socket) -> bool;
    fn close(&mut self, socket: Socket);
    fn local_address(&mut self, socket: Socket) -> Address;
    fn bind(&mut self, socket: Socket, address: Address) -> bool;
    fn listen(&mut self, backlog: u32) -> bool;
    fn connect(&mut self, address: Address, timeout: u32) -> (bool, bool);
    fn accept(&mut self) -> (bool, bool);
    fn wait(&mut self, socket: Socket, timeout: u32) -> bool;
    fn receive(&mut self, socket: Socket, capacity: usize) -> IoResult<Vec<u8>>;
    fn send(&mut self, socket: Socket, bytes: &[u8]) -> IoResult<usize>;
    fn file_open(&mut self, path: &[u8], arguments: (u32, u32, u32)) -> bool;
    fn file_length(&mut self) -> u32;
    fn file_seek_zero(&mut self);
    fn file_truncate_zero(&mut self);
    fn file_read(&mut self, capacity: usize) -> IoResult<Vec<u8>>;
    fn file_write(&mut self, data: &[u8]) -> bool;
    fn file_close(&mut self);
    fn effect(&mut self, effect: Effect);
    /// May reenter supported Session methods. The core has already applied the
    /// published state changes and socket cleanup at a terminal callback boundary.
    fn notify(&mut self, session: &mut Session, message: Message, list: Option<&[listing::Entry]>);
}

/// Body 00812ba0: the buffer and output reply belong to the session.
#[derive(Clone, Debug, Default)]
pub struct Responses {
    pub buffered: Vec<u8>,
    pub reply: Vec<u8>,
    pub class: i32,
    pub final_line: bool,
}

pub(super) fn atol(text: &[u8]) -> i32 {
    let mut i = 0;
    while text.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let negative = text.get(i) == Some(&b'-');
    if negative || text.get(i) == Some(&b'+') {
        i += 1;
    }
    let mut value = 0i32;
    while let Some(b @ b'0'..=b'9') = text.get(i) {
        value = value.wrapping_mul(10).wrapping_add(i32::from(*b - b'0'));
        i += 1;
    }
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}

impl Responses {
    pub fn read(&mut self, io: &mut impl Io, timeout: u32) -> Contract<bool> {
        self.final_line = false;
        loop {
            if let Some(lf) = self.buffered.iter().position(|b| *b == b'\n') {
                let mut reply: Vec<_> = self.buffered.drain(..=lf).collect();
                while reply.last().is_some_and(|b| matches!(b, b'\r' | b'\n' | b' ')) {
                    reply.pop();
                }
                if reply.len() < 4 {
                    return Err(Unsupported::NativeInputDomain("00812ba0 short reply"));
                }
                self.class = atol(&reply[..3]) / 100;
                self.final_line = reply[3] != b'-';
                self.reply = reply;
                return Ok(true);
            }
            if self.buffered.len() >= 2048 {
                return Err(Unsupported::NativeInputDomain("00812ba0 full unterminated reply"));
            }
            if !io.wait(Socket::Control, timeout) {
                return Ok(false);
            }
            let capacity = 2048 - self.buffered.len();
            let Ok(bytes) = io.receive(Socket::Control, capacity) else {
                return Ok(false);
            };
            if bytes.len() > capacity {
                return Err(Unsupported::PrimitiveContract("receive exceeds requested capacity"));
            }
            // A successful zero receive makes no progress, just as the native loop.
            self.buffered.extend_from_slice(&bytes);
        }
    }

    /// 00813d30 skips final class-1 lines, not merely multiline continuations.
    pub fn completed(&mut self, io: &mut impl Io, timeout: u32) -> Contract<bool> {
        loop {
            let result = self.read(io, timeout)?;
            if !result || self.final_line && self.class != 1 {
                return Ok(result);
            }
        }
    }
    fn final_reply(&mut self, io: &mut impl Io, timeout: u32) -> Contract<bool> {
        loop {
            let result = self.read(io, timeout)?;
            if !result || self.final_line {
                return Ok(result);
            }
        }
    }
}

/// 00811da0. No retry after any partial 512-byte piece.
pub fn send_chunks(io: &mut impl Io, bytes: &[u8], count: i32) -> Contract<bool> {
    if count <= 0 {
        return Ok(true);
    }
    if count as usize > bytes.len() {
        return Err(Unsupported::NativeInputDomain("00811da0 input count"));
    }
    for piece in bytes[..count as usize].chunks(512) {
        if io.send(Socket::Data, piece) != Ok(piece.len()) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// 00811cd0 classifier, including its close/destruction and binary-on-failure output.
pub fn classify(io: &mut impl Io, path: &[u8], output: &mut u32) -> Contract<bool> {
    *output = 203;
    let opened = io.file_open(path, (1, 2, 1));
    let result = if opened {
        let capacity = io.file_length().min(512) as usize;
        match io.file_read(capacity) {
            Ok(bytes) if bytes.len() <= capacity => {
                *output = if bytes.iter().all(|b| matches!(b, b'\r' | b'\n' | 0x20..=0x7e)) {
                    201
                } else {
                    203
                };
                true
            }
            Ok(_) => {
                io.file_close();
                io.effect(Effect::DestroyFile);
                return Err(Unsupported::PrimitiveContract("classifier read count"));
            }
            Err(_) => false,
        }
    } else {
        false
    };
    io.file_close();
    io.effect(Effect::DestroyFile);
    Ok(result)
}

/// 00811e70; an existing listener is never rebound.
pub fn ensure_listener(io: &mut impl Io) -> bool {
    if io.is_open(Socket::Listener) {
        return true;
    }
    if !io.create(Socket::Listener) {
        return false;
    }
    let mut address = io.local_address(Socket::Control);
    address.port_word = 0;
    if !io.bind(Socket::Listener, address) || !io.listen(5) {
        io.close(Socket::Listener);
        return false;
    }
    true
}
pub fn ensure_control(io: &mut impl Io) -> bool {
    if io.is_open(Socket::Control) {
        return true;
    }
    if !io.create(Socket::Control) {
        return false;
    }
    if !io.bind(Socket::Control, Address::default()) {
        io.close(Socket::Control);
        return false;
    }
    true
}
pub fn lookup(io: &mut impl Io, host: &[u8]) -> Option<[u8; 4]> {
    io.effect(Effect::ConstructLookupString);
    let result = io.lookup(host);
    io.effect(Effect::DestroyLookupString);
    result
}
/// 00811f30, independently callable without control/listener work.
pub fn ensure_data(io: &mut impl Io) -> bool {
    if io.is_open(Socket::Data) {
        return true;
    }
    if !io.create(Socket::Data) {
        return false;
    }
    if !io.bind(Socket::Data, Address::default()) {
        io.close(Socket::Data);
        return false;
    }
    true
}

#[derive(Clone, Debug)]
pub struct Deferred {
    pub first: Vec<u8>,
    pub second: Option<Vec<u8>>,
    pub borrowed_argument: Option<u32>,
}

/// 00812390 defaults. IDs are nonowning identities; reference effects are emitted
/// explicitly. Owning a Vec in this portable model does not retain a native target.
#[derive(Clone, Debug)]
pub struct Session {
    pub references: u32,
    pub selector: u32,
    pub requested_type: u32,
    pub current_type: u32,
    pub start_time: u32,
    pub timeout: u32,
    pub transferred: u32,
    pub target: Option<u32>,
    pub user: Vec<u8>,
    pub password: Vec<u8>,
    pub account: Vec<u8>,
    pub pending_host: Vec<u8>,
    pub default_port: u16,
    pub address: Address,
    pub initialized: bool,
    pub state: u32,
    pub operation: u32,
    pub login_stage: u32,
    pub transfer_stage: u32,
    pub local: Vec<u8>,
    pub remote: Vec<u8>,
    pub buffer: Vec<u8>,
    pub responses: Responses,
    pub extra: Vec<u8>,
    pub stored_directory: Vec<u8>,
    pub async_preference: bool,
    pub work: bool,
    pub deferred: Deferred,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            references: 0,
            selector: 310,
            requested_type: 203,
            current_type: 201,
            start_time: 0,
            timeout: 15000,
            transferred: 0,
            target: None,
            user: b"anonymous".to_vec(),
            password: b"user".to_vec(),
            account: Vec::new(),
            pending_host: Vec::new(),
            default_port: 0x1500,
            address: Address::default(),
            initialized: false,
            state: 0,
            operation: 100,
            login_stage: 400,
            transfer_stage: 500,
            local: Vec::new(),
            remote: Vec::new(),
            buffer: vec![0; 2048],
            responses: Responses::default(),
            extra: Vec::new(),
            stored_directory: Vec::new(),
            async_preference: false,
            work: false,
            deferred: Deferred {
                first: Vec::new(),
                second: None,
                borrowed_argument: None,
            },
        }
    }
}

impl Session {
    pub fn initialize(&mut self) -> Contract<()> {
        // 00813d70 was corrected to scheduler construction, not this operation.
        // The actual session-initialization contract is still absent (WWW Q19).
        Err(Unsupported::OwnedBody("Q19 FTP session initialization"))
    }
    pub fn query_interface(&mut self, io: &mut impl Io, identity: u32, iid: u32, output: &mut u32) -> bool {
        if !matches!(iid, 1 | 0x452294aa) {
            return false;
        }
        *output = identity;
        self.references = self.references.wrapping_add(1);
        io.effect(Effect::AddTarget(identity));
        true
    }
    pub fn set_target(&mut self, io: &mut impl Io, target: Option<u32>) {
        if target == self.target {
            return;
        }
        if let Some(target) = target {
            io.effect(Effect::AddTarget(target));
        }
        if let Some(old) = self.target {
            io.effect(Effect::ReleaseTarget(old));
        }
        self.target = target;
    }
    pub fn release(&mut self, io: &mut impl Io) -> Contract<u32> {
        if self.references > 1 {
            self.references -= 1;
            return Ok(self.references);
        }
        self.destroy(io, 1)?;
        Ok(0)
    }

    /// Body 00812290: short sends resubmit the whole original command, not its suffix.
    /// Successful zero sends can loop forever; no invented retry/timeout policy.
    pub fn send_command(&mut self, io: &mut impl Io, command: &[u8]) -> bool {
        if self.state == 0 {
            return false;
        }
        let mut sent = 0u32;
        while (sent as usize) < command.len() {
            let Ok(count) = io.send(Socket::Control, command) else {
                return false;
            };
            sent = sent.wrapping_add(count as u32);
        }
        true
    }
    fn start(&mut self, io: &mut impl Io) {
        io.effect(Effect::ConstructThread);
        io.effect(Effect::StartDispatcher(self.operation));
        io.effect(Effect::DestroyThread);
    }
    fn defer(&mut self, io: &mut impl Io, operation: u32) -> bool {
        if self.async_preference && !self.work && self.target.is_some() {
            self.work = true;
            self.state = 4;
            self.operation = operation;
            self.start(io);
            true
        } else {
            false
        }
    }
    /// 008120e0: message acquire/callback/release; target is borrowed, not retained.
    pub fn message(&mut self, io: &mut impl Io, message: Message, list: Option<&[listing::Entry]>) {
        if self.target.is_none() {
            return;
        }
        io.effect(Effect::CreateMessage(message.kind));
        io.effect(Effect::AddMessage);
        io.notify(self, message, list);
        io.effect(Effect::ReleaseMessage);
    }
    fn event(&mut self, io: &mut impl Io, kind: u32, result: bool) {
        self.message(
            io,
            Message {
                kind,
                words: [0, 1, 0, 0, u32::from(result), 0],
            },
            None,
        );
    }
    fn close_if_open(io: &mut impl Io, socket: Socket) {
        if io.is_open(socket) {
            io.close(socket);
        }
    }
    fn command_reply(&mut self, io: &mut impl Io, command: &[u8]) -> Contract<bool> {
        if !self.send_command(io, command) {
            return Ok(false);
        }
        self.responses.completed(io, self.timeout)
    }
    fn command_class(&mut self, io: &mut impl Io, command: &[u8], class: i32) -> Contract<bool> {
        Ok(self.command_reply(io, command)? && self.responses.class == class)
    }
    /// The seven state-2 commands from FTP control §9. PWD output identities are
    /// borrowed by asynchronous dispatch; Rust never dereferences a saved identity.
    #[allow(clippy::too_many_arguments)]
    pub fn simple_command(
        &mut self,
        io: &mut impl Io,
        operation: u32,
        first: &[u8],
        second: &[u8],
        output: Option<&mut Vec<u8>>,
        output_identity: Option<u32>,
    ) -> Contract<bool> {
        let (verb, kind) = match operation {
            108 => (b"PWD".as_slice(), 0x57386931),
            109 => (b"CWD".as_slice(), 0x57386932),
            110 => (b"CDUP".as_slice(), 0x57386933),
            112 => (b"MKD".as_slice(), 0x57386934),
            113 => (b"RMD".as_slice(), 0x57386935),
            105 => (b"DELE".as_slice(), 0x57386936),
            106 => (b"RNFR".as_slice(), 0x57386937),
            _ => return Err(Unsupported::NativeInputDomain("FTP command operation")),
        };
        if self.state != 2 {
            self.work = false;
            return Ok(false);
        }
        if self.async_preference && !self.work && self.target.is_some() {
            if operation == 108 {
                self.deferred.borrowed_argument = output_identity;
            } else if operation == 106 {
                self.deferred.first = second.to_vec();
            } else {
                self.deferred.first = first.to_vec();
            }
            self.work = true;
            self.state = 4;
            self.operation = operation;
            self.start(io);
            return Ok(true);
        }
        self.state = 4;
        self.operation = operation;
        self.transfer_stage = 504;
        self.start_time = io.now();
        self.remote = first.to_vec();
        if matches!(operation, 109 | 112 | 113) {
            if self.remote.is_empty() {
                return Err(Unsupported::NativeInputDomain("empty FTP path final-byte read"));
            }
            if self.remote.last().is_some_and(|b| matches!(b, b'/' | b'\\')) {
                self.remote.pop();
            }
        }
        if operation == 106 {
            self.extra = second.to_vec();
        }
        let wire = |verb: &[u8], arg: Option<&[u8]>| {
            let mut bytes = verb.to_vec();
            if let Some(arg) = arg {
                bytes.push(b' ');
                bytes.extend(arg.split(|b| *b == 0).next().unwrap_or_default());
            }
            bytes.extend(b"\r\n");
            bytes
        };
        let command = wire(
            verb,
            if matches!(operation, 108 | 110) {
                None
            } else {
                Some(&self.remote)
            },
        );
        let mut success = false;
        if self.send_command(io, &command) {
            self.transfer_stage = 507;
            success = self.responses.completed(io, self.timeout)?
                && self.responses.class == if operation == 106 { 3 } else { 2 };
            if success && operation == 106 {
                self.transfer_stage = 505;
                if self.send_command(io, &wire(b"RNTO", Some(&self.extra))) {
                    self.transfer_stage = 507;
                    success = self.responses.completed(io, self.timeout)? && self.responses.class == 2;
                } else {
                    success = false;
                }
            }
        }
        if operation == 108 {
            if success {
                let reply = &self.responses.reply;
                let quote = reply.iter().position(|b| *b == b'"');
                if let Some(start) = quote.filter(|i| (*i as u32) < (reply.len() as u32).wrapping_sub(2)) {
                    let Some(end) = reply[start + 1..]
                        .iter()
                        .position(|b| *b == b'"')
                        .map(|i| start + 1 + i)
                    else {
                        return Err(Unsupported::NativeInputDomain("PWD missing closing quote"));
                    };
                    self.responses.reply = reply[start + 1..end].to_vec();
                    self.stored_directory = self.responses.reply.clone();
                    if let Some(output) = output {
                        *output = self.responses.reply.clone();
                    }
                } else {
                    success = false;
                }
            }
            if !success {
                self.responses.reply.clear();
            }
        }
        self.state = 2;
        self.operation = 100;
        self.transfer_stage = 500;
        self.work = false;
        if operation == 108 && self.target.is_some() {
            io.effect(Effect::BorrowedPwdReply(self.responses.reply.clone()));
            // Nonzero logical identities denote this session and its borrowed reply.
            self.message(
                io,
                Message {
                    kind,
                    words: [0, 1, 0, 1, u32::from(success), 0],
                },
                None,
            );
        } else {
            self.event(io, kind, success);
        }
        Ok(success)
    }
    /// 00816af0: forced route preserves work, nonforced clears it AFTER callback.
    pub fn disconnect(&mut self, io: &mut impl Io, force: bool) -> Contract<bool> {
        if self.state == 0 {
            return Ok(true);
        }
        if !force && self.defer(io, 101) {
            return Ok(true);
        }
        self.state = 3;
        if self.send_command(io, b"QUIT\r\n") {
            let _ = self.responses.completed(io, self.timeout)?;
        }
        for socket in [Socket::Data, Socket::Listener, Socket::Control] {
            Self::close_if_open(io, socket);
        }
        self.state = 0;
        self.event(io, 0x57386939, true);
        if !force {
            self.work = false;
        }
        Ok(true)
    }
    /// 00815d30. Boolean result is NOT authenticated-state success.
    pub fn login(&mut self, io: &mut impl Io) -> Contract<bool> {
        if self.state != 0 || self.user.is_empty() {
            self.event(io, 0x57386938, false);
            self.work = false;
            return Ok(false);
        }
        if self.defer(io, 101) {
            return Ok(true);
        }
        self.state = 1;
        self.login_stage = 401;
        if !ensure_control(io) {
            self.state = 0;
            self.login_stage = 400;
            self.event(io, 0x57386938, false);
            self.work = false;
            return Ok(false);
        }
        if !self.pending_host.is_empty() {
            if let Some(host) = numeric_host(&self.pending_host) {
                self.address.host = host;
                self.pending_host.clear();
            } else {
                let Some(address) = lookup(io, &self.pending_host) else {
                    self.state = 0;
                    self.login_stage = 400;
                    self.event(io, 0x57386938, false);
                    self.work = false;
                    return Ok(false);
                };
                self.address.host = address;
                self.pending_host.clear();
            }
        } else if self.address.host == [0; 4] {
            self.state = 0;
            self.login_stage = 400;
            self.event(io, 0x57386938, false);
            self.work = false;
            return Ok(false);
        }
        if self.address.port_word == 0 {
            self.address.port_word = self.default_port;
        }
        self.login_stage = 403;
        let (call, accepted) = io.connect(self.address, self.timeout);
        let mut result = call && accepted;
        if result {
            self.login_stage = 404;
            result = self.responses.completed(io, self.timeout)?;
            if result {
                let mut accepted = true;
                for (verb, value, stage, exact) in [
                    (b"USER ".as_slice(), self.user.clone(), 405, false),
                    (b"PASS ", self.password.clone(), 406, false),
                    (b"ACCT ", self.account.clone(), 407, true),
                ] {
                    // PASS/ACCT are conditional on the previous reply being class 3.
                    if stage > 405 && self.responses.class != 3 {
                        break;
                    }
                    self.login_stage = stage;
                    let mut command = verb.to_vec();
                    command.extend(value);
                    command.extend(b"\r\n");
                    result = self.command_reply(io, &command)?;
                    accepted = result
                        && if exact {
                            self.responses.class == 2
                        } else {
                            self.responses.class < 4
                        };
                    if !accepted {
                        break;
                    }
                }
                if accepted {
                    self.login_stage = 408;
                    result = self.command_reply(io, b"SYST\r\n")?;
                    accepted = result && self.responses.class == 2;
                }
                if accepted {
                    listing::detect(&self.responses.reply, &mut self.selector);
                    self.state = 2;
                    self.login_stage = 400;
                    self.extra.clear();
                    self.current_type = 201;
                } else {
                    self.disconnect(io, true)?;
                }
            } else {
                self.disconnect(io, true)?;
            }
        }
        self.event(io, 0x57386938, result);
        self.work = false;
        Ok(result)
    }
    fn port(&mut self, io: &mut impl Io) -> Contract<bool> {
        self.transfer_stage = 503;
        let address = io.local_address(Socket::Listener);
        let port = address.port_word.swap_bytes();
        let command = format!(
            "PORT {},{},{},{},{},{}\r\n",
            address.host[0],
            address.host[1],
            address.host[2],
            address.host[3],
            port >> 8,
            port & 255
        );
        self.command_class(io, command.as_bytes(), 2)
    }
    fn data_command(&mut self, io: &mut impl Io, verb: &[u8]) -> Contract<bool> {
        self.transfer_stage = 504;
        let mut command = verb.to_vec();
        command.extend(&self.remote);
        command.extend(b"\r\n");
        if !self.send_command(io, &command) {
            return Ok(false);
        }
        self.transfer_stage = 507;
        if !self.responses.final_reply(io, self.timeout)? || self.responses.class != 1 {
            return Ok(false);
        }
        let (call, accepted) = io.accept();
        if !call || !accepted {
            return Ok(false);
        }
        io.close(Socket::Listener);
        Ok(true)
    }
    fn terminal(&mut self, io: &mut impl Io, kind: u32, result: bool) {
        self.work = false;
        self.state = 2;
        self.operation = 100;
        self.transfer_stage = 500;
        Self::close_if_open(io, Socket::Listener);
        Self::close_if_open(io, Socket::Data);
        self.event(io, kind, result);
    }
    fn progress(&mut self, io: &mut impl Io, total: i32, previous: &mut i32, upload: bool, length: u32) {
        if self.target.is_some() && total >= previous.wrapping_add(16384) {
            let rounded = (total / 16384).wrapping_mul(16384);
            self.message(
                io,
                Message {
                    kind: if upload { 0x5738692d } else { 0x5738692f },
                    words: [rounded as u32, 1, length, 0, 0, 0],
                },
                None,
            );
            *previous = rounded;
        }
    }
    /// 00814560 / 00813e90. `borrowed_argument` records the native second-pointer
    /// lifetime requirement; it is never dereferenced by Rust after the entry call.
    pub fn transfer(
        &mut self,
        io: &mut impl Io,
        upload: bool,
        first: &[u8],
        second: Option<&[u8]>,
        borrowed_argument: Option<u32>,
    ) -> Contract<bool> {
        if self.state != 2 {
            self.work = false;
            return Ok(false);
        }
        let operation = if upload { 103 } else { 104 };
        if self.async_preference && !self.work && self.target.is_some() {
            self.deferred.first = first.to_vec();
            self.deferred.borrowed_argument = borrowed_argument;
            self.defer(io, operation);
            return Ok(true);
        }
        self.state = 4;
        self.operation = operation;
        self.transfer_stage = 501;
        self.transferred = 0;
        self.start_time = io.now();
        if upload {
            self.local = first.to_vec();
            self.remote = second.unwrap_or_else(|| leaf(first)).to_vec();
        } else {
            self.remote = first.to_vec();
            self.local = second.unwrap_or_else(|| leaf(first)).to_vec();
            if second.is_some() && leaf(&self.local).is_empty() {
                self.local.extend(leaf(first));
            }
        }
        let (result, file_exists) = self.transfer_body(io, upload)?;
        self.terminal(io, if upload { 0x5738692c } else { 0x5738692e }, result);
        if file_exists {
            if upload {
                io.file_close();
            }
            io.effect(Effect::DestroyFile);
        }
        Ok(result)
    }
    fn transfer_body(&mut self, io: &mut impl Io, upload: bool) -> Contract<(bool, bool)> {
        if !ensure_listener(io) || !ensure_data(io) {
            return Ok((false, false));
        }
        if self.requested_type != self.current_type {
            self.transfer_stage = 502;
            let mut selected = self.requested_type;
            if selected == 200 {
                if upload {
                    if !classify(io, &self.local, &mut selected)? {
                        return Ok((false, false));
                    }
                } else {
                    selected = 203;
                }
            }
            let command = match selected {
                201 => b"TYPE A\r\n",
                202 => b"TYPE E\r\n",
                _ => b"TYPE I\r\n",
            };
            if !self.command_class(io, command, 2)? {
                return Ok((false, false));
            }
            self.current_type = selected;
        }
        if !self.port(io)? || !self.data_command(io, if upload { b"STOR " } else { b"RETR " })? {
            return Ok((false, false));
        }
        if !io.file_open(&self.local, if upload { (1, 2, 1) } else { (2, 3, 1) }) {
            return Ok((false, true));
        }
        io.file_seek_zero();
        let length = if upload {
            io.file_length()
        } else {
            io.file_truncate_zero();
            0
        };
        let mut total = 0i32;
        let mut previous = 0i32;
        let mut write_failed = false;
        loop {
            let bytes = if upload {
                io.file_read(self.buffer.len())
            } else {
                if !io.wait(Socket::Data, self.timeout) {
                    break;
                }
                io.receive(Socket::Data, self.buffer.len())
            };
            let Ok(bytes) = bytes else {
                break;
            };
            if bytes.len() > self.buffer.len() {
                return Err(Unsupported::PrimitiveContract("transfer read count"));
            }
            if bytes.is_empty() {
                break;
            }
            if upload {
                let _ = send_chunks(io, &bytes, bytes.len() as i32)?;
            } else if !io.file_write(&bytes) {
                write_failed = true;
                break;
            }
            self.transferred = self.transferred.wrapping_add(bytes.len() as u32);
            total = total.wrapping_add(bytes.len() as i32);
            self.progress(io, total, &mut previous, upload, length);
        }
        if !upload {
            io.file_close();
        }
        if write_failed {
            return Ok((false, true));
        }
        Self::close_if_open(io, Socket::Data);
        let result = self.responses.completed(io, self.timeout)? && self.responses.class == 2;
        Ok((result, true))
    }
    /// 008163d0 entry + control acquisition. The published data loop cannot be
    /// reached portably through the still-O 00811f00 option helper; see `listing_bytes`.
    pub fn acquire_listing(&mut self, io: &mut impl Io, path: &[u8], borrowed_list: Option<u32>) -> Contract<bool> {
        if self.state != 2 {
            self.work = false;
            return Ok(false);
        }
        if self.async_preference && !self.work && self.target.is_some() {
            self.deferred.first = path.to_vec();
            self.deferred.borrowed_argument = borrowed_list;
            self.defer(io, 111);
            return Ok(true);
        }
        if borrowed_list.is_some() {
            return Err(Unsupported::NativeInputDomain(
                "acquisition needs a live borrowed list, not an identity",
            ));
        }
        self.acquire_listing_into(io, path, None)
    }
    pub fn acquire_listing_into(
        &mut self,
        io: &mut impl Io,
        path: &[u8],
        output: Option<&mut Vec<listing::Entry>>,
    ) -> Contract<bool> {
        if self.state != 2 {
            self.work = false;
            return Ok(false);
        }
        if path.is_empty() {
            return Err(Unsupported::NativeInputDomain("008163d0 empty path last-byte read"));
        }
        self.state = 4;
        self.operation = 111;
        self.transfer_stage = 501;
        self.transferred = 0;
        self.start_time = io.now();
        self.remote = path.to_vec();
        if self.remote.last().is_some_and(|b| matches!(b, b'/' | b'\\')) {
            self.remote.pop();
        }
        if !ensure_listener(io) || !ensure_data(io) {
            self.finish_listing(io, false, &[]);
            return Ok(false);
        }
        if self.current_type != 201 {
            self.transfer_stage = 502;
            if !self.command_class(io, b"TYPE A\r\n", 2)? {
                self.finish_listing(io, false, &[]);
                return Ok(false);
            }
            self.current_type = 201;
        }
        if !self.port(io)? || !self.data_command(io, b"LIST ")? {
            self.finish_listing(io, false, &[]);
            return Ok(false);
        }
        // Latest parent section 4.15 qualifies 00811f00 as closing +0xf0,
        // superseding the older receive-options label in the companion.
        Self::close_if_open(io, Socket::Listener);
        let bytes = self.listing_bytes(io)?;
        Self::close_if_open(io, Socket::Data);
        self.finish_listing_bytes(io, &bytes, output)
    }
    /// Supported interior of 008163d0, on a snapshot AFTER the O receive-option
    /// helper. This does not implement or bypass that helper at the entry point.
    pub fn listing_bytes(&mut self, io: &mut impl Io) -> Contract<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut capacity = self.buffer.len();
        let mut request = capacity;
        loop {
            if !io.wait(Socket::Data, self.timeout) {
                break;
            }
            let Ok(block) = io.receive(Socket::Data, request) else {
                break;
            };
            if block.len() > request || bytes.len() + block.len() > capacity {
                return Err(Unsupported::NativeInputDomain("008163d0 receive range"));
            }
            if block.is_empty() {
                break;
            }
            request = block.len();
            bytes.extend(block);
            if bytes.len() == capacity {
                capacity += 256;
                request = 256;
            }
        }
        self.buffer = bytes.clone();
        Ok(bytes)
    }
    /// Supported suffix after the O 00811f90 helper; temporary lists die after callback.
    pub fn finish_listing_bytes(
        &mut self,
        io: &mut impl Io,
        bytes: &[u8],
        output: Option<&mut Vec<listing::Entry>>,
    ) -> Contract<bool> {
        let mut temporary = Vec::new();
        let list = output.unwrap_or(&mut temporary);
        let result = self.responses.completed(io, self.timeout)?
            && self.responses.class == 2
            && listing::replace(bytes, self.selector, list)?;
        self.finish_listing(io, result, list);
        Ok(result)
    }
    fn finish_listing(&mut self, io: &mut impl Io, result: bool, list: &[listing::Entry]) {
        self.work = false;
        self.state = 2;
        self.operation = 100;
        self.transfer_stage = 500;
        Self::close_if_open(io, Socket::Listener);
        Self::close_if_open(io, Socket::Data);
        self.message(
            io,
            Message {
                kind: 0x57386930,
                words: [0, 1, 0, 1, u32::from(result), 0],
            },
            Some(list),
        );
    }
    /// 00816990. The opcode is read once; no state reset or session hold.
    pub fn dispatch(&mut self, io: &mut impl Io) -> Contract<u32> {
        let operation = self.operation;
        match operation {
            101 => {
                self.login(io)?;
            }
            102 => {
                self.disconnect(io, false)?;
            }
            103 | 104 => {
                if self.state != 2 {
                    self.work = false;
                    return Ok(0);
                }
                // A live borrowed native second argument cannot be recovered from an ID.
                return Err(Unsupported::NativeInputDomain("dispatcher borrowed second argument"));
            }
            111 => {
                let path = self.deferred.first.clone();
                self.acquire_listing(io, &path, self.deferred.borrowed_argument)?;
            }
            105 | 106 | 108..=110 | 112 | 113 => {
                let first = self.deferred.first.clone();
                let second = self.deferred.second.clone().unwrap_or_default();
                self.simple_command(io, operation, &first, &second, None, self.deferred.borrowed_argument)?;
            }
            _ => {}
        }
        Ok(0)
    }
    /// 00816cc0. A false initialized flag leaves even an existing target untouched.
    pub fn shutdown(&mut self, io: &mut impl Io) -> Contract<bool> {
        if !self.initialized {
            return Ok(true);
        }
        self.initialized = false;
        if let Some(target) = self.target.take() {
            io.effect(Effect::ReleaseTarget(target));
        }
        if self.state != 0 {
            self.disconnect(io, true)?;
        }
        for socket in [Socket::Control, Socket::Listener, Socket::Data] {
            Self::close_if_open(io, socket);
        }
        self.state = 0;
        self.operation = 100;
        self.login_stage = 400;
        self.transfer_stage = 500;
        Ok(true)
    }
    /// 00816db0 / 00817540 / 00816d90: no join, including when storage is freed.
    pub fn destroy(&mut self, io: &mut impl Io, deleting_flags: u32) -> Contract<()> {
        self.shutdown(io)?;
        for field in ["a2c", "a18", "a08", "1f8", "1ec", "1e0", "1d4", "1c8"] {
            io.effect(Effect::DestroyBuffer(field));
        }
        for socket in [Socket::Data, Socket::Listener, Socket::Control] {
            io.effect(Effect::DestroySocket(socket));
        }
        for field in ["host", "account", "password", "user"] {
            io.effect(Effect::DestroyBuffer(field));
        }
        if deleting_flags & 1 != 0 {
            io.effect(Effect::DeleteSession);
        }
        Ok(())
    }
}

/// Numeric dotted-quad domain only. Named resolution's owned wrapper stays Q9.
pub fn numeric_host(text: &[u8]) -> Option<[u8; 4]> {
    let text = std::str::from_utf8(text).ok()?;
    let mut address = [0; 4];
    let mut parts = text.split('.');
    for byte in &mut address {
        *byte = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        None
    } else {
        Some(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Fake {
        open: [bool; 3],
        replies: VecDeque<Vec<u8>>,
        data: VecDeque<IoResult<Vec<u8>>>,
        reads: VecDeque<IoResult<Vec<u8>>>,
        sends: Vec<Vec<u8>>,
        effects: Vec<Effect>,
        events: Vec<(Message, u32, bool)>,
        capacities: Vec<usize>,
        partial: VecDeque<IoResult<usize>>,
        connect: (bool, bool),
        length: u32,
        file_open: bool,
        file_write: bool,
        destroyed: bool,
        reenter: bool,
        worker_effects: Vec<super::super::worker::Effect>,
        worker_callbacks: Vec<(u32, u32, bool)>,
    }
    fn index(socket: Socket) -> usize {
        match socket {
            Socket::Control => 0,
            Socket::Listener => 1,
            Socket::Data => 2,
        }
    }
    impl Fake {
        fn ready(reply: &[u8]) -> Self {
            Self {
                open: [true; 3],
                replies: [reply.to_vec()].into(),
                connect: (true, true),
                file_open: true,
                file_write: true,
                ..Self::default()
            }
        }
    }
    impl Io for Fake {
        fn lookup(&mut self, _: &[u8]) -> Option<[u8; 4]> {
            None
        }
        fn now(&mut self) -> u32 {
            77
        }
        fn is_open(&mut self, s: Socket) -> bool {
            self.open[index(s)]
        }
        fn create(&mut self, s: Socket) -> bool {
            self.open[index(s)] = true;
            true
        }
        fn close(&mut self, s: Socket) {
            self.open[index(s)] = false;
        }
        fn local_address(&mut self, _: Socket) -> Address {
            Address {
                host: [1, 2, 3, 4],
                port_word: 0x3412,
            }
        }
        fn bind(&mut self, _: Socket, _: Address) -> bool {
            true
        }
        fn listen(&mut self, backlog: u32) -> bool {
            assert_eq!(backlog, 5);
            true
        }
        fn connect(&mut self, _: Address, _: u32) -> (bool, bool) {
            self.connect
        }
        fn accept(&mut self) -> (bool, bool) {
            (true, true)
        }
        fn wait(&mut self, _: Socket, _: u32) -> bool {
            true
        }
        fn receive(&mut self, s: Socket, capacity: usize) -> IoResult<Vec<u8>> {
            if s == Socket::Control {
                self.replies.pop_front().ok_or(IoFailure)
            } else {
                self.capacities.push(capacity);
                self.data.pop_front().unwrap_or(Ok(Vec::new()))
            }
        }
        fn send(&mut self, _: Socket, bytes: &[u8]) -> IoResult<usize> {
            self.sends.push(bytes.to_vec());
            self.partial.pop_front().unwrap_or(Ok(bytes.len()))
        }
        fn file_open(&mut self, _: &[u8], _: (u32, u32, u32)) -> bool {
            self.file_open
        }
        fn file_length(&mut self) -> u32 {
            self.length
        }
        fn file_seek_zero(&mut self) {}
        fn file_truncate_zero(&mut self) {}
        fn file_read(&mut self, _: usize) -> IoResult<Vec<u8>> {
            self.reads.pop_front().unwrap_or(Ok(Vec::new()))
        }
        fn file_write(&mut self, _: &[u8]) -> bool {
            self.file_write
        }
        fn file_close(&mut self) {}
        fn effect(&mut self, effect: Effect) {
            if effect == Effect::DestroyFile {
                self.destroyed = true;
            }
            self.effects.push(effect);
        }
        fn notify(&mut self, session: &mut Session, message: Message, _: Option<&[listing::Entry]>) {
            self.events.push((message, session.state, self.destroyed));
            if message.kind == 0x5738692c || message.kind == 0x5738692e {
                assert!(!self.open[1] && !self.open[2]);
                if self.reenter {
                    self.reenter = false;
                    session.async_preference = true;
                    assert_eq!(session.transfer(self, false, b"again", None, None), Ok(true));
                }
            }
        }
    }
    fn session() -> Session {
        Session {
            state: 2,
            target: Some(9),
            address: Address {
                host: [1, 2, 3, 4],
                port_word: 0,
            },
            ..Session::default()
        }
    }
    #[test]
    fn sup01_sup04_sup05_sup08_and_scheduler_destruction() {
        use super::super::{
            http,
            worker::{Effect as W, Job, Record, Scheduler},
        };
        let mut io = Fake::default();
        let mut s = Session {
            target: Some(9),
            ..Session::default()
        };
        s.set_target(&mut io, Some(9));
        assert!(io.effects.is_empty());
        struct Http(Vec<http::Effect>);
        impl http::Host for Http {
            fn effect(&mut self, e: http::Effect) {
                self.0.push(e);
            }
            fn notify(&mut self, _: &mut http::Client, _: http::Job, _: bool) {}
        }
        let mut recorder = Http(Vec::new());
        let mut client = http::Client::new(None);
        client.target = Some(9);
        client.set_target(&mut recorder, Some(9));
        assert_eq!(recorder.0, [http::Effect::AddTarget(9), http::Effect::ReleaseTarget(9)]);
        let mut output = 123;
        assert!(!s.query_interface(&mut io, 7, 2, &mut output));
        assert_eq!(output, 123);
        assert!(s.query_interface(&mut io, 7, 0x452294aa, &mut output));
        assert_eq!((output, s.references), (7, 1));
        s.references = 0;
        assert_eq!(s.release(&mut io), Ok(0));
        assert_eq!(io.effects.last(), Some(&Effect::DeleteSession));
        let mut scheduler = Scheduler {
            record: Some(Record::default()),
            initialized: true,
            target: Some(9),
            ..Scheduler::default()
        };
        assert_eq!(scheduler.adjust_worker_reference(&mut io, false), u32::MAX);
        let mut job = Job {
            id: 1,
            status: 1,
            url: b"u".to_vec(),
            destination: b"d".to_vec(),
            time: 2,
            progress: 3,
            custom: 9,
        };
        let copy = job.clone();
        job.url[0] = b'x';
        job.destination[0] = b'y';
        assert_eq!(
            (copy.url.as_slice(), copy.destination.as_slice()),
            (b"u".as_slice(), b"d".as_slice())
        );
        scheduler.queue.push_back(copy);
        io.effects.clear();
        io.worker_effects.clear();
        assert_eq!(scheduler.erase_range(&mut io, 0, 1), 0);
        assert!(io.effects.is_empty() && io.worker_callbacks.is_empty());
        assert_eq!(
            io.worker_effects,
            [W::UnlinkQueueNode, W::EraseFirstString, W::DestroyQueueNode]
        );
        scheduler.destroy(&mut io, 1).unwrap();
        assert_eq!(io.effects, [Effect::ReleaseTarget(9)]);
        assert_eq!(io.worker_effects.last(), Some(&W::FreeScheduler));
        assert!(ensure_control(&mut io));
        assert!(io.open[0]);
        assert!(lookup(&mut io, b"host").is_none());
        assert_eq!(
            &io.effects[io.effects.len() - 2..],
            [Effect::ConstructLookupString, Effect::DestroyLookupString]
        );
    }
    #[test]
    fn ftpc01_through_ftpc06() {
        let mut io = Fake::default();
        let mut s = session();
        s.state = 4;
        s.work = true;
        for operation in [105, 106, 108, 109, 110, 112, 113] {
            assert_eq!(
                s.simple_command(&mut io, operation, b"old", b"new", None, None),
                Ok(false)
            );
            assert!(!s.work);
            assert_eq!(s.state, 4);
        }
        assert!(io.sends.is_empty() && io.events.is_empty()); // FTPC01
        s = session();
        s.async_preference = true;
        s.deferred.second = Some(b"untouched".to_vec());
        assert_eq!(s.simple_command(&mut io, 106, b"old", b"new", None, None), Ok(true));
        assert_eq!(s.deferred.first, b"new");
        assert_eq!(s.deferred.second.as_deref(), Some(b"untouched".as_slice()));
        s.dispatch(&mut io).unwrap();
        assert!(io.sends.is_empty());
        assert_eq!(s.state, 4); // FTPC02
        for (operation, verb) in [(109, b"CWD".as_slice()), (112, b"MKD"), (113, b"RMD")] {
            s = session();
            io = Fake::default();
            io.replies.push_back(b"250 okay\r\n".to_vec());
            assert_eq!(s.simple_command(&mut io, operation, b"a//", b"", None, None), Ok(true));
            assert_eq!(io.sends, vec![[verb, b" a/\r\n"].concat()]);
            assert_eq!((s.state, s.operation, s.transfer_stage), (2, 100, 500));
            assert_eq!(io.events[0].1, 2); // FTPC03
        }
        for (class, expected, sends) in [(b"250 first\r\n".as_slice(), false, 1), (b"350 first\r\n", true, 2)] {
            s = session();
            io = Fake::default();
            io.replies.extend([class.to_vec(), b"250 renamed\r\n".to_vec()]);
            assert_eq!(s.simple_command(&mut io, 106, b"old", b"new", None, None), Ok(expected));
            assert_eq!(io.sends.len(), sends); // FTPC04
        }
        s = session();
        io = Fake::default();
        io.replies.push_back(b"257 \"/a b\" ready\r\n".to_vec());
        let mut out = b"old".to_vec();
        assert_eq!(
            s.simple_command(&mut io, 108, b"", b"", Some(&mut out), Some(42)),
            Ok(true)
        );
        assert_eq!(out, b"/a b");
        assert_eq!(s.stored_directory, out);
        assert_eq!(s.responses.reply, out); // FTPC05
        s = session();
        io = Fake::default();
        io.partial.push_back(Err(IoFailure));
        out = b"old".to_vec();
        assert_eq!(
            s.simple_command(&mut io, 108, b"", b"", Some(&mut out), Some(42)),
            Ok(false)
        );
        assert_eq!(out, b"old");
        assert!(s.responses.reply.is_empty());
        assert!(io.replies.is_empty()); // FTPC06
    }
    #[test]
    fn ftpc07_ftpc08_scheduler_admission() {
        use super::super::worker::{Effect as W, Scheduler};
        let mut io = Fake::default();
        let mut scheduler = Scheduler::new(Some(9));
        assert_eq!(scheduler.timeout, 15);
        assert_eq!(scheduler.lookup, Some(9));
        scheduler.counter = u32::MAX;
        scheduler.default_directory = b"default".to_vec();
        assert_eq!(scheduler.admit(&mut io, b"url", Some(b"x"), 42), 0);
        assert_eq!(scheduler.queue[0].id, 0);
        assert_eq!(scheduler.queue[0].destination, b"x\\");
        assert_eq!(
            (
                scheduler.queue[0].status,
                scheduler.queue[0].time,
                scheduler.queue[0].progress,
                scheduler.queue[0].custom
            ),
            (1, 0, 0, 42)
        );
        assert_eq!(&io.worker_effects[..3], &[W::Lock, W::AllocateQueueNode, W::Unlock]);
        assert!(io.worker_effects.contains(&W::StartWorker)); // FTPC07
        io.worker_effects.clear();
        assert_eq!(scheduler.admit(&mut io, b"", None, 0), 0);
        assert!(io.worker_effects.is_empty());
        for destination in [None, Some(b"".as_slice())] {
            scheduler.admit(&mut io, b"url", destination, 0);
            assert_eq!(scheduler.queue.back().unwrap().destination, b"default");
            // FTPC08
        }
    }

    impl super::super::worker::Host for Fake {
        fn worker_effect(&mut self, effect: super::super::worker::Effect) {
            self.worker_effects.push(effect);
        }
        fn complete_job(
            &mut self,
            _: &mut super::super::worker::Scheduler,
            job: &super::super::worker::Job,
            success: bool,
        ) {
            self.worker_callbacks.push((job.status, job.progress, success));
        }
    }

    #[test]
    fn outer_worker_suspend_stop_cancel_and_nonftp_preserved_status() {
        use super::super::worker::{Effect as W, Job, Record, Scheduler, Step};
        let job = Job {
            id: 8,
            status: 3,
            url: b"http://host/x".to_vec(),
            destination: vec![],
            time: 0,
            progress: 0,
            custom: 9,
        };
        let mut worker = Scheduler {
            queue: [job.clone()].into(),
            target: Some(9),
            ..Scheduler::default()
        };
        let mut io = Fake::default();
        worker.start(&mut io);
        worker.record.as_mut().unwrap().suspend = 1;
        assert_eq!(worker.dequeue(&mut io), Ok(Step::Suspended));
        assert_eq!(worker.queue.len(), 1);
        assert!(io.worker_effects.contains(&W::Sleep(1000000)));
        worker.record.as_mut().unwrap().suspend = 0;
        assert_eq!(worker.dequeue(&mut io), Ok(Step::Dequeued));
        worker.process_current(&mut io).unwrap();
        assert_eq!(io.worker_callbacks, [(3, 100, false)]);
        assert_eq!(worker.dequeue(&mut io), Ok(Step::Exited));
        assert!(worker.record.is_none());
        assert_eq!(Scheduler::wrapper_return(&mut io), 0);
        assert_eq!(io.worker_effects.last(), Some(&W::ReleaseScheduler));
        for stop in [false, true] {
            let mut worker = Scheduler {
                target: Some(1),
                record: Some(Record {
                    cancel: true,
                    stop,
                    current: Some(job.clone()),
                    ..Record::default()
                }),
                ..Scheduler::default()
            };
            let mut io = Fake::default();
            worker.final_boundary(&mut io).unwrap();
            assert!(!worker.record.as_ref().unwrap().cancel);
            assert!(worker.record.as_ref().unwrap().current.is_none());
            if stop {
                assert!(io.worker_callbacks.is_empty());
            } else {
                assert_eq!(io.worker_callbacks, [(7, 100, false)]);
            }
        }
    }

    #[test]
    fn outer_worker_init_barrier_and_rejected_login_true_is_not_transfer_success() {
        use super::super::worker::{Job, Record, Scheduler};
        let job = Job {
            id: 8,
            status: 3,
            url: b"ftp://1.2.3.4/x".to_vec(),
            destination: vec![],
            time: 0,
            progress: 0,
            custom: 9,
        };
        let mut worker = Scheduler {
            target: Some(1),
            record: Some(Record {
                current: Some(job),
                ..Record::default()
            }),
            ..Scheduler::default()
        };
        let mut io = Fake::default();
        assert_eq!(
            worker.process_current(&mut io),
            Err(Unsupported::OwnedBody("Q19 FTP session initialization"))
        );
        let mut s = Session {
            initialized: true,
            address: Address {
                host: [1, 2, 3, 4],
                port_word: 0,
            },
            ..Session::default()
        };
        let mut io = Fake::ready(b"220 greeting\r\n530 rejected\r\n221 bye\r\n");
        worker
            .transfer_current(&mut io, &mut s, &mut crate::rzurl::RzUrl::from_text(b"ftp://1.2.3.4/x"))
            .unwrap();
        assert_eq!(io.worker_callbacks, [(3, 100, false)]);
    }

    #[test]
    fn response_helpers_and_prefix_resending() {
        let mut io = Fake::ready(b"150-first\r\n125 next\r\n226 done\r\n");
        let mut response = Responses::default();
        assert_eq!(response.read(&mut io, 1), Ok(true));
        assert!(!response.final_line);
        assert_eq!(response.completed(&mut io, 1), Ok(true));
        assert_eq!(response.class, 2);
        let mut s = session();
        io.partial = [Ok(6), Ok(4)].into();
        assert!(s.send_command(&mut io, b"0123456789"));
        assert_eq!(io.sends, [b"0123456789", b"0123456789"]);
        s.state = 0;
        assert!(!s.send_command(&mut io, b""));
        let mut io = Fake::ready(b"2\r\n");
        assert!(Responses::default().read(&mut io, 1).is_err());
    }
    #[test]
    fn classifier_and_short_chunk_no_retry() {
        for (bytes, expected) in [(b"A\r\n".as_slice(), 201), (b"A\tB", 203), (b"", 201)] {
            let mut io = Fake {
                file_open: true,
                length: bytes.len() as u32,
                reads: [Ok(bytes.to_vec())].into(),
                ..Fake::default()
            };
            let mut result = 999;
            assert_eq!(classify(&mut io, b"file", &mut result), Ok(true));
            assert_eq!(result, expected);
            assert!(io.destroyed);
        }
        let mut io = Fake {
            partial: [Ok(300)].into(),
            ..Fake::default()
        };
        assert_eq!(send_chunks(&mut io, &[0; 512], 512), Ok(false));
        assert_eq!(io.sends.len(), 1);
        assert_eq!(send_chunks(&mut io, &[], -1), Ok(true));
    }
    #[test]
    fn deferred_reentry_rejects_without_terminal_or_rollback() {
        let mut s = session();
        s.async_preference = true;
        let mut io = Fake::default();
        assert_eq!(s.transfer(&mut io, false, b"dir/a", Some(b"out/"), Some(99)), Ok(true));
        assert_eq!(s.state, 4);
        assert!(s.work);
        assert_eq!(s.deferred.borrowed_argument, Some(99));
        assert_eq!(s.dispatch(&mut io), Ok(0));
        assert!(!s.work);
        assert_eq!(s.state, 4);
        assert!(io.events.is_empty());
        s.state = 2;
        assert_eq!(s.acquire_listing(&mut io, b"abc//", Some(8)), Ok(true));
        assert_eq!(s.dispatch(&mut io), Ok(0));
        assert_eq!(s.state, 4);
        assert!(io.events.is_empty());
    }
    #[test]
    fn retr_zero_data_final_reply_and_paths() {
        let mut s = session();
        s.requested_type = 200;
        s.current_type = 200;
        let mut io = Fake::ready(b"299 port\r\n150-first\r\n125 ready\r\n150 extra\r\n226 done\r\n");
        assert_eq!(s.transfer(&mut io, false, b"dir/a.bin", Some(b"out/"), None), Ok(true));
        assert_eq!(s.local, b"out/a.bin");
        assert_eq!(s.transferred, 0);
        assert_eq!(
            io.sends,
            [b"PORT 1,2,3,4,18,52\r\n".to_vec(), b"RETR dir/a.bin\r\n".to_vec()]
        );
        assert_eq!(io.events[0].1, 2);
        assert_eq!(io.events[0].0.words[4], 1);
    }
    #[test]
    fn stor_ignores_sender_and_read_failure_and_keeps_wrapper_through_callback() {
        let mut s = session();
        s.requested_type = 201;
        let mut io = Fake::ready(b"200 port\r\n199 ready\r\n226 done\r\n");
        io.reads = [Ok(vec![1; 512]), Err(IoFailure)].into();
        io.partial = [Ok(100), Ok(100), Ok(300)].into();
        assert_eq!(s.transfer(&mut io, true, b"dir/file", None, None), Ok(true));
        assert_eq!(s.remote, b"file");
        assert_eq!(s.transferred, 512);
        assert!(!io.events[0].2);
        assert!(io.destroyed);
    }
    #[test]
    fn retr_write_failure_skips_final_read_and_progress_rounds_once() {
        let mut s = session();
        s.requested_type = 201;
        let mut io = Fake::ready(b"200 port\r\n150 ready\r\n");
        io.data = [Ok(vec![1; 3])].into();
        io.file_write = false;
        assert_eq!(s.transfer(&mut io, false, b"file", None, None), Ok(false));
        assert_eq!(s.transferred, 0);
        let mut previous = 0;
        s.progress(&mut io, 40000, &mut previous, false, 0);
        assert_eq!(previous, 32768);
        assert_eq!(io.events.last().unwrap().0.words[0], 32768);
        let before = io.events.len();
        previous = 0;
        s.progress(&mut io, 16383, &mut previous, false, 0);
        assert_eq!(io.events.len(), before);
        s.progress(&mut io, 16384, &mut previous, false, 0);
        assert_eq!(io.events.len(), before + 1);
    }
    #[test]
    fn terminal_reentry_and_login_boolean_state_channels() {
        let mut s = session();
        s.requested_type = 201;
        let mut io = Fake::ready(b"200 port\r\n150 ready\r\n226 done\r\n");
        io.reenter = true;
        assert_eq!(s.transfer(&mut io, false, b"file", None, None), Ok(true));
        assert_eq!(s.state, 4);
        let mut s = session();
        s.state = 0;
        let mut io = Fake::ready(b"220 greeting\r\n530 rejected\r\n221 bye\r\n");
        assert_eq!(s.login(&mut io), Ok(true));
        assert_eq!(s.state, 0);
        assert_eq!(
            io.events.iter().map(|e| e.0.kind).collect::<Vec<_>>(),
            [0x57386939, 0x57386938]
        );
        assert_eq!(io.events[1].0.words[4], 1);
        let mut s = session();
        s.state = 0;
        let mut io = Fake::ready(b"");
        io.connect = (false, true);
        assert_eq!(s.login(&mut io), Ok(false));
        assert_eq!((s.state, s.login_stage), (1, 403));
        assert!(io.open[0]);
    }
    #[test]
    fn accepted_login_and_deferred_disconnect_literal_101() {
        let mut s = session();
        s.state = 0;
        let mut io = Fake::ready(b"500 greeting\r\n331 password\r\n332 account\r\n230 ok\r\n215 UNIX Windows_NT\r\n");
        assert_eq!(s.login(&mut io), Ok(true));
        assert_eq!(s.selector, 314);
        assert_eq!(s.state, 2);
        s.async_preference = true;
        assert_eq!(s.disconnect(&mut io, false), Ok(true));
        assert_eq!(s.operation, 101);
        assert_eq!(s.dispatch(&mut io), Ok(0));
        assert_eq!(s.state, 4);
        assert_eq!(io.events.last().unwrap().0.words[4], 0);
    }
    #[test]
    fn listing_partial_capacity_and_replacement_suffix() {
        let mut s = session();
        s.buffer = vec![0; 256];
        let mut io = Fake::ready(b"226 done\r\n");
        io.data = [Ok(vec![b'a'; 100]), Ok(vec![b'b'; 100]), Err(IoFailure)].into();
        assert_eq!(s.listing_bytes(&mut io).unwrap().len(), 200);
        assert_eq!(io.capacities, [256, 100, 100]);
        let mut output = vec![listing::Entry {
            name: b"old".to_vec(),
            kind: 4,
        }];
        assert_eq!(
            s.finish_listing_bytes(&mut io, b"? rejected\n", Some(&mut output)),
            Ok(true)
        );
        assert!(output.is_empty());
        assert_eq!(s.state, 2);
    }
    #[test]
    fn shutdown_release_before_disconnect_uninitialized_and_forced_work() {
        let mut s = session();
        s.work = true;
        let mut io = Fake::ready(b"221 bye\r\n");
        assert_eq!(s.shutdown(&mut io), Ok(true));
        assert!(io.effects.is_empty());
        assert_eq!(s.state, 2);
        s.initialized = true;
        assert_eq!(s.shutdown(&mut io), Ok(true));
        assert_eq!(io.effects[0], Effect::ReleaseTarget(9));
        assert!(io.events.is_empty());
        assert!(s.work);
        assert!(!s.initialized);
        assert_eq!(s.state, 0);
        s.destroy(&mut io, 1).unwrap();
        assert_eq!(io.effects.last(), Some(&Effect::DeleteSession));
    }
}
