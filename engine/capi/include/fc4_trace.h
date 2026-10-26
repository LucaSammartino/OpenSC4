/* SPDX-License-Identifier: GPL-3.0-or-later */
#pragma once
#include "fc4.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Optional tooling transport. No game objects or process pointers in files. */
bool fc4_trace_configure(const uint8_t* dir, size_t dir_len, const uint8_t* scenario, size_t scenario_len);
void fc4_trace_budget_expected(int32_t year, int32_t month, int32_t day,
                               const int64_t* wa, const int64_t* tax, int64_t total, int64_t income_tax,
                               bool compare_income, bool expense_mismatch, int32_t recorded_mismatches);
#ifdef __cplusplus
}
#endif
