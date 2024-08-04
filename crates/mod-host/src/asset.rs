use std::{cell::OnceCell, collections::HashMap, io::Write, path::PathBuf, sync::{Arc, OnceLock, RwLock}};

use me3_mod_host_assets::{ffi::{self, DLWString}, hook::RSResourceFileRequest, log_file, mapping::AssetMapping};
use me3_mod_protocol::package::Package;
use retour::Function;
use thiserror::Error;
use crate::{detour::{install_detour, Detour, DetourError}, host::ModHost};

pub type OpenHookFn = extern "C" fn(*mut RSResourceFileRequest) -> bool;

#[derive(Debug, Default)]
pub struct AssetLoadHook {
    mapping: Arc<AssetMapping>,
}

impl AssetLoadHook {
    pub fn new(mapping: AssetMapping) -> Self {
        log_file().write().unwrap()
            .write_all(format!("Asset mapping: {mapping:#?}\n").as_bytes()).unwrap();

        Self { mapping: Arc::new(mapping) }
    }

    /// Attachs the asset load hook to a mod host
    pub fn attach(&mut self, host: &mut ModHost) -> Result<(), DetourError> {
        let hook_instance: Arc<OnceCell<Arc<Detour<OpenHookFn>>>> = Default::default();

        let hook = {
            let hook_instance = hook_instance.clone();
            let mapping = self.mapping.clone();

            host.hook(self.get_hook_location())
                .with_closure(move |request: *mut RSResourceFileRequest| -> bool {
                    let mut resource_path = unsafe { &request.as_mut().unwrap().resource_path };
                    let resource_path_string = unsafe { ffi::get_dlstring_contents(resource_path) };

                    log_file().write().unwrap()
                        .write_all(format!("Requested asset: {resource_path_string}\n").as_bytes()).unwrap();

                    if let Some(mapped_override) = mapping.get_override(&resource_path_string) {
                        log_file().write().unwrap()
                            .write_all(format!("Found override. {resource_path_string:?} -> {mapped_override:?}\n").as_bytes()).unwrap();

                        unsafe {
                            ffi::set_dlstring_contents(
                                resource_path as *const _ as *mut _,
                                mapped_override.clone()
                            );
                        }
                    }

                    hook_instance
                        .get().unwrap()
                        .trampoline()(request)
                })
                .install()?
        };

        hook_instance.set(hook);

        Ok(())
    }

    // TODO: call into AssetHookLocationProvider trait and either AOB or do
    // vftable lookups depending on the game?
    fn get_hook_location(&self) -> OpenHookFn {
        unsafe { std::mem::transmute::<usize, OpenHookFn>(0x140128730usize) }
    }
}
