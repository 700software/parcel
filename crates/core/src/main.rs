use std::{collections::HashMap, env};

use parcel_core::{
  config::parcel_rc_config::ParcelRcConfig, fs::memory_file_system::MemoryFileSystem,
  package_manager::MockPackageManager,
};

fn main() {
  let project_root = env::current_dir().unwrap();
  let parcel_rc = String::from(
    r#"
      {
        "bundler": "@parcel/bundler-default",
        "compressors": {
          "*": ["@parcel/compressor-raw"]
        },
        "namers": ["@parcel/namer-default"],
        "optimizers": {
          "*.{js,mjs,cjs}": ["@parcel/optimizer-swc"]
        },
        "packagers": {
          "*.{js,mjs,cjs}": "@parcel/packager-js"
        },
        "reporters": ["@parcel/reporter-dev-server"],
        "resolvers": ["@parcel/resolver-default"],
        "runtimes": ["@parcel/runtime-js"],
        "transformers": {
          "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": [
            "@parcel/transformer-js"
          ],
        }
      }
    "#,
  );

  let (parcel_config, _files) = ParcelRcConfig::new(
    &MemoryFileSystem::new(HashMap::from([(project_root.join(".parcelrc"), parcel_rc)])),
    &MockPackageManager::default(),
  )
  .load(&project_root, None, None)
  .unwrap();
}
