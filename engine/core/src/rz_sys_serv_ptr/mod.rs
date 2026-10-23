// SPDX-License-Identifier: GPL-3.0-or-later
//! System-service pointer utility, from `docs/specs/system-service-pointer.md` §§2–4.
//!
//! A wrapper is one pointer-sized field. Construction looks a service up through
//! the framework and stores whatever the lookup wrote, ignoring its status.
//! Destruction, clear-and-release, assignment and the temporary lookup helpers are
//! separate operations with different field/call orders (§6: not one uniform smart
//! pointer). Pointers are opaque tokens, never dereferenced. Foreign methods return
//! normally and do not reenter (SSP3 is unsupported); AddRef/Release results are
//! ignored, as natively.

#[cfg(test)]
mod tests;

/// An opaque nonnull interface pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pointer(u32);

impl Pointer {
    pub const fn new(word: u32) -> Option<Self> {
        if word == 0 {
            None
        } else {
            Some(Self(word))
        }
    }
    pub const fn word(self) -> u32 {
        self.0
    }
}

/// The two literal words passed to the framework lookup (§2.1). They are
/// independent; neither is derived from the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub service: u32,
    pub interface: u32,
}

/// The framework's virtual lookup (`+0x14`): service, interface, output field.
/// It may write `output` (with a pointer or null) or leave it untouched; the
/// returned status is ignored by every body here (§2).
pub trait Framework {
    fn lookup(&mut self, service: u32, interface: u32, output: &mut Option<Pointer>) -> bool;
}

/// The interface's reference methods: virtual `+0x04` (AddRef) and `+0x08`
/// (Release). `field` is what the wrapper holds while the call runs, which is
/// the ordering the spec pins down.
pub trait References {
    fn add_ref(&mut self, target: Pointer, field: Option<Pointer>);
    fn release(&mut self, target: Pointer, field: Option<Pointer>);
}

/// Stored-pointer wrapper (§2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SysServPtr {
    field: Option<Pointer>,
}

impl SysServPtr {
    /// §2: clear, borrow the framework (no AddRef/Release), and when it exists
    /// look the identity up into the field. Status is not tested, so a false
    /// lookup that wrote a pointer still leaves it stored.
    pub fn construct(identity: Identity, framework: Option<&mut dyn Framework>) -> Self {
        let mut wrapper = Self::default();
        wrapper.construct_in_place(identity, framework);
        wrapper
    }

    /// The constructor applied to existing storage: the old word is overwritten,
    /// never released. This is storage initialization, not a reset (§6).
    pub fn construct_in_place(&mut self, identity: Identity, framework: Option<&mut dyn Framework>) {
        self.field = None;
        if let Some(framework) = framework {
            framework.lookup(identity.service, identity.interface, &mut self.field);
        }
    }

    pub fn from_field(field: Option<Pointer>) -> Self {
        Self { field }
    }

    pub fn get(&self) -> Option<Pointer> {
        self.field
    }

    /// §3.1 (D74/D52) and §3.4 (Draw cleanup): one Release of a nonnull field,
    /// which is NOT cleared. The field still names the released pointer
    /// afterwards; the caller must not release this lifetime again.
    pub fn release(&mut self, refs: &mut dyn References) {
        if let Some(target) = self.field {
            refs.release(target, self.field);
        }
    }

    /// Ends the wrapper's lifetime with the §3.1 rule.
    pub fn destroy(mut self, refs: &mut dyn References) {
        self.release(refs);
    }

    /// §3.2: clear the field BEFORE releasing the captured pointer, then hand the
    /// field back. §3.2.1: callers pass that field on as a foreign output address.
    pub fn clear_and_release(&mut self, refs: &mut dyn References) -> &mut Option<Pointer> {
        if let Some(target) = self.field.take() {
            refs.release(target, self.field);
        }
        &mut self.field
    }

    /// §3.3: AddRef the new pointer, then Release the old one, and only then
    /// store. No equality shortcut: assigning the held pointer does both calls.
    pub fn assign(&mut self, new: Option<Pointer>, refs: &mut dyn References) -> &mut Self {
        if let Some(new) = new {
            refs.add_ref(new, self.field);
        }
        if let Some(old) = self.field {
            refs.release(old, self.field);
        }
        self.field = new;
        self
    }
}

/// §4.1: look up into a private temporary, keep its value, release the
/// temporary with the §3.1 rule, and return the kept value. No reference
/// survives these bodies; the result is valid only if something outside keeps
/// the object alive.
pub fn lookup_temporary(
    identity: Identity,
    framework: Option<&mut dyn Framework>,
    refs: &mut dyn References,
) -> Option<Pointer> {
    let mut local = SysServPtr::construct(identity, framework);
    let saved = local.get();
    local.release(refs);
    saved
}

/// Which §2.2 instruction form a stored body uses. The forms share one
/// algorithm; the distinction only identifies the native body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    C40,
    C37,
}

/// One §2.2 stored-construction body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stored {
    pub label: &'static str,
    pub form: Form,
    pub identity: Identity,
}

const fn stored(label: &'static str, form: Form, service: u32, interface: u32) -> Stored {
    Stored {
        label,
        form,
        identity: Identity { service, interface },
    }
}

/// §2.2, in the spec's order. Labels are discovery names, not proof of type.
pub const STORED: [Stored; 22] = [
    stored("cIGZSndSys", Form::C40, 0x23CE_CBA0, 0x1060_77D2),
    stored("cIGZGraphicSystem", Form::C40, 0xC416_025C, 0x0073_283C),
    stored("cIGZMessageServer2", Form::C40, 0x04FA_845B, 0x6522_94C7),
    stored("cIGZPersistResourceManager", Form::C40, 0x056B_906E, 0x656B_8EFC),
    stored("cIGZLuaScriptServer", Form::C40, 0x4BD1_0133, 0x4BD1_0129),
    stored("cIGZDrawSystem", Form::C40, 0xEB2E_A3CB, 0x4B2E_A3B3),
    stored("cIGZUIScriptService", Form::C40, 0x5A35_6E15, 0xFA35_62FA),
    stored("cIGZRegistry", Form::C40, 0x2507_6B9E, 0xA237_613C),
    stored("cIGZClipboardManager", Form::C40, 0x27EB_FFFD, 0x07EC_0010),
    stored("cIGZMessageServer", Form::C40, 0x6406_3387, 0xC108_5722),
    stored("cIGZFileSystem", Form::C40, 0x2073_215D, 0xE020_3660),
    stored("cIHitMan", Form::C40, 0x1B2C_E671, 0xBB1B_D9A8),
    stored("cIGZSndMgr", Form::C40, 0x1AF8_5BFB, 0x3AF8_5BB7),
    stored("nGZLog::cIService", Form::C40, 0x09B6_9C4E, 0x09B6_9C46),
    stored("cIGZCanvas", Form::C40, 0xA346_0DEB, 0x66B2_17A1),
    stored("cIGZFontSys", Form::C40, 0x2067_3960, 0x0067_3960),
    stored("cIGZRealtimeDebugDisplay", Form::C40, 0x8414_A2C2, 0x6054_A2FB),
    stored("cIGZLanguageManager", Form::C40, 0x441E_5070, 0x0054_B7D5),
    stored("cIGZWinCtrlMgr", Form::C40, 0xC2C2_EB0F, 0x22C2_EB1F),
    stored("cIGZWWWSystem", Form::C37, 0x449B_1939, 0x449B_1912),
    stored("cIHitDebugInfo", Form::C37, 0xFBC9_CD04, 0x7BBA_0CBD),
    stored("cIGZWinIMEServer", Form::C37, 0x2AC4_5447, 0x2AC4_4AE7),
];

/// §4.1 temporary lookup helper (Tick label).
pub const TICK_TEMPORARY: Identity = Identity {
    service: 0x86AD_10EC,
    interface: 0x86AD_10ED,
};
/// §4.1 temporary lookup helper (AEMSEvents label).
pub const AEMS_EVENTS_TEMPORARY: Identity = Identity {
    service: 0xAC1F_AE31,
    interface: 0x8C1F_AEB7,
};

/// The stored identity for a §2.2 discovery label.
pub fn stored_identity(label: &str) -> Option<Identity> {
    STORED.iter().find(|s| s.label == label).map(|s| s.identity)
}
