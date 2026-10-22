// SPDX-License-Identifier: GPL-3.0-or-later

//! Independent, scripted public feature-manager scenarios (§10). No reference
//! implementation: expected results and resource traces come from JSON fixtures.

use std::collections::BTreeMap;

use fc4_core::feature_manager::{FeatureManager, FeatureProcess, PackResources, ResourceKey};

use crate::foundations::hex_decode;
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{arr, check_expect, check_state, ji, opt, str_of, u32_of, R};

#[derive(Default)]
struct Resources {
    files: BTreeMap<u32, Vec<u8>>,
    trace: Vec<Json>,
}

struct File(u32, Vec<u8>);

impl AsRef<[u8]> for File {
    fn as_ref(&self) -> &[u8] {
        &self.1
    }
}

impl PackResources for Resources {
    type File = File;

    fn register_static_shutdown(&mut self, _: &FeatureProcess) {
        self.trace.push(Json::Str("register:0".into()));
    }

    fn open_resource(&mut self, key: ResourceKey) -> Option<File> {
        self.trace.push(Json::Str(format!(
            "open:{:08X}:{:08X}:{:08X}",
            key.type_id, key.group_id, key.instance_id
        )));
        self.files
            .get(&key.instance_id)
            .map(|bytes| File(key.instance_id, bytes.clone()))
    }

    fn release_file(&mut self, file: File) {
        self.trace.push(Json::Str(format!("release:{:08X}", file.0)));
    }
}

fn read_files(value: &Json) -> R<BTreeMap<u32, Vec<u8>>> {
    let mut files = BTreeMap::new();
    for file in arr(value, "resources")? {
        let pack = u32_of(field(file, "pack")?, "pack")?;
        if !(1..=32).contains(&pack) {
            return Err("resource fixture pack must be 1..32".into());
        }
        let bytes = if let Some(hex) = opt(file, "hex") {
            hex_decode(str_of(hex, "hex")?)?
        } else {
            str_of(field(file, "text")?, "text")?.as_bytes().to_vec()
        };
        // A primitive store: one already-resolved highest-priority record per key.
        files.insert(0x2B56_3700 + pack, bytes);
    }
    Ok(files)
}

pub(crate) fn exec(_: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    (name == "fm_scenario").then(|| scenario(op))
}

fn scenario(op: &Json) -> R<()> {
    let process = FeatureProcess::new();
    let mut resources = Resources {
        files: read_files(field(op, "resources")?)?,
        ..Resources::default()
    };
    let mut managers = BTreeMap::from([("a".to_owned(), FeatureManager::new(process.clone()))]);
    let sentinel = FeatureManager::new(process.clone()).interface();
    let mut output = sentinel.clone();
    for (index, step) in arr(field(op, "actions")?, "actions")?.iter().enumerate() {
        let mut run = || -> R<()> {
            let name = str_of(field(step, "method")?, "method")?;
            let object = opt(step, "object")
                .map(|v| str_of(v, "object"))
                .transpose()?
                .unwrap_or("a");
            if name == "new" {
                if managers.contains_key(object) {
                    return Err(format!("object '{object}' already exists"));
                }
                managers.insert(object.to_owned(), FeatureManager::new(process.clone()));
            }
            let manager = managers
                .get_mut(object)
                .ok_or_else(|| format!("unknown object '{object}'"))?;
            let argument = || u32_of(field(step, "arg")?, "arg");
            let result = match name {
                "new" => Json::Bool(true),
                "init" => manager.init(&mut resources).map(Json::Bool).unwrap_or(Json::Null),
                "shutdown" => manager.shutdown().map(Json::Bool).unwrap_or(Json::Null),
                "add_ref" => manager.add_ref().map(ji).unwrap_or(Json::Null),
                "release" => manager.release().map(ji).unwrap_or(Json::Null),
                "delete" => manager
                    .deleting_destructor(argument()?)
                    .map(|_| Json::Bool(true))
                    .unwrap_or(Json::Null),
                "query_interface" => Json::Bool(manager.query_interface(argument()?, &mut output)),
                "feature" => manager
                    .is_feature_enabled(argument()?)
                    .map(Json::Bool)
                    .unwrap_or(Json::Null),
                "pack" => manager
                    .is_expansion_pack_installed(argument()?)
                    .map(Json::Bool)
                    .unwrap_or(Json::Null),
                "current" => manager.get_current_expansion_pack().map(ji).unwrap_or(Json::Null),
                "static_shutdown" => Json::Bool(process.static_shutdown()),
                "replace_resources" => {
                    resources.files = read_files(field(step, "resources")?)?;
                    Json::Bool(true)
                }
                "trace" => Json::Array(resources.trace.clone()),
                _ => return Err(format!("unknown feature-manager method '{name}'")),
            };
            check_expect(step, result)?;
            if let Some(want) = opt(step, "state") {
                let deletion = manager.deletion();
                check_state(
                    &[
                        ("count".into(), manager.reference_count().map(ji).unwrap_or(Json::Null)),
                        (
                            "initialized".into(),
                            manager.initialized().map(Json::Bool).unwrap_or(Json::Null),
                        ),
                        (
                            "current".into(),
                            manager.get_current_expansion_pack().map(ji).unwrap_or(Json::Null),
                        ),
                        (
                            "output_same".into(),
                            Json::Bool(output.is_some() && output == manager.interface()),
                        ),
                        ("output_untouched".into(), Json::Bool(output == sentinel)),
                        ("destroyed".into(), Json::Bool(deletion.is_some())),
                        (
                            "freed".into(),
                            deletion.map(|d| Json::Bool(d.allocation_freed)).unwrap_or(Json::Null),
                        ),
                        (
                            "destructor_count".into(),
                            deletion.map(|d| ji(d.count_at_destruction)).unwrap_or(Json::Null),
                        ),
                        (
                            "destructor_initialized".into(),
                            deletion
                                .map(|d| Json::Bool(d.initialized_at_destruction))
                                .unwrap_or(Json::Null),
                        ),
                    ],
                    want,
                )?;
            }
            Ok(())
        };
        run().map_err(|e| format!("feature-manager action {index}: {e}"))?;
    }
    Ok(())
}
