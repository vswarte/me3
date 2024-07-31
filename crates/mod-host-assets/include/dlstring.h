#ifndef DANTELION_DL_ALLOCATOR_H
#define DANTELION_DL_ALLOCATOR_H

#include <rust/cxx.h>
#include <string>
#include <iostream>
#include <cstdint>
#include <locale>
#include <codecvt>
#include <type_traits>

#if defined(_ITERATOR_DEBUG_LEVEL) && _ITERATOR_DEBUG_LEVEL > 0
#error "_ITERATOR_DEBUG_LEVEL" must be defined as "0" for STL containers to be compatible with the ELDEN RING ABI.
#endif

namespace DLKR {
class DLAllocationInterface {
public:
    virtual ~DLAllocationInterface() = default;
    virtual uint32_t GetAllocatorId() = 0;
    virtual int32_t _unk0x10() = 0;
    virtual uint32_t& GetHeapFlags(uint32_t& out) = 0;
    virtual uint64_t GetHeapCapacity() = 0;
    virtual uint64_t GetHeapSize() = 0;
    virtual uint64_t GetBackingHeapCapacity() = 0;
    virtual uint64_t GetAllocationCount() = 0;
    virtual uint64_t GetSizeOfAllocation(void* pAllocation) = 0;
    virtual void* AllocateMemory(uint64_t sizeBytes) = 0;
    virtual void* AllocateAlignedMemory(uint64_t sizeBytes, uint64_t alignment) = 0;
    virtual void* ReallocateMemory(void* pAllocation, uint64_t sizeBytes) = 0;
    virtual void* ReallocateAlignedMemory(void* pAllocation, uint64_t sizeBytes, uint64_t alignment) = 0;
    virtual void FreeMemory(void* pAllocation) = 0;
};

template <typename T>
class DLAllocatorAdapter {
public:
    using value_type = T;
    using size_type = uint64_t;
    using difference_type = int64_t;

    using propagate_on_container_move_assignment = std::true_type;
    using is_always_equal = std::false_type;

    template <typename U>
    DLAllocatorAdapter(const DLAllocatorAdapter<U>& other) noexcept : allocator(other.allocator) {}

    T* allocate(size_type count) {
        return reinterpret_cast<T*>(allocator.AllocateAlignedMemory(count * sizeof(T), alignof(T)));
    }

    void deallocate(T* pAllocation, size_type count = 0) {
        allocator.FreeMemory(reinterpret_cast<void*>(pAllocation));
    }

    template <typename T1, typename T2>
    friend bool operator==(const DLAllocatorAdapter<T1>& lhs, const DLAllocatorAdapter<T2>& rhs) noexcept;

private:
    DLAllocationInterface& allocator;
};

template <typename T1, typename T2>
bool operator==(const DLAllocatorAdapter<T1>& lhs, const DLAllocatorAdapter<T2>& rhs) noexcept {
    return &lhs.allocator == &rhs.allocator;
}
}

template <typename CharT>
struct DLString {
    std::basic_string<CharT, std::char_traits<CharT>, DLKR::DLAllocatorAdapter<CharT>> inner;
    bool unk;
};

using DLWString = DLString<char16_t>;

size_t get_dlstring_len(const DLWString& string) {
    return string.inner.size();
}

rust::String get_dlstring_contents(const DLWString& string) {
    return rust::String(string.inner.data());
}

void set_dlstring_contents(DLWString* string, rust::String contents) {
    const std::string& temp = std::string(contents);
    std::wstring_convert<std::codecvt_utf8_utf16<char16_t>, char16_t> convert;
    std::u16string converted = convert.from_bytes(temp);
    string->inner.assign(converted.data());
    // return rust::String(string.inner.data());
}


#endif
