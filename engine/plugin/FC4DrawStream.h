// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>
#include <cstring>
#include <initializer_list>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace fc4::drawstream {

// A method-boundary trace, not pixels. Arguments are copied before forwarding;
// results and out-parameters afterwards. No borrowed pointer is owned.
enum class Domain : uint32_t { Context, Surface, Font, Message, Command, State, Return, Query };
struct Event {
    Domain domain{};
    uint32_t operation = 0, receiver = 0;
    std::vector<uint32_t> words;
    std::vector<uint8_t> bytes;
    bool operator==(const Event& other) const {
        return domain == other.domain && operation == other.operation && receiver == other.receiver &&
            words == other.words && bytes == other.bytes;
    }
};
struct ResourceKey { uint32_t type = 0, group = 0, instance = 0; };

// Shared by the native pass and the core pass of ONE invocation. Same-process
// identities cover unkeyed borrowed objects; images additionally carry their TGI.
class Objects {
public:
    uint32_t Borrow(const void* object) {
        if (!object) return 0;
        const auto found = objects_.find(object);
        if (found != objects_.end()) return found->second;
        const auto id = static_cast<uint32_t>(objects_.size() + 1);
        objects_.emplace(object, id);
        return id;
    }
    void Image(const void* image, ResourceKey key) { images_[Borrow(image)] = key; }
    bool ImageKey(uint32_t id, ResourceKey& key) const {
        const auto found = images_.find(id);
        if (found == images_.end()) return false;
        key = found->second;
        return true;
    }
private:
    std::unordered_map<const void*, uint32_t> objects_;
    std::unordered_map<uint32_t, ResourceKey> images_;
};

class Stream {
public:
    explicit Stream(size_t eventLimit = 16384, size_t byteLimit = 4 * 1024 * 1024)
        : eventLimit_(eventLimit), byteLimit_(byteLimit) {}
    void Append(Event event) {
        const auto bytes = event.bytes.size();
        if (events_.size() >= eventLimit_ || bytes > byteLimit_ - used_ ||
            event.words.size() > (byteLimit_ - used_ - bytes) / sizeof(uint32_t)) {
            Refuse("trace capacity");
            return;
        }
        used_ += bytes + event.words.size() * sizeof(uint32_t);
        events_.push_back(std::move(event));
    }
    void Call(Domain domain, uint32_t operation, uint32_t receiver,
              std::initializer_list<uint32_t> words = {}) {
        Append({domain, operation, receiver, words, {}});
    }
    void Data(Domain domain, uint32_t operation, uint32_t receiver,
              const void* data, size_t size, std::initializer_list<uint32_t> words = {}) {
        if (size > byteLimit_ || (size && !data)) { Refuse("invalid payload"); return; }
        Event event{domain, operation, receiver, words, {}};
        if (size) {
            const auto* begin = static_cast<const uint8_t*>(data);
            event.bytes.assign(begin, begin + size);
        }
        Append(std::move(event));
    }
    void Image(Domain domain, uint32_t operation, uint32_t receiver, const Objects& objects,
               uint32_t image, std::initializer_list<uint32_t> geometry) {
        ResourceKey key;
        if (!objects.ImageKey(image, key)) { Refuse("image has no resource key"); return; }
        Event event{domain, operation, receiver, {image, key.type, key.group, key.instance}, {}};
        event.words.insert(event.words.end(), geometry.begin(), geometry.end());
        Append(std::move(event));
    }
    void Refuse(const char* reason) { if (reason_.empty()) reason_ = reason; }
    bool Valid() const { return reason_.empty(); }
    const std::string& Reason() const { return reason_; }
    const std::vector<Event>& Events() const { return events_; }
private:
    size_t eventLimit_, byteLimit_, used_ = 0;
    std::string reason_;
    std::vector<Event> events_;
};

// Native getters run once against borrowed objects. Core getters replay those
// exact responses in order; an extra/missing/different query invalidates credit.
class Inputs {
public:
    void Observe(Event request, Event response) { entries_.push_back({std::move(request), std::move(response)}); }
    const Event* Read(const Event& request) {
        if (cursor_ == entries_.size() || !(entries_[cursor_].request == request)) {
            valid_ = false;
            return nullptr;
        }
        return &entries_[cursor_++].response;
    }
    bool Complete() const { return valid_ && cursor_ == entries_.size(); }
private:
    struct Entry { Event request, response; };
    std::vector<Entry> entries_;
    size_t cursor_ = 0;
    bool valid_ = true;
};

enum Output : uint32_t { Return = 1, OutParameters = 2, OwnerFields = 4, Collaborators = 8, All = 15 };
struct Result { bool exact = false, complete = false; size_t firstDifference = 0; };
inline Result Compare(const Stream& native, const Stream& core, uint32_t captured, const Inputs& inputs) {
    const auto& a = native.Events();
    const auto& b = core.Events();
    size_t first = 0;
    while (first < a.size() && first < b.size() && a[first] == b[first]) ++first;
    const bool complete = native.Valid() && core.Valid() && inputs.Complete() && captured == All;
    return {complete && first == a.size() && first == b.size(), complete, first};
}
struct Method {
    uint64_t calls = 0, complete = 0, mismatches = 0, incomplete = 0;
    void Observe(Result result) {
        ++calls;
        if (result.complete) { ++complete; if (!result.exact) ++mismatches; }
        else ++incomplete;
    }
};
inline uint32_t FloatWord(float value) {
    uint32_t word;
    std::memcpy(&word, &value, sizeof(word));
    return word;
}
}  // namespace fc4::drawstream
