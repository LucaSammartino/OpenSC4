// SPDX-License-Identifier: GPL-3.0-or-later

use crate::json::Json;
use crate::runner::HostRef;
use crate::scalar::R;

pub(crate) fn exec(_host: &HostRef, _name: &str, _op: &Json) -> Option<R<()>> {
    None
}
