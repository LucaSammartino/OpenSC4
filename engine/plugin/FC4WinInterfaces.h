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

// The window manager, the window and the legacy message server as the game
// compiles them (docs/specs/modal-dialogs.md §2-§3, docs/abi/interfaces.tsv
// rows with notes "re-modal:"), checked by tests/abi_conformance_test:
//
//   cFC4IGZWinMgr         cIGZWinMgr (IID 0x5A4, framework service 0xA417445E),
//                         slots 3-63. The SDK header is wrong here (§3.2
//                         errata): it stops at slot 53, its slot 13 takes one
//                         argument instead of two, and its overloads of
//                         SendMessageToTarget / PostMessageToTarget land in
//                         the order MSVC gives them. Here every slot has a
//                         name of its own, in table order; slots 3-53 keep the
//                         SDK's names, 54-63 the names of the table notes.
//   cFC4IGZWin            cIGZWin (IID 0x22BA0121), slots 3-67 (the dialog
//                         closer calls slot 67 GetFlag only). Overloads have
//                         names of their own; slots the table names through
//                         identical-code folding keep the SDK's names.
//   cFC4IGZMessageServer  the legacy cIGZMessageServer (IID 0xC1085722,
//                         framework service 0x64063387), slots 3-11: the
//                         server of cGZMessage notices such as the modal
//                         begin / end 0x62BA7B42 / 0x62BA7B43 (§2.5).
// A subscriber implements the SDK's cIGZMessageTarget (IID 0xE98B2F57; slot 3
// DoMessage(cGZMessage&), pop 4), whose declaration matches the table.
// None of these methods returns in ST0 (the x87 rule, docs/abi/README.md).
#pragma once

#include "cIGZUnknown.h"

#include <cstdint>

class cGZMessage;
class cIGZFrameWork;
class cIGZMessageTarget;
class cIGZString;

static const uint32_t kFC4_SRVID_cIGZWinMgr = 0xA417445E;
static const uint32_t kFC4_IID_cIGZWinMgr = 0x5A4;
static const uint32_t kFC4_IID_cIGZWin = 0x22BA0121;
static const uint32_t kFC4_SRVID_cIGZMessageServer = 0x64063387;
static const uint32_t kFC4_IID_cIGZMessageServer = 0xC1085722;

class cFC4IGZWin : public cIGZUnknown {
public:
    virtual bool DoMessage(cGZMessage& msg) = 0;                                    // 3
    virtual bool Init() = 0;                                                        // 4
    virtual bool Shutdown() = 0;                                                    // 5
    virtual void* GetWindowManager() = 0;                                           // 6
    virtual bool SetWindowManager(void* winMgr) = 0;                                // 7
    virtual void* GetKeyboard() = 0;                                                // 8
    virtual bool SetKeyboard(void* keyboard) = 0;                                   // 9
    virtual cFC4IGZWin* GetMainWindow() = 0;                                        // 10 (forward)
    virtual cFC4IGZWin* GetParentWin() = 0;                                         // 11
    virtual bool SetParentWin(cFC4IGZWin* win) = 0;                                 // 12
    virtual int32_t GetChildCount() = 0;                                            // 13
    virtual bool ChildAdd(cFC4IGZWin* win) = 0;                                     // 14
    virtual bool ChildRemove(cFC4IGZWin* win) = 0;                                  // 15
    virtual bool ChildDelete(cFC4IGZWin* win) = 0;                                  // 16 (forward)
    virtual bool ChildDeleteAbsolute(cFC4IGZWin* win) = 0;                          // 17
    virtual bool ChildDeleteAll() = 0;                                              // 18
    virtual bool ChildExists19(uint32_t winOrId) = 0;                               // 19 (ChildExists)
    virtual bool ChildExists20(uint32_t winOrId) = 0;                               // 20 (ChildExists)
    virtual bool IsWinInParentChain(cFC4IGZWin* win) = 0;                           // 21
    virtual bool IsWinInChildChain(cFC4IGZWin* win) = 0;                            // 22
    virtual bool PullToFront() = 0;                                                 // 23
    virtual bool SendToBack() = 0;                                                  // 24
    virtual bool ChildToFront25(uint32_t winOrId) = 0;                              // 25 (ChildToFront)
    virtual bool ChildToFront26(uint32_t winOrId) = 0;                              // 26 (ChildToFront)
    virtual bool ChildToBack(cFC4IGZWin* win) = 0;                                  // 27
    virtual bool ChildStepFront(cFC4IGZWin* win) = 0;                               // 28
    virtual bool ChildStepBack(cFC4IGZWin* win) = 0;                                // 29
    virtual bool MoveRelativeTo(cFC4IGZWin* win, bool unknown) = 0;                 // 30
    virtual bool ChildMoveRelative(cFC4IGZWin* a, cFC4IGZWin* b, bool unknown) = 0; // 31
    virtual bool EnumChildren(uint32_t riid, void* callback, void* context) = 0;    // 32
    virtual bool SortChildren(void* callback, void* context, bool unknown) = 0;     // 33
    virtual cFC4IGZWin* GetChildWindowFromID(uint32_t id) = 0;                      // 34
    virtual cFC4IGZWin* GetChildWindowFromIDRecursive(uint32_t id) = 0;             // 35
    virtual bool GetChildAs(uint32_t id, uint32_t riid, void** out) = 0;            // 36
    virtual bool GetChildAsRecursive(uint32_t id, uint32_t riid, void** out) = 0;   // 37
    virtual cFC4IGZWin* GetWindowFromPoint(int32_t x, int32_t z) = 0;               // 38 (forward)
    virtual cFC4IGZWin* GetChildWindowFromPoint(int32_t x, int32_t z) = 0;          // 39
    virtual cFC4IGZWin* GetChildWindowFromCursorPoint(int32_t x, int32_t z) = 0;    // 40
    virtual int32_t GetW() const = 0;                                               // 41
    virtual int32_t GetH() const = 0;                                               // 42
    virtual int32_t GetL() const = 0;                                               // 43
    virtual int32_t GetT() const = 0;                                               // 44
    virtual int32_t GetR() const = 0;                                               // 45
    virtual int32_t GetB() const = 0;                                               // 46
    virtual bool GetAreaInto(void* rect) const = 0;                                 // 47 (GetArea, pop 4)
    virtual int32_t* GetArea() const = 0;                                           // 48
    virtual bool GetAreaAbsoluteInto(void* rect) const = 0;                         // 49 (GetAreaAbsolute, pop 4)
    virtual int32_t* GetAreaAbsolute() const = 0;                                   // 50
    virtual bool SetW(int32_t w) = 0;                                               // 51
    virtual bool SetH(int32_t h) = 0;                                               // 52
    virtual bool SetSize(int32_t w, int32_t h) = 0;                                 // 53
    virtual bool SetAreaRect(const void* rect) = 0;                                 // 54 (SetArea, pop 4)
    virtual bool SetAreaLTRB(int32_t l, int32_t t, int32_t r, int32_t b) = 0;       // 55 (SetArea, pop 16)
    virtual bool GZWinMoveTo(int32_t x, int32_t y) = 0;                             // 56
    virtual bool GZWinOffset(int32_t dx, int32_t dy) = 0;                           // 57
    virtual bool FitRectToWindow(void* rect, int32_t unknown) = 0;                  // 58
    virtual bool ScreenToWindowCoordinates(int32_t& x, int32_t& y) const = 0;       // 59
    virtual bool WindowToScreenCoordinates(int32_t& x, int32_t& y) const = 0;       // 60
    virtual bool WindowToWindowCoordinates(cFC4IGZWin* to, int32_t& x, int32_t& y) const = 0;  // 61
    virtual bool IsPointInWindowScreenCoordinates(int32_t x, int32_t y) const = 0;  // 62
    virtual uint32_t GetID() const = 0;                                             // 63
    virtual bool SetID(uint32_t id) = 0;                                            // 64
    virtual uint32_t GetInstanceID() const = 0;                                     // 65
    virtual bool SetInstanceID(uint32_t id) = 0;                                    // 66
    // (flags & mask) != 0 in EAX; 0x1000 = running as a modal window (§2.2).
    virtual bool GetFlag(uint32_t mask) const = 0;                                  // 67
};

class cFC4IGZWinMgr : public cIGZUnknown {
public:
    virtual cFC4IGZWin* GetMainWindow() = 0;                                        // 3
    virtual bool SetMainWindow(cFC4IGZWin* win) = 0;                                // 4
    virtual bool GetMainWinAs(uint32_t riid, void** out) = 0;                       // 5
    virtual void Plot() = 0;                                                        // 6
    virtual bool AppendToPlotStrategy(cFC4IGZWin* win) = 0;                         // 7
    virtual bool ExecutePlotStrategy() = 0;                                         // 8
    virtual bool GZWinMsgPost(uint32_t type, uint32_t data1, uint32_t data2) = 0;   // 9 (AL only)
    virtual bool GZWinMsgPostMouse(uint32_t type, int32_t x, int32_t z, uint32_t flags) = 0;  // 10
    virtual bool GZWinMsgPostKeyboard(uint32_t type, uint32_t key, uint32_t flags) = 0;      // 11 (forward)
    // 12: SendMessageToTarget(win, type, d1, d2, d3), synchronous; AL only.
    virtual bool SendMessageToTarget5(cFC4IGZWin* win, uint32_t type, uint32_t d1, uint32_t d2, uint32_t d3) = 0;
    // 13: SendMessageToTarget(win, cGZMessage&), two arguments (the SDK says one).
    virtual bool SendMessageToTargetMsg(cFC4IGZWin* win, cGZMessage& msg) = 0;
    // 14: PostMessageToTarget(win, type, d1, d2, d3): queued, delivered at the
    // manager's next tick. EAX is not a status (§3.2), hence void.
    virtual void PostMessageToTarget5(cFC4IGZWin* win, uint32_t type, uint32_t d1, uint32_t d2, uint32_t d3) = 0;
    // 15: PostMessageToTarget(win, cGZMessage const&) (a button's path).
    virtual void PostMessageToTargetMsg(cFC4IGZWin* win, const cGZMessage& msg) = 0;
    virtual bool ProcessPostedMessages() = 0;                                       // 16
    virtual bool ProcessMouseMessage(cGZMessage& msg) = 0;                          // 17
    virtual bool ProcessKeyboardMessage(cGZMessage& msg) = 0;                       // 18
    virtual bool FlushInputMessageQueue() = 0;                                      // 19
    virtual bool AddWindowToValidList(cFC4IGZWin* win) = 0;                         // 20
    virtual bool RemoveWindowFromValidList(cFC4IGZWin* win) = 0;                    // 21
    virtual bool CleanUpWindowReferences(cFC4IGZWin* win) = 0;                      // 22
    virtual bool DestroyWindow(cFC4IGZWin* win) = 0;                                // 23
    // 24: looks the pointer VALUE up (safe on a stale pointer); AL only.
    virtual bool IsWindowValid(cFC4IGZWin* win) = 0;
    virtual cFC4IGZWin* GetWindowFromPoint(int32_t x, int32_t z) = 0;               // 25
    virtual bool SetCurrentMouseWin(cFC4IGZWin* win) = 0;                           // 26
    virtual void* GetCanvas() = 0;                                                  // 27
    virtual cFC4IGZWin* GZGetCapture() = 0;                                         // 28
    virtual bool GZSetCapture(cFC4IGZWin* win) = 0;                                 // 29
    virtual bool GZReleaseCapture(cFC4IGZWin* win) = 0;                             // 30
    virtual bool GZShowCursor(bool show) = 0;                                       // 31
    virtual void* GetCursorManager() = 0;                                           // 32
    virtual bool GetCursorScreenPosition(int32_t& x, int32_t& z) = 0;               // 33
    virtual bool GetCursorRelativePosition(cFC4IGZWin* win, int32_t& x, int32_t& z) = 0;  // 34
    virtual bool SetCursorPosition(int32_t x, int32_t z) = 0;                       // 35
    virtual cFC4IGZWin* GZGetFocus() = 0;                                           // 36
    virtual bool GZSetFocus(cFC4IGZWin* win) = 0;                                   // 37
    virtual void* GetKeyboard() = 0;                                                // 38
    virtual bool GZRegisterKeyFilter(void* filter) = 0;                             // 39
    virtual bool GZUnregisterKeyFilter(void* filter) = 0;                           // 40
    virtual uint32_t DoModalWin(cFC4IGZWin* win) = 0;                               // 41
    virtual bool IsModal() = 0;                                                     // 42
    virtual int32_t GetModalNestCount() = 0;                                        // 43
    virtual uint32_t GZMsgBox(const cIGZString& text, const cIGZString& caption, uint32_t buttons, bool unknown,
                              uint32_t flags) = 0;                                  // 44
    virtual bool SetGZMsgBoxCMDID(uint32_t a, uint32_t b, bool c) = 0;              // 45
    virtual bool GetGZMsgBoxCMDID(uint32_t& a, uint32_t& b, bool& c) = 0;           // 46
    virtual bool DebugIsFocusGadgetVisible() = 0;                                   // 47
    virtual void DebugSetFocusGadgetVisible(bool visible) = 0;                      // 48
    virtual bool DebugIsCaptureGadgetVisible() = 0;                                 // 49
    virtual void DebugSetCaptureGadgetVisible(bool visible) = 0;                    // 50
    virtual bool DebugIsInvalidateGadgetVisible() = 0;                              // 51
    virtual void DebugSetInvalidateGadgetVisible(bool visible) = 0;                 // 52
    virtual void DebugDumpWindowList() = 0;                                         // 53
    // 54: EndModal(win, result) = GZWinMsgPost(0xF, win, result); AL only. Not in the SDK.
    virtual bool EndModal(cFC4IGZWin* win, uint32_t result) = 0;
    virtual void InvalidateAllWindows() = 0;                                        // 55
    virtual void DestroyWindowAbsolute(cFC4IGZWin* win) = 0;                        // 56
    virtual void DestroyCondemnedWindows() = 0;                                     // 57
    virtual bool IsWindowOrParentInCondemnedList(cFC4IGZWin* win) = 0;              // 58
    virtual uint32_t IncrementBusyCount() = 0;                                      // 59
    virtual void DecrementBusyCount() = 0;                                          // 60
    virtual void RunKeyFilters(cGZMessage& msg) = 0;                                // 61
    virtual void AddDeferredEndModal(cFC4IGZWin* win, uint32_t result) = 0;         // 62
    virtual uint32_t ModalMessageLoop(cFC4IGZWin* win, cIGZFrameWork* fw, uint32_t defaultResult,
                                      uint32_t quitResult) = 0;                     // 63
};

// cISC43DRender (vtable cSC43DRender::vftable 0x00ABAB60), slots 3-8 only:
// the landfill zot's orientation (landfill.md §4.7.5, LG21) reads the view
// rotation through cISC4View3DWin slot 4 GetRenderer (borrowed) and this
// slot 8 GetViewUtilities, which returns a BORROWED pointer to an object
// that is not a GZCOM object (no QueryInterface, never released); the
// rotation is its int32 at +0x10. Declared here rather than through the SDK
// header, whose later slots return classes by value.
class cFC4ISC43DRender : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                        // 3
    virtual bool Shutdown() = 0;                                                    // 4
    virtual bool SetMapScaleFactor(int32_t factor) = 0;                             // 5
    virtual bool SetGraphicSystem(void* graphicSystem) = 0;                         // 6
    virtual bool SetGDriver(void* driver) = 0;                                      // 7
    virtual void* GetViewUtilities() = 0;                                           // 8 (borrowed, not GZCOM)
};

class cFC4IGZMessageServer : public cIGZUnknown {
public:
    virtual bool MessageSend(const cGZMessage& msg) = 0;                            // 3 (synchronous)
    virtual bool MessagePost(const cGZMessage& msg, bool highPriority) = 0;         // 4
    // 5: the server AddRefs and keeps the target (not QueryInterface'd).
    virtual bool AddNotification(cIGZMessageTarget* target, uint32_t type) = 0;
    virtual bool RemoveNotification(cIGZMessageTarget* target, uint32_t type) = 0;  // 6
    // 00930273 enqueues a copied message/target, then unlocks; it defines no
    // Boolean result. Do not treat the unlock's return register as success.
    virtual void GeneralMessagePostToTarget(const cGZMessage& msg, cIGZMessageTarget* target) = 0;  // 7
    virtual bool CancelGeneralMessagePostsToTarget(cIGZMessageTarget* target) = 0;  // 8
    virtual bool OnTick(uint32_t unknown) = 0;                                      // 9
    virtual uint32_t GetMessageQueueSize() = 0;                                     // 10
    virtual void* SetAlwaysClearQueueOnTick(bool clear) = 0;                        // 11
};
