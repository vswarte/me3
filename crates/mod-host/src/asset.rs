use std::{cell::OnceCell, collections::HashMap, io::Write, path::PathBuf, sync::{Arc, OnceLock, RwLock}};

mod string;
mod allocator;

use me3_mod_host_assets::{ffi::{self, DLWString}, hook::RSResourceFileRequest, log_file, mapping::AssetMapping};
use me3_mod_protocol::package::Package;
use retour::Function;
use string::DLString;
use thiserror::Error;
use crate::{detour::{install_detour, Detour, DetourError}, host::ModHost};

pub type OpenHookFn = extern "C" fn(*mut RSResourceFileRequest) -> bool;
static HOOK: OnceLock<Arc<Detour<OpenHookFn>>> = OnceLock::new();

// TODO: we can probably swap out the string and do minimal copy comparisons instead?
static ASSET_MAPPING: OnceLock<RwLock<HashMap<String, Vec<u16>>>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum AssetLoadHookError {
    #[error("Could not place vfs detour")]
    Detour(#[from] DetourError),

    #[error("Package source specified is not a directory {0}.")]
    InvalidDirectory(PathBuf),

    #[error("Could not read directory while discovering override assets {0}")]
    ReadDir(std::io::Error),

    #[error("Could not acquire directory entry")]
    DirEntryAcquire(std::io::Error),
}

#[derive(Debug, Default)]
pub struct AssetLoadHook {
    mapping: AssetMapping,
}

impl AssetLoadHook {
    pub fn new(mapping: AssetMapping) -> Self {
        Self { mapping }
    }

    /// Attachs the asset load hook to a mod host
    pub fn attach(&mut self, host: &mut ModHost) -> Result<(), AssetLoadHookError> {
        let hook_instance: OnceCell<Arc<Detour<OpenHookFn>>> = OnceCell::new();

        let ptr = unsafe { std::mem::transmute::<usize, OpenHookFn>(0x140128730usize) };
        let hook_fn = host.hook(ptr)
            .with_closure(move |request: *mut RSResourceFileRequest| -> bool {
                hook_instance.get()
                    .unwrap()
                    .trampoline()(request)
            })
            .install()
            .expect("Lmao");

        hook_instance.set(hook_fn);

        HOOK.get_or_init(|| {
            // TODO: dynamic lookup

            host.hook(ptr)
                .with(hook)
                .install()
                .expect("Could not install file loading hook")
        });

        log_file().write().unwrap()
            .write_all(String::from("Loaded\n").as_bytes());

        Ok(())
    }
}

extern "C" fn hook(request: *mut RSResourceFileRequest) -> bool {
    let mut resource_path = unsafe { &request.as_mut().unwrap().resource_path };
    let resource_path_string = unsafe { ffi::get_dlstring_contents(resource_path) };

    log_file().write().unwrap()
        .write_all(format!("Requested assset: {resource_path_string}\n").as_bytes()).unwrap();

    if resource_path_string.ends_with("bd_m_1620.partsbnd.dcx") {
        unsafe {
            ffi::set_dlstring_contents(
                resource_path as *const _ as *mut _,
                "Z:/home/vincent/.steam/steam/steamapps/common/ELDEN RING/Game/override/parts/bd_m_1620.partsbnd.dcx".to_string()
            );
        }
    }

    HOOK.get()
        .unwrap()
        .trampoline()(request)
}
