// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl ScriptServer {
    fn name_equal(&mut self, host: &mut impl Foreign, key: Borrow, left: &[u8], right: &[u8]) -> Result<bool> {
        let a = self.bytes(host, key, Primitive::Lowercase(left.to_vec()))?;
        let b = self.bytes(host, key, Primitive::Lowercase(right.to_vec()))?;
        Ok(a == b)
    }
    fn find_breakpoint(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32) -> Result<Option<usize>> {
        let key = self.record_key(id);
        let Some(r) = self.records.get(&id) else { return Ok(None); };
        let entries = r.breakpoints.clone();
        for (index, entry) in entries.iter().enumerate() {
            if self.name_equal(host, key, &entry.name, name)? && entry.line == line {
                // String helpers must not invalidate the borrowed list. No deferred
                // mutation semantics are supplied for that foreign precondition.
                if self.records[&id].breakpoints != entries { return Err(Failure::InvalidatingCallback); }
                return Ok(Some(index));
            }
        }
        if self.records[&id].breakpoints != entries { return Err(Failure::InvalidatingCallback); }
        Ok(None)
    }
    fn find_or_create_breakpoint(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32) -> Result<(usize, bool)> {
        if let Some(i) = self.find_breakpoint(host, id, name, line)? { return Ok((i, false)); }
        let key = self.record_key(id);
        let stored_id = if let Some(t) = self.records[&id].thread { self.integer(host, key, Primitive::ThreadId(t))? } else { -1 };
        let entries = &mut self.records.get_mut(&id).unwrap().breakpoints;
        let index = entries.len();
        entries.push(Breakpoint { id: stored_id, name: name.to_vec(), line, ..Breakpoint::default() });
        Ok((index, true))
    }
    fn breakpoint_notification(&mut self, host: &mut impl Foreign, id: i32) -> Result<()> {
        self.notify(host, self.record_key(id), BREAKPOINT_CHANGED, id, None)
    }
    pub fn get_breakpoint_exists(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32) -> Result<bool> {
        Ok(self.find_breakpoint(host, id, name, line)?.is_some())
    }
    /// `remove == false` creates/enables; true removes (§6.3.6 slot 12).
    pub fn set_breakpoint_exists(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32, remove: bool) -> Result<bool> {
        if !self.records.contains_key(&id) { return Ok(false); }
        let changed = if remove {
            if let Some(i) = self.find_breakpoint(host, id, name, line)? {
                self.records.get_mut(&id).unwrap().breakpoints.remove(i); true
            } else { false }
        } else {
            let (i, created) = self.find_or_create_breakpoint(host, id, name, line)?;
            self.records.get_mut(&id).unwrap().breakpoints[i].enabled = 1;
            created
        };
        if changed { self.breakpoint_notification(host, id)?; }
        Ok(true)
    }
    pub fn toggle_breakpoint_exists(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32) -> Result<bool> {
        if !self.records.contains_key(&id) { return Ok(false); }
        if let Some(i) = self.find_breakpoint(host, id, name, line)? { self.records.get_mut(&id).unwrap().breakpoints.remove(i); }
        else { self.find_or_create_breakpoint(host, id, name, line)?; }
        self.breakpoint_notification(host, id)?;
        Ok(true)
    }
    pub fn get_breakpoint_enabled(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32) -> Result<bool> {
        Ok(self.find_breakpoint(host, id, name, line)?.is_some_and(|i| self.records[&id].breakpoints[i].enabled != 0))
    }
    pub fn set_breakpoint_enabled(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32, enabled: u8) -> Result<bool> {
        if !self.records.contains_key(&id) { return Ok(false); }
        let (i, created) = self.find_or_create_breakpoint(host, id, name, line)?;
        let entry = &mut self.records.get_mut(&id).unwrap().breakpoints[i];
        let changed = entry.enabled != enabled;
        entry.enabled = enabled;
        if created || changed { self.breakpoint_notification(host, id)?; }
        Ok(true)
    }
    pub fn get_breakpoint_condition(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32, output: &mut Vec<u8>) -> Result<bool> {
        output.clear();
        let Some(i) = self.find_breakpoint(host, id, name, line)? else { return Ok(false); };
        output.clone_from(&self.records[&id].breakpoints[i].condition);
        Ok(true)
    }
    pub fn set_breakpoint_condition(&mut self, host: &mut impl Foreign, id: i32, name: &[u8], line: i32, condition: &[u8]) -> Result<bool> {
        if !self.records.contains_key(&id) { return Ok(false); }
        let (i, created) = self.find_or_create_breakpoint(host, id, name, line)?;
        let entry = &mut self.records.get_mut(&id).unwrap().breakpoints[i];
        let equal = entry.condition == condition;
        // Literal compatibility branch: unequal text is never assigned.
        if equal { entry.condition = condition.to_vec(); }
        if created || equal { self.breakpoint_notification(host, id)?; }
        Ok(true)
    }
    pub fn get_breakpoint_entry_list(&mut self, host: &mut impl Foreign, id: i32, filter: &[u8], output: &mut Vec<Breakpoint>) -> Result<bool> {
        output.clear();
        let Some(r) = self.records.get(&id) else { return Ok(false); };
        let entries = r.breakpoints.clone();
        let key = self.record_key(id);
        for entry in &entries {
            if self.name_equal(host, key, &entry.name, filter)? { output.push(entry.clone()); }
        }
        if self.records[&id].breakpoints != entries { return Err(Failure::InvalidatingCallback); }
        Ok(true)
    }
    pub fn should_break(&mut self, host: &mut impl Foreign, id: i32) -> Result<bool> {
        let Some(r) = self.records.get(&id) else { return Ok(false); };
        if r.breakpoints.is_empty() { return Ok(false); }
        let key = self.record_key(id);
        let thread = self.thread_at(id)?;
        let (name, line) = match self.call(host, key, Primitive::DebugLocation { thread, depth: 0 })? {
            Value::Location { name, line } => (name, line), _ => return Err(Failure::UnexpectedReply)
        };
        // Deliberately independent of enabled, condition and transition counters.
        Ok(self.find_breakpoint(host, id, &name, line)?.is_some())
    }
}
