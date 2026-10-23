// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub struct Session<'a, A: LuaApi> {
    pub(super) api: &'a mut A,
    pub(super) state: Handle,
    pub(super) identity: Handle,
    pub(super) reporter: Reporter,
}

impl<A: LuaApi> Session<'_, A> {
    pub(super) fn op(&mut self, op: Op) -> Result<Output> { self.api.invoke(Some(self.state), op) }
    fn guard<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let change = self.api.precision() == 24;
        if change { self.api.set_precision(53); }
        let result = f(self);
        if change { self.api.set_precision(24); }
        result
    }
    /// Complete thin-wrapper surface (§§5/6.2): each variant maps to precisely
    /// one library primitive. Compound host operations below never become oracles.
    /// Unref alone bypasses the precision guard (Q-14).
    pub fn execute(&mut self, op: Op) -> Result<Output> {
        if matches!(op, Op::Unref(_, _)) { self.op(op) } else { self.guard(|s| s.op(op)) }
    }
    pub fn get_top(&mut self) -> Result<i32> { self.execute(Op::GetTop)?.int() }
    pub fn set_top(&mut self, index: i32) -> Result<()> { self.execute(Op::SetTop(index))?; Ok(()) }
    pub fn pop(&mut self, count: i32) -> Result<()> {
        self.set_top(count.wrapping_neg().wrapping_sub(1))
    }
    pub fn to_sint32(&mut self, index: i32) -> Result<i32> {
        self.guard(|s| Ok(crate::x87::ftol32(s.op(Op::ToNumber(index))?.number()?)))
    }
    pub fn to_uint32(&mut self, index: i32) -> Result<u32> { Ok(self.to_sint32(index)? as u32) }
    pub fn is_type(&mut self, index: i32, kind: i32) -> Result<bool> {
        self.guard(|s| Ok(s.op(Op::Type(index))?.int()? == kind))
    }
    pub fn is_none_or_nil(&mut self, index: i32) -> Result<bool> {
        self.guard(|s| Ok(s.op(Op::Type(index))?.int()? <= 0))
    }
    pub fn get_upvalue_index(index: i32) -> i32 { GLOBALS.wrapping_sub(index) }
    pub fn get_registry_table(&mut self) -> Result<()> { self.execute(Op::PushValue(REGISTRY))?; Ok(()) }
    pub fn get_globals(&mut self) -> Result<()> { self.execute(Op::PushValue(GLOBALS))?; Ok(()) }
    pub(super) fn get_global_inner(&mut self, name: &[u8]) -> Result<()> {
        self.op(Op::PushString(c_string(name)))?;
        self.op(Op::GetTable(GLOBALS))?;
        Ok(())
    }
    pub fn get_global(&mut self, name: &[u8]) -> Result<()> { self.guard(|s| s.get_global_inner(name)) }
    pub(super) fn set_global_inner(&mut self, name: &[u8]) -> Result<()> {
        self.op(Op::PushString(c_string(name)))?;
        self.op(Op::Insert(-2))?;
        self.op(Op::SetTable(GLOBALS))?;
        Ok(())
    }
    pub fn set_global(&mut self, name: &[u8]) -> Result<()> { self.guard(|s| s.set_global_inner(name)) }
    pub fn push_string(&mut self, bytes: &[u8]) -> Result<()> {
        self.execute(Op::PushString(c_string(bytes)))?; Ok(())
    }
    pub(super) fn register_inner(&mut self, callback: Callback, name: &[u8]) -> Result<()> {
        self.op(Op::PushCClosure(callback, 0))?;
        self.set_global_inner(name)
    }
    pub fn register(&mut self, callback: Callback, name: &[u8]) -> Result<()> {
        self.guard(|s| s.register_inner(callback, name))
    }
    pub fn reference(&mut self, _ignored: i32) -> Result<i32> { self.execute(Op::Ref(REGISTRY))?.int() }
    pub fn get_ref(&mut self, reference: i32) -> Result<()> {
        self.execute(Op::RawGetI(REGISTRY, reference))?; Ok(())
    }
    pub fn unref(&mut self, reference: i32) -> Result<()> {
        self.op(Op::Unref(REGISTRY, reference))?; Ok(())
    }
    pub fn error(&mut self, _ignored: &[u8]) -> Result<()> { self.execute(Op::Raise)?; Ok(()) }

    pub fn call_protected(&mut self, nargs: i32, nresults: i32, errfn: i32, pop_error: bool) -> Result<i32> {
        self.guard(|s| s.protected_inner(nargs, nresults, errfn, pop_error))
    }
    pub(super) fn protected_inner(&mut self, nargs: i32, nresults: i32, errfn: i32, pop_error: bool) -> Result<i32> {
        let traceback = errfn == 0 && self.reporter != Reporter::Disabled;
        let handler = if traceback {
            let index = self.op(Op::GetTop)?.int()?.wrapping_sub(nargs);
            self.op(Op::PushString(b"_TRACEBACK".to_vec()))?;
            self.op(Op::RawGet(GLOBALS))?;
            self.op(Op::Insert(index))?;
            index
        } else { errfn };
        let status = self.op(Op::PCall(nargs, nresults, handler))?.int()?;
        if traceback { self.op(Op::Remove(handler))?; }
        if status != 0 {
            let message = self.op(Op::ToString(-1))?.bytes()?;
            self.report(status, message.as_deref());
            if pop_error { self.op(Op::SetTop(-2))?; }
        }
        Ok(status)
    }
    pub(super) fn report(&mut self, status: i32, message: Option<&[u8]>) {
        match self.reporter {
            Reporter::Disabled => (),
            Reporter::Callback(callback) => self.api.report_callback(callback, self.identity, status, message),
            Reporter::Default => {
                let kind = match status { 1 => "Run-time", 2 => "File-related", 3 => "Syntax", 4 => "Allocation", _ => "Unknown" };
                // §15 Q8: exact formatting is unsettled. Thread numeric IDs are LH4;
                // preserve wrapper identity explicitly rather than invent an ID.
                let mut text = format!("GZLua5 script error: {kind}; thread wrapper {}; ", self.identity.0).into_bytes();
                text.extend_from_slice(message.unwrap_or_default());
                self.api.log(&text);
            }
        }
    }
    pub fn do_file(&mut self, name: &[u8]) -> Result<i32> {
        self.guard(|s| s.do_inner(Op::LoadFile(c_string(name))))
    }
    pub fn do_string(&mut self, source: &[u8]) -> Result<i32> {
        let source = c_string(source);
        self.do_buffer(&source, &source)
    }
    pub fn do_buffer(&mut self, source: &[u8], name: &[u8]) -> Result<i32> {
        self.guard(|s| s.do_inner(Op::LoadBuffer { bytes: source.to_vec(), name: c_string(name) }))
    }
    pub(super) fn do_inner(&mut self, load: Op) -> Result<i32> {
        let loaded = self.op(load)?.int()?;
        let status = if loaded == 0 { self.op(Op::PCall(0, MULTRET, 0))?.int()? } else { loaded };
        if status != 0 {
            self.get_global_inner(b"_ALERT")?;
            if self.op(Op::Type(-1))?.int()? == 6 {
                self.op(Op::Insert(-2))?;
                self.op(Op::Call(1, 0))?;
            } else {
                self.op(Op::SetTop(-2))?;
                let message = self.op(Op::ToString(-1))?.bytes()?;
                self.api.stderr(message.as_deref());
                self.op(Op::SetTop(-2))?;
            }
        }
        Ok(status)
    }
    pub fn tag(&mut self, index: i32) -> Result<i32> {
        self.guard(|s| {
            let index = s.absolute(index)?;
            s.op(Op::PushCClosure(Callback::Tag, 0))?;
            s.op(Op::PushValue(index))?;
            if s.protected_inner(1, 1, 0, true)? != 0 { return Ok(-1); }
            let value = crate::x87::ftol32(s.op(Op::ToNumber(-1))?.number()?);
            s.op(Op::SetTop(-2))?;
            Ok(if value == 0 { -1 } else { value })
        })
    }
    pub fn new_tag(&mut self) -> Result<i32> {
        self.guard(|s| {
            s.op(Op::PushCClosure(Callback::NewTag, 0))?;
            if s.protected_inner(0, 1, 0, true)? != 0 { return Ok(-1); }
            let value = if s.op(Op::IsNumber(1))?.boolean()? {
                crate::x87::ftol32(s.op(Op::ToNumber(1))?.number()?)
            } else { -1 };
            s.op(Op::SetTop(-2))?;
            Ok(if value > 0 { value } else { -1 })
        })
    }
    pub fn get_tag_method(&mut self, tag: i32, event: &[u8]) -> Result<()> {
        self.guard(|s| {
            s.op(Op::PushCClosure(Callback::GetTagMethod, 0))?;
            s.op(Op::PushNumber(f64::from(tag)))?;
            s.op(Op::PushString(c_string(event)))?;
            if s.protected_inner(2, 1, 0, true)? != 0 { s.op(Op::PushNil)?; }
            Ok(())
        })
    }
    pub fn set_tag_method(&mut self, tag: i32, event: &[u8]) -> Result<()> {
        self.guard(|s| {
            let function = s.op(Op::GetTop)?.int()?;
            s.op(Op::PushCClosure(Callback::SetTagMethod, 0))?;
            s.op(Op::PushNumber(f64::from(tag)))?;
            s.op(Op::PushString(c_string(event)))?;
            s.op(Op::PushValue(function))?;
            s.protected_inner(3, 0, 0, true)?;
            s.op(Op::SetTop(-2))?;
            Ok(())
        })
    }
    pub fn set_tag(&mut self, tag: i32) -> Result<()> {
        self.guard(|s| {
            let value = s.op(Op::GetTop)?.int()?;
            s.op(Op::PushCClosure(Callback::SetTag, 0))?;
            s.op(Op::PushValue(value))?;
            s.op(Op::PushNumber(f64::from(tag)))?;
            s.protected_inner(2, 0, 0, true)?;
            Ok(())
        })
    }
    pub fn copy_tag_methods(&mut self, dst: i32, src: i32) -> Result<i32> {
        self.guard(|s| {
            s.op(Op::PushCClosure(Callback::CopyTagMethods, 0))?;
            s.op(Op::PushNumber(f64::from(dst)))?;
            s.op(Op::PushNumber(f64::from(src)))?;
            s.protected_inner(2, 0, 0, true)?;
            Ok(dst)
        })
    }
    pub fn clone_tag(&mut self, source: i32) -> Result<i32> {
        let tag = self.new_tag()?;
        self.copy_tag_methods(tag, source)?;
        Ok(tag)
    }
    pub fn push_user_tag(&mut self, pointer: u32, tag: i32) -> Result<()> {
        self.guard(|s| {
            let data = s.op(Op::NewUserdata(4))?.handle()?.ok_or(Error::InvalidUserdata)?;
            s.op(Op::WritePointer32(data, pointer))?;
            s.set_tag(tag)
        })
    }
    pub fn to_user_tag(&mut self, index: i32) -> Result<Option<Handle>> {
        self.guard(|s| {
            // Q-3 is an invalid native dereference, not a successful null result.
            let data = s.op(Op::ToUserdata(index))?.handle()?.ok_or(Error::InvalidUserdata)?;
            s.op(Op::ReadPointer32(data))?.handle()
        })
    }
    pub fn get_n(&mut self, index: i32) -> Result<i32> {
        self.guard(|s| {
            if s.op(Op::Type(index))?.int()? != 5 { return Ok(0); }
            let top = s.op(Op::GetTop)?.int()?;
            let absolute = s.absolute(index)?;
            s.get_global_inner(b"getn")?;
            let result = if s.op(Op::Type(-1))?.int()? != 6 { index } else {
                s.op(Op::PushValue(absolute))?;
                if s.protected_inner(1, 1, 0, true)? != 0 { index }
                else { crate::x87::ftol32(s.op(Op::ToNumber(-1))?.number()?) }
            };
            s.op(Op::SetTop(top))?;
            Ok(result)
        })
    }
    pub(super) fn absolute(&mut self, index: i32) -> Result<i32> {
        if index >= 0 || index <= REGISTRY { Ok(index) }
        else { Ok(self.op(Op::GetTop)?.int()?.wrapping_add(index).wrapping_add(1)) }
    }
    pub fn dispatch_callback(&mut self, callback: Callback) -> Result<i32> {
        self.guard(|s| s.callback_inner(callback))
    }
}

pub(super) fn c_string(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().take_while(|b| *b != 0).collect()
}
