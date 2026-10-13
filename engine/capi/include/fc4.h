/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
 * Copyright (C) 2026 FreeCity4 contributors
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version. This program is distributed WITHOUT ANY WARRANTY; see
 * the GNU General Public License for more details. You should have received a
 * copy of the license along with this program (see LICENSE at the repository
 * root); if not, see <https://www.gnu.org/licenses/>.
 */

/*
 * fc4.h - the C ABI of the FreeCity4 core (Rust crate fc4-capi).
 *
 * Hand-written; it must stay in sync with engine/capi/src/lib.rs. The layout
 * checks at the bottom catch drift at compile time on the C/C++ side, and
 * fc4_abi_version() lets a consumer refuse a mismatched library.
 *
 * Conventions
 *   - All functions use the platform's default C calling convention.
 *   - `bool` is C99/C++ bool (1 byte).
 *   - Strings are UTF-8, passed as pointer + length (no NUL terminator needed).
 *   - Every function is safe to call re-entrantly from inside a host callback
 *     (e.g. an agent's deliver() may add/remove agents or pause).
 *   - No Rust panic ever crosses this boundary: release builds abort, and each
 *     entry point catches unwinding and returns its documented fallback value.
 *   - A NULL `fc4_sim*` is tolerated by every function (it returns the fallback).
 *   - Single-threaded: one simulator must only be used from one thread at a time.
 */
#ifndef FC4_H
#define FC4_H

#include <stddef.h>
#include <stdint.h>
#ifndef __cplusplus
#include <stdbool.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* ABI history: 1 = initial; 2 = adds fc4_sim_on_post_city_init;
 * 3 = adds fc4_sim_get_debug and fc4_sim_get_agent_info (diagnostics);
 * 4 = adds the limit-timer calibration (fc4_timer_source, spec §5.5);
 * 5 = adds host streams (fc4_ostream / fc4_istream / fc4_ref);
 * 6 = adds the budget (fc4_budget / fc4_department / fc4_line_item);
 * 7 = adds fc4_department_find_local_funding_percent (budget erratum);
 * 8 = adds the land value simulator (fc4_landvalue);
 * 9 = adds host-built property holders (fc4_props) and the weather (fc4_weather);
 * 10 = adds fc4_props_set_typed and the ordinances (fc4_ordinance_sim / fc4_ordinance);
 * 11 = adds the demand simulator and demand nodes (fc4_demand_sim / fc4_demand_node);
 * 12 = adds the game's grids as host objects (fc4_grid_api) and the power simulator (fc4_power);
 * 13 = adds the plumbing simulator (fc4_plumbing);
 * 14 = adds the flammability simulator (fc4_flammability);
 * 15 = adds the budget shadow's tax scan (fc4_budget_shadow_tax). */
#define FC4_ABI_VERSION 15u

/* Return codes of fc4_sim_load. */
#define FC4_OK                    0
#define FC4_ERR_UNSUPPORTED_VERSION (-1)
#define FC4_ERR_TRUNCATED         (-2)
#define FC4_ERR_INVALID_ARGUMENT  (-3)
#define FC4_ERR_PANIC             (-100)

/* Agent levels (spec §2.2). */
#define FC4_LEVEL_ONE_SHOT      0u
#define FC4_LEVEL_EMERGENCY     1u
#define FC4_LEVEL_MINUTE        2u
#define FC4_LEVEL_HOUR          3u
#define FC4_LEVEL_DAY           4u
#define FC4_LEVEL_WEEK          5u
#define FC4_LEVEL_MONTH         6u
#define FC4_LEVEL_YEAR          7u
#define FC4_LEVEL_SPECIFIC_DAY  8u
#define FC4_LEVEL_SPECIFIC_TIME 9u
#define FC4_LEVEL_IDLE          10u
#define FC4_LEVEL_COUNT         11u

/* Opaque simulator instance. */
typedef struct fc4_sim fc4_sim;

/* Opaque agent identity chosen by the host (e.g. a cIGZMessageTarget2* value). */
typedef uint64_t fc4_agent;

/* A scheduler or broadcast message (the cIGZMessage2Standard fields used). */
typedef struct fc4_message {
    uint32_t type;
    uint32_t reserved; /* always 0; explicit padding */
    int64_t data1;
    int64_t data2;
    int64_t data3;
} fc4_message;

/* Host callbacks. `now_ms` is required; any other callback may be NULL
 * (a NULL deliver counts as "finished"). `user` is passed back verbatim. */
typedef struct fc4_host {
    uint32_t struct_size; /* = sizeof(fc4_host) */
    void* user;
    /* Monotonic milliseconds; only differences are used (wrapping). */
    uint32_t (*now_ms)(void* user);
    /* Deliver synchronously; return the agent's "finished" flag (spec §4). */
    bool (*deliver)(void* user, fc4_sim* sim, fc4_agent agent, const fc4_message* msg);
    /* Exactly one add_ref per registration added, one release per removal. */
    void (*add_ref)(void* user, fc4_agent agent);
    void (*release)(void* user, fc4_agent agent);
    /* Queued broadcast (MessagePost). */
    void (*post)(void* user, const fc4_message* msg);
} fc4_host;

/* Tuning inputs (spec §7). A NULL pointer means "property missing".
 * ms_per_day is used only with exactly 3 values, dilation with exactly 5. */
typedef struct fc4_tuning {
    const uint32_t* ms_per_day;
    size_t ms_per_day_count;
    const float* animation_dilation;
    size_t animation_dilation_count;
} fc4_tuning;

/* Calendar fields of one day. */
typedef struct fc4_date {
    int32_t julian;
    int32_t year;
    int32_t month;        /* 1..12 */
    int32_t day_of_month; /* 1..31 */
    int32_t day_of_year;  /* 1..366 */
    int32_t weekday;      /* 1 = Monday .. 7 = Sunday */
} fc4_date;

/* Full observable state (diagnostics / tests). */
typedef struct fc4_state {
    uint32_t struct_size; /* set by the caller to sizeof(fc4_state) */
    uint8_t initialized;
    uint8_t in_tick;
    uint8_t city_established;
    uint8_t speed_change_enabled;
    int32_t pause_count;
    int32_t hidden_pause_count;
    int32_t emergency_pause_count;
    int32_t sim_speed;
    int32_t sim_time;
    uint32_t frame_budget_ms;
    uint32_t gate_level;
    uint32_t ms_per_day;
    int32_t target_fps;
    int32_t julian_day;
    uint32_t ms_per_day_table[3];
    float animation_dilation[5];
    uint32_t level_counters[FC4_LEVEL_COUNT];
    uint8_t level_fired[FC4_LEVEL_COUNT];
    uint8_t reserved;
    uint32_t agent_counts[FC4_LEVEL_COUNT];
} fc4_state;

/* Scheduler/barrier state for diagnostics (spec §5.2 step 8). */
typedef struct fc4_debug {
    uint32_t struct_size; /* set by the caller to sizeof(fc4_debug) */
    uint32_t interval_elapsed_ms;
    uint32_t slice_elapsed_ms;
    uint32_t ms_per_day;
    uint8_t ready;                         /* interval timer >= msPerDay */
    uint8_t one_shot_or_emergency_pending; /* level 0 or 1 has agents: time suspended */
    uint8_t hours_complete;                /* levelCounter[Hour] >= 24 */
    uint8_t day_level_done;                /* cursor[Day] at end */
    uint8_t tomorrow_starts_week;
    uint8_t tomorrow_starts_month;
    uint8_t tomorrow_starts_year;
    uint8_t day_gate_open; /* every condition for the next day holds now */
    uint8_t blocked_by_week;
    uint8_t blocked_by_month;
    uint8_t blocked_by_year;
    uint8_t reserved;
    int32_t cursors[FC4_LEVEL_COUNT]; /* index in registration order, -1 = at end */
    int32_t idle_cursor;
} fc4_debug;

/* One agent registration (diagnostics). */
typedef struct fc4_agent_info {
    fc4_agent agent;
    uint32_t flags;
    uint8_t enabled;
    uint8_t reserved[3];
} fc4_agent_info;

/* Sources and sink for the CPU-cycle limit-timer calibration (spec §5.5).
 * `qpc` and `tsc` are required; `publish` may be NULL. */
typedef struct fc4_timer_source {
    uint32_t struct_size; /* = sizeof(fc4_timer_source) */
    void* user;
    uint64_t (*qpc)(void* user); /* high-resolution wall clock, ticks */
    uint64_t (*tsc)(void* user); /* processor time-stamp counter, cycles */
    /* CyclesToMillisecs (ms per cycle) and MillisecsToCycles (cycles per ms). */
    void (*publish)(void* user, float cycles_to_ms, float ms_to_cycles);
    uint64_t qpc_frequency; /* wall-clock ticks per second, > 0 */
} fc4_timer_source;

typedef void (*fc4_name_callback)(void* ctx, const char* name, size_t name_len);

/* ---- library ---------------------------------------------------------- */
uint32_t fc4_abi_version(void);

/* Tooling: where a Rust panic is reported (message, location, thread and a
 * backtrace, as one UTF-8 block with lines separated by '\n') before the panic
 * goes on; release builds then abort with a fail-fast. Installs the panic
 * hook on the first call; a NULL sink stops the reports. Any thread. */
typedef void (*fc4_log_sink)(void* ctx, const uint8_t* text, size_t len);
void fc4_set_panic_log(fc4_log_sink sink, void* ctx);
/* Tooling: panics on purpose (tests of the report). Release builds abort the
 * process; builds that unwind return. */
void fc4_debug_panic(void);

/* ---- lifetime --------------------------------------------------------- */
/* Returns NULL if `host` is NULL, too small, or has no now_ms. */
fc4_sim* fc4_sim_create(const fc4_host* host);
/* Releases every remaining agent registration, then frees the simulator. */
void fc4_sim_destroy(fc4_sim* sim);

/* ---- lifecycle (spec §9; GZCOM registrations are the caller's job) ---- */
bool fc4_sim_init(fc4_sim* sim, const fc4_tuning* tuning /* nullable */);
bool fc4_sim_shutdown(fc4_sim* sim);
bool fc4_sim_is_initialized(const fc4_sim* sim);

/* ---- per-frame (spec §5) ----------------------------------------------- */
bool fc4_sim_on_tick(fc4_sim* sim);
void fc4_sim_simulate_time_passage(fc4_sim* sim);
void fc4_sim_simulate_agents(fc4_sim* sim);
void fc4_sim_simulate_idle_time(fc4_sim* sim);
void fc4_sim_begin_new_interval(fc4_sim* sim, uint32_t level);

/* ---- pause and speed (spec §6) ----------------------------------------- */
bool fc4_sim_pause(fc4_sim* sim);
bool fc4_sim_resume(fc4_sim* sim);
bool fc4_sim_hidden_pause(fc4_sim* sim);
bool fc4_sim_hidden_resume(fc4_sim* sim);
/* Notification 0x26D31EC1 (PostCityInit, spec §9): releases the hidden pause set
 * by Init. The caller unsubscribes from the notification itself. */
bool fc4_sim_on_post_city_init(fc4_sim* sim);
bool fc4_sim_emergency_pause(fc4_sim* sim);
bool fc4_sim_emergency_resume(fc4_sim* sim);
bool fc4_sim_is_paused(const fc4_sim* sim);
bool fc4_sim_is_hidden_paused(const fc4_sim* sim);
bool fc4_sim_is_emergency_paused(const fc4_sim* sim);
bool fc4_sim_is_any_paused(const fc4_sim* sim);
int32_t fc4_sim_get_sim_speed(const fc4_sim* sim);
bool fc4_sim_set_sim_speed(fc4_sim* sim, int32_t speed);
int32_t fc4_sim_get_sim_time(const fc4_sim* sim);
bool fc4_sim_set_sim_time(fc4_sim* sim, int32_t time);
bool fc4_sim_set_max_milliseconds_per_tick(fc4_sim* sim, uint32_t ms);
float fc4_sim_get_animation_time_dilation(const fc4_sim* sim);
bool fc4_sim_set_city_established(fc4_sim* sim, bool established);
bool fc4_sim_is_city_established(const fc4_sim* sim);

/* ---- agents (spec §4) --------------------------------------------------- */
bool fc4_sim_add_agent(fc4_sim* sim, fc4_agent agent, uint32_t level,
                       const char* name, size_t name_len, uint32_t flags);
bool fc4_sim_remove_agent(fc4_sim* sim, fc4_agent agent, uint32_t level);
bool fc4_sim_remove_agent_all_levels(fc4_sim* sim, fc4_agent agent);
bool fc4_sim_remove_all_agents(fc4_sim* sim);
bool fc4_sim_remove_all_agents_at(fc4_sim* sim, uint32_t level);
bool fc4_sim_get_agent_enabled(const fc4_sim* sim, const char* name, size_t name_len);
bool fc4_sim_set_agent_enabled(fc4_sim* sim, const char* name, size_t name_len, bool enabled);
/* Calls `cb` once per registration, level by level; returns the count. */
size_t fc4_sim_enumerate_agents(const fc4_sim* sim, fc4_name_callback cb, void* ctx);

/* ---- date ----------------------------------------------------------------- */
int32_t fc4_sim_get_julian_day(const fc4_sim* sim);
bool fc4_sim_get_date(const fc4_sim* sim, fc4_date* out);
int32_t fc4_julian_from_civil(int32_t year, int32_t month, int32_t day);
void fc4_date_from_julian(int32_t julian, fc4_date* out);

/* ---- persistence (spec §8) ------------------------------------------------ */
/* Writes the record if `capacity` suffices; always returns the record size. */
size_t fc4_sim_save(const fc4_sim* sim, uint8_t* buffer, size_t capacity);
/* Returns FC4_OK or an FC4_ERR_* code. On error the state is unchanged. */
int32_t fc4_sim_load(fc4_sim* sim, const uint8_t* buffer, size_t length);
/* Byte widths (1, 2 or 4) of each field of a record of `version`, in order,
 * for hosts with typed streams. Returns the field count, 0 if unsupported. */
size_t fc4_record_layout(uint16_t version, const uint8_t** widths);

/* ---- cheats (spec §9) ------------------------------------------------------ */
/* `text` is the full cheat line ("FPS 30"). Returns true if the id is handled. */
bool fc4_sim_handle_cheat(fc4_sim* sim, uint32_t cheat_id, const char* text, size_t text_len);

/* ---- limit-timer calibration (spec §5.5) ------------------------------------ */
/* Installs the calibration source/sink (copied). `initial_ms_per_cycle` is the
 * host's estimate of milliseconds per TSC cycle. Takes the first sample point
 * and publishes both coefficients. From then on fc4_sim_on_tick calibrates and
 * publishes before doing anything else. Returns false (nothing installed) for
 * an invalid source, a zero frequency or an estimate that is not positive. */
bool fc4_sim_set_timer_source(fc4_sim* sim, const fc4_timer_source* source, float initial_ms_per_cycle);
/* One calibration step without the rest of OnTick (tests). */
void fc4_sim_calibrate_tick(fc4_sim* sim);
/* Current coefficients; false if no timer source is installed. Pointers may be NULL. */
bool fc4_sim_get_calibration(const fc4_sim* sim, float* cycles_to_ms, float* ms_to_cycles, uint64_t* samples);

/* ---- host streams (foundations §2) ------------------------------------------
 * The core writes and reads its records through these callbacks; the GZCOM shim
 * implements them on top of the game's cIGZOStream / cIGZIStream (and the
 * segment streams for references). Every callback follows the stream model of
 * foundations §2.1: false once the stream is in its (sticky) error state. */

/* An object reference (foundations §2.7). */
typedef struct fc4_ref {
    uint64_t token;    /* host identity: an interface pointer; 0 = null reference */
    uint32_t class_id; /* the object's GetGZCLSID (writes) / its class (reads) */
    uint32_t reserved;
    void* core_handle; /* reads: the core handle if the object is one of ours (e.g. fc4_department*) */
} fc4_ref;

typedef void (*fc4_bytes_sink)(void* ctx, const uint8_t* data, size_t len);

typedef struct fc4_ostream {
    uint32_t struct_size; /* = sizeof(fc4_ostream) */
    void* user;
    bool (*put_u8)(void* user, uint8_t v);
    bool (*put_u16)(void* user, uint16_t v);
    bool (*put_u32)(void* user, uint32_t v);
    bool (*put_u64)(void* user, uint64_t v);
    bool (*put_f32)(void* user, float v);
    bool (*put_f64)(void* user, double v);
    bool (*put_bytes)(void* user, const uint8_t* data, size_t len); /* SetVoid */
    bool (*put_str)(void* user, const uint8_t* data, size_t len);   /* SetGZStr */
    bool (*put_ref)(void* user, const fc4_ref* ref);                /* WriteGZSerializable */
    int32_t (*error)(void* user);
} fc4_ostream;

typedef struct fc4_istream {
    uint32_t struct_size; /* = sizeof(fc4_istream) */
    void* user;
    bool (*get_u8)(void* user, uint8_t* out);
    bool (*get_u16)(void* user, uint16_t* out);
    bool (*get_u32)(void* user, uint32_t* out);
    bool (*get_u64)(void* user, uint64_t* out);
    bool (*get_f32)(void* user, float* out);
    bool (*get_f64)(void* user, double* out);
    bool (*get_bytes)(void* user, uint8_t* out, size_t n);           /* GetVoid */
    bool (*get_str)(void* user, fc4_bytes_sink sink, void* sink_ctx); /* GetGZStr, delivered as UTF-8 */
    /* ReadGZSerializable, then QueryInterface to `iid` (0 = keep as read). A host
     * object comes back with ONE reference owned by the caller; a non-null object
     * lacking `iid` reads as a null reference (the stream stays good). */
    bool (*get_ref)(void* user, uint32_t iid, fc4_ref* out);
    bool (*skip)(void* user, size_t n);
    int32_t (*error)(void* user);
} fc4_istream;

/* ---- budget (docs/specs/budget.md) -------------------------------------------
 * Three core objects, each fronted by one GZCOM object that owns its handle:
 * the budget simulator (fc4_budget), departments (fc4_department) and line
 * items (fc4_line_item). Objects the core hands back - departments, line items,
 * property holders, occupants - cross the ABI as TOKENS: the host identity
 * (interface pointer) the object was created with or passed in as. Returned
 * tokens are borrowed unless stated otherwise.
 *
 * Reference counting: the core calls host->add_ref / host->release on tokens it
 * keeps (departments in the budget's map, line items in a department, locally
 * funded property holders). host->create_object creates a department or line
 * item through the class registry and returns its core handle; the core then
 * owns the one reference that object was created with.
 *
 * Every function tolerates a NULL handle and returns its fallback (0 / false). */

typedef struct fc4_budget fc4_budget;
typedef struct fc4_department fc4_department;
typedef struct fc4_line_item fc4_line_item;

/* A message the budget posts (MessagePost, spec §10.1). */
typedef struct fc4_budget_message {
    uint32_t type;
    uint32_t reserved;
    int64_t data1, data2, data3;
    uint64_t void1;   /* SetVoid1: a token, 0 = none */
    uint64_t unknown; /* SetIGZUnknown: a token, 0 = none */
} fc4_budget_message;

/* One lot for ComputeTax (spec §5.2). */
typedef struct fc4_lot_info {
    uint8_t has_building;
    uint8_t livable;
    uint8_t land_value; /* 0..255 at the lot's origin cell */
    uint8_t reserved;
    uint32_t purpose;         /* building profile PurposeType */
    uint32_t building_wealth; /* building profile wealth */
    uint32_t lot_wealth;      /* cISC4Lot::GetOccupantWealth */
    int64_t building_value;   /* building property 0xE91A0B5F, 0 if absent */
    uint32_t size_x, size_z;
} fc4_lot_info;

/* cISC4BudgetSimulator::BudgetItem, with a 32-bit reserved tail. */
typedef struct fc4_budget_item {
    uint32_t department_id;
    uint32_t line_item;
    uint32_t purpose;
    int32_t index;
    int64_t cost;
    float local_funding; /* fraction (1.0 = 100 %) */
    uint32_t reserved;
} fc4_budget_item;

typedef struct fc4_loan_info {
    uint32_t months_remaining;
    uint32_t years_remaining;
    int64_t total_loan_amount;
    int64_t remaining_payment;
    int64_t monthly_payment;
} fc4_loan_info;

typedef struct fc4_bond {
    int32_t due_month, due_year;
    int64_t amount, full_cost, remaining, paid, monthly_payment, principal_left;
} fc4_bond;

typedef struct fc4_budget_group {
    uint32_t id;
    uint32_t department_count;
    int64_t total_expense;
    int64_t total_income;
} fc4_budget_group;

/* Diagnostics view of the budget simulator (spec §3.1). */
typedef struct fc4_budget_state {
    uint32_t struct_size; /* caller sets = sizeof(fc4_budget_state) */
    uint8_t initialized, loaded_from_save, agriculture_rate_fix, reserved0;
    uint32_t panel_flag;
    uint32_t bond_count;
    int64_t funds, shadow_funds, total_borrowed, last_tax_income;
    int64_t ytd_income, ytd_expense, ytd_tax_income, ytd_ordinance_income, ytd_ordinance_expense, ytd_bond_payments;
    int64_t last_month_income, last_month_expense, yearly_income, yearly_expense;
    float tax_rate[12];
    float max_tax_rate[12];
    int64_t weighted_assessed[12];
    int64_t monthly_tax[12];
    uint32_t department_count, group_count;
    uint32_t risky_money_counter, reserved1;
} fc4_budget_state;

/* One lot as the tax scan saw it (fc4_budget_shadow_tax). */
#define FC4_TAX_LOT_COUNTED 0u
#define FC4_TAX_LOT_NO_BUILDING 1u
#define FC4_TAX_LOT_NOT_LIVABLE 2u
#define FC4_TAX_LOT_UNTAXED 3u
typedef struct fc4_tax_lot {
    uint32_t status;  /* FC4_TAX_LOT_* */
    int32_t group;    /* 0..11, -1 = none; for a not-livable lot the group it would have had */
    int64_t assessed; /* §5.2 assessed value; for a not-livable lot what it would have added */
    float tax;        /* lotTax (0 unless counted) */
    uint32_t reserved;
} fc4_tax_lot;

/* The result of fc4_budget_shadow_tax (§5.2, §6 GetTaxIncome(t)). */
typedef struct fc4_tax_scan {
    uint32_t struct_size; /* caller sets = sizeof(fc4_tax_scan) */
    uint32_t lots;        /* lots visited */
    float multiplier;     /* DifficultyTaxMultiplier[difficulty] */
    float total;          /* the float32 running total */
    int64_t weighted_assessed[12];
    int64_t monthly_tax[12]; /* trunc(groupTax[t]) */
    int64_t tax_income[12];  /* GetTaxIncome(t) with the given rates */
    float group_tax[12];
    uint32_t counted[12];    /* lots counted per group */
    int64_t total_tax;       /* ComputeTax's result, trunc(total) */
} fc4_tax_scan;

typedef void (*fc4_tax_lot_callback)(void* ctx, uint32_t index, const fc4_lot_info* lot, const fc4_tax_lot* result);

typedef void (*fc4_lot_visitor)(void* ctx, const fc4_lot_info* lot);
typedef void (*fc4_token_callback)(void* ctx, uint64_t token);
typedef void (*fc4_budget_item_callback)(void* ctx, const fc4_budget_item* item);
typedef void (*fc4_loan_callback)(void* ctx, const fc4_loan_info* loan);
typedef void (*fc4_group_callback)(void* ctx, const fc4_budget_group* group, const uint8_t* name, size_t name_len,
                                   const uint32_t* department_ids);

/* What the budget needs from the game. Copied at creation. Any callback may be
 * NULL: it then reads as "nothing there" (0 / false / absent). Property and
 * query callbacks must not call back into the budget. */
typedef struct fc4_budget_host {
    uint32_t struct_size; /* = sizeof(fc4_budget_host) */
    void* user;
    void (*post)(void* user, const fc4_budget_message* msg);
    void (*show_budget_window)(void* user, uint32_t panel);
    uint32_t (*difficulty)(void* user);                       /* 0..2 */
    void (*sim_date)(void* user, int32_t* year, int32_t* month); /* month 1..12 */
    bool (*city_established)(void* user);
    void (*city_size)(void* user, float* x, float* z);
    int64_t (*ordinance_income)(void* user);
    int64_t (*ordinance_expense)(void* user);
    int64_t (*neighbor_income)(void* user);
    int64_t (*neighbor_expense)(void* user);
    void (*for_each_lot)(void* user, fc4_lot_visitor visit, void* ctx);
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*set_history_queue_length)(void* user, uint32_t series, uint32_t length);
    bool (*try_start_disaster)(void* user, uint32_t id, float x, float z); /* false if it does not exist */
    uint32_t (*rng_seed)(void* user);
    bool (*ltext)(void* user, uint32_t group, uint32_t instance, fc4_bytes_sink sink, void* ctx);
    bool (*building_type_name)(void* user, uint32_t building_type, fc4_bytes_sink sink, void* ctx);
    /* Array properties of a property holder: return the number of values the
     * property has (copying at most `capacity` into `out`), or -1 if absent. */
    int32_t (*get_u32s)(void* user, uint64_t holder, uint32_t prop, uint32_t* out, size_t capacity);
    int32_t (*get_i64s)(void* user, uint64_t holder, uint32_t prop, int64_t* out, size_t capacity);
    int32_t (*get_f32s)(void* user, uint64_t holder, uint32_t prop, float* out, size_t capacity);
    bool (*set_u32s)(void* user, uint64_t holder, uint32_t prop, const uint32_t* values, size_t count);
    bool (*set_i64s)(void* user, uint64_t holder, uint32_t prop, const int64_t* values, size_t count);
    bool (*set_f32s)(void* user, uint64_t holder, uint32_t prop, const float* values, size_t count);
    bool (*copy_property)(void* user, uint64_t src, uint64_t dst, uint32_t prop);
    uint64_t (*occupant_of)(void* user, uint64_t holder); /* 0 = not an occupant */
    uint32_t (*class_of)(void* user, uint64_t token);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* Creates a department (0xE990BFFC) or line item (0xAA313C9F) through the
     * class registry; returns its core handle (fc4_department* / fc4_line_item*). */
    void* (*create_object)(void* user, uint32_t class_id);
} fc4_budget_host;

/* Lifetime: called by the GZCOM object that fronts the core object; `token` is
 * that object's identity. Destroy drops the GZCOM object's share. */
fc4_budget* fc4_budget_create(const fc4_budget_host* host, uint64_t token);
void fc4_budget_destroy(fc4_budget* b);
fc4_department* fc4_department_create(const fc4_budget_host* host, uint64_t token);
void fc4_department_destroy(fc4_department* d);
fc4_line_item* fc4_line_item_create(const fc4_budget_host* host, uint64_t token);
void fc4_line_item_destroy(fc4_line_item* l);

/* Persistence (spec §11). References read back as our own objects arrive as
 * core handles through fc4_ref.core_handle. */
bool fc4_budget_write(const fc4_budget* b, const fc4_ostream* out);
bool fc4_budget_read(fc4_budget* b, const fc4_istream* in);
bool fc4_department_write(const fc4_department* d, const fc4_ostream* out);
bool fc4_department_read(fc4_department* d, const fc4_istream* in);
bool fc4_line_item_write(const fc4_line_item* l, const fc4_ostream* out);
bool fc4_line_item_read(fc4_line_item* l, const fc4_istream* in);

/* Budget simulator: lifecycle (§4). `tuning_holder` is the tuning exemplar as a
 * property-holder token (read through get_i64s / get_f32s), 0 = code defaults.
 * `fix_cs3_weight` enables the intended behaviour of §13.1 (off = compat). */
bool fc4_budget_init(fc4_budget* b, uint64_t tuning_holder, bool fix_cs3_weight);
bool fc4_budget_shutdown(fc4_budget* b);
bool fc4_budget_is_initialized(fc4_budget* b);
bool fc4_budget_handle_message(fc4_budget* b, uint32_t type, int64_t data1); /* DoMessage minus cheats */
bool fc4_budget_handle_cheat(fc4_budget* b, uint32_t cheat_id, const char* text, size_t text_len);
void fc4_budget_simulate_day(fc4_budget* b);   /* §5.7 */
void fc4_budget_settle_month(fc4_budget* b);   /* §5.1 */
int64_t fc4_budget_compute_tax(fc4_budget* b); /* §5.2 */

/* Budget shadow (diagnostics): ComputeTax (§5.2) without a budget object, the
 * same scan the budget simulator runs, over host->for_each_lot, with the tuning
 * read from `tuning_holder` (0 = code defaults) through get_i64s / get_f32s,
 * the given difficulty and 12 group rates (percent, `rates[0..11]`). Calls no
 * other host callback: nothing is stored, posted or written. `cb` (may be
 * NULL) sees every lot in visit order. False (and `out` untouched) on bad
 * arguments. */
bool fc4_budget_shadow_tax(const fc4_budget_host* host, uint64_t tuning_holder, bool fix_cs3_weight,
                           uint32_t difficulty, const float* rates, fc4_tax_lot_callback cb, void* ctx,
                           fc4_tax_scan* out);

/* Budget simulator: methods of cISC4BudgetSimulator (§6), in SDK order. */
bool fc4_budget_set_total_funds(fc4_budget* b, int64_t v);
int64_t fc4_budget_get_total_funds(fc4_budget* b);
bool fc4_budget_deposit_funds(fc4_budget* b, int64_t v);
bool fc4_budget_withdraw_funds(fc4_budget* b, int64_t v);
int64_t fc4_budget_get_min_allowable_funds(fc4_budget* b);
int64_t fc4_budget_get_ytd_income(fc4_budget* b);
int64_t fc4_budget_get_est_income(fc4_budget* b);
int64_t fc4_budget_get_ytd_expenses(fc4_budget* b);
int64_t fc4_budget_get_est_expenses(fc4_budget* b);
int64_t fc4_budget_get_total_monthly_expense(fc4_budget* b); /* 64-bit; see spec §6 [compat] */
int64_t fc4_budget_get_total_yearly_expense(fc4_budget* b);
int64_t fc4_budget_get_total_monthly_income(fc4_budget* b);
int64_t fc4_budget_get_total_yearly_income(fc4_budget* b);
int64_t fc4_budget_get_tax_income(fc4_budget* b);
int64_t fc4_budget_get_tax_income_for(fc4_budget* b, int32_t tax_group);
int64_t fc4_budget_set_total_monthly_expense(fc4_budget* b, int64_t v);
int64_t fc4_budget_set_total_yearly_expense(fc4_budget* b, int64_t v);
int64_t fc4_budget_set_total_monthly_income(fc4_budget* b, int64_t v);
int64_t fc4_budget_set_total_yearly_income(fc4_budget* b, int64_t v);
bool fc4_budget_show_budget_window(fc4_budget* b);
uint64_t fc4_budget_create_department(fc4_budget* b, uint32_t id, uint32_t group); /* token, 0 on failure */
bool fc4_budget_remove_department(fc4_budget* b, uint32_t id);
uint64_t fc4_budget_get_department(const fc4_budget* b, uint32_t id);
uint64_t fc4_budget_get_department_by_name(const fc4_budget* b, const uint8_t* name, size_t name_len);
size_t fc4_budget_get_all_groups(const fc4_budget* b, fc4_group_callback cb, void* ctx);
bool fc4_budget_set_group_name(fc4_budget* b, uint32_t id, const uint8_t* name, size_t name_len);
/* Department tokens in ascending ID; the caller takes its own references. */
size_t fc4_budget_get_departments_in_group(const fc4_budget* b, uint32_t id, fc4_token_callback cb, void* ctx);
void fc4_budget_needed_funding_changed(fc4_budget* b, const fc4_department* d);
void fc4_budget_funding_percentage_changed(fc4_budget* b, const fc4_department* d);
int64_t fc4_budget_get_funding(fc4_budget* b); /* always 0 [compat] */
float fc4_budget_get_tax_rate(fc4_budget* b, uint32_t tax_group);
bool fc4_budget_set_tax_rate(fc4_budget* b, uint32_t tax_group, float rate);
int64_t fc4_budget_get_weighted_assessed_tax_value(fc4_budget* b, uint32_t tax_group);
int64_t fc4_budget_get_bond_increment(fc4_budget* b);
bool fc4_budget_issue_bond(fc4_budget* b, uint32_t amount);
int64_t fc4_budget_get_total_borrowed(fc4_budget* b);
int64_t fc4_budget_get_current_borrowing_limit(fc4_budget* b);
int64_t fc4_budget_get_current_bond_limit(fc4_budget* b);
int64_t fc4_budget_get_current_max_outstanding_bonds_limit(fc4_budget* b);
int64_t fc4_budget_get_total_monthly_bond_payments(fc4_budget* b);
size_t fc4_budget_get_all_loans(const fc4_budget* b, fc4_loan_callback cb, void* ctx);
int32_t fc4_budget_get_loan_time_in_months(fc4_budget* b);
int64_t fc4_budget_get_monthly_payment_for_loan(fc4_budget* b, int64_t loan);
int64_t fc4_budget_get_full_cost_of_loan(fc4_budget* b, int64_t loan);
bool fc4_budget_get_budget_item_info(const fc4_budget* b, uint64_t holder, fc4_budget_item_callback cb, void* ctx);
bool fc4_budget_get_budget_item_for_purpose(const fc4_budget* b, uint64_t holder, uint32_t purpose,
                                            fc4_budget_item* out);
bool fc4_budget_change_budget_item_line(fc4_budget* b, uint64_t holder, uint32_t index, uint32_t line);
bool fc4_budget_change_budget_item_cost(fc4_budget* b, uint64_t holder, uint32_t index, int64_t cost);
bool fc4_budget_change_budget_item_local_funding(fc4_budget* b, uint64_t holder, uint32_t index, float fraction);
bool fc4_budget_add_budget_item_to_budget(fc4_budget* b, uint64_t holder, uint32_t index);
bool fc4_budget_remove_budget_item_from_budget(fc4_budget* b, uint64_t holder, uint32_t index);
uint64_t fc4_budget_get_line_item_from_budget_item(const fc4_budget* b, const fc4_budget_item* item);
bool fc4_budget_copy_budget_item_properties(fc4_budget* b, uint64_t src, uint64_t dst);

/* Budget simulator: inspection. */
bool fc4_budget_get_state(const fc4_budget* b, fc4_budget_state* out);
bool fc4_budget_get_bond(const fc4_budget* b, size_t index, fc4_bond* out); /* issue order */

/* Departments: cISC4DepartmentBudget (§7), in SDK order. */
uint32_t fc4_department_get_id(fc4_department* d);
bool fc4_department_get_name(const fc4_department* d, fc4_bytes_sink sink, void* ctx);
bool fc4_department_set_name(fc4_department* d, uint32_t ltext_group, uint32_t ltext_instance);
uint32_t fc4_department_get_budget_group(fc4_department* d);
bool fc4_department_set_budget_group(fc4_department* d, uint32_t group);
bool fc4_department_is_fixed_funding(fc4_department* d);
bool fc4_department_set_fixed_funding(fc4_department* d, bool fixed);
float fc4_department_get_funding_percentage(fc4_department* d);
bool fc4_department_set_funding_percentage(fc4_department* d, float q, uint32_t arg);
float fc4_department_get_max_funding_percentage(fc4_department* d);
bool fc4_department_set_max_funding_percentage(fc4_department* d, float q);
int64_t fc4_department_get_ideal_monthly_funding(fc4_department* d);
int64_t fc4_department_get_total_expenses(fc4_department* d);
int64_t fc4_department_get_total_income(fc4_department* d);
uint64_t fc4_department_create_line_item(fc4_department* d, uint32_t id, bool is_local); /* token */
uint64_t fc4_department_create_line_item_for_building_type(fc4_department* d, uint32_t building_type, bool is_local);
bool fc4_department_remove_line_item(fc4_department* d, uint32_t id);
uint64_t fc4_department_get_line_item(const fc4_department* d, uint32_t id);
size_t fc4_department_get_all_line_items(const fc4_department* d, fc4_token_callback cb, void* ctx);
bool fc4_department_add_locally_funded_object(fc4_department* d, uint64_t holder, uint32_t purpose);
bool fc4_department_remove_locally_funded_object(fc4_department* d, uint64_t holder, uint32_t purpose);
bool fc4_department_set_local_funding_percent(fc4_department* d, uint64_t holder, float q, uint32_t purpose);
bool fc4_department_set_local_full_funding(fc4_department* d, uint64_t holder, int64_t cost, uint32_t purpose);
float fc4_department_get_local_funding_percent(fc4_department* d, uint64_t holder, uint32_t purpose);
/* True and *out = the item's local funding fraction if the object is registered and its budget item exists;
 * false otherwise (*out untouched). */
bool fc4_department_find_local_funding_percent(fc4_department* d, uint64_t holder, uint32_t purpose, float* out);
int64_t fc4_department_get_local_full_funding(fc4_department* d, uint64_t holder, uint32_t purpose);
bool fc4_department_is_locally_funded_object(fc4_department* d, uint64_t holder, uint32_t purpose);
bool fc4_department_recalculate(fc4_department* d); /* RecalculateAllLineItemCurrentExpenses */
int64_t fc4_department_get_total_spending(fc4_department* d);
bool fc4_department_set_total_spending(fc4_department* d, int64_t v);
size_t fc4_department_get_locally_funded_items_by_purpose(const fc4_department* d, uint32_t purpose,
                                                          fc4_token_callback cb, void* ctx);
bool fc4_department_has_locally_funded_items_by_purpose(fc4_department* d, uint32_t purpose);

/* Line items: cISC4LineItem (§8), in SDK order. */
uint32_t fc4_line_item_get_id(fc4_line_item* l);
uint32_t fc4_line_item_get_type(fc4_line_item* l);
bool fc4_line_item_set_type(fc4_line_item* l, uint32_t type);
bool fc4_line_item_get_name(const fc4_line_item* l, fc4_bytes_sink sink, void* ctx);
bool fc4_line_item_set_name(fc4_line_item* l, uint32_t group, uint32_t instance);
int64_t fc4_line_item_get_secondary_info(fc4_line_item* l);
bool fc4_line_item_set_secondary_info(fc4_line_item* l, int64_t v);
int64_t fc4_line_item_get_full_expenses(fc4_line_item* l);
int64_t fc4_line_item_get_current_expenses(fc4_line_item* l);
bool fc4_line_item_set_full_expenses(fc4_line_item* l, int64_t v);
bool fc4_line_item_add_to_full_expenses(fc4_line_item* l, int64_t v);
int64_t fc4_line_item_get_income(fc4_line_item* l);
bool fc4_line_item_set_income(fc4_line_item* l, int64_t v);
bool fc4_line_item_add_to_income(fc4_line_item* l, int64_t v);
uint32_t fc4_line_item_get_display_flags(fc4_line_item* l);
bool fc4_line_item_set_display_flag(fc4_line_item* l, uint32_t flag, bool on);
bool fc4_line_item_is_local(fc4_line_item* l);
uint64_t fc4_line_item_get_owner(const fc4_line_item* l); /* department token or 0 */

/* ---- land value (docs/specs/landvalue.md, docs/specs/simgrid.md) --------------
 * One core object fronted by the GZCOM land value simulator. Its grids are the
 * GAME's own cSC4SimGrid objects (simgrid.md §6.3: other subsystems read their
 * memory directly), created by the host through GZCOM and held by the core as
 * TOKENS (interface pointers). The core owns one reference per grid token it
 * holds: the one create_grid returns, or the one a stream read hands over; it
 * gives it back with host->release. Grid values cross the ABI as flat arrays,
 * x outer, z inner (count_x * count_z values of the grid's geometry). */
typedef struct fc4_landvalue fc4_landvalue;

typedef struct fc4_grid_geometry {
    int32_t tract_size, shift, count_x, count_z;
    float width_x, width_z, inv_width_x, inv_width_z;
} fc4_grid_geometry;

#define FC4_GRID_U8  0u /* cSC4SimGrid<uint8_t>, CLSID 0x49B9E602 */
#define FC4_GRID_F32 1u /* cSC4SimGrid<float>,   CLSID 0x49B9E60A */

/* What land value needs from the game (landvalue.md §12). Missing callbacks
 * behave as "nothing there" (0, false, no grid). */
typedef struct fc4_landvalue_host {
    uint32_t struct_size; /* sizeof(fc4_landvalue_host) */
    void* user;
    void (*terrain_cells)(void* user, int32_t* x, int32_t* z); /* CellCountX/Z */
    bool (*is_land_vertex)(void* user, uint32_t x, uint32_t z);
    float (*altitude)(void* user, float x, float z);
    float (*min_allowed_height)(void* user);
    float (*max_allowed_height)(void* user);
    uint32_t (*zone_type)(void* user, int32_t x, int32_t z);
    /* Lot developer GetAreaSlope({x1, z1, x2, z2}, true), inclusive cells. */
    float (*area_slope)(void* user, int32_t x1, int32_t z1, int32_t x2, int32_t z2);
    uint64_t (*developer)(void* user, uint32_t developer_type); /* growth developer or 0 */
    uint64_t (*desirability_map)(void* user, uint64_t developer); /* int16 grid (borrowed) or 0 */
    /* AddRef, Init, SetTractSize, SetInstanceID; returns the grid with one
     * reference for the core, or 0. */
    uint64_t (*create_grid)(void* user, uint32_t kind, int32_t tract_size, uint32_t instance_id);
    void (*grid_geometry)(void* user, uint64_t grid, fc4_grid_geometry* out);
    void (*grid_set_tract_size)(void* user, uint64_t grid, int32_t tract_size);
    void (*grid_read_u8)(void* user, uint64_t grid, uint8_t* out, size_t count);
    void (*grid_read_i16)(void* user, uint64_t grid, int16_t* out, size_t count);
    void (*grid_write_u8)(void* user, uint64_t grid, const uint8_t* values, size_t count);
    void (*grid_write_f32)(void* user, uint64_t grid, const float* values, size_t count);
    void (*grid_shutdown)(void* user, uint64_t grid);
    void (*release)(void* user, uint64_t grid);
    void (*set_history_queue_length)(void* user, uint32_t series, uint32_t length);
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*post)(void* user, uint32_t msg_type); /* type-only MessagePost */
} fc4_landvalue_host;

/* The tuning exemplar (§8). A NULL array means the property is absent (a
 * present empty array needs a non-NULL pointer). */
typedef struct fc4_landvalue_tuning {
    uint32_t struct_size; /* sizeof(fc4_landvalue_tuning) */
    uint32_t has_tract_size;
    int64_t tract_size;
    const int64_t* intrinsic; size_t intrinsic_count;
    const int64_t* wealth; size_t wealth_count;
    const int64_t* water; size_t water_count;
    const float* altitude_curve; size_t altitude_curve_count;
    const uint32_t* developer_ids; size_t developer_id_count;
    const float* developer_weights; size_t developer_weight_count;
    const float* desirability_curve; size_t desirability_curve_count;
} fc4_landvalue_tuning;

/* Diagnostics. */
typedef struct fc4_landvalue_state {
    uint32_t struct_size;
    uint8_t initialized, dirty, ignore_zoning, average_land_value;
    uint32_t pipeline_state; /* 0..8 running, 9 finished */
    uint32_t developer_count;
    int32_t sum, count;
    float min_alt, max_alt, mean_alt, mode_alt, pivot, low_range, high_range;
    int32_t top_index;
} fc4_landvalue_state;

/* The host table is copied. NULL if it is missing or too small. */
fc4_landvalue* fc4_landvalue_create(const fc4_landvalue_host* host);
/* Releases grid references still held (a loaded object never initialised). */
void fc4_landvalue_destroy(fc4_landvalue* lv);
/* Init / Shutdown, core part (§4.1 steps 2-7 and 10; §4.2 grids and curves).
 * tuning NULL = no exemplar (current values stay). */
bool fc4_landvalue_init(fc4_landvalue* lv, const fc4_landvalue_tuning* tuning);
bool fc4_landvalue_shutdown(fc4_landvalue* lv);
/* DoMessage (§5): always true. data3 is the SimIdle budget in ms. */
bool fc4_landvalue_handle_message(fc4_landvalue* lv, uint32_t msg_type, int64_t data3);
/* Step(budget) (§6): completes at most one pipeline state. */
void fc4_landvalue_step(fc4_landvalue* lv, uint32_t budget_ms);
/* Grid tokens, borrowed (no reference): 0 intrinsic, 1 total, 2 slope. */
uint64_t fc4_landvalue_grid(const fc4_landvalue* lv, uint32_t which);
/* GetLandValueType for a land value byte (the caller reads the total grid). */
uint32_t fc4_landvalue_type_of(const fc4_landvalue* lv, uint8_t value);
uint8_t fc4_landvalue_average(const fc4_landvalue* lv);
float fc4_landvalue_mode_altitude(const fc4_landvalue* lv);
bool fc4_landvalue_get_state(const fc4_landvalue* lv, fc4_landvalue_state* out);
/* Persistence (§10). Grid references go out as fc4_ref {token, class}; on read
 * they are requested with IID 0xC9BD939A and arrive with one reference each. */
bool fc4_landvalue_write(const fc4_landvalue* lv, const fc4_ostream* out);
bool fc4_landvalue_read(fc4_landvalue* lv, const fc4_istream* in);

/* ---- property holders ---------------------------------------------------------
 * A plain property holder the host fills with the properties of a game exemplar
 * (a subsystem's tuning) before handing it to the core, which copies it. A
 * single value is stored as a scalar, several (or none) as an array; the core
 * converts numbers between types when it reads them. */
typedef struct fc4_props fc4_props;

fc4_props* fc4_props_create(void);
void fc4_props_destroy(fc4_props* p);
/* Set (replace) property `id`. `values` may be NULL when `count` is 0. */
bool fc4_props_set_u32s(fc4_props* p, uint32_t id, const uint32_t* values, size_t count);
bool fc4_props_set_i32s(fc4_props* p, uint32_t id, const int32_t* values, size_t count);
bool fc4_props_set_i64s(fc4_props* p, uint32_t id, const int64_t* values, size_t count);
bool fc4_props_set_f32s(fc4_props* p, uint32_t id, const float* values, size_t count);
bool fc4_props_set_bools(fc4_props* p, uint32_t id, const bool* values, size_t count);
/* A string property (bytes, no terminator). */
bool fc4_props_set_string(fc4_props* p, uint32_t id, const uint8_t* bytes, size_t len);
size_t fc4_props_count(const fc4_props* p);
/* Set property `id` exactly as a game variant holds it: mem_type = the
 * cIGZVariant type without the array bit (0x01 bool .. 0x0E RZChar); data =
 * `count` elements in that type's layout (bool: 1 byte). A scalar
 * (is_array false) needs count 1. False for other types or a bad count. */
bool fc4_props_set_typed(fc4_props* p, uint32_t id, uint16_t mem_type, bool is_array, const void* data,
                         size_t count);

/* ---- weather (docs/specs/weather.md) ------------------------------------------
 * The weather simulator's core. The shim owns the GZCOM object (agents,
 * PostCityInit subscription, city/terrain references) and keeps the
 * SC4Vector-compatible mirror of the moisture grid (§11.2). */
typedef struct fc4_weather fc4_weather;

typedef struct fc4_weather_host {
    uint32_t struct_size; /* sizeof(fc4_weather_host) */
    void* user;
    /* City (§2, §4): SizeX/Z and CellWidthX/Z in world units, GetWorldHemisphere,
     * and the calendar month (1..12) of the simulation date. */
    void (*city_size)(void* user, float* x, float* z);
    void (*city_cell_width)(void* user, float* x, float* z);
    int32_t (*world_hemisphere)(void* user);
    int32_t (*sim_month)(void* user);
    /* cISC4Region::GetWaterPrefs; false = no region (defaults 60, 100). */
    bool (*water_prefs)(void* user, uint8_t* lo, uint8_t* hi);
    /* Terrain (vertex/cell indices unless named world). */
    bool (*terrain_available)(void* user);
    float (*altitude_at_vertex)(void* user, int32_t x, int32_t z);
    void (*vertex_normal)(void* user, int32_t x, int32_t z, float* xyz /* 3 floats */);
    bool (*cell_has_water)(void* user, int32_t x, int32_t z);
    float (*altitude)(void* user, float world_x, float world_z);
    float (*sea_level)(void* user);
    /* Posts a message with void1 = the weather simulator (IGZUnknown) and data2. */
    void (*post)(void* user, uint32_t msg_type, uint32_t data2);
} fc4_weather_host;

typedef struct fc4_weather_state {
    uint32_t struct_size;
    uint8_t initialized, loaded_from_save, post_init_done, pad;
    int32_t count_x, count_z, last_season, cells_per_week, wind_month;
    uint32_t water_rect_count;
    float ambient_temp, ambient_moisture, wind_speed, wind_dir_x, wind_dir_z, target_wind_speed;
} fc4_weather_state;

/* The host table is copied. The generator is seeded as Seed(0xFFFFFFFF) with
 * these clock values (§3). NULL if the table is missing or too small. */
fc4_weather* fc4_weather_create(const fc4_weather_host* host, uint32_t wall_clock_seed, uint32_t process_clock);
void fc4_weather_destroy(fc4_weather* w);
/* Init (§8.1, core part); tuning = the tuning exemplar's properties (copied),
 * NULL = no exemplar. Shutdown (§8.2, core part). */
bool fc4_weather_init(fc4_weather* w, const fc4_props* tuning);
bool fc4_weather_shutdown(fc4_weather* w);
bool fc4_weather_do_post_city_init(fc4_weather* w);
/* DoMessage (§5): SimNewDay (data1 = Julian day), SimNewWeek, SimNewMonth
 * (data3 = month), PostCityInit. Always true. */
bool fc4_weather_handle_message(fc4_weather* w, uint32_t msg_type, uint32_t data1, uint32_t data3);
/* cISC4WeatherSimulator methods (§4); axis 0 = x, 1 = z. */
float fc4_weather_cell_width(const fc4_weather* w, uint32_t axis);
uint32_t fc4_weather_cell_count(const fc4_weather* w, uint32_t axis);
int32_t fc4_weather_position_to_cell(const fc4_weather* w, float x, float z, int32_t* cx, int32_t* cz);
void fc4_weather_cell_to_position(const fc4_weather* w, int32_t cx, int32_t cz, float* x, float* z);
void fc4_weather_cell_to_standard_city_cell(const fc4_weather* w, int32_t cx, int32_t cz, int32_t* sx, int32_t* sz);
void fc4_weather_standard_city_cell_to_cell(const fc4_weather* w, int32_t sx, int32_t sz, int32_t* cx, int32_t* cz);
int32_t fc4_weather_season(const fc4_weather* w);
int32_t fc4_weather_season_month(const fc4_weather* w);
void fc4_weather_update_moisture(fc4_weather* w, int32_t x0, int32_t z0, int32_t x1, int32_t z1);
uint8_t fc4_weather_moisture_value(const fc4_weather* w, float x, float z);
uint8_t fc4_weather_moisture_type(const fc4_weather* w, float x, float z);
void fc4_weather_set_manual_moisture_addition(fc4_weather* w, float x0, float z0, float x1, float z1, float v);
float fc4_weather_humidity(const fc4_weather* w, float x, float z);
/* GetAmbientWind: returns the speed; v (2 floats, may be NULL) = direction x speed. */
float fc4_weather_ambient_wind(const fc4_weather* w, float* v);
/* Copies the moisture grid x-major (out[x * count_z + z], at most `capacity`
 * bytes; out may be NULL) and reports its shape. Returns the cell count. */
size_t fc4_weather_moisture_grid(const fc4_weather* w, uint8_t* out, size_t capacity, int32_t* count_x,
                                 int32_t* count_z);
bool fc4_weather_get_state(const fc4_weather* w, fc4_weather_state* out);
/* Persistence (§10). */
bool fc4_weather_write(const fc4_weather* w, const fc4_ostream* out);
bool fc4_weather_read(fc4_weather* w, const fc4_istream* in);

/* ---- ordinances (docs/specs/ordinance.md) --------------------------------------
 * The ordinance simulator and the standard ordinance classes (§3.6). The shim
 * owns the GZCOM objects: the simulator (agent, messages) and one object per
 * ordinance, each owning an fc4_ordinance. Ordinance tokens are
 * cISC4Ordinance*; the simulator's token is its IGZUnknown. Foreign
 * cISC4Ordinance objects (DLL plugins) are called back through ordinance_call,
 * ordinance_text and ordinance_misc_property. */
typedef struct fc4_ordinance_sim fc4_ordinance_sim;
typedef struct fc4_ordinance fc4_ordinance;

/* cISC4Ordinance operations (fc4_ordinance_call, the host's ordinance_call),
 * in slot order without GetName / GetDescription / GetMiscProperties.
 * Booleans are 0 / 1; CHANCE_AVAILABILITY and MONTHLY_INCOME_FACTOR return
 * the float's bits; SET_* / FORCE_* take the value in `arg`. */
enum {
    FC4_ORD_INIT = 0,
    FC4_ORD_SHUTDOWN = 1,
    FC4_ORD_CURRENT_MONTHLY_INCOME = 2,
    FC4_ORD_ID = 3,
    FC4_ORD_YEAR_FIRST_AVAILABLE = 4,
    FC4_ORD_CHANCE_AVAILABILITY = 5,
    FC4_ORD_ENACTMENT_INCOME = 6,
    FC4_ORD_RETRACMENT_INCOME = 7,
    FC4_ORD_MONTHLY_CONSTANT_INCOME = 8,
    FC4_ORD_MONTHLY_INCOME_FACTOR = 9,
    FC4_ORD_ADVISOR_ID = 10,
    FC4_ORD_IS_AVAILABLE = 11,
    FC4_ORD_IS_ON = 12,
    FC4_ORD_IS_ENABLED = 13,
    FC4_ORD_MONTHLY_ADJUSTED_INCOME = 14,
    FC4_ORD_CHECK_CONDITIONS = 15,
    FC4_ORD_IS_INCOME_ORDINANCE = 16,
    FC4_ORD_SIMULATE = 17,
    FC4_ORD_SET_AVAILABLE = 18,
    FC4_ORD_SET_ON = 19,
    FC4_ORD_SET_ENABLED = 20,
    FC4_ORD_FORCE_AVAILABLE = 21,
    FC4_ORD_FORCE_ON = 22,
    FC4_ORD_FORCE_MONTHLY_ADJUSTED_INCOME = 23
};

typedef void (*fc4_key_visitor)(void* ctx, uint32_t type_id, uint32_t group, uint32_t instance);

typedef struct fc4_ordinance_host {
    uint32_t struct_size; /* sizeof(fc4_ordinance_host) */
    void* user;
    /* Resources (§8.1, §3.2). for_each_exemplar_key visits exemplar keys (the
     * host may skip exemplars whose type property 0x10 is not 14; the core checks
     * again). exemplar returns a NEW fc4_props with all of the exemplar's
     * properties (the core destroys it), NULL if missing. ltext delivers a
     * localized LTEXT string; false if not found. */
    void (*for_each_exemplar_key)(void* user, fc4_key_visitor visit, void* ctx);
    fc4_props* (*exemplar)(void* user, uint32_t type_id, uint32_t group, uint32_t instance);
    bool (*ltext)(void* user, uint32_t type_id, uint32_t group, uint32_t instance, fc4_bytes_sink sink, void* ctx);
    /* Objects. create_ordinance: a new standard ordinance of `class_id` with one
     * reference owned by the core (its token in *token), NULL if none.
     * foreign_ordinance: the cISC4Ordinance of an object read from a save
     * (token = its IGZUnknown with one reference): on success returns the
     * ordinance token, carrying one reference, and has dropped the input
     * reference; 0 = not an ordinance (the input reference is untouched and the
     * core releases it). */
    fc4_ordinance* (*create_ordinance)(void* user, uint32_t class_id, uint64_t* token);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    uint64_t (*foreign_ordinance)(void* user, uint64_t token);
    int64_t (*ordinance_call)(void* user, uint64_t token, uint32_t op, int64_t arg);
    /* which 0 = GetName, 1 = GetDescription. */
    bool (*ordinance_text)(void* user, uint64_t token, uint32_t which, fc4_bytes_sink sink, void* ctx);
    /* Copies property `prop` of GetMiscProperties() into `out`; false if absent. */
    bool (*ordinance_misc_property)(void* user, uint64_t token, uint32_t prop, fc4_props* out);
    /* Posts a message with void1 = the ordinance simulator (§7.1). */
    void (*post)(void* user, uint32_t msg_type, uint32_t data2, uint32_t data3);
    /* City (§3.4, §3.6). sim_year: false if there is no simulator; the expense
     * lookups: false if the department / line item is missing. */
    bool (*sim_year)(void* user, uint32_t* year);
    int32_t (*residential_population)(void* user);
    bool (*has_budget_simulator)(void* user);
    bool (*department_total_expenses)(void* user, uint32_t dept, int64_t* out);
    bool (*line_item_full_expenses)(void* user, uint32_t dept, uint32_t line, int64_t* out);
    uint32_t (*landfill_capacity)(void* user);
    int32_t (*power_plant_count)(void* user, int32_t power_type);
    uint32_t (*police_station_count)(void* user);
    /* The fraction behind GetRecyclingCenterEffect's pointer (§3.6). */
    float (*recycling_center_effect)(void* user);
    /* LegalizeGambling's SetOn(false) side effects (§3.6). */
    void (*lots_with_building_type)(void* user, uint32_t building_type, fc4_token_callback cb, void* ctx);
    void (*demolish_lots)(void* user, const uint64_t* lots, size_t count);
    void (*hide_conditional_building)(void* user, uint32_t building_type);
    void (*remove_view_input_control)(void* user, uint32_t control_id);
} fc4_ordinance_host;

typedef struct fc4_ordinance_sim_state {
    uint32_t struct_size;
    uint8_t initialized, loaded_from_save;
    uint16_t pad;
    uint32_t count; /* map entries */
    uint32_t pad2;
    int64_t monthly_expense, monthly_income, pending;
} fc4_ordinance_sim_state;

/* The simulator (§4). Host tables are copied; NULL if missing or too small. */
fc4_ordinance_sim* fc4_ordinance_sim_create(const fc4_ordinance_host* host, uint64_t token);
/* Releases every map entry (§8.2: ordinances live until the simulator dies). */
void fc4_ordinance_sim_destroy(fc4_ordinance_sim* s);
/* Init (§8.1, core part: loads the standard ordinances unless loaded from a
 * save; after a load, re-keys the entries by GetID() [choice]) and Shutdown
 * (§8.2, core part: the map is kept). */
bool fc4_ordinance_sim_init(fc4_ordinance_sim* s);
bool fc4_ordinance_sim_shutdown(fc4_ordinance_sim* s);
/* AddOrdinance / RemoveOrdinance (§4.2): token = the cISC4Ordinance*, o = its
 * fc4_ordinance when it is one of ours, NULL for a foreign one. */
bool fc4_ordinance_sim_add(fc4_ordinance_sim* s, uint64_t token, const fc4_ordinance* o);
bool fc4_ordinance_sim_remove(fc4_ordinance_sim* s, uint64_t token, const fc4_ordinance* o);
/* which: 0 IsOrdinanceAvailable, 1 IsOrdinanceOn, 2 IsOrdinanceAvailableButOff,
 * 3 IsOrdinanceEnabled. */
bool fc4_ordinance_sim_query(const fc4_ordinance_sim* s, uint32_t which, uint32_t id);
/* which: 0 SetOrdinanceAvailable (§4.3), 1 SetOrdinanceOn (§4.4), 2 SetOrdinanceEnabled. */
bool fc4_ordinance_sim_set(fc4_ordinance_sim* s, uint32_t which, uint32_t id, bool value);
/* which: 0 GetOrdinanceAvailableArray, 1 GetOrdinanceOnArray, 2 GetOrdinanceIDArray:
 * IDs in key order while fewer than *n were written (out NULL = capacity 0);
 * *n := the number written; returns the total. */
uint32_t fc4_ordinance_sim_array(const fc4_ordinance_sim* s, uint32_t which, uint32_t* out, uint32_t* n);
/* which: 0 GetOrdinanceAvailableCount, 1 GetOrdinanceOnCount. */
uint32_t fc4_ordinance_sim_count(const fc4_ordinance_sim* s, uint32_t which);
/* GetOrdinanceByID / ByName: the entry's token (no reference added), 0 if none. */
uint64_t fc4_ordinance_sim_by_id(const fc4_ordinance_sim* s, uint32_t id);
uint64_t fc4_ordinance_sim_by_name(const fc4_ordinance_sim* s, const uint8_t* name, size_t len);
uint32_t fc4_ordinance_sim_id_by_name(const fc4_ordinance_sim* s, const uint8_t* name, size_t len);
/* which: 0 GetOrdinanceMonthlyExpense, 1 GetOrdinanceMonthlyIncome. */
int64_t fc4_ordinance_sim_money(const fc4_ordinance_sim* s, uint32_t which);
/* §6.1: the offset is a 32-bit sum, sign-extended. */
int64_t fc4_ordinance_sim_cumulative_offset(const fc4_ordinance_sim* s, uint32_t prop, uint32_t index);
float fc4_ordinance_sim_cumulative_factor(const fc4_ordinance_sim* s, uint32_t prop, uint32_t index);
/* DoMessage (§5): SimNewMonth, SimBegin. Always true. */
bool fc4_ordinance_sim_handle_message(fc4_ordinance_sim* s, uint32_t msg_type);
bool fc4_ordinance_sim_get_state(const fc4_ordinance_sim* s, fc4_ordinance_sim_state* out);
/* Persistence (§9.1). Ordinances go out as references by token; on read, ours
 * arrive with their core handle (fc4_ref.core_handle = the fc4_ordinance*),
 * others with one reference offered to foreign_ordinance. */
bool fc4_ordinance_sim_write(const fc4_ordinance_sim* s, const fc4_ostream* out);
bool fc4_ordinance_sim_read(fc4_ordinance_sim* s, const fc4_istream* in);

/* A standard ordinance (§3) of class `class_id` for the GZCOM object `token`;
 * NULL for a class that is not one of §3.6. */
fc4_ordinance* fc4_ordinance_create(const fc4_ordinance_host* host, uint32_t class_id, uint64_t token);
void fc4_ordinance_destroy(fc4_ordinance* o);
uint32_t fc4_ordinance_class_id(const fc4_ordinance* o);
/* SetKey (§3.5, concrete class) and the key it holds (false if none). */
void fc4_ordinance_set_key(fc4_ordinance* o, uint32_t type_id, uint32_t group, uint32_t instance);
bool fc4_ordinance_get_key(const fc4_ordinance* o, uint32_t* type_id, uint32_t* group, uint32_t* instance);
/* One cISC4Ordinance operation (FC4_ORD_*). */
int64_t fc4_ordinance_call(fc4_ordinance* o, uint32_t op, int64_t arg);
/* which 0 = GetName, 1 = GetDescription: the bytes go to `sink`. */
bool fc4_ordinance_text(const fc4_ordinance* o, uint32_t which, fc4_bytes_sink sink, void* ctx);
/* One property of GetMiscProperties() (own, then the parent exemplar), copied
 * into `out`; false if absent. */
bool fc4_ordinance_misc_property(const fc4_ordinance* o, uint32_t prop, fc4_props* out);
/* Persistence (§9.2). Read accepts any version. */
bool fc4_ordinance_write(const fc4_ordinance* o, const fc4_ostream* out);
bool fc4_ordinance_read(fc4_ordinance* o, const fc4_istream* in);

/* ---- demand (docs/specs/demand.md) ---------------------------------------------
 * The demand simulator and demand nodes. The shim owns the GZCOM objects: the
 * simulator (agents, subscriptions, the services it reads) and one object per
 * node, each owning an fc4_demand_node. Node tokens are cISC4Demand*; lots,
 * connections and regional cities cross as their interface pointers.
 * Re-entrancy: callbacks must not call back into the demand simulator. */
typedef struct fc4_demand_sim fc4_demand_sim;
typedef struct fc4_demand_node fc4_demand_node;

/* Node values (fc4_demand_node_get / _set / _add). */
enum {
    FC4_DEMAND_SUPPLY = 0,
    FC4_DEMAND_DEMAND = 1,
    FC4_DEMAND_NEW_SUPPLY = 2,
    FC4_DEMAND_NEW_DEMAND = 3,
    FC4_DEMAND_ACTIVE_DEMAND = 4,
    FC4_DEMAND_ECONOMY_MODIFIER = 5,
    FC4_DEMAND_ACTIVE_MAX = 6,
    FC4_DEMAND_ACTIVE_MIN = 7,
    FC4_DEMAND_TAX_MODIFIER = 8,
    FC4_DEMAND_DEMAND_CAP = 9
};

typedef struct fc4_demand_connection {
    uint64_t token;     /* the cISC4NeighborConnection* */
    uint64_t city;      /* GetCityOnOtherSideOfConnection (0 = none) */
    uint32_t conn_type; /* GetType */
    uint32_t reserved;
} fc4_demand_connection;

typedef struct fc4_demand_growth_developer {
    uint32_t sub_type;    /* GetSubType (the developer type) */
    uint32_t kind;        /* GetType: 1 R, 2 C, 3 I */
    uint32_t has_source;  /* 0: GetSourceDemand returned no node */
    float supply, demand; /* the source node's committed values */
    uint32_t reserved;
} fc4_demand_growth_developer;

typedef struct fc4_demand_sim_state {
    uint32_t struct_size;
    uint8_t initialized, loaded_from_save, census_state, pad;
    uint32_t node_count, link_count;
    int32_t cursor_x, cursor_z;
    float co_economy_bonus, i_economy_bonus, commercial_relief, industrial_relief;
    uint32_t export_city_count;
} fc4_demand_sim_state;

typedef void (*fc4_capacity_visitor)(void* ctx, uint32_t map_index, uint32_t dev_type, uint16_t capacity);
typedef void (*fc4_u32_callback)(void* ctx, uint32_t value);
typedef void (*fc4_connection_callback)(void* ctx, const fc4_demand_connection* c);
typedef void (*fc4_growth_developer_callback)(void* ctx, const fc4_demand_growth_developer* g);
typedef void (*fc4_population_visitor)(void* ctx, uint32_t key, int32_t value);

/* The DemandHost trait of fc4_core::demand as callbacks (the exact calls: §14). */
typedef struct fc4_demand_host {
    uint32_t struct_size; /* sizeof(fc4_demand_host) */
    void* user;
    /* Resources (§4.2): every exemplar key (type 0x6534284A, any group; the host
     * may skip exemplars whose ExemplarType is not 3, the core checks again);
     * exemplar returns a NEW fc4_props (the core destroys it), NULL if missing. */
    void (*for_each_exemplar_key)(void* user, fc4_key_visitor visit, void* ctx);
    fc4_props* (*exemplar)(void* user, uint32_t type_id, uint32_t group, uint32_t instance);
    void (*warn)(void* user, const uint8_t* text, size_t len);
    /* Nodes. create_node: a new node (class 0x8990C12D) with one reference owned
     * by the core, its token in *token. post_active_demand: ActiveDemandChanged
     * (0x426840A0) from `node` with data1 = key and data2 (§3.4). */
    fc4_demand_node* (*create_node)(void* user, uint64_t* token);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    void (*post_active_demand)(void* user, uint64_t node, uint32_t key, int32_t data2);
    /* City and clock: CellCountX/Z; today = the simulation date's Julian day;
     * now_ms = a millisecond wall clock (census slicing, §6.2). */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    uint32_t (*today)(void* user);
    uint32_t (*now_ms)(void* user);
    /* Census (§6.3). census_lot: false unless GetLot(x, z, true) exists, has
     * capacity and a building type. building_profile: false = no profile; the
     * capacity list goes to `visit` (vector index, developer type, value). */
    bool (*census_lot)(void* user, int32_t x, int32_t z, uint32_t* building_type, uint32_t* wealth);
    bool (*building_profile)(void* user, uint32_t building_type, uint32_t* purpose, fc4_capacity_visitor visit,
                             void* ctx);
    uint32_t (*developer_type)(void* user, uint32_t purpose, uint32_t wealth);
    int16_t (*desirability)(void* user, uint32_t dev_type, int32_t x, int32_t z);
    uint16_t (*lot_capacity)(void* user, int32_t x, int32_t z, uint32_t dev_type);
    float (*cell_eq)(void* user, int32_t x, int32_t z);
    float (*cell_workforce_percent)(void* user, int32_t x, int32_t z);
    /* Occupancy (§7). lot_excluded: the building has bool 0x27812844 = true. */
    uint32_t (*lot_building_type)(void* user, uint64_t lot);
    bool (*lot_excluded)(void* user, uint64_t lot);
    void (*all_building_types)(void* user, fc4_u32_callback cb, void* ctx);
    fc4_props* (*building_exemplar)(void* user, uint32_t building_type);
    /* Modifiers (§8.1, §8.2). history_max: false = no data in the window. */
    bool (*history_max)(void* user, uint32_t series, uint32_t from_day, uint32_t to_day, float* max);
    float (*ordinance_factor)(void* user, uint32_t prop, uint32_t index);
    float (*nation_economy_modifier)(void* user, uint32_t demand_id);
    /* Cap relief (§8.3). */
    void (*connections)(void* user, fc4_connection_callback cb, void* ctx);
    float (*traffic_arrived)(void* user, uint64_t connection);
    float (*airport_trips)(void* user);
    float (*seaport_trips)(void* user);
    /* Region (§9). Cities are cISC4RegionalCity tokens (borrowed). */
    void (*all_connected_cities)(void* user, fc4_token_callback cb, void* ctx);
    void (*connected_cities)(void* user, fc4_token_callback cb, void* ctx);
    uint64_t (*own_regional_city)(void* user);
    int32_t (*city_population)(void* user, uint64_t city, uint32_t pop_type);
    int32_t (*city_extrapolated_population)(void* user, uint64_t city, uint32_t pop_type);
    int32_t (*city_allowable_extrapolation)(void* user, uint64_t city, uint32_t pop_type);
    float (*city_tax_rate)(void* user, uint64_t city, uint32_t tax_type);
    float (*city_workforce_percentage)(void* user, uint64_t city);
    void (*city_extrapolate_growth)(void* user, uint64_t city, uint32_t pop_type, float amount);
    void (*growth_developers)(void* user, fc4_growth_developer_callback cb, void* ctx);
    uint32_t (*day_of_last_successful_build)(void* user, uint32_t dev_type);
    /* History (§10.4). */
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*set_history_queue_length)(void* user, uint32_t series, uint32_t length);
} fc4_demand_host;

/* The simulator (§5, §13). Host tables are copied; NULL if missing or too small. */
fc4_demand_sim* fc4_demand_sim_create(const fc4_demand_host* host, uint64_t token);
/* Releases every node in the map. */
void fc4_demand_sim_destroy(fc4_demand_sim* s);
/* Init / Shutdown, core part (§13): subscriptions and agents are the shim's. */
bool fc4_demand_sim_init(fc4_demand_sim* s);
bool fc4_demand_sim_shutdown(fc4_demand_sim* s);
/* GetDemand (initializes first if needed): the node's token, no reference
 * added; 0 if none. */
uint64_t fc4_demand_sim_get_demand(fc4_demand_sim* s, uint32_t demand_id, uint32_t demand_index);
void fc4_demand_sim_update_occupant_effects(fc4_demand_sim* s, uint32_t building_type, float old_pct, float new_pct);
/* CalculateJobsPerUnitOfDemand: out = 3 floats (may be NULL). */
void fc4_demand_sim_jobs_per_unit(const fc4_demand_sim* s, uint32_t demand_id, uint32_t demand_index, float* out);
uint32_t fc4_demand_sim_jobs_by_census(const fc4_demand_sim* s, uint32_t census_type);
float fc4_demand_sim_neutral_tax_rate(const fc4_demand_sim* s);
/* GetLocalPopulationSummary: (key, value) in key order; returns the count. */
size_t fc4_demand_sim_local_population_summary(const fc4_demand_sim* s, fc4_population_visitor visit, void* ctx);
/* DoMessage (§10.2) for everything but kMessageStateChanged. Always true. */
bool fc4_demand_sim_handle_message(fc4_demand_sim* s, uint32_t msg_type, uint32_t data1, uint32_t data2,
                                   uint32_t data3);
/* kMessageStateChanged (§7): lot = the message's IGZUnknown; then data1..data4. */
void fc4_demand_sim_state_changed(fc4_demand_sim* s, uint64_t lot, uint32_t old_state, uint32_t new_state,
                                  uint32_t building_type, uint32_t wealth);
void fc4_demand_sim_set_economy_bonuses(fc4_demand_sim* s, float co, float i);
bool fc4_demand_sim_get_state(const fc4_demand_sim* s, fc4_demand_sim_state* out);
/* Persistence (§12.1): nodes go out as references by token; on read, ours
 * arrive with their core handle (fc4_ref.core_handle = the fc4_demand_node*, no
 * reference: the map takes one with add_ref); anything else is released and
 * skipped. */
bool fc4_demand_sim_write(const fc4_demand_sim* s, const fc4_ostream* out);
bool fc4_demand_sim_read(fc4_demand_sim* s, const fc4_istream* in);

/* A demand node (§3) for the GZCOM object `token`; only post_active_demand of
 * the host table is used. */
fc4_demand_node* fc4_demand_node_create(const fc4_demand_host* host, uint64_t token);
void fc4_demand_node_destroy(fc4_demand_node* n);
bool fc4_demand_node_init(fc4_demand_node* n);
bool fc4_demand_node_shutdown(fc4_demand_node* n);
bool fc4_demand_node_simulation_begin(fc4_demand_node* n);
uint32_t fc4_demand_node_id(fc4_demand_node* n);
bool fc4_demand_node_set_id(fc4_demand_node* n, uint32_t id);
uint32_t fc4_demand_node_region_use(fc4_demand_node* n);
bool fc4_demand_node_set_region_use(fc4_demand_node* n, uint32_t v);
/* EndOfCycle: folds the pending values and recalculates (posts); true if a
 * value was pending. */
bool fc4_demand_node_end_of_cycle(fc4_demand_node* n);
/* An FC4_DEMAND_* value. */
float fc4_demand_node_get(const fc4_demand_node* n, uint32_t which);
/* Setters: SUPPLY, DEMAND (committed), ACTIVE_MAX, ACTIVE_MIN, ECONOMY_MODIFIER,
 * TAX_MODIFIER, DEMAND_CAP; false for the others. */
bool fc4_demand_node_set(fc4_demand_node* n, uint32_t which, float v);
/* AddToSupplyValue (SUPPLY) / AddToDemandValue (DEMAND). */
bool fc4_demand_node_add(fc4_demand_node* n, uint32_t which, float v);
/* Persistence (§12.2): versions 0-2 are read. */
bool fc4_demand_node_write(const fc4_demand_node* n, const fc4_ostream* out);
bool fc4_demand_node_read(fc4_demand_node* n, const fc4_istream* in);

/* ---- the game's grids, shared by the grid simulators ----------------------------
 *
 * The game's own cSC4SimGrid<T> objects (simgrid.md §6.4), driven by the core.
 * A grid is a token holding ONE reference (released through `release`). `kind`
 * is the grid class ID of the element type (0x49B9E602 uint8 ... 0x49B9E60B
 * double); values cross as doubles (exact for every type up to 32 bits). */
typedef struct fc4_grid_api {
    uint32_t struct_size; /* sizeof(fc4_grid_api) */
    void* user;
    /* A new grid through GZCOM: Init, SetTractSize(tract_size),
     * SetInstanceID(instance). 0 = failure. */
    uint64_t (*create)(void* user, uint32_t kind, uint32_t instance, int32_t tract_size);
    uint32_t (*class_of)(void* user, uint64_t grid); /* GetGZCLSID, 0 if unknown */
    void (*geometry)(void* user, uint64_t grid, fc4_grid_geometry* out);
    double (*get)(void* user, uint64_t grid, uint32_t kind, int32_t tx, int32_t tz);          /* GetTractValue */
    void (*set)(void* user, uint64_t grid, uint32_t kind, int32_t tx, int32_t tz, double v);  /* SetTractValue */
    void (*fill)(void* user, uint64_t grid, uint32_t kind, double v);                         /* SetTractValues */
    void (*shutdown)(void* user, uint64_t grid);
    void (*release)(void* user, uint64_t grid);
} fc4_grid_api;

/* An inclusive cell rectangle. */
typedef struct fc4_cell_rect {
    int32_t x1, z1, x2, z2;
} fc4_cell_rect;

/* ---- power (docs/specs/power.md) -------------------------------------------------
 *
 * Tokens: occupants (also poles, lines and plants: the cISC4Occupant
 * identity), lots, deals, connections, cities, departments, line items; 0 is
 * null. A message delivered while another call into the same object is
 * running is queued and handled when that call returns. */
typedef struct fc4_power fc4_power;

typedef struct fc4_power_post {
    uint32_t msg_type;
    uint32_t data[4];
    uint32_t reserved;
    uint64_t unknown; /* the IGZUnknown, 0 = none */
} fc4_power_post;

typedef struct fc4_power_pole {
    uint32_t ignored; /* pole slot 13 */
    int32_t cell_x, cell_z;
} fc4_power_pole;

typedef struct fc4_power_line {
    uint32_t preview;
    int32_t cell_x, cell_z;
} fc4_power_line;

typedef struct fc4_power_deal {
    uint64_t token;
    uint64_t city; /* GetCityOnOtherSideOfConnection */
    int32_t amount;
    uint32_t reserved;
} fc4_power_deal;
typedef void (*fc4_power_deal_callback)(void* ctx, const fc4_power_deal* deal);

typedef struct fc4_power_budget_item {
    uint32_t department;
    uint32_t line;
    int64_t cost;
    uint32_t index;
    float local_funding; /* a fraction */
} fc4_power_budget_item;

typedef struct fc4_power_message {
    uint32_t msg_type;
    uint32_t data1, data2, data3, data4;
    uint32_t reserved;
    uint64_t void1;     /* the IGZUnknown as a token, 0 = null */
    fc4_cell_rect rect; /* the four int32 at void2 (zoning changed) */
} fc4_power_message;

typedef struct fc4_power_found {
    uint32_t has_lowest_funded;
    float lowest_funding;
    uint64_t lowest_funded; /* one reference for the caller */
    uint32_t has_most_worn;
    int32_t wear_percent;
    uint64_t most_worn; /* one reference for the caller */
} fc4_power_found;

typedef struct fc4_power_state {
    uint32_t struct_size;
    uint8_t initialized, running, enabled, recompute;
    uint32_t state;
    int32_t plant_count;
    uint32_t pole_count, consumers, powered, unpowered, requirement;
    uint32_t last_production, last_consumption, last_import, last_export, last_demand;
    float deficit;
    uint32_t reentrant_calls;
    int64_t cost_base;
    uint64_t department;
    uint64_t grid;
} fc4_power_state;

/* What the power simulator needs from the game: one callback per PowerHost
 * method (engine/core/src/power/mod.rs, power.md §11.1). Missing callbacks
 * behave as "nothing there" (0, false, no object). */
typedef struct fc4_power_host {
    uint32_t struct_size; /* sizeof(fc4_power_host) */
    void* user;
    fc4_grid_api grids;
    /* city, tuning, ordinance, simulator */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    void (*cell_widths)(void* user, float* x, float* z);
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL if missing */
    float (*cumulative_factor)(void* user, uint32_t id, uint32_t index);
    uint32_t (*sim_time)(void* user);
    void (*acquire_services)(void* user);
    void (*release_services)(void* user);
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t name_len, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, const uint32_t* ids, size_t count);
    void (*unsubscribe)(void* user, const uint32_t* ids, size_t count);
    void (*post)(void* user, const fc4_power_post* msg);
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*set_history_queue_length)(void* user, uint32_t series, uint32_t length);
    /* references */
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* A reference read from a save (QI'd to iid, one reference) to the object's
     * canonical token (one reference); 0 = dropped. */
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
    /* occupants */
    uint32_t (*occupant_type)(void* user, uint64_t occ);
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    void (*set_occupant_property)(void* user, uint64_t occ, uint32_t id, uint16_t mem_type, bool is_array,
                                  const void* data, size_t count);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    void (*occupant_bounding_box)(void* user, uint64_t occ, float* min_xyz, float* max_xyz);
    void (*occupant_bounding_cells)(void* user, uint64_t occ, fc4_cell_rect* out);
    bool (*occupant_in_manager)(void* user, uint64_t occ);
    bool (*building_type)(void* user, uint64_t occ, uint32_t* out);
    bool (*power_pole)(void* user, uint64_t occ, fc4_power_pole* out);
    void (*break_all_connections)(void* user, uint64_t pole, bool flag);
    bool (*pole_is_distressed)(void* user, uint64_t pole);
    void (*set_pole_distressed)(void* user, uint64_t pole, bool on);
    bool (*power_line)(void* user, uint64_t occ, fc4_power_line* out);
    bool (*line_connected_poles)(void* user, uint64_t line, uint64_t* first, uint64_t* second);
    void (*remove_pole_connection)(void* user, uint64_t first, uint64_t second); /* first.RemoveConnection(second, 0) */
    bool (*network_occupant_cell)(void* user, uint64_t occ, int32_t* x, int32_t* z);
    bool (*has_network_flag)(void* user, uint64_t occ, uint32_t flag);
    uint32_t (*network_flags)(void* user, uint64_t occ);
    void (*occupants_in_cells)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    void (*remove_occupant)(void* user, uint64_t occ);
    /* zones, tools */
    uint32_t (*zone_type)(void* user, int32_t x, int32_t z);
    void (*power_line_tool_zone_change)(void* user, const fc4_cell_rect* rect, uint32_t data3);
    /* lots */
    uint64_t (*lot_at)(void* user, int32_t x, int32_t z);
    uint64_t (*occupant_lot)(void* user, uint64_t occ);
    uint64_t (*as_lot)(void* user, uint64_t token);
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    uint32_t (*lot_state)(void* user, uint64_t lot);
    bool (*lot_needs_power)(void* user, uint64_t lot);
    bool (*lot_has_power)(void* user, uint64_t lot);
    void (*set_lot_has_power)(void* user, uint64_t lot, bool on);
    void (*set_lot_condition)(void* user, uint64_t lot, uint32_t condition);
    void (*demolish_lots)(void* user, const uint64_t* lots, size_t count, uint32_t flags);
    void (*add_radiation)(void* user, const float* xyz, float strength, float radius);
    bool (*current_view_input_control)(void* user, uint32_t* id);
    void (*remove_current_view_input_control)(void* user);
    /* neighbours */
    void (*find_deals)(void* user, uint32_t deal_type, fc4_power_deal_callback cb, void* ctx);
    int32_t (*deal_amount_used)(void* user, uint64_t deal);
    void (*set_deal_amount_used)(void* user, uint64_t deal, int32_t used);
    void (*set_deal_flags)(void* user, uint64_t deal, uint32_t flags, bool on);
    bool (*deal_type)(void* user, uint64_t token, uint32_t* out);
    void (*mark_deals_dirty)(void* user);
    void (*connections_of_type)(void* user, uint32_t type, fc4_token_callback cb, void* ctx);
    void (*connections_to_city)(void* user, uint32_t type, uint64_t city, fc4_token_callback cb, void* ctx);
    uint64_t (*find_unvisited_connection)(void* user, uint32_t type, uint64_t city);
    uint64_t (*connection_on_other_side)(void* user, uint64_t conn);
    void (*connection_cell)(void* user, uint64_t conn, int32_t* x, int32_t* z);
    void (*set_connection_visited)(void* user, uint64_t conn, bool on);
    bool (*connection_is_active)(void* user, uint64_t conn);
    void (*set_connection_active)(void* user, uint64_t conn, bool on);
    /* budget */
    uint64_t (*department)(void* user, uint32_t id);
    uint64_t (*create_department)(void* user, uint32_t id, uint32_t group);
    void (*set_department_name)(void* user, uint64_t dept, uint32_t group, uint32_t instance);
    void (*set_department_max_funding)(void* user, uint64_t dept, float fraction);
    float (*department_funding)(void* user, uint64_t dept);
    int64_t (*budget_funding)(void* user, uint64_t dept);
    uint64_t (*create_line_item)(void* user, uint64_t dept, uint32_t line, bool locally_funded);
    uint64_t (*create_line_item_for_building_type)(void* user, uint64_t dept, uint32_t line, bool flag);
    uint64_t (*line_item)(void* user, uint64_t dept, uint32_t line);
    void (*set_line_item_name)(void* user, uint64_t item, uint32_t group, uint32_t instance);
    void (*set_line_item_display_flag)(void* user, uint64_t item, uint32_t flag, bool on);
    int64_t (*line_item_secondary_info)(void* user, uint64_t item);
    void (*set_line_item_secondary_info)(void* user, uint64_t item, int64_t value);
    void (*set_line_item_full_expenses)(void* user, uint64_t item, int64_t value);
    bool (*budget_item_for_purpose)(void* user, uint64_t occ, uint32_t purpose, fc4_power_budget_item* out);
    void (*change_budget_item_line)(void* user, uint64_t occ, uint32_t index, uint32_t line);
    void (*change_budget_item_cost)(void* user, uint64_t occ, uint32_t index, int64_t cost);
    void (*add_locally_funded_object)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    void (*remove_locally_funded_object)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    bool (*is_locally_funded_object)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    bool (*set_local_funding_percent)(void* user, uint64_t dept, uint64_t occ, float fraction, uint32_t purpose);
    void (*set_local_full_funding)(void* user, uint64_t dept, uint64_t occ, int64_t amount, uint32_t purpose);
} fc4_power_host;

/* The host table is copied; `token` is the simulator's own host identity. NULL
 * on a short table. */
fc4_power* fc4_power_create(const fc4_power_host* host, uint64_t token);
void fc4_power_destroy(fc4_power* p);
bool fc4_power_init(fc4_power* p);                                   /* §4.1 */
bool fc4_power_shutdown(fc4_power* p);                               /* §4.2 */
bool fc4_power_do_message(fc4_power* p, const fc4_power_message* m); /* §4.3, §5: always true */
/* The argument-free numeric slots 9, 11-15, 17, 21, 22, 24, 26-29, 39, 41-44. */
int64_t fc4_power_slot_value(const fc4_power* p, uint32_t slot);
void fc4_power_position_to_cell(const fc4_power* p, float x, float z, int32_t* cx, int32_t* cz); /* 5 */
void fc4_power_cell_to_position(const fc4_power* p, int32_t cx, int32_t cz, float* x, float* z); /* 6 */
int32_t fc4_power_annual_consumption_by_type(const fc4_power* p, int32_t type);                  /* 10 */
int32_t fc4_power_consumption_at(const fc4_power* p, int32_t x, int32_t z);                       /* 16 */
int32_t fc4_power_plant_count_of_type(const fc4_power* p, int32_t type);                           /* 23 */
/* 25: every plant with one reference for the callback (released if cb is NULL). */
size_t fc4_power_plants(const fc4_power* p, fc4_token_callback cb, void* ctx);
bool fc4_power_find_plants(const fc4_power* p, fc4_power_found* out);                          /* 30 */
bool fc4_power_is_occupant_powered(const fc4_power* p, uint64_t occ);                           /* 31 */
bool fc4_power_occupant_consumed(const fc4_power* p, uint64_t occ, uint32_t* out);              /* 32 */
bool fc4_power_occupant_produced(const fc4_power* p, uint64_t occ, uint32_t* produced, uint32_t* used); /* 33 */
bool fc4_power_occupant_max_produced(const fc4_power* p, uint64_t occ, uint32_t* out);          /* 34 */
uint64_t fc4_power_powered_grid(const fc4_power* p); /* 20, 35, 36: borrowed token, 0 = none */
bool fc4_power_is_powered(const fc4_power* p, int32_t x, int32_t z);                             /* 19, 37 */
bool fc4_power_is_wired(const fc4_power* p, int32_t x, int32_t z);                               /* 38 */
float fc4_power_chance_of_strike(const fc4_power* p);                                             /* 40 */
float fc4_power_plant_efficiency(const fc4_power* p, uint64_t occ);                               /* 45 */
float fc4_power_plant_condition(const fc4_power* p, uint64_t occ);                                /* 46 */
float fc4_power_strike_efficiency(const fc4_power* p);                                            /* 47 */
uint64_t fc4_power_department(const fc4_power* p); /* 48 forwards to it; borrowed, 0 = none */
bool fc4_power_generation_cost(const fc4_power* p, uint64_t occ, int64_t* out);                 /* 49 */
bool fc4_power_plant_funding(const fc4_power* p, uint64_t occ, float* out);                     /* 50 */
bool fc4_power_set_plant_funding(const fc4_power* p, uint64_t occ, float fraction);             /* 51 */
bool fc4_power_get_state(const fc4_power* p, fc4_power_state* out);
/* Tooling: the recompute report (one line per cycle, then one per plant and per
 * flood). Collected only while switched on; take delivers the last complete
 * cycle's text (UTF-8, lines separated by '\n') once and returns false when
 * there is none. */
void fc4_power_set_report(const fc4_power* p, bool on);
bool fc4_power_take_report(const fc4_power* p, fc4_bytes_sink sink, void* ctx);
bool fc4_power_write(const fc4_power* p, const fc4_ostream* out); /* §9 */
bool fc4_power_read(fc4_power* p, const fc4_istream* in);         /* §9: false only for version > 6 */

/* ---- plumbing (docs/specs/plumbing.md) ---------------------------------------
 *
 * Tokens: occupants (also pipes and water producers: the cISC4Occupant
 * identity), lots, deals, connections, cities, departments, line items; 0 is
 * null. Both grids are the game's own cSC4SimGrid<uint8> (fc4_grid_api).
 * A message delivered while another call into the same object is running is
 * queued and handled when that call returns. */
typedef struct fc4_plumbing fc4_plumbing;

/* GetBudgetItemForPurpose (slot 51) as the budget reports it. */
typedef struct fc4_plumbing_budget_item {
    uint32_t department;
    uint32_t line;
    int64_t cost;
    uint32_t index;
    uint32_t reserved;
} fc4_plumbing_budget_item;

/* A message for fc4_plumbing_do_message (§5.1). `occupant` is GetVoid1() as a
 * cISC4Occupant (Insert / RemoveOccupant); `deal` is GetIGZUnknown() queried
 * for cISC4NeighborDeal (deal status), 0 when it is null or not a deal. */
typedef struct fc4_plumbing_message {
    uint32_t msg_type;
    uint32_t data1, data2, data3;
    uint32_t reserved;
    uint64_t occupant;
    uint64_t deal;
} fc4_plumbing_message;

/* GetWaterProducerInfo outputs (slot 24). */
typedef struct fc4_plumbing_producer_info {
    uint32_t production;
    uint32_t max_capacity;
    float efficiency;
    bool polluted;
    uint32_t used; /* property 0x08D9763D, 0 if absent */
} fc4_plumbing_producer_info;

/* FindWaterBuildings results (slot 27). Each occupant is a BORROWED token (the
 * core's producer list holds the references); the plugin AddRefs one it writes
 * to a non-null output pointer. */
typedef struct fc4_plumbing_found {
    uint32_t has_lowest_funded;
    float lowest_funding;
    uint64_t lowest_funded;
    uint32_t has_worst_condition;
    int32_t condition_percent;
    uint64_t worst_condition;
} fc4_plumbing_found;

/* Diagnostics. */
typedef struct fc4_plumbing_state {
    uint32_t struct_size;
    uint8_t initialized, grids_initialized, enabled, recompute;
    uint32_t state; /* 0..9, 9 = idle */
    int32_t count_x, count_z;
    int32_t pass_counter;
    uint32_t producers, num_pipes, num_distressed, num_burst, num_polluted;
    uint32_t average_water_pollution;
    uint32_t watered_cells, building_cells;
    uint32_t reentrant_calls;
    uint32_t reserved;
    float pipe_condition, factor;
    int64_t demand, total_water_cost;
    int64_t last_produced, last_consumed, last_imported, last_exported;
    uint64_t department;
    uint64_t working_grid;
    uint64_t published_grid;
} fc4_plumbing_state;

/* What the plumbing simulator needs from the game (fc4.h: one callback per
 * PlumbingHost method, engine/core/src/plumbing/mod.rs, plumbing.md §11).
 * Missing callbacks behave as "nothing there" (0, false, no object). */
typedef struct fc4_plumbing_host {
    uint32_t struct_size; /* sizeof(fc4_plumbing_host) */
    void* user;
    fc4_grid_api grids;
    /* city */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    void (*position_to_cell)(void* user, float x, float z, int32_t* cx, int32_t* cz);
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL if missing */
    uint32_t (*wall_clock_seed)(void* user);
    /* simulator, messages, history */
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t name_len, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, const uint32_t* ids, size_t count);
    void (*unsubscribe)(void* user, const uint32_t* ids, size_t count);
    /* Posts a standard message: WATER_MAP_CHANGED (0x6946DC34) carries
     * void1 = IGZUnknown = the simulator; DEAL_STATUS (0xEA418D8F) carries
     * data1 (§6.7 step 9). */
    void (*post)(void* user, uint32_t msg_type, uint32_t data1);
    void (*record_history)(void* user, uint32_t id, uint32_t value);
    /* false = no ordinance simulator (the core then reads 1.0). */
    bool (*cumulative_factor)(void* user, uint32_t id, uint32_t index, float* out);
    int32_t (*water_value)(void* user, int32_t x, int32_t z);
    /* occupants */
    uint32_t (*occupant_type)(void* user, uint64_t occ);
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    void (*set_occupant_property)(void* user, uint64_t occ, uint32_t id, uint16_t mem_type, bool is_array,
                                  const void* data, size_t count);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    void (*occupant_bounding_box)(void* user, uint64_t occ, float* min_xyz, float* max_xyz);
    void (*occupant_bounding_cells)(void* user, uint64_t occ, fc4_cell_rect* out);
    bool (*building_type)(void* user, uint64_t occ, uint32_t* out);
    void (*buildings_in_cell)(void* user, int32_t x, int32_t z, fc4_token_callback cb, void* ctx);
    /* network occupants (pipes) */
    bool (*is_network_occupant)(void* user, uint64_t occ);
    bool (*has_network_flag)(void* user, uint64_t occ, uint32_t flag);
    uint32_t (*network_flags)(void* user, uint64_t occ);
    void (*set_network_flag)(void* user, uint64_t occ, uint32_t flag);
    void (*clear_network_flag)(void* user, uint64_t occ, uint32_t flag);
    bool (*connects_to_neighbor)(void* user, uint64_t occ, uint32_t dir, uint32_t network_type);
    void (*occupied_cell)(void* user, uint64_t occ, int32_t* x, int32_t* z);
    /* The direct call to cSC4PipeOccupant::SetWateredState (docs/abi/direct-calls.md). */
    void (*set_pipe_watered)(void* user, uint64_t occ, bool watered);
    /* lots */
    uint64_t (*lot_at)(void* user, int32_t x, int32_t z);
    uint64_t (*occupant_lot)(void* user, uint64_t occ);
    void (*lot_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    bool (*lot_needs_power)(void* user, uint64_t lot);
    bool (*lot_has_power)(void* user, uint64_t lot);
    void (*set_lot_condition)(void* user, uint64_t lot, uint8_t condition);
    /* terrain */
    bool (*terrain_cell_counts)(void* user, int32_t* x, int32_t* z); /* false = no terrain */
    bool (*cell_has_water)(void* user, int32_t x, int32_t z);
    void (*redisplay_terrain)(void* user, int32_t min_x, int32_t min_z, int32_t max_x, int32_t max_z);
    /* disasters: false = the pipe-burst manager is missing (§7.1). */
    bool (*attempt_pipe_burst)(void* user, int32_t x, int32_t z);
    /* neighbours */
    void (*find_deals)(void* user, uint32_t deal_type, fc4_token_callback cb, void* ctx);
    uint32_t (*deal_type)(void* user, uint64_t deal);
    uint64_t (*deal_city)(void* user, uint64_t deal);
    int32_t (*deal_amount)(void* user, uint64_t deal);
    int32_t (*deal_amount_used)(void* user, uint64_t deal);
    void (*set_deal_amount_used)(void* user, uint64_t deal, int32_t used);
    void (*set_deal_flags)(void* user, uint64_t deal, uint32_t flags, bool on);
    void (*connections_of_type)(void* user, uint32_t connection_type, fc4_token_callback cb, void* ctx);
    void (*connections_of_type_to_city)(void* user, uint32_t connection_type, uint64_t city, fc4_token_callback cb,
                                        void* ctx);
    uint64_t (*find_unvisited_connection)(void* user, uint32_t connection_type, uint64_t city);
    uint64_t (*connection_on_other_side)(void* user, uint64_t conn);
    void (*connection_cell)(void* user, uint64_t conn, int32_t* x, int32_t* z);
    void (*set_connection_visited)(void* user, uint64_t conn, bool visited);
    bool (*connection_is_active)(void* user, uint64_t conn);
    void (*set_connection_active)(void* user, uint64_t conn, bool active);
    void (*mark_deals_dirty)(void* user);
    /* budget */
    uint64_t (*create_department)(void* user, uint32_t id, uint32_t group);
    void (*set_department_name)(void* user, uint64_t dept, uint32_t group, uint32_t instance);
    void (*set_department_max_funding)(void* user, uint64_t dept, float fraction);
    uint64_t (*create_line_item)(void* user, uint64_t dept, uint32_t line, bool locally_funded);
    void (*set_line_item_name)(void* user, uint64_t line_item, uint32_t group, uint32_t instance);
    void (*remove_line_item)(void* user, uint64_t dept, uint32_t line);
    float (*department_funding)(void* user, uint64_t dept);
    uint64_t (*department_by_id)(void* user, uint32_t id);
    uint64_t (*line_item)(void* user, uint64_t dept, uint32_t line);
    uint64_t (*create_line_item_for_building_type)(void* user, uint64_t dept, uint32_t line, bool locally_funded);
    void (*set_line_item_display_flag)(void* user, uint64_t line_item, uint32_t flag, bool on);
    void (*add_to_line_item_expenses)(void* user, uint64_t line_item, int64_t delta);
    void (*set_line_item_expenses)(void* user, uint64_t line_item, int64_t value);
    int64_t (*line_item_secondary_info)(void* user, uint64_t line_item);
    void (*set_line_item_secondary_info)(void* user, uint64_t line_item, int64_t value);
    bool (*budget_item_for_purpose)(void* user, uint64_t occ, uint32_t purpose, fc4_plumbing_budget_item* out);
    void (*change_budget_item_line)(void* user, uint64_t occ, uint32_t index, uint32_t line);
    void (*change_budget_item_cost)(void* user, uint64_t occ, uint32_t index, int64_t cost);
    uint64_t (*line_item_from_budget_item)(void* user, const fc4_plumbing_budget_item* item);
    bool (*local_funding)(void* user, uint64_t dept, uint64_t occ, float* out);
    /* references */
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* A reference read from a save (QI'd to iid, one reference) to the object's
     * canonical token (one reference); 0 = dropped. */
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
} fc4_plumbing_host;

fc4_plumbing* fc4_plumbing_create(const fc4_plumbing_host* host, uint64_t token);
void fc4_plumbing_destroy(fc4_plumbing* p);
bool fc4_plumbing_init(fc4_plumbing* p);          /* §4.1 */
bool fc4_plumbing_shutdown(fc4_plumbing* p);      /* §4.2 */
/* §5.1: true except for the day and idle messages, which return DoIdle's result.
 * A message delivered during another call is queued. */
bool fc4_plumbing_do_message(fc4_plumbing* p, const fc4_plumbing_message* m);
/* The argument-free slots that return a number: 10 (with an occupant: use
 * fc4_plumbing_water_production), 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
 * 23. Slots 14-17 and 23 return the low 32 bits of a 64-bit total (§10.4). */
int64_t fc4_plumbing_slot_value(const fc4_plumbing* p, uint32_t slot);
uint8_t fc4_plumbing_supply_value(const fc4_plumbing* p, int32_t x, int32_t z);        /* 5 */
bool fc4_plumbing_are_watered_buildings_in_cell(const fc4_plumbing* p, int32_t x, int32_t z); /* 8 */
bool fc4_plumbing_is_building_watered(const fc4_plumbing* p, uint64_t occ);           /* 9 */
uint32_t fc4_plumbing_water_production(const fc4_plumbing* p, uint64_t occ);          /* 10 */
bool fc4_plumbing_is_cell_watered(const fc4_plumbing* p, int32_t x, int32_t z);       /* 11 */
/* Slot 24: false = the occupant is not a producer (outputs untouched). */
bool fc4_plumbing_get_producer_info(const fc4_plumbing* p, uint64_t occ, fc4_plumbing_producer_info* out);
/* Slot 25: false = the condition or its curve is absent (output untouched). */
bool fc4_plumbing_producer_condition_percent(const fc4_plumbing* p, uint64_t occ, float* out);
/* Slot 26: false = no budget item for the purpose (output untouched). */
bool fc4_plumbing_occupant_monthly_upkeep_cost(const fc4_plumbing* p, uint64_t occ, int64_t* out);
bool fc4_plumbing_find_water_buildings(const fc4_plumbing* p, fc4_plumbing_found* out); /* 27 */
/* Slot 28: every producer occupant in list order, each AddRef'd for the caller.
 * Without a callback the references are given back. */
size_t fc4_plumbing_water_producers(const fc4_plumbing* p, fc4_token_callback cb, void* ctx);
uint64_t fc4_plumbing_watered_grid(const fc4_plumbing* p); /* 6, 7: borrowed token, 0 = none */
bool fc4_plumbing_get_state(const fc4_plumbing* p, fc4_plumbing_state* out);
bool fc4_plumbing_write(const fc4_plumbing* p, const fc4_ostream* out); /* §9 */
bool fc4_plumbing_read(fc4_plumbing* p, const fc4_istream* in);         /* §9: false for version != 4 */

/* ---- flammability (docs/specs/flammability.md) ------------------------------
 *
 * Tokens: occupants and lots (the cISC4Occupant / cISC4Lot identity); 0 is
 * null. Both grids are the game's own cSC4SimGrid<uint8> (fc4_grid_api): the
 * map view walks the adjusted grid's memory through GetGridData (§13.2).
 * A message delivered while another call into the same object is running is
 * queued and handled when that call returns. */
typedef struct fc4_flammability fc4_flammability;

/* A message for fc4_flammability_do_message (§5). `occupant` is GetVoid1() as
 * a cISC4Occupant (Insert / Remove / ChangeOccupant); `lot` is GetIGZUnknown()
 * queried for cISC4Lot (ConditionChanged), 0 when it is null or not a lot. */
typedef struct fc4_flammability_message {
    uint32_t msg_type;
    uint32_t reserved;
    uint64_t occupant;
    uint64_t lot;
} fc4_flammability_message;

/* Diagnostics. */
typedef struct fc4_flammability_state {
    uint32_t struct_size;
    uint8_t initialized;
    uint8_t pad0[3];
    int32_t count_x, count_z;
    uint8_t average;
    uint8_t pad1[3];
    float global_multiplier;
    int32_t zoned_sum, zoned_count;
    uint32_t candidates, hotspots;
    int32_t tract_size, cycle_period, tracts_per_day;
    uint32_t reserved;
    uint64_t adjusted_grid;
    uint64_t raw_grid;
    uint32_t reentrant_calls;
    uint32_t reserved2;
} fc4_flammability_state;

/* What the flammability simulator needs from the game (fc4.h: one callback per
 * FlammabilityHost method, engine/core/src/flammability/mod.rs, §12). Missing
 * callbacks behave as "nothing there" (0, false, no object). The agent
 * registration and the six notifications of §4.1 steps 2-3 are the plugin's,
 * not callbacks. */
typedef struct fc4_flammability_host {
    uint32_t struct_size; /* sizeof(fc4_flammability_host) */
    void* user;
    fc4_grid_api grids;
    /* city */
    void (*city_cell_counts)(void* user, int32_t* x, int32_t* z);
    void (*position_to_cell)(void* user, float x, float z, int32_t* cx, int32_t* cz);
    bool (*is_tutorial)(void* user);
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL if missing */
    /* occupants */
    uint32_t (*occupant_type)(void* user, uint64_t occ);
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    void (*occupant_bounds)(void* user, uint64_t occ, fc4_cell_rect* out);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    void (*occupants_in_cells)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    /* lots */
    uint64_t (*occupant_lot)(void* user, uint64_t occ);
    uint8_t (*lot_condition)(void* user, uint64_t lot);
    uint64_t (*lot_building)(void* user, uint64_t lot);
    uint64_t (*random_lot)(void* user);
    /* other subsystems; the bool/int32 returns encode the core's Option: a
     * missing service reads "not watered" / "unzoned" / factor 1.0 (§4.1 step 5
     * [choice]). */
    bool (*cumulative_factor)(void* user, uint32_t prop, uint32_t index, float* out);
    bool (*current_season)(void* user, int32_t* out);
    /* -1 = no plumbing simulator, 0 = not watered, 1 = watered. */
    int32_t (*is_cell_watered)(void* user, int32_t x, int32_t z);
    bool (*zone_type)(void* user, int32_t x, int32_t z, uint32_t* out);
    void (*record_history)(void* user, uint32_t history_type, int32_t value);
    /* fire (§6.8): false = the fire manager or its interface is missing. */
    bool (*ignite_cell)(void* user, uint32_t x, uint32_t z, bool use_probability, bool teleport, uint32_t fire_iid);
    void (*set_fire_cause)(void* user, uint32_t cause);
    /* messages: MessagePost with void1 = IGZUnknown = the simulator (§9). */
    void (*post)(void* user, uint32_t msg_type);
} fc4_flammability_host;

fc4_flammability* fc4_flammability_create(const fc4_flammability_host* host, uint64_t token, uint32_t wall_clock_seed,
                                          uint32_t process_clock);
void fc4_flammability_destroy(fc4_flammability* p);
bool fc4_flammability_init(fc4_flammability* p);      /* §4.1 */
bool fc4_flammability_shutdown(fc4_flammability* p);  /* §4.2 */
/* §5: always true for a live object. A message delivered during another call
 * is queued and handled when the outermost call returns. */
bool fc4_flammability_do_message(fc4_flammability* p, const fc4_flammability_message* m);
bool fc4_flammability_is_cell_flammable(const fc4_flammability* p, int32_t x, int32_t z); /* slot 6 */
/* Slot 7: false = no hotspot or its tract value is 0, and no output is
 * written (§13.2). */
bool fc4_flammability_hotspot(const fc4_flammability* p, uint8_t* value, int32_t* x, int32_t* z);
uint8_t fc4_flammability_average(const fc4_flammability* p); /* slot 8 */
uint64_t fc4_flammability_grid(const fc4_flammability* p);   /* slot 5: borrowed token, 0 = none */
bool fc4_flammability_get_state(const fc4_flammability* p, fc4_flammability_state* out);
bool fc4_flammability_write(const fc4_flammability* p, const fc4_ostream* out); /* §10 */
bool fc4_flammability_read(fc4_flammability* p, const fc4_istream* in);         /* §10: false for version != 2 */

/* ---- inspection ------------------------------------------------------------ */
bool fc4_sim_get_state(const fc4_sim* sim, fc4_state* out);
/* Diagnostics: reads the clock once, changes nothing. */
bool fc4_sim_get_debug(const fc4_sim* sim, fc4_debug* out);
/* Diagnostics: the registration at `index` of `level` (registration order).
 * The name is copied NUL-terminated (truncated to name_capacity - 1); `name`
 * may be NULL. Returns false if there is no such registration. */
bool fc4_sim_get_agent_info(const fc4_sim* sim, uint32_t level, size_t index, fc4_agent_info* out, char* name,
                            size_t name_capacity);

#ifdef __cplusplus
} /* extern "C" */
#endif

/* Layout checks (must match #[repr(C)] in engine/capi/src/lib.rs). */
#ifdef __cplusplus
static_assert(sizeof(fc4_message) == 32, "fc4_message layout");
static_assert(sizeof(fc4_date) == 24, "fc4_date layout");
static_assert(sizeof(fc4_debug) == 76, "fc4_debug layout");
static_assert(sizeof(fc4_agent_info) == 16, "fc4_agent_info layout");
static_assert(sizeof(fc4_timer_source) == (sizeof(void*) == 4 ? 32 : 48), "fc4_timer_source layout");
static_assert(sizeof(fc4_state) == 4 + 4 + 5 * 4 + 5 * 4 + 3 * 4 + 5 * 4 + 11 * 4 + 12 + 11 * 4, "fc4_state layout");
static_assert(sizeof(fc4_budget_message) == 48, "fc4_budget_message layout");
static_assert(sizeof(fc4_lot_info) == 32, "fc4_lot_info layout");
static_assert(sizeof(fc4_budget_item) == 32, "fc4_budget_item layout");
static_assert(sizeof(fc4_loan_info) == 32, "fc4_loan_info layout");
static_assert(sizeof(fc4_bond) == 56, "fc4_bond layout");
static_assert(sizeof(fc4_budget_group) == 24, "fc4_budget_group layout");
static_assert(sizeof(fc4_budget_state) == 432, "fc4_budget_state layout");
static_assert(sizeof(fc4_budget_host) == (sizeof(void*) == 4 ? 124 : 248), "fc4_budget_host layout");
static_assert(sizeof(fc4_tax_lot) == 24, "fc4_tax_lot layout");
static_assert(sizeof(fc4_tax_scan) == 408, "fc4_tax_scan layout");
static_assert(sizeof(fc4_grid_geometry) == 32, "fc4_grid_geometry layout");
static_assert(sizeof(fc4_landvalue_state) == 56, "fc4_landvalue_state layout");
static_assert(sizeof(fc4_landvalue_host) == (sizeof(void*) == 4 ? 92 : 184), "fc4_landvalue_host layout");
static_assert(sizeof(fc4_landvalue_tuning) == (sizeof(void*) == 4 ? 72 : 128), "fc4_landvalue_tuning layout");
static_assert(sizeof(fc4_weather_state) == 56, "fc4_weather_state layout");
static_assert(sizeof(fc4_weather_host) == (sizeof(void*) == 4 ? 56 : 112), "fc4_weather_host layout");
static_assert(sizeof(fc4_ordinance_sim_state) == 40, "fc4_ordinance_sim_state layout");
static_assert(sizeof(fc4_ordinance_host) == (sizeof(void*) == 4 ? 104 : 208), "fc4_ordinance_host layout");
static_assert(sizeof(fc4_demand_connection) == 24, "fc4_demand_connection layout");
static_assert(sizeof(fc4_demand_growth_developer) == 24, "fc4_demand_growth_developer layout");
static_assert(sizeof(fc4_demand_sim_state) == 44, "fc4_demand_sim_state layout");
static_assert(sizeof(fc4_demand_host) == (sizeof(void*) == 4 ? 172 : 344), "fc4_demand_host layout");
static_assert(sizeof(fc4_grid_api) == (sizeof(void*) == 4 ? 40 : 80), "fc4_grid_api layout");
static_assert(sizeof(fc4_cell_rect) == 16, "fc4_cell_rect layout");
static_assert(sizeof(fc4_power_post) == 32, "fc4_power_post layout");
static_assert(sizeof(fc4_power_pole) == 12, "fc4_power_pole layout");
static_assert(sizeof(fc4_power_line) == 12, "fc4_power_line layout");
static_assert(sizeof(fc4_power_deal) == 24, "fc4_power_deal layout");
static_assert(sizeof(fc4_power_budget_item) == 24, "fc4_power_budget_item layout");
static_assert(sizeof(fc4_power_message) == 48, "fc4_power_message layout");
static_assert(sizeof(fc4_power_found) == 32, "fc4_power_found layout");
static_assert(sizeof(fc4_power_state) == 88, "fc4_power_state layout");
static_assert(sizeof(fc4_power_host) == (sizeof(void*) == 4 ? 400 : 800), "fc4_power_host layout");
static_assert(sizeof(fc4_plumbing_budget_item) == 24, "fc4_plumbing_budget_item layout");
static_assert(sizeof(fc4_plumbing_message) == 40, "fc4_plumbing_message layout");
static_assert(sizeof(fc4_plumbing_producer_info) == 20, "fc4_plumbing_producer_info layout");
static_assert(sizeof(fc4_plumbing_found) == 32, "fc4_plumbing_found layout");
static_assert(sizeof(fc4_plumbing_state) == 144, "fc4_plumbing_state layout");
static_assert(sizeof(fc4_plumbing_host) == (sizeof(void*) == 4 ? 356 : 712), "fc4_plumbing_host layout");
static_assert(sizeof(fc4_flammability_message) == 24, "fc4_flammability_message layout");
static_assert(sizeof(fc4_flammability_state) == 80, "fc4_flammability_state layout");
static_assert(sizeof(fc4_flammability_host) == (sizeof(void*) == 4 ? 132 : 264), "fc4_flammability_host layout");
#else
_Static_assert(sizeof(fc4_message) == 32, "fc4_message layout");
_Static_assert(sizeof(fc4_date) == 24, "fc4_date layout");
_Static_assert(sizeof(fc4_debug) == 76, "fc4_debug layout");
_Static_assert(sizeof(fc4_agent_info) == 16, "fc4_agent_info layout");
#endif

#endif /* FC4_H */
