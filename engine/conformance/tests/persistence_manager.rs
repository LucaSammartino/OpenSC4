// SPDX-License-Identifier: GPL-3.0-or-later
//! Executes the independent public specification inputs, not original-owner
//! outputs. JSON fixtures carry setup, operations and independently stated
//! expected returns, pointers, reference counts and callback traces.
#[path = "persistence_manager/support.rs"]
mod support;
use fc4_conformance::json::Json;
use fc4_core::persistence_manager::*;
use std::collections::BTreeMap;
use support::*;

fn cache(j: &Json, names: &BTreeMap<String, CacheId>) -> CacheId {
    if let Some(name) = j.as_str() {
        names[name]
    } else {
        CacheId::Foreign(Object(u64::from(number(j))))
    }
}
fn run(id: &str) {
    let fixture = Json::parse(include_str!("../vectors/persistence_manager/independent.json")).unwrap();
    let j = array(&fixture, "cases")
        .iter()
        .find(|j| j.get("id").and_then(Json::as_str) == Some(id))
        .unwrap();
    let mut m = PersistManager::new(Host::from_case(j));
    let mut names = BTreeMap::from([("default".into(), m.default_cache())]);
    for (index, s) in array(j, "steps").iter().enumerate() {
        let op = s.get("op").unwrap().as_str().unwrap();
        let mut result = None;
        let mut output = Some(Object(999));
        let k = s.get("key").map(key).unwrap_or_default();
        let iid = s.get("iid").map_or(RESOURCE_IID, number);
        let args = if let Some(a) = s.get("args") {
            let a = a.as_array().unwrap();
            [number(&a[0]), number(&a[1])]
        } else {
            [0, 0]
        };
        let r = || object(s, "resource");
        let seg = || object(s, "segment");
        match op {
            "remove_default" => {
                let d = m.default_cache();
                result = Some(m.remove_cache(d));
            }
            "new_cache" => {
                let id = m.objects.create();
                names.insert(s.get("name").unwrap().as_str().unwrap().into(), id);
            }
            "add_cache_ref" => {
                m.objects.add_ref(cache(s.get("cache").unwrap(), &names));
            }
            "register_cache" => result = Some(m.register_cache(cache(s.get("cache").unwrap(), &names))),
            "remove_cache" => result = Some(m.remove_cache(cache(s.get("cache").unwrap(), &names))),
            "register_factory" => {
                result = Some(m.register_factory(n(s, "id"), n(s, "type"), Some(object(s, "factory"))))
            }
            "com_factory" => {
                m.objects.host.com = s.get("factory").map(|_| object(s, "factory"));
                m.objects.host.com_result = flag(s, "com_result", false);
                result = Some(m.register_factory(n(s, "id"), n(s, "type"), None));
            }
            "many_factories" => {
                for i in 0..n(s, "count") {
                    assert!(m.register_factory(n(s, "start") + i, n(s, "start") + i, Some(object(s, "factory"))));
                }
            }
            "remove_factory" => result = Some(m.remove_factory(n(s, "id"))),
            "factory_lookup" => result = Some(m.factory_for_type(n(s, "type"), &mut output)),
            "factory_count" => assert_eq!(m.factory_count(), n(s, "expect")),
            "factory_at" => {
                output = m.factory_at(n(s, "index"));
            }
            "append_segment" => result = Some(m.append_segment(seg())),
            "prepend_segment" => result = Some(m.prepend_segment(seg())),
            "register_segment" => result = Some(m.register_segment(seg())),
            "remove_segment" => result = Some(m.remove_segment(seg())),
            "has_segment" => result = Some(m.has_segment(seg())),
            "segment_lookup" => result = Some(m.segment_for_key(k, &mut output)),
            "segment_id" => result = Some(m.segment_by_id(n(s, "id"), &mut output)),
            "segment_at" => output = m.segment_at(n(s, "index")),
            "segments" => {
                let expected: Vec<_> = array(s, "expect")
                    .iter()
                    .map(|i| Some(Object(u64::from(number(i)))))
                    .collect();
                assert_eq!(
                    (0..m.segment_count()).map(|i| m.segment_at(i)).collect::<Vec<_>>(),
                    expected
                );
            }
            "array" => {
                let mut capacity = n(s, "capacity");
                let mut out = vec![Some(Object(999)); capacity as usize];
                let total = m.segment_array(
                    if flag(s, "null", false) { None } else { Some(&mut out) },
                    &mut capacity,
                );
                assert_eq!(total, n(s, "total"));
                assert_eq!(capacity, n(s, "written"));
                if !flag(s, "null", false) {
                    assert_eq!(
                        &out[..capacity as usize],
                        array(s, "expect")
                            .iter()
                            .map(|i| Some(Object(u64::from(number(i)))))
                            .collect::<Vec<_>>()
                    );
                }
            }
            "visit" => {
                let mut visits = Vec::new();
                let stop = s.get("stop").map(number);
                m.visit_segments(Some(&mut |o, _| {
                    visits.push(o.0 as u32);
                    Some(o.0 as u32) != stop
                }));
                assert_eq!(visits, array(s, "expect").iter().map(number).collect::<Vec<_>>());
            }
            "visit_null" => m.visit_segments(None),
            "load" => result = Some(m.load(k, iid, &mut output, args)),
            "uncached" => result = Some(m.load_uncached(k, iid, &mut output, args)),
            "create" => result = Some(m.create(n(s, "type"), iid, &mut output, args)),
            "allocate_instance" => assert_eq!(m.allocate_instance(), n(s, "expect")),
            "create_as" => result = Some(m.create_as(k, iid, &mut output, args)),
            "insert_hook" => result = Some(m.insert_cache(k, r())),
            "register_resource" => result = Some(m.register_resource(s.get("resource").map(|_| r()))),
            "register_as" => result = Some(m.register_resource_as(r(), k)),
            "is_cached" => result = Some(m.is_cached(k)),
            "is_in_segments" => result = Some(m.is_in_segments(k)),
            "unregister_resource" => result = Some(m.unregister_resource(k)),
            "cached_keys" | "segment_keys" => {
                let f = s.get("filter").and_then(filter);
                let (count, list) = if op == "cached_keys" {
                    m.cached_keys(f)
                } else {
                    m.segment_keys(f)
                };
                assert_eq!(count, n(s, "count"));
                assert_eq!(list.keys(), keys(s, "expect"));
                assert_eq!(list.reference_count(), 1);
            }
            "cached_type" | "segment_type" => {
                let (count, list) = if op == "cached_type" {
                    m.cached_keys_of_type(n(s, "type"))
                } else {
                    m.segment_keys_of_type(n(s, "type"))
                };
                assert_eq!(count, n(s, "count"));
                assert_eq!(list.keys(), keys(s, "expect"));
            }
            "cache_insert" | "cache_get" | "cache_erase" | "cache_contains" | "cache_accepts" | "cache_filter"
            | "cache_enum" | "cache_type" | "cache_purge_one" | "cache_purge" | "cache_tick" | "cache_auto"
            | "cache_qi" | "cache_shutdown" | "cache_destroy" => {
                let c = cache(s.get("cache").unwrap(), &names);
                m.objects.with_builtin(c, |c, h| match op {
                    "cache_insert" => result = Some(c.insert(h, k, r())),
                    "cache_get" => result = Some(c.get(h, k, &mut output)),
                    "cache_erase" => result = Some(c.erase(h, k)),
                    "cache_contains" => result = Some(c.contains(h, k)),
                    "cache_accepts" => result = Some(c.accepts(h, k)),
                    "cache_filter" => c.set_filter(h, s.get("filter").and_then(filter)),
                    "cache_enum" | "cache_type" => {
                        let mut list = KeyList::new();
                        let count = if op == "cache_enum" {
                            c.enumerate(h, Some(&mut list), s.get("filter").and_then(filter))
                        } else {
                            c.enumerate_type(h, Some(&mut list), n(s, "type"))
                        };
                        assert_eq!(count, n(s, "count"));
                        assert_eq!(list.keys(), keys(s, "expect"));
                    }
                    "cache_purge_one" => result = Some(c.purge_one(h)),
                    "cache_purge" => result = Some(c.purge(h)),
                    "cache_tick" => {
                        for _ in 0..n(s, "count") {
                            assert!(c.update(h, 0));
                        }
                        assert_eq!(c.maintenance_counter(), n(s, "counter"));
                    }
                    "cache_auto" => c.set_automatic_purge(h, n(s, "value") as u8),
                    "cache_qi" => {
                        let mut out = Some(CacheId::Null);
                        result = Some(c.query_interface(h, iid, &mut out));
                        if !result.unwrap() {
                            assert_eq!(out, Some(CacheId::Null));
                        }
                    }
                    "cache_shutdown" => c.shutdown(h),
                    "cache_destroy" => c.destroy(h, n(s, "flags")),
                    _ => unreachable!(),
                });
            }
            "virtual_purge" => {
                let c = cache(s.get("cache").unwrap(), &names);
                let mut remaining = n(s, "true_count");
                for expected in array(s, "passes") {
                    let mut calls = 0;
                    let ret = m.objects.with_builtin(c, |c, h| {
                        c.purge_with(h, |_, _| {
                            calls += 1;
                            if remaining == 0 {
                                false
                            } else {
                                remaining -= 1;
                                true
                            }
                        })
                    });
                    assert_eq!(calls, n(expected, "calls"));
                    assert_eq!(ret, flag(expected, "expect", false));
                }
            }
            "resource_key" => assert_eq!(m.objects.host.resource_key(r()), k),
            "set_key" => m.objects.host.resource_set_key(r(), k),
            "add_ref" => {
                m.objects.host.add_ref(object(s, "object"));
            }
            "release" => {
                m.objects.host.release(object(s, "object"));
            }
            "refs" => {
                for e in array(s, "expect") {
                    let a = e.as_array().unwrap();
                    assert_eq!(
                        m.objects
                            .host
                            .refs
                            .get(&Object(u64::from(number(&a[0]))))
                            .copied()
                            .unwrap_or(0),
                        number(&a[1]),
                        "{id} step {index}: refs {a:?}"
                    );
                }
            }
            "builtin_refs" => assert_eq!(
                m.objects
                    .builtin(cache(s.get("cache").unwrap(), &names))
                    .unwrap()
                    .reference_count(),
                n(s, "expect")
            ),
            "cache_add_ref_return" => assert_eq!(
                m.objects.add_ref(cache(s.get("cache").unwrap(), &names)),
                n(s, "expect")
            ),
            "write_resource" => result = Some(m.write_resource(r(), k, seg())),
            "write_key" => result = Some(m.write_key(k, seg())),
            "write_keys" => {
                let mut list = KeyList::new();
                for k in keys(s, "keys") {
                    list.insert(k);
                }
                result = Some(m.write_keys(&list, seg()));
            }
            "open_record" => result = Some(m.open_record(k, &mut output, n(s, "writable"))),
            "close_record" => {
                output = Some(object(s, "record"));
                result = Some(m.close_record(None, &mut output));
            }
            "created_records" => assert_eq!(m.objects.host.created_records, keys(s, "expect")),
            "opened_keys" => assert_eq!(
                m.objects.host.opened.iter().map(|(_, k, _)| *k).collect::<Vec<_>>(),
                keys(s, "expect")
            ),
            "args" => assert_eq!(
                m.objects.host.arguments,
                array(s, "expect")
                    .iter()
                    .map(|v| {
                        let a = v.as_array().unwrap();
                        [number(&a[0]), number(&a[1])]
                    })
                    .collect::<Vec<_>>()
            ),
            "set" => m.objects.host.set(s),
            "reset" => m.objects.host.reset(),
            "trace" => assert_eq!(
                m.objects.host.calls,
                array(s, "expect")
                    .iter()
                    .map(|s| s.as_str().unwrap().to_owned())
                    .collect::<Vec<_>>(),
                "{id} trace at {index}"
            ),
            "events" => assert_eq!(
                m.objects.host.events,
                array(s, "expect")
                    .iter()
                    .map(|s| s.as_str().unwrap().to_owned())
                    .collect::<Vec<_>>(),
                "{id} events at {index}"
            ),
            "shutdown" => result = Some(m.shutdown()),
            "initialize" => result = Some(m.initialize()),
            "update" => result = Some(m.update(n(s, "argument"))),
            "auto" => assert_eq!(m.automatic_purge(), n(s, "expect") as u8),
            "set_auto" => m.set_automatic_purge(n(s, "value") as u8),
            "purge" => assert_eq!(m.purge(), n(s, "expect") as u8),
            "qi" => {
                let mut out = Some(InterfaceView::Manager);
                result = Some(m.query_interface(InterfaceView::Unknown, iid, &mut out));
                assert_eq!(
                    out,
                    Some(match s.get("view").and_then(Json::as_str).unwrap_or("manager") {
                        "service" => InterfaceView::Service,
                        "unknown" => InterfaceView::Unknown,
                        _ => InterfaceView::Manager,
                    })
                );
            }
            "manager_add_ref" => assert_eq!(m.add_ref(), n(s, "expect")),
            "manager_release" => assert_eq!(m.release(InterfaceView::Unknown), n(s, "expect")),
            "destroy" => m.destroy(n(s, "flags")),
            "allocated" => assert_eq!(m.allocation_freed(), Some(flag(s, "freed", false))),
            _ => panic!("unknown operation {op}"),
        }
        if let Some(expected) = s.get("expect").and_then(Json::as_bool) {
            assert_eq!(result, Some(expected), "{id} step {index} {op}");
        }
        if let Some(expected) = s.get("out") {
            assert_eq!(
                output,
                if expected == &Json::Null {
                    None
                } else {
                    Some(Object(u64::from(number(expected))))
                },
                "{id} step {index} output"
            );
        }
        if flag(s, "release_out", false) {
            m.objects.host.release(output.unwrap());
        }
    }
}
macro_rules! cases { ($($test:ident => $id:literal),* $(,)?) => { $(#[test] fn $test(){run($id);})* }; }
cases! {
    pm01=>"PM01", pm02=>"PM02", pm03=>"PM03", pm04=>"PM04", pm05=>"PM05", pm06=>"PM06",
    pm07=>"PM07", pm08=>"PM08", pm09=>"PM09", pm10=>"PM10", pm11=>"PM11", pm12=>"PM12",
    pm13=>"PM13", pm14=>"PM14", pm15=>"PM15", pm16=>"PM16", pm17=>"PM17", pm18=>"PM18",
    pm19=>"PM19", pm20=>"PM20", pm21=>"PM21", pm22=>"PM22", pm23=>"PM23", pm24=>"PM24",
    pm25=>"PM25", pm26=>"PM26", pm27=>"PM27", pm28=>"PM28", pm29=>"PM29", pm30=>"PM30",
    pmu01=>"PMU01", pmu02=>"PMU02", pmu03=>"PMU03", pmu04=>"PMU04", pmu05=>"PMU05", pmu06=>"PMU06",
    foreign_and_lifetime=>"EX01", byte_broadcasts=>"EX02", segment_enumeration=>"EX03", failure_cleanup=>"EX04"
}
