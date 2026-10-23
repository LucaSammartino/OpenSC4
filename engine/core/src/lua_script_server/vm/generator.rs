// SPDX-License-Identifier: GPL-3.0-or-later
//! Specified part of cGZLua5CommandGenerator (§6.3.2 foreign endpoint identity).
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    CommandServer,
    MainThread,
    ParameterSet,
}
pub trait GeneratorApi: LuaApi {
    fn register_generator(&mut self, server: Handle, generator: Handle) -> bool;
    fn unregister_generator(&mut self, server: Handle, generator: Handle) -> bool;
    /// Borrowed view, just like cIGZLua5::AsIGZLua5Thread.
    fn main_thread(&mut self, lua: Handle) -> Result<Thread>;
    fn add_ref(&mut self, endpoint: Endpoint, object: Handle);
    fn release(&mut self, endpoint: Endpoint, object: Handle);
}

#[derive(Clone, Debug)]
pub struct CommandGenerator {
    pub identity: Handle,
    server: Option<Handle>,
    thread: Option<Thread>,
    name: Vec<u8>,
    parameters: Vec<Handle>,
    working: Vec<Handle>,
}
impl CommandGenerator {
    pub fn new(identity: Handle) -> Self {
        // LH5: no native constructor fields are specified. Empty Rust storage
        // is not a claim about the native constructor's other defaults.
        Self {
            identity,
            server: None,
            thread: None,
            name: Vec::new(),
            parameters: Vec::new(),
            working: Vec::new(),
        }
    }
    pub fn supports(iid: u32) -> bool {
        matches!(iid, 1 | GENERATOR_IID | COMMAND_GENERATOR_IID)
    }
    /// Borrowed sibling identity: this conversion neither acquires nor releases.
    pub fn as_command_generator(&self) -> Handle {
        self.identity
    }
    pub fn name(&self) -> &[u8] {
        &self.name
    }
    pub fn retained_server(&self) -> Option<Handle> {
        self.server
    }
    pub fn retained_thread(&self) -> Option<Handle> {
        self.thread.as_ref().map(|t| t.identity)
    }
    pub fn init<A: GeneratorApi>(
        &mut self,
        api: &mut A,
        server: Option<Handle>,
        lua: Option<Handle>,
        name: &[u8],
    ) -> Result<bool> {
        if name.is_empty() {
            return Ok(false);
        }
        let Some(lua) = lua else {
            return Ok(false);
        };
        let Some(server) = server else {
            return Ok(false);
        };
        // Reinitialization order/defaults are not specified (LH5).
        if self.server.is_some() || self.thread.is_some() {
            return Err(Error::Unsupported("LH5"));
        }
        if !api.register_generator(server, self.identity) {
            return Ok(false);
        }
        let thread = api.main_thread(lua)?;
        let valid = {
            let mut s = thread.session(api, Reporter::Default)?;
            s.get_global(name)?;
            let kind = s.execute(Op::Type(-1))?.int()?;
            match kind {
                5 => {
                    s.pop(1)?;
                    true
                }
                0 => {
                    s.pop(1)?;
                    s.execute(Op::NewTable)?;
                    s.set_global(name)?;
                    true
                }
                _ => {
                    s.pop(1)?;
                    false
                }
            }
        };
        if !valid {
            api.unregister_generator(server, self.identity);
            return Ok(false);
        }
        api.add_ref(Endpoint::MainThread, thread.identity);
        self.thread = Some(thread);
        api.add_ref(Endpoint::CommandServer, server);
        self.server = Some(server);
        self.name = name.to_vec();
        // LH5: "remaining setup" is not described. Do not promise success,
        // generate commands from guesses, or delegate this owner to the old host.
        Err(Error::Unsupported("LH5"))
    }
    pub fn shutdown<A: GeneratorApi>(&mut self, api: &mut A) -> bool {
        let status = self
            .server
            .map(|p| api.unregister_generator(p, self.identity))
            .unwrap_or(true);
        if let Some(server) = self.server.take() {
            api.release(Endpoint::CommandServer, server);
        }
        if let Some(thread) = self.thread.take() {
            api.release(Endpoint::MainThread, thread.identity);
        }
        for parameter in self.parameters.drain(..) {
            api.release(Endpoint::ParameterSet, parameter);
        }
        self.working.clear();
        status
    }
    pub fn generic_init(&mut self) -> Result<bool> {
        Err(Error::Unsupported("LH5"))
    }
    pub fn generate_commands(&mut self) -> Result<()> {
        Err(Error::Unsupported("LH5"))
    }
}
