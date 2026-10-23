// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::atomic::{AtomicI32, Ordering};
static TAG_COUNTER: AtomicI32 = AtomicI32::new(0);
const TAG_TABLE: f64 = 1262289877.0;
const TAG_FIELD: f64 = 187693250.0;

impl<A: LuaApi> Session<'_, A> {
    pub(super) fn open_compatibility(&mut self) -> Result<bool> {
        let source = self.api.compatibility_source();
        if self.do_inner(Op::LoadBuffer {
            bytes: source.clone(),
            name: source,
        })? != 0
        {
            return Ok(false);
        }
        for (name, callback) in [
            (b"tag".as_slice(), Callback::Tag),
            (b"settag", Callback::SetTag),
            (b"settagmethod", Callback::SetTagMethod),
            (b"gettagmethod", Callback::GetTagMethod),
            (b"newtag", Callback::NewTag),
            (b"copytagmethods", Callback::CopyTagMethods),
            (b"globals", Callback::Globals),
        ] {
            self.register_inner(callback, name)?;
        }
        Ok(true)
    }
    fn metatable(&mut self, tag: i32) -> Result<()> {
        let tag = self.absolute(tag)?;
        self.op(Op::PushNumber(TAG_TABLE))?;
        self.op(Op::RawGet(REGISTRY))?;
        if self.op(Op::Type(-1))?.int()? == 0 {
            self.op(Op::SetTop(-2))?;
            self.op(Op::NewTable)?;
            self.op(Op::PushNumber(TAG_TABLE))?;
            self.op(Op::PushValue(-2))?;
            self.op(Op::RawSet(REGISTRY))?;
        }
        self.op(Op::PushValue(tag))?;
        self.op(Op::RawGet(-2))?;
        if self.op(Op::Type(-1))?.int()? == 0 {
            self.op(Op::SetTop(-2))?;
            self.op(Op::NewTable)?;
            self.op(Op::PushNumber(TAG_FIELD))?;
            self.op(Op::PushValue(tag))?;
            self.op(Op::RawSet(-3))?;
            self.op(Op::PushValue(tag))?;
            self.op(Op::PushValue(-2))?;
            self.op(Op::RawSet(-4))?;
        }
        self.op(Op::Remove(-2))?;
        Ok(())
    }
    fn event_key(&mut self) -> Result<()> {
        let event = self.op(Op::ToString(2))?.bytes()?;
        if let Some(name) = event.as_deref().and_then(metamethod) {
            self.op(Op::PushString(name.to_vec()))?;
        } else {
            self.op(Op::PushNil)?;
        }
        Ok(())
    }
    pub(super) fn callback_inner(&mut self, callback: Callback) -> Result<i32> {
        match callback {
            Callback::Alert => {
                if self.op(Op::Type(-1))?.int()? == 4 {
                    let text = self.op(Op::ToString(-1))?.bytes()?;
                    self.report(1, text.as_deref());
                }
                Ok(0)
            }
            Callback::NewTag => {
                let tag = TAG_COUNTER.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
                self.op(Op::PushNumber(f64::from(tag)))?;
                Ok(1)
            }
            Callback::Globals => {
                self.op(Op::PushValue(GLOBALS))?;
                Ok(1)
            }
            Callback::Tag => {
                if self.op(Op::GetMetatable(1))?.int()? != 0 {
                    self.op(Op::PushNumber(TAG_FIELD))?;
                    self.op(Op::RawGet(-2))?;
                    if self.op(Op::Type(-1))?.int()? == 3 {
                        return Ok(1);
                    }
                }
                self.op(Op::PushNumber(-1.0))?;
                Ok(1)
            }
            Callback::SetTag => {
                self.metatable(2)?;
                self.op(Op::SetMetatable(1))?;
                Ok(0)
            }
            Callback::GetTagMethod => {
                self.metatable(1)?;
                self.event_key()?;
                self.op(Op::RawGet(-2))?;
                Ok(1)
            }
            Callback::SetTagMethod => {
                self.metatable(1)?;
                self.event_key()?;
                self.op(Op::RawGet(-2))?;
                self.event_key()?;
                self.op(Op::PushValue(3))?;
                // An unmapped event supplies a nil key and must raise in Lua.
                self.op(Op::RawSet(-4))?;
                Ok(1)
            }
            Callback::CopyTagMethods => {
                self.metatable(1)?;
                let dst = self.op(Op::GetTop)?.int()?;
                self.metatable(2)?;
                let src = self.op(Op::GetTop)?.int()?;
                self.op(Op::PushNil)?;
                while self.op(Op::Next(src))?.int()? != 0 {
                    if self.op(Op::Type(-2))?.int()? == 4 {
                        self.op(Op::PushValue(-2))?;
                        self.op(Op::PushValue(-2))?;
                        self.op(Op::RawSet(dst))?;
                    }
                    self.op(Op::SetTop(-2))?;
                }
                self.op(Op::PushValue(1))?;
                Ok(1)
            }
            Callback::CoroutineCreate | Callback::CoroutineResume => Err(Error::Unsupported("LH4")),
            Callback::Foreign(_) => Err(Error::BadPrimitiveResult),
        }
    }
}

pub fn metamethod(event: &[u8]) -> Option<&'static [u8]> {
    Some(match event {
        b"index" | b"gettable" => b"__index",
        b"newindex" | b"settable" => b"__newindex",
        b"gc" => b"__gc",
        b"add" => b"__add",
        b"sub" => b"__sub",
        b"mul" => b"__mul",
        b"div" => b"__div",
        b"pow" => b"__pow",
        b"unm" => b"__unm",
        b"le" => b"__le",
        b"concat" => b"__concat",
        b"function" => b"__call",
        _ => return None,
    })
}
