use std::{fs::File, sync::{OnceLock, RwLock}};

pub mod mapping;
pub mod hook;

#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("dlstring.h");

        pub type DLWString;

        pub unsafe fn get_dlstring_len(string: &DLWString) -> usize;
        pub unsafe fn get_dlstring_contents(string: &DLWString) -> String;
        pub unsafe fn set_dlstring_contents(string: *mut DLWString, contents: String);
    }
}

static LOG_HANDLE: OnceLock<RwLock<File>> = OnceLock::new();

pub fn log_file() -> &'static RwLock<File> {
    LOG_HANDLE.get_or_init(|| {
        RwLock::new(std::fs::File::create("file_hook.log").unwrap())
    })
}
