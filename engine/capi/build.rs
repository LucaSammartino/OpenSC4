// SPDX-License-Identifier: GPL-3.0-or-later
// Generate typed callback thunks from the C ABI declarations. New/changed
// signatures fail the build rather than silently escaping the trace boundary.
use std::{env, fs, path::Path};
#[path = "src/trace/hash.rs"]
mod hash;

fn fields(file: &str, name: &str) -> Vec<(String, String)> {
    let text = fs::read_to_string(format!("src/{file}.rs")).unwrap();
    let body = text
        .split(&format!("pub struct {name} {{"))
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    body.lines()
        .filter_map(|line| {
            let field = line.trim().strip_prefix("pub ")?;
            let (name, ty) = field.split_once(": ").unwrap();
            Some((name.to_owned(), ty.trim_end_matches(',').to_owned()))
        })
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=../core/src");
    println!("cargo:rerun-if-changed=../formats/src");
    println!("cargo:rerun-if-changed=src");
    println!(
        "cargo:rustc-env=FC4_REPLAY_SOURCE_HASH={}",
        hash::source_hash(Path::new("..")).unwrap()
    );
    let mut out =
        String::from("use crate::{budget::*, power::*, pollution::*, hostgrid::*, landvalue::*, props::*};\n");
    for (file, names) in [
        ("budget", "fc4_lot_info fc4_tax_scan fc4_budget_message"),
        (
            "power",
            "fc4_power_post fc4_power_pole fc4_power_line fc4_power_deal fc4_power_budget_item fc4_power_message",
        ),
        ("pollution", "fc4_pollution_budget_item"),
        ("hostgrid", "fc4_cell_rect"),
        ("landvalue", "fc4_grid_geometry"),
        ("power_shadow", "fc4_power_capture_ref fc4_power_shadow_counts"),
        ("pollution_shadow", "fc4_pollution_capture_ref"),
    ] {
        println!("cargo:rerun-if-changed=src/{file}.rs");
        for name in names.split_whitespace() {
            let ty = if file.ends_with("_shadow") {
                format!("crate::{file}::{name}")
            } else {
                name.to_owned()
            };
            let fields = fields(file, name);
            out += &format!("impl Wire for {ty} {{ fn put(&self, b: &mut Vec<u8>) {{");
            for (f, _) in &fields {
                out += &format!("self.{f}.put(b);");
            }
            out += "} fn get(r: &mut Reader<'_>) -> Result<Self, String> { Ok(Self {";
            for (f, _) in &fields {
                out += &format!("{f}: Wire::get(r)?,");
            }
            out += "}) } }\n";
        }
    }
    for (key, file, ty) in [
        ("budget", "budget", "fc4_budget_host"),
        ("power", "power", "fc4_power_host"),
        ("pollution", "pollution", "fc4_pollution_host"),
        ("power_grid", "hostgrid", "fc4_grid_api"),
        ("pollution_grid", "hostgrid", "fc4_grid_api"),
    ] {
        let owner = key.split('_').next().unwrap();
        let slot = key.to_uppercase();
        let fields = fields(file, ty);
        out += &format!("thread_local! {{ static {slot}: RefCell<Option<{ty}>> = const {{ RefCell::new(None) }}; }}\n");
        out += &format!(
            "pub(super) fn wrap_{key}(h: {ty}) -> {ty} {{ {slot}.with(|s| *s.borrow_mut() = Some(h)); let mut w = h;"
        );
        if !key.contains('_') && key != "budget" {
            out += &format!("w.grids = wrap_{key}_grid(h.grids);");
        }
        for (field, sig) in &fields {
            if !sig.starts_with("Option<unsafe extern") {
                continue;
            }
            out += &format!("w.{field} = h.{field}.map(|_| {key}_{field} as _);");
        }
        out += "w }\n";
        out += &format!("pub(super) fn put_{key}(h: &{ty}, b: &mut Vec<u8>) {{");
        if !key.contains('_') && key != "budget" {
            out += &format!("put_{key}_grid(&h.grids,b);");
        }
        for (f, sig) in &fields {
            if sig.starts_with("Option<unsafe extern") {
                out += &format!("h.{f}.is_some().put(b);");
            }
        }
        out += "}\n";
        // All host fields are integers, raw pointers or nullable function pointers.
        out += &format!("pub(super) fn get_{key}(r: &mut Reader<'_>) -> Result<{ty}, String> {{ let mut h: {ty} = unsafe {{ std::mem::zeroed() }}; h.struct_size = std::mem::size_of::<{ty}>() as u32;");
        if !key.contains('_') && key != "budget" {
            out += &format!("h.grids = get_{key}_grid(r)?;");
        }
        for (f, sig) in &fields {
            if sig.starts_with("Option<unsafe extern") {
                out += &format!("if bool::get(r)? {{ h.{f} = Some({key}_{f}); }}");
            }
        }
        out += "Ok(h) }\n";
        for (field, sig) in &fields {
            if !sig.starts_with("Option<unsafe extern") {
                continue;
            }
            let args = sig.split("fn(").nth(1).unwrap().split(')').next().unwrap();
            let args: Vec<_> = args.split(", ").collect();
            assert_eq!(args[0], "U");
            let ret = sig
                .split(" -> ")
                .nth(1)
                .map(|s| s.trim_end_matches('>'))
                .unwrap_or("()");
            let ret = if sig.ends_with("-> *mut fc4_props>") {
                "*mut fc4_props"
            } else {
                ret
            };
            let mut pre = String::new();
            let mut post = String::new();
            let mut apply = String::new();
            let mut callargs: Vec<String> = (1..args.len()).map(|i| format!("a{i}")).collect();
            let mut declarations = String::new();
            let enumeration = args.iter().position(|t| {
                matches!(
                    *t,
                    "fc4_token_callback" | "fc4_lot_visitor" | "fc4_power_deal_callback" | "fc4_bytes_sink"
                )
            });
            if let Some(i) = enumeration {
                assert_eq!(args[i + 1], "*mut c_void");
                let (elem, proxy) = match args[i] {
                    "fc4_token_callback" => ("u64", "tokens"),
                    "fc4_lot_visitor" => ("fc4_lot_info", "lots"),
                    "fc4_power_deal_callback" => ("fc4_power_deal", "deals"),
                    "fc4_bytes_sink" => ("Vec<u8>", "bytes"),
                    _ => unreachable!(),
                };
                declarations += &format!("let mut collected: Vec<{elem}> = Vec::new(); let mut tap = Tap {{ values: &mut collected, callback: a{i}, user: a{} }};", i+1);
                callargs[i - 1] = format!("Some(tap_{proxy})");
                callargs[i] = "&mut tap as *mut _ as *mut c_void".to_owned();
                post += "collected.put(&mut answer);";
                apply += &format!(
                    "let values = Vec::<{elem}>::get(&mut r)?; if replaying(\"{owner}\") {{ for v in values {{"
                );
                let inv = match proxy {
                    "tokens" => format!("f(a{},v)", i + 1),
                    "bytes" => format!("f(a{},v.as_ptr(),v.len())", i + 1),
                    _ => format!("f(a{},&v)", i + 1),
                };
                apply += &format!("if let Some(f) = a{i} {{ {inv}; }} }} }}");
            }
            for (i, arg) in args.iter().enumerate().skip(1) {
                if enumeration.is_some_and(|j| i == j || i == j + 1) {
                    continue;
                }
                let a = format!("a{i}");
                if let Some(t) = arg.strip_prefix("*mut ") {
                    assert_ne!(t, "c_void", "unhandled context {key}.{field}");
                    pre += &format!("(!{a}.is_null()).put(&mut input);");
                    if t == "fc4_props" {
                        post += &format!("put_props({a}, &mut answer);");
                        apply += &format!("set_props({a}, &mut r)?;");
                    } else {
                        let n = if args.get(i + 1) == Some(&"usize") {
                            format!("a{}", i + 1)
                        } else if t == "f32" && (field == "occupant_position" || field == "occupant_bounding_box") {
                            "3".to_owned()
                        } else {
                            "1".to_owned()
                        };
                        post += &format!("put_ptr({a},{n},&mut answer);");
                        apply += &format!("get_ptr({a},{n},&mut r)?;");
                    }
                } else if let Some(t) = arg.strip_prefix("*const ") {
                    let n = if args.get(i + 1) == Some(&"usize") {
                        format!("a{}", i + 1)
                    } else if t == "f32" && field == "add_radiation" {
                        "3".to_owned()
                    } else {
                        "1".to_owned()
                    };
                    if t == "c_void" {
                        // Property write data uses the ABI variant memory type.
                        pre += &format!("put_raw({a},a{}, {n}, &mut input);", i - 2);
                    } else {
                        pre += &format!("put_ptr({a},{n},&mut input);");
                    }
                } else {
                    pre += &format!("{a}.put(&mut input);");
                }
            }
            let (ret_put, ret_get) = match ret {
                "*mut fc4_props" => (
                    "put_props(result,&mut answer);".to_owned(),
                    "new_props(&mut r)?".to_owned(),
                ),
                "*mut c_void" => (
                    "(result as usize as u64).put(&mut answer);".to_owned(),
                    "u64::get(&mut r)? as usize as *mut c_void".to_owned(),
                ),
                _ => (
                    "result.put(&mut answer);".to_owned(),
                    format!("<{ret} as Wire>::get(&mut r)?"),
                ),
            };
            let params = args
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, t)| format!(",a{i}:{t}"))
                .collect::<String>();
            out += &format!("#[allow(unused_mut,clippy::let_unit_value)] unsafe extern \"C\" fn {key}_{field}(_user:*mut c_void{params}) -> {ret} {{ let mut live_result: {ret} = Default::default(); let mut input=Vec::new(); {pre} let answer=exchange(\"{owner}\",\"{key}.{field}\",input,|| {{ let original={slot}.with(|s| *s.borrow()).expect(\"trace host\"); {declarations} let result=(original.{field}.expect(\"present callback\"))(original.user,{}); live_result=result; let mut answer=Vec::new(); {post} {ret_put} answer }}); if !replaying(\"{owner}\") {{ return live_result; }} let decoded=(|| -> Result<{ret},String> {{ let mut r=Reader(&answer); {apply} let result={ret_get}; r.end()?; Ok(result) }})(); decode_callback(\"{owner}\",decoded) }}\n",callargs.join(","));
        }
    }
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("trace_hosts.rs"), out).unwrap();
}
