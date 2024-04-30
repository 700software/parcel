use std::path::PathBuf;

use parcel_core::{
  fs::memory_file_system::MemoryFileSystem, package_manager::MockPackageManager,
  parcel_config::ParcelConfig,
};

fn main() {
  let fs = MemoryFileSystem::default();
  let package_manager = MockPackageManager::new();
  let config = ParcelConfig::new(&fs, &package_manager);

  let err = config.load(&PathBuf::from("/"), None, None);

  println!("{}", err.unwrap_err());
}
