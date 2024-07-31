use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use thiserror::Error;

// TODO: we can probably swap out the string and do minimal copy comparisons instead?
// static ASSET_MAPPING: OnceLock<RwLock<HashMap<String, Vec<u16>>>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct AssetMapping {
    mapping: HashMap<String, Vec<u16>>,
}

#[derive(Debug, Error)]
pub enum AssetMappingError {
    #[error("Package source specified is not a directory {0}.")]
    InvalidDirectory(PathBuf),

    #[error("Could not read directory while discovering override assets {0}")]
    ReadDir(std::io::Error),

    #[error("Could not acquire directory entry")]
    DirEntryAcquire(std::io::Error),
}

impl AssetMapping {
    ///  Traverses a folder structure adding everything it discovers as an
    ///  asset mapping.
    pub fn scan_directory<P: AsRef<Path>>(&mut self, base_directory: P) -> Result<(), AssetMappingError> {
        let base_directory = base_directory.as_ref();
        if (!base_directory.is_dir()) {
            return Err(AssetMappingError::InvalidDirectory(
                base_directory.to_path_buf()
            ));
        }

        let mut paths_to_scan = VecDeque::from(vec![base_directory.to_path_buf()]);
        while let Some(current_path) = paths_to_scan.pop_front() {
            for entry in fs::read_dir(current_path).map_err(AssetMappingError::ReadDir)? {
                let entry = entry.map_err(AssetMappingError::DirEntryAcquire)?;

                if !entry.path().is_dir() {
                    let asset_path = normalize_path(entry.path());
                    let vfs_path = convert_to_vfs_path(
                        normalize_path(base_directory).as_path(),
                        asset_path.as_path(),
                    );

                    self.mapping.insert(vfs_path, encode_path(asset_path));
                    // let vfs_path = asset_path.file_name().unwrap().to_string_lossy().to_string();

                    // log_file().write().unwrap()
                    //     .write_all(format!("Discovered override asset: {:?}\n", vfs_path).as_bytes());

                    // let _ = ASSET_MAPPING.get_or_init(Default::default)
                    //     .write()
                    //     .unwrap()
                    //     .insert(vfs_path, encode_path(asset_path));
                    //     .insert(vfs_path, Self::encode_path(asset_path));
                } else {
                    paths_to_scan.push_back(entry.path());
                }
            }
        }

        Ok(())
    }
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
    encode_string(
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
