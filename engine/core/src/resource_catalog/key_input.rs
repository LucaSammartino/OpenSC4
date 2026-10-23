// SPDX-License-Identifier: GPL-3.0-or-later
//! Key-input owner decisions from resource-catalog §§5–6. Controls and foreign
//! parser storage must be valid; malformed text has no invented rejection rule.

use super::{Effect, Effects, Handle, Key};

pub const SPINNER_ID: u32 = 0x04C93038;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputEffect {
    ReadInstanceText,
    ParseUint32,
    FindSpinner,
    Reentrancy(u8),
    QuerySpinner612ce0c3,
    SpinnerWrite(u32),
    ReleaseSpinner,
    InstanceText(String),
    SelectInstanceText,
    FocusInstanceText,
    Filename(String),
    ForwardMessage { message: u32, argument: u32 },
    DelegateMessage(u32),
    ModalClose(u32),
    HeldSlot10(Handle),
    HeldWindowSlot14(Handle),
    InstallTables,
    DestroyFilename,
}

pub trait InputPrimitives: Effects {
    fn input_effect(&mut self, effect: InputEffect);
    /// Opaque raw returned word. Parsing policy is foreign (RC5).
    fn parse_uint32(&mut self) -> u32;
    /// Raw QI success, with initialized valid spinner-control storage.
    fn spinner_qi(&mut self) -> bool;
    fn name_bytes(&mut self) -> Vec<u8>;
    /// Raw window-manager AL, distinct from the supplied modal close result.
    fn close(&mut self, result: u32) -> bool;
    fn delegate_message(&mut self, message: u32) -> bool;
}

/// Byte-oriented ASCII sanitation; this generates display text only.
pub fn filename(name: &[u8], instance: u32) -> String {
    if name.is_empty() {
        return format!("{instance:08x}");
    }
    let clean: String = name
        .iter()
        .map(|&b| {
            if b.is_ascii_alphabetic() {
                char::from(b)
            } else {
                '_'
            }
        })
        .collect();
    format!("{clean}_{instance:08x}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Search {
    candidate: Key,
    direction: i32,
    supplied: u32,
    complete: bool,
}

/// Can only be constructed by a free occupancy reply, not an arbitrary final key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchComplete {
    key: Key,
    supplied: u32,
}

impl SearchComplete {
    pub fn key(&self) -> Key {
        self.key
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchProgress {
    Query(Key),
    Complete(SearchComplete),
}

impl Search {
    pub fn new(current: Key, requested: u32) -> Self {
        let signed = requested as i32;
        let normalized = if signed < 1 {
            0x7fff
        } else if signed > 0x7fff {
            1
        } else {
            requested
        };
        Self {
            candidate: Key::new(current.t, current.g, normalized),
            direction: if requested < current.i { -1 } else { 1 },
            supplied: requested,
            complete: false,
        }
    }

    pub fn query(&self) -> Key {
        self.candidate
    }
    pub fn direction(&self) -> i32 {
        self.direction
    }

    /// One raw service slot-38 occupancy reply. There is no universal bound or
    /// native failure on a cycle. An all-occupied trace remains Query forever;
    /// a caller may suspend it as a domain boundary, never report native false.
    pub fn reply(&mut self, occupied: bool) -> SearchProgress {
        if self.complete || !occupied {
            self.complete = true;
            return SearchProgress::Complete(SearchComplete {
                key: self.candidate,
                supplied: self.supplied,
            });
        }
        let stepped = (self.candidate.i as i32).wrapping_add(self.direction);
        self.candidate.i = if stepped <= 0 {
            0x7fff
        } else if stepped > 0x7fff {
            0
        } else {
            stepped as u32
        };
        SearchProgress::Query(self.candidate)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpinnerStart {
    Return(bool),
    Search(Search),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    Spinner { control: u32, value: u32 },
    Forward4348b111,
    TextNotification { source: u32 },
    Other(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageProgress {
    Return(bool),
    Search(Search),
}

/// Three held pointers represent references already held by this owner. No QI
/// ownership policy is inferred; text and spinner traces deliberately differ.
#[derive(Debug)]
pub struct KeyInput {
    pub key: Key,
    pub reentrancy: u8,
    pub held: [Option<Handle>; 3],
    pub display_filename: String,
}

impl KeyInput {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            reentrancy: 0,
            held: [None; 3],
            display_filename: String::new(),
        }
    }

    pub fn set_filename(&mut self, host: &mut impl InputPrimitives) {
        self.display_filename = filename(&host.name_bytes(), self.key.i);
        host.input_effect(InputEffect::Filename(self.display_filename.clone()));
    }

    fn spinner_write(&mut self, release_after_qi: bool, host: &mut impl InputPrimitives) {
        host.input_effect(InputEffect::FindSpinner);
        self.reentrancy = 1;
        host.input_effect(InputEffect::Reentrancy(1));
        host.input_effect(InputEffect::QuerySpinner612ce0c3);
        if host.spinner_qi() {
            host.input_effect(InputEffect::SpinnerWrite(self.key.i));
            // RC5: text notification has NO Release; do not repair it here.
            if release_after_qi {
                host.input_effect(InputEffect::ReleaseSpinner);
            }
        }
        self.reentrancy = 0;
        host.input_effect(InputEffect::Reentrancy(0));
    }

    pub fn text_notification(&mut self, source: u32, host: &mut impl InputPrimitives) -> bool {
        match source {
            0x283B52CE => self.set_filename(host),
            0xE83B22EE => {
                host.input_effect(InputEffect::ReadInstanceText);
                host.input_effect(InputEffect::ParseUint32);
                self.key.i = host.parse_uint32();
                self.spinner_write(false, host);
                self.set_filename(host);
            }
            _ => {}
        }
        true
    }

    pub fn spinner_start(&self, control: u32, requested: u32) -> SpinnerStart {
        if control != SPINNER_ID {
            return SpinnerStart::Return(false);
        }
        if self.reentrancy != 0 {
            return SpinnerStart::Return(true);
        }
        SpinnerStart::Search(Search::new(self.key, requested))
    }

    /// Resume only after the steppable search obtains a free reply. This owns
    /// the remaining decoded continuation, not allocator setup at RC5's split.
    pub fn spinner_finish(
        &mut self,
        completed: SearchComplete,
        host: &mut impl InputPrimitives,
    ) -> bool {
        self.key.i = completed.key.i;
        if self.key.i != completed.supplied {
            self.spinner_write(true, host);
        }
        host.input_effect(InputEffect::InstanceText((self.key.i as i32).to_string()));
        host.input_effect(InputEffect::SelectInstanceText);
        host.input_effect(InputEffect::FocusInstanceText);
        self.set_filename(host);
        false
    }

    /// Typed primitive message fields, not native pointer decoding.
    pub fn message(
        &mut self,
        message: Message,
        host: &mut impl InputPrimitives,
    ) -> MessageProgress {
        match message {
            Message::Spinner { control, value } => match self.spinner_start(control, value) {
                SpinnerStart::Return(al) => MessageProgress::Return(al),
                SpinnerStart::Search(search) => MessageProgress::Search(search),
            },
            Message::Forward4348b111 => {
                host.input_effect(InputEffect::ForwardMessage {
                    message: 0x4348B111,
                    argument: 0,
                });
                MessageProgress::Return(true)
            }
            Message::TextNotification { source } => {
                MessageProgress::Return(self.text_notification(source, host))
            }
            Message::Other(id) => {
                host.input_effect(InputEffect::DelegateMessage(id));
                MessageProgress::Return(host.delegate_message(id))
            }
        }
    }

    pub fn command(&self, command: u32, control: u32, host: &mut impl InputPrimitives) -> bool {
        if command != 0x287259F6 {
            return false;
        }
        let result = match control {
            0x24C93018 => 0,
            0x24C93019 => 1,
            _ => return false,
        };
        host.input_effect(InputEffect::ModalClose(result));
        host.close(result)
    }

    pub fn shutdown(&mut self, host: &mut impl InputPrimitives) {
        host.emit(Effect::HideOwner);
        host.emit(Effect::Window158Zero);
        for h in &mut self.held[..2] {
            if let Some(pointer) = h.take() {
                host.input_effect(InputEffect::HeldSlot10(pointer));
                host.emit(Effect::Release(pointer));
            }
        }
        if let Some(pointer) = self.held[2].take() {
            host.input_effect(InputEffect::HeldWindowSlot14(pointer));
            host.emit(Effect::Release(pointer));
        }
        host.emit(Effect::BaseShutdown);
    }

    /// Key-input deleting destructor explicitly calls Shutdown, unlike catalog.
    pub fn destroy(&mut self, flag: u32, host: &mut impl InputPrimitives) {
        host.input_effect(InputEffect::InstallTables);
        self.shutdown(host);
        host.input_effect(InputEffect::DestroyFilename);
        host.emit(Effect::DestroyBaseWindow);
        if flag & 1 != 0 {
            host.emit(Effect::HeapFree);
        }
    }
}
