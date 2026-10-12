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
 * 3 = adds fc4_sim_get_debug and fc4_sim_get_agent_info (diagnostics). */
#define FC4_ABI_VERSION 3u

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

typedef void (*fc4_name_callback)(void* ctx, const char* name, size_t name_len);

/* ---- library ---------------------------------------------------------- */
uint32_t fc4_abi_version(void);

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
static_assert(sizeof(fc4_state) == 4 + 4 + 5 * 4 + 5 * 4 + 3 * 4 + 5 * 4 + 11 * 4 + 12 + 11 * 4, "fc4_state layout");
#else
_Static_assert(sizeof(fc4_message) == 32, "fc4_message layout");
_Static_assert(sizeof(fc4_date) == 24, "fc4_date layout");
_Static_assert(sizeof(fc4_debug) == 76, "fc4_debug layout");
_Static_assert(sizeof(fc4_agent_info) == 16, "fc4_agent_info layout");
#endif

#endif /* FC4_H */
