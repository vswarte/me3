use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf, StripPrefixError};

use thiserror::Error;

use crate::log_file;

#[derive(Debug, Default)]
pub struct AssetMapping {
    map: HashMap<String, String>,
    virtual_roots: HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum AssetMappingError {
    #[error("Package source specified is not a directory {0}.")]
    InvalidDirectory(PathBuf),

    #[error("Could not read directory while discovering override assets {0}")]
    ReadDir(std::io::Error),

    #[error("Could not acquire directory entry")]
    DirEntryAcquire(std::io::Error),

    #[error("Could not acquire directory entry")]
    StripPrefix(#[from] StripPrefixError),
}

impl AssetMapping {
    pub fn new(virtual_roots: HashMap<String, String>) -> Self {
        Self {
            virtual_roots,
            ..Default::default()
        }
    }

    ///  Traverses a folder structure, mapping discovered assets into itself.
    pub fn scan_directory<P: AsRef<Path>>(
        &mut self,
        base_directory: P,
    ) -> Result<(), AssetMappingError> {
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
                    let vfs_path = path_to_asset_lookup_key(
                        normalize_path(base_directory).as_path(),
                        asset_path.as_path(),
                    )?;

                    self.map.insert(
                        vfs_path,
                        asset_path.to_string_lossy().to_string(),
                    );
                } else {
                    paths_to_scan.push_back(entry.path());
                }
            }
        }

        Ok(())
    }

    pub fn get_override(&self, path: &str) -> Option<&String> {
        let key = self.resolve_virtual_root(path);

        log_file().write().unwrap()
            .write_all(format!("Lookup key: {key}\n").as_bytes()).unwrap();

        self.map.get(&key)
    }

    fn resolve_virtual_root(&self, input: &str) -> String {
        input.split_once(":/")
            .and_then(|r| self.virtual_roots.get(r.0).map(|a| (r.1, a)))
            .map(|r| format!("{}{}", r.1, r.0))
            .unwrap_or(input.to_string())
    }
}

/// Normalizes paths to use / as a path seperator
fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    PathBuf::from(path.as_ref().to_string_lossy().replace('\\', "/"))
}

/// Turns an asset path into an asset lookup key using the mods base path.
fn path_to_asset_lookup_key<P: AsRef<Path>>(base: P, path: P) -> Result<String, StripPrefixError>{
    path.as_ref().strip_prefix(base)
        .map(|p| {
            // TODO: This doesn't have to be here if we get the game to resolve
            // HACK: Account for one-off with hkxbhds
            let path = p.to_string_lossy().to_lowercase();

            if path.contains("-hkxbhd") {
                path.split('/')
                    .filter(|p| !p.contains("-hkxbhd"))
                    .filter(|p| p.len() != 3 || p == &"map")
                    .collect::<Vec<_>>()
                    .join("/")
            } else {
                path
            }
        })
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use crate::mapping::{path_to_asset_lookup_key, AssetMapping};

    #[test]
    fn file_request_path_virtual_root_rewrites() {
        let mut asset_mapping = AssetMapping::new(HashMap::from([
            (String::from("data0"), String::from("")),
            (String::from("data1"), String::from("")),
            (String::from("data2"), String::from("")),
            (String::from("data3"), String::from("")),
            (String::from("regulation"), String::from("")),
            (String::from("event"), String::from("event/")),
            (String::from("sfxbnd"), String::from("sfx/")),
        ]));

        assert_eq!(
            asset_mapping.resolve_virtual_root("regulation:/regulation.bin"),
            "regulation.bin"
        );

        assert_eq!(
            asset_mapping.resolve_virtual_root("data0:/menu/02_010_equiptop.gfx"),
            "menu/02_010_equiptop.gfx"
        );

        assert_eq!(
            asset_mapping.resolve_virtual_root("event:/m60_41_38_00.emevd.dcx"),
            "event/m60_41_38_00.emevd.dcx"
        );
    }

    #[test]
    fn asset_path_lookup_keys() {
        const FAKE_MOD_BASE: &str = "D:/ModBase/"; 
        let base_path = PathBuf::from(FAKE_MOD_BASE);

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                &PathBuf::from(format!("{FAKE_MOD_BASE}/parts/aet/aet007/aet007_071.tpf.dcx")),
            ).unwrap(),
            "parts/aet/aet007/aet007_071.tpf.dcx",
        );

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                &PathBuf::from(format!("{FAKE_MOD_BASE}/hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx")),
            ).unwrap(),
            "hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx",
        );

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                &PathBuf::from(format!("{FAKE_MOD_BASE}/regulation.bin")),
            ).unwrap(),
            "regulation.bin",
        );
    }
}
