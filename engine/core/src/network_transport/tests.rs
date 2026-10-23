//! Literal portable fixtures from network-transport §§2–6/8, not native executions.
use super::*;
use std::collections::VecDeque;

const PEER: [u8; 16] = [
    2, 0, 0x12, 0x34, 1, 0x23, 0x45, 0x67, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
];
const LOCAL: [u8; 16] = [
    2, 0, 0xab, 0xcd, 0x89, 0xab, 0xcd, 0xef, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
];

#[derive(Debug, PartialEq)]
enum Command {
    Create(u32, u32, u32),
    Close(u32),
    Shutdown(u32, u32),
    Bind(u32, [u8; 16], u32),
    Listen(u32, u32),
    Connect(u32, [u8; 16], u32),
    Accept(u32, [u8; 16], u32),
    Recv(u32, usize, i32, u32),
    Send(u32, Vec<u8>, i32, u32),
    Select(u32, Option<Vec<u32>>, Option<Vec<u32>>, Option<(u32, u32)>),
    Member(u32, Vec<u32>),
    Ioctl(u32, u32, u32),
    Name(bool, u32, [u8; 16], u32),
    Error,
    TimerNew,
    TimerDrop,
    Cleanup(u32),
    FreeRecord(u32),
    Release(u32),
    FreeObject(u32),
}

struct Step {
    command: Command,
    result: u32,
    bytes: &'static [u8],
    size: Option<u32>,
    sets: Option<(Vec<u32>, Vec<u32>)>,
}
fn step(command: Command, result: u32) -> Step {
    Step {
        command,
        result,
        bytes: &[],
        size: None,
        sets: None,
    }
}
fn bytes(command: Command, result: u32, bytes: &'static [u8]) -> Step {
    Step {
        bytes,
        ..step(command, result)
    }
}
struct Fixture(VecDeque<Step>);
impl Fixture {
    fn new(steps: Vec<Step>) -> Self {
        Self(steps.into())
    }
    fn done(&self) {
        assert!(self.0.is_empty(), "unconsumed raw operations");
    }
}
impl RawHost for Fixture {
    fn raw(&mut self, op: RawOp<'_>) -> u32 {
        let s = self.0.pop_front().expect("unexpected raw operation");
        let command = match op {
            RawOp::Create {
                family,
                style,
                protocol,
            } => Command::Create(family, style, protocol),
            RawOp::Close(h) => Command::Close(h),
            RawOp::Shutdown { handle, how } => Command::Shutdown(handle, how),
            RawOp::Bind {
                handle,
                address,
                length,
            } => Command::Bind(handle, address.0, length),
            RawOp::Listen { handle, backlog } => Command::Listen(handle, backlog),
            RawOp::Connect {
                handle,
                address,
                length,
            } => Command::Connect(handle, address.0, length),
            RawOp::Accept { handle, peer, length } => {
                let c = Command::Accept(handle, peer.0, *length);
                peer.0[..s.bytes.len()].copy_from_slice(s.bytes);
                if let Some(n) = s.size {
                    *length = n;
                }
                c
            }
            RawOp::Recv {
                handle,
                buffer,
                count,
                flags,
            } => {
                let c = Command::Recv(handle, buffer.len(), count, flags);
                buffer[..s.bytes.len()].copy_from_slice(s.bytes);
                c
            }
            RawOp::Send {
                handle,
                buffer,
                count,
                flags,
            } => Command::Send(handle, buffer.to_vec(), count, flags),
            RawOp::Select {
                nfds,
                mut read,
                mut write,
                timeout,
            } => {
                let c = Command::Select(
                    nfds,
                    read.as_deref().cloned(),
                    write.as_deref().cloned(),
                    timeout.map(|t| (t.seconds, t.microseconds)),
                );
                if let Some((r, w)) = s.sets {
                    if let Some(set) = read.as_mut() {
                        **set = r;
                    }
                    if let Some(set) = write.as_mut() {
                        **set = w;
                    }
                }
                c
            }
            RawOp::Membership { handle, set } => Command::Member(handle, set.clone()),
            RawOp::Ioctl {
                handle,
                command,
                output,
            } => {
                let c = Command::Ioctl(handle, command, *output);
                if let Some(n) = s.size {
                    *output = n;
                }
                c
            }
            RawOp::GetName {
                peer,
                handle,
                address,
                length,
            } => {
                let c = Command::Name(peer, handle, address.0, *length);
                address.0[..s.bytes.len()].copy_from_slice(s.bytes);
                if let Some(n) = s.size {
                    *length = n;
                }
                c
            }
            RawOp::LastError => Command::Error,
            RawOp::ConstructTimer => Command::TimerNew,
            RawOp::DestroyTimer => Command::TimerDrop,
            RawOp::CleanupAuxiliary(h) => Command::Cleanup(h),
            RawOp::FreeAuxiliary(p) => Command::FreeRecord(p),
            RawOp::ReleaseTarget(p) => Command::Release(p),
            RawOp::FreeObject(p) => Command::FreeObject(p),
        };
        assert_eq!(command, s.command);
        s.result
    }
}
fn stream() -> Stream {
    let mut s = Stream::default();
    s.socket.handle = 41;
    s.socket.bound = 1;
    s.state = 3;
    s
}
fn fields(s: &Stream) -> (u32, u8, u32, u32, u32) {
    (
        s.socket.handle,
        s.socket.bound,
        s.state,
        s.socket.error,
        s.socket.raw_error,
    )
}

#[test]
fn construction_and_full_width_setters() {
    let mut s = Stream::default();
    assert_eq!(fields(&s), (0xffff_ffff, 0, 0, 1, 0));
    assert_eq!(
        (
            s.socket.id,
            s.socket.family,
            s.socket.style,
            s.socket.protocol,
            s.socket.save
        ),
        (0, 2, 1, 0, 0)
    );
    assert_eq!(&s.socket.local.0[..2], &[0, 0]);
    assert_eq!(&s.socket.peer.0[..2], &[0, 0]);
    assert_eq!(
        (s.socket.target, s.socket.notification_id, s.socket.auxiliary),
        (None, 0, None)
    );
    assert_eq!(s.socket.set_id(0xfedc_ba98), 0xfedc_ba98);
    assert_eq!(s.socket.set_save(0x1234_56a5), 0xa5);
    assert_eq!((s.socket.id, s.socket.save), (0xfedc_ba98, 0xa5));
    assert!(!s.socket.is_created());
    assert!(s.socket.set_style(23, 19, 17));
    assert_eq!((s.socket.family, s.socket.style, s.socket.protocol), (23, 19, 17));
}

#[test]
fn l1_l2_create_overwrites_without_close_even_on_failure() {
    for (raw, expected, error) in [(73, true, (1, 0)), (0xffff_ffff, false, (34, 10061))] {
        let mut s = stream();
        let mut steps = vec![step(Command::Create(2, 1, 0), raw)];
        if !expected {
            steps.push(step(Command::Error, 10061));
        }
        let mut h = Fixture::new(steps);
        assert_eq!(s.socket.create(&mut h), expected);
        assert_eq!(fields(&s), (raw, 1, 3, error.0, error.1));
        h.done();
    }
}

#[test]
fn l3_invalid_stream_close_repeated() {
    let mut s = stream();
    s.socket.handle = INVALID;
    let mut h = Fixture::new(vec![]);
    assert!(s.close(&mut h));
    assert!(s.close(&mut h));
    assert_eq!(fields(&s), (0xffff_ffff, 0, 0, 1, 0));
    h.done();
}

#[test]
fn l4_close_failure_invalidates_and_preserves_error_on_repeat() {
    let mut s = stream();
    let mut h = Fixture::new(vec![step(Command::Close(41), u32::MAX), step(Command::Error, 10038)]);
    assert!(!s.close(&mut h));
    assert!(s.close(&mut h));
    assert_eq!(fields(&s), (0xffff_ffff, 0, 0, 11, 10038));
    h.done();
}

#[test]
fn l5_l6_connect_zero_keeps_timer_lifetime_on_every_return() {
    for (raw, err, result, completed, state, e, r) in [
        (0, None, true, 1, 2, 1, 0),
        (u32::MAX, Some(10035), true, 0, 3, 8, 10035),
        (u32::MAX, Some(10061), false, 0, 3, 34, 10061),
    ] {
        let mut s = stream();
        s.socket.peer = RawAddress(PEER);
        let mut steps = vec![step(Command::TimerNew, 0), step(Command::Connect(41, PEER, 16), raw)];
        if let Some(err) = err {
            steps.push(step(Command::Error, err));
        }
        steps.push(step(Command::TimerDrop, 0));
        let mut h = Fixture::new(steps);
        let mut out = 7;
        assert_eq!(s.connect(&mut h, &mut out, 0), Ok(result));
        assert_eq!(out, completed);
        assert_eq!(fields(&s), (41, 1, state, e, r));
        h.done();
    }
}

#[test]
fn nonzero_connect_timeout_is_unsupported_without_side_effects() {
    let mut s = stream();
    let mut out = 7;
    let mut h = Fixture::new(vec![]);
    assert_eq!(s.connect(&mut h, &mut out, 1), Err(Unsupported::TimerScheduling));
    assert_eq!(out, 7);
    assert_eq!(fields(&s), (41, 1, 3, 1, 0));
    h.done();
}

#[test]
fn l7_stop_listen_failure_and_invalid_repeat() {
    let mut s = stream();
    let mut h = Fixture::new(vec![step(Command::Close(41), u32::MAX), step(Command::Error, 10061)]);
    assert!(!s.stop_listen(&mut h));
    assert!(s.stop_listen(&mut h));
    assert_eq!(fields(&s), (0xffff_ffff, 1, 3, 34, 10061));
    h.done();
}

#[test]
fn l8_invalid_shutdown_sets_state_only() {
    let mut s = stream();
    s.socket.handle = INVALID;
    s.state = 2;
    let mut h = Fixture::new(vec![]);
    assert!(s.shutdown(&mut h, 2));
    assert_eq!(fields(&s), (0xffff_ffff, 1, 4, 1, 0));
    h.done();
}

#[test]
fn shutdown_failure_and_listen_success_and_failure() {
    let mut s = stream();
    let mut h = Fixture::new(vec![
        step(Command::Shutdown(41, 2), 1),
        step(Command::Error, 10038),
        step(Command::Listen(41, 17), 0),
        step(Command::Listen(41, 18), 1),
        step(Command::Error, 10061),
        step(Command::Close(41), 0),
    ]);
    assert!(!s.shutdown(&mut h, 2));
    assert_eq!(fields(&s), (41, 1, 4, 11, 10038));
    assert!(s.start_listen(&mut h, 17));
    assert_eq!(s.state, 3);
    assert!(!s.start_listen(&mut h, 18));
    assert_eq!(s.state, 3);
    assert!(s.stop_listen(&mut h));
    assert_eq!(fields(&s), (0xffff_ffff, 1, 0, 34, 10061));
    h.done();
}

#[test]
fn l9_accept_wouldblock_keeps_raw_peer_writes_and_handle_output() {
    let mut s = stream();
    let mut peer = RawAddress([0xaa; 16]);
    let mut out = 77;
    let mut completed = 7;
    let mut h = Fixture::new(vec![
        bytes(Command::Accept(41, [0xaa; 16], 16), u32::MAX, &[0x31, 0x42]),
        step(Command::Error, 10035),
    ]);
    assert!(s.accept(&mut h, &mut out, &mut peer, &mut completed));
    assert_eq!((out, completed), (0xffff_ffff, 0));
    assert_eq!(&peer.0[..3], &[0x31, 0x42, 0xaa]);
    assert_eq!(fields(&s), (41, 1, 3, 8, 10035));
    h.done();
}

#[test]
fn l10_object_accept_replaces_destination_without_closing_old() {
    let mut listener = stream();
    listener.socket.set_style(9, 8, 7);
    let mut dest = stream();
    dest.socket.handle = 77;
    dest.socket.peer = RawAddress([0xaa; 16]);
    let mut h = Fixture::new(vec![bytes(Command::Accept(41, [0xaa; 16], 16), 89, &PEER)]);
    let mut completed = 7;
    assert!(listener.accept_into(&mut h, &mut dest, &mut completed));
    assert_eq!(completed, 1);
    assert_eq!(fields(&dest), (89, 1, 2, 1, 0));
    assert_eq!(dest.socket.peer.0, PEER);
    assert_eq!((dest.socket.family, dest.socket.style, dest.socket.protocol), (9, 8, 7));
    h.done();
}

#[test]
fn accept_failure_does_not_style_or_state_destination() {
    let mut listener = stream();
    let mut dest = stream();
    dest.socket.handle = 77;
    let mut h = Fixture::new(vec![
        step(Command::Accept(41, [0; 16], 16), u32::MAX),
        step(Command::Error, 10061),
    ]);
    let mut completed = 7;
    assert!(!listener.accept_into(&mut h, &mut dest, &mut completed));
    assert_eq!((completed, dest.socket.handle, dest.state), (0, 0xffff_ffff, 3));
    h.done();
}

#[test]
fn l11_teardown_uses_base_close_and_ordered_opaque_cleanup() {
    let mut s = stream();
    s.socket.auxiliary = Some(Auxiliary { record: 81, handle: 82 });
    s.socket.target = Some(83);
    let mut h = Fixture::new(vec![
        step(Command::Close(41), 0),
        step(Command::Cleanup(82), 0),
        step(Command::FreeRecord(81), 0),
        step(Command::Release(83), 99),
        step(Command::FreeObject(84), 0),
    ]);
    assert_eq!(s.teardown(&mut h, 84, 3), 84);
    assert_eq!(fields(&s), (0xffff_ffff, 0, 3, 1, 0));
    assert_eq!((s.socket.auxiliary, s.socket.target), (None, None));
    h.done();
}

#[test]
fn saved_handle_teardown_skips_close_and_does_not_normalize_save() {
    let mut s = stream();
    s.socket.set_save(0xa5);
    let mut h = Fixture::new(vec![]);
    assert_eq!(s.teardown(&mut h, 84, 2), 84);
    assert_eq!(fields(&s), (41, 1, 3, 1, 0));
    assert_eq!(s.socket.save, 0xa5);
    h.done();
}

#[test]
fn destructor_close_failure_keeps_stream_state_and_still_releases_resources() {
    let mut s = stream();
    s.socket.target = Some(83);
    let mut h = Fixture::new(vec![
        step(Command::Close(41), u32::MAX),
        step(Command::Error, 10038),
        step(Command::Release(83), 0),
    ]);
    assert_eq!(s.teardown(&mut h, 84, 0), 84);
    assert_eq!(fields(&s), (0xffff_ffff, 0, 3, 11, 10038));
    assert_eq!(s.socket.target, None);
    h.done();
}

#[test]
fn object_accept_ignores_false_style_result_after_handle_peer_and_state() {
    struct Destination {
        inner: Stream,
        calls: Vec<&'static str>,
    }
    impl AcceptDestination for Destination {
        fn socket_mut(&mut self) -> &mut Socket {
            self.calls.push("socket");
            &mut self.inner.socket
        }
        fn set_connected(&mut self) {
            assert_eq!(self.inner.socket.handle, 89);
            assert_eq!(self.inner.socket.peer.0, PEER);
            self.calls.push("state");
            self.inner.state = 2;
        }
        fn set_socket_style(&mut self, family: u32, style: u32, protocol: u32) -> bool {
            assert_eq!(self.inner.state, 2);
            assert_eq!((family, style, protocol), (9, 8, 7));
            self.calls.push("style");
            false
        }
    }
    let mut listener = stream();
    listener.socket.set_style(9, 8, 7);
    let mut dest = Destination {
        inner: stream(),
        calls: vec![],
    };
    let mut h = Fixture::new(vec![bytes(Command::Accept(41, [0; 16], 16), 89, &PEER)]);
    let mut completed = 7;
    assert!(listener.accept_into(&mut h, &mut dest, &mut completed));
    assert_eq!(completed, 1);
    assert_eq!(dest.calls, ["socket", "state", "style"]);
    h.done();
}

#[test]
fn successful_bind_and_address_connect_overload() {
    let mut s = stream();
    let mut h = Fixture::new(vec![
        step(Command::Bind(41, LOCAL, 16), 0),
        step(Command::TimerNew, 0),
        step(Command::Connect(41, PEER, 16), 0),
        step(Command::TimerDrop, 0),
    ]);
    assert!(s.socket.bind(&mut h, RawAddress(LOCAL)));
    assert_eq!(s.socket.bound, 1);
    let mut completed = 7;
    assert_eq!(s.connect_address(&mut h, RawAddress(PEER), &mut completed, 0), Ok(true));
    assert_eq!((completed, s.socket.peer.0, s.state), (1, PEER, 2));
    h.done();
}

#[test]
fn captured_callback_has_no_owner_access_and_preserves_raw_error_bits() {
    let s = stream();
    let mut h = Fixture::new(vec![
        step(Command::Close(77), 0),
        step(Command::Close(89), 1),
        step(Command::Error, 0xfedc_ba98),
    ]);
    assert_eq!(captured_close(&mut h, 77), 0);
    assert_eq!(captured_close(&mut h, 89), 0xfedc_ba98);
    assert_eq!(fields(&s), (41, 1, 3, 1, 0));
    h.done();
}

#[test]
fn i1_one_partial_read_preserves_stale_errors_and_unwritten_suffix() {
    let mut s = stream();
    s.socket.error = 34;
    s.socket.raw_error = 10061;
    let mut h = Fixture::new(vec![bytes(Command::Recv(41, 8, 8, 0), 3, &[0x31, 0x42, 0x53])]);
    let mut buffer = [0xaa; 8];
    let mut count = 8;
    assert_eq!(s.read(&mut h, &mut buffer, &mut count), Ok(true));
    assert_eq!(count, 3);
    assert_eq!(buffer, [0x31, 0x42, 0x53, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa]);
    assert_eq!(fields(&s), (41, 1, 3, 34, 10061));
    h.done();
}

#[test]
fn i2_i3_zero_and_negative_read_bypasses_and_zero_other_transfers() {
    let mut s = stream();
    let mut h = Fixture::new(vec![]);
    let mut buffer = [0xaa; 8];
    for mut count in [0, -2] {
        let original = count;
        assert_eq!(s.read(&mut h, &mut buffer, &mut count), Ok(false));
        assert_eq!(count, original);
    }
    let mut count = 0;
    assert_eq!(s.peek(&mut h, &mut buffer, &mut count), Ok(true));
    assert_eq!(s.read_oob(&mut h, &mut buffer, &mut count), Ok(true));
    assert_eq!(s.write(&mut h, &buffer, &mut count, 4), Ok(true));
    assert_eq!(s.write_oob(&mut h, &buffer, &mut count, 4), Ok(true));
    assert_eq!(buffer, [0xaa; 8]);
    assert_eq!(fields(&s), (41, 1, 3, 1, 0));
    h.done();
}

#[test]
fn negative_extents_and_short_storage_are_unsupported() {
    let mut s = stream();
    let mut h = Fixture::new(vec![]);
    let mut buffer = [0xaa; 8];
    let mut count = -2;
    assert_eq!(s.peek(&mut h, &mut buffer, &mut count), Err(Unsupported::BufferExtent));
    assert_eq!(
        s.read_oob(&mut h, &mut buffer, &mut count),
        Err(Unsupported::BufferExtent)
    );
    assert_eq!(s.write(&mut h, &buffer, &mut count, 0), Err(Unsupported::BufferExtent));
    assert_eq!(
        s.write_oob(&mut h, &buffer, &mut count, 0),
        Err(Unsupported::BufferExtent)
    );
    assert_eq!(count, -2);
    count = 9;
    assert_eq!(s.read(&mut h, &mut buffer, &mut count), Err(Unsupported::BufferExtent));
    h.done();
}

#[test]
fn i4_receive_errors_keep_raw_writes_and_no_retry() {
    for (err, e, expected) in [(10035, 8, true), (10036, 9, true), (10061, 34, false)] {
        let mut s = stream();
        let mut count = 8;
        let mut buffer = [0xaa; 8];
        let mut h = Fixture::new(vec![
            bytes(Command::Recv(41, 8, 8, 0), u32::MAX, &[0x31]),
            step(Command::Error, err),
        ]);
        assert_eq!(s.read(&mut h, &mut buffer, &mut count), Ok(expected));
        assert_eq!(count, 0);
        assert_eq!(&buffer[..2], &[0x31, 0xaa]);
        assert_eq!(fields(&s), (41, 1, 3, e, err));
        h.done();
    }
}

#[test]
fn i5_eof_closes_stream_and_close_failure_overrides_eof_error() {
    for (close, e, r) in [(0, 30, 10057), (u32::MAX, 11, 10038)] {
        let mut s = stream();
        let mut count = 8;
        let mut buffer = [0xaa; 8];
        let mut steps = vec![step(Command::Recv(41, 8, 8, 0), 0), step(Command::Close(41), close)];
        if close != 0 {
            steps.push(step(Command::Error, 10038));
        }
        let mut h = Fixture::new(steps);
        assert_eq!(s.read(&mut h, &mut buffer, &mut count), Ok(false));
        assert_eq!(count, 0);
        assert_eq!(fields(&s), (0xffff_ffff, 0, 0, e, r));
        h.done();
    }
}

#[test]
fn peek_and_oob_use_flags_and_eof_close() {
    for flags in [2, 1] {
        let mut s = stream();
        let mut count = 8;
        let mut buffer = [0xaa; 8];
        let mut h = Fixture::new(vec![
            step(Command::Recv(41, 8, 8, flags), 0),
            step(Command::Close(41), 0),
        ]);
        let result = if flags == 2 {
            s.peek(&mut h, &mut buffer, &mut count)
        } else {
            s.read_oob(&mut h, &mut buffer, &mut count)
        };
        assert_eq!(result, Ok(false));
        assert_eq!(fields(&s), (0xffff_ffff, 0, 0, 30, 10057));
        h.done();
    }
}

#[test]
fn i6_partial_and_zero_send_succeed_without_eof_close() {
    for n in [3, 0] {
        let mut s = stream();
        let mut count = 8;
        let mut h = Fixture::new(vec![step(Command::Send(41, vec![0xaa; 8], 8, 4), n)]);
        assert_eq!(s.write(&mut h, &[0xaa; 8], &mut count, 4), Ok(true));
        assert_eq!(count, n as i32);
        assert_eq!(fields(&s), (41, 1, 3, 1, 0));
        h.done();
    }
}

#[test]
fn i7_send_wouldblock_and_oob_flag_or() {
    let mut s = stream();
    let mut count = 8;
    let mut h = Fixture::new(vec![
        step(Command::Send(41, vec![0xaa; 8], 8, 4), u32::MAX),
        step(Command::Error, 10035),
        step(Command::Send(41, vec![0xaa; 8], 8, 5), 3),
    ]);
    assert_eq!(s.write(&mut h, &[0xaa; 8], &mut count, 4), Ok(true));
    assert_eq!(count, 0);
    count = 8;
    assert_eq!(s.write_oob(&mut h, &[0xaa; 8], &mut count, 4), Ok(true));
    assert_eq!(count, 3);
    assert_eq!(fields(&s), (41, 1, 3, 8, 10035));
    h.done();
}

#[test]
fn a1_a2_raw_address_bytes_survive_setters_bind_and_connect() {
    let mut s = stream();
    assert!(s.socket.set_peer(RawAddress(PEER)));
    assert!(s.socket.set_local(RawAddress(LOCAL)));
    assert_eq!(u16::from_le_bytes([s.socket.peer.0[2], s.socket.peer.0[3]]), 0x3412);
    assert_eq!(
        u32::from_le_bytes(s.socket.peer.0[4..8].try_into().unwrap()),
        0x6745_2301
    );
    let mut h = Fixture::new(vec![
        step(Command::Bind(41, LOCAL, 16), 1),
        step(Command::Error, 10049),
        step(Command::TimerNew, 0),
        step(Command::Connect(41, PEER, 16), 0),
        step(Command::TimerDrop, 0),
    ]);
    assert!(!s.socket.bind_local(&mut h));
    assert_eq!(s.socket.local.0, LOCAL);
    assert_eq!(fields(&s), (41, 0, 3, 22, 10049));
    let mut completed = 7;
    assert_eq!(s.connect(&mut h, &mut completed, 0), Ok(true));
    assert_eq!(completed, 1);
    assert_eq!(fields(&s), (41, 0, 2, 22, 10049));
    h.done();
}

#[test]
fn a3_raw_port_patterns_no_native_adapter_certification() {
    let a = RawAddress([2, 0, 0x34, 0x12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let b = RawAddress(PEER);
    assert_eq!(u16::from_le_bytes([a.0[2], a.0[3]]), 0x1234);
    assert_eq!(u16::from_le_bytes([b.0[2], b.0[3]]), 0x3412);
    assert_eq!(require_native_address_adapter(), Err(Unsupported::NativeAddressAdapter));
}

#[test]
fn get_names_use_input_handle_and_keep_partial_writes_on_error() {
    let mut s = stream();
    let mut output = RawAddress([0xaa; 16]);
    let mut h = Fixture::new(vec![
        bytes(Command::Name(false, 77, [0xaa; 16], 16), 1, &[2, 0]),
        step(Command::Error, 10038),
        bytes(
            Command::Name(
                true,
                41,
                [
                    2, 0, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa,
                ],
                16,
            ),
            0,
            &PEER,
        ),
    ]);
    assert!(!s.socket.get_address(&mut h, false, &77, &mut output));
    assert_eq!(&output.0[..3], &[2, 0, 0xaa]);
    assert!(s.socket.get_my_address(&mut h, true, &mut output));
    assert_eq!(output.0, PEER);
    assert_eq!(s.socket.peer.0, [0; 16]);
    assert_eq!(fields(&s), (41, 1, 3, 11, 10038));
    h.done();
}

#[test]
fn r1_r2_unsigned_timeout_pairs_and_null() {
    for (ms, time) in [
        (999, Some((0, 999000))),
        (1000, Some((1, 0))),
        (1001, Some((1, 1000))),
        (0xffff_fffe, Some((4294967, 294000))),
        (0xffff_ffff, None),
    ] {
        let mut s = stream();
        let mut h = Fixture::new(vec![step(Command::Select(1, Some(vec![41]), None, time), 1)]);
        assert!(s.socket.read_ready(&mut h, ms));
        assert_eq!(fields(&s), (41, 1, 3, 1, 0));
        h.done();
    }
}

#[test]
fn r3_read_error_invalidates_write_error_keeps_handle() {
    for read in [true, false] {
        let mut s = stream();
        s.state = 2;
        let cmd = if read {
            Command::Select(1, Some(vec![41]), None, Some((0, 0)))
        } else {
            Command::Select(1, None, Some(vec![41]), Some((0, 0)))
        };
        let mut h = Fixture::new(vec![step(cmd, u32::MAX), step(Command::Error, 10038)]);
        assert!(!if read {
            s.socket.read_ready(&mut h, 0)
        } else {
            s.socket.write_ready(&mut h, 0)
        });
        assert_eq!(fields(&s), (if read { 0xffff_ffff } else { 41 }, 1, 2, 11, 10038));
        h.done();
    }
}

#[test]
fn r4_combined_select_zero_still_memberships_error_has_no_error_fetch() {
    let mut s = stream();
    let mut h = Fixture::new(vec![
        step(Command::Select(0, Some(vec![41]), Some(vec![41]), Some((0, 0))), 0),
        step(Command::Member(41, vec![41]), 0),
        step(Command::Member(41, vec![41]), 0),
        step(
            Command::Select(0, Some(vec![41]), Some(vec![41]), Some((0, 0))),
            u32::MAX,
        ),
    ]);
    assert_eq!(s.socket.readable_or_writable(&mut h, 3, 0), 0);
    assert_eq!(s.socket.readable_or_writable(&mut h, 3, 0), 0xffff_fffc);
    assert_eq!(fields(&s), (41, 1, 3, 1, 0));
    h.done();
}

#[test]
fn combined_ignores_high_mask_bits_and_observes_mutated_sets() {
    let mut s = stream();
    let mut select = step(Command::Select(0, Some(vec![41]), None, None), 1);
    select.sets = Some((vec![], vec![]));
    let mut h = Fixture::new(vec![
        select,
        step(Command::Member(41, vec![]), 0),
        step(Command::Member(41, vec![]), 7),
    ]);
    assert_eq!(s.socket.readable_or_writable(&mut h, 0xffff_fffd, u32::MAX), 2);
    h.done();
}

#[test]
fn r5_waiting_size_select_error_preserves_sentinel_and_handle() {
    let mut s = stream();
    let mut out = 0xdead_beef;
    let mut h = Fixture::new(vec![
        step(Command::Select(1, Some(vec![41]), None, Some((1, 0))), u32::MAX),
        step(Command::Error, 10038),
    ]);
    assert!(!s.socket.waiting_size(&mut h, &mut out, 1000));
    assert_eq!(out, 0xdead_beef);
    assert_eq!(fields(&s), (41, 1, 3, 11, 10038));
    h.done();
}

#[test]
fn r6_ioctl_failure_overwrites_raw_output_and_skips_select() {
    let mut s = stream();
    let mut out = 0xdead_beef;
    let mut ioctl = step(Command::Ioctl(41, 0x4004_667f, 0xdead_beef), 1);
    ioctl.size = Some(7);
    let mut h = Fixture::new(vec![ioctl, step(Command::Error, 10038)]);
    assert!(!s.socket.waiting_size(&mut h, &mut out, 0));
    assert_eq!(out, 0);
    assert_eq!(fields(&s), (41, 1, 3, 11, 10038));
    h.done();
}

#[test]
fn waiting_size_timeout_and_positive_select_ioctl_success() {
    let mut s = stream();
    let mut out = 0xdead_beef;
    let mut ioctl = step(Command::Ioctl(41, 0x4004_667f, 0), 0);
    ioctl.size = Some(17);
    let mut h = Fixture::new(vec![
        step(Command::Select(1, Some(vec![41]), None, Some((1, 1_000))), 0),
        step(Command::Select(1, Some(vec![41]), None, None), 1),
        ioctl,
    ]);
    assert!(s.socket.waiting_size(&mut h, &mut out, 1001));
    assert_eq!(out, 0);
    assert!(s.socket.waiting_size(&mut h, &mut out, u32::MAX));
    assert_eq!(out, 17);
    h.done();
}

#[test]
fn r7_connected_select_zero_is_true_error_invalidates_without_close() {
    let mut s = stream();
    s.state = 0;
    let mut h = Fixture::new(vec![
        step(Command::Select(1, Some(vec![41]), None, Some((0, 0))), 0),
        step(Command::Select(1, Some(vec![41]), None, Some((0, 0))), u32::MAX),
        step(Command::Error, 10038),
    ]);
    assert!(s.is_connected(&mut h));
    assert_eq!(fields(&s), (41, 1, 0, 1, 0));
    s.state = 2;
    assert!(!s.is_connected(&mut h));
    assert_eq!(fields(&s), (0xffff_ffff, 1, 0, 11, 10038));
    s.state = 3;
    assert!(!s.is_connected(&mut h));
    assert_eq!(s.state, 3);
    h.done();
}

#[test]
fn error_translation_complete_literal_table_and_unknown_patterns() {
    let pairs = [
        (10004, 2),
        (10009, 3),
        (10013, 4),
        (10014, 5),
        (10022, 6),
        (10024, 7),
        (10035, 8),
        (10036, 9),
        (10037, 10),
        (10038, 11),
        (10039, 12),
        (10040, 13),
        (10041, 14),
        (10042, 15),
        (10043, 16),
        (10044, 17),
        (10045, 18),
        (10046, 19),
        (10047, 20),
        (10048, 21),
        (10049, 22),
        (10050, 23),
        (10051, 24),
        (10052, 25),
        (10053, 26),
        (10054, 27),
        (10055, 28),
        (10056, 29),
        (10057, 30),
        (10058, 31),
        (10059, 32),
        (10060, 33),
        (10061, 34),
        (10062, 35),
        (10063, 36),
        (10064, 37),
        (10065, 38),
        (10066, 39),
        (10067, 40),
        (10068, 41),
        (10069, 42),
        (10070, 43),
        (10071, 44),
        (10101, 45),
        (10091, 46),
        (10092, 47),
        (10093, 48),
        (11001, 49),
        (11002, 50),
        (11003, 51),
        (11004, 52),
        (0, 1),
        (10003, 1),
        (10005, 1),
        (10034, 1),
        (10072, 1),
        (10090, 1),
        (10094, 1),
        (10100, 1),
        (10102, 1),
        (11000, 1),
        (11005, 1),
        (0xffff_ffff, 1),
        (0x8000_0000, 1),
    ];
    for (raw, translated) in pairs {
        assert_eq!(translate_error(raw), translated, "{raw}");
    }
}
