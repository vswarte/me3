#include "dlstring.h"

namespace dlstring {
    size_t get_dlstring_len(const DLStringHandle& handle) {
        return handle.string.size();
    }
}
