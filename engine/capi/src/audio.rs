// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The audio layer over the C ABI (fc4.h "audio", docs/specs/audio.md).
//!
//! The plugin's `cSC4Automaton::StartSFX` / `StopSFX` (slots 61 and 62), the
//! `cSC4AudioEventHandler`, the `cSC4AudioSoundManager` and the effects
//! system's sound component forward their work here. Section references (`§n`)
//! point at `docs/specs/audio.md`.
//!
//! Four opaque handles: [`fc4_audio`] is one instance of the whole layer
//! (the sound-event pool, the automaton's SFX slots, the audio event handler,
//! the sound manager and the process-global instance-id counter); [`fc4_audio_msg`]
//! is a `cRZMessage2Standard`, [`fc4_audio_snd_event`] a `cGZSndEventStd` and
//! [`fc4_audio_component`] the effects sound component. Tokens are opaque host
//! identities (fc4.h): the automaton's occupant, instance handles. 0 is null.
//!
//! # Threads
//!
//! The game calls getters from a second thread while the main thread is inside
//! `OnTick` (HANDOFF §5.1), so the core keeps its state in `RefCell`s and this
//! layer publishes a [`fc4_audio_state`] snapshot behind a `Mutex` after every
//! main-thread call; the getters read only that. `fc4_audio_instance_id` is the
//! one exception: it is a counter, so it is a `AtomicU32` and is safe from
//! either thread.

use std::sync::{Mutex, MutexGuard};

use fc4_core::audio::automaton::AutomatonSfxHost;
use fc4_core::audio::data;
use fc4_core::audio::data::{resolve_extension, SampleContext, SoundManagerTuning};
use fc4_core::audio::effect_component::SoundComponent;
use fc4_core::audio::event_handler::AudioEventHandler;
use fc4_core::audio::message::StandardMessage;
use fc4_core::audio::pool::{page_of, ReleaseOutcome, SndEventPool};
use fc4_core::audio::snd_event::SndEventStd;
use fc4_core::audio::sound_manager::{AudioSoundManager, StopResult};

use crate::guard;

/// The C ABI's opaque audio-layer handle.
#[repr(C)]
pub struct fc4_audio {
    _private: [u8; 0],
}

/// The C ABI's opaque `cRZMessage2Standard`.
#[repr(C)]
pub struct fc4_audio_msg {
    _private: [u8; 0],
}

/// The C ABI's opaque `cGZSndEventStd`.
#[repr(C)]
pub struct fc4_audio_snd_event {
    _private: [u8; 0],
}

/// The C ABI's opaque effects sound component.
#[repr(C)]
pub struct fc4_audio_component {
    _private: [u8; 0],
}

/// One posted message (§6.4): the four fields and the message-server priority.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_audio_post {
    pub struct_size: u32,
    /// `+0x0C` — the type.
    pub type_: u32,
    /// `+0x10` — data1 (the SFX id).
    pub data1: u32,
    /// `+0x14` — data2 (the instance id).
    pub data2: u32,
    /// The `MessagePost` `highPriority` argument: always 0 (§6.4).
    pub high_priority: u32,
    /// `+0x08` — the presence flags: `0x87` for both SFX messages (§6.4).
    pub flags: u32,
    /// `+0x24` — void1, the automaton's occupant.
    pub void1: u64,
}

/// One sound-event pool page (§4.2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_audio_pool_page {
    pub struct_size: u32,
    /// The page base address; the page is `p & 0xFFFFC000` (§4.2).
    pub base: u32,
    /// `+0x08` — the free-list head.
    pub free_head: u32,
    /// `+0x0C` — the count / state word (marker included).
    pub state: u32,
    /// The live-object count: `state` with the `0x40000000` marker removed.
    pub live: u32,
    /// Whether the page carries the in-service marker.
    pub in_service: u32,
    /// 1 when the page is on the active ring.
    pub active: u32,
    /// 1 when the page left the ring but was not handed back.
    pub retired: u32,
    /// 1 when the page went back through pool vtable slot 3.
    pub handed_back: u32,
}

/// The release outcome of §4.2: 0 nothing, 1 handed back (post-dec 0),
/// 2 retired (post-dec `0x40000045`).
pub const FC4_AUDIO_RELEASE_NONE: u32 = 0;
/// The page was handed back through pool vtable slot 3.
pub const FC4_AUDIO_RELEASE_HANDED_BACK: u32 = 1;
/// The page was retired: the marker cleared and the page unlinked.
pub const FC4_AUDIO_RELEASE_RETIRED: u32 = 2;

/// [`StopResult`]: 1 stopped, 0 unknown (§8.3, §14.12).
pub const FC4_AUDIO_STOP_STOPPED: u32 = 1;
/// The instance id was not present: `_StopSound` did nothing.
pub const FC4_AUDIO_STOP_UNKNOWN: u32 = 0;

/// A `SampleContext` (§10.2): 0 HitLists, 1 TLO, 2 sample-data.
pub const FC4_AUDIO_CTX_HITLISTS: u32 = 0;
/// The TLO segment.
pub const FC4_AUDIO_CTX_TLO: u32 = 1;
/// A `[SampleDataSegments]` group.
pub const FC4_AUDIO_CTX_SAMPLE: u32 = 2;

/// The published snapshot the second-thread getters read (HANDOFF §5.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_audio_state {
    pub struct_size: u32,
    /// `automaton+0x8C` — the sound instance id (§3.3).
    pub instance_id: u32,
    /// How many messages the automaton has posted.
    pub posts: u32,
    /// How many sound instances the manager holds.
    pub live_sounds: u32,
    /// How many events the audio event handler has posted to the sound system.
    pub snd_sys_events: u32,
    /// The rate-limit window at `+0x48` (§7.1).
    pub rate_window_ms: u32,
    /// The pool's live object count across its pages.
    pub pooled_events: u32,
    /// `+0x14` — the manager's master volume.
    pub master_volume: u32,
    /// The tuning key `0xAABAA0D3` (`+0x64`).
    pub voice_gate: u32,
    /// The tuning key `0x0A946A57` (`+0x68`).
    pub one_shot: u32,
    /// The tuning key `0x4A946A5D` (`+0x6C`).
    pub update_period: u32,
    /// The component's `+0x1C` started byte.
    pub component_active: u32,
    /// Padding, so the struct is 8-byte aligned on both targets.
    pub reserved: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_audio_post>() == 32);
    assert!(std::mem::size_of::<fc4_audio_pool_page>() == 36);
    assert!(std::mem::size_of::<fc4_audio_state>() == 52);
};

/// The body of an [`fc4_audio`] handle. Mutable state is behind `RefCell`s
/// (main thread only); the snapshot is behind a `Mutex` for the getters.
struct Audio {
    pool: std::cell::RefCell<SndEventPool>,
    automaton: std::cell::RefCell<AutomatonSfxHost>,
    handler: std::cell::RefCell<AudioEventHandler>,
    manager: std::cell::RefCell<AudioSoundManager>,
    component: std::cell::RefCell<Option<SoundComponent>>,
    ids: std::sync::atomic::AtomicU32,
    snapshot: Mutex<fc4_audio_state>,
}

struct MsgHandle {
    m: std::cell::RefCell<StandardMessage>,
}
struct SndEventHandle {
    e: std::cell::RefCell<SndEventStd>,
}
struct ComponentHandle {
    c: std::cell::RefCell<SoundComponent>,
}

impl Audio {
    fn publish(&self) {
        let pooled = {
            let p = self.pool.borrow();
            p.active_pages()
                .iter()
                .chain(p.retired_pages().iter())
                .map(SndEventPool::live_count)
                .sum()
        };
        let (posts, instance_id) = {
            let a = self.automaton.borrow();
            (a.posted.len() as u32, a.state.instance_id)
        };
        let (live_sounds, master_volume, voice_gate, one_shot, update_period) = {
            let m = self.manager.borrow();
            (
                m.instances.len() as u32,
                m.master_volume,
                m.tuning.voice_gate,
                m.tuning.one_shot,
                m.tuning.update_period,
            )
        };
        let (snd_sys_events, rate_window_ms) = {
            let h = self.handler.borrow();
            (h.events.len() as u32, h.limiter.window_ms)
        };
        let component_active = self
            .component
            .borrow()
            .as_ref()
            .map(|c| u32::from(c.is_active()))
            .unwrap_or(0);
        let mut s = lock(&self.snapshot);
        *s = fc4_audio_state {
            struct_size: std::mem::size_of::<fc4_audio_state>() as u32,
            instance_id,
            posts,
            live_sounds,
            snd_sys_events,
            rate_window_ms,
            pooled_events: pooled,
            master_volume,
            voice_gate,
            one_shot,
            update_period,
            component_active,
            reserved: 0,
        };
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---- the layer --------------------------------------------------------

/// Create one instance of the audio layer. Returns NULL on failure.
#[no_mangle]
pub extern "C" fn fc4_audio_create() -> *mut fc4_audio {
    guard(std::ptr::null_mut(), || {
        let a = Audio {
            pool: std::cell::RefCell::new(SndEventPool::new()),
            automaton: std::cell::RefCell::new(AutomatonSfxHost::new()),
            handler: std::cell::RefCell::new(AudioEventHandler::new()),
            manager: std::cell::RefCell::new(AudioSoundManager::new()),
            component: std::cell::RefCell::new(None),
            ids: std::sync::atomic::AtomicU32::new(0),
            snapshot: Mutex::new(fc4_audio_state {
                struct_size: std::mem::size_of::<fc4_audio_state>() as u32,
                ..fc4_audio_state::default()
            }),
        };
        Box::into_raw(Box::new(a)) as *mut fc4_audio
    })
}

/// Destroy a handle from [`fc4_audio_create`]. NULL is a no-op.
///
/// SAFETY: `a` must be NULL or a live pointer from [`fc4_audio_create`].
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_destroy(a: *mut fc4_audio) {
    if !a.is_null() {
        guard((), || drop(Box::from_raw(a as *mut Audio)));
    }
}

/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
unsafe fn with_audio<R>(a: *const fc4_audio, fallback: R, f: impl FnOnce(&Audio) -> R) -> R {
    match (a as *const Audio).as_ref() {
        Some(h) => guard(fallback, || {
            let r = f(h);
            h.publish();
            r
        }),
        None => fallback,
    }
}

/// Read the published snapshot (safe from the second thread).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`; `out` must be valid.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_get_state(a: *const fc4_audio, out: *mut fc4_audio_state) -> bool {
    guard(false, || match (a as *const Audio).as_ref() {
        Some(h) => match out.as_mut() {
            Some(o) => {
                *o = *lock(&h.snapshot);
                true
            }
            None => false,
        },
        None => false,
    })
}

// ---- the message object (§3.2) ----------------------------------------

/// Create a `cRZMessage2Standard`. NULL on failure.
#[no_mangle]
pub extern "C" fn fc4_audio_msg_create() -> *mut fc4_audio_msg {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(MsgHandle {
            m: std::cell::RefCell::new(StandardMessage::new()),
        })) as *mut fc4_audio_msg
    })
}

/// Destroy a message handle. NULL is a no-op.
///
/// SAFETY: `m` must be NULL or a live pointer from [`fc4_audio_msg_create`].
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_destroy(m: *mut fc4_audio_msg) {
    if !m.is_null() {
        guard((), || drop(Box::from_raw(m as *mut MsgHandle)));
    }
}

/// SAFETY: `m` must be NULL or a live `fc4_audio_msg*`.
unsafe fn with_msg<R>(m: *const fc4_audio_msg, fallback: R, f: impl FnOnce(&MsgHandle) -> R) -> R {
    match (m as *const MsgHandle).as_ref() {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// `SetType` (§3.2): type 0 stores 0 and clears presence bit 0 (§14.4).
///
/// SAFETY: `m` must be a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_set_type(m: *mut fc4_audio_msg, t: u32) {
    with_msg(m, (), |h| h.m.borrow_mut().set_type(t));
}

/// `SetData1` (§3.2).
///
/// SAFETY: `m` must be a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_set_data1(m: *mut fc4_audio_msg, v: u32) {
    with_msg(m, (), |h| h.m.borrow_mut().set_data1(v));
}

/// `SetData2` (§3.2).
///
/// SAFETY: `m` must be a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_set_data2(m: *mut fc4_audio_msg, v: u32) {
    with_msg(m, (), |h| h.m.borrow_mut().set_data2(v));
}

/// `SetData3` (§3.2).
///
/// SAFETY: `m` must be a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_set_data3(m: *mut fc4_audio_msg, v: u32) {
    with_msg(m, (), |h| h.m.borrow_mut().set_data3(v));
}

/// `SetData4` (§3.2).
///
/// SAFETY: `m` must be a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_set_data4(m: *mut fc4_audio_msg, v: u32) {
    with_msg(m, (), |h| h.m.borrow_mut().set_data4(v));
}

/// `SetVoid1` (§3.2): a null target clears presence bit 7.
///
/// SAFETY: `m` must be a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_set_void1(m: *mut fc4_audio_msg, p: u64) {
    with_msg(m, (), |h| h.m.borrow_mut().set_void1(p));
}

/// Read the presence flags at `+0x08` (§3.2).
///
/// SAFETY: `m` must be NULL or a live `fc4_audio_msg*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_msg_flags(m: *const fc4_audio_msg) -> u32 {
    with_msg(m, 0, |h| u32::from(h.m.borrow().flags))
}

// ---- the sound event (§3.1) -------------------------------------------

/// Create a `cGZSndEventStd` with the **default** constructor (`0x0096BB0E`):
/// type `0xDB53383B`, `+0x30` and `+0x34` left at 0 (§3.1).
#[no_mangle]
pub extern "C" fn fc4_audio_snd_event_create() -> *mut fc4_audio_snd_event {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(SndEventHandle {
            e: std::cell::RefCell::new(SndEventStd::new()),
        })) as *mut fc4_audio_snd_event
    })
}

/// Create one with the **five-argument** constructor (`0x0096BA6D`): stores
/// `payload` at `+0x30` and writes 1 to `+0x34`.
#[no_mangle]
pub extern "C" fn fc4_audio_snd_event_create5(
    payload: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    data4: u32,
) -> *mut fc4_audio_snd_event {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(SndEventHandle {
            e: std::cell::RefCell::new(SndEventStd::with_payload(payload, data1, data2, data3, data4)),
        })) as *mut fc4_audio_snd_event
    })
}

/// Destroy a sound-event handle. NULL is a no-op.
///
/// SAFETY: `e` must be NULL or a live pointer from the create functions.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_snd_event_destroy(e: *mut fc4_audio_snd_event) {
    if !e.is_null() {
        guard((), || drop(Box::from_raw(e as *mut SndEventHandle)));
    }
}

/// SAFETY: `e` must be NULL or a live `fc4_audio_snd_event*`.
unsafe fn with_event<R>(e: *const fc4_audio_snd_event, fallback: R, f: impl FnOnce(&SndEventHandle) -> R) -> R {
    match (e as *const SndEventHandle).as_ref() {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// `QueryInterface` (§3.1): `0x5B523BA4` answers `this+0x2C` and AddRefs.
/// Returns the interface offset, or `0xFFFFFFFF` when the IID falls through.
///
/// SAFETY: `e` must be a live `fc4_audio_snd_event*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_snd_event_qi(e: *mut fc4_audio_snd_event, iid: u32) -> u32 {
    with_event(e, 0xFFFF_FFFF, |h| match h.e.borrow_mut().query_interface(iid) {
        Some(off) => off as u32,
        None => 0xFFFF_FFFF,
    })
}

/// The `+0x2C` payload interface's `SetValue` (§3.1).
///
/// SAFETY: `e` must be a live `fc4_audio_snd_event*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_snd_event_set_value(e: *mut fc4_audio_snd_event, v: u32) {
    with_event(e, (), |h| h.e.borrow_mut().set_value(v));
}

/// The `+0x2C` payload interface's `GetValue` (§3.1).
///
/// SAFETY: `e` must be NULL or a live `fc4_audio_snd_event*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_snd_event_get_value(e: *const fc4_audio_snd_event) -> u32 {
    with_event(e, 0, |h| h.e.borrow().get_value())
}

/// The `+0x2C` payload interface's `GetFlag` (§3.1).
///
/// SAFETY: `e` must be NULL or a live `fc4_audio_snd_event*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_snd_event_get_flag(e: *const fc4_audio_snd_event) -> u32 {
    with_event(e, 0, |h| u32::from(h.e.borrow().get_flag()))
}

/// The `+0x2C` payload interface's `GetOuter` (§3.1): `this - 0x2C`.
#[no_mangle]
pub extern "C" fn fc4_audio_snd_event_get_outer(secondary: u32) -> u32 {
    SndEventStd::get_outer(secondary)
}

// ---- the automaton's SFX slots (§6) -----------------------------------

/// `cSC4Automaton::StartSFX` (slot 61, `+0xF4`, `0x00509950`): `ret 4`.
/// Returns 1 on success and 0 when the id is 0 (§6.2).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_start_sfx(a: *mut fc4_audio, sfx_id: u32, occupant: u64) -> u32 {
    with_audio(a, 0, |h| h.automaton.borrow_mut().start_sfx(sfx_id, occupant))
}

/// `cSC4Automaton::StopSFX` (slot 62, `+0xF8`, `0x00505B50`): `ret 4`.
/// Always returns 1 (§6.3, §14.3).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_stop_sfx(a: *mut fc4_audio, sfx_id: u32, occupant: u64) -> u32 {
    with_audio(a, 0, |h| h.automaton.borrow_mut().stop_sfx(sfx_id, occupant))
}

/// `HandleEnterState` (`0x00511450`): calls slot 61 only when the SFX id at
/// `state+0x18` is non-zero (§6.1). Returns 1 when the slot was called.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_handle_enter_state(a: *mut fc4_audio, sfx_id: u32, occupant: u64) -> u32 {
    with_audio(a, 0, |h| {
        u32::from(h.automaton.borrow_mut().handle_enter_state(sfx_id, occupant))
    })
}

/// `HandleExitState` (`0x0050E3F0`): calls slot 62 under the same guard
/// (§6.1). Returns 1 when the slot was called.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_handle_exit_state(a: *mut fc4_audio, sfx_id: u32, occupant: u64) -> u32 {
    with_audio(a, 0, |h| {
        u32::from(h.automaton.borrow_mut().handle_exit_state(sfx_id, occupant))
    })
}

/// Read back the `index`th posted message (§6.4).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`; `out` must be valid.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_get_post(a: *const fc4_audio, index: u32, out: *mut fc4_audio_post) -> bool {
    guard(false, || {
        let h = match (a as *const Audio).as_ref() {
            Some(h) => h,
            None => return false,
        };
        let o = match out.as_mut() {
            Some(o) => o,
            None => return false,
        };
        let a = h.automaton.borrow();
        match a.posted.get(index as usize) {
            Some(m) => {
                *o = fc4_audio_post {
                    struct_size: std::mem::size_of::<fc4_audio_post>() as u32,
                    type_: m.msg_type,
                    data1: m.data1,
                    data2: m.data2,
                    high_priority: m.high_priority,
                    flags: u32::from(m.flags),
                    void1: m.void1,
                };
                true
            }
            None => false,
        }
    })
}

// ---- the instance-id counter (§8.4) -----------------------------------

/// `cSC4AudioSoundManager::GenerateNewInstanceId` (`0x004F7BB0`): the
/// pre-increment value of one process-wide counter, never 0 (§8.4). The
/// counter is a single `AtomicU32`, so this is safe from either thread.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_instance_id(a: *mut fc4_audio) -> u32 {
    guard(0, || {
        let h = match (a as *const Audio).as_ref() {
            Some(h) => h,
            None => return 0,
        };
        use std::sync::atomic::Ordering;
        // The same sequence the original inlines (§8.4): a zero counter is
        // forced to 1 first, the pre-increment value is returned and the
        // counter is incremented. The compare-exchange must start from what is
        // actually in the word, so a zero counter stores 2 and returns 1.
        loop {
            let cur = h.ids.load(Ordering::Relaxed);
            let (want, next) = if cur == 0 {
                (0u32, 2u32) // force to 1, then inc: return the pre-inc 1
            } else {
                (cur, cur.wrapping_add(1))
            };
            if h.ids
                .compare_exchange(want, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return if cur == 0 { 1 } else { cur };
            }
        }
    })
}

// ---- the sound-event pool (§4) ----------------------------------------

/// `FUN_005E16E0` (§4.2) — allocate a sound event from the pool. Returns the
/// object address in the pool's address space.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_pool_alloc(a: *mut fc4_audio) -> u32 {
    with_audio(a, 0, |h| h.pool.borrow_mut().alloc())
}

/// `FUN_005E1780` (§4.2) — release. A null pointer is a no-op (§4.3). Returns
/// one of [`FC4_AUDIO_RELEASE_NONE`], [`FC4_AUDIO_RELEASE_HANDED_BACK`] or
/// [`FC4_AUDIO_RELEASE_RETIRED`].
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_pool_free(a: *mut fc4_audio, p: u32) -> u32 {
    with_audio(a, FC4_AUDIO_RELEASE_NONE, |h| match h.pool.borrow_mut().free(p) {
        ReleaseOutcome::None => FC4_AUDIO_RELEASE_NONE,
        ReleaseOutcome::HandedBack => FC4_AUDIO_RELEASE_HANDED_BACK,
        ReleaseOutcome::Retired => FC4_AUDIO_RELEASE_RETIRED,
    })
}

/// The page `p` belongs to: `p & 0xFFFFC000` (§4.2).
#[no_mangle]
pub extern "C" fn fc4_audio_pool_page_of(p: u32) -> u32 {
    page_of(p)
}

/// Whether a request of `size` bytes uses the pool (§4.1): only `0x38`.
#[no_mangle]
pub extern "C" fn fc4_audio_pool_size_uses(size: u32) -> u32 {
    u32::from(fc4_core::audio::pool::size_uses_pool(size as usize))
}

/// Read back the state of the page at `base`.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`; `out` must be valid.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_pool_get_page(
    a: *const fc4_audio,
    base: u32,
    out: *mut fc4_audio_pool_page,
) -> bool {
    guard(false, || {
        let h = match (a as *const Audio).as_ref() {
            Some(h) => h,
            None => return false,
        };
        let o = match out.as_mut() {
            Some(o) => o,
            None => return false,
        };
        let p = h.pool.borrow();
        let active = p.active_pages().iter().any(|q| q.base == base);
        let retired = p.retired_pages().iter().any(|q| q.base == base);
        let handed = p.handed_back_pages().contains(&base);
        if let Some(page) = p.page(base) {
            *o = fc4_audio_pool_page {
                struct_size: std::mem::size_of::<fc4_audio_pool_page>() as u32,
                base,
                free_head: page.free_head,
                state: page.state,
                live: SndEventPool::live_count(page),
                in_service: u32::from(page.state & fc4_core::audio::pool::STATE_MARKER != 0),
                active: u32::from(active),
                retired: u32::from(retired),
                handed_back: u32::from(handed),
            };
            true
        } else {
            false
        }
    })
}

// ---- the audio event handler (§7) -------------------------------------

/// `Init` (`0x004EFE80`, §7.2): subscribe the 13 main message types.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_handler_init(a: *mut fc4_audio) -> u32 {
    with_audio(a, 0, |h| h.handler.borrow_mut().init())
}

/// `HandleCityInitShutdownMessages` (`0x004EE970`, §7.2): `0x26D31EC1` adds
/// the city set of 11 types, `0x26D31EC2` removes exactly that set.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_handler_city_message(a: *mut fc4_audio, type_: u32) {
    with_audio(a, (), |h| h.handler.borrow_mut().handle_city_init_shutdown(type_));
}

/// `HandleAutomataMessages` (`0x004EE560`, §7.3) for a play (`0x8A385DD8`).
/// `rand` is the value `rand()` returned; `payload` is `payload->slot9()`.
/// Returns the handler's `al`.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_handler_play(
    a: *mut fc4_audio,
    payload: u32,
    rand: u32,
    sound_id: u32,
    instance: u32,
) -> u32 {
    with_audio(a, 0, |h| {
        h.handler.borrow_mut().handle_play(payload, rand, sound_id, instance)
    })
}

/// `HandleAutomataMessages` for a stop (`0xAA396960`, §7.3).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_handler_stop(a: *mut fc4_audio, sound_id: u32, instance: u32) -> u32 {
    with_audio(a, 0, |h| h.handler.borrow_mut().handle_stop(sound_id, instance))
}

// ---- the sound manager (§8) -------------------------------------------

/// `cSC4AudioSoundManager::DoMessage` (`0x004F9430`, §8.3): 1 when the type is
/// one of the eight, 0 otherwise. In particular `0x8A385DD8` and `0xAA396960`
/// are **not** handled (§14.11).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_manager_message(a: *mut fc4_audio, type_: u32) -> u32 {
    with_audio(a, 0, |h| u32::from(h.manager.borrow_mut().do_message(type_)))
}

/// `_StartSound` (`0x004F9D70`, §8.3): returns the instance id in use. An
/// instance id that is already live is stopped first.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_manager_start(a: *mut fc4_audio, sound_id: u32, instance: u32, param: u32) -> u32 {
    with_audio(a, 0, |h| h.manager.borrow_mut().start_sound(sound_id, instance, param))
}

/// `_StopSound` (`0x004F94C0`, §8.3): [`FC4_AUDIO_STOP_STOPPED`] or
/// [`FC4_AUDIO_STOP_UNKNOWN`]. A miss does nothing (§14.12).
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_manager_stop(a: *mut fc4_audio, instance: u32, hard: u32) -> u32 {
    with_audio(a, FC4_AUDIO_STOP_UNKNOWN, |h| {
        match h.manager.borrow_mut().stop_sound(instance, hard != 0) {
            StopResult::Stopped => FC4_AUDIO_STOP_STOPPED,
            StopResult::Unknown => FC4_AUDIO_STOP_UNKNOWN,
        }
    })
}

/// `_ReadTuningValues` (`0x004F9250`, §10.8). `read_ok` 0 restores the
/// defaults **and logs the failure**. No clamping is applied.
///
/// SAFETY: `a` must be NULL or a live `fc4_audio*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_manager_tuning(
    a: *mut fc4_audio,
    read_ok: u32,
    voice_gate: u32,
    one_shot: u32,
    update_period: u32,
) {
    with_audio(a, (), |h| {
        let read = (read_ok != 0).then_some(SoundManagerTuning {
            voice_gate,
            one_shot,
            update_period,
            ..SoundManagerTuning::default()
        });
        h.manager.borrow_mut().read_tuning_values(read);
    });
}

// ---- the effects sound component (§9) ---------------------------------

/// Build a sound component from a `cSC4SoundDescription` (§10.9).
#[no_mangle]
pub extern "C" fn fc4_audio_component_create(
    sound_id: u32,
    inv_location_rate: f32,
    length: u32,
) -> *mut fc4_audio_component {
    guard(std::ptr::null_mut(), || {
        let d = data::SoundDescription {
            unused: 0,
            sound_id,
            inv_location_rate,
            length,
        };
        Box::into_raw(Box::new(ComponentHandle {
            c: std::cell::RefCell::new(SoundComponent::new(d)),
        })) as *mut fc4_audio_component
    })
}

/// Destroy a component handle. NULL is a no-op.
///
/// SAFETY: `c` must be NULL or a live pointer from [`fc4_audio_component_create`].
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_component_destroy(c: *mut fc4_audio_component) {
    if !c.is_null() {
        guard((), || drop(Box::from_raw(c as *mut ComponentHandle)));
    }
}

/// SAFETY: `c` must be NULL or a live `fc4_audio_component*`.
unsafe fn with_comp<R>(c: *const fc4_audio_component, fallback: R, f: impl FnOnce(&ComponentHandle) -> R) -> R {
    match (c as *const ComponentHandle).as_ref() {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// `Start(mode)` (slot 0, `0x005BE430`, §9.2). The mode is never read (§9.1).
///
/// SAFETY: `c` must be NULL or a live `fc4_audio_component*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_component_start(
    c: *mut fc4_audio_component,
    mode: u32,
    this_addr: u32,
    snd_sys: u32,
) {
    with_comp(c, (), |h| h.c.borrow_mut().start(mode, this_addr, snd_sys != 0));
}

/// `Stop(mode)` (slot 1, `0x005BE4A0`, §9.2). The mode is never read (§14.14);
/// `+0x1C` is cleared before the sound-system check.
///
/// SAFETY: `c` must be NULL or a live `fc4_audio_component*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_component_stop(c: *mut fc4_audio_component, mode: u32) {
    with_comp(c, (), |h| h.c.borrow_mut().stop(mode));
}

/// `Update(dt)` (slot 4, `0x005BE620`, §9.2). Posts `0xCA45190F` with integer
/// coordinates once per period and self-stops past `length` (§14.15).
///
/// SAFETY: `c` must be NULL or a live `fc4_audio_component*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_component_update(c: *mut fc4_audio_component, dt: f32) {
    with_comp(c, (), |h| h.c.borrow_mut().update(dt));
}

/// `SetTransform` (slot 3, `0x005BE4E0`, §9.1).
///
/// SAFETY: `c` must be NULL or a live `fc4_audio_component*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_component_set_transform(c: *mut fc4_audio_component, x: f32, y: f32, z: f32) {
    with_comp(c, (), |h| h.c.borrow_mut().set_transform(x, y, z));
}

/// `IsActive` (slot 2, `0x00580B50`): the `+0x1C` started byte (§9.3).
///
/// SAFETY: `c` must be NULL or a live `fc4_audio_component*`.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_component_is_active(c: *const fc4_audio_component) -> u32 {
    with_comp(c, 0, |h| u32::from(h.c.borrow().is_active()))
}

/// The ignore bit of the component record `+0x10` (§9.3): bit 0 becomes
/// runtime flag bit 2 (`runtime+0x40 |= 4`).
#[no_mangle]
pub extern "C" fn fc4_audio_ignore_flags(record_flags: u32, runtime_flags: u32) -> u32 {
    fc4_core::audio::effect_component::runtime_flags_for(record_flags, runtime_flags)
}

/// `CheckFinished` (`0x005BFD70`, §9.3): 1 when the component is considered.
#[no_mangle]
pub extern "C" fn fc4_audio_check_finished_considers(runtime_flags: u32) -> u32 {
    u32::from(fc4_core::audio::effect_component::check_finished_considers(
        runtime_flags,
    ))
}

// ---- the data (§10) ---------------------------------------------------

/// Resolve a resource type to its extension in a DB segment (§10.2,
/// §10.11): `context` is one of [`FC4_AUDIO_CTX_HITLISTS`],
/// [`FC4_AUDIO_CTX_TLO`] or [`FC4_AUDIO_CTX_SAMPLE`]. Writes up to
/// `cap` bytes of the extension (without a NUL when `cap` is too small) and
/// returns its length, or `0xFFFFFFFF` when the type resolves to nothing.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_data_extension(type_: u32, context: u32, out: *mut u8, cap: u32) -> u32 {
    guard(0xFFFF_FFFF, || {
        let ctx = match context {
            FC4_AUDIO_CTX_HITLISTS => SampleContext::HitLists,
            FC4_AUDIO_CTX_TLO => SampleContext::Tlo,
            FC4_AUDIO_CTX_SAMPLE => SampleContext::SampleData,
            _ => return 0xFFFF_FFFF,
        };
        let ext = match resolve_extension(type_, ctx) {
            Some(e) => e,
            None => return 0xFFFF_FFFF,
        };
        let bytes = ext.as_bytes();
        if !out.is_null() {
            let n = bytes.len().min(cap as usize);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, n);
        }
        bytes.len() as u32
    })
}

/// The `Sound.dat` inventory (§10.1): 2776 index entries.
#[no_mangle]
pub extern "C" fn fc4_audio_data_entries() -> u32 {
    data::SOUND_DAT_ENTRIES
}

/// The `Sound.dat` size in bytes (§10.1): 122,372,241.
#[no_mangle]
pub extern "C" fn fc4_audio_data_bytes() -> u64 {
    data::SOUND_DAT_BYTES
}

/// Whether an AEMS bank header carries the `"AB"` magic (§10.7, **AU1**):
/// reported, not required to load.
#[no_mangle]
pub extern "C" fn fc4_audio_aems_magic_present(version: u32, magic0: u8, magic1: u8) -> u32 {
    u32::from(magic0 == b'A' && magic1 == b'B' && version != 0)
}

/// The module-record stride a bank of `version` selects (§10.7): `0x1C` for
/// version >= 8, `0x14` below.
#[no_mangle]
pub extern "C" fn fc4_audio_aems_module_stride(version: u32) -> u32 {
    if version >= 8 {
        data::AEMS_MODULE_STRIDE_V8 as u32
    } else {
        data::AEMS_MODULE_STRIDE_V7 as u32
    }
}

// ---- the director (§1.1) ----------------------------------------------

/// `cSC4AudioCOMDirector::GetDirectorID` (`0x004EB1A0`): `0xFDB5CA17`.
#[no_mangle]
pub extern "C" fn fc4_audio_director_id() -> u32 {
    fc4_core::audio::director::DIRECTOR_ID
}

/// How many classes `cSC4AudioCOMDirector`'s constructor registers (§1.1):
/// 92 — nine named managers plus the 83 leaves of §11.
#[no_mangle]
pub extern "C" fn fc4_audio_director_class_count() -> u32 {
    fc4_core::audio::director::AUDIO_DIRECTOR.class_count
}

/// Copy the `index`th name of the `OnStart` order (§1.1) into `out` as a
/// NUL-terminated C string. Returns the name's length without the NUL, or
/// `0xFFFFFFFF` when `index` is out of range. The names are Rust `&str`s, so
/// they are copied rather than handed out as pointers.
///
/// SAFETY: `out` must be valid for `cap` bytes when non-NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_audio_director_on_start(index: u32, out: *mut u8, cap: u32) -> u32 {
    guard(0xFFFF_FFFF, || {
        let name = match fc4_core::audio::director::ON_START_ORDER.get(index as usize) {
            Some(s) => *s,
            None => return 0xFFFF_FFFF,
        };
        let bytes = name.as_bytes();
        if !out.is_null() && cap != 0 {
            let n = bytes.len().min(cap as usize - 1);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, n);
            *out.add(n) = 0;
        }
        bytes.len() as u32
    })
}

// ---- nothing is saved (§12) -------------------------------------------

/// Whether the audio layer has a city save record (§12): always 0.
#[no_mangle]
pub extern "C" fn fc4_audio_has_save_record() -> u32 {
    u32::from(fc4_core::audio::save::HAS_SAVE_RECORD)
}
