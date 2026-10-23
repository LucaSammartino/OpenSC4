// SPDX-License-Identifier: GPL-3.0-or-later
use super::{References, Result, Token, Unsupported};
pub const TOOLS: &[(u32,&str)] = &[
    (0xCBADE58E,"PlaceNetwork"),(0x2BADF573,"PlaceNetworkIntersection"),(0x2BB48D26,"PlaceLot"),
    (0xEBB48D31,"PlaceZone"),(0x2BB48D41,"Demolish"),(0x0BB9853C,"GetViewTarget"),(0x8BB756A4,"SetViewTarget")];
pub const MISC: &[(u32,&str)] = &[
    (0xABAC4EE4,"ListCommands"),(0x2BB40833,"SetDebugLevel"),(0x6BBC0DAD,"CreateGZLog"),(0x8BA12D18,"GZLog"),
    (0x4BC4581C,"SetGZLogLevel"),(0x8BBD797F,"Assert"),(0x0BA12D1B,"RZCheckHeap"),(0xABAC067C,"ExecuteScript"),
    (0x4C149310,"GetDateAndTime"),(0xEBAC13FA,"GetTimerTime"),(0x2BAC2385,"GetFrameCount"),(0x0BBEF8A5,"GetFrameRate"),
    (0xABAC26B5,"ExecuteCheat"),(0xCBAC32F4,"WaitForMessage2"),(0xEBAC3957,"CancelWaitForMessage2"),
    (0x0BBC1580,"GetPathDirectory"),(0xEBBC158B,"GetPathFileName"),(0xCBB14BDC,"GetDirectory"),
    (0x8BB14DCB,"GetRandomNumber"),(0x8BB14DD6,"GetRandomInteger"),(0x2BB17574,"TakeSnapshot"),
    (0xCBB17C11,"GameDelay"),(0xCBB17C12,"GamePause"),(0x4BB17C4D,"ReadRegistry"),(0x2BB17C4E,"WriteRegistry"),
    (0x0BB17C64,"ViewWebBrowser"),(0xABB17C74,"SendMessage"),(0x0BB5C013,"GetAppState"),(0x6BB9BBE3,"SetScriptAutoYield"),
    (0x0BC24F90,"CreateException"),(0x0BCE8AA5,"GetPopupModalDialogsSafe"),(0xABCE8AA7,"GetPopupModalDialogsEnabled"),
    (0x6BCE8AE6,"SetPopupModalDialogsEnabled"),(0x6BCECFCE,"GetOccupantCount"),(0xCC363667,"GetMemInfo")];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Misc, Tool }
pub trait Host: References {
    /// Service results are acquired references, retained until Shutdown.
    fn command_service(&mut self) -> Option<Token>;
    fn message_service(&mut self) -> Option<Token>;
    /// Pass the raw helper list; duplicate suppression remains GZF4.
    fn subscribe(&mut self, service: Token, target: Token, ids: &[u32]);
    fn unsubscribe(&mut self, service: Token, target: Token, ids: &[u32]);
    fn register(&mut self, service: Token, id: u32, name: &str, description: Option<&str>);
    fn unregister(&mut self, service: Token, id: u32);
}
pub struct Support {
    kind: Kind,
    refs: crate::commands::RefCount,
    initialized: bool,
    commands: Option<Token>,
    messages: Option<Token>,
    notifications: Vec<u32>,
    pub pending_waits: Vec<Token>,
    pub random_seed_argument: u32,
}
impl Support {
    pub fn new(kind: Kind) -> Self {
        let mut ids: Vec<_> = match kind { Kind::Misc => MISC, Kind::Tool => TOOLS }.iter().map(|e| e.0).collect();
        if kind == Kind::Misc { for id in &mut ids { if *id == 0xABCE8AA7 { *id = 0x0BCE8AA5; } } }
        Self { kind, refs: Default::default(), initialized: false, commands: None, messages: None,
            notifications: ids, pending_waits: Vec::new(), random_seed_argument: 0 }
    }
    pub fn initialized(&self) -> bool { self.initialized }
    pub fn notifications(&self) -> &[u32] { &self.notifications }
    pub fn inventory(&self) -> &'static [(u32,&'static str)] { match self.kind { Kind::Misc => MISC, Kind::Tool => TOOLS } }
    pub fn add_ref(&mut self) -> u32 { self.refs.add_ref() }
    pub fn release(&mut self, host: &mut impl Host, this: Token) -> u32 {
        let n = self.refs.release(); if self.refs.is_destroyed() { self.shutdown(host,this); } n
    }
    pub fn init(&mut self, host: &mut impl Host, this: Token) -> bool {
        if self.initialized { return true } self.initialized = true;
        self.messages = host.message_service(); self.commands = host.command_service();
        if let Some(m) = self.messages { host.subscribe(m,this,&self.notifications); }
        if let Some(c) = self.commands { for &(id,name) in self.inventory() { host.register(c,id,name,None); } } true
    }
    pub fn shutdown(&mut self, host: &mut impl Host, this: Token) -> bool {
        if !self.initialized { return true } self.initialized = false;
        if let Some(c) = self.commands.take() { for &(id,_) in self.inventory() { host.unregister(c,id); } host.release(c); }
        if let Some(m) = self.messages.take() { host.unsubscribe(m,this,&self.notifications); host.release(m); } true
    }
    /// Command type/data1/data2 are passed by the existing command dispatcher.
    /// The command-specific schemas, statuses and pending waits are unsettled.
    pub fn do_message(&mut self, _id: u32, _input: Option<&crate::commands::SetRef>, _output: Option<&crate::commands::SetRef>) -> Result<()> {
        Err(Unsupported(match self.kind { Kind::Misc => "GZF4", Kind::Tool => "GZF5" }))
    }
}
