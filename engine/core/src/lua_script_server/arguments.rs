// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl ScriptServer {
    pub(super) fn stack(&mut self, host: &mut impl Foreign, borrow: Borrow, thread: Handle, op: StackOp) -> Result<()> {
        self.unit(host, borrow, Primitive::Stack { thread, op })
    }

    pub fn setup_global_arguments(&mut self, host: &mut impl Foreign) -> Result<bool> {
        let Some(lua) = self.lua else {
            return Ok(false);
        };
        let thread = self
            .handle(host, None, Primitive::MainThread(lua))?
            .ok_or(Failure::MissingThread)?;
        let top = self.integer(host, None, Primitive::GetTop(thread))?;
        for op in [
            StackOp::PushString(b"arg".to_vec()),
            StackOp::NewUserdata(1),
            StackOp::PushValue(-1),
            StackOp::Insert(-3),
            StackOp::RawSet(GLOBALS),
            StackOp::NewTable,
            StackOp::PushArgIndexCallback { upvalues: 0 },
            StackOp::SetField {
                index: -2,
                name: "__index",
            },
            StackOp::SetMetatable(-2),
            StackOp::SetTop(top),
        ] {
            self.stack(host, None, thread, op)?;
        }
        Ok(true)
    }

    pub(super) fn setup_thread_arguments(&mut self, host: &mut impl Foreign, record: Option<&Record>) -> Result<bool> {
        let Some(record) = record else {
            return Ok(false);
        };
        let Some(thread) = record.thread else {
            return Ok(false);
        };
        self.stack(host, None, thread, StackOp::PushLightUserdata(thread))?;
        self.stack(host, None, thread, StackOp::NewTable)?;
        self.table_value(
            host,
            thread,
            StackOp::PushNumber(0.0),
            StackOp::PushString(record.name.clone()),
        )?;
        let tokens = if record.arguments.is_empty() {
            Vec::new()
        } else {
            match self.call(host, None, Primitive::ParseCommandLine(record.arguments.clone()))? {
                Value::Tokens(tokens) => tokens,
                _ => return Err(Failure::UnexpectedReply),
            }
        };
        let mut next: i32 = 1;
        for token in tokens {
            let stripped = token.iter().position(|c| *c != b'-' && *c != b'/').map(|n| &token[n..]);
            let Some(stripped) = stripped else {
                continue;
            };
            let (name, value) = if let Some(colon) = stripped.iter().position(|c| *c == b':') {
                (&stripped[..colon], StackOp::PushString(stripped[colon + 1..].to_vec()))
            } else {
                (stripped, StackOp::PushBoolean(true))
            };
            self.table_value(host, thread, StackOp::PushString(name.to_vec()), value)?;
            self.table_value(
                host,
                thread,
                StackOp::PushNumber(f64::from(next)),
                StackOp::PushString(token),
            )?;
            next = next.wrapping_add(1);
        }
        self.table_value(
            host,
            thread,
            StackOp::PushString(b"count".to_vec()),
            StackOp::PushNumber(f64::from(next)),
        )?;
        self.stack(host, None, thread, StackOp::RawSet(REGISTRY))?;
        Ok(true)
    }

    fn table_value(&mut self, host: &mut impl Foreign, thread: Handle, key: StackOp, value: StackOp) -> Result<()> {
        self.stack(host, None, thread, key)?;
        self.stack(host, None, thread, value)?;
        self.stack(host, None, thread, StackOp::RawSet(-3))
    }

    /// Return count for Lua's C callback protocol; temporary stack values are left
    /// below the result, to be discarded by that foreign protocol.
    pub fn argument_index(&mut self, host: &mut impl Foreign, state: Handle) -> Result<i32> {
        let Some(thread) = self.handle(host, None, Primitive::WrapperFromState(state))? else {
            return Ok(0);
        };
        if self.integer(host, None, Primitive::GetTop(thread))? <= 0 {
            return Ok(0);
        }
        self.stack(host, None, thread, StackOp::PushLightUserdata(thread))?;
        self.stack(host, None, thread, StackOp::RawGet(REGISTRY))?;
        if self.integer(host, None, Primitive::Type { thread, index: -1 })? == 5 {
            self.stack(host, None, thread, StackOp::PushValue(-2))?;
            self.stack(host, None, thread, StackOp::RawGet(-2))?;
        } else {
            self.stack(host, None, thread, StackOp::PushNil)?;
        }
        Ok(1)
    }

    pub fn line_hook(&mut self, host: &mut impl Foreign, state: Handle) -> Result<bool> {
        let Some(thread) = self.handle(host, None, Primitive::WrapperFromState(state))? else {
            return Ok(false);
        };
        let token = match self.call(host, None, Primitive::UserContext(thread))? {
            Value::Record(Some(token)) => token,
            Value::Record(None) => return Ok(false),
            _ => return Err(Failure::UnexpectedReply),
        };
        let id = self
            .records
            .iter()
            .find_map(|(&id, r)| (r.token == token).then_some(id))
            .ok_or(Failure::InvalidatingCallback)?;
        let key = Some((id, token));
        match self.call(host, key, Primitive::RawYieldField(state))? {
            Value::U16(0) => {}
            Value::U16(_) => return Ok(false),
            _ => return Err(Failure::UnexpectedReply),
        }
        if !self.records[&id].auto_yield {
            return Ok(false);
        }
        let line = self.integer(host, key, Primitive::DebugLine { state, options: "l" })?;
        if self.records[&id].last_line == line {
            return Ok(false);
        }
        self.records.get_mut(&id).unwrap().last_line = line;
        self.unit(host, key, Primitive::Yield { state, results: 0 })?;
        Ok(true)
    }
}
