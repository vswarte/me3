#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]

use std::{collections::HashMap, sync::OnceLock};

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

        /// TODO: Ask game nicely to resolve this for us
        let mut asset_mapping = AssetMapping::new(HashMap::from([
            (String::from("data0"), String::from("")),
            (String::from("data1"), String::from("")),
            (String::from("data2"), String::from("")),
            (String::from("data3"), String::from("")),

            (String::from("param"), String::from("param/")),
            (String::from("event"), String::from("event/")),
            (String::from("other"), String::from("other/")),
            // (String::from("shader"), String::from("shader/")),
            (String::from("action"), String::from("action/")),
            (String::from("material"), String::from("material/")),
            (String::from("gparam"), String::from("param/drawparam/")),
            // (String::from("wwise_moaeibnd"), String::from("sound/")),
            (String::from("font"), String::from("font/")),
            (String::from("menu"), String::from("menu/")),
            (String::from("msg"), String::from("msg/")),
            (String::from("sfxbnd"), String::from("sfx/")),
            (String::from("chranibnd"), String::from("chr/")),
            (String::from("chrbehbnd"), String::from("chr/")),
            (String::from("chrtexbnd"), String::from("chr/")),
            (String::from("chrbnd"), String::from("chr/")),
            (String::from("actscript"), String::from("action/script/")),
            // (String::from("facegen"), String::from("facegen/")),
            (String::from("map"), String::from("map/")),
            // (String::from("entryfilelist"), String::from("map/entryfilelist/")),
            (String::from("aiscript"), String::from("script/")),
            (String::from("talkscript"), String::from("script/talk/")),
            // (String::from("maptpf"), String::from("map/")),
            (String::from("mapstudio"), String::from("map/mapstudio/")),
            (String::from("asset"), String::from("asset/")),
            // (String::from("breakgeom"), String::from("map/breakgeom/")),
            // (String::from("onav"), String::from("map/onav/")),
            //(String::from("maphkx"), String::from("map/")),
            (String::from("parts"), String::from("parts/")),
            (String::from("hkxbnd"), String::from("map/")),

            (String::from("regulation"), String::from("")),
        ]));

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
