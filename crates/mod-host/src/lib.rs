#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]

use std::sync::OnceLock;

use asset::AssetLoadHook;
use me3_launcher_attach_protocol::{AttachError, AttachRequest, AttachResult, Attachment};
use me3_mod_host_assets::mapping::AssetMapping;
use crate::host::{hook::thunk::ThunkPool, ModHost};

mod detour;
mod host;
mod asset;

static INSTANCE: OnceLock<usize> = OnceLock::new();
/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        let mut host = ModHost::new(ThunkPool::new()?);

        let mut asset_mapping = AssetMapping::default();
        let asset_load_results = request.packages.iter()
            .map(|p| asset_mapping.scan_directory(&p.source.0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AttachError("Failed to scan asset folder".to_string()))?;

        let mut asset_hook = AssetLoadHook::new(asset_mapping);
        asset_hook.attach(&mut host);

        host.attach();

        let host = ModHost::get_attached_mut();

        Ok(Attachment)
    }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let _ = INSTANCE.set(instance);
    }

    1
}
