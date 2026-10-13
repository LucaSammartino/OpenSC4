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
        plugin/FC4Fpu.h
        plugin/FC4FpuLogic.h
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
        plugin/cFC4Flammability.cpp
        plugin/cFC4Flammability.h
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

    # Loads the built FreeCity4.dll like a GZCOM host and checks the diagnostics log.
    add_executable(fc4_plugin_load_test tests/plugin_load_test.cpp "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp"
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
    # Autotest: child processes that end with the verdict and TerminateProcess.
    add_test(NAME plugin_autotest_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>" --autotest)
    # All of them write FreeCity4.log next to the DLL.
    set_tests_properties(plugin_load_test plugin_disable_test plugin_ini_disable_test plugin_ini_enable_test
        plugin_ini_enable_disabled_test plugin_ini_enable_weather_test plugin_ini_enable_ordinance_test
        plugin_ini_enable_demand_test plugin_ini_enable_plumbing_test plugin_autotest_test
        PROPERTIES RUN_SERIAL TRUE)
else()
    message(STATUS "FreeCity4.dll is only built with MSVC for 32-bit x86; skipping the plugin target.")
endif()
