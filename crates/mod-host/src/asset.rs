use std::{char::decode_utf16, collections::{HashMap, VecDeque}, ffi::c_void, fs::{self, File, OpenOptions}, io::Write, ops::Index, path::{Component, Path, PathBuf}, ptr::copy_nonoverlapping, string::FromUtf16Error, sync::{Arc, OnceLock, RwLock}};

mod string;
mod allocator;

use me3_mod_protocol::package::Package;
use retour::Function;
use string::DLString;
use thiserror::Error;
use crate::{detour::{install_detour, Detour, DetourError}, host::ModHost};

type OpenHookFn = extern "C" fn(*mut RSResourceFileRequest) -> bool;

static HOOK: OnceLock<Arc<Detour<OpenHookFn>>> = OnceLock::new();

// TODO: move pointer locating to a trait so that we can implement one for AC6
// VFS path input: data0:/menu/somefile.gfx, parts:/some_fucking_weapon.partsbnd.dcx
extern "C" fn hook(request: *mut RSResourceFileRequest) -> bool {
    let asset = unsafe { request.as_ref() }.unwrap().resource_path.as_string();

    // log_file().write().unwrap()
    //     .write_all(format!("Requested resource: {asset}\n").as_bytes()).unwrap();

    let asset_normalized = asset.to_lowercase()
        .replace(":/", "/")
        .replace("data0/", "")
        .replace("data1/", "")
        .replace("data2/", "")
        .replace("data3/", "");

    log_file().write().unwrap()
        .write_all(format!("Asset request: {asset}\n").as_bytes()).unwrap();

    let asset_path = PathBuf::from(&asset_normalized);
    let asset_name = asset_path.file_name().unwrap().to_string_lossy();

    // log_file().write().unwrap()
    //     .write_all(format!("Asset name: {asset_name}\n").as_bytes()).unwrap();

    if let Some(rewrite) = ASSET_MAPPING.get().unwrap() 
        .read().unwrap()
        // .get(&asset_normalized) {
        .get(asset_name.as_ref()) {

        // # SAFETY
        //
        // might deref a bunch of null or worse if request isn't an instance of RSResourceFileRequest.
        // The function being hooked is a method on this vftable, so something's amis when this
        // isn't an instance of RSResourceFileRequest.
        unsafe {
            request.as_mut()
                .unwrap()
                .resource_path
                .set_contents(rewrite);
        }
    }

    HOOK.get()
        .unwrap()
        .trampoline()(request)
}

#[repr(C)]
struct RSResourceFileRequest {
    pub vfptr: usize,
    _unk8: [u8; 0x48],
    pub resource_path: DLString,
}

static LOG_HANDLE: OnceLock<RwLock<File>> = OnceLock::new();

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
pub struct AssetLoadHook;

impl AssetLoadHook {
    /// Attachs the asset load hook to a mod host
    pub fn attach(&mut self, host: &mut ModHost) -> Result<(), AssetLoadHookError> {
        HOOK.get_or_init(|| {
            // TODO: dynamic lookup
            let ptr = unsafe { std::mem::transmute::<usize, OpenHookFn>(0x140128730usize) };

            host.hook(ptr)
                .with(hook)
                .install()
                .expect("Could not install file loading hook")
        });

        log_file().write().unwrap()
            .write_all(String::from("Loaded\n").as_bytes());

        Ok(())
    }

    ///  Traverses a folder structure adding everything it discovers as an
    ///  asset mapping.
    pub fn scan_directory(&mut self, base_directory: &Path) -> Result<(), AssetLoadHookError> {
        if (!base_directory.is_dir()) {
            return Err(AssetLoadHookError::InvalidDirectory(base_directory.to_path_buf()));
        }

        let mut paths_to_scan = VecDeque::from(vec![base_directory.to_path_buf()]);
        while let Some(current_path) = paths_to_scan.pop_front() {
            for entry in fs::read_dir(current_path).map_err(AssetLoadHookError::ReadDir)? {
                let entry = entry.map_err(AssetLoadHookError::DirEntryAcquire)?;

                if !entry.path().is_dir() {
                    let asset_path = Self::normalize_path(entry.path());
                    let vfs_path = Self::convert_to_vfs_path(
                        Self::normalize_path(base_directory).as_path(),
                        asset_path.as_path(),
                    );
                    // let vfs_path = asset_path.file_name().unwrap().to_string_lossy().to_string();

                    log_file().write().unwrap()
                        .write_all(format!("Discovered override asset: {:?}\n", vfs_path).as_bytes());

                    let _ = ASSET_MAPPING.get_or_init(Default::default)
                        .write()
                        .unwrap()
                        .insert(vfs_path, Self::encode_path(asset_path));
                        // .insert(vfs_path, Self::encode_path(asset_path));
                } else {
                    paths_to_scan.push_back(entry.path());
                }
            }
        }

        Ok(())
    }

    /// Normalizes paths to use / as a path seperator
    fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
        PathBuf::from(path.as_ref().to_string_lossy().replace('\\', "/"))
    }

    /// Turns an asset path into an asset path based on the mods base path.
    /// INPUT EXAMPLE: menu/somefile.gfx, parts/someweapon.partsbnd.dcx
    /// OUTPUT EXAMPLE: menu:/somefile.gfx, parts:/someweapon.partsbnd.dcx
    fn convert_to_vfs_path<P: AsRef<Path>>(base: P, path: P) -> String {
        // TODO: error reporting
        let relative_path = match path.as_ref().strip_prefix(base) {
            Ok(path) => path,
            Err(_) => panic!("File path is not relative to the base path"),
        };

        let mut components = relative_path.iter()
            .map(|comp| comp.to_string_lossy().replace('\\', "/"));
        let first_component = components.next()
            .expect("Relative path should have at least one component");

        format!("{}/{}", first_component, components.collect::<Vec<_>>().join("/"))
    }

    /// Encodes the input into a utf16 string with terminator into a Vec<u16>
    fn encode_path<P: AsRef<Path>>(path: P) -> Vec<u16> {
        Self::encode_string(
            path.as_ref()
                .as_os_str()
                .to_string_lossy()
        )
    }

    /// Encodes the input into a utf16 string with terminator into a Vec<u16>
    fn encode_string<S: AsRef<str>>(string: S) -> Vec<u16> {
        let mut buffer = string.as_ref()
            .to_lowercase()
            .as_bytes()
            .iter()
            .copied()
            .map(u16::from)
            .collect::<Vec<_>>();

        // Slap on the null terminator
        buffer.push(0);

        buffer
    }
}

fn log_file() -> &'static RwLock<File> {
    LOG_HANDLE.get_or_init(|| {
        RwLock::new(std::fs::File::create("file_hook.log").unwrap())
    })
}
