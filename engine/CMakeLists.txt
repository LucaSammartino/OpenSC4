# SPDX-License-Identifier: GPL-3.0-or-later
#
# FreeCity4 engine build.
#
#   - The simulation core is Rust (Cargo workspace in this directory). CMake
#     invokes cargo to build the fc4-capi static library for the same
#     architecture as the C++ compiler, and runs `cargo test` from CTest.
#   - On Windows/MSVC x86 (32-bit) it also builds the GZCOM plugin
#     FreeCity4.dll (engine/plugin) against the gzcom-dll SDK submodule.
#
# Typical Windows build (x86 Developer environment):
#   cmake -S engine -B build-x86 -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
#   cmake --build build-x86 && ctest --test-dir build-x86 --output-on-failure

cmake_minimum_required(VERSION 3.20)
project(FreeCity4 VERSION 0.1.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Static CRT, release flavour in every configuration: the Rust staticlib is
# built with +crt-static (libcmt), and FreeCity4.dll must not depend on any
# runtime DLL.
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")

enable_testing()

# Checked plugin build (in-game bisection tooling): no optimisation and the
# MSVC run-time checks (/RTC1: ESP verified after every call through a
# pointer or vtable, local-variable frames guarded), plus out-parameter
# canaries (plugin/FC4Checked.h). Use a separate build directory, e.g.
#   cmake -S engine -B build-x86-checked -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DFC4_CHECKED=ON
option(FC4_CHECKED "Build the checked plugin (/Od /RTC1, out-parameter canaries)" OFF)
if(FC4_CHECKED AND MSVC)
    # /RTC1 cannot be combined with optimisation: strip /O* from every configuration.
    foreach(fc4_lang C CXX)
        foreach(fc4_cfg "" _DEBUG _RELEASE _RELWITHDEBINFO _MINSIZEREL)
            string(REGEX REPLACE "/O[0-9a-z]+" "" CMAKE_${fc4_lang}_FLAGS${fc4_cfg} "${CMAKE_${fc4_lang}_FLAGS${fc4_cfg}}")
            string(REGEX REPLACE "/RTC[1csu]+" "" CMAKE_${fc4_lang}_FLAGS${fc4_cfg} "${CMAKE_${fc4_lang}_FLAGS${fc4_cfg}}")
        endforeach()
    endforeach()
    message(STATUS "FC4_CHECKED: plugin built with /Od /RTC1 and out-parameter canaries")
endif()

# ---- Rust core (cargo) ----------------------------------------------------------

find_program(CARGO_EXECUTABLE cargo
    HINTS "$ENV{CARGO_HOME}/bin" "$ENV{USERPROFILE}/.cargo/bin" "$ENV{HOME}/.cargo/bin"
    REQUIRED)

set(FC4_IS_WIN32_X86 OFF)
if(MSVC)
    if(CMAKE_SIZEOF_VOID_P EQUAL 4)
        set(FC4_RUST_TARGET "i686-pc-windows-msvc")
        set(FC4_IS_WIN32_X86 ON)
    else()
        set(FC4_RUST_TARGET "x86_64-pc-windows-msvc")
    endif()
endif()
set(FC4_RUST_TARGET "${FC4_RUST_TARGET}" CACHE STRING "Rust target triple for fc4-capi (empty = host)")

set(FC4_CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo")
set(FC4_CARGO_TARGET_ARGS "")
set(FC4_CARGO_OUT_DIR "${FC4_CARGO_TARGET_DIR}/release")
if(FC4_RUST_TARGET)
    set(FC4_CARGO_TARGET_ARGS --target ${FC4_RUST_TARGET})
    set(FC4_CARGO_OUT_DIR "${FC4_CARGO_TARGET_DIR}/${FC4_RUST_TARGET}/release")
endif()

if(MSVC)
    set(FC4_CAPI_LIB "${FC4_CARGO_OUT_DIR}/fc4_capi.lib")
    # From `cargo rustc -- --print native-static-libs`.
    set(FC4_CAPI_SYSTEM_LIBS kernel32 ntdll userenv ws2_32 dbghelp)
else()
    set(FC4_CAPI_LIB "${FC4_CARGO_OUT_DIR}/libfc4_capi.a")
    set(FC4_CAPI_SYSTEM_LIBS pthread dl m)
endif()

# Cargo does its own incremental build; always ask it. It must run from
# engine/ so that engine/.cargo/config.toml (static CRT) applies.
add_custom_target(fc4_capi_cargo ALL
    COMMAND ${CMAKE_COMMAND} -E env "CARGO_TARGET_DIR=${FC4_CARGO_TARGET_DIR}"
            ${CARGO_EXECUTABLE} build --release -p fc4-capi ${FC4_CARGO_TARGET_ARGS}
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    BYPRODUCTS "${FC4_CAPI_LIB}"
    COMMENT "cargo build fc4-capi (${FC4_RUST_TARGET})"
    USES_TERMINAL
    VERBATIM)

add_library(fc4_capi INTERFACE)
target_include_directories(fc4_capi INTERFACE "${CMAKE_CURRENT_SOURCE_DIR}/capi/include")
target_link_libraries(fc4_capi INTERFACE "${FC4_CAPI_LIB}" ${FC4_CAPI_SYSTEM_LIBS})
add_dependencies(fc4_capi fc4_capi_cargo)

# ---- tests ---------------------------------------------------------------------------

# Rust unit tests + conformance vectors, for the same target as the plugin.
add_test(NAME cargo_test
    COMMAND ${CMAKE_COMMAND} -E env "CARGO_TARGET_DIR=${FC4_CARGO_TARGET_DIR}"
            ${CARGO_EXECUTABLE} test --workspace ${FC4_CARGO_TARGET_ARGS}
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")

# The C ABI exercised from C++ with this compiler/architecture/CRT.
add_executable(fc4_capi_smoke tests/capi_smoke.cpp)
target_link_libraries(fc4_capi_smoke PRIVATE fc4_capi)
add_test(NAME capi_smoke COMMAND fc4_capi_smoke)

# ---- GZCOM plugin (Windows x86 only) -------------------------------------------------

set(FC4_GZCOM_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/gzcom-dll/gzcom-dll")

if(FC4_IS_WIN32_X86)
    if(NOT EXISTS "${FC4_GZCOM_DIR}/include/cRZCOMDllDirector.h")
        message(FATAL_ERROR "gzcom-dll submodule missing: run `git submodule update --init`")
    endif()

    add_library(FreeCity4 SHARED
        plugin/cFC4Director.cpp
        plugin/cFC4Simulator.cpp
        plugin/cFC4Simulator.h
        plugin/cFC4Date.cpp
        plugin/cFC4Date.h
        plugin/cFC4Message2Standard.cpp
        plugin/cFC4Message2Standard.h
        plugin/FC4Log.cpp
        plugin/FC4Log.h
        plugin/dllmain.cpp
        plugin/FC4Diag.cpp
        plugin/FC4Diag.h
        plugin/FC4Watchdog.cpp
        plugin/FC4Watchdog.h
        plugin/FC4Config.cpp
        plugin/FC4Config.h
        plugin/FC4Seed.cpp
        plugin/FC4Seed.h
        plugin/FC4Pacing.cpp
        plugin/FC4Pacing.h
        plugin/FC4RefAudit.cpp
        plugin/FC4RefAudit.h
        plugin/FC4Checked.cpp
        plugin/FC4Checked.h
        plugin/FC4Autoload.cpp
        plugin/FC4Autoload.h
        plugin/FC4Autotest.cpp
        plugin/FC4Autotest.h
        plugin/FC4DiagLogic.h
        plugin/FC4Observer.cpp
        plugin/FC4Fpu.cpp
        plugin/FC4FailFast.cpp
        plugin/FC4FailFast.h
        plugin/FC4Observer.h
        plugin/FC4ClockProbe.cpp
        plugin/FC4ClockProbe.h
        plugin/FC4Popups.cpp
        plugin/FC4Popups.h
        plugin/FC4Fpu.h
        plugin/FC4FpuLogic.h
        plugin/FC4DiagContext.h
        plugin/FC4TestSeam.cpp
        plugin/FC4TestSeam.h
        plugin/cFC4Variant.cpp
        plugin/cFC4Variant.h
        plugin/compat/FC4Compat.cpp
        plugin/compat/FC4Compat.h
        plugin/FC4Properties.cpp
        plugin/FC4Properties.h
        plugin/FC4Streams.cpp
        plugin/FC4Streams.h
        plugin/FC4Subsystems.cpp
        plugin/FC4Subsystems.h
        plugin/cFC4Budget.cpp
        plugin/cFC4Budget.h
        plugin/FC4BudgetHost.cpp
        plugin/FC4BudgetShadow.cpp
        plugin/FC4DemandShadow.cpp
        plugin/cFC4LandValue.cpp
        plugin/cFC4LandValue.h
        plugin/cFC4Weather.cpp
        plugin/cFC4Weather.h
        plugin/cFC4Ordinance.cpp
        plugin/cFC4Ordinance.h
        plugin/cFC4Demand.cpp
        plugin/cFC4Demand.h
        plugin/FC4CoreProps.cpp
        plugin/FC4CoreProps.h
        plugin/FC4HostGrid.cpp
        plugin/FC4HostGrid.h
        plugin/cFC4Power.cpp
        plugin/cFC4Power.h
        plugin/cFC4Plumbing.cpp
        plugin/cFC4Plumbing.h
        plugin/FC4PlumbingShadow.cpp
        plugin/FC4PlumbingShadow.h
        plugin/cFC4Flammability.cpp
        plugin/cFC4Flammability.h
        plugin/cFC4Pollution.cpp
        plugin/cFC4Pollution.h
        plugin/FC4PollutionShadow.cpp
        plugin/FC4PollutionShadow.h
        plugin/FC4PowerShadow.cpp
        plugin/FC4PowerShadow.h
        plugin/cFC4Protection.cpp
        plugin/cFC4Protection.h
        plugin/FC4ProtectionShadow.cpp
        plugin/FC4ProtectionShadow.h
        plugin/cFC4Crime.cpp
        plugin/cFC4Crime.h
        plugin/FC4CrimeInterfaces.h
        plugin/FC4CrimeShadow.cpp
        plugin/FC4CrimeShadow.h
        plugin/cFC4Aura.cpp
        plugin/cFC4Aura.h
        plugin/FC4AuraShadow.cpp
        plugin/FC4AuraShadow.h
        plugin/cFC4Flora.cpp
        plugin/cFC4Flora.h
        plugin/FC4FloraShadow.cpp
        plugin/FC4FloraShadow.h
        plugin/cFC4Landfill.cpp
        plugin/cFC4Landfill.h
        plugin/FC4LandfillShadow.cpp
        plugin/FC4LandfillShadow.h
        plugin/FC4TrafficShadow.cpp
        plugin/FC4TrafficShadow.h
        plugin/FC4WriteCapture.cpp
        plugin/FC4WriteCapture.h
        plugin/FC4LandValueShadow.cpp
        plugin/FC4LandValueShadow.h
        plugin/FC4FlammabilityShadow.cpp
        plugin/FC4FlammabilityShadow.h
        plugin/cFC4Residential.cpp
        plugin/cFC4Residential.h
        plugin/FC4ResidentialShadow.cpp
        plugin/FC4ResidentialShadow.h
        plugin/cFC4ZoneManager.cpp
        plugin/cFC4ZoneManager.h
        plugin/FC4ZonesShadow.cpp
        plugin/FC4ZonesShadow.h
        plugin/FC4GameSet.h
        plugin/FC4PropertyHolder.cpp
        plugin/FC4PropertyHolder.h
        plugin/cFC4Neighbors.cpp
        plugin/cFC4Neighbors.h
        plugin/FC4NeighborsShadow.cpp
        plugin/FC4NeighborsShadow.h
        plugin/FC4GrowthProbe.cpp
        plugin/FC4GrowthProbe.h
        plugin/FC4GrowthProbeHook.cpp
        plugin/FC4GrowthProbeHook.h
        plugin/FC4GrowthProbeLogic.h
        plugin/cFC4Development.cpp
        plugin/cFC4Development.h
        plugin/cFC4GrowthDeveloper.cpp
        plugin/cFC4GrowthDeveloper.h
        plugin/FC4DevelopmentShadow.cpp
        plugin/FC4DevelopmentShadow.h
        plugin/FC4DevelopmentInterfaces.h
        plugin/FC4GameMapBuild.h
        # The traffic simulator, its network model and its pathfinders (M5 glue).
        plugin/cFC4TrafficSimulator.cpp
        plugin/cFC4TrafficSimulator.h
        # The history warehouse and the 24-hour clock, their shared lock and
        # test hooks, and their shadows (docs/specs/history.md, clock24.md).
        plugin/FC4Clock24Shadow.cpp
        plugin/FC4Clock24Shadow.h
        plugin/FC4HistClock.cpp
        plugin/FC4HistClock.h
        plugin/FC4HistoryShadow.cpp
        plugin/FC4HistoryShadow.h
        plugin/cFC4Clock24.cpp
        plugin/cFC4Clock24.h
        plugin/cFC4History.cpp
        plugin/cFC4History.h
        # The tract developer (tract-developer.md): `enable = tract`, tract_shadow.
        plugin/cFC4TractDeveloper.cpp
        plugin/cFC4TractDeveloper.h
        plugin/FC4TractHost.cpp
        plugin/FC4TractInterfaces.h
        plugin/FC4TractShadow.cpp
        plugin/FC4TractShadow.h
        # The lot developer (lot-developer.md): `enable = lotdev`, lotdev_shadow.
        plugin/cFC4LotDeveloper.cpp
        plugin/cFC4LotDeveloper.h
        plugin/FC4LotDevHost.cpp
        plugin/FC4LotDevInterfaces.h
        plugin/FC4LotDevShadow.cpp
        plugin/FC4LotDevShadow.h
        # The commercial / industrial simulators, the civic building simulator and the
        # park manager (cmglue): `enable = commercial, industrial, civic, parks`, their shadows.
        plugin/cFC4Commercial.cpp
        plugin/cFC4Commercial.h
        plugin/FC4CommercialShadow.cpp
        plugin/FC4CommercialShadow.h
        plugin/cFC4Civic.cpp
        plugin/cFC4Civic.h
        plugin/FC4CivicShadow.cpp
        plugin/FC4CivicShadow.h
        plugin/cFC4Parks.cpp
        plugin/cFC4Parks.h
        plugin/FC4ParksShadow.cpp
        plugin/FC4ParksShadow.h
        # The terrain shadow (terrain.md; the glue16 lane): terrain_shadow.
        plugin/FC4TerrainShadow.cpp
        plugin/FC4TerrainShadow.h
        # The disaster layer's host (disasters.md; glue13) and its shadow: disasters_shadow.
        plugin/FC4DisasterHost.cpp
        plugin/FC4DisasterHost.h
        plugin/FC4DisasterShadow.cpp
        plugin/FC4DisasterShadow.h
        # The glue17 lane: the city situation manager, the demolition utility
        # and the dispatch manager (city-situation.md, demolition.md,
        # dispatch.md) and their shadows: citysit_shadow, demolition_shadow,
        # dispatch_shadow. FC4CoreShadow is the record round trip the three
        # shadows share.
        plugin/FC4CoreShadow.cpp
        plugin/FC4CoreShadow.h
        plugin/FC4CitySitShadow.cpp
        plugin/FC4CitySitShadow.h
        plugin/FC4DemolitionShadow.cpp
        plugin/FC4DemolitionShadow.h
        plugin/FC4DispatchShadow.cpp
        plugin/FC4DispatchShadow.h
        # The dispatch manager's GZCOM class (enable = dispatch).
        plugin/cFC4Dispatch.cpp
        plugin/cFC4Dispatch.h
        # The zone developer / lot manager (zones.md §1.2, §7-§11.3): enable = zonedev,
        # zonedev_shadow.
        plugin/cFC4ZoneDeveloper.cpp
        plugin/cFC4ZoneDeveloper.h
        plugin/FC4ZoneDevHost.cpp
        plugin/FC4ZoneDevShadow.cpp
        plugin/FC4ZoneDevShadow.h
        # The harness lane's tooling: sim_turbo, autotest_end_date, script, close_dialogs.
        plugin/FC4HarnessLogic.h
        plugin/FC4Turbo.cpp
        plugin/FC4Turbo.h
        plugin/FC4Script.cpp
        plugin/FC4Script.h
        plugin/FC4Dialogs.cpp
        plugin/FC4Dialogs.h
        plugin/FC4WinInterfaces.h
        # gzcom-dll SDK sources (LGPL-2.1-or-later). Not the SDK's
        # cRZMessage2Standard: foundations §5.1 says it differs from the game.
        "${FC4_GZCOM_DIR}/src/cRZCOMDllDirector.cpp"
        "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp"
        "${FC4_GZCOM_DIR}/src/SC4String.cpp"
        "${FC4_GZCOM_DIR}/src/cRZBaseVariant.cpp"
        "${FC4_GZCOM_DIR}/src/StringResourceManager.cpp")
    target_include_directories(FreeCity4 PRIVATE "${FC4_GZCOM_DIR}/include" plugin)
    target_compile_definitions(FreeCity4 PRIVATE
        _CRT_SECURE_NO_WARNINGS NOMINMAX WIN32_LEAN_AND_MEAN
        FC4_VERSION_STRING="${PROJECT_VERSION}")
    target_compile_options(FreeCity4 PRIVATE /W3 /permissive- /utf-8 /EHsc)
    if(FC4_CHECKED)
        target_compile_definitions(FreeCity4 PRIVATE FC4_CHECKED=1)
        target_compile_options(FreeCity4 PRIVATE /Od /Oy- /RTC1)
    endif()
    target_link_libraries(FreeCity4 PRIVATE fc4_capi)
    set_target_properties(FreeCity4 PROPERTIES OUTPUT_NAME "FreeCity4" PREFIX "")
    # Exports: GZDllGetGZCOMDirector (from cRZCOMDllDirector.cpp) and the
    # test seam FC4TestSeam (plugin/FC4TestSeam.h; never called by the game).

    # foundations §7 item 12: the plugin's standard message class.
    add_executable(fc4_message_test
        tests/message_test.cpp
        plugin/cFC4Message2Standard.cpp
        "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp")
    target_include_directories(fc4_message_test PRIVATE "${FC4_GZCOM_DIR}/include" plugin)
    target_compile_definitions(fc4_message_test PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX)
    target_compile_options(fc4_message_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME message_test COMMAND fc4_message_test)

    # Single ownership of elements pushed into game containers
    # (plugin/FC4GameContainers.h; in-game run 8 heap corruption).
    add_executable(fc4_game_container_test
        tests/game_container_test.cpp
        "${FC4_GZCOM_DIR}/src/SC4String.cpp"
        "${FC4_GZCOM_DIR}/src/cRZCOMDllDirector.cpp"
        "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp")
    target_include_directories(fc4_game_container_test PRIVATE "${FC4_GZCOM_DIR}/include" plugin)
    target_compile_definitions(fc4_game_container_test PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX)
    target_compile_options(fc4_game_container_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME game_container_test COMMAND fc4_game_container_test)

    # The game's STLport map (plugin/FC4GameMap.h): the read-only walk, the
    # building profile capacity maps and the GetLocalPopulationSummary fill,
    # against fake maps built as docs/abi/allocators.md describes them.
    add_executable(fc4_game_map_test tests/game_map_test.cpp)
    target_include_directories(fc4_game_map_test PRIVATE plugin)
    target_compile_options(fc4_game_map_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME game_map_test COMMAND fc4_game_map_test)

    # The game's STLport set (plugin/FC4GameSet.h): PlaceZone's outLots unique
    # insert against sets built and checked as docs/abi/allocators.md describes.
    add_executable(fc4_game_set_test tests/game_set_test.cpp)
    target_include_directories(fc4_game_set_test PRIVATE plugin)
    target_compile_options(fc4_game_set_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME game_set_test COMMAND fc4_game_set_test)

    # The master lot list through the safe lot iterator
    # (plugin/FC4SafeLotIterator.h, budget.md §5.2 BQ3) against fake lists
    # built as aura.md §6.5 and docs/abi/allocators.md describe them.
    add_executable(fc4_safe_lot_iterator_test tests/safe_lot_iterator_test.cpp)
    target_include_directories(fc4_safe_lot_iterator_test PRIVATE plugin)
    target_compile_options(fc4_safe_lot_iterator_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME safe_lot_iterator_test COMMAND fc4_safe_lot_iterator_test)

    # foundations §16: the slot layout of every game interface we implement.
    add_executable(fc4_vtable_layout_test tests/vtable_layout_test.cpp)
    target_include_directories(fc4_vtable_layout_test PRIVATE "${FC4_GZCOM_DIR}/include" plugin tests)
    target_compile_definitions(fc4_vtable_layout_test PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX)
    target_compile_options(fc4_vtable_layout_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME vtable_layout_test COMMAND fc4_vtable_layout_test)

    # The watchdog's stall arithmetic and the autotest verdict rules.
    add_executable(fc4_diag_logic_test tests/diag_logic_test.cpp)
    target_include_directories(fc4_diag_logic_test PRIVATE plugin)
    target_compile_options(fc4_diag_logic_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME diag_logic_test COMMAND fc4_diag_logic_test)

    # The x87 precision re-assert (foundations §18.2): the game helper's mode
    # arithmetic and how to report a re-assert.
    add_executable(fc4_fpu_logic_test tests/fpu_logic_test.cpp)
    target_include_directories(fc4_fpu_logic_test PRIVATE plugin)
    target_compile_options(fc4_fpu_logic_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME fpu_logic_test COMMAND fc4_fpu_logic_test)

    # The growth probe (building-development.md §12): the counting hook on a
    # fake vtable in this process (forwarding, count, result, callee-pop,
    # registers and x87 state, restore) and its day and summary lines.
    add_executable(fc4_growth_probe_test tests/growth_probe_test.cpp plugin/FC4GrowthProbeHook.cpp)
    target_include_directories(fc4_growth_probe_test PRIVATE plugin)
    target_compile_definitions(fc4_growth_probe_test PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX)
    target_compile_options(fc4_growth_probe_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME growth_probe_test COMMAND fc4_growth_probe_test)

    # Every interface we implement or call against docs/abi/interfaces.tsv
    # (callee-pop bytes, return channel, slot count), read at run time.
    set(FC4_ABI_TSV "${CMAKE_CURRENT_SOURCE_DIR}/../docs/abi/interfaces.tsv")
    add_executable(fc4_abi_conformance_test tests/abi_conformance_test.cpp tests/abi_entries.inc)
    target_include_directories(fc4_abi_conformance_test PRIVATE "${FC4_GZCOM_DIR}/include" plugin tests)
    target_compile_definitions(fc4_abi_conformance_test PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX
        FC4_ABI_TSV="${FC4_ABI_TSV}" FC4_PLUGIN_DIR="${CMAKE_CURRENT_SOURCE_DIR}/plugin")
    target_compile_options(fc4_abi_conformance_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME abi_conformance_test COMMAND fc4_abi_conformance_test
        --plugin-dir "${CMAKE_CURRENT_SOURCE_DIR}/plugin" "${FC4_ABI_TSV}")

    # Every use the plugin makes of a game method's return value, against the
    # slot's `ret` channel in docs/abi and what the specs say it returns
    # (tests/check_ret_channels.py). Skipped when no Python 3 is found.
    find_package(Python3 COMPONENTS Interpreter QUIET)
    if(Python3_Interpreter_FOUND)
        add_test(NAME ret_channel_check
            COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/check_ret_channels.py")
    endif()

    # Loads the built FreeCity4.dll like a GZCOM host and checks the diagnostics log.
    add_executable(fc4_plugin_load_test tests/plugin_load_test.cpp plugin/cFC4Variant.cpp
        "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp"
        "${FC4_GZCOM_DIR}/src/cRZBaseVariant.cpp" "${FC4_GZCOM_DIR}/src/cSCBaseProperty.cpp")
    target_include_directories(fc4_plugin_load_test PRIVATE "${FC4_GZCOM_DIR}/include" plugin capi/include)
    target_compile_options(fc4_plugin_load_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_dependencies(fc4_plugin_load_test FreeCity4)
    if(FC4_CHECKED)
        set(FC4_LOAD_TEST_MODE --checked)
    else()
        set(FC4_LOAD_TEST_MODE --normal)
    endif()
    add_test(NAME plugin_load_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" ${FC4_LOAD_TEST_MODE})
    add_test(NAME plugin_disable_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --disabled)
    # FreeCity4.ini `disable =` wins over FC4_DISABLE.
    add_test(NAME plugin_ini_disable_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-disabled)
    # Default-off subsystems (land value): `enable =` turns them on, `disable =` still wins.
    add_test(NAME plugin_ini_enable_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable)
    add_test(NAME plugin_ini_enable_disabled_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-disabled)
    add_test(NAME plugin_ini_enable_weather_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-weather)
    add_test(NAME plugin_ini_enable_ordinance_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-ordinance)
    add_test(NAME plugin_ini_enable_demand_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-demand)
    add_test(NAME plugin_ini_enable_plumbing_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-plumbing)
    add_test(NAME plugin_ini_enable_pollution_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-pollution)
    add_test(NAME plugin_ini_enable_crime_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-crime)
    # Autotest: child processes that end with the verdict and TerminateProcess.
    add_test(NAME plugin_autotest_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --autotest)
    # All of them write FreeCity4.log next to the DLL.
    set_tests_properties(plugin_load_test plugin_disable_test plugin_ini_disable_test plugin_ini_enable_test
        plugin_ini_enable_disabled_test plugin_ini_enable_weather_test plugin_ini_enable_ordinance_test
        plugin_ini_enable_demand_test plugin_ini_enable_plumbing_test plugin_autotest_test
        plugin_ini_enable_pollution_test plugin_ini_enable_crime_test
        PROPERTIES RUN_SERIAL TRUE)
    # popup_dialogs (FC4Popups.h): child processes, one FreeCity4.ini each.
    add_test(NAME plugin_ini_popups_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-popups)
    set_tests_properties(plugin_ini_popups_test PROPERTIES RUN_SERIAL TRUE)
    # Protection (protection.md): `enable = protection` (no city: neutral
    # getters, lifecycle, status lines) and the grid shadow on fake originals.
    add_test(NAME plugin_ini_enable_protection_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-protection)
    add_test(NAME plugin_ini_protection_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-protection-shadow)
    set_tests_properties(plugin_ini_enable_protection_test plugin_ini_protection_shadow_test
        PROPERTIES RUN_SERIAL TRUE)
    # Aura (aura.md): `enable = aura` (no city: neutral getters, lifecycle,
    # status lines) and the aura shadow following a fake original step by step
    # (seam ops 14 and 35).
    add_test(NAME plugin_ini_enable_aura_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-aura)
    set_tests_properties(plugin_ini_enable_aura_test PROPERTIES RUN_SERIAL TRUE)
    # Flora (flora.md): `enable = flora` (no city: the registry's key-0 record,
    # neutral calls, lifecycle, status lines) and the census shadow on a fake
    # original (seam op 17).
    add_test(NAME plugin_ini_enable_flora_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-flora)
    set_tests_properties(plugin_ini_enable_flora_test PROPERTIES RUN_SERIAL TRUE)
    # Landfill (landfill.md): `enable = landfill` (no city: neutral getters,
    # regions, lifecycle, the record, threads, status lines) and the landfill
    # shadow on fake originals (seam op 18).
    add_test(NAME plugin_ini_enable_landfill_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-landfill)
    set_tests_properties(plugin_ini_enable_landfill_test PROPERTIES RUN_SERIAL TRUE)
    # The land value and flammability grid shadows on fake originals (seam
    # ops 23, 24), and both requested but inactive with our simulators on.
    add_test(NAME plugin_ini_landvalue_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-landvalue-shadow)
    add_test(NAME plugin_ini_flammability_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-flammability-shadow)
    add_test(NAME plugin_ini_lvfl_shadow_inactive_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-lvfl-shadow-inactive)
    set_tests_properties(plugin_ini_landvalue_shadow_test plugin_ini_flammability_shadow_test
        plugin_ini_lvfl_shadow_inactive_test PROPERTIES RUN_SERIAL TRUE)
    # Residential (residential.md, residential-education-health.md): `enable =
    # residential` (no city: neutral getters, AL returns, lifecycle, the record,
    # status lines) and the residential shadow on fake originals (seam op 25).
    add_test(NAME plugin_ini_enable_residential_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-residential)
    set_tests_properties(plugin_ini_enable_residential_test PROPERTIES RUN_SERIAL TRUE)
    # Development (building-development.md, growth-developer.md): `enable =
    # development` (no city: neutral slots, AL returns, lifecycle, the record,
    # threads, status lines).
    add_test(NAME plugin_ini_enable_development_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-development)
    set_tests_properties(plugin_ini_enable_development_test PROPERTIES RUN_SERIAL TRUE)
    # The development shadow (FC4DevelopmentShadow.h): ON with the original
    # unit, its list comparison and record bridge (seam op 34).
    add_test(NAME plugin_ini_development_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-development-shadow)
    set_tests_properties(plugin_ini_development_shadow_test PROPERTIES RUN_SERIAL TRUE)
    # Zones (zones.md): `enable = zones` (no city: the QI table, neutral getters,
    # the slots' return channels, lifecycle, the record round trip, status
    # lines) and, through seam op 38 with stub hosts, PlaceZone (reject, abort,
    # accept, outLots), ZoningChange, the dezone handler and the zones shadow
    # (zones_shadow = 1: INACTIVE with `enable = zones`; runs on a fake original).
    add_test(NAME plugin_ini_enable_zones_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-zones)
    set_tests_properties(plugin_ini_enable_zones_test PROPERTIES RUN_SERIAL TRUE)
    # The zone developer / lot manager (zones.md §1-§11): `enable = zonedev`
    # (no city: the QI table and sub-object offsets, neutral getters, the
    # record round trip, status lines) and, through seam op 53 with stub hosts
    # and malloc'd B / C blocks, Init / Shutdown, STAGE A's refused lot
    # creation, stub lots in the lists (slots 7, 10), the safe lot iterator's
    # raw layout (slot 15), DeleteLot and the zonedev shadow on a fake original
    # (zonedev_shadow = 1: INACTIVE with `enable = zonedev`).
    add_test(NAME plugin_ini_enable_zonedev_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-zonedev)
    set_tests_properties(plugin_ini_enable_zonedev_test PROPERTIES RUN_SERIAL TRUE)
    # The tract developer (tract-developer.md): `enable = tract` (no city: the
    # QI table, the slots' return channels, lifecycle, the record round trip,
    # status lines) and, through seam op 45 with stub hosts, Init / Shutdown
    # with the filter registered and unregistered, Grow on a stub tract, and
    # the tract shadow (tract_shadow = 1: INACTIVE with `enable = tract`; runs
    # on a fake original).
    add_test(NAME plugin_ini_enable_tract_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-tract)
    set_tests_properties(plugin_ini_enable_tract_test PROPERTIES RUN_SERIAL TRUE)
    # The lot developer (lot-developer.md): `enable = lotdev` (no city: the QI
    # table, Release's 0, the record round trip, lifecycle without services)
    # and, through seam op 52 with stub hosts, Init's registrations and the
    # lot developer shadow (lotdev_shadow = 1: INACTIVE with `enable = lotdev`).
    add_test(NAME plugin_ini_enable_lotdev_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-lotdev)
    set_tests_properties(plugin_ini_enable_lotdev_test PROPERTIES RUN_SERIAL TRUE)
    # The cmglue lane (tests/plugin_cmglue_test.inc): `enable = commercial,
    # industrial` (no city: the QI tables, slot 5, the records' round trips,
    # Init without services, DoMessage).
    add_test(NAME plugin_ini_enable_commercial_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-commercial)
    set_tests_properties(plugin_ini_enable_commercial_test PROPERTIES RUN_SERIAL TRUE)
    # `enable = civic` (no city: the QI table, slots 5-8, the record with the
    # mod-193 order, DoMessage).
    add_test(NAME plugin_ini_enable_civic_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-civic)
    set_tests_properties(plugin_ini_enable_civic_test PROPERTIES RUN_SERIAL TRUE)
    # `enable = parks`: the park manager (parks.md): QI table, record, Init
    # without a city, a fake zoo's escape (the ZooEscape message, §4.5).
    add_test(NAME plugin_ini_enable_parks_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-parks)
    set_tests_properties(plugin_ini_enable_parks_test PROPERTIES RUN_SERIAL TRUE)
    # `terrain_shadow = 1`: the terrain shadow (glue16) on a stand-in original
    # built inside the DLL (seam op 58): exact, lost messages, perturbations.
    add_test(NAME plugin_ini_terrain_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-terrain-shadow)
    set_tests_properties(plugin_ini_terrain_shadow_test PROPERTIES RUN_SERIAL TRUE)
    # `enable = neighbors`: the neighbours simulator, its connections (an
    # original's accepted on load) and deals, through a fake city and region.
    add_test(NAME plugin_ini_enable_neighbors_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-neighbors)
    set_tests_properties(plugin_ini_enable_neighbors_test PROPERTIES RUN_SERIAL TRUE)
    # Traffic (traffic-simulator.md, traffic-network.md, pathfinder.md): `enable =
    # traffic` (no city: the QI table, the x87 slots, the record round trip,
    # status lines) and, through seam op 42 with a stub host, the slot-6
    # pathfinders (FindPath, the compressed route, the caller's route deque).
    add_test(NAME plugin_ini_enable_traffic_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-traffic)
    set_tests_properties(plugin_ini_enable_traffic_test PROPERTIES RUN_SERIAL TRUE)
    # The history warehouse (docs/specs/history.md): off by default; `enable =
    # history` registers it (QueryInterface quirk, slots, record, the record
    # message reaching the core through the test hooks of op 54).
    add_test(NAME plugin_ini_enable_history_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-history)
    set_tests_properties(plugin_ini_enable_history_test PROPERTIES RUN_SERIAL TRUE)
    # The history shadow (FC4HistoryShadow.h): inactive with ours; through
    # seam op 54 op 1, on FreeCity4's warehouse as the "original": the load
    # round trip, messages, the pass steps, the month comparison exact, then a
    # record the shadow did not see found.
    add_test(NAME plugin_ini_history_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-history-shadow)
    set_tests_properties(plugin_ini_history_shadow_test PROPERTIES RUN_SERIAL TRUE)
    # The 24-hour clock (docs/specs/clock24.md): off by default; `enable =
    # clock24` registers it (the QI table with the +0xC system service, the
    # game-form slots, the framework and cheat registration in order, the
    # step, the cheats, the 26-byte record and versions 1 / 3, threads).
    add_test(NAME plugin_ini_enable_clock24_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-enable-clock24)
    set_tests_properties(plugin_ini_enable_clock24_test PROPERTIES RUN_SERIAL TRUE)
    # The 24-hour clock shadow (FC4Clock24Shadow.h): inactive with ours;
    # through seam op 54 op 2, on FreeCity4's clock as the "original": the
    # load, mirrored steps and messages exact, then a magnification and a
    # missing message found.
    add_test(NAME plugin_ini_clock24_shadow_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-clock24-shadow)
    set_tests_properties(plugin_ini_clock24_shadow_test PROPERTIES RUN_SERIAL TRUE)
    # References read from a save (in-game run 88): land value grids and a
    # foreign ordinance, with and without a save before Shutdown, and land value
    # grids created through GZCOM; no reference warning, each released once.
    add_test(NAME plugin_saved_refs_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --saved-refs)
    set_tests_properties(plugin_saved_refs_test PROPERTIES RUN_SERIAL TRUE)
    # The growth calibration probe (building-development.md §12, seam op 39):
    # `growth_probe = 1` on a fake tract developer vtable and a fake city (day
    # lines, cut, summary, restore), and off by default.
    add_test(NAME plugin_ini_growth_probe_test
        COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-growth-probe)
    set_tests_properties(plugin_ini_growth_probe_test PROPERTIES RUN_SERIAL TRUE)
    # The harness lane (tooling): the pure logic (dates, rates, the script
    # parser and scheduler), and sim_turbo on our simulator (FC4Turbo.h): off by
    # default, the WARNING with `disable = simulator`, bad values, until a date.
    add_executable(fc4_harness_logic_test tests/harness_logic_test.cpp)
    target_include_directories(fc4_harness_logic_test PRIVATE plugin)
    target_compile_options(fc4_harness_logic_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME harness_logic_test COMMAND fc4_harness_logic_test)
    add_test(NAME plugin_ini_turbo_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-turbo)
    set_tests_properties(plugin_ini_turbo_test PROPERTIES RUN_SERIAL TRUE)
    # script = <path> (FC4Script.h): dated actions through a stub zone manager
    # (the region layout, the arguments, the bits it clears) and our simulator;
    # a refused script and an unreadable one (test seam op 46).
    add_test(NAME plugin_ini_script_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-script)
    set_tests_properties(plugin_ini_script_test PROPERTIES RUN_SERIAL TRUE)
    # close_dialogs (FC4Dialogs.h): a fake window manager and legacy message
    # server (test seam op 47): the button, the EndModal fallback, a stale
    # window, nested dialogs, SC4GameDelay, a late subscription; off: nothing.
    add_test(NAME plugin_ini_dialogs_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --ini-dialogs)
    set_tests_properties(plugin_ini_dialogs_test PROPERTIES RUN_SERIAL TRUE)
else()
    message(STATUS "FreeCity4.dll is only built with MSVC for 32-bit x86; skipping the plugin target.")
endif()
