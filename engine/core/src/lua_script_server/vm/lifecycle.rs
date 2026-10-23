// SPDX-License-Identifier: GPL-3.0-or-later
//! Registry decisions from lua-host §§6.2.1–6.2.3. Lua execution stays foreign.
use super::*;

/// Only factory, reference/storage and overridable object hooks are foreign.
pub trait ThreadApi: LuaApi {
    /// Borrowed identity of the owner's registry storage (not a host operation).
    fn registry_identity(&mut self, owner: Handle) -> Handle;
    /// Returns an owned factory temporary with its existing acquired count.
    fn factory_thread(&mut self, class: u32, iid: u32) -> Result<Option<Thread>>;
    fn retain(&mut self, object: Handle);
    fn release(&mut self, object: Handle);
    fn on_create(&mut self, thread: &mut Thread, owner: &Thread, context: Handle, id: i32) -> Result<bool> {
        thread.on_create(self, owner, context, id)
    }
    fn on_destroy(&mut self, thread: &mut Thread) -> Result<bool> {
        thread.on_destroy(self)
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub context: Handle,
    pub flags: u32,
    pub parent: Option<Handle>,
    pub children: Vec<Handle>,
    thread: Option<Thread>,
}
impl Record {
    pub(super) fn main(context: Handle) -> Self {
        Self {
            context,
            flags: 0x10000000,
            parent: None,
            children: vec![],
            thread: None,
        }
    }
}
impl Thread {
    pub fn query_interface<A: ThreadApi>(&self, api: &mut A, iid: u32) -> Option<Handle> {
        if !self.supports(iid) {
            return None;
        }
        self.add_ref();
        api.retain(self.identity);
        Some(self.identity)
    }
    pub fn get_id(&self) -> i32 {
        self.state.lock().unwrap().id.unwrap_or(-1)
    }
    pub fn context(&self) -> Option<Handle> {
        self.state.lock().unwrap().context
    }
    pub fn owner(&self) -> Option<Handle> {
        self.state.lock().unwrap().owner.as_ref().map(|t| t.identity)
    }
    pub fn user_context(&self) -> Option<Handle> {
        self.state.lock().unwrap().user_context
    }
    pub fn references(&self) -> u32 {
        self.state.lock().unwrap().references
    }
    pub fn set_user_context(&mut self, value: Option<Handle>) {
        self.state.lock().unwrap().user_context = value;
    }
    pub fn add_ref(&self) -> u32 {
        let mut state = self.state.lock().unwrap();
        state.references = state.references.wrapping_add(1);
        state.references
    }
    /// Returns zero for deletion even when the incoming count was already zero.
    /// Native storage deletion belongs to the adapter; this releases owned state.
    pub fn release<A: ThreadApi + ?Sized>(&self, api: &mut A) -> u32 {
        let mut state = self.state.lock().unwrap();
        if state.references > 1 {
            state.references -= 1;
            return state.references;
        }
        state.references = 0;
        let owner = state.owner.take();
        drop(state);
        if let Some(owner) = owner {
            api.release(owner.identity);
            owner.release(api);
        }
        0
    }
    pub fn on_create<A: ThreadApi + ?Sized>(
        &mut self,
        api: &mut A,
        owner: &Thread,
        context: Handle,
        id: i32,
    ) -> Result<bool> {
        {
            let mut state = self.state.lock().unwrap();
            state.id = Some(id);
            state.context = Some(context);
        }
        if self.owner() != Some(owner.identity) {
            owner.add_ref();
            api.retain(owner.identity);
            let old = self.state.lock().unwrap().owner.replace(owner.clone());
            if let Some(old) = old {
                api.release(old.identity);
                old.release(api);
            }
        }
        guarded(api, |api| {
            api.invoke(Some(context), Op::PushCClosure(Callback::Alert, 0))?;
            api.invoke(Some(context), Op::PushString(b"_ALERT".to_vec()))?;
            api.invoke(Some(context), Op::Insert(-2))?;
            api.invoke(Some(context), Op::SetTable(GLOBALS))?;
            Ok(())
        })?;
        Ok(true)
    }
    pub fn on_destroy<A: ThreadApi + ?Sized>(&mut self, api: &mut A) -> Result<bool> {
        let owner = {
            let mut state = self.state.lock().unwrap();
            state.context = None;
            state.owner.take()
        };
        if let Some(owner) = owner {
            api.release(owner.identity);
            owner.release(api);
        }
        self.state.lock().unwrap().id = None;
        Ok(true)
    }
    pub fn is_valid<A: LuaApi>(&self, api: &mut A) -> Result<bool> {
        let Some(context) = self.context() else {
            return Ok(false);
        };
        guarded(api, |api| {
            Ok(self.get_id() != -1 && api.invoke(Some(context), Op::GetExtraOwner)?.handle()? == Some(self.identity))
        })
    }
    pub fn pause<A: LuaApi>(&self, api: &mut A, results: i32) -> Result<i32> {
        self.session(api, Reporter::Disabled)?
            .execute(Op::Yield(results))?
            .int()
    }
    pub fn destroy<A: ThreadApi>(&self, host: &mut LuaHost, api: &mut A) -> Result<bool> {
        if self.context().is_none() {
            return Ok(false);
        }
        guarded(api, |api| host.destroy_from(api, self.get_id(), self.context()))
    }
}
impl LuaHost {
    fn thread(&self, id: i32) -> Option<&Thread> {
        let record = self.records.get(&id)?;
        Some(record.thread.as_ref().unwrap_or(&self.main_thread))
    }
    fn thread_mut(&mut self, id: i32) -> Option<&mut Thread> {
        let record = self.records.get_mut(&id)?;
        Some(record.thread.as_mut().unwrap_or(&mut self.main_thread))
    }
    pub fn record(&self, id: i32) -> Option<&Record> {
        self.records.get(&id)
    }
    /// Borrowed core object view, separate from the raw context backpointer.
    pub fn thread_view(&self, id: i32) -> Option<&Thread> {
        self.thread(id)
    }
    /// Borrowed: caller must explicitly retain the identity across destruction.
    pub fn get_thread<A: LuaApi>(&self, api: &mut A, id: i32) -> Result<Option<Handle>> {
        let Some(record) = self.records.get(&id) else {
            return Ok(None);
        };
        guarded(api, |api| api.invoke(Some(record.context), Op::GetExtraOwner)?.handle())
    }
    pub fn status<A: LuaApi>(&self, api: &mut A, thread: &Thread) -> Result<i32> {
        let Some(context) = thread.context() else {
            return Ok(-1);
        };
        let Some(record) = self.records.get(&thread.get_id()).filter(|_| thread.get_id() != -1) else {
            return Ok(-1);
        };
        if record.flags & 0x10000000 != 0 {
            return Ok(2);
        }
        guarded(api, |api| {
            let frame = api.invoke(Some(context), Op::HasActivationFrame)?.boolean()?;
            let stack = api.invoke(Some(context), Op::GetTop)?.int()?;
            Ok(i32::from(frame || stack != 0))
        })
    }
    pub fn create_thread<A: ThreadApi>(
        &mut self,
        api: &mut A,
        flags: u32,
        class: u32,
        output: &mut Option<Handle>,
    ) -> Result<i32> {
        *output = None;
        guarded(api, |api| {
            let source = self.main_thread.context().ok_or(Error::InvalidContext)?;
            self.create_from(api, source, flags, class, output)
        })
    }
    pub fn create_from<A: ThreadApi>(
        &mut self,
        api: &mut A,
        source: Handle,
        flags: u32,
        class: u32,
        output: &mut Option<Handle>,
    ) -> Result<i32> {
        *output = None;
        let context = api
            .invoke(Some(source), Op::NewThread)?
            .handle()?
            .ok_or(Error::BadPrimitiveResult)?;
        api.invoke(Some(source), Op::Move(context, 2))?;
        let Some(mut thread) = api.factory_thread(class, THREAD_IID)? else {
            return Ok(-1);
        };
        let temporary = thread.identity;
        let temporary_view = thread.clone();
        let result = (|| {
            let owner = self.registry_owner.as_ref().ok_or(Error::InvalidContext)?;
            if !api.on_create(&mut thread, owner, context, self.next_id)? {
                return Ok(-1);
            }
            let parent_id = self
                .records
                .keys()
                .copied()
                .find(|id| self.records[id].context == source);
            let parent = if flags & 0x10 != 0 {
                let Some(id) = parent_id else {
                    return Ok(-1);
                };
                let Some(_) = self.get_thread(api, id)? else {
                    return Ok(-1);
                };
                Some(self.records[&id].context)
            } else {
                None
            };
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
            if id == -1 {
                return Ok(-1);
            }
            if let Some(parent_id) = parent_id.filter(|_| parent.is_some()) {
                self.records.get_mut(&parent_id).unwrap().children.push(context);
            }
            api.invoke(Some(context), Op::SetExtraOwner(thread.identity))?;
            api.retain(thread.identity);
            thread.add_ref();
            // Root the coroutine object, which is still the child's top value.
            api.invoke(Some(context), Op::PushLightUserdata(context))?;
            api.invoke(Some(context), Op::Insert(-2))?;
            api.invoke(Some(context), Op::RawSet(REGISTRY))?;
            api.retain(thread.identity);
            thread.add_ref();
            *output = Some(thread.identity);
            self.records.insert(
                id,
                Record {
                    context,
                    flags,
                    parent,
                    children: vec![],
                    thread: Some(thread),
                },
            );
            Ok(id)
        })();
        api.release(temporary);
        temporary_view.release(api);
        result
    }
    pub fn destroy_thread<A: ThreadApi>(&mut self, api: &mut A, id: i32) -> Result<bool> {
        guarded(api, |api| self.destroy_from(api, id, self.main_thread.context()))
    }
    pub fn destroy_from<A: ThreadApi>(&mut self, api: &mut A, id: i32, requester: Option<Handle>) -> Result<bool> {
        let Some(record) = self.records.get(&id) else {
            return Ok(false);
        };
        let context = Some(record.context);
        if record.flags & 0x10 != 0 && requester.is_some() && requester != context && requester != record.parent {
            return Ok(false);
        }
        while let Some(child) = self.records.get(&id).and_then(|r| r.children.last()).copied() {
            let Some(child_id) = self
                .records
                .keys()
                .copied()
                .find(|key| self.records[key].context == child)
            else {
                return Ok(false);
            };
            if !self.destroy_from(api, child_id, context)? {
                return Ok(false);
            }
        }
        let Some(identity) = self.get_thread(api, id)? else {
            return Ok(false);
        };
        let context = context.ok_or(Error::InvalidContext)?;
        if !api.on_destroy(self.thread_mut(id).unwrap())? {
            return Ok(false);
        }
        let parent = self.records[&id].parent;
        if let Some(parent) = parent {
            if let Some(parent_id) = self
                .records
                .keys()
                .copied()
                .find(|key| self.records[key].context == parent)
            {
                let children = &mut self.records.get_mut(&parent_id).unwrap().children;
                if let Some(index) = children.iter().position(|value| *value == context) {
                    children.swap_remove(index);
                }
            }
        }
        api.invoke(Some(context), Op::PushLightUserdata(context))?;
        api.invoke(Some(context), Op::PushNil)?;
        api.invoke(Some(context), Op::RawSet(REGISTRY))?;
        api.release(identity);
        self.thread(id).unwrap().release(api);
        api.invoke(Some(context), Op::ClearExtraOwner)?;
        self.records.remove(&id);
        Ok(true)
    }
    pub fn resume_thread<A: ThreadApi>(
        &mut self,
        api: &mut A,
        id: i32,
        nargs: i32,
        results: Option<&mut i32>,
    ) -> Result<bool> {
        guarded(api, |api| {
            let source = self.main_thread.context().ok_or(Error::InvalidContext)?;
            let initial = api.invoke(Some(source), Op::GetTop)?.int()?;
            let code = self.resume_from(api, source, id, nargs)?;
            if code != 0 && self.reporter != Reporter::Disabled {
                let view = self.get_thread(api, id)?;
                let thread = view.unwrap_or(Handle(0));
                let thread_id = view.and_then(|_| self.thread(id).map(Thread::get_id));
                let text = api
                    .invoke(Some(source), Op::ToString(-1))?
                    .bytes()?
                    .unwrap_or_else(|| b"<unknown description>".to_vec());
                Session {
                    api,
                    state: source,
                    identity: thread,
                    thread_id,
                    reporter: self.reporter,
                }
                .report(code, Some(&text));
            }
            let count = api
                .invoke(Some(source), Op::GetTop)?
                .int()?
                .wrapping_sub(initial.wrapping_sub(nargs));
            if let Some(results) = results {
                *results = count;
            } else {
                api.invoke(Some(source), Op::SetTop(count.wrapping_neg().wrapping_sub(1)))?;
            }
            Ok(code == 0)
        })
    }
    pub fn resume_from<A: ThreadApi>(&mut self, api: &mut A, source: Handle, id: i32, nargs: i32) -> Result<i32> {
        let error = match self.records.get(&id) {
            None => Some(format!("Resuming Lua thread with invalid ID: {id}.")),
            Some(record) if record.flags & 0x10000000 != 0 => Some(format!("Lua thread {id} is already running.")),
            Some(record) if record.flags & 0x100 != 0 && record.parent != Some(source) => {
                Some(format!("Lua thread {id} is only resumable by its parent."))
            }
            _ => None,
        };
        if let Some(error) = error {
            api.invoke(Some(source), Op::PushString(error.into_bytes()))?;
            return Ok(1);
        }
        let context = self.records[&id].context;
        api.invoke(Some(source), Op::Move(context, nargs))?;
        self.records.get_mut(&id).unwrap().flags |= 0x10000000;
        let code = api.invoke(Some(context), Op::Resume(nargs))?.int()?;
        if let Some(record) = self.records.get_mut(&id) {
            record.flags &= !0x10000000;
        }
        let count = api.invoke(Some(context), Op::GetTop)?.int()?;
        api.invoke(Some(context), Op::Move(source, count))?;
        if let Some(record) = self.records.get(&id) {
            if record.flags & 1 == 0 && self.status(api, self.thread(id).unwrap())? == 0 {
                self.destroy_from(api, id, Some(source))?;
            }
        }
        Ok(code)
    }
    pub fn as_thread(&self) -> &Thread {
        &self.main_thread
    }
    pub fn query_interface<A: ThreadApi>(&self, api: &mut A, iid: u32) -> Option<Handle> {
        self.main_thread.query_interface(api, iid)
    }
    pub fn add_ref(&self) -> u32 {
        self.main_thread.add_ref()
    }
    /// Shared Lua5/main-thread count. Self-held references keep an open host alive.
    pub fn release<A: ThreadApi>(&mut self, api: &mut A) -> Result<u32> {
        if self.main_thread.references() > 1 {
            return Ok(self.main_thread.release(api));
        }
        self.destroy(api)?;
        Ok(0)
    }
    pub fn destroy<A: ThreadApi>(&mut self, api: &mut A) -> Result<()> {
        self.close(api)?;
        // §6.2.1: destruction destroys the registry even when a failed custom
        // main hook left context null, making Close skip its shutdown branch.
        self.shutdown_registry(api)?;
        self.main_thread.release(api);
        Ok(())
    }
    pub(super) fn shutdown_registry<A: ThreadApi>(&mut self, api: &mut A) -> Result<()> {
        while let Some(id) = self.records.keys().next().copied() {
            self.destroy_from(api, id, None)?;
        }
        if let Some(owner) = self.registry_owner.take() {
            api.release(owner.identity);
            owner.release(api);
        }
        Ok(())
    }
}
