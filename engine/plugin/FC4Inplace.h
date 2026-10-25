// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

namespace fc4 {
// S3 experiment; every entry point is inert unless `inplace` selects a function.
void InplaceStartup();
void InplaceCityLoaded();
void InplaceReport();
}
