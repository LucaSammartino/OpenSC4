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
 * 15 = adds the budget shadow's tax scan (fc4_budget_shadow_tax);
 * 16 = adds the demand shadow (fc4_demand_shadow);
 * 17 = adds the pollution simulator (fc4_pollution) and its shadow (fc4_pollution_shadow);
 * 18 = adds the protection simulators (fc4_protection);
 * 19 = adds the crime simulator and the crime shadow (fc4_crime / fc4_crime_shadow);
 * 20 = adds the aura simulator (fc4_aura) and its shadow (fc4_aura_shadow);
 * 21 = adds the flora simulator (fc4_flora, fc4_flora_registry) and its shadow (fc4_flora_shadow);
 * 22 = adds the landfill developer (fc4_landfill) and its shadow (fc4_landfill_shadow);
 * 23 = adds the power shadow (fc4_power_shadow);
 * 24 = adds the plumbing shadow (fc4_plumbing_shadow);
 * 25 = adds the land value and flammability shadows (fc4_landvalue_shadow / fc4_flammability_shadow);
 * 26 = adds the residential simulator (fc4_residential) and its shadow (fc4_residential_shadow);
 * 27 = adds fc4_protection_shadow_unadjusted_grid (protection.md §2, PR5);
 * 28 = adds the neighbours simulator (fc4_neighbors, fc4_neighbor_deal,
 *      fc4_neighbor_connection) and its shadow (fc4_neighbors_shadow);
 * 30 = the aura shadow follows the original step by step (fc4_aura_shadow_before_step / _step / _occupant);
 *      fc4_flora_host's obliteration segment callbacks become record streams (§10.3);
 * 33 = adds fc4_power_shadow_message and the power shadow's city start (options `flags`,
 *      FC4_POWER_SHADOW_CITY_START; power.md PW3, PW4, PW5, PW7); the plumbing shadow's
 *      fc4_plumbing_shadow_city_start and fc4_plumbing_original.pass_counter (plumbing.md
 *      PS3, PS6); the landfill host's insert_landfill_tile and fc4_landfill_state's
 *      game_tiles / game_tile_failures (landfill.md LG3, LG8);
 * 34 = adds the zone manager (fc4_zones, zones.md) and its shadow mode (fc4_zones_create_shadow);
 * 35 = adds the building development simulator and growth developers (fc4_development / fc4_growth)
 *      and the development shadow (taken as 29 on its lane; renumbered at merge);
 * 41 = adds the turbo clock (tooling: fc4_sim_set_turbo / fc4_sim_get_turbo / fc4_sim_take_turbo_auto_off);
 * 44 = adds the traffic simulator, its network model and its pathfinders (fc4_traffic, fc4_traffic_pf)
 *      (taken as 37 on its lane; renumbered at merge);
 * 45 = neighbours answers (neighbors.md §15.4): foreign deals are kept as the listed deal
 *      (fc4_neighbors_add_foreign_deal, fc4_neighbors_host's foreign_deal / foreign_deal_call /
 *      release_deal, NG4); the neighbours shadow input carries the original's deals_dirty /
 *      stats_dirty / busy bytes (NG7) (taken as 47 on its lane; renumbered at merge);
 * 46 = adds the history warehouse (fc4_history) and the 24-hour clock (fc4_clock24);
 * 47 = adds the commercial simulator and industrial stub (fc4_commercial / fc4_industrial),
 *      the civic building simulator (fc4_civic) and the park manager (fc4_park);
 * 48 = adds the tract developer (fc4_tract, tract-developer.md) and its shadow mode
 *      (fc4_tract_create_shadow, the fc4_tract_probe_* functions; taken as 40 on its lane,
 *      then 45 and 47; renumbered at merge).
 * 55 = adds the history warehouse and 24-hour clock glue (docs/specs/history.md, clock24.md):
 *      their shadow modes (history_shadow, clock24_shadow) and the save sweep mode for the
 *      two records (0x89EFA536, 0xA7E7F929); taken as 55 on the histclock lane. */
#define FC4_ABI_VERSION 55u

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
    float total;          /* the float32 running total, lot by lot in visit order (BQ3) */
    int64_t weighted_assessed[12];
    int64_t monthly_tax[12]; /* trunc(groupTax[t]) */
    int64_t tax_income[12];  /* GetTaxIncome(t) with the given rates */
    float group_tax[12];
    uint32_t counted[12];    /* lots counted per group */
    int64_t total_tax;       /* ComputeTax's result, trunc(total), never the sum of the lines */
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
    /* Every lot, in the lot manager's master-list order (§5.2): the tax total
     * is a float32 running sum, so only that order matches it exactly (BQ3). */
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
    /* Lot developer GetAreaSlope({x1, z1, x2, z2}, true), inclusive cells;
     * read-only in the original (landvalue.md §6, LV3). */
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
    /* Growth developer slot 10 GrowthAndDeclineListsAreValid (state < 17) of
     * a developer: true while its desirability map cannot change
     * (landvalue.md §7.4, LV2). Read by the land value shadow only; NULL =
     * unknown. */
    bool (*developer_lists_valid)(void* user, uint64_t developer);
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
     * capacity list goes to `visit` (vector index, developer type, value).
     * `visit` may be NULL: the purpose alone, no capacity walk (the census,
     * §6.3 / DQ5). */
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
/* CalculateJobsPerUnitOfDemand: out = the 3 outputs (may be NULL), each the
 * demand.md 4.1 link factor, percent x 0.01f (DQ12: rounded once below 2^31,
 * twice from 2^31 up). The game's record is four floats: the plugin writes
 * them to jobs[1..3] and never touches jobs[0]. */
void fc4_demand_sim_jobs_per_unit(const fc4_demand_sim* s, uint32_t demand_id, uint32_t demand_index, float* out);
/* GetJobsBySensus (DQ10): (jobs[3] + jobs[2]) + jobs[1] times the supply,
 * the low 32 bits of _ftol, no clamp; slot 9 returns it in EAX. */
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

/* Demand shadow (diagnostics): FreeCity4's propagation (§4.4) and monthly
 * modifiers (§8.1), the code its demand simulator runs, over node states read
 * from another demand simulator (the original game's). Nothing is created in
 * the game, stored, posted or written. */
typedef struct fc4_demand_shadow fc4_demand_shadow;

/* One node's state (§3.1). */
typedef struct fc4_demand_node_state {
    uint32_t key, region_use;
    float supply, demand, new_supply, new_demand;
    float active_demand, active_min, active_max;
    float economy_modifier, tax_modifier, demand_cap;
} fc4_demand_node_state;

typedef struct fc4_demand_shadow_info {
    uint32_t struct_size; /* caller sets = sizeof(fc4_demand_shadow_info) */
    uint32_t exemplars, keys, links;
    uint32_t tuning_found; /* 0: the tuning exemplar is missing (code defaults) */
    uint32_t reserved;
} fc4_demand_shadow_info;

typedef struct fc4_demand_shadow_stats {
    uint32_t struct_size; /* caller sets = sizeof(fc4_demand_shadow_stats) */
    uint32_t nodes, passes, settled, transfers, end_of_cycles;
    uint32_t missing_targets; /* link visits whose target is not among the nodes */
    uint32_t reserved;
} fc4_demand_shadow_stats;

/* §8.1 for one total node. */
typedef struct fc4_demand_modifier {
    float tax_modifier, economy_modifier;
    float max_rate, ratio, tax_curve, ordinance_factor, nation;
    uint32_t from_history; /* 0: no history in the window, max_rate = neutral * 10 */
} fc4_demand_modifier;

typedef struct fc4_demand_modifiers {
    uint32_t struct_size; /* caller sets = sizeof(fc4_demand_modifiers) */
    uint32_t today;
    float neutral_raw, neutral;
    fc4_demand_modifier node[12]; /* 0x21010, 0x21020, ... 0x24400 (§8.1 order) */
} fc4_demand_modifiers;

/* The graph (§4.2) and tuning (§11.1) from the exemplars `host` serves
 * (for_each_exemplar_key and exemplar only). NULL on bad arguments. */
fc4_demand_shadow* fc4_demand_shadow_create(const fc4_demand_host* host);
void fc4_demand_shadow_destroy(fc4_demand_shadow* s);
bool fc4_demand_shadow_get_info(const fc4_demand_shadow* s, fc4_demand_shadow_info* out);
/* The graph's keys (exemplar nodes and link targets), ascending; returns their number. */
uint32_t fc4_demand_shadow_keys(const fc4_demand_shadow* s, fc4_u32_callback cb, void* ctx);
/* Whether propagation can change the key's totals other than by folding its
 * own pending values (a transfer target, or a Sums node). */
bool fc4_demand_shadow_is_fed(const fc4_demand_shadow* s, uint32_t key);
/* DoDemandIndexLinks over `count` states (distinct keys) into `output` (same
 * order; may equal `input`); `stats` may be NULL. False on bad arguments. */
bool fc4_demand_shadow_propagate(const fc4_demand_shadow* s, const fc4_demand_node_state* input, uint32_t count,
                                 fc4_demand_node_state* output, fc4_demand_shadow_stats* stats);
/* GetNeutralTaxRate (unclamped) from the supplies r, c, i of 0x1001, 0x3001, 0x4001; the sum is
 * f32(f32(i + c) + r), as the simulator's (demand §5, DQ9). */
float fc4_demand_shadow_neutral_tax_rate(const fc4_demand_shadow* s, float r, float c, float i);
/* §8.1 from those supplies, the saved CO / I economy bonuses, and host's today,
 * history_max, ordinance_factor and nation_economy_modifier (no other callback). */
bool fc4_demand_shadow_modifiers(const fc4_demand_shadow* s, const fc4_demand_host* host, float r, float c, float i,
                                 float co_bonus, float i_bonus, fc4_demand_modifiers* out);
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

/* ---- power shadow (tooling; engine/capi/src/power_shadow.rs) -----------------
 *
 * With the ORIGINAL power simulator running, each of its SimNewMonth
 * deliveries and each SimNewDay / SimIdle delivery that does work (one DoIdle
 * step, §6) is wrapped: before it the plugin captures the original's §9.1
 * record (its own Write into memory: primitives little-endian, references as
 * u32 index + 1 (0 = null) then u32 class ID, each object listed once with its
 * tokens) and its slots 27-29, and fc4_power_shadow_begin copies the live
 * powered grid and runs FreeCity4's step on the record with every input read
 * through a read-only host table (write callbacks NULL; grids: geometry and get
 * only); a month also rebuilds the bookkeeping from scratch (§5.1 for every
 * live occupant, a wire rebuild of the map). After the delivery the plugin
 * captures again and fc4_power_shadow_finish compares the record's fields and
 * bitmaps, the consumption map, the powered grid cell by cell, the counts and
 * the occupant properties, deal usage, connection flags and budget item costs
 * FreeCity4 wrote (kept in the shadow) with the game's. The unsaved values
 * the step needs are mirrored: known from a city start
 * (FC4_POWER_SHADOW_CITY_START), the dirty rectangle merged from the messages
 * the plugin forwards (fc4_power_shadow_message). Not compared (not
 * readable): posted messages, history, budget line items, demolitions, lot
 * conditions, the poles the month's shuffle picks (the count is compared),
 * deal flags. */
typedef struct fc4_power_shadow fc4_power_shadow;

/* fc4_power_capture_ref.flags: the object answers cISC4PowerPoleOccupant
 * (0xA8C86142); the pole is known by `occupant`. */
#define FC4_POWER_CAPTURE_POLE 1u

typedef struct fc4_power_capture_ref {
    uint64_t occupant;   /* cISC4Occupant 0x26BB767E (plants, poles); borrowed; 0 = none */
    uint64_t grid;       /* cISC4SimGrid 0xC9BD939A */
    uint64_t department; /* cISC4DepartmentBudget 0x223E2B67 */
    uint32_t class_id;   /* GetGZCLSID() */
    uint32_t flags;      /* FC4_POWER_CAPTURE_POLE */
} fc4_power_capture_ref;

typedef struct fc4_power_capture {
    uint32_t struct_size;
    uint32_t ok; /* non-zero: Write returned true and the stream stayed good */
    const uint8_t* bytes;
    size_t len;
    const fc4_power_capture_ref* refs;
    size_t ref_count;
} fc4_power_capture;

/* The original's unsaved plant counts: slots 27, 28, 29. */
typedef struct fc4_power_shadow_counts {
    uint32_t struct_size;
    uint32_t ok; /* non-zero: read */
    uint32_t overworked, aged, distressed;
    uint32_t reserved;
} fc4_power_shadow_counts;

/* fc4_power_shadow_options.flags: created at the city's PostCityInit (a load
 * or a new city), when the original was just constructed and initialised: its
 * unsaved pass values and brokenFraction are 0, it has no dirty rectangle and
 * its ordinance factor is the one read at creation (power.md §5.3, §7.6, §8,
 * §9.1). Without it (a shadow started mid-city) they are unknown until the
 * original shows them. (ABI 33; the field was `reserved`.) */
#define FC4_POWER_SHADOW_CITY_START 1u

typedef struct fc4_power_shadow_options {
    uint32_t struct_size;
    uint32_t max_cells; /* differing cells listed per line (0 = 8, at most 64) */
    uint32_t max_lines; /* cycle lines with a difference per month (0 = 32; UINT32_MAX = all) */
    uint32_t flags;     /* FC4_POWER_SHADOW_CITY_START */
    /* cISC4NeighborConnection::GetVisitedThisCycle (slot 11) of a connection
     * token, called with the host table's `user`: the shadow emulates
     * FindUnvisitedConnection with it. NULL: the live FindUnvisitedConnection. */
    bool (*connection_visited)(void* user, uint64_t connection);
} fc4_power_shadow_options;

typedef struct fc4_power_shadow_stats {
    uint32_t struct_size;
    uint32_t deliveries, idle_skipped, steps, cycles, months, clean, incomplete, reruns, not_enabled;
    int32_t last_mismatches; /* -1: not compared */
    uint32_t uncached_reads;
    uint64_t mismatches, not_exact, bookkeeping_mismatches;
} fc4_power_shadow_stats;

/* `options` may be NULL (defaults). NULL when the table is short or the grid
 * table has no geometry / get. */
fc4_power_shadow* fc4_power_shadow_create(const fc4_power_host* host, const fc4_power_shadow_options* options);
void fc4_power_shadow_destroy(fc4_power_shadow* s);
/* Whether to wrap this delivery: every SimNewMonth and SimNewDay; a SimIdle
 * unless the original was idle after the last wrapped delivery (state 4, no
 * recompute) and `event` is false (`event`: one of the original's §5
 * messages arrived since). A delivery passed through is counted. */
bool fc4_power_shadow_wants(fc4_power_shadow* s, uint32_t msg_type, bool event);
/* 0 = ran, -1 = not run (finish reports why), -2 = not a day, idle or month.
 * `counts` may be NULL. */
int32_t fc4_power_shadow_begin(fc4_power_shadow* s, uint32_t msg_type, int32_t julian,
                               const fc4_power_capture* before, const fc4_power_shadow_counts* counts);
/* The delivery's difference count (-1 = not compared); log lines go to `sink`,
 * one call per line (each ends in "mismatches=N"): the cycle line at the end
 * of a recompute cycle (capped per month), the month, bookkeeping and month
 * summary lines after a SimNewMonth, INCOMPLETE lines (at most 3 a month). */
int32_t fc4_power_shadow_finish(fc4_power_shadow* s, const fc4_power_capture* after,
                                const fc4_power_shadow_counts* counts, fc4_bytes_sink sink, void* ctx);
bool fc4_power_shadow_get_stats(fc4_power_shadow* s, fc4_power_shadow_stats* out);
/* One of the original's messages as it is delivered to the original (the
 * plugin's message target, inside or outside a wrapped delivery): a removal
 * (void1 = the occupant's cISC4Occupant token) or a zoning change (rect)
 * merges the rectangle the original merges into its dirty rectangle (§5.3,
 * PW5); a tuning reload re-reads the tuning and the ordinance factor (§8,
 * PW7). Reads go through the host table at once. True for these messages. */
bool fc4_power_shadow_message(fc4_power_shadow* s, const fc4_power_message* m); /* ABI 33 */

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

/* ---- plumbing shadow (tooling; engine/capi/src/plumbing_shadow.rs) ----------
 *
 * With the ORIGINAL plumbing simulator running, every SimIdle / SimNewDay
 * (one DoIdle: one pass state, §6) and SimNewMonth delivery to its agent is
 * wrapped by fc4_plumbing_shadow_before / _after. The shadow follows the
 * original's water pass from outside: it sees state 0 run (its passCounter,
 * read through `pass_counter`, moves; PS6; without it, a probe cell of the
 * next working grid changes in state 2), captures the original's record (its
 * own Write, through `capture`) after state 1 and runs FreeCity4's pass on it
 * over private grid copies, reading every input through the host table (read
 * callbacks and the grid table's geometry / get only; nothing is written to
 * the game). Our states 2 to 5 run one per delivery, each in the
 * fc4_plumbing_shadow_before of the delivery in which the original runs the
 * same state, on the original's flag map of that moment (from its record), so
 * they read what the original reads; a pass has no time limit (it is followed
 * until its swap or its restart). At the original's swap (GetWateredGrid
 * changes) it compares the published grid cell by cell, the producers, deals
 * and water connections;
 * after the original's state 8, the totals, the building counts, demand (the
 * original's running sum, read then; PS4, PS8), pipes, sources and the
 * producers' budget items and condition (ageing inferred from the conditions
 * when unknown; PS7). One line per pass. The month (ApplyPipeFunding, §7.1)
 * is compared on its own with the funding of the record's department (PS5):
 * the distressed target exactly when the shadow was made at the city start
 * (fc4_plumbing_shadow_city_start; the original's pipe condition is then 0,
 * PS3), otherwise not exact; the random bursts by range. Not compared (not
 * readable): pipe watered states, redisplay, posts, history, line-item
 * expenses, lot conditions, deal flags, the original's pipe condition and
 * pass state. */
typedef struct fc4_plumbing_shadow fc4_plumbing_shadow;

/* One reference of a captured record, queried for each interface the reader
 * asks for (0 = the object does not answer it); borrowed identities. */
typedef struct fc4_plumbing_capture_ref {
    uint64_t grid;       /* cISC4SimGrid 0xC9BD939A */
    uint64_t occupant;   /* cISC4Occupant 0x26BB767E */
    uint64_t network;    /* the pipe 0x49C05B9F (only a pipe answers it) */
    uint64_t department; /* cISC4DepartmentBudget 0x223E2B67 */
    uint32_t class_id;   /* GetGZCLSID() */
    uint32_t reserved;
} fc4_plumbing_capture_ref;

/* A captured record: primitives little-endian, references as uint32
 * objectID (index into `refs` + 1, 0 = null) then uint32 classID. */
typedef struct fc4_plumbing_capture {
    uint32_t struct_size;
    uint32_t ok; /* Write returned true and the stream stayed good */
    const uint8_t* bytes;
    size_t len;
    const fc4_plumbing_capture_ref* refs;
    size_t ref_count;
} fc4_plumbing_capture;

/* The original simulator, read-only. */
typedef struct fc4_plumbing_original {
    uint32_t struct_size; /* sizeof(fc4_plumbing_original) */
    void* user;
    uint64_t (*watered_grid)(void* user); /* GetWateredGrid (slot 7): borrowed grid token, 0 = none */
    /* Slots 12-23: 14-17 and 23 as their signed 32-bit EAX, the others
     * zero-extended. */
    int64_t (*slot_value)(void* user, uint32_t slot);
    bool (*producer_info)(void* user, uint64_t occupant, fc4_plumbing_producer_info* out); /* slot 24 */
    bool (*connection_visited)(void* user, uint64_t connection); /* GetVisitedThisCycle; may be NULL */
    /* The original's own Write into memory; the view stays valid until the
     * next call. */
    bool (*capture)(void* user, fc4_plumbing_capture* out);
    /* ABI 33: the original's passCounter (record field 7, written only by
     * state 0; plumbing.md §6.1, PS6) through its own Write, references not
     * looked at; false when it failed. May be NULL: a pass start is then seen
     * on a probe cell. */
    bool (*pass_counter)(void* user, int32_t* out);
} fc4_plumbing_original;

typedef struct fc4_plumbing_shadow_stats {
    uint32_t struct_size;
    uint32_t deliveries;
    uint32_t passes_seen;       /* swaps seen */
    uint32_t passes_compared;   /* pass lines */
    uint32_t passes_clean;      /* exact, compared to the end, no difference */
    uint32_t passes_not_exact;  /* the city changed while the pass ran: differences not counted */
    uint32_t passes_incomplete; /* not compared, or the end not seen */
    uint32_t months, months_clean;
    int32_t last_mismatches; /* -1: nothing compared yet */
    uint32_t captures;
    uint32_t reserved;
    uint64_t cells, cell_mismatches, scalar_mismatches;
} fc4_plumbing_shadow_stats;

/* `host`: only its read callbacks and the grid table's geometry / get are
 * called (write callbacks may be NULL). `orig`: watered_grid, slot_value and
 * capture are required. `max_cells`: differing cells listed per line (0 = 8,
 * at most 64). `passes_per_day`: passes replayed per game day (0 = every
 * pass); the others are only followed. NULL for a missing or short table. */
fc4_plumbing_shadow* fc4_plumbing_shadow_create(const fc4_plumbing_host* host, const fc4_plumbing_original* orig,
                                                uint32_t max_cells, uint32_t passes_per_day);
void fc4_plumbing_shadow_destroy(fc4_plumbing_shadow* s);
/* Before the original's SimIdle / SimNewDay / SimNewMonth (`julian` = data1):
 * 0; -2 for any other message type (nothing done). */
int32_t fc4_plumbing_shadow_before(fc4_plumbing_shadow* s, uint32_t msg_type, int32_t julian);
/* After it: log lines go to `sink`, one call per line (a pass line at the end
 * of a replayed pass, INCOMPLETE lines, the month line). Returns the counted
 * differences of a comparison finished in this delivery, -1 if none. */
int32_t fc4_plumbing_shadow_after(fc4_plumbing_shadow* s, fc4_bytes_sink sink, void* ctx);
bool fc4_plumbing_shadow_get_stats(fc4_plumbing_shadow* s, fc4_plumbing_shadow_stats* out);
/* A shadow made at the city start (PostCityInit), before its first delivery:
 * the original's pipe condition and ageing flag are its constructor's (0,
 * false) after every load (plumbing.md §7.1, PS3), so the month's distressed
 * target is compared exactly. False for NULL or after a delivery. (ABI 33) */
bool fc4_plumbing_shadow_city_start(fc4_plumbing_shadow* s);

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
    /* Plumbing slot 7 GetWateredGrid(): the published watered grid as a
     * token (0 = none). Read by the flammability shadow only: non-NULL lets
     * it poll the watered state (by content, FL7, FL9; the pointer is only
     * counted); NULL = not polled. */
    uint64_t (*watered_grid)(void* user);
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

/* ---- pollution (docs/specs/pollution.md) --------------------------------------
 *
 * Tokens: occupants (the cISC4Occupant identity), deals, visual effects, the
 * department; 0 is null. The eight grids are the game's own
 * cSC4SimGrid<int16> (fc4_grid_api), in save order: air, water, garbage,
 * radiation sources, then the air, water, garbage and radiation values.
 *
 * Threads: the game calls the getters from a second thread while the main
 * thread is inside a day or a month. Every function marked "any thread" reads
 * the core's locked state; the others are main-thread only. A message
 * delivered while another main-thread call into the same object is running is
 * queued and handled when the outermost call returns. */
typedef struct fc4_pollution fc4_pollution;

/* GetBudgetItemForPurpose as the budget reports it (SDK BudgetItem). */
typedef struct fc4_pollution_budget_item {
    uint32_t department;
    uint32_t line;
    uint32_t purpose;
    uint32_t index;
    int64_t cost;
    float local_funding;
    uint32_t reserved;
} fc4_pollution_budget_item;

/* One grid's statistics: tracts, non-zero tracts, sum, minimum and maximum of
 * the raw int16 values. */
typedef struct fc4_pollution_grid_stats {
    uint32_t tracts, nonzero;
    int64_t sum;
    int32_t min, max;
} fc4_pollution_grid_stats;

/* Diagnostics. */
typedef struct fc4_pollution_state {
    uint32_t struct_size;
    uint8_t initialized, loaded, garbage_uncollected, effects_visible;
    uint32_t tracts_per_day;
    uint32_t cycle_resets;    /* tract-iterator resets at the end of a cycle */
    uint32_t reentrant_calls; /* messages queued during another call */
    uint32_t lists[4];        /* water treatment, incinerators, waste-to-energy, recycling */
    int32_t avg_air, avg_water;
    uint32_t avg_garbage, total_uncollected;
    int32_t garbage_supply, garbage_capacity;
    uint32_t active_landfill_cells, developed_tracts;
    float efficiency, recycling_effect;
    uint32_t cell_x, cell_z;
    uint32_t reserved;
    /* produced, imported, exported, recycled (truncated), incinerated,
     * waste-to-energy, landfilled */
    int64_t last_month[7];
    int64_t month[7];
    int64_t last_month_cost;
    uint64_t department;
    fc4_pollution_grid_stats grids[8];
} fc4_pollution_state;

/* What the pollution simulator needs from the game: one callback per
 * PollutionHost method (engine/core/src/pollution/simulator.rs), in the
 * trait's order. Missing callbacks behave as "nothing there" (0, false, no
 * object), except cell_widths (16 m), cumulative_factor and
 * department_funding (1.0). */
typedef struct fc4_pollution_host {
    uint32_t struct_size; /* sizeof(fc4_pollution_host) */
    void* user;
    fc4_grid_api grids;
    /* city */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    void (*cell_widths)(void* user, float* x, float* z);
    void (*position_to_cell)(void* user, float x, float z, int32_t* cx, int32_t* cz);
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL if missing */
    float (*altitude)(void* user, float x, float z);
    /* simulator, messages, history */
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t len, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, const uint32_t* msg_types, size_t n);
    void (*unsubscribe)(void* user, const uint32_t* msg_types, size_t n);
    void (*post)(void* user, uint32_t msg_type, uint32_t data1); /* MessagePost; data2..4 = 0, no void / IGZUnknown (§5.2) */
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*set_history_queue_length)(void* user, uint32_t series, uint32_t length);
    void (*add_prop_requester)(void* user);
    void (*remove_prop_requester)(void* user);
    /* other simulators */
    uint32_t (*zone_type)(void* user, int32_t x, int32_t z);
    uint8_t (*air_traffic)(void* user, int32_t x, int32_t z); /* the traffic simulator's air map */
    float (*cumulative_factor)(void* user, uint32_t id, uint32_t index);
    void (*find_deals)(void* user, uint32_t deal_type, fc4_token_callback cb, void* ctx);
    int32_t (*deal_amount)(void* user, uint64_t deal);
    int32_t (*deal_amount_used)(void* user, uint64_t deal);
    void (*set_deal_amount_used)(void* user, uint64_t deal, int32_t used);
    void (*mark_deals_dirty)(void* user);
    bool (*is_occupant_powered)(void* user, uint64_t occ);
    int32_t (*population)(void* user);
    void (*landfill_usage)(void* user, uint32_t* usage, uint32_t* capacity);
    uint32_t (*landfill_tile_count)(void* user);
    void (*set_lot_condition)(void* user, int32_t cell_x, int32_t cell_z, uint32_t condition);
    /* occupants */
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    void (*set_occupant_property)(void* user, uint64_t occ, uint32_t id, uint16_t mem_type, bool is_array,
                                  const void* data, size_t count);
    void (*remove_occupant_property)(void* user, uint64_t occ, uint32_t id);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    void (*occupant_bounding_box)(void* user, uint64_t occ, float* min_xyz, float* max_xyz);
    /* host objects */
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
    /* budget */
    uint64_t (*create_department)(void* user, uint32_t id, uint32_t group); /* borrowed; the core AddRefs */
    void (*set_department_name)(void* user, uint64_t dept, uint32_t group, uint32_t instance);
    void (*set_department_max_funding)(void* user, uint64_t dept, float fraction);
    void (*create_line_item)(void* user, uint64_t dept, uint32_t line, bool local_funding, uint32_t group,
                             uint32_t instance);
    float (*department_funding)(void* user, uint64_t dept);
    void (*set_line_item_full_expenses)(void* user, uint64_t dept, uint32_t line, int64_t value);
    bool (*budget_item_for_purpose)(void* user, uint64_t occ, uint32_t purpose, fc4_pollution_budget_item* out);
    void (*add_budget_item_to_budget)(void* user, uint64_t occ, uint32_t index);
    void (*remove_budget_item_from_budget)(void* user, uint64_t occ, uint32_t index);
    void (*change_budget_item_cost)(void* user, uint64_t occ, uint32_t index, int64_t cost);
    void (*add_to_budget_item_line_expenses)(void* user, const fc4_pollution_budget_item* item, int64_t delta);
    /* visual effects (0 = none) */
    uint64_t (*create_visual_effect)(void* user, uint32_t id, uint32_t kind);
    void (*set_effect_position)(void* user, uint64_t effect, const float* xyz);
    void (*set_effect_param)(void* user, uint64_t effect, uint32_t param, float value);
    void (*start_effect)(void* user, uint64_t effect, uint32_t t);
    void (*stop_effect)(void* user, uint64_t effect, uint32_t t);
    bool (*is_effect_started)(void* user, uint64_t effect);
} fc4_pollution_host;

/* `token`: the simulator's own host identity; `fx_seed` seeds the
 * visual-effect generator. */
fc4_pollution* fc4_pollution_create(const fc4_pollution_host* host, uint64_t token, uint32_t fx_seed);
/* A loaded, never-initialised object gives its list and department references back. */
void fc4_pollution_destroy(fc4_pollution* p);
bool fc4_pollution_init(fc4_pollution* p);     /* §4.1 */
bool fc4_pollution_shutdown(fc4_pollution* p); /* §4.1 */
/* §5.3: always true. `occupant`: the occupant of Insert / RemoveOccupant;
 * `view_level`: the uint32 void2 points at for PreViewLevelChange. */
bool fc4_pollution_do_message(fc4_pollution* p, uint32_t msg_type, uint64_t occupant, uint32_t view_level);
/* Any thread. Slots 10, 11, 12, 20-24, 31, 32, 34-45, 67-70 (booleans 0 / 1);
 * 0 for any other slot. */
int64_t fc4_pollution_slot_value(const fc4_pollution* p, uint32_t slot);
/* Any thread. Slots 7 / 8 / 9 (the value), 13-16 and 49 (0 / 1). */
int32_t fc4_pollution_slot_cell(const fc4_pollution* p, uint32_t slot, int32_t x, int32_t z);
uint64_t fc4_pollution_grid(const fc4_pollution* p, uint32_t kind); /* 5: borrowed token; any thread */
uint16_t fc4_pollution_value(const fc4_pollution* p, uint32_t kind, int32_t x, int32_t z); /* 6; any thread */
/* 17-19 (kind 0 / 1 / 2); any thread. */
bool fc4_pollution_worst_tract(const fc4_pollution* p, uint32_t kind, uint32_t* x, uint32_t* z, int32_t* v);
/* Any thread. 25 SetIsRadioactive (v != 0), 26-28 Set{Air,Water,Garbage}Value
 * (v as int16), 50 SetIsActiveLandfill (v != 0); false for any other slot. */
bool fc4_pollution_set_cell(fc4_pollution* p, uint32_t slot, int32_t x, int32_t z, int32_t v);
int32_t fc4_pollution_add_to_garbage_supply(fc4_pollution* p, int32_t n); /* 29 */
int32_t fc4_pollution_set_garbage_capacity(fc4_pollution* p, int32_t n);  /* 30 */
int64_t fc4_pollution_garbage_put_in_landfill(fc4_pollution* p, int32_t n); /* 51: the new total */
float fc4_pollution_recycling_center_effect(const fc4_pollution* p); /* 33: the value it points at */
float fc4_pollution_garbage_scaling_factor(const fc4_pollution* p);  /* 48 */
/* 46: incinerators, waste-to-energy, recycling as BORROWED tokens (the core's
 * lists hold the references); the caller AddRefs each one it hands out. */
size_t fc4_pollution_garbage_buildings(const fc4_pollution* p, fc4_token_callback cb, void* ctx);
uint32_t fc4_pollution_building_garbage_capacity(const fc4_pollution* p, uint64_t occ); /* 47 */
/* 52 GetMaxCapacity(prop), 53 GetCurrentCapacity(prop), 58-64 (prop ignored):
 * `*out` gets the value the slot writes; returns the slot's bool. */
bool fc4_pollution_occupant_u32(const fc4_pollution* p, uint32_t slot, uint64_t occ, uint32_t prop, uint32_t* out);
/* 54-57 (SC4Percentage as float): `*out` is written on success, and for slot
 * 56 also on failure (1.0). */
bool fc4_pollution_occupant_percent(const fc4_pollution* p, uint32_t slot, uint64_t occ, float* out);
bool fc4_pollution_maintenance_cost(const fc4_pollution* p, uint64_t occ, int64_t* out); /* 65: always writes */
void fc4_pollution_add_radiation(fc4_pollution* p, float x, float y, float z, float strength, float radius); /* 66 */
/* cISC4PropRequester: slot 3 GetRequestedShare (inclusive cells), slots 4, 5, 6. */
int32_t fc4_pollution_requested_share(const fc4_pollution* p, int32_t x0, int32_t z0, int32_t x1, int32_t z1);
uint32_t fc4_pollution_prop_requester_slot(const fc4_pollution* p, uint32_t slot);
bool fc4_pollution_get_state(const fc4_pollution* p, fc4_pollution_state* out); /* main thread */
uint64_t fc4_pollution_grid_by_index(const fc4_pollution* p, uint32_t index); /* save order; borrowed; any thread */
bool fc4_pollution_write(const fc4_pollution* p, const fc4_ostream* out); /* §6 */
bool fc4_pollution_read(fc4_pollution* p, const fc4_istream* in);         /* §6: versions 0-4; before Init */

/* ---- pollution shadow (tooling) -------------------------------------------------
 *
 * With the ORIGINAL pollution simulator running, each of its SimNewDay /
 * SimNewMonth deliveries is wrapped: before it, the plugin captures the
 * original's record (its own Write into a capture stream) and calls
 * fc4_pollution_shadow_begin, which copies the live grids the record refers
 * to and runs FreeCity4's day or month on them with every input read through
 * the host table (read callbacks and the grid table's geometry / get only;
 * nothing is written to the game). After the delivery the plugin captures
 * the record again and calls fc4_pollution_shadow_finish, which compares the
 * eight grids cell by cell, the saved fields, and the occupant properties
 * and deal usage with the original's. Not compared (not readable): the
 * radiation sum (not saved), posted messages, history, budget values, visual
 * effects, worst tracts. */
typedef struct fc4_pollution_shadow fc4_pollution_shadow;

/* One reference of a captured record, queried for each interface the reader
 * asks for (0 = the object does not answer it); borrowed identities. */
typedef struct fc4_pollution_capture_ref {
    uint64_t grid;       /* cISC4SimGrid 0xC9BD939A */
    uint64_t occupant;   /* cISC4Occupant 0x26BB767E */
    uint64_t department; /* cISC4DepartmentBudget 0x223E2B67 */
    uint32_t class_id;   /* GetGZCLSID() */
    uint32_t reserved;
} fc4_pollution_capture_ref;

/* A captured record: primitives little-endian, references as uint32
 * objectID (index into `refs` + 1, 0 = null) then uint32 classID. */
typedef struct fc4_pollution_capture {
    uint32_t struct_size;
    uint32_t ok; /* Write returned true and the stream stayed good */
    const uint8_t* bytes;
    size_t len;
    const fc4_pollution_capture_ref* refs;
    size_t ref_count;
} fc4_pollution_capture;

typedef struct fc4_pollution_shadow_stats {
    uint32_t struct_size;
    uint32_t deliveries, clean, incomplete, reruns;
    int32_t last_mismatches; /* -1: not compared */
    uint32_t uncached_reads;
    uint32_t reserved;
    uint64_t cells, cell_mismatches, field_mismatches, prop_mismatches;
} fc4_pollution_shadow_stats;

/* `max_cells`: differing cells listed per grid and line (0 = 4, at most 64). */
fc4_pollution_shadow* fc4_pollution_shadow_create(const fc4_pollution_host* host, uint32_t max_cells);
void fc4_pollution_shadow_destroy(fc4_pollution_shadow* s);
/* 0 = ran, -1 = not run (finish reports why), -2 = not a day or month. */
int32_t fc4_pollution_shadow_begin(fc4_pollution_shadow* s, uint32_t msg_type, int32_t julian,
                                   const fc4_pollution_capture* before);
/* The delivery's difference count (-1 = not compared); log lines go to `sink`,
 * one call per line (a day with a difference, at most 8 a month; after a
 * month, the month line). */
int32_t fc4_pollution_shadow_finish(fc4_pollution_shadow* s, const fc4_pollution_capture* after, fc4_bytes_sink sink,
                                    void* ctx);
bool fc4_pollution_shadow_get_stats(fc4_pollution_shadow* s, fc4_pollution_shadow_stats* out);

/* ---- protection: fire and police (docs/specs/protection.md) ----------------
 *
 * One handle is one simulator: kind 0 = fire (cSC4FireProtectionSimulator,
 * 0x2990C05A), 1 = police (cSC4PoliceSimulator, 0x6990C04F). Tokens:
 * occupants (stations, jails, automata) by their cISC4Occupant identity,
 * departments, lots, grids and the air dispatch source; 0 is null. The core
 * owns one reference on every token it keeps (stations, jails, departments,
 * the air source): create_department and create_air_source hand one over, and
 * a saved reference read back by fc4_protection_read either arrives with one
 * (a host object) or gets one through add_ref (fc4_ref.core_handle set).
 * Both grids are the game's own cSC4SimGrid (fc4_grid_api: uint8 adjusted,
 * int16 unadjusted, §2). A message delivered while another call into the same
 * object is running is queued and handled when that call returns.
 *
 * Threads: the core is single-threaded. The caller serialises every call on
 * one handle (the plugin holds a per-object lock). */
typedef struct fc4_protection fc4_protection;

/* GetBudgetItemForPurpose with the item's department looked up by its ID
 * (§6.1): has_department = GetDepartmentBudget(item.departmentID) != NULL;
 * locally_funded = that department's IsLocallyFundedObjectInDepartment. */
typedef struct fc4_protection_budget_item {
    uint32_t has_department;
    uint32_t locally_funded;
    float local_funding; /* item.localFundingPercentage (a fraction) */
    uint32_t reserved;
    int64_t cost;
    uint64_t department; /* borrowed */
} fc4_protection_budget_item;

/* GetDispatchInfo (§5). Tokens borrowed. `station` is the record's +0x20,
 * the source station whose dispatch effect draws the cone (§7.2, PR2);
 * `call_id` its +0x08 and `active` its +0x10 (PR1). `reserved` (formerly the
 * vehicle automaton) is not read: leave it 0. */
typedef struct fc4_protection_dispatch_info {
    uint64_t station;
    uint64_t reserved;
    uint32_t call_id;
    uint32_t active;
    float x, z;
} fc4_protection_dispatch_info;

/* A message for fc4_protection_do_message (§5). `occupant` is GetVoid1() as
 * a cISC4Occupant (occupant messages); `unknown` is GetIGZUnknown() (the
 * police crime message). */
typedef struct fc4_protection_message {
    uint32_t msg_type;
    uint32_t data1;
    uint32_t data2;
    uint32_t reserved;
    uint64_t occupant;
    uint64_t unknown;
} fc4_protection_message;

/* Diagnostics. covered_tracts / max_coverage / coverage_sum: the adjusted
 * grid's tracts with a value > 0, its largest value and the sum. */
typedef struct fc4_protection_state {
    uint32_t struct_size;
    uint32_t kind;
    uint8_t initialized, loaded, strike_flag, reserved0;
    uint32_t stations;
    uint32_t tracts;
    int32_t count_x, count_z;
    uint32_t covered_tracts, max_coverage, coverage_sum;
    int32_t tracts_per_day;
    int32_t zoned, sum, non_zero;
    float avg, pct;
    uint32_t strike_date, next_call;
    uint32_t jails, funded_jails, jailbreaks;
    int32_t jail_capacity, jail_population, jailed, arrests_last, crimes_last;
    uint32_t reentrant_calls;
    uint32_t reserved1;
    uint64_t department, jail_department, adjusted_grid, unadjusted_grid, air_source;
} fc4_protection_state;

/* What the protection simulators need from the game (one callback per
 * ProtectionHost method, engine/core/src/protection/mod.rs, §14). Missing
 * callbacks behave as "nothing there" (0, false, no object); the read-only
 * shadow table leaves every callback that changes the game NULL. `kind` is
 * 0 = fire, 1 = police. */
typedef struct fc4_protection_host {
    uint32_t struct_size; /* sizeof(fc4_protection_host) */
    void* user;
    fc4_grid_api grids;
    /* lifecycle (§4) */
    void (*acquire_services)(void* user, uint32_t kind);
    void (*release_services)(void* user, uint32_t kind);
    /* AddAgent(target, level, name, flags) once per level, in order. */
    void (*add_agent)(void* user, const char* name, size_t name_len, const uint32_t* levels, size_t count,
                      uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, const uint32_t* msg_types, size_t count);
    void (*unsubscribe)(void* user, const uint32_t* msg_types, size_t count);
    void (*register_dispatch_source)(void* user);
    void (*unregister_dispatch_source)(void* user);
    void (*set_grid_instance)(void* user, uint64_t grid, uint32_t instance);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* resources: a NEW fc4_props (the core destroys it), NULL if missing */
    fc4_props* (*tuning)(void* user, uint32_t kind);
    /* budget (§4.1, §6.1, §6.6) */
    uint64_t (*create_department)(void* user, uint32_t id, uint32_t group, uint32_t name_group,
                                  uint32_t name_instance, float max_funding); /* one reference, 0 = none */
    uint32_t (*department_id)(void* user, uint64_t dept);
    float (*department_funding)(void* user, uint64_t dept); /* *GetFundingPercentage() */
    void (*add_locally_funded)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    void (*remove_locally_funded)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    bool (*set_local_funding)(void* user, uint64_t dept, uint64_t occ, float pct, uint32_t purpose);
    bool (*budget_item)(void* user, uint64_t occ, uint32_t purpose, fc4_protection_budget_item* out);
    /* occupants */
    uint32_t (*occupant_type)(void* user, uint64_t occ);
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    void (*set_occupant_property)(void* user, uint64_t occ, uint32_t id, uint16_t mem_type, bool is_array,
                                  const void* data, size_t count);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    bool (*is_prop_occupant)(void* user, uint64_t occ);
    int32_t (*lot_road_access)(void* user, uint64_t occ); /* -1 = no lot, 0 = no road, 1 = road */
    /* city, simulator, zones, terrain, disasters */
    uint32_t (*sim_day)(void* user);
    bool (*lot_location)(void* user, uint64_t unknown, int32_t* x, int32_t* z); /* false = not a lot */
    void (*cell_center_to_position)(void* user, int32_t cell_x, int32_t cell_z, float* x, float* z);
    uint32_t (*zone_value)(void* user, int32_t cell_x, int32_t cell_z);
    float (*sea_level)(void* user);
    float (*altitude)(void* user, float x, float z);
    bool (*disaster_ongoing)(void* user);
    /* dispatch manager (§5, §8): dispatch_ids writes at most `capacity` IDs
     * (never called with 0) and returns the number written. */
    uint32_t (*dispatch_count)(void* user, uint32_t dispatch_type);
    uint32_t (*dispatch_ids)(void* user, uint32_t dispatch_type, uint32_t* ids, uint32_t capacity);
    bool (*dispatch_info)(void* user, uint32_t id, fc4_protection_dispatch_info* out);
    /* automata controllers (§6.3, §9.5) */
    void (*add_controllers)(void* user, const char* group, size_t group_len, uint64_t occ);
    void (*remove_controllers)(void* user, const char* group, size_t group_len, uint64_t occ);
    /* other simulators, history, messages */
    bool (*police_percent_coverage)(void* user, float* out); /* false = no police simulator */
    void (*record_history)(void* user, uint32_t id, int32_t value);
    void (*post)(void* user, uint32_t msg_type, uint64_t subject); /* §12; subject 0 = none */
    /* air dispatch sources (§8.4): one reference for the core, 0 = none */
    uint64_t (*create_air_source)(void* user, uint32_t kind, uint32_t* class_id);
    void (*air_source_init)(void* user, uint64_t source);
    void (*air_source_shutdown)(void* user, uint64_t source);
} fc4_protection_host;

/* Which sub-object QueryInterface(iid) answers with (§1): 0 = none, 1 = the
 * primary interface (also cIGZUnknown), 2 = cIGZMessageTarget2, 3 = the
 * dispatch source (0xC9C00435), 4 = cIGZSerializable. */
uint32_t fc4_protection_query_interface(uint32_t kind, uint32_t iid);
fc4_protection* fc4_protection_create(const fc4_protection_host* host, uint32_t kind, uint64_t token,
                                      uint32_t wall_clock_seed, uint32_t process_clock);
void fc4_protection_destroy(fc4_protection* p);
bool fc4_protection_init(fc4_protection* p);     /* §4.1 */
bool fc4_protection_shutdown(fc4_protection* p); /* §4.2 */
bool fc4_protection_do_message(fc4_protection* p, const fc4_protection_message* m); /* §5: always true */
/* Primary interfaces (§15.1 fire slot / §15.2 police slot). List getters call
 * `cb` once per object (borrowed tokens; the caller AddRefs what it keeps). */
uint64_t fc4_protection_grid(const fc4_protection* p);            /* 5: borrowed, 0 = none */
uint64_t fc4_protection_department(const fc4_protection* p);      /* 6: borrowed */
uint64_t fc4_protection_jail_department(const fc4_protection* p); /* police 9: borrowed */
bool fc4_protection_is_station(const fc4_protection* p, uint64_t occ); /* 7 / 22 */
bool fc4_protection_station_funding(const fc4_protection* p, uint64_t occ, int32_t* cost, float* pct); /* 8 / 7 */
bool fc4_protection_set_station_funding(fc4_protection* p, uint64_t occ, float pct);                  /* 9 / 8 */
bool fc4_protection_protection_station_funding(const fc4_protection* p, uint64_t occ, float* out); /* MT 11 */
float fc4_protection_station_efficiency(const fc4_protection* p, uint64_t occ); /* 10 / 25 */
bool fc4_protection_is_on_strike(const fc4_protection* p);                      /* 11 / 10 */
float fc4_protection_chance_of_strike(const fc4_protection* p);                 /* 12 / 11 */
bool fc4_protection_begin_strike(fc4_protection* p, uint64_t occ);              /* 13 / 12 */
bool fc4_protection_end_strike(fc4_protection* p);                              /* 14 / 13 */
uint32_t fc4_protection_strike_start_date(const fc4_protection* p);             /* 15 / 14 */
uint32_t fc4_protection_strike_length(const fc4_protection* p);                 /* 16 / 15 */
/* 17 / 16: appends (no clear); returns the number appended. */
int32_t fc4_protection_striking_stations(const fc4_protection* p, fc4_token_callback cb, void* ctx);
bool fc4_protection_is_station_on_strike(const fc4_protection* p, uint64_t occ); /* 18 / 17 */
float fc4_protection_average_coverage(const fc4_protection* p);                  /* 19 / 18 */
float fc4_protection_percent_coverage(const fc4_protection* p); /* 20 / 19: the value behind the pointer */
/* 21 / 20 (§7.3): fire zeroes both outputs first; police leaves them when it
 * fails. */
bool fc4_protection_occupant_coverage(const fc4_protection* p, uint64_t occ, float pct, float* cover_radius,
                                      float* station_radius);
/* 22 / 21 and dispatch-source 6: the caller clears its list first. */
uint32_t fc4_protection_stations(const fc4_protection* p, fc4_token_callback cb, void* ctx);
uint32_t fc4_protection_station_count(const fc4_protection* p);                     /* 23 / 24 */
float fc4_protection_station_dispatch_radius(const fc4_protection* p, uint64_t occ); /* 24 / 23 */
int32_t fc4_protection_total_dispatch_count(const fc4_protection* p);              /* 25 / 26 */
/* Police jails (§9, §15.2). */
int32_t fc4_protection_reset_unjailed_criminals(fc4_protection* p);              /* 27 */
uint32_t fc4_protection_jail_capacity_of(const fc4_protection* p, uint64_t occ); /* 28 */
uint32_t fc4_protection_jail_capacity(const fc4_protection* p);                  /* 29 */
uint32_t fc4_protection_jail_population(const fc4_protection* p);                /* 30 */
uint32_t fc4_protection_criminal_count(const fc4_protection* p);                 /* 31 */
uint32_t fc4_protection_jails(const fc4_protection* p, fc4_token_callback cb, void* ctx); /* 32: clear first */
bool fc4_protection_is_jail(const fc4_protection* p, uint64_t occ);              /* 33 */
uint32_t fc4_protection_arrest_count(const fc4_protection* p);                   /* 34 */
uint32_t fc4_protection_jail_count(const fc4_protection* p);                     /* 35 */
float fc4_protection_jailed_percentage(const fc4_protection* p);                 /* 36 */
int64_t fc4_protection_jail_upkeep_cost(const fc4_protection* p, uint64_t occ);  /* 37 */
bool fc4_protection_process_monthly_crime_stats(fc4_protection* p);              /* 38 */
bool fc4_protection_is_in_jailbreak(const fc4_protection* p, uint64_t occ);      /* 39 */
bool fc4_protection_begin_jailbreak(fc4_protection* p, uint64_t occ);            /* 40 */
bool fc4_protection_end_jailbreak(fc4_protection* p, uint64_t occ);              /* 41 */
/* 42: 0 and no callback when empty (the caller's list stays as it is); else
 * the caller clears its list first. */
int32_t fc4_protection_jailbreak_buildings(const fc4_protection* p, fc4_token_callback cb, void* ctx);
uint32_t fc4_protection_station_arrest_count(const fc4_protection* p, uint64_t occ); /* 43 */
uint32_t fc4_protection_station_crime_count(const fc4_protection* p, uint64_t occ);  /* 44 */
/* The dispatch source (§8, §15.4 slot). */
bool fc4_protection_can_handle_dispatch_type(const fc4_protection* p, uint32_t t);           /* 3 */
bool fc4_protection_can_dispatch(const fc4_protection* p, float x, float z, bool* all_busy); /* 4 */
int32_t fc4_protection_max_possible_dispatch_count(const fc4_protection* p);                /* 5 */
/* 7: NULL flags / base = not requested; each output written as soon as its
 * property is read (§8.1). */
bool fc4_protection_dispatches_at_station(const fc4_protection* p, uint64_t occ, uint32_t* flags, uint32_t* base,
                                          uint32_t* count);
uint64_t fc4_protection_dispatch_from_nearest_station(fc4_protection* p, float x, float z, uint32_t* id); /* 8 */
bool fc4_protection_dispatch_from_station(fc4_protection* p, uint64_t occ, uint32_t* id);               /* 9 */
bool fc4_protection_revoke_dispatch_from_station(fc4_protection* p, uint64_t occ, uint32_t id);         /* 10 */
bool fc4_protection_dispatch_automata_properties(const fc4_protection* p, uint32_t* b, float* c3, uint32_t* d);
bool fc4_protection_get_state(const fc4_protection* p, fc4_protection_state* out);
bool fc4_protection_write(const fc4_protection* p, const fc4_ostream* out); /* §11 */
bool fc4_protection_read(fc4_protection* p, const fc4_istream* in);         /* §11 */
/* The grid shadow: a handle on a read-only host table (grids unused), never
 * initialised. fc4_protection_shadow_run reads the tuning, takes the
 * original's stations (list order), department and strike flag as BORROWED
 * tokens (valid for this call and later queries on the handle until the next
 * run), rebuilds the unadjusted grid (§5.1) in a core grid of geometry `geom`
 * and derives the adjusted grid (§2, factor 1.0). fc4_protection_shadow_grid
 * copies the adjusted grid (x outer: x * count_z + z) and returns the number
 * of tracts; fc4_protection_shadow_unadjusted_grid likewise the unadjusted
 * int16 grid (PR5: the original keeps its own at primary +0x84). Destroy
 * with fc4_protection_destroy (releases nothing). */
fc4_protection* fc4_protection_shadow_create(const fc4_protection_host* host, uint32_t kind);
bool fc4_protection_shadow_run(fc4_protection* p, const fc4_grid_geometry* geom, const uint64_t* stations,
                               size_t count, uint64_t department, bool strike_flag);
size_t fc4_protection_shadow_grid(const fc4_protection* p, uint8_t* out, size_t n);
size_t fc4_protection_shadow_unadjusted_grid(const fc4_protection* p, int16_t* out, size_t n); /* ABI 27 */

/* ---- crime (docs/specs/crime.md) ---------------------------------------------
 *
 * Tokens: lots, building occupants, occupants and grids (the game objects'
 * identities); 0 is null. Both crime grids are the game's own
 * cSC4SimGrid<uint8> (fc4_grid_api); the population grids are uint16 and the
 * EQ grids float ones.
 *
 * The crime type list and the generator are process-wide (§7, §9): the first
 * fc4_crime_create seeds the generator with derive_seed(session_seed,
 * 0x6C09C3F6) (foundations §6.3); later objects share both. The original's
 * session value is cRZTime, u32 seconds since 1901-01-01 00:00 UTC
 * (time() + 2177452800, §9 / CQ5), the value every clock-seeded generator
 * shares (CQ21; fc4_core::rng::rz_time_from_unix).
 *
 * Threads (foundations §19): the game calls getters from a second thread while
 * the main thread is inside a call. The functions marked "any thread" read a
 * snapshot the main thread publishes at the end of every outermost call and
 * never touch the core; every other function is main-thread only. A message
 * delivered while another call into the same object is running is queued and
 * handled when that call returns. */
typedef struct fc4_crime fc4_crime;

/* A message for fc4_crime_do_message (§5). */
typedef struct fc4_crime_message {
    uint32_t msg_type;
    uint32_t data1;    /* GetData1() (exemplar changed: the group) */
    uint32_t data2;    /* GetData2() (exemplar changed: the instance) */
    uint32_t reserved;
    uint64_t occupant; /* GetVoid1() as a cISC4Occupant (inserted / removed) */
} fc4_crime_message;

/* One element of GetCrimeIncidentArray: the game's 28-byte tCrimeEvent
 * (§11.4) with `has_lot` in a padding byte. has_lot = 0: an incident without a
 * lot; the game's element keeps its previous content. */
typedef struct fc4_crime_event {
    uint32_t cell_x, cell_z;
    uint32_t type_id;
    uint8_t visuals_created;
    uint8_t has_lot;
    uint8_t pad[2];
    uint32_t icon_type, icon_group, icon_instance;
} fc4_crime_event;

/* One occupant of the bounding-box query of §11.3. */
typedef struct fc4_crime_nearby {
    uint64_t token;
    uint32_t occupant_type;
    uint8_t in_crime_group;        /* sims: IsOccupantGroup(0x4102) */
    uint8_t has_incident_property; /* props: HasProperty(0x8AA864D5) */
    uint8_t answers_iid;           /* the kind's QueryInterface succeeded */
    uint8_t pad;
    float position[3];
    uint32_t prop_state; /* props: GetState() */
} fc4_crime_nearby;
typedef void (*fc4_crime_nearby_callback)(void* ctx, const fc4_crime_nearby* o);

#define FC4_CRIME_NEAREST_SIM 0u
#define FC4_CRIME_NEAREST_PROP 1u

/* Diagnostics. */
typedef struct fc4_crime_state {
    uint32_t struct_size;
    uint8_t initialized, loaded_from_save, has_message_server, pad0;
    uint32_t pipeline_state;
    uint32_t monthly_crime_count;
    uint32_t average_bits; /* the stored float32 (NaN 0xFFC00000 when unzoned) */
    int32_t hotspot_x, hotspot_z, hotspot_value;
    float ordinance_factor;
    int32_t criminal_base, crimes_to_commit;
    uint32_t incidents;
    uint32_t types; /* the process-wide type list's length (0 = not loaded) */
    uint32_t tract_size;
    float decay, reoffend;
    uint32_t census[12]; /* pop[wealth][bucket] (§6.1), wealth-major */
    float jobless[3];
    uint32_t incident_epoch, reentrant_calls, reserved;
    uint64_t crime_grid, effect_grid;
} fc4_crime_state;

/* What the crime simulator needs from the game (one callback per CrimeHost
 * method, engine/core/src/crime/mod.rs, §13). Missing callbacks behave as
 * "nothing there" (0, false, no object). */
typedef struct fc4_crime_host {
    uint32_t struct_size; /* sizeof(fc4_crime_host) */
    void* user;
    fc4_grid_api grids;
    /* lifecycle glue (§4) */
    void (*add_agent)(void* user, const uint8_t* name, size_t len, const uint32_t* levels, uint32_t count,
                      uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, const uint32_t* msg_types, uint32_t count);
    void (*unsubscribe)(void* user, const uint32_t* msg_types, uint32_t count);
    void (*acquire_services)(void* user);
    void (*release_services)(void* user);
    void (*add_prop_requester)(void* user);
    /* resources (§7, §8) */
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL if missing */
    void (*for_each_type_key)(void* user, fc4_key_visitor visit, void* ctx);
    fc4_props* (*exemplar)(void* user, uint32_t type_id, uint32_t group, uint32_t instance); /* NEW or NULL */
    /* A crime type name, §7 steps 2-3 (CQ8): type_id is always 0x2026960B.
     * Look up (type, group + language ID, instance), then (type, group,
     * instance) if that is missing and the language ID is not 0; copy the text
     * and detokenize it in place (skipped with no detokenizer). true: loaded,
     * the text went to the sink. false and text sent to the sink: the
     * detokenizer returned false. false and nothing sent: no such LTEXT. A
     * failed load discards any text sent: the name is empty unless 0xAA12FCB0
     * exists (CQ16). The core replaces U+2022 itself (step 4). */
    bool (*ltext)(void* user, uint32_t type_id, uint32_t group, uint32_t instance, fc4_bytes_sink sink, void* ctx);
    /* residential and demand (§6.1, §6.2): three grid tokens, one reference each */
    void (*population_grids)(void* user, uint64_t* out3);
    void (*eq_grids)(void* user, uint64_t* out3);
    bool (*demand)(void* user, uint32_t id, float* value, float* max, float* min);
    /* police and ordinances (§6.4, §6.6) */
    uint64_t (*police_grid)(void* user); /* one reference handed over, 0 = none */
    uint32_t (*jail_population)(void* user);
    int32_t (*reset_unjailed_criminals)(void* user);
    void (*process_monthly_crime_stats)(void* user);
    bool (*ordinance_crime_factor)(void* user, float* out);
    /* lots and zones (§6.6-§6.9, §11): lot tokens are borrowed */
    uint64_t (*random_lot)(void* user);
    uint64_t (*lot_at)(void* user, int32_t cell_x, int32_t cell_z);
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    bool (*lot_location)(void* user, uint64_t lot, int32_t* x, int32_t* z);
    uint8_t (*lot_condition)(void* user, uint64_t lot);
    bool (*lot_building)(void* user, uint64_t lot, uint64_t* building, uint32_t* purpose, uint32_t* wealth);
    void (*lot_center_cell)(void* user, uint64_t lot, int32_t* x, int32_t* z);
    uint32_t (*zone_type)(void* user, int32_t cell_x, int32_t cell_z);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* occupants and props (§6.10, §11.3, §11.6) */
    bool (*occupant_has_property)(void* user, uint64_t occ, uint32_t id);
    /* The raw storage of an array property (count x element size bytes, the
     * variant's RefUint8 data whatever its type, no conversion; §6.10, CQ13;
     * a string is its stored bytes, "AB" gives m = 65, r = 66, CQ22); false
     * if absent, a scalar or an object array (CQ22 [choice]). */
    bool (*occupant_property_bytes)(void* user, uint64_t occ, uint32_t id, fc4_bytes_sink sink, void* ctx);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    void (*occupants_in_box)(void* user, const float* min3, const float* max3, uint32_t kind,
                             fc4_crime_nearby_callback cb, void* ctx);
    uint32_t (*prop_crime_incident_id)(void* user, uint32_t prop);
    /* arson (§6.8) */
    bool (*disaster_layer_present)(void* user);
    void (*ignite)(void* user, uint64_t building, uint32_t cause);
    /* messages (§10) */
    void (*post_crime_committed)(void* user, uint32_t type_id, uint64_t lot, const uint8_t* name, size_t len);
    void (*post_crime_map_updated)(void* user);
} fc4_crime_host;

/* The host table is copied; `token` is the simulator's own identity. */
fc4_crime* fc4_crime_create(const fc4_crime_host* host, uint64_t token, uint32_t session_seed);
void fc4_crime_destroy(fc4_crime* p);
bool fc4_crime_init(fc4_crime* p);     /* §4.1 */
bool fc4_crime_shutdown(fc4_crime* p); /* §4.2 */
/* §5: always true for a live object; queued during another call. */
bool fc4_crime_do_message(fc4_crime* p, const fc4_crime_message* m);
/* Any thread (the published snapshot). */
uint64_t fc4_crime_grid(const fc4_crime* p);          /* slot 5: borrowed token, 0 = none */
uint32_t fc4_crime_average(const fc4_crime* p);       /* slot 6 */
uint32_t fc4_crime_monthly_count(const fc4_crime* p); /* slot 7 */
/* Slot 8: false and nothing written when the value is 0 or less. */
bool fc4_crime_hotspot(const fc4_crime* p, uint8_t* value, int32_t* x, int32_t* z);
uint32_t fc4_crime_incident_count(const fc4_crime* p);
/* Slot 11: copies up to `capacity` events, returns the incident count. */
uint32_t fc4_crime_incidents(const fc4_crime* p, fc4_crime_event* out, uint32_t capacity);
bool fc4_crime_set_visuals(const fc4_crime* p, uint32_t index, bool value); /* slot 12 */
bool fc4_crime_get_visuals(const fc4_crime* p, uint32_t index);             /* slot 13 */
/* Slots 9/10 (§11.3): the selection over the occupants the box query
 * returned, in its order: the index of the candidate, or -1. Pure. */
int32_t fc4_crime_nearest_of(uint32_t kind, const float* pos3, float radius, const fc4_crime_nearby* occupants,
                             uint32_t count);
/* Main thread. Prop requester slot 3 (rect x1, z1, x2, z2) and slot 6 (0 = none). */
int32_t fc4_crime_requested_share(const fc4_crime* p, int32_t x1, int32_t z1, int32_t x2, int32_t z2);
uint32_t fc4_crime_select_prop(const fc4_crime* p, const uint32_t* props, uint32_t count, int32_t cell_x,
                               int32_t cell_z);
bool fc4_crime_get_state(const fc4_crime* p, fc4_crime_state* out);
bool fc4_crime_write(const fc4_crime* p, const fc4_ostream* out); /* §12 */
bool fc4_crime_read(fc4_crime* p, const fc4_istream* in);         /* §12: false for version != 6 */

/* The crime shadow (tooling, engine/plugin/FC4CrimeShadow.h): the tuning and
 * the type list read through a host table of which only tuning,
 * for_each_type_key, exemplar and ltext are called, and the replay of
 * pipeline state 4's grid work on grids read from the original simulator. */
typedef struct fc4_crime_shadow fc4_crime_shadow;

typedef struct fc4_crime_shadow_info {
    uint32_t struct_size;
    uint32_t types;
    uint8_t tuning_found;
    uint8_t pad[3];
    uint32_t tract_size;
    float decay, reoffend;
} fc4_crime_shadow_info;

/* One incident to replay: its lot's bounding rect (has_rect = 0: no lot) and type. */
typedef struct fc4_crime_shadow_incident {
    fc4_cell_rect rect;
    uint32_t type_id;
    uint8_t has_rect;
    uint8_t pad[3];
} fc4_crime_shadow_incident;

typedef struct fc4_crime_shadow_month_in {
    uint32_t struct_size;
    fc4_grid_geometry geometry;
    const uint8_t* pre; /* count_x * count_z values, x outer: the grid before the step */
    int32_t hotspot_x, hotspot_z, hotspot_value; /* the hotspot before the step (tracts) */
    const fc4_crime_shadow_incident* incidents;
    uint32_t incident_count;
    const uint32_t* zones; /* count_x * count_z corner-cell zone types, x outer; NULL = no average */
} fc4_crime_shadow_month_in;

typedef struct fc4_crime_shadow_month_out {
    uint32_t struct_size;
    int32_t hotspot_x, hotspot_z, hotspot_value;
    uint32_t count;         /* incidents stamped (severity > 0) */
    uint32_t unknown_types; /* incidents of a type not in the list */
    uint32_t no_rect;       /* counted incidents with no rect (not stamped) */
    uint32_t severity_sum;
    uint32_t average_bits; /* the stored average's float32 bits, 0 when zones was NULL */
    uint8_t average_computed;
    uint8_t pad[3];
} fc4_crime_shadow_month_out;

fc4_crime_shadow* fc4_crime_shadow_create(const fc4_crime_host* host);
void fc4_crime_shadow_destroy(fc4_crime_shadow* s);
bool fc4_crime_shadow_get_info(const fc4_crime_shadow* s, fc4_crime_shadow_info* out);
/* §6.7: 1 = a candidate, 0 = not, -1 = unknown type. */
int32_t fc4_crime_shadow_candidate(const fc4_crime_shadow* s, uint32_t type_id, uint32_t zone_type, bool has_building,
                                   uint32_t purpose, uint32_t wealth, uint8_t condition);
/* Decay, the hotspot re-read, each incident stamped in order, the zoned
 * average; `post` receives count_x * count_z values. */
bool fc4_crime_shadow_month(const fc4_crime_shadow* s, const fc4_crime_shadow_month_in* in, uint8_t* post,
                            fc4_crime_shadow_month_out* out);

/* ---- aura (docs/specs/aura.md) ---------------------------------------------
 *
 * cSC4AuraSimulator (0x0990BDD8): the monthly time-sliced pipeline (§6), the
 * occupant effects (§7), the getters (§8) and the save record (§11). The host
 * table has one callback per AuraHost method (engine/core/src/aura/mod.rs)
 * except the aura's own grids, which are the game's cSC4SimGrid<int8> /
 * <int16> through `grids` (int8 0x49B9E603, int16 0x49B9E605; the map view
 * reads them as raw memory, §14.2). The other simulators' grids are read
 * through `input_grid` (a BORROWED token per FC4_AURA_INPUT_*, valid until the
 * outermost call into the core returns) and `grids.geometry` / `grids.get`
 * with the grid's element class (pollution int16, grade maps float, crime /
 * trip / congestion uint8, population uint16). Occupants are tokens (their
 * cISC4Occupant identity); 0 is null. Missing callbacks read as "nothing
 * there" (0, false, none).
 *
 * Threads: the getters (fc4_aura_ratings, fc4_aura_grid) may be called from
 * any thread, also during a main-thread call: they read a snapshot published
 * after every main-thread call, never the core. Every other function is
 * main-thread. A message, transient effect or strike buster delivered while
 * another main-thread call is running is queued and handled when that call
 * returns (fc4_aura_state.reentrant_calls). */
typedef struct fc4_aura fc4_aura;

/* `which` of input_grid. */
#define FC4_AURA_INPUT_AIR        0u
#define FC4_AURA_INPUT_WATER      1u
#define FC4_AURA_INPUT_GARBAGE    2u
#define FC4_AURA_INPUT_RADIATION  3u
#define FC4_AURA_INPUT_SCHOOL     4u  /* residential slot 39, cISC4SimGrid<float> */
#define FC4_AURA_INPUT_HOSPITAL   5u  /* residential slot 40, cISC4SimGrid<float> */
#define FC4_AURA_INPUT_CRIME      6u
#define FC4_AURA_INPUT_TRIP       7u
#define FC4_AURA_INPUT_CONGESTION 8u
#define FC4_AURA_INPUT_POP1       9u  /* GetPopulationGrids (slot 25): R$, R$$, R$$$ */
#define FC4_AURA_INPUT_POP2       10u
#define FC4_AURA_INPUT_POP3       11u
#define FC4_AURA_INPUT_COUNT      12u

/* `which` of fc4_aura_grid (slots 12-15). */
#define FC4_AURA_GRID_AURA      0u
#define FC4_AURA_GRID_TRANSIENT 1u
#define FC4_AURA_GRID_PARK      2u
#define FC4_AURA_GRID_LANDMARK  3u

/* A lot (cISC4Lot): GetBoundingRect (inclusive cells), GetZoneType,
 * GetOccupantWealth, GetBuilding as a cISC4Occupant token (borrowed; 0 none). */
typedef struct fc4_aura_lot {
    int32_t x1, z1, x2, z2;
    uint32_t zone_type;
    int32_t occupant_wealth;
    uint64_t building;
} fc4_aura_lot;

/* One (developer type, count) pair of GetPopulations (slot 54): every entry
 * of every map, counts of 0 included (aura.md 6.5.1). */
typedef void (*fc4_aura_population_sink)(void* ctx, uint32_t developer_type, int32_t count);

/* What any thread may read: slots 8-10 and the four grid tokens of slots
 * 12-15 (borrowed; 0 = none), as of the last main-thread call. */
typedef struct fc4_aura_ratings {
    uint32_t struct_size;
    int8_t mayor_rating, max_rating, min_rating, pad;
    int32_t max_x, max_z, min_x, min_z; /* cells */
    uint32_t reserved[2];
    uint64_t aura_grid, transient_grid, park_map, landmark_map;
} fc4_aura_ratings;

/* Diagnostics (main thread). */
typedef struct fc4_aura_state {
    uint32_t struct_size;
    uint8_t initialized, re_add_landmarks, has_lot_iterator, pad0;
    int32_t state; /* pipeline state 0..17 (§6) */
    uint32_t cycle_start_day;
    int32_t mayor_rating, unused_rating, global_effect;
    int16_t max_rating, min_rating;
    int32_t max_x, max_z, min_x, min_z;
    int32_t effective_base_aura[3];
    uint32_t hotspots, low_tracts, departments;
    float riot_roll;
    uint32_t slice_step;
    int32_t slice_miss;
    uint32_t aura_tracts; /* over the aura grid: count, sum, min, max */
    int32_t aura_sum, aura_min, aura_max;
    uint32_t base_tracts; /* base tracts with data (!= -128) */
    uint32_t reentrant_calls;
    float long_decay, short_decay;
    uint32_t reserved;
} fc4_aura_state;

typedef struct fc4_aura_host {
    uint32_t struct_size; /* sizeof(fc4_aura_host) */
    void* user;
    fc4_grid_api grids;
    /* city and simulator */
    void (*cell_width)(void* user, float* x, float* z);                             /* city 91 / 92 */
    void (*position_to_cell)(void* user, float x, float z, int32_t* cx, int32_t* cz); /* city 95 */
    uint32_t (*sim_date_number)(void* user);                                        /* simulator 8 */
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL if missing */
    void (*publish_base_aura)(void* user, const int32_t* base_aura3); /* §10: R$, R$$, R$$$ */
    /* lots */
    uint32_t (*lot_count)(void* user);
    uint64_t (*lot_iterator)(void* user); /* a fresh iterator (0 = none) */
    /* The iterator's next lot and its population pairs; false at the end. */
    bool (*lot_next)(void* user, uint64_t it, fc4_aura_lot* out, fc4_aura_population_sink sink, void* ctx);
    void (*lot_iterator_release)(void* user, uint64_t it);
    bool (*lot_at)(void* user, int32_t x, int32_t z, fc4_aura_lot* out); /* GetLot(x, z, false) */
    /* GetLandValueType(x, z); negative = no land value simulator (aura.md 6.5.1,
     * AU10: the value/wealth term is skipped); NULL reads as none */
    int32_t (*land_value_type)(void* user, int32_t x, int32_t z);
    /* occupants */
    bool (*occupant_has_property)(void* user, uint64_t occ, uint32_t id);
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    uint32_t (*occupant_type)(void* user, uint64_t occ);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    bool (*budget_item_department)(void* user, uint64_t occ, uint32_t purpose, uint32_t* department);
    /* budget, ordinances */
    bool (*department_funding)(void* user, uint32_t department, float* fraction);
    float (*tax_rate)(void* user, int32_t index);
    int32_t (*cumulative_offset)(void* user, uint32_t prop, uint32_t index);
    /* the other simulators' grids (FC4_AURA_INPUT_*): a borrowed token, 0 = none */
    uint64_t (*input_grid)(void* user, uint32_t which);
    /* effects on the game */
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*attempt_disaster_start)(void* user, uint32_t manager, int32_t x, int32_t z); /* cells, result ignored */
    void (*end_strike)(void* user, uint32_t which); /* 0 police, 1 fire, 2 health, 3 school (§8.4) */
    void (*post)(void* user, uint32_t msg_type);    /* queued; void1 = IGZUnknown = the simulator */
    /* CPU-cycle limit timer (simulator §5.5); none = never expires */
    void (*limit_timer_start)(void* user, uint32_t ms);
    bool (*limit_timer_expired)(void* user);
    /* framework plumbing (§4.1, §4.2) */
    void (*subscribe)(void* user, const uint32_t* ids, uint32_t count);
    void (*unsubscribe)(void* user, const uint32_t* ids, uint32_t count);
    void (*add_agent)(void* user, uint32_t level, const char* name, uint32_t flags);
    void (*remove_agent)(void* user);
    bool (*registry_add)(void* user); /* §4.3; false = not published (not an error) */
    void (*registry_remove)(void* user);
} fc4_aura_host;

/* The host table is copied; `token` is the simulator's own identity; the two
 * clock values seed its cRZRandom (§9). NULL for a short table. */
fc4_aura* fc4_aura_create(const fc4_aura_host* host, uint64_t token, uint32_t wall_clock_seed,
                          uint32_t process_clock);
void fc4_aura_destroy(fc4_aura* p); /* releases the grids and a held lot iterator */
bool fc4_aura_init(fc4_aura* p);     /* §4.1 */
bool fc4_aura_shutdown(fc4_aura* p); /* §4.2 */
/* §5: true except for a late SimNewDay; void1 = the occupant token of the
 * occupant messages (keep it alive until the outermost call returns when this
 * is called from inside another call: it is queued). */
bool fc4_aura_do_message(fc4_aura* p, uint32_t msg_type, uint32_t data1, uint32_t data2, uint32_t data3,
                         uint64_t void1);
void fc4_aura_add_transient_effect_type(fc4_aura* p, float x, float z, uint32_t type); /* slot 6, §8.1 */
void fc4_aura_add_transient_effect(fc4_aura* p, float x, float z, float radius, float magnitude); /* slot 7 */
void fc4_aura_bust_strike(fc4_aura* p, uint32_t type);                                          /* slot 11 */
/* Any thread: the snapshot (false for NULL), one grid token of it. */
bool fc4_aura_get_ratings(const fc4_aura* p, fc4_aura_ratings* out);
uint64_t fc4_aura_grid(const fc4_aura* p, uint32_t which);
bool fc4_aura_get_state(const fc4_aura* p, fc4_aura_state* out);
bool fc4_aura_write(const fc4_aura* p, const fc4_ostream* out); /* §11.2 */
bool fc4_aura_read(fc4_aura* p, const fc4_istream* in);         /* §11.3; Read runs before Init */

/* ---- aura shadow (tooling; engine/capi/src/aura_shadow.rs) -----------------
 *
 * With the ORIGINAL aura simulator running, FreeCity4's pipeline follows it
 * step by step (aura.md §14.3). The plugin reads the original's state (s32
 * at +0x70) before and after each SimIdle / SimNewDay / SimNewMonth delivery
 * to its agent; when it moved, fc4_aura_shadow_step runs our steps from..to-1
 * at once (a return to 0: to the end of our cycle), reading every input live
 * through a read-only host table (write callbacks NULL; grids: geometry and
 * get only). Occupant inserted / removed reach ours when the original
 * receives them (fc4_aura_shadow_occupant). When the original's state
 * returns to 0 the plugin captures its record (its own Write into memory:
 * primitives little-endian, references as u32 index + 1 (0 = null) then u32
 * class ID, each object listed once with its grid token) and
 * fc4_aura_shadow_cycle compares it tract by tract with ours (base, aura,
 * park map, landmark map, transient; the work grid after each step 5..13;
 * mayor rating, best / worst, hotspots); ours then restarts from the record.
 * The riot roll is not compared. */
typedef struct fc4_aura_shadow fc4_aura_shadow;

typedef struct fc4_aura_capture_ref {
    uint64_t grid;     /* as cISC4SimGrid (IID 0xC9BD939A), borrowed; 0 = not a grid */
    uint32_t class_id; /* GetGZCLSID() */
    uint32_t reserved;
} fc4_aura_capture_ref;

typedef struct fc4_aura_capture {
    uint32_t struct_size;
    uint32_t ok; /* non-zero: Write returned true and the stream stayed good */
    const uint8_t* bytes;
    size_t len;
    const fc4_aura_capture_ref* refs;
    size_t ref_count;
} fc4_aura_capture;

typedef struct fc4_aura_shadow_stats {
    uint32_t struct_size;
    uint32_t cycles, compared, clean, incomplete;
    int32_t last_mismatches; /* fc4_aura_shadow_cycle's result for the last cycle; -1 = not compared */
    uint32_t rating_mismatches;
    uint32_t desynced; /* compared cycles in which our state did not follow the original's */
    uint64_t tracts, base_mismatches, aura_mismatches;
    uint64_t aura_transient; /* aura differences in tracts whose final transient differs */
} fc4_aura_shadow_stats;

/* One move of the original's state, in one delivery to its agent (ABI 30). */
typedef struct fc4_aura_shadow_step_in {
    uint32_t struct_size;
    int32_t from, to;   /* the original's state before / after (0..17) */
    int32_t julian;     /* the current date */
    uint64_t work_grid; /* the original's work grid (+0x44) as cISC4SimGrid<int8>, borrowed; 0 = not compared */
} fc4_aura_shadow_step_in;

/* `cells_x` / `cells_z`: the city's cell counts; `max_cells`: differing tracts
 * listed per grid and line (0 = 4, at most 64). */
fc4_aura_shadow* fc4_aura_shadow_create(const fc4_aura_host* host, int32_t cells_x, int32_t cells_z,
                                        uint32_t max_cells);
void fc4_aura_shadow_destroy(fc4_aura_shadow* s);
/* City load (PostCityInit): ours starts from the record (Read, then Init:
 * state 1, as the original's), with a fresh tax baseline. One line. */
bool fc4_aura_shadow_baseline(fc4_aura_shadow* s, const fc4_aura_capture* capture, fc4_bytes_sink sink, void* ctx);
/* ABI 30. Before a delivery that finds the original in `state`: for 15 and
 * 17 (they read the transient grid; 17 decays it) the original's transient
 * grid (GetTransientAuraGrid, borrowed) is copied into ours. True when copied. */
bool fc4_aura_shadow_before_step(fc4_aura_shadow* s, int32_t state, uint64_t transient_grid);
/* ABI 30. The original's state moved: our steps from..to-1 run now (to 0: to
 * the end of our cycle), and our work grid is compared after a step 5..13.
 * Returns our pipeline calls, -1 when nothing ran. */
int32_t fc4_aura_shadow_step(fc4_aura_shadow* s, const fc4_aura_shadow_step_in* step);
/* ABI 30. The original received occupant inserted (`inserted`) or removed:
 * ours receives the same. False when there is no simulator of ours yet. */
bool fc4_aura_shadow_occupant(fc4_aura_shadow* s, uint64_t occupant, bool inserted);
/* The original's state returned to 0: the cycle followed is compared with
 * its record, one line; ours then restarts from the record (state 0).
 * Returns the differences (base + aura + park map + landmark map + transient
 * tracts, work cells, one each for the rating, best / worst and hotspots),
 * -1 when nothing was compared. */
int32_t fc4_aura_shadow_cycle(fc4_aura_shadow* s, const fc4_aura_capture* capture, int32_t julian,
                              fc4_bytes_sink sink, void* ctx);
bool fc4_aura_shadow_get_stats(fc4_aura_shadow* s, fc4_aura_shadow_stats* out);

/* ---- flora (docs/specs/flora.md) -------------------------------------------
 * Two objects. fc4_flora_registry is the process-wide flora type registry
 * (§3.2): the host creates ONE per process and hands it to every simulator.
 * For each record the game can see it holds the game-layout block the host
 * built (docs/abi/layouts.md "Static flora record"; GetStaticFloraData
 * returns it); the host keeps one reference on every block for the life of
 * the process. fc4_flora is one cISC4FloraSimulator.
 *
 * Threads: every fc4_flora_* call runs under the registry's process-wide
 * re-entrant lock (one thread at a time; the owner may re-enter from a host
 * callback). The fc4_flora_registry_* getters (slots 7-14) never take it:
 * they read a snapshot, refreshed after every call that can change the
 * registry, so a second thread never waits for a month in progress.
 * A message delivered during another call is queued and handled when the
 * outermost call returns. Tokens are opaque host identities, 0 = null. */
typedef struct fc4_flora_registry fc4_flora_registry;
typedef struct fc4_flora fc4_flora;

/* A static flora record's values (§3.3), in the game layout's order. */
typedef struct fc4_flora_record {
    uint32_t type_id, next_type, seed_type;
    uint8_t water, flags, reserved[2];
    float floats[14]; /* property 0x083DC35B, in order */
    float cluster_radius, cluster_height;
    uint8_t table[256]; /* [elevation index][moisture type] */
    uint32_t unnamed, family_count;
    int64_t plop_cost;
    const uint32_t* families; /* family_count IDs, borrowed for the call (NULL when 0) */
} fc4_flora_record;

/* A flora occupant's values (§3.4). */
typedef struct fc4_flora_occupant {
    uint32_t flora_type, birth_date, last_seeding_date;
    int8_t state; /* signed: 0 young .. 4 dying */
    uint8_t orientation, reserved[2];
} fc4_flora_occupant;

typedef struct fc4_flora_param {
    uint32_t as_u32;
    float as_f32;
} fc4_flora_param;

/* A message for fc4_flora_do_message (§5). */
typedef struct fc4_flora_message {
    uint32_t msg_type, data1, data2, data3;
    uint64_t occupant;            /* GetVoid1() as a cISC4Occupant (insert / remove occupant) */
    const uint8_t* text;          /* the cheat's command line (UTF-8, not terminated; NULL = none) */
    size_t text_len;
    const fc4_flora_param* params; /* PlaceFlora's command parameters (NULL = none) */
    size_t param_count;
} fc4_flora_message;

/* Diagnostics and the status line's counters (host calls the core made,
 * by phase: SimNewMonth, SimNewDay, anything else). */
typedef struct fc4_flora_state {
    uint32_t struct_size;
    uint8_t initialized, enabled, propagation, propagation_forced, preserve, registry_loaded, pad[2];
    float size_x, size_z;
    int32_t cells_x, cells_z;
    uint32_t today, period, blocks_per_month, wild_seeds_per_month;
    int32_t iterator_count, iterator_index;
    uint32_t burnt_list;
    float burn_accumulator, burnt_removal_accumulator, burn_rate, burnt_removal_step, elevation_scale;
    /* the registry (process-wide) */
    uint32_t map_len, type_count, family_count, block_count;
    /* counters since creation */
    uint32_t created, create_failed; /* create_flora_occupant gave one / none */
    uint32_t planted;                /* inserts outside the monthly pass (tool, lots, cheats) */
    uint32_t spread;                 /* inserts during it (seeds, wild seeds) */
    uint32_t refused;                /* inserts the occupant manager refused */
    uint32_t aged;                   /* stage changes to 1 or 2 */
    uint32_t reborn;                 /* birth := today, state 0 */
    uint32_t dying;                  /* stage 4 (low score) */
    uint32_t burnt;                  /* KillFloraOccupant */
    uint32_t cleared;                /* removals during SimNewDay (burnt trees) */
    uint32_t removed_month;          /* removals during SimNewMonth */
    uint32_t removed_other;          /* demolition, buildings, networks, terrain */
    uint32_t moved;
    uint32_t blocks_visited, occupants_visited, months, days;
    uint32_t records_without_block;
    uint32_t reentrant_calls, queued_messages, lock_waits, reserved;
} fc4_flora_state;

/* The game-layout record builder. */
typedef struct fc4_flora_registry_host {
    uint32_t struct_size;
    void* user;
    /* Builds a block with the values of `rec` holding `exemplar` (its
     * reference passes to the block; 0 = none) and keeps one reference on it
     * for the life of the process. Returns its address, or 0 (the host then
     * releases `exemplar`). */
    uint64_t (*create_block)(void* user, const fc4_flora_record* rec, uint64_t exemplar);
    void (*set_block_seed)(void* user, uint64_t block, uint32_t seed_type); /* §6.11 */
    void (*release_exemplar)(void* user, uint64_t exemplar);                /* not kept */
} fc4_flora_registry_host;

/* One flora exemplar: NEW props (or NULL; taken) and one exemplar reference
 * (0 = none; taken). */
typedef void (*fc4_flora_exemplar_visitor)(void* ctx, uint32_t type_id, uint32_t group, uint32_t instance,
                                           fc4_props* props, uint64_t exemplar);

/* Everything the flora simulator asks of the game (one callback per core
 * FloraHost method, in its order). Out-pointers are written only when the
 * callback has a value. */
typedef struct fc4_flora_host {
    uint32_t struct_size;
    void* user;
    /* city, clock, resources */
    bool (*city_exists)(void* user);
    void (*city_size)(void* user, float* x, float* z);
    void (*city_cell_counts)(void* user, int32_t* x, int32_t* z);
    bool (*city_established)(void* user);
    uint32_t (*sim_date_number)(void* user);
    fc4_props* (*tuning)(void* user); /* NEW (taken) or NULL */
    /* each exemplar of type 0x6534284A in the resource manager's order */
    void (*flora_exemplars)(void* user, fc4_flora_exemplar_visitor visit, void* ctx);
    uint64_t (*obliterated_segment)(void* user);
    /* framework */
    void (*register_app_shutdown_callback)(void* user);
    void (*subscribe)(void* user, const uint32_t* ids, size_t count);
    void (*unsubscribe)(void* user, const uint32_t* ids, size_t count);
    void (*register_command)(void* user, uint32_t id, const uint8_t* name, size_t len);
    void (*unregister_command)(void* user, uint32_t id);
    void (*add_agent)(void* user, uint32_t level, const uint8_t* name, size_t len, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*acquire_services)(void* user);
    void (*release_services)(void* user);
    void (*register_cheat)(void* user, uint32_t id, const uint8_t* name, size_t len);
    void (*unregister_cheat)(void* user, uint32_t id);
    void (*post)(void* user, uint32_t msg_type, uint32_t data2);
    /* terrain, lots, weather */
    bool (*terrain_initialized)(void* user);
    float (*altitude)(void* user, float x, float z);
    float (*sea_level)(void* user);
    void (*face_normal)(void* user, float x, float z, float* out3); /* out3 zeroed by the caller */
    float (*lot_savvy_altitude)(void* user, float x, float z, bool* on_lot);
    uint32_t (*moisture_type)(void* user, float x, float z);
    /* occupants */
    uint32_t (*occupant_type)(void* user, uint64_t occ);
    bool (*occupant_has_property)(void* user, uint64_t occ, uint32_t id);
    void (*occupant_position)(void* user, uint64_t occ, float* out3);
    void (*set_occupant_position)(void* user, uint64_t occ, const float* pos3);
    void (*occupant_bounding_box)(void* user, uint64_t occ, float* min3, float* max3);
    bool (*occupant_in_manager)(void* user, uint64_t occ);
    void (*set_occupant_in_manager)(void* user, uint64_t occ, bool on);
    bool (*network_flags)(void* user, uint64_t occ, uint32_t* out); /* false: no cISC4NetworkOccupant */
    bool (*flora_occupant)(void* user, uint64_t occ, fc4_flora_occupant* out); /* false: no cISC4FloraOccupant */
    void (*set_flora_type)(void* user, uint64_t occ, uint32_t v);
    void (*set_birth_date)(void* user, uint64_t occ, uint32_t v);
    void (*set_last_seeding_date)(void* user, uint64_t occ, uint32_t v);
    void (*set_flora_state)(void* user, uint64_t occ, int8_t v);
    void (*set_flora_orientation)(void* user, uint64_t occ, uint32_t v);
    void (*set_flora_record)(void* user, uint64_t occ, uint64_t block); /* 0 = null record */
    void (*post_occupant_change)(void* user, uint64_t occ);
    uint64_t (*create_flora_occupant)(void* user); /* one reference for the caller; 0 = none */
    void (*init_occupant)(void* user, uint64_t occ);
    void (*add_ref)(void* user, uint64_t occ);
    void (*release)(void* user, uint64_t occ);
    /* The occupant (as `iid`) of a reference read from a save: `token` is the
     * reference the stream handed over and passes to this call; 0 = none
     * (the host dropped it). */
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
    /* occupant manager */
    bool (*has_occupant_manager)(void* user);
    bool (*insert_occupant)(void* user, uint64_t occ);
    bool (*remove_occupant)(void* user, uint64_t occ);
    void (*move_occupant)(void* user, uint64_t occ);
    /* occ_type 0 with blocking = the blocking filter (§6.4.1) */
    void (*occupants_in_bbox)(void* user, const float* min3, const float* max3, uint32_t occ_type, bool blocking,
                              fc4_token_callback cb, void* ctx);
    /* rect NULL = null corners (the whole city) */
    void (*occupants_in_cells)(void* user, const fc4_cell_rect* rect, uint32_t occ_type, fc4_token_callback cb,
                               void* ctx);
    void (*occupants_in_manager_cells)(void* user, uint32_t occ_type, fc4_token_callback cb, void* ctx);
    /* obliteration segment (§10.3, ABI 30): QueryInterface for cISC4DBSegment,
     * then OpenOStream(key, &s, true) (`write`) or OpenIStream(key, &s); the
     * stream token, 0 = not open. The core puts / gets each record's u16
     * version itself. */
    uint64_t (*segment_open)(void* user, uint64_t seg, uint32_t type, uint32_t group, uint32_t instance,
                             bool write);
    void (*segment_close)(void* user, uint64_t stream); /* Close*Stream, then both references released */
    bool (*stream_put_u16)(void* user, uint64_t stream, uint16_t v);
    bool (*stream_put_u32)(void* user, uint64_t stream, uint32_t v);
    bool (*stream_get_u16)(void* user, uint64_t stream, uint16_t* out); /* false: failed, *out untouched */
    bool (*stream_get_u32)(void* user, uint64_t stream, uint32_t* out);
    bool (*stream_error)(void* user, uint64_t stream); /* GetError() != 0 */
    /* the occupant's cIGZSerializable::Write / Read on the stream */
    bool (*write_occupant)(void* user, uint64_t occ, uint64_t stream);
    bool (*read_occupant)(void* user, uint64_t occ, uint64_t stream);
} fc4_flora_host;

/* The registry (one per process; the host table is copied). */
fc4_flora_registry* fc4_flora_registry_create(const fc4_flora_registry_host* host);
void fc4_flora_registry_destroy(fc4_flora_registry* r); /* the simulators keep theirs alive */
/* Any thread: */
uint64_t fc4_flora_registry_record(const fc4_flora_registry* r, uint32_t type_id); /* slot 7; 0 = none */
/* Slot 11; 0 is valid after an Init, invalid after an exemplar-change reload until the next Init (FQ14). */
bool fc4_flora_registry_is_type_valid(const fc4_flora_registry* r, uint32_t type_id);
bool fc4_flora_registry_is_family_valid(const fc4_flora_registry* r, uint32_t id);      /* slot 12 */
/* Slot 9: NULL buffer: *count := N; else copies min(*count, N) and stores it. */
uint32_t fc4_flora_registry_types_into(const fc4_flora_registry* r, uint32_t* buffer, uint32_t* count);
/* `which` 0 = type array (registration order), 1 = every map key (ascending,
 * 0 included), 2 = family IDs. `cb` runs after the snapshot is released. */
uint32_t fc4_flora_registry_list(const fc4_flora_registry* r, uint32_t which, fc4_token_callback cb, void* ctx);
uint32_t fc4_flora_registry_family(const fc4_flora_registry* r, uint32_t family, fc4_token_callback cb, void* ctx);
/* Returns "loaded" (the first Init of the process ran). */
bool fc4_flora_registry_sizes(const fc4_flora_registry* r, uint32_t* map_len, uint32_t* types, uint32_t* families,
                              uint32_t* blocks);

/* The simulator. `token` is its own host identity; the two clock values seed
 * its cRZRandom (§3.1, §8). */
fc4_flora* fc4_flora_create(const fc4_flora_host* host, const fc4_flora_registry* registry, uint64_t token,
                            uint32_t wall_clock_seed, uint32_t process_clock);
void fc4_flora_destroy(fc4_flora* f); /* never-initialised: releases the loaded burnt list */
bool fc4_flora_init(fc4_flora* f);
bool fc4_flora_shutdown(fc4_flora* f);
bool fc4_flora_do_message(fc4_flora* f, const fc4_flora_message* m); /* always true */
/* Slot 5: props NEW (taken; NULL = false), exemplar one reference (taken). */
bool fc4_flora_add_exemplar(fc4_flora* f, fc4_props* props, uint64_t exemplar);
/* Slot 6: the caller's block and its values; false = already registered
 * (no reference taken); true: the caller gives the registry one reference. */
bool fc4_flora_add_record(fc4_flora* f, const fc4_flora_record* values, uint64_t block);
bool fc4_flora_location_ok(fc4_flora* f, uint32_t type_id, float x, float z, uint64_t exclude);
uint32_t fc4_flora_best_type(fc4_flora* f, float x, float z, float* score);
/* Slot 17: pos (3 floats) is rewritten; returns the inserted occupant (borrowed) or 0. */
uint64_t fc4_flora_add_new_occupant(fc4_flora* f, uint32_t type_id, float* pos, bool force);
bool fc4_flora_add_occupant(fc4_flora* f, uint64_t occ);
bool fc4_flora_demolish_occupant(fc4_flora* f, uint64_t occ, int32_t flags);
bool fc4_flora_demolish_cells(fc4_flora* f, const fc4_cell_rect* rect, int32_t flags);
bool fc4_flora_demolish_bbox(fc4_flora* f, const float* min3, const float* max3, int32_t flags);
bool fc4_flora_kill(fc4_flora* f, uint64_t occ);
bool fc4_flora_save_obliterated(fc4_flora* f, uint64_t segment);
bool fc4_flora_get_state(const fc4_flora* f, fc4_flora_state* out);
bool fc4_flora_write(const fc4_flora* f, const fc4_ostream* out);
bool fc4_flora_read(fc4_flora* f, const fc4_istream* in);

/* ---- flora census shadow (tooling) -------------------------------------------
 * A differential of the ORIGINAL flora simulator: the registry after load
 * against one built from the same exemplars, and the census of flora
 * occupants around each monthly pass against §6.7.2's deterministic rule
 * (engine/capi/src/flora_shadow.rs says exactly what is compared). */
typedef struct fc4_flora_shadow fc4_flora_shadow;

typedef struct fc4_flora_census_entry {
    uint64_t token;
    uint32_t flora_type, birth_date, last_seeding_date;
    int8_t state;
    uint8_t in_manager, is_flora, reserved;
    float pos[3], bbox_min[3], bbox_max[3];
    uint32_t moisture; /* GetMoistureType(x, z) before the pass */
    float altitude;    /* GetAltitude(x, z) before the pass */
    uint32_t reserved2;
} fc4_flora_census_entry;

typedef struct fc4_flora_shadow_month_in {
    uint32_t struct_size;
    uint32_t julian;   /* data1 */
    uint32_t sim_date; /* GetSimDateNumber() */
    float sea_level;
    const uint8_t* before; /* the original's own Write before / after (NULL = not captured) */
    size_t before_len;
    const uint8_t* after;
    size_t after_len;
    const fc4_flora_census_entry* census_before;
    size_t before_count;
    const fc4_flora_census_entry* census_after;
    size_t after_count;
    const fc4_flora_record* records; /* the original's, from its blocks, by type_id */
    size_t record_count;
} fc4_flora_shadow_month_in;

typedef struct fc4_flora_shadow_stats {
    uint32_t struct_size;
    uint32_t months, incomplete;
    int32_t last_mismatches;
    uint32_t registry_checks, registry_mismatches;
    uint32_t occupants, visited, matched, mismatched, vanished, appeared;
    uint32_t predicted_reborn, predicted_aged, predicted_removed, predicted_seeded;
} fc4_flora_shadow_stats;

fc4_flora_shadow* fc4_flora_shadow_create(void);
void fc4_flora_shadow_destroy(fc4_flora_shadow* s);
/* tuning: NEW (taken) or NULL; returns the elevation scale (16 / 0xC9DBDD02, else 0.08). */
float fc4_flora_shadow_set_tuning(fc4_flora_shadow* s, fc4_props* tuning);
/* `exemplars` enumerates as the host's flora_exemplars (exemplar references
 * must be 0); records[i] is the record of types[i] (type_id 0 = none); key0
 * NULL = none. One line to `sink`; returns the differences (-1 = bad args). */
int32_t fc4_flora_shadow_registry(fc4_flora_shadow* s,
                                  void (*exemplars)(void* user, fc4_flora_exemplar_visitor visit, void* ctx),
                                  void* user, const uint32_t* types, size_t type_count,
                                  const fc4_flora_record* records, const fc4_flora_record* key0, bool key0_valid,
                                  fc4_bytes_sink sink, void* ctx);
/* One line to `sink`; returns the mismatches, -2 = not compared (the line
 * says why), -1 = bad arguments. */
int32_t fc4_flora_shadow_month(fc4_flora_shadow* s, const fc4_flora_shadow_month_in* m, fc4_bytes_sink sink,
                               void* ctx);
bool fc4_flora_shadow_get_stats(fc4_flora_shadow* s, fc4_flora_shadow_stats* out);

/* ---- landfill developer (docs/specs/landfill.md) ----------------------------
 *
 * cSC4LandfillDeveloper (0xEA09D4AE). Tokens: the sanitation department, the
 * zot props and every game occupant by its cISC4Occupant identity; 0 is null.
 * The core owns one reference on the department (get_department_budget hands
 * out a borrowed one; the core add_refs it on PostCityInit) and on every zot
 * (add_lot_prop hands out a borrowed one; the core add_refs it; a zot read
 * back by fc4_landfill_read arrives with one, or gets one through add_ref when
 * fc4_ref.core_handle is set). Shutdown releases them.
 *
 * Landfill tiles (§4.8.1, LG3): with insert_landfill_tile the host builds
 * each tile as a game cSC4LandfillOccupant (tile exemplar attached through
 * cISCExemplarPropertyHolder::SetDefaultExemplar) and inserts it; it is then
 * found through occupant_at, priced from its exemplar (§4.9.1, LG11),
 * removed through occupant_remove and saved by the occupant manager (§6.4,
 * LG8), like the original's. Without the callback, or when it makes none,
 * the tile is kept in this layer instead: the host's occupant callbacks
 * never see it, and only pollution's active-landfill flag shows it. A load
 * rebuilds no tile a cell already has; a flagged mask cell without one (a
 * city saved while tiles were kept here) gets its tile back on PostCityInit
 * [choice].
 *
 * A message delivered while another call into the same object is running is
 * queued and handled when that call returns.
 *
 * Threads: the core is single-threaded. The caller serialises every call on
 * one handle (the plugin holds a per-object lock). */
typedef struct fc4_landfill fc4_landfill;

/* A cell region (SC4CellRegion<long>): the inclusive rectangle and
 * (x1 - x0 + 1) * (z1 - z0 + 1) mask bytes, index (z - z0) * width + (x - x0),
 * non-zero = a mask cell. mask NULL: every cell set. A short mask is padded
 * with cleared cells; a rectangle of more than 2^20 cells is refused. */
typedef struct fc4_landfill_region {
    int32_t x0, z0, x1, z1;
    const uint8_t* mask;
    size_t len;
} fc4_landfill_region;

/* The zone effects init_zone_effects created (§4.10): one reference each for
 * the core, 0 = none. */
typedef struct fc4_landfill_effects {
    uint64_t truck;
    uint64_t seagull;
} fc4_landfill_effects;

/* Diagnostics. `moved`, `supply_delta`, `free_capacity` and
 * `capacity_reported` are the last SimNewMonth's. */
typedef struct fc4_landfill_state {
    uint32_t struct_size;
    uint8_t initialized, loaded, capacity_reported, reserved0;
    uint32_t zones, connected_zones;
    uint32_t cells;           /* mask cells over all zones */
    uint32_t heaps, valid_heaps, active_heaps;
    uint32_t zots;            /* zones with a zot prop */
    uint32_t virtual_tiles;   /* tiles kept in this layer */
    uint32_t active_cells;    /* cells this object flagged (and the rebuild found flagged) */
    uint32_t reentrant_calls;
    uint32_t capacity_total, used_total;
    int32_t free_capacity;    /* the last SetGarbageCapacity value */
    uint32_t rebuilt_tiles;   /* tiles rebuilt at the last PostCityInit after a load */
    uint64_t stored;          /* sum of contained over every heap */
    int64_t moved;            /* sum of GarbagePutInLandfill */
    int64_t supply_delta;     /* sum of AddToGarbageSupply */
    int64_t maintenance;      /* slot 7 */
    uint64_t department;      /* borrowed; 0 = none */
    uint32_t game_tiles;      /* tiles inserted through insert_landfill_tile since creation (ABI 33) */
    uint32_t game_tile_failures; /* insert_landfill_tile calls that made none (kept here instead) */
} fc4_landfill_state;

/* What the landfill developer needs from the game (one callback per
 * LandfillHost method that reaches the game, engine/core/src/landfill/mod.rs,
 * §8, plus two pollution reads). Missing callbacks read as 0 / false / no-op
 * (department funding as 1.0). Returned tokens are borrowed unless stated. */
typedef struct fc4_landfill_host {
    uint32_t struct_size; /* sizeof(fc4_landfill_host) */
    void* user;
    /* lifecycle (§1.1, §1.2) */
    void (*acquire_services)(void* user);
    void (*release_services)(void* user);
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t len, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    fc4_props* (*tuning)(void* user);                                                     /* NEW or NULL */
    fc4_props* (*exemplar)(void* user, uint32_t type, uint32_t group, uint32_t instance); /* NEW or NULL */
    /* city (slots 93, 94, 97) */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    void (*cell_center_to_position)(void* user, int32_t x, int32_t z, float* px, float* pz);
    /* terrain (slots 13, 19, 29, 49, 68) */
    float (*altitude)(void* user, float px, float pz);
    float (*altitude_at_nearest_grid)(void* user, float px, float pz);
    float (*sea_level)(void* user);
    void (*set_altitude)(void* user, int32_t x1, int32_t z1, int32_t x2, int32_t z2, float altitude);
    void (*redisplay_terrain)(void* user, int32_t x1, int32_t z1, int32_t x2, int32_t z2);
    /* pollution simulator (slots 20, 21, 24, 29, 30, 50, 51; reads 22, 49) */
    uint32_t (*garbage_supply)(void* user);
    uint32_t (*landfill_tile_capacity)(void* user);
    int32_t (*total_garbage_uncollected)(void* user);
    void (*add_to_garbage_supply)(void* user, int32_t delta);
    void (*set_garbage_capacity)(void* user, int32_t available);
    void (*set_is_active_landfill)(void* user, int32_t x, int32_t z, bool on);
    void (*garbage_put_in_landfill)(void* user, int32_t n);
    int32_t (*garbage_capacity)(void* user);
    bool (*is_active_landfill)(void* user, int32_t x, int32_t z);
    /* budget (§4.9) */
    uint64_t (*get_department_budget)(void* user, uint32_t id);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    float (*department_funding_percentage)(void* user, uint64_t dept);
    bool (*line_item_current_expenses)(void* user, uint64_t dept, uint32_t line, int64_t* out);
    void (*set_line_item_full_expenses)(void* user, uint64_t dept, uint32_t line, int64_t value);
    /* the first value of `property` of the occupant's property holder */
    bool (*occupant_item_cost)(void* user, uint64_t occ, uint32_t property, int64_t* out);
    /* zone manager (slots 6, 11, 12) */
    int32_t (*zone_count)(void* user, int32_t zone_type);
    int32_t (*zone_type)(void* user, int32_t x, int32_t z);
    bool (*place_zone)(void* user, const fc4_landfill_region* region, int32_t zone_type);
    /* occupants (game occupants only) */
    uint64_t (*occupant_at)(void* user, int32_t x, int32_t z, uint32_t occupant_type);
    bool (*occupant_remove)(void* user, uint64_t occ);
    void (*occupant_changed)(void* user, uint64_t occ);
    void (*occupant_position)(void* user, uint64_t occ, float* xyz);
    void (*occupant_set_position)(void* user, uint64_t occ, const float* xyz);
    void (*occupant_set_removal_flags)(void* user, uint64_t occ, uint32_t flags);
    void (*occupant_post_message)(void* user, uint64_t occ, uint32_t message);
    bool (*is_landfill_occupant)(void* user, uint64_t occ);
    bool (*is_empty_landfill_tile)(void* user, uint64_t occ);
    void (*remove_incompatible_occupants)(void* user, int32_t x, int32_t z);
    void (*set_empty_cell_texture)(void* user, int32_t x, int32_t z);
    void (*remove_base_texture)(void* user, int32_t x, int32_t z);
    /* prop manager (slots 26, 29) */
    uint64_t (*add_lot_prop)(void* user, uint32_t prop_id, const float* xyz, int32_t orientation);
    bool (*remove_prop)(void* user, uint64_t occ);
    int32_t (*zot_orientation)(void* user);
    /* traffic */
    uint64_t (*find_network_tile)(void* user, int32_t x, int32_t z, uint32_t flag_mask, bool flag);
    void (*subnetworks_in_region)(void* user, const fc4_landfill_region* region, fc4_token_callback cb, void* ctx);
    void (*occupant_count_for_all_subnetworks)(void* user, uint32_t developer_type, fc4_token_callback cb,
                                               void* ctx);
    /* lot developer (slot 21) */
    bool (*attempt_area_levelling)(void* user, int32_t x1, int32_t z1, int32_t x2, int32_t z2, float f,
                                   bool with_water, bool water_flag, float max_area_slope);
    /* 3D view */
    uint32_t (*current_view_level)(void* user);
    /* zone effects (§4.10, rendering only) */
    void (*init_zone_effects)(void* user, int32_t cx, int32_t cz, int32_t x1, int32_t z1, int32_t x2, int32_t z2,
                              fc4_landfill_effects* out);
    void (*shutdown_zone_effects)(void* user, uint64_t truck, uint64_t seagull, bool seagull_started);
    void (*set_truck_enabled)(void* user, uint64_t truck, bool enabled);
    void (*start_seagull)(void* user, uint64_t seagull, uint32_t flag);
    void (*stop_seagull)(void* user, uint64_t seagull, uint32_t flag);
    /* history (§5): MessageSend of 0x89EFA536 */
    void (*send_history)(void* user, uint32_t series, uint32_t value);
    /* tiles (§4.8.1 step 3, LG3; appended in ABI 33): build a game
     * cSC4LandfillOccupant for cell (x, z) with the tile exemplar
     * (0x6534284A, 0x07BDDF1C, tile_id) attached, the orientation, flag 4,
     * removal flags 2 and the world position xyz; Init it and insert it into
     * the occupant manager. Its cISC4Occupant token (borrowed: the occupant
     * manager holds it), 0 if none was made (the tile is then kept here). The
     * core has already set the cell's active-landfill flag. */
    uint64_t (*insert_landfill_tile)(void* user, int32_t x, int32_t z, uint32_t tile_id, int32_t orientation,
                                     const float* xyz);
} fc4_landfill_host;

/* QueryInterface (§1): whether the class answers `iid`. */
bool fc4_landfill_query_interface(uint32_t iid);
/* GetGZCLSID: 0xEA09D4AE. */
uint32_t fc4_landfill_class_id(void);
/* The table is copied; `token` is the developer's own identity. NULL for a
 * NULL or short table. */
fc4_landfill* fc4_landfill_create(const fc4_landfill_host* host, uint64_t token);
/* Releases what a never-initialised (loaded) object still holds. */
void fc4_landfill_destroy(fc4_landfill* p);
bool fc4_landfill_init(fc4_landfill* p);
/* Each initialised zone removes its zot prop (remove_prop) and releases it
 * (§4.4.9, LG13); the zot of a zone never initialised is released only.
 * Drops the tiles kept here. */
bool fc4_landfill_shutdown(fc4_landfill* p);
/* DoMessage (§3.2): `region` is the zoning message's GetVoid2 (copied), NULL
 * otherwise. Always true. */
bool fc4_landfill_do_message(fc4_landfill* p, uint32_t msg_type, uint32_t data1, uint32_t data2, uint32_t data3,
                             uint32_t data4, const fc4_landfill_region* region);
float fc4_landfill_efficiency(const fc4_landfill* p);          /* slot 5, a fraction */
int64_t fc4_landfill_maintenance_costs(const fc4_landfill* p); /* slot 7 */
int64_t fc4_landfill_current_budget(const fc4_landfill* p);    /* slot 8 */
/* Slot 9 (an int64, high dword 0); 0 for a NULL or empty region. */
int64_t fc4_landfill_zoning_cost(const fc4_landfill* p, const fc4_landfill_region* region);
void fc4_landfill_usage(const fc4_landfill* p, uint32_t* usage, uint32_t* capacity); /* slot 10 */
int32_t fc4_landfill_tile_count(const fc4_landfill* p);                              /* slot 11 */
/* The tile kept here for a cell: false when there is none (a game tile made
 * through insert_landfill_tile is not reported). */
bool fc4_landfill_tile_at(const fc4_landfill* p, int32_t x, int32_t z, uint32_t* tile_id, int32_t* orientation);
bool fc4_landfill_get_state(const fc4_landfill* p, fc4_landfill_state* out);
/* §6.1; the zot references go out with class 0. Read runs before Init. */
bool fc4_landfill_write(const fc4_landfill* p, const fc4_ostream* out);
bool fc4_landfill_read(fc4_landfill* p, const fc4_istream* in);

/* ---- landfill shadow (tooling, no spec behaviour) ----------------------------
 *
 * With the ORIGINAL developer running: before its SimNewMonth, capture its
 * record (its own Write, in the fc4_pollution_capture format with the
 * occupant field of each reference set) and read its slot 7; begin loads the
 * record into a private developer in shadow mode (every write recorded, never
 * sent; no reference taken), restores that maintenance and runs our month.
 * After the original's delivery, capture again and read slot 7 again; finish
 * compares the zones and heaps, the free capacity with garbage_capacity, the
 * garbage moved with the supply's change, the maintenance, and the
 * active-landfill flags with is_active_landfill, and hands one line to
 * `sink`. The shadow only calls the table's read callbacks. */
typedef struct fc4_landfill_shadow fc4_landfill_shadow;

typedef struct fc4_landfill_shadow_stats {
    uint32_t struct_size;
    uint32_t months, clean, incomplete;
    int32_t last_mismatches; /* -1: not compared */
    uint32_t reserved;
    uint64_t zones_compared, heaps_compared, heap_mismatches;
    uint64_t cells_compared, cell_mismatches, field_mismatches;
} fc4_landfill_shadow_stats;

/* max_diffs: the differences listed per month (0 -> 8, at most 64). */
fc4_landfill_shadow* fc4_landfill_shadow_create(const fc4_landfill_host* host, uint32_t max_diffs);
void fc4_landfill_shadow_destroy(fc4_landfill_shadow* s);
/* 0 = ran; -1 = not run (finish reports why). */
int32_t fc4_landfill_shadow_begin(fc4_landfill_shadow* s, int32_t julian, const fc4_pollution_capture* before,
                                  int64_t maintenance);
/* The difference count, -1 when not compared. */
int32_t fc4_landfill_shadow_finish(fc4_landfill_shadow* s, const fc4_pollution_capture* after, int64_t maintenance,
                                   fc4_bytes_sink sink, void* ctx);
bool fc4_landfill_shadow_get_stats(fc4_landfill_shadow* s, fc4_landfill_shadow_stats* out);
/* ---- land value shadow (tooling; engine/capi/src/landvalue_shadow.rs) -------
 *
 * With the ORIGINAL land value simulator running: at the end of each of its
 * passes (LandValueMapChanged 0x87CC75B3, posted at the end of state 8) the
 * plugin reads its grids and scalars through the interface and FreeCity4's
 * pass (states 0-8) runs on the original's live inputs through a read-only
 * host table (write callbacks NULL; our grids are local, the intrinsic grid
 * starts as a copy of the original's). Compared: intrinsic, total and slope
 * grids tract by tract, GetLandValueType at each total tract's first cell,
 * the average, the mode altitude and the record's mean altitude. */
typedef struct fc4_landvalue_shadow fc4_landvalue_shadow;

/* What the plugin read from the original (grids x outer, count_x * count_z). */
typedef struct fc4_landvalue_original {
    uint32_t struct_size;
    uint32_t has_record; /* non-zero: its own Write parsed (version 1) */
    fc4_grid_geometry intrinsic_geometry; /* slot 7 */
    const uint8_t* intrinsic;
    fc4_grid_geometry total_geometry; /* slot 8 */
    const uint8_t* total;
    fc4_grid_geometry slope_geometry; /* slot 9 */
    const float* slope;
    const uint8_t* types; /* slot 5 at (tx*S, tz*S) per total tract; NULL = not read */
    uint8_t average;        /* slot 10 */
    uint8_t record_average; /* the record's average */
    uint8_t record_flag;    /* the record's byte 5: dirty or a pass running */
    uint8_t reserved;
    float mode_altitude; /* slot 11 */
    float mean_altitude; /* the record's mean altitude (no getter) */
} fc4_landvalue_original;

typedef struct fc4_landvalue_shadow_stats {
    uint32_t struct_size;
    uint32_t passes, compared, clean;
    int32_t last_mismatches; /* -1 = not compared */
    uint32_t intrinsic_mismatches, total_mismatches, slope_mismatches, type_mismatches, scalar_mismatches;
    uint32_t desirability_reruns;  /* passes compared with a developer's previous desirability map */
    uint32_t desirability_changes; /* snapshot changes (note_day and after each comparison; LV2) */
} fc4_landvalue_shadow_stats;

/* `host`: only its read callbacks are called (create_grid, the writes,
 * release, the history and post may be NULL). `tuning`: the exemplar (copied;
 * NULL = missing: code defaults). `max_cells`: differing tracts listed per
 * grid (0 = 4, at most 64). */
fc4_landvalue_shadow* fc4_landvalue_shadow_create(const fc4_landvalue_host* host, const fc4_landvalue_tuning* tuning,
                                                  uint32_t max_cells);
void fc4_landvalue_shadow_destroy(fc4_landvalue_shadow* s);
/* SimNewDay: snapshots each listed developer's desirability map (read-only)
 * while its slot 10 (host->developer_lists_valid) is true or unknown, so the
 * snapshot before a change is the map from before the developer's last
 * state-17 copy (landvalue.md §7.4, LV2); returns how many snapshots
 * changed. Our pass applies the developers in ascending pointer order, as
 * the original does (LV1). */
uint32_t fc4_landvalue_shadow_note_day(fc4_landvalue_shadow* s);
/* The end of an original pass: one line to `sink`. Returns the differences,
 * -1 when nothing was compared (INCOMPLETE). */
int32_t fc4_landvalue_shadow_compare(fc4_landvalue_shadow* s, const fc4_landvalue_original* orig, int32_t julian,
                                     fc4_bytes_sink sink, void* ctx);
/* The counters as the last call that finished left them (callable from the
 * sink or a host callback). */
bool fc4_landvalue_shadow_get_stats(const fc4_landvalue_shadow* s, fc4_landvalue_shadow_stats* out);

/* ---- flammability shadow (tooling; engine/capi/src/flammability_shadow.rs) --
 *
 * With the ORIGINAL flammability simulator running: a mirror simulator (ours,
 * on its own grids) gets the original's six messages
 * (fc4_flammability_shadow_message, in the same delivery as the original's) and
 * reads live inputs through a read-only host table; at each of the
 * original's cycle ends (SimCycleComplete 0x08F415FE) the plugin reads the
 * original's record and getters and the shadow compares: the raw grid
 * exactly, the adjusted grid against the values the cycle could have
 * written, the average's range, the multiplier, the hotspots (every rotation
 * of the visiting order), and the getters (slots 6-8) on the resynchronised
 * mirror. The fire roll, the iterator start and the hotspot shuffle (random
 * draws) are not compared. */
typedef struct fc4_flammability_shadow fc4_flammability_shadow;

typedef struct fc4_flammability_shadow_hotspot {
    int32_t x, z, weight;
} fc4_flammability_shadow_hotspot;

typedef struct fc4_flammability_original {
    uint32_t struct_size;
    uint32_t has_record; /* non-zero: its own Write parsed (version 2) */
    uint64_t simulator;  /* the original's identity (a new one restarts the baseline) */
    fc4_grid_geometry geometry;     /* the record's adjusted grid */
    fc4_grid_geometry raw_geometry; /* the record's raw grid */
    const uint8_t* adjusted;        /* x outer */
    const uint8_t* raw;
    const fc4_flammability_shadow_hotspot* hotspots; /* the record's, in list order */
    const uint8_t* flammable;                 /* slot 6 per city cell, x outer; NULL = not read */
    uint32_t hotspot_count;
    float global_multiplier; /* the record's */
    uint8_t average;         /* the record's */
    uint8_t average_getter;  /* slot 8 */
    uint8_t hotspot_found;   /* slot 7's result */
    uint8_t hotspot_value;
    int32_t hotspot_x, hotspot_z;
    int32_t cells_x, cells_z; /* the city's cell counts (flammable) */
} fc4_flammability_original;

typedef struct fc4_flammability_shadow_stats {
    uint32_t struct_size;
    uint32_t cycles, compared, clean;
    int32_t last_mismatches; /* -1 = not compared */
    uint32_t raw_mismatches, adjusted_mismatches, unvisited, scalar_mismatches, hotspot_mismatches,
        getter_mismatches;
    uint32_t events; /* messages mirrored */
} fc4_flammability_shadow_stats;

/* `host`: only its read callbacks are called (the grid API, record_history,
 * ignite_cell, set_fire_cause, random_lot and post may be NULL).
 * `max_cells`: differing tracts listed per grid (0 = 4, at most 64). */
fc4_flammability_shadow* fc4_flammability_shadow_create(const fc4_flammability_host* host, uint32_t max_cells);
void fc4_flammability_shadow_destroy(fc4_flammability_shadow* s);
/* One of the original's six messages; false when not mirrored (no baseline
 * yet, or another type). SimNewDay (0x66956814) means the original's day
 * delivery has just finished: the day is counted and the watered state
 * polled (§6.5, FL7/FL8). Until the first such day, every other call (e.g.
 * the plumbing pass end WaterMapChanged 0x6946DC34) first polls it. A poll
 * reads host->is_cell_watered for every tract and compares by content (the
 * host->watered_grid pointer is only counted; NULL = nothing polled): the
 * tracts whose state changed are marked for the cycle (FL5-FL7). */
bool fc4_flammability_shadow_message(fc4_flammability_shadow* s, const fc4_flammability_message* m);
/* The original's cycle end: the first (or after a new simulator) is the
 * baseline; later ones are compared (after a poll; a tract marked in the
 * cycle may hold its value under either watered state). The marks of the
 * changes seen after the last wrap day (by the day count since the
 * original's Init) are carried into the next cycle (FL8). One line. Returns
 * the differences, -1 when nothing was compared. */
int32_t fc4_flammability_shadow_cycle(fc4_flammability_shadow* s, const fc4_flammability_original* orig,
                                      int32_t julian, fc4_bytes_sink sink, void* ctx);
/* The counters as the last call that finished left them (callable from the
 * sink or a host callback). */
bool fc4_flammability_shadow_get_stats(const fc4_flammability_shadow* s, fc4_flammability_shadow_stats* out);

/* ---- residential (docs/specs/residential.md, residential-education-health.md) ----
 *
 * The residential simulator, education and health included (class 0x4990C013).
 * Tokens: occupants (the cISC4Occupant identity), departments
 * (cISC4DepartmentBudget), line items, visual effects, grids; 0 is null. The
 * residential grids are the game's own cSC4SimGrid<T> through fc4_grid_api
 * (uint8 0x49B9E602, uint16 0x49B9E604, float 0x49B9E60A); the building map
 * and the growth developers' population maps are BORROWED grid tokens read
 * through the same table.
 *
 * Threads: the core is main-thread only. After every outermost main-thread
 * call a snapshot and a detached copy of the state are published; a getter on
 * the main thread with no call running uses the core, anywhere else it runs on
 * a throw-away view of the detached copy (grids borrowed, writes ignored). A
 * state change made anywhere else (a message, a strike end, a funding or
 * auto-budget setter) is queued, its occupant held, and handled when the next
 * outermost main-thread call returns. Init, Shutdown, Read, get_state and
 * set_work_budget are refused off the main thread or during a call. */
typedef struct fc4_residential fc4_residential;

/* `which` of fc4_residential_buildings (slots 47, 49, 61). */
#define FC4_RESIDENTIAL_LIST_SCHOOL           0u
#define FC4_RESIDENTIAL_LIST_HOSPITAL         1u
#define FC4_RESIDENTIAL_LIST_HEALTH_AFFECTING 2u

/* GetBudgetItemForPurpose's item (budget.md §2). */
typedef struct fc4_residential_budget_item {
    uint32_t department;
    uint32_t line; /* 0 = use the building type */
    uint32_t purpose;
    uint32_t index;
    int64_t cost;
    float local_funding; /* a fraction (1.0 = 100 %) */
    uint32_t reserved;
} fc4_residential_budget_item;

/* The query record of slots 14, 15, 26 and 27 (REH §4.11): the game record's
 * fields after its leading cRZString (+0x14 .. +0x38 in order), the two costs
 * widened to 64 bits. */
typedef struct fc4_residential_query_data {
    uint32_t capacity, effective_capacity, capacity_per_staff, effective_per_staff, served;
    float grade, staff_funding;
    uint32_t reserved0;
    int64_t staff_cost;
    float vehicle_funding;
    uint32_t reserved1;
    int64_t vehicle_cost;
} fc4_residential_query_data;

/* Slots 43 / 44. */
typedef struct fc4_residential_min_max {
    uint32_t min_x, min_z, max_x, max_z;
    float min, max;
} fc4_residential_min_max;

/* What any thread may read: published after every outermost main-thread call.
 * Grid tokens are borrowed (0 = none). */
typedef struct fc4_residential_snapshot {
    uint32_t struct_size;
    uint8_t initialized;
    uint8_t school_strike, health_strike;             /* slots 6 / 7 */
    uint8_t school_auto_budget, hospital_auto_budget; /* slots 52 / 54 */
    uint8_t pad[3];
    uint32_t state;                                         /* the daily state machine (§5.1) */
    int32_t population;                                     /* slot 16 */
    int32_t school_building_count, hospital_building_count; /* slots 46 / 48 */
    int32_t population_size, eq_size;
    uint32_t reserved;
    uint64_t proximity[3];                 /* slot 5 (R$, R$$, R$$$) */
    uint64_t population_grids[3];          /* slot 25 */
    uint64_t age;                          /* slot 41 */
    uint64_t school_grade, hospital_grade; /* slots 39 / 40 */
    uint64_t eq[3], hq[3];                 /* slots 23 / 24 */
} fc4_residential_snapshot;

/* Diagnostics (main thread with no call running). */
typedef struct fc4_residential_state {
    uint32_t struct_size;
    uint8_t initialized, city_started, dirty, tuning_loaded;
    uint8_t school_strike, health_strike, school_auto_budget, hospital_auto_budget;
    uint32_t state;
    int32_t population, running_population;
    uint32_t population_by_wealth[3]; /* the sum of each population grid's tracts */
    uint32_t global_le;               /* slot 34 */
    uint64_t le_sum;
    float global_eq, global_hq;               /* slots 32 / 33 */
    int32_t school_covered, hospital_covered; /* the last coverage states' counters (REH §4.4) */
    int32_t school_buildings, hospital_buildings;
    uint32_t pending_school, pending_hospital, health_affecting, other;
    int32_t population_size, eq_size;
    int32_t tracts_x, tracts_z;     /* the population R$ grid */
    int32_t iterator_x, iterator_z; /* state 4's scan position */
    int32_t school_strike_start, health_strike_start;
    uint32_t departments, grids; /* held (of 4 / of 25) */
    uint32_t reentrant_calls, deferred_calls, view_calls;
    uint32_t reserved;
} fc4_residential_state;

/* create_visual_effect: the host cannot show effects (RG1; treated as
 * created). */
#define FC4_EFFECT_NOT_SHOWN UINT64_MAX

/* What the residential simulator needs from the game: one callback per
 * ResidentialHost / EducationHealthHost method of the core
 * (engine/core/src/residential), in the traits' order, plus hand_out. Missing
 * callbacks behave as "nothing there" (0, false, no object); a missing
 * department_funding, or a NaN from it, is a department whose funding cannot
 * be read, and the output keeps the caller's value (REH §4.2, RG10). */
typedef struct fc4_residential_host {
    uint32_t struct_size; /* sizeof(fc4_residential_host) */
    void* user;
    fc4_grid_api grids;
    /* lifecycle (§4.1, §4.2) */
    void (*acquire_services)(void* user);
    void (*acquire_developers)(void* user);
    bool (*acquire_demand_node)(void* user);
    bool (*acquire_message_server)(void* user);
    bool (*acquire_budget_simulator)(void* user);
    bool (*has_simulator)(void* user);
    void (*add_agent)(void* user, uint32_t level, const char* name, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    void (*release_message_server_and_budget)(void* user);
    void (*release_services)(void* user);
    void (*set_history_queue_length)(void* user, uint32_t series, uint32_t length);
    /* CreateDepartmentBudget, AddRef, name, max and initial funding (and the
     * secondary departments' local line item): a token holding one reference,
     * 0 = none. */
    uint64_t (*create_department)(void* user, uint32_t id, uint32_t name, float max_funding, float initial_funding);
    void (*release_department)(void* user, uint64_t dept);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* AddRef for a reference handed to the game (slots 21, 22, 47, 49, 61);
     * NULL = add_ref. */
    void (*hand_out)(void* user, uint64_t token);
    /* resources (§7): a NEW fc4_props (the core destroys it), NULL if missing */
    fc4_props* (*tuning)(void* user);
    /* the monthly pass's inputs (§5.3, §5.5) */
    uint64_t (*building_map)(void* user); /* BDS slot 41: BORROWED cISC4SimGrid<uint8>, 0 = none */
    uint8_t (*building_map_key)(void* user, uint32_t purpose, uint32_t wealth);
    /* growth developer slot 15 of the developer type: BORROWED cISC4SimGrid<int16>, 0 = none */
    uint64_t (*developer_population_map)(void* user, uint32_t developer_type);
    uint16_t (*pollution_value)(void* user, uint32_t kind, int32_t cell_x, int32_t cell_z);
    uint32_t (*sim_month)(void* user);
    uint32_t (*sim_day_number)(void* user);
    float (*altitude)(void* user, float x, float z);
    /* visual effects (§5.5 step 2, REH §4.7 step 3, RG1): the effect (one
     * reference, released by start_and_release_effect), 0 = the create
     * failed (one draw), FC4_EFFECT_NOT_SHOWN = the host cannot show effects
     * (treated as created: both draws, the flag). NULL = cannot show. */
    uint64_t (*create_visual_effect)(void* user, const char* name);
    void (*set_effect_transform)(void* user, uint64_t effect, const float* translation_xyz);
    void (*start_and_release_effect)(void* user, uint64_t effect);
    /* messages (§5.7, §8, REH §5) */
    void (*record_history)(void* user, uint32_t series, int32_t value);
    void (*post_strike_message)(void* user, uint32_t msg_type, uint64_t occ, uint32_t data2, uint32_t data3);
    /* occupants (§4.3, §6.6) */
    uint64_t (*occupant_as_building)(void* user, uint64_t occ);
    bool (*is_occupant_group)(void* user, uint64_t occ, uint32_t group);
    bool (*occupant_bool_property)(void* user, uint64_t occ, uint32_t id, bool* out);
    bool (*set_occupant_bool_property)(void* user, uint64_t occ, uint32_t id, bool value);
    /* a reference read with `iid` (one reference handed over): the token the
     * core keeps, 0 = refused */
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
    /* education and health: budget (REH §4.1, §4.2) */
    bool (*budget_item)(void* user, uint64_t occ, uint32_t purpose, fc4_residential_budget_item* out);
    void (*change_budget_item_line)(void* user, uint64_t occ, uint32_t index, uint32_t line);
    uint64_t (*department_budget)(void* user, uint32_t id); /* borrowed, 0 = none */
    float (*department_funding)(void* user, uint64_t dept); /* NaN = cannot be read (RG10) */
    bool (*is_locally_funded)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    void (*add_locally_funded)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    void (*remove_locally_funded)(void* user, uint64_t dept, uint64_t occ, uint32_t purpose);
    void (*set_local_funding)(void* user, uint64_t dept, uint64_t occ, float fraction, uint32_t purpose);
    uint64_t (*line_item)(void* user, uint64_t dept, uint32_t line); /* borrowed, 0 = none */
    uint64_t (*create_line_item_for_building_type)(void* user, uint64_t dept, uint32_t line, bool local_funding);
    void (*remove_line_item)(void* user, uint64_t dept, uint32_t line);
    int64_t (*line_item_count)(void* user, uint64_t line_item);
    void (*set_line_item_count)(void* user, uint64_t line_item, int64_t count);
    void (*set_line_item_display_flag)(void* user, uint64_t line_item, uint32_t flag, bool on);
    /* education and health: occupants (REH §2, §4.1, §4.4-§4.6) */
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* out);
    void (*set_occupant_u32)(void* user, uint64_t occ, uint32_t id, uint32_t value);
    void (*set_occupant_f32)(void* user, uint64_t occ, uint32_t id, float value);
    uint32_t (*building_type)(void* user, uint64_t occ);
    uint32_t (*building_slot11)(void* user, uint64_t occ);
    bool (*occupant_lot_center)(void* user, uint64_t occ, float* xz); /* false = no lot */
    void (*occupant_position)(void* user, uint64_t occ, float* xz);
    /* ordinances (REH §4.3) */
    void (*ordinances_on)(void* user, fc4_u32_callback cb, void* ctx);
    fc4_props* (*ordinance_properties)(void* user, uint32_t id); /* a NEW fc4_props, NULL = skip it */
    /* automata (REH §4.9) */
    void (*add_controllers)(void* user, const char* group_name, uint64_t occ);
    void (*remove_controllers)(void* user, const char* group_name, uint64_t occ);
} fc4_residential_host;

/* Constructor values (§3). `token`: the simulator's own identity; the class's
 * cRZRandom is seeded with derive_seed(session_seed, 0x4990C013). The calling
 * thread becomes the main thread. */
fc4_residential* fc4_residential_create(const fc4_residential_host* host, uint64_t token, uint32_t session_seed);
/* Queued calls are dropped (their occupants released); grids still held are released. */
void fc4_residential_destroy(fc4_residential* p);
bool fc4_residential_init(fc4_residential* p);     /* §4.1: false without the demand node, server or simulator */
bool fc4_residential_shutdown(fc4_residential* p); /* §4.2 */
/* §4.3: the step's "finished" value (false only when a time-sliced NewDay
 * state ran out of budget). `void1`: the occupant token of the occupant
 * messages. Queued (true) off the main thread or during a call. */
bool fc4_residential_do_message(fc4_residential* p, uint32_t msg_type, uint32_t data1, uint32_t data2, uint32_t data3,
                                uint64_t void1);
void fc4_residential_set_work_budget(fc4_residential* p, uint32_t units); /* §5.2 [choice]; default unbounded */
bool fc4_residential_get_snapshot(const fc4_residential* p, fc4_residential_snapshot* out); /* any thread */
bool fc4_residential_get_state(const fc4_residential* p, fc4_residential_state* out);
/* §9.2: the version-7 record; grids as fc4_ref {token, grid CLSID}, occupants
 * and departments as {token, 0}. Off the main thread the detached state is
 * written. */
bool fc4_residential_write(const fc4_residential* p, const fc4_ostream* out);
bool fc4_residential_read(fc4_residential* p, const fc4_istream* in); /* §9.2; before Init */
/* Slots 8 / 9: whether a strike was ended. Off the main thread the end is
 * queued and the answer is the published strike flag. */
bool fc4_residential_end_strike(fc4_residential* p, bool school);
float fc4_residential_strike_chance(const fc4_residential* p, bool school); /* 10 / 11 */
float fc4_residential_system_rating(const fc4_residential* p, bool school); /* 12 / 13 */
/* 14 / 15 (REH §4.11, EH12): resets `totals` +0x14..+0x28 (in/out; the rest
 * untouched), adds the query fields of every building that has one of
 * `groups`, whatever its query returned, and returns their number; `groups`
 * NULL = no group. */
uint32_t fc4_residential_system_totals(const fc4_residential* p, bool school, const uint32_t* groups,
                                       uint32_t group_count, fc4_residential_query_data* totals);
bool fc4_residential_set_occupant_funding(fc4_residential* p, uint64_t occ, float staff, float vehicle,
                                          bool school); /* 19: true */
/* 20: `staff` / `vehicle` in/out; true. */
bool fc4_residential_get_occupant_funding(const fc4_residential* p, uint64_t occ, float* staff, float* vehicle,
                                          bool school);
/* 21 / 22: a NEW float grid per call handed out with one reference (0 = none);
 * `min` / `max` get the range (255 / 0 without a grid). */
uint64_t fc4_residential_average_grid(const fc4_residential* p, bool is_eq, float* min, float* max);
/* 26 / 27: the query record (in/out); false at the first missing field, with
 * the fields read before it written (EH10). */
bool fc4_residential_get_query_data(const fc4_residential* p, bool school, uint64_t occ, fc4_residential_query_data* data);
bool fc4_residential_estimate_capacity(const fc4_residential* p, uint64_t occ, uint32_t* capacity,
                                       uint32_t* staff); /* 28: true */
uint8_t fc4_residential_cell_life_expectancy(const fc4_residential* p, int32_t x, int32_t z);     /* 29 */
float fc4_residential_cell_workforce(const fc4_residential* p, int32_t x, int32_t z);           /* 30 */
float fc4_residential_global_workforce(const fc4_residential* p);                               /* 31 */
float fc4_residential_global_quotient(const fc4_residential* p, bool is_eq);                    /* 32 / 33 */
uint8_t fc4_residential_global_le(const fc4_residential* p);                                    /* 34 */
float fc4_residential_cell_quotient(const fc4_residential* p, bool is_eq, int32_t x, int32_t z); /* 35 / 36 */
float fc4_residential_cell_quotient_by_wealth(const fc4_residential* p, bool is_eq, int32_t x, int32_t z,
                                              uint8_t wealth); /* 37 / 38 (wealth 1..3) */
uint8_t fc4_residential_new_age_by_wealth(const fc4_residential* p, uint8_t wealth); /* 42 */
/* 43 / 44: false (all outputs 0) when the population is 0. */
bool fc4_residential_quotient_min_max(const fc4_residential* p, bool is_eq, fc4_residential_min_max* out);
/* 45: false (outputs untouched) for a building that is neither a school nor a hospital. */
bool fc4_residential_occupant_coverage(const fc4_residential* p, uint64_t occ, float level, float* distance,
                                       float* radius);
/* 47 / 49 / 61 (FC4_RESIDENTIAL_LIST_*): each occupant goes to `sink` with one
 * reference handed out for the caller's list. `groups` NULL = every group;
 * slot 61 ignores `groups`. Returns the count. */
uint32_t fc4_residential_buildings(const fc4_residential* p, uint32_t which, const uint32_t* groups,
                                   uint32_t group_count, fc4_token_callback sink, void* ctx);
bool fc4_residential_set_global_auto_budget(fc4_residential* p, bool school, bool on); /* 53 / 55: the value */
bool fc4_residential_auto_budget(const fc4_residential* p, uint64_t occ);              /* 56 */
bool fc4_residential_set_auto_budget(const fc4_residential* p, uint64_t occ, bool on); /* 57 */
bool fc4_residential_ideal_funding(const fc4_residential* p, uint64_t occ, float* out); /* 58 */

/* ---- residential shadow (tooling) ---------------------------------------------
 *
 * With the ORIGINAL residential simulator running, each SimNewDay delivery to
 * its agent is wrapped (one state of the monthly cycle per day, §5.1): before
 * it, the plugin captures the original's record (its own Write into a capture
 * stream) and calls fc4_residential_shadow_begin, which copies the 25 live
 * grids the record refers to, loads the record into a private core over the
 * copies, restores the simulator tuning and ordinance effects from the game
 * and runs FreeCity4's daily state, reading every input through the host
 * table (read callbacks and the grid table's geometry / get only; nothing is
 * written to the game). States 1, 2 and 4 run at the finish instead, where
 * the original's progress is known (a state it stopped part-way runs as far
 * as it got). After the delivery the plugin captures the record again, reads
 * the original's population, global EQ / HQ and LE, and calls
 * fc4_residential_shadow_finish, which compares the 25 grids cell by cell,
 * the saved fields, the occupant properties and local funding, and the four
 * scalars. fc4_residential_shadow_describe gives the one start-up line of
 * what is not readable (hence not compared). */
typedef struct fc4_residential_shadow fc4_residential_shadow;

/* One reference of a captured record, queried for each interface the reader
 * asks for (0 = the object does not answer it); borrowed identities. */
typedef struct fc4_residential_capture_ref {
    uint64_t grid;       /* cISC4SimGrid 0xC9BD939A */
    uint64_t occupant;   /* cISC4Occupant 0x26BB767E */
    uint64_t department; /* cISC4DepartmentBudget 0x223E2B67 */
    uint32_t class_id;   /* GetGZCLSID() */
    uint32_t reserved;
} fc4_residential_capture_ref;

/* A captured record: primitives little-endian, references as uint32
 * objectID (index into `refs` + 1, 0 = null) then uint32 classID. */
typedef struct fc4_residential_capture {
    uint32_t struct_size;
    uint32_t ok; /* Write returned true and the stream stayed good */
    const uint8_t* bytes;
    size_t len;
    const fc4_residential_capture_ref* refs;
    size_t ref_count;
} fc4_residential_capture;

/* The original's getters after its delivery. */
typedef struct fc4_residential_shadow_scalars {
    uint32_t struct_size;
    int32_t population;         /* slot 16 */
    float global_eq, global_hq; /* slots 32 / 33 */
    uint32_t global_le;         /* slot 34 (AL, zero-extended) */
    uint32_t reserved;
} fc4_residential_shadow_scalars;

typedef struct fc4_residential_shadow_stats {
    uint32_t struct_size;
    uint32_t deliveries, compared, clean, incomplete;
    uint32_t idle;            /* state-10 days (not compared) */
    uint32_t cycles;          /* monthly cycles seen to end (state 9 -> 10) */
    int32_t last_mismatches;  /* -1: not compared */
    uint64_t cells, cell_mismatches, field_mismatches;
    uint64_t prop_mismatches; /* occupant properties and local funding */
    uint64_t scalar_mismatches;
} fc4_residential_shadow_stats;

/* `max_cells`: differing cells listed per grid and line (0 = 4, at most 64). */
fc4_residential_shadow* fc4_residential_shadow_create(const fc4_residential_host* host, uint32_t max_cells);
void fc4_residential_shadow_destroy(fc4_residential_shadow* s);
/* Before a SimNewDay delivery (`julian` = its data1): 0 = ready, -1 = not
 * run (finish reports why), -2 = state 10 (the original does nothing; a
 * finish returns -2 and logs nothing). */
int32_t fc4_residential_shadow_begin(fc4_residential_shadow* s, int32_t julian, const fc4_residential_capture* before);
/* The delivery's difference count (-1 = not compared, -2 = nothing to
 * compare); log lines go to `sink`, one call per line: one per delivery
 * ("residential shadow YYYY-MM-DD state N (name): ...; mismatches=N", or
 * INCOMPLETE and why) and, after the delivery that ends a cycle, the cycle
 * line. `scalars` NULL: not compared. */
int32_t fc4_residential_shadow_finish(fc4_residential_shadow* s, const fc4_residential_capture* after,
                                      const fc4_residential_shadow_scalars* scalars, fc4_bytes_sink sink, void* ctx);
bool fc4_residential_shadow_get_stats(fc4_residential_shadow* s, fc4_residential_shadow_stats* out);
/* One line: what is not readable, hence not compared. */
void fc4_residential_shadow_describe(fc4_bytes_sink sink, void* ctx);

/* ---- development (docs/specs/building-development.md, growth-developer.md) ----
 *
 * The building development simulator (class 0x8990BE7A) and the growth
 * developers (class 0x0990C138), replaced as one unit. The host implements
 * fc4_development_host (every DevelopmentHost / GrowthDevelopers method of
 * engine/core/src/development_simulator, in the traits' order) and
 * fc4_growth_host (engine/core/src/growth_developer); the plugin's classes
 * forward their slots to the fc4_development_* / fc4_growth_* functions.
 *
 * The process-wide statics (BDS §2: the building catalogue, the tried-tract
 * marks and the BDS generator; growth §7: the salt generator and the time
 * slice step) are one fc4_development_process and one fc4_growth_statics the
 * host creates once per process. The catalogue is exported from the process
 * (fc4_development_catalogue_*) so the host can keep the game-layout copies
 * slots 11-22, 27-28, 37-40, 42-43 and 45 return.
 *
 * Tokens are opaque host identities: developers (cISC4GrowthDeveloper*),
 * lots, occupants, the construction node (cISC4Demand*) and grids
 * (fc4_grid_api). 0 is the null object. The cores own one reference on every
 * developer, special building, node, source demand and source map token they
 * keep, and on every occupant create_building_occupant hands over. A saved
 * reference read back for one of the host's objects arrives with that
 * reference; one naming a FreeCity4 object (fc4_ref::core_handle set) is
 * given one (add_ref). Lot tokens are borrowed.
 *
 * Re-entrancy: no core borrow is held across a host callback; a message
 * delivered while a call on the same handle is running is queued until the
 * outermost call returns, other re-entrant calls run at once (counted in
 * reentrant_calls). Threads: single-threaded cores; the plugin serialises
 * every call on the BDS, its process statics and every growth developer with
 * one process-wide lock. */
typedef struct fc4_development_process fc4_development_process;
typedef struct fc4_development fc4_development;
typedef struct fc4_growth_statics fc4_growth_statics;
typedef struct fc4_growth fc4_growth;

typedef struct fc4_res_key {
    uint32_t type_id, group, instance;
} fc4_res_key;

/* GrowthProfile: the developer token, the tract and its stage (the tract
 * developer reads the low byte). */
typedef struct fc4_growth_profile {
    uint64_t developer;
    int32_t x, z, stage;
    uint32_t reserved;
} fc4_growth_profile;

/* One growth or decline list entry: the game's 12-byte {x, z, stage}. */
typedef struct fc4_growth_item {
    int32_t x, z, max_stage;
} fc4_growth_item;

/* ConstructionProfile in the game's layout (BDS §4.8: 0x24 bytes). */
typedef struct fc4_construction_profile {
    int32_t zone;
    int32_t unknown2;
    uint32_t wealth, purpose;
    uint8_t land_value_min, land_value_max;
    uint8_t reserved[2];
    uint32_t building_type;
    int32_t orientation;
    int32_t x, z;
} fc4_construction_profile;

/* One lot configuration as the maximum density table reads it (§4.7.4). */
typedef struct fc4_lot_config {
    uint32_t size_x, size_z;
    uint32_t zones;       /* bit z set: IsCompatibleWithZoneType(z), z in 0..16 */
    uint32_t has_primary; /* 1: a primary building configuration object */
    uint32_t orientation;
    float x, z;
    uint32_t n_ids;
    const uint32_t* ids; /* the object's building or family IDs */
} fc4_lot_config;

/* The catalogue's building profile (§2; the host builds the game layout). */
typedef struct fc4_building_profile {
    uint32_t purpose, wealth;
    uint16_t capacity;
    uint8_t value_499afa38;
    uint8_t flag_27812836;
    uint32_t category_flags;
    float occupant_size[3];
    uint32_t reserved0;
    int64_t plop_cost;
    uint32_t n_capacities; /* capacity table entries (handed to the callback) */
    uint32_t reserved1;
} fc4_building_profile;

typedef struct fc4_development_message {
    uint32_t msg_type, data1, data2, data3, data4;
    uint32_t reserved;
    uint64_t occupant; /* GetVoid1() as a cISC4Occupant (occupant messages), 0 otherwise */
    uint64_t lot;      /* GetIGZUnknown() as a cISC4Lot (StateChanged), 0 otherwise */
} fc4_development_message;

/* One PlaceBuilding command parameter. */
typedef struct fc4_command_param {
    uint32_t as_u32;
    float as_f32;
} fc4_command_param;

/* The advice subject §4.10 builds and discards. */
typedef struct fc4_advice_subject {
    float x, z;
    uint32_t data1;
    uint32_t reserved;
    uint64_t lot;
} fc4_advice_subject;

/* One decline plan entry. */
typedef struct fc4_plan_entry {
    uint32_t day;
    int32_t x, z;
    uint32_t stage;
    uint64_t developer;
} fc4_plan_entry;

/* Diagnostics; the counters are cumulative since the handle was created. */
typedef struct fc4_development_state {
    uint32_t struct_size; /* = sizeof(fc4_development_state) */
    uint8_t initialized, loaded;
    uint8_t state;  /* the planning state machine, 0-4 (§4.2) */
    uint8_t cheats; /* bit 0 ignore plumbing, 1 power, 2 caps, 3 debug growth, 4 coffee bar */
    uint32_t developers, special_buildings, highest_stages, decline_plan;
    uint32_t marked_types, marked_cells;
    uint32_t building_types; /* in the process catalogue */
    uint32_t grow_budget;    /* §4.4.1; 0xFFFFFFFF = unlimited */
    int32_t total_resources, free_resources;
    float supply, demand;
    uint32_t grow_calls, built;       /* tract developer Grow calls, those > 0 */
    uint32_t decline_calls, declined; /* tract developer Decline calls, those > 0 */
    uint32_t built_by_wealth[4];      /* by the developer's wealth */
    uint32_t declined_by_wealth[4];
    uint32_t built_by_stage[16]; /* by the profile's stage (larger count as 15) */
    uint32_t reentrant_calls;
    int32_t cell_count_x, cell_count_z;
    uint32_t reserved;
    uint64_t rng_steps; /* of the process-wide BDS generator */
    uint64_t building_map;
    uint64_t construction_node;
} fc4_development_state;

/* One exemplar: NEW props (the callee owns them; may be NULL) and the
 * localized building name (property 0x8A416A99) when has_name. */
typedef void (*fc4_development_exemplar_visitor)(void* ctx, const fc4_res_key* key, fc4_props* props,
                                                 const uint8_t* name, size_t name_len, bool has_name);
typedef void (*fc4_lot_config_visitor)(void* ctx, const fc4_lot_config* config);
/* A growth or decline list, delivered in one call. */
typedef void (*fc4_growth_items_sink)(void* ctx, const fc4_growth_item* items, size_t n);
typedef void (*fc4_type_token_callback)(void* ctx, uint32_t key, uint64_t token);
typedef void (*fc4_type_value_callback)(void* ctx, uint32_t key, uint32_t value);
typedef void (*fc4_plan_entry_callback)(void* ctx, const fc4_plan_entry* e);
/* One maximum density entry: zone, building map key, the raw entry and
 * GetMaxCapacity's value for it (float(trunc(entry))). */
typedef void (*fc4_density_callback)(void* ctx, uint32_t zone, uint32_t key, float entry, float max_capacity);
typedef void (*fc4_key_float_callback)(void* ctx, uint32_t key, float value);

typedef struct fc4_development_host {
    uint32_t struct_size; /* sizeof(fc4_development_host) */
    void* user;
    fc4_grid_api grids;
    /* clock, city, terrain */
    uint32_t (*sim_day)(void* user);
    uint32_t (*difficulty)(void* user);
    void (*cell_widths)(void* user, float* out2);
    void (*terrain_cell_counts)(void* user, int32_t* out2);
    void (*position_to_cell)(void* user, float x, float z, int32_t* out2);
    void (*cell_center)(void* user, int32_t x, int32_t z, float* out2); /* the centre's (x, z) */
    float (*altitude)(void* user, float x, float z);
    /* resources: every exemplar whose ExemplarType is one of `types` (all
     * when n_types is 0), in the resource key list's order */
    void (*exemplars)(void* user, const uint32_t* types, size_t n_types, fc4_development_exemplar_visitor visit,
                      void* ctx);
    void (*lot_configurations)(void* user, fc4_lot_config_visitor visit, void* ctx);
    fc4_props* (*tuning)(void* user); /* NEW props (the core destroys them), NULL if missing */
    /* framework */
    void (*register_static_cleanup)(void* user);
    void (*post_building_types_updated)(void* user, uint32_t count);
    void (*subscribe)(void* user, const uint32_t* types, size_t n);
    void (*unsubscribe)(void* user, const uint32_t* types, size_t n);
    void (*register_command)(void* user, uint32_t id, const char* name, size_t len);
    void (*unregister_command)(void* user, uint32_t id);
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t len, uint32_t flags);
    void (*remove_agent)(void* user);
    void (*register_cheat)(void* user, uint32_t id, const char* name, size_t len);
    void (*unregister_cheat)(void* user, uint32_t id);
    void (*acquire_services)(void* user);
    void (*release_services)(void* user);
    /* demand */
    bool (*census_supply)(void* user, uint32_t id, float* out);
    uint64_t (*create_construction_node)(void* user); /* new cSC4Demand, one reference, not Inited */
    void (*node_init)(void* user, uint64_t node);
    void (*node_shutdown)(void* user, uint64_t node);
    float (*node_supply)(void* user, uint64_t node);
    float (*node_demand)(void* user, uint64_t node);
    void (*node_set_supply)(void* user, uint64_t node, float v);
    void (*node_set_demand)(void* user, uint64_t node, float v);
    void (*node_add_to_demand)(void* user, uint64_t node, float v);
    bool (*node_end_of_cycle)(void* user, uint64_t node);
    float (*demand_active_value)(void* user, uint64_t node); /* a developer's source node */
    /* tract developer: Grow (the host makes and frees the outcome list; flag
     * arrives false) and Decline */
    int32_t (*tract_grow)(void* user, const fc4_growth_profile* profile, bool* flag);
    int32_t (*tract_decline)(void* user, const fc4_growth_profile* profile);
    /* lots, lot manager, lot developer */
    uint32_t (*lot_building_type)(void* user, uint64_t lot);
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    bool (*lot_growth_stage)(void* user, uint64_t lot, uint8_t* out);
    uint64_t (*lot_building)(void* user, uint64_t lot);
    uint32_t (*lot_state)(void* user, uint64_t lot);
    void (*lot_center)(void* user, uint64_t lot, float* out3);
    float (*lot_slope)(void* user, uint64_t lot);
    bool (*lot_demolishable)(void* user, uint64_t lot);
    uint64_t (*lot_at)(void* user, int32_t x, int32_t z); /* GetLot(x, z, true), borrowed */
    /* borrowed tokens kept alive until the outermost BDS call returns */
    void (*lots_in_cell_rect)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    float (*lot_developer_slope)(void* user, uint64_t lot);
    void (*update_lot_elements)(void* user, uint64_t lot, uint32_t kind);
    float (*area_altitude)(void* user, const float* rect4);
    /* zones, land value, power, demolition, budget */
    int8_t (*zone_type)(void* user, int32_t x, int32_t z);
    void (*dezone)(void* user, const fc4_cell_rect* rect, bool flag);
    bool (*powered)(void* user, int32_t x, int32_t z);
    uint32_t (*land_value_type)(void* user, int32_t x, int32_t z);
    uint8_t (*land_value)(void* user, int32_t x, int32_t z);
    int64_t (*demolish_lots)(void* user, const uint64_t* lots, size_t n); /* the cost */
    int64_t (*total_funds)(void* user);
    void (*set_total_funds)(void* user, int64_t v);
    /* occupants */
    bool (*has_occupant_manager)(void* user);
    bool (*insert_occupant)(void* user, uint64_t occ);
    bool (*remove_occupant)(void* user, uint64_t occ);
    bool (*building_occupant_at_cell)(void* user, int32_t x, int32_t z);
    bool (*occupant_has_property)(void* user, uint64_t occ, uint32_t id);
    bool (*occupant_property_i64)(void* user, uint64_t occ, uint32_t id, int64_t* out);
    bool (*occupant_in_group)(void* user, uint64_t occ, uint32_t group);
    void (*occupant_position)(void* user, uint64_t occ, float* out3);
    uint64_t (*create_building_occupant)(void* user); /* one reference for the core */
    void (*set_building_type)(void* user, uint64_t occ, uint32_t type);
    bool (*load_building_occupant)(void* user, uint64_t occ, uint32_t type); /* §4.9 */
    void (*occupant_bounding_box)(void* user, uint64_t occ, float* out6);    /* min then max */
    void (*set_occupant_bounding_box)(void* user, uint64_t occ, const float* box6);
    void (*set_occupant_position)(void* user, uint64_t occ, const float* pos3);
    void (*set_occupant_orientation)(void* user, uint64_t occ, int32_t orientation);
    void (*init_occupant)(void* user, uint64_t occ);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* advisors */
    void (*push_advisor_event)(void* user, uint32_t advisor, uint32_t event, const fc4_advice_subject* subject);
    /* growth developers (the game's cISC4GrowthDeveloper slots) */
    uint64_t (*create_developer)(void* user, const fc4_res_key* key); /* class 0x0990C138, one reference */
    bool (*developer_init)(void* user, uint64_t dev, const fc4_res_key* key); /* Init(holder of key) */
    bool (*developer_shutdown)(void* user, uint64_t dev);
    uint32_t (*developer_sub_type)(void* user, uint64_t dev);
    uint32_t (*developer_purpose)(void* user, uint64_t dev);
    bool (*developer_lists_valid)(void* user, uint64_t dev);
    /* which: 0 GetGrowthList, 1 GetDeclineList; delivered in one call */
    void (*developer_list)(void* user, uint64_t dev, uint32_t which, fc4_growth_items_sink sink, void* ctx);
    uint64_t (*developer_source_demand)(void* user, uint64_t dev);
    void (*developer_start_map_update)(void* user, uint64_t dev);
    bool (*developer_map_update_finished)(void* user, uint64_t dev);
} fc4_development_host;

/* The process statics; `session_seed` fixes the clock seeds (tests). */
fc4_development_process* fc4_development_process_create(uint32_t session_seed);
void fc4_development_process_destroy(fc4_development_process* pp);
bool fc4_development_catalogue_loaded(const fc4_development_process* pp); /* §1.3 step 1 has run */
/* which: 0 building types, 1 families, 2 foundation types, 3 wall types,
 * 4 profiled types. Returns the count; copies at most n. */
size_t fc4_development_catalogue_types(const fc4_development_process* pp, uint32_t which, uint32_t* out, size_t n);
size_t fc4_development_catalogue_family(const fc4_development_process* pp, uint32_t family, uint32_t* out, size_t n);
/* which: 0 building, 2 foundation, 3 wall exemplar key of type t */
bool fc4_development_catalogue_key(const fc4_development_process* pp, uint32_t which, uint32_t t, fc4_res_key* out);
/* which: 0 exemplar name (0x20), 1 localized building name; valid building types only */
bool fc4_development_catalogue_name(const fc4_development_process* pp, uint32_t which, uint32_t t,
                                   fc4_bytes_sink sink, void* ctx);
/* The profile of t; the capacity table goes to cb (key, capacity). */
bool fc4_development_catalogue_profile(const fc4_development_process* pp, uint32_t t, fc4_building_profile* out,
                                       fc4_type_value_callback cb, void* ctx);
size_t fc4_development_catalogue_density(const fc4_development_process* pp, fc4_density_callback cb, void* ctx);
float fc4_development_catalogue_max_capacity(const fc4_development_process* pp, uint32_t zone, uint32_t purpose,
                                             uint32_t wealth); /* slot 45 */
uint32_t fc4_development_developer_type(uint32_t purpose, uint32_t wealth);
uint32_t fc4_development_wealth_of(uint32_t dev_type);
uint32_t fc4_development_building_map_key(uint32_t purpose, uint32_t wealth); /* 4p + w + 1 */
/* 0 none, 1 primary, 2 message target, 3 message target 2, 4 serializable */
uint32_t fc4_development_query_interface(uint32_t iid);

fc4_development* fc4_development_create(const fc4_development_host* host, const fc4_development_process* pp,
                                        uint64_t token);
void fc4_development_destroy(fc4_development* p); /* releases every token the core holds */
bool fc4_development_init(fc4_development* p);    /* §1.3 */
bool fc4_development_shutdown(fc4_development* p);
bool fc4_development_do_message(fc4_development* p, const fc4_development_message* m,
                                const fc4_command_param* params, size_t n_params);
void fc4_development_set_grow_budget(fc4_development* p, uint32_t n); /* §4.4.1; 0 = unlimited */
/* The primary interface (§7.1); the slot in the comment. */
bool fc4_development_location_is_ok(const fc4_development* p, uint32_t t, float x, float z); /* 5 */
uint64_t fc4_development_add_building_at_cell(fc4_development* p, uint32_t t, int32_t cx, int32_t cz,
                                              int32_t orientation); /* 6 */
uint64_t fc4_development_add_building(fc4_development* p, uint32_t t, const float* pos3, int32_t orientation); /* 7 */
bool fc4_development_insert_occupant(fc4_development* p, uint64_t occ);    /* 8 / 24 */
int64_t fc4_development_demolition_cost(const fc4_development* p, uint64_t occ); /* 10 */
bool fc4_development_demolish_foundation(fc4_development* p, uint64_t occ); /* 25 */
uint64_t fc4_development_developer(const fc4_development* p, uint32_t dev_type); /* 32, borrowed */
uint32_t fc4_development_developers(const fc4_development* p, fc4_type_token_callback cb, void* ctx); /* 34 */
uint64_t fc4_development_building_map(const fc4_development* p);                                     /* 41 */
uint32_t fc4_development_special_buildings(const fc4_development* p, fc4_token_callback cb, void* ctx); /* 44 */
/* 46: `reason` written only on failure */
bool fc4_development_is_suitable(const fc4_development* p, const fc4_construction_profile* profile, int32_t* reason);
uint64_t fc4_development_plop_building(fc4_development* p, const fc4_construction_profile* profile); /* 47 */
int32_t fc4_development_free_resources(const fc4_development* p);                                    /* 48 */
int32_t fc4_development_total_resources(const fc4_development* p);                                   /* 49 */
bool fc4_development_claim(fc4_development* p, uint16_t amount);                                      /* 50 */
bool fc4_development_release_resources(fc4_development* p, uint16_t amount);                         /* 51 */
int32_t fc4_development_last_successful_build(const fc4_development* p, uint32_t dev_type);           /* 52 */
float fc4_development_stage_population_multiplier(const fc4_development* p);                         /* 53 */
bool fc4_development_cheat(const fc4_development* p, uint32_t which); /* 54-58: which 0..4 */
uint32_t fc4_development_highest_stages(const fc4_development* p, fc4_type_value_callback cb, void* ctx);
uint32_t fc4_development_decline_plan(const fc4_development* p, fc4_plan_entry_callback cb, void* ctx);
bool fc4_development_get_state(const fc4_development* p, fc4_development_state* out);
bool fc4_development_write(const fc4_development* p, const fc4_ostream* out); /* record v4 (§6) */
bool fc4_development_read(fc4_development* p, const fc4_istream* in);        /* before Init */

/* ---- growth developer (docs/specs/growth-developer.md) ---- */
typedef struct fc4_growth_message {
    uint32_t msg_type, data1, data2, data3, data4;
    int32_t rect[4]; /* ZoningChange: void2's {x1, z1, x2, z2} */
    uint32_t reserved;
    uint64_t unknown; /* StateChanged: GetIGZUnknown() as a lot (borrowed), 0 = none */
} fc4_growth_message;

typedef struct fc4_growth_state {
    uint32_t struct_size; /* = sizeof(fc4_growth_state) */
    uint8_t initialized;
    uint8_t update_state; /* 0 idle, 1..20 (§3.2) */
    uint8_t has_salt, message_server;
    uint32_t developer_type, purpose, wealth;
    int32_t target_zone;
    uint32_t growth_items, decline_items;
    int32_t total_capacity, last_phantom_sum;
    float last_city_size; /* of the last state 20 (§5.5) */
    uint32_t last_salt, start_day, specials, services, tract_size, reentrant_calls, reserved;
    uint64_t draws; /* salts drawn from the shared generator (all developers) */
    uint64_t source_demand;
    uint64_t desirability_map, capacity_map, population_map, max_capacity_map, zoned_map, working_map;
} fc4_growth_state;

/* source_map codes */
#define FC4_GROWTH_SOURCE_INTRINSIC_LAND_VALUE  0u
#define FC4_GROWTH_SOURCE_TOTAL_LAND_VALUE      1u
#define FC4_GROWTH_SOURCE_SLOPE                 2u
#define FC4_GROWTH_SOURCE_RESIDENTIAL_PROXIMITY 3u /* arg: wealth */
#define FC4_GROWTH_SOURCE_COMMERCIAL_PROXIMITY  4u /* arg: type */
#define FC4_GROWTH_SOURCE_PARK                  5u
#define FC4_GROWTH_SOURCE_POLLUTION             6u /* arg: index */
#define FC4_GROWTH_SOURCE_SCHOOL_GRADE          7u
#define FC4_GROWTH_SOURCE_HOSPITAL_GRADE        8u
#define FC4_GROWTH_SOURCE_CRIME                 9u
#define FC4_GROWTH_SOURCE_TRIP_LENGTH           10u
#define FC4_GROWTH_SOURCE_COMMERCIAL_TRAFFIC    11u
#define FC4_GROWTH_SOURCE_LANDMARK              12u
#define FC4_GROWTH_SOURCE_TRANSIENT_AURA        13u

typedef struct fc4_growth_host {
    uint32_t struct_size; /* sizeof(fc4_growth_host) */
    void* user;
    fc4_grid_api grids;
    /* lifecycle; services: 0 city, 1 BDS, 2 occupant manager, 3 zone
     * manager, 4 land value, 5 traffic (false when null) */
    bool (*acquire_service)(void* user, uint32_t service);
    void (*release_service)(void* user, uint32_t service);
    bool (*acquire_message_server)(void* user);
    void (*release_message_server)(void* user);
    void (*subscribe)(void* user, const uint32_t* types, size_t n);
    void (*unsubscribe)(void* user, const uint32_t* types, size_t n);
    bool (*simulator_present)(void* user);
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t len, uint32_t flags);
    void (*remove_agent)(void* user);
    uint32_t (*sim_date_number)(void* user);
    void (*grid_set_tract_size)(void* user, uint64_t grid, int32_t size);
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* demand and neighbours */
    uint64_t (*get_demand)(void* user, uint32_t id, uint32_t index); /* borrowed */
    float (*query_supply)(void* user, uint64_t node);
    void (*add_to_demand_value)(void* user, uint64_t node, float v);
    float (*active_demand)(void* user, uint64_t node);
    uint32_t (*connected_cities)(void* user);
    uint32_t (*all_connected_cities)(void* user);
    /* the BDS */
    float (*stage_population_multiplier)(void* user);
    uint64_t (*tract_developer)(void* user); /* borrowed */
    void (*special_buildings)(void* user, fc4_token_callback cb, void* ctx); /* borrowed, list order */
    float (*max_capacity)(void* user, int32_t zone, uint32_t purpose, uint32_t wealth);
    uint32_t (*building_purpose)(void* user, uint32_t type);
    /* the tract developer */
    void (*update_population)(void* user, uint64_t tract_developer, const fc4_growth_profile* profile);
    /* zones, lots, occupants, city */
    int8_t (*zone_type)(void* user, int32_t x, int32_t z);
    uint32_t (*lot_building_type)(void* user, uint64_t lot, bool flag);
    uint16_t (*lot_capacity)(void* user, uint64_t lot, uint32_t dev_type, bool flag);
    void (*lot_size)(void* user, uint64_t lot, uint8_t* out2);
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    void (*lot_location)(void* user, uint64_t lot, int32_t* out2);
    bool (*occupant_property)(void* user, uint64_t occ, uint32_t id, fc4_props* scratch);
    void (*occupant_position)(void* user, uint64_t occ, float* out3);
    void (*position_to_cell)(void* user, float x, float z, int32_t* out2);
    /* source maps: a grid token with one reference for the core; 0 = null */
    uint64_t (*source_map)(void* user, uint32_t code, uint32_t arg);
    /* time slicing and seeds */
    void (*limit_timer_start)(void* user, uint32_t ms);
    bool (*limit_timer_expired)(void* user);
    uint32_t (*wall_clock_seed)(void* user);
} fc4_growth_host;

fc4_growth_statics* fc4_growth_statics_create(uint32_t session_seed);
void fc4_growth_statics_destroy(fc4_growth_statics* s);
/* 0 none, 1 primary, 2 message target 2, 3 serializable */
uint32_t fc4_growth_query_interface(uint32_t iid);
fc4_growth* fc4_growth_create(const fc4_growth_host* host, const fc4_growth_statics* statics, uint64_t token);
void fc4_growth_destroy(fc4_growth* p);                /* releases every token the core holds */
bool fc4_growth_init(fc4_growth* p, fc4_props* props); /* slot 3; props NEW (consumed), NULL fails */
bool fc4_growth_shutdown(fc4_growth* p);
bool fc4_growth_do_message(fc4_growth* p, const fc4_growth_message* m);
int32_t fc4_growth_target_zone(const fc4_growth* p);
uint32_t fc4_growth_sub_type(const fc4_growth* p);
uint32_t fc4_growth_purpose(const fc4_growth* p);
uint32_t fc4_growth_wealth(const fc4_growth* p);
/* Writes the info fields (zone .. land value range); the rest untouched. */
void fc4_growth_construction_info(const fc4_growth* p, fc4_construction_profile* out);
bool fc4_growth_lists_valid(const fc4_growth* p);
/* which: 0 growth, 1 decline (after the §5.4 sort). Returns the count; copies at most n. */
size_t fc4_growth_list(const fc4_growth* p, uint32_t which, fc4_growth_item* out, size_t n);
/* which: 0 D, 1 capacity, 2 population, 3 max capacity, 4 zoned, 5 working (borrowed) */
uint64_t fc4_growth_grid(const fc4_growth* p, uint32_t which);
int32_t fc4_growth_total_capacity(const fc4_growth* p);
uint64_t fc4_growth_source_demand(const fc4_growth* p);
void fc4_growth_caps(const fc4_growth* p, uint8_t* out4);
int32_t fc4_growth_desirability_threshold(const fc4_growth* p);
float fc4_growth_abandonment_threshold(fc4_growth* p, int32_t key);
uint32_t fc4_growth_monopoly_use(const fc4_growth* p);
size_t fc4_growth_styles(const fc4_growth* p, fc4_key_float_callback cb, void* ctx);
void fc4_growth_start_map_update(fc4_growth* p);
bool fc4_growth_map_update_finished(const fc4_growth* p);
int32_t fc4_growth_estimate_desirability(const fc4_growth* p, int32_t x, int32_t z);
void fc4_growth_subjective_thresholds(const fc4_growth* p, int32_t i, int32_t* min, int32_t* max);
bool fc4_growth_source_active_demand(const fc4_growth* p, float* out);
bool fc4_growth_get_state(const fc4_growth* p, fc4_growth_state* out);
bool fc4_growth_write(const fc4_growth* p, const fc4_ostream* out); /* record v1 */
bool fc4_growth_read(fc4_growth* p, const fc4_istream* in);        /* before Init */
/* ---- zone manager (docs/specs/zones.md §1.1, §2-§6, §10.1, §11.1) -------------
 *
 * The zone manager (cSC4ZoneManager, class 0x298F9B2D, city component 6): the
 * zone grid (a game int8 grid 0x49B9E603 through `grids`, tract size 1,
 * instance 0, saved by reference), the per-type zone counts, the tuning, the
 * dezone under new networks (InsertOccupant 0x99EF1142) and PlaceZone. The
 * zone developer / lot manager stays the game's: the host reaches it for
 * get_lot, lots_in_cell_rect and do_parcellization.
 *
 * Tokens are host identities of any interface of the object (the host queries
 * what it needs); those the host returns are BORROWED unless stated. 0 = none.
 *
 * Threads: single-threaded; the plugin serialises every call on one object and
 * publishes fc4_zones_get_state / fc4_zones_get_tuning for its other threads.
 * Re-entrant calls (a dezone during a commit, GetZoneType from the zone
 * developer) run at once and are counted.
 *
 * Shadow mode (fc4_zones_create_shadow): reads go to the game, writes are
 * swallowed (services, subscriptions, funds, the commit's demolition,
 * parcellization, ZoningChange, lot textures); references read are borrowed
 * (nothing taken, adopted or released). Give the shadow a grid table without
 * set / fill / shutdown / release. */

typedef struct fc4_zones fc4_zones;

/* The "Zone Manager" tuning exemplar (§4). */
#define FC4_ZONES_TUNING_TYPE 0x6534284Au
#define FC4_ZONES_TUNING_GROUP 0xE7E2C2DBu
#define FC4_ZONES_TUNING_INSTANCE 0xE9482490u

/* The services the record saves (§10.1 field 13), in record order. */
#define FC4_ZONES_SERVICE_CITY 0u
#define FC4_ZONES_SERVICE_OCCUPANT_MANAGER 1u
#define FC4_ZONES_SERVICE_TERRAIN 2u
#define FC4_ZONES_SERVICE_POLLUTION 3u
#define FC4_ZONES_SERVICE_BUDGET 4u

/* A cell region (the game's SC4CellRegion<long>, §11.4): an inclusive rect and
 * one byte per cell, index (z - z0) * width + (x - x0), non-zero = set. NULL
 * mask: every cell set. A short mask reads as cleared cells past its end. */
typedef struct fc4_zone_region {
    int32_t x0, z0, x1, z1;
    uint8_t* mask;
    size_t len;
} fc4_zone_region;

/* PlaceZone's outputs (§6). */
typedef struct fc4_zones_place_result {
    uint32_t struct_size;
    uint8_t ok;        /* AL */
    uint8_t committed; /* a change was committed and ZoningChange posted */
    uint8_t reserved[2];
    int32_t error;     /* 0 ok, 1 funds, 2 landfill on water, 3 too small, 4 too large, 5 blocked */
    uint32_t accepted; /* cells of the ZoningChange region (0 unless committed) */
    uint32_t cleared;  /* region bits this call cleared (rejected cells) */
    uint32_t lots;     /* lots to demolish (*outLots) */
    int64_t cost;      /* *outCost */
} fc4_zones_place_result;

/* What a DoMessage did (§5.1). */
typedef struct fc4_zones_dezone {
    uint32_t struct_size;
    uint8_t dezoned; /* PlaceZone(rect, 0, ...) ran */
    uint8_t reserved[3];
    fc4_cell_rect rect; /* the occupant's bounding cells */
    uint32_t changed;   /* cells re-zoned (the ZoningChange region) */
    uint32_t lots;      /* lots handed to DemolishLots(true, ...) */
} fc4_zones_dezone;

/* Diagnostics and the getters' snapshot. */
typedef struct fc4_zones_state {
    uint32_t struct_size;
    uint8_t initialized, loaded, display_alpha, default_alpha;
    uint32_t zone_count[16];                          /* slot 6 */
    uint32_t undeveloped[16];                         /* slot 7 (never updated after Init, §2) */
    uint32_t undeveloped_total, abandoned_tile_count; /* slots 8, 10 */
    uint32_t failure1_entries;                        /* development-failure map 1 entries */
    uint32_t reentrant_calls, place_calls, dezones;
    int32_t grid_count_x, grid_count_z; /* 0 x 0 without a grid or before its record is read */
    uint64_t grid;                      /* slot 5 (borrowed); 0 = none */
} fc4_zones_state;

/* The tuning (§4), indexed by zone type. */
typedef struct fc4_zones_tuning {
    uint32_t struct_size;
    uint8_t display_alpha, default_alpha;
    uint8_t reserved[2];
    int32_t min_size[16], max_size[16];
    int64_t create_cost[16], destroy_cost[16];
    uint32_t texture[16], drag_color[16];
    float max_slope[16];
} fc4_zones_tuning;

/* Everything the zone manager asks of the game. */
typedef struct fc4_zones_host {
    uint32_t struct_size;
    void* user;
    fc4_grid_api grids; /* the zone grid (int8 0x49B9E603) */
    /* lifecycle and services (§1.1, §10.1). acquire_services: Init step 2 (the city, occupant manager,
     * terrain, pollution and budget simulators still null, and the message server). */
    void (*acquire_services)(void* user);
    void (*release_services)(void* user);
    void (*subscribe)(void* user, uint32_t msg);
    void (*unsubscribe)(void* user, uint32_t msg);
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it); NULL = missing */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    uint64_t (*service_ref)(void* user, uint32_t which); /* Write: the token saved for FC4_ZONES_SERVICE_*; 0 = null */
    /* Read: service `which` as read (its IID's interface; 0 = null). The host takes the token's ONE
     * reference and replaces the service it held. */
    void (*adopt_service)(void* user, uint32_t which, uint64_t token);
    /* ZoningChange 0x07699455 (§5.3): MessagePost of a message owning a copy of the region. */
    void (*post_zoning_change)(void* user, const fc4_zone_region* region, int32_t zone_type, bool check_slope);
    /* the zone developer / lot manager (§13.1) */
    uint64_t (*get_lot)(void* user, int32_t x, int32_t z); /* lot manager slot 6 GetLot(x, z, false) */
    /* lot manager slot 7: cb(ctx, lot) per lot, ONE reference each for the core */
    void (*lots_in_cell_rect)(void* user, const fc4_cell_rect* rect, bool origin_only, fc4_token_callback cb,
                              void* ctx);
    void (*do_parcellization)(void* user, const fc4_zone_region* region, int32_t zone_type, bool flag);
    /* budget, demolition, landfill (§6) */
    int64_t (*total_funds)(void* user);
    bool (*withdraw_funds)(void* user, int64_t amount);
    /* DemolishLots(commit, lots, checkSlope, flags, 0, 0, &cost, 0) -> cost */
    int64_t (*demolish_lots)(void* user, bool commit, const uint64_t* lots, size_t count, bool check_slope,
                             uint32_t flags);
    int64_t (*landfill_zoning_cost)(void* user, const fc4_zone_region* region);
    /* network tiles (§6) */
    uint64_t (*find_network_tile)(void* user, int32_t x, int32_t z); /* FindNetworkTile(x, z, 0x1F4F, 1) */
    bool (*tile_occupant_flag_set)(void* user, uint64_t tile, uint32_t flag);
    bool (*tile_is_of_type)(void* user, uint64_t tile, uint32_t type);
    int64_t (*tile_demolition_cost)(void* user, uint64_t tile);
    /* terrain, lot developer */
    bool (*cell_has_water)(void* user, int32_t x, int32_t z);
    float (*area_slope)(void* user, const fc4_cell_rect* rect, bool with_water); /* lot developer slot 29 */
    void (*set_empty_lot_base_texture)(void* user, uint64_t lot);                 /* lot developer slot 43 */
    /* occupants (§5.1, §6) */
    uint64_t (*landfill_at)(void* user, int32_t x, int32_t z); /* occupant manager slot 33, type 0x6A11BD2B */
    void (*occupant_bounding_cells)(void* user, uint64_t occupant, fc4_cell_rect* out); /* occupant slot 11 */
    bool (*occupant_can_remove)(void* user, uint64_t occupant, uint32_t flags);        /* occupant slot 14 */
    /* QueryInterface(cISC4NetworkOccupant) + slot 9: 1 / 0, -1 = not a network occupant */
    int32_t (*network_has_any_flag)(void* user, uint64_t occupant, uint32_t flags);
    /* references */
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* lots and lot configurations */
    uint64_t (*lot_configuration)(void* user, uint64_t lot);
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    int32_t (*lot_zone_type)(void* user, uint64_t lot);
    uint32_t (*lot_state)(void* user, uint64_t lot);
    uint64_t (*lot_building)(void* user, uint64_t lot);
    bool (*cfg_compatible_with_zone)(void* user, uint64_t cfg, int32_t zone_type);
} fc4_zones_host;

bool fc4_zones_query_interface(uint32_t iid);  /* 1, 0x67652433, 0x452294AA, 0xE4FDA3D4 */
uint32_t fc4_zones_class_id(void);             /* 0x298F9B2D */
bool fc4_zones_is_rci_zone(int32_t zone_type); /* slot 13 */
/* NULL when the table is incomplete. The table is copied. */
fc4_zones* fc4_zones_create(const fc4_zones_host* host);
fc4_zones* fc4_zones_create_shadow(const fc4_zones_host* host);
/* A grid still held is shut down and released. */
void fc4_zones_destroy(fc4_zones* z);
bool fc4_zones_init(fc4_zones* z);
bool fc4_zones_shutdown(fc4_zones* z);
/* Always true. `occupant` = void1 (0 = none); `out` may be NULL. */
bool fc4_zones_do_message(fc4_zones* z, uint32_t msg_type, uint64_t occupant, uint32_t data3, uint32_t data4,
                          fc4_zones_dezone* out);
/* Slot 11 (§6). The region is written back (rejected cells cleared; a NULL mask cannot report them).
 * `lots` (may be NULL): every lot to demolish, borrowed, called at the pricing call, before the commit;
 * the caller takes its own references. A NULL region, or one above 1,048,576 cells or 4096 a side, fails
 * with error 0 and calls nothing. */
bool fc4_zones_place_zone(fc4_zones* z, fc4_zone_region* region, int32_t zone_type, bool commit, bool skip_funds,
                          bool check_slope, bool skip_size_check, bool check_lot_inside, fc4_token_callback lots,
                          void* ctx, fc4_zones_place_result* out);
uint64_t fc4_zones_grid(const fc4_zones* z);                                      /* slot 5 */
uint32_t fc4_zones_zone_count(const fc4_zones* z, int32_t zone_type);             /* slot 6 */
uint32_t fc4_zones_undeveloped_tile_count(const fc4_zones* z, int32_t zone_type); /* slot 7 */
uint32_t fc4_zones_undeveloped_total(const fc4_zones* z);                         /* slot 8 */
/* Slot 9: inserts a zero entry into map 1 when absent (§11.1 quirk). */
uint32_t fc4_zones_development_failure_count(fc4_zones* z, int32_t zone, int32_t result);
uint32_t fc4_zones_abandoned_tile_count(const fc4_zones* z);          /* slot 10 */
int32_t fc4_zones_zone_type(const fc4_zones* z, int32_t x, int32_t z_); /* slot 12 */
/* Slot 14: false for type >= 16; key = {type, group, instance} (3 x uint32). */
bool fc4_zones_texture_for_zone(const fc4_zones* z, int32_t zone_type, uint32_t* key);
uint32_t fc4_zones_drag_color(const fc4_zones* z, int32_t zone_type); /* slot 15 */
uint8_t fc4_zones_display_alpha(const fc4_zones* z);                  /* slot 16 */
uint8_t fc4_zones_default_display_alpha(const fc4_zones* z);          /* slot 17 */
void fc4_zones_set_display_alpha(fc4_zones* z, uint8_t alpha);        /* slot 18 */
int32_t fc4_zones_min_size(const fc4_zones* z, int32_t zone_type);    /* slot 19 */
int32_t fc4_zones_max_size(const fc4_zones* z, int32_t zone_type);    /* slot 20 */
bool fc4_zones_get_state(const fc4_zones* z, fc4_zones_state* out);
bool fc4_zones_get_tuning(const fc4_zones* z, fc4_zones_tuning* out);
/* Game writes the last place_zone / do_message call swallowed (shadow mode). */
uint32_t fc4_zones_last_swallowed(const fc4_zones* z);
bool fc4_zones_write(const fc4_zones* z, const fc4_ostream* out);
/* Runs before Init: nothing is sized from the city. */
bool fc4_zones_read(fc4_zones* z, const fc4_istream* in);

/* ---- neighbours (docs/specs/neighbors.md; engine/capi/src/neighbors.rs) ------
 *
 * The plugin's cISC4NeighborsSimulator forwards its slots to fc4_neighbors_*,
 * its deals (cISC4NeighborDeal) to fc4_neighbor_deal_* and its connections
 * (cISC4NeighborConnection) to fc4_neighbor_connection_*. Tokens are host
 * identities (regional cities, the city, occupants, props, connections,
 * deals); 0 is null. A connection or deal of ours is a core object behind a
 * handle that its GZCOM object owns (the object destroys the handle when it
 * dies); the core asks for the object through `connection_object` /
 * `deal_object`. A foreign connection (an original accepted on load, §15.2)
 * is read and written through the `foreign_connection` callbacks; foreign
 * deals reach the core as data. The core is re-entrant: a message delivered
 * during another call (the status report SetAmount SENDS, §5.3) is handled at
 * once. Main thread only (the plugin serialises other threads). */
typedef struct fc4_neighbors fc4_neighbors;
typedef struct fc4_neighbor_deal fc4_neighbor_deal;
typedef struct fc4_neighbor_connection fc4_neighbor_connection;

/* A deal's state (§5.1; the property holder is not part of it). Dates are
 * Julian days. */
typedef struct fc4_neighbor_deal_data {
    uint32_t deal_type;
    int32_t other_x, other_z; /* regionCellOtherSide */
    int32_t city_x, city_z;   /* regionCellCitySide */
    int32_t amount, inc_amount, inc_amount2, last_amount, last_inc_amount, amount_used;
    uint32_t flags;
    int64_t cost;
    uint32_t start_date, expiration_date, current_date, suspension_date;
} fc4_neighbor_deal_data;

/* A connection's state (§4.1). */
typedef struct fc4_neighbor_connection_state {
    uint32_t conn_type;
    int32_t x, z;             /* the connection cell */
    int32_t other_x, other_z; /* otherRegionCell */
    uint8_t visited, active;
    uint8_t reserved[2];
} fc4_neighbor_connection_state;

/* What UpdateCommodityStats reads (§8.2). A department is missing when its
 * has_* byte is 0. */
typedef struct fc4_neighbors_figures {
    double total_funds, monthly_income;
    double power_production, power_requirement;
    double water_capacity, water_demand;
    double garbage_recycled, waste_to_energy_capacity, incinerator_capacity, garbage_abducted, garbage_produced;
    uint8_t has_power_department, has_water_department, has_sanitation_department;
    uint8_t reserved[5];
    double power_department_expenses, water_department_expenses, sanitation_department_expenses;
    double waste_to_energy_expenses; /* line item 0x1F470000 of the power department */
} fc4_neighbors_figures;

typedef struct fc4_neighbors_message {
    uint32_t msg_type, data1, data2, data3;
    uint64_t occupant; /* GetVoid1() as cISC4Occupant (0xAA7A014D, 0x99EF1143); 0 = none */
} fc4_neighbors_message;

typedef struct fc4_neighbors_state {
    uint32_t struct_size;
    uint8_t initialized, ready, deals_dirty, stats_dirty, ui_disabled, busy;
    uint8_t reserved0[2];
    uint32_t connections, foreign_connections, deals, reserved1;
    int64_t income, expense; /* §10.1 */
} fc4_neighbors_state;

typedef void (*fc4_neighbor_link_callback)(void* ctx, uint32_t conn_type, int32_t rx, int32_t rz);
typedef void (*fc4_neighbor_deal_data_callback)(void* ctx, const fc4_neighbor_deal_data* data);
/* One property in the game's native variant layout (as fc4_props_set_typed). */
typedef void (*fc4_neighbor_property_callback)(void* ctx, uint32_t id, uint16_t mem_type, bool is_array,
                                               const void* data, size_t count);

/* One callback per NeighborsHost method (engine/core/src/neighbors/mod.rs), in
 * the trait's order. */
typedef struct fc4_neighbors_host {
    uint32_t struct_size; /* sizeof(fc4_neighbors_host) */
    void* user;
    /* lifecycle */
    fc4_props* (*tuning)(void* user); /* a NEW fc4_props (the core destroys it), NULL = missing */
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t len, uint32_t flags);
    void (*remove_agents)(void* user);
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    /* city */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    uint32_t (*today)(void* user);
    void (*utility_figures)(void* user, fc4_neighbors_figures* out);
    void (*landfill_usage)(void* user, uint32_t* usage, uint32_t* capacity);
    void (*send_history)(void* user, uint32_t series, int32_t value);
    /* region */
    uint64_t (*current_city)(void* user);
    uint64_t (*current_city_object)(void* user);
    uint64_t (*city_at)(void* user, int32_t rx, int32_t rz);
    void (*city_position)(void* user, uint64_t city, int32_t* x, int32_t* z);
    void (*city_size)(void* user, uint64_t city, int32_t* x, int32_t* z);
    bool (*city_established)(void* user, uint64_t city);
    /* stat: 0 exported, 1 imported, 2 produced, 3 demanded, 4 cost per unit */
    double (*city_stat)(void* user, uint64_t city, uint32_t stat, int32_t commodity);
    void (*set_city_stat)(void* user, uint64_t city, uint32_t stat, int32_t commodity, double value);
    double (*city_budget)(void* user, uint64_t city);
    void (*set_city_budget)(void* user, uint64_t city, double value);
    void (*set_city_income)(void* user, uint64_t city, double value);
    /* city.FindConnection(type, x, z): the connection (borrowed) and its GetIsActive; 0 = none */
    uint64_t (*city_find_connection)(void* user, uint64_t city, uint32_t conn_type, int32_t x, int32_t z,
                                     bool* active);
    void (*city_connections)(void* user, uint64_t city, fc4_neighbor_link_callback cb, void* ctx);
    void (*change_symmetric_connection)(void* user, uint64_t city, uint64_t conn, bool add);
    void (*reload_city)(void* user, uint64_t city);
    void (*update_local_deals)(void* user, uint64_t city);
    void (*local_deals)(void* user, uint64_t city, fc4_neighbor_deal_data_callback cb, void* ctx);
    /* SetLocalDeals with the deal objects deals[0..count]; kept[i] := 0 for
     * every deal the region removed from the list. */
    void (*set_local_deals)(void* user, uint64_t city, const uint64_t* deals, size_t count, uint8_t* kept);
    void (*update_import_export)(void* user, uint64_t city);
    /* messages: a status report (§5.3), deal 0 for the retry report */
    void (*post_status)(void* user, uint64_t deal, uint32_t status);
    void (*send_status)(void* user, uint64_t deal, uint32_t status);
    void (*post_connection_message)(void* user, uint32_t msg_type, uint32_t conn_type, int32_t x, int32_t z);
    /* markers */
    void (*cell_center)(void* user, int32_t x, int32_t z, float* px, float* pz);
    float (*marker_altitude)(void* user, float px, float pz);
    float (*max_surface_level)(void* user, int32_t x, int32_t z);
    /* position: 3 floats; bool_props[0..count] are set true; returns the prop (0 = none) */
    uint64_t (*add_marker_prop)(void* user, const float* position, int32_t orientation, const uint32_t* bool_props,
                                size_t count);
    void (*marker_props)(void* user, int32_t x, int32_t z, fc4_token_callback cb, void* ctx);
    void (*remove_prop)(void* user, uint64_t prop);
    void (*move_prop)(void* user, uint64_t prop, const float* position);
    /* occupants */
    uint32_t (*occupant_type)(void* user, uint64_t occupant);
    void (*occupant_networks)(void* user, uint64_t occupant, fc4_token_callback cb, void* ctx); /* type numbers */
    void (*occupant_cell)(void* user, uint64_t occupant, int32_t* x, int32_t* z);
    bool (*power_pole_flag)(void* user, uint64_t occupant);
    /* objects (§15.2) */
    /* The GZCOM object for a new connection of ours: takes the NEW handle,
     * returns the object's token holding one reference for the list; 0 = none
     * (the core destroys the handle). */
    uint64_t (*connection_object)(void* user, fc4_neighbor_connection* handle);
    /* The GZCOM object for a deal of ours (first handed to the host): takes
     * the NEW handle, returns the token; no reference is held for the core. */
    uint64_t (*deal_object)(void* user, fc4_neighbor_deal* handle);
    bool (*foreign_connection)(void* user, uint64_t conn, fc4_neighbor_connection_state* out);
    /* -1 = leave, 0 = false, 1 = true */
    void (*set_foreign_connection_flags)(void* user, uint64_t conn, int32_t visited, int32_t active);
    void (*repair_foreign_connection)(void* user, uint64_t conn, uint64_t city);
    void (*release_connection)(void* user, uint64_t conn); /* the list's reference */
    /* Foreign deals (neighbors.md §15.4 NG4; ABI 45, appended): a caller's
     * deal object kept as the listed deal (fc4_neighbors_add_foreign_deal).
     * foreign_deal reads its state through its interface (false =
     * unreadable); foreign_deal_call runs FC4_NEIGHBOR_DEAL_CALL_* on it
     * (returns the call's bool, true for the void ones); release_deal
     * releases the reference the core was given. */
    bool (*foreign_deal)(void* user, uint64_t deal, fc4_neighbor_deal_data* out);
    bool (*foreign_deal_call)(void* user, uint64_t deal, uint32_t op, int64_t a, int64_t b);
    void (*release_deal)(void* user, uint64_t deal);
} fc4_neighbors_host;

/* foreign_deal_call ops: 0-11 = the setters numbered as in
 * fc4_neighbor_deal_set_field (a, b; dates are Julian days), then: */
#define FC4_NEIGHBOR_DEAL_CALL_SET_AMOUNT     12u /* SetAmount(a) */
#define FC4_NEIGHBOR_DEAL_CALL_UPDATE_COST    13u
#define FC4_NEIGHBOR_DEAL_CALL_UPDATE_STATE   14u
#define FC4_NEIGHBOR_DEAL_CALL_CANCEL         15u
#define FC4_NEIGHBOR_DEAL_CALL_ACTIVATE       16u /* returns Activate's bool */
#define FC4_NEIGHBOR_DEAL_CALL_SET_FLAGS      17u /* SetFlags(a, b != 0); returns its bool */

/* `token`: the simulator's host identity; `session`: the session seed
 * (foundations §6.3) of the proposal generator. NULL for a short table. */
fc4_neighbors* fc4_neighbors_create(const fc4_neighbors_host* host, uint64_t token, uint32_t session);
/* Also releases the connections a loaded, never initialised object holds. */
void fc4_neighbors_destroy(fc4_neighbors* p);
void fc4_neighbors_set_two_curves(fc4_neighbors* p, bool on); /* §8.3 [choice], off by default */
bool fc4_neighbors_init(fc4_neighbors* p);                    /* §14.1 */
bool fc4_neighbors_shutdown(fc4_neighbors* p);                /* §14.2 */
bool fc4_neighbors_do_message(fc4_neighbors* p, const fc4_neighbors_message* m); /* §11.1 */
bool fc4_neighbors_get_state(const fc4_neighbors* p, fc4_neighbors_state* out);
bool fc4_neighbors_write(const fc4_neighbors* p, const fc4_ostream* out); /* §13 */
bool fc4_neighbors_read(fc4_neighbors* p, const fc4_istream* in);         /* §13: false unless version 2 */
/* 5: 0 = exists, 1 = ask the player (`neighbour`: the established neighbour, 0 = the default name). */
int32_t fc4_neighbors_request_create_connection(fc4_neighbors* p, uint32_t t, int32_t x, int32_t z,
                                                uint64_t* neighbour);
uint64_t fc4_neighbors_create_connection(fc4_neighbors* p, uint32_t t, int32_t x, int32_t z, bool flip); /* 6 */
void fc4_neighbors_delete_connection(fc4_neighbors* p, uint32_t t, int32_t x, int32_t z);                /* 7 */
void fc4_neighbors_delete_occupant_connection(fc4_neighbors* p, uint64_t occupant);                     /* 8 */
void fc4_neighbors_delete_all_connections_of_type(fc4_neighbors* p, uint32_t t);                        /* 9 */
bool fc4_neighbors_has_connection(const fc4_neighbors* p, uint32_t t, int32_t x, int32_t z);            /* 10 */
uint64_t fc4_neighbors_find_connection(const fc4_neighbors* p, uint32_t t, int32_t x, int32_t z);       /* 11 */
uint64_t fc4_neighbors_find_city_on_other_side_of_connection(const fc4_neighbors* p, int32_t x, int32_t z); /* 12 */
/* 13 / 14: the other city's connection (borrowed) and its GetIsActive. */
uint64_t fc4_neighbors_find_connection_on_other_side(const fc4_neighbors* p, uint32_t t, int32_t x, int32_t z,
                                                     bool* active);
/* §2.2 with the current regional city: false without one. */
bool fc4_neighbors_neighbouring_region_cell(const fc4_neighbors* p, int32_t x, int32_t z, int32_t* rx, int32_t* rz);
uint64_t fc4_neighbors_find_unvisited_connection(const fc4_neighbors* p, uint32_t t, uint64_t city); /* 15 */
/* 16-19: which 0 = all, 1 = of type, 2 = of type to city, 3 = to city. Borrowed tokens; returns the count. */
size_t fc4_neighbors_connections(const fc4_neighbors* p, uint32_t which, uint32_t t, uint64_t city,
                                 fc4_token_callback cb, void* ctx);
/* 20-22: which 0 = adjacent, 1 = connected, 2 = all connected. Borrowed tokens. */
size_t fc4_neighbors_cities(const fc4_neighbors* p, uint32_t which, bool include_unestablished,
                            fc4_token_callback cb, void* ctx);
uint64_t fc4_neighbors_find_connection_suitable_for_deal(const fc4_neighbors* p, uint32_t deal_type,
                                                         uint64_t city); /* 23 */
uint64_t fc4_neighbors_propose_deal(fc4_neighbors* p, uint32_t deal_type, uint64_t city); /* 24: 0 = none */
size_t fc4_neighbors_propose_deals(fc4_neighbors* p, bool add, fc4_token_callback cb, void* ctx); /* 25 */
bool fc4_neighbors_add_deal(fc4_neighbors* p, const fc4_neighbor_deal* deal);                      /* 26, ours */
/* 26 for a foreign deal, the old way: a deal of ours with its state is added; its token.
 * Superseded by fc4_neighbors_add_foreign_deal (NG4). */
uint64_t fc4_neighbors_add_deal_data(fc4_neighbors* p, const fc4_neighbor_deal_data* data);
/* 26 for a foreign deal (NG4, ABI 45): the object itself is listed. `deal` holds one
 * reference for the core, released through release_deal; on false it is still the caller's. */
bool fc4_neighbors_add_foreign_deal(fc4_neighbors* p, uint64_t deal);
void fc4_neighbors_set_ui_disable_flag(fc4_neighbors* p, bool on); /* 27 */
/* 28-30: which 0 = all, 1 = of type (slot 30), 2 = of type with city (slot 29). */
size_t fc4_neighbors_find_deals(fc4_neighbors* p, uint32_t which, uint32_t deal_type, uint64_t city,
                                fc4_token_callback cb, void* ctx);
/* The deals' states in list order: returns the count, fills out[0..cap]. */
size_t fc4_neighbors_deal_data(const fc4_neighbors* p, fc4_neighbor_deal_data* out, size_t cap);
bool fc4_neighbors_holds_deal(const fc4_neighbors* p, const fc4_neighbor_deal* deal);
void fc4_neighbors_claim_deal(fc4_neighbors* p, const fc4_neighbor_deal* deal); /* "the current simulator" */
void fc4_neighbors_mark_deals_dirty(fc4_neighbors* p);                          /* 31 */
int64_t fc4_neighbors_total_income(const fc4_neighbors* p);                     /* 32 */
int64_t fc4_neighbors_total_expense(const fc4_neighbors* p);                    /* 33 */
void fc4_neighbors_update_commodity_stats(fc4_neighbors* p, uint64_t city);     /* 34: the city object */
double fc4_neighbors_deal_cost(const fc4_neighbors* p, uint32_t deal_type, double amount, uint64_t city); /* 35 */
double fc4_neighbors_deal_max_amount(const fc4_neighbors* p, uint32_t deal_type, uint64_t city);          /* 36 */

/* Deals (cISC4NeighborDeal). */
fc4_neighbor_deal* fc4_neighbor_deal_create(void); /* Proposed + Silent, no simulator; the caller owns it */
void fc4_neighbor_deal_destroy(fc4_neighbor_deal* h);
void fc4_neighbor_deal_set_token(fc4_neighbor_deal* h, uint64_t token); /* the object that owns the handle */
bool fc4_neighbor_deal_get(const fc4_neighbor_deal* h, fc4_neighbor_deal_data* out);
/* field 0 type, 1 other side (a, b), 2 city side (a, b), 3 lastAmount, 4
 * lastIncAmount, 5 incAmount, 6 incAmount2, 7 amountUsed, 8-11 start,
 * expiration, current, suspension date (a). */
void fc4_neighbor_deal_set_field(fc4_neighbor_deal* h, uint32_t field, int32_t a, int32_t b);
void fc4_neighbor_deal_set_amount(fc4_neighbor_deal* h, int32_t a);
void fc4_neighbor_deal_update_cost(fc4_neighbor_deal* h);
void fc4_neighbor_deal_copy(fc4_neighbor_deal* h, const fc4_neighbor_deal_data* src);
void fc4_neighbor_deal_update_state(fc4_neighbor_deal* h);
void fc4_neighbor_deal_cancel(fc4_neighbor_deal* h);
bool fc4_neighbor_deal_activate(fc4_neighbor_deal* h);
bool fc4_neighbor_deal_set_flags(fc4_neighbor_deal* h, uint32_t f, bool on);
fc4_neighbor_deal* fc4_neighbor_deal_clone(const fc4_neighbor_deal* h, bool silent); /* a NEW handle */
bool fc4_neighbor_deal_write(const fc4_neighbor_deal* h, const fc4_ostream* out); /* §5.6 */
bool fc4_neighbor_deal_read(fc4_neighbor_deal* h, const fc4_istream* in);

/* Connections (cISC4NeighborConnection). */
fc4_neighbor_connection* fc4_neighbor_connection_create(void);
void fc4_neighbor_connection_destroy(fc4_neighbor_connection* h);
bool fc4_neighbor_connection_get(const fc4_neighbor_connection* h, fc4_neighbor_connection_state* out);
/* field 0 type (a), 1 cell (a, b), 2 other region cell (a, b), 3 visited (a != 0), 4 active (a != 0). */
void fc4_neighbor_connection_set_field(fc4_neighbor_connection* h, uint32_t field, int32_t a, int32_t b);
bool fc4_neighbor_connection_inner_cell(const fc4_neighbor_connection* h, int32_t count_x, int32_t count_z,
                                        int32_t* x, int32_t* z);
void fc4_neighbor_connection_repair(fc4_neighbor_connection* h, int32_t count_x, int32_t count_z, int32_t pos_x,
                                    int32_t pos_z, int32_t size_x, int32_t size_z);
void fc4_neighbor_connection_set_props(fc4_neighbor_connection* h, const fc4_props* props); /* a copy */
size_t fc4_neighbor_connection_props(const fc4_neighbor_connection* h, fc4_neighbor_property_callback cb,
                                     void* ctx);
bool fc4_neighbor_connection_write(const fc4_neighbor_connection* h, const fc4_ostream* out); /* §4.3 */
bool fc4_neighbor_connection_read(fc4_neighbor_connection* h, const fc4_istream* in);         /* version 1 only */
uint32_t fc4_neighbor_connection_class_id(void);                                               /* 0x0A134F24 */

/* ---- neighbours shadow (tooling; engine/capi/src/neighbors_shadow.rs) --------
 *
 * Our core's month on a snapshot of the ORIGINAL simulator's inputs, read
 * through the game's interfaces only: an ordinary fc4_neighbors* whose host
 * is the snapshot (every fc4_neighbors_* query works on it, and
 * fc4_neighbors_do_message runs the month). Writes stay in the snapshot;
 * status reports are counted (a SENT one is delivered to the shadow at
 * once); UpdateImportExport is reproduced as a delta on the stored values. */
typedef struct fc4_neighbors_shadow_city {
    uint64_t token;
    int32_t pos_x, pos_z, size_x, size_z;
    uint8_t established;
    uint8_t reserved[7];
    double budget, income;
    double stats[5][3]; /* [stat][commodity - 1]: exported, imported, produced, demanded, cost per unit */
} fc4_neighbors_shadow_city;

typedef struct fc4_neighbors_shadow_conn {
    uint64_t city; /* the regional city holding it (the current city's: the original's list, in order) */
    uint64_t token;
    fc4_neighbor_connection_state state;
} fc4_neighbors_shadow_conn;

typedef struct fc4_neighbors_shadow_input {
    uint32_t struct_size;
    uint32_t today;
    int32_t count_x, count_z;
    uint64_t current_city, city_object;
    uint32_t landfill_usage, landfill_capacity;
    /* The original's state bytes (neighbors.md §15.4 NG7: ready +0x0C, uiDisabled +0x3E,
     * dealsDirty +0x3C, statsDirty +0x3D, busy +0x3F), as the shim read them. */
    uint8_t ready, ui_disabled, two_curves;
    uint8_t deals_dirty, stats_dirty, busy; /* ABI 45; 0 before (the earlier fixed values) */
    uint8_t reserved[2];
    fc4_neighbors_figures figures;
    const fc4_props* tuning; /* borrowed (copied); NULL = missing */
    const fc4_neighbors_shadow_city* cities;
    size_t city_count;
    const fc4_neighbors_shadow_conn* conns;
    size_t conn_count;
    const fc4_neighbor_deal_data* deals; /* the original's FindDealsAll, in order */
    size_t deal_count;
} fc4_neighbors_shadow_input;

typedef struct fc4_neighbors_shadow_counters {
    uint32_t posts, sends, retries, connection_messages, set_local_deals, update_import_export, history, reserved;
} fc4_neighbors_shadow_counters;

/* Everything is copied; destroy with fc4_neighbors_destroy. */
fc4_neighbors* fc4_neighbors_shadow_create(const fc4_neighbors_shadow_input* input);
/* A city of the snapshot as the shadow left it (false: not a shadow / unknown city). */
bool fc4_neighbors_shadow_city_get(const fc4_neighbors* p, uint64_t city, fc4_neighbors_shadow_city* out);
bool fc4_neighbors_shadow_get_counters(const fc4_neighbors* p, fc4_neighbors_shadow_counters* out);

/* ---- traffic (docs/specs/traffic-simulator.md, traffic-network.md, pathfinder.md) ----
 *
 * The traffic simulator (cSC4TrafficSimulator, class 0x6990C1AA, simulator
 * type 0x88027F0D), its network model and the pathfinders it hands out
 * through slot 6, as one replacement unit (traffic-simulator.md §9,
 * pathfinder.md §9). The plugin's cISC4TrafficSimulator forwards its 44 slots
 * to fc4_traffic_*, the cISC4PathFinder objects of slot 6 forward theirs to
 * fc4_traffic_pf_*.
 *
 * Grids are the game's own (fc4_grid_api): u8 0x49B9E602, u32 0x49B9E606,
 * f32 0x49B9E60A, saved by reference. Tokens are host identities (lots,
 * occupants, buildings, connections, holders, departments, grids); those the
 * host returns are BORROWED unless stated. 0 = none. Holder tokens in
 * 0x000FC4F000000000 .. 0x000FC4F00000FFFF are the simulator's own
 * neighbour pools: the host must not hand those out.
 *
 * Threads: single-threaded; the plugin serialises every call on one object
 * and answers other threads from its snapshot (fc4_traffic_get_state).
 * Re-entrancy: a call made from inside a host callback (a lot's own
 * UpdateTrafficVolume asking slots 26 and 27) is answered from a snapshot
 * taken at the start of the outermost call; a message delivered re-entrantly
 * is queued and run when the outermost call ends. Both are counted. */

typedef struct fc4_traffic fc4_traffic;
typedef struct fc4_traffic_pf fc4_traffic_pf;

/* The kind of a traffic source. */
#define FC4_TRAFFIC_SOURCE_LOT 0u
#define FC4_TRAFFIC_SOURCE_CONNECTION 1u

/* The maps of slots 8, 9, 10, 13. */
#define FC4_TRAFFIC_MAP_AIR 0u
#define FC4_TRAFFIC_MAP_COMMERCIAL 1u
#define FC4_TRAFFIC_MAP_CONGESTION 2u
#define FC4_TRAFFIC_MAP_TRIP_LENGTH 3u

/* The size of the slot-34 statistics block (fc4_traffic_stats_block). */
#define FC4_TRAFFIC_STATS_BLOCK 0x23Cu

/* One cSC4TripData (traffic-simulator.md §2). The routes are byte strings in
 * the compressed route format (pathfinder.md §4.15), valid for the call. */
typedef struct fc4_traffic_trip {
    uint32_t count;
    uint8_t trip_type, strategy, flags, status;
    int16_t dest_x, dest_z;
    float trip_length;
    uint32_t travel_types_used;
    const uint8_t* route;
    size_t route_len;
    const uint8_t* return_route;
    size_t return_len;
} fc4_traffic_trip;

typedef void (*fc4_traffic_trip_callback)(void* ctx, const fc4_traffic_trip* trip);

/* An established neighbouring city (traffic-network.md §2.7.4). */
typedef struct fc4_traffic_city {
    int32_t x, z;
    int32_t population[3];
} fc4_traffic_city;

typedef void (*fc4_traffic_city_callback)(void* ctx, const fc4_traffic_city* city);
/* One (developer type, count) entry of map `map` of a lot's population maps. */
typedef void (*fc4_traffic_population_callback)(void* ctx, uint32_t map, uint32_t developer_type, uint32_t count);
/* One path of a path info: its exit byte and its first vertex. */
typedef void (*fc4_traffic_exit_callback)(void* ctx, uint8_t exit, uint64_t vertex);
typedef void (*fc4_traffic_u32_callback)(void* ctx, uint32_t value);
typedef void (*fc4_traffic_bytes_callback)(void* ctx, const uint8_t* bytes, size_t len);

/* A message the simulator posts (§5, MessagePost). */
typedef struct fc4_traffic_post {
    uint32_t msg_type, data1, data2, data3;
} fc4_traffic_post;

/* GetTransitSwitchQueryData's out (slot 35, §4.16). */
typedef struct fc4_traffic_switch_query {
    uint32_t available, max_capacity;
    float load_fraction;
    uint32_t usage, cost;
} fc4_traffic_switch_query;

/* One FerryRouteInfo of slot 31 without its two routes (they come with the
 * callback). */
typedef struct fc4_traffic_ferry_route {
    uint64_t lot_a, lot_b;
    uint32_t ferry_type;
    uint16_t counter1[2], counter2[2];
    uint8_t neighbour, reserved;
    int16_t neighbour_x, neighbour_z;
    uint16_t reserved2;
} fc4_traffic_ferry_route;

typedef void (*fc4_traffic_ferry_callback)(void* ctx, const fc4_traffic_ferry_route* info, const uint8_t* route_ab,
                                           size_t len_ab, const uint8_t* route_ba, size_t len_ba);

/* The state (diagnostics, the log lines, the plugin's snapshot). */
typedef struct fc4_traffic_state {
    uint32_t struct_size;
    uint8_t initialized, loaded;
    uint8_t cycle_state;  /* the work-cycle state (§3.2) */
    uint8_t strike_state; /* 2 = no strike, 0 started, 1 on strike (§4.12) */
    uint32_t day_index, days;
    int32_t total, processed, pass_total, to_reroute, to_search, last_day;
    uint32_t pass_counters[10];
    uint32_t units_per_slice;
    float trip_scale, trip_scale_for_display, last_monthly_income, freight_scaling; /* slots 14, 15, 18, 27 */
    float transit_income, toll_income, avg_car_commute;
    float pct[4][3]; /* slot 26 for wealth 0..3 */
    uint32_t attempted[3], routed[3], job_capacity[3], free_dest[4];
    uint32_t type_counts[9]; /* per travel type, the last cycle (§4.6) */
    uint8_t freight_byte, reserved[3];
    uint32_t accident_cells, accidents_posted, posted, native_updates, trip_reads, trip_writes;
    uint32_t reentrant_calls, queued_messages;
    uint32_t potholes, road_damaged, rail_damaged, edge_records, switches, ferry_terminals, reserved2;
    uint64_t air, commercial, congestion, trip_length; /* slots 8, 9, 10, 13 (borrowed) */
    uint64_t roads, mass_transit;                      /* the two departments */
} fc4_traffic_state;

/* Everything the traffic unit asks of the game. */
typedef struct fc4_traffic_host {
    uint32_t struct_size; /* sizeof(fc4_traffic_host) */
    void* user;
    fc4_grid_api grids;
    /* framework (§1, §5) */
    void (*add_agent)(void* user, uint32_t level, const uint8_t* name, size_t len, uint32_t flags);
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*post_message)(void* user, const fc4_traffic_post* msg); /* queued */
    void (*add_ref)(void* user, uint64_t token);                   /* a switch building the model keeps */
    void (*release)(void* user, uint64_t token);
    /* city, tuning, dates */
    void (*cell_counts)(void* user, int32_t* x, int32_t* z);
    void (*position_to_cell)(void* user, float x, float z, int32_t* cx, int32_t* cz);
    float (*cell_width)(void* user);
    fc4_props* (*tuning)(void* user); /* NEW fc4_props the core destroys, or NULL */
    uint32_t (*sim_day_of_month)(void* user);
    uint32_t (*sim_date_number)(void* user);
    uint64_t (*sim_clock_ms)(void* user); /* game time in ms (§4.13 [choice]) */
    /* departments (budget.md §7-§8) */
    uint64_t (*create_department)(void* user, uint32_t id, float max_funding); /* ONE reference, 0 = none */
    void (*release_department)(void* user, uint64_t department);
    float (*department_funding)(void* user, uint64_t department);
    void (*set_line_item_income)(void* user, uint64_t department, uint32_t item, int64_t income);
    void (*update_network_costs)(void* user, uint64_t department, bool mass_transit, const uint32_t* cells13,
                                 const float* tile_costs13);
    int64_t (*activate_budget_items)(void* user, uint64_t building, uint64_t lot);
    int64_t (*deactivate_budget_items)(void* user, uint64_t holder);
    bool (*budget_item_for_purpose)(void* user, uint64_t building, uint32_t purpose, uint32_t* department,
                                    int64_t* cost);
    uint64_t (*department_budget)(void* user, uint32_t department_id);
    void (*record_history)(void* user, uint32_t id, uint32_t value);
    /* lots */
    void (*lots)(void* user, fc4_token_callback cb, void* ctx);
    uint64_t (*as_lot)(void* user, uint64_t object);
    uint32_t (*lot_zone)(void* user, uint64_t lot);
    bool (*lot_is_livable)(void* user, uint64_t lot);
    bool (*lot_has_road_access)(void* user, uint64_t lot);
    uint64_t (*lot_link)(void* user, uint64_t lot);
    uint32_t (*lot_travel_population)(void* user, uint64_t lot, uint32_t developer_type);
    uint8_t (*lot_strategy_byte)(void* user, uint64_t lot);
    void (*clear_lot_flag)(void* user, uint64_t lot, uint32_t flag);
    uint64_t (*lot_at)(void* user, int32_t x, int32_t z, bool anchor_only);
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    void (*lot_location)(void* user, uint64_t lot, int32_t* x, int32_t* z);
    uint32_t (*lot_facing)(void* user, uint64_t lot);
    uint64_t (*lot_building)(void* user, uint64_t lot);
    void (*lot_populations)(void* user, uint64_t lot, fc4_traffic_population_callback cb, void* ctx);
    uint64_t (*lot_holder)(void* user, uint64_t lot);
    /* traffic sources (cISC4TrafficSource) */
    bool (*update_traffic_volume)(void* user, uint32_t kind, uint64_t source); /* true: the source ran it */
    bool (*trip_list)(void* user, uint32_t kind, uint64_t source, fc4_traffic_trip_callback cb, void* ctx);
    void (*set_trip_list)(void* user, uint32_t kind, uint64_t source, const fc4_traffic_trip* trips, size_t count);
    /* property holders */
    bool (*holder_property)(void* user, uint64_t holder, uint32_t id, fc4_props* out);
    /* AddProperty: mem type, array flag, `count` elements in the native layout;
     * in place for an existing property of the same type and length. */
    void (*set_holder_property)(void* user, uint64_t holder, uint32_t id, uint16_t mem_type, bool is_array,
                                const void* data, size_t count);
    bool (*remove_holder_property)(void* user, uint64_t holder, uint32_t id);
    /* neighbour connections */
    uint64_t (*connection_holder)(void* user, uint64_t connection);
    bool (*take_connection_volume_dirty)(void* user, uint64_t connection);
    uint64_t (*find_connection)(void* user, uint32_t connection_type, int32_t x, int32_t z);
    uint32_t (*connection_type)(void* user, uint64_t connection);
    uint64_t (*connection_on_other_side)(void* user, uint64_t connection);
    bool (*established_city_position)(void* user, uint64_t connection, int32_t* x, int32_t* z);
    void (*connected_cities)(void* user, fc4_traffic_city_callback cb, void* ctx);
    bool (*regional_demand)(void* user, uint32_t w, float* out);
    void (*connections_of_type)(void* user, uint32_t connection_type, fc4_token_callback cb, void* ctx);
    void (*all_connections)(void* user, fc4_token_callback cb, void* ctx);
    void (*connection_cell)(void* user, uint64_t connection, int32_t* x, int32_t* z);
    /* other simulators */
    float (*workforce_percent)(void* user, int32_t x, int32_t z);
    int16_t (*developer_population)(void* user, uint32_t developer_type, int32_t x, int32_t z);
    bool (*is_ordinance_on)(void* user, uint32_t id);
    void (*ordinances_on)(void* user, fc4_traffic_u32_callback cb, void* ctx);
    bool (*ordinance_property)(void* user, uint32_t id, uint32_t prop, fc4_props* out);
    /* occupants */
    uint64_t (*as_network_occupant)(void* user, uint64_t object);
    void (*network_occupant_cell)(void* user, uint64_t occupant, int32_t* x, int32_t* z);
    void (*occupants_in_cells)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    bool (*occupant_filter_includes)(void* user, uint64_t filter, uint64_t occupant);
    bool (*cell_has_network_occupant)(void* user, int32_t x, int32_t z);
    void (*network_occupants_in_rect)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    void (*subway_occupants_in_rect)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    void (*prop_occupants_in_rect)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    void (*occupant_position)(void* user, uint64_t occupant, float* xyz);
    void (*occupant_bounding_cells)(void* user, uint64_t occupant, fc4_cell_rect* out);
    uint32_t (*prop_type)(void* user, uint64_t occupant);
    bool (*occupant_property)(void* user, uint64_t occupant, uint32_t id, fc4_props* out);
    bool (*has_network_flag)(void* user, uint64_t occupant, uint32_t flag);
    bool (*has_any_network_flag)(void* user, uint64_t occupant, uint32_t mask);
    bool (*is_of_type)(void* user, uint64_t occupant, uint32_t network_type);
    bool (*path_connection)(void* user, uint64_t occupant, uint32_t path_type, uint32_t entry, uint32_t exit);
    uint64_t (*path_info)(void* user, uint64_t occupant);
    bool (*path_info_turn_around)(void* user, uint64_t path_info, uint32_t path_type, uint32_t entry);
    void (*path_info_exits_from_entry)(void* user, uint64_t path_info, uint32_t path_type, uint32_t entry,
                                       fc4_traffic_exit_callback cb, void* ctx);
    void (*path_info_entries_near)(void* user, uint64_t path_info, uint32_t path_type, uint64_t vertex,
                                   uint32_t exit, float radius, fc4_traffic_u32_callback cb, void* ctx);
    bool (*is_tunnel)(void* user, uint64_t occupant);
    uint64_t (*tunnel_paired_portal)(void* user, uint64_t occupant);
    /* terrain and water */
    bool (*cell_under_water)(void* user, int32_t x, int32_t z);
    bool (*is_navigable)(void* user, int32_t x, int32_t z);
    float (*water_depth)(void* user, int32_t x, int32_t z);
    bool (*body_of_water)(void* user, int32_t x, int32_t z, uint16_t* out);
    /* language, view, accidents */
    bool (*drives_on_right)(void* user);
    bool (*view_cell_rect)(void* user, fc4_cell_rect* out);
    void (*create_accident)(void* user, int32_t x, int32_t z, int64_t duration_ms);
} fc4_traffic_host;

/* QueryInterface(iid) (§1): 1, 0x68027F37, 0x452294AA, 0xE4FDA3D4. */
bool fc4_traffic_query_interface(uint32_t iid);
uint32_t fc4_traffic_class_id(void);       /* 0x6990C1AA */
uint32_t fc4_traffic_simulator_type(void); /* slot 5: 0x88027F0D */
uint32_t fc4_traffic_default_units_per_slice(void); /* §3.4 [choice] */
/* NULL when the table is incomplete. The table is copied. */
fc4_traffic* fc4_traffic_create(const fc4_traffic_host* host, uint32_t session_seed);
/* Releases what is still held (grids, departments, switch buildings). */
void fc4_traffic_destroy(fc4_traffic* p);
/* Init (slot 3); units_per_slice 0 keeps the default. */
bool fc4_traffic_init(fc4_traffic* p, uint32_t units_per_slice);
bool fc4_traffic_shutdown(fc4_traffic* p); /* slot 4 */
/* DoMessage (§3.1); `object` = void1 / the IGZUnknown. Re-entrantly: queued, true. */
bool fc4_traffic_do_message(fc4_traffic* p, uint32_t msg_type, uint32_t data1, uint32_t data2, uint32_t data3,
                            uint64_t object);
bool fc4_traffic_write(const fc4_traffic* p, const fc4_ostream* out);
/* Runs before Init: nothing is sized from the city. A failed Read changes nothing. */
bool fc4_traffic_read(fc4_traffic* p, const fc4_istream* in);
bool fc4_traffic_get_state(const fc4_traffic* p, fc4_traffic_state* out);
uint64_t fc4_traffic_map(const fc4_traffic* p, uint32_t which); /* slots 8, 9, 10, 13: borrowed */
float fc4_traffic_map_average(const fc4_traffic* p, uint32_t which);
float fc4_traffic_background_traffic(const fc4_traffic* p, int32_t x, int32_t z); /* slot 11 */
/* Slot 12: the 33 u16 (last[16], usage, current[16]); false (zeros) for the shared zero record. */
bool fc4_traffic_edge_density(const fc4_traffic* p, uint8_t x, uint8_t z, uint32_t travel_type, bool evening,
                              uint16_t* out33);
/* Slots 14, 15, 18, 27 for which = 0..3. */
float fc4_traffic_scalar(const fc4_traffic* p, uint32_t which);
bool fc4_traffic_is_road_damaged(const fc4_traffic* p, int32_t x, int32_t z);   /* slot 16 */
bool fc4_traffic_check_rail_accident(fc4_traffic* p, int32_t x, int32_t z);     /* slot 17 */
void fc4_traffic_set_max_trip_capacity(fc4_traffic* p, uint64_t holder, uint32_t k, uint32_t value); /* 19 */
uint32_t fc4_traffic_max_trip_capacity(const fc4_traffic* p, uint64_t holder, uint32_t k);           /* 20 */
uint32_t fc4_traffic_traffic_arrived(const fc4_traffic* p, uint64_t holder, uint32_t k);             /* 21 */
bool fc4_traffic_desired_lot_insertion_point(const fc4_traffic* p);                                  /* 22 */
float fc4_traffic_capacity(const fc4_traffic* p, uint32_t n, int32_t x, int32_t z);                  /* 23 */
float fc4_traffic_travel_time_ratio(const fc4_traffic* p, int32_t x, int32_t z, uint32_t t);         /* 24 */
uint32_t fc4_traffic_connection_count(const fc4_traffic* p, uint32_t n, int32_t x, int32_t z);       /* 25 */
void fc4_traffic_travel_strategy_percentages(const fc4_traffic* p, uint32_t wealth, float* out3);    /* 26 */
/* Slot 28: the switch buildings in table order (borrowed); returns the count. */
size_t fc4_traffic_transit_switches(const fc4_traffic* p, fc4_token_callback cb, void* ctx);
bool fc4_traffic_ferry_route_between_tiles(const fc4_traffic* p, int32_t x1, int32_t z1, int32_t x2, int32_t z2,
                                           fc4_traffic_bytes_callback cb, void* ctx); /* 29 */
size_t fc4_traffic_all_ferry_routes(const fc4_traffic* p, fc4_traffic_bytes_callback cb, void* ctx); /* 30 */
size_t fc4_traffic_ferry_routes_in_use(const fc4_traffic* p, fc4_traffic_ferry_callback cb, void* ctx); /* 31 */
uint32_t fc4_traffic_ferry_terminal_count(const fc4_traffic* p, uint32_t ferry_type); /* 32 */
/* Slot 33: `cb` always called (empty when not found). */
bool fc4_traffic_water_route(fc4_traffic* p, int32_t x1, int32_t z1, int32_t x2, int32_t z2,
                             fc4_traffic_bytes_callback cb, void* ctx);
/* Slot 34: the block (FC4_TRAFFIC_STATS_BLOCK bytes) into out; returns its size, 0 re-entrantly. */
size_t fc4_traffic_stats_block(const fc4_traffic* p, uint8_t* out, size_t len);
bool fc4_traffic_transit_switch_query_data(fc4_traffic* p, int32_t x, int32_t z,
                                           fc4_traffic_switch_query* out); /* 35 */
bool fc4_traffic_are_lots_connected(const fc4_traffic* p, uint64_t a, uint64_t b);              /* 36 */
uint32_t fc4_traffic_connected_occupant_count(const fc4_traffic* p, uint64_t lot, uint32_t dev); /* 37 */
uint32_t fc4_traffic_connected_destination_count(const fc4_traffic* p, uint64_t lot, int32_t k); /* 38 */
size_t fc4_traffic_subnetworks_for_lot(const fc4_traffic* p, uint64_t lot, fc4_traffic_u32_callback cb, void* ctx,
                                       bool* ok); /* 39 */
size_t fc4_traffic_subnetworks_in_rectangle(const fc4_traffic* p, const fc4_cell_rect* rect,
                                            fc4_traffic_u32_callback cb, void* ctx); /* 40 */
size_t fc4_traffic_subnetworks_in_region(const fc4_traffic* p, const fc4_zone_region* region,
                                         fc4_traffic_u32_callback cb, void* ctx); /* 41 */
size_t fc4_traffic_occupant_count_for_all_subnetworks(const fc4_traffic* p, uint32_t dev,
                                                      fc4_traffic_u32_callback cb, void* ctx); /* 42 */
/* Slot 7: false for lot 0 or a lot without trips. */
bool fc4_traffic_setup_path_finder_for_lot(fc4_traffic* p, fc4_traffic_pf* pf, uint64_t lot);

/* ---- the pathfinders of slot 6 (pathfinder.md). Each belongs to the
 * simulator that created it; calls that search name both. */

/* The configuration a pathfinder holds (diagnostics, tests). */
typedef struct fc4_traffic_pf_state {
    uint32_t struct_size;
    uint8_t initialised, suspended, has_route, destination_flags;
    uint32_t flags, filter;
    uint16_t allowed, starting;
    uint32_t density, max_work_time;
    float trip_length;
    fc4_cell_rect start, end;
    int32_t city_x, city_z;
} fc4_traffic_pf_state;

/* CreatePathFinder (slot 6): a new, initialised pathfinder. */
fc4_traffic_pf* fc4_traffic_pf_create(fc4_traffic* p);
void fc4_traffic_pf_destroy(fc4_traffic_pf* pf);
bool fc4_traffic_pf_init(fc4_traffic* p, fc4_traffic_pf* pf); /* slot 3: false when initialised */
bool fc4_traffic_pf_shutdown(fc4_traffic_pf* pf);             /* slot 4 */
bool fc4_traffic_pf_is_initialised(const fc4_traffic_pf* pf);
/* Slots 5-7, 9-15, 18, 21, 22 by number (a, b: the arguments); false for others. */
bool fc4_traffic_pf_set(fc4_traffic_pf* pf, uint32_t slot, uint32_t a, uint32_t b);
/* Slot 8: on = keep a route of the pathfinder's own (fc4_traffic_pf_route). */
void fc4_traffic_pf_set_route_info(fc4_traffic_pf* pf, bool on);
void fc4_traffic_pf_set_starting_cost_array(fc4_traffic_pf* pf, const float* costs9); /* 16; NULL = default */
void fc4_traffic_pf_set_max_trip_length(fc4_traffic_pf* pf, float v);                 /* 17 */
void fc4_traffic_pf_set_rect(fc4_traffic_pf* pf, bool end, const fc4_cell_rect* rect); /* 19, 20 */
uint32_t fc4_traffic_pf_find_path(fc4_traffic* p, fc4_traffic_pf* pf); /* 23: 0, 1, 2 */
uint32_t fc4_traffic_pf_traffic_arrived(const fc4_traffic_pf* pf);     /* 24 */
float fc4_traffic_pf_trip_length(const fc4_traffic_pf* pf);            /* 25 */
float fc4_traffic_pf_trip_length_for_display(const fc4_traffic* p, const fc4_traffic_pf* pf); /* 26 */
void fc4_traffic_pf_end_cell(const fc4_traffic_pf* pf, int32_t* x, int32_t* z);              /* 27 */
uint8_t fc4_traffic_pf_last_travel_type(const fc4_traffic_pf* pf);                          /* 28 */
void fc4_traffic_pf_apply_traffic(fc4_traffic* p, fc4_traffic_pf* pf, uint32_t style);       /* 29 */
bool fc4_traffic_pf_was_trip_reversible(const fc4_traffic* p, const fc4_traffic_pf* pf);     /* 30 */
void fc4_traffic_pf_release_path_nodes(fc4_traffic_pf* pf);                                  /* 31 */
/* Slot 32: the route bytes into cb (always called); returns the mask. */
uint32_t fc4_traffic_pf_compressed_route(const fc4_traffic* p, const fc4_traffic_pf* pf,
                                         fc4_traffic_bytes_callback cb, void* ctx);
/* The route of the last FindPath (root -> result), one {x, z, travelType}
 * (3 bytes) per call; returns the count. */
size_t fc4_traffic_pf_route(const fc4_traffic_pf* pf, fc4_traffic_bytes_callback cb, void* ctx);
bool fc4_traffic_pf_get_state(const fc4_traffic_pf* pf, fc4_traffic_pf_state* out);

/* ---- history warehouse (docs/specs/history.md; engine/capi/src/history.rs) ----
 * cSC4HistoryWarehouse's core: the series, the yearly thinning pass, the
 * queries and the record. The shim owns the GZCOM object (QueryInterface,
 * reference count, the +4 message target, the agents) and turns cIGZDate
 * arguments into day numbers (GetDayNumber). Dates are Julian day numbers. */
typedef struct fc4_history fc4_history;

typedef struct fc4_history_host {
    uint32_t struct_size; /* sizeof(fc4_history_host) */
    void* user;
    /* cISC4Simulator slot 8: the simulation date number. */
    uint32_t (*sim_date)(void* user);
    /* A millisecond timer for the yearly pass (§4.3); only differences are used. */
    uint32_t (*now_ms)(void* user);
    /* Subscribes / unsubscribes the +4 message target to one message type. */
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    /* AddAgent(+4, level, name, flags) / RemoveAgent(+4) (both levels) on the simulator. */
    void (*add_agent)(void* user, uint32_t level, const char* name, size_t name_len, uint32_t flags);
    void (*remove_agent)(void* user);
    /* MessageSend (synchronous) of a standard message: type, data1, data2. The
     * shim may deliver it back to fc4_history_do_message from inside the call. */
    void (*send)(void* user, uint32_t msg_type, uint32_t data1, uint32_t data2);
} fc4_history_host;

typedef struct fc4_history_point {
    int32_t value;
    uint32_t date;
} fc4_history_point;

typedef struct fc4_history_state {
    uint32_t struct_size;
    uint8_t initialized, has_cursor, pad[2];
    uint32_t pass_state, cursor, pass_counter, series_count, total_points;
} fc4_history_state;

/* The host table is copied. NULL if the table is missing or too small. */
fc4_history* fc4_history_create(const fc4_history_host* host);
void fc4_history_destroy(fc4_history* h);
/* QueryInterface (§1): true for 1, 0x69F4A803 and 0xE4FDA3D4 only (not
 * cIGZMessageTarget2, [quirk]). */
bool fc4_history_answers_iid(uint32_t iid);
/* Init (slot 3) and Shutdown (slot 4), §3. */
bool fc4_history_init(fc4_history* h);
bool fc4_history_shutdown(fc4_history* h);
/* DoMessage (§3): 0x89EFA536 record (data1 = key, data2 = value),
 * 0x89EFA537 queue length, SimNewDay (the pass step; false = call again this
 * interval), SimNewYear. True otherwise (and for a re-entrant call). */
bool fc4_history_do_message(fc4_history* h, uint32_t msg_type, uint32_t data1, uint32_t data2);
/* RecordHistoryData (slot 12, §4.1): sends the record message through the
 * host's send without touching the warehouse. Returns false (HW12). */
bool fc4_history_record(const fc4_history* h, uint32_t key, int32_t value);
/* SetHistoryDataQueueLength (slot 13, §4.2). */
bool fc4_history_set_queue_length(fc4_history* h, uint32_t key, uint32_t n);
/* Coalesce (slot 9, §4.4; granularity 0 month, 1 half-year, 2 year, 3 five
 * years), Dispose (slot 10) and Dispose(key, from, to) (slot 11), §4.5. */
bool fc4_history_coalesce(fc4_history* h, uint32_t key, uint32_t from, uint32_t to, int32_t granularity);
bool fc4_history_dispose(fc4_history* h, uint32_t key);
bool fc4_history_dispose_range(fc4_history* h, uint32_t key, uint32_t from, uint32_t to);
/* Queries (slots 5-8, §4.6-§4.8). precision_bits = the caller's x87
 * precision: 24 (the simulators) or 53 (Lua; any value other than 24).
 * Outputs (may be NULL) are untouched when the result is false. */
bool fc4_history_get_count(const fc4_history* h, uint32_t key, uint32_t from, uint32_t to, float* out);
bool fc4_history_get_trend(const fc4_history* h, uint32_t key, uint32_t from, uint32_t to, uint32_t precision_bits,
                           float* slope, float* intercept);
bool fc4_history_get_point(const fc4_history* h, uint32_t key, uint32_t date, uint32_t precision_bits, float* out);
bool fc4_history_get_min_max_average(const fc4_history* h, uint32_t key, uint32_t from, uint32_t to,
                                     uint32_t precision_bits, float* min, float* max, float* avg);
/* Diagnostics. fc4_history_keys copies at most `capacity` keys in ascending
 * order (out may be NULL) and returns the key count. fc4_history_get_series
 * copies at most `capacity` points (out may be NULL), sets *count to the
 * point count; false for a missing key. */
bool fc4_history_get_state(const fc4_history* h, fc4_history_state* out);
size_t fc4_history_keys(const fc4_history* h, uint32_t* out, size_t capacity);
bool fc4_history_get_series(const fc4_history* h, uint32_t key, uint32_t* max_count, fc4_history_point* out,
                            size_t capacity, size_t* count);
/* Persistence (§7): Write writes version 2; Read takes versions 0-2 and adds
 * to the series already there. */
bool fc4_history_write(const fc4_history* h, const fc4_ostream* out);
bool fc4_history_read(fc4_history* h, const fc4_istream* in);

/* ---- 24-hour clock (docs/specs/clock24.md; engine/capi/src/clock24.rs) ------
 * cSC424HourClock's core: the time of day, the visual time, the pause
 * counters, the cheats and the record. The shim owns the GZCOM object
 * (QueryInterface, reference count, the system service, the message target)
 * and reads the "Main Simulator" exemplar. Messages are posted (queued), so
 * no callback re-enters the clock; call it from the main thread only. */
typedef struct fc4_clock24 fc4_clock24;

typedef struct fc4_clock24_host {
    uint32_t struct_size; /* sizeof(fc4_clock24_host) */
    void* user;
    /* A monotonic clock in microseconds (the timer's unit 3, §4.3). */
    uint64_t (*now_micros)(void* user);
    /* MessagePost of a standard message (§3.3): void1 and the IGZUnknown
     * field = the clock, data2, data3. */
    void (*post)(void* user, uint32_t msg_type, uint32_t data2, uint32_t data3);
    /* Init / Shutdown (§4.2): the framework's system services and on-idle list. */
    void (*add_system_service)(void* user);
    void (*remove_system_service)(void* user);
    void (*add_to_on_idle)(void* user);
    void (*remove_from_on_idle)(void* user);
    /* The cheat code manager (§3.2): whether it exists, RegisterCheatCode,
     * the message-target notification, UnregisterCheatCode. */
    bool (*has_cheat_manager)(void* user);
    void (*register_cheat)(void* user, uint32_t id, const char* name, size_t name_len);
    void (*add_cheat_notification)(void* user, uint32_t id);
    void (*remove_cheat_notification)(void* user, uint32_t id);
    void (*unregister_cheat)(void* user, uint32_t id);
} fc4_clock24_host;

/* The "Main Simulator" exemplar's clock properties (§6): 0x897E709F
 * magnification, 0x09B73421 night start, 0x09B73422 night end. has_* = 0:
 * the property is missing. */
typedef struct fc4_clock24_tuning {
    uint32_t struct_size; /* sizeof(fc4_clock24_tuning) */
    uint8_t has_magnification, has_night_start, has_night_end, pad;
    float magnification;
    uint32_t night_start, night_end;
} fc4_clock24_tuning;

typedef struct fc4_clock24_state {
    uint32_t struct_size;
    uint8_t initialized, day_night_notifications, timer_running, pad;
    int32_t pause_count, hidden_pause_count;
    float magnification, min_magnification, max_magnification;
    uint32_t seconds, visual_seconds, night_start, night_end;
    int32_t visual_mode;
    uint64_t timer_elapsed_us;
} fc4_clock24_state;

/* The host table is copied. NULL if the table is missing or too small. */
fc4_clock24* fc4_clock24_create(const fc4_clock24_host* host);
void fc4_clock24_destroy(fc4_clock24* c);
/* QueryInterface (§1): 1, 0x681BD53A, 0x452294AA, 0xE4FDA3D4 and the
 * system service's 0x287FB697 [inferred]. */
bool fc4_clock24_answers_iid(uint32_t iid);
/* Init (slot 3, §4.2): tuning NULL = no exemplar (nothing changes); false for
 * a too-small tuning struct. Shutdown (slot 4). */
bool fc4_clock24_init(fc4_clock24* c, const fc4_clock24_tuning* tuning);
bool fc4_clock24_shutdown(fc4_clock24* c);
/* OnIdle (system-service slot 11, §4.3); the argument is not used. True. */
bool fc4_clock24_on_idle(fc4_clock24* c, uint32_t idle_count);
/* DoMessage (§4.8): the cheat message 0x230E27AC with data1 = the cheat ID and
 * the typed text (text may be NULL). True. */
bool fc4_clock24_do_message(fc4_clock24* c, uint32_t msg_type, uint32_t data1, const char* text, size_t text_len);
/* cISC424HourClock slots 5-29 (§8; the game's forms, not the SDK's). */
void fc4_clock24_set_visual_time_mode(fc4_clock24* c, int32_t mode);                   /* 5 */
int32_t fc4_clock24_get_visual_time_mode(const fc4_clock24* c);                        /* 6 */
void fc4_clock24_get_time_percentage(const fc4_clock24* c, float* out);                /* 7 */
void fc4_clock24_set_time_percentage(fc4_clock24* c, float p);                         /* 8 */
void fc4_clock24_get_visual_time_percentage(const fc4_clock24* c, float* out);         /* 9 */
uint32_t fc4_clock24_get_hour(const fc4_clock24* c);                                   /* 10 */
void fc4_clock24_set_hour(fc4_clock24* c, uint32_t hour);                              /* 11 */
uint32_t fc4_clock24_get_visual_hour(const fc4_clock24* c);                            /* 12 */
uint32_t fc4_clock24_get_minute(const fc4_clock24* c);                                 /* 13 */
uint32_t fc4_clock24_set_minute(fc4_clock24* c, uint32_t minute);                      /* 14: the new seconds */
uint32_t fc4_clock24_get_visual_minute(const fc4_clock24* c);                          /* 15 */
uint32_t fc4_clock24_get_seconds(const fc4_clock24* c);                                /* 16 */
uint32_t fc4_clock24_set_seconds(fc4_clock24* c, uint32_t seconds);                    /* 17: GetMinute */
uint32_t fc4_clock24_get_visual_seconds(const fc4_clock24* c);                         /* 18 */
uint32_t fc4_clock24_get_event_hour(const fc4_clock24* c, uint32_t index);             /* 19 */
uint32_t fc4_clock24_get_time_magnification(const fc4_clock24* c, float* out);         /* 20: the bits */
bool fc4_clock24_set_time_magnification(fc4_clock24* c, float m);                      /* 21 */
uint32_t fc4_clock24_get_time_magnification_min_max(const fc4_clock24* c, float* min,
                                                    float* max);                       /* 22: max's bits */
bool fc4_clock24_set_time_magnification_min_max(fc4_clock24* c, float min, float max); /* 23 */
bool fc4_clock24_pause(fc4_clock24* c, bool on);                                       /* 24 */
bool fc4_clock24_is_paused(const fc4_clock24* c);                                      /* 25 */
bool fc4_clock24_hidden_pause(fc4_clock24* c, bool on);                                /* 26 */
bool fc4_clock24_is_hidden_paused(const fc4_clock24* c);                               /* 27 */
bool fc4_clock24_is_night(const fc4_clock24* c);                                       /* 28 */
void fc4_clock24_enable_day_night_notifications(fc4_clock24* c, bool on);              /* 29 */
bool fc4_clock24_get_state(const fc4_clock24* c, fc4_clock24_state* out);
/* Persistence (§7): Write writes version 2 (26 bytes); Read has no version
 * check and skips version 1's timer record. */
bool fc4_clock24_write(const fc4_clock24* c, const fc4_ostream* out);
bool fc4_clock24_read(fc4_clock24* c, const fc4_istream* in);
/* ---- tract developer (docs/specs/tract-developer.md §1-§4, §6, §7) -------------
 *
 * The tract developer (cSC4TractDeveloper, class 0x2990C142, city component 17):
 * Grow (§4.3), Decline (§4.19), UpdatePopulation (§4.18), the style rotation
 * (§4.20, the only saved state, §6) and the tuning (§4.21). It owns no grid. The
 * BDS, the growth developers, the lot developer, the lot manager and the lot
 * configuration manager are reached through the host, whoever implements them.
 *
 * Tokens: growth developers, lots, lot configurations, configuration filters and
 * network tiles; 0 = none. Every token the host hands out is BORROWED and must
 * stay valid until the top-level call that received it returns (the core never
 * AddRefs or releases): a host keeps the references its lists gave it until
 * then. The core orders lots and configurations by token (the original orders
 * them by address, §9.5 item 1): hand out the object addresses.
 *
 * Statics (§9.1): the first fc4_tract_create of the process seeds the shared
 * file-local generator with derive_seed(session_seed, 0x4C09C113) (foundations
 * §6.3, §4.22 [choice]); later objects share it. The C rand() of R1 / R8 is the
 * core's own (state 1, foundations §12.6).
 *
 * Threads: single-threaded; the plugin serialises the calls on one object. A
 * call while the object is already inside the core (a host callback coming
 * back) gets the fallback and is counted (fc4_tract_state.reentrant).
 *
 * Shadow mode (fc4_tract_create_shadow): reads go to the game; writes are
 * swallowed and counted (services, agents, the message server, construction,
 * abandonment, reoccupation, lots created / deleted / turned / state changes,
 * demolition, terrain, construction resources, filters); SetPopulation calls
 * are recorded for fc4_tract_shadow_update_population. Own statics. */

typedef struct fc4_tract fc4_tract;

/* The "Building Development" tuning exemplar (§4.21). */
#define FC4_TRACT_TUNING_TYPE 0x6534284Au
#define FC4_TRACT_TUNING_GROUP 0xE7E2C2DBu
#define FC4_TRACT_TUNING_INSTANCE 0xE8DA7677u

/* dev_cap's `which` (growth developer slots 19-22) and cheat's (BDS slots 54-57). */
#define FC4_TRACT_CAP_POWER 0u
#define FC4_TRACT_CAP_WATER 1u
#define FC4_TRACT_CAP_FIRE 2u
#define FC4_TRACT_CAP_PARK 3u
#define FC4_TRACT_CHEAT_IGNORE_PLUMBING 0u
#define FC4_TRACT_CHEAT_IGNORE_POWER 1u
#define FC4_TRACT_CHEAT_IGNORE_CAPS 2u
#define FC4_TRACT_CHEAT_DEBUG_GROWTH 3u
/* The most bits cfg_required_roads may report (the core reads four). */
#define FC4_TRACT_MAX_ROAD_BITS 32u

/* GrowthProfile as the slots read it (§7.1); only the low byte of stage is used (Grow). */
typedef struct fc4_tract_profile {
    uint64_t developer;
    int32_t x, z;
    int32_t stage;
    int32_t reserved;
} fc4_tract_profile;

/* GetConstructionInfo's fields the tract developer reads (growth-developer §6.2). */
typedef struct fc4_tract_construction {
    int32_t zone;
    int32_t unknown2;
    uint32_t wealth;
    uint32_t purpose;
    uint8_t land_value_min, land_value_max;
    uint8_t reserved[2];
    uint32_t building_type;
    int32_t x, z;
} fc4_tract_construction;

/* BuildingProfile purpose (+0x00), wealth (+0x04), capacity (+0x08) (docs/abi
 * layouts.md); an unknown type reads as zeros. */
typedef struct fc4_tract_building_profile {
    uint32_t purpose;
    uint32_t wealth;
    uint16_t capacity;
    uint16_t reserved;
} fc4_tract_building_profile;

typedef void (*fc4_tract_style_callback)(void* ctx, uint32_t group, float fraction);
typedef void (*fc4_tract_population_callback)(void* ctx, uint64_t lot, uint32_t dev_type, uint16_t population);

/* Missing callbacks answer neutrally (0, false, empty, none). */
typedef struct fc4_tract_host {
    uint32_t struct_size;
    void* user;
    /* lifecycle (§1.3, §1.4): false = a service (or the simulator, the message server) is missing */
    bool (*acquire_services)(void* user);
    bool (*add_agent)(void* user, uint32_t level, uint32_t message);
    bool (*acquire_message_server)(void* user);
    void (*release_message_server)(void* user);
    void (*remove_agent)(void* user);
    void (*release_services)(void* user);
    /* A NEW fc4_props with the tuning exemplar's properties (§4.21), or NULL. */
    fc4_props* (*tuning)(void* user);
    /* growth developer (growth-developer.md): slots 5, 6, 7, 8, 9; the desirability
     * map's (slot 13) tract size, shift and tract value; the capacity map's (slot 14)
     * tract value; GetSourceDemand (18) then cISC4Demand slot 12 (false: no node);
     * the caps (19-22); GetMonopolyUse (25); GetStylePercentages (26) in key order. */
    int32_t (*dev_target_zone)(void* user, uint64_t dev);
    uint32_t (*dev_subtype)(void* user, uint64_t dev);
    uint32_t (*dev_purpose)(void* user, uint64_t dev);
    uint32_t (*dev_wealth)(void* user, uint64_t dev);
    void (*dev_construction_info)(void* user, uint64_t dev, fc4_tract_construction* out);
    int32_t (*dev_desirability_tract_size)(void* user, uint64_t dev);
    uint32_t (*dev_desirability_shift)(void* user, uint64_t dev);
    int16_t (*dev_desirability_value)(void* user, uint64_t dev, int32_t tx, int32_t tz);
    uint32_t (*dev_capacity_value)(void* user, uint64_t dev, int32_t tx, int32_t tz);
    bool (*dev_source_demand)(void* user, uint64_t dev, float* out);
    uint8_t (*dev_cap)(void* user, uint64_t dev, uint32_t which);
    int32_t (*dev_monopoly_use)(void* user, uint64_t dev);
    void (*dev_style_percentages)(void* user, uint64_t dev, fc4_tract_style_callback cb, void* ctx);
    /* BDS (building-development.md): slot 39; slot 19 (the types in an order whose
     * insertion into a 193-bucket STLport hash_set reproduces the set); slot 14 +
     * IsOccupantGroup; the bool property 0x2781283F; the uint32 array property
     * 0x29B55F73 (false: absent); slot 33 (no reference); slots 50, 51; 54-57. */
    void (*building_profile)(void* user, uint32_t building, fc4_tract_building_profile* out);
    void (*building_types_by_family)(void* user, uint32_t family, fc4_u32_callback cb, void* ctx);
    bool (*building_in_occupant_group)(void* user, uint32_t building, uint32_t group);
    bool (*building_is_monopoly)(void* user, uint32_t building);
    bool (*building_field_ids)(void* user, uint32_t building, fc4_u32_callback cb, void* ctx);
    uint64_t (*developer_for)(void* user, uint32_t purpose, uint32_t wealth);
    bool (*claim_construction_resources)(void* user, uint16_t amount);
    void (*release_construction_resources)(void* user, uint16_t amount);
    bool (*cheat)(void* user, uint32_t which);
    /* lot developer (lot-developer.md): slots 36, 6, 8, 9, 29 */
    bool (*needs_anchor_building)(void* user, const fc4_cell_rect* rect, uint32_t purpose);
    bool (*start_construction)(void* user, uint64_t lot, uint64_t cfg, uint32_t building, bool flag);
    bool (*abandon_building)(void* user, uint64_t lot, uint32_t reason);
    bool (*reoccupy_building)(void* user, uint64_t lot, uint32_t dev_type);
    float (*area_slope)(void* user, const fc4_cell_rect* rect);
    /* lot manager (zones.md): slots 6, 7, 9, 10, 20, 22, 24 */
    uint64_t (*get_lot)(void* user, int32_t x, int32_t z);
    void (*lots_in_cell_rect)(void* user, const fc4_cell_rect* rect, fc4_token_callback cb, void* ctx);
    void (*lots_surrounding_lot)(void* user, uint64_t lot, uint32_t flags, fc4_token_callback cb, void* ctx);
    void (*lots_surrounding_rect)(void* user, const fc4_cell_rect* rect, uint32_t flags, fc4_token_callback cb,
                                  void* ctx);
    int32_t (*can_create_lot)(void* user, int32_t x, int32_t z, int32_t w, int32_t h);
    uint64_t (*create_lot_if_possible)(void* user, int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing);
    bool (*delete_lot)(void* user, uint64_t lot);
    /* lots (cISC4Lot) */
    void (*lot_bounding_rect)(void* user, uint64_t lot, fc4_cell_rect* out);
    float (*lot_slope)(void* user, uint64_t lot);
    void (*lot_location)(void* user, uint64_t lot, int32_t* x, int32_t* z);
    void (*lot_size)(void* user, uint64_t lot, uint32_t* w, uint32_t* h);
    int32_t (*lot_facing)(void* user, uint64_t lot);
    void (*lot_set_facing)(void* user, uint64_t lot, int32_t facing);
    bool (*lot_has_road_access)(void* user, uint64_t lot);
    bool (*lot_has_organic_road_access)(void* user, uint64_t lot, uint32_t purpose);
    int32_t (*lot_zone_type)(void* user, uint64_t lot);
    uint32_t (*lot_state)(void* user, uint64_t lot);
    void (*lot_set_state)(void* user, uint64_t lot, uint32_t state, uint32_t wealth, uint32_t reason);
    bool (*lot_is_livable)(void* user, uint64_t lot);
    bool (*lot_has_capacity)(void* user, uint64_t lot);
    uint32_t (*lot_days_since_state_change)(void* user, uint64_t lot);
    uint32_t (*lot_occupant_wealth)(void* user, uint64_t lot);
    bool (*lot_is_historical)(void* user, uint64_t lot);
    uint32_t (*lot_building_type)(void* user, uint64_t lot);
    uint16_t (*lot_capacity)(void* user, uint64_t lot, uint32_t dev_type, bool flag);
    uint16_t (*lot_available_capacity)(void* user, uint64_t lot, uint32_t dev_type, bool flag);
    void (*lot_set_population)(void* user, uint64_t lot, uint32_t dev_type, uint16_t population);
    /* The growth stage of the lot's configuration; -1 without one. */
    int32_t (*lot_configuration_stage)(void* user, uint64_t lot);
    /* lot configurations (cISC4LotConfiguration slots 7, 9, 8, 11, 12, 13, 15-18, 21, 24) */
    void (*cfg_size)(void* user, uint64_t cfg, uint32_t* w, uint32_t* h);
    float (*cfg_min_slope)(void* user, uint64_t cfg);
    float (*cfg_max_slope)(void* user, uint64_t cfg);
    uint8_t (*cfg_growth_stage)(void* user, uint64_t cfg);
    uint16_t (*cfg_min_capacity)(void* user, uint64_t cfg);
    uint16_t (*cfg_max_capacity)(void* user, uint64_t cfg);
    bool (*cfg_compatible_zone)(void* user, uint64_t cfg, int32_t zone);
    bool (*cfg_compatible_wealth)(void* user, uint64_t cfg, uint32_t wealth);
    bool (*cfg_compatible_purpose)(void* user, uint64_t cfg, uint32_t purpose);
    bool (*cfg_compatible_style)(void* user, uint64_t cfg, uint32_t style);
    /* Writes up to `cap` bytes (0 / 1) and returns the bit count. */
    size_t (*cfg_required_roads)(void* user, uint64_t cfg, uint8_t* bits, size_t cap);
    /* false: no primary building object; else its object-ID list. */
    bool (*cfg_primary_building_ids)(void* user, uint64_t cfg, fc4_u32_callback cb, void* ctx);
    /* lot configuration manager: slot 8; a filter {wealth, purpose} allocated and
     * registered (slot 18); unregistered (slot 19) and freed; slot 9 */
    uint64_t (*get_lot_configuration)(void* user, uint32_t id);
    uint64_t (*register_configuration_filter)(void* user, uint32_t wealth, uint32_t purpose);
    void (*unregister_configuration_filter)(void* user, uint64_t filter);
    void (*configurations_by_filter)(void* user, uint64_t filter, fc4_token_callback cb, void* ctx);
    /* city grids and services: zone manager slot 12; land value slot 6; the powered
     * grid's cell value; plumbing slot 11; the fire grid (uint8); the park map
     * (int16); the trip length map (§4.8 row 7); city slots 93 / 94; FindNetworkTile
     * (x, z, 0xC4B, 1); network occupant IsOfType; DemolishLots(true, [lot], 0, 0x800,
     * false, 0, 0, 0); RedisplayTerrain(0, 0, rect, 0). */
    int32_t (*zone_at)(void* user, int32_t x, int32_t z);
    uint8_t (*land_value)(void* user, int32_t x, int32_t z);
    int32_t (*powered_value)(void* user, int32_t x, int32_t z);
    bool (*is_cell_watered)(void* user, int32_t x, int32_t z);
    uint8_t (*fire_coverage)(void* user, int32_t x, int32_t z);
    int16_t (*park_value)(void* user, int32_t x, int32_t z);
    int32_t (*road_access_value)(void* user, int32_t x, int32_t z);
    void (*city_cell_counts)(void* user, int32_t* x, int32_t* z);
    uint64_t (*find_network_tile)(void* user, int32_t x, int32_t z);
    bool (*network_is_of_type)(void* user, uint64_t tile, uint32_t network_type);
    bool (*demolish_lot)(void* user, uint64_t lot);
    void (*redisplay_terrain)(void* user, const fc4_cell_rect* rect);
} fc4_tract_host;

typedef struct fc4_tract_grow_result {
    uint32_t struct_size;
    int32_t built;            /* the return value (may be negative) */
    uint8_t out_flag;
    uint8_t reserved[3];
    uint32_t outcome_count;   /* codes appended to the outcome list */
    uint32_t outcomes[16];    /* the first 16 of them */
    uint32_t scored;          /* candidates scored (§4.14) */
    int32_t desired, desired_max;
    uint32_t reserved1;
    uint64_t chosen_cfg;      /* PickBuilding's last choice (0: none) */
    uint32_t chosen_building;
    uint32_t reserved2;
} fc4_tract_grow_result;

/* The style state, flags and counters (counters from creation). */
typedef struct fc4_tract_state {
    uint32_t struct_size;
    uint8_t initialized, configured, all_styles, reserved;
    uint32_t current_style;
    int32_t current_style_index;
    int32_t years_since_style_change;
    int32_t years_between_style_changes;
    uint32_t active_style_count;
    uint32_t active_styles[16];
    uint32_t filters;
    uint32_t grow_calls, decline_calls, update_population_calls;
    uint32_t reserved1;
    int64_t built_total, declined_total;
    uint32_t outcomes[28];    /* outcome codes appended, by code (§4.1) */
    uint32_t scored;
    uint32_t constructions, reoccupations, abandonments, lots_created, lots_deleted, demolitions, populations_set,
        resource_claims;
    uint32_t reentrant;       /* calls refused: the object was inside the core */
    uint32_t swallowed;       /* shadow: game writes swallowed */
    uint32_t reserved2;
} fc4_tract_state;

typedef struct fc4_tract_tuning {
    uint32_t struct_size;
    float overbuild;
    float reoccupy[2];
    float redevelop_size;
    float demolition_costs[2];
    float new_lot_costs[2];
    float occupancy_variation;
    float min_vacancy[2];
    uint8_t kick_out;
    uint8_t reserved[3];
    uint32_t available_style_count;
    uint32_t available_styles[16];
} fc4_tract_tuning;

/* QueryInterface (§1.1): 1, 0xA818ED1B, 0x452294AA, 0xE4FDA3D4. */
bool fc4_tract_query_interface(uint32_t iid);
uint32_t fc4_tract_class_id(void); /* 0x2990C142 */
/* The configuration filter's IsLotConfigurationIncluded (§4.6, §7.4) through
 * `host`'s configuration reads; needs no object. */
bool fc4_tract_filter_includes(const fc4_tract_host* host, uint64_t cfg, uint32_t wealth, uint32_t purpose);

/* The host table is copied; NULL when it is incomplete. */
fc4_tract* fc4_tract_create(const fc4_tract_host* host, uint32_t session_seed);
fc4_tract* fc4_tract_create_shadow(const fc4_tract_host* host);
void fc4_tract_destroy(fc4_tract* p);
bool fc4_tract_init(fc4_tract* p);                       /* §1.3 */
bool fc4_tract_shutdown(fc4_tract* p);                   /* §1.4: true; the style state is kept */
bool fc4_tract_do_message(fc4_tract* p, uint32_t msg);   /* §3: Year runs NewYear; true */
/* Slot 5 (§4.3): returns built; each appended code goes to `outcome` (may be NULL), in order. */
int32_t fc4_tract_grow(fc4_tract* p, const fc4_tract_profile* profile, fc4_u32_callback outcome, void* ctx,
                       bool* out_flag, fc4_tract_grow_result* out);
int32_t fc4_tract_decline(fc4_tract* p, const fc4_tract_profile* profile);           /* slot 6 (§4.19) */
void fc4_tract_update_population(fc4_tract* p, const fc4_tract_profile* profile);    /* slot 7 (§4.18) */
uint32_t fc4_tract_current_style(const fc4_tract* p);                                /* slot 8 */
/* Slot 9: copies up to `cap` styles to `out` (may be NULL); returns the count. */
size_t fc4_tract_active_styles(const fc4_tract* p, uint32_t* out, size_t cap);
void fc4_tract_set_active_styles(fc4_tract* p, const uint32_t* v, size_t n);         /* slot 10 */
bool fc4_tract_is_using_all_styles(const fc4_tract* p);                              /* slot 11 */
void fc4_tract_use_all_styles(fc4_tract* p, bool all);                               /* slot 12 */
int32_t fc4_tract_years_between_style_changes(const fc4_tract* p);                   /* slot 13 */
void fc4_tract_set_years_between_style_changes(fc4_tract* p, int32_t years);         /* slot 14 */
/* The §4.1 debug histogram (off by default; reset by each Grow): 13 x 28, row-major. */
void fc4_tract_set_histogram(fc4_tract* p, bool on);
bool fc4_tract_histogram(const fc4_tract* p, uint32_t* out, size_t cap);
bool fc4_tract_get_state(const fc4_tract* p, fc4_tract_state* out);
bool fc4_tract_get_tuning(const fc4_tract* p, fc4_tract_tuning* out);
/* ReadTuning and its style fix-ups (§4.21) without the rest of Init. */
void fc4_tract_read_tuning(fc4_tract* p);
bool fc4_tract_write(const fc4_tract* p, const fc4_ostream* out);   /* §6 */
bool fc4_tract_read(fc4_tract* p, const fc4_istream* in);           /* §6; before Init */
/* Shadow cores only (-1 / false otherwise): UpdatePopulation with each SetPopulation
 * handed to `cb` instead of the game (returns the count); Grow's setup for the
 * probes; the probes (§4.7, §4.8 with redevelop false, §4.17), not counted. */
int32_t fc4_tract_shadow_update_population(fc4_tract* p, const fc4_tract_profile* profile,
                                           fc4_tract_population_callback cb, void* ctx);
bool fc4_tract_probe_setup(fc4_tract* p, const fc4_tract_profile* profile, bool relaxed);
int32_t fc4_tract_probe_configuration_suitable(const fc4_tract* p, uint64_t cfg);
int32_t fc4_tract_probe_lot_suitable(const fc4_tract* p, uint64_t lot);
int32_t fc4_tract_probe_building_compatible(const fc4_tract* p, uint32_t building, int32_t lo, int32_t hi);

/* ---- commercial and industrial (docs/specs/commercial-industrial.md) ---------
 * The commercial simulator (cSC4CommercialSimulator, 0x8990C080) and the
 * industrial stub (0xA990C022). The shim owns the GZCOM objects (the
 * message-target and serializable sub-objects, the agent) and forwards
 * their slots here. Tokens are opaque host identities: occupants, the two
 * demand nodes and grids. The CO$$ / CO$$$ proximity grids are the game's
 * own cSC4SimGrid<uint8> objects made through `grids` (the core holds one
 * reference on each); fc4_commercial_proximity_map returns the grid's
 * token, which the shim turns back into the game's interface pointer. The
 * building map is read through the same table without a reference. A
 * missing callback reads as 0 / false / none. */
typedef struct fc4_commercial fc4_commercial;
typedef struct fc4_industrial fc4_industrial;

typedef struct fc4_commercial_host {
    uint32_t struct_size; /* sizeof(fc4_commercial_host) */
    void* user;
    /* AddAgent(messageTarget, level, name, flags) / RemoveAgent on the city's simulator (§4.1, §4.2). */
    void (*add_agent)(void* user, uint32_t level, const char* name, uint32_t flags);
    void (*remove_agent)(void* user);
    /* Hold the cached message server; false = none. Called only while none is held. */
    bool (*acquire_message_server)(void* user);
    /* Message server slots 5 / 6 with the message-target sub-object. */
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    void (*release_message_server)(void* user);
    /* Hold the cached city / building development simulator; false = none (§4.1 step 4). */
    bool (*acquire_city)(void* user);
    void (*release_city)(void* user);
    bool (*acquire_bds)(void* user);
    void (*release_bds)(void* user);
    /* city.GetDemandSimulator() (slot 56) is not NULL. */
    bool (*has_demand_simulator)(void* user);
    /* GetDemand(demand_id, 0), AddRef'ed for the core (0 = none); released with release_demand_node. */
    uint64_t (*demand_node)(void* user, uint32_t demand_id);
    void (*release_demand_node)(void* user, uint64_t node);
    /* BDS slot 41 GetBuildingMap(): a grid token readable through `grids`, not held (0 = none). */
    uint64_t (*building_map)(void* user);
    /* BDS slot 42 GetBuildingMapKey(purpose, wealth); only the low byte matters. */
    uint8_t (*building_map_key)(void* user, uint32_t purpose, uint32_t wealth);
    /* occupant QueryInterface(0x87DDA3A9), then the building profile's first u32
     * (the purpose); false when not a building occupant. The host releases the
     * interface. */
    bool (*building_purpose)(void* user, uint64_t occupant, uint32_t* purpose);
    /* The game's grids (required: a full-size table). */
    fc4_grid_api grids;
} fc4_commercial_host;

typedef struct fc4_commercial_state {
    uint32_t struct_size;
    uint8_t initialized, dirty, city_held, bds_held, message_server_held, destroyed;
    uint8_t flag; /* the saved flag byte (§8.1): dirty || state != 3 */
    uint8_t reserved0;
    int32_t state; /* the DoIdle state machine (§4.3) */
    uint32_t ref_count, reserved1;
    uint64_t demand_nodes[2];
    uint64_t co2_grid, co3_grid; /* the CO$$ / CO$$$ grid tokens (0 = none) */
} fc4_commercial_state;

/* Constructed, not initialised, reference count 0. The host table is copied;
 * NULL for a NULL or short table, or an incomplete `grids`. */
fc4_commercial* fc4_commercial_create(const fc4_commercial_host* host);
/* Frees the handle (and the grids' references if they are still held). */
void fc4_commercial_destroy(fc4_commercial* c);
/* QueryInterface (§1): the IIDs the object answers. */
bool fc4_commercial_query_interface(uint32_t iid);
uint32_t fc4_commercial_add_ref(fc4_commercial* c);
/* Release (§2.3): 0 when the object was destroyed (Shutdown ran); the handle
 * stays valid until fc4_commercial_destroy. */
uint32_t fc4_commercial_release(fc4_commercial* c);
bool fc4_commercial_init(fc4_commercial* c);
bool fc4_commercial_shutdown(fc4_commercial* c);
/* DoMessage (§3): the idle and day messages run one DoIdle step; an occupant
 * insert / remove (data3 = occupant type, void1 = the occupant) may mark the
 * maps dirty. Always true. */
bool fc4_commercial_do_message(fc4_commercial* c, uint32_t msg_type, uint32_t data1, uint32_t data2,
                               uint32_t data3, uint64_t void1);
/* One DoIdle step (§4.3). */
void fc4_commercial_do_idle(fc4_commercial* c);
/* Slot 5 GetProximityMap(developerType) (§4.6): the grid token, 0 for any other
 * type or no grid. No reference is added. */
uint64_t fc4_commercial_proximity_map(const fc4_commercial* c, uint32_t developer_type);
bool fc4_commercial_get_state(const fc4_commercial* c, fc4_commercial_state* out);
/* Persistence (§8.1): 19 bytes when both grids exist. Read: false unless the
 * version is 1; the grid references are read with IID 0xC9BD939A and each one
 * the stream hands over becomes the core's reference. */
bool fc4_commercial_write(const fc4_commercial* c, const fc4_ostream* out);
bool fc4_commercial_read(fc4_commercial* c, const fc4_istream* in);

/* The industrial stub (§7): constructed, reference count 0. */
fc4_industrial* fc4_industrial_create(void);
void fc4_industrial_destroy(fc4_industrial* p);
bool fc4_industrial_query_interface(uint32_t iid);
uint32_t fc4_industrial_add_ref(fc4_industrial* p);
uint32_t fc4_industrial_release(fc4_industrial* p);
bool fc4_industrial_init(fc4_industrial* p);
bool fc4_industrial_shutdown(fc4_industrial* p);
bool fc4_industrial_do_message(fc4_industrial* p, uint32_t msg_type);
bool fc4_industrial_is_initialized(const fc4_industrial* p);
/* Persistence (§8.2): `01 00`; Read is false unless the version is 1. */
bool fc4_industrial_write(const fc4_industrial* p, const fc4_ostream* out);
bool fc4_industrial_read(fc4_industrial* p, const fc4_istream* in);

/* ---- civic building simulator (docs/specs/civic-buildings.md) ----------------
 * cSC4CivicBuildingSimulator (0x0A413481). The shim owns the GZCOM objects and
 * forwards their slots here. Tokens are opaque host identities: occupants (a
 * building and its occupant are one token), departments, line items and
 * lots. The core holds one reference on the Landmarks department and on
 * every list entry. Statuses (§4.7): the original's slot 6 returns a pointer
 * callers write through; the core keeps the statuses, fc4_civic_get_status
 * copies one out and fc4_civic_update_status writes one back, so a shim that
 * hands out a stable ConditionalBuildingStatus* keeps a mirror and writes it
 * back. A missing callback reads as 0 / false / none. */
typedef struct fc4_civic fc4_civic;

typedef struct fc4_civic_host {
    uint32_t struct_size; /* sizeof(fc4_civic_host) */
    void* user;
    /* AddAgent(+4, level, name, flags) / RemoveAgent(+4) on the city's simulator. */
    void (*add_agent)(void* user, uint32_t level, const char* name, uint32_t flags);
    void (*remove_agent)(void* user);
    /* Message server slots 5 / 6 with the message-target sub-object. */
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    /* Init step 6: the Lua table `civic` with get_landmark_count() (=
     * fc4_civic_landmark_count); Shutdown unregisters it. */
    void (*register_lua)(void* user);
    void (*unregister_lua)(void* user);
    /* AddRef / Release on a host object. */
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    /* The token of a save reference (a token the host's fc4_istream handed over)
     * read with `iid`; the reference is transferred to the core. 0 = none (the
     * host drops it). */
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
    /* budget.GetDepartmentBudget(id) (not held; 0 = none) and
     * CreateDepartmentBudget(id, group) (the core AddRefs the one it holds). */
    uint64_t (*department_budget)(void* user, uint32_t id);
    uint64_t (*create_department_budget)(void* user, uint32_t id, uint32_t group);
    void (*set_department_name)(void* user, uint64_t dept, uint32_t group, uint32_t instance);
    void (*set_max_funding)(void* user, uint64_t dept, float max);
    /* *dept.GetFundingPercentage() (a fraction); false when unreadable. */
    bool (*department_funding)(void* user, uint64_t dept, float* out);
    /* budget.GetBudgetItemForPurpose(holder, purpose, &item); false when it fails. */
    bool (*budget_item)(void* user, uint64_t occupant, uint32_t purpose, fc4_budget_item* out);
    void (*change_budget_item_line)(void* user, uint64_t occupant, int32_t index, uint32_t line);
    void (*remove_budget_item)(void* user, uint64_t occupant, int32_t index);
    /* dept.GetLineItem(line) / CreateLineItemForBuildingType(line, isLocal) (0 = none) / RemoveLineItem(line). */
    uint64_t (*line_item)(void* user, uint64_t dept, uint32_t line);
    uint64_t (*create_line_item)(void* user, uint64_t dept, uint32_t line, bool local);
    void (*remove_line_item)(void* user, uint64_t dept, uint32_t line);
    /* Line item AddToFullExpenses / AddToIncome / GetIncome / SetIncome (clamps at 0) / SetType. */
    void (*add_to_full_expenses)(void* user, uint64_t line, int64_t amount);
    void (*add_to_income)(void* user, uint64_t line, int64_t amount);
    int64_t (*line_income)(void* user, uint64_t line);
    void (*set_line_income)(void* user, uint64_t line, int64_t income);
    void (*set_line_type)(void* user, uint64_t line, uint32_t line_type);
    /* occ.GetType(); a bool / u32 property of occ.AsPropertyHolder() (false = absent). */
    uint32_t (*occupant_type)(void* user, uint64_t occupant);
    bool (*occupant_bool_property)(void* user, uint64_t occupant, uint32_t id, bool* out);
    bool (*occupant_u32_property)(void* user, uint64_t occupant, uint32_t id, uint32_t* out);
    /* occ.QueryInterface(0x87DDA3A9) succeeds (the host releases the interface). */
    bool (*is_building_occupant)(void* user, uint64_t occupant);
    uint32_t (*building_type)(void* user, uint64_t building);
    /* BDS.GetBuildingKeyFromType(b.GetBuildingType()) (slot 14): the key's instance; false when it fails. */
    bool (*building_key_instance)(void* user, uint64_t building, uint32_t* instance);
    bool (*is_occupant_group)(void* user, uint64_t occupant, uint32_t group);
    /* The lot-state message's IGZUnknown QueryInterface(0xE818FCED): the lot, with a
     * reference the core releases (0 = not a lot). */
    uint64_t (*message_lot)(void* user, uint64_t unknown);
    /* lot.GetBuilding() (slot 48) as the building's occupant token (0 = none). */
    uint64_t (*lot_building)(void* user, uint64_t lot);
    /* lotManager.GetOccupantLot(occ) (slot 14) (0 = none). */
    uint64_t (*occupant_lot)(void* user, uint64_t occupant);
    uint32_t (*lot_condition)(void* user, uint64_t lot);
    void (*set_lot_condition)(void* user, uint64_t lot, uint32_t condition);
    /* city.GetAutomataControllerManager(), if any: AddControllersFromOccupantGroup(name, occ, 0). */
    void (*add_controllers)(void* user, const char* name, uint64_t occupant);
} fc4_civic_host;

typedef struct fc4_civic_state {
    uint32_t struct_size;
    uint8_t initialized, destroyed;
    uint16_t reserved0;
    uint32_t ref_count, exclusions, statuses, disease_centers;
    uint32_t maintained;  /* GetLandmarkCount() */
    uint32_t temporaries; /* temporary generators made so far (§5) */
    uint64_t landmarks;   /* the Landmarks department (0 = none) */
} fc4_civic_state;

/* Constructed, reference count 0. The host table is copied (NULL for a NULL or
 * short table). `session_seed` is foundations §6.3's S (the ape escape's
 * temporary generators, §5). */
fc4_civic* fc4_civic_create(const fc4_civic_host* host, uint32_t session_seed);
void fc4_civic_destroy(fc4_civic* c);
bool fc4_civic_query_interface(uint32_t iid);
uint32_t fc4_civic_add_ref(fc4_civic* c);
/* 0 when the object was destroyed (Shutdown ran); the handle stays valid until fc4_civic_destroy. */
uint32_t fc4_civic_release(fc4_civic* c);
bool fc4_civic_init(fc4_civic* c);
bool fc4_civic_shutdown(fc4_civic* c);
/* DoMessage (§3): SimNewMonth, occupant inserted / removed (void1 = the
 * occupant), lot state changed (data2 = the new state, `unknown` = the lot
 * message's IGZUnknown). Always true. */
bool fc4_civic_do_message(fc4_civic* c, uint32_t msg_type, uint32_t data1, uint32_t data2, uint32_t data3,
                          uint64_t void1, uint64_t unknown);
/* Slot 5 IsBuildingExcluded(id). */
bool fc4_civic_is_building_excluded(const fc4_civic* c, uint32_t id);
/* Slot 6 GetConditionalBuildingStatus(key), copied out: false when there is no
 * entry. The tooltip is copied NUL-terminated into `text` (truncated to
 * capacity - 1) and its full byte length stored in `text_len`. Every out
 * pointer may be NULL. */
bool fc4_civic_get_status(const fc4_civic* c, uint32_t key, uint8_t* status, uint16_t* building_count,
                          char* text, size_t capacity, size_t* text_len);
/* Slot 7 UpdateConditionalBuildingStatus(key, status): find or insert `key`,
 * then copy the status, count and text_len bytes of `text` (NULL = empty).
 * Returns true. */
bool fc4_civic_update_status(fc4_civic* c, uint32_t key, uint8_t status, uint16_t building_count,
                             const char* text, size_t text_len);
/* The status keys / exclusion IDs in hash order (the record's order). Return
 * the count; copy at most `capacity` (`out` may be NULL). */
size_t fc4_civic_status_keys(const fc4_civic* c, uint32_t* out, size_t capacity);
size_t fc4_civic_exclusions(const fc4_civic* c, uint32_t* out, size_t capacity);
/* Slot 8 GetLandmarkCount(). */
uint32_t fc4_civic_landmark_count(const fc4_civic* c);
/* `which` 0 = disease centers, 1 = maintained buildings, as tokens (0 = a null
 * entry). As fc4_civic_status_keys. */
size_t fc4_civic_list(const fc4_civic* c, uint32_t which, uint64_t* out, size_t capacity);
bool fc4_civic_get_state(const fc4_civic* c, fc4_civic_state* out);
/* Persistence (§7). Read: false unless the version is 6; references come back
 * through resolve_ref (the department with IID 0x223E2B67, list entries with
 * 0x26BB767E). */
bool fc4_civic_write(const fc4_civic* c, const fc4_ostream* out);
bool fc4_civic_read(fc4_civic* c, const fc4_istream* in);

/* ---- park manager (docs/specs/parks.md) ---------------------------------------
 * cSC4ParkManager (0xAA3FE6BC). The shim owns the GZCOM objects and forwards
 * their slots here. Tokens are opaque host identities: occupants (a building
 * and its occupant are one token), the department, line items and lots. The
 * core holds one reference on the department and on every list entry; the
 * city, budget simulator, message server and lot manager are held through
 * the acquire_* / release_* pairs. A missing callback reads as 0 / false /
 * none. */
typedef struct fc4_park fc4_park;

typedef struct fc4_park_host {
    uint32_t struct_size; /* sizeof(fc4_park_host) */
    void* user;
    /* AddAgent(+4, level, name, flags) / RemoveAgent(+4) on the city's simulator. */
    void (*add_agent)(void* user, uint32_t level, const char* name, uint32_t flags);
    void (*remove_agent)(void* user);
    /* Hold the message server; false = none. Called only while none is held. */
    bool (*acquire_message_server)(void* user);
    void (*subscribe)(void* user, uint32_t msg_type);
    void (*unsubscribe)(void* user, uint32_t msg_type);
    void (*release_message_server)(void* user);
    /* Hold the city / the budget simulator / the lot manager; false = none. */
    bool (*acquire_city)(void* user);
    void (*release_city)(void* user);
    bool (*acquire_budget)(void* user);
    void (*release_budget)(void* user);
    bool (*acquire_lot_manager)(void* user);
    void (*release_lot_manager)(void* user);
    /* AddRef / Release on a host object; resolve_ref as fc4_civic_host. */
    void (*add_ref)(void* user, uint64_t token);
    void (*release)(void* user, uint64_t token);
    uint64_t (*resolve_ref)(void* user, uint64_t token, uint32_t iid);
    /* The department, as fc4_civic_host. */
    uint64_t (*department_budget)(void* user, uint32_t id);
    uint64_t (*create_department_budget)(void* user, uint32_t id, uint32_t group);
    void (*set_department_name)(void* user, uint64_t dept, uint32_t group, uint32_t instance);
    void (*set_max_funding)(void* user, uint64_t dept, float max);
    bool (*department_funding)(void* user, uint64_t dept, float* out);
    /* budget.GetBudgetItemForPurpose / ChangeBudgetItemLine / GetLineItemFromBudgetItem. */
    bool (*budget_item)(void* user, uint64_t occupant, uint32_t purpose, fc4_budget_item* out);
    void (*change_budget_item_line)(void* user, uint64_t occupant, int32_t index, uint32_t line);
    uint64_t (*line_item_from_budget_item)(void* user, const fc4_budget_item* item);
    /* dept.GetLineItem / CreateLineItemForBuildingType (0 = none). */
    uint64_t (*line_item)(void* user, uint64_t dept, uint32_t line);
    uint64_t (*create_line_item)(void* user, uint64_t dept, uint32_t line, bool local);
    /* Line item SetDisplayFlag / AddToFullExpenses / Get- and SetSecondaryInfoField (64-bit). */
    void (*set_line_display_flag)(void* user, uint64_t line, uint32_t flag, bool on);
    void (*add_to_full_expenses)(void* user, uint64_t line, int64_t amount);
    int64_t (*line_secondary)(void* user, uint64_t line);
    void (*set_line_secondary)(void* user, uint64_t line, int64_t value);
    /* occ.GetType(); occ.AsPropertyHolder() has property `id` (any value); IsOccupantGroup. */
    uint32_t (*occupant_type)(void* user, uint64_t occupant);
    bool (*occupant_has_property)(void* user, uint64_t occupant, uint32_t id);
    bool (*is_occupant_group)(void* user, uint64_t occupant, uint32_t group);
    /* occ.QueryInterface(0x87DDA3A9) succeeds (the host releases the interface). */
    bool (*is_building_occupant)(void* user, uint64_t occupant);
    uint32_t (*building_type)(void* user, uint64_t building);
    /* The lot manager's GetLot(cellX, cellZ, false) at the cell city.PositionToCell
     * gives for the occupant's position (0 = none); lot.SetCondition(c). */
    uint64_t (*occupant_position_lot)(void* user, uint64_t occupant);
    void (*set_lot_condition)(void* user, uint64_t lot, uint32_t condition);
    /* Posts 0xAA667CE2: a new standard message with void1 = the manager
     * (`manager`, the token given at creation) and IGZUnknown = the zoo,
     * MessagePost(msg, false). */
    void (*post_zoo_escape)(void* user, uint64_t manager, uint64_t zoo);
    /* city.GetAutomataControllerManager(), if any: AddControllersFromOccupantGroup(name, occ, 0). */
    void (*add_controllers)(void* user, const char* name, uint64_t occupant);
} fc4_park_host;

typedef struct fc4_park_state {
    uint32_t struct_size;
    uint8_t initialized, destroyed, tuning_read, underfunded;
    uint8_t city_held, budget_held, message_server_held, lot_manager_held;
    uint32_t ref_count, parks, zoos;
    float distress_threshold;
    uint32_t curve_points; /* kept points of the escape curve */
    uint32_t reserved;
    uint64_t department; /* Parks and Recreation (0 = none) */
} fc4_park_state;

/* Constructed, reference count 0. The host table is copied (NULL for a NULL or
 * short table). `session_seed` seeds the member generator (§5:
 * derive(0xAA3FE6BC)); `manager` is the manager's own token (void1 of ZooEscape). */
fc4_park* fc4_park_create(const fc4_park_host* host, uint32_t session_seed, uint64_t manager);
void fc4_park_destroy(fc4_park* p);
bool fc4_park_query_interface(uint32_t iid);
uint32_t fc4_park_add_ref(fc4_park* p);
/* Release (§8): 0 when the object was destroyed (Shutdown ran); the handle stays valid until fc4_park_destroy. */
uint32_t fc4_park_release(fc4_park* p);
/* Init (§4.1). `tuning` = the "Parks Manager" exemplar's properties (copied;
 * NULL = none); read at the object's first Init only. */
bool fc4_park_init(fc4_park* p, const fc4_props* tuning);
bool fc4_park_shutdown(fc4_park* p);
/* DoMessage (§3): SimNewMonth, occupant inserted / removed (void1 = the occupant). Always true. */
bool fc4_park_do_message(fc4_park* p, uint32_t msg_type, uint32_t data1, uint32_t data2, uint32_t data3,
                         uint64_t void1);
/* §4.4 step 5's escape probability for a funding fraction, with the object's curve. */
double fc4_park_escape_probability(const fc4_park* p, float funding);
/* `which` 0 = parks, 1 = zoos, as tokens (0 = a null entry). Returns the
 * length; copies at most `capacity` (`out` may be NULL). */
size_t fc4_park_list(const fc4_park* p, uint32_t which, uint64_t* out, size_t capacity);
bool fc4_park_get_state(const fc4_park* p, fc4_park_state* out);
/* Persistence (§7). Read: false unless the version is 2; list entries come
 * back through resolve_ref (IID 0x26BB767E). */
bool fc4_park_write(const fc4_park* p, const fc4_ostream* out);
bool fc4_park_read(fc4_park* p, const fc4_istream* in);

/* ---- turbo clock (tooling, not simulator.md; fc4_core::Turbo) --------------
 * Tooling deviation for test runs (the plugin's sim_turbo). Off (the default)
 * changes nothing. On: the day gate's "ready" (spec §5.2 step 3) is always true
 * (the done[...] conditions still hold a day for its agents), and frameBudgetMs
 * is `frame_ms` while neither paused nor hidden-paused (the §5.1 value
 * otherwise). Pause and hidden pause stop time as without turbo.
 * `until_julian` (0 = none): turbo turns itself off on the first new day on or
 * after it, and clears it; fc4_sim_take_turbo_auto_off then returns that day
 * once. Takes effect from the next tick. Always returns true for a live sim. */
bool fc4_sim_set_turbo(fc4_sim* sim, bool on, uint32_t frame_ms, int32_t until_julian);
/* Pointers may be NULL. False only for a NULL sim. */
bool fc4_sim_get_turbo(const fc4_sim* sim, bool* on, uint32_t* frame_ms, int32_t* until_julian);
/* The Julian day of the last automatic switch-off, once; 0 if none since the last call. */
int32_t fc4_sim_take_turbo_auto_off(fc4_sim* sim);

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
static_assert(sizeof(fc4_landvalue_host) == (sizeof(void*) == 4 ? 96 : 192), "fc4_landvalue_host layout");
static_assert(sizeof(fc4_landvalue_tuning) == (sizeof(void*) == 4 ? 72 : 128), "fc4_landvalue_tuning layout");
static_assert(sizeof(fc4_weather_state) == 56, "fc4_weather_state layout");
static_assert(sizeof(fc4_weather_host) == (sizeof(void*) == 4 ? 56 : 112), "fc4_weather_host layout");
static_assert(sizeof(fc4_ordinance_sim_state) == 40, "fc4_ordinance_sim_state layout");
static_assert(sizeof(fc4_ordinance_host) == (sizeof(void*) == 4 ? 104 : 208), "fc4_ordinance_host layout");
static_assert(sizeof(fc4_demand_connection) == 24, "fc4_demand_connection layout");
static_assert(sizeof(fc4_demand_growth_developer) == 24, "fc4_demand_growth_developer layout");
static_assert(sizeof(fc4_demand_sim_state) == 44, "fc4_demand_sim_state layout");
static_assert(sizeof(fc4_demand_host) == (sizeof(void*) == 4 ? 172 : 344), "fc4_demand_host layout");
static_assert(sizeof(fc4_demand_node_state) == 48, "fc4_demand_node_state layout");
static_assert(sizeof(fc4_demand_shadow_info) == 24, "fc4_demand_shadow_info layout");
static_assert(sizeof(fc4_demand_shadow_stats) == 32, "fc4_demand_shadow_stats layout");
static_assert(sizeof(fc4_demand_modifiers) == 400, "fc4_demand_modifiers layout");
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
static_assert(sizeof(fc4_power_capture_ref) == 32, "fc4_power_capture_ref layout");
static_assert(sizeof(fc4_power_capture) == (sizeof(void*) == 4 ? 24 : 40), "fc4_power_capture layout");
static_assert(sizeof(fc4_power_shadow_counts) == 24, "fc4_power_shadow_counts layout");
static_assert(sizeof(fc4_power_shadow_options) == (sizeof(void*) == 4 ? 20 : 24), "fc4_power_shadow_options layout");
static_assert(sizeof(fc4_power_shadow_stats) == 72, "fc4_power_shadow_stats layout");
static_assert(sizeof(fc4_plumbing_budget_item) == 24, "fc4_plumbing_budget_item layout");
static_assert(sizeof(fc4_plumbing_message) == 40, "fc4_plumbing_message layout");
static_assert(sizeof(fc4_plumbing_producer_info) == 20, "fc4_plumbing_producer_info layout");
static_assert(sizeof(fc4_plumbing_found) == 32, "fc4_plumbing_found layout");
static_assert(sizeof(fc4_plumbing_state) == 144, "fc4_plumbing_state layout");
static_assert(sizeof(fc4_plumbing_host) == (sizeof(void*) == 4 ? 356 : 712), "fc4_plumbing_host layout");
static_assert(sizeof(fc4_plumbing_capture_ref) == 40, "fc4_plumbing_capture_ref layout");
static_assert(sizeof(fc4_plumbing_capture) == (sizeof(void*) == 4 ? 24 : 40), "fc4_plumbing_capture layout");
static_assert(sizeof(fc4_plumbing_original) == (sizeof(void*) == 4 ? 32 : 64), "fc4_plumbing_original layout");
static_assert(sizeof(fc4_plumbing_shadow_stats) == 72, "fc4_plumbing_shadow_stats layout");
static_assert(sizeof(fc4_flammability_message) == 24, "fc4_flammability_message layout");
static_assert(sizeof(fc4_flammability_state) == 80, "fc4_flammability_state layout");
static_assert(sizeof(fc4_flammability_host) == (sizeof(void*) == 4 ? 136 : 272), "fc4_flammability_host layout");
static_assert(sizeof(fc4_pollution_budget_item) == 32, "fc4_pollution_budget_item layout");
static_assert(sizeof(fc4_pollution_grid_stats) == 24, "fc4_pollution_grid_stats layout");
static_assert(sizeof(fc4_pollution_state) == 408, "fc4_pollution_state layout");
static_assert(sizeof(fc4_pollution_host) == (sizeof(void*) == 4 ? 256 : 512), "fc4_pollution_host layout");
static_assert(sizeof(fc4_pollution_capture_ref) == 32, "fc4_pollution_capture_ref layout");
static_assert(sizeof(fc4_pollution_capture) == (sizeof(void*) == 4 ? 24 : 40), "fc4_pollution_capture layout");
static_assert(sizeof(fc4_pollution_shadow_stats) == 64, "fc4_pollution_shadow_stats layout");
static_assert(sizeof(fc4_protection_budget_item) == 32, "fc4_protection_budget_item layout");
static_assert(sizeof(fc4_protection_dispatch_info) == 32, "fc4_protection_dispatch_info layout");
static_assert(sizeof(fc4_protection_message) == 32, "fc4_protection_message layout");
static_assert(sizeof(fc4_protection_state) == 152, "fc4_protection_state layout");
static_assert(sizeof(fc4_protection_host) == (sizeof(void*) == 4 ? 220 : 440), "fc4_protection_host layout");
static_assert(sizeof(fc4_crime_message) == 24, "fc4_crime_message layout");
static_assert(sizeof(fc4_crime_event) == 28, "fc4_crime_event layout");
static_assert(sizeof(fc4_crime_nearby) == 32, "fc4_crime_nearby layout");
static_assert(sizeof(fc4_crime_state) == 152, "fc4_crime_state layout");
static_assert(sizeof(fc4_crime_host) == (sizeof(void*) == 4 ? 200 : 400), "fc4_crime_host layout");
static_assert(sizeof(fc4_crime_shadow_info) == 24, "fc4_crime_shadow_info layout");
static_assert(sizeof(fc4_crime_shadow_incident) == 24, "fc4_crime_shadow_incident layout");
static_assert(sizeof(fc4_crime_shadow_month_in) == (sizeof(void*) == 4 ? 64 : 88), "fc4_crime_shadow_month_in layout");
static_assert(sizeof(fc4_crime_shadow_month_out) == 40, "fc4_crime_shadow_month_out layout");
static_assert(sizeof(fc4_aura_lot) == 32, "fc4_aura_lot layout");
static_assert(sizeof(fc4_aura_ratings) == 64, "fc4_aura_ratings layout");
static_assert(sizeof(fc4_aura_state) == 120, "fc4_aura_state layout");
static_assert(sizeof(fc4_aura_host) == (sizeof(void*) == 4 ? 176 : 352), "fc4_aura_host layout");
static_assert(sizeof(fc4_aura_capture_ref) == 16, "fc4_aura_capture_ref layout");
static_assert(sizeof(fc4_aura_capture) == (sizeof(void*) == 4 ? 24 : 40), "fc4_aura_capture layout");
static_assert(sizeof(fc4_aura_shadow_stats) == 64, "fc4_aura_shadow_stats layout");
static_assert(sizeof(fc4_aura_shadow_step_in) == 24, "fc4_aura_shadow_step_in layout");
static_assert(sizeof(fc4_flora_record) == 360, "fc4_flora_record layout");
static_assert(sizeof(fc4_flora_occupant) == 16, "fc4_flora_occupant layout");
static_assert(sizeof(fc4_flora_param) == 8, "fc4_flora_param layout");
static_assert(sizeof(fc4_flora_message) == (sizeof(void*) == 4 ? 40 : 56), "fc4_flora_message layout");
static_assert(sizeof(fc4_flora_state) == 180, "fc4_flora_state layout");
static_assert(sizeof(fc4_flora_registry_host) == (sizeof(void*) == 4 ? 20 : 40), "fc4_flora_registry_host layout");
static_assert(sizeof(fc4_flora_host) == (sizeof(void*) == 4 ? 260 : 520), "fc4_flora_host layout");
static_assert(sizeof(fc4_flora_census_entry) == 72, "fc4_flora_census_entry layout");
static_assert(sizeof(fc4_flora_shadow_month_in) == (sizeof(void*) == 4 ? 56 : 96),
              "fc4_flora_shadow_month_in layout");
static_assert(sizeof(fc4_flora_shadow_stats) == 64, "fc4_flora_shadow_stats layout");
static_assert(sizeof(fc4_landfill_region) == (sizeof(void*) == 4 ? 24 : 32), "fc4_landfill_region layout");
static_assert(sizeof(fc4_landfill_effects) == 16, "fc4_landfill_effects layout");
static_assert(sizeof(fc4_landfill_state) == 112, "fc4_landfill_state layout");
static_assert(sizeof(fc4_landfill_host) == (sizeof(void*) == 4 ? 252 : 504), "fc4_landfill_host layout");
static_assert(sizeof(fc4_landfill_shadow_stats) == 72, "fc4_landfill_shadow_stats layout");
static_assert(sizeof(fc4_landvalue_original) == (sizeof(void*) == 4 ? 132 : 152), "fc4_landvalue_original layout");
static_assert(sizeof(fc4_landvalue_shadow_stats) == 48, "fc4_landvalue_shadow_stats layout");
static_assert(sizeof(fc4_flammability_shadow_hotspot) == 12, "fc4_flammability_shadow_hotspot layout");
static_assert(sizeof(fc4_flammability_original) == (sizeof(void*) == 4 ? 128 : 144), "fc4_flammability_original layout");
static_assert(sizeof(fc4_flammability_shadow_stats) == 48, "fc4_flammability_shadow_stats layout");
static_assert(sizeof(fc4_residential_budget_item) == 32, "fc4_residential_budget_item layout");
static_assert(sizeof(fc4_residential_query_data) == 56, "fc4_residential_query_data layout");
static_assert(sizeof(fc4_residential_min_max) == 24, "fc4_residential_min_max layout");
static_assert(sizeof(fc4_residential_snapshot) == 160, "fc4_residential_snapshot layout");
static_assert(sizeof(fc4_residential_state) == 144, "fc4_residential_state layout");
static_assert(sizeof(fc4_residential_host) == (sizeof(void*) == 4 ? 292 : 584), "fc4_residential_host layout");
static_assert(sizeof(fc4_residential_capture_ref) == 32, "fc4_residential_capture_ref layout");
static_assert(sizeof(fc4_residential_capture) == (sizeof(void*) == 4 ? 24 : 40), "fc4_residential_capture layout");
static_assert(sizeof(fc4_residential_shadow_scalars) == 24, "fc4_residential_shadow_scalars layout");
static_assert(sizeof(fc4_residential_shadow_stats) == 72, "fc4_residential_shadow_stats layout");
static_assert(sizeof(fc4_res_key) == 12, "fc4_res_key layout");
static_assert(sizeof(fc4_growth_profile) == 24, "fc4_growth_profile layout");
static_assert(sizeof(fc4_growth_item) == 12, "fc4_growth_item layout");
static_assert(sizeof(fc4_construction_profile) == 0x24, "fc4_construction_profile layout");
static_assert(sizeof(fc4_lot_config) == (sizeof(void*) == 4 ? 36 : 40), "fc4_lot_config layout");
static_assert(sizeof(fc4_building_profile) == 48, "fc4_building_profile layout");
static_assert(sizeof(fc4_development_message) == 40, "fc4_development_message layout");
static_assert(sizeof(fc4_command_param) == 8, "fc4_command_param layout");
static_assert(sizeof(fc4_advice_subject) == 24, "fc4_advice_subject layout");
static_assert(sizeof(fc4_plan_entry) == 24, "fc4_plan_entry layout");
static_assert(sizeof(fc4_development_state) == 208, "fc4_development_state layout");
static_assert(sizeof(fc4_development_host) == (sizeof(void*) == 4 ? 388 : 776), "fc4_development_host layout");
static_assert(sizeof(fc4_growth_message) == 48, "fc4_growth_message layout");
static_assert(sizeof(fc4_growth_state) == 136, "fc4_growth_state layout");
static_assert(sizeof(fc4_growth_host) == (sizeof(void*) == 4 ? 200 : 400), "fc4_growth_host layout");
static_assert(sizeof(fc4_zone_region) == (sizeof(void*) == 4 ? 24 : 32), "fc4_zone_region layout");
static_assert(sizeof(fc4_zones_place_result) == 32, "fc4_zones_place_result layout");
static_assert(sizeof(fc4_zones_dezone) == 32, "fc4_zones_dezone layout");
static_assert(sizeof(fc4_zones_state) == 176, "fc4_zones_state layout");
static_assert(sizeof(fc4_zones_tuning) == 584, "fc4_zones_tuning layout");
static_assert(sizeof(fc4_zones_host) == (sizeof(void*) == 4 ? 188 : 376), "fc4_zones_host layout");
static_assert(sizeof(fc4_neighbor_deal_data) == 72, "fc4_neighbor_deal_data layout");
static_assert(sizeof(fc4_neighbor_connection_state) == 24, "fc4_neighbor_connection_state layout");
static_assert(sizeof(fc4_neighbors_figures) == 128, "fc4_neighbors_figures layout");
static_assert(sizeof(fc4_neighbors_message) == 24, "fc4_neighbors_message layout");
static_assert(sizeof(fc4_neighbors_state) == 48, "fc4_neighbors_state layout");
static_assert(sizeof(fc4_neighbors_host) == (sizeof(void*) == 4 ? 216 : 432), "fc4_neighbors_host layout");
static_assert(sizeof(fc4_neighbors_shadow_city) == 168, "fc4_neighbors_shadow_city layout");
static_assert(sizeof(fc4_neighbors_shadow_conn) == 40, "fc4_neighbors_shadow_conn layout");
static_assert(sizeof(fc4_neighbors_shadow_input) == (sizeof(void*) == 4 ? 208 : 232), "fc4_neighbors_shadow_input layout");
static_assert(sizeof(fc4_neighbors_shadow_counters) == 32, "fc4_neighbors_shadow_counters layout");
static_assert(sizeof(fc4_history_host) == (sizeof(void*) == 4 ? 36 : 72), "fc4_history_host layout");
static_assert(sizeof(fc4_history_point) == 8, "fc4_history_point layout");
static_assert(sizeof(fc4_history_state) == 28, "fc4_history_state layout");
static_assert(sizeof(fc4_clock24_host) == (sizeof(void*) == 4 ? 52 : 104), "fc4_clock24_host layout");
static_assert(sizeof(fc4_clock24_tuning) == 20, "fc4_clock24_tuning layout");
static_assert(sizeof(fc4_clock24_state) == 56, "fc4_clock24_state layout");
static_assert(sizeof(fc4_commercial_state) == 56, "fc4_commercial_state layout");
static_assert(sizeof(fc4_commercial_host) == (sizeof(void*) == 4 ? 112 : 224), "fc4_commercial_host layout");
static_assert(sizeof(fc4_civic_state) == 40, "fc4_civic_state layout");
static_assert(sizeof(fc4_civic_host) == (sizeof(void*) == 4 ? 160 : 320), "fc4_civic_host layout");
static_assert(sizeof(fc4_park_state) == 48, "fc4_park_state layout");
static_assert(sizeof(fc4_park_host) == (sizeof(void*) == 4 ? 160 : 320), "fc4_park_host layout");
static_assert(sizeof(fc4_tract_profile) == 24, "fc4_tract_profile layout");
static_assert(sizeof(fc4_tract_construction) == 32, "fc4_tract_construction layout");
static_assert(sizeof(fc4_tract_building_profile) == 12, "fc4_tract_building_profile layout");
static_assert(sizeof(fc4_tract_host) == (sizeof(void*) == 4 ? 368 : 736), "fc4_tract_host layout");
static_assert(sizeof(fc4_tract_grow_result) == 112, "fc4_tract_grow_result layout");
static_assert(sizeof(fc4_tract_state) == 288, "fc4_tract_state layout");
static_assert(sizeof(fc4_tract_tuning) == 120, "fc4_tract_tuning layout");
#else
_Static_assert(sizeof(fc4_message) == 32, "fc4_message layout");
_Static_assert(sizeof(fc4_date) == 24, "fc4_date layout");
_Static_assert(sizeof(fc4_debug) == 76, "fc4_debug layout");
_Static_assert(sizeof(fc4_agent_info) == 16, "fc4_agent_info layout");
_Static_assert(sizeof(fc4_power_capture_ref) == 32, "fc4_power_capture_ref layout");
_Static_assert(sizeof(fc4_power_shadow_counts) == 24, "fc4_power_shadow_counts layout");
_Static_assert(sizeof(fc4_power_shadow_stats) == 72, "fc4_power_shadow_stats layout");
_Static_assert(sizeof(fc4_neighbor_deal_data) == 72, "fc4_neighbor_deal_data layout");
_Static_assert(sizeof(fc4_neighbors_shadow_city) == 168, "fc4_neighbors_shadow_city layout");
_Static_assert(sizeof(fc4_history_host) == (sizeof(void*) == 4 ? 36 : 72), "fc4_history_host layout");
_Static_assert(sizeof(fc4_history_point) == 8, "fc4_history_point layout");
_Static_assert(sizeof(fc4_history_state) == 28, "fc4_history_state layout");
_Static_assert(sizeof(fc4_clock24_host) == (sizeof(void*) == 4 ? 52 : 104), "fc4_clock24_host layout");
_Static_assert(sizeof(fc4_clock24_tuning) == 20, "fc4_clock24_tuning layout");
_Static_assert(sizeof(fc4_clock24_state) == 56, "fc4_clock24_state layout");
_Static_assert(sizeof(fc4_commercial_state) == 56, "fc4_commercial_state layout");
_Static_assert(sizeof(fc4_civic_state) == 40, "fc4_civic_state layout");
_Static_assert(sizeof(fc4_park_state) == 48, "fc4_park_state layout");
#endif

#endif /* FC4_H */
