// SPDX-License-Identifier: GPL-3.0-or-later
//! App-owned city algorithms, application.md §11. Foreign objects are opaque.
use super::{Application, Boundary};
use crate::gz_messages::Token;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CityState {
    pub region_owner: Option<Token>,
    pub city: Option<Token>,
    pub regional_city: Option<Token>,
    pub segment: Option<Token>,
    pub template_segment: Option<Token>,
    pub new_city_specification: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// cSC4AutoGamePause(0, 1, 1, 1).
    Pause,
    /// cRZAutoCursor(15).
    BusyCursor,
    /// cSC4AutoFlushInputMessageQueue(1000).
    FlushInput,
    /// Unnamed §11.16 scoped object constructed with 1 (body open).
    SaveUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Object {
    City,
    Segment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    BlocksSaving(Token),
    AltDown,
    CityPath(Token),
    RegionalPath(Token),
    SegmentInit(Token),
    SegmentOpen(Token, bool),
    SegmentClose(Token),
    SegmentLoadStore(Token),
    AppVersion(Token),
    RootVersion(Token),
    AppRootVersion(Token),
    ReadCity(Token),
    ReadRegionalCity(Token),
    TempPath,
    Exists(Vec<u8>),
    Remove(Vec<u8>),
    Rename(Vec<u8>, Vec<u8>),
    Dialog(u32, u32),
    Confirm(u32, u32),
    SavedCitySuffix,
    Region(Token),
    RegionalValue(Token),
    Zoom,
    Rotation,
    HasSourceView,
    HasDestinationView,
    ObliterationWriteResult(Token),
    TerrainDirectory,
    TerrainModal,
    TerrainSelection,
    TerrainCurrentDirectory,
    RootWidth,
    RootHeight,
    DialogWidth,
    DialogHeight,
    HasRootWindow,
    HasGlobalApp,
}

/// A reply has separate channels so null pointers, empty strings and false
/// remain distinct. The adapter fills the channel appropriate to the query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    pub flag: bool,
    pub number: i32,
    pub token: Option<Token>,
    pub text: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Scope(Scope, bool),
    Pause(bool, [bool; 4]),
    Pump(u8, [u32; 3]),
    CityMessage {
        city: Token,
        kind: u32,
        d1: u32,
        d2: u32,
        post: bool,
    },
    PackageMessage {
        segment: Token,
        kind: u32,
        path: Vec<u8>,
        target: Vec<u8>,
        flag: bool,
    },
    CityInit(Token),
    CityShutdown(Token),
    Release(Token),
    Retain(Token),
    SegmentPath(Token, Vec<u8>),
    SegmentShutdown(Token),
    SegmentClose(Token),
    RegisterSegment(Token),
    UnregisterSegment(Token),
    ResourceIdle,
    DestroyScreen,
    CityScreen,
    RegionScreen,
    QuitService44,
    AttachCity(Token, Token),
    SetCityPath(Token, Vec<u8>),
    CitySlot5c(Token, u32),
    CitySlot3c(Token),
    SetCity3c(Token, u32),
    SetCity18(Token, i32),
    RemoveRegional(Token, Token),
    ReloadRegional(Token, Token),
    DirectedQuit,
    Notify(u32, u32, Vec<u8>),
    WriteAppVersion(Token, u32),
    WriteModuleVersion(Token),
    WriteRoot(Token, u16, Option<Token>, Option<Token>),
    SaveObjectStore(Token),
    WriteObliteration(Token, Token),
    WriteAdjacent(Token),
    Zoom(i32),
    Rotation(i32),
    TerrainIniOpen(Vec<u8>),
    TerrainFlags(u32),
    TerrainSetDirectory(Vec<u8>),
    TerrainCenter(i32, i32),
    TerrainRegister(bool),
    TerrainIniWrite(Vec<u8>),
    TerrainRelease,
    TerrainIniRelease,
    TerrainButton(Token),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionalCity {
    pub token: Token,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Only calls on foreign city/segment/region/window/OS objects occur here.
/// App algorithms (dialogs, branching, ordering, replacement and cleanup) live
/// below. create/read return one owned reference; all other tokens are borrowed.
pub trait CityHost: super::save_path::SavePathHost {
    fn query(&mut self, query: Query) -> Reply;
    fn effect(&mut self, effect: Effect);
    fn create(&mut self, object: Object) -> Token;
    fn regional_geometry(&mut self, city: Token) -> RegionalCity;
    fn all_cities(&mut self, region: Token) -> Vec<RegionalCity>;
    fn city_at(&mut self, region: Token, x: i32, y: i32) -> Token;
    fn adjacent_cities(&mut self, city: Token) -> Vec<Token>;
}

// Effects addressing an absent resource manager/view/window/message server
// are no-ops in the foreign adapter. DirectedQuit means enqueue message
// 0x29A89B05 to the app's v2 target (not broadcast and not synchronous), then
// release the locally created message. RegisterSegment retains independently;
// UnregisterSegment releases that registration, not the app's local reference.

fn scoped<R>(host: &mut impl CityHost, scopes: &[Scope], f: impl FnOnce(&mut dyn CityHost) -> R) -> R {
    for scope in scopes {
        host.effect(Effect::Scope(*scope, true));
    }
    let result = f(host);
    for scope in scopes.iter().rev() {
        host.effect(Effect::Scope(*scope, false));
    }
    result
}

fn pump(host: &mut dyn CityHost) {
    host.effect(Effect::Pump(1, [100, 500, 5000]));
    host.effect(Effect::Pump(2, [100, 500, 5000]));
}

/// Inclusive overlap includes shared edges and corners (§11.4).
pub fn touches(a: RegionalCity, b: RegionalCity) -> bool {
    let axis = |p: i32, size: i32, q: i32, other: i32| {
        i64::from(q) >= i64::from(p) - i64::from(other) && i64::from(q) <= i64::from(p) + i64::from(size)
    };
    axis(a.x, a.width, b.x, b.width) && axis(a.y, a.height, b.y, b.height)
}

impl Application {
    fn set_regional_city(&mut self, regional: Option<Token>, host: &mut dyn CityHost) {
        if let Some(regional) = regional {
            host.effect(Effect::Retain(regional));
        }
        if let Some(old) = self.city_state.regional_city.take() {
            host.effect(Effect::Release(old));
        }
        self.city_state.regional_city = regional;
    }
    fn city_message(&self, host: &mut dyn CityHost, city: Token, kind: u32, d1: u32, post: bool) {
        if self.message_server2_available {
            host.effect(Effect::CityMessage {
                city,
                kind,
                d1,
                d2: 0,
                post,
            });
        }
    }

    fn city_init(&self, host: &mut dyn CityHost, city: Token, new: bool) {
        self.city_message(host, city, 0x26D3_1EC0, u32::from(new), false);
        host.effect(Effect::CityInit(city));
        for (kind, d1) in [
            (0x2B96_B3EA, 0),
            (0x26D3_1EC1, u32::from(new)),
            (0xAB96_B05F, 0),
            (0xEA8A_E29A, 0),
        ] {
            self.city_message(host, city, kind, d1, true);
        }
        pump(host);
        host.effect(Effect::ResourceIdle);
    }

    pub fn new_city(&mut self, host: &mut impl CityHost) -> bool {
        self.new_city_inner(host)
    }

    fn new_city_inner(&mut self, host: &mut dyn CityHost) -> bool {
        if self.city_state.city.is_some() {
            return false;
        }
        host.effect(Effect::Scope(Scope::BusyCursor, true));
        host.effect(Effect::Scope(Scope::FlushInput, true));
        host.effect(Effect::DestroyScreen);
        host.effect(Effect::CityScreen);
        let city = host.create(Object::City);
        self.city_state.city = Some(city);
        if let Some(regional) = self.city_state.regional_city {
            host.effect(Effect::AttachCity(regional, city));
        }
        self.city_init(host, city, true);
        host.effect(Effect::Scope(Scope::FlushInput, false));
        host.effect(Effect::Scope(Scope::BusyCursor, false));
        true
    }

    pub fn unregister_city_segment(&mut self, host: &mut dyn CityHost) {
        if let Some(segment) = self.city_state.segment.take() {
            host.effect(Effect::UnregisterSegment(segment));
            host.effect(Effect::SegmentShutdown(segment));
            host.effect(Effect::SegmentClose(segment));
            host.effect(Effect::Release(segment));
        }
    }

    pub fn register_city_segment(&mut self, path: &[u8], host: &mut dyn CityHost) -> bool {
        let segment = host.create(Object::Segment);
        let init = host.query(Query::SegmentInit(segment)).flag;
        host.effect(Effect::SegmentPath(segment, path.to_vec()));
        if init && host.query(Query::SegmentOpen(segment, false)).flag {
            self.city_state.segment = Some(segment);
            host.effect(Effect::RegisterSegment(segment));
            true
        } else {
            host.effect(Effect::SegmentShutdown(segment));
            host.effect(Effect::Release(segment));
            false
        }
    }

    pub fn partial_close_city(&mut self, host: &mut dyn CityHost) -> bool {
        let found = if let Some(city) = self.city_state.city {
            host.effect(Effect::Pause(true, [false, true, true, true]));
            pump(host);
            self.city_message(host, city, 0x26D3_1EC2, 0, false);
            host.effect(Effect::CityShutdown(city));
            self.city_message(host, city, 0x26D3_1EC3, 0, true);
            host.effect(Effect::Release(city));
            self.city_state.city = None;
            self.unregister_city_segment(host);
            true
        } else {
            false
        };
        host.effect(Effect::ResourceIdle);
        found
    }

    pub fn close_city(&mut self, host: &mut dyn CityHost) -> bool {
        if !self.partial_close_city(host) {
            return false;
        }
        if let Some(regional) = self.city_state.regional_city {
            if let Some(region) = self
                .city_state
                .region_owner
                .and_then(|owner| host.query(Query::Region(owner)).token)
            {
                if host.query(Query::RegionalPath(regional)).text.is_empty() {
                    host.effect(Effect::RemoveRegional(region, regional));
                } else {
                    let closed = host.regional_geometry(regional);
                    for other in host.all_cities(region) {
                        if touches(closed, other) {
                            let reload = if other.token == regional {
                                regional
                            } else {
                                host.city_at(region, other.x, other.y)
                            };
                            host.effect(Effect::ReloadRegional(region, reload));
                        }
                    }
                }
            }
        }
        host.effect(Effect::ResourceIdle);
        true
    }

    pub fn request_new_city(&mut self, regional: Option<Token>, host: &mut impl CityHost) -> Result<bool, Boundary> {
        if self.city_state.city.is_some() {
            self.request_close_city(true, host)?;
        }
        if self.city_state.city.is_some() {
            return Ok(false);
        }
        self.set_regional_city(regional, host);
        Ok(self.new_city_inner(host))
    }

    pub fn request_close_city(&mut self, ask: bool, host: &mut dyn CityHost) -> Result<bool, Boundary> {
        if self.city_state.city.is_none() {
            return Ok(false);
        }
        if !ask {
            return Ok(self.close_city(host));
        }
        host.effect(Effect::Scope(Scope::Pause, true));
        let result = (|| {
            // Literal two-way Yes/No confirmation; no third outcome is published.
            if host.query(Query::Confirm(0x2A56_CDA8, 0x2A56_CD9B)).flag {
                let alt = host.query(Query::AltDown).flag;
                if !self.request_save_city(true, alt, host)? {
                    return Ok(false);
                }
            }
            Ok(self.close_city(host))
        })();
        host.effect(Effect::Scope(Scope::Pause, false));
        result
    }

    fn blocks_saving(&self, host: &mut dyn CityHost) -> bool {
        self.city_state
            .city
            .is_some_and(|city| host.query(Query::BlocksSaving(city)).flag)
    }

    pub fn request_go_to_region_view(&mut self, ask: bool, host: &mut impl CityHost) -> Result<bool, Boundary> {
        scoped(host, if ask { &[Scope::Pause] } else { &[] }, |host| {
            if ask {
                if self.blocks_saving(host) {
                    if host.query(Query::Dialog(0x4A89_B3F2, 0x2A96_ED21)).number as u32 != 0xEA57_DA59 {
                        return Ok(false);
                    }
                } else {
                    match host.query(Query::Dialog(0x6A55_3AA4, 0xAA92_1F4F)).number as u32 {
                        0x8A92_1F5B => {
                            pump(host);
                            let alt = host.query(Query::AltDown).flag;
                            self.request_save_city(false, alt, host)?;
                        }
                        0x8A92_1F65 => {}
                        _ => return Ok(false),
                    }
                }
            }
            host.effect(Effect::QuitService44);
            host.effect(Effect::QuitService44);
            self.close_city(host);
            if self.city_state.city.is_none() {
                host.effect(Effect::DestroyScreen);
                host.effect(Effect::RegionScreen);
            }
            Ok(true)
        })
    }

    pub fn request_quit(&mut self, ask: bool, save: bool, host: &mut impl CityHost) -> Result<bool, Boundary> {
        scoped(host, &[Scope::Pause], |host| {
            let save = if self.blocks_saving(host) {
                if ask && host.query(Query::Dialog(0x4A89_B3F2, 0x2A96_ED21)).number as u32 != 0xEA57_DA59 {
                    return Ok(false);
                }
                false
            } else if ask {
                match host.query(Query::Dialog(0x0A55_161D, 0xAA92_1F4F)).number as u32 {
                    0x8A92_1F5B => true,
                    0x8A92_1F65 => false,
                    _ => return Ok(false),
                }
            } else {
                save
            };
            if save {
                pump(host);
                let alt = host.query(Query::AltDown).flag;
                self.request_save_city(false, alt, host)?;
            }
            if self.message_server2_available {
                host.effect(Effect::DirectedQuit);
            }
            Ok(true)
        })
    }

    pub fn request_quit_from_region(&mut self, ask: bool, host: &mut impl CityHost) -> bool {
        scoped(host, if ask { &[Scope::Pause] } else { &[] }, |host| {
            if ask && host.query(Query::Dialog(0x4A55_1B4C, 0xAA92_1F4F)).number as u32 == 0x8A92_1F70 {
                return false;
            }
            if self.message_server2_available {
                host.effect(Effect::DirectedQuit);
            }
            true
        })
    }

    pub fn request_save_city(&mut self, notify: bool, flag: bool, host: &mut dyn CityHost) -> Result<bool, Boundary> {
        host.effect(Effect::Scope(Scope::Pause, true));
        let result = (|| {
            let Some(city) = self.city_state.city else {
                return Ok(false);
            };
            if self.blocks_saving(host) {
                host.effect(Effect::Notify(0xAA66_6AFD, 0xCA66_6B15, Vec::new()));
                return Ok(false);
            }
            let path = host.query(Query::CityPath(city)).text;
            if path.is_empty() {
                return Ok(false);
            }
            let result = self.save_city(&path, flag, host)?;
            let path = host.query(Query::CityPath(city)).text;
            let name = path.rsplit(|b| matches!(b, b'/' | b'\\')).next().unwrap_or_default();
            if !result || notify {
                let suffix = [b" (".as_slice(), name, b")"].concat();
                host.effect(Effect::Notify(
                    0x2A56_CDA8,
                    if result { 0x2A56_CE36 } else { 0x4A56_CE71 },
                    suffix,
                ));
            }
            Ok(result)
        })();
        host.effect(Effect::Scope(Scope::Pause, false));
        result
    }

    pub fn save_city_current(&mut self, flag: bool, host: &mut impl CityHost) -> Result<bool, Boundary> {
        let Some(city) = self.city_state.city else {
            return Ok(false);
        };
        let path = host.query(Query::CityPath(city)).text;
        if path.is_empty() {
            return Ok(false);
        }
        self.save_city(&path, flag, host)
    }

    /// Target naming is app-owned; region interfaces and code-page conversion
    /// remain primitive foreign operations.
    pub fn save_city(&mut self, path: &[u8], flag: bool, host: &mut dyn CityHost) -> Result<bool, Boundary> {
        let scopes = [Scope::BusyCursor, Scope::FlushInput, Scope::Pause, Scope::SaveUnknown];
        for scope in scopes {
            host.effect(Effect::Scope(scope, true));
        }
        let result = (|| {
            let Some(city) = self.city_state.city else {
                return Ok(false);
            };
            let base = host
                .save_directory(city)
                .ok_or(Boundary::InvalidNativeInput("save requires a selected region"))?;
            let name = host.city_name(city);
            let id = host.city_id(city);
            let target = super::save_path::create_save_path(&base, &name, id, host);
            let old = host.query(Query::CityPath(city)).text;
            host.effect(Effect::SetCityPath(city, target.clone()));
            let temp = host.query(Query::TempPath).text;
            let mut result = false;
            if !temp.is_empty() {
                let segment = host.create(Object::Segment);
                if host.query(Query::SegmentInit(segment)).flag {
                    host.effect(Effect::SegmentPath(segment, temp.clone()));
                    if host.query(Query::SegmentOpen(segment, true)).flag {
                        for kind in [0x26C6_3343, 0x26C6_3346]
                            .into_iter()
                            .filter(|_| self.message_server2_available)
                        {
                            host.effect(Effect::PackageMessage {
                                segment,
                                kind,
                                path: path.to_vec(),
                                target: target.clone(),
                                flag,
                            });
                        }
                        host.effect(Effect::WriteAppVersion(segment, 65));
                        host.effect(Effect::WriteModuleVersion(segment));
                        self.write_city_root(segment, host);
                        for kind in [0x26C6_3344, 0x26C6_3345]
                            .into_iter()
                            .filter(|_| self.message_server2_available)
                        {
                            host.effect(Effect::PackageMessage {
                                segment,
                                kind,
                                path: path.to_vec(),
                                target: target.clone(),
                                flag,
                            });
                        }
                        host.effect(Effect::SaveObjectStore(segment));
                        result = host.query(Query::SegmentClose(segment)).flag;
                    }
                }
                host.effect(Effect::SegmentShutdown(segment));
                host.effect(Effect::Release(segment));
            }
            self.unregister_city_segment(host);
            if result {
                host.effect(Effect::CitySlot3c(city));
                result = replace_file(host, &temp, &target);
                // The worked failure example explicitly preserves the old file.
                if result && target != path && host.query(Query::Exists(path.to_vec())).flag {
                    host.query(Query::Remove(path.to_vec()));
                }
            }
            if !temp.is_empty() {
                host.query(Query::Remove(temp));
            }
            let final_path = if result { target } else { old };
            host.effect(Effect::SetCityPath(city, final_path.clone()));
            self.register_city_segment(&final_path, host);
            if result {
                let current = self
                    .city_state
                    .regional_city
                    .map(|r| host.query(Query::RegionalPath(r)).text);
                for adjacent in host.adjacent_cities(city) {
                    if Some(host.query(Query::RegionalPath(adjacent)).text) != current {
                        host.effect(Effect::WriteAdjacent(adjacent));
                    }
                }
            }
            Ok(result)
        })();
        for scope in scopes.into_iter().rev() {
            host.effect(Effect::Scope(scope, false));
        }
        result
    }

    pub fn write_city_root(&self, segment: Token, host: &mut dyn CityHost) {
        host.effect(Effect::WriteRoot(
            segment,
            2,
            self.city_state.city,
            self.city_state.regional_city,
        ));
    }

    /// Read is independent of an initialized city. Each deserialized object has
    /// one owned reference; a failed regional read leaves the half-read city for
    /// LoadCity's cleanup, as §11.9 specifies.
    pub fn read_city_root(&mut self, segment: Token, host: &mut dyn CityHost) -> bool {
        if host.query(Query::AppRootVersion(segment)).number != 2 {
            return false;
        }
        let Some(city) = host.query(Query::ReadCity(segment)).token else {
            return false;
        };
        self.city_state.city = Some(city);
        let Some(regional) = host.query(Query::ReadRegionalCity(segment)).token else {
            return false;
        };
        if let Some(old) = self.city_state.regional_city.replace(regional) {
            host.effect(Effect::Release(old));
        }
        true
    }

    pub fn load_city(&mut self, path: &[u8], regional: Option<Token>, host: &mut impl CityHost) -> bool {
        if self.city_state.city.is_some() {
            self.close_city(host);
        }
        self.set_regional_city(regional, host);
        scoped(host, &[Scope::BusyCursor, Scope::FlushInput], |host| {
            let segment = host.create(Object::Segment);
            let init = host.query(Query::SegmentInit(segment)).flag;
            host.effect(Effect::SegmentPath(segment, path.to_vec()));
            let mut success = false;
            if init && host.query(Query::SegmentOpen(segment, false)).flag {
                let version = host.query(Query::AppVersion(segment)).number;
                if (62..=65).contains(&version) {
                    if self.message_server2_available {
                        host.effect(Effect::PackageMessage {
                            segment,
                            kind: 0x26C6_3340,
                            path: path.to_vec(),
                            target: Vec::new(),
                            flag: false,
                        });
                    }
                    host.effect(Effect::Retain(segment));
                    self.city_state.segment = Some(segment);
                    host.effect(Effect::RegisterSegment(segment));
                    if host.query(Query::SegmentLoadStore(segment)).flag
                        && host.query(Query::RootVersion(segment)).number == 1
                        && self.read_city_root(segment, host)
                    {
                        let city = self.city_state.city.expect("read_city_root established city");
                        if let Some(regional) = self.city_state.regional_city {
                            let value = host.query(Query::RegionalValue(regional)).number;
                            host.effect(Effect::SetCity18(city, value));
                        }
                        host.effect(Effect::SetCityPath(city, path.to_vec()));
                        host.effect(Effect::SetCity3c(city, 0));
                        host.effect(Effect::DestroyScreen);
                        host.effect(Effect::CityScreen);
                        if self.message_server2_available {
                            for kind in [0x26C6_3341, 0x26C6_3342] {
                                host.effect(Effect::PackageMessage {
                                    segment,
                                    kind,
                                    path: path.to_vec(),
                                    target: Vec::new(),
                                    flag: false,
                                });
                            }
                        }
                        self.city_init(host, city, false);
                        success = true;
                    }
                    if !success {
                        if let Some(city) = self.city_state.city.take() {
                            host.effect(Effect::Release(city));
                        }
                        self.unregister_city_segment(host);
                    }
                }
            }
            host.effect(Effect::Release(segment));
            pump(host);
            host.effect(Effect::ResourceIdle);
            success
        })
    }

    pub fn request_load_city(&mut self, host: &mut impl CityHost) -> Result<bool, Boundary> {
        if self.city_state.city.is_some() {
            self.request_close_city(true, host)?;
        }
        if self.city_state.city.is_some() {
            return Ok(false);
        }
        let mut path = b"SavedCity".to_vec();
        path.extend(host.query(Query::SavedCitySuffix).text);
        let result = self.load_city(&path, None, host);
        if !result {
            host.effect(Effect::Notify(0xCA56_CE8C, 0x4A56_CEA6, Vec::new()));
        }
        Ok(result)
    }

    pub fn do_city_obliteration(&mut self, host: &mut impl CityHost) -> Result<bool, Boundary> {
        scoped(host, &[Scope::BusyCursor], |host| {
            let temp = host.query(Query::TempPath).text;
            if temp.is_empty() {
                return Err(Boundary::InvalidNativeInput(
                    "obliteration temp-name failure has indeterminate return byte",
                ));
            }
            let Some(city) = self.city_state.city else {
                return Err(Boundary::InvalidNativeInput("obliteration requires an open city"));
            };
            let output = host.create(Object::Segment);
            let init = host.query(Query::SegmentInit(output)).flag;
            if init {
                host.effect(Effect::SegmentPath(output, temp.clone()));
                if host.query(Query::SegmentOpen(output, true)).flag {
                    host.effect(Effect::WriteObliteration(city, output));
                    host.query(Query::ObliterationWriteResult(city));
                    host.effect(Effect::SegmentClose(output));
                }
                host.effect(Effect::SegmentShutdown(output));
            }
            host.effect(Effect::Release(output));
            let (zoom, rotation) = if host.query(Query::HasSourceView).flag {
                (host.query(Query::Zoom).number, host.query(Query::Rotation).number)
            } else {
                (0, 0)
            };
            let path = host.query(Query::CityPath(city)).text;
            host.effect(Effect::CitySlot5c(city, 0));
            self.partial_close_city(host);
            if !path.is_empty() {
                host.query(Query::Remove(path.clone()));
            }
            let segment = host.create(Object::Segment);
            host.query(Query::SegmentInit(segment));
            host.effect(Effect::SegmentPath(segment, temp.clone()));
            let mut writeback = Ok(false);
            if host.query(Query::SegmentOpen(segment, false)).flag {
                self.city_state.template_segment = Some(segment);
                self.new_city_inner(host);
                if let Some(city) = self.city_state.city {
                    host.effect(Effect::CitySlot5c(city, 0));
                    host.effect(Effect::SetCityPath(city, path.clone()));
                }
                self.city_state.template_segment = None;
                host.effect(Effect::SegmentClose(segment));
                if self.city_state.city.is_some() {
                    writeback = self.save_city(&path, false, host);
                }
            }
            if host.query(Query::HasDestinationView).flag {
                host.effect(Effect::Zoom(zoom));
                host.effect(Effect::Rotation(rotation));
            }
            host.effect(Effect::SegmentShutdown(segment));
            host.effect(Effect::Release(segment));
            host.query(Query::Remove(temp));
            // SaveCityForObliteration returns true regardless of write outcome.
            writeback.map(|_| true)
        })
    }

    pub fn load_height_field(&mut self, out: &mut Vec<u8>, host: &mut impl CityHost) -> bool {
        let ini = if host.query(Query::HasGlobalApp).flag {
            match self.app_interface_getter(29, None, host) {
                Ok(super::compatibility::AppValue::String(path)) => path,
                _ => unreachable!("INI path getter returns a string"),
            }
        } else {
            Vec::new()
        };
        host.effect(Effect::TerrainIniOpen(ini));
        let directory = host.query(Query::TerrainDirectory).text;
        host.effect(Effect::TerrainFlags(0x10000));
        host.effect(Effect::TerrainFlags(0x100));
        host.effect(Effect::TerrainSetDirectory(directory));
        if host.query(Query::HasRootWindow).flag {
            let x = (host.query(Query::RootWidth).number - host.query(Query::DialogWidth).number) / 2;
            let y = (host.query(Query::RootHeight).number - host.query(Query::DialogHeight).number) / 2;
            host.effect(Effect::TerrainCenter(x, y));
        }
        host.effect(Effect::TerrainRegister(true));
        let mut result = false;
        if host.query(Query::TerrainModal).number != 0 {
            let directory = host.query(Query::TerrainCurrentDirectory).text;
            host.effect(Effect::TerrainIniWrite(directory));
            let selection = host.query(Query::TerrainSelection);
            if selection.flag {
                *out = selection.text;
                result = true;
            }
        }
        host.effect(Effect::TerrainRegister(false));
        host.effect(Effect::TerrainRelease);
        host.effect(Effect::TerrainIniRelease);
        result
    }

    pub fn terrain_message(
        &mut self,
        window: Token,
        source_id: u32,
        words: [u32; 3],
        host: &mut impl CityHost,
    ) -> bool {
        if source_id != 0x27DF_05BE || words[..2] != [3, 0x2872_59F6] {
            return false;
        }
        match words[2] {
            0x20 => host.effect(Effect::TerrainButton(window)),
            0x30 => {
                let mut path = Vec::new();
                if self.load_height_field(&mut path, host) {
                    path.truncate(511);
                    self.city_state.new_city_specification = path;
                }
            }
            _ => {}
        }
        true
    }
}

fn replace_file(host: &mut dyn CityHost, temp: &[u8], target: &[u8]) -> bool {
    let backup = [target, b"_"].concat();
    let existed = host.query(Query::Exists(target.to_vec())).flag;
    if existed {
        if host.query(Query::Exists(backup.clone())).flag && !host.query(Query::Remove(backup.clone())).flag {
            return false;
        }
        if !host.query(Query::Rename(target.to_vec(), backup.clone())).flag {
            return false;
        }
    }
    if host.query(Query::Rename(temp.to_vec(), target.to_vec())).flag {
        if existed {
            host.query(Query::Remove(backup));
        }
        true
    } else {
        host.query(Query::Remove(temp.to_vec()));
        if existed {
            host.query(Query::Rename(backup, target.to_vec()));
        }
        false
    }
}
