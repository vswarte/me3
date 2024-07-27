#include <cstdint>
#include <cstddef>
#include <memory>
#include <string>
#include "rust/cxx.h"

namespace test_project {
    size_t get_u16string_len(const std::u16string& s);
    // void set_u16string(std::u16string& s, rust::Str content);
    // rust::String get_u16string_content(const std::u16string& s);
}
