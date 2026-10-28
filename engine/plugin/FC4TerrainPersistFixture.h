// SPDX-License-Identifier: GPL-3.0-or-later
// In-memory database for the terrain glue regression (seam 59).
#pragma once

#include "cGZPersistResourceKey.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistDBSegment.h"

#include <cstring>
#include <map>
#include <tuple>
#include <vector>

namespace fc4 {

class TerrainRecordFixture final : public cIGZPersistDBRecord {
public:
    TerrainRecordFixture(cGZPersistResourceKey key, const std::vector<uint8_t>& data) : key_(key), data_(data) {}
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { const auto refs = --refs_; if (!refs) delete this; return refs; }
    void GetKey(cGZPersistResourceKey& key) const override { key = key_; }
    uint16_t GetAccessFlags() const override { return 1; }
    bool Close() override { return true; }
    bool GetFieldVoid(void* out, uint32_t size) override {
        if (position_ > data_.size() || size > data_.size() - position_) return false;
        std::memcpy(out, data_.data() + position_, size);
        position_ += size;
        return true;
    }
    bool SetFieldVoid(const void*, uint32_t) override { return false; }
    uint32_t GetSize() override { return static_cast<uint32_t>(data_.size()); }
    bool SetSize(uint32_t) override { return false; }
    uint32_t GetPosition() override { return position_; }
    uint32_t SeekAbsolute(uint32_t position) override { position_ = position; return position_; }
    uint32_t SeekRelative(int32_t offset) override { position_ += offset; return position_; }
private:
    cGZPersistResourceKey key_;
    std::vector<uint8_t> data_;
    uint32_t refs_ = 1, position_ = 0;
};

class TerrainSegmentFixture final : public cIGZPersistDBSegment {
public:
    using Key = std::tuple<uint32_t, uint32_t, uint32_t>;
    std::map<Key, std::vector<uint8_t>> records;
    static Key Index(const cGZPersistResourceKey& key) { return {key.type, key.group, key.instance}; }
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool Init() override { return true; }
    bool Shutdown() override { return true; }
    bool Open(bool, bool) override { return true; }
    bool IsOpen() const override { return true; }
    bool Close() override { return true; }
    bool Flush() override { return true; }
    void GetPath(cIGZString&) const override {}
    bool SetPath(const cIGZString&) override { return false; }
    bool Lock() override { return true; }
    bool Unlock() override { return true; }
    uint32_t GetSegmentID() const override { return 0; }
    bool SetSegmentID(const uint32_t&) override { return false; }
    uint32_t GetRecordCount(cIGZPersistResourceKeyFilter*) override { return static_cast<uint32_t>(records.size()); }
    uint32_t GetResourceKeyList(cIGZPersistResourceKeyList*, cIGZPersistResourceKeyFilter*) override { return 0; }
    bool GetResourceKeyList(cIGZPersistResourceKeyList&) override { return false; }
    bool TestForRecord(const cGZPersistResourceKey& key) override { return records.count(Index(key)) != 0; }
    uint32_t GetRecordSize(const cGZPersistResourceKey& key) override {
        const auto found = records.find(Index(key));
        return found == records.end() ? 0 : static_cast<uint32_t>(found->second.size());
    }
    bool OpenRecord(const cGZPersistResourceKey& key, cIGZPersistDBRecord** out, cIGZFile::AccessMode) override {
        const auto found = records.find(Index(key));
        if (found == records.end()) { *out = nullptr; return false; }
        *out = new TerrainRecordFixture(key, found->second);
        return true;
    }
    bool CreateNewRecord(const cGZPersistResourceKey&, cIGZPersistDBRecord**) override { return false; }
    bool CloseRecord(cIGZPersistDBRecord*) override { return true; }
    bool CloseRecord(cIGZPersistDBRecord**) override { return true; }
    bool AbortRecord(cIGZPersistDBRecord*) override { return false; }
    bool AbortRecord(cIGZPersistDBRecord**) override { return false; }
    bool DeleteRecord(const cGZPersistResourceKey& key) override { return records.erase(Index(key)) != 0; }
    uint32_t ReadRecord(const cGZPersistResourceKey&, void*, uint32_t&) override { return 0; }
    bool WriteRecord(const cGZPersistResourceKey& key, void* data, uint32_t size) override {
        const auto* bytes = static_cast<const uint8_t*>(data);
        records[Index(key)] = std::vector<uint8_t>(bytes, bytes + size);
        return true;
    }
    bool Init(uint32_t, const cIGZString&, bool) override { return true; }
};

}  // namespace fc4
