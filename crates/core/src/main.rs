use std::{collections::HashMap, path::PathBuf};

use parcel_core::{
  fs::memory_file_system::MemoryFileSystem, package_manager::MockPackageManager,
  parcel_config::ParcelConfig,
};

fn main() {
  let config = ParcelConfig::new(
    MemoryFileSystem::new(HashMap::new()),
    MockPackageManager::new(),
  );

  let err = config.load(&PathBuf::from("/"), None, None);

  println!("{}", err.unwrap_err());
}
