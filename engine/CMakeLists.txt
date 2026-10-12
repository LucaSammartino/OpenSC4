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
        # gzcom-dll SDK sources (LGPL-2.1-or-later). Not the SDK's
        # cRZMessage2Standard: foundations §5.1 says it differs from the game.
        "${FC4_GZCOM_DIR}/src/cRZCOMDllDirector.cpp"
        "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp"
        "${FC4_GZCOM_DIR}/src/SC4String.cpp")
    target_include_directories(FreeCity4 PRIVATE "${FC4_GZCOM_DIR}/include" plugin)
    target_compile_definitions(FreeCity4 PRIVATE
        _CRT_SECURE_NO_WARNINGS NOMINMAX WIN32_LEAN_AND_MEAN
        FC4_VERSION_STRING="${PROJECT_VERSION}")
    target_compile_options(FreeCity4 PRIVATE /W3 /permissive- /utf-8 /EHsc)
    target_link_libraries(FreeCity4 PRIVATE fc4_capi)
    set_target_properties(FreeCity4 PROPERTIES OUTPUT_NAME "FreeCity4" PREFIX "")
    # The only export is GZDllGetGZCOMDirector (from cRZCOMDllDirector.cpp).

    # foundations §7 item 12: the plugin's standard message class.
    add_executable(fc4_message_test
        tests/message_test.cpp
        plugin/cFC4Message2Standard.cpp
        "${FC4_GZCOM_DIR}/src/cRZBaseString.cpp")
    target_include_directories(fc4_message_test PRIVATE "${FC4_GZCOM_DIR}/include" plugin)
    target_compile_definitions(fc4_message_test PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX)
    target_compile_options(fc4_message_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_test(NAME message_test COMMAND fc4_message_test)

    # Loads the built FreeCity4.dll like a GZCOM host and checks the diagnostics log.
    add_executable(fc4_plugin_load_test tests/plugin_load_test.cpp)
    target_include_directories(fc4_plugin_load_test PRIVATE "${FC4_GZCOM_DIR}/include")
    target_compile_options(fc4_plugin_load_test PRIVATE /W3 /permissive- /utf-8 /EHsc)
    add_dependencies(fc4_plugin_load_test FreeCity4)
    add_test(NAME plugin_load_test COMMAND fc4_plugin_load_test "$<TARGET_FILE:FreeCity4>")
else()
    message(STATUS "FreeCity4.dll is only built with MSVC for 32-bit x86; skipping the plugin target.")
endif()
