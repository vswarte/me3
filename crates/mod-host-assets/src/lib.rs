use std::{fs::File, sync::{OnceLock, RwLock}};

pub mod mapping;
pub mod hook;

#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("dl_string_bridge.hpp");

        pub type DLWString;

        pub fn get_dlwstring_contents(string: &DLWString) -> String;
        pub fn set_dlwstring_contents(string: &DLWString, contents: &[u16]);
    }
}
