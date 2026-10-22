// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! The notification registry shared by both message servers (§4.2; §4A uses
//! the same algorithms): per message type, the targets in registration order
//! without duplicates, plus the deferred (pending) registration list.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};

use super::Token;

/// One deferred registration request (§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending {
    pub add: bool,
    pub target: Token,
    pub ty: u32,
}

/// Registry state (§4.1 +0x64 depth, +0x68 pending, +0x6C map).
#[derive(Debug, Default, Clone)]
pub struct Registry {
    pub(crate) depth: u32,
    pub(crate) pending: VecDeque<Pending>,
    pub(crate) map: BTreeMap<u32, Vec<Token>>,
}

impl Registry {
    /// The targets registered for `ty`, in registration order.
    pub fn targets(&self, ty: u32) -> &[Token] {
        self.map.get(&ty).map_or(&[], |v| v.as_slice())
    }

    /// The registered types in ascending order.
    pub fn types(&self) -> impl Iterator<Item = u32> + '_ {
        self.map.keys().copied()
    }

    /// The deferred requests, oldest first.
    pub fn pending(&self) -> impl Iterator<Item = Pending> + '_ {
        self.pending.iter().copied()
    }

    /// The registry depth.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Append `target` to `ty`'s list unless present. True if appended.
    fn insert(&mut self, target: Token, ty: u32) -> bool {
        let list = self.map.entry(ty).or_default();
        if list.contains(&target) {
            return false;
        }
        list.push(target);
        true
    }

    /// Remove every occurrence of `target` from `ty`'s list, freeing an
    /// emptied list. Returns how many were removed.
    fn erase(&mut self, target: Token, ty: u32) -> usize {
        let Some(list) = self.map.get_mut(&ty) else {
            return 0;
        };
        let before = list.len();
        list.retain(|&t| t != target);
        let removed = before - list.len();
        if list.is_empty() {
            self.map.remove(&ty);
        }
        removed
    }

    /// Snapshot for a send: the list of `ty`, then (when `wildcard`) the
    /// list of type 0.
    pub(crate) fn snapshot(&self, ty: u32, wildcard: bool) -> Vec<Token> {
        let mut out = self.targets(ty).to_vec();
        if wildcard && ty != 0 {
            out.extend_from_slice(self.targets(0));
        }
        out
    }
}

/// A server that embeds a [`Registry`].
pub trait HasRegistry {
    const RAW_PENDING: bool = false;
    fn registry_mut(&mut self) -> &mut Registry;
}

/// Target reference callbacks. `release_target` receives the server so a
/// target dropping its last reference may re-enter it (its destructor may
/// unregister), which is the path that reaches the pending list (GZ3).
pub trait RegistryHost<S: ?Sized> {
    fn add_ref_target(&mut self, server: &mut S, target: Token);
    fn release_target(&mut self, server: &mut S, target: Token);
}

/// AddNotification (§4.2). Always true.
pub fn add_notification<S, H>(server: &mut S, host: &mut H, target: Token, ty: u32) -> bool
where
    S: HasRegistry + ?Sized,
    H: RegistryHost<S> + ?Sized,
{
    if server.registry_mut().depth != 0 {
        defer(server, host, Pending { add: true, target, ty });
        return true;
    }
    let reg = server.registry_mut();
    reg.depth = 1;
    if reg.insert(target, ty) {
        host.add_ref_target(server, target);
    }
    server.registry_mut().depth = server.registry_mut().depth.wrapping_sub(1);
    flush(server, host);
    true
}

/// RemoveNotification (§4.2). Always true, also when nothing was registered.
pub fn remove_notification<S, H>(server: &mut S, host: &mut H, target: Token, ty: u32) -> bool
where
    S: HasRegistry + ?Sized,
    H: RegistryHost<S> + ?Sized,
{
    if server.registry_mut().depth != 0 {
        defer(server, host, Pending { add: false, target, ty });
        return true;
    }
    let reg = server.registry_mut();
    reg.depth = 1;
    let removed = reg.erase(target, ty);
    // GZ3/GZ6b: unlink before Release, while depth remains positive.
    for _ in 0..removed {
        host.release_target(server, target);
    }
    server.registry_mut().depth = server.registry_mut().depth.wrapping_sub(1);
    flush(server, host);
    true
}

/// Flush (§4.2): if not already flushing and the depth is 0, replay the
/// pending entries front first through add/remove, popping each.
pub fn flush<S, H>(server: &mut S, host: &mut H)
where
    S: HasRegistry + ?Sized,
    H: RegistryHost<S> + ?Sized,
{
    if server.registry_mut().depth != 0 {
        return;
    }
    let flag = if S::RAW_PENDING { &FLUSH1 } else { &FLUSH2 };
    if flag
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let _guard = FlushGuard(flag);
    while let Some(&p) = server.registry_mut().pending.front() {
        if p.add {
            add_notification(server, host, p.target, p.ty);
        } else {
            remove_notification(server, host, p.target, p.ty);
        }
        // Generation 1 drops the add hold BEFORE unlinking; generation 2
        // unlinks first. Replay sees the still-linked front in both cases.
        if S::RAW_PENDING && p.add {
            host.release_target(server, p.target);
        }
        server.registry_mut().pending.pop_front();
        if !S::RAW_PENDING {
            host.release_target(server, p.target);
        }
    }
}

static FLUSH1: AtomicBool = AtomicBool::new(false);
static FLUSH2: AtomicBool = AtomicBool::new(false);

struct FlushGuard(&'static AtomicBool);
impl Drop for FlushGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn defer<S: HasRegistry + ?Sized, H: RegistryHost<S> + ?Sized>(server: &mut S, host: &mut H, p: Pending) {
    if S::RAW_PENDING {
        server.registry_mut().pending.push_back(p);
        if p.add {
            host.add_ref_target(server, p.target);
        }
    } else {
        // GZ6b: two temporary counted wrappers, then the list copy; the
        // temporaries are destroyed before returning from the locked call.
        host.add_ref_target(server, p.target);
        host.add_ref_target(server, p.target);
        server.registry_mut().pending.push_back(p);
        host.add_ref_target(server, p.target);
        host.release_target(server, p.target);
        host.release_target(server, p.target);
    }
}

/// Apply pending (§4.2), run at the start of every MessageSend: an add
/// appends the target if absent; a remove deletes every occurrence; then the
/// list is cleared.
pub fn apply_pending<S, H>(server: &mut S, host: &mut H)
where
    S: HasRegistry + ?Sized,
    H: RegistryHost<S> + ?Sized,
{
    // No depth/flush gate: a nested send applies still-linked pending work.
    // Precondition: foreign callbacks must not invalidate the native live
    // pending/type-list iterator (section 5.1's controlled callbacks do not).
    let mut i = 0;
    while let Some(p) = server.registry_mut().pending.get(i).copied() {
        if p.add {
            if server.registry_mut().insert(p.target, p.ty) && !S::RAW_PENDING {
                host.add_ref_target(server, p.target);
            }
        } else {
            for _ in 0..server.registry_mut().erase(p.target, p.ty) {
                host.release_target(server, p.target);
            }
        }
        i += 1;
    }
    let pending = std::mem::take(&mut server.registry_mut().pending);
    if !S::RAW_PENDING {
        for p in pending {
            host.release_target(server, p.target);
        }
    }
    // GZ-V14: raw adds transfer their hold on insertion; duplicate raw adds
    // deliberately leak it. Raw removes have no hold. Bulk clear releases none.
}

/// Destruction (§4.7): depth 0, free every type list with its references,
/// then the pending list. [choice] Lists are released in ascending type
/// order, each in registration order (GZ8; cleanup visitation is unspecified).
pub fn clear<S, H>(server: &mut S, host: &mut H)
where
    S: HasRegistry + ?Sized,
    H: RegistryHost<S> + ?Sized,
{
    let reg = server.registry_mut();
    reg.depth = 0;
    let map = std::mem::take(&mut reg.map);
    let pending = std::mem::take(&mut reg.pending);
    for t in map.into_values().flatten() {
        host.release_target(server, t);
    }
    if !S::RAW_PENDING {
        for p in pending {
            host.release_target(server, p.target);
        }
    }
}
