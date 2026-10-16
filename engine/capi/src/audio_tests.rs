// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! C ABI tests for the audio layer (fc4.h "audio", docs/specs/audio.md).
//! Each test names the conformance test of §16 it stands behind.

use std::ptr;

use super::audio::*;

/// A handle that destroys itself, so a test cannot leak one.
struct Layer(*mut fc4_audio);
impl Layer {
    fn new() -> Layer {
        let p = fc4_audio_create();
        assert!(!p.is_null());
        Layer(p)
    }
}
impl Drop for Layer {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from `fc4_audio_create` and is used once.
        unsafe { fc4_audio_destroy(self.0) }
    }
}

/// §16.1: only 0x38 uses the pool.
#[test]
fn pool_is_used_only_for_0x38() {
    assert_eq!(fc4_audio_pool_size_uses(0x38), 1);
    assert_eq!(fc4_audio_pool_size_uses(0x2C), 0);
    assert_eq!(fc4_audio_pool_size_uses(0), 0);
}

/// §16.5: the free list is LIFO and `delete(0)` is a no-op.
#[test]
fn pool_free_list_is_lifo() {
    let a = Layer::new();
    // SAFETY: `a.0` is a live handle; the returned addresses are pool values.
    let p1 = unsafe { fc4_audio_pool_alloc(a.0) };
    let p2 = unsafe { fc4_audio_pool_alloc(a.0) };
    assert_ne!(p1, p2);
    assert_eq!(fc4_audio_pool_page_of(p1), fc4_audio_pool_page_of(p2));

    // SAFETY: `a.0` is live.
    assert_eq!(unsafe { fc4_audio_pool_free(a.0, p1) }, FC4_AUDIO_RELEASE_NONE);
    assert_eq!(unsafe { fc4_audio_pool_free(a.0, p2) }, FC4_AUDIO_RELEASE_NONE);
    assert_eq!(unsafe { fc4_audio_pool_free(a.0, 0) }, FC4_AUDIO_RELEASE_NONE);

    // SAFETY: `a.0` is live.
    let q1 = unsafe { fc4_audio_pool_alloc(a.0) };
    let q2 = unsafe { fc4_audio_pool_alloc(a.0) };
    assert_eq!(q1, p2, "LIFO: the last freed is the first returned");
    assert_eq!(q2, p1);

    // A fresh read of the page: both objects are live again, so the free list
    // has moved on and the live count is back where it was.
    let mut after = fc4_audio_pool_page::default();
    // SAFETY: `a.0` is live and `after` is a valid out-parameter.
    assert!(unsafe { fc4_audio_pool_get_page(a.0, fc4_audio_pool_page_of(p1), &mut after) });
    assert_eq!(after.live, 2);
    assert_eq!(after.in_service, 1);
}

/// §16.6: the marker on entry, and the live count the allocs produce.
#[test]
fn pool_state_word_carries_the_marker() {
    let a = Layer::new();
    // SAFETY: `a.0` is a live handle.
    let p = unsafe { fc4_audio_pool_alloc(a.0) };
    let mut page = fc4_audio_pool_page::default();
    // SAFETY: `a.0` is live and `page` is a valid out-parameter.
    assert!(unsafe { fc4_audio_pool_get_page(a.0, fc4_audio_pool_page_of(p), &mut page) });
    assert_eq!(page.in_service, 1, "the page carries the 0x40000000 marker");
    assert_eq!(page.state & 0x4000_0000, 0x4000_0000);
    assert_eq!(page.live, 1, "alloc increments");
    assert_eq!(page.active, 1);
}

/// §16.2 / §16.3: the sound event's two constructors and its payload
/// interface.
#[test]
fn snd_event_constructors_and_payload() {
    // SAFETY: the create functions return owned handles.
    let e = fc4_audio_snd_event_create();
    assert!(!e.is_null());
    // SAFETY: `e` is a live handle.
    assert_eq!(unsafe { fc4_audio_snd_event_get_value(e) }, 0);
    assert_eq!(unsafe { fc4_audio_snd_event_get_flag(e) }, 0);
    assert_eq!(unsafe { fc4_audio_snd_event_qi(e, 0x5B52_3BA4) }, 0x2C);
    assert_eq!(
        unsafe { fc4_audio_snd_event_qi(e, 0x1234_5678) },
        0xFFFF_FFFF,
        "any other IID falls through"
    );
    // SAFETY: `e` is live.
    unsafe {
        fc4_audio_snd_event_set_value(e, 0x00AB_CDEF);
        assert_eq!(fc4_audio_snd_event_get_value(e), 0x00AB_CDEF);
        assert_eq!(fc4_audio_snd_event_get_flag(e), 1);
        fc4_audio_snd_event_destroy(e);
    }
    assert_eq!(fc4_audio_snd_event_get_outer(0x00B0_002C), 0x00B0_0000);

    // SAFETY: the create function returns an owned handle.
    let f = fc4_audio_snd_event_create5(0x00C0_FFEE, 1, 2, 3, 4);
    assert!(!f.is_null());
    // SAFETY: `f` is live.
    unsafe {
        assert_eq!(fc4_audio_snd_event_get_value(f), 0x00C0_FFEE);
        assert_eq!(fc4_audio_snd_event_get_flag(f), 1);
        fc4_audio_snd_event_destroy(f);
    }
}

/// §16.4: the presence flags, and type 0 clearing bit 0.
#[test]
fn message_presence_flags() {
    // SAFETY: the create function returns an owned handle.
    let m = fc4_audio_msg_create();
    assert!(!m.is_null());
    // SAFETY: `m` is live.
    unsafe {
        fc4_audio_msg_set_type(m, 0x8A38_5DD8);
        fc4_audio_msg_set_data1(m, 1);
        fc4_audio_msg_set_data2(m, 2);
        fc4_audio_msg_set_void1(m, 0x00B0_0004);
        assert_eq!(fc4_audio_msg_flags(m), 0x87);
        fc4_audio_msg_set_type(m, 0);
        assert_eq!(fc4_audio_msg_flags(m) & 1, 0, "type 0 clears bit 0 (14.4)");
        fc4_audio_msg_set_void1(m, 0);
        assert_eq!(fc4_audio_msg_flags(m) & 0x80, 0);
        fc4_audio_msg_destroy(m);
    }
}

/// §16.7 / §16.8 / §16.9: the SFX slots.
#[test]
fn sfx_slots() {
    let a = Layer::new();
    // SAFETY: `a.0` is live.
    unsafe {
        assert_eq!(fc4_audio_start_sfx(a.0, 0, 0x00B0_0004), 0, "id 0 does nothing");
        let mut st = fc4_audio_state::default();
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.posts, 0);
        assert_eq!(st.instance_id, 0);

        assert_eq!(fc4_audio_start_sfx(a.0, 0x00CA_FE01, 0x00B0_0004), 1);
        let mut post = fc4_audio_post::default();
        assert!(fc4_audio_get_post(a.0, 0, &mut post));
        assert_eq!(post.type_, 0x8A38_5DD8);
        assert_eq!(post.data1, 0x00CA_FE01);
        assert_eq!(post.data2, 1, "a fresh instance id");
        assert_eq!(post.void1, 0x00B0_0004);
        assert_eq!(post.high_priority, 0, "MessagePost(..., 0)");
        assert_eq!(post.flags, 0x87);

        assert_eq!(fc4_audio_stop_sfx(a.0, 0x00CA_FE02, 0x00B0_0004), 1);
        assert!(fc4_audio_get_post(a.0, 1, &mut post));
        assert_eq!(post.type_, 0xAA39_6960);
        assert_eq!(post.data1, 0x00CA_FE02);
        assert_eq!(post.data2, 1);
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.instance_id, 1, "StopSFX does not clear +0x8C (14.1)");
    }
}

/// §16.11: the state entry / exit guards.
#[test]
fn state_entry_and_exit_are_guarded() {
    let a = Layer::new();
    // SAFETY: `a.0` is live.
    unsafe {
        assert_eq!(fc4_audio_handle_enter_state(a.0, 0, 0), 0);
        assert_eq!(fc4_audio_handle_exit_state(a.0, 0, 0), 0);
        assert_eq!(fc4_audio_handle_enter_state(a.0, 7, 0), 1);
        assert_eq!(fc4_audio_handle_exit_state(a.0, 7, 0), 1);
        let mut st = fc4_audio_state::default();
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.posts, 2);
    }
}

/// §16.20: the instance-id counter is global, pre-increment and never 0.
#[test]
fn instance_id_never_zero() {
    let a = Layer::new();
    // SAFETY: `a.0` is live.
    unsafe {
        assert_eq!(fc4_audio_instance_id(a.0), 1);
        assert_eq!(fc4_audio_instance_id(a.0), 2);
        assert_eq!(fc4_audio_instance_id(a.0), 3);
    }
}

/// §16.21: the sound manager does not see the automata messages.
#[test]
fn manager_does_not_see_the_automata_messages() {
    let a = Layer::new();
    // SAFETY: `a.0` is live.
    unsafe {
        assert_eq!(fc4_audio_manager_message(a.0, 0x8A38_5DD8), 0);
        assert_eq!(fc4_audio_manager_message(a.0, 0xAA39_6960), 0);
        for t in [
            0x28E9_397Bu32,
            0x28E9_397C,
            0x26D3_1EC2,
            0x4348_B111,
            0x4A7F_B7E9,
            0x8A4A_69C7,
            0xAA7F_B7E0,
            0xC53D_10AA,
        ] {
            assert_eq!(fc4_audio_manager_message(a.0, t), 1, "0x{t:08X}");
        }
    }
}

/// §16.22: the manager instance table.
#[test]
fn manager_instance_table() {
    let a = Layer::new();
    // SAFETY: `a.0` is live.
    unsafe {
        assert_eq!(fc4_audio_manager_start(a.0, 1, 0x2A, 0), 0x2A);
        assert_eq!(fc4_audio_manager_start(a.0, 2, 0x2A, 0), 0x2A);
        let mut st = fc4_audio_state::default();
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.live_sounds, 1, "the old instance is stopped first");
        assert_eq!(fc4_audio_manager_stop(a.0, 0x99, 0), FC4_AUDIO_STOP_UNKNOWN);
        assert_eq!(fc4_audio_manager_stop(a.0, 0x2A, 0), FC4_AUDIO_STOP_STOPPED);
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.live_sounds, 0);
    }
}

/// §16.19: tuning is unclamped and a failed read restores the defaults.
#[test]
fn tuning_is_unclamped() {
    let a = Layer::new();
    // SAFETY: `a.0` is live.
    unsafe {
        fc4_audio_manager_tuning(a.0, 1, 0, u32::MAX, 0);
        let mut st = fc4_audio_state::default();
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.voice_gate, 0);
        assert_eq!(st.one_shot, u32::MAX);
        assert_eq!(st.update_period, 0);
        fc4_audio_manager_tuning(a.0, 0, 0, 0, 0);
        assert!(fc4_audio_get_state(a.0, &mut st));
        assert_eq!(st.voice_gate, 3);
        assert_eq!(st.one_shot, 10);
        assert_eq!(st.update_period, 30);
    }
}

/// §16.23: the effects sound component.
#[test]
fn sound_component_lifecycle() {
    // SAFETY: the create function returns an owned handle.
    let c = fc4_audio_component_create(0x00CA_FE01, 2.0, 50);
    assert!(!c.is_null());
    // SAFETY: `c` is live.
    unsafe {
        assert_eq!(fc4_audio_component_is_active(c), 0);
        fc4_audio_component_set_transform(c, 1.7, -2.2, 3.9);
        fc4_audio_component_start(c, 0, 0x00B0_1234, 1);
        assert_eq!(fc4_audio_component_is_active(c), 1);
        fc4_audio_component_update(c, 1.0);
        fc4_audio_component_stop(c, 0);
        assert_eq!(fc4_audio_component_is_active(c), 0);
        fc4_audio_component_destroy(c);
    }
}

/// §16.24: the ignore bit becomes runtime flag bit 2.
#[test]
fn ignore_bit_sets_runtime_flag_bit_2() {
    assert_eq!(fc4_audio_ignore_flags(1, 0), 0x04);
    assert_eq!(fc4_audio_ignore_flags(0, 0x04), 0);
    assert_eq!(fc4_audio_check_finished_considers(0x04), 0);
    assert_eq!(fc4_audio_check_finished_considers(0x00), 1);
}

/// §16.18: sound data by type and extension.
#[test]
fn data_type_and_extension() {
    let mut buf = [0u8; 8];
    // SAFETY: `buf` is a valid out-parameter with a real capacity.
    unsafe {
        let n = fc4_audio_data_extension(0x2026_960B, FC4_AUDIO_CTX_HITLISTS, buf.as_mut_ptr(), 8);
        assert_eq!(&buf[..n as usize], b"str");
        let n = fc4_audio_data_extension(0x2026_960B, FC4_AUDIO_CTX_TLO, buf.as_mut_ptr(), 8);
        assert_eq!(&buf[..n as usize], b"hit");
        let n = fc4_audio_data_extension(0x9D79_6DB4, FC4_AUDIO_CTX_TLO, buf.as_mut_ptr(), 8);
        assert_eq!(&buf[..n as usize], b"tlo");
        assert_eq!(
            fc4_audio_data_extension(0x0B8D_821A, FC4_AUDIO_CTX_TLO, buf.as_mut_ptr(), 8),
            0xFFFF_FFFF,
            "an unbound type resolves to nothing"
        );
    }
    assert_eq!(fc4_audio_data_entries(), 2776);
    assert_eq!(fc4_audio_data_bytes(), 122_372_241);
}

/// §10.7 (AU1): the AEMS bank header's version gates.
#[test]
fn aems_version_gates() {
    assert_eq!(fc4_audio_aems_module_stride(9), 0x1C);
    assert_eq!(fc4_audio_aems_module_stride(8), 0x1C);
    assert_eq!(fc4_audio_aems_module_stride(7), 0x14);
    assert_eq!(fc4_audio_aems_magic_present(9, b'A', b'B'), 1);
    assert_eq!(fc4_audio_aems_magic_present(9, b'X', b'Y'), 0);
}

/// §16.16: the director's identity and OnStart order.
#[test]
fn director_identity_and_order() {
    assert_eq!(fc4_audio_director_id(), 0xFDB5_CA17);
    assert_eq!(fc4_audio_director_class_count(), 92);
    let mut buf = [0u8; 64];
    // SAFETY: `buf` is a valid out-parameter with a real capacity.
    unsafe {
        let n = fc4_audio_director_on_start(0, buf.as_mut_ptr(), 64);
        assert_eq!(&buf[..n as usize], b"GetGZSoundCOMDirector");
        assert_eq!(buf[n as usize], 0, "NUL-terminated");
        assert_eq!(
            fc4_audio_director_on_start(4, buf.as_mut_ptr(), 64),
            34,
            "cFreshnessCOMDirector::GetDirector"
        );
        assert_eq!(fc4_audio_director_on_start(5, buf.as_mut_ptr(), 64), 0xFFFF_FFFF);
        assert_eq!(
            fc4_audio_director_on_start(0, ptr::null_mut(), 0),
            21,
            "GetGZSoundCOMDirector"
        );
    }
}

/// §16.17: nothing in the audio layer is saved.
#[test]
fn nothing_is_saved() {
    assert_eq!(fc4_audio_has_save_record(), 0);
}

/// A NULL handle is refused rather than dereferenced.
#[test]
fn null_handles_are_safe() {
    // SAFETY: every call passes NULL, which each entry point treats as absent.
    unsafe {
        assert!(!fc4_audio_get_state(ptr::null(), ptr::null_mut()));
        assert_eq!(fc4_audio_start_sfx(ptr::null_mut(), 1, 0), 0);
        assert_eq!(fc4_audio_pool_alloc(ptr::null_mut()), 0);
        assert_eq!(fc4_audio_pool_free(ptr::null_mut(), 0), FC4_AUDIO_RELEASE_NONE);
        assert_eq!(fc4_audio_instance_id(ptr::null_mut()), 0);
        assert_eq!(fc4_audio_msg_flags(ptr::null()), 0);
        assert_eq!(fc4_audio_component_is_active(ptr::null()), 0);
        assert!(!fc4_audio_pool_get_page(ptr::null(), 0, ptr::null_mut()));
        assert!(!fc4_audio_get_post(ptr::null(), 0, ptr::null_mut()));
        fc4_audio_destroy(ptr::null_mut());
        fc4_audio_msg_destroy(ptr::null_mut());
        fc4_audio_snd_event_destroy(ptr::null_mut());
        fc4_audio_component_destroy(ptr::null_mut());
    }
}
