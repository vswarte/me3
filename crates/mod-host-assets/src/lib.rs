#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("dlstring.h");

        type U16String = std::u16string;

        fn get_u16string_len(s: &U16String) -> usize;
        // fn set_u16string(s: &mut U16String, content: &str);
        // fn get_u16string_content(s: &U16String) -> String;
    }
}
