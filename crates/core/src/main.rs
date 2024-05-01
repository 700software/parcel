use std::path::PathBuf;

use parcel_core::{
  config::parcel_rc_config::ParcelRcConfig, fs::memory_file_system::MemoryFileSystem,
  package_manager::MockPackageManager,
};

fn main() {
  let fs = MemoryFileSystem::default();
  let package_manager = MockPackageManager::new();
  let config = ParcelRcConfig::new(&fs, &package_manager);

  let err = config.load(&PathBuf::from("/"), None, None);

  println!("{}", err.unwrap_err());
}
