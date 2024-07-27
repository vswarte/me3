#include "test_project.h"
#include <iostream>

namespace test_project {
    size_t get_u16string_len(const std::u16string& s) {
        return s.size();
    }

    // void print_u16string(const std::u16string& s) {
    //     std::wcout << reinterpret_cast<const wchar_t*>(s.c_str()) << std::endl;
    // }
    //
    // void set_u16string(std::u16string& s, rust::Str content) {
    //     std::u16string new_content(content.begin(), content.end());
    //     s = new_content;
    // }
    //
    // rust::String get_u16string_content(const std::u16string& s) {
    //     std::string utf8_content;
    //     for (char16_t c : s) {
    //         // Simplified conversion; you might want to use proper UTF-16 to UTF-8 conversion
    //         utf8_content += static_cast<char>(c);
    //     }
    //     return rust::String(utf8_content);
    // }
}
