// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent path vectors from paths.md §§1–2, 8. Expectations are literal JSON bytes/text.

use fc4_core::paths::{helpers, CurrentDirectory, RzPath};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{arr, opt, str_of, R};

struct Cwd<'a>(&'a Json);
impl CurrentDirectory for Cwd<'_> {
    fn current_drive(&mut self) -> Option<u8> {
        opt(self.0, "drive")
            .and_then(Json::as_str)
            .and_then(|s| s.as_bytes().first().copied())
    }
    fn drive_directory(&mut self, drive: u8) -> Option<Vec<u8>> {
        self.0
            .get("cwd")?
            .get(&char::from(drive).to_string())?
            .as_str()
            .map(|s| s.as_bytes().to_vec())
    }
}

fn bytes<'a>(op: &'a Json, key: &str) -> R<&'a [u8]> {
    Ok(str_of(field(op, key)?, key)?.as_bytes())
}

fn text(bytes: &[u8]) -> R<Json> {
    Ok(Json::Str(
        std::str::from_utf8(bytes).map_err(|e| e.to_string())?.to_owned(),
    ))
}

fn count(op: &Json, key: &str, default: usize) -> R<usize> {
    match opt(op, key) {
        None => Ok(default),
        Some(j) => j
            .as_i64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| format!("invalid {key}")),
    }
}

fn optional_text(value: Option<Vec<u8>>) -> R<Json> {
    value.as_deref().map(text).unwrap_or(Ok(Json::Null))
}

pub(crate) fn exec(_: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let method = name.strip_prefix("path_")?;
    Some((|| {
        let input = bytes(op, "input")?;
        let got = match method {
            "is_full" => Json::Bool(helpers::is_full_path(input).map_err(|e| format!("unsupported CRT byte: {e:?}"))?),
            "normalize" => {
                let max = count(op, "max", 520)?;
                optional_text(if let Some(base) = opt(op, "base") {
                    helpers::fullpath_with_base(Some(input), max, str_of(base, "base")?.as_bytes())
                } else {
                    helpers::fullpath_current(Some(input), max, &mut Cwd(op))
                })?
            }
            "split" => {
                let p = helpers::splitpath(input);
                Json::Array(
                    [p.drive, p.directory, p.name, p.extension]
                        .iter()
                        .map(|p| text(p))
                        .collect::<R<_>>()?,
                )
            }
            "split_directory" => match helpers::split_directory(
                input,
                i32::try_from(opt(op, "count").and_then(Json::as_i64).unwrap_or(2))
                    .map_err(|_| "invalid signed count".to_owned())?,
            ) {
                Some(parts) => Json::Array(parts.iter().map(|p| text(p)).collect::<R<_>>()?),
                None => Json::Null,
            },
            "owner" => {
                let mut path = RzPath::new(input, &mut Cwd(op)).map_err(|e| format!("unsupported CRT byte: {e:?}"))?;
                let mut results = Vec::new();
                let mut out = Vec::new();
                for call in arr(field(op, "calls")?, "calls")? {
                    let method = str_of(field(call, "method")?, "method")?;
                    let result = match method {
                        "get_path" => {
                            path.get_path(&mut out).map_err(|e| format!("unsupported CRT byte: {e:?}"))?;
                            text(&out)?
                        }
                        "get_drive_and_directory" => {
                            path.get_drive_and_directory(&mut out).map_err(|e| format!("unsupported CRT byte: {e:?}"))?;
                            text(&out)?
                        }
                        "get_file_name_and_extension" => {
                            path.get_file_name_and_extension(&mut out).map_err(|e| format!("unsupported CRT byte: {e:?}"))?;
                            text(&out)?
                        }
                        "get_drive" => text(path.get_drive().map_err(|e| format!("unsupported CRT byte: {e:?}"))?)?,
                        "get_directory" => text(path.get_directory())?,
                        "get_file_name" => text(path.get_file_name())?,
                        "file_name" => text(path.file_name())?,
                        "get_extension" => text(path.get_extension())?,
                        "set_file_name" => Json::Bool(path.set_file_name(
                            bytes(call, "value")?,
                            call.get("split").and_then(Json::as_bool).unwrap_or(false),
                        )),
                        "set_extension" => Json::Bool(path.set_extension(
                            bytes(call, "value")?,
                            call.get("split").and_then(Json::as_bool).unwrap_or(false),
                        )),
                        "set_path" => Json::Bool(path.set_path(bytes(call, "value")?, &mut Cwd(op))
                            .map_err(|e| format!("unsupported CRT byte: {e:?}"))?),
                        "set_buffer" => {
                            path.set_buffer(count(call, "parts", 15)? as u32)
                                .map_err(|e| format!("unsupported CRT byte: {e:?}"))?;
                            Json::Null
                        }
                        _ => return Err(format!("unknown path method {method}")),
                    };
                    results.push(result);
                }
                Json::Array(results)
            }
            _ => return Err(format!("unknown path op {method}")),
        };
        let expected = field(op, "expect")?;
        if &got == expected {
            Ok(())
        } else {
            Err(format!("path result {got:?}, expected {expected:?}"))
        }
    })())
}
