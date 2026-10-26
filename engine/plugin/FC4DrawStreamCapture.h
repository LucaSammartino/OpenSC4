// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4DrawStream.h"

namespace fc4 {
void DrawStreamStartup();
void DrawStreamMonth(int32_t julian);

namespace drawstream {
// Installations live for the process. A scope only borrows the context, records
// calls on its calling thread, and restores the preceding scope on destruction.
// The owner adapter must separately capture fonts, surfaces, owner state and
// method outputs before it can request complete comparison credit.
class Capture {
public:
    Capture(void* context, Stream& stream, Objects& objects, Inputs* inputs = nullptr);
    ~Capture();
    Capture(const Capture&) = delete;
    Capture& operator=(const Capture&) = delete;
    static Capture* Current(void* context);
    void Before(uint32_t slot, std::initializer_list<uint32_t> arguments);
    void After(uint32_t slot, std::initializer_list<uint32_t> arguments, uint32_t result);
    void Refuse() noexcept { stream_.Refuse("context recording failed"); }
private:
    void* context_;
    Stream& stream_;
    Objects& objects_;
    Inputs* inputs_;
    Capture* previous_;
};

// Core-side recording context. It serializes the core's own calls, and supplies
// only getter outputs observed during this invocation's native pass. It never
// calls the borrowed context or executes drawing/message/command side effects.
// Unsupported context slots refuse credit. Font/surface and owner outputs must
// still be bound by the owner adapter before comparing with Output::All.
class ContextReplay {
public:
    ContextReplay(void* context, Stream& stream, Objects& objects, Inputs& inputs)
        : context_(context), stream_(stream), objects_(objects), inputs_(inputs) {}
    uint32_t Invoke(uint32_t slot, std::initializer_list<uint32_t> arguments = {});
private:
    void* context_;
    Stream& stream_;
    Objects& objects_;
    Inputs& inputs_;
};
}  // namespace drawstream
}  // namespace fc4
