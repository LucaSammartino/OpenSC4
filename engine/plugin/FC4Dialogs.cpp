// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4Dialogs.h"

#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4WinInterfaces.h"
#include "compat/FC4Compat.h"

#include "cGZMessage.h"
#include "cIGZFrameWork.h"
#include "cIGZMessageTarget.h"
#include "cRZCOMDllDirector.h"

#include <cctype>
#include <climits>
#include <cstdio>
#include <string>
#include <vector>

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

namespace fc4 {
namespace {

// modal-dialogs.md §2.5, §2.6, §2.2.
constexpr uint32_t kMsgModalBegin = 0x62BA7B42;
constexpr uint32_t kMsgModalEnd = 0x62BA7B43;
constexpr uint32_t kCommandMessage = 3;              // a command message's GZ type
constexpr uint32_t kButtonPressed = 0x287259F6;      // its data1: "a button was pressed"
constexpr uint32_t kButtonId = 0;                    // the only button ID we send (§3.2, §3.3)
constexpr uint32_t kEndModalResult = 0;              // EndModal(window, 0) (§3.2 step 3)
constexpr uint32_t kFlagModal = 0x1000;              // cIGZWin flag: running as a modal window
constexpr size_t kMaxRecords = 32;                   // nesting deeper than this is not tracked [choice]
constexpr int kFocusWalkLimit = 64;                  // parents walked from the focus at most [choice]

// Waiting: before the press. Pressed / EndModalSent: the call is made, the
// end notice is awaited. GaveUp: no end notice after EndModal; only the
// window's validity is still checked. Skipped: SC4GameDelay's window, never
// touched.
enum class Step { Waiting, Pressed, EndModalSent, GaveUp, Skipped };

struct Record {
    uint32_t seq;
    cFC4IGZWin* win;  // borrowed from the notice: never AddRef'd or released; IsWindowValid before every call
    uint32_t id;
    int32_t depth;    // the nest count before this dialog (the begin notice's data3)
    uint32_t beginFrame;
    Step step;
    uint32_t callFrame;        // the press or EndModal that is awaited (for K)
    uint32_t timerFrame;       // where the current wait started (restarted when the record is innermost again)
    uint32_t notCurrentSince;  // the first frame a due call found the window not current (0: none)
    bool warnedNotCurrent;
    bool adopted;              // found through the focus, without a begin notice
};

bool g_read = false;
bool g_enabled = false;
volatile LONG g_closed = 0;
bool g_started = false;     // between DialogsStart and DialogsStop
bool g_subscribed = false;
bool g_warnedSubscribe = false;
bool g_warnedWinMgr = false;
uint32_t g_frame = 0;
uint32_t g_seq = 0;
std::vector<Record> g_records;  // in begin order: the back is the innermost

// Test seam (kFC4SeamDialogs): borrowed, never AddRef'd or released.
cFC4IGZWinMgr* g_testWinMgr = nullptr;
cFC4IGZMessageServer* g_testServer = nullptr;
const void* g_testGameDelayVtable = nullptr;

std::string Trimmed(const std::string& s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

unsigned long Hex(uint32_t v) { return static_cast<unsigned long>(v); }
unsigned long Ptr(const void* p) { return static_cast<unsigned long>(reinterpret_cast<uintptr_t>(p)); }

// "0x<id> (window 0x<ptr>, depth N)"
struct Name {
    char text[64];
    explicit Name(const Record& r) {
        std::snprintf(text, sizeof(text), "0x%08lX (window 0x%08lX, depth %ld)", Hex(r.id), Ptr(r.win),
                      static_cast<long>(r.depth));
    }
};

// A framework service with the one reference GetSystemService gave us (the
// caller releases it with FC4_RELEASE), or nullptr and `why`.
template <class T>
T* AcquireService(uint32_t srvid, uint32_t iid, const char* what, const char** why) {
    cIGZFrameWork* fw = RZGetFrameWork();
    if (fw == nullptr) {
        *why = "no framework";
        return nullptr;
    }
    FC4_OUT(T*, out, what);
    const bool ok = fw->GetSystemService(srvid, iid, reinterpret_cast<void**>(out.ptr()));  // framework slot 5
    out.Check();
    if (!ok || out.get() == nullptr) {
        *why = "GetSystemService failed";
        return nullptr;
    }
    FC4_ACQUIRED(out.get(), "GetSystemService");
    return out.get();
}

// The window manager or the legacy message server for one use: the test's
// (borrowed) or the framework's (one reference, given back by Done()).
template <class T>
struct Service {
    T* ptr = nullptr;
    bool counted = false;
    Service(T* test, uint32_t srvid, uint32_t iid, const char* what, const char** why) {
        ptr = test;
        if (ptr == nullptr) {
            ptr = AcquireService<T>(srvid, iid, what, why);
            counted = ptr != nullptr;
        }
    }
    Service(const Service&) = delete;
    Service& operator=(const Service&) = delete;
    ~Service() { Done(); }
    void Done() {
        if (counted) FC4_RELEASE(ptr);
        counted = false;
        ptr = nullptr;
    }
};

struct WinMgr : Service<cFC4IGZWinMgr> {
    explicit WinMgr(const char** why)
        : Service(g_testWinMgr, kFC4_SRVID_cIGZWinMgr, kFC4_IID_cIGZWinMgr,
                  "cIGZWinMgr* (GetSystemService out pointer)", why) {}
};

struct MsgServer : Service<cFC4IGZMessageServer> {
    explicit MsgServer(const char** why)
        : Service(g_testServer, kFC4_SRVID_cIGZMessageServer, kFC4_IID_cIGZMessageServer,
                  "cIGZMessageServer* (GetSystemService out pointer)", why) {}
};

const void* GameDelayVtable() {
    return g_testGameDelayVtable != nullptr ? g_testGameDelayVtable : compat::GameDelayWindowVtable();
}

int FindRecord(uint32_t seq) {
    for (size_t i = 0; i < g_records.size(); ++i) {
        if (g_records[i].seq == seq) return static_cast<int>(i);
    }
    return -1;
}

void CountClosed() { InterlockedIncrement(&g_closed); }

// ---- the notices (inside DoModalWin: nothing is called here) ----------------------------------------

void OnBegin(uint32_t id, uint32_t window, uint32_t depth) {
    cFC4IGZWin* win = reinterpret_cast<cFC4IGZWin*>(static_cast<uintptr_t>(window));
    for (size_t i = 0; i < g_records.size(); ++i) {
        if (g_records[i].win == win) {  // its end notice never came; the window is modal again
            Log("dialogs: WARNING could not close %s: a new begin notice for the same window came before its end "
                "notice; the old record is dropped",
                Name(g_records[i]).text);
            g_records.erase(g_records.begin() + static_cast<std::ptrdiff_t>(i));
            break;
        }
    }
    Record r{};
    r.seq = ++g_seq;
    r.win = win;
    r.id = id;
    r.depth = static_cast<int32_t>(depth);
    r.beginFrame = g_frame;
    r.step = Step::Waiting;
    if (g_records.size() >= kMaxRecords) {
        Log("dialogs: WARNING could not close %s: %u dialogs are already recorded; this one is not tracked",
            Name(r).text, static_cast<unsigned>(kMaxRecords));
        return;
    }
    FC4_LOGV("dialogs: begin notice %s at frame %lu", Name(r).text, static_cast<unsigned long>(g_frame));
    g_records.push_back(r);
}

void LogEnd(const Record& r) {
    const Name n(r);
    switch (r.step) {
        case Step::Pressed:
            CountClosed();
            Log("dialogs: closed %s by button after %lu frame(s)", n.text,
                static_cast<unsigned long>(g_frame - r.callFrame));
            break;
        case Step::EndModalSent:
        case Step::GaveUp:
            CountClosed();
            Log("dialogs: closed %s by EndModal fallback after %lu frame(s)", n.text,
                static_cast<unsigned long>(g_frame - r.callFrame));
            break;
        case Step::Waiting:
            Log("dialogs: %s ended by itself after %lu frame(s)", n.text,
                static_cast<unsigned long>(g_frame - r.beginFrame));
            break;
        case Step::Skipped:
            FC4_LOGV("dialogs: %s (SC4GameDelay) ended by itself after %lu frame(s)", n.text,
                     static_cast<unsigned long>(g_frame - r.beginFrame));
            break;
    }
}

void OnEnd(uint32_t id, uint32_t window, uint32_t depthAfter) {
    cFC4IGZWin* win = reinterpret_cast<cFC4IGZWin*>(static_cast<uintptr_t>(window));
    int at = -1;
    for (size_t i = g_records.size(); i-- > 0;) {
        if (g_records[i].win == win) {
            at = static_cast<int>(i);
            break;
        }
    }
    if (at < 0) {
        FC4_LOGV("dialogs: end notice 0x%08lX (window 0x%08lX, depth %ld) without a record", Hex(id), Ptr(win),
                 static_cast<long>(depthAfter));
        return;
    }
    // Records inside this one: their loops ended first, so their end notices
    // were missed.
    for (size_t j = static_cast<size_t>(at) + 1; j < g_records.size(); ++j) {
        Log("dialogs: WARNING could not close %s: no end notice came before the end of the dialog around it; record "
            "dropped",
            Name(g_records[j]).text);
    }
    LogEnd(g_records[static_cast<size_t>(at)]);
    g_records.erase(g_records.begin() + at, g_records.end());
    // The dialog around it is the current one again. A call already made on
    // it restarts its wait: an end its handler asked for while this one ran
    // was deferred and is re-posted now (§2.4); an EndModal on top of it
    // would leave a second end for a window that is gone (§3.4).
    if (!g_records.empty()) {
        Record& o = g_records.back();
        o.timerFrame = g_frame;
        o.notCurrentSince = 0;
    }
}

class cFC4DialogTarget final : public cIGZMessageTarget {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != GZIID_cIGZMessageTarget && riid != GZIID_cIGZUnknown) {
            *ppvObj = nullptr;
            return false;
        }
        *ppvObj = static_cast<cIGZMessageTarget*>(this);
        AddRef();
        return true;
    }
    // A static object: counted, never deleted (the server AddRefs it at
    // AddNotification and releases it at RemoveNotification).
    uint32_t AddRef() override { return static_cast<uint32_t>(InterlockedIncrement(&refs_)); }
    uint32_t Release() override {
        const LONG r = InterlockedDecrement(&refs_);
        return r > 0 ? static_cast<uint32_t>(r) : 0u;
    }
    // The legacy server ignores the result (interfaces.tsv, the audio event handler's slot 3).
    bool DoMessage(cGZMessage& msg) override {
        if (!g_enabled || !g_subscribed) return true;
        if (msg.dwMessageType == kMsgModalBegin) {
            OnBegin(msg.dwData1, msg.dwData2, msg.dwData3);
        } else if (msg.dwMessageType == kMsgModalEnd) {
            OnEnd(msg.dwData1, msg.dwData2, msg.dwData3);
        }
        return true;
    }

private:
    volatile LONG refs_ = 0;
};

cFC4DialogTarget g_target;

// ---- the subscription --------------------------------------------------------------------------------

struct SubscribeCall {
    cFC4IGZMessageServer* server;
    bool add;  // AddNotification, else RemoveNotification
};

void SubscribeStep(void* p) {
    auto* c = static_cast<SubscribeCall*>(p);
    for (uint32_t type : {kMsgModalBegin, kMsgModalEnd}) {
        if (c->add) {
            c->server->AddNotification(&g_target, type);  // slot 5; the result is ignored (§3.2 table)
        } else {
            c->server->RemoveNotification(&g_target, type);  // slot 6
        }
    }
}

bool Subscribe(const char* when) {
    const char* why = "";
    MsgServer ms(&why);
    if (ms.ptr == nullptr) {
        if (!g_warnedSubscribe) {
            Log("dialogs: WARNING could not subscribe to the modal notices at %s: no legacy message server 0x%08lX "
                "(%s); retried at each frame",
                when, Hex(kFC4_SRVID_cIGZMessageServer), why);
        }
        g_warnedSubscribe = true;
        return false;
    }
    SubscribeCall c{ms.ptr, true};
    const bool clean = SehGuard("cIGZMessageServer::AddNotification", &SubscribeStep, &c);
    ms.Done();
    if (!clean) {
        Log("dialogs: WARNING structured exception in cIGZMessageServer::AddNotification at %s; the dialog closer "
            "stops",
            when);
        g_started = false;
        return false;
    }
    g_subscribed = true;
    Log("dialogs: subscribed to the modal begin / end notices 0x%08lX / 0x%08lX (legacy message server 0x%08lX) "
        "at %s",
        Hex(kMsgModalBegin), Hex(kMsgModalEnd), Hex(kFC4_SRVID_cIGZMessageServer), when);
    return true;
}

// ---- a dialog that was running before the subscription (§3.2, the focus walk) ------------------------

struct AdoptCall {
    cFC4IGZWinMgr* wm;
    int32_t nest;
    cFC4IGZWin* found;
    uint32_t id;
};

void AdoptStep(void* p) {
    auto* c = static_cast<AdoptCall*>(p);
    c->nest = c->wm->GetModalNestCount();  // slot 43
    if (c->nest <= 0) return;
    cFC4IGZWin* w = c->wm->GZGetFocus();  // slot 36: DoModalWin gave the dialog the focus
    for (int i = 0; i < kFocusWalkLimit && w != nullptr; ++i) {
        if (!c->wm->IsWindowValid(w)) return;  // slot 24
        if (w->GetFlag(kFlagModal)) {         // cIGZWin slot 67
            c->found = w;
            c->id = w->GetID();  // cIGZWin slot 63
            return;
        }
        w = w->GetParentWin();  // cIGZWin slot 11
    }
}

void AdoptRunning() {
    const char* why = "";
    WinMgr wm(&why);
    if (wm.ptr == nullptr) {
        Log("dialogs: WARNING no window manager 0x%08lX (%s): a dialog that is already open is not looked for",
            Hex(kFC4_SRVID_cIGZWinMgr), why);
        return;
    }
    AdoptCall c{wm.ptr, 0, nullptr, 0};
    const bool clean = SehGuard("cIGZWinMgr focus walk", &AdoptStep, &c);
    wm.Done();
    if (!clean) {
        Log("dialogs: WARNING structured exception in the focus walk; a dialog that is already open is not looked for");
        return;
    }
    if (c.nest <= 0) return;
    if (c.found == nullptr) {
        Log("dialogs: WARNING a modal loop is running (nest count %ld) but neither the focused window nor a parent of "
            "it is modal; that dialog is not closed",
            static_cast<long>(c.nest));
        return;
    }
    Record r{};
    r.seq = ++g_seq;
    r.win = c.found;
    r.id = c.id;
    r.depth = c.nest - 1;
    r.beginFrame = g_frame;
    r.step = Step::Waiting;
    r.adopted = true;
    Log("dialogs: adopted %s: it was open before the subscription (found through the focus; with nested dialogs "
        "only the focused one is found)",
        Name(r).text);
    g_records.push_back(r);
}

// ---- the close sequence (§3.2) --------------------------------------------------------------------------

enum class Want { Press, EndModal, ValidOnly };

struct ActCall {
    cFC4IGZWinMgr* wm;
    cFC4IGZWin* win;
    int32_t depth;
    const void* gameDelayVtable;
    Want want;
    bool valid;
    bool gameDelay;
    int32_t nest;
    bool flag;
    bool called;
    bool endModalOk;
};

void ActStep(void* p) {
    auto* c = static_cast<ActCall*>(p);
    c->valid = c->wm->IsWindowValid(c->win);  // slot 24: first, safe on a stale pointer
    if (!c->valid || c->want == Want::ValidOnly) return;
    // SC4GameDelay's window (§3.4): only its vtable pointer is read, nothing is called on it.
    if (c->gameDelayVtable != nullptr && *reinterpret_cast<const void* const*>(c->win) == c->gameDelayVtable) {
        c->gameDelay = true;
        return;
    }
    c->nest = c->wm->GetModalNestCount();  // slot 43
    if (c->nest != c->depth + 1) return;
    c->flag = c->win->GetFlag(kFlagModal);  // cIGZWin slot 67
    if (!c->flag) return;
    if (c->want == Want::Press) {
        // slot 14: what a click on button 0 does; delivered at the manager's next tick.
        c->wm->PostMessageToTarget5(c->win, kCommandMessage, kButtonPressed, kButtonId, 0);
    } else {
        c->endModalOk = c->wm->EndModal(c->win, kEndModalResult);  // slot 54
    }
    c->called = true;
}

void Act() {
    Record& r = g_records.back();
    const uint32_t seq = r.seq;
    Want want = Want::Press;
    switch (r.step) {
        case Step::Skipped:
            return;  // never touched
        case Step::Waiting:
            if (g_frame - r.beginFrame < kDialogsGraceFrames) return;
            want = Want::Press;
            break;
        case Step::Pressed:
            if (g_frame - r.timerFrame < kDialogsFallbackFrames) return;
            want = Want::EndModal;
            break;
        case Step::EndModalSent:
            if (g_frame - r.timerFrame < kDialogsFallbackFrames) return;
            Log("dialogs: WARNING could not close %s: no end notice %lu frame(s) after EndModal; nothing more is "
                "called on it",
                Name(r).text, static_cast<unsigned long>(g_frame - r.callFrame));
            r.step = Step::GaveUp;
            r.timerFrame = g_frame;
            return;
        case Step::GaveUp:
            if (g_frame - r.timerFrame < kDialogsFallbackFrames) return;
            r.timerFrame = g_frame;
            want = Want::ValidOnly;  // is the record stale?
            break;
    }

    const char* why = "";
    WinMgr wm(&why);
    if (wm.ptr == nullptr) {
        if (!g_warnedWinMgr) {
            Log("dialogs: WARNING could not close %s: no window manager 0x%08lX (%s); retried at each frame",
                Name(r).text, Hex(kFC4_SRVID_cIGZWinMgr), why);
        }
        g_warnedWinMgr = true;
        return;
    }
    ActCall c{wm.ptr, r.win, r.depth, want == Want::Press ? GameDelayVtable() : nullptr, want, false, false, -1,
              false, false, false};
    const bool clean = SehGuard("cIGZWinMgr dialog close", &ActStep, &c);
    wm.Done();

    const int at = FindRecord(seq);  // (no notice can arrive during these calls; re-found all the same)
    if (at < 0) return;
    Record& q = g_records[static_cast<size_t>(at)];
    if (!clean) {
        Log("dialogs: WARNING could not close %s: structured exception in the window manager calls; record dropped",
            Name(q).text);
        g_records.erase(g_records.begin() + at);
        return;
    }
    if (!c.valid) {
        Log("dialogs: WARNING could not close %s: the window is no longer valid (IsWindowValid false) and no end "
            "notice came; record dropped, nothing called on it",
            Name(q).text);
        g_records.erase(g_records.begin() + at);
        return;
    }
    if (want == Want::ValidOnly) return;
    if (c.gameDelay) {
        Log("dialogs: left %s alone: SC4GameDelay's invisible modal ends itself", Name(q).text);
        q.step = Step::Skipped;
        return;
    }
    if (!c.called) {
        // Not the current modal window (a nested dialog we have no record of,
        // or the frame DoModalWin runs after its loop): checked again next frame.
        if (q.notCurrentSince == 0) q.notCurrentSince = g_frame;
        if (!q.warnedNotCurrent && g_frame - q.notCurrentSince >= kDialogsFallbackFrames) {
            Log("dialogs: WARNING could not close %s: not the current modal window for %lu frame(s) (nest count "
                "%ld, expected %ld; modal flag %s); still checked each frame",
                Name(q).text, static_cast<unsigned long>(g_frame - q.notCurrentSince), static_cast<long>(c.nest),
                static_cast<long>(q.depth) + 1, c.nest == q.depth + 1 ? (c.flag ? "1" : "0") : "not read");
            q.warnedNotCurrent = true;
        }
        return;
    }
    q.notCurrentSince = 0;
    q.callFrame = g_frame;
    q.timerFrame = g_frame;
    if (want == Want::Press) {
        q.step = Step::Pressed;
        FC4_LOGV("dialogs: pressed button %lu on %s (PostMessageToTarget(window, 3, 0x%08lX, %lu, 0)) at frame %lu",
                 Hex(kButtonId), Name(q).text, Hex(kButtonPressed), Hex(kButtonId), static_cast<unsigned long>(g_frame));
    } else {
        q.step = Step::EndModalSent;
        FC4_LOGV("dialogs: no end notice %lu frame(s) after the button; EndModal(window, 0) on %s -> %d",
                 static_cast<unsigned long>(kDialogsFallbackFrames), Name(q).text, c.endModalOk ? 1 : 0);
    }
}

}  // namespace

void DialogsReload() {
    g_read = true;
    g_enabled = false;
    const char* source = "";
    const std::string raw = Trimmed(Setting("close_dialogs", "FC4_CLOSE_DIALOGS", &source));
    if (raw.empty()) return;  // off: nothing is logged
    const long v = SettingInt("close_dialogs", "FC4_CLOSE_DIALOGS", LONG_MIN);
    if (v == 1) {
        g_enabled = true;
        Log("dialogs: close_dialogs=1 (%s): game modal dialogs are ended through the window manager "
            "(modal-dialogs.md 3.2): button 0 %lu frame(s) after the begin notice, EndModal %lu frame(s) after that "
            "if no end notice came; subscribed at PostAppInit",
            source, static_cast<unsigned long>(kDialogsGraceFrames), static_cast<unsigned long>(kDialogsFallbackFrames));
    } else if (v != 0) {
        Log("config: WARNING close_dialogs='%s' (%s) is not 0/1; the dialog closer is off", raw.c_str(), source);
    }
}

void DialogsStart() {
    if (!g_read) DialogsReload();
    if (!g_enabled || g_started) return;
    g_started = true;
    g_warnedSubscribe = false;
    Subscribe("PostAppInit");
}

void DialogsStop() {
    const bool was = g_subscribed;
    g_started = false;
    g_subscribed = false;
    g_records.clear();
    if (!was) return;
    const char* why = "";
    MsgServer ms(&why);
    if (ms.ptr == nullptr) {
        Log("dialogs: WARNING could not unsubscribe at PreAppShutdown: no legacy message server (%s)", why);
        return;
    }
    SubscribeCall c{ms.ptr, false};
    const bool clean = SehGuard("cIGZMessageServer::RemoveNotification", &SubscribeStep, &c);
    ms.Done();
    Log("dialogs: unsubscribed at PreAppShutdown%s (%ld dialog(s) closed)",
        clean ? "" : " (WARNING structured exception in RemoveNotification)", DialogsClosedCount());
}

void DialogsOnTick() {
    if (!g_enabled || !g_started) return;
    ++g_frame;
    if (!g_subscribed) {
        if (!Subscribe("a frame")) return;
        AdoptRunning();  // a dialog already open has sent its begin notice to nobody
    }
    if (!g_records.empty()) Act();
}

bool DialogsEnabled() { return g_enabled; }

long DialogsClosedCount() { return g_closed; }

}  // namespace fc4

intptr_t DialogsTestSeam(const FC4DialogsTestInput* in) {
    using namespace fc4;
    if (in == nullptr) return -1;
    switch (in->action) {
        case kFC4DialogsTestServices:
            g_testWinMgr = static_cast<cFC4IGZWinMgr*>(in->winMgr);
            g_testServer = static_cast<cFC4IGZMessageServer*>(in->msgServer);
            g_testGameDelayVtable = in->gameDelayVtable;
            return 1;
        case kFC4DialogsTestStart:
            DialogsStart();
            return g_subscribed ? 1 : 0;
        case kFC4DialogsTestStop: {
            const bool was = g_subscribed;
            DialogsStop();
            return was ? 1 : 0;
        }
        case kFC4DialogsTestClosed:
            return DialogsClosedCount();
        case kFC4DialogsTestOpen:
            return static_cast<intptr_t>(g_records.size());
        default:
            return -1;
    }
}
