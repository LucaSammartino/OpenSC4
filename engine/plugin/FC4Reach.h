// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstddef>
#include <cstdint>
#include <iosfwd>
#include <memory>
#include <string>
#include <vector>

namespace fc4 {
namespace reach {

struct Contract {
    uint32_t address;
    uint32_t size;
    std::string name;
};

// Only interface metadata is read. No executable file is opened.
bool ReadContracts(std::istream& input, std::vector<Contract>& out, std::string& error);

struct Row {
    uint32_t address = 0;
    uint32_t entries = 0;
    uint32_t windowsHit = 0;
    uint32_t firstWindow = 0;
    uint32_t lastWindow = 0;
    bool capped = false;
    std::string skipped;
};

struct Summary {
    uint32_t functions = 0, armed = 0, reached = 0, skipped = 0, windows = 0, traps = 0;
    uint32_t ignored = 0, abortedSteps = 0, slotExhaustions = 0;
    uint32_t wxBreakpoints = 0, wxSteps = 0;
};

// One tracer per process. Prepare must precede other entry patches; Start
// must follow them. Stop restores bytes but retains the VEH for in-flight
// breakpoint/step exceptions. Destroy only after traced threads have joined.
class Tracer {
public:
    Tracer();
    ~Tracer();
    Tracer(const Tracer&) = delete;
    Tracer& operator=(const Tracer&) = delete;
    bool Prepare(const std::vector<Contract>& contracts, uintptr_t preferredBase,
                 uintptr_t actualBase, size_t imageSize);
    bool Start(uint32_t cap);
    void NextWindow();
    void Stop();
    bool Suspend();
    void Resume();
    std::vector<Row> Rows() const;
    Summary GetSummary() const;
    bool Write(const std::string& path) const;
    // Future entry-hook installers can use this before taking an entry.
    bool Owns(uintptr_t address) const;
    void* IgnoreThread();
    static void UnignoreThread(void* token);

private:
    struct State;
    std::unique_ptr<State> state_;
};

}  // namespace reach

// Off unless reach=on. Snapshot before Subsystems(), arm after city-load
// shadow/replace/compare installation, stop before those hooks are restored.
void ReachPrepare();
void ReachStart();
void ReachNewMonth(int32_t julian);
void ReachStop();

// Pause the census for constructed monthly shadow passes. Scripted game
// actions run outside this scope. Pauses are process-wide and documented as
// excluded intervals, avoiding unbounded traps on fixture-heavy getters.
class ReachProbeScope {
public:
    ReachProbeScope();
    ~ReachProbeScope();
private:
    void* token_;
};

}  // namespace fc4
