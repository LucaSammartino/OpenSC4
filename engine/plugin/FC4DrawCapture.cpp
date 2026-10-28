// SPDX-License-Identifier: GPL-3.0-or-later
// An observation tool, not a replacement driver. No on-disk executable writes.
#include "FC4DrawCapture.h"
#include "FC4DrawCaptureLogic.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4WinInterfaces.h"
#include "compat/FC4Compat.h"
#include "cIGZFrameWork.h"
#include "cIGZApp.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Simulator.h"
#include "cISC4View3DWin.h"
#include "cISC43DRender.h"
#include "cIGZWin.h"
#include "cIGZString.h"
#include "cRZCOMDllDirector.h"
#include <windows.h>
#include <algorithm>
#include <array>
#include <cstdio>
#include <map>
#include <mutex>
#include <string>
#include <utility>

namespace fc4 { namespace {
using namespace drawcapture;
constexpr uint64_t MaxBuffer = 64ull * 1024 * 1024;
void* originals[3][83]{};
const char* drivers[] = {"dx7", "opengl", "software"};
std::recursive_mutex captureMutex;
struct Arrays { uint32_t base = 0, stride = 0, bytes = 0, rowLength = 0; };
std::map<void*, Arrays> arrays;
FILE* stream = nullptr;
bool enabled = false, active = false, finished = false, failed = false, sweep = false;
unsigned frame = 0, call = 0, scenario = 0, inScenario = 0, warmup = 0;
unsigned framesPerScenario = 0, warmupFrames = 0, scenarioCount = 0, waitTicks = 0;
int zoom = 0, rotation = 0;
uint64_t calls = 0, draws = 0, uiFrames = 0, incomplete = 0, frameDraws = 0;
DWORD mainThread = 0;
thread_local bool inspecting = false;
std::string Number(uint64_t n) { return std::to_string(n); }
void Write(const std::string& text) {
    if (!stream || failed) return;
    if (std::fwrite(text.data(), 1, text.size(), stream) != text.size() || std::fputc('\n', stream) == EOF)
        failed = true;
}
// Leaf SEH helpers: an invalid or unbounded buffer refuses completeness instead
// of crashing the game or comparing two truncated hashes as equal.
bool SafeHash(uintptr_t base, uint64_t row, uint64_t stride, uint64_t rows, uint64_t* hash) {
    if (row > MaxBuffer || rows > MaxBuffer || (rows && row > MaxBuffer / rows)) return false;
    if (rows && row && (!base || base > UINT32_MAX - row ||
        (rows - 1) * stride > UINT32_MAX - base - row)) return false;
    __try {
        uint64_t h = HashStart;
        for (uint64_t i = 0; i < rows; ++i)
            h = Hash(h, reinterpret_cast<const void*>(base + i * stride), static_cast<size_t>(row));
        *hash = h;
        return true;
    } __except(EXCEPTION_EXECUTE_HANDLER) { return false; }
}
bool IndexRange(uint32_t address, uint32_t count, unsigned size, uint32_t* first, uint32_t* last) {
    if (uint64_t(count) * size > MaxBuffer || !address ||
        uint64_t(address) + uint64_t(count) * size > UINT32_MAX) return false;
    __try {
        uint32_t lo = UINT32_MAX, hi = 0;
        for (uint32_t i = 0; i < count; ++i) {
            const uintptr_t p = uintptr_t(address) + i * size;
            // DX7 truncates 32-bit indices to 16 bits (renderer-driver 8.2).
            const uint32_t v = size == 1 ? *reinterpret_cast<const uint8_t*>(p) :
                size == 2 ? *reinterpret_cast<const uint16_t*>(p) :
                (*reinterpret_cast<const uint32_t*>(p) & 0xffff);
            lo = (std::min)(lo, v); hi = (std::max)(hi, v);
        }
        *first = lo; *last = hi; return true;
    } __except(EXCEPTION_EXECUTE_HANDLER) { return false; }
}
struct Buffers {
    std::string json;
    void Add(const char* name, uint32_t address, uint64_t row, uint64_t stride = 0,
             uint64_t rows = 1, const char* status = nullptr) {
        if (!json.empty()) json += ',';
        json += '"'; json += name; json += "\":{\"bytes\":" + Number(row * rows);
        uint64_t h = HashStart;
        const bool ok = status || SafeHash(address, row, stride ? stride : row, rows, &h);
        if (!ok) { status = "unreadable-or-unbounded"; ++incomplete; }
        if (status) { json += ",\"status\":\""; json += status; json += '"'; }
        else {
            char hex[20]; std::snprintf(hex, sizeof(hex), "%016llx", h);
            json += ",\"hash\":\""; json += hex; json += '"';
        }
        json += '}';
    }
    void Unknown(const char* name, uint32_t address) {
        ++incomplete; Add(name, address, 0, 0, 1, "unknown-extent");
    }
};
// Output-only pointers have no input contents. In particular, never hash the
// uninitialised destinations of GetBoolean/GetFloat (DX7 no-ops, spec 12.2).
void Record(unsigned driver, unsigned slot, void* self, uint32_t* a) {
    auto& state = arrays[self];
    if (slot == 5) {
        state.bytes = VertexBytes(a[0]); state.stride = a[1] ? a[1] : state.bytes; state.base = a[2];
    }
    if (slot == 26 && a[0] == 0) state.rowLength = a[1];
    if (!active || inspecting) return;
    Buffers b;
    auto input = [&](unsigned i, uint64_t bytes, const char* name) {
        const auto pointer = a[i]; b.Add(name, pointer, bytes); a[i] = !!pointer;
    };
    auto output = [&](unsigned i, uint64_t bytes, const char* name) {
        const auto pointer = a[i]; b.Add(name, pointer, bytes, 0, 1, "output"); a[i] = !!pointer;
    };
    auto vertices = [&](uint32_t first, uint32_t count) {
        const uint64_t start = uint64_t(state.base) + uint64_t(first) * state.stride;
        if ((count && (!state.bytes || !state.base)) || start > UINT32_MAX)
            b.Unknown("vertices", state.base);
        else b.Add("vertices", static_cast<uint32_t>(start), state.bytes, state.stride, count);
    };
    switch (slot) {
    case 0: output(1, 4, "interface"); break;
    case 3: ++draws; ++frameDraws; vertices(a[1], static_cast<int32_t>(a[2]) > 0 ? a[2] : 0); break;
    case 4: {
        ++draws; ++frameDraws;
        const uint32_t n = static_cast<int32_t>(a[1]) > 0 ? a[1] : 0;
        const unsigned size = a[2] == 1 ? 1 : a[2] == 3 ? 2 : a[2] == 5 ? 4 : 0;
        uint32_t first = 0, last = 0;
        if (!size) b.Unknown("indices", a[3]);
        else if (n && !IndexRange(a[3], n, size, &first, &last)) b.Unknown("vertices", state.base);
        else vertices(first, n ? last - first + 1 : 0);
        input(3, uint64_t(n) * size, "indices"); break;
    }
    case 5: b.Add("vertices", a[2], 0, 0, 1, "deferred-to-draw"); a[2] = !!a[2]; break;
    case 7: input(1, uint64_t(a[0]) * 4, "element_kinds"); break;
    case 25: case 61: {
        const bool tex = slot == 25;
        const unsigned w = tex ? 3 : 4, f = tex ? 6 : 6, p = tex ? 8 : 9;
        const auto pixels = PixelBytes(a[w], a[w+1], a[f], a[f+1], tex ? state.rowLength : a[8]);
        if (pixels.valid) b.Add("pixels", a[p], pixels.row, pixels.stride, pixels.rows);
        else b.Unknown("pixels", a[p]);
        a[p] = !!a[p]; break;
    }
    case 27: input(2, 16, "color"); break;
    case 30: input(1, a[0] == 1 ? 16 : (a[0] >= 2 && a[0] <= 4 ? 4 : 0), "fog"); break;
    case 35: output(1, uint64_t(a[0]) * 4, "textures"); break;
    case 36: input(1, uint64_t(a[0]) * 4, "textures"); break;
    case 38: input(1, uint64_t(a[0]) * 4, "textures"); input(2, uint64_t(a[0]) * 4, "priorities"); break;
    case 39: input(1, uint64_t(a[0]) * 4, "textures"); output(2, a[0], "residency"); break;
    case 41: input(0, 64, "matrix"); break;
    case 47: case 49: output(1, 0, "ignored_output"); break;
    case 48: output(1, 4, "integer"); break;
    case 53: input(0, a[0] ? 64 : 0, "matrix"); break;
    case 62: input(0, 10, "combiner"); break;
    case 64: output(0, 52, "mode"); break;
    case 65: output(1, 52, "mode"); break;
    case 66: a[1] = !!a[1]; break; // HWND identity is not a pixel input.
    case 68: case 69: case 70: case 71: case 72: case 73: {
        const unsigned p = (slot == 69 || slot == 71 || slot == 73) ? 8 : 6;
        // DX7 ignores these payloads (11); other implementations are outside
        // the buffer-extent proof. Refuse completeness if they are exercised.
        for (unsigned i : {p, p+2}) {
            if (driver == 0) b.Add(i == p ? "pixels" : "auxiliary", a[i], 0, 0, 1, "ignored-dx7");
            else b.Unknown(i == p ? "pixels" : "auxiliary", a[i]);
            a[i] = !!a[i];
        }
        break;
    }
    case 76: output(0, 16, "viewport"); break;
    case 82: b.Unknown("punt", a[1]); a[1] = !!a[1]; break;
    default: break;
    }
    std::string args;
    for (unsigned i = 0; i < Words[slot]; ++i) { if (i) args += ','; args += Number(a[i]); }
    Write("{\"type\":\"call\",\"frame\":" + Number(frame) + ",\"call\":" + Number(call++) +
          ",\"driver\":\"" + drivers[driver] + "\",\"slot\":" + Number(slot) +
          ",\"method\":\"" + Methods[slot] + "\",\"thread\":" +
          Number(GetCurrentThreadId() == mainThread ? 0 : 1) + ",\"args\":[" + args +
          "],\"buffers\":{" + b.json + "}}");
    ++calls;
}
template<size_t> using ArgWord = uint32_t;
template<unsigned Driver, unsigned Slot, size_t... I>
uint32_t __fastcall Tap(void* self, void*, ArgWord<I>... args) {
    try {
        std::lock_guard<std::recursive_mutex> lock(captureMutex);
        uint32_t words[] = {args..., 0};
        Record(Driver, Slot, self, words);
    } catch (...) { failed = true; }
    // Raw stack-word transport preserves float/double bits and EAX. All
    // documented driver returns are void, AL or EAX; no x87-valued result.
    using Fn = uint32_t(__fastcall*)(void*, void*, ArgWord<I>...);
    return reinterpret_cast<Fn>(originals[Driver][Slot])(self, nullptr, args...);
}
template<unsigned Driver, unsigned Slot, size_t... I>
void* Hook(std::index_sequence<I...>) { return reinterpret_cast<void*>(&Tap<Driver, Slot, I...>); }
template<unsigned Driver, size_t... Slot>
std::array<void*, 83> Hooks(std::index_sequence<Slot...>) {
    return {Hook<Driver, Slot>(std::make_index_sequence<Words[Slot]>{})...};
}

// UI getters run outside the driver transcript. Traversal order, parent index
// and child ordinal disambiguate duplicate window IDs without heap addresses.
unsigned uiNodes = 0;
bool UiNode(cIGZWin* win, int parent, unsigned depth);
struct Children { int parent; unsigned depth; };
bool UiChild(cIGZWin*, uint32_t, void* child, void* context) {
    const auto* c = static_cast<Children*>(context);
    return UiNode(static_cast<cIGZWin*>(child), c->parent, c->depth);
}
bool UiNode(cIGZWin* win, int parent, unsigned depth) {
    if (!win || depth > 64 || uiNodes >= 16384) { ++incomplete; return false; }
    const auto node = uiNodes++;
    int32_t rect[4]{};
    reinterpret_cast<cFC4IGZWin*>(win)->GetAreaInto(rect);
    auto* text = win->GetCaption();
    Buffers b;
    b.Add("text", text ? reinterpret_cast<uint32_t>(text->ToChar()) : 0, text ? text->Strlen() : 0);
    Write("{\"type\":\"ui\",\"frame\":" + Number(frame) + ",\"node\":" + Number(node) +
          ",\"parent\":" + std::to_string(parent) + ",\"id\":" + Number(win->GetID()) +
          ",\"rect\":[" + std::to_string(rect[0]) + ',' + std::to_string(rect[1]) + ',' +
          std::to_string(rect[2]) + ',' + std::to_string(rect[3]) + "],\"visible\":" +
          // [abi-ret] SDK cIGZWin slot 71 bool is observation-only; log its AL, never drive game state.
          Number(win->IsVisible()) + ",\"buffers\":{" + b.json + "}}");
    Children children{static_cast<int>(node), depth+1};
    reinterpret_cast<cFC4IGZWin*>(win)->EnumChildren(kFC4_IID_cIGZWin, reinterpret_cast<void*>(&UiChild), &children);
    return true;
}
cISC4App* App() {
    auto* fw = RZGetFrameWork(); auto* app = fw ? fw->Application() : nullptr;
    cISC4App* result = nullptr;
    if (app) app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&result));
    return result;
}
cISC4View3DWin* View(cIGZWin* root) {
    auto* main = reinterpret_cast<cFC4IGZWin*>(root);
    auto* shell = main ? main->GetChildWindowFromID(0x6104489A) : nullptr;
    cISC4View3DWin* view = nullptr;
    if (shell) shell->GetChildAs(0x9A47B417, 0xFA47B3F9, reinterpret_cast<void**>(&view));
    return view;
}
void EndFrame() {
    if (!active) return;
    Write("{\"type\":\"end_frame\",\"frame\":" + Number(frame) + ",\"calls\":" + Number(call) +
          ",\"draw_calls\":" + Number(frameDraws) + "}");
    if (!frameDraws) ++incomplete;
    active = false;
    ++frame; ++inScenario;
    std::fflush(stream);
}
void Finish() {
    Write("{\"type\":\"end\",\"frames\":" + Number(frame) + ",\"calls\":" + Number(calls) +
          ",\"draw_calls\":" + Number(draws) + ",\"ui_frames\":" + Number(uiFrames) +
          ",\"incomplete\":" + Number(incomplete) + ",\"complete\":" + (failed ? "false" : "true") + "}");
    if (stream) { if (std::fclose(stream) != 0) failed = true; stream = nullptr; }
    // Queues running an older launcher can preserve evidence before slot reuse.
    // The primary stream always remains beside the game's log.
    const auto exportPath = Setting("draw_capture_export", nullptr);
    if (!failed && !exportPath.empty()) {
        const auto source = std::string(LogPath()) + ".draw.jsonl";
        if (!CopyFileA(source.c_str(), exportPath.c_str(), FALSE)) {
            failed = true;
            Log("draw capture: export failed error=%lu path=%s", GetLastError(), exportPath.c_str());
        }
    }
    finished = true;
    Log("draw capture: finished frames=%u calls=%llu draws=%llu ui_frames=%llu incomplete=%llu io_failed=%u",
        frame, calls, draws, uiFrames, incomplete, failed);
}
} // namespace

void DrawCaptureStartup() {
    if (enabled || !SettingInt("draw_capture", nullptr, 0)) return;
    void** tables[3]{};
    if (!compat::DrawCaptureTables(tables)) { Log("draw capture: refused: unidentified driver tables"); return; }
    const auto dx = Hooks<0>(std::make_index_sequence<83>{});
    const auto gl = Hooks<1>(std::make_index_sequence<83>{});
    const auto sw = Hooks<2>(std::make_index_sequence<83>{});
    const std::array<void*,83>* hooks[] = {&dx, &gl, &sw};
    // Validate/protect all tables before changing any entry (all or nothing).
    DWORD protection[3]{};
    unsigned protectedTables = 0;
    for (; protectedTables < 3; ++protectedTables)
        if (!VirtualProtect(tables[protectedTables], 83 * sizeof(void*), PAGE_READWRITE, &protection[protectedTables])) break;
    if (protectedTables == 3) {
        for (unsigned d = 0; d < 3; ++d) for (unsigned s = 0; s < 83; ++s) originals[d][s] = tables[d][s];
        for (unsigned d = 0; d < 3; ++d) for (unsigned s = 0; s < 83; ++s)
            InterlockedExchangePointer(tables[d] + s, (*hooks[d])[s]);
        enabled = true;
    }
    for (unsigned d = 0; d < protectedTables; ++d) { DWORD ignored; VirtualProtect(tables[d], 83*sizeof(void*), protection[d], &ignored); }
    if (!enabled) { Log("draw capture: refused: table protection failed"); return; }
    mainThread = GetCurrentThreadId();
    framesPerScenario = static_cast<unsigned>((std::max)(1L, (std::min)(600L, SettingInt("draw_capture_frames", nullptr, 4))));
    warmupFrames = static_cast<unsigned>((std::max)(1L, (std::min)(3600L, SettingInt("draw_capture_warmup", nullptr, 30))));
    sweep = SettingInt("draw_capture_scenarios", nullptr, 0) != 0;
    scenarioCount = sweep ? 10 : 1;
    zoom = static_cast<int>((std::max)(1L, (std::min)(5L, SettingInt("draw_capture_zoom", nullptr, 3))));
    rotation = static_cast<int>(SettingInt("draw_capture_rotation", nullptr, 0)) & 3;
    Log("draw capture: armed slots=83 drivers=3 frames=%u warmup=%u scenarios=%u; buffer pointers are presence bits; vertices hash at draw",
        framesPerScenario, warmupFrames, scenarioCount);
}
void DrawCaptureTick(cISC4City* city) {
    if (!enabled || finished || !city) return;
    // Camera changes can pump modal framework ticks before returning. Those
    // ticks must not advance the scenario or exhaust its wait budget.
    static bool ticking = false;
    if (ticking) return;
    struct TickScope { bool& value; ~TickScope() { value = false; } } tickScope{ticking};
    ticking = true;
    try {
        std::lock_guard<std::recursive_mutex> lock(captureMutex);
        EndFrame();
        if (failed) { Finish(); return; }
        if (inScenario == framesPerScenario) { ++scenario; inScenario = 0; warmup = 0; }
        if (scenario == scenarioCount) { Finish(); return; }
        auto* app = App();
        auto* root = app ? app->GetMainWindow() : nullptr;
        auto* view = View(root);
        auto* sim = city->GetSimulator();
        if (!view || !sim || (scenario == 0 && warmup == 0 && sim->IsHiddenPaused())) {
            if (view) view->Release(); if (app) app->Release();
            if (++waitTicks == 3600) { ++incomplete; Finish(); }
            return;
        }
        if (!sim->IsPaused()) sim->Pause();
        if (warmup == 0) {
            if (sweep) { zoom = static_cast<int>(scenario / 2) + 1; rotation = scenario % 2; }
            view->SetZoomAndRotation(zoom, rotation);
            Log("draw capture: scenario=%u zoom=%d rotation=%d paused=%u hidden=%u", scenario, zoom, rotation, sim->IsPaused(), sim->IsHiddenPaused());
        }
        auto* renderer = view->GetRenderer();
        if (renderer) renderer->ForceFullRedraw();
        view->Release();
        ++warmup;
        // Rotation temporarily hidden-pauses the simulator. Continue requesting
        // redraws while it settles; never record the transition as a paused view.
        if (warmup <= warmupFrames || sim->IsHiddenPaused()) {
            if (app) app->Release(); return;
        }
        if (!stream) {
            const std::string path = std::string(LogPath()) + ".draw.jsonl";
            stream = std::fopen(path.c_str(), "wb");
            if (!stream) { failed = true; if (app) app->Release(); Finish(); return; }
            Write("{\"type\":\"header\",\"schema\":\"fc4.draw/1\",\"hash\":\"fnv1a64\",\"frames\":" +
                Number(framesPerScenario * scenarioCount) + ",\"scenarios\":" + Number(scenarioCount) +
                ",\"schedule\":\"" + (Setting("schedule", nullptr) == "fixed" ? "fixed" : "other") + "\"}");
            Log("draw capture: stream=%s", path.c_str());
        }
        call = 0; frameDraws = 0;
        Write("{\"type\":\"frame\",\"frame\":" + Number(frame) + ",\"scenario\":" + Number(scenario) +
              ",\"zoom\":" + std::to_string(zoom) + ",\"rotation\":" + std::to_string(rotation) +
              ",\"paused\":" + Number(sim->IsPaused()) + ",\"hidden\":" + Number(sim->IsHiddenPaused()) + "}");
        inspecting = true; uiNodes = 0;
        if (root) { UiNode(root, -1, 0); ++uiFrames; } else ++incomplete;
        inspecting = false;
        if (app) app->Release();
        active = true;
    } catch (...) { inspecting = false; failed = true; Finish(); }
}
} // namespace fc4
